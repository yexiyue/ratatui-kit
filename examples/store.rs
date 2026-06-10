use ratatui::{
    style::{Style, Stylize},
    text::Line,
};
use ratatui_kit::ratatui;
use ratatui_kit::{
    crossterm::event::{Event, KeyCode, KeyEventKind},
    prelude::tui_input::backend::crossterm::EventHandler,
    prelude::*,
    ratatui::layout::Constraint,
};

#[derive(Store, Default)]
pub struct CounterAndTextInput {
    pub count: i32,
    pub value: String,
}

#[tokio::main]
async fn main() {
    let routes = routes! {
        "/" => HomePage,
        "/counter" => CounterPage,
        "/input" => InputPage,

    };

    element!(RouterProvider(
        routes:routes,
        index_path:"/",
    ))
    .fullscreen()
    .await
    .expect("Failed to run the application");
}

#[component]
fn HomePage(mut hooks: Hooks) -> impl Into<AnyElement<'static>> {
    let store = &COUNTER_AND_TEXT_INPUT_STORE;
    let (count, value) = use_stores!(store.count, store.value);
    let mut navigate = hooks.use_navigate();
    hooks.use_events(move |event| {
        if let Event::Key(key_event) = event
            && key_event.kind == KeyEventKind::Press
        {
            match key_event.code {
                KeyCode::Char('1') => navigate.push("/counter"),
                KeyCode::Char('2') => navigate.push("/input"),

                _ => {}
            }
        }
    });
    element!(
        Border(
            style:Style::default().blue(),
            height:Constraint::Length(10),
            gap:1,
            top_title:Line::from("🏠 Store 全局状态仪表盘").centered().bold(),
        ){
            $Line::from(format!("全局计数: {}", count.get()))
            $Line::from(format!("全局输入: {}", value.read().as_str()))
            $Line::from("1. 计数器页面 (Counter)")
            $Line::from("2. 文本输入页面")
        }
    )
}

#[component]
fn CounterPage(mut hooks: Hooks) -> impl Into<AnyElement<'static>> {
    let store = &COUNTER_AND_TEXT_INPUT_STORE;
    let mut count = use_stores!(store.count);
    let value = use_stores!(store.value);
    let mut navigate = hooks.use_navigate();
    hooks.use_future(async move {
        loop {
            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            count += 1;
        }
    });
    hooks.use_events(move |event| {
        if let Event::Key(key_event) = event
            && key_event.kind == KeyEventKind::Press
            && key_event.code == KeyCode::Esc
        {
            navigate.back();
        }
    });
    element!(
        Border(
            style:Style::default().green(),
            height:Constraint::Length(6),
            top_title:Line::from("计数器页面 (ESC 返回)").centered(),
        ){
            $Line::from(format!("全局输入: {}", value.read().as_str()))
            $Line::styled(
                format!("Counter: {}", count.get()),
                Style::default().fg(ratatui::style::Color::Green).bold(),
            ).centered().bold().underlined()
        }
    )
}

#[component]
fn InputPage(mut hooks: Hooks) -> impl Into<AnyElement<'static>> {
    let store = &COUNTER_AND_TEXT_INPUT_STORE;
    let (mut value, count) = use_stores!(store.value, store.count);
    // 注:TextArea 暂时下线,改用单行 Input;输入实时同步到 store 的 String 字段。
    let input = hooks.use_state(tui_input::Input::default);
    let mut navigate = hooks.use_navigate();
    hooks.use_events(move |event| {
        if let Event::Key(key_event) = event
            && key_event.kind == KeyEventKind::Press
        {
            if key_event.code == KeyCode::Esc {
                navigate.back();
            } else {
                input.write().handle_event(&event);
                value.set(input.read().value().to_string());
            }
        }
    });
    element!(
        Border(
            style:Style::default().cyan(),
            height:Constraint::Length(7),
            top_title:Line::from("文本输入页面 (ESC 返回)").centered(),
        ){
            $Line::from(format!("全局计数: {}", count.get()))
            Input(
                input: input.read().clone(),
                cursor_style: Style::default().on_cyan(),
                placeholder: "请输入内容...".to_string(),
                placeholder_style: Style::default().cyan(),
                hide_cursor: false,
            )
        }
    )
}
