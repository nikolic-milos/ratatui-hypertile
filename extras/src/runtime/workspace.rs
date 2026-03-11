use ratatui::prelude::*;
use ratatui_hypertile::{EventOutcome, HypertileEvent};

use super::{HypertileRuntime, HypertileRuntimeBuilder};

struct Tab {
    label: String,
    runtime: HypertileRuntime,
}

/// Tabbed container that owns one [`HypertileRuntime`] per tab.
pub struct WorkspaceRuntime {
    tabs: Vec<Tab>,
    active: usize,
    builder_factory: Box<dyn Fn() -> HypertileRuntimeBuilder>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorkspaceAction {
    NewTab,
    CloseTab(usize),
    NextTab,
    PrevTab,
    GoToTab(usize),
    RenameTab(usize, String),
}

impl WorkspaceRuntime {
    /// Creates a workspace. The factory is used for each new tab.
    pub fn new(builder_factory: impl Fn() -> HypertileRuntimeBuilder + 'static) -> Self {
        let first = builder_factory().build();
        Self {
            tabs: vec![Tab {
                label: "1".to_string(),
                runtime: first,
            }],
            active: 0,
            builder_factory: Box::new(builder_factory),
        }
    }

    pub fn active_runtime(&self) -> &HypertileRuntime {
        &self.tabs[self.active].runtime
    }

    pub fn active_runtime_mut(&mut self) -> &mut HypertileRuntime {
        &mut self.tabs[self.active].runtime
    }

    pub fn tab_count(&self) -> usize {
        self.tabs.len()
    }

    pub fn active_tab_index(&self) -> usize {
        self.active
    }

    pub fn tab_labels(&self) -> Vec<(&str, bool)> {
        self.tabs
            .iter()
            .enumerate()
            .map(|(i, tab)| (tab.label.as_str(), i == self.active))
            .collect()
    }

    /// Adds a new tab and switches to it.
    pub fn new_tab(&mut self) {
        let label = (self.tabs.len() + 1).to_string();
        let runtime = (self.builder_factory)().build();
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

    pub fn handle_event(&mut self, event: HypertileEvent) -> EventOutcome {
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
        WorkspaceRuntime::new(HypertileRuntimeBuilder::default)
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
