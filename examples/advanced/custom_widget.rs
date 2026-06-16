//! 原生 ratatui widget 桥接示例。
//!
//! 展示手写 `Component` 如何在 `draw` 中渲染一个 stateful ratatui widget。

use ratatui_kit::{
    crossterm::event::{Event, KeyCode, KeyEventKind},
    prelude::*,
    ratatui::{
        layout::{Constraint, Direction, Flex},
        style::{Color, Style, Stylize},
        text::{Line, Span},
        widgets::{Block, Borders, List, ListItem, ListState, Padding},
    },
};

#[derive(Clone, Copy)]
struct Deployment {
    service: &'static str,
    target: &'static str,
    status: &'static str,
    latency: u16,
}

const DEPLOYMENTS: [Deployment; 5] = [
    Deployment {
        service: "api-gateway",
        target: "production",
        status: "ready",
        latency: 34,
    },
    Deployment {
        service: "worker-sync",
        target: "staging",
        status: "running",
        latency: 48,
    },
    Deployment {
        service: "search-index",
        target: "canary",
        status: "queued",
        latency: 61,
    },
    Deployment {
        service: "billing-ledger",
        target: "review",
        status: "blocked",
        latency: 93,
    },
    Deployment {
        service: "docs-site",
        target: "preview",
        status: "ready",
        latency: 27,
    },
];

#[tokio::main]
async fn main() {
    element!(CustomWidgetApp)
        .fullscreen()
        .await
        .expect("Failed to run the application");
}

#[component]
fn CustomWidgetApp(mut hooks: Hooks) -> impl Into<AnyElement<'static>> {
    let list_state = hooks.use_state(|| {
        let mut state = ListState::default();
        state.select(Some(0));
        state
    });
    let mut status = hooks.use_state(|| "ready".to_string());
    let mut exit = hooks.use_exit();

    hooks.use_event_handler(EventScope::Current, EventPriority::Normal, move |event| {
        let Event::Key(key) = event else {
            return EventResult::Ignored;
        };
        if key.kind != KeyEventKind::Press {
            return EventResult::Ignored;
        }

        match key.code {
            KeyCode::Char('j') | KeyCode::Down => {
                move_selection(list_state, 1);
                status.set(selection_message("selected", list_state));
            }
            KeyCode::Char('k') | KeyCode::Up => {
                move_selection(list_state, -1);
                status.set(selection_message("selected", list_state));
            }
            KeyCode::Enter => {
                status.set(selection_message("submitted", list_state));
            }
            KeyCode::Char('q') | KeyCode::Char('Q') => exit(),
            _ => return EventResult::Ignored,
        }

        EventResult::Consumed
    });

    let selected = list_state
        .read()
        .selected()
        .and_then(|index| DEPLOYMENTS.get(index).copied())
        .unwrap_or(DEPLOYMENTS[0]);
    let status_view = status.read().clone();
    let queue_props = DeployQueueProps { state: list_state };

    element!(
        Center(
            width: Constraint::Length(92),
            height: Constraint::Length(22),
        ) {
            Border(
                flex_direction: Direction::Vertical,
                border_style: Style::new().blue(),
                top_title: Line::from(" custom widget bridge ").blue().bold().centered(),
                bottom_title: Line::from(" j/k move | Enter submit | q quit ").dark_gray().centered(),
            ) {
                View(
                    flex_direction: Direction::Horizontal,
                    gap: 2,
                ) {
                    View(width: Constraint::Fill(2)) {
                        DeployQueue(..queue_props)
                    }
                    View(
                        width: Constraint::Fill(1),
                        flex_direction: Direction::Vertical,
                        gap: 1,
                    ) {
                        Border(
                            height: Constraint::Length(8),
                            flex_direction: Direction::Vertical,
                            justify_content: Flex::Center,
                            border_style: Style::new().cyan(),
                            top_title: Line::from(" selected ").cyan().bold().centered(),
                        ) {
                            Text(text: Line::from(format!("service: {}", selected.service)).centered())
                            Text(text: Line::from(format!("target:  {}", selected.target)).centered())
                            Text(text: Line::from(format!("status:  {}", selected.status)).centered())
                            Text(text: Line::from(format!("latency: {}ms", selected.latency)).centered())
                        }
                        Border(
                            height: Constraint::Fill(1),
                            flex_direction: Direction::Vertical,
                            justify_content: Flex::Center,
                            border_style: Style::new().green(),
                            top_title: Line::from(" event ").green().bold().centered(),
                        ) {
                            Text(text: Line::from(status_view).centered(), wrap: true)
                        }
                    }
                }
            }
        }
    )
}

#[derive(Props)]
struct DeployQueueProps {
    state: State<ListState>,
}

struct DeployQueue {
    state: State<ListState>,
}

impl Component for DeployQueue {
    type Props<'a> = DeployQueueProps;

    fn new(props: &Self::Props<'_>) -> Self {
        Self { state: props.state }
    }

    fn update(
        &mut self,
        props: &mut Self::Props<'_>,
        _hooks: Hooks,
        _updater: &mut ComponentUpdater,
    ) {
        self.state = props.state;
    }

    fn draw(&mut self, drawer: &mut ComponentDrawer<'_, '_>) {
        let items = DEPLOYMENTS
            .iter()
            .map(|deployment| {
                ListItem::new(Line::from(vec![
                    Span::raw(format!("{:<15}", deployment.service)),
                    Span::styled(
                        format!(" {:<10}", deployment.target),
                        Style::new().fg(Color::Cyan),
                    ),
                    Span::styled(
                        format!(" {:<8}", deployment.status),
                        status_style(deployment.status),
                    ),
                    Span::raw(format!(" {:>3}ms", deployment.latency)),
                ]))
            })
            .collect::<Vec<_>>();

        let list = List::new(items)
            .block(
                Block::new()
                    .borders(Borders::ALL)
                    .border_style(Style::new().dark_gray())
                    .title(Line::from(" deploy queue ").cyan().bold().centered())
                    .padding(Padding::horizontal(1)),
            )
            .highlight_style(Style::new().black().on_cyan())
            .highlight_symbol("> ");

        drawer.render_stateful_widget(list, drawer.area, &mut self.state.write_no_update());
    }
}

fn move_selection(state: State<ListState>, offset: isize) {
    let selected = state.read().selected().unwrap_or(0);
    let next = selected
        .saturating_add_signed(offset)
        .min(DEPLOYMENTS.len().saturating_sub(1));
    state.write().select(Some(next));
}

fn selection_message(action: &str, state: State<ListState>) -> String {
    let selected = state.read().selected().unwrap_or(0);
    let deployment = DEPLOYMENTS[selected];
    format!("{action}: {} -> {}", deployment.service, deployment.target)
}

fn status_style(status: &str) -> Style {
    match status {
        "ready" => Style::new().fg(Color::Green),
        "running" => Style::new().fg(Color::Yellow),
        "queued" => Style::new().fg(Color::Magenta),
        "blocked" => Style::new().fg(Color::Red),
        _ => Style::new(),
    }
}
