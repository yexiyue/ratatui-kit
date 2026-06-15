use any_key::AnyHash;
use std::sync::Arc;

/// 元素在协调(reconciliation)中的身份标识。
///
/// 分两种形态以避免无谓的堆分配:
/// - [`ElementKey::Decl`]:仅由声明点稳定(宏在每个 `element!` 调用点烘焙一个 u128)。
///   这是最常见情形(用户没写 `key:`),**零堆分配**。
/// - [`ElementKey::User`]:用户显式给了 `key:`(列表项需要),存 `(decl_key, 用户值)` 元组,
///   经 `any_key::AnyHash` 类型擦除。**单次堆分配**(此前为 `Arc<Box<dyn>>` 的双重分配)。
///
/// 两个变体天然不互相碰撞;`Decl(同一 u128)` 与此前「无用户 key 用同一 decl_key」语义一致。
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub enum ElementKey {
    /// 仅声明点稳定的 key,零堆分配。
    Decl(u128),
    /// 带用户值的稳定身份(`(decl_key, 用户值)`),单次堆分配。
    User(Arc<dyn AnyHash + Send + Sync>),
}

impl ElementKey {
    /// 构造仅声明点稳定的 key(无用户 `key:`),零堆分配。
    pub fn decl(decl_key: u128) -> Self {
        Self::Decl(decl_key)
    }

    /// 构造带用户值的 key。`key` 通常是 `(decl_key, 用户表达式)` 元组,
    /// 要求其类型 `Eq + Hash + 'static + Send + Sync`(经 `AnyHash` 擦除)。
    pub fn user<T>(key: T) -> Self
    where
        T: Send + Sync + AnyHash,
    {
        Self::User(Arc::new(key))
    }
}

#[cfg(test)]
mod tests {
    use super::ElementKey;
    use std::collections::HashSet;

    #[test]
    fn decl_and_user_never_collide() {
        // 不同变体永不相等,即便内含的 decl_key 相同。
        assert_ne!(ElementKey::decl(42), ElementKey::user((42u128, "x")));
    }

    #[test]
    fn decl_equality_by_value() {
        assert_eq!(ElementKey::decl(7), ElementKey::decl(7));
        assert_ne!(ElementKey::decl(7), ElementKey::decl(8));
    }

    #[test]
    fn user_equality_by_tuple() {
        assert_eq!(
            ElementKey::user((1u128, "a")),
            ElementKey::user((1u128, "a"))
        );
        // 同声明点(decl_key 相同)、不同用户值 → 不等(列表项区分的来源)。
        assert_ne!(
            ElementKey::user((1u128, "a")),
            ElementKey::user((1u128, "b"))
        );
        // 不同声明点、相同用户值 → 不等(位置稳定性的来源)。
        assert_ne!(
            ElementKey::user((1u128, "a")),
            ElementKey::user((2u128, "a"))
        );
    }

    #[test]
    fn hash_consistent_with_eq() {
        let mut set = HashSet::new();
        set.insert(ElementKey::decl(1));
        set.insert(ElementKey::decl(1)); // 重复,去重
        set.insert(ElementKey::user((1u128, "a")));
        // decl(1) 去重为 1 个,user(..) 独立 1 个。
        assert_eq!(set.len(), 2);
    }
}
