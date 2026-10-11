// Scrollbars 组件：滚动视图的滚动条配置与渲染，支持横向/纵向滚动条、可见性控制、自定义样式，
// 以及 `over_border` 开关(滚动条盖在 block 边框上还是退到框内)。
//
// ## 用法示例
// ```rust
// element!(ScrollView(
//     scrollbars: Scrollbars {
//         vertical_scrollbar_visibility: ScrollbarVisibility::Always,
//         horizontal_scrollbar_visibility: ScrollbarVisibility::Automatic,
//         over_border: false, // 想让滚动条退到边框内侧时设 false(默认 true 盖边框)
//         ..Default::default()
//     },
//     // ...
// ))
// ```
// 可灵活控制滚动条的显示策略和样式，适合长列表、表格、文档等场景。

use ratatui::{
    buffer::Buffer,
    layout::{Position, Rect, Size},
    style::{Modifier, Style},
    symbols::scrollbar::Set,
    text::Span,
    widgets::{Scrollbar, ScrollbarOrientation, ScrollbarState, StatefulWidget},
};
use ratatui_kit_macros::Props;

/// 滚动条外观。符号配置同时用于绘制和命中几何,避免 opaque widget 的配置丢失。
#[derive(Debug, Clone, Hash)]
pub struct ScrollbarStyle<'a> {
    /// 整体样式。
    pub style: Style,
    /// 滑块样式(叠加到整体样式上)。
    pub thumb_style: Style,
    /// 轨道样式(叠加到整体样式上)。
    pub track_style: Style,
    /// 指针悬停时叠加到滑块的样式。
    pub hover_style: Style,
    /// 抓住滑块期间叠加的样式(优先于悬停)。
    pub drag_style: Style,
    /// 自定义符号集;None 按纵横方向使用 ratatui 默认符号。
    pub symbols: Option<Set<'a>>,
    /// 是否显示首尾箭头。
    pub arrows: bool,
}

impl Default for ScrollbarStyle<'_> {
    fn default() -> Self {
        Self {
            style: Style::default(),
            thumb_style: Style::default(),
            track_style: Style::default(),
            hover_style: Style::default().add_modifier(Modifier::BOLD),
            drag_style: Style::default().add_modifier(Modifier::BOLD | Modifier::REVERSED),
            symbols: None,
            arrows: true,
        }
    }
}

impl ScrollbarStyle<'_> {
    pub(crate) fn widget_with_state(
        &self,
        orientation: ScrollbarOrientation,
        hovered: bool,
        dragging: bool,
    ) -> Scrollbar<'_> {
        let feedback = if dragging {
            self.drag_style
        } else if hovered {
            self.hover_style
        } else {
            Style::default()
        };
        let mut scrollbar = Scrollbar::new(orientation)
            .style(self.style)
            .thumb_style(self.style.patch(self.thumb_style).patch(feedback))
            .track_style(self.style.patch(self.track_style));
        if let Some(symbols) = &self.symbols {
            scrollbar = scrollbar.symbols(symbols.clone());
        }
        if !self.arrows {
            scrollbar = scrollbar.begin_symbol(None).end_symbol(None);
        }
        scrollbar
    }

    pub(crate) fn arrow_lengths(&self) -> (u16, u16) {
        if !self.arrows {
            return (0, 0);
        }
        self.symbols.as_ref().map_or((1, 1), |symbols| {
            (
                Span::raw(symbols.begin).width().min(u16::MAX as usize) as u16,
                Span::raw(symbols.end).width().min(u16::MAX as usize) as u16,
            )
        })
    }
}

/// 一条滚动轴的渲染几何,由 [`Scrollbars::render_ref`] 每帧记录。
///
/// ScrollView 和自绘滚动条 hook 共用同一套滑块数学。
/// `usable` 与 `begin_length` 使用绘制时的符号配置,支持自定义符号及无箭头轨道。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct AxisGeometry {
    /// 滑轨整条区域(含箭头符号格)。
    pub track: Rect,
    /// 扣掉箭头格后的可用滑轨长度。
    pub usable: u16,
    /// 首箭头占据的坐标长度。
    pub begin_length: u16,
    /// 视口在该轴上的长度。
    pub viewport: usize,
    /// 可滚动量(content - viewport),最大偏移(含端点),ratatui `content_length` 要加 1。
    pub scrollable: usize,
    /// 当前偏移。
    pub offset: usize,
}

impl AxisGeometry {
    pub(crate) fn new(
        track: Rect,
        viewport: usize,
        scrollable: usize,
        offset: usize,
        vertical: bool,
        arrows: (u16, u16),
    ) -> Option<Self> {
        let full = if vertical { track.height } else { track.width };
        let usable = full.saturating_sub(arrows.0.saturating_add(arrows.1));
        if track.is_empty() || usable == 0 || viewport == 0 || scrollable == 0 {
            return None;
        }

        Some(Self {
            track,
            usable,
            begin_length: arrows.0,
            viewport,
            scrollable,
            offset,
        })
    }

    // 箭头是单步按钮,不应与滑轨点击一样跳到端点或开启拖拽。
    fn arrow_seek(&self, position: Position, vertical: bool) -> Option<usize> {
        let along = if vertical {
            position.y - self.track.y
        } else {
            position.x - self.track.x
        };
        if along < self.begin_length {
            Some(self.offset.saturating_sub(1))
        } else if along >= self.begin_length + self.usable {
            Some(self.offset.saturating_add(1).min(self.scrollable))
        } else {
            None
        }
    }

    fn max_position(&self) -> usize {
        self.scrollable
    }

    /// `max_viewport_position`(ratatui `part_lengths` 同名量):偏移换算滑块位置的分母。
    fn max_viewport_position(&self) -> usize {
        self.max_position().saturating_add(self.viewport)
    }

    /// 滑块长度(ratatui 同式:四舍五入除法,钳到 [1, usable])。
    pub fn thumb_len(&self) -> u16 {
        let raw = (self.viewport as u128 * self.usable as u128
            + self.max_viewport_position() as u128 / 2)
            / self.max_viewport_position().max(1) as u128;
        (raw as u16).clamp(1, self.usable)
    }

    /// 滑块起点(usable 坐标;ratatui 同式,钳到滑轨内)。
    pub fn thumb_start(&self) -> u16 {
        let raw = (self.offset as u128 * self.usable as u128
            + self.max_viewport_position() as u128 / 2)
            / self.max_viewport_position().max(1) as u128;
        (raw as u16).clamp(0, self.usable - self.thumb_len())
    }

    /// 滑块中心(usable 坐标,半格精度)。
    pub fn thumb_center(&self) -> f64 {
        f64::from(self.thumb_start()) + f64::from(self.thumb_len()) / 2.0
    }

    /// 反算:目标滑块中心 → 内容偏移。端点精确(钳到 `[0, max_position]`),
    /// 与 `thumb_start` 使用同一舍入公式,指针驱动下无累积误差。
    pub fn offset_for_thumb_center(&self, center: f64) -> usize {
        let half = f64::from(self.thumb_len()) / 2.0;
        if center <= half {
            return 0;
        }
        if center >= f64::from(self.usable) - half {
            return self.max_position();
        }
        let raw = ((center - f64::from(self.thumb_len()) / 2.0)
            * self.max_viewport_position() as f64
            / f64::from(self.usable))
        .round();
        (raw.max(0.0) as usize).min(self.max_position())
    }

    /// 指针位置在滑轨上的坐标(usable 坐标,钳进 `[0, usable]`;箭头格算作两端)。
    /// 不校验交叉轴 —— 拖拽经指针捕获,指针拖出滑轨列/行时仍要能沿主轴换算。
    pub fn along_clamped(&self, position: Position, vertical: bool) -> f64 {
        let raw = if vertical {
            position.y.saturating_sub(self.track.y)
        } else {
            position.x.saturating_sub(self.track.x)
        };
        f64::from(raw.saturating_sub(self.begin_length).min(self.usable))
    }
}

/// 一次渲染后记录的两轴滚动条几何;`None` = 该轴滚动条未显示。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct ScrollbarGeometry {
    /// 纵向滚动条几何。
    pub vertical: Option<AxisGeometry>,
    /// 横向滚动条几何。
    pub horizontal: Option<AxisGeometry>,
}

impl ScrollbarGeometry {
    /// 命中测试:指针落在哪条滑轨上,返回该轴几何与指针的 usable 坐标。
    pub fn hit(&self, position: Position) -> Option<(&AxisGeometry, f64, bool)> {
        if let Some(axis) = &self.vertical
            && axis.track.contains(position)
        {
            return Some((axis, axis.along_clamped(position, true), true));
        }
        if let Some(axis) = &self.horizontal
            && axis.track.contains(position)
        {
            return Some((axis, axis.along_clamped(position, false), false));
        }
        None
    }
}

/// 渲染几何与抓点共用一个状态机,由视图或自绘 hook 写回各自的滚动状态。
#[derive(Default)]
pub(crate) struct ScrollbarInteraction {
    pub(crate) geometry: ScrollbarGeometry,
    drag: Option<DragAxis>,
    hovered: Option<bool>,
}

#[derive(Clone, Copy)]
struct DragAxis {
    vertical: bool,
    grab: f64,
}

impl ScrollbarInteraction {
    pub(crate) fn is_hovered(&self, vertical: bool) -> bool {
        self.hovered == Some(vertical)
    }
    pub(crate) fn is_dragging(&self, vertical: bool) -> bool {
        self.drag.is_some_and(|drag| drag.vertical == vertical)
    }
    pub(crate) fn handle(
        &mut self,
        event: crate::PointerEvent,
        mut seek: impl FnMut(usize, bool),
    ) -> crate::EventResult {
        use crate::{EventResult, PointerKind};
        use crossterm::event::MouseButton;
        match event.kind {
            PointerKind::Enter | PointerKind::Move | PointerKind::Leave => {
                let next = if matches!(event.kind, PointerKind::Leave) {
                    None
                } else {
                    self.geometry
                        .hit(event.position)
                        .map(|(_, _, vertical)| vertical)
                };
                if self.hovered != next {
                    self.hovered = next;
                    EventResult::Consumed
                } else {
                    EventResult::Ignored
                }
            }
            PointerKind::Down(MouseButton::Left) => {
                let Some((axis, along, vertical)) = self.geometry.hit(event.position) else {
                    return EventResult::Ignored;
                };
                let axis = *axis;
                if let Some(target) = axis.arrow_seek(event.position, vertical) {
                    seek(target, vertical);
                    return EventResult::Consumed;
                }
                let start = f64::from(axis.thumb_start());
                let on_thumb = along >= start && along < start + f64::from(axis.thumb_len());
                let grab = if on_thumb {
                    along - axis.thumb_center()
                } else {
                    0.0
                };
                self.hovered = Some(vertical);
                self.drag = Some(DragAxis { vertical, grab });
                // 抓住滑块不反算,否则像素量化会让原偏移在 Down 时跳动。
                if !on_thumb {
                    seek(axis.offset_for_thumb_center(along), vertical);
                }
                EventResult::Consumed
            }
            PointerKind::Drag(MouseButton::Left) => {
                self.hovered = self
                    .geometry
                    .hit(event.position)
                    .map(|(_, _, vertical)| vertical);
                let Some(drag) = self.drag else {
                    return EventResult::Ignored;
                };
                let axis = if drag.vertical {
                    self.geometry.vertical
                } else {
                    self.geometry.horizontal
                };
                if let Some(axis) = axis {
                    let along = axis.along_clamped(event.position, drag.vertical);
                    seek(
                        axis.offset_for_thumb_center(along - drag.grab),
                        drag.vertical,
                    );
                }
                EventResult::Consumed
            }
            PointerKind::Cancel(MouseButton::Left) => {
                self.drag = None;
                self.hovered = None;
                EventResult::Ignored
            }
            PointerKind::Up(MouseButton::Left) => {
                self.hovered = self
                    .geometry
                    .hit(event.position)
                    .map(|(_, _, vertical)| vertical);
                if self.drag.take().is_some() {
                    EventResult::Consumed
                } else {
                    EventResult::Ignored
                }
            }
            _ => EventResult::Ignored,
        }
    }
}

#[derive(Debug, Default, Clone, Copy, Eq, PartialEq, Hash)]
// 滚动条可见性枚举。
pub enum ScrollbarVisibility {
    // 仅在需要时渲染滚动条。
    #[default]
    Automatic,
    // 始终渲染滚动条。
    Always,
    // 从不渲染滚动条（隐藏）。
    Never,
}

#[derive(Props, Clone, Hash)]
// 滚动条配置。
pub struct Scrollbars<'a> {
    // 纵向滚动条可见性。
    pub vertical_scrollbar_visibility: ScrollbarVisibility,
    // 横向滚动条可见性。
    pub horizontal_scrollbar_visibility: ScrollbarVisibility,
    // 纵向滚动条样式。
    pub vertical_scrollbar: ScrollbarStyle<'a>,
    // 横向滚动条样式。
    pub horizontal_scrollbar: ScrollbarStyle<'a>,
    /// 有 block 边框时,滚动条画在边框环上(不占内容区)还是退到 `block.inner()` 内
    /// (占用一行/一列)。默认 `true`(盖边框,观感更好);无 block 时退化为框内。
    pub over_border: bool,
}

impl Default for Scrollbars<'_> {
    fn default() -> Self {
        Self {
            vertical_scrollbar_visibility: ScrollbarVisibility::Automatic,
            horizontal_scrollbar_visibility: ScrollbarVisibility::Automatic,
            vertical_scrollbar: ScrollbarStyle::default(),
            horizontal_scrollbar: ScrollbarStyle::default(),
            over_border: true,
        }
    }
}

/// 布局、裁剪、绘制与命中共同使用的本帧解析结果。
#[derive(Debug, Clone, Copy)]
pub(crate) struct ResolvedLayout {
    pub content: Size,
    pub offset: Position,
    pub viewport: Rect,
    pub vertical_track: Option<Rect>,
    pub horizontal_track: Option<Rect>,
}

impl Scrollbars<'_> {
    // 单轴可见性:无「显示一条会挤占另一轴」的耦合时用(over_border 模式,滚动条在边框上不占内容)。
    fn axis_show(visibility: ScrollbarVisibility, space: i32) -> bool {
        match visibility {
            ScrollbarVisibility::Always => true,
            ScrollbarVisibility::Never => false,
            ScrollbarVisibility::Automatic => space < 0,
        }
    }

    /// 候选视口决定交叉轴尺寸,主轴尺寸由调用者固定在原 inner 上。
    /// 只接受显隐自洽的候选,后续绘制直接使用该结果。
    pub(crate) fn resolve_layout(
        &self,
        outer: Rect,
        inner: Rect,
        border_ring: bool,
        mut content_size: impl FnMut(Size) -> Size,
    ) -> ResolvedLayout {
        let ring = self.over_border && border_ring;
        let resolve = |horizontal: bool, vertical: bool| Rect {
            width: inner.width.saturating_sub((vertical && !ring) as u16),
            height: inner.height.saturating_sub((horizontal && !ring) as u16),
            ..inner
        };
        let build = |content: Size, viewport: Rect, horizontal: bool, vertical: bool| {
            let track = |area: Rect| {
                let area = area.intersection(outer);
                (!area.is_empty()).then_some(area)
            };
            ResolvedLayout {
                content,
                offset: Position::ORIGIN,
                viewport,
                vertical_track: vertical
                    .then(|| {
                        track(Rect::new(
                            if ring {
                                outer.right().saturating_sub(1)
                            } else {
                                viewport.right()
                            },
                            viewport.y,
                            1,
                            viewport.height,
                        ))
                    })
                    .flatten(),
                horizontal_track: horizontal
                    .then(|| {
                        track(Rect::new(
                            viewport.x,
                            if ring {
                                outer.bottom().saturating_sub(1)
                            } else {
                                viewport.bottom()
                            },
                            viewport.width,
                            1,
                        ))
                    })
                    .flatten(),
            }
        };
        for (horizontal, vertical) in [(false, false), (false, true), (true, false), (true, true)] {
            let viewport = resolve(horizontal, vertical);
            let content = content_size(viewport.as_size());
            let expected = (
                Self::axis_show(
                    self.horizontal_scrollbar_visibility,
                    i32::from(viewport.width) - i32::from(content.width),
                ),
                Self::axis_show(
                    self.vertical_scrollbar_visibility,
                    i32::from(viewport.height) - i32::from(content.height),
                ),
            );
            if expected == (horizontal, vertical) {
                return build(content, viewport, horizontal, vertical);
            }
        }
        // 极小视口可能在弹性交叉轴与两条轨道间形成循环;保守预留所有允许的轨道,
        // 空轨道由 build 裁掉,避免越界写入或未预留的轨道覆盖正文。
        let horizontal = self.horizontal_scrollbar_visibility != ScrollbarVisibility::Never;
        let vertical = self.vertical_scrollbar_visibility != ScrollbarVisibility::Never;
        let viewport = resolve(horizontal, vertical);
        build(
            content_size(viewport.as_size()),
            viewport,
            horizontal,
            vertical,
        )
    }

    fn render_visible_area(&self, dst: Rect, buf: &mut Buffer, src: Rect, scroll_buffer: &Buffer) {
        for (src_row, dst_row) in src.rows().zip(dst.rows()) {
            for (src_col, dst_col) in src_row.columns().zip(dst_row.columns()) {
                buf[dst_col] = scroll_buffer[src_col].clone();
            }
        }
    }

    // 朝向内部固定,避免调用方设错朝向导致布局/渲染不一致(只放开符号/样式覆盖)。
    fn render_scrollbar(
        scrollbar: Scrollbar<'_>,
        area: Rect,
        buf: &mut Buffer,
        position: u16,
        content_len: u16,
        viewport_len: u16,
    ) {
        let hidden = content_len.saturating_sub(viewport_len);
        let mut scrollbar_state = ScrollbarState::new(hidden as usize + 1)
            .position(position as usize)
            .viewport_content_length(viewport_len as usize);
        scrollbar.render(area, buf, &mut scrollbar_state);
    }

    /// 给定未扣轨道前的空余尺寸,返回固定内容所需的两轴滚动条。
    pub fn visible_scrollbars(&self, horizontal_space: i32, vertical_space: i32) -> (bool, bool) {
        let mut shown = (
            Self::axis_show(self.horizontal_scrollbar_visibility, horizontal_space),
            Self::axis_show(self.vertical_scrollbar_visibility, vertical_space),
        );
        loop {
            let next = (
                Self::axis_show(
                    self.horizontal_scrollbar_visibility,
                    horizontal_space.saturating_sub(i32::from(shown.1)),
                ),
                Self::axis_show(
                    self.vertical_scrollbar_visibility,
                    vertical_space.saturating_sub(i32::from(shown.0)),
                ),
            );
            if next == shown {
                return shown;
            }
            shown = next;
        }
    }

    /// 按本帧已解析的视口/轨道绘制,不再重新判断显隐或裁剪偏移。
    pub(crate) fn render_ref(
        &self,
        layout: ResolvedLayout,
        buf: &mut Buffer,
        scroll_buffer: &Buffer,
        interaction: &ScrollbarInteraction,
    ) -> ScrollbarGeometry {
        let content = layout.content;
        let viewport = layout.viewport.as_size();
        let x = layout.offset.x;
        let y = layout.offset.y;
        // 把可见窗口 blit 到 inner 左上角(严格限制在 inner 内,无滚动条方向的边框得以保留)。
        let src = Rect::new(x, y, viewport.width, viewport.height).intersection(scroll_buffer.area);
        let dst = layout.viewport;
        self.render_visible_area(dst, buf, src, scroll_buffer);

        // 滚动条位置:ring 时 viewport = inner,故 `inner.x + viewport.width == inner.right()`(边框环上);
        // inset 时即 inner 内最后一列/行。两种情形同一表达式,无需分支。
        let mut geometry = ScrollbarGeometry::default();
        if let Some(area) = layout.vertical_track {
            Self::render_scrollbar(
                self.vertical_scrollbar.widget_with_state(
                    ScrollbarOrientation::VerticalRight,
                    interaction.is_hovered(true),
                    interaction.is_dragging(true),
                ),
                area,
                buf,
                y,
                content.height,
                viewport.height,
            );
            geometry.vertical = AxisGeometry::new(
                area,
                viewport.height as usize,
                content.height.saturating_sub(viewport.height) as usize,
                y as usize,
                true,
                self.vertical_scrollbar.arrow_lengths(),
            );
        }
        if let Some(area) = layout.horizontal_track {
            Self::render_scrollbar(
                self.horizontal_scrollbar.widget_with_state(
                    ScrollbarOrientation::HorizontalBottom,
                    interaction.is_hovered(false),
                    interaction.is_dragging(false),
                ),
                area,
                buf,
                x,
                content.width,
                viewport.width,
            );
            geometry.horizontal = AxisGeometry::new(
                area,
                viewport.width as usize,
                content.width.saturating_sub(viewport.width) as usize,
                x as usize,
                false,
                self.horizontal_scrollbar.arrow_lengths(),
            );
        }
        geometry
    }
}

#[cfg(test)]
mod interaction_tests {
    use super::*;
    use crate::{PointerEvent, PointerKind};
    use crossterm::event::{KeyModifiers, MouseButton};

    #[test]
    fn cancellation_clears_drag_feedback_without_seeking() {
        let track = Rect::new(2, 3, 1, 12);
        let mut interaction = ScrollbarInteraction::default();
        interaction.geometry.vertical = AxisGeometry::new(track, 10, 100, 0, true, (1, 1));
        let event = |kind| PointerEvent {
            kind,
            position: Position::new(2, 4),
            screen_position: Position::new(2, 4),
            modifiers: KeyModifiers::NONE,
            area: track,
        };
        interaction.handle(event(PointerKind::Down(MouseButton::Left)), |_, _| {
            panic!("thumb grab must not seek")
        });
        assert!(interaction.is_dragging(true));
        interaction.handle(event(PointerKind::Cancel(MouseButton::Left)), |_, _| {
            panic!("cancel must not seek")
        });
        assert!(!interaction.is_dragging(true));
        assert!(!interaction.is_hovered(true));
    }

    #[test]
    fn arrow_clicks_step_without_starting_a_drag() {
        for vertical in [true, false] {
            let track = if vertical {
                Rect::new(2, 3, 1, 12)
            } else {
                Rect::new(2, 3, 12, 1)
            };
            let axis = AxisGeometry::new(track, 10, 100, 50, vertical, (1, 1)).unwrap();
            let mut interaction = ScrollbarInteraction::default();
            if vertical {
                interaction.geometry.vertical = Some(axis);
            } else {
                interaction.geometry.horizontal = Some(axis);
            }
            for (position, expected) in [
                (Position::new(2, 3), 49),
                (
                    if vertical {
                        Position::new(2, 14)
                    } else {
                        Position::new(13, 3)
                    },
                    51,
                ),
            ] {
                let mut target = None;
                interaction.handle(
                    PointerEvent {
                        kind: PointerKind::Down(MouseButton::Left),
                        position,
                        screen_position: position,
                        modifiers: KeyModifiers::NONE,
                        area: track,
                    },
                    |offset, _| target = Some(offset),
                );
                assert_eq!(target, Some(expected));
                assert!(!interaction.is_dragging(vertical));
            }
        }
    }
}

#[cfg(test)]
mod layout_tests {
    use super::*;
    use ratatui::widgets::{Block, Borders, Padding};

    #[test]
    fn resolved_layout_covers_ring_inset_and_partial_borders() {
        let outer = Rect::new(4, 7, 20, 10);
        let content = Size::new(30, 20);
        for block in [
            Block::bordered(),
            Block::new().borders(Borders::RIGHT),
            Block::bordered().padding(Padding::uniform(1)),
            Block::new()
                .borders(Borders::RIGHT)
                .padding(Padding::uniform(1))
                .title_bottom("status"),
        ] {
            let inner = block.inner(outer);
            for over_border in [true, false] {
                let bars = Scrollbars {
                    over_border,
                    ..Default::default()
                };
                let border_ring = super::super::has_border_ring(&block);
                let layout = bars.resolve_layout(outer, inner, border_ring, |_| content);
                let ring = bars.over_border && border_ring;
                assert_eq!(layout.viewport.width, inner.width - u16::from(!ring));
                assert_eq!(layout.viewport.height, inner.height - u16::from(!ring));
                let vertical = layout.vertical_track.unwrap();
                let horizontal = layout.horizontal_track.unwrap();
                assert_eq!(
                    vertical,
                    Rect::new(
                        if ring {
                            outer.right() - 1
                        } else {
                            layout.viewport.right()
                        },
                        inner.y,
                        1,
                        layout.viewport.height
                    )
                );
                assert_eq!(
                    horizontal,
                    Rect::new(
                        inner.x,
                        if ring {
                            outer.bottom() - 1
                        } else {
                            layout.viewport.bottom()
                        },
                        layout.viewport.width,
                        1
                    )
                );
                assert_eq!(vertical.intersection(outer), vertical);
                assert_eq!(horizontal.intersection(outer), horizontal);
            }
        }
    }

    #[test]
    fn padding_and_titles_do_not_impersonate_missing_border_edges() {
        for bits in 0..16 {
            let borders = Borders::from_bits_truncate(bits);
            let block = Block::new()
                .borders(borders)
                .padding(Padding::uniform(2))
                .title_bottom("status");
            assert_eq!(
                super::super::has_border_ring(&block),
                borders.contains(Borders::RIGHT | Borders::BOTTOM)
            );
        }
    }

    #[test]
    fn fixed_content_visibility_is_resolved_once_for_all_policies() {
        use ScrollbarVisibility::{Always, Automatic, Never};
        let outer = Rect::new(0, 0, 10, 5);
        for horizontal in [Always, Automatic, Never] {
            for vertical in [Always, Automatic, Never] {
                let bars = Scrollbars {
                    horizontal_scrollbar_visibility: horizontal,
                    vertical_scrollbar_visibility: vertical,
                    ..Default::default()
                };
                for content in [Size::new(8, 3), Size::new(10, 5), Size::new(12, 8)] {
                    let layout = bars.resolve_layout(outer, outer, false, |_| content);
                    let expected = bars.visible_scrollbars(
                        10 - i32::from(content.width),
                        5 - i32::from(content.height),
                    );
                    assert_eq!(layout.horizontal_track.is_some(), expected.0);
                    assert_eq!(layout.vertical_track.is_some(), expected.1);
                    assert_eq!(
                        layout.viewport.as_size(),
                        Size::new(10 - u16::from(expected.1), 5 - u16::from(expected.0))
                    );
                    assert_eq!(layout.content, content);
                }
            }
        }
    }

    #[test]
    fn tiny_viewports_clip_tracks_and_render_without_out_of_bounds_cells() {
        for width in 0..=2 {
            for height in 0..=2 {
                let area = Rect::new(0, 0, width, height);
                for visibility in [
                    ScrollbarVisibility::Automatic,
                    ScrollbarVisibility::Always,
                    ScrollbarVisibility::Never,
                ] {
                    let bars = Scrollbars {
                        horizontal_scrollbar_visibility: visibility,
                        vertical_scrollbar_visibility: visibility,
                        ..Default::default()
                    };
                    let layout = bars.resolve_layout(area, area, false, |page| {
                        Size::new(page.width.saturating_mul(2), 3)
                    });
                    let content =
                        Buffer::empty(Rect::new(0, 0, layout.content.width, layout.content.height));
                    let mut destination = Buffer::empty(area);
                    bars.render_ref(
                        layout,
                        &mut destination,
                        &content,
                        &ScrollbarInteraction::default(),
                    );
                    assert_eq!(destination.area, area);
                    for track in [layout.horizontal_track, layout.vertical_track]
                        .into_iter()
                        .flatten()
                    {
                        assert_eq!(track.intersection(area), track);
                    }
                }
            }
        }
    }
}
