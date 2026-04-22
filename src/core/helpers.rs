use crate::core::{Node, PaneId, StateError};
use ratatui::layout::Rect;
use std::collections::HashSet;

pub(super) const MIN_SPLIT_RATIO: f32 = 0.1;
pub(super) const MAX_SPLIT_RATIO: f32 = 0.9;

pub(super) fn validate_unique_pane_ids(node: &Node) -> Result<(), StateError> {
    fn walk(node: &Node, seen: &mut HashSet<PaneId>) -> Result<(), StateError> {
        match node {
            Node::Pane(id) => {
                if !seen.insert(*id) {
                    return Err(StateError::DuplicatePaneId(*id));
                }
            }
            Node::Split { first, second, .. } => {
                walk(first, seen)?;
                walk(second, seen)?;
            }
        }

        Ok(())
    }

    let mut seen = HashSet::new();
    walk(node, &mut seen)
}

pub(crate) fn normalize_ratio(ratio: f32) -> f32 {
    if !ratio.is_finite() {
        return 0.5;
    }

    ratio.clamp(MIN_SPLIT_RATIO, MAX_SPLIT_RATIO)
}

pub(super) fn normalize_tree(node: Node) -> Node {
    match node {
        Node::Pane(id) => Node::Pane(id),
        Node::Split {
            direction,
            ratio,
            first,
            second,
        } => Node::Split {
            direction,
            ratio: normalize_ratio(ratio),
            first: Box::new(normalize_tree(*first)),
            second: Box::new(normalize_tree(*second)),
        },
    }
}

pub(super) fn max_pane_id(node: &Node) -> u64 {
    match node {
        Node::Pane(id) => id.get(),
        Node::Split { first, second, .. } => max_pane_id(first).max(max_pane_id(second)),
    }
}

pub(super) fn shrink_rect(rect: Rect, gap: u16) -> Rect {
    let half = gap / 2;
    let x = rect.x.saturating_add(half);
    let y = rect.y.saturating_add(half);
    let w = rect.width.saturating_sub(gap);
    let h = rect.height.saturating_sub(gap);
    if w == 0 || h == 0 {
        rect
    } else {
        Rect::new(x, y, w, h)
    }
}

pub(super) fn compute_recursive(
    node: &Node,
    area: Rect,
    cache: &mut Vec<(PaneId, Rect)>,
    gap: u16,
) {
    match node {
        Node::Pane(id) => {
            let rect = if gap > 0 {
                shrink_rect(area, gap)
            } else {
                area
            };
            cache.push((*id, rect));
        }
        Node::Split {
            direction,
            ratio,
            first,
            second,
        } => {
            let ratio = normalize_ratio(*ratio);
            let (first_area, second_area) = split_rect(area, *direction, ratio);
            compute_recursive(first, first_area, cache, gap);
            compute_recursive(second, second_area, cache, gap);
        }
    }
}

fn split_rect(area: Rect, direction: ratatui::layout::Direction, ratio: f32) -> (Rect, Rect) {
    use ratatui::layout::Direction;
    match direction {
        Direction::Horizontal => {
            let first_w = (area.width as f32 * ratio).round() as u16;
            let second_w = area.width.saturating_sub(first_w);
            (
                Rect::new(area.x, area.y, first_w, area.height),
                Rect::new(
                    area.x.saturating_add(first_w),
                    area.y,
                    second_w,
                    area.height,
                ),
            )
        }
        Direction::Vertical => {
            let first_h = (area.height as f32 * ratio).round() as u16;
            let second_h = area.height.saturating_sub(first_h);
            (
                Rect::new(area.x, area.y, area.width, first_h),
                Rect::new(area.x, area.y.saturating_add(first_h), area.width, second_h),
            )
        }
    }
}

pub(super) fn rect_center(rect: Rect) -> (i32, i32) {
    (
        rect.x as i32 + rect.width as i32 / 2,
        rect.y as i32 + rect.height as i32 / 2,
    )
}

pub(super) fn ranges_overlap(start_a: u16, len_a: u16, start_b: u16, len_b: u16) -> bool {
    let end_a = start_a.saturating_add(len_a);
    let end_b = start_b.saturating_add(len_b);
    start_a < end_b && start_b < end_a
}

pub(super) fn collect_pane_ids(node: &Node) -> Vec<PaneId> {
    let mut ids = Vec::new();
    fn walk(node: &Node, ids: &mut Vec<PaneId>) {
        match node {
            Node::Pane(id) => ids.push(*id),
            Node::Split { first, second, .. } => {
                walk(first, ids);
                walk(second, ids);
            }
        }
    }
    walk(node, &mut ids);
    ids
}

pub(super) fn leftmost_leaf_id(node: &Node) -> Option<PaneId> {
    let mut current = node;
    loop {
        match current {
            Node::Pane(id) => return Some(*id),
            Node::Split { first, .. } => {
                current = first;
            }
        }
    }
}

pub(super) fn leftmost_leaf_path(node: &Node) -> Vec<usize> {
    let mut path = Vec::new();
    let mut current = node;

    while let Node::Split { first, .. } = current {
        path.push(0);
        current = first;
    }

    path
}

pub(super) fn node_at_path<'a>(node: &'a Node, path: &[usize]) -> Option<&'a Node> {
    let mut current = node;
    for &idx in path {
        let Node::Split { first, second, .. } = current else {
            return None;
        };

        current = if idx == 0 { first } else { second };
    }
    Some(current)
}

pub(super) fn node_mut_at_path<'a>(
    node: &'a mut Node,
    path: &[usize],
) -> Result<&'a mut Node, StateError> {
    let mut current = node;
    for &idx in path {
        let Node::Split { first, second, .. } = current else {
            return Err(StateError::InvalidPath);
        };

        current = if idx == 0 { first } else { second };
    }

    Ok(current)
}
