use crate::runtime::constants::{
    DEFAULT_PALETTE_HEIGHT_PERCENT, DEFAULT_PALETTE_MAX_ITEMS, DEFAULT_PALETTE_WIDTH_PERCENT,
    DEFAULT_PLUGIN_TYPE,
};
use crate::runtime::{HypertileRuntime, RuntimeError};
use ratatui::layout::Direction;
use ratatui_hypertile::{EventOutcome, HypertileEvent, KeyChord, KeyCode, PaneId};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct FuzzyMatch {
    gaps: usize,
    start: usize,
    len: usize,
}

#[derive(Debug, Clone)]
pub(super) struct PaletteState {
    pub width_percent: u16,
    pub height_percent: u16,
    pub max_items: usize,
    pub show: bool,
    pub selected: usize,
    pub query: String,
    pub items: Vec<String>,
    pub target_pane: Option<PaneId>,
    filtered_cache: Option<Vec<String>>,
    cache_query: String,
}

impl Default for PaletteState {
    fn default() -> Self {
        Self {
            width_percent: DEFAULT_PALETTE_WIDTH_PERCENT,
            height_percent: DEFAULT_PALETTE_HEIGHT_PERCENT,
            max_items: DEFAULT_PALETTE_MAX_ITEMS,
            show: false,
            selected: 0,
            query: String::new(),
            items: Vec::new(),
            target_pane: None,
            filtered_cache: None,
            cache_query: String::new(),
        }
    }
}

impl PaletteState {
    pub(super) fn with_config(width_percent: u16, height_percent: u16, max_items: usize) -> Self {
        Self {
            width_percent,
            height_percent,
            max_items,
            ..Self::default()
        }
    }

    pub(super) fn invalidate_cache(&mut self) {
        self.cache_query.clear();
        self.filtered_cache = None;
    }
}

impl HypertileRuntime {
    pub(super) fn open_palette(&mut self) -> Result<EventOutcome, RuntimeError> {
        self.open_palette_for_target(None)
    }

    pub(super) fn open_palette_for_target(
        &mut self,
        target_pane: Option<PaneId>,
    ) -> Result<EventOutcome, RuntimeError> {
        self.palette.items = self
            .registry
            .registered_types()
            .filter(|t| *t != DEFAULT_PLUGIN_TYPE)
            .map(str::to_string)
            .collect::<Vec<_>>();
        self.palette.items.sort();
        self.palette.query.clear();
        self.palette.selected = 0;
        self.palette.invalidate_cache();
        self.palette.target_pane = target_pane;
        self.palette.show = !self.palette.items.is_empty();
        if self.palette.show {
            Ok(EventOutcome::Consumed)
        } else {
            self.palette.target_pane = None;
            Ok(EventOutcome::Ignored)
        }
    }

    fn refresh_filtered_palette_cache(&mut self) {
        let query = self.palette.query.trim().to_ascii_lowercase();

        if query.is_empty() {
            self.palette.invalidate_cache();
            return;
        }

        if self.palette.cache_query == query && self.palette.filtered_cache.is_some() {
            return;
        }

        let mut scored = self
            .palette
            .items
            .iter()
            .filter_map(|item| fuzzy_score(&query, item).map(|score| (score, item)))
            .collect::<Vec<_>>();

        scored.sort_by(|(a_score, a_item), (b_score, b_item)| {
            a_score.cmp(b_score).then_with(|| a_item.cmp(b_item))
        });

        self.palette.cache_query = query;
        self.palette.filtered_cache = Some(
            scored
                .into_iter()
                .map(|(_, item)| item.clone())
                .collect::<Vec<_>>(),
        );
    }

    pub(super) fn filtered_palette_items(&self) -> &[String] {
        self.palette
            .filtered_cache
            .as_deref()
            .unwrap_or(self.palette.items.as_slice())
    }

    pub(super) fn clamp_palette_selection(&mut self) {
        self.refresh_filtered_palette_cache();
        let filtered_len = self.filtered_palette_items().len();
        if filtered_len == 0 {
            self.palette.selected = 0;
            return;
        }
        self.palette.selected = self.palette.selected.min(filtered_len - 1);
    }

    pub(super) fn handle_palette_event(
        &mut self,
        event: &HypertileEvent,
    ) -> Option<Result<EventOutcome, RuntimeError>> {
        if !self.palette.show {
            return None;
        }

        match event {
            HypertileEvent::Key(KeyChord {
                code: KeyCode::Escape,
                modifiers,
            }) if modifiers.is_empty() => {
                self.palette.show = false;
                self.palette.target_pane = None;
                Some(Ok(EventOutcome::Consumed))
            }
            HypertileEvent::Key(KeyChord {
                code: KeyCode::Down | KeyCode::Tab,
                modifiers,
            }) if modifiers.is_empty() => {
                self.refresh_filtered_palette_cache();
                let filtered_len = self.filtered_palette_items().len();
                if filtered_len != 0 {
                    self.palette.selected = (self.palette.selected + 1).min(filtered_len - 1);
                }
                Some(Ok(EventOutcome::Consumed))
            }
            HypertileEvent::Key(KeyChord {
                code: KeyCode::Up | KeyCode::BackTab,
                modifiers,
            }) if modifiers.is_empty() => {
                self.palette.selected = self.palette.selected.saturating_sub(1);
                Some(Ok(EventOutcome::Consumed))
            }
            HypertileEvent::Key(KeyChord {
                code: KeyCode::Enter,
                modifiers,
            }) if modifiers.is_empty() => {
                self.refresh_filtered_palette_cache();
                let selected = self.palette.selected;
                let plugin_type = self.filtered_palette_items().get(selected).cloned();
                let Some(plugin_type) = plugin_type else {
                    self.palette.show = false;
                    self.palette.target_pane = None;
                    return Some(Ok(EventOutcome::Ignored));
                };
                let target = self.palette.target_pane.take();
                if let Some(pane_id) = target {
                    Some(self.replace_pane_plugin(pane_id, &plugin_type).map(|_| {
                        self.palette.show = false;
                        EventOutcome::Consumed
                    }))
                } else {
                    let direction = self.auto_split_direction();
                    Some(self.split_focused(direction, &plugin_type).map(|_| {
                        self.palette.show = false;
                        EventOutcome::Consumed
                    }))
                }
            }
            HypertileEvent::Key(KeyChord {
                code: KeyCode::Backspace,
                modifiers,
            }) if modifiers.is_empty() => {
                self.palette.query.pop();
                self.palette.invalidate_cache();
                self.clamp_palette_selection();
                Some(Ok(EventOutcome::Consumed))
            }
            HypertileEvent::Key(KeyChord {
                code: KeyCode::Char(ch),
                modifiers,
            }) if modifiers.is_empty() => {
                self.palette.query.push(*ch);
                self.palette.invalidate_cache();
                self.clamp_palette_selection();
                Some(Ok(EventOutcome::Consumed))
            }
            HypertileEvent::Tick => None,
            _ => Some(Ok(EventOutcome::Consumed)),
        }
    }

    pub(super) fn auto_split_direction(&self) -> Direction {
        self.core
            .focused_pane()
            .and_then(|id| self.core.pane_rect(id))
            .map(|rect| {
                if rect.width >= rect.height {
                    Direction::Horizontal
                } else {
                    Direction::Vertical
                }
            })
            .unwrap_or(Direction::Horizontal)
    }
}

fn fuzzy_score(query: &str, candidate: &str) -> Option<FuzzyMatch> {
    let mut query_iter = query.chars();
    let mut current_query = query_iter.next()?;
    let mut first_match = None;
    let mut last_match = 0usize;
    let mut gaps = 0usize;

    for (index, ch) in candidate.chars().enumerate() {
        if ch.to_ascii_lowercase() != current_query {
            continue;
        }

        if first_match.is_some() {
            gaps += index.saturating_sub(last_match + 1);
        } else {
            first_match = Some(index);
        }

        last_match = index;
        match query_iter.next() {
            Some(next) => current_query = next,
            None => {
                return Some(FuzzyMatch {
                    gaps,
                    start: first_match.unwrap_or(usize::MAX),
                    len: candidate.len(),
                });
            }
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        HypertilePlugin,
        runtime::{HypertileRuntime, InputMode, SplitBehavior},
    };
    use ratatui::{buffer::Buffer, layout::Rect};

    struct Dummy;
    impl HypertilePlugin for Dummy {
        fn render(&mut self, _area: Rect, _buf: &mut Buffer, _is_focused: bool) {}
    }

    #[test]
    fn split_shortcut_can_open_palette_for_new_pane() {
        let mut runtime = HypertileRuntime::builder()
            .with_split_behavior(SplitBehavior::PromptPalette)
            .build();
        runtime.register_plugin_type("cpu", || Dummy);

        let before = runtime.registry.instance_count();
        let outcome = runtime.handle_event(HypertileEvent::Key(KeyChord::new(KeyCode::Char('s'))));
        assert!(outcome.is_consumed());
        assert_eq!(runtime.registry.instance_count(), before + 1);
        assert!(runtime.palette.show);

        let target = runtime
            .palette
            .target_pane
            .expect("split behavior should target new pane");

        runtime.palette.query = "cpu".to_string();
        runtime.clamp_palette_selection();
        let apply = runtime
            .handle_palette_event(&HypertileEvent::Key(KeyChord::new(KeyCode::Enter)))
            .expect("palette should handle enter")
            .expect("palette apply should succeed");
        assert!(apply.is_consumed());
        assert_eq!(runtime.registry.plugin_type_for(target), Some("cpu"));
        assert_eq!(runtime.registry.instance_count(), before + 1);
    }

    #[test]
    fn split_shortcut_can_create_placeholder_without_opening_palette() {
        let mut runtime = HypertileRuntime::builder()
            .with_split_behavior(SplitBehavior::Placeholder)
            .build();
        runtime.register_plugin_type("cpu", || Dummy);

        let before = runtime.registry.instance_count();
        let outcome = runtime.handle_event(HypertileEvent::Key(KeyChord::new(KeyCode::Char('s'))));
        assert!(outcome.is_consumed());
        assert_eq!(runtime.registry.instance_count(), before + 1);
        assert!(!runtime.palette.show);
        assert_eq!(runtime.palette.target_pane, None);

        let focused = runtime.focused_pane().expect("split should focus new pane");
        assert_eq!(runtime.registry.plugin_type_for(focused), Some("block"));
    }

    #[test]
    fn interact_on_mounted_plugin_switches_to_plugin_input_mode() {
        let mut runtime = HypertileRuntime::new();
        runtime.register_plugin_type("cpu", || Dummy);
        runtime.replace_focused_plugin("cpu").unwrap();
        assert_eq!(runtime.mode(), InputMode::Layout);

        let outcome = runtime.handle_event(HypertileEvent::Key(KeyChord::new(KeyCode::Enter)));
        assert!(outcome.is_consumed());
        assert_eq!(runtime.mode(), InputMode::PluginInput);
    }
}
