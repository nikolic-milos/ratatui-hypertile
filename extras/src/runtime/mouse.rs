use crate::runtime::{HypertileRuntime, InputMode, RuntimeError};
use ratatui::layout::{Direction, Rect};
use ratatui_hypertile::{
    EventOutcome, HypertileEvent, MouseButton, MouseEvent, MouseEventKind, PaneId,
};
use std::time::Instant;

const SPLIT_HIT_TOLERANCE: u16 = 1;
const MOVE_DRAG_THRESHOLD: u16 = 2;

#[derive(Debug, Default)]
pub(super) enum MouseDragState {
    #[default]
    None,
    Resize {
        split_path: Vec<usize>,
        direction: Direction,
        rect: Rect,
    },
    Move {
        pane_id: PaneId,
        start_column: u16,
        start_row: u16,
        current_column: u16,
        current_row: u16,
        origin: Rect,
        preview_active: bool,
    },
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct MouseResizeHover {
    pub(super) direction: Direction,
    pub(super) rect: Rect,
    pub(super) ratio: f32,
}

impl MouseDragState {
    pub(super) fn clear(&mut self) {
        *self = Self::None;
    }

    pub(super) fn dragged_pane(&self) -> Option<PaneId> {
        match self {
            Self::Move {
                pane_id,
                preview_active: true,
                ..
            } => Some(*pane_id),
            _ => None,
        }
    }

    pub(super) fn preview_rect(&self) -> Option<Rect> {
        match self {
            Self::Move {
                start_column,
                start_row,
                current_column,
                current_row,
                origin,
                preview_active: true,
                ..
            } => Some(translate_rect(
                *origin,
                i32::from(*current_column) - i32::from(*start_column),
                i32::from(*current_row) - i32::from(*start_row),
            )),
            _ => None,
        }
    }
}

impl HypertileRuntime {
    pub(super) fn handle_mouse_event(
        &mut self,
        mouse: MouseEvent,
    ) -> Result<EventOutcome, RuntimeError> {
        match self.mode {
            InputMode::Layout => self.handle_layout_mouse(mouse),
            InputMode::PluginInput => self.handle_plugin_mouse(mouse),
        }
    }

    fn handle_layout_mouse(&mut self, mouse: MouseEvent) -> Result<EventOutcome, RuntimeError> {
        match mouse.kind {
            MouseEventKind::Down(MouseButton::Left) => self.start_layout_mouse_drag(mouse),
            MouseEventKind::Drag(MouseButton::Left) => self.update_layout_mouse_drag(mouse),
            MouseEventKind::Up(MouseButton::Left) => self.finish_layout_mouse_drag(mouse),
            MouseEventKind::Moved => self.update_layout_mouse_hover(mouse),
            _ => Ok(EventOutcome::Ignored),
        }
    }

    fn start_layout_mouse_drag(&mut self, mouse: MouseEvent) -> Result<EventOutcome, RuntimeError> {
        self.mouse_drag.clear();
        self.mouse_resize_hover = None;

        if let Some(split) = self
            .core
            .split_at(mouse.column, mouse.row, SPLIT_HIT_TOLERANCE)
        {
            self.animation_state.clear();
            self.mouse_drag = MouseDragState::Resize {
                split_path: split.path,
                direction: split.direction,
                rect: split.rect,
            };
            return Ok(EventOutcome::Consumed);
        }

        let Some(pane_id) = self.core.pane_at(mouse.column, mouse.row) else {
            return Ok(EventOutcome::Ignored);
        };
        let Some(origin) = self.core.pane_rect(pane_id) else {
            return Ok(EventOutcome::Ignored);
        };

        self.core.focus_pane(pane_id)?;
        self.animation_state.clear();
        self.mouse_drag = MouseDragState::Move {
            pane_id,
            start_column: mouse.column,
            start_row: mouse.row,
            current_column: mouse.column,
            current_row: mouse.row,
            origin,
            preview_active: false,
        };
        Ok(EventOutcome::Consumed)
    }

    fn update_layout_mouse_drag(
        &mut self,
        mouse: MouseEvent,
    ) -> Result<EventOutcome, RuntimeError> {
        if let MouseDragState::Resize {
            split_path,
            direction,
            rect,
        } = &self.mouse_drag
        {
            let ratio = ratio_from_mouse(*direction, *rect, mouse);
            if self.core.try_set_split_ratio(split_path, ratio)? {
                self.animation_state.clear();
            }
            return Ok(EventOutcome::Consumed);
        }

        self.update_move_drag(mouse)
    }

    fn update_move_drag(&mut self, mouse: MouseEvent) -> Result<EventOutcome, RuntimeError> {
        let MouseDragState::Move {
            start_column,
            start_row,
            current_column,
            current_row,
            preview_active,
            ..
        } = &mut self.mouse_drag
        else {
            return Ok(EventOutcome::Ignored);
        };

        *current_column = mouse.column;
        *current_row = mouse.row;
        if drag_threshold_met(*start_column, *start_row, mouse.column, mouse.row) {
            *preview_active = true;
        }
        Ok(EventOutcome::Consumed)
    }

    fn update_layout_mouse_hover(
        &mut self,
        mouse: MouseEvent,
    ) -> Result<EventOutcome, RuntimeError> {
        let next = self
            .core
            .split_at(mouse.column, mouse.row, SPLIT_HIT_TOLERANCE)
            .map(|split| MouseResizeHover {
                direction: split.direction,
                rect: split.rect,
                ratio: split.ratio,
            });

        if self.mouse_resize_hover == next {
            return Ok(EventOutcome::Ignored);
        }

        self.mouse_resize_hover = next;
        Ok(EventOutcome::Consumed)
    }

    fn finish_layout_mouse_drag(
        &mut self,
        mouse: MouseEvent,
    ) -> Result<EventOutcome, RuntimeError> {
        let (pane_id, start_column, start_row, origin, preview_active) =
            match std::mem::take(&mut self.mouse_drag) {
                MouseDragState::Move {
                    pane_id,
                    start_column,
                    start_row,
                    current_column: _,
                    current_row: _,
                    origin,
                    preview_active,
                } => (pane_id, start_column, start_row, origin, preview_active),
                MouseDragState::Resize { .. } => return Ok(EventOutcome::Consumed),
                MouseDragState::None => return Ok(EventOutcome::Ignored),
            };

        if !preview_active && !drag_threshold_met(start_column, start_row, mouse.column, mouse.row)
        {
            return Ok(EventOutcome::Consumed);
        }

        let preview = translate_rect(
            origin,
            i32::from(mouse.column) - i32::from(start_column),
            i32::from(mouse.row) - i32::from(start_row),
        );
        let Some(target_id) = drop_target(self, pane_id, mouse, preview) else {
            return Ok(EventOutcome::Consumed);
        };
        if target_id == pane_id {
            return Ok(EventOutcome::Consumed);
        }

        let can_animate =
            self.animation_config.enabled && self.animation_state.last_area().is_some();
        let now = Instant::now();
        if can_animate {
            self.capture_displayed_rects(now);
        }

        if !self.core.try_swap_panes(pane_id, target_id)? {
            return Ok(EventOutcome::Consumed);
        }

        if can_animate {
            self.start_action_animation(now);
        } else {
            self.animation_state.clear();
        }
        Ok(EventOutcome::Consumed)
    }

    fn handle_plugin_mouse(&mut self, mouse: MouseEvent) -> Result<EventOutcome, RuntimeError> {
        let Some(pane_id) = self.core.pane_at(mouse.column, mouse.row) else {
            return Ok(EventOutcome::Ignored);
        };

        let mut focus_changed = false;
        if matches!(mouse.kind, MouseEventKind::Down(MouseButton::Left)) {
            focus_changed = self.core.focused_pane() != Some(pane_id);
            self.core.focus_pane(pane_id)?;
        }

        let outcome = self.forward_to_plugin_id(pane_id, &HypertileEvent::Mouse(mouse));
        if focus_changed {
            return Ok(EventOutcome::Consumed);
        }
        Ok(outcome)
    }
}

fn ratio_from_mouse(direction: Direction, rect: Rect, mouse: MouseEvent) -> f32 {
    match direction {
        Direction::Horizontal => {
            if rect.width == 0 {
                return 0.5;
            }
            let offset = mouse.column.saturating_sub(rect.x).min(rect.width);
            f32::from(offset) / f32::from(rect.width)
        }
        Direction::Vertical => {
            if rect.height == 0 {
                return 0.5;
            }
            let offset = mouse.row.saturating_sub(rect.y).min(rect.height);
            f32::from(offset) / f32::from(rect.height)
        }
    }
}

fn drag_threshold_met(start_column: u16, start_row: u16, column: u16, row: u16) -> bool {
    start_column.abs_diff(column).max(start_row.abs_diff(row)) >= MOVE_DRAG_THRESHOLD
}

fn majority_overlap_target(
    runtime: &HypertileRuntime,
    dragged: PaneId,
    preview: Rect,
) -> Option<PaneId> {
    let dragged_area = preview.area();
    if dragged_area == 0 {
        return None;
    }

    runtime
        .core
        .state()
        .panes()
        .filter(|(pane_id, _)| *pane_id != dragged)
        .filter_map(|(pane_id, rect)| {
            let overlap = preview.intersection(rect).area();
            (overlap.saturating_mul(2) > dragged_area).then_some((pane_id, overlap))
        })
        .max_by_key(|(_, overlap)| *overlap)
        .map(|(pane_id, _)| pane_id)
}

fn drop_target(
    runtime: &HypertileRuntime,
    dragged: PaneId,
    mouse: MouseEvent,
    preview: Rect,
) -> Option<PaneId> {
    if let Some(target) = runtime.core.pane_at(mouse.column, mouse.row)
        && target != dragged
    {
        return Some(target);
    }

    majority_overlap_target(runtime, dragged, preview)
}

fn translate_rect(rect: Rect, dx: i32, dy: i32) -> Rect {
    Rect::new(
        translate_u16(rect.x, dx),
        translate_u16(rect.y, dy),
        rect.width,
        rect.height,
    )
}

fn translate_u16(value: u16, delta: i32) -> u16 {
    if delta.is_negative() {
        value.saturating_sub(delta.unsigned_abs().min(u32::from(u16::MAX)) as u16)
    } else {
        value.saturating_add(delta.min(i32::from(u16::MAX)) as u16)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::registry::HypertilePlugin;
    use ratatui::{buffer::Buffer, layout::Direction, layout::Rect};
    use ratatui_hypertile::{HypertileEvent, MouseButton, MouseEventKind};
    use std::{cell::RefCell, rc::Rc};

    struct RecordingPlugin {
        events: Rc<RefCell<Vec<MouseEvent>>>,
    }

    impl HypertilePlugin for RecordingPlugin {
        fn render(&mut self, _area: Rect, _buf: &mut Buffer, _is_focused: bool) {}

        fn on_event(&mut self, event: &HypertileEvent) -> EventOutcome {
            let HypertileEvent::Mouse(mouse) = event else {
                return EventOutcome::Ignored;
            };
            self.events.borrow_mut().push(*mouse);
            EventOutcome::Consumed
        }
    }

    fn render_once(runtime: &mut HypertileRuntime, area: Rect) {
        let mut buffer = Buffer::empty(area);
        runtime.render(area, &mut buffer);
    }

    #[test]
    fn layout_mode_left_click_focuses_hit_pane() {
        let mut runtime = HypertileRuntime::new();
        let right = runtime
            .split_focused(Direction::Horizontal, "block")
            .unwrap();
        assert_eq!(runtime.focused_pane(), Some(right));

        render_once(&mut runtime, Rect::new(0, 0, 100, 20));

        let outcome = runtime.handle_event(HypertileEvent::Mouse(MouseEvent::new(
            MouseEventKind::Down(MouseButton::Left),
            1,
            1,
        )));

        assert_eq!(outcome, EventOutcome::Consumed);
        assert_eq!(runtime.focused_pane(), Some(PaneId::ROOT));
    }

    #[test]
    fn layout_mode_drag_on_split_resizes_panes() {
        let mut runtime = HypertileRuntime::new();
        let right = runtime
            .split_focused(Direction::Horizontal, "block")
            .unwrap();
        render_once(&mut runtime, Rect::new(0, 0, 100, 20));

        let down = runtime.handle_event(HypertileEvent::Mouse(MouseEvent::new(
            MouseEventKind::Down(MouseButton::Left),
            50,
            1,
        )));
        let drag = runtime.handle_event(HypertileEvent::Mouse(MouseEvent::new(
            MouseEventKind::Drag(MouseButton::Left),
            30,
            1,
        )));
        let up = runtime.handle_event(HypertileEvent::Mouse(MouseEvent::new(
            MouseEventKind::Up(MouseButton::Left),
            30,
            1,
        )));
        render_once(&mut runtime, Rect::new(0, 0, 100, 20));

        assert_eq!(down, EventOutcome::Consumed);
        assert_eq!(drag, EventOutcome::Consumed);
        assert_eq!(up, EventOutcome::Consumed);
        assert_eq!(runtime.pane_rect(PaneId::ROOT).unwrap().width, 30);
        assert_eq!(runtime.pane_rect(right).unwrap().x, 30);
    }

    #[test]
    fn layout_mode_drag_pane_to_pane_swaps_them() {
        let mut runtime = HypertileRuntime::new();
        let right = runtime
            .split_focused(Direction::Horizontal, "block")
            .unwrap();
        render_once(&mut runtime, Rect::new(0, 0, 100, 20));

        let down = runtime.handle_event(HypertileEvent::Mouse(MouseEvent::new(
            MouseEventKind::Down(MouseButton::Left),
            1,
            1,
        )));
        let drag = runtime.handle_event(HypertileEvent::Mouse(MouseEvent::new(
            MouseEventKind::Drag(MouseButton::Left),
            75,
            1,
        )));
        let up = runtime.handle_event(HypertileEvent::Mouse(MouseEvent::new(
            MouseEventKind::Up(MouseButton::Left),
            75,
            1,
        )));
        render_once(&mut runtime, Rect::new(0, 0, 100, 20));

        assert_eq!(down, EventOutcome::Consumed);
        assert_eq!(drag, EventOutcome::Consumed);
        assert_eq!(up, EventOutcome::Consumed);
        assert_eq!(runtime.focused_pane(), Some(PaneId::ROOT));
        assert_eq!(runtime.pane_rect(PaneId::ROOT).unwrap().x, 50);
        assert_eq!(runtime.pane_rect(right).unwrap().x, 0);
    }

    #[test]
    fn layout_mode_drag_preview_tracks_mouse_before_drop() {
        let mut runtime = HypertileRuntime::new();
        let _ = runtime
            .split_focused(Direction::Horizontal, "block")
            .unwrap();
        render_once(&mut runtime, Rect::new(0, 0, 100, 20));

        runtime.handle_event(HypertileEvent::Mouse(MouseEvent::new(
            MouseEventKind::Down(MouseButton::Left),
            1,
            1,
        )));
        runtime.handle_event(HypertileEvent::Mouse(MouseEvent::new(
            MouseEventKind::Drag(MouseButton::Left),
            10,
            4,
        )));

        assert_eq!(
            runtime.mouse_drag.preview_rect(),
            Some(Rect::new(9, 3, 50, 20))
        );
    }

    #[test]
    fn layout_mode_drag_without_majority_overlap_does_not_swap() {
        let mut runtime = HypertileRuntime::new();
        let right = runtime
            .split_focused(Direction::Horizontal, "block")
            .unwrap();
        render_once(&mut runtime, Rect::new(0, 0, 100, 20));

        runtime.handle_event(HypertileEvent::Mouse(MouseEvent::new(
            MouseEventKind::Down(MouseButton::Left),
            1,
            1,
        )));
        runtime.handle_event(HypertileEvent::Mouse(MouseEvent::new(
            MouseEventKind::Drag(MouseButton::Left),
            20,
            1,
        )));
        runtime.handle_event(HypertileEvent::Mouse(MouseEvent::new(
            MouseEventKind::Up(MouseButton::Left),
            20,
            1,
        )));
        render_once(&mut runtime, Rect::new(0, 0, 100, 20));

        assert_eq!(runtime.pane_rect(PaneId::ROOT).unwrap().x, 0);
        assert_eq!(runtime.pane_rect(right).unwrap().x, 50);
    }

    #[test]
    fn layout_mode_mouse_move_tracks_resize_hover() {
        let mut runtime = HypertileRuntime::new();
        runtime
            .split_focused(Direction::Horizontal, "block")
            .unwrap();
        render_once(&mut runtime, Rect::new(0, 0, 100, 20));

        let hover = runtime.handle_event(HypertileEvent::Mouse(MouseEvent::new(
            MouseEventKind::Moved,
            50,
            5,
        )));

        assert_eq!(hover, EventOutcome::Consumed);
        assert_eq!(
            runtime.mouse_resize_hover,
            Some(MouseResizeHover {
                direction: Direction::Horizontal,
                rect: Rect::new(0, 0, 100, 20),
                ratio: 0.5,
            })
        );

        let clear = runtime.handle_event(HypertileEvent::Mouse(MouseEvent::new(
            MouseEventKind::Moved,
            10,
            5,
        )));

        assert_eq!(clear, EventOutcome::Consumed);
        assert_eq!(runtime.mouse_resize_hover, None);
    }

    #[test]
    fn layout_mode_hovered_split_renders_resize_indicator() {
        let mut runtime = HypertileRuntime::new();
        runtime
            .split_focused(Direction::Horizontal, "block")
            .unwrap();
        runtime.focus_pane(PaneId::ROOT).unwrap();

        let area = Rect::new(0, 0, 100, 20);
        let mut buffer = Buffer::empty(area);
        runtime.render(area, &mut buffer);
        assert_eq!(buffer.cell((50, 5)).unwrap().symbol(), "│");

        runtime.handle_event(HypertileEvent::Mouse(MouseEvent::new(
            MouseEventKind::Moved,
            50,
            5,
        )));

        let mut buffer = Buffer::empty(area);
        runtime.render(area, &mut buffer);

        assert_eq!(
            buffer.cell((50, 5)).unwrap().symbol(),
            runtime.border_config().focused_border_set.vertical_left
        );
    }

    #[test]
    fn render_area_change_cancels_active_drag() {
        let mut runtime = HypertileRuntime::new();
        runtime
            .split_focused(Direction::Horizontal, "block")
            .unwrap();
        render_once(&mut runtime, Rect::new(0, 0, 100, 20));

        runtime.handle_event(HypertileEvent::Mouse(MouseEvent::new(
            MouseEventKind::Down(MouseButton::Left),
            50,
            1,
        )));
        assert!(matches!(runtime.mouse_drag, MouseDragState::Resize { .. }));

        render_once(&mut runtime, Rect::new(0, 0, 80, 20));
        assert!(matches!(runtime.mouse_drag, MouseDragState::None));

        let drag = runtime.handle_event(HypertileEvent::Mouse(MouseEvent::new(
            MouseEventKind::Drag(MouseButton::Left),
            20,
            1,
        )));
        render_once(&mut runtime, Rect::new(0, 0, 80, 20));

        assert_eq!(drag, EventOutcome::Ignored);
        assert_eq!(runtime.pane_rect(PaneId::ROOT).unwrap().width, 40);
    }

    #[test]
    fn layout_mode_large_pane_drops_on_smaller_pane_under_cursor() {
        let mut runtime = HypertileRuntime::new();
        let logs = runtime.split_focused(Direction::Vertical, "block").unwrap();
        runtime.focus_pane(PaneId::ROOT).unwrap();
        let network = runtime
            .split_focused(Direction::Horizontal, "block")
            .unwrap();
        render_once(&mut runtime, Rect::new(0, 0, 100, 40));

        runtime.handle_event(HypertileEvent::Mouse(MouseEvent::new(
            MouseEventKind::Down(MouseButton::Left),
            1,
            25,
        )));
        runtime.handle_event(HypertileEvent::Mouse(MouseEvent::new(
            MouseEventKind::Drag(MouseButton::Left),
            10,
            5,
        )));
        runtime.handle_event(HypertileEvent::Mouse(MouseEvent::new(
            MouseEventKind::Up(MouseButton::Left),
            10,
            5,
        )));
        render_once(&mut runtime, Rect::new(0, 0, 100, 40));

        assert_eq!(runtime.pane_rect(logs).unwrap(), Rect::new(0, 0, 50, 20));
        assert_eq!(
            runtime.pane_rect(network).unwrap(),
            Rect::new(50, 0, 50, 20)
        );
        assert_eq!(
            runtime.pane_rect(PaneId::ROOT).unwrap(),
            Rect::new(0, 20, 100, 20)
        );
    }

    #[test]
    fn plugin_input_routes_mouse_to_hit_pane() {
        let events = Rc::new(RefCell::new(Vec::new()));
        let mut runtime = HypertileRuntime::new();
        runtime.register_plugin_type("record", {
            let events = events.clone();
            move || RecordingPlugin {
                events: events.clone(),
            }
        });

        runtime.replace_focused_plugin("record").unwrap();
        let right = runtime
            .split_focused(Direction::Horizontal, "record")
            .unwrap();
        runtime.focus_pane(PaneId::ROOT).unwrap();
        runtime.set_mode(InputMode::PluginInput);
        render_once(&mut runtime, Rect::new(0, 0, 100, 20));

        let mouse = MouseEvent::new(MouseEventKind::Down(MouseButton::Left), 75, 1);
        let outcome = runtime.handle_event(HypertileEvent::Mouse(mouse));

        assert_eq!(outcome, EventOutcome::Consumed);
        assert_eq!(runtime.focused_pane(), Some(right));
        assert_eq!(events.borrow().as_slice(), &[mouse]);
    }

    #[test]
    fn plugin_input_click_focus_change_consumes_even_if_plugin_ignores() {
        let mut runtime = HypertileRuntime::new();
        let right = runtime
            .split_focused(Direction::Horizontal, "block")
            .unwrap();
        runtime.focus_pane(PaneId::ROOT).unwrap();
        runtime.set_mode(InputMode::PluginInput);
        render_once(&mut runtime, Rect::new(0, 0, 100, 20));

        // The "block" plugin ignores events, so only the focus change can
        // consume the click.
        let click = MouseEvent::new(MouseEventKind::Down(MouseButton::Left), 75, 1);
        assert_eq!(
            runtime.handle_event(HypertileEvent::Mouse(click)),
            EventOutcome::Consumed
        );
        assert_eq!(runtime.focused_pane(), Some(right));

        assert_eq!(
            runtime.handle_event(HypertileEvent::Mouse(click)),
            EventOutcome::Ignored
        );
    }
}
