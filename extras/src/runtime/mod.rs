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
    EventOutcome, Hypertile as CoreHypertile, HypertileEvent, KeyChord, KeyCode, PaneId,
    PaneSnapshot, raw,
    raw::Node as CoreNode,
};
use std::collections::HashSet;

pub use builder::HypertileRuntimeBuilder;
#[cfg(feature = "crossterm")]
pub use crossterm::{event_from_crossterm, keychord_from_crossterm};
pub use keymap::MoveBindings;
pub use tab_bar::{TabBar, TabBarItem};
pub use types::{BorderConfig, InputMode, RuntimeError, SplitBehavior};
pub use widget::{HypertileView, ModeIndicator};
pub use workspace::{WorkspaceAction, WorkspaceRuntime};

use constants::DEFAULT_PLUGIN_TYPE;
use keymap::RuntimeAction;
use palette::PaletteState;

/// Core engine, plugins, modal input, and palette.
///
/// `i` enters plugin input mode, `ESC` returns to layout mode.
/// Use the core [`Hypertile`](ratatui_hypertile::Hypertile) directly
/// if you want custom input handling.
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
    default_split_plugin_type: String,
    default_move_scope: ratatui_hypertile::MoveScope,
    move_bindings: MoveBindings,
    split_behavior: SplitBehavior,
    border_config: BorderConfig,
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

    /// Replaces the tree and syncs plugins.
    pub fn set_root(&mut self, root: CoreNode) -> Result<(), RuntimeError> {
        self.core.set_root(root)?;
        self.sync_registry_to_core();
        Ok(())
    }

    pub fn reset(&mut self) {
        self.core.reset();
        self.sync_registry_to_core();
    }

    /// Splits the focused pane and mounts a new plugin.
    pub fn split_focused(
        &mut self,
        direction: Direction,
        plugin_type: &str,
    ) -> Result<PaneId, RuntimeError> {
        let plugin = self.registry.instantiate_plugin(plugin_type)?;
        let pane_id = self.core.split_focused(direction)?;
        self.registry
            .mount_plugin_instance(pane_id, plugin_type, plugin);
        Ok(pane_id)
    }

    pub fn close_focused(&mut self) -> Result<PaneId, RuntimeError> {
        let removed_id = self.core.close_focused()?;
        self.registry.remove_plugin_if_exists(removed_id);
        Ok(removed_id)
    }

    /// Unmounts the focused pane's current plugin and mounts a new `plugin_type`.
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

    /// Focuses `pane_id`, then replaces its plugin with a new `plugin_type` instance.
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
        Ok(())
    }

    /// Handles one event and returns any runtime error directly.
    pub fn try_handle_event(
        &mut self,
        event: HypertileEvent,
    ) -> Result<EventOutcome, RuntimeError> {
        if let Some(outcome) = self.handle_palette_event(&event) {
            return outcome;
        }

        match event {
            HypertileEvent::Action(action) => Ok(self.core.apply_action(action)),
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

    /// Like [`try_handle_event`](Self::try_handle_event), but returns `Ignored` on error.
    pub fn handle_event(&mut self, event: HypertileEvent) -> EventOutcome {
        self.try_handle_event(event)
            .unwrap_or(EventOutcome::Ignored)
    }

    fn handle_layout_key(&mut self, chord: KeyChord) -> Result<EventOutcome, RuntimeError> {
        match self.default_layout_action(chord) {
            Some(RuntimeAction::Core(action)) => Ok(self.core.apply_action(action)),
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

    fn sync_registry_to_core(&mut self) {
        let keep: HashSet<PaneId> = raw::collect_pane_ids(self.core.root()).into_iter().collect();
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
