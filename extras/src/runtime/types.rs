use crate::registry::RegistryError;
use ratatui::prelude::*;
use ratatui::symbols::border;
use ratatui::widgets::Borders;
use ratatui_hypertile::StateError;
use std::time::Duration;

/// Controls the small slide animation used when panes move.
///
/// Only pane moves are animated right now. If you enable this, make sure your
/// event loop also checks [`HypertileRuntime::next_frame_in`](super::HypertileRuntime::next_frame_in)
/// so the animation keeps advancing between input events.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AnimationConfig {
    pub enabled: bool,
    pub duration: Duration,
    pub frame_interval: Duration,
}

impl Default for AnimationConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            duration: Duration::from_millis(180),
            frame_interval: Duration::from_millis(16),
        }
    }
}

/// Border look used by the built-in fallback pane rendering.
///
/// Plugins that fully paint their own content can ignore this. It mainly
/// matters for the placeholder pane and for apps that rely on the default pane
/// frame.
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

/// Decides what a split shortcut should put in the new pane.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SplitBehavior {
    /// Use the plugin name set on the builder or runtime.
    DefaultPlugin,
    /// Start with the built-in placeholder pane.
    Placeholder,
    /// Open the palette and let the user pick a plugin right away.
    PromptPalette,
}

/// Tells the runtime where keyboard input goes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputMode {
    /// Keys drive focus, splits, movement, resize, and the palette.
    Layout,
    /// Keys are forwarded to the focused plugin.
    PluginInput,
}

/// Errors returned by [`HypertileRuntime`](super::HypertileRuntime).
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
