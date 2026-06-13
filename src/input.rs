use ratatui::layout::Direction;
use std::ops::{BitOr, BitOrAssign};

/// Event delivered to the layout engine or runtime.
///
/// The core engine only acts on `Action`. Key and tick are for higher-level code.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum HypertileEvent {
    Key(KeyChord),
    Mouse(MouseEvent),
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

/// Mouse button.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
}

/// What the mouse did.
///
/// `Drag` means the mouse moved while the button was held down.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MouseEventKind {
    Down(MouseButton),
    Up(MouseButton),
    Drag(MouseButton),
    Moved,
    ScrollUp,
    ScrollDown,
    ScrollLeft,
    ScrollRight,
}

/// Backend-agnostic mouse event.
///
/// `column` and `row` are absolute terminal coordinates, not pane-local.
/// Use [`Hypertile::pane_at`](crate::Hypertile::pane_at) or
/// [`Hypertile::split_at`](crate::Hypertile::split_at) to map a position to
/// the layout.
///
/// ```
/// use ratatui_hypertile::{MouseButton, MouseEvent, MouseEventKind};
///
/// let click = MouseEvent::new(MouseEventKind::Down(MouseButton::Left), 10, 4);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MouseEvent {
    pub kind: MouseEventKind,
    pub column: u16,
    pub row: u16,
    pub modifiers: Modifiers,
}

impl MouseEvent {
    pub const fn new(kind: MouseEventKind, column: u16, row: u16) -> Self {
        Self {
            kind,
            column,
            row,
            modifiers: Modifiers::NONE,
        }
    }

    pub const fn with_modifiers(
        kind: MouseEventKind,
        column: u16,
        row: u16,
        modifiers: Modifiers,
    ) -> Self {
        Self {
            kind,
            column,
            row,
            modifiers,
        }
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
    /// Swap with the nearest pane in that direction (requires layout).
    Window,
    /// Swap inside the nearest ancestor split on that axis.
    Split,
}

/// Layout command for [`Hypertile::apply_action`](crate::Hypertile::apply_action).
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

/// Whether an event handler used an event.
///
/// `Consumed` means the handler acted on the event: it changed state, or it
/// claimed the event during an exclusive interaction like a drag or modal.
/// It does not promise that the next redraw looks different.
///
/// `Ignored` means the handler did nothing with the event, so callers can
/// skip a redraw and pass the event to a fallback handler. Animation redraws
/// are timed separately by the runtime's frame-timing API.
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
