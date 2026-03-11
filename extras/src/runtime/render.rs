use std::fmt::Write;

use crate::runtime::HypertileRuntime;
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Style},
    widgets::{
        Block, Borders, Clear, List, ListItem, ListState, Paragraph, StatefulWidget, Widget,
    },
};
use ratatui_hypertile::PaneId;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PaneBarItem {
    pub pane_id: PaneId,
    pub plugin_type: String,
    pub is_focused: bool,
}

#[derive(Debug, Clone)]
pub struct PaneBar {
    title: String,
    items: Vec<PaneBarItem>,
}

impl PaneBar {
    pub fn new(items: Vec<PaneBarItem>) -> Self {
        Self {
            title: "Pane Bar".to_string(),
            items,
        }
    }

    pub fn from_runtime(runtime: &HypertileRuntime) -> Self {
        Self::new(runtime.pane_bar_items())
    }

    pub fn with_title(mut self, title: impl Into<String>) -> Self {
        self.title = title.into();
        self
    }
}

impl Widget for PaneBar {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let text = if self.items.is_empty() {
            "no panes".to_string()
        } else {
            let mut text = String::new();
            for (i, item) in self.items.iter().enumerate() {
                if i > 0 {
                    text.push_str(" | ");
                }
                let marker = if item.is_focused { '*' } else { ' ' };
                let _ = write!(
                    text,
                    "{}{}:{}",
                    marker,
                    item.pane_id.get(),
                    item.plugin_type
                );
            }
            text
        };

        Paragraph::new(text)
            .block(Block::default().borders(Borders::ALL).title(self.title))
            .render(area, buf);
    }
}

impl HypertileRuntime {
    pub fn pane_bar_items(&self) -> Vec<PaneBarItem> {
        let focused = self.core.focused_pane();
        let highlight = self.core.state().focus_highlight();
        self.core
            .panes_iter()
            .map(|pane| PaneBarItem {
                pane_id: pane.id,
                plugin_type: self
                    .registry
                    .plugin_type_for(pane.id)
                    .unwrap_or("unknown")
                    .to_string(),
                is_focused: highlight && Some(pane.id) == focused,
            })
            .collect()
    }

    /// Renders panes and the palette overlay if it is open.
    pub fn render(&mut self, area: Rect, buf: &mut Buffer) {
        self.core.compute_layout(area);
        let focused = self.core.focused_pane();
        let highlight = self.core.state().focus_highlight();
        let state = self.core.state();

        for (pane_id, rect) in state.panes() {
            let is_focused = highlight && Some(pane_id) == focused;
            if let Some(plugin) = self.registry.plugin(pane_id) {
                plugin.render(rect, buf, is_focused);
            } else {
                self.render_fallback_pane(pane_id, rect, buf, is_focused);
            }
        }

        if self.palette.show {
            self.render_palette(area, buf);
        }
    }

    /// Fallback renderer for panes without a plugin.
    pub fn render_fallback_pane(
        &self,
        pane_id: PaneId,
        area: Rect,
        buf: &mut Buffer,
        is_focused: bool,
    ) {
        let cfg = &self.border_config;
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

        let block = Block::default().borders(Borders::ALL).title(format!(
            "Add Plugin ({}/{}) q='{}'",
            self.palette.selected + 1,
            filtered.len(),
            self.palette.query
        ));
        let inner = block.inner(popup);
        block.render(popup, buf);

        let items = visible
            .iter()
            .map(|name| ListItem::new(name.as_str()))
            .collect::<Vec<_>>();
        let list =
            List::new(items).highlight_style(Style::default().fg(Color::Black).bg(Color::White));
        let mut state = ListState::default();
        state.select(Some(selected));
        StatefulWidget::render(list, inner, buf, &mut state);
    }
}

fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    let w = area.width * percent_x / 100;
    let h = area.height * percent_y / 100;
    let x = area.x + (area.width.saturating_sub(w)) / 2;
    let y = area.y + (area.height.saturating_sub(h)) / 2;
    Rect::new(x, y, w, h)
}
