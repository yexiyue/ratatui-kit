## ADDED Requirements

### Requirement: 借用派生句柄绑定到借用期

`AnyProps::borrow` SHALL 返回生命周期绑定到 `&mut self` 借用期的句柄(签名形如 `fn borrow(&mut self) -> AnyProps<'_>`);`From<&'a mut AnyElement<'b>> for AnyElement` SHALL 产出借用期 `'a` 的 `AnyElement<'a>`,而非把借用放大到源的 `'b`。派生的借用句柄 MUST NOT 在源可变借用结束后继续可用。

#### Scenario: 借用句柄不可逃逸借用期
- **WHEN** 从 `&'a mut AnyElement<'b>` 派生一个 `AnyElement`
- **THEN** 该派生句柄的生命周期被 `'a` 约束,无法存活到 `'a` 之后(由借用检查器在编译期保证)

#### Scenario: 悬垂用例被编译期拒绝
- **WHEN** trybuild 用例尝试让派生的借用 `AnyElement`/`AnyProps` 逃逸出源的可变借用作用域
- **THEN** 编译失败(借用检查器报错),而非通过编译产生指向已释放数据的悬垂指针

### Requirement: downcast 前类型已校验且 debug 可断言

`AnyProps` SHALL 记录其承载值的 `TypeId`;`downcast_ref_unchecked`/`downcast_mut_unchecked` 在 debug 构建中 SHALL 以 `debug_assert` 校验记录的 `TypeId` 与目标类型一致;release 构建 MUST NOT 引入额外的 TypeId 比较运行时开销。

#### Scenario: 类型不匹配在 debug 期被捕获
- **WHEN** debug 构建中以与承载值不符的类型 `T` 调用 downcast
- **THEN** `debug_assert` 失败并指明类型不匹配,而非读出错误类型造成 UB

#### Scenario: release 零开销
- **WHEN** release 构建执行 downcast
- **THEN** 不存在 TypeId 比较的运行时分支

### Requirement: context 同类型二次借用快速可诊断

当某类型的 context 已被借用、同帧再次借用时,`get_context`/`get_context_mut` SHALL 给出"该 context 已被借用"的诊断,而非误导性的"未找到该 context"。

#### Scenario: 已借用时给出正确诊断
- **WHEN** 持有某类型 context 的守卫期间,再次 `use_context(_mut)` 同类型
- **THEN** 错误信息指明"已被借用"(指向需 drop 守卫),而非"未找到"(误导去查 Provider 注入)
