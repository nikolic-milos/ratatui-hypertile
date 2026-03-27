use std::fmt::Write;

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    widgets::{Block, Borders, Paragraph, Widget},
};

use super::workspace::WorkspaceRuntime;

/// One entry in a [`TabBar`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TabBarItem {
    pub label: String,
    pub is_active: bool,
}

/// Widget that lists workspace tabs.
#[derive(Debug, Clone)]
pub struct TabBar {
    pub items: Vec<TabBarItem>,
}

impl TabBar {
    /// Builds tab items from the current workspace state.
    pub fn from_workspace(workspace: &WorkspaceRuntime) -> Self {
        Self {
            items: workspace
                .tab_labels()
                .map(|(label, active)| TabBarItem {
                    label: label.to_owned(),
                    is_active: active,
                })
                .collect(),
        }
    }
}

impl Widget for TabBar {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let text = if self.items.is_empty() {
            "no tabs".to_string()
        } else {
            let mut text = String::new();
            for (i, item) in self.items.iter().enumerate() {
                if i > 0 {
                    text.push_str(" | ");
                }
                let marker = if item.is_active { '*' } else { ' ' };
                let _ = write!(text, "{}{}", marker, item.label);
            }
            text
        };

        Paragraph::new(text)
            .block(Block::default().borders(Borders::ALL).title("Tabs"))
            .render(area, buf);
    }
}
