use crate::registry::Registry;
use crate::runtime::{BorderConfig, HypertileRuntime, mouse::MouseResizeHover};
use ratatui::{
    buffer::Buffer,
    layout::{Direction, Rect},
    style::{Color, Style},
    widgets::{Block, Borders, Clear, List, ListItem, ListState, StatefulWidget, Widget},
};
use ratatui_hypertile::PaneId;
use std::time::Instant;

impl HypertileRuntime {
    /// Renders panes and the palette overlay if it is open.
    ///
    /// Call [`next_frame_in`](super::HypertileRuntime::next_frame_in) after drawing if
    /// you want move animations to keep updating.
    pub fn render(&mut self, area: Rect, buf: &mut Buffer) {
        let now = Instant::now();
        let previous_area = self.animation_state.last_area();
        self.animation_state.remember_area(area);
        if previous_area.is_some() && previous_area != Some(area) {
            self.mouse_drag.clear();
            self.mouse_resize_hover = None;
        }
        self.core.compute_layout(area);
        let focused = self.core.focused_pane();
        let highlight = self.core.state().focus_highlight();
        let registry = &mut self.registry;
        let border_config = &self.border_config;
        let dragged_pane = self.mouse_drag.dragged_pane();
        let dragged_rect = self.mouse_drag.preview_rect();
        let panes = self
            .animation_state
            .display_rects(area, self.core.state().panes(), now);

        for &(pane_id, rect) in panes {
            if Some(pane_id) == dragged_pane {
                continue;
            }
            let is_focused = highlight && Some(pane_id) == focused;
            render_runtime_pane(
                &mut *registry,
                border_config,
                pane_id,
                rect,
                buf,
                is_focused,
            );
        }

        let clipped_drag = dragged_rect
            .map(|rect| rect.intersection(area))
            .filter(|rect| !rect.is_empty());
        if let (Some(pane_id), Some(rect)) = (dragged_pane, clipped_drag) {
            Clear.render(rect, buf);
            let is_focused = highlight && Some(pane_id) == focused;
            render_runtime_pane(
                &mut *registry,
                border_config,
                pane_id,
                rect,
                buf,
                is_focused,
            );
        }

        if let Some(hover) = self.mouse_resize_hover {
            render_resize_hover(hover, area, border_config, buf);
        }

        if self.palette.show {
            self.render_palette(area, buf);
        }
    }

    pub(super) fn render_palette(&mut self, area: Rect, buf: &mut Buffer) {
        let filtered = self.filtered_palette_items();
        if filtered.is_empty() {
            return;
        }

        let popup = centered_rect(
            self.palette.width_percent,
            self.palette.height_percent,
            area,
        );
        Clear.render(popup, buf);

        let max_visible = self.palette.max_items.max(1).min(filtered.len());
        let start = self
            .palette
            .selected
            .saturating_sub(max_visible.saturating_sub(1));
        let end = (start + max_visible).min(filtered.len());
        let visible = &filtered[start..end];
        let selected = self.palette.selected.saturating_sub(start);

        let title = if self.palette.query.is_empty() {
            " Plugins ".to_string()
        } else {
            format!(" {} ", self.palette.query)
        };
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Rgb(137, 180, 250)))
            .title(title);
        let inner = block.inner(popup);
        block.render(popup, buf);

        let items = visible
            .iter()
            .map(|name| ListItem::new(format!("  {name}  ")))
            .collect::<Vec<_>>();
        let list = List::new(items).highlight_style(
            Style::default()
                .fg(Color::Rgb(30, 30, 46))
                .bg(Color::Rgb(137, 180, 250))
                .bold(),
        );
        let mut state = ListState::default();
        state.select(Some(selected));
        StatefulWidget::render(list, inner, buf, &mut state);
    }
}

fn render_runtime_pane(
    registry: &mut Registry,
    cfg: &BorderConfig,
    pane_id: PaneId,
    area: Rect,
    buf: &mut Buffer,
    is_focused: bool,
) {
    if let Some(plugin) = registry.plugin_mut(pane_id) {
        plugin.render(area, buf, is_focused);
    } else {
        render_fallback_pane(cfg, pane_id, area, buf, is_focused);
    }
}

fn render_fallback_pane(
    cfg: &BorderConfig,
    pane_id: PaneId,
    area: Rect,
    buf: &mut Buffer,
    is_focused: bool,
) {
    let mut block = Block::default()
        .borders(cfg.borders)
        .border_set(cfg.border_set)
        .border_style(cfg.border_style)
        .title(format!("Pane {}", pane_id.get()));
    if is_focused {
        block = block
            .border_set(cfg.focused_border_set)
            .border_style(cfg.focused_border_style);
    }
    block.render(area, buf);
}

fn render_resize_hover(
    hover: MouseResizeHover,
    bounds: Rect,
    cfg: &BorderConfig,
    buf: &mut Buffer,
) {
    match hover.direction {
        Direction::Horizontal => {
            let x = split_line_position(hover.rect.x, hover.rect.width, hover.ratio);
            let y_start = hover.rect.y.max(bounds.y);
            let y_end = hover
                .rect
                .y
                .saturating_add(hover.rect.height)
                .min(bounds.y.saturating_add(bounds.height));

            for y in y_start..y_end {
                if let Some(cell) = buf.cell_mut((x, y)) {
                    cell.set_symbol(cfg.focused_border_set.vertical_left)
                        .set_style(cfg.focused_border_style);
                }
            }
        }
        Direction::Vertical => {
            let y = split_line_position(hover.rect.y, hover.rect.height, hover.ratio);
            let x_start = hover.rect.x.max(bounds.x);
            let x_end = hover
                .rect
                .x
                .saturating_add(hover.rect.width)
                .min(bounds.x.saturating_add(bounds.width));

            for x in x_start..x_end {
                if let Some(cell) = buf.cell_mut((x, y)) {
                    cell.set_symbol(cfg.focused_border_set.horizontal_top)
                        .set_style(cfg.focused_border_style);
                }
            }
        }
    }
}

fn split_line_position(start: u16, length: u16, ratio: f32) -> u16 {
    let offset = (f32::from(length) * ratio).round() as u16;
    let offset = offset.min(length.saturating_sub(1));
    start.saturating_add(offset)
}

fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    let w = area.width * percent_x / 100;
    let h = area.height * percent_y / 100;
    let x = area.x + (area.width.saturating_sub(w)) / 2;
    let y = area.y + (area.height.saturating_sub(h)) / 2;
    Rect::new(x, y, w, h)
}
