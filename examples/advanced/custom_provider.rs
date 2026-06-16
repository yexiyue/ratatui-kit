//! 自定义 Provider 示例。
//!
//! 展示业务侧如何用 `ContextProvider` 注入作用域配置，并在子树中局部覆盖。

use ratatui_kit::{
    crossterm::event::{Event, KeyCode, KeyEventKind},
    prelude::*,
    ratatui::{
        layout::{Constraint, Direction, Flex},
        style::{Color, Style, Stylize},
        text::{Line, Span},
        widgets::Padding,
    },
};

#[derive(Clone, Copy)]
struct WorkspaceProfile {
    name: &'static str,
    region: &'static str,
    mode: &'static str,
    accent_name: &'static str,
    accent: Color,
    reviewers: usize,
    pipelines: usize,
}

#[derive(Clone, Copy)]
struct WorkspaceSettings {
    profile: WorkspaceProfile,
    density: Density,
}

#[derive(Clone, Copy)]
enum Density {
    Comfortable,
    Compact,
}

const PROFILES: [WorkspaceProfile; 3] = [
    WorkspaceProfile {
        name: "runtime lab",
        region: "local",
        mode: "design",
        accent_name: "cyan",
        accent: Color::Cyan,
        reviewers: 2,
        pipelines: 4,
    },
    WorkspaceProfile {
        name: "release desk",
        region: "prod",
        mode: "audit",
        accent_name: "yellow",
        accent: Color::Yellow,
        reviewers: 4,
        pipelines: 7,
    },
    WorkspaceProfile {
        name: "ops console",
        region: "edge",
        mode: "incident",
        accent_name: "magenta",
        accent: Color::Magenta,
        reviewers: 3,
        pipelines: 5,
    },
];

const AUDIT_PROFILE: WorkspaceProfile = WorkspaceProfile {
    name: "nested audit",
    region: "shadow",
    mode: "override",
    accent_name: "green",
    accent: Color::Green,
    reviewers: 1,
    pipelines: 2,
};

#[tokio::main]
async fn main() {
    element!(CustomProviderApp)
        .fullscreen()
        .await
        .expect("Failed to run the application");
}

#[component]
fn CustomProviderApp(mut hooks: Hooks) -> impl Into<AnyElement<'static>> {
    let mut profile_index = hooks.use_state(|| 0usize);
    let mut density = hooks.use_state(|| Density::Comfortable);
    let mut status = hooks.use_state(|| "provider mounted: runtime lab".to_string());
    let mut exit = hooks.use_exit();

    hooks.use_event_handler(EventScope::Current, EventPriority::Normal, move |event| {
        let Event::Key(key) = event else {
            return EventResult::Ignored;
        };
        if key.kind != KeyEventKind::Press {
            return EventResult::Ignored;
        }

        match key.code {
            KeyCode::Char('t') | KeyCode::Char('T') => {
                let next = (profile_index.get() + 1) % PROFILES.len();
                profile_index.set(next);
                status.set(format!("outer provider switched: {}", PROFILES[next].name));
            }
            KeyCode::Char('d') | KeyCode::Char('D') => {
                let next_density = density.get().toggled();
                density.set(next_density);
                status.set(format!("density changed: {}", next_density.label()));
            }
            KeyCode::Char('q') | KeyCode::Char('Q') => exit(),
            _ => return EventResult::Ignored,
        }

        EventResult::Consumed
    });

    let settings = WorkspaceSettings {
        profile: PROFILES[profile_index.get()],
        density: density.get(),
    };
    let status_view = status.read().clone();

    element!(
        ContextProvider(value: Context::owned(settings)) {
            Center(
                width: Constraint::Length(94),
                height: Constraint::Length(23),
            ) {
                Border(
                    flex_direction: Direction::Vertical,
                    gap: settings.density.gap(),
                    border_style: Style::new().blue(),
                    top_title: Line::from(" custom provider scope ").blue().bold().centered(),
                    bottom_title: Line::from(" t theme | d density | q quit ").dark_gray().centered(),
                ) {
                    ProviderHeader(status: status_view)
                    View(
                        flex_direction: Direction::Horizontal,
                        gap: 2,
                    ) {
                        WorkspaceSummary
                        View(
                            width: Constraint::Fill(1),
                            flex_direction: Direction::Vertical,
                            gap: settings.density.gap(),
                        ) {
                            ScopedAuditProvider
                            ContextProbe
                        }
                    }
                }
            }
        }
    )
}

#[derive(Default, Props)]
struct ProviderHeaderProps {
    status: String,
}

#[component]
fn ProviderHeader(props: &ProviderHeaderProps, hooks: Hooks) -> impl Into<AnyElement<'static>> {
    let (name, mode, accent, density_label) = {
        let settings = hooks.use_context::<WorkspaceSettings>();
        (
            settings.profile.name,
            settings.profile.mode,
            settings.profile.accent,
            settings.density.label(),
        )
    };

    element!(
        Border(
            height: Constraint::Length(5),
            flex_direction: Direction::Vertical,
            justify_content: Flex::Center,
            border_style: Style::new().fg(accent),
            padding: Padding::horizontal(1),
        ) {
            Text(text: Line::from(vec![
                Span::styled(name, Style::new().fg(accent).bold()),
                Span::raw(" / "),
                Span::styled(mode, Style::new().fg(Color::White)),
                Span::raw(" / "),
                Span::styled(density_label, Style::new().fg(Color::DarkGray)),
            ]).centered())
            Text(text: Line::from(props.status.clone()).centered(), style: Style::new().dark_gray(), wrap: true)
        }
    )
}

#[component]
fn WorkspaceSummary(hooks: Hooks) -> impl Into<AnyElement<'static>> {
    let profile = {
        let settings = hooks.use_context::<WorkspaceSettings>();
        settings.profile
    };

    element!(
        Border(
            width: Constraint::Fill(1),
            flex_direction: Direction::Vertical,
            gap: 1,
            border_style: Style::new().fg(profile.accent),
            top_title: Line::from(" outer provider ").fg(profile.accent).bold().centered(),
            padding: Padding::horizontal(1),
        ) {
            SettingRow(label: "workspace", value: profile.name.to_string(), accent: profile.accent)
            SettingRow(label: "region", value: profile.region.to_string(), accent: profile.accent)
            SettingRow(label: "mode", value: profile.mode.to_string(), accent: profile.accent)
            SettingRow(label: "accent", value: profile.accent_name.to_string(), accent: profile.accent)
            SettingRow(label: "reviewers", value: profile.reviewers.to_string(), accent: profile.accent)
            SettingRow(label: "pipelines", value: profile.pipelines.to_string(), accent: profile.accent)
        }
    )
}

#[component]
fn ScopedAuditProvider(hooks: Hooks) -> impl Into<AnyElement<'static>> {
    let density = {
        let settings = hooks.use_context::<WorkspaceSettings>();
        settings.density
    };
    let scoped = WorkspaceSettings {
        profile: AUDIT_PROFILE,
        density,
    };

    element!(
        ContextProvider(value: Context::owned(scoped)) {
            ScopedAuditPanel
        }
    )
}

#[component]
fn ScopedAuditPanel(hooks: Hooks) -> impl Into<AnyElement<'static>> {
    let (profile, density_label) = {
        let settings = hooks.use_context::<WorkspaceSettings>();
        (settings.profile, settings.density.label())
    };

    element!(
        Border(
            height: Constraint::Length(8),
            flex_direction: Direction::Vertical,
            justify_content: Flex::Center,
            border_style: Style::new().fg(profile.accent),
            top_title: Line::from(" nested override ").fg(profile.accent).bold().centered(),
            padding: Padding::horizontal(1),
        ) {
            Text(text: Line::from(format!("workspace: {}", profile.name)).centered())
            Text(text: Line::from(format!("mode:      {}", profile.mode)).centered())
            Text(text: Line::from(format!("density:   {}", density_label)).centered())
            Text(text: Line::from("nearest provider wins").fg(Color::DarkGray).centered())
        }
    )
}

#[component]
fn ContextProbe(hooks: Hooks) -> impl Into<AnyElement<'static>> {
    let (workspace, region, accent) = hooks
        .try_use_context::<WorkspaceSettings>()
        .map(|settings| {
            (
                settings.profile.name.to_string(),
                settings.profile.region.to_string(),
                settings.profile.accent,
            )
        })
        .unwrap_or_else(|| ("<missing>".to_string(), "<missing>".to_string(), Color::Red));

    element!(
        Border(
            height: Constraint::Fill(1),
            flex_direction: Direction::Vertical,
            justify_content: Flex::Center,
            border_style: Style::new().fg(accent),
            top_title: Line::from(" sibling probe ").fg(accent).bold().centered(),
            padding: Padding::horizontal(1),
        ) {
            Text(text: Line::from(format!("sees outer: {}", workspace)).centered())
            Text(text: Line::from(format!("region:     {}", region)).centered())
            Text(text: Line::from("nested override did not leak").fg(Color::DarkGray).centered())
        }
    )
}

#[derive(Props)]
struct SettingRowProps {
    label: &'static str,
    value: String,
    accent: Color,
}

impl Default for SettingRowProps {
    fn default() -> Self {
        Self {
            label: "",
            value: String::new(),
            accent: Color::Reset,
        }
    }
}

#[component]
fn SettingRow(props: &SettingRowProps, _hooks: Hooks) -> impl Into<AnyElement<'static>> {
    element!(
        View(height: Constraint::Length(1)) {
            Text(text: Line::from(vec![
                Span::styled(format!("{:<11}", props.label), Style::new().fg(props.accent)),
                Span::raw(props.value.clone()),
            ]))
        }
    )
}

impl Density {
    fn toggled(self) -> Self {
        match self {
            Density::Comfortable => Density::Compact,
            Density::Compact => Density::Comfortable,
        }
    }

    fn label(self) -> &'static str {
        match self {
            Density::Comfortable => "comfortable",
            Density::Compact => "compact",
        }
    }

    fn gap(self) -> i32 {
        match self {
            Density::Comfortable => 1,
            Density::Compact => 0,
        }
    }
}
