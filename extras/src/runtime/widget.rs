use ratatui::prelude::*;
use ratatui::style::Color;

use super::types::InputMode;
use super::HypertileRuntime;

/// [`StatefulWidget`] wrapper for [`HypertileRuntime`](super::HypertileRuntime).
pub struct HypertileView;

impl StatefulWidget for HypertileView {
    type State = HypertileRuntime;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        state.render(area, buf);
    }
}

/// Shows the current mode as a short label like `LAYOUT` or `INPUT`.
///
/// Place it wherever you want, e.g. bottom left of the screen.
pub struct ModeIndicator {
    mode: InputMode,
}

impl ModeIndicator {
    pub fn new(mode: InputMode) -> Self {
        Self { mode }
    }
}

impl Widget for ModeIndicator {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let (label, bg) = match self.mode {
            InputMode::Layout => (" LAYOUT ", Color::Rgb(137, 180, 250)),
            InputMode::PluginInput => (" INPUT ", Color::Rgb(166, 227, 161)),
        };
        Span::styled(
            label,
            Style::default().fg(Color::Rgb(30, 30, 46)).bg(bg).bold(),
        )
        .render(area, buf);
    }
}
