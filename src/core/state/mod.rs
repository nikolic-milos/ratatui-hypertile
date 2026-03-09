mod focus;
mod movement;
mod mutation;
#[cfg(test)]
mod tests;

use crate::core::helpers::{
    collect_pane_ids as collect_pane_ids_impl, compute_recursive, find_pane_path,
    leftmost_leaf_path, max_pane_id, normalize_tree, validate_unique_pane_ids,
};
use crate::core::{Node, PaneId, StateError};
use ratatui::layout::Rect;
use std::collections::BTreeMap;

/// Mutable tree state.
///
/// Prefer using [`Hypertile`](crate::Hypertile) instead of this directly.
#[derive(Debug, Clone)]
pub struct HypertileState {
    root: Node,
    focused_path: Vec<usize>,
    layout_cache: BTreeMap<PaneId, Rect>,
    sorted_panes: Vec<(PaneId, Rect)>,
    dirty: bool,
    last_area: Option<Rect>,
    highlight_focus: bool,
    next_pane_id: u64,
    gap: u16,
}

impl Default for HypertileState {
    fn default() -> Self {
        Self::new()
    }
}

impl HypertileState {
    pub fn new() -> Self {
        Self {
            root: Node::Pane(PaneId::ROOT),
            focused_path: vec![],
            layout_cache: BTreeMap::new(),
            sorted_panes: Vec::new(),
            dirty: true,
            last_area: None,
            highlight_focus: true,
            next_pane_id: PaneId::ROOT.get() + 1,
            gap: 0,
        }
    }

    /// Replaces the tree and resets focus to the leftmost leaf.
    pub fn set_root(&mut self, root: Node) -> Result<(), StateError> {
        let normalized = normalize_tree(root);
        validate_unique_pane_ids(&normalized)?;
        self.root = normalized;
        self.focused_path = leftmost_leaf_path(&self.root);
        self.next_pane_id = max_pane_id(&self.root).saturating_add(1);
        self.invalidate_layout_cache();
        Ok(())
    }

    pub fn allocate_pane_id(&mut self) -> PaneId {
        let id = PaneId::new(self.next_pane_id);
        self.next_pane_id = self.next_pane_id.saturating_add(1);
        id
    }

    pub fn root(&self) -> &Node {
        &self.root
    }

    pub fn pane_ids(&self) -> Vec<PaneId> {
        collect_pane_ids_impl(&self.root)
    }

    pub fn set_focus_highlight(&mut self, enabled: bool) {
        self.highlight_focus = enabled;
    }

    pub fn focus_highlight(&self) -> bool {
        self.highlight_focus
    }

    pub fn compute_layout(&mut self, area: Rect) {
        self.sync_focus_path();

        if !self.dirty && self.last_area == Some(area) {
            return;
        }

        self.layout_cache.clear();
        compute_recursive(&self.root, area, &mut self.layout_cache, self.gap);

        self.sorted_panes.clear();
        self.sorted_panes
            .extend(self.layout_cache.iter().map(|(id, rect)| (*id, *rect)));
        self.sorted_panes
            .sort_unstable_by(|(id_a, ra), (id_b, rb)| {
                (ra.y, ra.x, *id_a).cmp(&(rb.y, rb.x, *id_b))
            });

        self.last_area = Some(area);
        self.dirty = false;
    }

    pub(super) fn invalidate_layout_cache(&mut self) {
        self.dirty = true;
        self.layout_cache.clear();
        self.sorted_panes.clear();
    }

    pub fn pane_rect(&self, id: PaneId) -> Option<Rect> {
        self.layout_cache.get(&id).copied()
    }

    pub fn panes(&self) -> impl Iterator<Item = (PaneId, Rect)> + '_ {
        self.layout_cache.iter().map(|(id, rect)| (*id, *rect))
    }

    pub fn panes_geometric_order(&self) -> &[(PaneId, Rect)] {
        &self.sorted_panes
    }
    pub fn pane_path(&self, pane_id: PaneId) -> Option<Vec<usize>> {
        find_pane_path(&self.root, pane_id)
    }

    pub fn walk_preorder<F>(&self, mut visit: F)
    where
        F: FnMut(&[usize], &Node),
    {
        fn walk<F>(node: &Node, path: &mut Vec<usize>, visit: &mut F)
        where
            F: FnMut(&[usize], &Node),
        {
            visit(path.as_slice(), node);
            if let Node::Split { first, second, .. } = node {
                path.push(0);
                walk(first, path, visit);
                path.pop();

                path.push(1);
                walk(second, path, visit);
                path.pop();
            }
        }

        let mut path = Vec::new();
        walk(&self.root, &mut path, &mut visit);
    }

    pub fn gap(&self) -> u16 {
        self.gap
    }

    pub fn set_gap(&mut self, gap: u16) {
        if self.gap != gap {
            self.gap = gap;
            self.invalidate_layout_cache();
        }
    }

    pub fn reset(&mut self) {
        *self = Self::new();
    }
}

pub fn collect_pane_ids(node: &Node) -> Vec<PaneId> {
    collect_pane_ids_impl(node)
}
