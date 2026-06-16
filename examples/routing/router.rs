use ratatui_kit::{
    crossterm::event::{Event, KeyCode, KeyEventKind},
    prelude::*,
    ratatui::{
        layout::{Constraint, Direction, Flex},
        style::{Style, Stylize},
        text::Line,
    },
};

#[derive(Clone)]
struct RouteNotice {
    message: String,
}

#[derive(Clone, Copy, Default)]
struct Project {
    slug: &'static str,
    name: &'static str,
    owner: &'static str,
    status: &'static str,
    health: u8,
}

const PROJECTS: [Project; 3] = [
    Project {
        slug: "atlas",
        name: "Atlas index",
        owner: "core",
        status: "shipping",
        health: 96,
    },
    Project {
        slug: "boreal",
        name: "Boreal notes",
        owner: "docs",
        status: "review",
        health: 82,
    },
    Project {
        slug: "cygnus",
        name: "Cygnus tasks",
        owner: "apps",
        status: "design",
        health: 71,
    },
];

#[tokio::main]
async fn main() {
    let routes = routes! {
        "/" => AppShell {
            "/" => OverviewPage,
            "/projects/:slug" => ProjectDetailPage,
            "/projects" => ProjectsPage,
            "/activity" => ActivityPage,
        },
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
fn AppShell(mut hooks: Hooks) -> impl Into<AnyElement<'static>> {
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
            KeyCode::Char('1') => navigate.push("/"),
            KeyCode::Char('2') => navigate.push("/projects"),
            KeyCode::Char('3') => navigate.push("/activity"),
            KeyCode::Char('b') => navigate.back(),
            KeyCode::Char('f') => navigate.forward(),
            KeyCode::Char('r') => navigate.replace("/"),
            KeyCode::Char('q') | KeyCode::Char('Q') => exit(),
            _ => return EventResult::Ignored,
        }

        EventResult::Consumed
    });

    element!(
        Center(
            width: Constraint::Length(92),
            height: Constraint::Length(22),
        ) {
            Border(
                flex_direction: Direction::Vertical,
                border_style: Style::new().blue(),
                top_title: Line::from(" router workspace ").blue().bold().centered(),
                bottom_title: Line::from(" 1 overview | 2 projects | 3 activity | b/f history | r reset | q quit ").dark_gray().centered(),
            ) {
                View(
                    flex_direction: Direction::Horizontal,
                    gap: 2,
                ) {
                    Border(
                        width: Constraint::Length(28),
                        flex_direction: Direction::Vertical,
                        border_style: Style::new().dark_gray(),
                        top_title: Line::from(" navigation ").centered(),
                    ) {
                        View(height: Constraint::Length(1)) {
                            Text(text: "1  Overview")
                        }
                        View(height: Constraint::Length(1)) {
                            Text(text: "2  Projects")
                        }
                        View(height: Constraint::Length(1)) {
                            Text(text: "3  Activity")
                        }
                        View(height: Constraint::Length(1)) {
                            Text(text: "")
                        }
                        View(height: Constraint::Length(1)) {
                            Text(text: "b  Back")
                        }
                        View(height: Constraint::Length(1)) {
                            Text(text: "f  Forward")
                        }
                        View(height: Constraint::Length(1)) {
                            Text(text: "r  Replace home")
                        }
                    }
                    View(width: Constraint::Fill(1)) {
                        Outlet
                    }
                }
            }
        }
    )
}

#[component]
fn OverviewPage(_hooks: Hooks) -> impl Into<AnyElement<'static>> {
    element!(
        Border(
            flex_direction: Direction::Vertical,
            justify_content: Flex::Center,
            border_style: Style::new().cyan(),
            top_title: Line::from(" overview ").cyan().bold().centered(),
        ) {
            View(height: Constraint::Length(1)) {
                Text(text: Line::from("RouterProvider owns the history stack.").centered())
            }
            View(height: Constraint::Length(1)) {
                Text(text: Line::from("The shell stays mounted while Outlet swaps pages.").centered())
            }
            View(height: Constraint::Length(1)) {
                Text(text: Line::from("Press 2, choose a project, then use b/f.").centered())
            }
        }
    )
}

#[component]
fn ProjectsPage(mut hooks: Hooks) -> impl Into<AnyElement<'static>> {
    let mut selected = hooks.use_state(|| 0usize);
    let mut navigate = hooks.use_navigate();

    hooks.use_event_handler(EventScope::Current, EventPriority::High, move |event| {
        let Event::Key(key) = event else {
            return EventResult::Ignored;
        };
        if key.kind != KeyEventKind::Press {
            return EventResult::Ignored;
        }

        match key.code {
            KeyCode::Up | KeyCode::Char('k') if selected.get() > 0 => {
                selected -= 1;
            }
            KeyCode::Down | KeyCode::Char('j') if selected.get() + 1 < PROJECTS.len() => {
                selected += 1;
            }
            KeyCode::Enter => {
                let project = PROJECTS[selected.get()];
                navigate.push_with_state(
                    &format!("/projects/{}", project.slug),
                    RouteNotice {
                        message: format!("selected: {}", project.name),
                    },
                );
            }
            _ => return EventResult::Ignored,
        }

        EventResult::Consumed
    });

    let selected_index = selected.get();

    element!(
        Border(
            flex_direction: Direction::Vertical,
            border_style: Style::new().green(),
            top_title: Line::from(" projects ").green().bold().centered(),
            bottom_title: Line::from(" j/k move | Enter open detail ").dark_gray().centered(),
        ) {
            for (index, project) in PROJECTS.into_iter().enumerate() {
                ProjectRow(project: project, active: index == selected_index, key: project.slug)
            }
        }
    )
}

#[derive(Default, Props)]
struct ProjectRowProps {
    project: Project,
    active: bool,
}

#[component]
fn ProjectRow(props: &ProjectRowProps, _hooks: Hooks) -> impl Into<AnyElement<'static>> {
    let marker = if props.active { ">" } else { " " };
    let style = if props.active {
        Style::new().black().on_green()
    } else {
        Style::new()
    };

    element!(
        View(height: Constraint::Length(1)) {
            Text(text: Line::styled(
                format!(
                    "{marker} {:<14} owner {:<4} status {:<8} health {:>3}%",
                    props.project.name,
                    props.project.owner,
                    props.project.status,
                    props.project.health,
                ),
                style,
            ))
        }
    )
}

#[component]
fn ProjectDetailPage(hooks: Hooks) -> impl Into<AnyElement<'static>> {
    let slug = {
        let params = hooks.use_params();
        params.get("slug").cloned().unwrap_or_default()
    };
    let project = project_by_slug(&slug);
    let notice = hooks
        .try_use_route_state::<RouteNotice>()
        .map(|state| state.message.clone())
        .unwrap_or_else(|| "opened without route state".to_string());

    element!(
        Border(
            flex_direction: Direction::Vertical,
            justify_content: Flex::Center,
            border_style: Style::new().magenta(),
            top_title: Line::from(" project detail ").magenta().bold().centered(),
            bottom_title: Line::from(" dynamic params + optional RouteState ").dark_gray().centered(),
        ) {
            if let Some(project) = project {
                View(height: Constraint::Length(1)) {
                    Text(text: Line::from(format!("slug param: {}", project.slug)).centered())
                }
                View(height: Constraint::Length(1)) {
                    Text(text: Line::from(format!("project: {} / {}", project.name, project.status)).centered())
                }
                View(height: Constraint::Length(1)) {
                    Text(text: Line::from(format!("state: {notice}")).centered())
                }
            } else {
                View(height: Constraint::Length(1)) {
                    Text(text: Line::from(format!("unknown project slug: {slug}")).centered())
                }
            }
        }
    )
}

#[component]
fn ActivityPage(_hooks: Hooks) -> impl Into<AnyElement<'static>> {
    element!(
        Border(
            flex_direction: Direction::Vertical,
            justify_content: Flex::Center,
            border_style: Style::new().yellow(),
            top_title: Line::from(" activity ").yellow().bold().centered(),
        ) {
            View(height: Constraint::Length(1)) {
                Text(text: Line::from("History is scoped to this RouterProvider.").centered())
            }
            View(height: Constraint::Length(1)) {
                Text(text: Line::from("b/f moves through history.").centered())
            }
            View(height: Constraint::Length(1)) {
                Text(text: Line::from("r replaces this entry with overview.").centered())
            }
        }
    )
}

fn project_by_slug(slug: &str) -> Option<Project> {
    PROJECTS
        .iter()
        .copied()
        .find(|project| project.slug == slug)
}
