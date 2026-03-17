use crate::core::helpers::{
    collect_pane_ids as collect_pane_ids_impl, leftmost_leaf_id, ranges_overlap, rect_center,
};
use crate::core::{Node, PaneId, StateError};
use crate::input::Towards;
use ratatui::layout::{Direction, Rect};

use super::HypertileState;

impl HypertileState {
    /// Wraps around, top left to bottom right.
    pub fn focus_next(&mut self) -> bool {
        self.cycle_focus(true)
    }

    /// Same as [`focus_next`](Self::focus_next) but backwards.
    pub fn focus_prev(&mut self) -> bool {
        self.cycle_focus(false)
    }

    /// Moves focus to the nearest pane in the requested direction.
    ///
    /// Uses cached pane rectangles from [`Self::compute_layout`] to find the best match.
    ///
    /// Returns `Ok(true)` if focus moved, `Ok(false)` if there was nowhere to go,
    /// and [`StateError::LayoutUnavailable`] if layout data has not been computed.
    pub fn focus_direction(
        &mut self,
        dir: Direction,
        towards: Towards,
    ) -> Result<bool, StateError> {
        let Some(focused_id) = self.focused_pane() else {
            return Ok(false);
        };
        let Some(focused_rect) = self.layout_cache.get(&focused_id).copied() else {
            return Err(StateError::LayoutUnavailable);
        };

        let Some(target_id) = self.best_directional_target(focused_id, focused_rect, dir, towards)
        else {
            return Ok(false);
        };
        let Some(path) = self.pane_path(target_id) else {
            return Ok(false);
        };
        if path == self.focused_path {
            return Ok(false);
        }

        self.focused_path = path;
        Ok(true)
    }

    /// Focuses the pane with the given id.
    ///
    /// Returns [`StateError::UnknownPaneId`] if the pane is not in the tree.
    pub fn focus_pane(&mut self, pane_id: PaneId) -> Result<(), StateError> {
        let Some(path) = self.pane_path(pane_id) else {
            return Err(StateError::UnknownPaneId(pane_id));
        };
        self.focused_path = path;
        Ok(())
    }

    /// Returns the currently focused pane id.
    ///
    /// If `focused_path` is stale, falls back to the leftmost leaf under the deepest valid node.
    #[must_use]
    pub fn focused_pane(&self) -> Option<PaneId> {
        let mut current = &self.root;

        for &idx in &self.focused_path {
            let Node::Split { first, second, .. } = current else {
                break;
            };
            current = if idx == 0 { first } else { second };
        }

        leftmost_leaf_id(current)
    }

    /// Rebuilds `focused_path` to match the current tree.
    ///
    /// Safe to call even if the current focus does not resolve to a pane.
    pub fn sync_focus_path(&mut self) {
        if let Some(id) = self.focused_pane()
            && let Some(correct_path) = self.pane_path(id)
            && correct_path != self.focused_path
        {
            self.focused_path = correct_path;
        }
    }

    fn cycle_focus(&mut self, forward: bool) -> bool {
        let Some(focused_id) = self.focused_pane() else {
            return false;
        };

        let next_id = if !self.sorted_panes.is_empty() {
            Self::cycle_in_sorted(&self.sorted_panes, focused_id, forward)
        } else {
            let ids = collect_pane_ids_impl(&self.root);
            Self::cycle_in_ids(&ids, focused_id, forward)
        };

        let Some(next_id) = next_id else {
            return false;
        };

        if let Some(path) = self.pane_path(next_id) {
            self.focused_path = path;
            return true;
        }

        false
    }

    fn cycle_in_sorted(
        sorted: &[(PaneId, Rect)],
        focused: PaneId,
        forward: bool,
    ) -> Option<PaneId> {
        let len = sorted.len();
        if len == 0 {
            return None;
        }
        let idx = sorted
            .iter()
            .position(|(id, _)| *id == focused)
            .unwrap_or(0);
        let next = if forward {
            (idx + 1) % len
        } else {
            (idx + len - 1) % len
        };
        let next_id = sorted[next].0;
        if next_id == focused {
            None
        } else {
            Some(next_id)
        }
    }

    fn cycle_in_ids(ids: &[PaneId], focused: PaneId, forward: bool) -> Option<PaneId> {
        let len = ids.len();
        if len == 0 {
            return None;
        }
        let idx = ids.iter().position(|id| *id == focused).unwrap_or(0);
        let next = if forward {
            (idx + 1) % len
        } else {
            (idx + len - 1) % len
        };
        let next_id = ids[next];
        if next_id == focused {
            None
        } else {
            Some(next_id)
        }
    }

    /// Picks the pane to focus when moving in one direction.
    ///
    /// Considers every cached pane except the current one, filters to panes that
    /// are actually in the requested direction, and picks the closest match.
    ///
    /// Overlap on the perpendicular axis is preferred. If nothing overlaps, the
    /// nearest non-overlapping pane wins.
    pub(super) fn best_directional_target(
        &self,
        focused_id: PaneId,
        focused_rect: ratatui::layout::Rect,
        dir: Direction,
        towards: Towards,
    ) -> Option<PaneId> {
        struct DirectionalCandidate {
            pane_id: PaneId,
            primary_dist: i32,
            secondary_dist: i32,
        }

        impl DirectionalCandidate {
            fn is_closer_than(&self, other: &Self) -> bool {
                (self.primary_dist, self.secondary_dist)
                    < (other.primary_dist, other.secondary_dist)
            }
        }

        let focused_center = rect_center(focused_rect);

        let directional_metrics =
            |center: (i32, i32), rect: ratatui::layout::Rect| -> Option<(i32, i32, bool)> {
                let (primary, secondary, overlaps) = match dir {
                    Direction::Horizontal => (
                        center.0 - focused_center.0,
                        (center.1 - focused_center.1).abs(),
                        ranges_overlap(focused_rect.y, focused_rect.height, rect.y, rect.height),
                    ),
                    Direction::Vertical => (
                        center.1 - focused_center.1,
                        (center.0 - focused_center.0).abs(),
                        ranges_overlap(focused_rect.x, focused_rect.width, rect.x, rect.width),
                    ),
                };

                let in_direction = match towards {
                    Towards::End => primary > 0,
                    Towards::Start => primary < 0,
                };

                if in_direction {
                    Some((primary.abs(), secondary, overlaps))
                } else {
                    None
                }
            };

        let mut best_overlap: Option<DirectionalCandidate> = None;
        let mut best_any: Option<DirectionalCandidate> = None;

        for (id, rect) in self.layout_cache.iter().map(|(id, rect)| (*id, *rect)) {
            if id == focused_id {
                continue;
            }

            let center = rect_center(rect);
            let Some((primary_dist, secondary_dist, overlaps_axis)) =
                directional_metrics(center, rect)
            else {
                continue;
            };

            let candidate = DirectionalCandidate {
                pane_id: id,
                primary_dist,
                secondary_dist,
            };

            let bucket = if overlaps_axis {
                &mut best_overlap
            } else {
                &mut best_any
            };

            if bucket
                .as_ref()
                .is_none_or(|current| candidate.is_closer_than(current))
            {
                *bucket = Some(candidate);
            }
        }

        best_overlap.or(best_any).map(|c| c.pane_id)
    }
}
