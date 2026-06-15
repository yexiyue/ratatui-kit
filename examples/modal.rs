use ratatui::{
    style::{Style, Stylize},
    text::Line,
};
use ratatui_kit::prelude::tui_input::backend::crossterm::EventHandler;
use ratatui_kit::prelude::*;
use ratatui_kit::ratatui;
use ratatui_kit::{
    crossterm::event::{Event, KeyCode, KeyEventKind},
    ratatui::widgets::Paragraph,
};

#[tokio::main]
async fn main() {
    element!(JsonEditor)
        .fullscreen()
        .await
        .expect("Failed to run the application");
}

#[component]
fn JsonEditor(mut hooks: Hooks) -> impl Into<AnyElement<'static>> {
    // 注:TextArea 组件随 ratatui 0.30 迁移暂时下线(tui-textarea 尚无 0.30 兼容版),
    // 这里改用单行 Input 演示;Modal + 实时 JSON 校验逻辑保持不变。
    let json_text = hooks.use_state(|| tui_input::Input::new("{ \"key\": \"value\" }".to_string()));
    let mut open = hooks.use_state(|| false);
    let mut formatted = hooks.use_state(String::new);
    let mut error = hooks.use_state(String::new);

    // 实时解析 JSON
    hooks.use_effect(
        move || match serde_json::from_str::<serde_json::Value>(json_text.read().value()) {
            Ok(val) => {
                let pretty = serde_json::to_string_pretty(&val).unwrap_or_default();
                formatted.set(pretty);
                error.set(String::new());
            }
            Err(e) => {
                formatted.set(String::new());
                error.set(e.to_string());
            }
        },
        [json_text.read().value().to_string()],
    );

    // 事件处理：Tab 弹出 Modal,其余按键交给输入框
    hooks.use_events(move |event| {
        if let Event::Key(key_event) = event
            && key_event.kind == KeyEventKind::Press
        {
            if key_event.code == KeyCode::Tab {
                open.set(!open.get());
            } else {
                json_text.write().handle_event(&event);
            }
        }
    });

    let info_line = if error.read().is_empty() {
        Line::styled("JSON 格式正确", Style::default().green()).centered()
    } else {
        Line::styled(
            format!("JSON 错误: {}", error.read().as_str()),
            Style::default().red(),
        )
        .centered()
    };

    let modal_title = if error.read().is_empty() {
        Line::styled("JSON 格式化：", Style::default().green())
    } else {
        Line::styled("JSON 错误：", Style::default().red())
    };

    let modal_content = if error.read().is_empty() {
        Paragraph::new(formatted.read().clone())
    } else {
        Paragraph::new(error.read().clone())
    };

    element!(
        View{
            View{
                Border(
                    border_style:Style::default().blue(),
                    top_title:info_line,
                    bottom_title:Line::from("按 Tab 查看格式化/校验结果，Ctrl+C 退出").centered(),
                ){
                    Input(
                        input: json_text.read().clone(),
                        cursor_style: Style::default().on_blue(),
                        placeholder: "请输入 JSON...".to_string(),
                        placeholder_style: Style::default().blue(),
                        hide_cursor: false,
                    )
                }
            }
            Modal(
                open:open.get(),
                width:ratatui::layout::Constraint::Percentage(60),
                height:ratatui::layout::Constraint::Percentage(60),
                style:Style::default().dim(),
            ){
                Border(
                    top_title:Line::from("格式化/校验结果").centered().yellow(),
                    padding:ratatui::widgets::Padding::new(2,2,1,1),
                ) {
                    View(height:ratatui::layout::Constraint::Length(1),){
                        $modal_title
                    }
                    View{
                        $modal_content
                    }
                }
            }
        }
    )
}
