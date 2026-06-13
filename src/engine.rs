use crate::core::{HypertileState, Node, PaneId, StateError};
use crate::input::{EventOutcome, HypertileAction, HypertileEvent, MoveScope};
use crate::types::{PaneSnapshot, SplitPolicy, SplitSnapshot};
use ratatui::layout::{Direction, Rect};

const DEFAULT_RESIZE_STEP: f32 = 0.05;

/// Builder for [`Hypertile`].
///
/// ```
/// use ratatui_hypertile::{HypertileBuilder, SplitPolicy};
///
/// let layout = HypertileBuilder::default()
///     .with_split_policy(SplitPolicy::Golden)
///     .with_gap(1)
///     .build();
/// ```
#[derive(Debug, Clone)]
pub struct HypertileBuilder {
    highlight_focus: bool,
    resize_step: f32,
    split_policy: SplitPolicy,
    gap: u16,
}

impl Default for HypertileBuilder {
    fn default() -> Self {
        Self {
            highlight_focus: true,
            resize_step: DEFAULT_RESIZE_STEP,
            split_policy: SplitPolicy::default(),
            gap: 0,
        }
    }
}

impl HypertileBuilder {
    /// When disabled, [`PaneSnapshot::is_focused`](crate::PaneSnapshot::is_focused)
    /// is always `false` so you can draw your own focus indicator.
    pub fn with_focus_highlight(mut self, enabled: bool) -> Self {
        self.highlight_focus = enabled;
        self
    }

    /// Non-finite or non-positive values are ignored. Defaults to 5%.
    pub fn with_resize_step(mut self, step: f32) -> Self {
        if step.is_finite() && step > 0.0 {
            self.resize_step = step;
        }
        self
    }

    pub fn with_split_policy(mut self, policy: SplitPolicy) -> Self {
        self.split_policy = policy;
        self
    }

    /// Gap between pane borders, in cells.
    pub fn with_gap(mut self, gap: u16) -> Self {
        self.gap = gap;
        self
    }

    /// Starts with a single root pane.
    pub fn build(self) -> Hypertile {
        let mut state = HypertileState::new();
        state.set_focus_highlight(self.highlight_focus);
        state.set_gap(self.gap);

        Hypertile {
            state,
            resize_step: self.resize_step,
            split_policy: self.split_policy,
        }
    }
}

/// BSP tiling layout engine.
///
/// Send [`HypertileAction`]s through [`apply_action`](Self::apply_action) or
/// call the split/close/resize/move methods directly. Layout rectangles are
/// computed lazily via [`compute_layout`](Self::compute_layout) and cached
/// until the tree or area changes.
///
/// ```
/// use ratatui::layout::Direction;
/// use ratatui_hypertile::Hypertile;
///
/// let mut layout = Hypertile::new();
/// let pane = layout.split_focused(Direction::Horizontal).unwrap();
/// assert_eq!(layout.focused_pane(), Some(pane));
/// ```
#[derive(Debug, Clone)]
pub struct Hypertile {
    state: HypertileState,
    resize_step: f32,
    split_policy: SplitPolicy,
}

impl Default for Hypertile {
    fn default() -> Self {
        Self::new()
    }
}

impl Hypertile {
    #[must_use]
    pub fn builder() -> HypertileBuilder {
        HypertileBuilder::default()
    }

    #[must_use]
    pub fn new() -> Self {
        Self::builder().build()
    }

    pub fn state(&self) -> &HypertileState {
        &self.state
    }

    pub fn state_mut(&mut self) -> &mut HypertileState {
        &mut self.state
    }

    /// No-op if the area and tree have not changed. Must be called before
    /// [`panes`](Self::panes), [`pane_rect`](Self::pane_rect), or directional
    /// focus/move actions.
    pub fn compute_layout(&mut self, area: Rect) {
        self.state.compute_layout(area);
    }

    /// Non-finite or non-positive values are ignored.
    pub fn set_resize_step(&mut self, step: f32) {
        if step.is_finite() && step > 0.0 {
            self.resize_step = step;
        }
    }

    pub fn resize_step(&self) -> f32 {
        self.resize_step
    }

    pub fn set_split_policy(&mut self, policy: SplitPolicy) {
        self.split_policy = policy;
    }

    pub fn split_policy(&self) -> SplitPolicy {
        self.split_policy
    }

    pub fn gap(&self) -> u16 {
        self.state.gap()
    }

    pub fn set_gap(&mut self, gap: u16) {
        self.state.set_gap(gap);
    }

    #[must_use]
    pub fn focused_pane(&self) -> Option<PaneId> {
        self.state.focused_pane()
    }

    pub fn focus_pane(&mut self, pane_id: PaneId) -> Result<(), StateError> {
        self.state.focus_pane(pane_id)
    }

    #[must_use]
    pub fn pane_rect(&self, pane_id: PaneId) -> Option<Rect> {
        self.state.pane_rect(pane_id)
    }

    /// Pane under the given terminal position.
    ///
    /// Uses cached rectangles, so it returns `None` before the first
    /// [`compute_layout`](Self::compute_layout).
    #[must_use]
    pub fn pane_at(&self, column: u16, row: u16) -> Option<PaneId> {
        self.state.pane_at(column, row)
    }

    /// Like [`pane_at`](Self::pane_at) but returns the pane's rect and focus
    /// state too.
    #[must_use]
    pub fn pane_snapshot_at(&self, column: u16, row: u16) -> Option<PaneSnapshot> {
        let id = self.state.pane_at(column, row)?;
        let rect = self.state.pane_rect(id)?;
        let focused = self.state.focused_pane();
        Some(PaneSnapshot {
            id,
            rect,
            is_focused: self.state.focus_highlight() && focused == Some(id),
        })
    }

    /// Split whose border is within `tolerance` cells of the position.
    ///
    /// The closest border wins; on a tie the deepest split wins. Returns
    /// `None` before the first [`compute_layout`](Self::compute_layout).
    #[must_use]
    pub fn split_at(&self, column: u16, row: u16, tolerance: u16) -> Option<SplitSnapshot> {
        self.state.split_at(column, row, tolerance)
    }

    /// `0` means first child, `1` means second.
    #[must_use]
    pub fn pane_path(&self, pane_id: PaneId) -> Option<Vec<usize>> {
        self.state.pane_path(pane_id)
    }

    pub fn root(&self) -> &Node {
        self.state.root()
    }

    /// Replaces the tree, normalizes it, validates unique ids, and resets focus.
    pub fn set_root(&mut self, root: Node) -> Result<(), StateError> {
        self.state.set_root(root)
    }

    pub fn reset(&mut self) {
        self.state.reset();
    }

    /// Visits every node in pre-order.
    pub fn walk_preorder<F>(&self, visit: F)
    where
        F: FnMut(&[usize], &Node),
    {
        self.state.walk_preorder(visit)
    }

    /// Panes in geometric order. Requires a prior [`compute_layout`](Self::compute_layout).
    #[must_use]
    pub fn panes(&self) -> Vec<PaneSnapshot> {
        self.panes_iter().collect()
    }

    /// Like [`panes`](Self::panes) but without allocating.
    pub fn panes_iter(&self) -> impl Iterator<Item = PaneSnapshot> + '_ {
        let focused = self.state.focused_pane();
        let highlight = self.state.focus_highlight();
        self.state
            .panes_geometric_order()
            .iter()
            .map(move |&(id, rect)| PaneSnapshot {
                id,
                rect,
                is_focused: highlight && focused == Some(id),
            })
    }

    /// Splits the focused pane with the current [`SplitPolicy`].
    pub fn split_focused(&mut self, direction: Direction) -> Result<PaneId, StateError> {
        let pane_id = self.state.allocate_pane_id();
        self.state
            .split_with_ratio(direction, pane_id, self.split_policy.ratio())?;
        Ok(pane_id)
    }

    /// Sibling gets promoted into the parent's slot.
    pub fn close_focused(&mut self) -> Result<PaneId, StateError> {
        self.state.remove_focused()
    }

    /// Sets the ratio on the parent split, not the pane.
    pub fn set_focused_ratio(&mut self, ratio: f32) -> Result<(), StateError> {
        self.state.set_focused_ratio(ratio).map(|_| ())
    }

    /// Sets the ratio of the split at `split_path`.
    ///
    /// The path uses the same format as [`pane_path`](Self::pane_path). The
    /// ratio is clamped to the valid range. Returns an error if the path
    /// does not lead to a split node.
    pub fn set_split_ratio(&mut self, split_path: &[usize], ratio: f32) -> Result<(), StateError> {
        self.state.set_split_ratio(split_path, ratio).map(|_| ())
    }

    /// Like [`set_split_ratio`](Self::set_split_ratio) but reports whether
    /// the ratio actually changed.
    pub fn try_set_split_ratio(
        &mut self,
        split_path: &[usize],
        ratio: f32,
    ) -> Result<bool, StateError> {
        self.state.set_split_ratio(split_path, ratio)
    }

    /// Swaps the positions of two panes in the tree.
    ///
    /// Focus follows the pane id, so a focused pane stays focused in its
    /// new position. Swapping a pane with itself does nothing.
    pub fn swap_panes(&mut self, first: PaneId, second: PaneId) -> Result<(), StateError> {
        self.state.swap_panes(first, second).map(|_| ())
    }

    /// Like [`swap_panes`](Self::swap_panes) but reports whether anything
    /// changed (`false` when `first == second`).
    pub fn try_swap_panes(&mut self, first: PaneId, second: PaneId) -> Result<bool, StateError> {
        self.state.swap_panes(first, second)
    }

    /// Like [`try_apply_action`](Self::try_apply_action) but returns `Ignored` on error.
    pub fn apply_action(&mut self, action: HypertileAction) -> EventOutcome {
        self.try_apply_action(action)
            .unwrap_or(EventOutcome::Ignored)
    }

    /// `Consumed` when state changed, `Ignored` when valid but no effect.
    pub fn try_apply_action(
        &mut self,
        action: HypertileAction,
    ) -> Result<EventOutcome, StateError> {
        let changed = match action {
            HypertileAction::FocusNext => self.state.focus_next(),
            HypertileAction::FocusPrev => self.state.focus_prev(),
            HypertileAction::FocusDirection { direction, towards } => {
                self.state.focus_direction(direction, towards)?
            }
            HypertileAction::SplitFocused { direction } => {
                self.split_focused(direction)?;
                true
            }
            HypertileAction::CloseFocused => {
                self.close_focused()?;
                true
            }
            HypertileAction::ResizeFocused { delta } => self.state.resize_focused(delta)?,
            HypertileAction::SetFocusedRatio { ratio } => self.state.set_focused_ratio(ratio)?,
            HypertileAction::MoveFocused {
                direction,
                towards,
                scope,
            } => match scope {
                MoveScope::Split => self.state.move_pane_split(direction, towards)?,
                MoveScope::Window => self.state.move_pane_window(direction, towards)?,
            },
        };
        Ok(if changed {
            EventOutcome::Consumed
        } else {
            EventOutcome::Ignored
        })
    }
    /// Handles one event at the core-engine level.
    /// Only [`HypertileEvent::Action`] is interpreted here; key and tick events are ignored.
    pub fn try_handle_event(&mut self, event: HypertileEvent) -> Result<EventOutcome, StateError> {
        match event {
            HypertileEvent::Action(action) => self.try_apply_action(action),
            HypertileEvent::Key(_) | HypertileEvent::Mouse(_) | HypertileEvent::Tick => {
                Ok(EventOutcome::Ignored)
            }
        }
    }

    /// Like [`try_handle_event`](Self::try_handle_event), but returns
    /// [`EventOutcome::Ignored`] on error.
    pub fn handle_event(&mut self, event: HypertileEvent) -> EventOutcome {
        self.try_handle_event(event)
            .unwrap_or(EventOutcome::Ignored)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::input::{MoveScope, Towards};

    #[test]
    fn split_policy_golden_sets_non_half_ratio() {
        let mut hypertile = Hypertile::builder()
            .with_split_policy(SplitPolicy::Golden)
            .build();

        let _ = hypertile.split_focused(Direction::Horizontal).unwrap();
        match hypertile.root() {
            Node::Split { ratio, .. } => assert!((*ratio - 0.618).abs() < 0.001),
            Node::Pane(_) => panic!("root should be split"),
        }
    }

    #[test]
    fn window_move_without_layout_is_ignored_by_apply_action() {
        let mut hypertile = Hypertile::new();
        let _ = hypertile.split_focused(Direction::Horizontal).unwrap();

        assert_eq!(
            hypertile.apply_action(HypertileAction::MoveFocused {
                direction: Direction::Horizontal,
                towards: Towards::Start,
                scope: MoveScope::Window,
            }),
            EventOutcome::Ignored
        );

        assert_eq!(
            hypertile.try_apply_action(HypertileAction::MoveFocused {
                direction: Direction::Horizontal,
                towards: Towards::Start,
                scope: MoveScope::Window,
            }),
            Err(StateError::LayoutUnavailable)
        );
    }

    #[test]
    fn pane_snapshot_at_reports_rect_and_focus() {
        let mut hypertile = Hypertile::new();
        let right = hypertile.split_focused(Direction::Horizontal).unwrap();

        assert_eq!(hypertile.pane_snapshot_at(75, 5), None);

        hypertile.compute_layout(Rect::new(0, 0, 100, 20));

        let focused = hypertile.pane_snapshot_at(75, 5).unwrap();
        assert_eq!(focused.id, right);
        assert_eq!(focused.rect, Rect::new(50, 0, 50, 20));
        assert!(focused.is_focused);

        let unfocused = hypertile.pane_snapshot_at(10, 5).unwrap();
        assert_eq!(unfocused.id, PaneId::ROOT);
        assert!(!unfocused.is_focused);

        assert_eq!(hypertile.pane_snapshot_at(150, 5), None);
    }
}
