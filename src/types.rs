use crate::core::{PaneId, normalize_ratio};
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
    /// 50/50 split (ratio = 0.5).
    #[default]
    Half,
    /// Golden-ratio split (ratio = 0.618).
    Golden,
    /// Custom ratio, clamped to `0.1..=0.9`.
    Fixed(f32),
}

impl SplitPolicy {
    pub(crate) fn ratio(self) -> f32 {
        match self {
            Self::Half => 0.5,
            Self::Golden => 0.618,
            Self::Fixed(ratio) => normalize_ratio(ratio),
        }
    }
}

/// A pane's id, rectangle, and focus state at one point in time.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PaneSnapshot {
    pub id: PaneId,
    pub rect: Rect,
    pub is_focused: bool,
}
