use crate::runtime::HypertileRuntime;
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Style},
    widgets::{Block, Borders, Clear, List, ListItem, ListState, StatefulWidget, Widget},
};
use ratatui_hypertile::PaneId;

impl HypertileRuntime {
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

    pub(crate) fn render_fallback_pane(
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

fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    let w = area.width * percent_x / 100;
    let h = area.height * percent_y / 100;
    let x = area.x + (area.width.saturating_sub(w)) / 2;
    let y = area.y + (area.height.saturating_sub(h)) / 2;
    Rect::new(x, y, w, h)
}
