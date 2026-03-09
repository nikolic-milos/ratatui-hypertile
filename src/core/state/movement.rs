use crate::core::helpers::{node_at_path, node_mut_at_path};
use crate::core::{Node, StateError};
use crate::input::Towards;
use ratatui::layout::Direction;

use super::HypertileState;

impl HypertileState {
    /// Swaps the focused pane with its sibling in the nearest split on `move_dir`.
    ///
    /// Returns `Ok(true)` if a swap happened, `Ok(false)` if there was no
    /// matching split, and [`StateError::ParentNodeNotSplit`] if the parent node is not a split.
    pub fn move_pane_split(
        &mut self,
        move_dir: Direction,
        towards: Towards,
    ) -> Result<bool, StateError> {
        if self.focused_path.is_empty() {
            return Ok(false);
        }

        let mut target: Option<usize> = None;
        for i in (0..self.focused_path.len()).rev() {
            let child_idx = self.focused_path[i];

            let is_match = matches!(
                node_at_path(&self.root, &self.focused_path[..i]),
                Some(Node::Split { direction, .. }) if *direction == move_dir
            );

            if !is_match {
                continue;
            }

            if (towards == Towards::End && child_idx == 0)
                || (towards == Towards::Start && child_idx == 1)
            {
                target = Some(i);
                break;
            }
        }

        if let Some(target_i) = target {
            let parent = node_mut_at_path(&mut self.root, &self.focused_path[..target_i])?;
            let Node::Split { first, second, .. } = parent else {
                return Err(StateError::ParentNodeNotSplit);
            };

            std::mem::swap(first, second);
            self.focused_path[target_i] = 1 - self.focused_path[target_i];
            self.invalidate_layout_cache();
            return Ok(true);
        }

        Ok(false)
    }

    /// Swaps the focused pane with the nearest pane in the requested direction.
    ///
    /// Returns `Ok(true)` if a swap happened, `Ok(false)` if there was nowhere
    /// to move, and [`StateError::LayoutUnavailable`] if layout data is missing.
    pub fn move_pane_window(
        &mut self,
        move_dir: Direction,
        towards: Towards,
    ) -> Result<bool, StateError> {
        let Some(focused_id) = self.focused_pane() else {
            return Ok(false);
        };
        let Some(focused_rect) = self.layout_cache.get(&focused_id).copied() else {
            return Err(StateError::LayoutUnavailable);
        };

        let Some(target_id) =
            self.best_directional_target(focused_id, focused_rect, move_dir, towards)
        else {
            return Ok(false);
        };

        if target_id == focused_id {
            return Ok(false);
        }

        let Some(focused_path) = self.pane_path(focused_id) else {
            return Err(StateError::UnknownPaneId(focused_id));
        };
        let Some(target_path) = self.pane_path(target_id) else {
            return Err(StateError::UnknownPaneId(target_id));
        };

        let focused_node = node_mut_at_path(&mut self.root, &focused_path)?;
        let Node::Pane(focused_leaf_id) = focused_node else {
            return Err(StateError::FocusedNodeNotPane);
        };
        let original_focused_id = *focused_leaf_id;
        *focused_leaf_id = target_id;

        let target_node = node_mut_at_path(&mut self.root, &target_path)?;
        let Node::Pane(target_leaf_id) = target_node else {
            return Err(StateError::FocusedNodeNotPane);
        };
        *target_leaf_id = original_focused_id;

        self.focused_path = target_path;
        self.invalidate_layout_cache();
        Ok(true)
    }
}
