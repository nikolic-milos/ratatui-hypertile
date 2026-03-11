use ratatui::layout::Direction;

/// Stable pane identifier.
///
/// ```
/// use ratatui_hypertile::PaneId;
///
/// assert_eq!(PaneId::ROOT.get(), 0);
/// assert_eq!(PaneId::new(42).get(), 42);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct PaneId(u64);

impl PaneId {
    pub const ROOT: Self = Self(0);

    pub fn new(id: u64) -> Self {
        Self(id)
    }

    pub fn get(self) -> u64 {
        self.0
    }
}

impl std::fmt::Display for PaneId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Clone)]
pub enum Node {
    Pane(PaneId),
    Split {
        direction: Direction,
        ratio: f32,
        first: Box<Node>,
        second: Box<Node>,
    },
}

/// Returned when a raw tree mutation or focus operation cannot be applied.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StateError {
    InvalidPath,
    FocusedNodeNotPane,
    ParentNodeNotSplit,
    CannotRemoveRootPane,
    DuplicatePaneId(PaneId),
    UnknownPaneId(PaneId),
    LayoutUnavailable,
}

impl std::fmt::Display for StateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidPath => write!(f, "invalid pane path"),
            Self::FocusedNodeNotPane => write!(f, "focused node is not a pane"),
            Self::ParentNodeNotSplit => write!(f, "focused pane parent is not a split"),
            Self::CannotRemoveRootPane => write!(f, "cannot remove root pane"),
            Self::DuplicatePaneId(id) => write!(f, "duplicate pane id: {id}"),
            Self::UnknownPaneId(id) => write!(f, "unknown pane id: {id}"),
            Self::LayoutUnavailable => {
                write!(f, "directional layout actions require a computed layout")
            }
        }
    }
}

impl std::error::Error for StateError {}
