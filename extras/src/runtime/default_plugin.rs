use crate::registry::HypertilePlugin;
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Style, Stylize},
    symbols::border,
    widgets::{Block, Borders, Widget},
};

pub(super) struct DefaultBlockPlugin {
    title: String,
}

impl DefaultBlockPlugin {
    pub(super) fn new(title: &str) -> Self {
        Self {
            title: title.to_string(),
        }
    }
}

impl HypertilePlugin for DefaultBlockPlugin {
    fn render(&mut self, area: Rect, buf: &mut Buffer, is_focused: bool) {
        let mut block = Block::default()
            .borders(Borders::ALL)
            .title(self.title.as_str());
        if is_focused {
            block = block
                .border_set(border::THICK)
                .border_style(Style::default().fg(Color::Yellow))
                .bold();
        }
        block.render(area, buf);
    }
}
