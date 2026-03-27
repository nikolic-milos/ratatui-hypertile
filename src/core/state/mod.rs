mod focus;
mod movement;
mod mutation;
#[cfg(test)]
mod tests;

use crate::core::helpers::{
    collect_pane_ids as collect_pane_ids_impl, compute_recursive, leftmost_leaf_path, max_pane_id,
    normalize_tree, validate_unique_pane_ids,
};
use crate::core::{Node, PaneId, StateError};
use ratatui::layout::Rect;
use std::collections::HashMap;

/// Mutable tree state.
///
/// Prefer using [`Hypertile`](crate::Hypertile) instead of this directly.
#[derive(Debug, Clone)]
pub struct HypertileState {
    root: Node,
    focused_path: Vec<usize>,
    pane_paths: HashMap<PaneId, Vec<usize>>,
    pane_ids_preorder: Vec<PaneId>,
    layout_cache: Vec<(PaneId, Rect)>,
    sorted_panes: Vec<(PaneId, Rect)>,
    sorted_pane_index: HashMap<PaneId, usize>,
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
        let mut pane_paths = HashMap::new();
        pane_paths.insert(PaneId::ROOT, Vec::new());

        Self {
            root: Node::Pane(PaneId::ROOT),
            focused_path: vec![],
            pane_paths,
            pane_ids_preorder: vec![PaneId::ROOT],
            layout_cache: Vec::new(),
            sorted_panes: Vec::new(),
            sorted_pane_index: HashMap::new(),
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
        self.rebuild_pane_index();
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

    /// Empty until you call [`compute_layout`](Self::compute_layout).
    pub fn pane_ids(&self) -> impl Iterator<Item = PaneId> + '_ {
        self.layout_cache.iter().map(|(id, _)| *id)
    }

    pub fn set_focus_highlight(&mut self, enabled: bool) {
        self.highlight_focus = enabled;
    }

    pub fn focus_highlight(&self) -> bool {
        self.highlight_focus
    }

    /// Skips if the area and tree are unchanged.
    pub fn compute_layout(&mut self, area: Rect) {
        if !self.dirty && self.last_area == Some(area) {
            return;
        }

        self.layout_cache.clear();
        compute_recursive(&self.root, area, &mut self.layout_cache, self.gap);
        self.layout_cache.sort_unstable_by_key(|(id, _)| *id);

        self.sorted_panes.clear();
        self.sorted_panes.extend(self.layout_cache.iter().copied());
        self.sorted_panes
            .sort_unstable_by(|(id_a, ra), (id_b, rb)| {
                (ra.y, ra.x, *id_a).cmp(&(rb.y, rb.x, *id_b))
            });
        self.sorted_pane_index.clear();

        self.last_area = Some(area);
        self.dirty = false;
    }

    pub(super) fn invalidate_layout_cache(&mut self) {
        self.dirty = true;
        self.layout_cache.clear();
        self.sorted_panes.clear();
        self.sorted_pane_index.clear();
    }

    pub fn pane_rect(&self, id: PaneId) -> Option<Rect> {
        self.layout_cache
            .binary_search_by_key(&id, |(pane_id, _)| *pane_id)
            .ok()
            .map(|idx| self.layout_cache[idx].1)
    }

    pub fn panes(&self) -> impl Iterator<Item = (PaneId, Rect)> + '_ {
        self.layout_cache.iter().copied()
    }

    /// Sorted top to bottom, left to right.
    pub fn panes_geometric_order(&self) -> &[(PaneId, Rect)] {
        &self.sorted_panes
    }

    /// `0` means first child, `1` means second.
    pub fn pane_path(&self, pane_id: PaneId) -> Option<Vec<usize>> {
        self.pane_paths.get(&pane_id).cloned()
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

    /// Invalidates the layout cache if the value changed.
    pub fn set_gap(&mut self, gap: u16) {
        if self.gap != gap {
            self.gap = gap;
            self.invalidate_layout_cache();
        }
    }

    pub fn reset(&mut self) {
        *self = Self::new();
    }

    pub(super) fn pane_path_cached(&self, pane_id: PaneId) -> Option<&[usize]> {
        self.pane_paths.get(&pane_id).map(Vec::as_slice)
    }

    pub(super) fn focus_path_for(&mut self, pane_id: PaneId) -> bool {
        let Some(path) = self.pane_paths.get(&pane_id).cloned() else {
            return false;
        };

        if path == self.focused_path {
            return false;
        }

        self.focused_path = path;
        true
    }

    fn rebuild_pane_index(&mut self) {
        fn walk(
            node: &Node,
            path: &mut Vec<usize>,
            pane_paths: &mut HashMap<PaneId, Vec<usize>>,
            pane_ids_preorder: &mut Vec<PaneId>,
        ) {
            match node {
                Node::Pane(id) => {
                    pane_paths.insert(*id, path.clone());
                    pane_ids_preorder.push(*id);
                }
                Node::Split { first, second, .. } => {
                    path.push(0);
                    walk(first, path, pane_paths, pane_ids_preorder);
                    path.pop();

                    path.push(1);
                    walk(second, path, pane_paths, pane_ids_preorder);
                    path.pop();
                }
            }
        }

        self.pane_paths.clear();
        self.pane_ids_preorder.clear();

        let mut path = Vec::new();
        walk(
            &self.root,
            &mut path,
            &mut self.pane_paths,
            &mut self.pane_ids_preorder,
        );
    }
}

/// Doesn't need layout, walks the tree directly.
pub fn collect_pane_ids(node: &Node) -> Vec<PaneId> {
    collect_pane_ids_impl(node)
}
