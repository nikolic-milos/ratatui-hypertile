mod types;

pub use types::{PluginContext, RegistryError};

use ratatui::{buffer::Buffer, layout::Rect};
use ratatui_hypertile::{EventOutcome, HypertileEvent, PaneId};
use std::collections::{BTreeMap, HashMap, HashSet};

/// Trait implemented by pane-local plugins stored in [`Registry`].
pub trait HypertilePlugin {
    fn render(&self, area: Rect, buf: &mut Buffer, is_focused: bool);

    /// Handles an event forwarded by the runtime or registry.
    /// Return [`EventOutcome::Consumed`] to mark it handled.
    fn on_event(&mut self, _event: &HypertileEvent) -> EventOutcome {
        EventOutcome::Ignored
    }

    fn on_mount(&mut self, _ctx: PluginContext) {}

    fn on_unmount(&mut self, _ctx: PluginContext) {}

    /// Returns serializable state to include in persisted plugin snapshots.
    #[cfg(feature = "serde")]
    fn save_state(&self) -> Option<serde_json::Value> {
        None
    }

    /// Restores state previously returned by [`save_state`](Self::save_state).
    #[cfg(feature = "serde")]
    fn load_state(&mut self, _state: &serde_json::Value) -> Result<(), String> {
        Ok(())
    }
}

struct PluginInstance {
    plugin_type: String,
    plugin: Box<dyn HypertilePlugin>,
}

/// Stores plugin factories and mounted plugin instances keyed by pane id.
#[derive(Default)]
pub struct Registry {
    factories: BTreeMap<String, Box<dyn Fn() -> Box<dyn HypertilePlugin>>>,
    instances: HashMap<PaneId, PluginInstance>,
}

impl Registry {
    /// Registers a factory for `plugin_type`.
    /// Registering the same name again replaces the previous factory.
    pub fn register_plugin_type<F, P>(&mut self, plugin_type: &str, factory: F)
    where
        F: Fn() -> P + 'static,
        P: HypertilePlugin + 'static,
    {
        self.factories.insert(
            plugin_type.to_string(),
            Box::new(move || Box::new(factory())),
        );
    }

    /// Returns registered plugin type names in sorted order.
    pub fn registered_types(&self) -> impl Iterator<Item = &str> {
        self.factories.keys().map(String::as_str)
    }

    /// Creates a fresh plugin, calls `on_mount`, and stores it for `pane_id`.
    pub fn spawn_plugin(
        &mut self,
        plugin_type: &str,
        pane_id: PaneId,
    ) -> Result<(), RegistryError> {
        if self.instances.contains_key(&pane_id) {
            return Err(RegistryError::DuplicatePane(pane_id));
        }
        let mut plugin = self.instantiate_plugin(plugin_type)?;
        plugin.on_mount(PluginContext { pane_id });
        self.instances.insert(
            pane_id,
            PluginInstance {
                plugin_type: plugin_type.to_string(),
                plugin,
            },
        );
        Ok(())
    }

    /// Creates a plugin without mounting or storing it.
    pub fn instantiate_plugin(
        &self,
        plugin_type: &str,
    ) -> Result<Box<dyn HypertilePlugin>, RegistryError> {
        let factory = self
            .factories
            .get(plugin_type)
            .ok_or_else(|| RegistryError::UnknownPluginType(plugin_type.to_string()))?;
        Ok(factory())
    }

    /// Calls `on_mount` and stores an existing plugin instance for `pane_id`.
    /// If `pane_id` already has a plugin, the old instance is replaced.
    pub fn mount_plugin_instance(
        &mut self,
        pane_id: PaneId,
        plugin_type: &str,
        mut plugin: Box<dyn HypertilePlugin>,
    ) {
        plugin.on_mount(PluginContext { pane_id });
        self.instances.insert(
            pane_id,
            PluginInstance {
                plugin_type: plugin_type.to_string(),
                plugin,
            },
        );
    }

    /// Calls `on_unmount` and removes the plugin for `pane_id`.
    pub fn remove_plugin(&mut self, pane_id: PaneId) -> Result<(), RegistryError> {
        let Some(mut instance) = self.instances.remove(&pane_id) else {
            return Err(RegistryError::MissingPane(pane_id));
        };
        instance.plugin.on_unmount(PluginContext { pane_id });
        Ok(())
    }

    /// Returns `true` if a plugin was removed.
    pub fn remove_plugin_if_exists(&mut self, pane_id: PaneId) -> bool {
        self.remove_plugin(pane_id).is_ok()
    }

    /// Unmounts and removes every mounted plugin.
    pub fn clear(&mut self) {
        let pane_ids = self.instances.keys().copied().collect::<Vec<_>>();
        for pane_id in pane_ids {
            let _ = self.remove_plugin(pane_id);
        }
    }

    /// Unmounts any plugin whose pane id is not in `keep`.
    pub fn retain_only(&mut self, keep: &HashSet<PaneId>) {
        let to_remove: Vec<PaneId> = self
            .instances
            .keys()
            .filter(|pane_id| !keep.contains(pane_id))
            .copied()
            .collect();

        for pane_id in to_remove {
            let _ = self.remove_plugin(pane_id);
        }
    }

    pub fn plugin_type_for(&self, pane_id: PaneId) -> Option<&str> {
        self.instances
            .get(&pane_id)
            .map(|instance| instance.plugin_type.as_str())
    }

    pub fn plugin(&self, pane_id: PaneId) -> Option<&dyn HypertilePlugin> {
        self.instances
            .get(&pane_id)
            .map(|instance| instance.plugin.as_ref())
    }

    pub fn plugin_mut(&mut self, pane_id: PaneId) -> Option<&mut (dyn HypertilePlugin + 'static)> {
        self.instances
            .get_mut(&pane_id)
            .map(move |instance| instance.plugin.as_mut())
    }

    pub fn instance_count(&self) -> usize {
        self.instances.len()
    }

    /// Forwards `event` to every mounted plugin.
    /// Returns [`EventOutcome::Consumed`] if any plugin consumes it.
    pub fn broadcast_event(&mut self, event: &HypertileEvent) -> EventOutcome {
        let mut consumed = false;
        for instance in self.instances.values_mut() {
            if instance.plugin.on_event(event).is_consumed() {
                consumed = true;
            }
        }
        if consumed {
            EventOutcome::Consumed
        } else {
            EventOutcome::Ignored
        }
    }
}
