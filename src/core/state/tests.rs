use super::*;
use crate::core::Node;
use crate::input::Towards;
use ratatui::layout::{Direction, Rect};

fn area(width: u16, height: u16) -> Rect {
    Rect::new(0, 0, width, height)
}

#[test]
fn split_updates_focus_to_new_pane() {
    let mut state = HypertileState::new();
    let new_id = state.allocate_pane_id();
    state.split(Direction::Horizontal, new_id).unwrap();

    assert_eq!(state.focused_pane(), Some(new_id));
    assert_eq!(state.focused_path, vec![1]);
}

#[test]
fn remove_focused_promotes_sibling() {
    let mut state = HypertileState::new();
    let id1 = state.allocate_pane_id();
    let id2 = state.allocate_pane_id();

    state.split(Direction::Horizontal, id1).unwrap();
    state.split(Direction::Vertical, id2).unwrap();
    state.focused_path = vec![1, 0];

    assert_eq!(state.remove_focused().unwrap(), id1);
    assert_eq!(state.focused_pane(), Some(id2));
}

#[test]
fn remove_root_returns_error() {
    let mut state = HypertileState::new();
    let error = state.remove_focused().unwrap_err();
    assert_eq!(error, StateError::CannotRemoveRootPane);
}

#[test]
fn ratio_updates_clamp_and_set_absolute() {
    let mut state = HypertileState::new();
    let id1 = state.allocate_pane_id();
    state.split(Direction::Horizontal, id1).unwrap();

    let mut changed = false;
    for _ in 0..100 {
        changed |= state.resize_focused(-0.1).unwrap();
    }
    assert!(changed);

    if let Node::Split { ratio, .. } = &state.root {
        assert!((*ratio - 0.9).abs() < 0.001);
    } else {
        panic!("root should remain split");
    }

    changed = false;
    for _ in 0..100 {
        changed |= state.resize_focused(0.1).unwrap();
    }
    assert!(changed);

    if let Node::Split { ratio, .. } = &state.root {
        assert!((*ratio - 0.1).abs() < 0.001);
    } else {
        panic!("root should remain split");
    }

    assert!(state.set_focused_ratio(0.3).unwrap());

    if let Node::Split { ratio, .. } = &state.root {
        assert!((*ratio - 0.3).abs() < 0.001);
    } else {
        panic!("root should remain split");
    }
}

#[test]
fn layout_recomputes_when_area_changes() {
    let mut state = HypertileState::new();
    let id1 = state.allocate_pane_id();
    state.split(Direction::Horizontal, id1).unwrap();

    state.compute_layout(area(100, 20));
    let original = state.pane_rect(id1).unwrap();

    state.compute_layout(area(200, 20));
    let resized = state.pane_rect(id1).unwrap();

    assert_ne!(original.width, resized.width);
}

#[test]
fn move_pane_swaps_with_matching_axis() {
    let mut state = HypertileState::new();
    let right = state.allocate_pane_id();
    state.split(Direction::Horizontal, right).unwrap();

    assert!(
        state
            .move_pane_split(Direction::Horizontal, Towards::Start)
            .unwrap()
    );
    assert_eq!(state.focused_path, vec![0]);
    assert_eq!(state.focused_pane(), Some(right));
}

#[test]
fn directional_focus_moves_to_nearest_pane() {
    let mut state = HypertileState::new();
    let right = state.allocate_pane_id();
    let bottom_right = state.allocate_pane_id();

    state.split(Direction::Horizontal, right).unwrap();
    state.split(Direction::Vertical, bottom_right).unwrap();
    state.compute_layout(area(120, 40));

    assert_eq!(state.focused_pane(), Some(bottom_right));

    assert!(
        state
            .focus_direction(Direction::Horizontal, Towards::Start)
            .unwrap()
    );
    assert_eq!(state.focused_pane(), Some(PaneId::ROOT));

    assert!(
        state
            .focus_direction(Direction::Horizontal, Towards::End)
            .unwrap()
    );
    assert_eq!(state.focused_pane(), Some(right));

    assert!(
        state
            .focus_direction(Direction::Vertical, Towards::End)
            .unwrap()
    );
    assert_eq!(state.focused_pane(), Some(bottom_right));
}

#[test]
fn focus_next_cycles_through_all_panes() {
    let mut state = HypertileState::new();
    let id1 = state.allocate_pane_id();
    let id2 = state.allocate_pane_id();
    state.split(Direction::Horizontal, id1).unwrap();
    state.split(Direction::Vertical, id2).unwrap();
    state.compute_layout(area(120, 40));

    let start = state.focused_pane().unwrap();
    let mut visited = vec![start];
    for _ in 0..10 {
        assert!(state.focus_next());
        let current = state.focused_pane().unwrap();
        if current == start {
            break;
        }
        visited.push(current);
    }

    assert_eq!(visited.len(), 3);
    assert!(visited.contains(&PaneId::ROOT));
    assert!(visited.contains(&id1));
    assert!(visited.contains(&id2));
}

#[test]
fn set_root_resets_focus_and_rejects_duplicates() {
    let mut state = HypertileState::new();
    let tree = Node::Split {
        direction: Direction::Horizontal,
        ratio: 0.5,
        first: Box::new(Node::Pane(PaneId::new(10))),
        second: Box::new(Node::Pane(PaneId::new(20))),
    };

    state.set_root(tree).unwrap();
    assert_eq!(state.focused_path, vec![0]);
    assert_eq!(state.focused_pane(), Some(PaneId::new(10)));

    let next = state.allocate_pane_id();
    assert!(next.get() > 20);
    let dup = Node::Split {
        direction: Direction::Horizontal,
        ratio: 0.5,
        first: Box::new(Node::Pane(PaneId::new(1))),
        second: Box::new(Node::Pane(PaneId::new(1))),
    };

    let err = state.set_root(dup).unwrap_err();
    assert_eq!(err, StateError::DuplicatePaneId(PaneId::new(1)));

    assert_eq!(state.focused_pane(), Some(PaneId::new(10)));
    state.compute_layout(area(100, 50));
    assert_eq!(
        state.pane_ids().collect::<Vec<_>>(),
        vec![PaneId::new(10), PaneId::new(20)]
    );
}

#[test]
fn gap_shrinks_leaf_pane_rects() {
    let mut state = HypertileState::new();
    let id1 = state.allocate_pane_id();
    state.split(Direction::Horizontal, id1).unwrap();

    state.set_gap(0);
    state.compute_layout(area(100, 50));
    let rect_no_gap = state.pane_rect(PaneId::ROOT).unwrap();

    state.set_gap(4);
    state.compute_layout(area(100, 50));
    let rect_with_gap = state.pane_rect(PaneId::ROOT).unwrap();

    assert!(rect_with_gap.width < rect_no_gap.width);
    assert!(rect_with_gap.height < rect_no_gap.height);
    assert_eq!(rect_with_gap.width, rect_no_gap.width - 4);
    assert_eq!(rect_with_gap.height, rect_no_gap.height - 4);
}
