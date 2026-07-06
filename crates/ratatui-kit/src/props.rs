use std::any::TypeId;

use ratatui_kit_macros::Props;

// 组件属性 trait，所有可作为组件 props 的类型都需实现此 trait。
//
// - 推荐使用 `#[derive(Props)]` 自动实现。
pub trait Props {}

// 用于处理原始指针释放的trait
// 通过类型擦除实现对未知类型的内存释放
trait DropRaw {
    fn drop_raw(&self, raw: *mut ());
}

// 类型擦除的具体实现
// 使用PhantomData标记实际类型T
struct DropRawImpl<T> {
    _marker: std::marker::PhantomData<T>,
}

impl<T> DropRaw for DropRawImpl<T> {
    // 实际执行内存释放的方法
    // 将原始指针转换为Box后释放
    fn drop_raw(&self, raw: *mut ()) {
        unsafe {
            let _ = Box::from_raw(raw as *mut T);
        }
    }
}

// 类型擦除容器
// 支持存储任意类型的属性（owned/borrowed）
// raw: 指向实际数据的原始指针
// drop: 可选的drop处理函数（对于owned类型）
// _marker: 生命周期标记
#[doc(hidden)]
pub struct AnyProps<'a> {
    raw: *mut (),
    type_id: TypeId,
    drop: Option<Box<dyn DropRaw + 'a>>,
    _marker: std::marker::PhantomData<&'a mut ()>,
}

impl<'a> AnyProps<'a> {
    // 创建拥有所有权的AnyProps实例
    // T: 实现Props trait的类型
    pub(crate) fn owned<T>(props: T, type_id: TypeId) -> Self
    where
        T: Props + 'a,
    {
        // 将属性堆分配并转换为原始指针
        let raw = Box::into_raw(Box::new(props));
        Self {
            raw: raw as *mut (),
            type_id,
            // 保存对应的drop处理实现
            drop: Some(Box::new(DropRawImpl::<T> {
                _marker: std::marker::PhantomData,
            })),
            _marker: std::marker::PhantomData,
        }
    }

    // 创建借用的AnyProps实例
    // 不持有所有权，不负责内存释放
    pub(crate) fn borrowed<T: Props>(props: &'a mut T, type_id: TypeId) -> Self {
        Self {
            raw: props as *const _ as *mut (),
            type_id,
            drop: None, // 不负责内存释放
            _marker: std::marker::PhantomData,
        }
    }

    // 创建只读借用版本
    // drop字段设为None表示不处理内存释放
    pub(crate) fn borrow(&mut self) -> AnyProps<'_> {
        Self {
            raw: self.raw,
            type_id: self.type_id,
            drop: None,
            _marker: std::marker::PhantomData,
        }
    }

    // 不安全的下转型方法（不可变引用）
    // 调用者必须确保实际类型与T匹配
    pub(crate) unsafe fn downcast_ref_unchecked<T: Props>(&self, expected_type_id: TypeId) -> &T {
        debug_assert_eq!(
            self.type_id, expected_type_id,
            "AnyProps type mismatch before immutable downcast"
        );
        unsafe { &*(self.raw as *const T) }
    }

    // 不安全的下转型方法（可变引用）
    // 调用者必须确保实际类型与T匹配
    pub(crate) unsafe fn downcast_mut_unchecked<T: Props>(
        &mut self,
        expected_type_id: TypeId,
    ) -> &mut T {
        debug_assert_eq!(
            self.type_id, expected_type_id,
            "AnyProps type mismatch before mutable downcast"
        );
        unsafe { &mut *(self.raw as *mut T) }
    }
}

// 实现Drop trait用于资源释放
impl Drop for AnyProps<'_> {
    fn drop(&mut self) {
        // 如果存在drop处理器，执行内存释放
        if let Some(drop) = self.drop.take() {
            drop.drop_raw(self.raw);
        }
    }
}

#[derive(Debug, Clone, Default, Props)]
// 空属性类型，表示组件不需要任何 props。
//
// 可用于无参数组件或默认占位。
pub struct NoProps;
