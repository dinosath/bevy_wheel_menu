
//! Headless wheel menu library for Bevy.
//! 
//! This library provides the logic and data structures for wheel menus.
//! Rendering is left to the application.

pub mod mesh;
pub mod editor;

use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Configuration for a wheel menu.
#[derive(Component, Clone)]
pub struct WheelMenu {
    /// Number of slices in the wheel.
    pub slices: usize,
    /// Outer radius of the wheel.
    pub radius: f32,
    /// Inner radius (hole in the center).
    pub inner_radius: f32,
    /// Deadzone for input (0.0 - 1.0).
    pub deadzone: f32,
    /// Gap between slices in radians.
    pub gap: f32,
    /// Total angular span of the wheel in radians.  `TAU` is a full circle;
    /// `PI` is a half wheel, etc.  Slices are distributed across this span.
    pub arc_span: f32,
    /// Angle (radians) at which the first slice begins, measured CCW from the
    /// +X axis.  Use this to rotate / re-anchor a partial-arc wheel.
    pub arc_offset: f32,
    /// When `true`, slices touch (no gap) and panels are sized to overlap.
    pub overlap: bool,
}

impl Default for WheelMenu {
    fn default() -> Self {
        Self {
            slices: 8,
            radius: 120.0,
            inner_radius: 40.0,
            deadzone: 0.25,
            gap: 0.02,
            arc_span: std::f32::consts::TAU,
            arc_offset: 0.0,
            overlap: false,
        }
    }
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
    Weapon     { name: String, icon: String },
    Spell      { name: String, icon: String },
    Consumable { name: String, icon: String, count: u32 },
    Shout      { name: String, icon: String },
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
        Self { items, current_item: 0 }
    }
    pub fn current(&self) -> Option<&ActionItem> {
        self.items.get(self.current_item)
    }
    pub fn cycle_next(&mut self) {
        if self.items.is_empty() { return; }
        self.current_item = (self.current_item + 1) % self.items.len();
    }
    pub fn cycle_prev(&mut self) {
        if self.items.is_empty() { return; }
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
/// Attach alongside [`WheelMenu`] and [`WheelState`].  Enum variants guarantee
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
    pub fn base(&self) -> Color { Color::srgba(self.base_color[0], self.base_color[1], self.base_color[2], self.base_color[3]) }
    /// Convert the stored hover color into a Bevy [`Color`].
    pub fn hover(&self) -> Color { Color::srgba(self.hover_color[0], self.hover_color[1], self.hover_color[2], self.hover_color[3]) }
    /// Convert the stored selected color into a Bevy [`Color`].
    pub fn selected(&self) -> Color { Color::srgba(self.selected_color[0], self.selected_color[1], self.selected_color[2], self.selected_color[3]) }
    /// Convert the stored text color into a Bevy [`Color`].
    pub fn text(&self) -> Color { Color::srgba(self.text_color[0], self.text_color[1], self.text_color[2], self.text_color[3]) }
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

/// Plugin that provides headless wheel-menu logic.
///
/// Registers all messages and chains all built-in systems.  See the examples
/// for how to wire rendering and game-specific reactions on top.
pub struct WheelMenuPlugin;

/// Alias so call-sites can use either name.
pub type ActionWheelPlugin = WheelMenuPlugin;

impl Plugin for WheelMenuPlugin {
    fn build(&self, app: &mut App) {
        app
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
            .add_systems(Update, (
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
            ).chain());
    }
}

/// System that reads gamepad input and updates WheelState.
pub fn update_wheel_input(
    gamepads: Query<&Gamepad>,
    mut q: Query<&mut WheelState>,
) {
    for mut state in &mut q {
        state.dir = Vec2::ZERO;
        for gamepad in &gamepads {
            let x = gamepad.get(GamepadAxis::LeftStickX).unwrap_or(0.0);
            let y = gamepad.get(GamepadAxis::LeftStickY).unwrap_or(0.0);
            let v = Vec2::new(x, y);
            if v.length() > 0.25 {
                state.dir = v.normalize();
            }
        }
    }
}

/// Determines which slice is hovered and handles per-mode activation:
/// - [`CastingMode::ReleaseToUse`]: fires [`WheelMenuSelected`] when the stick
///   returns to centre.
/// - [`CastingMode::Direct`]: fires [`WheelMenuSelected`] immediately on hover.
pub fn update_wheel_hover(
    mut q: Query<(Entity, &WheelMenu, &mut WheelState, Option<&WheelMenuConfig>)>,
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
                let idx = ((rel / menu.arc_span) * menu.slices as f32).floor() as usize;
                state.hovered = Some(idx.min(menu.slices.saturating_sub(1)));
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
                        _ => continue,              // another mode is active
                    }
                }
                if let Some(i) = state.hovered {
                    ev.write(WheelMenuSelected { index: i, menu_entity: entity });
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
            opened_ev.write(WheelOpened { menu_entity: entity });
        } else if !is_open && state.open {
            state.open = false;
            closed_ev.write(WheelClosed { menu_entity: entity });
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
        if !next && !prev { continue; }

        for (menu_entity, state) in &wheel_q {
            if let Some(hovered) = state.hovered {
                for (slice, mut slot) in &mut slot_q {
                    if slice.index == hovered {
                        let previous_item = slot.current_item;
                        if next { slot.cycle_next(); } else { slot.cycle_prev(); }
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
pub fn update_wheel_time_scale(
    q: Query<&WheelMenuConfig>,
    mut time: ResMut<Time<Virtual>>,
) {
    let effective = q.iter().fold(1.0_f32, |acc, cfg| {
        let scale = match cfg.time_mode {
            TimeMode::Normal  => 1.0,
            TimeMode::Slow(s) => s,
            TimeMode::Pause   => 0.0,
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
                hold.holding  = false;
                continue;
            }
        };
        match state.hovered {
            Some(index) => {
                hold.holding  = true;
                hold.progress = (hold.progress + time.delta_secs() / duration).clamp(0.0, 1.0);
                progress_ev.write(WheelMenuHoldProgress {
                    index,
                    progress: hold.progress,
                    menu_entity: entity,
                });
                if hold.progress >= 1.0 {
                    activate_ev.write(WheelMenuHoldActivated { index, menu_entity: entity });
                    hold.progress = 0.0;
                }
            }
            None => {
                hold.holding  = false;
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
                ev.write(WheelSwitched { previous, current: set.active, menu_entity: entity });
            }
            if gamepad.just_pressed(set.prev_button) {
                let previous = set.active;
                set.active = (set.active + set.count - 1) % set.count;
                ev.write(WheelSwitched { previous, current: set.active, menu_entity: entity });
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
    mut q: Query<(Entity, &WheelMenu, &WheelState, &mut WheelEditMode)>,
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
                        && hovered + 1 < menu.slices
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
                commands.entity(menu).insert(ActiveSlotContext { slot_entity });
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
        (Entity, Option<&WheelInputOverride>, Option<&ActiveSlotContext>),
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
pub fn slice_angles(menu: &WheelMenu, index: usize) -> (f32, f32) {
    let slice_angle = menu.arc_span / menu.slices as f32;
    let half_gap = if menu.overlap { 0.0 } else { menu.gap / 2.0 };
    let a0 = menu.arc_offset + index as f32 * slice_angle + half_gap;
    let a1 = menu.arc_offset + (index + 1) as f32 * slice_angle - half_gap;
    (a0, a1)
}

/// Helper to get the center position of a slice (for placing icons/text).
pub fn slice_center(menu: &WheelMenu, index: usize) -> Vec2 {
    let (a0, a1) = slice_angles(menu, index);
    let center_angle = (a0 + a1) / 2.0;
    let center_radius = (menu.inner_radius + menu.radius) / 2.0;
    Vec2::new(center_angle.cos() * center_radius, center_angle.sin() * center_radius)
}

/// Returns a full-screen `bevy_ui` overlay [`Node`] that centers its children,
/// authored with the [`bsn!`] macro.
///
/// Spawn it with `commands.spawn_scene(wheel_overlay())` and attach the
/// wheel-menu logic components ([`WheelMenu`] and [`WheelState`]) to the
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
    menu: &WheelMenu,
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
    menu: &WheelMenu,
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
