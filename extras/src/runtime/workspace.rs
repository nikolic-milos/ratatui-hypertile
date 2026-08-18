use ratatui::prelude::*;
use ratatui_hypertile::{EventOutcome, HypertileEvent, KeyCode, Modifiers};
use std::time::Duration;

use super::HypertileRuntime;

/// Uniquely identifies a tab
#[derive(Debug, Clone, Copy, Eq, PartialEq, PartialOrd, Ord, Hash)]
pub struct TabId(usize);

impl TabId {
    /// Extracts the underlying primitive value from the identifier
    pub const fn get(&self) -> usize {
        self.0
    }
}

/// Represents a tab in the application.
struct Tab {
    /// Unique identifier.
    tab_id: TabId,
    /// Display text shown in the tab header.
    label: String,
    /// Execution environment for the tab's content.
    runtime: HypertileRuntime,
}

/// Small tab manager around [`HypertileRuntime`].
///
/// Use this when one runtime is not enough and you want a lightweight
/// workspace model without building it yourself. It intercepts a few `Ctrl+...`
/// keys for tab management and forwards everything else to the active tab.
pub struct WorkspaceRuntime {
    /// Manages the collection of open tabs in the workspace.
    tabs: Vec<Tab>,
    /// Index of the currently focused tab.
    active: usize,
    /// Monotonically increasing source of unique identifiers for tabs. Never
    /// decrements, even when tabs are closed.
    next_tab_id: usize,
    /// Produces new tab runtimes on demand.
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
        /// Default initial tab when the application starts
        const FIRST_TAB: TabId = TabId(0);
        /// Starting index for dynamically created tabs after the default tab
        const FIRST_NEXT_ID: usize = FIRST_TAB.0 + 1;

        let first = factory();
        Self {
            tabs: vec![Tab {
                label: "1".to_string(),
                runtime: first,
                tab_id: FIRST_TAB,
            }],
            active: 0,
            next_tab_id: FIRST_NEXT_ID,
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

    /// Returns the active tab id
    pub fn active_tab_id(&self) -> TabId {
        self.tabs[self.active].tab_id
    }

    /// Returns an iterator over the identifiers of all currently open tabs
    pub fn open_tab_ids(&self) -> impl Iterator<Item = TabId> {
        self.tabs.iter().map(|t| t.tab_id)
    }

    /// Returns `true` if the given tab id is open
    ///
    /// To check if it's the active tab, use [`WorkspaceRuntime::active_tab_id`]
    pub fn is_tab_open(&self, tab_id: TabId) -> bool {
        self.tabs.iter().any(|t| t.tab_id == tab_id)
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
        self.tabs.push(Tab {
            label,
            runtime,
            tab_id: TabId(self.next_tab_id),
        });
        self.next_tab_id += 1;
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
