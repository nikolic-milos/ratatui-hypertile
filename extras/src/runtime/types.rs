use crate::registry::RegistryError;
use ratatui::prelude::*;
use ratatui::symbols::border;
use ratatui::widgets::Borders;
use ratatui_hypertile::StateError;

/// Border styling for panes and the focused-pane highlight.
#[derive(Debug, Clone)]
pub struct BorderConfig {
    pub borders: Borders,
    pub border_set: border::Set<'static>,
    pub border_style: Style,
    pub focused_border_set: border::Set<'static>,
    pub focused_border_style: Style,
}

impl Default for BorderConfig {
    fn default() -> Self {
        Self {
            borders: Borders::ALL,
            border_set: border::PLAIN,
            border_style: Style::default(),
            focused_border_set: border::THICK,
            focused_border_style: Style::default().fg(Color::Yellow).bold(),
        }
    }
}

/// Chooses which plugin appears after a split shortcut.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SplitBehavior {
    /// Mount the configured default split plugin type.
    DefaultPlugin,
    /// Mount the built-in placeholder plugin.
    Placeholder,
    /// Open the plugin palette so the user can choose a plugin.
    PromptPalette,
}

/// `Layout` captures keys for tiling, `PluginInput` forwards them to the focused plugin.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputMode {
    Layout,
    PluginInput,
}

/// Wraps layout-state and registry errors produced by [`super::HypertileRuntime`].
#[derive(Debug, Clone)]
pub enum RuntimeError {
    State(StateError),
    Registry(RegistryError),
    NoFocusedPane,
}

impl std::fmt::Display for RuntimeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::State(err) => write!(f, "{err}"),
            Self::Registry(err) => write!(f, "{err}"),
            Self::NoFocusedPane => write!(f, "no focused pane"),
        }
    }
}

impl std::error::Error for RuntimeError {}

impl From<StateError> for RuntimeError {
    fn from(value: StateError) -> Self {
        Self::State(value)
    }
}

impl From<RegistryError> for RuntimeError {
    fn from(value: RegistryError) -> Self {
        Self::Registry(value)
    }
}
