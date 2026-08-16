//! Ready-made runtime on top of [`ratatui_hypertile`].
//!
//! Includes a plugin registry, vim-style modal input, a command palette,
//! workspace tabs, pane-move animations, and a crossterm adapter.
//!
//! Use [`HypertileRuntime`] for a single tiling view or [`WorkspaceRuntime`]
//! for a tabbed workspace.
//!
//! ```
//! use ratatui_hypertile_extras::HypertileRuntime;
//!
//! let runtime = HypertileRuntime::new();
//! assert!(runtime.focused_pane().is_some());
//! ```

mod registry;
mod runtime;

pub use registry::{HypertilePlugin, PluginContext, Registry, RegistryError};
pub use runtime::{
    AnimationConfig, BorderConfig, HypertileRuntime, HypertileRuntimeBuilder, HypertileView,
    InputMode, ModeIndicator, MoveBindings, RuntimeError, SplitBehavior, TabBar, TabBarItem, TabId,
    WorkspaceAction, WorkspaceRuntime,
};

#[cfg(feature = "crossterm")]
pub use runtime::{
    event_from_crossterm, hypertile_event_from_crossterm, keychord_from_crossterm,
    mouse_event_from_crossterm,
};
