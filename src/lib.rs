//! Headless wheel menu library for Bevy.
//!
//! This library provides the logic and data structures for wheel menus.
//! Rendering is left to the application.

pub mod editor;

use bevy::color::Alpha;
use bevy::prelude::*;
use bevy::render::render_resource::{AsBindGroup, ShaderType};
use bevy::shader::ShaderRef;
use leafwing_input_manager::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Default filename for the persisted [`QuickActionConfig`].
/// Resolved relative to the process working directory (the project root when
/// running via `cargo run`).
pub const CONFIG_FILE: &str = "quickactions_config.ron";

// ─── gamepad icon set ─────────────────────────────────────────────────────────────

/// Which family of controller button icons to show in the editor UI.
///
/// Auto-detected from the connected gamepad's USB vendor/product IDs and device
/// name when a controller connects; defaults to [`GamepadIconSet::Xbox`] when no
/// controller is present or the type is unknown.
///
/// Icon assets live under `assets/icons/<set>/Default/`.
#[derive(Resource, Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum GamepadIconSet {
    /// Xbox / generic controller — A / B / X / Y / LB / RB / LT / RT …
    #[default]
    Xbox,
    /// PlayStation 4 DualShock 4 — Cross / Circle / Square / Triangle …
    PS4,
    /// PlayStation 5 DualSense — Cross / Circle / Square / Triangle …
    PS5,
    /// Nintendo Switch Pro Controller / Joy-Con — B / A / Y / X / + / − …
    Switch,
}

impl GamepadIconSet {
    /// Base asset path (relative to `assets/`) for this set's Default style.
    pub fn base_path(self) -> &'static str {
        match self {
            Self::Xbox => "icons/XGamepad/Default",
            Self::PS4 => "icons/P4Gamepad/Default",
            Self::PS5 => "icons/P5Gamepad/Default",
            Self::Switch => "icons/SGamepad/Default",
        }
    }

    /// Returns the asset path for a gamepad button label (e.g. `"LB"`, `"A"`,
    /// `"Start"`) as produced by [`editor::gamepad_btn_label`].
    ///
    /// Returns `None` when the label has no mapped asset for this set.
    pub fn icon_path(self, label: &str) -> Option<String> {
        let base = self.base_path();
        let file: &str = match self {
            Self::Xbox => match label {
                "A" => "T_X_A_Color.png",
                "B" => "T_X_B_Color.png",
                "X" => "T_X_X_Color.png",
                "Y" => "T_X_Y_Color.png",
                "LB" => "T_X_LB.png",
                "RB" => "T_X_RB.png",
                "LT" => "T_X_LT.png",
                "RT" => "T_X_RT.png",
                "Start" => "T_X_Share.png",
                "Select" => "T_X_Share-1.png",
                "LS" => "T_X_Left_Stick_Click.png",
                "RS" => "T_X_Right_Stick_Click.png",
                "DUp" => "T_X_Dpad_Up.png",
                "DDown" => "T_X_Dpad_Down.png",
                "DLeft" => "T_X_Dpad_Left.png",
                "DRight" => "T_X_Dpad_Right.png",
                _ => return None,
            },
            Self::PS4 => match label {
                "A" => "T_P4_Cross.png",
                "B" => "T_P4_Circle.png",
                "X" => "T_P4_Square.png",
                "Y" => "T_P4_Triangle.png",
                "LB" => "T_P4_L1.png",
                "RB" => "T_P4_R1.png",
                "LT" => "T_P4_L2.png",
                "RT" => "T_P4_R2.png",
                "Start" => "T_P4_Options.png",
                "Select" => "T_P4_Share.png",
                "LS" => "T_P4_Left_Stick_Click.png",
                "RS" => "T_P4_Right_Stick_Click.png",
                "DUp" => "T_P4_Dpad_UP.png",
                "DDown" => "T_P4_Dpad_Down.png",
                "DLeft" => "T_P4_Dpad_Left.png",
                "DRight" => "T_P4_Dpad_Right.png",
                _ => return None,
            },
            Self::PS5 => match label {
                "A" => "T_P5_Cross.png",
                "B" => "T_P5_Circle.png",
                "X" => "T_P5_Square.png",
                "Y" => "T_P5_Triangle.png",
                "LB" => "T_P5_L1.png",
                "RB" => "T_P5_R1.png",
                "LT" => "T_P5_L2.png",
                "RT" => "T_P5_R2.png",
                "Start" => "T_P5_Options.png",
                "Select" => "T_P5_Share.png",
                "LS" => "T_P5_Left_Stick_Click_Alt.png",
                "RS" => "T_P5_Right_Stick_Click_Alt.png",
                "DUp" => "T_P5_Dpad_UP.png",
                "DDown" => "T_P5_Dpad_Down.png",
                "DLeft" => "T_P5_Dpad_Left.png",
                "DRight" => "T_P5_Dpad_Right.png",
                _ => return None,
            },
            Self::Switch => match label {
                // Nintendo physical layout: South=B, East=A, West=Y, North=X
                "A" => "T_S_B.png",
                "B" => "T_S_A.png",
                "X" => "T_S_Y.png",
                "Y" => "T_S_X.png",
                "LB" => "T_S_LB.png",
                "RB" => "T_S_RB.png",
                "LT" => "T_S_LT.png",
                "RT" => "T_S_RT.png",
                "Start" => "T_S_Plus.png",
                "Select" => "T_S_Minus.png",
                "LS" => "T_S_L.png",
                "RS" => "T_S_R.png",
                "DUp" => "T_S_Dpad_Up.png",
                "DDown" => "T_S_Dpad_Down.png",
                "DLeft" => "T_S_Dpad_Left.png",
                "DRight" => "T_S_Dpad_Right.png",
                _ => return None,
            },
        };
        Some(format!("{}/{}", base, file))
    }

    /// Detect icon set from USB vendor / product IDs reported by gilrs.
    pub fn from_ids(vendor: Option<u16>, product: Option<u16>) -> Self {
        match (vendor, product) {
            (Some(0x054C), Some(0x0CE6)) => Self::PS5, // DualSense
            (Some(0x054C), _) => Self::PS4,            // Other Sony
            (Some(0x057E), _) => Self::Switch,         // Nintendo
            (Some(0x045E), _) => Self::Xbox,           // Microsoft
            _ => Self::Xbox,
        }
    }

    /// Detect icon set from the controller's human-readable name string.
    pub fn from_name(name: &str) -> Self {
        let n = name.to_lowercase();
        if n.contains("dualsense") || n.contains("ps5") {
            Self::PS5
        } else if n.contains("dualshock") || n.contains("ps4") || n.contains("ps3") {
            Self::PS4
        } else if n.contains("switch")
            || n.contains("joy-con")
            || n.contains("joycon")
            || n.contains("pro controller")
            || n.contains("nintendo")
        {
            Self::Switch
        } else {
            Self::Xbox
        }
    }
}

/// System: updates [`GamepadIconSet`] when a gamepad is detected.
/// USB IDs take priority; controller name is used as a fallback.
fn detect_gamepad_icon_set(
    added: Query<(&Gamepad, Option<&Name>), Added<Gamepad>>,
    mut icon_set: ResMut<GamepadIconSet>,
) {
    for (gamepad, name) in &added {
        let by_id = GamepadIconSet::from_ids(gamepad.vendor_id(), gamepad.product_id());
        *icon_set = if by_id != GamepadIconSet::Xbox {
            by_id
        } else {
            // IDs inconclusive — try device name
            name.map(|n| GamepadIconSet::from_name(n.as_str()))
                .unwrap_or(GamepadIconSet::Xbox)
        };
        break; // first connected controller wins
    }
}

/// Leafwing-backed input actions for gamepad wheel navigation.
///
/// Attach an `InputMap<WheelNavAction>` + `ActionState<WheelNavAction>` to an entity
/// (done automatically by `WheelMenuPlugin` via `setup_wheel_nav_input`), then read
/// `ActionState` to drive `WheelState`.
#[derive(Actionlike, Reflect, Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum WheelNavAction {
    /// Right-stick direction — selects the hovered segment.
    #[actionlike(DualAxis)]
    Navigate,
    /// Confirm / select hovered segment (South / A).
    Confirm,
    /// Cancel / close wheel (East / B).
    Cancel,
    /// Cycle active item forward in the hovered slot (North / Y).
    CycleForward,
    /// Cycle active item backward in the hovered slot (West / X).
    CycleBack,
}

/// Marks a slice within a wheel menu.
#[derive(Component, Clone)]
pub struct WheelSlice {
    /// Index of this slice (0-based).
    pub index: usize,
}

/// Optional content for a wheel slice.
#[derive(Component, Clone, Default)]
pub struct WheelSliceContent {
    /// Label text for this slice.
    pub label: Option<String>,
    /// Icon path/identifier for this slice.
    pub icon: Option<String>,
}

/// Current input state of a wheel menu.
#[derive(Component, Default, Clone)]
pub struct WheelState {
    /// Current input direction (normalized).
    pub dir: Vec2,
    /// Currently hovered slice index.
    pub hovered: Option<usize>,
    /// Whether the wheel was open (any slice hovered) last frame.
    /// Updated by [`emit_lifecycle`]; do not write manually.
    pub open: bool,
}

/// Message sent when a slice is selected.
#[derive(Message, Clone)]
pub struct WheelMenuSelected {
    /// Index of the selected slice.
    pub index: usize,
    /// Entity of the wheel menu.
    pub menu_entity: Entity,
}

/// Marker for the hover state changing.
#[derive(Message, Clone)]
pub struct WheelMenuHoverChanged {
    /// Previously hovered slice (if any).
    pub previous: Option<usize>,
    /// Currently hovered slice (if any).
    pub current: Option<usize>,
    /// Entity of the wheel menu.
    pub menu_entity: Entity,
}

// ─── action data model ─────────────────────────────────────────────────────

/// Trait for game-specific actions stored in a [`WheelSlot`] and executed when
/// [`ActionTriggered`] fires.  Implement this to add custom behaviour.
pub trait ActionBehavior: Send + Sync + 'static {
    fn execute(&self, commands: &mut Commands);
    fn label(&self) -> &str;
    fn icon(&self) -> &str;
}

/// The kind of item a wheel slot can hold.
///
/// Use [`ActionItem::Custom`] with a boxed [`ActionBehavior`] for any
/// game-specific item not covered by the built-in variants.
pub enum ActionItem {
    Weapon {
        name: String,
        icon: String,
    },
    Spell {
        name: String,
        icon: String,
    },
    Consumable {
        name: String,
        icon: String,
        count: u32,
    },
    Shout {
        name: String,
        icon: String,
    },
    Custom(Box<dyn ActionBehavior>),
}

impl ActionItem {
    pub fn label(&self) -> &str {
        match self {
            Self::Weapon { name, .. }
            | Self::Spell { name, .. }
            | Self::Shout { name, .. }
            | Self::Consumable { name, .. } => name,
            Self::Custom(b) => b.label(),
        }
    }
    pub fn icon(&self) -> &str {
        match self {
            Self::Weapon { icon, .. }
            | Self::Spell { icon, .. }
            | Self::Shout { icon, .. }
            | Self::Consumable { icon, .. } => icon,
            Self::Custom(b) => b.icon(),
        }
    }
}

/// A wheel slot that can hold **multiple** [`ActionItem`]s.
///
/// The player cycles through items with a button (right/left thumbstick press
/// by default via [`update_slot_cycle`]) without leaving the hovered slice.
/// Attach alongside [`WheelSlice`] on the slice entity.
#[derive(Component)]
pub struct WheelSlot {
    pub items: Vec<ActionItem>,
    /// Index of the currently displayed / active item.
    pub current_item: usize,
}

impl WheelSlot {
    pub fn new(items: Vec<ActionItem>) -> Self {
        Self {
            items,
            current_item: 0,
        }
    }
    pub fn current(&self) -> Option<&ActionItem> {
        self.items.get(self.current_item)
    }
    pub fn cycle_next(&mut self) {
        if self.items.is_empty() {
            return;
        }
        self.current_item = (self.current_item + 1) % self.items.len();
    }
    pub fn cycle_prev(&mut self) {
        if self.items.is_empty() {
            return;
        }
        self.current_item = (self.current_item + self.items.len() - 1) % self.items.len();
    }
}

// ─── config enums ─────────────────────────────────────────────────────────────────

/// How the wheel interacts with game time while it is open.
///
/// Exactly **one** variant is active per wheel entity.
#[derive(Clone, Default, Serialize, Deserialize, PartialEq, Debug)]
pub enum TimeMode {
    /// No time manipulation.
    #[default]
    Normal,
    /// Slow [`Time<Virtual>`] to `scale` (0.0 = pause, 1.0 = real time).
    Slow(f32),
    /// Fully pause virtual time.
    Pause,
}

/// How an action is confirmed and triggered from the wheel.
///
/// Using an enum guarantees exactly **one** mode is active; there are no
/// conflicting flag combinations.
#[derive(Clone, Default, Serialize, Deserialize, PartialEq, Debug)]
pub enum CastingMode {
    /// Standard: press the confirm button (South/A) while hovering a slice.
    #[default]
    Vanilla,
    /// Release-to-Use: release the stick from a hovered slice to select it.
    ReleaseToUse,
    /// Dwell on a slice for `duration` seconds to trigger it.
    HoldToActivate { duration: f32 },
    /// Activate immediately when a new slice is hovered (no confirm step).
    Direct,
}

/// How the wheel opens and closes.
#[derive(Clone, Default, Serialize, Deserialize, PartialEq, Debug)]
pub enum WheelToggleMode {
    /// Press the hotkey once to open; press again to close.
    Toggle,
    /// Hold the key / button to keep open; release to close.
    #[default]
    Hold,
    /// Short press → toggle mode; long press → hold mode.
    Hybrid { hold_threshold_secs: f32 },
}

// ─── wheel config component ───────────────────────────────────────────────────────

/// Full configuration for a wheel menu.
///
/// Attach alongside [`WheelData`] and [`WheelState`].  Enum variants guarantee
/// **conflict-free** feature combinations: only one `TimeMode` and one
/// `CastingMode` can be active at a time.
#[derive(Component, Clone)]
pub struct WheelMenuConfig {
    /// Controls whether / how game time is slowed while the wheel is alive.
    pub time_mode: TimeMode,
    /// Determines how items are confirmed and activated.
    pub casting_mode: CastingMode,
    /// Controls how the wheel opens and closes (informational for the app;
    /// the library does not manage spawn/despawn lifecycle).
    pub toggle_mode: WheelToggleMode,
    /// Reset hover to `None` when the stick enters the deadzone.
    pub auto_snap: bool,
    /// When `true`, emit an event so the app can suppress movement / combat
    /// while the wheel is visible.
    pub block_gameplay_input: bool,
}

impl Default for WheelMenuConfig {
    fn default() -> Self {
        Self {
            time_mode: TimeMode::Normal,
            casting_mode: CastingMode::Vanilla,
            toggle_mode: WheelToggleMode::Hold,
            auto_snap: true,
            block_gameplay_input: false,
        }
    }
}

/// Tracks hold-activation progress for a wheel.  Attach when using
/// [`CastingMode::HoldToActivate`].
/// Read [`WheelHoldState::progress`] (0.0–1.0) to drive a progress indicator.
#[derive(Component, Default, Clone)]
pub struct WheelHoldState {
    /// Elapsed hold fraction on the current slice (0.0 = just started,
    /// 1.0 = threshold reached).
    pub progress: f32,
    /// Whether the player is currently dwelling on a slice.
    pub holding: bool,
}

/// Tracks multiple wheels in a wheel-set and which one is active.
///
/// Attach to the wheel-set entity to let the player cycle between wheels with
/// shoulder buttons.  The active index wraps around automatically.
#[derive(Component, Clone)]
pub struct WheelSet {
    /// Index of the currently active wheel (0-based).
    pub active: usize,
    /// Total number of wheels in this set.
    pub count: usize,
    /// Gamepad button that cycles to the previous wheel.
    pub prev_button: GamepadButton,
    /// Gamepad button that cycles to the next wheel.
    pub next_button: GamepadButton,
}

impl Default for WheelSet {
    fn default() -> Self {
        Self {
            active: 0,
            count: 1,
            prev_button: GamepadButton::LeftTrigger,
            next_button: GamepadButton::RightTrigger,
        }
    }
}

/// Item-count data for a single slice.  When `current` drops to or below
/// `low_threshold` the library emits [`WheelMenuLowCount`] once.
#[derive(Component, Clone, Default)]
pub struct WheelSliceCount {
    /// Current item count.
    pub current: u32,
    /// Maximum item count (used for display; does not affect logic).
    pub max: u32,
    /// Emit [`WheelMenuLowCount`] when `current <= low_threshold`.
    pub low_threshold: u32,
    /// Internal: whether the low-count event has already been fired for the
    /// current low state.  Resets when `current` rises above the threshold.
    pub low_notified: bool,
}

/// Edit-mode state for a wheel.  While active the player can reorder slices
/// with D-pad Up/Down.
#[derive(Component, Default, Clone)]
pub struct WheelEditMode {
    /// Whether edit mode is currently active.
    pub active: bool,
    /// Optional gamepad button that toggles edit mode.
    pub toggle_button: Option<GamepadButton>,
}

// ─── visuals, audio & hierarchy ────────────────────────────────────────────────

/// Visual configuration / skin for a wheel.  Attach to a wheel entity; the
/// renderer (application side) reads these colors when building slice panels.
#[derive(Component, Clone, Serialize, Deserialize, PartialEq, Debug)]
pub struct WheelStyle {
    /// Base slice color.
    pub base_color: [f32; 4],
    /// Color of the hovered slice.
    pub hover_color: [f32; 4],
    /// Color of the active / equipped slice.
    pub selected_color: [f32; 4],
    /// Label / icon text color.
    pub text_color: [f32; 4],
    /// Named skin identifier the app can switch on (e.g. "dark", "neon").
    pub skin: String,
}

impl Default for WheelStyle {
    fn default() -> Self {
        Self {
            base_color: [0.08, 0.12, 0.18, 0.85],
            hover_color: [0.2, 0.5, 0.9, 0.95],
            selected_color: [0.1, 0.7, 0.4, 0.9],
            text_color: [0.85, 0.88, 0.92, 1.0],
            skin: "default".into(),
        }
    }
}

impl WheelStyle {
    /// Convert the stored base color into a Bevy [`Color`].
    pub fn base(&self) -> Color {
        Color::srgba(
            self.base_color[0],
            self.base_color[1],
            self.base_color[2],
            self.base_color[3],
        )
    }
    /// Convert the stored hover color into a Bevy [`Color`].
    pub fn hover(&self) -> Color {
        Color::srgba(
            self.hover_color[0],
            self.hover_color[1],
            self.hover_color[2],
            self.hover_color[3],
        )
    }
    /// Convert the stored selected color into a Bevy [`Color`].
    pub fn selected(&self) -> Color {
        Color::srgba(
            self.selected_color[0],
            self.selected_color[1],
            self.selected_color[2],
            self.selected_color[3],
        )
    }
    /// Convert the stored text color into a Bevy [`Color`].
    pub fn text(&self) -> Color {
        Color::srgba(
            self.text_color[0],
            self.text_color[1],
            self.text_color[2],
            self.text_color[3],
        )
    }
}

/// Sound asset paths a wheel plays on lifecycle events.  Attach to a wheel
/// entity; [`play_wheel_audio`] turns the library's lifecycle messages into
/// `AudioPlayer` spawns when an [`AssetServer`] is available.
#[derive(Component, Clone, Default, Serialize, Deserialize, PartialEq, Debug)]
pub struct WheelAudio {
    /// Played when the wheel opens.
    pub open: Option<String>,
    /// Played when the hovered slice changes.
    pub hover: Option<String>,
    /// Played when a slice is selected.
    pub select: Option<String>,
    /// Played when a submenu is entered.
    pub submenu: Option<String>,
}

/// Parent/child links for nested submenu wheels.  Attach to a wheel entity to
/// describe its position in the wheel hierarchy.
#[derive(Component, Clone, Default)]
pub struct WheelHierarchy {
    /// Parent wheel entity, if this is a submenu.
    pub parent: Option<Entity>,
    /// Child submenu wheel entities, indexed by the slice that opens them.
    pub children: Vec<Entity>,
}

// ─── contextual input override system ──────────────────────────────────────────

/// Abstract input the player can press, independent of the physical device.
///
/// Map raw gamepad buttons / keys onto these so the same binding table works
/// across devices.  Used as the key of [`WheelInputOverride::bindings`] and
/// [`GlobalBindings::bindings`].
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub enum InputAction {
    /// Primary confirm (South / A).
    PrimaryConfirm,
    /// Secondary (East / B).
    Secondary,
    /// West / X face button.
    ButtonX,
    /// North / Y face button.
    ButtonY,
    /// Cycle to the next item in a multi-item slot.
    CycleNext,
    /// Cycle to the previous item in a multi-item slot.
    CyclePrev,
    /// An application-defined input slot.
    Custom(u32),
}

/// A resolved action produced by [`resolve_input`].
#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub enum WheelAction {
    /// Use the active item in the hovered slot.
    UseSlot,
    /// Use a specific item index inside the hovered slot.
    UseItem(usize),
    /// Cycle the hovered slot's active item.
    CycleItem { forward: bool },
    /// Open the submenu attached to the hovered slot.
    OpenSubmenu,
    /// A named, application-defined action (fallback bucket).
    Named(String),
}

/// Per-slot or per-wheel input binding table.
///
/// Placed on a **slice** entity it overrides wheel- and global-level bindings
/// while that slice is the [`ActiveSlotContext`].  Placed on a **wheel** entity
/// it overrides only the global bindings.  Priority order, highest first:
///
/// 1. Active slot override  2. Wheel override  3. [`GlobalBindings`]
#[derive(Component, Clone, Default, Serialize, Deserialize)]
pub struct WheelInputOverride {
    /// Input → action lookup table.
    pub bindings: HashMap<InputAction, WheelAction>,
    /// Reserved for future tie-breaking between same-level overrides.
    pub priority: u8,
}

/// Records which slice entity is currently active (selected) on a wheel so its
/// [`WheelInputOverride`] takes priority.  Maintained automatically by
/// [`update_active_slot_context`] for slices carrying a [`WheelSliceLink`].
#[derive(Component, Clone, Copy)]
pub struct ActiveSlotContext {
    /// The slice entity whose overrides are active.
    pub slot_entity: Entity,
}

/// Links a slice entity back to its owning wheel entity so the library can
/// auto-maintain [`ActiveSlotContext`].  Attach alongside [`WheelSlice`].
#[derive(Component, Clone, Copy)]
pub struct WheelSliceLink {
    /// The wheel (root) entity that owns this slice.
    pub menu: Entity,
}

/// Lowest-priority, application-wide fallback bindings.
#[derive(Resource, Clone, Default)]
pub struct GlobalBindings {
    /// Input → action lookup table consulted when no override matches.
    pub bindings: HashMap<InputAction, WheelAction>,
}

/// Resolves a single [`InputAction`] against the slot → wheel → global priority
/// chain.  Returns the first matching [`WheelAction`], or `None` if unbound.
pub fn resolve_input(
    input: InputAction,
    active_slot: Option<&WheelInputOverride>,
    wheel: Option<&WheelInputOverride>,
    global: &GlobalBindings,
) -> Option<WheelAction> {
    if let Some(slot) = active_slot {
        if let Some(action) = slot.bindings.get(&input) {
            return Some(action.clone());
        }
    }
    if let Some(w) = wheel {
        if let Some(action) = w.bindings.get(&input) {
            return Some(action.clone());
        }
    }
    global.bindings.get(&input).cloned()
}

/// Emitted whenever a pressed input resolves to a [`WheelAction`] through the
/// contextual override chain.  Applications listen here to run the action.
#[derive(Message, Clone)]
pub struct WheelActionResolved {
    /// The input that was pressed.
    pub input: InputAction,
    /// The action it resolved to.
    pub action: WheelAction,
    /// The wheel the action applies to.
    pub menu_entity: Entity,
}

/// Default gamepad-button → [`InputAction`] mapping used by
/// [`resolve_wheel_input`].
const DEFAULT_BUTTON_MAP: &[(GamepadButton, InputAction)] = &[
    (GamepadButton::South, InputAction::PrimaryConfirm),
    (GamepadButton::East, InputAction::Secondary),
    (GamepadButton::West, InputAction::ButtonX),
    (GamepadButton::North, InputAction::ButtonY),
    (GamepadButton::RightThumb, InputAction::CycleNext),
    (GamepadButton::LeftThumb, InputAction::CyclePrev),
];

// ─── additional messages ──────────────────────────────────────────────────────

/// Emitted when the active wheel in a [`WheelSet`] changes.
#[derive(Message, Clone)]
pub struct WheelSwitched {
    /// Previously active wheel index.
    pub previous: usize,
    /// Newly active wheel index.
    pub current: usize,
    /// Entity of the wheel-set.
    pub menu_entity: Entity,
}

/// Emitted every frame while the player holds the stick on a slice (when
/// [`CastingMode::HoldToActivate`] is active).
#[derive(Message, Clone)]
pub struct WheelMenuHoldProgress {
    /// Slice being held.
    pub index: usize,
    /// Fraction complete (0.0 → 1.0).  Drive a circular fill indicator with this.
    pub progress: f32,
    /// Entity of the wheel menu.
    pub menu_entity: Entity,
}

/// Emitted once when hold-activation completes (progress reaches 1.0).
#[derive(Message, Clone)]
pub struct WheelMenuHoldActivated {
    /// Slice that was held.
    pub index: usize,
    /// Entity of the wheel menu.
    pub menu_entity: Entity,
}

/// Emitted once each time a slice's item count drops to or below its threshold.
#[derive(Message, Clone)]
pub struct WheelMenuLowCount {
    /// Slice index (matching [`WheelSlice::index`]).
    pub index: usize,
    /// Current count at the time of emission.
    pub current: u32,
    /// The threshold that was crossed.
    pub threshold: u32,
    /// Entity of the slice that carries [`WheelSliceCount`].
    pub slice_entity: Entity,
}

/// Emitted when edit mode is toggled.
#[derive(Message, Clone)]
pub struct WheelEditModeChanged {
    /// Whether edit mode is now active.
    pub active: bool,
    /// Entity of the wheel menu.
    pub menu_entity: Entity,
}

/// Emitted (in edit mode) when the player requests a slice reorder.
/// The application is responsible for actually swapping slice data.
#[derive(Message, Clone)]
pub struct WheelSliceReorder {
    /// Index of the slice to move.
    pub from_index: usize,
    /// Target position.
    pub to_index: usize,
    /// Entity of the wheel menu.
    pub menu_entity: Entity,
}

// ─── lifecycle & action messages ─────────────────────────────────────────────────────

/// Emitted the first frame the stick leaves the deadzone (wheel conceptually
/// opened). Useful for playing a sound or animating the wheel in.
#[derive(Message, Clone)]
pub struct WheelOpened {
    pub menu_entity: Entity,
}

/// Emitted when the stick returns to centre after having hovered a slice
/// (wheel conceptually closed). Useful for hiding the overlay.
#[derive(Message, Clone)]
pub struct WheelClosed {
    pub menu_entity: Entity,
}

/// Emitted when the player confirms a slice selection (all casting modes).
/// This is the single normalised "something was chosen" signal.
#[derive(Message, Clone)]
pub struct SlotSelected {
    pub slot_index: usize,
    pub menu_entity: Entity,
}

/// Emitted alongside [`SlotSelected`] as a prompt to execute the action.
/// Applications listen here to call [`ActionBehavior::execute`] or apply
/// game-specific effects.
#[derive(Message, Clone)]
pub struct ActionTriggered {
    pub slot_index: usize,
    pub menu_entity: Entity,
}

/// Emitted when the active item inside a [`WheelSlot`] is cycled.
#[derive(Message, Clone)]
pub struct WheelSlotItemChanged {
    pub slot_index: usize,
    pub previous_item: usize,
    pub current_item: usize,
    pub menu_entity: Entity,
}

// ─── plugin ───────────────────────────────────────────────────────────────────

/// Emitted by [`hud_stick_nav`] when the player releases the right stick while
/// [`WheelHudState::open`] is `true` (release-to-use selection).
#[derive(Message, Clone, Debug)]
pub struct HudSegmentSelected {
    /// Index of the active [`ActionSet`].
    pub set: usize,
    /// Index of the entry within that set (a [`SetEntry::Wheel`] or the
    /// first wheel inside a [`SetEntry::WheelSet`]).
    pub entry: usize,
    /// Wheel index inside a `WheelSet`, `None` for a bare `Wheel`.
    pub wheel: Option<usize>,
    /// Slot index within the wheel.
    pub slot: usize,
}

/// The unified wheel-menu plugin.
///
/// | Constructor | Core wheel logic | HUD canvas | Editor sidebar |
/// |---|---|---|---|
/// | `QuickActionHudPlugin::core()` | ✓ | | |
/// | `QuickActionHudPlugin::default()` | ✓ | ✓ | |
/// | `QuickActionHudPlugin::with_editor()` | ✓ | ✓ | ✓ |
///
/// The **core** systems are the full wheel input / hover / selection / hold
/// pipeline (everything that was in the old `WheelMenuPlugin`).
/// The **HUD canvas** renders [`QuickActionConfig`]-driven wheels and action
/// buttons on screen.
/// The **editor sidebar** adds the in-app config editor.
pub struct QuickActionHudPlugin {
    /// Render the [`QuickActionConfig`]-driven HUD canvas.
    pub hud: bool,
    /// Enable the in-app editor sidebar.
    pub editor: bool,
}

impl Default for QuickActionHudPlugin {
    fn default() -> Self {
        Self {
            hud: true,
            editor: false,
        }
    }
}

impl QuickActionHudPlugin {
    /// Core wheel systems only — no HUD canvas, no editor.
    /// Drop-in replacement for the old `WheelMenuPlugin` when you manage your
    /// own rendering.
    pub fn core() -> Self {
        Self {
            hud: false,
            editor: false,
        }
    }

    /// Full HUD canvas **with** the editor sidebar enabled.
    pub fn with_editor() -> Self {
        Self {
            hud: true,
            editor: true,
        }
    }
}

impl Plugin for QuickActionHudPlugin {
    fn build(&self, app: &mut App) {
        // ── core wheel input / logic ──────────────────────────────────────────
        app.add_plugins(InputManagerPlugin::<WheelNavAction>::default())
            .init_resource::<GlobalBindings>()
            // selection messages
            .add_message::<WheelMenuSelected>()
            .add_message::<WheelMenuHoverChanged>()
            // lifecycle messages
            .add_message::<WheelOpened>()
            .add_message::<WheelClosed>()
            // action messages
            .add_message::<SlotSelected>()
            .add_message::<ActionTriggered>()
            .add_message::<WheelSlotItemChanged>()
            .add_message::<WheelActionResolved>()
            // wheel-set messages
            .add_message::<WheelSwitched>()
            // hold messages
            .add_message::<WheelMenuHoldProgress>()
            .add_message::<WheelMenuHoldActivated>()
            // misc
            .add_message::<WheelMenuLowCount>()
            .add_message::<WheelEditModeChanged>()
            .add_message::<WheelSliceReorder>()
            .add_systems(Startup, setup_wheel_nav_input)
            .add_systems(
                Update,
                (
                    update_wheel_input,
                    update_wheel_hover,
                    emit_selection,
                    emit_lifecycle,
                    update_slot_cycle,
                    update_wheel_time_scale,
                    update_wheel_hold,
                    update_wheel_set,
                    check_low_counts,
                    update_edit_mode,
                    update_active_slot_context,
                    resolve_wheel_input,
                )
                    .chain(),
            );

        // ── HUD canvas ────────────────────────────────────────────────────────
        if self.hud {
            app.add_plugins(UiMaterialPlugin::<WedgeMaterial>::default())
                .init_resource::<QuickActionConfig>()
                .init_resource::<WheelHudState>()
                .init_resource::<GamepadIconSet>()
                .add_message::<HudSegmentSelected>()
                .add_systems(PostStartup, try_autoload_config)
                .add_systems(
                    Update,
                    (
                        detect_gamepad_icon_set,
                        hud_button_feedback,
                        hud_stick_nav,
                        rebuild_hud,
                    )
                        .chain(),
                );
        }

        // ── editor sidebar ────────────────────────────────────────────────────
        if self.editor {
            editor::register_editor_systems(app);
        }
    }
}

/// Backward-compat wrapper — use [`QuickActionHudPlugin::core()`] instead.
///
/// Provides core wheel logic with no HUD canvas.
pub struct WheelMenuPlugin;
impl Plugin for WheelMenuPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(QuickActionHudPlugin::core());
    }
}

/// Alias kept for call-sites that used `ActionWheelPlugin`.
pub type ActionWheelPlugin = WheelMenuPlugin;

/// Spawns the global entity that holds the [`WheelNavAction`] input map.
/// Called once at startup by [`WheelMenuPlugin`].
fn setup_wheel_nav_input(mut commands: Commands) {
    commands.spawn((
        ActionState::<WheelNavAction>::default(),
        InputMap::<WheelNavAction>::default()
            .with_dual_axis(WheelNavAction::Navigate, GamepadStick::RIGHT)
            .with(WheelNavAction::Confirm, GamepadButton::South)
            .with(WheelNavAction::Cancel, GamepadButton::East)
            .with(WheelNavAction::CycleForward, GamepadButton::North)
            .with(WheelNavAction::CycleBack, GamepadButton::West),
    ));
}

/// Reads the right-stick via [`WheelNavAction::Navigate`] (leafwing) and updates every
/// [`WheelState::dir`]. Falls back to the raw left stick when no leafwing entity exists.
pub fn update_wheel_input(
    nav_states: Query<&ActionState<WheelNavAction>>,
    gamepads: Query<&Gamepad>,
    mut wheel_states: Query<&mut WheelState>,
) {
    // Primary: right stick via leafwing
    let mut nav_dir = Vec2::ZERO;
    for action_state in nav_states.iter() {
        let pair = action_state.axis_pair(&WheelNavAction::Navigate);
        if pair.length() > 0.25 {
            nav_dir = pair;
            break;
        }
    }

    // Fallback: raw left stick (keeps existing example code working)
    if nav_dir == Vec2::ZERO {
        for gamepad in &gamepads {
            let x = gamepad.get(GamepadAxis::LeftStickX).unwrap_or(0.0);
            let y = gamepad.get(GamepadAxis::LeftStickY).unwrap_or(0.0);
            let v = Vec2::new(x, y);
            if v.length() > 0.25 {
                nav_dir = v;
                break;
            }
        }
    }

    let dir = if nav_dir.length() > 0.25 {
        nav_dir.normalize()
    } else {
        Vec2::ZERO
    };
    for mut ws in &mut wheel_states {
        ws.dir = dir;
    }
}

/// Determines which slice is hovered and handles per-mode activation:
/// - [`CastingMode::ReleaseToUse`]: fires [`WheelMenuSelected`] when the stick
///   returns to centre.
/// - [`CastingMode::Direct`]: fires [`WheelMenuSelected`] immediately on hover.
pub fn update_wheel_hover(
    mut q: Query<(
        Entity,
        &WheelData,
        &mut WheelState,
        Option<&WheelMenuConfig>,
    )>,
    mut hover_ev: MessageWriter<WheelMenuHoverChanged>,
    mut select_ev: MessageWriter<WheelMenuSelected>,
) {
    for (entity, menu, mut state, config) in &mut q {
        let previous = state.hovered;

        if state.dir.length() < menu.deadzone {
            state.hovered = None;
        } else {
            let a = state.dir.y.atan2(state.dir.x);
            // Angle relative to the arc start, wrapped into [0, TAU).
            let rel = (a - menu.arc_offset).rem_euclid(std::f32::consts::TAU);
            if rel <= menu.arc_span {
                let idx = ((rel / menu.arc_span) * menu.slots.len().max(1) as f32).floor() as usize;
                state.hovered = Some(idx.min(menu.slots.len().max(1).saturating_sub(1)));
            } else {
                // Direction points outside a partial arc.
                state.hovered = None;
            }
        }

        if previous != state.hovered {
            hover_ev.write(WheelMenuHoverChanged {
                previous,
                current: state.hovered,
                menu_entity: entity,
            });

            if let Some(cfg) = config {
                match &cfg.casting_mode {
                    CastingMode::ReleaseToUse => {
                        // Fire when stick returns to centre after hovering.
                        if let (Some(prev_idx), None) = (previous, state.hovered) {
                            select_ev.write(WheelMenuSelected {
                                index: prev_idx,
                                menu_entity: entity,
                            });
                        }
                    }
                    CastingMode::Direct => {
                        // Fire immediately when a new slice is entered.
                        if let Some(idx) = state.hovered {
                            select_ev.write(WheelMenuSelected {
                                index: idx,
                                menu_entity: entity,
                            });
                        }
                    }
                    _ => {}
                }
            }
        }
    }
}

/// Emits [`WheelMenuSelected`] on confirm-button press for [`CastingMode::Vanilla`].
///
/// All other casting modes handle their own activation logic.
pub fn emit_selection(
    gamepads: Query<&Gamepad>,
    q: Query<(Entity, &WheelState, Option<&WheelMenuConfig>)>,
    mut ev: MessageWriter<WheelMenuSelected>,
) {
    for gamepad in &gamepads {
        if gamepad.just_pressed(GamepadButton::South) {
            for (entity, state, config) in &q {
                if let Some(cfg) = config {
                    match cfg.casting_mode {
                        CastingMode::Vanilla => {} // fall through
                        _ => continue,             // another mode is active
                    }
                }
                if let Some(i) = state.hovered {
                    ev.write(WheelMenuSelected {
                        index: i,
                        menu_entity: entity,
                    });
                }
            }
        }
    }
}

// ─── additional systems ───────────────────────────────────────────────────────

/// Emits [`WheelOpened`] the first frame a wheel gains a hovered slice, and
/// [`WheelClosed`] the first frame it loses one.  Runs after [`update_wheel_hover`].
pub fn emit_lifecycle(
    mut q: Query<(Entity, &mut WheelState)>,
    mut opened_ev: MessageWriter<WheelOpened>,
    mut closed_ev: MessageWriter<WheelClosed>,
) {
    for (entity, mut state) in &mut q {
        let is_open = state.hovered.is_some();
        if is_open && !state.open {
            state.open = true;
            opened_ev.write(WheelOpened {
                menu_entity: entity,
            });
        } else if !is_open && state.open {
            state.open = false;
            closed_ev.write(WheelClosed {
                menu_entity: entity,
            });
        }
    }
}

/// Cycles the active item in a [`WheelSlot`] when right-thumb (next) or
/// left-thumb (prev) is pressed while hovering the corresponding slice.
/// Emits [`WheelSlotItemChanged`] on each cycle.
pub fn update_slot_cycle(
    gamepads: Query<&Gamepad>,
    wheel_q: Query<(Entity, &WheelState)>,
    mut slot_q: Query<(&WheelSlice, &mut WheelSlot)>,
    mut ev: MessageWriter<WheelSlotItemChanged>,
) {
    for gamepad in &gamepads {
        let next = gamepad.just_pressed(GamepadButton::RightThumb);
        let prev = gamepad.just_pressed(GamepadButton::LeftThumb);
        if !next && !prev {
            continue;
        }

        for (menu_entity, state) in &wheel_q {
            if let Some(hovered) = state.hovered {
                for (slice, mut slot) in &mut slot_q {
                    if slice.index == hovered {
                        let previous_item = slot.current_item;
                        if next {
                            slot.cycle_next();
                        } else {
                            slot.cycle_prev();
                        }
                        if slot.current_item != previous_item {
                            ev.write(WheelSlotItemChanged {
                                slot_index: hovered,
                                previous_item,
                                current_item: slot.current_item,
                                menu_entity,
                            });
                        }
                    }
                }
            }
        }
    }
}

/// Applies the configured time scale to [`Time<Virtual>`] based on each
/// [`WheelMenuConfig::time_mode`].  When multiple wheel entities are alive the
/// most restrictive (lowest) scale wins.
pub fn update_wheel_time_scale(q: Query<&WheelMenuConfig>, mut time: ResMut<Time<Virtual>>) {
    let effective = q.iter().fold(1.0_f32, |acc, cfg| {
        let scale = match cfg.time_mode {
            TimeMode::Normal => 1.0,
            TimeMode::Slow(s) => s,
            TimeMode::Pause => 0.0,
        };
        acc.min(scale)
    });
    time.set_relative_speed(effective);
}

/// Tracks dwell time on a hovered slice for [`CastingMode::HoldToActivate`].
/// Emits [`WheelMenuHoldProgress`] each frame and [`WheelMenuHoldActivated`]
/// when `duration` is reached.
pub fn update_wheel_hold(
    time: Res<Time>,
    mut q: Query<(Entity, &WheelMenuConfig, &WheelState, &mut WheelHoldState)>,
    mut progress_ev: MessageWriter<WheelMenuHoldProgress>,
    mut activate_ev: MessageWriter<WheelMenuHoldActivated>,
) {
    for (entity, config, state, mut hold) in &mut q {
        let duration = match config.casting_mode {
            CastingMode::HoldToActivate { duration } => duration,
            _ => {
                hold.progress = 0.0;
                hold.holding = false;
                continue;
            }
        };
        match state.hovered {
            Some(index) => {
                hold.holding = true;
                hold.progress = (hold.progress + time.delta_secs() / duration).clamp(0.0, 1.0);
                progress_ev.write(WheelMenuHoldProgress {
                    index,
                    progress: hold.progress,
                    menu_entity: entity,
                });
                if hold.progress >= 1.0 {
                    activate_ev.write(WheelMenuHoldActivated {
                        index,
                        menu_entity: entity,
                    });
                    hold.progress = 0.0;
                }
            }
            None => {
                hold.holding = false;
                hold.progress = 0.0;
            }
        }
    }
}

/// Cycles the active wheel in a [`WheelSet`] when the configured shoulder
/// buttons are pressed.  The index wraps around at both ends.
pub fn update_wheel_set(
    gamepads: Query<&Gamepad>,
    mut q: Query<(Entity, &mut WheelSet)>,
    mut ev: MessageWriter<WheelSwitched>,
) {
    for (entity, mut set) in &mut q {
        if set.count < 2 {
            continue;
        }
        for gamepad in &gamepads {
            if gamepad.just_pressed(set.next_button) {
                let previous = set.active;
                set.active = (set.active + 1) % set.count;
                ev.write(WheelSwitched {
                    previous,
                    current: set.active,
                    menu_entity: entity,
                });
            }
            if gamepad.just_pressed(set.prev_button) {
                let previous = set.active;
                set.active = (set.active + set.count - 1) % set.count;
                ev.write(WheelSwitched {
                    previous,
                    current: set.active,
                    menu_entity: entity,
                });
            }
        }
    }
}

/// Emits [`WheelMenuLowCount`] once each time a [`WheelSliceCount`] transitions
/// from above to at-or-below its threshold.  The flag resets when the count
/// rises above the threshold again.
pub fn check_low_counts(
    mut q: Query<(Entity, &WheelSlice, &mut WheelSliceCount)>,
    mut ev: MessageWriter<WheelMenuLowCount>,
) {
    for (entity, slice, mut count) in &mut q {
        let is_low = count.max > 0 && count.current <= count.low_threshold;
        if is_low && !count.low_notified {
            count.low_notified = true;
            ev.write(WheelMenuLowCount {
                index: slice.index,
                current: count.current,
                threshold: count.low_threshold,
                slice_entity: entity,
            });
        } else if !is_low {
            count.low_notified = false;
        }
    }
}

/// Toggles edit mode when the configured button is pressed, and emits
/// [`WheelSliceReorder`] events when D-pad Up/Down is pressed while hovering a
/// slice in edit mode.
pub fn update_edit_mode(
    gamepads: Query<&Gamepad>,
    mut q: Query<(Entity, &WheelData, &WheelState, &mut WheelEditMode)>,
    mut mode_ev: MessageWriter<WheelEditModeChanged>,
    mut reorder_ev: MessageWriter<WheelSliceReorder>,
) {
    for (entity, menu, state, mut edit) in &mut q {
        for gamepad in &gamepads {
            if let Some(btn) = edit.toggle_button {
                if gamepad.just_pressed(btn) {
                    edit.active = !edit.active;
                    mode_ev.write(WheelEditModeChanged {
                        active: edit.active,
                        menu_entity: entity,
                    });
                }
            }
            if edit.active {
                if let Some(hovered) = state.hovered {
                    if gamepad.just_pressed(GamepadButton::DPadUp) && hovered > 0 {
                        reorder_ev.write(WheelSliceReorder {
                            from_index: hovered,
                            to_index: hovered - 1,
                            menu_entity: entity,
                        });
                    }
                    if gamepad.just_pressed(GamepadButton::DPadDown)
                        && hovered + 1 < menu.slots.len().max(1)
                    {
                        reorder_ev.write(WheelSliceReorder {
                            from_index: hovered,
                            to_index: hovered + 1,
                            menu_entity: entity,
                        });
                    }
                }
            }
        }
    }
}

/// Maintains [`ActiveSlotContext`] on each wheel from its currently hovered
/// slice.  Only slices carrying a [`WheelSliceLink`] participate, so the link
/// back to the owning wheel is explicit and query scans stay cheap.
pub fn update_active_slot_context(
    mut commands: Commands,
    wheel_q: Query<(Entity, &WheelState)>,
    slice_q: Query<(Entity, &WheelSlice, &WheelSliceLink)>,
) {
    for (menu, state) in &wheel_q {
        let mut found: Option<Entity> = None;
        if let Some(hovered) = state.hovered {
            for (slice_entity, slice, link) in &slice_q {
                if link.menu == menu && slice.index == hovered {
                    found = Some(slice_entity);
                    break;
                }
            }
        }
        match found {
            Some(slot_entity) => {
                commands
                    .entity(menu)
                    .insert(ActiveSlotContext { slot_entity });
            }
            None => {
                commands.entity(menu).remove::<ActiveSlotContext>();
            }
        }
    }
}

/// Reads gamepad face/thumb buttons, maps them onto [`InputAction`]s, resolves
/// each against the slot → wheel → global override chain, and emits
/// [`WheelActionResolved`].  This implements the contextual input-override
/// system: a hovered slot's bindings take priority over the wheel's, which take
/// priority over [`GlobalBindings`].
pub fn resolve_wheel_input(
    gamepads: Query<&Gamepad>,
    global: Res<GlobalBindings>,
    wheel_q: Query<
        (
            Entity,
            Option<&WheelInputOverride>,
            Option<&ActiveSlotContext>,
        ),
        With<WheelState>,
    >,
    slot_q: Query<&WheelInputOverride, Without<WheelState>>,
    mut ev: MessageWriter<WheelActionResolved>,
) {
    for (menu, wheel_override, active_slot) in &wheel_q {
        let slot_override = active_slot.and_then(|ctx| slot_q.get(ctx.slot_entity).ok());
        for gamepad in &gamepads {
            for (button, input) in DEFAULT_BUTTON_MAP {
                if gamepad.just_pressed(*button) {
                    if let Some(action) =
                        resolve_input(*input, slot_override, wheel_override, &global)
                    {
                        ev.write(WheelActionResolved {
                            input: *input,
                            action,
                            menu_entity: menu,
                        });
                    }
                }
            }
        }
    }
}

/// Helper to calculate slice angles with gap.
pub fn slice_angles(menu: &WheelData, index: usize) -> (f32, f32) {
    let n = menu.slots.len().max(1);
    let slice_angle = menu.arc_span / n as f32;
    let half_gap = if menu.overlap { 0.0 } else { menu.gap / 2.0 };
    let a0 = menu.arc_offset + index as f32 * slice_angle + half_gap;
    let a1 = menu.arc_offset + (index + 1) as f32 * slice_angle - half_gap;
    (a0, a1)
}

/// Helper to get the center position of a slice (for placing icons/text).
pub fn slice_center(menu: &WheelData, index: usize) -> Vec2 {
    let (a0, a1) = slice_angles(menu, index);
    let center_angle = (a0 + a1) / 2.0;
    let center_radius = (menu.inner_radius + menu.outer_radius) / 2.0;
    Vec2::new(
        center_angle.cos() * center_radius,
        center_angle.sin() * center_radius,
    )
}

/// Returns a full-screen `bevy_ui` overlay [`Node`] that centers its children,
/// authored with the [`bsn!`] macro.
///
/// Spawn it with `commands.spawn_scene(wheel_overlay())` and attach the
/// wheel-menu logic components ([`WheelData`] and [`WheelState`]) to the
/// resulting entity.
pub fn wheel_overlay() -> impl bevy::scene::prelude::Scene {
    bsn! {
        Node {
            position_type: PositionType::Absolute,
            left: {px(0.)},
            top: {px(0.)},
            width: {percent(100.)},
            height: {percent(100.)},
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
        }
    }
}

/// Returns a zero-size hub [`Node`] used as the positioning origin for slices,
/// authored with the [`bsn!`] macro.
///
/// Spawn it as a child of [`wheel_overlay`] and parent each slice panel to it so
/// the absolutely-positioned panels are laid out relative to the screen center.
pub fn wheel_hub() -> impl bevy::scene::prelude::Scene {
    bsn! {
        Node { width: {px(0.)}, height: {px(0.)} }
    }
}

/// Returns an absolutely-positioned, rounded slice panel centered on the radial
/// position of slice `index`, authored with the [`bsn!`] macro.
///
/// `size` is the panel's width/height in logical pixels and `color` its
/// background color. The panel is laid out as a centered vertical column so
/// icons and labels can be added as children.
pub fn wheel_slice_panel(
    menu: &WheelData,
    index: usize,
    size: f32,
    color: Color,
) -> impl bevy::scene::prelude::Scene {
    wheel_slice_panel_styled(menu, index, size, color, size * 0.18)
}

/// Like [`wheel_slice_panel`] but with an explicit `corner_radius`, letting the
/// caller pick a slot shape: `size * 0.5` ≈ round, `size * 0.18` ≈ rounded,
/// `0.0` = square.
pub fn wheel_slice_panel_styled(
    menu: &WheelData,
    index: usize,
    size: f32,
    color: Color,
    corner_radius: f32,
) -> impl bevy::scene::prelude::Scene {
    let center = slice_center(menu, index);
    // Math coordinates are y-up and centered on the wheel; UI is y-down relative
    // to the hub, so flip the y axis and offset by half the panel size.
    let left = center.x - size / 2.0;
    let top = -center.y - size / 2.0;
    bsn! {
        Node {
            position_type: PositionType::Absolute,
            left: {px(left)},
            top: {px(top)},
            width: {px(size)},
            height: {px(size)},
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            flex_direction: FlexDirection::Column,
            row_gap: {px(2.)},
            border_radius: {BorderRadius::all(px(corner_radius))},
        }
        BackgroundColor({color})
    }
}

/// Returns a circular center-disc [`Node`] of diameter `radius * 2`, centered
/// on the hub via absolute positioning.
///
/// Spawn as a child of [`wheel_hub`].  Add label or icon children afterward
/// with `commands.entity(disc).add_child(...)`.
pub fn wheel_center_disc(radius: f32, color: Color) -> impl bevy::scene::prelude::Scene {
    bsn! {
        Node {
            position_type: PositionType::Absolute,
            left: {px(-radius)},
            top: {px(-radius)},
            width: {px(radius * 2.0)},
            height: {px(radius * 2.0)},
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            border_radius: {BorderRadius::all(px(radius))},
        }
        BackgroundColor({color})
    }
}

/// Like [`wheel_center_disc`] but draws a coloured ring border around the hub.
///
/// Use this instead of [`wheel_center_disc`] to get the golden ring shown in
/// the reference screenshots.
pub fn wheel_center_ring(
    radius: f32,
    bg: Color,
    ring_color: Color,
    ring_width: f32,
) -> impl bevy::scene::prelude::Scene {
    bsn! {
        Node {
            position_type: PositionType::Absolute,
            left: {px(-radius)},
            top: {px(-radius)},
            width: {px(radius * 2.0)},
            height: {px(radius * 2.0)},
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            border_radius: {BorderRadius::all(px(radius))},
            border: {UiRect::all(px(ring_width))},
        }
        BackgroundColor({bg})
        BorderColor::all(ring_color)
    }
}

/// A large dark disc that fills the full wheel area, placed behind all slices.
///
/// Spawn as a child of [`wheel_hub`] **before** the slices so it sits at the
/// back of the z-order.
pub fn wheel_bg_disc(outer_radius: f32, color: Color) -> impl bevy::scene::prelude::Scene {
    let r = outer_radius + 4.0;
    bsn! {
        Node {
            position_type: PositionType::Absolute,
            left: {Val::Px(-r)},
            top: {Val::Px(-r)},
            width: {Val::Px(r * 2.0)},
            height: {Val::Px(r * 2.0)},
            border_radius: {BorderRadius::all(Val::Px(r))},
        }
        BackgroundColor({color})
    }
}

/// A thin amber/gold ring just outside the wheel — approximates the dashed
/// outer border visible in the reference screenshots.
pub fn wheel_outer_ring(
    outer_radius: f32,
    color: Color,
    border_w: f32,
) -> impl bevy::scene::prelude::Scene {
    let r = outer_radius + 18.0;
    bsn! {
        Node {
            position_type: PositionType::Absolute,
            left: {Val::Px(-r)},
            top: {Val::Px(-r)},
            width: {Val::Px(r * 2.0)},
            height: {Val::Px(r * 2.0)},
            border_radius: {BorderRadius::all(Val::Px(r))},
            border: {UiRect::all(Val::Px(border_w))},
        }
        BackgroundColor({Color::NONE})
        BorderColor::all(color)
    }
}

/// Absolutely-positioned rectangular slice panel, sized to better fill a
/// segment of the wheel than the square [`wheel_slice_panel_styled`].
///
/// `width` and `height` are in logical pixels.  Use `corner_radius` ≈
/// `min(width, height) * 0.15` for the rounded look shown in the screenshots.
pub fn wheel_slice_panel_rect(
    menu: &WheelData,
    index: usize,
    width: f32,
    height: f32,
    color: Color,
    corner_radius: f32,
) -> impl bevy::scene::prelude::Scene {
    let center = slice_center(menu, index);
    let left = center.x - width / 2.0;
    let top = -center.y - height / 2.0;
    bsn! {
        Node {
            position_type: PositionType::Absolute,
            left: {px(left)},
            top: {px(top)},
            width: {px(width)},
            height: {px(height)},
            justify_content: JustifyContent::SpaceBetween,
            align_items: AlignItems::Center,
            flex_direction: FlexDirection::Column,
            padding: {UiRect::all(px(6.))},
            border_radius: {BorderRadius::all(px(corner_radius))},
        }
        BackgroundColor({color})
    }
}

/// Returns a [`Text`] node sized for a slice **icon** (typically an emoji or
/// large glyph).
///
/// Spawn as a child of [`wheel_slice_panel`] or insert marker components with
/// `.insert(MyMarker)` on the returned `EntityCommands`.
pub fn wheel_slice_icon(
    icon: String,
    font_size: f32,
    color: Color,
) -> impl bevy::scene::prelude::Scene {
    bsn! {
        Text({icon})
        TextFont { font_size: {FontSize::Px(font_size)} }
        TextColor({color})
    }
}

/// Returns a [`Text`] node sized for a slice **label** (name, count, cooldown,
/// etc.).
///
/// Spawn as a child of [`wheel_slice_panel`] or insert marker components with
/// `.insert(MyMarker)` on the returned `EntityCommands`.
pub fn wheel_slice_label(
    text: String,
    font_size: f32,
    color: Color,
) -> impl bevy::scene::prelude::Scene {
    bsn! {
        Text({text})
        TextFont { font_size: {FontSize::Px(font_size)} }
        TextColor({color})
    }
}

// ─────────────────────────────────────────────────────────────────────────────────
// QUICK-ACTION CONFIG — data model for the editor and the HUD
// ─────────────────────────────────────────────────────────────────────────────────

/// Placement reference for a floating quick-action button.
#[derive(Clone, Copy, PartialEq, Serialize, Deserialize, Debug, Default)]
pub enum PositionMode {
    #[default]
    Relative,
    Absolute,
}
impl PositionMode {
    pub fn label(self) -> &'static str {
        match self {
            Self::Relative => "Relative",
            Self::Absolute => "Absolute",
        }
    }
    pub fn next(self) -> Self {
        match self {
            Self::Relative => Self::Absolute,
            Self::Absolute => Self::Relative,
        }
    }
}

/// Shape of a quick-action HUD button.
#[derive(Clone, Copy, PartialEq, Serialize, Deserialize, Debug, Default)]
pub enum ActionShape {
    #[default]
    Rounded,
    Round,
    Square,
    Diamond,
}
impl ActionShape {
    pub fn label(self) -> &'static str {
        match self {
            Self::Rounded => "Rounded",
            Self::Round => "Round",
            Self::Square => "Square",
            Self::Diamond => "Diamond",
        }
    }
    pub fn next(self) -> Self {
        match self {
            Self::Rounded => Self::Round,
            Self::Round => Self::Square,
            Self::Square => Self::Diamond,
            Self::Diamond => Self::Rounded,
        }
    }
}

/// Visual theme for a wheel.
#[derive(Clone, Copy, PartialEq, Serialize, Deserialize, Debug, Default)]
pub enum WheelTheme {
    #[default]
    Dark,
    Light,
}
impl WheelTheme {
    pub fn label(self) -> &'static str {
        match self {
            Self::Dark => "dark",
            Self::Light => "light",
        }
    }
    pub fn next(self) -> Self {
        match self {
            Self::Dark => Self::Light,
            Self::Light => Self::Dark,
        }
    }
}

/// Shape rendered for each segment panel inside the wheel.
#[derive(Clone, Copy, PartialEq, Serialize, Deserialize, Debug, Default)]
pub enum SegmentShape {
    #[default]
    Rounded,
    Square,
    Circle,
    /// Asymmetric corners — outer large, inner small.
    Wedge,
    /// Real `Mesh2d` wedge arc (uses `bevy::mesh`).
    Pie,
}
impl SegmentShape {
    pub fn label(self) -> &'static str {
        match self {
            Self::Rounded => "Rounded",
            Self::Square => "Square",
            Self::Circle => "Circle",
            Self::Wedge => "Wedge",
            Self::Pie => "Pie",
        }
    }
    pub fn next(self) -> Self {
        match self {
            Self::Rounded => Self::Square,
            Self::Square => Self::Circle,
            Self::Circle => Self::Wedge,
            Self::Wedge => Self::Pie,
            Self::Pie => Self::Rounded,
        }
    }
}

// ── palette helpers ─────────────────────────────────────────────────────────────
pub const ICON_PALETTE: &[&str] = &["◆", "●", "★", "▲", "✦", "✚", "◈", "○", "◐", "✱"];
pub const COMMAND_PALETTE: &[&str] = &[
    "none", "attack", "heal", "block", "dash", "reload", "interact", "jump", "crouch", "sprint",
];
pub fn cycle_palette<'a>(list: &[&'a str], current: &str) -> &'a str {
    let idx = list.iter().position(|s| *s == current).unwrap_or(0);
    list[(idx + 1) % list.len()]
}

// ── serde defaults ──────────────────────────────────────────────────────────────
fn _default_true() -> bool {
    true
}
fn _default_action_color() -> String {
    "#3b82f6".into()
}
fn _default_action_width() -> f32 {
    80.0
}
fn _default_action_height() -> f32 {
    28.0
}
fn _default_outer_radius() -> f32 {
    110.0
}
fn _default_inner_radius() -> f32 {
    38.0
}
fn _full_opacity() -> f32 {
    1.0
}
fn _default_highlight_color() -> String {
    "#f59e0b".into()
}
fn _default_segment_scale() -> f32 {
    1.0
}
fn _default_border_width() -> f32 {
    2.0
}
fn _default_deadzone() -> f32 {
    0.3
}
fn _default_gap() -> f32 {
    0.04
}
fn _default_arc_span() -> f32 {
    std::f32::consts::TAU
}
fn _default_arc_offset() -> f32 {
    std::f32::consts::FRAC_PI_6
}

// ── slot / item data ─────────────────────────────────────────────────────────────

/// One item in a slot's cycle carousel.
#[derive(Clone, Serialize, Deserialize, Debug, Default)]
pub struct SlotItem {
    pub name: String,
    pub icon: String,
}

/// Per-segment data for the editor config.
#[derive(Clone, Serialize, Deserialize, Debug, Default)]
#[serde(default)]
pub struct WheelSlotData {
    pub name: String,
    pub icon: String,
    /// Captured input label: keyboard key or "GP:…" gamepad.
    pub input: String,
    pub items: Vec<SlotItem>,
}
impl WheelSlotData {
    pub fn named(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            ..Default::default()
        }
    }
}

// ── quick action ─────────────────────────────────────────────────────────────────

/// A key-bound floating HUD button.
#[derive(Clone, Serialize, Deserialize, Debug)]
#[serde(default)]
pub struct QuickAction {
    pub name: String,
    /// Keyboard key or gamepad button ("GP:\u{2026}" prefix) that triggers this action.
    pub key: String,
    pub icon: String,
    pub command: String,
    pub hold: bool,
    pub show_on_menu: bool,
    pub opacity: f32,
    pub position: PositionMode,
    pub radius: f32,
    pub shape: ActionShape,
    #[serde(default = "_default_action_color")]
    pub color: String,
    #[serde(default = "_default_action_width")]
    pub width: f32,
    #[serde(default = "_default_action_height")]
    pub height: f32,
    #[serde(default = "_default_true")]
    pub enabled: bool,
}
impl Default for QuickAction {
    fn default() -> Self {
        Self {
            name: "Action".into(),
            key: String::new(),
            icon: "◆".into(),
            command: "none".into(),
            hold: false,
            show_on_menu: true,
            opacity: 1.0,
            position: PositionMode::Relative,
            radius: 48.0,
            shape: ActionShape::Rounded,
            color: _default_action_color(),
            width: _default_action_width(),
            height: _default_action_height(),
            enabled: true,
        }
    }
}

// ── wheel config data ────────────────────────────────────────────────────────────

/// Editor data-model wheel — one radial menu with named segments.
#[derive(Component, Clone, Serialize, Deserialize, Debug)]
#[serde(default)]
pub struct WheelData {
    pub name: String,
    pub cooldown_secs: f32,
    pub slots: Vec<WheelSlotData>,
    #[serde(default)]
    pub theme: WheelTheme,
    #[serde(default = "_default_outer_radius")]
    pub outer_radius: f32,
    #[serde(default = "_default_inner_radius")]
    pub inner_radius: f32,
    #[serde(default = "_default_true")]
    pub show_labels: bool,
    #[serde(default)]
    pub segment_shape: SegmentShape,
    #[serde(default = "_default_true")]
    pub show_icon: bool,
    #[serde(default = "_default_highlight_color")]
    pub highlight_color: String,
    #[serde(default = "_default_segment_scale")]
    pub segment_scale: f32,
    /// Overall opacity of the wheel overlay (0.0 – 1.0).
    #[serde(default = "_full_opacity")]
    pub opacity: f32,
    /// Hex color for the inner-radius border ring; empty = no border.
    #[serde(default)]
    pub inner_border: String,
    /// Hex color for the outer-radius border ring; empty = no border.
    #[serde(default)]
    pub outer_border: String,
    /// Width in px of the outer border ring; only used when `outer_border` is set.
    #[serde(default = "_default_border_width")]
    pub outer_border_width: f32,
    /// Width in px of the inner hub ring; only used when `inner_border` is set.
    #[serde(default = "_default_border_width")]
    pub inner_border_width: f32,
    /// Hex background color for the full wheel disc; empty = use theme.
    #[serde(default)]
    pub bg_color: String,
    /// Opacity of the wheel background disc (0.0 – 1.0).
    #[serde(default = "_full_opacity")]
    pub bg_opacity: f32,
    /// Hex background color for the hub (inner circle); empty = use theme.
    #[serde(default)]
    pub hub_color: String,
    /// Opacity of the hub (inner circle) background (0.0 – 1.0).
    #[serde(default = "_full_opacity")]
    pub hub_opacity: f32,
    /// Stick deadzone for the headless ECS input API (0.0–1.0).
    #[serde(default = "_default_deadzone")]
    pub deadzone: f32,
    /// Gap between segments in radians (used by the headless ECS API).
    #[serde(default = "_default_gap")]
    pub gap: f32,
    /// Total angular span in radians (TAU = full circle).
    #[serde(default = "_default_arc_span")]
    pub arc_span: f32,
    /// Angle of the first segment, CCW from +X axis.
    #[serde(default = "_default_arc_offset")]
    pub arc_offset: f32,
    /// When true, segments touch with no gap.
    #[serde(default)]
    pub overlap: bool,
}
impl Default for WheelData {
    fn default() -> Self {
        Self {
            name: "Wheel".into(),
            cooldown_secs: 6.0,
            slots: vec![WheelSlotData::named("Slot 1")],
            theme: WheelTheme::Dark,
            outer_radius: _default_outer_radius(),
            inner_radius: _default_inner_radius(),
            show_labels: true,
            segment_shape: SegmentShape::Rounded,
            show_icon: true,
            highlight_color: "#f59e0b".into(),
            segment_scale: 1.0,
            opacity: 1.0,
            inner_border: String::new(),
            outer_border: String::new(),
            outer_border_width: 2.0,
            inner_border_width: 2.0,
            bg_color: String::new(),
            bg_opacity: 1.0,
            hub_color: String::new(),
            hub_opacity: 1.0,
            deadzone: _default_deadzone(),
            gap: _default_gap(),
            arc_span: _default_arc_span(),
            arc_offset: _default_arc_offset(),
            overlap: false,
        }
    }
}
impl WheelData {
    pub fn new(name: impl Into<String>, n: usize) -> Self {
        Self {
            name: name.into(),
            slots: (0..n.max(1))
                .map(|i| WheelSlotData::named(format!("Slot {}", i + 1)))
                .collect(),
            ..Default::default()
        }
    }
}

/// A named group of [`WheelData`] entries the player can switch between.
/// Serialized as `WheelSet` for RON compatibility.
#[derive(Clone, Serialize, Deserialize, Debug)]
#[serde(default)]
pub struct WheelSetData {
    pub name: String,
    pub wheels: Vec<WheelData>,
    #[serde(default)]
    pub switch_key: String,
}
impl Default for WheelSetData {
    fn default() -> Self {
        Self {
            name: "Wheel Set".into(),
            wheels: Vec::new(),
            switch_key: String::new(),
        }
    }
}

/// One entry inside an [`ActionSet`].
#[derive(Clone, Serialize, Deserialize, Debug)]
pub enum SetEntry {
    Action(QuickAction),
    Wheel(WheelData),
    WheelSet(WheelSetData),
}

/// A named context group that holds quick actions and wheels.
#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct ActionSet {
    pub name: String,
    #[serde(default = "_full_opacity")]
    pub opacity: f32,
    #[serde(default)]
    pub input_override: bool,
    pub entries: Vec<SetEntry>,
    #[serde(default)]
    pub bg_image: String,
    #[serde(default = "_full_opacity")]
    pub bg_image_opacity: f32,
    #[serde(default)]
    pub next_wheel_key: String,
    #[serde(default)]
    pub prev_wheel_key: String,
    #[serde(default)]
    pub cycle_wheels: bool,
}

/// Returns the number of `Wheel` and `WheelSet` entries in a set.
pub fn count_wheel_entries(set: &ActionSet) -> usize {
    set.entries
        .iter()
        .filter(|e| matches!(e, SetEntry::Wheel(_) | SetEntry::WheelSet(_)))
        .count()
}

/// Whether the HUD overlay opens while a button is held (released = close)
/// or toggles open/closed on each press.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub enum HudOpenMode {
    /// Hold the trigger to keep the HUD open; releasing closes it.
    #[default]
    Hold,
    /// First press opens the HUD; second press closes it.
    Toggle,
}

impl HudOpenMode {
    pub fn label(&self) -> &'static str {
        match self {
            HudOpenMode::Hold => "Hold",
            HudOpenMode::Toggle => "Toggle",
        }
    }
    pub fn next(&self) -> Self {
        match self {
            HudOpenMode::Hold => HudOpenMode::Toggle,
            HudOpenMode::Toggle => HudOpenMode::Hold,
        }
    }
}

/// The complete editable document (Bevy `Resource`).
#[derive(Resource, Clone, Serialize, Deserialize, Debug)]
#[serde(default)]
pub struct QuickActionConfig {
    #[serde(default)]
    pub next_set_key: String,
    #[serde(default)]
    pub prev_set_key: String,
    /// Show the ActionSet tab bar in the HUD overlay.
    #[serde(default = "_default_true")]
    pub show_set_bar: bool,
    /// Pressing Next on the last set wraps to the first (cycle), otherwise stops.
    #[serde(default)]
    pub cycle_sets: bool,
    /// Key or gamepad button ("GP:…" prefix) that opens/closes the editor sidebar.
    #[serde(default)]
    pub edit_shortcut: String,
    /// Whether the HUD trigger button is a hold (release = close) or a toggle.
    #[serde(default)]
    pub hud_open_mode: HudOpenMode,
    /// Opacity of the full-screen HUD background overlay (0.0 = invisible, 1.0 = opaque).
    #[serde(default = "_full_opacity")]
    pub hud_bg_opacity: f32,
    pub sets: Vec<ActionSet>,
}

impl Default for QuickActionConfig {
    fn default() -> Self {
        let mut combat_wheel = WheelData::new("Combat Wheel", 6);
        combat_wheel.slots = vec![
            WheelSlotData::named("Map"),
            WheelSlotData::named("Attack"),
            WheelSlotData::named("Block"),
            WheelSlotData::named("Heal"),
            WheelSlotData::named("Ability"),
            WheelSlotData::named("Sprint"),
        ];
        Self {
            next_set_key: "Tab".into(),
            prev_set_key: "Q".into(),
            show_set_bar: true,
            cycle_sets: false,
            edit_shortcut: String::new(),
            hud_open_mode: HudOpenMode::Hold,
            hud_bg_opacity: 1.0,
            sets: vec![
                ActionSet {
                    name: "Combat".into(),
                    opacity: 1.0,
                    input_override: false,
                    entries: vec![
                        SetEntry::WheelSet(WheelSetData {
                            name: "Wheel Set".into(),
                            switch_key: String::new(),
                            wheels: vec![combat_wheel, WheelData::new("Wheel 2", 6)],
                        }),
                        SetEntry::Action(QuickAction {
                            name: "Interact".into(),
                            key: "E".into(),
                            icon: "◆".into(),
                            command: "interact".into(),
                            color: "#14b8a6".into(),
                            width: 90.0,
                            height: 28.0,
                            ..default()
                        }),
                        SetEntry::Action(QuickAction {
                            name: "Inventory".into(),
                            key: "I".into(),
                            icon: "◈".into(),
                            command: "none".into(),
                            color: "#8b5cf6".into(),
                            width: 80.0,
                            height: 28.0,
                            ..default()
                        }),
                    ],
                    bg_image: String::new(),
                    bg_image_opacity: 1.0,
                    next_wheel_key: String::new(),
                    prev_wheel_key: String::new(),
                    cycle_wheels: false,
                },
                ActionSet {
                    name: "Stealth".into(),
                    opacity: 1.0,
                    input_override: false,
                    entries: vec![
                        SetEntry::WheelSet(WheelSetData {
                            name: "Stealth Wheels".into(),
                            switch_key: String::new(),
                            wheels: vec![WheelData::new("Stealth Wheel", 4)],
                        }),
                        SetEntry::Action(QuickAction {
                            name: "Hide".into(),
                            key: "H".into(),
                            icon: "◐".into(),
                            command: "crouch".into(),
                            color: "#6366f1".into(),
                            width: 70.0,
                            height: 28.0,
                            ..default()
                        }),
                    ],
                    bg_image: String::new(),
                    bg_image_opacity: 1.0,
                    next_wheel_key: String::new(),
                    prev_wheel_key: String::new(),
                    cycle_wheels: false,
                },
            ],
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────────
// HUD STATE, COMPONENTS, AND RENDERING
// ─────────────────────────────────────────────────────────────────────────────────

/// Tags the root UI entity of the full-screen HUD.  Despawned on each rebuild.
#[derive(Component)]
pub struct WheelHudRoot;

/// Params uploaded to the wedge fragment shader.
#[derive(Clone, ShaderType)]
pub struct WedgeParams {
    pub color: Vec4,
    pub inner_r: f32,
    pub outer_r: f32,
    pub angle_start: f32,
    pub angle_end: f32,
}

/// UI material that renders a single annular sector (pie slice).
#[derive(Asset, AsBindGroup, TypePath, Clone)]
pub struct WedgeMaterial {
    #[uniform(0)]
    pub params: WedgeParams,
}

impl UiMaterial for WedgeMaterial {
    fn fragment_shader() -> ShaderRef {
        "shaders/wedge.wgsl".into()
    }
}

/// Shared state read by both the HUD renderer and the editor sidebar.
#[derive(Resource)]
pub struct WheelHudState {
    pub dirty: bool,
    /// Whether the HUD wheel overlay is currently open/visible.
    pub open: bool,
    /// Which [`ActionSet`] is currently displayed.
    pub active_set: usize,
    /// Whether the editor sidebar overlay is open.
    pub editor_open: bool,
    /// Highlighted segment: (set, entry, wheel, slot).
    pub highlighted: Option<(usize, usize, Option<usize>, usize)>,
    /// Which wheel entry within the active set is currently active.
    pub active_wheel_entry: usize,
}
impl Default for WheelHudState {
    fn default() -> Self {
        Self {
            dirty: true,
            open: false,
            active_set: 0,
            editor_open: false,
            highlighted: None,
            active_wheel_entry: 0,
        }
    }
}

/// Interactive button in the HUD (set tabs, edit toggle, etc.).
#[derive(Component, Clone)]
pub struct WheelHudButton {
    pub action: WheelHudAction,
    pub base: Color,
}

/// Actions that can be triggered directly from the HUD.
#[derive(Clone, Debug)]
pub enum WheelHudAction {
    SetActiveSet(usize),
    PrevSet,
    NextSet,
    ToggleEditor,
}

// ── HUD palette ──────────────────────────────────────────────────────────────────
pub const HUD_BG: Color = Color::srgb(0.055, 0.067, 0.086);
pub const HUD_SIDEBAR_BG: Color = Color::srgb(0.043, 0.055, 0.075);
pub const HUD_SIDEBAR_BORDER: Color = Color::srgb(0.10, 0.12, 0.15);
pub const HUD_GREEN: Color = Color::srgb(0.30, 0.74, 0.40);
pub const HUD_GREEN_BG: Color = Color::srgba(0.30, 0.74, 0.40, 0.14);
pub const HUD_TEXT: Color = Color::srgb(0.74, 0.79, 0.85);
pub const HUD_DIM: Color = Color::srgb(0.42, 0.47, 0.54);
pub const HUD_DIMMER: Color = Color::srgb(0.30, 0.34, 0.40);
pub const HUD_ICON: Color = Color::srgb(0.45, 0.53, 0.61);
pub const HUD_AMBER: Color = Color::srgb(0.82, 0.66, 0.25);
pub const HUD_BLUE: Color = Color::srgb(0.38, 0.62, 0.95);
pub const HUD_TEAL: Color = Color::srgb(0.52, 0.69, 0.75);
pub const HUD_BADGE_BORDER: Color = Color::srgb(0.26, 0.30, 0.36);
pub const HUD_ROW_SEL: Color = Color::srgba(0.38, 0.62, 0.95, 0.16);
pub const HUD_PANEL_CARD: Color = Color::srgb(0.08, 0.10, 0.15);

// ── internal helpers ─────────────────────────────────────────────────────────────

fn hud_child(
    commands: &mut Commands,
    parent: Entity,
    scene: impl bevy::scene::prelude::Scene,
) -> Entity {
    let e = commands.spawn_scene(scene).id();
    commands.entity(parent).add_child(e);
    e
}

fn hud_text(s: &str, size: f32, color: Color) -> impl bevy::scene::prelude::Scene {
    let s = s.to_string();
    let sz = size;
    bsn! {
        Text({s})
        TextFont { font_size: {FontSize::Px(sz)} }
        TextColor({color})
    }
}

fn hud_clickable(
    commands: &mut Commands,
    parent: Entity,
    scene: impl bevy::scene::prelude::Scene,
    action: WheelHudAction,
    base: Color,
) -> Entity {
    let e = commands
        .spawn_scene(scene)
        .insert(WheelHudButton { action, base })
        .id();
    commands.entity(parent).add_child(e);
    e
}

/// Parse `#rrggbb` hex string into a Bevy [`Color`].
pub fn parse_hex_color(hex: &str, alpha: f32) -> Color {
    let s = hex.trim_start_matches('#');
    if s.len() == 6 {
        if let (Ok(r), Ok(g), Ok(b)) = (
            u8::from_str_radix(&s[0..2], 16),
            u8::from_str_radix(&s[2..4], 16),
            u8::from_str_radix(&s[4..6], 16),
        ) {
            return Color::srgba(r as f32 / 255., g as f32 / 255., b as f32 / 255., alpha);
        }
    }
    Color::srgba(0.23, 0.51, 0.96, alpha)
}

pub fn hud_label_or(key: &str) -> String {
    if key.is_empty() {
        "—".into()
    } else {
        key.into()
    }
}

// ── canvas root ──────────────────────────────────────────────────────────────────

fn hud_canvas_root() -> impl bevy::scene::prelude::Scene {
    bsn! {
        Node {
            position_type: PositionType::Absolute,
            left: {Val::Px(0.)}, top: {Val::Px(0.)},
            right: {Val::Px(0.)}, bottom: {Val::Px(0.)},
            flex_direction: FlexDirection::Column,
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
        }
        BackgroundColor({HUD_BG.with_alpha(1.0)})
    }
}

// ── main HUD build ───────────────────────────────────────────────────────────────

pub fn build_hud_canvas(
    commands: &mut Commands,
    cfg: &QuickActionConfig,
    hud: &WheelHudState,
    asset_server: &AssetServer,
    icon_set: GamepadIconSet,
    wedge_materials: &mut Assets<WedgeMaterial>,
) {
    let root = commands
        .spawn_scene(hud_canvas_root())
        .insert(WheelHudRoot)
        .id();

    // Nothing to render while the wheel overlay is closed.
    if !hud.open {
        commands.entity(root).insert(BackgroundColor(Color::NONE));
        return;
    }

    // Apply the user-configured HUD background opacity.
    commands
        .entity(root)
        .insert(BackgroundColor(HUD_BG.with_alpha(cfg.hud_bg_opacity)));

    // Edit toggle button — visible only while the wheel is open, hidden when
    // the editor sidebar is already showing.
    if !hud.editor_open {
        let btn = hud_clickable(
            commands,
            root,
            bsn! {
                Node {
                    position_type: PositionType::Absolute,
                    top: {Val::Px(14.)}, left: {Val::Px(14.)},
                    flex_direction: FlexDirection::Row,
                    align_items: AlignItems::Center,
                    column_gap: {Val::Px(5.)},
                    padding: {UiRect::axes(Val::Px(10.), Val::Px(6.))},
                    border: {UiRect::all(Val::Px(1.))},
                    border_radius: {BorderRadius::all(Val::Px(5.))},
                }
                BorderColor::all(HUD_BADGE_BORDER)
                BackgroundColor({HUD_PANEL_CARD})
                Button
            },
            WheelHudAction::ToggleEditor,
            HUD_PANEL_CARD,
        );
        // Show the assigned edit shortcut icon (if it's a gamepad button).
        if let Some(lbl) = cfg.edit_shortcut.strip_prefix("GP:") {
            if let Some(path) = icon_set.icon_path(lbl) {
                let handle = asset_server.load::<Image>(path);
                let icon_e = commands
                    .spawn((
                        Node {
                            width: Val::Px(16.0),
                            height: Val::Px(16.0),
                            ..default()
                        },
                        ImageNode::new(handle),
                    ))
                    .id();
                commands.entity(btn).add_child(icon_e);
            }
        }
        hud_child(commands, btn, hud_text("⚙", 12., HUD_DIM));
        hud_child(commands, btn, hud_text("Edit", 10., HUD_DIM));
    }

    // Set tabs at the top centre (only when enabled in config).
    if cfg.show_set_bar {
        build_hud_set_tabs(commands, root, cfg, hud, asset_server, icon_set);
    }

    if cfg.sets.is_empty() {
        hud_child(
            commands,
            root,
            hud_text("No sets — open the editor to add one.", 12., HUD_DIMMER),
        );
        return;
    }

    if let Some(set) = cfg.sets.get(hud.active_set) {
        // Background image for this set, if configured.
        if !set.bg_image.is_empty() {
            let handle = asset_server.load::<Image>(set.bg_image.clone());
            let bg_e = commands
                .spawn((
                    Node {
                        position_type: PositionType::Absolute,
                        left: Val::Px(0.),
                        top: Val::Px(0.),
                        right: Val::Px(0.),
                        bottom: Val::Px(0.),
                        ..default()
                    },
                    ImageNode {
                        image: handle,
                        color: Color::WHITE.with_alpha(set.bg_image_opacity),
                        ..default()
                    },
                ))
                .id();
            commands.entity(root).add_child(bg_e);
        }

        // Clamp active_wheel_entry to a valid range.
        let n_wheels = count_wheel_entries(set);
        let target = if n_wheels == 0 {
            0
        } else {
            hud.active_wheel_entry.min(n_wheels - 1)
        };

        // Find the target-th Wheel / WheelSet entry.
        let mut rendered = false;
        let mut wcount = 0usize;
        for (ei, entry) in set.entries.iter().enumerate() {
            let is_wheel = matches!(entry, SetEntry::Wheel(_) | SetEntry::WheelSet(_));
            if !is_wheel {
                continue;
            }
            if wcount != target {
                wcount += 1;
                continue;
            }
            match entry {
                SetEntry::Wheel(w) => {
                    build_centered_wheel_hud(
                        commands,
                        root,
                        w,
                        hud.active_set,
                        ei,
                        None,
                        hud.highlighted,
                        wedge_materials,
                    );
                    rendered = true;
                }
                SetEntry::WheelSet(ws) => {
                    if let Some(w) = ws.wheels.first() {
                        build_centered_wheel_hud(
                            commands,
                            root,
                            w,
                            hud.active_set,
                            ei,
                            Some(0),
                            hud.highlighted,
                            wedge_materials,
                        );
                        rendered = true;
                    }
                }
                _ => {}
            }
            break;
        }
        if !rendered {
            hud_child(
                commands,
                root,
                hud_text("No wheels in this set.", 11., HUD_DIMMER),
            );
        }
        build_hud_action_buttons(commands, root, set, asset_server, icon_set);
    }
}

/// Renders a radial wheel preview centred in the HUD.
#[allow(clippy::too_many_arguments)]
pub fn build_centered_wheel_hud(
    commands: &mut Commands,
    parent: Entity,
    wheel: &WheelData,
    set: usize,
    entry: usize,
    w_idx: Option<usize>,
    highlighted: Option<(usize, usize, Option<usize>, usize)>,
    wedge_materials: &mut Assets<WedgeMaterial>,
) {
    let n_slices = wheel.slots.len().max(1);
    let hub = hud_child(commands, parent, wheel_hub());

    let is_pie = wheel.segment_shape == SegmentShape::Pie;
    if !is_pie {
        let bg_col = if wheel.bg_color.is_empty() {
            Color::srgba(0.096, 0.118, 0.157, wheel.bg_opacity)
        } else {
            parse_hex_color(&wheel.bg_color, wheel.bg_opacity)
        };
        hud_child(commands, hub, wheel_bg_disc(wheel.outer_radius, bg_col));
    }
    let outer_col = if wheel.outer_border.is_empty() {
        Color::srgba(0.75, 0.58, 0.15, 0.40)
    } else {
        parse_hex_color(&wheel.outer_border, 1.0)
    };
    let outer_bw = if wheel.outer_border.is_empty() {
        1.5_f32
    } else {
        wheel.outer_border_width.max(0.0)
    };
    hud_child(
        commands,
        hub,
        wheel_outer_ring(wheel.outer_radius, outer_col, outer_bw),
    );

    let slice_angle = std::f32::consts::TAU / n_slices as f32;
    let base_pw = (2.0 * wheel.outer_radius * (slice_angle / 2.0).sin() * 0.72).max(48.0);
    let base_ph = ((wheel.outer_radius - wheel.inner_radius) * 0.85).max(40.0);
    let panel_w = (base_pw * wheel.segment_scale).max(32.0);
    let panel_h = (base_ph * wheel.segment_scale).max(24.0);
    let min_dim = panel_w.min(panel_h);
    let highlight_col = parse_hex_color(&wheel.highlight_color, 1.0);
    let slice_bg = Color::srgb(0.13, 0.17, 0.23);
    let label_c = Color::srgb(0.84, 0.89, 0.94);
    let label_sz = (panel_h * 0.18).clamp(9.0, 13.0);

    for (i, slot) in wheel.slots.iter().enumerate() {
        if i >= n_slices {
            break;
        }
        let is_sel = highlighted
            .map(|(s, e, w, sl)| s == set && e == entry && w == w_idx && sl == i)
            .unwrap_or(false);
        let seg_color = if is_sel { highlight_col } else { slice_bg };

        if is_pie {
            let (a0, a1) = slice_angles(wheel, i);
            let mat_handle = wedge_materials.add(WedgeMaterial {
                params: WedgeParams {
                    color: seg_color.to_linear().to_vec4(),
                    inner_r: wheel.inner_radius,
                    outer_r: wheel.outer_radius,
                    angle_start: a0,
                    angle_end: a1,
                },
            });
            let dia = wheel.outer_radius * 2.0;
            let wedge_e = commands
                .spawn((
                    Node {
                        position_type: PositionType::Absolute,
                        left: Val::Px(-wheel.outer_radius),
                        top: Val::Px(-wheel.outer_radius),
                        width: Val::Px(dia),
                        height: Val::Px(dia),
                        ..default()
                    },
                    MaterialNode(mat_handle),
                ))
                .id();
            commands.entity(hub).add_child(wedge_e);
            let ctr = slice_center(wheel, i);
            let panel_e = commands
                .spawn_scene(bsn! {
                    Node {
                        position_type: PositionType::Absolute,
                        left:   {Val::Px(ctr.x - panel_w / 2.0)},
                        top:    {Val::Px(-ctr.y - panel_h / 2.0)},
                        width:  {Val::Px(panel_w)}, height: {Val::Px(panel_h)},
                        justify_content: JustifyContent::SpaceBetween,
                        align_items: AlignItems::Center,
                        flex_direction: FlexDirection::Column,
                        padding: {UiRect::all(Val::Px(6.))},
                    }
                    BackgroundColor({Color::NONE})
                })
                .id();
            commands.entity(hub).add_child(panel_e);
            if wheel.show_labels {
                hud_child(
                    commands,
                    panel_e,
                    wheel_slice_label(slot.name.to_uppercase(), label_sz, label_c),
                );
            }
            if wheel.show_icon && !slot.icon.is_empty() {
                hud_child(
                    commands,
                    panel_e,
                    wheel_slice_label(slot.icon.clone(), label_sz * 1.3, label_c),
                );
            } else if wheel.show_labels {
                hud_child(
                    commands,
                    panel_e,
                    bsn! { Node { width: {Val::Px(4.)}, height: {Val::Px(4.)} } },
                );
            }
        } else {
            let seg_br = match wheel.segment_shape {
                SegmentShape::Square => BorderRadius::all(Val::Px(0.0)),
                SegmentShape::Rounded => BorderRadius::all(Val::Px(min_dim * 0.14)),
                SegmentShape::Circle => BorderRadius::all(Val::Px(min_dim * 0.5)),
                SegmentShape::Wedge => BorderRadius {
                    top_left: Val::Px(min_dim * 0.40),
                    top_right: Val::Px(min_dim * 0.40),
                    bottom_left: Val::Px(min_dim * 0.05),
                    bottom_right: Val::Px(min_dim * 0.05),
                },
                SegmentShape::Pie => unreachable!(),
            };
            let ctr = slice_center(wheel, i);
            let panel_e = commands
                .spawn_scene(bsn! {
                    Node {
                        position_type: PositionType::Absolute,
                        left:   {Val::Px(ctr.x - panel_w / 2.0)},
                        top:    {Val::Px(-ctr.y - panel_h / 2.0)},
                        width:  {Val::Px(panel_w)}, height: {Val::Px(panel_h)},
                        justify_content: JustifyContent::SpaceBetween,
                        align_items: AlignItems::Center,
                        flex_direction: FlexDirection::Column,
                        padding: {UiRect::all(Val::Px(6.))},
                        border_radius: {seg_br},
                    }
                    BackgroundColor({seg_color})
                })
                .id();
            commands.entity(hub).add_child(panel_e);
            if wheel.show_labels {
                hud_child(
                    commands,
                    panel_e,
                    wheel_slice_label(slot.name.to_uppercase(), label_sz, label_c),
                );
            }
            if wheel.show_icon && !slot.icon.is_empty() {
                hud_child(
                    commands,
                    panel_e,
                    wheel_slice_label(slot.icon.clone(), label_sz * 1.3, label_c),
                );
            } else if wheel.show_labels {
                hud_child(
                    commands,
                    panel_e,
                    bsn! { Node { width: {Val::Px(4.)}, height: {Val::Px(4.)} } },
                );
            }
        }
    }

    // Centre hub ring.
    let disc_r = (wheel.inner_radius - 4.0).max(8.0);
    let ring_col = if wheel.inner_border.is_empty() {
        Color::srgb(0.82, 0.64, 0.16)
    } else {
        parse_hex_color(&wheel.inner_border, 1.0)
    };
    let hub_bg = if wheel.hub_color.is_empty() {
        Color::srgba(0.08, 0.10, 0.14, wheel.hub_opacity)
    } else {
        parse_hex_color(&wheel.hub_color, wheel.hub_opacity)
    };
    let inner_bw = if wheel.inner_border.is_empty() {
        3.0_f32
    } else {
        wheel.inner_border_width.max(0.0)
    };
    let center = hud_child(
        commands,
        hub,
        wheel_center_ring(disc_r, hub_bg, ring_col, inner_bw),
    );

    // Show highlighted slot info; show nothing by default.
    let hub_slot = highlighted.and_then(|(hs, he, hw, si)| {
        if hs == set && he == entry && hw == w_idx {
            wheel.slots.get(si)
        } else {
            None
        }
    });
    if let Some(slot) = hub_slot {
        let info_col = hud_child(
            commands,
            center,
            bsn! {
                Node {
                    flex_direction: FlexDirection::Column,
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::Center,
                    row_gap: {Val::Px(2.)},
                }
            },
        );
        let icon_sz = (disc_r * 0.55).clamp(10.0, 22.0);
        let name_sz = (disc_r * 0.22).clamp(7.0, 10.0);
        if !slot.icon.is_empty() {
            hud_child(commands, info_col, hud_text(&slot.icon, icon_sz, HUD_TEXT));
        }
        if !slot.name.is_empty() {
            hud_child(
                commands,
                info_col,
                hud_text(&slot.name.to_uppercase(), name_sz, HUD_DIM),
            );
        }
    }
}

/// Floating quick-action buttons in the bottom-right corner.
fn build_hud_action_buttons(
    commands: &mut Commands,
    parent: Entity,
    set: &ActionSet,
    asset_server: &AssetServer,
    icon_set: GamepadIconSet,
) {
    let btns: Vec<&QuickAction> = set
        .entries
        .iter()
        .filter_map(|e| {
            if let SetEntry::Action(a) = e {
                Some(a)
            } else {
                None
            }
        })
        .filter(|a| a.enabled)
        .collect();
    if btns.is_empty() {
        return;
    }

    let container = commands
        .spawn_scene(bsn! {
            Node {
                position_type: PositionType::Absolute,
                bottom: {Val::Px(60.)}, right: {Val::Px(36.)},
                flex_direction: FlexDirection::Column,
                row_gap: {Val::Px(8.)},
                align_items: AlignItems::FlexEnd,
            }
        })
        .id();
    commands.entity(parent).add_child(container);

    for qa in btns.iter().rev() {
        let eff = (set.opacity * qa.opacity).clamp(0.05, 1.0);
        let w = qa.width.max(40.0);
        let h = qa.height.max(20.0);
        let bg = parse_hex_color(&qa.color, eff * 0.85);
        let tc = HUD_TEXT.with_alpha(eff);
        let bc = HUD_BADGE_BORDER.with_alpha(eff);

        let row = hud_child(
            commands,
            container,
            bsn! {
                Node {
                    flex_direction: FlexDirection::Row,
                    align_items: AlignItems::Center,
                    column_gap: {Val::Px(5.)},
                }
            },
        );
        if !qa.key.is_empty() {
            // GP: key → show the controller button icon; keyboard key → text badge.
            let mut showed_icon = false;
            if let Some(btn_label) = qa.key.strip_prefix("GP:") {
                if let Some(path) = icon_set.icon_path(btn_label) {
                    let handle = asset_server.load::<Image>(path);
                    let icon_e = commands
                        .spawn((
                            Node {
                                width: Val::Px(22.0),
                                height: Val::Px(22.0),
                                ..default()
                            },
                            ImageNode::new(handle),
                        ))
                        .id();
                    commands.entity(row).add_child(icon_e);
                    showed_icon = true;
                }
            }
            if !showed_icon {
                // Keyboard fallback — bordered text badge.
                let key_disp = qa.key.strip_prefix("GP:").unwrap_or(&qa.key);
                let kb = hud_child(
                    commands,
                    row,
                    bsn! {
                        Node {
                            min_width: {Val::Px(16.)}, height: {Val::Px(16.)},
                            padding: {UiRect::horizontal(Val::Px(3.))},
                            justify_content: JustifyContent::Center,
                            align_items: AlignItems::Center,
                            border: {UiRect::all(Val::Px(1.))},
                            border_radius: {BorderRadius::all(Val::Px(2.))},
                        }
                        BorderColor::all(HUD_BADGE_BORDER)
                    },
                );
                hud_child(commands, kb, hud_text(key_disp, 8., HUD_DIM));
            }
        }
        let btn_node = commands
            .spawn_scene(bsn! {
                Node {
                    width: {Val::Px(w)}, height: {Val::Px(h)},
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    border: {UiRect::all(Val::Px(1.))},
                    border_radius: {BorderRadius::all(Val::Px(4.))},
                }
                BackgroundColor({bg})
                BorderColor::all(bc)
            })
            .id();
        commands.entity(row).add_child(btn_node);
        hud_child(commands, btn_node, hud_text(&qa.name, 10., tc));
    }
}

/// Set-selection tab bar pinned to the bottom of the HUD.
fn build_hud_set_tabs(
    commands: &mut Commands,
    parent: Entity,
    cfg: &QuickActionConfig,
    hud: &WheelHudState,
    asset_server: &AssetServer,
    icon_set: GamepadIconSet,
) {
    let bar = commands
        .spawn_scene(bsn! {
            Node {
                position_type: PositionType::Absolute,
                top: {Val::Px(12.)}, left: {Val::Px(0.)}, right: {Val::Px(0.)},
                flex_direction: FlexDirection::Row,
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
            }
        })
        .id();
    commands.entity(parent).add_child(bar);

    let prev_idx = hud.active_set.saturating_sub(1);
    let larrow = hud_clickable(
        commands,
        bar,
        bsn! {
            Node {
                width: {Val::Px(28.)}, height: {Val::Px(32.)},
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                border: {UiRect::all(Val::Px(1.))},
                border_radius: {BorderRadius::left(Val::Px(6.))},
            }
            BorderColor::all(HUD_SIDEBAR_BORDER)
            BackgroundColor({HUD_PANEL_CARD})
            Button
        },
        WheelHudAction::SetActiveSet(prev_idx),
        HUD_PANEL_CARD,
    );
    hud_child(commands, larrow, hud_text("‹", 14., HUD_DIM));
    // Overlay the assigned prev-set icon if it's a gamepad button.
    if let Some(lbl) = cfg.prev_set_key.strip_prefix("GP:") {
        if let Some(path) = icon_set.icon_path(lbl) {
            let handle = asset_server.load::<Image>(path);
            let e = commands
                .spawn((
                    Node {
                        width: Val::Px(18.0),
                        height: Val::Px(18.0),
                        ..default()
                    },
                    ImageNode::new(handle),
                ))
                .id();
            commands.entity(larrow).add_child(e);
        }
    }

    for (i, set) in cfg.sets.iter().enumerate() {
        let active = i == hud.active_set;
        let (bg, tc, bc) = if active {
            (Color::srgba(0.38, 0.62, 0.95, 0.20), HUD_TEXT, HUD_BLUE)
        } else {
            (HUD_PANEL_CARD, HUD_DIM, HUD_SIDEBAR_BORDER)
        };
        let tab = hud_clickable(
            commands,
            bar,
            bsn! {
                Node {
                    padding: {UiRect::axes(Val::Px(14.), Val::Px(7.))},
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    border: {UiRect::all(Val::Px(1.))},
                }
                BorderColor::all(bc)
                BackgroundColor({bg})
                Button
            },
            WheelHudAction::SetActiveSet(i),
            bg,
        );
        hud_child(commands, tab, hud_text(&set.name, 11., tc));
    }

    let next_idx = (hud.active_set + 1).min(cfg.sets.len().saturating_sub(1));
    let rarrow = hud_clickable(
        commands,
        bar,
        bsn! {
            Node {
                width: {Val::Px(28.)}, height: {Val::Px(32.)},
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                border: {UiRect::all(Val::Px(1.))},
                border_radius: {BorderRadius::right(Val::Px(6.))},
            }
            BorderColor::all(HUD_SIDEBAR_BORDER)
            BackgroundColor({HUD_PANEL_CARD})
            Button
        },
        WheelHudAction::SetActiveSet(next_idx),
        HUD_PANEL_CARD,
    );
    hud_child(commands, rarrow, hud_text("›", 14., HUD_DIM));
    // Overlay the assigned next-set icon if it's a gamepad button.
    if let Some(lbl) = cfg.next_set_key.strip_prefix("GP:") {
        if let Some(path) = icon_set.icon_path(lbl) {
            let handle = asset_server.load::<Image>(path);
            let e = commands
                .spawn((
                    Node {
                        width: Val::Px(18.0),
                        height: Val::Px(18.0),
                        ..default()
                    },
                    ImageNode::new(handle),
                ))
                .id();
            commands.entity(rarrow).add_child(e);
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────────
// WheelHudPlugin
// ─────────────────────────────────────────────────────────────────────────────────

/// Renders a full-screen HUD showing the active [`QuickActionConfig`] set.
///
/// Add this plugin (alongside [`WheelMenuPlugin`]) to display wheels and
/// quick-action buttons.  Add [`crate::editor::QuickActionEditorPlugin`] on top
/// to get the editor sidebar.
///
/// ```ignore
/// app.add_plugins((WheelMenuPlugin, WheelHudPlugin));
/// ```
/// Backward-compat wrapper — use [`QuickActionHudPlugin::default()`] instead.
///
/// Provides core wheel logic + HUD canvas (no editor).
pub struct WheelHudPlugin;
impl Plugin for WheelHudPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(QuickActionHudPlugin::default());
    }
}

fn hud_button_feedback(mut buttons: Query<(&WheelHudButton, &Interaction, &mut BackgroundColor)>) {
    for (btn, interaction, mut bg) in &mut buttons {
        *bg = match interaction {
            Interaction::Hovered => BackgroundColor(Color::srgba(1., 1., 1., 0.05)),
            Interaction::Pressed => BackgroundColor(Color::srgba(0.38, 0.62, 0.95, 0.16)),
            Interaction::None => BackgroundColor(btn.base),
        };
    }
}

/// Reads the right-stick axis ([`WheelNavAction::Navigate`]) and updates
/// [`WheelHudState::highlighted`] while the HUD wheel is open.
///
/// Uses **release-to-use**: the slot that was highlighted when the stick
/// returns to the dead-zone is emitted as a [`HudSegmentSelected`] event.
fn hud_stick_nav(
    nav_q: Query<&ActionState<WheelNavAction>>,
    mut hud: ResMut<WheelHudState>,
    cfg: Res<QuickActionConfig>,
    mut select_ev: MessageWriter<HudSegmentSelected>,
) {
    if !hud.open {
        return;
    }
    let Ok(action) = nav_q.single() else {
        return;
    };
    let stick = action.axis_pair(&WheelNavAction::Navigate);

    // Locate the active wheel entry (honoring active_wheel_entry).
    let Some(set) = cfg.sets.get(hud.active_set) else {
        return;
    };
    let mut found: Option<(usize, Option<usize>, usize)> = None;
    let target = hud.active_wheel_entry;
    let mut wcount = 0usize;
    for (ei, entry) in set.entries.iter().enumerate() {
        let is_wheel = matches!(entry, SetEntry::Wheel(_) | SetEntry::WheelSet(_));
        if !is_wheel {
            continue;
        }
        if wcount != target {
            wcount += 1;
            continue;
        }
        match entry {
            SetEntry::Wheel(w) => {
                found = Some((ei, None, w.slots.len()));
            }
            SetEntry::WheelSet(ws) => {
                if let Some(w) = ws.wheels.first() {
                    found = Some((ei, Some(0), w.slots.len()));
                }
            }
            _ => {}
        }
        break;
    }
    let Some((entry_idx, wheel_idx, n_slots)) = found else {
        return;
    };
    if n_slots == 0 {
        return;
    }

    const DEADZONE: f32 = 0.2;
    let prev = hud.highlighted;

    let new_highlight = if stick.length() < DEADZONE {
        None
    } else {
        // Same angle mapping as WheelData::arc_offset default (FRAC_PI_6).
        let a = stick.y.atan2(stick.x);
        let rel = (a - std::f32::consts::FRAC_PI_6).rem_euclid(std::f32::consts::TAU);
        let idx = ((rel / std::f32::consts::TAU) * n_slots as f32).floor() as usize;
        Some((hud.active_set, entry_idx, wheel_idx, idx.min(n_slots - 1)))
    };

    if prev != new_highlight {
        // Release-to-use: emit selection when stick returns to dead-zone.
        if let (Some((s, e, w, slot)), None) = (prev, new_highlight) {
            select_ev.write(HudSegmentSelected {
                set: s,
                entry: e,
                wheel: w,
                slot,
            });
        }
        hud.highlighted = new_highlight;
        hud.dirty = true;
    }
}

fn rebuild_hud(
    mut commands: Commands,
    mut hud: ResMut<WheelHudState>,
    cfg: Res<QuickActionConfig>,
    asset_server: Res<AssetServer>,
    icon_set: Res<GamepadIconSet>,
    old_hud: Query<Entity, With<WheelHudRoot>>,
    mut wedge_materials: ResMut<Assets<WedgeMaterial>>,
) {
    if !hud.dirty {
        return;
    }
    hud.dirty = false;

    for e in &old_hud {
        commands.entity(e).despawn();
    }

    if !cfg.sets.is_empty() && hud.active_set >= cfg.sets.len() {
        hud.active_set = cfg.sets.len() - 1;
    }

    build_hud_canvas(
        &mut commands,
        &cfg,
        &hud,
        &*asset_server,
        *icon_set,
        &mut wedge_materials,
    );
}

/// Runs in [`PostStartup`] when the HUD is enabled.
///
/// Reads [`CONFIG_FILE`] from the working directory and, if it exists and
/// parses cleanly, replaces the active [`QuickActionConfig`] resource.
///
/// Game `Startup` systems run first (setting game-specific defaults), then
/// this silently applies the user's saved preferences on top.
fn try_autoload_config(mut cfg: ResMut<QuickActionConfig>, mut hud: ResMut<WheelHudState>) {
    match std::fs::read_to_string(CONFIG_FILE) {
        Err(_) => {} // File absent — keep whatever Startup set.
        Ok(s) => match ron::from_str::<QuickActionConfig>(&s) {
            Ok(loaded) => {
                *cfg = loaded;
                hud.dirty = true;
                info!("[wheel_menu] config auto-loaded from {CONFIG_FILE}");
            }
            Err(e) => warn!("[wheel_menu] failed to parse {CONFIG_FILE}: {e}"),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn ron_config_parses() {
        let src = include_str!("../quickactions_config.ron");
        let _: QuickActionConfig = ron::from_str(src).expect("RON round-trip failed");
    }
}
