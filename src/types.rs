use crate::core::PaneId;
use ratatui::layout::Rect;

/// Split ratio policy.
///
/// ```
/// use ratatui_hypertile::SplitPolicy;
///
/// let policy = SplitPolicy::Golden;
/// let fixed = SplitPolicy::Fixed(0.3);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum SplitPolicy {
    #[default]
    Half,
    Golden,
    Fixed(f32),
}

impl SplitPolicy {
    pub(crate) fn ratio(self) -> f32 {
        match self {
            Self::Half => 0.5,
            Self::Golden => 0.618,
            Self::Fixed(ratio) => ratio,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PaneSnapshot {
    pub id: PaneId,
    pub rect: Rect,
    pub is_focused: bool,
}
