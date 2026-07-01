//! BSN-macro Quick Action Menu editor — v2 layout.
//!
//! ## Layout
//! * **Left sidebar** is context-sensitive:
//!   - Default: **navigation view** — wheel-set tree and button list for the
//!     active set, plus a set-switch key summary at the bottom.
//!   - When an item is selected: **editor panel** for that item (wheel, button,
//!     or wheel-set) with a `‹ Back` breadcrumb header.
//! * **Right canvas** always shows the **HUD preview** — the active set's wheel
//!   centred in the viewport, floating action buttons, and set tabs at the bottom.

use crate::*;
use bevy::color::Alpha;
use bevy::ecs::message::MessageReader;
use bevy::input::keyboard::{Key, KeyboardInput};
use bevy::input::ButtonState;
use bevy::prelude::*;
use serde::{Deserialize, Serialize};

// ─── document model ─────────────────────────────────────────────────────────────

/// Placement reference for a quick action's on-screen position.
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
    fn next(self) -> Self {
        match self {
            Self::Relative => Self::Absolute,
            Self::Absolute => Self::Relative,
        }
    }
}

/// Shape of a quick-action button.
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
    fn next(self) -> Self {
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
    fn next(self) -> Self {
        match self {
            Self::Dark => Self::Light,
            Self::Light => Self::Dark,
        }
    }
}

/// Shape of each segment panel in a wheel.
#[derive(Clone, Copy, PartialEq, Serialize, Deserialize, Debug, Default)]
pub enum SegmentShape {
    #[default]
    Rounded,
    Square,
    Circle,
    /// Asymmetric radius – outer corners large, inner corners small.
    Wedge,
    /// True pie / wedge mesh rendered with curved arcs (uses `Mesh2d`).
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
    fn next(self) -> Self {
        match self {
            Self::Rounded => Self::Square,
            Self::Square => Self::Circle,
            Self::Circle => Self::Wedge,
            Self::Wedge => Self::Pie,
            Self::Pie => Self::Rounded,
        }
    }
}

const ICON_PALETTE: &[&str] = &["◆", "●", "★", "▲", "✦", "✚", "◈", "○", "◐", "✱"];
const COMMAND_PALETTE: &[&str] = &[
    "none", "attack", "heal", "block", "dash", "reload", "interact", "jump", "crouch", "sprint",
];

fn cycle_in<'a>(list: &[&'a str], current: &str) -> &'a str {
    let idx = list.iter().position(|s| *s == current).unwrap_or(0);
    list[(idx + 1) % list.len()]
}

// ── serde defaults ──────────────────────────────────────────────────────────────

fn default_true() -> bool {
    true
}
fn default_action_color() -> String {
    "#3b82f6".into()
}
fn default_action_width() -> f32 {
    80.0
}
fn default_action_height() -> f32 {
    28.0
}
fn default_outer_radius() -> f32 {
    110.0
}
fn default_inner_radius() -> f32 {
    38.0
}
fn default_anim_speed() -> f32 {
    150.0
}
fn full_opacity() -> f32 {
    1.0
}
fn default_highlight_color() -> String {
    "#f59e0b".into()
}
fn default_segment_scale() -> f32 {
    1.0
}

// ── slot data model ─────────────────────────────────────────────────────────────

/// A single selectable item within a wheel slot (for multi-item cycling).
#[derive(Clone, Serialize, Deserialize, Debug, Default)]
pub struct SlotItem {
    pub name: String,
    pub icon: String,
}

/// All per-slot data for a single wheel segment.
#[derive(Clone, Serialize, Deserialize, Debug, Default)]
#[serde(default)]
pub struct WheelSlotData {
    pub name: String,
    /// Unicode symbol / emoji shown in the wheel preview.
    pub icon: String,
    /// Captured input label for this slot (keyboard key or "GP:…" gamepad button).
    pub input: String,
    /// Optional list of items the player can cycle through.
    pub items: Vec<SlotItem>,
}

impl WheelSlotData {
    fn named(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            ..Default::default()
        }
    }
}

// ── structs ─────────────────────────────────────────────────────────────────────

/// A key-bound quick-action button shown as a floating HUD element.
#[derive(Clone, Serialize, Deserialize, Debug)]
#[serde(default)]
pub struct QuickAction {
    pub name: String,
    /// Captured keyboard key-binding label (e.g. `"E"`). Empty means unbound.
    pub key: String,
    /// Captured gamepad button label (e.g. `"A"`). Empty means unbound.
    #[serde(default)]
    pub gamepad_button: String,
    pub icon: String,
    pub command: String,
    pub hold: bool,
    pub show_on_menu: bool,
    pub opacity: f32,
    pub position: PositionMode,
    pub radius: f32,
    pub shape: ActionShape,
    /// CSS hex color for the button (e.g. `"#8b5cf6"`).
    #[serde(default = "default_action_color")]
    pub color: String,
    #[serde(default = "default_action_width")]
    pub width: f32,
    #[serde(default = "default_action_height")]
    pub height: f32,
    #[serde(default = "default_true")]
    pub enabled: bool,
}
impl Default for QuickAction {
    fn default() -> Self {
        Self {
            name: "Action".into(),
            key: String::new(),
            gamepad_button: String::new(),
            icon: "◆".into(),
            command: "none".into(),
            hold: false,
            show_on_menu: true,
            opacity: 1.0,
            position: PositionMode::Relative,
            radius: 48.0,
            shape: ActionShape::Rounded,
            color: default_action_color(),
            width: default_action_width(),
            height: default_action_height(),
            enabled: true,
        }
    }
}

/// A radial wheel: named menu with labelled slots, configurable size / style.
#[derive(Clone, Serialize, Deserialize, Debug)]
#[serde(default)]
pub struct Wheel {
    pub name: String,
    pub cooldown_secs: f32,
    /// Per-segment data (name, icon, input binding, items).
    pub slots: Vec<WheelSlotData>,
    /// Key held to open / activate this wheel.
    #[serde(default)]
    pub hold_key: String,
    #[serde(default)]
    pub theme: WheelTheme,
    #[serde(default = "default_outer_radius")]
    pub outer_radius: f32,
    #[serde(default = "default_inner_radius")]
    pub inner_radius: f32,
    #[serde(default = "default_anim_speed")]
    pub anim_speed_ms: f32,
    #[serde(default = "default_true")]
    pub show_labels: bool,
    #[serde(default)]
    pub show_info_in_hub: bool,
    #[serde(default)]
    pub segment_shape: SegmentShape,
    #[serde(default = "default_true")]
    pub show_icon: bool,
    #[serde(default = "default_highlight_color")]
    pub highlight_color: String,
    #[serde(default = "default_segment_scale")]
    pub segment_scale: f32,
}
impl Default for Wheel {
    fn default() -> Self {
        Self {
            name: "Wheel".into(),
            cooldown_secs: 6.0,
            slots: vec![WheelSlotData::named("Slot 1")],
            hold_key: String::new(),
            theme: WheelTheme::Dark,
            outer_radius: default_outer_radius(),
            inner_radius: default_inner_radius(),
            anim_speed_ms: default_anim_speed(),
            show_labels: true,
            show_info_in_hub: false,
            segment_shape: SegmentShape::Rounded,
            show_icon: true,
            highlight_color: "#f59e0b".into(),
            segment_scale: 1.0,
        }
    }
}
impl Wheel {
    pub fn new(name: impl Into<String>, n: usize) -> Self {
        Self {
            name: name.into(),
            slots: (0..n.max(1))
                .map(|i| WheelSlotData::named(format!("Slot {}", i + 1)))
                .collect(),
            ..Default::default()
        }
    }
    /// Build a runtime [`WheelMenu`] using this wheel's radius settings.
    pub fn to_menu(&self) -> WheelMenu {
        WheelMenu {
            slices: self.slots.len().max(1),
            radius: self.outer_radius.max(40.0),
            inner_radius: self.inner_radius.max(8.0),
            deadzone: 0.3,
            gap: 0.04,
            arc_span: std::f32::consts::TAU,
            // π/6 puts the first boundary straight up so the wheel
            // has left/right symmetry (MAP upper-left, ATTACK upper-right …).
            arc_offset: std::f32::consts::FRAC_PI_6,
            overlap: false,
        }
    }
}

/// A group of wheels the player can switch between.
#[derive(Clone, Serialize, Deserialize, Debug)]
#[serde(default)]
pub struct WheelSet {
    pub name: String,
    pub wheels: Vec<Wheel>,
    /// Key that cycles through wheels in this set.
    #[serde(default)]
    pub switch_key: String,
}
impl Default for WheelSet {
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
    Wheel(Wheel),
    WheelSet(WheelSet),
}

/// A set of quick actions / wheels bound to a gameplay context.
#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct ActionSet {
    pub name: String,
    #[serde(default = "full_opacity")]
    pub opacity: f32,
    #[serde(default)]
    pub input_override: bool,
    pub entries: Vec<SetEntry>,
}

/// The complete editable document (lives as a Bevy [`Resource`]).
#[derive(Resource, Clone, Serialize, Deserialize, Debug)]
pub struct QuickActionConfig {
    #[serde(default)]
    pub next_set_key: String,
    #[serde(default)]
    pub prev_set_key: String,
    pub sets: Vec<ActionSet>,
}

impl Default for QuickActionConfig {
    fn default() -> Self {
        let mut combat_wheel = Wheel::new("Combat Wheel", 6);
        combat_wheel.hold_key = "Q".into();
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
            sets: vec![
                ActionSet {
                    name: "Combat".into(),
                    opacity: 1.0,
                    input_override: false,
                    entries: vec![
                        SetEntry::WheelSet(WheelSet {
                            name: "Wheel Set".into(),
                            switch_key: String::new(),
                            wheels: vec![combat_wheel, Wheel::new("Wheel 2", 6)],
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
                },
                ActionSet {
                    name: "Stealth".into(),
                    opacity: 1.0,
                    input_override: false,
                    entries: vec![
                        SetEntry::WheelSet(WheelSet {
                            name: "Stealth Wheels".into(),
                            switch_key: String::new(),
                            wheels: vec![Wheel::new("Stealth Wheel", 4)],
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
                },
            ],
        }
    }
}

// ─── selection & runtime state ───────────────────────────────────────────────────

#[derive(Clone, Copy, PartialEq, Debug, Default)]
pub enum Selection {
    #[default]
    None,
    Action {
        set: usize,
        entry: usize,
    },
    Wheel {
        set: usize,
        entry: usize,
        wheel: Option<usize>,
    },
    Set {
        set: usize,
    },
    SetSwitch,
    /// A [`SetEntry::WheelSet`] entry (to edit its name / switch-key).
    WheelSetEntry {
        set: usize,
        entry: usize,
    },
    Segment {
        set: usize,
        entry: usize,
        wheel: Option<usize>,
        slot: usize,
    },
}

#[derive(Clone, Copy, PartialEq, Debug, Default)]
pub enum EditFocus {
    #[default]
    None,
    Name,
    Key,
    /// Capturing a gamepad button for a quick action.
    GamepadButton,
    SetName,
    WheelName,
    SlotName(usize),
    NextSetKey,
    PrevSetKey,
    /// Capturing the hold-key for the selected wheel.
    WheelHoldKey,
    /// Typing the name of the selected wheel-set entry.
    WheelSetName,
    /// Capturing the cycle-key for the selected wheel-set entry.
    WheelSetSwitchKey,
    SlotIcon(usize),
    /// Capturing an input binding for a segment (keyboard or gamepad).
    SlotInput(usize),
    /// Editing the name of item `item` inside slot `slot`.
    SlotItemName(usize, usize),
    /// Editing the icon of item `item` inside slot `slot`.
    SlotItemIcon(usize, usize),
}

#[derive(Resource)]
pub struct EditorUiState {
    pub dirty: bool,
    pub selection: Selection,
    pub editing: EditFocus,
    pub config_path: String,
    /// Index of the set currently shown in the HUD preview.
    pub active_set: usize,
}
impl Default for EditorUiState {
    fn default() -> Self {
        Self {
            dirty: true,
            selection: Selection::None,
            editing: EditFocus::None,
            config_path: "quickactions_config.ron".into(),
            active_set: 0,
        }
    }
}

#[derive(Component)]
pub struct EditorRoot;
#[derive(Component)]
pub struct EditorCanvasRoot;
/// Marks `Mesh2d` entities spawned for the Pie-shape preview —
/// despawned by `rebuild_editor` on every UI rebuild.
#[derive(Component)]
pub struct WheelMeshPreview;
#[derive(Component)]
pub struct SegmentHoverColor(pub Color);

#[derive(Component, Clone)]
pub struct EditorButton {
    pub action: EditorAction,
    pub base: Color,
}

#[derive(Clone, Debug)]
pub enum EditorAction {
    // ── sets ────────────────────────────────────────────────────────────────
    AddSet,
    DeleteSet {
        set: usize,
    },
    SelectSet {
        set: usize,
    },
    EditSetName {
        set: usize,
    },
    SetOpacityDelta {
        set: usize,
        delta: f32,
    },
    ToggleInputOverride {
        set: usize,
    },
    /// Switch the set shown in the HUD canvas.
    SetActiveSet {
        set: usize,
    },
    // ── entries ─────────────────────────────────────────────────────────────
    AddAction {
        set: usize,
    },
    AddWheel {
        set: usize,
    },
    AddWheelSet {
        set: usize,
    },
    AddWheelToSet {
        set: usize,
        entry: usize,
    },
    DeleteEntry {
        set: usize,
        entry: usize,
    },
    DeleteWheelFromSet {
        set: usize,
        entry: usize,
        wheel: usize,
    },
    MoveEntryUp {
        set: usize,
        entry: usize,
    },
    MoveEntryDown {
        set: usize,
        entry: usize,
    },
    // ── selection ───────────────────────────────────────────────────────────
    SelectAction {
        set: usize,
        entry: usize,
    },
    SelectWheel {
        set: usize,
        entry: usize,
        wheel: Option<usize>,
    },
    SelectWheelSetEntry {
        set: usize,
        entry: usize,
    },
    SelectSetSwitch,
    /// Return to the navigation sidebar view.
    NavBack,
    // ── quick action editing ─────────────────────────────────────────────────
    EditName {
        set: usize,
        entry: usize,
    },
    CaptureKey {
        set: usize,
        entry: usize,
    },
    CycleIcon {
        set: usize,
        entry: usize,
    },
    CycleCommand {
        set: usize,
        entry: usize,
    },
    ToggleHold {
        set: usize,
        entry: usize,
    },
    ToggleShowOnMenu {
        set: usize,
        entry: usize,
    },
    ToggleEnabled {
        set: usize,
        entry: usize,
    },
    OpacityDelta {
        set: usize,
        entry: usize,
        delta: f32,
    },
    RadiusDelta {
        set: usize,
        entry: usize,
        delta: f32,
    },
    ActionWidthDelta {
        set: usize,
        entry: usize,
        delta: f32,
    },
    ActionHeightDelta {
        set: usize,
        entry: usize,
        delta: f32,
    },
    CyclePosition {
        set: usize,
        entry: usize,
    },
    CycleShape {
        set: usize,
        entry: usize,
    },
    // ── wheel editing ────────────────────────────────────────────────────────
    EditWheelName,
    CaptureWheelHoldKey,
    CycleWheelTheme,
    WheelCooldownDelta {
        delta: f32,
    },
    WheelOuterRadiusDelta {
        delta: f32,
    },
    WheelInnerRadiusDelta {
        delta: f32,
    },
    WheelAnimSpeedDelta {
        delta: f32,
    },
    ToggleWheelShowLabels,
    ToggleWheelShowInfoInHub,
    AddSlot,
    RemoveSlot,
    EditSlotName {
        slot: usize,
    },
    // ── wheel-set entry editing ──────────────────────────────────────────────
    EditWheelSetName {
        set: usize,
        entry: usize,
    },
    CaptureWheelSetSwitchKey {
        set: usize,
        entry: usize,
    },
    // ── set-switch shortcuts ─────────────────────────────────────────────────
    CaptureNextSetKey,
    CapturePrevSetKey,
    // ── persistence ──────────────────────────────────────────────────────────
    Save,
    Load,
    // ── segment editing ──────────────────────────────────────────────────────
    SelectSegment {
        set: usize,
        entry: usize,
        wheel: Option<usize>,
        slot: usize,
    },
    EditSlotIcon {
        slot: usize,
    },
    CycleSegmentShape,
    ToggleWheelShowIcon,
    CycleHighlightColor,
    SegmentScaleDelta {
        delta: f32,
    },
    // ── segment input / gamepad binding ─────────────────────────────────────────
    /// Capture a key or gamepad button as the input binding for segment `slot`.
    CaptureSlotInput {
        slot: usize,
    },
    /// Capture a gamepad button for a quick action.
    CaptureGamepadButton {
        set: usize,
        entry: usize,
    },
    // ── per-slot items ───────────────────────────────────────────────────────────
    AddSlotItem {
        slot: usize,
    },
    RemoveSlotItem {
        slot: usize,
        item: usize,
    },
    EditSlotItemName {
        slot: usize,
        item: usize,
    },
    EditSlotItemIcon {
        slot: usize,
        item: usize,
    },
}

// ─── plugin ──────────────────────────────────────────────────────────────────────

pub struct QuickActionEditorPlugin;
impl Plugin for QuickActionEditorPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<QuickActionConfig>()
            .init_resource::<EditorUiState>()
            .add_systems(
                Update,
                (
                    handle_editor_buttons,
                    editor_capture_key,
                    editor_capture_gamepad,
                    editor_text_input,
                    editor_button_feedback,
                    rebuild_editor,
                )
                    .chain(),
            );
    }
}

// ─── palette ─────────────────────────────────────────────────────────────────────

const BG_SIDEBAR: Color = Color::srgb(0.043, 0.055, 0.075);
const BG_MAIN: Color = Color::srgb(0.055, 0.067, 0.086);
const SIDEBAR_BORDER: Color = Color::srgb(0.10, 0.12, 0.15);
const GREEN: Color = Color::srgb(0.30, 0.74, 0.40);
const GREEN_BG: Color = Color::srgba(0.30, 0.74, 0.40, 0.14);
const TEXT: Color = Color::srgb(0.74, 0.79, 0.85);
const DIM: Color = Color::srgb(0.42, 0.47, 0.54);
const DIMMER: Color = Color::srgb(0.30, 0.34, 0.40);
const ICON: Color = Color::srgb(0.45, 0.53, 0.61);
const AMBER: Color = Color::srgb(0.82, 0.66, 0.25);
const BLUE: Color = Color::srgb(0.38, 0.62, 0.95);
const TEAL: Color = Color::srgb(0.52, 0.69, 0.75);
const BADGE_BORDER: Color = Color::srgb(0.26, 0.30, 0.36);
const ROW_SEL: Color = Color::srgba(0.38, 0.62, 0.95, 0.16);
const ROW_HOVER: Color = Color::srgba(1.0, 1.0, 1.0, 0.05);
const PANEL_CARD: Color = Color::srgb(0.08, 0.10, 0.15);
const CTRL_BG: Color = Color::srgb(0.11, 0.14, 0.19);

// ─── primitive bsn! helpers ──────────────────────────────────────────────────────

fn text(s: &str, size: f32, color: Color) -> impl Scene {
    bsn! {
        Text({s.to_string()})
        TextFont { font_size: {FontSize::Px(size)} }
        TextColor({color})
    }
}

fn hcluster() -> impl Scene {
    bsn! {
        Node {
            flex_direction: FlexDirection::Row,
            align_items: AlignItems::Center,
            column_gap: {px(7.)},
        }
    }
}

fn row_button(bg: Color) -> impl Scene {
    bsn! {
        Node {
            width: {percent(100.)}, height: {px(24.)},
            flex_direction: FlexDirection::Row,
            align_items: AlignItems::Center,
            justify_content: JustifyContent::SpaceBetween,
            padding: {UiRect::horizontal(px(4.))},
            border_radius: {BorderRadius::all(px(4.))},
        }
        BackgroundColor({bg})
        Button
    }
}

fn key_badge_box() -> impl Scene {
    bsn! {
        Node {
            width: {px(18.)}, height: {px(16.)},
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            border: {UiRect::all(px(1.))},
            border_radius: {BorderRadius::all(px(3.))},
        }
        BorderColor::all(BADGE_BORDER)
    }
}

fn add_button(icon: &str, icon_color: Color, label: &str, accent: Color) -> impl Scene {
    bsn! {
        Node {
            width: {percent(100.)}, height: {px(26.)},
            margin: {UiRect::top(px(4.))},
            flex_direction: FlexDirection::Row,
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            column_gap: {px(6.)},
            border: {UiRect::all(px(1.))},
            border_radius: {BorderRadius::all(px(4.))},
        }
        BorderColor::all(accent)
        BackgroundColor({Color::srgba(1., 1., 1., 0.015)})
        Button
        Children [
            text("+", 12., accent),
            text(icon, 10., icon_color),
            text(label, 10., accent),
        ]
    }
}

fn set_header_row(bg: Color) -> impl Scene {
    bsn! {
        Node {
            width: {percent(100.)}, height: {px(26.)},
            margin: {UiRect::top(px(4.))},
            flex_direction: FlexDirection::Row,
            align_items: AlignItems::Center,
            justify_content: JustifyContent::SpaceBetween,
            padding: {UiRect::horizontal(px(4.))},
            border_radius: {BorderRadius::all(px(4.))},
        }
        BackgroundColor({bg})
        Button
    }
}

#[allow(dead_code)]
fn col() -> impl Scene {
    bsn! { Node { flex_direction: FlexDirection::Column, row_gap: {px(1.)} } }
}

fn indent_col() -> impl Scene {
    bsn! {
        Node {
            flex_direction: FlexDirection::Column,
            padding: {UiRect::left(px(14.))},
            row_gap: {px(1.)},
        }
    }
}

fn sidebar() -> impl Scene {
    bsn! {
        Node {
            width: {px(260.)},
            height: {percent(100.)},
            flex_direction: FlexDirection::Column,
            border: {UiRect::right(px(1.))},
        }
        BackgroundColor({BG_SIDEBAR})
        BorderColor::all(SIDEBAR_BORDER)
    }
}

fn tree() -> impl Scene {
    bsn! {
        Node {
            flex_grow: 1.,
            flex_direction: FlexDirection::Column,
            padding: {UiRect::axes(px(12.), px(8.))},
            row_gap: {px(2.)},
            overflow: {Overflow::scroll_y()},
        }
    }
}

fn del_btn() -> impl Scene {
    bsn! {
        Node {
            width: {px(16.)}, height: {px(16.)},
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
        }
        Button
    }
}

fn footer_button(label: &str, accent: Color, filled: bool) -> impl Scene {
    let (bg, border) = if filled {
        (GREEN_BG, Color::NONE)
    } else {
        (Color::NONE, BADGE_BORDER)
    };
    bsn! {
        Node {
            padding: {UiRect::axes(px(16.), px(6.))},
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            border: {UiRect::all(px(1.))},
            border_radius: {BorderRadius::all(px(5.))},
        }
        BorderColor::all(border)
        BackgroundColor({bg})
        Button
        Children [ text(label, 11., accent) ]
    }
}

// ─── entity helpers ──────────────────────────────────────────────────────────────

fn child(commands: &mut Commands, parent: Entity, scene: impl Scene) -> Entity {
    let e = commands.spawn_scene(scene).id();
    commands.entity(parent).add_child(e);
    e
}

fn clickable(
    commands: &mut Commands,
    parent: Entity,
    scene: impl Scene,
    action: EditorAction,
    base: Color,
) -> Entity {
    let e = commands
        .spawn_scene(scene)
        .insert(EditorButton { action, base })
        .id();
    commands.entity(parent).add_child(e);
    e
}

#[derive(Clone)]
#[allow(dead_code)]
enum Badge {
    None,
    Key(String),
    Dim(String),
}

fn spawn_entry_row(
    commands: &mut Commands,
    parent: Entity,
    selected: bool,
    select_action: EditorAction,
    icon: &str,
    icon_col: Color,
    name: &str,
    name_col: Color,
    badge: Badge,
    del: Option<EditorAction>,
) {
    let bg = if selected { ROW_SEL } else { Color::NONE };
    let row = clickable(commands, parent, row_button(bg), select_action, bg);
    let left = child(commands, row, hcluster());
    child(commands, left, text(icon, 10., icon_col));
    child(commands, left, text(name, 11., name_col));
    let right = child(commands, row, hcluster());
    match &badge {
        Badge::Key(k) => {
            let kb = child(commands, right, key_badge_box());
            child(commands, kb, text(k, 8., DIM));
        }
        Badge::Dim(s) => {
            child(commands, right, text(s, 9., DIMMER));
        }
        Badge::None => {}
    }
    if let Some(da) = del {
        let dx = clickable(commands, right, del_btn(), da, Color::NONE);
        child(commands, dx, text("×", 10., DIMMER));
    }
}

// ─── rebuild ─────────────────────────────────────────────────────────────────────

fn rebuild_editor(
    mut commands: Commands,
    mut ui: ResMut<EditorUiState>,
    cfg: Res<QuickActionConfig>,
    windows: Query<&Window>,
    old_sidebar: Query<Entity, With<EditorRoot>>,
    old_canvas: Query<Entity, With<EditorCanvasRoot>>,
    old_meshes: Query<Entity, With<WheelMeshPreview>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    if !ui.dirty {
        return;
    }
    ui.dirty = false;

    for e in &old_sidebar {
        commands.entity(e).despawn();
    }
    for e in &old_canvas {
        commands.entity(e).despawn();
    }
    for e in &old_meshes {
        commands.entity(e).despawn();
    }

    if !cfg.sets.is_empty() {
        ui.active_set = ui.active_set.min(cfg.sets.len() - 1);
    }

    build_sidebar(&mut commands, &cfg, &ui);
    let win = windows.iter().next();
    let win_w = win.map(|w| w.width()).unwrap_or(1229.0);
    let win_h = win.map(|w| w.height()).unwrap_or(768.0);
    build_hud_canvas(
        &mut commands,
        &cfg,
        &ui,
        win_w,
        win_h,
        &mut meshes,
        &mut materials,
    );
}

// ─── sidebar ─────────────────────────────────────────────────────────────────────

fn build_sidebar(commands: &mut Commands, cfg: &QuickActionConfig, ui: &EditorUiState) {
    let root = commands.spawn_scene(sidebar()).insert(EditorRoot).id();

    match ui.selection {
        // Navigation view ─────────────────────────────────────────────────────
        Selection::None | Selection::Set { .. } => {
            build_nav_sidebar(commands, root, cfg, ui);
        }

        // Button / quick-action editor ────────────────────────────────────────
        Selection::Action { set, entry } => {
            let qa = cfg
                .sets
                .get(set)
                .and_then(|s| s.entries.get(entry))
                .and_then(|e| {
                    if let SetEntry::Action(a) = e {
                        Some(a)
                    } else {
                        None
                    }
                });
            if let Some(qa) = qa {
                build_editor_header(
                    commands,
                    root,
                    Option::<&str>::None,
                    &qa.name.clone(),
                    EditorAction::NavBack,
                );
                let scroll = child(commands, root, tree());
                spawn_action_editor(commands, scroll, ui, set, entry, qa);
                build_footer(commands, root, &ui.config_path);
            } else {
                build_nav_sidebar(commands, root, cfg, ui);
            }
        }

        // Wheel editor ────────────────────────────────────────────────────────
        Selection::Wheel { set, entry, wheel } => {
            let w_ref = cfg
                .sets
                .get(set)
                .and_then(|s| s.entries.get(entry))
                .and_then(|e| match (e, wheel) {
                    (SetEntry::Wheel(w), None) => Some(w as &Wheel),
                    (SetEntry::WheelSet(ws), Some(i)) => ws.wheels.get(i).map(|w| w as &Wheel),
                    _ => None,
                });
            if let Some(w) = w_ref {
                let parent_name: Option<String> = wheel.map(|_| {
                    cfg.sets
                        .get(set)
                        .and_then(|s| s.entries.get(entry))
                        .and_then(|e| {
                            if let SetEntry::WheelSet(ws) = e {
                                Some(ws.name.as_str())
                            } else {
                                None
                            }
                        })
                        .unwrap_or("Wheel Set")
                        .to_string()
                });
                let wname = w.name.clone();
                build_editor_header(
                    commands,
                    root,
                    parent_name.as_deref(),
                    &wname,
                    EditorAction::NavBack,
                );
                let scroll = child(commands, root, tree());
                spawn_wheel_editor(commands, scroll, ui, w, set, entry, wheel);
                build_footer(commands, root, &ui.config_path);
            } else {
                build_nav_sidebar(commands, root, cfg, ui);
            }
        }

        // Wheel-set entry editor ──────────────────────────────────────────────
        Selection::WheelSetEntry { set, entry } => {
            let ws = cfg
                .sets
                .get(set)
                .and_then(|s| s.entries.get(entry))
                .and_then(|e| {
                    if let SetEntry::WheelSet(ws) = e {
                        Some(ws)
                    } else {
                        None
                    }
                });
            if let Some(ws) = ws {
                let wname = ws.name.clone();
                build_editor_header(
                    commands,
                    root,
                    Option::<&str>::None,
                    &wname,
                    EditorAction::NavBack,
                );
                let scroll = child(commands, root, tree());
                spawn_wheelset_entry_editor(commands, scroll, ui, set, entry, ws);
                build_footer(commands, root, &ui.config_path);
            } else {
                build_nav_sidebar(commands, root, cfg, ui);
            }
        }

        // Set-switch editor ───────────────────────────────────────────────────
        Selection::SetSwitch => {
            build_editor_header(
                commands,
                root,
                Option::<&str>::None,
                "Set Switching",
                EditorAction::NavBack,
            );
            let scroll = child(commands, root, tree());
            let card = child(commands, scroll, editor_card());
            let nf = ui.editing == EditFocus::NextSetKey;
            let (nd, nc) = key_display(nf, &cfg.next_set_key);
            spawn_box_field(
                commands,
                card,
                "Next set",
                &nd,
                nc,
                if nf { AMBER } else { BADGE_BORDER },
                EditorAction::CaptureNextSetKey,
            );
            let pf = ui.editing == EditFocus::PrevSetKey;
            let (pd, pc) = key_display(pf, &cfg.prev_set_key);
            spawn_box_field(
                commands,
                card,
                "Prev set",
                &pd,
                pc,
                if pf { AMBER } else { BADGE_BORDER },
                EditorAction::CapturePrevSetKey,
            );
            build_footer(commands, root, &ui.config_path);
        }

        // Segment editor ──────────────────────────────────────────────────────
        Selection::Segment {
            set,
            entry,
            wheel,
            slot,
        } => {
            let w_ref = cfg
                .sets
                .get(set)
                .and_then(|s| s.entries.get(entry))
                .and_then(|e| match (e, wheel) {
                    (SetEntry::Wheel(w), None) => Some(w as &Wheel),
                    (SetEntry::WheelSet(ws), Some(i)) => ws.wheels.get(i).map(|w| w as &Wheel),
                    _ => None,
                });
            if let Some(w) = w_ref {
                let slot_name = w
                    .slots
                    .get(slot)
                    .map(|s| s.name.clone())
                    .unwrap_or_default();
                let wname = w.name.clone();
                build_editor_header(
                    commands,
                    root,
                    Some(wname.as_str()),
                    &slot_name,
                    EditorAction::NavBack,
                );
                let scroll = child(commands, root, tree());
                spawn_segment_editor(commands, scroll, ui, slot, w);
                build_footer(commands, root, &ui.config_path);
            } else {
                build_nav_sidebar(commands, root, cfg, ui);
            }
        }
    }
}

/// Breadcrumb header for editor panels.  `‹ [parent |] name`
fn build_editor_header(
    commands: &mut Commands,
    parent: Entity,
    parent_name: Option<&str>,
    item_name: &str,
    back_action: EditorAction,
) {
    let header = commands
        .spawn_scene(bsn! {
            Node {
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                padding: {UiRect::all(px(12.))},
                column_gap: {px(6.)},
                border: {UiRect::bottom(px(1.))},
            }
            BorderColor::all(SIDEBAR_BORDER)
        })
        .id();
    commands.entity(parent).add_child(header);

    let back = clickable(
        commands,
        header,
        bsn! {
            Node {
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                column_gap: {px(3.)},
                padding: {UiRect::axes(px(5.), px(3.))},
                border_radius: {BorderRadius::all(px(4.))},
            }
            Button
        },
        back_action,
        Color::NONE,
    );
    child(commands, back, text("‹", 15., DIM));

    if let Some(pn) = parent_name {
        child(commands, header, text(pn, 11., DIM));
        child(commands, header, text("|", 11., DIMMER));
    }
    child(commands, header, text(item_name, 11., TEXT));
}

/// Navigation sidebar: wheel-set tree + button list for the active set.
fn build_nav_sidebar(
    commands: &mut Commands,
    root: Entity,
    cfg: &QuickActionConfig,
    ui: &EditorUiState,
) {
    // Header: set name as title.
    let set_name = cfg
        .sets
        .get(ui.active_set)
        .map(|s| s.name.as_str())
        .unwrap_or("—");
    let header = commands
        .spawn_scene(bsn! {
            Node {
                flex_direction: FlexDirection::Row,
                justify_content: JustifyContent::SpaceBetween,
                align_items: AlignItems::Center,
                padding: {UiRect::all(px(14.))},
                border: {UiRect::bottom(px(1.))},
            }
            BorderColor::all(SIDEBAR_BORDER)
        })
        .id();
    commands.entity(root).add_child(header);
    child(commands, header, text(set_name, 13., TEXT));
    clickable(
        commands,
        header,
        bsn! {
            Node { padding: {UiRect::all(px(2.))} }
            Button
            Children [ text("⚙", 11., DIM) ]
        },
        EditorAction::SelectSet { set: ui.active_set },
        Color::NONE,
    );

    // Scrollable body.
    let scroll = child(commands, root, tree());

    if let Some(set) = cfg.sets.get(ui.active_set) {
        build_nav_wheel_section(commands, scroll, ui, set, ui.active_set);
        build_nav_button_section(commands, scroll, ui, set, ui.active_set);
    } else {
        child(
            commands,
            scroll,
            text("No sets yet — use Save/Load or add one.", 10., DIMMER),
        );
    }

    build_set_switch_summary(commands, root, cfg);
    build_footer(commands, root, &ui.config_path);
}

/// "~ WHEEL SET" navigation section.
fn build_nav_wheel_section(
    commands: &mut Commands,
    parent: Entity,
    ui: &EditorUiState,
    set: &ActionSet,
    si: usize,
) {
    // Section header.
    let sec = child(
        commands,
        parent,
        bsn! {
            Node {
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::SpaceBetween,
                padding: {UiRect::new(px(2.), px(0.), px(8.), px(2.))},
            }
        },
    );
    let hl = child(commands, sec, hcluster());
    child(commands, hl, text("~", 9., TEAL));
    child(commands, hl, text("WHEEL SET", 10., DIM));
    clickable(
        commands,
        sec,
        bsn! {
            Node {
                padding: {UiRect::axes(px(6.), px(2.))},
                border: {UiRect::all(px(1.))},
                border_radius: {BorderRadius::all(px(3.))},
            }
            BorderColor::all(BLUE)
            Button
            Children [ text("+", 10., BLUE) ]
        },
        EditorAction::AddWheelSet { set: si },
        Color::NONE,
    );

    let body = child(commands, parent, indent_col());
    let mut has_any = false;

    for (ei, entry) in set.entries.iter().enumerate() {
        match entry {
            SetEntry::Wheel(w) => {
                has_any = true;
                let sel = ui.selection
                    == (Selection::Wheel {
                        set: si,
                        entry: ei,
                        wheel: None,
                    });
                let badge = if w.hold_key.is_empty() {
                    Badge::None
                } else {
                    Badge::Key(w.hold_key.clone())
                };
                spawn_entry_row(
                    commands,
                    body,
                    sel,
                    EditorAction::SelectWheel {
                        set: si,
                        entry: ei,
                        wheel: None,
                    },
                    "○",
                    ICON,
                    &w.name,
                    TEXT,
                    badge,
                    Some(EditorAction::DeleteEntry { set: si, entry: ei }),
                );
            }
            SetEntry::WheelSet(ws) => {
                has_any = true;
                let ws_sel = ui.selection == (Selection::WheelSetEntry { set: si, entry: ei });
                let ws_bg = if ws_sel { ROW_SEL } else { Color::NONE };
                let wsh = clickable(
                    commands,
                    body,
                    set_header_row(ws_bg),
                    EditorAction::SelectWheelSetEntry { set: si, entry: ei },
                    ws_bg,
                );
                let whl = child(commands, wsh, hcluster());
                child(commands, whl, text("⊞", 10., BLUE));
                child(commands, whl, text(&ws.name, 11., TEXT));
                let whr = child(commands, wsh, hcluster());
                child(
                    commands,
                    whr,
                    text(&format!("{}w", ws.wheels.len()), 9., DIM),
                );
                let dx = clickable(
                    commands,
                    whr,
                    del_btn(),
                    EditorAction::DeleteEntry { set: si, entry: ei },
                    Color::NONE,
                );
                child(commands, dx, text("×", 10., DIMMER));

                let wsb = child(commands, body, indent_col());
                for (wi, w) in ws.wheels.iter().enumerate() {
                    let wsel = ui.selection
                        == (Selection::Wheel {
                            set: si,
                            entry: ei,
                            wheel: Some(wi),
                        });
                    let badge = if w.hold_key.is_empty() {
                        Badge::None
                    } else {
                        Badge::Key(w.hold_key.clone())
                    };
                    spawn_entry_row(
                        commands,
                        wsb,
                        wsel,
                        EditorAction::SelectWheel {
                            set: si,
                            entry: ei,
                            wheel: Some(wi),
                        },
                        "○",
                        ICON,
                        &w.name,
                        TEAL,
                        badge,
                        Some(EditorAction::DeleteWheelFromSet {
                            set: si,
                            entry: ei,
                            wheel: wi,
                        }),
                    );
                }
                let link = clickable(
                    commands,
                    wsb,
                    row_button(Color::NONE),
                    EditorAction::AddWheelToSet { set: si, entry: ei },
                    Color::NONE,
                );
                child(commands, link, text("+ add wheel", 10., DIM));
            }
            _ => {}
        }
    }
    if !has_any {
        child(commands, body, text("No wheels yet.", 10., DIMMER));
    }

    // Add standalone wheel.
    clickable(
        commands,
        body,
        add_button("○", BLUE, "Add Wheel", BLUE),
        EditorAction::AddWheel { set: si },
        Color::srgba(1., 1., 1., 0.015),
    );
}

/// "~ BUTTONS" navigation section.
fn build_nav_button_section(
    commands: &mut Commands,
    parent: Entity,
    ui: &EditorUiState,
    set: &ActionSet,
    si: usize,
) {
    // Section header.
    let sec = child(
        commands,
        parent,
        bsn! {
            Node {
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::SpaceBetween,
                padding: {UiRect::new(px(2.), px(0.), px(8.), px(10.))},
            }
        },
    );
    let hl = child(commands, sec, hcluster());
    child(commands, hl, text("~", 9., AMBER));
    child(commands, hl, text("BUTTONS", 10., DIM));
    clickable(
        commands,
        sec,
        bsn! {
            Node {
                padding: {UiRect::axes(px(6.), px(2.))},
                border: {UiRect::all(px(1.))},
                border_radius: {BorderRadius::all(px(3.))},
            }
            BorderColor::all(AMBER)
            Button
            Children [ text("+", 10., AMBER) ]
        },
        EditorAction::AddAction { set: si },
        Color::NONE,
    );

    let body = child(commands, parent, indent_col());
    let mut has_any = false;

    for (ei, entry) in set.entries.iter().enumerate() {
        if let SetEntry::Action(qa) = entry {
            has_any = true;
            let sel = ui.selection == (Selection::Action { set: si, entry: ei });
            let badge = if qa.key.is_empty() {
                Badge::None
            } else {
                Badge::Key(qa.key.clone())
            };
            spawn_entry_row(
                commands,
                body,
                sel,
                EditorAction::SelectAction { set: si, entry: ei },
                "□",
                ICON,
                &qa.name,
                TEXT,
                badge,
                Some(EditorAction::DeleteEntry { set: si, entry: ei }),
            );
        }
    }
    if !has_any {
        child(commands, body, text("No buttons yet.", 10., DIMMER));
    }
}

/// Compact "SET SWITCH KEYS" bar above the footer.
fn build_set_switch_summary(commands: &mut Commands, parent: Entity, cfg: &QuickActionConfig) {
    let area = commands
        .spawn_scene(bsn! {
            Node {
                flex_direction: FlexDirection::Column,
                padding: {UiRect::axes(px(12.), px(8.))},
                row_gap: {px(6.)},
                border: {UiRect::top(px(1.))},
            }
            BorderColor::all(SIDEBAR_BORDER)
        })
        .id();
    commands.entity(parent).add_child(area);

    let title_row = child(commands, area, hcluster());
    child(commands, title_row, text("~", 9., TEAL));
    child(commands, title_row, text("SET SWITCH KEYS", 9., DIM));

    let keys_row = child(
        commands,
        area,
        bsn! {
            Node {
                flex_direction: FlexDirection::Row,
                justify_content: JustifyContent::SpaceBetween,
            }
        },
    );

    let prev = clickable(
        commands,
        keys_row,
        bsn! {
            Node {
                flex_direction: FlexDirection::Row, align_items: AlignItems::Center,
                column_gap: {px(4.)},
                padding: {UiRect::axes(px(6.), px(3.))},
                border: {UiRect::all(px(1.))},
                border_radius: {BorderRadius::all(px(4.))},
            }
            BorderColor::all(BADGE_BORDER)
            Button
        },
        EditorAction::SelectSetSwitch,
        Color::NONE,
    );
    child(commands, prev, text("‹", 11., DIM));
    child(commands, prev, text("PREV SET", 9., DIM));
    let pk = child(commands, prev, key_badge_box());
    child(commands, pk, text(&label_or(&cfg.prev_set_key), 8., TEAL));

    let next = clickable(
        commands,
        keys_row,
        bsn! {
            Node {
                flex_direction: FlexDirection::Row, align_items: AlignItems::Center,
                column_gap: {px(4.)},
                padding: {UiRect::axes(px(6.), px(3.))},
                border: {UiRect::all(px(1.))},
                border_radius: {BorderRadius::all(px(4.))},
            }
            BorderColor::all(BADGE_BORDER)
            Button
        },
        EditorAction::SelectSetSwitch,
        Color::NONE,
    );
    child(commands, next, text("NEXT SET", 9., DIM));
    let nk = child(commands, next, key_badge_box());
    child(commands, nk, text(&label_or(&cfg.next_set_key), 8., TEAL));
    child(commands, next, text("›", 11., DIM));
}

/// Save / Load footer.
fn build_footer(commands: &mut Commands, parent: Entity, path: &str) {
    let footer = commands
        .spawn_scene(bsn! {
            Node {
                flex_direction: FlexDirection::Column,
                row_gap: {px(8.)},
                padding: {UiRect::all(px(12.))},
                border: {UiRect::top(px(1.))},
            }
            BorderColor::all(SIDEBAR_BORDER)
        })
        .id();
    commands.entity(parent).add_child(footer);
    let row = child(
        commands,
        footer,
        bsn! {
            Node { flex_direction: FlexDirection::Row, column_gap: {px(8.)} }
        },
    );
    clickable(
        commands,
        row,
        footer_button("SAVE", GREEN, true),
        EditorAction::Save,
        GREEN_BG,
    );
    clickable(
        commands,
        row,
        footer_button("LOAD", DIM, false),
        EditorAction::Load,
        Color::NONE,
    );
    let cap = child(
        commands,
        footer,
        bsn! { Node { justify_content: JustifyContent::Center } },
    );
    child(commands, cap, text(path, 9., DIMMER));
}

// ─── HUD canvas ──────────────────────────────────────────────────────────────────

fn canvas_root() -> impl Scene {
    bsn! {
        Node {
            position_type: PositionType::Absolute,
            left: {px(260.)}, top: {px(0.)}, right: {px(0.)}, bottom: {px(0.)},
            flex_direction: FlexDirection::Column,
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
        }
        BackgroundColor({BG_MAIN})
    }
}

fn label_or(key: &str) -> String {
    if key.is_empty() {
        "—".into()
    } else {
        key.into()
    }
}

fn with_alpha(c: Color, a: f32) -> Color {
    c.with_alpha(a)
}

/// Parse a `#rrggbb` hex string into a Bevy Color (falls back to a blue tint).
fn parse_hex_color(hex: &str, alpha: f32) -> Color {
    let s = hex.trim_start_matches('#');
    if s.len() == 6 {
        if let (Ok(r), Ok(g), Ok(b)) = (
            u8::from_str_radix(&s[0..2], 16),
            u8::from_str_radix(&s[2..4], 16),
            u8::from_str_radix(&s[4..6], 16),
        ) {
            return Color::srgba(r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0, alpha);
        }
    }
    Color::srgba(0.23, 0.51, 0.96, alpha)
}

/// Build the HUD preview canvas (always visible, regardless of sidebar state).
fn build_hud_canvas(
    commands: &mut Commands,
    cfg: &QuickActionConfig,
    ui: &EditorUiState,
    win_w: f32,
    win_h: f32,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<ColorMaterial>,
) {
    let root = commands
        .spawn_scene(canvas_root())
        .insert(EditorCanvasRoot)
        .id();

    // "HUD PREVIEW" watermark.
    let watermark = commands
        .spawn_scene(bsn! {
            Node {
                position_type: PositionType::Absolute,
                top: {px(16.)}, left: {px(16.)},
            }
        })
        .id();
    commands.entity(root).add_child(watermark);
    child(commands, watermark, text("HUD PREVIEW", 10., DIMMER));

    // Set tabs at bottom.
    build_set_tabs(commands, root, cfg, ui);

    if cfg.sets.is_empty() {
        child(
            commands,
            root,
            text("No sets — add one in the sidebar.", 12., DIMMER),
        );
        return;
    }

    // World-space center of the canvas area (used by Pie-shape Mesh2d).
    // sidebar_w = 260; canvas spans [260, win_w].
    let sidebar_w = 260.0_f32;
    let wheel_cx = sidebar_w + (win_w - sidebar_w) / 2.0 - win_w / 2.0;
    let wheel_cy = 0.0_f32; // canvas is vertically centred
    let _ = win_h; // reserved for future use

    if let Some(set) = cfg.sets.get(ui.active_set) {
        // Render the first wheel or wheel-set found in the active set.
        let mut rendered = false;
        for (ei, entry) in set.entries.iter().enumerate() {
            match entry {
                SetEntry::Wheel(w) => {
                    build_centered_wheel(
                        commands,
                        root,
                        w,
                        ui.active_set,
                        ei,
                        None,
                        ui.selection,
                        wheel_cx,
                        wheel_cy,
                        meshes,
                        materials,
                    );
                    rendered = true;
                    break;
                }
                SetEntry::WheelSet(ws) => {
                    if let Some(w) = ws.wheels.first() {
                        build_centered_wheel(
                            commands,
                            root,
                            w,
                            ui.active_set,
                            ei,
                            Some(0),
                            ui.selection,
                            wheel_cx,
                            wheel_cy,
                            meshes,
                            materials,
                        );
                        rendered = true;
                    }
                    break;
                }
                _ => {}
            }
        }
        if !rendered {
            child(commands, root, text("No wheels in this set.", 11., DIMMER));
        }

        // Floating action buttons.
        build_hud_buttons(commands, root, set);
    }
}

/// Renders a wheel centred in the canvas.
/// - Rounded / Square / Circle / Wedge: BSN UI nodes (no interaction).
/// - Pie: real `Mesh2d` wedge per slice, tagged `WheelMeshPreview`.
/// Selected segment (from `Selection::Segment`) is highlighted but not interactive.
#[allow(clippy::too_many_arguments)]
fn build_centered_wheel(
    commands: &mut Commands,
    parent: Entity,
    wheel: &Wheel,
    set: usize,
    entry: usize,
    w_idx: Option<usize>,
    selection: Selection,
    wheel_cx: f32,
    wheel_cy: f32,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<ColorMaterial>,
) {
    let menu = wheel.to_menu();
    let hub = child(commands, parent, wheel_hub());

    let is_pie = wheel.segment_shape == SegmentShape::Pie;

    // Background disc and outer ring (skipped for Pie — mesh fills the area).
    if !is_pie {
        child(commands, hub, wheel_bg_disc(menu.radius));
    }
    child(commands, hub, wheel_outer_ring(menu.radius));

    // ── shared slice metrics ───────────────────────────────────────────
    let slice_angle = std::f32::consts::TAU / menu.slices.max(1) as f32;
    let base_panel_w = (2.0 * menu.radius * (slice_angle / 2.0).sin() * 0.72).max(48.0);
    let base_panel_h = ((menu.radius - menu.inner_radius) * 0.85).max(40.0);
    let panel_w = (base_panel_w * wheel.segment_scale).max(32.0);
    let panel_h = (base_panel_h * wheel.segment_scale).max(24.0);
    let min_dim = panel_w.min(panel_h);
    let highlight_col = parse_hex_color(&wheel.highlight_color, 1.0);
    let slice_bg = Color::srgb(0.13, 0.17, 0.23);
    let label_c = Color::srgb(0.84, 0.89, 0.94);
    let label_sz = (panel_h * 0.18).clamp(9.0, 13.0);

    for (i, slot) in wheel.slots.iter().enumerate() {
        if i >= menu.slices {
            break;
        }

        let slot_name = &slot.name;
        let icon = &slot.icon;
        let is_selected = matches!(selection,
            Selection::Segment { set: s, entry: e, wheel: ww, slot: sl }
            if s == set && e == entry && ww == w_idx && sl == i);
        let seg_color = if is_selected { highlight_col } else { slice_bg };

        if is_pie {
            // ── true Mesh2d wedge ──────────────────────────────────────────
            let (a0, a1) = slice_angles(&menu, i);
            let mesh = meshes.add(crate::mesh::wedge(menu.inner_radius, menu.radius, a0, a1));
            let mat = materials.add(ColorMaterial::from_color(seg_color));
            commands.spawn((
                Mesh2d(mesh),
                MeshMaterial2d(mat),
                Transform::from_xyz(wheel_cx, wheel_cy, 0.5),
                WheelMeshPreview,
            ));

            // Transparent UI panel just for label layout.
            let ctr = slice_center(&menu, i);
            let left = ctr.x - panel_w / 2.0;
            let top_pos = -ctr.y - panel_h / 2.0;
            let panel_e = commands
                .spawn_scene(bsn! {
                    Node {
                        position_type: PositionType::Absolute,
                        left: {px(left)}, top: {px(top_pos)},
                        width: {px(panel_w)}, height: {px(panel_h)},
                        justify_content: JustifyContent::SpaceBetween,
                        align_items: AlignItems::Center,
                        flex_direction: FlexDirection::Column,
                        padding: {UiRect::all(px(6.))},
                    }
                    // Transparent — mesh provides the visual background.
                    BackgroundColor({Color::NONE})
                })
                .id();
            commands.entity(hub).add_child(panel_e);

            if wheel.show_labels {
                child(
                    commands,
                    panel_e,
                    wheel_slice_label(slot_name.to_uppercase(), label_sz, label_c),
                );
            }
            if wheel.show_icon && !icon.is_empty() {
                child(
                    commands,
                    panel_e,
                    wheel_slice_label(icon.clone(), label_sz * 1.3, label_c),
                );
            } else if wheel.show_labels {
                child(
                    commands,
                    panel_e,
                    bsn! { Node { width: {px(4.)}, height: {px(4.)} } },
                );
            }
        } else {
            // ── BSN UI panel ────────────────────────────────────────────────────
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

            let ctr = slice_center(&menu, i);
            let left = ctr.x - panel_w / 2.0;
            let top_pos = -ctr.y - panel_h / 2.0;

            // Panel is purely visual — no Button / EditorButton.
            let panel_e = commands
                .spawn_scene(bsn! {
                    Node {
                        position_type: PositionType::Absolute,
                        left: {px(left)}, top: {px(top_pos)},
                        width: {px(panel_w)}, height: {px(panel_h)},
                        justify_content: JustifyContent::SpaceBetween,
                        align_items: AlignItems::Center,
                        flex_direction: FlexDirection::Column,
                        padding: {UiRect::all(px(6.))},
                        border_radius: {seg_br},
                    }
                    BackgroundColor({seg_color})
                })
                .id();
            commands.entity(hub).add_child(panel_e);

            if wheel.show_labels {
                child(
                    commands,
                    panel_e,
                    wheel_slice_label(slot_name.to_uppercase(), label_sz, label_c),
                );
            }
            if wheel.show_icon && !icon.is_empty() {
                child(
                    commands,
                    panel_e,
                    wheel_slice_label(icon.clone(), label_sz * 1.3, label_c),
                );
            } else if wheel.show_labels {
                child(
                    commands,
                    panel_e,
                    bsn! { Node { width: {px(4.)}, height: {px(4.)} } },
                );
            }
        }
    }

    // ── centre hub with golden ring ───────────────────────────────────
    let disc_r = (menu.inner_radius - 4.0).max(8.0);
    let ring_col = Color::srgb(0.82, 0.64, 0.16);
    let hub_bg = Color::srgb(0.08, 0.10, 0.14);
    let center = child(
        commands,
        hub,
        wheel_center_ring(disc_r, hub_bg, ring_col, 3.0),
    );
    let hub_label = if !wheel.hold_key.is_empty() {
        wheel.hold_key.clone()
    } else {
        wheel
            .name
            .chars()
            .next()
            .map(|c| c.to_string())
            .unwrap_or_default()
    };
    child(
        commands,
        center,
        wheel_slice_label(hub_label, (disc_r * 0.60).max(9.0), ring_col),
    );
}

/// Floating action buttons overlaid in the bottom-right of the canvas.
fn build_hud_buttons(commands: &mut Commands, parent: Entity, set: &ActionSet) {
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
                bottom: {px(60.)}, right: {px(36.)},
                flex_direction: FlexDirection::Column,
                row_gap: {px(8.)},
                align_items: AlignItems::FlexEnd,
            }
        })
        .id();
    commands.entity(parent).add_child(container);

    for qa in btns.iter().rev() {
        spawn_hud_button(commands, container, set.opacity, qa);
    }
}

/// One floating HUD button (a quick action rendered in the canvas).
fn spawn_hud_button(commands: &mut Commands, parent: Entity, set_opacity: f32, qa: &QuickAction) {
    let eff = (set_opacity * qa.opacity).clamp(0.05, 1.0);
    let w = qa.width.max(40.0);
    let h = qa.height.max(20.0);
    let bg = parse_hex_color(&qa.color, eff * 0.85);
    let tc = with_alpha(TEXT, eff);
    let bc = with_alpha(BADGE_BORDER, eff);

    let row = child(
        commands,
        parent,
        bsn! {
            Node {
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                column_gap: {px(5.)},
            }
        },
    );

    // Key badge
    if !qa.key.is_empty() {
        let kb = child(
            commands,
            row,
            bsn! {
                Node {
                    width: {px(16.)}, height: {px(14.)},
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    position_type: PositionType::Relative,
                    border: {UiRect::all(px(1.))},
                    border_radius: {BorderRadius::all(px(2.))},
                }
                BorderColor::all(BADGE_BORDER)
            },
        );
        child(commands, kb, text(&qa.key, 7., DIM));
    }

    // Main button rect
    let btn = child(
        commands,
        row,
        bsn! {
            Node {
                width: {px(w)}, height: {px(h)},
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                border: {UiRect::all(px(1.))},
                border_radius: {BorderRadius::all(px(4.))},
            }
            BackgroundColor({bg})
            BorderColor::all(bc)
        },
    );
    child(commands, btn, text(&qa.name, 10., tc));
}

/// Set-selection tabs pinned to the bottom of the canvas.
fn build_set_tabs(
    commands: &mut Commands,
    parent: Entity,
    cfg: &QuickActionConfig,
    ui: &EditorUiState,
) {
    let bar = commands
        .spawn_scene(bsn! {
            Node {
                position_type: PositionType::Absolute,
                bottom: {px(12.)}, left: {px(0.)}, right: {px(0.)},
                flex_direction: FlexDirection::Row,
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
            }
        })
        .id();
    commands.entity(parent).add_child(bar);

    // ‹ arrow
    let prev_idx = ui.active_set.saturating_sub(1);
    let larrow = clickable(
        commands,
        bar,
        bsn! {
            Node {
                width: {px(28.)}, height: {px(32.)},
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                border: {UiRect::all(px(1.))},
                border_radius: {BorderRadius::left(px(6.))},
            }
            BorderColor::all(SIDEBAR_BORDER)
            BackgroundColor({PANEL_CARD})
            Button
            Children [ text("‹", 14., DIM) ]
        },
        EditorAction::SetActiveSet { set: prev_idx },
        PANEL_CARD,
    );
    let _ = larrow;

    for (i, set) in cfg.sets.iter().enumerate() {
        let active = i == ui.active_set;
        let (bg, tc, bc) = if active {
            (Color::srgba(0.38, 0.62, 0.95, 0.20), TEXT, BLUE)
        } else {
            (PANEL_CARD, DIM, SIDEBAR_BORDER)
        };
        let tab = clickable(
            commands,
            bar,
            bsn! {
                Node {
                    padding: {UiRect::axes(px(14.), px(7.))},
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    border: {UiRect::all(px(1.))},
                }
                BorderColor::all(bc)
                BackgroundColor({bg})
                Button
            },
            EditorAction::SetActiveSet { set: i },
            bg,
        );
        child(commands, tab, text(&set.name, 11., tc));
    }

    // › arrow
    let next_idx = (ui.active_set + 1).min(cfg.sets.len().saturating_sub(1));
    let rarrow = clickable(
        commands,
        bar,
        bsn! {
            Node {
                width: {px(28.)}, height: {px(32.)},
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                border: {UiRect::all(px(1.))},
                border_radius: {BorderRadius::right(px(6.))},
            }
            BorderColor::all(SIDEBAR_BORDER)
            BackgroundColor({PANEL_CARD})
            Button
            Children [ text("›", 14., DIM) ]
        },
        EditorAction::SetActiveSet { set: next_idx },
        PANEL_CARD,
    );
    let _ = rarrow;
}

// ─── interaction ─────────────────────────────────────────────────────────────────

fn editor_button_feedback(
    mut buttons: Query<(
        &EditorButton,
        &Interaction,
        &mut BackgroundColor,
        Option<&SegmentHoverColor>,
    )>,
) {
    for (btn, interaction, mut bg, hover_col) in &mut buttons {
        *bg = match interaction {
            Interaction::Hovered => BackgroundColor(hover_col.map(|h| h.0).unwrap_or(ROW_HOVER)),
            Interaction::Pressed => BackgroundColor(ROW_SEL),
            Interaction::None => BackgroundColor(btn.base),
        };
    }
}

fn handle_editor_buttons(
    buttons: Query<(&EditorButton, &Interaction), Changed<Interaction>>,
    mut cfg: ResMut<QuickActionConfig>,
    mut ui: ResMut<EditorUiState>,
) {
    for (btn, interaction) in &buttons {
        if *interaction == Interaction::Pressed {
            apply_action(&btn.action, &mut cfg, &mut ui);
            ui.dirty = true;
        }
    }
}

// ─── action application ──────────────────────────────────────────────────────────

fn apply_action(action: &EditorAction, cfg: &mut QuickActionConfig, ui: &mut EditorUiState) {
    match *action {
        // ── sets ──────────────────────────────────────────────────────────────
        EditorAction::AddSet => {
            let n = cfg.sets.len() + 1;
            cfg.sets.push(ActionSet {
                name: format!("Set {}", n),
                opacity: 1.0,
                input_override: false,
                entries: Vec::new(),
            });
            ui.active_set = cfg.sets.len() - 1;
        }
        EditorAction::DeleteSet { set } => {
            if set < cfg.sets.len() {
                cfg.sets.remove(set);
            }
            let clear = matches!(ui.selection,
                Selection::Action { set: s, .. } | Selection::Wheel { set: s, .. }
                | Selection::Set { set: s } | Selection::WheelSetEntry { set: s, .. }
                if s == set);
            if clear {
                ui.selection = Selection::None;
                ui.editing = EditFocus::None;
            }
            if !cfg.sets.is_empty() {
                ui.active_set = ui.active_set.min(cfg.sets.len() - 1);
            }
        }
        EditorAction::SelectSet { set } => {
            ui.selection = Selection::Set { set };
            ui.editing = EditFocus::None;
        }
        EditorAction::EditSetName { set } => {
            ui.selection = Selection::Set { set };
            ui.editing = EditFocus::SetName;
        }
        EditorAction::SetOpacityDelta { set, delta } => {
            if let Some(s) = cfg.sets.get_mut(set) {
                s.opacity = (s.opacity + delta).clamp(0.0, 1.0);
            }
        }
        EditorAction::ToggleInputOverride { set } => {
            if let Some(s) = cfg.sets.get_mut(set) {
                s.input_override = !s.input_override;
            }
        }
        EditorAction::SetActiveSet { set } => {
            if !cfg.sets.is_empty() {
                ui.active_set = set.min(cfg.sets.len() - 1);
                ui.selection = Selection::None;
                ui.editing = EditFocus::None;
            }
        }
        // ── entries ───────────────────────────────────────────────────────────
        EditorAction::AddAction { set } => {
            if let Some(s) = cfg.sets.get_mut(set) {
                let n = s.entries.len() + 1;
                s.entries.push(SetEntry::Action(QuickAction {
                    name: format!("Action {}", n),
                    ..default()
                }));
            }
        }
        EditorAction::AddWheel { set } => {
            if let Some(s) = cfg.sets.get_mut(set) {
                s.entries.push(SetEntry::Wheel(Wheel::new("New Wheel", 6)));
            }
        }
        EditorAction::AddWheelSet { set } => {
            if let Some(s) = cfg.sets.get_mut(set) {
                s.entries.push(SetEntry::WheelSet(WheelSet {
                    name: "New Wheel Set".into(),
                    wheels: vec![Wheel::new("Wheel 1", 6)],
                    switch_key: String::new(),
                }));
            }
        }
        EditorAction::AddWheelToSet { set, entry } => {
            if let Some(SetEntry::WheelSet(ws)) =
                cfg.sets.get_mut(set).and_then(|s| s.entries.get_mut(entry))
            {
                let n = ws.wheels.len() + 1;
                ws.wheels.push(Wheel::new(format!("Wheel {}", n), 6));
            }
        }
        EditorAction::DeleteEntry { set, entry } => {
            if let Some(s) = cfg.sets.get_mut(set) {
                if entry < s.entries.len() {
                    s.entries.remove(entry);
                }
            }
            let clear = matches!(ui.selection,
                Selection::Action { set: s, entry: e } | Selection::Wheel { set: s, entry: e, .. }
                | Selection::WheelSetEntry { set: s, entry: e } if s == set && e == entry);
            if clear {
                ui.selection = Selection::None;
                ui.editing = EditFocus::None;
            }
        }
        EditorAction::DeleteWheelFromSet { set, entry, wheel } => {
            if let Some(SetEntry::WheelSet(ws)) =
                cfg.sets.get_mut(set).and_then(|s| s.entries.get_mut(entry))
            {
                if ws.wheels.len() > 1 && wheel < ws.wheels.len() {
                    ws.wheels.remove(wheel);
                }
            }
            let clear = matches!(ui.selection,
                Selection::Wheel { set: s, entry: e, wheel: Some(w) } if s == set && e == entry && w == wheel);
            if clear {
                ui.selection = Selection::None;
                ui.editing = EditFocus::None;
            }
        }
        EditorAction::MoveEntryUp { set, entry } => {
            if entry > 0 {
                if let Some(s) = cfg.sets.get_mut(set) {
                    if entry < s.entries.len() {
                        s.entries.swap(entry - 1, entry);
                    }
                }
                ui.selection = match ui.selection {
                    Selection::Action { set: s, entry: e } if s == set && e == entry => {
                        Selection::Action {
                            set,
                            entry: entry - 1,
                        }
                    }
                    Selection::Wheel {
                        set: s,
                        entry: e,
                        wheel: w,
                    } if s == set && e == entry => Selection::Wheel {
                        set,
                        entry: entry - 1,
                        wheel: w,
                    },
                    other => other,
                };
            }
        }
        EditorAction::MoveEntryDown { set, entry } => {
            let len = cfg.sets.get(set).map(|s| s.entries.len()).unwrap_or(0);
            if entry + 1 < len {
                if let Some(s) = cfg.sets.get_mut(set) {
                    s.entries.swap(entry, entry + 1);
                }
                ui.selection = match ui.selection {
                    Selection::Action { set: s, entry: e } if s == set && e == entry => {
                        Selection::Action {
                            set,
                            entry: entry + 1,
                        }
                    }
                    Selection::Wheel {
                        set: s,
                        entry: e,
                        wheel: w,
                    } if s == set && e == entry => Selection::Wheel {
                        set,
                        entry: entry + 1,
                        wheel: w,
                    },
                    other => other,
                };
            }
        }
        // ── selection ────────────────────────────────────────────────────────
        EditorAction::SelectAction { set, entry } => {
            ui.selection = Selection::Action { set, entry };
            ui.editing = EditFocus::None;
        }
        EditorAction::SelectWheel { set, entry, wheel } => {
            ui.selection = Selection::Wheel { set, entry, wheel };
            ui.editing = EditFocus::None;
        }
        EditorAction::SelectWheelSetEntry { set, entry } => {
            ui.selection = Selection::WheelSetEntry { set, entry };
            ui.editing = EditFocus::None;
        }
        EditorAction::SelectSetSwitch => {
            ui.selection = Selection::SetSwitch;
            ui.editing = EditFocus::None;
        }
        EditorAction::NavBack => {
            ui.editing = EditFocus::None;
            // From a segment, go back to the parent wheel editor.
            if let Selection::Segment {
                set, entry, wheel, ..
            } = ui.selection
            {
                ui.selection = Selection::Wheel { set, entry, wheel };
            } else {
                ui.selection = Selection::None;
            }
        }
        // ── quick action editing ─────────────────────────────────────────────
        EditorAction::EditName { set, entry } => {
            ui.selection = Selection::Action { set, entry };
            ui.editing = EditFocus::Name;
        }
        EditorAction::CaptureKey { set, entry } => {
            ui.selection = Selection::Action { set, entry };
            ui.editing = EditFocus::Key;
        }
        EditorAction::CycleIcon { set, entry } => {
            if let Some(a) = action_at(cfg, set, entry) {
                a.icon = cycle_in(ICON_PALETTE, &a.icon).into();
            }
        }
        EditorAction::CycleCommand { set, entry } => {
            if let Some(a) = action_at(cfg, set, entry) {
                a.command = cycle_in(COMMAND_PALETTE, &a.command).into();
            }
        }
        EditorAction::ToggleHold { set, entry } => {
            if let Some(a) = action_at(cfg, set, entry) {
                a.hold = !a.hold;
            }
        }
        EditorAction::ToggleShowOnMenu { set, entry } => {
            if let Some(a) = action_at(cfg, set, entry) {
                a.show_on_menu = !a.show_on_menu;
            }
        }
        EditorAction::ToggleEnabled { set, entry } => {
            if let Some(a) = action_at(cfg, set, entry) {
                a.enabled = !a.enabled;
            }
        }
        EditorAction::OpacityDelta { set, entry, delta } => {
            if let Some(a) = action_at(cfg, set, entry) {
                a.opacity = (a.opacity + delta).clamp(0.0, 1.0);
            }
        }
        EditorAction::RadiusDelta { set, entry, delta } => {
            if let Some(a) = action_at(cfg, set, entry) {
                a.radius = (a.radius + delta).clamp(8.0, 256.0);
            }
        }
        EditorAction::ActionWidthDelta { set, entry, delta } => {
            if let Some(a) = action_at(cfg, set, entry) {
                a.width = (a.width + delta).clamp(20.0, 300.0);
            }
        }
        EditorAction::ActionHeightDelta { set, entry, delta } => {
            if let Some(a) = action_at(cfg, set, entry) {
                a.height = (a.height + delta).clamp(12.0, 120.0);
            }
        }
        EditorAction::CyclePosition { set, entry } => {
            if let Some(a) = action_at(cfg, set, entry) {
                a.position = a.position.next();
            }
        }
        EditorAction::CycleShape { set, entry } => {
            if let Some(a) = action_at(cfg, set, entry) {
                a.shape = a.shape.next();
            }
        }
        // ── wheel editing ─────────────────────────────────────────────────────
        EditorAction::EditWheelName => {
            ui.editing = EditFocus::WheelName;
        }
        EditorAction::CaptureWheelHoldKey => {
            ui.editing = EditFocus::WheelHoldKey;
        }
        EditorAction::CycleWheelTheme => {
            if let Some(w) = wheel_at(cfg, ui.selection) {
                w.theme = w.theme.next();
            }
        }
        EditorAction::WheelCooldownDelta { delta } => {
            if let Some(w) = wheel_at(cfg, ui.selection) {
                w.cooldown_secs = (w.cooldown_secs + delta).clamp(0.0, 60.0);
            }
        }
        EditorAction::WheelOuterRadiusDelta { delta } => {
            if let Some(w) = wheel_at(cfg, ui.selection) {
                w.outer_radius = (w.outer_radius + delta).clamp(40.0, 300.0);
            }
        }
        EditorAction::WheelInnerRadiusDelta { delta } => {
            if let Some(w) = wheel_at(cfg, ui.selection) {
                w.inner_radius = (w.inner_radius + delta).clamp(8.0, 100.0);
            }
        }
        EditorAction::WheelAnimSpeedDelta { delta } => {
            if let Some(w) = wheel_at(cfg, ui.selection) {
                w.anim_speed_ms = (w.anim_speed_ms + delta).clamp(0.0, 2000.0);
            }
        }
        EditorAction::ToggleWheelShowLabels => {
            if let Some(w) = wheel_at(cfg, ui.selection) {
                w.show_labels = !w.show_labels;
            }
        }
        EditorAction::ToggleWheelShowInfoInHub => {
            if let Some(w) = wheel_at(cfg, ui.selection) {
                w.show_info_in_hub = !w.show_info_in_hub;
            }
        }
        EditorAction::AddSlot => {
            if let Some(w) = wheel_at(cfg, ui.selection) {
                let n = w.slots.len() + 1;
                w.slots.push(WheelSlotData::named(format!("Slot {}", n)));
            }
        }
        EditorAction::RemoveSlot => {
            if let Some(w) = wheel_at(cfg, ui.selection) {
                if w.slots.len() > 1 {
                    w.slots.pop();
                }
            }
        }
        EditorAction::EditSlotName { slot } => {
            ui.editing = EditFocus::SlotName(slot);
        }
        // ── wheel-set entry editing ───────────────────────────────────────────
        EditorAction::EditWheelSetName { set, entry } => {
            ui.selection = Selection::WheelSetEntry { set, entry };
            ui.editing = EditFocus::WheelSetName;
        }
        EditorAction::CaptureWheelSetSwitchKey { set, entry } => {
            ui.selection = Selection::WheelSetEntry { set, entry };
            ui.editing = EditFocus::WheelSetSwitchKey;
        }
        // ── set-switch shortcuts ──────────────────────────────────────────────
        EditorAction::CaptureNextSetKey => {
            ui.selection = Selection::SetSwitch;
            ui.editing = EditFocus::NextSetKey;
        }
        EditorAction::CapturePrevSetKey => {
            ui.selection = Selection::SetSwitch;
            ui.editing = EditFocus::PrevSetKey;
        }
        // ── persistence ───────────────────────────────────────────────────────
        EditorAction::Save => save_config(cfg, &ui.config_path),
        EditorAction::Load => {
            if let Some(loaded) = load_config(&ui.config_path) {
                *cfg = loaded;
                ui.selection = Selection::None;
                ui.active_set = 0;
            }
        }
        // ── segment editing ─────────────────────────────────────────────────
        EditorAction::SelectSegment {
            set,
            entry,
            wheel,
            slot,
        } => {
            ui.selection = Selection::Segment {
                set,
                entry,
                wheel,
                slot,
            };
            ui.editing = EditFocus::None;
        }
        EditorAction::EditSlotIcon { slot } => {
            ui.editing = EditFocus::SlotIcon(slot);
        }
        EditorAction::CycleSegmentShape => {
            if let Some(w) = wheel_at(cfg, ui.selection) {
                w.segment_shape = w.segment_shape.next();
            }
        }
        EditorAction::ToggleWheelShowIcon => {
            if let Some(w) = wheel_at(cfg, ui.selection) {
                w.show_icon = !w.show_icon;
            }
        }
        EditorAction::CycleHighlightColor => {
            const COLORS: &[&str] = &[
                "#f59e0b", "#3b82f6", "#14b8a6", "#8b5cf6", "#22c55e", "#ef4444", "#f97316",
            ];
            if let Some(w) = wheel_at(cfg, ui.selection) {
                w.highlight_color = cycle_in(COLORS, &w.highlight_color).into();
            }
        }
        EditorAction::SegmentScaleDelta { delta } => {
            if let Some(w) = wheel_at(cfg, ui.selection) {
                w.segment_scale = (w.segment_scale + delta).clamp(0.5, 2.0);
            }
        }
        // ── segment input / gamepad binding ─────────────────────────────────────────
        EditorAction::CaptureSlotInput { slot } => {
            ui.editing = EditFocus::SlotInput(slot);
        }
        EditorAction::CaptureGamepadButton { set, entry } => {
            ui.selection = Selection::Action { set, entry };
            ui.editing = EditFocus::GamepadButton;
        }
        // ── per-slot items ───────────────────────────────────────────────────────────
        EditorAction::AddSlotItem { slot } => {
            if let Some(w) = wheel_at(cfg, ui.selection) {
                if let Some(s) = w.slots.get_mut(slot) {
                    s.items.push(SlotItem::default());
                }
            }
        }
        EditorAction::RemoveSlotItem { slot, item } => {
            if let Some(w) = wheel_at(cfg, ui.selection) {
                if let Some(s) = w.slots.get_mut(slot) {
                    if item < s.items.len() {
                        s.items.remove(item);
                    }
                }
            }
            // Clear focus if it pointed to a removed item
            if matches!(ui.editing,
                EditFocus::SlotItemName(sl, it) | EditFocus::SlotItemIcon(sl, it)
                if sl == slot && it == item)
            {
                ui.editing = EditFocus::None;
            }
        }
        EditorAction::EditSlotItemName { slot, item } => {
            ui.editing = EditFocus::SlotItemName(slot, item);
        }
        EditorAction::EditSlotItemIcon { slot, item } => {
            ui.editing = EditFocus::SlotItemIcon(slot, item);
        }
    }
}

// ─── persistence ─────────────────────────────────────────────────────────────────

fn save_config(cfg: &QuickActionConfig, path: &str) {
    match ron::ser::to_string_pretty(cfg, ron::ser::PrettyConfig::default()) {
        Ok(s) => {
            let _ = std::fs::write(path, s);
        }
        Err(e) => eprintln!("[editor] save failed: {e}"),
    }
}

fn load_config(path: &str) -> Option<QuickActionConfig> {
    let s = std::fs::read_to_string(path).ok()?;
    match ron::from_str(&s) {
        Ok(c) => Some(c),
        Err(e) => {
            eprintln!("[editor] load failed: {e}");
            None
        }
    }
}

// ─── editor card / field helpers ─────────────────────────────────────────────────

fn editor_card() -> impl Scene {
    bsn! {
        Node {
            flex_direction: FlexDirection::Column,
            row_gap: {px(4.)},
            padding: {UiRect::all(px(8.))},
            margin: {UiRect::new(px(0.), px(0.), px(2.), px(4.))},
            border_radius: {BorderRadius::all(px(6.))},
        }
        BackgroundColor({PANEL_CARD})
    }
}

fn section_label(commands: &mut Commands, parent: Entity, label: &str) {
    let row = child(
        commands,
        parent,
        bsn! {
            Node { padding: {UiRect::new(px(0.), px(0.), px(6.), px(2.))} }
        },
    );
    child(commands, row, text(label, 10., DIM));
}

fn field_row() -> impl Scene {
    bsn! {
        Node {
            width: {percent(100.)}, min_height: {px(22.)},
            flex_direction: FlexDirection::Row,
            align_items: AlignItems::Center,
            column_gap: {px(6.)},
        }
    }
}

fn label_cell(s: &str) -> impl Scene {
    bsn! {
        Node { width: {px(82.)}, flex_direction: FlexDirection::Row, align_items: AlignItems::Center }
        Children [ text(s, 10., DIM) ]
    }
}

fn ctrl_box(accent: Color) -> impl Scene {
    bsn! {
        Node {
            flex_grow: 1., height: {px(20.)},
            padding: {UiRect::horizontal(px(6.))},
            flex_direction: FlexDirection::Row,
            align_items: AlignItems::Center,
            border: {UiRect::all(px(1.))},
            border_radius: {BorderRadius::all(px(4.))},
        }
        BorderColor::all(accent)
        Button
    }
}

fn mini_box() -> impl Scene {
    bsn! {
        Node {
            width: {px(22.)}, height: {px(20.)},
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            border_radius: {BorderRadius::all(px(4.))},
        }
        BackgroundColor({CTRL_BG})
        Button
    }
}

fn val_cell() -> impl Scene {
    bsn! {
        Node {
            flex_grow: 1., height: {px(20.)},
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
        }
    }
}

fn pill_box(bg: Color, accent: Color) -> impl Scene {
    bsn! {
        Node {
            padding: {UiRect::axes(px(10.), px(3.))},
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            border: {UiRect::all(px(1.))},
            border_radius: {BorderRadius::all(px(4.))},
        }
        BorderColor::all(accent)
        BackgroundColor({bg})
        Button
    }
}

fn spawn_field(commands: &mut Commands, parent: Entity, label: &str) -> Entity {
    let row = child(commands, parent, field_row());
    child(commands, row, label_cell(label));
    row
}

fn spawn_box_field(
    commands: &mut Commands,
    parent: Entity,
    label: &str,
    value: &str,
    value_color: Color,
    accent: Color,
    action: EditorAction,
) {
    let row = spawn_field(commands, parent, label);
    let b = clickable(commands, row, ctrl_box(accent), action, Color::NONE);
    child(commands, b, text(value, 11., value_color));
}

fn spawn_toggle_field(
    commands: &mut Commands,
    parent: Entity,
    label: &str,
    on: bool,
    action: EditorAction,
) {
    let row = spawn_field(commands, parent, label);
    let (bg, accent, txt, col) = if on {
        (GREEN_BG, GREEN, "ON", GREEN)
    } else {
        (Color::NONE, BADGE_BORDER, "OFF", DIM)
    };
    let p = clickable(commands, row, pill_box(bg, accent), action, bg);
    child(commands, p, text(txt, 10., col));
}

fn spawn_stepper_field(
    commands: &mut Commands,
    parent: Entity,
    label: &str,
    value: &str,
    dec: EditorAction,
    inc: EditorAction,
) {
    let row = spawn_field(commands, parent, label);
    let d = clickable(commands, row, mini_box(), dec, CTRL_BG);
    child(commands, d, text("−", 13., TEXT));
    let v = child(commands, row, val_cell());
    child(commands, v, text(value, 11., TEXT));
    let i = clickable(commands, row, mini_box(), inc, CTRL_BG);
    child(commands, i, text("+", 13., TEXT));
}

// ─── editor panels ────────────────────────────────────────────────────────────────

/// Button / quick-action editor.
fn spawn_action_editor(
    commands: &mut Commands,
    parent: Entity,
    ui: &EditorUiState,
    set: usize,
    entry: usize,
    qa: &QuickAction,
) {
    // Panel header: "BUTTON" label + gear/delete
    let hdr = child(
        commands,
        parent,
        bsn! {
            Node {
                flex_direction: FlexDirection::Row,
                justify_content: JustifyContent::SpaceBetween,
                align_items: AlignItems::Center,
                padding: {UiRect::bottom(px(4.))},
            }
        },
    );
    child(commands, hdr, text("BUTTON", 10., DIM));
    let dx = clickable(
        commands,
        hdr,
        del_btn(),
        EditorAction::DeleteEntry { set, entry },
        Color::NONE,
    );
    child(commands, dx, text("⚙", 10., DIMMER));

    let card = child(commands, parent, editor_card());

    // Label (name)
    let nf = ui.editing == EditFocus::Name;
    let nd = if nf {
        format!("{}|", qa.name)
    } else {
        qa.name.clone()
    };
    spawn_box_field(
        commands,
        card,
        "Label",
        &nd,
        TEXT,
        if nf { AMBER } else { BADGE_BORDER },
        EditorAction::EditName { set, entry },
    );

    // Keybind row
    {
        let row = spawn_field(commands, card, "Keybind");
        let kf = ui.editing == EditFocus::Key;
        let (kd, kc) = if kf {
            ("press key…".to_string(), AMBER)
        } else if qa.key.is_empty() {
            ("unbound".to_string(), DIM)
        } else {
            (qa.key.clone(), TEXT)
        };
        let kb = clickable(
            commands,
            row,
            bsn! {
                Node {
                    width: {px(44.)}, height: {px(20.)},
                    padding: {UiRect::horizontal(px(4.))},
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    border: {UiRect::all(px(1.))},
                    border_radius: {BorderRadius::all(px(4.))},
                }
                BorderColor::all(if kf { AMBER } else { BADGE_BORDER })
                Button
            },
            EditorAction::CaptureKey { set, entry },
            Color::NONE,
        );
        child(commands, kb, text(&kd, 10., kc));

        // Color swatch
        child(commands, row, text("Color", 9., DIM));
        let color_val = parse_hex_color(&qa.color, 1.0);
        let cs = child(
            commands,
            row,
            bsn! {
                Node {
                    width: {px(22.)}, height: {px(20.)},
                    border_radius: {BorderRadius::all(px(3.))},
                    border: {UiRect::all(px(1.))},
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                }
                BackgroundColor({color_val})
                BorderColor::all(BADGE_BORDER)
            },
        );
        child(commands, cs, text(&qa.color, 6., TEXT));
    }

    // Gamepad button row
    {
        let row = spawn_field(commands, card, "Gamepad");
        let gf = ui.editing == EditFocus::GamepadButton;
        let (gd, gc) = if gf {
            ("press button…".to_string(), AMBER)
        } else if qa.gamepad_button.is_empty() {
            ("unbound".to_string(), DIM)
        } else {
            (qa.gamepad_button.clone(), TEXT)
        };
        let gb = clickable(
            commands,
            row,
            bsn! {
                Node {
                    width: {px(56.)}, height: {px(20.)},
                    padding: {UiRect::horizontal(px(4.))},
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    border: {UiRect::all(px(1.))},
                    border_radius: {BorderRadius::all(px(4.))},
                }
                BorderColor::all(if gf { AMBER } else { BADGE_BORDER })
                Button
            },
            EditorAction::CaptureGamepadButton { set, entry },
            Color::NONE,
        );
        child(commands, gb, text(&gd, 9., gc));
    }

    // Action / command
    spawn_box_field(
        commands,
        card,
        "Action",
        &qa.command,
        TEAL,
        BADGE_BORDER,
        EditorAction::CycleCommand { set, entry },
    );

    // Width + Height on one row
    {
        let row = child(commands, card, field_row());
        child(commands, row, label_cell("Width"));
        let dw = clickable(
            commands,
            row,
            mini_box(),
            EditorAction::ActionWidthDelta {
                set,
                entry,
                delta: -4.0,
            },
            CTRL_BG,
        );
        child(commands, dw, text("−", 13., TEXT));
        let vw = child(commands, row, val_cell());
        child(commands, vw, text(&format!("{:.0}", qa.width), 11., TEXT));
        let iw = clickable(
            commands,
            row,
            mini_box(),
            EditorAction::ActionWidthDelta {
                set,
                entry,
                delta: 4.0,
            },
            CTRL_BG,
        );
        child(commands, iw, text("+", 13., TEXT));

        child(commands, row, text("H", 9., DIM));
        let dh = clickable(
            commands,
            row,
            mini_box(),
            EditorAction::ActionHeightDelta {
                set,
                entry,
                delta: -2.0,
            },
            CTRL_BG,
        );
        child(commands, dh, text("−", 13., TEXT));
        let vh = child(commands, row, val_cell());
        child(commands, vh, text(&format!("{:.0}", qa.height), 11., TEXT));
        let ih = clickable(
            commands,
            row,
            mini_box(),
            EditorAction::ActionHeightDelta {
                set,
                entry,
                delta: 2.0,
            },
            CTRL_BG,
        );
        child(commands, ih, text("+", 13., TEXT));
    }

    // Enabled toggle
    spawn_toggle_field(
        commands,
        card,
        "Enabled",
        qa.enabled,
        EditorAction::ToggleEnabled { set, entry },
    );

    // Reposition hint
    child(
        commands,
        parent,
        bsn! {
            Node { padding: {UiRect::new(px(4.), px(0.), px(8.), px(0.))} }
            Children [ text("Drag in the preview to reposition.", 9., DIMMER) ]
        },
    );
}

fn key_display(focus: bool, key: &str) -> (String, Color) {
    if focus {
        ("press a key…".to_string(), AMBER)
    } else if key.is_empty() {
        ("unbound".to_string(), DIM)
    } else {
        (key.to_string(), TEXT)
    }
}

/// Wheel editor panel.
fn spawn_wheel_editor(
    commands: &mut Commands,
    parent: Entity,
    ui: &EditorUiState,
    w: &Wheel,
    set: usize,
    entry: usize,
    w_idx: Option<usize>,
) {
    section_label(commands, parent, "WHEEL");
    let card = child(commands, parent, editor_card());

    // Name
    let nf = ui.editing == EditFocus::WheelName;
    let nd = if nf {
        format!("{}|", w.name)
    } else {
        w.name.clone()
    };
    spawn_box_field(
        commands,
        card,
        "Name",
        &nd,
        TEXT,
        if nf { AMBER } else { BADGE_BORDER },
        EditorAction::EditWheelName,
    );

    // Hold Key
    let kf = ui.editing == EditFocus::WheelHoldKey;
    let (kd, kc) = key_display(kf, &w.hold_key);
    spawn_box_field(
        commands,
        card,
        "Hold Key",
        &kd,
        kc,
        if kf { AMBER } else { BADGE_BORDER },
        EditorAction::CaptureWheelHoldKey,
    );

    // Theme
    spawn_box_field(
        commands,
        card,
        "Theme",
        w.theme.label(),
        TEXT,
        BADGE_BORDER,
        EditorAction::CycleWheelTheme,
    );

    // Outer Radius
    spawn_stepper_field(
        commands,
        card,
        "Outer Radius",
        &format!("{:.0}", w.outer_radius),
        EditorAction::WheelOuterRadiusDelta { delta: -5.0 },
        EditorAction::WheelOuterRadiusDelta { delta: 5.0 },
    );

    // Inner Radius
    spawn_stepper_field(
        commands,
        card,
        "Inner Radius",
        &format!("{:.0}", w.inner_radius),
        EditorAction::WheelInnerRadiusDelta { delta: -2.0 },
        EditorAction::WheelInnerRadiusDelta { delta: 2.0 },
    );

    // Anim Speed
    spawn_stepper_field(
        commands,
        card,
        "Anim ms",
        &format!("{:.0}", w.anim_speed_ms),
        EditorAction::WheelAnimSpeedDelta { delta: -25.0 },
        EditorAction::WheelAnimSpeedDelta { delta: 25.0 },
    );

    // Toggles
    spawn_toggle_field(
        commands,
        card,
        "Show labels",
        w.show_labels,
        EditorAction::ToggleWheelShowLabels,
    );
    spawn_toggle_field(
        commands,
        card,
        "Segment in hub",
        w.show_info_in_hub,
        EditorAction::ToggleWheelShowInfoInHub,
    );

    // Segment Shape
    spawn_box_field(
        commands,
        card,
        "Seg Shape",
        w.segment_shape.label(),
        TEXT,
        BADGE_BORDER,
        EditorAction::CycleSegmentShape,
    );

    // Segment Scale
    spawn_stepper_field(
        commands,
        card,
        "Seg Scale",
        &format!("{:.1}", w.segment_scale),
        EditorAction::SegmentScaleDelta { delta: -0.1 },
        EditorAction::SegmentScaleDelta { delta: 0.1 },
    );

    // Show Icons
    spawn_toggle_field(
        commands,
        card,
        "Show icons",
        w.show_icon,
        EditorAction::ToggleWheelShowIcon,
    );

    // Highlight color row
    {
        let hcol = parse_hex_color(&w.highlight_color, 1.0);
        let hex = w.highlight_color.clone();
        let hrow = spawn_field(commands, card, "Highlight");
        let b = clickable(
            commands,
            hrow,
            ctrl_box(BADGE_BORDER),
            EditorAction::CycleHighlightColor,
            Color::NONE,
        );
        child(
            commands,
            b,
            bsn! {
                Node {
                    width: {px(12.)}, height: {px(12.)},
                    border_radius: {BorderRadius::all(px(2.))},
                }
                BackgroundColor({hcol})
            },
        );
        child(commands, b, text(&hex, 11., TEXT));
    }

    // Segments section
    let seg_hdr = child(
        commands,
        parent,
        bsn! {
            Node {
                flex_direction: FlexDirection::Row,
                justify_content: JustifyContent::SpaceBetween,
                align_items: AlignItems::Center,
                padding: {UiRect::new(px(0.), px(0.), px(6.), px(8.))},
            }
        },
    );
    child(commands, seg_hdr, text("SEGMENTS", 10., DIM));
    clickable(
        commands,
        seg_hdr,
        bsn! {
            Node {
                padding: {UiRect::axes(px(8.), px(3.))},
                border: {UiRect::all(px(1.))},
                border_radius: {BorderRadius::all(px(3.))},
            }
            BorderColor::all(GREEN)
            BackgroundColor({GREEN_BG})
            Button
            Children [ text("+ Add", 9., GREEN) ]
        },
        EditorAction::AddSlot,
        Color::NONE,
    );

    let seg_card = child(commands, parent, editor_card());
    if w.slots.is_empty() {
        child(commands, seg_card, text("No segments.", 10., DIMMER));
    }
    for (i, slot) in w.slots.iter().enumerate() {
        let is_sel = ui.selection
            == (Selection::Segment {
                set,
                entry,
                wheel: w_idx,
                slot: i,
            });
        let row_bg = if is_sel { ROW_SEL } else { Color::NONE };
        let row = clickable(
            commands,
            seg_card,
            row_button(row_bg),
            EditorAction::SelectSegment {
                set,
                entry,
                wheel: w_idx,
                slot: i,
            },
            row_bg,
        );
        let left = child(commands, row, hcluster());
        child(commands, left, text(&format!("{}", i + 1), 9., DIMMER));
        if !slot.icon.is_empty() {
            child(commands, left, text(&slot.icon, 11., TEXT));
        }
        child(commands, left, text(&slot.name, 11., TEXT));
        // Show item count badge if there are items
        if !slot.items.is_empty() {
            child(
                commands,
                left,
                text(&format!("[{}]", slot.items.len()), 9., TEAL),
            );
        }
        // Show input badge
        if !slot.input.is_empty() {
            let right = child(commands, row, hcluster());
            let kb = child(commands, right, key_badge_box());
            child(commands, kb, text(&slot.input, 8., DIM));
        }
        let right2 = child(commands, row, hcluster());
        let dx = clickable(
            commands,
            right2,
            del_btn(),
            EditorAction::RemoveSlot,
            Color::NONE,
        );
        child(commands, dx, text("×", 10., DIMMER));
    }
}

/// Segment editor panel — per-slot name, icon, input binding, and items list.
fn spawn_segment_editor(
    commands: &mut Commands,
    parent: Entity,
    ui: &EditorUiState,
    slot: usize,
    w: &Wheel,
) {
    let slot_data = w.slots.get(slot);
    let slot_name = slot_data.map(|s| s.name.as_str()).unwrap_or("");
    let slot_icon = slot_data.map(|s| s.icon.as_str()).unwrap_or("");
    let slot_input = slot_data.map(|s| s.input.as_str()).unwrap_or("");
    let items = slot_data.map(|s| s.items.as_slice()).unwrap_or(&[]);

    section_label(commands, parent, "SEGMENT");
    let card = child(commands, parent, editor_card());

    // Name
    let nf = ui.editing == EditFocus::SlotName(slot);
    let nd = if nf {
        format!("{}|", slot_name)
    } else {
        slot_name.to_string()
    };
    spawn_box_field(
        commands,
        card,
        "Name",
        &nd,
        TEXT,
        if nf { AMBER } else { BADGE_BORDER },
        EditorAction::EditSlotName { slot },
    );

    // Icon
    let icon_f = ui.editing == EditFocus::SlotIcon(slot);
    let id = if icon_f {
        format!("{}|", slot_icon)
    } else {
        slot_icon.to_string()
    };
    spawn_box_field(
        commands,
        card,
        "Icon",
        &id,
        TEXT,
        if icon_f { AMBER } else { BADGE_BORDER },
        EditorAction::EditSlotIcon { slot },
    );

    // Input binding (keyboard key or gamepad button)
    let inp_kf = ui.editing == EditFocus::SlotInput(slot);
    let (inp_d, inp_c) = if inp_kf {
        ("press key / button…".to_string(), AMBER)
    } else if slot_input.is_empty() {
        ("unbound".to_string(), DIM)
    } else {
        (slot_input.to_string(), TEXT)
    };
    spawn_box_field(
        commands,
        card,
        "Input",
        &inp_d,
        inp_c,
        if inp_kf { AMBER } else { BADGE_BORDER },
        EditorAction::CaptureSlotInput { slot },
    );

    // ── Items section ───────────────────────────────────────────────────────────
    let items_hdr = child(
        commands,
        parent,
        bsn! {
            Node {
                flex_direction: FlexDirection::Row,
                justify_content: JustifyContent::SpaceBetween,
                align_items: AlignItems::Center,
                padding: {UiRect::new(px(0.), px(0.), px(6.), px(8.))},
            }
        },
    );
    child(commands, items_hdr, text("ITEMS", 10., DIM));
    clickable(
        commands,
        items_hdr,
        bsn! {
            Node {
                padding: {UiRect::axes(px(8.), px(3.))},
                border: {UiRect::all(px(1.))},
                border_radius: {BorderRadius::all(px(3.))},
            }
            BorderColor::all(TEAL)
            Button
            Children [ text("+ Add", 9., TEAL) ]
        },
        EditorAction::AddSlotItem { slot },
        Color::NONE,
    );

    let icard = child(commands, parent, editor_card());
    if items.is_empty() {
        child(
            commands,
            icard,
            text("No items. Add one above.", 10., DIMMER),
        );
    }
    for (ii, item) in items.iter().enumerate() {
        let item_row = child(
            commands,
            icard,
            bsn! {
                Node {
                    width: {percent(100.)}, height: {px(26.)},
                    flex_direction: FlexDirection::Row,
                    align_items: AlignItems::Center,
                    column_gap: {px(4.)},
                }
            },
        );
        child(commands, item_row, text(&format!("{}", ii + 1), 9., DIMMER));

        // Item name field
        let iname_f = ui.editing == EditFocus::SlotItemName(slot, ii);
        let item_nd = if iname_f {
            format!("{}|", item.name)
        } else if item.name.is_empty() {
            "name…".to_string()
        } else {
            item.name.clone()
        };
        let nb = clickable(
            commands,
            item_row,
            bsn! {
                Node {
                    flex_grow: 1., height: {px(20.)},
                    padding: {UiRect::horizontal(px(6.))},
                    flex_direction: FlexDirection::Row,
                    align_items: AlignItems::Center,
                    border: {UiRect::all(px(1.))},
                    border_radius: {BorderRadius::all(px(4.))},
                }
                BorderColor::all(if iname_f { AMBER } else { BADGE_BORDER })
                Button
            },
            EditorAction::EditSlotItemName { slot, item: ii },
            Color::NONE,
        );
        child(
            commands,
            nb,
            text(&item_nd, 10., if iname_f { AMBER } else { TEXT }),
        );

        // Item icon field
        let iicon_f = ui.editing == EditFocus::SlotItemIcon(slot, ii);
        let item_id = if iicon_f {
            format!("{}|", item.icon)
        } else if item.icon.is_empty() {
            "◆".to_string()
        } else {
            item.icon.clone()
        };
        let ib = clickable(
            commands,
            item_row,
            bsn! {
                Node {
                    width: {px(28.)}, height: {px(20.)},
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    border: {UiRect::all(px(1.))},
                    border_radius: {BorderRadius::all(px(4.))},
                }
                BorderColor::all(if iicon_f { AMBER } else { BADGE_BORDER })
                Button
            },
            EditorAction::EditSlotItemIcon { slot, item: ii },
            Color::NONE,
        );
        child(
            commands,
            ib,
            text(&item_id, 10., if iicon_f { AMBER } else { DIM }),
        );

        let dx = clickable(
            commands,
            item_row,
            del_btn(),
            EditorAction::RemoveSlotItem { slot, item: ii },
            Color::NONE,
        );
        child(commands, dx, text("×", 10., DIMMER));
    }
}

/// WheelSet-entry editor panel.
fn spawn_wheelset_entry_editor(
    commands: &mut Commands,
    parent: Entity,
    ui: &EditorUiState,
    set: usize,
    entry: usize,
    ws: &WheelSet,
) {
    section_label(commands, parent, "WHEEL SET");
    let card = child(commands, parent, editor_card());

    // Name
    let nf = ui.editing == EditFocus::WheelSetName;
    let nd = if nf {
        format!("{}|", ws.name)
    } else {
        ws.name.clone()
    };
    spawn_box_field(
        commands,
        card,
        "Name",
        &nd,
        TEXT,
        if nf { AMBER } else { BADGE_BORDER },
        EditorAction::EditWheelSetName { set, entry },
    );

    // Switch Key
    let kf = ui.editing == EditFocus::WheelSetSwitchKey;
    let (kd, kc) = key_display(kf, &ws.switch_key);
    spawn_box_field(
        commands,
        card,
        "Switch Key",
        &kd,
        kc,
        if kf { AMBER } else { BADGE_BORDER },
        EditorAction::CaptureWheelSetSwitchKey { set, entry },
    );

    // Wheels sub-list
    let wh_hdr = child(
        commands,
        parent,
        bsn! {
            Node {
                flex_direction: FlexDirection::Row,
                justify_content: JustifyContent::SpaceBetween,
                align_items: AlignItems::Center,
                padding: {UiRect::new(px(0.), px(0.), px(6.), px(8.))},
            }
        },
    );
    child(commands, wh_hdr, text("WHEELS", 10., DIM));
    clickable(
        commands,
        wh_hdr,
        bsn! {
            Node {
                padding: {UiRect::axes(px(8.), px(3.))},
                border: {UiRect::all(px(1.))},
                border_radius: {BorderRadius::all(px(3.))},
            }
            BorderColor::all(BLUE)
            Button
            Children [ text("+ Add", 9., BLUE) ]
        },
        EditorAction::AddWheelToSet { set, entry },
        Color::NONE,
    );

    let wcard = child(commands, parent, editor_card());
    if ws.wheels.is_empty() {
        child(commands, wcard, text("No wheels.", 10., DIMMER));
    }
    for (wi, w) in ws.wheels.iter().enumerate() {
        let wsel = ui.selection
            == (Selection::Wheel {
                set,
                entry,
                wheel: Some(wi),
            });
        let badge = if w.hold_key.is_empty() {
            Badge::None
        } else {
            Badge::Key(w.hold_key.clone())
        };
        spawn_entry_row(
            commands,
            wcard,
            wsel,
            EditorAction::SelectWheel {
                set,
                entry,
                wheel: Some(wi),
            },
            "○",
            ICON,
            &w.name,
            TEAL,
            badge,
            Some(EditorAction::DeleteWheelFromSet {
                set,
                entry,
                wheel: wi,
            }),
        );
    }

    // Hint
    child(
        commands,
        parent,
        bsn! {
            Node { padding: {UiRect::new(px(4.), px(0.), px(8.), px(0.))} }
            Children [ text("Select a wheel above to edit its\nsegments and settings.", 9., DIMMER) ]
        },
    );
}

// ─── helpers ─────────────────────────────────────────────────────────────────────

fn action_at<'a>(
    cfg: &'a mut QuickActionConfig,
    set: usize,
    entry: usize,
) -> Option<&'a mut QuickAction> {
    match cfg.sets.get_mut(set).and_then(|s| s.entries.get_mut(entry)) {
        Some(SetEntry::Action(a)) => Some(a),
        _ => None,
    }
}

fn wheel_at(cfg: &mut QuickActionConfig, sel: Selection) -> Option<&mut Wheel> {
    let (set, entry, wheel) = match sel {
        Selection::Wheel { set, entry, wheel } => (set, entry, wheel),
        Selection::Segment {
            set, entry, wheel, ..
        } => (set, entry, wheel),
        _ => return None,
    };
    match cfg.sets.get_mut(set).and_then(|s| s.entries.get_mut(entry)) {
        Some(SetEntry::Wheel(w)) if wheel.is_none() => Some(w),
        Some(SetEntry::WheelSet(ws)) => wheel.and_then(move |i| ws.wheels.get_mut(i)),
        _ => None,
    }
}

fn focused_name<'a>(cfg: &'a mut QuickActionConfig, ui: &EditorUiState) -> Option<&'a mut String> {
    match ui.editing {
        EditFocus::Name => match ui.selection {
            Selection::Action { set, entry } => action_at(cfg, set, entry).map(|a| &mut a.name),
            _ => None,
        },
        EditFocus::SetName => match ui.selection {
            Selection::Set { set } => cfg.sets.get_mut(set).map(|s| &mut s.name),
            _ => None,
        },
        EditFocus::WheelName => wheel_at(cfg, ui.selection).map(|w| &mut w.name),
        EditFocus::SlotName(i) => {
            wheel_at(cfg, ui.selection).and_then(move |w| w.slots.get_mut(i).map(|s| &mut s.name))
        }
        EditFocus::SlotIcon(i) => {
            wheel_at(cfg, ui.selection).and_then(move |w| w.slots.get_mut(i).map(|s| &mut s.icon))
        }
        EditFocus::SlotInput(i) => {
            wheel_at(cfg, ui.selection).and_then(move |w| w.slots.get_mut(i).map(|s| &mut s.input))
        }
        EditFocus::SlotItemName(slot, item) => wheel_at(cfg, ui.selection)
            .and_then(move |w| w.slots.get_mut(slot))
            .and_then(move |s| s.items.get_mut(item).map(|it| &mut it.name)),
        EditFocus::SlotItemIcon(slot, item) => wheel_at(cfg, ui.selection)
            .and_then(move |w| w.slots.get_mut(slot))
            .and_then(move |s| s.items.get_mut(item).map(|it| &mut it.icon)),
        EditFocus::WheelSetName => match ui.selection {
            Selection::WheelSetEntry { set, entry } => cfg
                .sets
                .get_mut(set)
                .and_then(|s| s.entries.get_mut(entry))
                .and_then(|e| {
                    if let SetEntry::WheelSet(ws) = e {
                        Some(&mut ws.name)
                    } else {
                        None
                    }
                }),
            _ => None,
        },
        _ => None,
    }
}

// ─── keyboard input ───────────────────────────────────────────────────────────────

fn editor_text_input(
    mut messages: MessageReader<KeyboardInput>,
    mut cfg: ResMut<QuickActionConfig>,
    mut ui: ResMut<EditorUiState>,
) {
    if !matches!(
        ui.editing,
        EditFocus::Name
            | EditFocus::SetName
            | EditFocus::WheelName
            | EditFocus::SlotName(_)
            | EditFocus::SlotIcon(_)
            | EditFocus::SlotInput(_)
            | EditFocus::SlotItemName(_, _)
            | EditFocus::SlotItemIcon(_, _)
            | EditFocus::WheelSetName
    ) {
        messages.clear();
        return;
    }

    let mut changed = false;
    let mut stop = false;
    for ev in messages.read() {
        if ev.state != ButtonState::Pressed {
            continue;
        }
        match &ev.logical_key {
            Key::Enter | Key::Escape => {
                stop = true;
                changed = true;
            }
            Key::Backspace => {
                if let Some(n) = focused_name(&mut cfg, &ui) {
                    n.pop();
                    changed = true;
                }
            }
            Key::Space => {
                if let Some(n) = focused_name(&mut cfg, &ui) {
                    if n.chars().count() < 24 {
                        n.push(' ');
                        changed = true;
                    }
                }
            }
            Key::Character(s) => {
                if let Some(n) = focused_name(&mut cfg, &ui) {
                    if n.chars().count() < 24 {
                        n.push_str(s);
                        changed = true;
                    }
                }
            }
            _ => {}
        }
    }
    if stop {
        ui.editing = EditFocus::None;
    }
    if changed {
        ui.dirty = true;
    }
}

fn editor_capture_key(
    keys: Res<ButtonInput<KeyCode>>,
    mut cfg: ResMut<QuickActionConfig>,
    mut ui: ResMut<EditorUiState>,
) {
    let focus = ui.editing;
    if !matches!(
        focus,
        EditFocus::Key
            | EditFocus::NextSetKey
            | EditFocus::PrevSetKey
            | EditFocus::WheelHoldKey
            | EditFocus::WheelSetSwitchKey
            | EditFocus::SlotInput(_)
    ) {
        return;
    }

    for key in keys.get_just_pressed() {
        if is_modifier(*key) {
            continue;
        }
        if *key != KeyCode::Escape {
            let label = key_label(*key);
            match focus {
                EditFocus::Key => {
                    if let Selection::Action { set, entry } = ui.selection {
                        if let Some(a) = action_at(&mut cfg, set, entry) {
                            a.key = label;
                        }
                    }
                }
                EditFocus::WheelHoldKey => {
                    if let Some(w) = wheel_at(&mut cfg, ui.selection) {
                        w.hold_key = label;
                    }
                }
                EditFocus::WheelSetSwitchKey => {
                    if let Selection::WheelSetEntry { set, entry } = ui.selection {
                        if let Some(SetEntry::WheelSet(ws)) =
                            cfg.sets.get_mut(set).and_then(|s| s.entries.get_mut(entry))
                        {
                            ws.switch_key = label;
                        }
                    }
                }
                EditFocus::NextSetKey => cfg.next_set_key = label,
                EditFocus::PrevSetKey => cfg.prev_set_key = label,
                EditFocus::SlotInput(slot) => {
                    if let Some(w) = wheel_at(&mut cfg, ui.selection) {
                        if let Some(s) = w.slots.get_mut(slot) {
                            s.input = label;
                        }
                    }
                }
                _ => {}
            }
        }
        ui.editing = EditFocus::None;
        ui.dirty = true;
        return;
    }
}

fn editor_capture_gamepad(
    gamepads: Query<&Gamepad>,
    mut cfg: ResMut<QuickActionConfig>,
    mut ui: ResMut<EditorUiState>,
) {
    let focus = ui.editing;
    if !matches!(focus, EditFocus::GamepadButton | EditFocus::SlotInput(_)) {
        return;
    }
    const BUTTONS: &[GamepadButton] = &[
        GamepadButton::South,
        GamepadButton::East,
        GamepadButton::North,
        GamepadButton::West,
        GamepadButton::LeftTrigger,
        GamepadButton::RightTrigger,
        GamepadButton::LeftTrigger2,
        GamepadButton::RightTrigger2,
        GamepadButton::Start,
        GamepadButton::Select,
        GamepadButton::LeftThumb,
        GamepadButton::RightThumb,
    ];
    for gamepad in &gamepads {
        for &btn in BUTTONS {
            if gamepad.just_pressed(btn) {
                let label = gamepad_btn_label(btn);
                match focus {
                    EditFocus::GamepadButton => {
                        if let Selection::Action { set, entry } = ui.selection {
                            if let Some(a) = action_at(&mut cfg, set, entry) {
                                a.gamepad_button = label;
                            }
                        }
                    }
                    EditFocus::SlotInput(slot) => {
                        if let Some(w) = wheel_at(&mut cfg, ui.selection) {
                            if let Some(s) = w.slots.get_mut(slot) {
                                s.input = format!("GP:{}", label);
                            }
                        }
                    }
                    _ => {}
                }
                ui.editing = EditFocus::None;
                ui.dirty = true;
                return;
            }
        }
    }
}

fn key_label(k: KeyCode) -> String {
    let dbg = format!("{:?}", k);
    let s = dbg.strip_prefix("Key").unwrap_or(&dbg);
    s.strip_prefix("Digit").unwrap_or(s).to_string()
}

fn gamepad_btn_label(btn: GamepadButton) -> String {
    match btn {
        GamepadButton::South => "A".into(),
        GamepadButton::East => "B".into(),
        GamepadButton::North => "Y".into(),
        GamepadButton::West => "X".into(),
        GamepadButton::LeftTrigger => "LB".into(),
        GamepadButton::RightTrigger => "RB".into(),
        GamepadButton::LeftTrigger2 => "LT".into(),
        GamepadButton::RightTrigger2 => "RT".into(),
        GamepadButton::Start => "Start".into(),
        GamepadButton::Select => "Select".into(),
        GamepadButton::LeftThumb => "LS".into(),
        GamepadButton::RightThumb => "RS".into(),
        _ => format!("{:?}", btn),
    }
}

fn is_modifier(k: KeyCode) -> bool {
    matches!(
        k,
        KeyCode::ShiftLeft
            | KeyCode::ShiftRight
            | KeyCode::ControlLeft
            | KeyCode::ControlRight
            | KeyCode::AltLeft
            | KeyCode::AltRight
            | KeyCode::SuperLeft
            | KeyCode::SuperRight
    )
}
