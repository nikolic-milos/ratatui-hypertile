//! Binary-space-partition tiling layout engine for [`ratatui`].
//!
//! This crate handles pane splits, focus navigation, resize, and movement.
//! It only computes rectangles; you render each pane yourself and handle
//! terminal input however you like.
//!
//! # Quick start
//!
//! ```
//! use ratatui::layout::Direction;
//! use ratatui_hypertile::{Hypertile, HypertileAction, EventOutcome};
//!
//! let mut layout = Hypertile::new();
//! let pane = layout.split_focused(Direction::Horizontal).unwrap();
//!
//! let outcome = layout.apply_action(HypertileAction::FocusNext);
//! assert_eq!(outcome, EventOutcome::Consumed);
//! ```
//!
//! # Two-crate design
//!
//! | Crate | Purpose |
//! |-------|---------|
//! | **`ratatui-hypertile`** (this crate) | Lightweight layout engine, no input handling, no rendering opinions |
//! | [`ratatui-hypertile-extras`](https://docs.rs/ratatui-hypertile-extras) | Full runtime with plugins, vim-style keymaps, command palette, workspace tabs, and pane-move animations |
//!
//! Use the core crate when you want full control over input and rendering.
//! Use extras when you want a working tiling UI out of the box.

mod core;
mod engine;
mod input;
mod types;
mod widget;

pub use crate::core::{PaneId, StateError};
pub use crate::engine::{Hypertile, HypertileBuilder};
pub use crate::input::{
    EventOutcome, HypertileAction, HypertileEvent, KeyChord, KeyCode, Modifiers, MouseButton,
    MouseEvent, MouseEventKind, MoveScope, Towards,
};
pub use crate::types::{PaneSnapshot, SplitPolicy, SplitSnapshot};
pub use crate::widget::HypertileWidget;

/// Low-level tree types and state.
///
/// Most apps should use [`Hypertile`] instead. Reach for `raw` when you need
/// direct tree manipulation, custom serialization, or your own action dispatch.
pub mod raw {
    pub use crate::core::{HypertileState, Node, PaneId, StateError, collect_pane_ids};
}

/// Re-exports every public type for quick `use ratatui_hypertile::prelude::*`.
pub mod prelude {
    pub use crate::{
        EventOutcome, Hypertile, HypertileAction, HypertileBuilder, HypertileEvent,
        HypertileWidget, KeyChord, KeyCode, Modifiers, MouseButton, MouseEvent, MouseEventKind,
        MoveScope, PaneId, PaneSnapshot, SplitPolicy, SplitSnapshot, StateError, Towards,
    };
}
