use crate::runtime::constants::{
    DEFAULT_PALETTE_HEIGHT_PERCENT, DEFAULT_PALETTE_MAX_ITEMS, DEFAULT_PALETTE_WIDTH_PERCENT,
    DEFAULT_PLUGIN_TYPE,
};
use crate::runtime::{HypertileRuntime, RuntimeError};
use ratatui::layout::Direction;
use ratatui_hypertile::{EventOutcome, HypertileEvent, KeyChord, KeyCode, PaneId};

/// What confirming a palette choice does.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum PaletteBehavior {
    /// Mount the chosen plugin, in the pane the palette was opened for if
    /// there is one, otherwise in a new split next to the focused pane.
    #[default]
    Apply,
    /// Leave the layout alone and keep the choice for
    /// [`HypertileRuntime::take_palette_selection`].
    EmitSelection,
}

/// Chooses which plugins the palette offers and what confirming one does.
///
/// By default the palette lists every registered plugin type except the
/// built-in placeholder, and confirming a choice mounts it. Apps with
/// singleton panes or internal plugin types can narrow the list and handle
/// the choice themselves:
///
/// ```
/// use ratatui_hypertile_extras::{HypertileRuntime, PaletteBehavior, PaletteConfig};
///
/// let mut runtime = HypertileRuntime::builder()
///     .with_palette_config(
///         PaletteConfig::default()
///             .with_allowed_plugin_types(["logs", "inspector"])
///             .with_behavior(PaletteBehavior::EmitSelection),
///     )
///     .build();
///
/// // After passing input to the runtime:
/// if let Some(selection) = runtime.take_palette_selection() {
///     // Focus an existing pane, or create one yourself.
///     println!("picked {}", selection.plugin_type);
/// }
/// ```
///
/// In a [`WorkspaceRuntime`](crate::WorkspaceRuntime) every tab has its own
/// runtime, so set this in the tab factory. Open state and pending
/// selections stay with the tab they happened in.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PaletteConfig {
    allowed_plugin_types: Option<Vec<String>>,
    behavior: PaletteBehavior,
}

impl PaletteConfig {
    /// Limits the palette to these plugin types.
    ///
    /// Names that are not registered are ignored, and the palette lists the
    /// rest sorted, not in the order given. An empty list disables the
    /// palette.
    pub fn with_allowed_plugin_types<I, S>(mut self, plugin_types: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.allowed_plugin_types = Some(plugin_types.into_iter().map(Into::into).collect());
        self
    }

    /// Sets what confirming a choice does.
    pub fn with_behavior(mut self, behavior: PaletteBehavior) -> Self {
        self.behavior = behavior;
        self
    }

    /// Returns the allowlist, or `None` when every registered plugin type is
    /// offered.
    pub fn allowed_plugin_types(&self) -> Option<&[String]> {
        self.allowed_plugin_types.as_deref()
    }

    /// Returns what confirming a choice does.
    pub fn behavior(&self) -> PaletteBehavior {
        self.behavior
    }

    fn allows(&self, plugin_type: &str) -> bool {
        self.allowed_plugin_types
            .as_ref()
            .is_none_or(|allowed| allowed.iter().any(|name| name == plugin_type))
    }
}

/// A choice confirmed in [`PaletteBehavior::EmitSelection`] mode.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub struct PaletteSelection {
    /// The chosen plugin type.
    pub plugin_type: String,
    /// The pane the palette was opened for, if any.
    ///
    /// This is set when a [`SplitBehavior::PromptPalette`](crate::SplitBehavior::PromptPalette)
    /// split or interacting with a placeholder opened the palette. The pane
    /// held the placeholder at that point and it is up to you to fill, close,
    /// or keep it. Check that it still exists first if your app changes the
    /// layout in between.
    pub target_pane: Option<PaneId>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct FuzzyMatch {
    gaps: usize,
    start: usize,
    len: usize,
}

#[derive(Debug, Clone)]
pub(super) struct PaletteState {
    pub config: PaletteConfig,
    pub width_percent: u16,
    pub height_percent: u16,
    pub max_items: usize,
    pub show: bool,
    pub selected: usize,
    pub query: String,
    pub items: Vec<String>,
    pub target_pane: Option<PaneId>,
    selection: Option<PaletteSelection>,
    filtered_cache: Option<Vec<String>>,
    cache_query: String,
}

impl Default for PaletteState {
    fn default() -> Self {
        Self {
            config: PaletteConfig::default(),
            width_percent: DEFAULT_PALETTE_WIDTH_PERCENT,
            height_percent: DEFAULT_PALETTE_HEIGHT_PERCENT,
            max_items: DEFAULT_PALETTE_MAX_ITEMS,
            show: false,
            selected: 0,
            query: String::new(),
            items: Vec::new(),
            target_pane: None,
            selection: None,
            filtered_cache: None,
            cache_query: String::new(),
        }
    }
}

impl PaletteState {
    pub(super) fn with_config(
        width_percent: u16,
        height_percent: u16,
        max_items: usize,
        config: PaletteConfig,
    ) -> Self {
        Self {
            config,
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
    /// Returns the current palette configuration.
    pub fn palette_config(&self) -> &PaletteConfig {
        &self.palette.config
    }

    /// Replaces the palette configuration.
    ///
    /// This closes the palette and drops any unclaimed selection.
    pub fn set_palette_config(&mut self, config: PaletteConfig) {
        self.discard_palette();
        self.palette.config = config;
    }

    /// Opens the palette without a target pane.
    ///
    /// Returns `false` and leaves the palette closed when there is nothing
    /// to offer. In [`PaletteBehavior::Apply`] mode a confirmed choice goes
    /// into a new split next to the focused pane.
    pub fn open_palette(&mut self) -> bool {
        self.open_palette_for_target(None)
    }

    /// Returns whether the palette is showing.
    ///
    /// While it is, key and mouse input belong to the palette, so pass events
    /// to the runtime before your own shortcuts. Use
    /// [`try_handle_event`](Self::try_handle_event) to tell a failed
    /// confirmation apart from ignored input.
    pub fn is_palette_open(&self) -> bool {
        self.palette.show
    }

    /// Closes the palette without choosing anything.
    ///
    /// A placeholder pane created for it stays in place, and a choice that
    /// was already confirmed is kept.
    pub fn close_palette(&mut self) {
        self.palette.show = false;
        self.palette.target_pane = None;
        self.palette.query.clear();
        self.palette.items.clear();
        self.palette.selected = 0;
        self.palette.invalidate_cache();
    }

    /// Takes the last choice confirmed in [`PaletteBehavior::EmitSelection`]
    /// mode.
    ///
    /// Each choice is returned once. Opening the palette again, changing its
    /// config, [`reset`](Self::reset), [`set_root`](Self::set_root), and
    /// mutable core access drop a choice that was not taken.
    pub fn take_palette_selection(&mut self) -> Option<PaletteSelection> {
        self.palette.selection.take()
    }

    /// Closes the palette and drops any unclaimed selection.
    pub(super) fn discard_palette(&mut self) {
        self.close_palette();
        self.palette.selection = None;
    }

    pub(super) fn open_palette_for_target(&mut self, target_pane: Option<PaneId>) -> bool {
        self.discard_palette();
        let mut items = self
            .registry
            .registered_types()
            .filter(|t| *t != DEFAULT_PLUGIN_TYPE && self.palette.config.allows(t))
            .map(str::to_string)
            .collect::<Vec<_>>();
        if items.is_empty() {
            return false;
        }
        items.sort();
        self.clear_transient_state();
        self.palette.items = items;
        self.palette.target_pane = target_pane;
        self.palette.show = true;
        true
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
                self.close_palette();
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
                    self.close_palette();
                    return Some(Ok(EventOutcome::Consumed));
                };
                let target_pane = self.palette.target_pane;
                if self.palette.config.behavior == PaletteBehavior::EmitSelection {
                    self.close_palette();
                    self.palette.selection = Some(PaletteSelection {
                        plugin_type,
                        target_pane,
                    });
                    return Some(Ok(EventOutcome::Consumed));
                }
                let result = if let Some(pane_id) = target_pane {
                    self.replace_pane_plugin(pane_id, &plugin_type)
                } else {
                    let direction = self.auto_split_direction();
                    self.split_focused(direction, &plugin_type).map(|_| ())
                };
                // Close on failure too. The target may be gone, and retrying
                // the same choice would fail the same way.
                self.close_palette();
                Some(result.map(|()| EventOutcome::Consumed))
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
        runtime::{HypertileRuntime, InputMode, SplitBehavior, mouse::MouseResizeHover},
    };
    use ratatui::{buffer::Buffer, layout::Rect};

    struct Dummy;
    impl HypertilePlugin for Dummy {
        fn render(&mut self, _area: Rect, _buf: &mut Buffer, _is_focused: bool) {}
    }

    fn key(code: KeyCode) -> HypertileEvent {
        HypertileEvent::Key(KeyChord::new(code))
    }

    fn emit_selection() -> PaletteConfig {
        PaletteConfig::default().with_behavior(PaletteBehavior::EmitSelection)
    }

    fn runtime_with(config: PaletteConfig) -> HypertileRuntime {
        let mut runtime = HypertileRuntime::builder()
            .with_split_behavior(SplitBehavior::PromptPalette)
            .with_palette_config(config)
            .build();
        runtime.register_plugin_type("cpu", || Dummy);
        runtime.register_plugin_type("logs", || Dummy);
        runtime
    }

    fn rendered_text(runtime: &HypertileRuntime) -> String {
        let area = Rect::new(0, 0, 80, 24);
        let mut buf = Buffer::empty(area);
        runtime.render_palette(area, &mut buf);
        buf.content.iter().map(|cell| cell.symbol()).collect()
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

    #[test]
    fn allowlist_filters_and_sorts_palette_items() {
        let mut runtime = runtime_with(
            PaletteConfig::default()
                .with_allowed_plugin_types(["logs", "cpu", "cpu", "block", "unknown"]),
        );
        runtime.register_plugin_type("internal", || Dummy);

        assert!(runtime.open_palette());
        assert_eq!(runtime.palette.items, ["cpu", "logs"]);
    }

    #[test]
    fn empty_allowlist_disables_palette() {
        let mut runtime =
            runtime_with(PaletteConfig::default().with_allowed_plugin_types(Vec::<String>::new()));

        assert!(!runtime.open_palette());
        assert_eq!(
            runtime.handle_event(key(KeyCode::Char('p'))),
            EventOutcome::Ignored
        );
        assert!(!runtime.is_palette_open());
    }

    #[test]
    fn opening_without_choices_keeps_transient_state() {
        let mut runtime = HypertileRuntime::new();
        runtime.mouse_resize_hover = Some(MouseResizeHover {
            direction: Direction::Horizontal,
            rect: Rect::new(0, 0, 10, 10),
            ratio: 0.5,
        });

        assert!(!runtime.open_palette());
        assert!(runtime.mouse_resize_hover.is_some());
    }

    #[test]
    fn emit_selection_reports_choice_once_without_mounting() {
        let mut runtime = runtime_with(emit_selection());
        let before = runtime.registry.instance_count();

        assert!(runtime.handle_event(key(KeyCode::Char('s'))).is_consumed());
        let target = runtime
            .palette
            .target_pane
            .expect("split should target new pane");
        assert!(runtime.handle_event(key(KeyCode::Enter)).is_consumed());

        assert!(!runtime.is_palette_open());
        assert_eq!(runtime.registry.instance_count(), before + 1);
        assert_eq!(
            runtime.registry.plugin_type_for(target),
            Some(DEFAULT_PLUGIN_TYPE)
        );
        assert_eq!(
            runtime.take_palette_selection(),
            Some(PaletteSelection {
                plugin_type: "cpu".into(),
                target_pane: Some(target),
            })
        );
        assert_eq!(runtime.take_palette_selection(), None);
    }

    #[test]
    fn emit_selection_from_open_palette_has_no_target() {
        let mut runtime = runtime_with(emit_selection());

        assert!(runtime.open_palette());
        runtime.handle_event(key(KeyCode::Down));
        runtime.handle_event(key(KeyCode::Enter));

        assert_eq!(
            runtime.take_palette_selection(),
            Some(PaletteSelection {
                plugin_type: "logs".into(),
                target_pane: None,
            })
        );
        assert_eq!(runtime.registry.instance_count(), 1);
    }

    #[test]
    fn escape_closes_palette_without_selection() {
        let mut runtime = runtime_with(emit_selection());

        assert!(runtime.open_palette());
        assert!(runtime.handle_event(key(KeyCode::Escape)).is_consumed());

        assert!(!runtime.is_palette_open());
        assert_eq!(runtime.take_palette_selection(), None);
    }

    #[test]
    fn failed_apply_closes_palette() {
        let mut runtime = runtime_with(PaletteConfig::default());
        runtime.handle_event(key(KeyCode::Char('s')));
        runtime.close_focused().unwrap();
        assert!(runtime.is_palette_open());

        assert!(runtime.try_handle_event(key(KeyCode::Enter)).is_err());
        assert!(!runtime.is_palette_open());
        assert_eq!(runtime.registry.instance_count(), 1);
    }

    #[test]
    fn reopening_or_reconfiguring_discards_unclaimed_selection() {
        let mut runtime = runtime_with(emit_selection());

        runtime.open_palette();
        runtime.handle_event(key(KeyCode::Enter));
        runtime.open_palette();
        assert_eq!(runtime.take_palette_selection(), None);

        runtime.handle_event(key(KeyCode::Enter));
        runtime.set_palette_config(emit_selection());
        assert_eq!(runtime.take_palette_selection(), None);
    }

    #[test]
    fn reset_discards_palette_state() {
        let mut runtime = runtime_with(emit_selection());

        runtime.handle_event(key(KeyCode::Char('s')));
        runtime.handle_event(key(KeyCode::Enter));
        runtime.reset();
        assert_eq!(runtime.take_palette_selection(), None);

        runtime.open_palette();
        runtime.reset();
        assert!(!runtime.is_palette_open());
    }

    #[test]
    fn palette_without_matches_shows_hint_and_enter_closes_it() {
        let mut runtime = runtime_with(emit_selection());
        runtime.open_palette();
        runtime.handle_event(key(KeyCode::Char('z')));

        assert!(rendered_text(&runtime).contains("No matching plugins"));
        assert!(runtime.handle_event(key(KeyCode::Enter)).is_consumed());
        assert!(!runtime.is_palette_open());
        assert_eq!(runtime.take_palette_selection(), None);
    }

    #[test]
    fn closed_palette_renders_nothing() {
        let runtime = runtime_with(PaletteConfig::default());

        assert_eq!(rendered_text(&runtime).trim(), "");
    }
    #[test]
    fn close_palette_keeps_confirmed_selection() {
        let mut runtime = runtime_with(emit_selection());

        runtime.open_palette();
        runtime.handle_event(key(KeyCode::Enter));
        runtime.close_palette();

        assert!(runtime.take_palette_selection().is_some());
    }

    #[test]
    fn set_root_discards_palette_state() {
        let mut runtime = runtime_with(emit_selection());
        let root = runtime.core().root().clone();

        runtime.open_palette();
        runtime.handle_event(key(KeyCode::Enter));
        runtime.set_root(root.clone()).unwrap();
        assert_eq!(runtime.take_palette_selection(), None);

        runtime.open_palette();
        runtime.set_root(root).unwrap();
        assert!(!runtime.is_palette_open());
    }

    #[test]
    fn core_reset_through_with_core_mut_discards_stale_target() {
        let mut runtime = runtime_with(emit_selection());

        runtime.handle_event(key(KeyCode::Char('s')));
        let target = runtime
            .palette
            .target_pane
            .expect("split should target new pane");
        runtime.handle_event(key(KeyCode::Enter));
        runtime.with_core_mut(|core| core.reset());
        let reused = runtime
            .split_focused(Direction::Horizontal, DEFAULT_PLUGIN_TYPE)
            .unwrap();

        assert_eq!(reused, target, "reset should reuse the pane id");
        assert_eq!(runtime.take_palette_selection(), None);
    }
}
