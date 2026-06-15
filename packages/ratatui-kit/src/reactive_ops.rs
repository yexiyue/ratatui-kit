//! `State<T>` 与 `AtomState<T>` 的算术运算符重载的单一来源。
//!
//! 二者订阅模型不同（本地单订阅 vs 全局多订阅），但运算符实现完全同构：
//! 二元运算经 `get()` 读值、`*Assign` 经 `try_write()` 写值并触发变更通知。
//! 故抽成一个宏，对两个类型各展开一次，避免 ~25 块重复 impl。
//!
//! 约束：目标类型须有 `fn get(&self) -> T`(T: Copy) 与 `fn try_write(&self) -> Option<impl DerefMut<Target = T>>`。

macro_rules! impl_reactive_ops {
    ($Ty:ident) => {
        impl<T: ::core::ops::Add<Output = T> + Copy + Sync + Send + 'static> ::core::ops::Add<T>
            for $Ty<T>
        {
            type Output = T;
            fn add(self, rhs: T) -> T {
                self.get() + rhs
            }
        }
        impl<T: ::core::ops::AddAssign<T> + Copy + Sync + Send + 'static> ::core::ops::AddAssign<T>
            for $Ty<T>
        {
            fn add_assign(&mut self, rhs: T) {
                if let Some(mut v) = self.try_write() {
                    *v += rhs;
                }
            }
        }
        impl<T: ::core::ops::Sub<Output = T> + Copy + Sync + Send + 'static> ::core::ops::Sub<T>
            for $Ty<T>
        {
            type Output = T;
            fn sub(self, rhs: T) -> T {
                self.get() - rhs
            }
        }
        impl<T: ::core::ops::SubAssign<T> + Copy + Sync + Send + 'static> ::core::ops::SubAssign<T>
            for $Ty<T>
        {
            fn sub_assign(&mut self, rhs: T) {
                if let Some(mut v) = self.try_write() {
                    *v -= rhs;
                }
            }
        }
        impl<T: ::core::ops::Mul<Output = T> + Copy + Sync + Send + 'static> ::core::ops::Mul<T>
            for $Ty<T>
        {
            type Output = T;
            fn mul(self, rhs: T) -> T {
                self.get() * rhs
            }
        }
        impl<T: ::core::ops::MulAssign<T> + Copy + Sync + Send + 'static> ::core::ops::MulAssign<T>
            for $Ty<T>
        {
            fn mul_assign(&mut self, rhs: T) {
                if let Some(mut v) = self.try_write() {
                    *v *= rhs;
                }
            }
        }
        impl<T: ::core::ops::Div<Output = T> + Copy + Sync + Send + 'static> ::core::ops::Div<T>
            for $Ty<T>
        {
            type Output = T;
            fn div(self, rhs: T) -> T {
                self.get() / rhs
            }
        }
        impl<T: ::core::ops::DivAssign<T> + Copy + Sync + Send + 'static> ::core::ops::DivAssign<T>
            for $Ty<T>
        {
            fn div_assign(&mut self, rhs: T) {
                if let Some(mut v) = self.try_write() {
                    *v /= rhs;
                }
            }
        }
    };
}

pub(crate) use impl_reactive_ops;
