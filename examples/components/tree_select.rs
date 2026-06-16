//! TreeSelect 内置组件示例。

use ratatui_kit::{
    components::tui_tree_widget::{TreeItem, TreeState},
    crossterm::event::{Event, KeyCode, KeyEventKind},
    prelude::*,
    ratatui::{
        layout::{Constraint, Direction, Flex},
        style::{Color, Style, Stylize},
        text::Line,
        widgets::Block,
    },
};

#[tokio::main]
async fn main() {
    element!(App)
        .fullscreen()
        .await
        .expect("Failed to run the application");
}

#[component]
fn App(mut hooks: Hooks) -> impl Into<AnyElement<'static>> {
    let mut empty = hooks.use_state(|| false);
    let tree_state = hooks.use_state(TreeState::<&'static str>::default);
    let mut submitted = hooks.use_state(|| "not submitted".to_string());
    let mut exit = hooks.use_exit();

    hooks.use_event_handler(EventScope::Current, EventPriority::Normal, move |event| {
        if let Event::Key(key) = event
            && key.kind == KeyEventKind::Press
        {
            match key.code {
                KeyCode::Char('e') => {
                    let was_empty = empty.get();
                    empty.set(!was_empty);
                    submitted.set(if was_empty {
                        "tree restored".to_string()
                    } else {
                        "empty tree".to_string()
                    });
                    return EventResult::Consumed;
                }
                KeyCode::Char('q') => {
                    exit();
                    return EventResult::Consumed;
                }
                _ => {}
            }
        }

        EventResult::Ignored
    });

    let items = if empty.get() {
        Vec::new()
    } else {
        demo_items()
    };
    let default_selection = if empty.get() {
        Vec::new()
    } else {
        vec!["components", "select"]
    };
    let mode = if empty.get() { "empty" } else { "ready" };
    let tree_hint = if empty.get() {
        " empty tree "
    } else {
        " default: components/select "
    };
    let path = path_label(tree_state.read().selected());
    let submitted_view = submitted.read().to_string();

    element!(
        Center(
            width: Constraint::Length(90),
            height: Constraint::Length(20),
        ) {
            Border(
                flex_direction: Direction::Vertical,
                border_style: Style::new().fg(Color::Blue),
                top_title: Line::from(" tree select ").fg(Color::Blue).bold().centered(),
                bottom_title: Line::from(" h/l fold | j/k move | Space toggle | Enter select | e empty | q quit ").dark_gray().centered(),
            ) {
                View(
                    flex_direction: Direction::Horizontal,
                    gap: 2,
                ) {
                    TreeSelect<&'static str>(
                        width: Constraint::Length(40),
                        state: tree_state,
                        active: true,
                        items: items,
                        default_selection: default_selection,
                        block: Block::bordered()
                            .border_style(Style::new().fg(Color::Cyan))
                            .title_top(Line::from(" component map ").centered())
                            .title_bottom(Line::from(tree_hint).centered()),
                        highlight_symbol: "> ",
                        node_closed_symbol: "+ ",
                        node_open_symbol: "- ",
                        node_no_children_symbol: "  ",
                        style: Style::new().fg(Color::White),
                        highlight_style: Style::new().fg(Color::Black).bg(Color::Green),
                        on_select: move |id: &'static str| {
                            submitted.set(format!("selected {id}"));
                        },
                    )
                    Border(
                        width: Constraint::Fill(1),
                        flex_direction: Direction::Vertical,
                        justify_content: Flex::Center,
                        border_style: Style::new().fg(Color::Cyan),
                        top_title: Line::from(" state ").fg(Color::Cyan).centered(),
                    ) {
                        View(height: Constraint::Length(1)) {
                            Text(text: Line::from(format!("mode: {mode}")).centered())
                        }
                        View(height: Constraint::Length(1)) {
                            Text(text: Line::from(format!("path: {path}")).centered())
                        }
                        View(height: Constraint::Length(1)) {
                            Text(text: Line::from(format!("submit: {submitted_view}")).centered())
                        }
                    }
                }
            }
        }
    )
}

fn demo_items() -> Vec<TreeItem<'static, &'static str>> {
    vec![
        branch(
            "components",
            "Components",
            vec![
                TreeItem::new_leaf("search-input", "SearchInput"),
                TreeItem::new_leaf("select", "Select"),
                TreeItem::new_leaf("multi-select", "MultiSelect"),
                TreeItem::new_leaf("tree-select", "TreeSelect"),
            ],
        ),
        branch(
            "overlays",
            "Overlays",
            vec![
                TreeItem::new_leaf("alert-modal", "AlertModal"),
                TreeItem::new_leaf("confirm-modal", "ConfirmModal"),
                TreeItem::new_leaf("shortcut-info", "ShortcutInfoModal"),
            ],
        ),
        branch(
            "runtime",
            "Runtime",
            vec![
                TreeItem::new_leaf("input-layers", "Input layers"),
                TreeItem::new_leaf("router", "Router"),
                TreeItem::new_leaf("atoms", "Atoms"),
            ],
        ),
    ]
}

fn branch(
    id: &'static str,
    label: &'static str,
    children: Vec<TreeItem<'static, &'static str>>,
) -> TreeItem<'static, &'static str> {
    TreeItem::new(id, label, children).expect("demo tree item identifiers are unique")
}

fn path_label(path: &[&str]) -> String {
    if path.is_empty() {
        "<none>".to_string()
    } else {
        path.join("/")
    }
}
