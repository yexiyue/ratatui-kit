// `#[component]` 只认 props/hooks 参数名：其它参数名应报错。
use ratatui_kit::prelude::*;

#[component]
fn App(foo: u8) -> impl Into<AnyElement<'static>> {
    element!(View {})
}

fn main() {}
