use ratatui::{
    style::{Style, Stylize},
    text::Line,
};
use ratatui_kit::ratatui::{self, layout::Constraint};
use ratatui_kit::{prelude::*, ratatui::layout::Direction};

#[tokio::main]
async fn main() {
    element!(Counter)
        .into_any()
        .fullscreen()
        .await
        .expect("Failed to run the application");
}

#[component]
fn Counter(mut hooks: Hooks) -> impl Into<AnyElement<'static>> {
    let mut state = hooks.use_state(|| 0);
    hooks.use_future(async move {
        loop {
            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            state += 1;
        }
    });

    let scroll_view_state = hooks.use_state(ScrollViewState::default);

    hooks.use_local_events(move |event| {
        scroll_view_state.write().handle_event(&event);
    });

    element!(
        View(
            flex_direction:ratatui::layout::Direction::Horizontal,
            gap:4,
        ){
            View{
                Border{
                    $Line::styled(
                        format!("Counter: {state}"),
                        Style::default().fg(ratatui::style::Color::Green).bold(),
                    )
                    .centered()
                    .bold()
                    .underlined()
                }
            }
            ScrollView(
                flex_direction:Direction::Vertical,
                scroll_view_state: scroll_view_state.get(),
            ){
                View( flex_direction:Direction::Horizontal,width:Constraint::Percentage(200), ){
                    View{
                        Border{
                            $Line::styled(
                                format!("Counter: {state}"),
                                Style::default().fg(ratatui::style::Color::Green).bold(),
                            )
                            .centered()
                            .bold()
                            .underlined()
                        }
                    }
                    View{
                        Border{
                            $Line::styled(
                                format!("Counter: {state}"),
                                Style::default().fg(ratatui::style::Color::Green).bold(),
                            )
                            .centered()
                            .bold()
                            .underlined()
                        }
                    }
                }
                View{
                    Border{
                        $Line::styled(
                            format!("Counter: {state}"),
                            Style::default().fg(ratatui::style::Color::Green).bold(),
                        )
                        .centered()
                        .bold()
                        .underlined()
                    }
                }
            }
        }
    )
}
