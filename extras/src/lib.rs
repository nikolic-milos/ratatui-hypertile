//! Ready-made runtime on top of [`ratatui_hypertile`].
//!
//! Includes a plugin registry, vim-style modal input, a command palette,
//! workspace tabs, and a crossterm adapter. Ships with specific defaults,
//! use the core crate directly if you need full control.
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
    BorderConfig, HypertileRuntime, HypertileRuntimeBuilder, HypertileView, InputMode,
    ModeIndicator, MoveBindings, RuntimeError, SplitBehavior, TabBar, TabBarItem,
    WorkspaceAction, WorkspaceRuntime,
};

#[cfg(feature = "crossterm")]
pub use runtime::{event_from_crossterm, keychord_from_crossterm};
