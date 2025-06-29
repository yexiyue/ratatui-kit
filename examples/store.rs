use ratatui::{
    style::{Style, Stylize},
    text::Line,
};
use ratatui_kit::ratatui;
use ratatui_kit::{
    crossterm::event::{Event, KeyCode, KeyEventKind},
    prelude::*,
    ratatui::layout::Constraint,
};

#[derive(Store)]
pub struct CounterAndTextInput {
    pub count: i32,
    pub value: String,
    pub is_focus: bool,
    pub should_exit: bool,
}

impl Default for CounterAndTextInput {
    fn default() -> Self {
        Self {
            count: 0,
            value: String::new(),
            is_focus: true,
            should_exit: false,
        }
    }
}

#[tokio::main]
async fn main() {
    // console_subscriber::init();

    // Using the `routes!` macro to define routes
    let routes = routes! {
        "/" => Counter {
            "/counter/:title/:test" => Counter2,
        },
        "/text-input" => MyTextInput
    };

    element!(RouterProvider(
        routes:routes,
        index_path:"/text-input",
    ))
    .into_any()
    .fullscreen()
    .await
    .expect("Failed to run the application");
}

#[component]
fn Counter(mut hooks: Hooks) -> impl Into<AnyElement<'static>> {
    let store = &COUNTER_AND_TEXT_INPUT_STORE;
    let mut state = use_stores!(store.count);

    hooks.use_future(async move {
        loop {
            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            state += 1;
        }
    });

    element!(
        View{
            Fragment{
                $Line::styled(
                    format!("Counter: {}", state),
                    Style::default().fg(ratatui::style::Color::Green).bold(),
                )
                .centered()
                .bold()
                .underlined()
            }
            Outlet
        }
    )
}

#[component]
fn MyTextInput(mut hooks: Hooks) -> impl Into<AnyElement<'static>> {
    // let mut value = hooks.use_state(String::new);
    // let mut is_focus = hooks.use_state(|| true);
    // let mut should_exit = hooks.use_state(|| false);

    let store = &COUNTER_AND_TEXT_INPUT_STORE;
    let (mut value, mut is_focus, mut should_exit) =
        use_stores!(store.value, store.is_focus, store.should_exit);

    let mut system_ctx = hooks.use_context_mut::<SystemContext>();

    let mut navigate = hooks.use_navigate();

    if should_exit.get() {
        system_ctx.exit();
    }

    hooks.use_events(move |event| {
        if let Event::Key(key_event) = event {
            if key_event.kind == KeyEventKind::Press && key_event.code == KeyCode::Esc {
                is_focus.set(false);
            }
            if key_event.kind == KeyEventKind::Press && key_event.code == KeyCode::Enter {
                is_focus.set(true);
                navigate.push("/counter/hello world params/111");
            }
            if key_event.kind == KeyEventKind::Press && key_event.code == KeyCode::Char('q') {
                should_exit.set(true);
            }
        }
    });

    element!(Border(
        height:Constraint::Length(5),
        style:if is_focus.get() {
            Style::default().green()
        } else {
            Style::default()
        },

    ) {
        TextArea(
            value: value.read().to_string(),
            is_focus:is_focus.get(),
            on_change: move |new_value: String| {
                value.set(new_value);
            },
            multiline: true,
            cursor_style: if is_focus.get() {
                Style::default().on_green()
             } else {
                Style::default()
            },
            placeholder: Some("Type something...".to_string()),
            placeholder_style:  if is_focus.get() {
                Style::default().green()
             } else {
                Style::default().dim()
            },
        )
    })
}

#[component]
fn Counter2(mut hooks: Hooks) -> impl Into<AnyElement<'static>> {
    let mut state = hooks.use_state(|| 0);

    let mut navigate = hooks.use_navigate();
    // let title = hooks.use_route_state::<String>();
    // let title = &*title.unwrap();
    let title = hooks.use_params().get("title").cloned().unwrap_or_default();
    let test = hooks.use_params().get("test").cloned().unwrap_or_default();

    hooks.use_future(async move {
        loop {
            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            state += 1;
        }
    });

    hooks.use_events(move |event| {
        if let Event::Key(key_event) = event {
            if key_event.kind == KeyEventKind::Press && key_event.code == KeyCode::Esc {
                navigate.push("/text-input");
            }
        }
    });

    element!(
        $Line::styled(
            format!("{}: {} -- {}",title, state,test),
            Style::default().fg(ratatui::style::Color::Yellow).bold(),
        )
        .centered()
        .bold()
        .underlined()
    )
}

#[component]
fn Counter3(mut hooks: Hooks) -> impl Into<AnyElement<'static>> {
    let mut state = hooks.use_state(|| 0);

    let mut navigate = hooks.use_navigate();

    hooks.use_future(async move {
        loop {
            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            state += 1;
        }
    });

    hooks.use_events(move |event| {
        if let Event::Key(key_event) = event {
            if key_event.kind == KeyEventKind::Press && key_event.code == KeyCode::Enter {
                navigate.replace("/counter/counter2/test");
            }

            if key_event.kind == KeyEventKind::Press && key_event.code == KeyCode::Esc {
                navigate.push("/counter/counter2/test");
            }
        }
    });

    element!(
        $Line::styled(
            format!("Counter3 -- {}", state),
            Style::default().fg(ratatui::style::Color::Yellow).bold(),
        )
        .centered()
        .bold()
        .underlined()
    )
}
