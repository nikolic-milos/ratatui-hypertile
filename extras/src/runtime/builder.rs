use crate::registry::Registry;
use crate::runtime::constants::{
    DEFAULT_PALETTE_HEIGHT_PERCENT, DEFAULT_PALETTE_MAX_ITEMS, DEFAULT_PALETTE_WIDTH_PERCENT,
    DEFAULT_PLUGIN_TITLE, DEFAULT_PLUGIN_TYPE,
};
use crate::runtime::default_plugin::DefaultBlockPlugin;
use crate::runtime::palette::PaletteState;
use crate::runtime::{BorderConfig, HypertileRuntime, InputMode, MoveBindings, SplitBehavior};
use ratatui_hypertile::{HypertileBuilder as CoreBuilder, MoveScope, PaneId, SplitPolicy};

/// Builder for [`HypertileRuntime`](super::HypertileRuntime).
///
/// ```
/// use ratatui_hypertile_extras::{HypertileRuntimeBuilder, MoveBindings, SplitBehavior};
///
/// let runtime = HypertileRuntimeBuilder::default()
///     .with_move_bindings(MoveBindings::VimAndShiftArrows)
///     .with_split_behavior(SplitBehavior::PromptPalette)
///     .build();
/// ```
#[derive(Debug, Clone)]
pub struct HypertileRuntimeBuilder {
    pub(super) core_builder: CoreBuilder,
    pub(super) default_split_plugin_type: String,
    pub(super) palette_width_percent: u16,
    pub(super) palette_height_percent: u16,
    pub(super) palette_max_items: usize,
    pub(super) default_move_scope: MoveScope,
    pub(super) move_bindings: MoveBindings,
    pub(super) split_behavior: SplitBehavior,
    pub(super) border_config: BorderConfig,
}

impl Default for HypertileRuntimeBuilder {
    fn default() -> Self {
        Self {
            core_builder: CoreBuilder::default(),
            default_split_plugin_type: DEFAULT_PLUGIN_TYPE.to_string(),
            palette_width_percent: DEFAULT_PALETTE_WIDTH_PERCENT,
            palette_height_percent: DEFAULT_PALETTE_HEIGHT_PERCENT,
            palette_max_items: DEFAULT_PALETTE_MAX_ITEMS,
            default_move_scope: MoveScope::Window,
            move_bindings: MoveBindings::VimAndShiftArrows,
            split_behavior: SplitBehavior::Placeholder,
            border_config: BorderConfig::default(),
        }
    }
}

impl HypertileRuntimeBuilder {
    pub fn with_focus_highlight(mut self, enabled: bool) -> Self {
        self.core_builder = self.core_builder.with_focus_highlight(enabled);
        self
    }

    /// Plugin type used for splits when the palette is not used.
    pub fn with_default_split_plugin(mut self, plugin_type: &str) -> Self {
        self.default_split_plugin_type = plugin_type.to_string();
        self
    }

    /// Sets the ratio delta used by resize commands.
    /// Non-finite or non-positive values are ignored.
    pub fn with_resize_step(mut self, step: f32) -> Self {
        self.core_builder = self.core_builder.with_resize_step(step);
        self
    }

    pub fn with_split_policy(mut self, policy: SplitPolicy) -> Self {
        self.core_builder = self.core_builder.with_split_policy(policy);
        self
    }

    /// Sets palette size as percentages of the terminal area.
    /// Values are clamped to `10..=100`.
    pub fn with_palette_size(mut self, width_percent: u16, height_percent: u16) -> Self {
        self.palette_width_percent = width_percent.clamp(10, 100);
        self.palette_height_percent = height_percent.clamp(10, 100);
        self
    }

    /// Sets the maximum number of visible palette items.
    /// Values smaller than `1` are treated as `1`.
    pub fn with_palette_max_items(mut self, max_items: usize) -> Self {
        self.palette_max_items = max_items.max(1);
        self
    }

    /// Sets how move commands resolve by default.
    pub fn with_default_move_scope(mut self, scope: MoveScope) -> Self {
        self.default_move_scope = scope;
        self
    }

    pub fn with_move_bindings(mut self, move_bindings: MoveBindings) -> Self {
        self.move_bindings = move_bindings;
        self
    }

    /// Sets what happens after a split shortcut.
    pub fn with_split_behavior(mut self, behavior: SplitBehavior) -> Self {
        self.split_behavior = behavior;
        self
    }

    /// Sets pane border styles.
    pub fn with_border_config(mut self, config: BorderConfig) -> Self {
        self.border_config = config;
        self
    }

    pub fn with_gap(mut self, gap: u16) -> Self {
        self.core_builder = self.core_builder.with_gap(gap);
        self
    }

    /// Builds the runtime with a placeholder plugin in the root pane.
    pub fn build(self) -> HypertileRuntime {
        let core = self.core_builder.build();

        let mut registry = Registry::default();
        registry.register_plugin_type(DEFAULT_PLUGIN_TYPE, || {
            DefaultBlockPlugin::new(DEFAULT_PLUGIN_TITLE)
        });
        let _ = registry.spawn_plugin(DEFAULT_PLUGIN_TYPE, PaneId::ROOT);

        HypertileRuntime {
            core,
            registry,
            mode: InputMode::Layout,
            palette: PaletteState::with_config(
                self.palette_width_percent,
                self.palette_height_percent,
                self.palette_max_items,
            ),
            default_split_plugin_type: self.default_split_plugin_type,
            default_move_scope: self.default_move_scope,
            move_bindings: self.move_bindings,
            split_behavior: self.split_behavior,
            border_config: self.border_config,
        }
    }
}
