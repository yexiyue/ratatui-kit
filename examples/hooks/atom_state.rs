//! Atom 全局状态示例。
//!
//! 展示进程级 Atom 如何在多个页面和组件之间共享，同时把临时输入草稿留在局部 `use_state`。

use ratatui_kit::{
    crossterm::event::{Event, KeyCode, KeyEventKind},
    prelude::tui_input::backend::crossterm::EventHandler,
    prelude::*,
    ratatui::{
        layout::{Alignment, Constraint, Direction, Flex},
        style::{Color, Style, Stylize},
        text::Line,
    },
};

static FOCUS: Atom<String> = Atom::new(|| "Review runtime".to_string());
static SCORE: Atom<i32> = Atom::new(|| 2);
static LAST_EVENT: Atom<String> = Atom::new(|| "dashboard initialized".to_string());

#[tokio::main]
async fn main() {
    let routes = routes! {
        "/" => Dashboard,
        "/focus" => FocusEditor,
        "/score" => ScoreEditor,
    };

    element!(RouterProvider(
        routes: routes,
        index_path: "/",
    ))
    .fullscreen()
    .await
    .expect("Failed to run the application");
}

#[component]
fn Dashboard(mut hooks: Hooks) -> impl Into<AnyElement<'static>> {
    let mut navigate = hooks.use_navigate();
    let mut exit = hooks.use_exit();

    hooks.use_event_handler(EventScope::Current, EventPriority::Normal, move |event| {
        let Event::Key(key) = event else {
            return EventResult::Ignored;
        };
        if key.kind != KeyEventKind::Press {
            return EventResult::Ignored;
        }

        match key.code {
            KeyCode::Char('1') => {
                navigate.push("/focus");
                EventResult::Consumed
            }
            KeyCode::Char('2') => {
                navigate.push("/score");
                EventResult::Consumed
            }
            KeyCode::Char('q') | KeyCode::Char('Q') => {
                exit();
                EventResult::Consumed
            }
            _ => EventResult::Ignored,
        }
    });

    element!(
        Center(
            width: Constraint::Length(94),
            height: Constraint::Length(22),
        ) {
            Border(
                flex_direction: Direction::Vertical,
                gap: 1,
                border_style: Style::new().blue(),
                top_title: Line::from(" atom state ").blue().bold().centered(),
                bottom_title: Line::from(" 1 focus editor | 2 score editor | q quit ").dark_gray().centered(),
            ) {
                View(
                    flex_direction: Direction::Horizontal,
                    gap: 2,
                    height: Constraint::Length(8),
                ) {
                    FocusCard
                    ScoreCard
                }
                EventCard
                Border(
                    flex_direction: Direction::Vertical,
                    border_style: Style::new().dark_gray(),
                    top_title: Line::from(" state boundary ").dark_gray().centered(),
                ) {
                    View(height: Constraint::Length(1)) {
                        Text(text: Line::from("Atom stores committed app state.").centered())
                    }
                    View(height: Constraint::Length(1)) {
                        Text(text: Line::from("Editors keep temporary drafts in local use_state.").centered())
                    }
                    View(height: Constraint::Length(1)) {
                        Text(text: Line::from("Only subscribed components wake when atoms change.").centered())
                    }
                }
            }
        }
    )
}

#[component]
fn FocusCard(mut hooks: Hooks) -> impl Into<AnyElement<'static>> {
    let focus = hooks.use_atom(&FOCUS);
    element!(
        Border(
            width: Constraint::Fill(1),
            flex_direction: Direction::Vertical,
            justify_content: Flex::Center,
            border_style: Style::new().cyan(),
            top_title: Line::from(" shared focus ").cyan().centered(),
        ) {
            View(height: Constraint::Length(1)) {
                Text(
                    text: Line::from(focus.read().to_string()).yellow().bold(),
                    alignment: Alignment::Center,
                )
            }
            View(height: Constraint::Length(1)) {
                Text(text: Line::from("subscribes to the FOCUS atom").dark_gray().centered())
            }
        }
    )
}

#[component]
fn ScoreCard(mut hooks: Hooks) -> impl Into<AnyElement<'static>> {
    let score = hooks.use_atom(&SCORE);
    element!(
        Border(
            width: Constraint::Fill(1),
            flex_direction: Direction::Vertical,
            justify_content: Flex::Center,
            border_style: Style::new().cyan(),
            top_title: Line::from(" shared score ").cyan().centered(),
        ) {
            View(height: Constraint::Length(1)) {
                Text(
                    text: Line::from(format!("{:02}", score.get())).green().bold(),
                    alignment: Alignment::Center,
                )
            }
            View(height: Constraint::Length(1)) {
                Text(text: Line::from("same Atom, different component").dark_gray().centered())
            }
        }
    )
}

#[component]
fn EventCard(mut hooks: Hooks) -> impl Into<AnyElement<'static>> {
    let last_event = hooks.use_atom(&LAST_EVENT);
    element!(
        Border(
            height: Constraint::Length(5),
            flex_direction: Direction::Vertical,
            justify_content: Flex::Center,
            border_style: Style::new().cyan(),
            top_title: Line::from(" last event ").cyan().centered(),
        ) {
            Text(
                text: Line::from(last_event.read().to_string()),
                alignment: Alignment::Center,
            )
        }
    )
}

#[component]
fn FocusEditor(mut hooks: Hooks) -> impl Into<AnyElement<'static>> {
    let draft = hooks.use_state(tui_input::Input::default);
    let mut focus = hooks.use_atom(&FOCUS);
    let mut last_event = hooks.use_atom(&LAST_EVENT);
    let mut navigate = hooks.use_navigate();

    hooks.use_event_handler(EventScope::Current, EventPriority::Normal, move |event| {
        let Event::Key(key) = &event else {
            return EventResult::Ignored;
        };
        if key.kind != KeyEventKind::Press {
            return EventResult::Ignored;
        }

        match key.code {
            KeyCode::Esc => {
                last_event.set("focus edit canceled".to_string());
                navigate.back();
            }
            KeyCode::Enter => {
                let value = draft.read().value().trim().to_string();
                if !value.is_empty() {
                    focus.set(value.clone());
                    last_event.set(format!("focus committed: {value}"));
                } else {
                    last_event.set("empty focus ignored".to_string());
                }
                navigate.replace("/");
            }
            _ => {
                draft.write().handle_event(&event);
                let value = draft.read().value().to_string();
                if value.is_empty() {
                    last_event.set("draft cleared".to_string());
                } else {
                    last_event.set(format!("drafting: {value}"));
                }
            }
        }

        EventResult::Consumed
    });

    let focus = focus.read().to_string();

    element!(
        Center(
            width: Constraint::Length(94),
            height: Constraint::Length(18),
        ) {
            Border(
                flex_direction: Direction::Vertical,
                gap: 1,
                border_style: Style::new().yellow(),
                top_title: Line::from(" focus editor ").yellow().bold().centered(),
                bottom_title: Line::from(" Enter commit to Atom | Esc back ").dark_gray().centered(),
            ) {
                View(height: Constraint::Length(1)) {
                    Text(text: Line::from(format!("current atom value: {focus}")).centered())
                }
                Border(
                    height: Constraint::Length(3),
                    border_style: Style::new().cyan(),
                    top_title: Line::from(" local draft ").cyan().centered(),
                ) {
                    Input(
                        input: draft.read().clone(),
                        placeholder: "type a new focus".to_string(),
                        placeholder_style: Style::new().dark_gray(),
                        cursor_style: Style::new().bg(Color::Yellow),
                        hide_cursor: false,
                    )
                }
                View(height: Constraint::Length(1)) {
                    Text(text: Line::from("Draft is local use_state; Enter writes FOCUS Atom.").centered())
                }
            }
        }
    )
}

#[component]
fn ScoreEditor(mut hooks: Hooks) -> impl Into<AnyElement<'static>> {
    let mut score = hooks.use_atom(&SCORE);
    let mut last_event = hooks.use_atom(&LAST_EVENT);
    let mut navigate = hooks.use_navigate();

    hooks.use_event_handler(EventScope::Current, EventPriority::Normal, move |event| {
        let Event::Key(key) = event else {
            return EventResult::Ignored;
        };
        if key.kind != KeyEventKind::Press {
            return EventResult::Ignored;
        }

        match key.code {
            KeyCode::Char('+') | KeyCode::Char('=') => {
                score += 1;
                last_event.set(format!("score increased to {}", score.get()));
                EventResult::Consumed
            }
            KeyCode::Char('-') if score.get() > 0 => {
                score -= 1;
                last_event.set(format!("score decreased to {}", score.get()));
                EventResult::Consumed
            }
            KeyCode::Char('r') | KeyCode::Char('R') => {
                score.set(0);
                last_event.set("score reset to 0".to_string());
                EventResult::Consumed
            }
            KeyCode::Esc => {
                navigate.back();
                EventResult::Consumed
            }
            _ => EventResult::Ignored,
        }
    });

    element!(
        Center(
            width: Constraint::Length(76),
            height: Constraint::Length(16),
        ) {
            Border(
                flex_direction: Direction::Vertical,
                justify_content: Flex::Center,
                gap: 1,
                border_style: Style::new().green(),
                top_title: Line::from(" score editor ").green().bold().centered(),
                bottom_title: Line::from(" + increase | - decrease | r reset | Esc back ").dark_gray().centered(),
            ) {
                View(height: Constraint::Length(1)) {
                    Text(
                        text: Line::from(format!("score atom: {}", score.get())).green().bold(),
                        alignment: Alignment::Center,
                    )
                }
                View(height: Constraint::Length(1)) {
                    Text(text: Line::from("This page and dashboard share SCORE.").centered())
                }
            }
        }
    )
}
