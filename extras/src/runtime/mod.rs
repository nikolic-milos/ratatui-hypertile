mod animation;
mod builder;
mod constants;
#[cfg(feature = "crossterm")]
mod crossterm;
mod default_plugin;
mod keymap;
mod palette;
mod render;
mod tab_bar;
mod types;
mod widget;
pub(crate) mod workspace;

use crate::registry::{HypertilePlugin, Registry};
use ratatui::layout::{Direction, Rect};
use ratatui_hypertile::{
    EventOutcome, Hypertile as CoreHypertile, HypertileAction, HypertileEvent, KeyChord, KeyCode,
    PaneId, PaneSnapshot, raw, raw::Node as CoreNode,
};
use std::collections::HashSet;
use std::sync::Arc;
use std::time::{Duration, Instant};

pub use builder::HypertileRuntimeBuilder;
#[cfg(feature = "crossterm")]
pub use crossterm::{event_from_crossterm, keychord_from_crossterm};
pub use keymap::MoveBindings;
pub use tab_bar::{TabBar, TabBarItem};
pub use types::{AnimationConfig, BorderConfig, InputMode, RuntimeError, SplitBehavior};
pub use widget::{HypertileView, ModeIndicator};
pub use workspace::{WorkspaceAction, WorkspaceRuntime};

use animation::AnimationState;
use constants::DEFAULT_PLUGIN_TYPE;
use keymap::RuntimeAction;
use palette::PaletteState;

/// Ready-made runtime for apps that want tiling plus plugins without building
/// an event loop from scratch.
///
/// It owns the core layout engine, a plugin registry, the built-in palette,
/// and the default layout-mode key handling. A typical loop is:
///
/// 1. register your plugin types
/// 2. send input through [`handle_event`](Self::handle_event) or
///    [`try_handle_event`](Self::try_handle_event)
/// 3. draw with [`render`](Self::render)
/// 4. if animations are enabled, use [`next_frame_in`](Self::next_frame_in)
///    to decide when to wake up for the next frame
///
/// `i` enters plugin input mode and `Esc` returns to layout mode. Use the core
/// [`Hypertile`](ratatui_hypertile::Hypertile) directly if you want full
/// control over input and rendering.
///
/// ```
/// use ratatui_hypertile_extras::HypertileRuntime;
///
/// let runtime = HypertileRuntime::new();
/// assert_eq!(runtime.registry().instance_count(), 1);
/// ```
pub struct HypertileRuntime {
    core: CoreHypertile,
    registry: Registry,
    mode: InputMode,
    palette: PaletteState,
    default_split_plugin_type: Arc<str>,
    default_move_scope: ratatui_hypertile::MoveScope,
    move_bindings: MoveBindings,
    split_behavior: SplitBehavior,
    border_config: BorderConfig,
    animation_config: AnimationConfig,
    animation_state: AnimationState,
}

impl Default for HypertileRuntime {
    fn default() -> Self {
        Self::new()
    }
}

impl HypertileRuntime {
    pub fn builder() -> HypertileRuntimeBuilder {
        HypertileRuntimeBuilder::default()
    }

    pub fn new() -> Self {
        Self::builder().build()
    }

    pub fn core(&self) -> &CoreHypertile {
        &self.core
    }

    pub fn registry(&self) -> &Registry {
        &self.registry
    }

    pub fn mode(&self) -> InputMode {
        self.mode
    }

    pub fn set_mode(&mut self, mode: InputMode) {
        self.mode = mode;
    }

    pub fn set_resize_step(&mut self, step: f32) {
        self.core.set_resize_step(step);
    }

    pub fn resize_step(&self) -> f32 {
        self.core.resize_step()
    }

    pub fn set_default_move_scope(&mut self, scope: ratatui_hypertile::MoveScope) {
        self.default_move_scope = scope;
    }

    pub fn move_bindings(&self) -> MoveBindings {
        self.move_bindings
    }

    pub fn set_move_bindings(&mut self, bindings: MoveBindings) {
        self.move_bindings = bindings;
    }

    pub fn split_behavior(&self) -> SplitBehavior {
        self.split_behavior
    }

    pub fn set_split_behavior(&mut self, behavior: SplitBehavior) {
        self.split_behavior = behavior;
    }

    pub fn border_config(&self) -> &BorderConfig {
        &self.border_config
    }

    pub fn set_border_config(&mut self, config: BorderConfig) {
        self.border_config = config;
    }

    pub fn animation_config(&self) -> AnimationConfig {
        self.animation_config
    }

    pub fn set_animation_config(&mut self, config: AnimationConfig) {
        self.animation_config = config;
        if !config.enabled {
            self.animation_state.clear();
        }
    }

    /// Tells your event loop when to draw again for an in-flight move animation.
    ///
    /// When this returns `None`, there is no pending animation work. The usual
    /// pattern is to combine this with your normal input timeout and redraw
    /// when whichever one happens first.
    pub fn next_frame_in(&self) -> Option<Duration> {
        self.animation_state
            .next_frame_in(Instant::now(), self.animation_config)
    }

    /// Registers a plugin under a string name so splits, palette actions, or
    /// explicit replacement calls can create it later.
    pub fn register_plugin_type<F, P>(&mut self, plugin_type: &str, factory: F)
    where
        F: Fn() -> P + 'static,
        P: HypertilePlugin + 'static,
    {
        self.registry.register_plugin_type(plugin_type, factory);
    }

    pub fn focused_pane(&self) -> Option<PaneId> {
        self.core.focused_pane()
    }

    pub fn focus_pane(&mut self, pane_id: PaneId) -> Result<(), RuntimeError> {
        self.core.focus_pane(pane_id)?;
        Ok(())
    }

    pub fn pane_rect(&self, pane_id: PaneId) -> Option<Rect> {
        self.core.pane_rect(pane_id)
    }

    pub fn panes(&self) -> Vec<PaneSnapshot> {
        self.core.panes()
    }

    /// Replaces the whole tree and remounts placeholder plugins where needed.
    ///
    /// Use this when you already have a layout tree you want the runtime to
    /// own. Any old animation state is dropped.
    pub fn set_root(&mut self, root: CoreNode) -> Result<(), RuntimeError> {
        self.core.set_root(root)?;
        self.animation_state.clear();
        self.sync_registry_to_core();
        Ok(())
    }

    pub fn reset(&mut self) {
        self.core.reset();
        self.animation_state.clear();
        self.sync_registry_to_core();
    }

    /// Splits the focused pane and mounts a fresh plugin instance in the new pane.
    pub fn split_focused(
        &mut self,
        direction: Direction,
        plugin_type: &str,
    ) -> Result<PaneId, RuntimeError> {
        let plugin = self.registry.instantiate_plugin(plugin_type)?;
        let pane_id = self.core.split_focused(direction)?;
        self.registry
            .mount_plugin_instance(pane_id, plugin_type, plugin);
        self.animation_state.clear();
        Ok(pane_id)
    }

    pub fn close_focused(&mut self) -> Result<PaneId, RuntimeError> {
        let removed_id = self.core.close_focused()?;
        self.registry.remove_plugin_if_exists(removed_id);
        self.animation_state.clear();
        Ok(removed_id)
    }

    /// Replaces the focused pane's plugin without changing the layout.
    pub fn replace_focused_plugin(&mut self, plugin_type: &str) -> Result<(), RuntimeError> {
        let Some(pane_id) = self.core.focused_pane() else {
            return Err(RuntimeError::NoFocusedPane);
        };

        let plugin = self.registry.instantiate_plugin(plugin_type)?;
        let _ = self.registry.remove_plugin_if_exists(pane_id);
        self.registry
            .mount_plugin_instance(pane_id, plugin_type, plugin);
        Ok(())
    }

    /// Replaces one pane's plugin by id.
    ///
    /// This also focuses that pane so follow-up layout commands keep working on
    /// the pane you just changed.
    pub fn replace_pane_plugin(
        &mut self,
        pane_id: PaneId,
        plugin_type: &str,
    ) -> Result<(), RuntimeError> {
        // Validate first to avoid partial state updates.
        let plugin = self.registry.instantiate_plugin(plugin_type)?;
        self.core.focus_pane(pane_id)?;
        let _ = self.registry.remove_plugin_if_exists(pane_id);
        self.registry
            .mount_plugin_instance(pane_id, plugin_type, plugin);
        Ok(())
    }

    pub fn set_focused_ratio(&mut self, ratio: f32) -> Result<(), RuntimeError> {
        self.core.set_focused_ratio(ratio)?;
        self.animation_state.clear();
        Ok(())
    }

    /// Handles one event and gives layout or registry errors back to the caller.
    ///
    /// Use this if your app wants to log failures or show them to the user
    /// instead of silently treating them as ignored input.
    pub fn try_handle_event(
        &mut self,
        event: HypertileEvent,
    ) -> Result<EventOutcome, RuntimeError> {
        if let Some(outcome) = self.handle_palette_event(&event) {
            return outcome;
        }

        match event {
            HypertileEvent::Action(action) => Ok(self.apply_core_action(action)),
            HypertileEvent::Tick => Ok(self.registry.broadcast_event(&HypertileEvent::Tick)),
            HypertileEvent::Key(chord) => {
                if chord.code == KeyCode::Escape && chord.modifiers.is_empty() {
                    if self.mode == InputMode::PluginInput {
                        self.mode = InputMode::Layout;
                        return Ok(EventOutcome::Consumed);
                    }
                    return Ok(EventOutcome::Ignored);
                }

                match self.mode {
                    InputMode::Layout => self.handle_layout_key(chord),
                    InputMode::PluginInput => {
                        Ok(self.forward_to_plugin(&HypertileEvent::Key(chord)))
                    }
                }
            }
        }
    }

    /// Like [`try_handle_event`](Self::try_handle_event), but turns errors into
    /// [`EventOutcome::Ignored`](ratatui_hypertile::EventOutcome::Ignored).
    pub fn handle_event(&mut self, event: HypertileEvent) -> EventOutcome {
        self.try_handle_event(event)
            .unwrap_or(EventOutcome::Ignored)
    }

    fn handle_layout_key(&mut self, chord: KeyChord) -> Result<EventOutcome, RuntimeError> {
        match self.default_layout_action(chord) {
            Some(RuntimeAction::Core(action)) => Ok(self.apply_core_action(action)),
            Some(RuntimeAction::SplitDefault(direction)) => self.handle_split_shortcut(direction),
            Some(RuntimeAction::OpenPalette) => self.open_palette(),
            Some(RuntimeAction::InteractFocused) => self.handle_interact_focused(),
            Some(RuntimeAction::EnterPluginInput) => {
                self.mode = InputMode::PluginInput;
                Ok(EventOutcome::Consumed)
            }
            None => Ok(EventOutcome::Ignored),
        }
    }

    fn handle_split_shortcut(
        &mut self,
        direction: Direction,
    ) -> Result<EventOutcome, RuntimeError> {
        match self.split_behavior {
            SplitBehavior::DefaultPlugin => {
                let plugin_type = self.default_split_plugin_type.clone();
                self.split_focused(direction, &plugin_type)?;
            }
            SplitBehavior::Placeholder => {
                self.split_focused(direction, DEFAULT_PLUGIN_TYPE)?;
            }
            SplitBehavior::PromptPalette => {
                let pane_id = self.split_focused(direction, DEFAULT_PLUGIN_TYPE)?;
                self.open_palette_for_target(Some(pane_id))?;
            }
        }
        Ok(EventOutcome::Consumed)
    }

    fn handle_interact_focused(&mut self) -> Result<EventOutcome, RuntimeError> {
        let Some(pane_id) = self.core.focused_pane() else {
            return Ok(EventOutcome::Ignored);
        };

        match self.registry.plugin_type_for(pane_id) {
            None | Some(DEFAULT_PLUGIN_TYPE) => self.open_palette_for_target(Some(pane_id)),
            Some(_) => {
                self.mode = InputMode::PluginInput;
                Ok(EventOutcome::Consumed)
            }
        }
    }

    fn forward_to_plugin(&mut self, event: &HypertileEvent) -> EventOutcome {
        let Some(pane_id) = self.core.focused_pane() else {
            return EventOutcome::Ignored;
        };
        let Some(plugin) = self.registry.plugin_mut(pane_id) else {
            return EventOutcome::Ignored;
        };
        plugin.on_event(event)
    }

    fn apply_core_action(&mut self, action: HypertileAction) -> EventOutcome {
        let can_animate = self.can_animate_action(action);
        let now = Instant::now();
        if can_animate {
            self.capture_displayed_rects(now);
        }

        let outcome = self.core.apply_action(action);
        if !outcome.is_consumed() {
            return outcome;
        }

        if can_animate {
            self.start_action_animation(now);
        } else if Self::action_changes_layout(action) {
            self.animation_state.clear();
        }

        outcome
    }

    fn can_animate_action(&self, action: HypertileAction) -> bool {
        self.animation_config.enabled
            && matches!(action, HypertileAction::MoveFocused { .. })
            && self.animation_state.last_area().is_some()
    }

    fn action_changes_layout(action: HypertileAction) -> bool {
        matches!(
            action,
            HypertileAction::SplitFocused { .. }
                | HypertileAction::CloseFocused
                | HypertileAction::ResizeFocused { .. }
                | HypertileAction::SetFocusedRatio { .. }
                | HypertileAction::MoveFocused { .. }
        )
    }

    fn capture_displayed_rects(&mut self, now: Instant) -> bool {
        let Some(area) = self.animation_state.last_area() else {
            return false;
        };
        self.core.compute_layout(area);
        self.animation_state
            .capture_before(area, self.core.state().panes(), now);
        true
    }

    fn start_action_animation(&mut self, now: Instant) {
        let Some(area) = self.animation_state.last_area() else {
            return;
        };

        self.core.compute_layout(area);
        self.animation_state
            .start(area, self.core.state().panes(), now, self.animation_config);
    }

    fn sync_registry_to_core(&mut self) {
        let keep: HashSet<PaneId> = raw::collect_pane_ids(self.core.root())
            .into_iter()
            .collect();
        self.registry.retain_only(&keep);

        for &pane_id in &keep {
            if self.registry.plugin(pane_id).is_none() {
                let _ = self.registry.spawn_plugin(DEFAULT_PLUGIN_TYPE, pane_id);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn set_root_syncs_placeholder_plugins_for_new_panes() {
        let mut runtime = HypertileRuntime::new();
        let tree = CoreNode::Split {
            direction: Direction::Horizontal,
            ratio: 0.5,
            first: Box::new(CoreNode::Pane(PaneId::ROOT)),
            second: Box::new(CoreNode::Pane(PaneId::new(7))),
        };

        runtime.set_root(tree).unwrap();

        assert_eq!(runtime.registry().instance_count(), 2);
        assert_eq!(
            runtime.registry().plugin_type_for(PaneId::ROOT),
            Some("block")
        );
        assert_eq!(
            runtime.registry().plugin_type_for(PaneId::new(7)),
            Some("block")
        );
    }
}
