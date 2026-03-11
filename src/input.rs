use ratatui::layout::Direction;
use std::ops::{BitOr, BitOrAssign};

/// Event delivered to the layout engine or runtime.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum HypertileEvent {
    Key(KeyChord),
    Action(HypertileAction),
    Tick,
}

/// Backend-agnostic key code.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum KeyCode {
    Char(char),
    Enter,
    Escape,
    Tab,
    BackTab,
    Backspace,
    Home,
    End,
    PageUp,
    PageDown,
    Delete,
    Insert,
    F(u8),
    Up,
    Down,
    Left,
    Right,
}

/// Modifier keys.
///
/// ```
/// use ratatui_hypertile::Modifiers;
///
/// let combo = Modifiers::SHIFT | Modifiers::CTRL;
/// assert!(combo.contains(Modifiers::SHIFT));
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Modifiers(u8);

impl Modifiers {
    pub const NONE: Self = Self(0);
    pub const SHIFT: Self = Self(1 << 0);
    pub const CTRL: Self = Self(1 << 1);
    pub const ALT: Self = Self(1 << 2);

    pub fn contains(self, other: Self) -> bool {
        (self.0 & other.0) == other.0
    }

    pub fn is_empty(self) -> bool {
        self.0 == 0
    }
}

impl BitOr for Modifiers {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        Self(self.0 | rhs.0)
    }
}

impl BitOrAssign for Modifiers {
    fn bitor_assign(&mut self, rhs: Self) {
        self.0 |= rhs.0;
    }
}

/// Key code plus modifiers.
///
/// ```
/// use ratatui_hypertile::{KeyChord, KeyCode, Modifiers};
///
/// let chord = KeyChord::with_modifiers(KeyCode::Char('h'), Modifiers::CTRL);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct KeyChord {
    pub code: KeyCode,
    pub modifiers: Modifiers,
}

impl KeyChord {
    pub const fn new(code: KeyCode) -> Self {
        Self {
            code,
            modifiers: Modifiers::NONE,
        }
    }

    pub const fn with_modifiers(code: KeyCode, modifiers: Modifiers) -> Self {
        Self { code, modifiers }
    }
}

/// `Start` means left or up. `End` means right or down.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Towards {
    Start,
    End,
}

/// How pane moves are resolved.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MoveScope {
    /// Swap with the nearest pane in that direction.
    Window,
    /// Swap inside the nearest ancestor split on that axis.
    Split,
}

/// Command understood by [`crate::Hypertile`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum HypertileAction {
    FocusNext,
    FocusPrev,
    FocusDirection {
        direction: Direction,
        towards: Towards,
    },
    SplitFocused {
        direction: Direction,
    },
    CloseFocused,
    ResizeFocused {
        delta: f32,
    },
    SetFocusedRatio {
        ratio: f32,
    },
    MoveFocused {
        direction: Direction,
        towards: Towards,
        scope: MoveScope,
    },
}

/// Whether an event handler consumed an event.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventOutcome {
    Ignored,
    Consumed,
}

impl EventOutcome {
    pub fn is_consumed(self) -> bool {
        matches!(self, Self::Consumed)
    }
}
