use std::sync::Arc;

use ratatui_kit::{
    crossterm::event::{Event, KeyCode, KeyEventKind},
    prelude::*,
    ratatui::{
        layout::{Constraint, Direction},
        style::{Color, Modifier, Style, Stylize},
        text::Line,
        widgets::Block,
    },
};

#[derive(Clone)]
struct Deployment {
    service: &'static str,
    owner: &'static str,
    env: &'static str,
    version: &'static str,
    status: Status,
    latency: u64,
    note: &'static str,
}

#[derive(Clone, Copy)]
enum Status {
    Healthy,
    Warming,
    Failed,
    Disabled,
}

const DEPLOYMENTS: [Deployment; 10] = [
    Deployment {
        service: "gateway-入口服务",
        owner: "平台组 / Platform",
        env: "prod",
        version: "v2.8.1",
        status: Status::Healthy,
        latency: 24,
        note: "中英文 mixed content; normal width",
    },
    Deployment {
        service: "api-core-核心 API",
        owner: "后端 Backend",
        env: "prod-cn-hangzhou",
        version: "v4.12.0+build.20260703.sha.abcdef1234567890",
        status: Status::Healthy,
        latency: 37,
        note: "超长版本号会被列宽裁剪，验证单行截断表现",
    },
    Deployment {
        service: "worker-异步任务处理器-with-a-very-very-long-service-name",
        owner: "数据平台 Data Platform",
        env: "prod",
        version: "v1.9.7",
        status: Status::Warming,
        latency: 142,
        note: "service 列极长：中文 + English + hyphenated text",
    },
    Deployment {
        service: "billing-计费",
        owner: "财务系统组",
        env: "staging",
        version: "v0.18.4",
        status: Status::Failed,
        latency: 999,
        note: "FAILED row 使用红色状态 + 延迟高亮",
    },
    Deployment {
        service: "search 🔍 搜索",
        owner: "搜索组 Search",
        env: "preview",
        version: "v3.1.5",
        status: Status::Healthy,
        latency: 58,
        note: "emoji + CJK width smoke test",
    },
    Deployment {
        service: "sync-同步",
        owner: "客户端 Client",
        env: "dev",
        version: "v0.7.2",
        status: Status::Warming,
        latency: 220,
        note: "黄色表示 warming，latency 超过 120ms 也会自定义样式",
    },
    Deployment {
        service: "metrics-指标",
        owner: "SRE / 可观测性",
        env: "prod-eu-west-1",
        version: "v2.3.0",
        status: Status::Healthy,
        latency: 18,
        note: "region 名称较长，Env 列固定宽度",
    },
    Deployment {
        service: "auth-认证授权服务",
        owner: "安全 Security",
        env: "prod",
        version: "v5.0.1",
        status: Status::Healthy,
        latency: 31,
        note: "自定义 owner/service/status/latency 每列样式",
    },
    Deployment {
        service: "实验性灰度服务 experimental-canary-service-name-overflow",
        owner: "增长 Growth / 增长实验",
        env: "canary-1%",
        version: "v2026.07.03-canary-super-long-build-metadata",
        status: Status::Disabled,
        latency: 0,
        note: "disabled 行使用暗色；同时包含多个超长字段",
    },
    Deployment {
        service: "报告导出-ReportExporter",
        owner: "BI 团队 / Business Intelligence",
        env: "prod-ap-southeast-1-blue-green-slot-a",
        version: "v12.0.0-rc.1",
        status: Status::Failed,
        latency: 1_240,
        note: "极端长 env + 高延迟，用来观察列裁剪与高亮",
    },
];

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
    let mut compact = hooks.use_state(|| false);
    let mut exit = hooks.use_exit();

    hooks.use_event_handler(EventScope::Current, EventPriority::Normal, move |event| {
        if let Event::Key(key) = event
            && key.kind == KeyEventKind::Press
        {
            match key.code {
                KeyCode::Char('c') => {
                    compact.set(!compact.get());
                    return EventResult::Consumed;
                }
                KeyCode::Char('e') => {
                    empty.set(!empty.get());
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

    let rows = if empty.get() {
        Vec::new()
    } else {
        DEPLOYMENTS.to_vec()
    };
    let (terminal_width, _) = hooks.use_terminal_size();
    let available_width = terminal_width.saturating_sub(6);
    let table_max_width = if compact.get() { 72 } else { 94 };
    let table_min_width = if compact.get() { 48 } else { 56 };
    let table_width = available_width
        .saturating_sub(1)
        .clamp(table_min_width, table_max_width);
    let scroll_view_width = table_width.saturating_add(1);
    let frame_width = scroll_view_width
        .saturating_add(2)
        .min(terminal_width.max(1));
    let mode = if empty.get() { "empty" } else { "stress" };
    let density = if compact.get() { "compact" } else { "wide" };
    let columns = if compact.get() {
        compact_columns()
    } else {
        wide_columns()
    };
    let render_deployment_row: RenderTableRow<Deployment> =
        Arc::new(|deployment: &Deployment, _| {
            let status_style = deployment.status.style();
            let service_style = Style::new().fg(Color::White).add_modifier(Modifier::BOLD);
            let owner_style = Style::new().fg(Color::Rgb(180, 180, 220));
            let latency_style = match deployment.latency {
                0 => Style::new().fg(Color::DarkGray),
                1..=80 => Style::new().fg(Color::Green),
                81..=200 => Style::new().fg(Color::Yellow),
                _ => Style::new().fg(Color::Red).add_modifier(Modifier::BOLD),
            };
            let note_style = if matches!(deployment.status, Status::Disabled) {
                Style::new().fg(Color::DarkGray)
            } else {
                Style::new().fg(Color::Gray)
            };

            vec![
                TableCell::new(deployment.service).style(service_style),
                TableCell::new(deployment.owner).style(owner_style),
                TableCell::new(deployment.env).style(Style::new().fg(Color::Cyan)),
                TableCell::new(deployment.version),
                TableCell::new(deployment.status.label()).style(status_style),
                TableCell::new(format!("{}ms", deployment.latency))
                    .style(latency_style)
                    .alignment(TableCellAlignment::Right),
                TableCell::new(deployment.note).style(note_style),
            ]
        });

    element!(
        Center(width: Constraint::Length(frame_width), height: Constraint::Length(24)) {
            Border(
                flex_direction: Direction::Vertical,
                border_style: Style::new().fg(Color::Blue),
                top_title: Line::from(" table stress example ").fg(Color::Blue).bold().centered(),
                bottom_title: Line::from(" mouse wheel/PageUp/PageDown/j/k scroll | c compact | e empty | q quit ").dark_gray().centered(),
            ) {
                View(flex_direction: Direction::Vertical, gap: 1) {
                    ScrollView(
                        width: Constraint::Length(scroll_view_width),
                        height: Constraint::Length(16),
                        flex_direction: Direction::Vertical,
                        scroll_bars: ScrollBars {
                            vertical_scrollbar_visibility: ScrollbarVisibility::Always,
                            horizontal_scrollbar_visibility: ScrollbarVisibility::Automatic,
                            ..Default::default()
                        },
                    ) {
                        Table<Deployment>(
                            width: Constraint::Length(table_width),
                            active: false,
                            rows: rows,
                            columns: columns,
                            block: Block::bordered()
                                .border_style(Style::new().fg(Color::Cyan))
                                .title_top(Line::from(" 中英文 mixed / very long fields / custom styles ").centered())
                                .title_bottom(Line::from(format!(" density: {density} · ScrollView clips overflowing table ")).centered()),
                            header_style: Style::new().fg(Color::Cyan).add_modifier(Modifier::BOLD),
                            row_style: Style::new().fg(Color::White),
                            highlight_style: Style::new().fg(Color::White),
                            highlight_symbol: None,
                            column_spacing: if compact.get() { 1u16 } else { 2u16 },
                            wrap_mode: if compact.get() { TableWrapMode::Truncate } else { TableWrapMode::Wrap },
                            border_mode: if compact.get() { TableBorderMode::Outer } else { TableBorderMode::Grid },
                            border_style: Style::new().fg(Color::DarkGray),
                            horizontal_line_style: Style::new().fg(Color::DarkGray),
                            row_separator: true,
                            cell_padding: 1u16,
                            render_row: Some(render_deployment_row),
                        )
                    }
                    Border(
                        height: Constraint::Length(6),
                        flex_direction: Direction::Vertical,
                        border_style: Style::new().fg(Color::Cyan),
                        top_title: Line::from(" what to inspect ").fg(Color::Cyan).centered(),
                    ) {
                        View(height: Constraint::Length(1)) {
                            Text(text: Line::from(format!(
                                "mode: {mode} / {density} · width: term {terminal_width} / table {table_width}"
                            )).centered())
                        }
                        View(height: Constraint::Length(1)) {
                            Text(text: Line::from("CJK mixed · long fields · wide wraps / compact truncates · ScrollView clips overflow").centered())
                        }
                        View(height: Constraint::Length(1)) {
                            Text(text: Line::from("Body row separators use the same border color").centered())
                        }
                        View(height: Constraint::Length(1)) {
                            Text(text: Line::from("This example disables Table selection so ScrollView owns j/k and arrow keys").centered())
                        }
                    }
                }
            }
        }
    )
}

fn wide_columns() -> Vec<TableColumn> {
    vec![
        TableColumn::new("Service / 服务", Constraint::Length(22)),
        TableColumn::new("Owner / 负责人", Constraint::Length(18)),
        TableColumn::new("Env", Constraint::Length(14)),
        TableColumn::new("Version", Constraint::Length(18)).min_table_width(86),
        TableColumn::new("Status", Constraint::Length(10)).alignment(TableCellAlignment::Center),
        TableColumn::new("Latency", Constraint::Length(9)).alignment(TableCellAlignment::Right),
        TableColumn::new("Note / 备注", Constraint::Fill(1)).min_table_width(100),
    ]
}

fn compact_columns() -> Vec<TableColumn> {
    vec![
        TableColumn::new("Service / 服务", Constraint::Length(16)),
        TableColumn::new("Owner", Constraint::Length(10)),
        TableColumn::new("Env", Constraint::Length(8)),
        TableColumn::new("Version", Constraint::Length(10)).min_table_width(86),
        TableColumn::new("Status", Constraint::Length(8)).alignment(TableCellAlignment::Center),
        TableColumn::new("Latency", Constraint::Length(7)).alignment(TableCellAlignment::Right),
        TableColumn::new("Note", Constraint::Fill(1)).min_table_width(100),
    ]
}

impl Status {
    fn label(self) -> &'static str {
        match self {
            Status::Healthy => "healthy 正常",
            Status::Warming => "warming 预热",
            Status::Failed => "failed 失败",
            Status::Disabled => "disabled 停用",
        }
    }

    fn style(self) -> Style {
        match self {
            Status::Healthy => Style::new().fg(Color::Green).add_modifier(Modifier::BOLD),
            Status::Warming => Style::new().fg(Color::Yellow),
            Status::Failed => Style::new().fg(Color::Red).add_modifier(Modifier::BOLD),
            Status::Disabled => Style::new().fg(Color::DarkGray),
        }
    }
}
