use super::types::AnimationConfig;
use ratatui::layout::Rect;
use ratatui_hypertile::PaneId;
use std::collections::HashMap;
use std::time::{Duration, Instant};

#[derive(Debug, Default)]
pub(super) struct AnimationState {
    active: Option<ActiveAnimation>,
    last_area: Option<Rect>,
    before_panes: HashMap<PaneId, Rect>,
    display_panes: Vec<(PaneId, Rect)>,
    moving_panes: Vec<(PaneId, Rect)>,
}

impl AnimationState {
    pub(super) fn clear(&mut self) {
        self.active = None;
        self.before_panes.clear();
        self.display_panes.clear();
        self.moving_panes.clear();
    }

    pub(super) fn last_area(&self) -> Option<Rect> {
        self.last_area
    }

    pub(super) fn remember_area(&mut self, area: Rect) {
        if self.last_area != Some(area) {
            self.active = None;
        }
        self.last_area = Some(area);
    }

    pub(super) fn capture_before<I>(&mut self, area: Rect, panes: I, now: Instant)
    where
        I: IntoIterator<Item = (PaneId, Rect)>,
    {
        self.before_panes.clear();

        let Some(active) = self.active.as_ref() else {
            self.before_panes.extend(panes);
            return;
        };

        if active.area != area || active.is_finished(now) {
            self.active = None;
            self.before_panes.extend(panes);
            return;
        }

        let progress = active.progress(now);
        for (pane_id, rect) in panes {
            let rect = active
                .transition_for(pane_id)
                .map_or(rect, |transition| transition.interpolate(progress));
            self.before_panes.insert(pane_id, rect);
        }
    }

    pub(super) fn start<I>(&mut self, area: Rect, panes: I, now: Instant, config: AnimationConfig)
    where
        I: IntoIterator<Item = (PaneId, Rect)>,
    {
        if !config.enabled {
            self.active = None;
            return;
        }

        let mut transitions = self.active.take().map_or_else(HashMap::new, |mut active| {
            active.transitions.clear();
            active.transitions
        });

        for (pane_id, to) in panes {
            let Some(from) = self.before_panes.get(&pane_id).copied() else {
                continue;
            };
            if from != to {
                transitions.insert(pane_id, RectTransition { from, to });
            }
        }

        if transitions.is_empty() {
            self.active = None;
            return;
        }

        self.active = Some(ActiveAnimation {
            area,
            started_at: now,
            duration: normalize_duration(config.duration),
            transitions,
        });
    }

    pub(super) fn next_frame_in(&self, now: Instant, config: AnimationConfig) -> Option<Duration> {
        let active = self.active.as_ref()?;
        if !config.enabled || active.is_finished(now) {
            return None;
        }
        active.next_frame_in(now, normalize_frame_interval(config.frame_interval))
    }

    pub(super) fn display_rects<I>(
        &mut self,
        area: Rect,
        panes: I,
        now: Instant,
    ) -> &[(PaneId, Rect)]
    where
        I: IntoIterator<Item = (PaneId, Rect)>,
    {
        self.display_panes.clear();
        self.moving_panes.clear();

        let Some(active) = self.active.as_ref() else {
            self.display_panes.extend(panes);
            return self.display_panes.as_slice();
        };

        if active.area != area || active.is_finished(now) {
            self.active = None;
            self.display_panes.extend(panes);
            return self.display_panes.as_slice();
        }

        let progress = active.progress(now);
        for (pane_id, rect) in panes {
            if let Some(transition) = active.transition_for(pane_id) {
                self.moving_panes
                    .push((pane_id, transition.interpolate(progress)));
            } else {
                self.display_panes.push((pane_id, rect));
            }
        }

        self.display_panes.extend_from_slice(&self.moving_panes);
        self.display_panes.as_slice()
    }
}

#[derive(Debug)]
struct ActiveAnimation {
    area: Rect,
    started_at: Instant,
    duration: Duration,
    transitions: HashMap<PaneId, RectTransition>,
}

impl ActiveAnimation {
    fn is_finished(&self, now: Instant) -> bool {
        now.saturating_duration_since(self.started_at) >= self.duration
    }

    fn progress(&self, now: Instant) -> f32 {
        let elapsed = now.saturating_duration_since(self.started_at);
        ease_out_cubic(elapsed.as_secs_f32() / self.duration.as_secs_f32()).clamp(0.0, 1.0)
    }

    fn next_frame_in(&self, now: Instant, frame_interval: Duration) -> Option<Duration> {
        let elapsed = now.saturating_duration_since(self.started_at);
        if elapsed >= self.duration {
            return None;
        }

        let until_end = self.duration - elapsed;
        let until_next_frame = remaining_until_next_frame(elapsed, frame_interval);
        Some(until_end.min(until_next_frame))
    }

    fn transition_for(&self, pane_id: PaneId) -> Option<RectTransition> {
        self.transitions.get(&pane_id).copied()
    }
}

#[derive(Debug, Clone, Copy)]
struct RectTransition {
    from: Rect,
    to: Rect,
}

impl RectTransition {
    fn interpolate(self, progress: f32) -> Rect {
        Rect::new(
            lerp_u16(self.from.x, self.to.x, progress),
            lerp_u16(self.from.y, self.to.y, progress),
            lerp_u16(self.from.width, self.to.width, progress),
            lerp_u16(self.from.height, self.to.height, progress),
        )
    }
}

fn normalize_duration(duration: Duration) -> Duration {
    if duration.is_zero() {
        Duration::from_millis(1)
    } else {
        duration
    }
}

fn normalize_frame_interval(frame_interval: Duration) -> Duration {
    if frame_interval.is_zero() {
        Duration::from_millis(16)
    } else {
        frame_interval
    }
}

fn remaining_until_next_frame(elapsed: Duration, frame_interval: Duration) -> Duration {
    let elapsed_nanos = elapsed.as_nanos();
    let frame_nanos = frame_interval.as_nanos();
    let next_frame_nanos = elapsed_nanos
        .saturating_div(frame_nanos)
        .saturating_add(1)
        .saturating_mul(frame_nanos);
    saturating_duration_from_nanos(next_frame_nanos.saturating_sub(elapsed_nanos))
}

fn saturating_duration_from_nanos(nanos: u128) -> Duration {
    Duration::from_nanos(nanos.min(u128::from(u64::MAX)) as u64)
}

fn ease_out_cubic(progress: f32) -> f32 {
    let t = progress.clamp(0.0, 1.0);
    1.0 - (1.0 - t).powi(3)
}

fn lerp_u16(from: u16, to: u16, progress: f32) -> u16 {
    let from = f32::from(from);
    let to = f32::from(to);
    (from + (to - from) * progress)
        .round()
        .clamp(0.0, f32::from(u16::MAX)) as u16
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_config(duration_ms: u64, frame_interval_ms: u64) -> AnimationConfig {
        AnimationConfig {
            enabled: true,
            duration: Duration::from_millis(duration_ms),
            frame_interval: Duration::from_millis(frame_interval_ms),
        }
    }

    #[test]
    fn transition_reaches_end_rect() {
        let transition = RectTransition {
            from: Rect::new(0, 0, 10, 5),
            to: Rect::new(20, 6, 8, 4),
        };

        assert_eq!(transition.interpolate(0.0), Rect::new(0, 0, 10, 5));
        assert_eq!(transition.interpolate(1.0), Rect::new(20, 6, 8, 4));
    }

    #[test]
    fn next_frame_in_tracks_remaining_time_to_frame_boundary_and_end() {
        let now = Instant::now();
        let mut state = AnimationState::default();
        state.capture_before(
            Rect::new(0, 0, 20, 5),
            [(PaneId::ROOT, Rect::new(0, 0, 10, 5))],
            now,
        );
        state.start(
            Rect::new(0, 0, 20, 5),
            [(PaneId::ROOT, Rect::new(10, 0, 10, 5))],
            now,
            test_config(40, 16),
        );

        assert_eq!(
            state.next_frame_in(now + Duration::from_millis(10), test_config(40, 16)),
            Some(Duration::from_millis(6))
        );
        assert_eq!(
            state.next_frame_in(now + Duration::from_millis(35), test_config(40, 16)),
            Some(Duration::from_millis(5))
        );
        assert_eq!(
            state.next_frame_in(now + Duration::from_millis(50), test_config(40, 16)),
            None
        );
    }

    #[test]
    fn interrupted_animation_restarts_from_current_displayed_rect() {
        let now = Instant::now();
        let area = Rect::new(0, 0, 40, 5);
        let pane_id = PaneId::ROOT;
        let mut state = AnimationState::default();
        state.capture_before(area, [(pane_id, Rect::new(0, 0, 10, 5))], now);
        state.start(
            area,
            [(pane_id, Rect::new(20, 0, 10, 5))],
            now,
            test_config(100, 16),
        );

        let restart_at = now + Duration::from_millis(50);
        state.capture_before(area, [(pane_id, Rect::new(20, 0, 10, 5))], restart_at);
        state.start(
            area,
            [(pane_id, Rect::new(30, 0, 10, 5))],
            restart_at,
            test_config(100, 16),
        );

        assert_eq!(
            state.display_rects(area, [(pane_id, Rect::new(30, 0, 10, 5))], restart_at),
            &[(pane_id, Rect::new(18, 0, 10, 5))]
        );
    }
}
