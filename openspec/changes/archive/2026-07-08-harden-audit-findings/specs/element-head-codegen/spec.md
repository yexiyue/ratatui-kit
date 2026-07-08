## ADDED Requirements

### Requirement: with_layout_style 误用给出友好编译错误

`#[with_layout_style]` 应用于非具名字段结构体(元组结构体或单元结构体)时 SHALL 产生指向该结构体、说明"只能用于具名字段结构体"的 `compile_error!`,MUST NOT 让用户看到宏展开后误导性的下游报错(如 "no field ...")。

#### Scenario: 元组/单元结构体上的清晰报错
- **WHEN** 在元组结构体或单元结构体上标注 `#[with_layout_style]`
- **THEN** 编译报错明确指出"只能用于具名字段结构体",而非展开内部的误导性错误

#### Scenario: 具名字段结构体正常注入
- **WHEN** 在具名字段结构体上标注 `#[with_layout_style]`
- **THEN** 正常注入布局字段并生成 `layout_style()`(行为不变)

### Requirement: 宏 codegen 无多余中转(行为不变)

`element!`/adapter 的 codegen 在无 props/无 children 分支 SHALL NOT 产生多余的 `let mut` 中转绑定;adapter 输出形态 SHALL 与 `to_element_expr` 的"带外层括号元素表达式"约定一致。该清理 MUST NOT 改变任何展开后的运行时行为。

#### Scenario: 清理后行为等价
- **WHEN** 清理无 props/无 children 分支的 codegen 中转后重新编译
- **THEN** 现有 `element!`/`routes!`/adapter 用例行为与清理前完全一致(测试不回归)
