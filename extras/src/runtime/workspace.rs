use ratatui::prelude::*;
use ratatui_hypertile::{EventOutcome, HypertileEvent, KeyCode, Modifiers};
use std::time::Duration;

use super::HypertileRuntime;

struct Tab {
    label: String,
    runtime: HypertileRuntime,
}

/// Small tab manager around [`HypertileRuntime`].
///
/// Use this when one runtime is not enough and you want a lightweight
/// workspace model without building it yourself. It intercepts a few `Ctrl+...`
/// keys for tab management and forwards everything else to the active tab.
pub struct WorkspaceRuntime {
    tabs: Vec<Tab>,
    active: usize,
    factory: Box<dyn Fn() -> HypertileRuntime>,
}

/// Command understood by [`WorkspaceRuntime`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorkspaceAction {
    /// Add a tab and switch to it.
    NewTab,
    /// Remove one tab by index.
    CloseTab(usize),
    /// Move to the next tab, wrapping at the end.
    NextTab,
    /// Move to the previous tab, wrapping at the start.
    PrevTab,
    /// Focus a specific tab by index.
    GoToTab(usize),
    /// Replace one tab label.
    RenameTab(usize, String),
}

impl WorkspaceRuntime {
    /// Creates a workspace from a runtime factory.
    ///
    /// The factory is reused for every new tab, so it should return a fully
    /// configured runtime with your plugin registrations already in place.
    pub fn new(factory: impl Fn() -> HypertileRuntime + 'static) -> Self {
        let first = factory();
        Self {
            tabs: vec![Tab {
                label: "1".to_string(),
                runtime: first,
            }],
            active: 0,
            factory: Box::new(factory),
        }
    }

    pub fn active_runtime(&self) -> &HypertileRuntime {
        &self.tabs[self.active].runtime
    }

    pub fn active_runtime_mut(&mut self) -> &mut HypertileRuntime {
        &mut self.tabs[self.active].runtime
    }

    /// Mirrors [`HypertileRuntime::next_frame_in`] for the active tab.
    pub fn next_frame_in(&self) -> Option<Duration> {
        self.tabs[self.active].runtime.next_frame_in()
    }

    pub fn tab_count(&self) -> usize {
        self.tabs.len()
    }

    pub fn active_tab_index(&self) -> usize {
        self.active
    }

    pub fn tab_labels(&self) -> impl Iterator<Item = (&str, bool)> {
        self.tabs
            .iter()
            .enumerate()
            .map(move |(i, tab)| (tab.label.as_str(), i == self.active))
    }

    /// Adds a new tab and switches to it.
    pub fn new_tab(&mut self) {
        let label = (self.tabs.len() + 1).to_string();
        let runtime = (self.factory)();
        self.tabs.push(Tab { label, runtime });
        self.active = self.tabs.len() - 1;
    }

    /// Does nothing if this is the last tab or the index is out of range.
    pub fn close_tab(&mut self, index: usize) {
        if self.tabs.len() <= 1 || index >= self.tabs.len() {
            return;
        }
        self.tabs.remove(index);
        if self.active >= self.tabs.len() {
            self.active = self.tabs.len() - 1;
        } else if self.active > index {
            self.active -= 1;
        }
    }

    /// Wraps around.
    pub fn next_tab(&mut self) {
        self.active = (self.active + 1) % self.tabs.len();
    }

    /// Wraps around.
    pub fn prev_tab(&mut self) {
        self.active = (self.active + self.tabs.len() - 1) % self.tabs.len();
    }

    /// Does nothing if the index is out of range.
    pub fn go_to_tab(&mut self, index: usize) {
        if index < self.tabs.len() {
            self.active = index;
        }
    }

    /// Does nothing if the index is out of range.
    pub fn rename_tab(&mut self, index: usize, label: String) {
        if let Some(tab) = self.tabs.get_mut(index) {
            tab.label = label;
        }
    }

    pub fn apply_workspace_action(&mut self, action: WorkspaceAction) {
        match action {
            WorkspaceAction::NewTab => self.new_tab(),
            WorkspaceAction::CloseTab(i) => self.close_tab(i),
            WorkspaceAction::NextTab => self.next_tab(),
            WorkspaceAction::PrevTab => self.prev_tab(),
            WorkspaceAction::GoToTab(i) => self.go_to_tab(i),
            WorkspaceAction::RenameTab(i, label) => self.rename_tab(i, label),
        }
    }

    /// Handles one event for the active tab.
    ///
    /// `Ctrl+t`, `Ctrl+w`, `Ctrl+n`, `Ctrl+p`, `Ctrl+Left`, and `Ctrl+Right`
    /// are reserved for tab management. Everything else goes to the active
    /// runtime.
    pub fn handle_event(&mut self, event: HypertileEvent) -> EventOutcome {
        if let HypertileEvent::Key(chord) = &event
            && chord.modifiers == Modifiers::CTRL
        {
            match chord.code {
                KeyCode::Char('t') => {
                    self.new_tab();
                    return EventOutcome::Consumed;
                }
                KeyCode::Char('w') => {
                    self.close_tab(self.active);
                    return EventOutcome::Consumed;
                }
                KeyCode::Char('n') | KeyCode::Right => {
                    self.next_tab();
                    return EventOutcome::Consumed;
                }
                KeyCode::Char('p') | KeyCode::Left => {
                    self.prev_tab();
                    return EventOutcome::Consumed;
                }
                _ => {}
            }
        }
        self.tabs[self.active].runtime.handle_event(event)
    }

    pub fn render(&mut self, area: Rect, buf: &mut Buffer) {
        self.tabs[self.active].runtime.render(area, buf);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_workspace() -> WorkspaceRuntime {
        WorkspaceRuntime::new(HypertileRuntime::new)
    }

    #[test]
    fn tab_lifecycle_keeps_active_index_valid() {
        let mut ws = test_workspace();
        ws.new_tab();
        ws.new_tab();
        ws.go_to_tab(0);
        ws.close_tab(0);
        assert_eq!(ws.tab_count(), 2);
        assert_eq!(ws.active_tab_index(), 0);
        ws.next_tab();
        assert_eq!(ws.active_tab_index(), 1);
        ws.prev_tab();
        assert_eq!(ws.active_tab_index(), 0);
        ws.close_tab(0);
        ws.close_tab(0);
        assert_eq!(ws.tab_count(), 1);
        assert_eq!(ws.active_tab_index(), 0);
    }
}
