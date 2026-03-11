use std::collections::{HashMap, HashSet};

use ratatui_hypertile::{EventOutcome, HypertileEvent, PaneId};

use super::HypertilePlugin;
use super::types::PluginContext;

/// Typed registry for one plugin type.
///
/// ```
/// use ratatui::prelude::*;
/// use ratatui_hypertile::PaneId;
/// use ratatui_hypertile_extras::{HypertilePlugin, TypedRegistry};
///
/// struct Counter(u32);
/// impl HypertilePlugin for Counter {
///     fn render(&self, _area: Rect, _buf: &mut Buffer, _focused: bool) {}
/// }
///
/// let mut reg = TypedRegistry::new(|| Counter(0));
/// reg.spawn(PaneId::ROOT);
/// assert_eq!(reg.plugin(PaneId::ROOT).unwrap().0, 0);
/// ```
pub struct TypedRegistry<P> {
    factory: Box<dyn Fn() -> P>,
    instances: HashMap<PaneId, P>,
}

impl<P> TypedRegistry<P> {
    pub fn new(factory: impl Fn() -> P + 'static) -> Self {
        Self {
            factory: Box::new(factory),
            instances: HashMap::new(),
        }
    }

    pub fn plugin(&self, pane_id: PaneId) -> Option<&P> {
        self.instances.get(&pane_id)
    }

    pub fn plugin_mut(&mut self, pane_id: PaneId) -> Option<&mut P> {
        self.instances.get_mut(&pane_id)
    }

    pub fn instance_count(&self) -> usize {
        self.instances.len()
    }
}

impl<P: HypertilePlugin> TypedRegistry<P> {
    /// Gets or creates a plugin for `pane_id`.
    pub fn spawn(&mut self, pane_id: PaneId) -> &mut P {
        self.instances.entry(pane_id).or_insert_with(|| {
            let mut instance = (self.factory)();
            instance.on_mount(PluginContext { pane_id });
            instance
        })
    }

    /// Calls `on_unmount` and removes the plugin for `pane_id`.
    pub fn remove(&mut self, pane_id: PaneId) -> Option<P> {
        let mut instance = self.instances.remove(&pane_id)?;
        instance.on_unmount(PluginContext { pane_id });
        Some(instance)
    }

    /// Unmounts and removes every mounted plugin.
    pub fn clear(&mut self) {
        let pane_ids: Vec<PaneId> = self.instances.keys().copied().collect();
        for pane_id in pane_ids {
            if let Some(mut instance) = self.instances.remove(&pane_id) {
                instance.on_unmount(PluginContext { pane_id });
            }
        }
    }

    /// Unmounts any plugin whose pane id is not in `keep`.
    pub fn retain_only(&mut self, keep: &HashSet<PaneId>) {
        let to_remove: Vec<PaneId> = self
            .instances
            .keys()
            .filter(|id| !keep.contains(id))
            .copied()
            .collect();
        for pane_id in to_remove {
            if let Some(mut instance) = self.instances.remove(&pane_id) {
                instance.on_unmount(PluginContext { pane_id });
            }
        }
    }

    /// Forwards `event` to every mounted plugin.
    /// Returns [`EventOutcome::Consumed`] if any plugin consumes it.
    pub fn broadcast_event(&mut self, event: &HypertileEvent) -> EventOutcome {
        let mut consumed = false;
        for instance in self.instances.values_mut() {
            if instance.on_event(event).is_consumed() {
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
