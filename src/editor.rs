//! BSN-macro Quick Action Menu editor.
//!
//! The editor is a self-contained Bevy plugin ([`QuickActionEditorPlugin`]) that
//! lets you author a [`QuickActionConfig`] at runtime. The document is a list of
//! **action sets**; each set holds an ordered list of entries, where every entry
//! is one of:
//!
//! - a **quick action** (a named, key-bound button),
//! - a standalone **wheel** (a radial menu), or
//! - a **wheel set** (a group of wheels the player can switch between).
//!
//! The whole left sidebar — header, `SETS` bar, the scrollable set/entry tree and
//! the save / load footer — is declared with the [`bsn!`](bevy::prelude::bsn)
//! macro and rebuilt only when the document or selection changes. The right-hand
//! canvas is a live overview of every set and its children (quick actions and
//! wheel sets); selecting a wheel swaps it to that wheel's radial preview. Each
//! set has an inline settings submenu (name / opacity / input override), and a
//! pinned submenu at the top of the tree configures the next / previous set-switch
//! shortcuts.

use crate::*;
use bevy::color::Alpha;
use bevy::ecs::message::MessageReader;
use bevy::input::keyboard::{Key, KeyboardInput};
use bevy::input::ButtonState;
use bevy::prelude::*;
use serde::{Deserialize, Serialize};

// ─── document model ─────────────────────────────────────────────────────────────

/// Position reference for a quick action's on-screen placement.
#[derive(Clone, Copy, PartialEq, Serialize, Deserialize, Debug, Default)]
pub enum PositionMode {
    /// Positioned relative to its anchor / menu.
    #[default]
    Relative,
    /// Positioned at an absolute screen location.
    Absolute,
}

impl PositionMode {
    /// Short editor label.
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

/// Button shape for a quick action.
#[derive(Clone, Copy, PartialEq, Serialize, Deserialize, Debug, Default)]
pub enum ActionShape {
    /// Rounded square (default).
    #[default]
    Rounded,
    /// Full circle.
    Round,
    /// Sharp square.
    Square,
    /// Diamond.
    Diamond,
}

impl ActionShape {
    /// Short editor label.
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

/// Glyphs cycled through when assigning an action icon.
const ICON_PALETTE: &[&str] = &["◆", "●", "★", "▲", "✦", "✚", "◈", "○", "◐", "✱"];
/// Commands cycled through when assigning an action command.
const COMMAND_PALETTE: &[&str] =
    &["none", "attack", "heal", "block", "dash", "reload", "interact", "jump", "crouch", "sprint"];

/// Returns the element after `current` in `list` (wrapping).
fn cycle_in<'a>(list: &[&'a str], current: &str) -> &'a str {
    let idx = list.iter().position(|s| *s == current).unwrap_or(0);
    list[(idx + 1) % list.len()]
}

/// A key-bound quick-action button.
#[derive(Clone, Serialize, Deserialize, Debug)]
#[serde(default)]
pub struct QuickAction {
    /// Display name.
    pub name: String,
    /// Captured key-binding label (e.g. `"X"`). Empty means unbound.
    pub key: String,
    /// Icon glyph.
    pub icon: String,
    /// Assigned command id.
    pub command: String,
    /// Hold-to-activate (vs press).
    pub hold: bool,
    /// Whether the action is shown on the menu.
    pub show_on_menu: bool,
    /// Opacity multiplier, 0.0–1.0.
    pub opacity: f32,
    /// Placement reference.
    pub position: PositionMode,
    /// Button radius in logical pixels.
    pub radius: f32,
    /// Button shape.
    pub shape: ActionShape,
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
        }
    }
}

/// A single radial wheel: a named menu with labelled slots and a cooldown.
#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct Wheel {
    /// Display name.
    pub name: String,
    /// Cooldown in seconds (shown as the `Ns` badge).
    pub cooldown_secs: f32,
    /// Slot labels. Their count is the number of slices.
    pub slots: Vec<String>,
}

impl Wheel {
    /// A fresh wheel with `n` generic slots.
    pub fn new(name: impl Into<String>, n: usize) -> Self {
        Self {
            name: name.into(),
            cooldown_secs: 6.0,
            slots: (0..n.max(1)).map(|i| format!("Slot {}", i + 1)).collect(),
        }
    }

    /// Build a runtime [`WheelMenu`] for previewing this wheel.
    pub fn to_menu(&self) -> WheelMenu {
        WheelMenu {
            slices: self.slots.len().max(1),
            radius: 170.0,
            inner_radius: 58.0,
            deadzone: 0.3,
            gap: 0.03,
            arc_span: std::f32::consts::TAU,
            arc_offset: 0.0,
            overlap: false,
        }
    }
}

/// A group of wheels the player can switch between.
#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct WheelSet {
    /// Display name.
    pub name: String,
    /// The wheels in switch order.
    pub wheels: Vec<Wheel>,
}

/// One entry inside an [`ActionSet`].
#[derive(Clone, Serialize, Deserialize, Debug)]
pub enum SetEntry {
    /// A key-bound quick action.
    Action(QuickAction),
    /// A standalone wheel.
    Wheel(Wheel),
    /// A group of wheels.
    WheelSet(WheelSet),
}

/// A set of quick actions / wheels (e.g. bound to a gameplay context).
#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct ActionSet {
    /// Display name.
    pub name: String,
    /// Opacity multiplier applied to the whole set, 0.0–1.0.
    #[serde(default = "full_opacity")]
    pub opacity: f32,
    /// When `true`, the set's binding overrides its children's input bindings.
    #[serde(default)]
    pub input_override: bool,
    /// Ordered entries.
    pub entries: Vec<SetEntry>,
}

/// Serde default for opacity fields (fully opaque).
fn full_opacity() -> f32 {
    1.0
}

/// The complete editable document. Lives as a Bevy [`Resource`] and is what the
/// editor mutates and saves / loads as RON.
#[derive(Resource, Clone, Serialize, Deserialize, Debug)]
pub struct QuickActionConfig {
    /// Key that switches to the next set. Empty means unbound.
    #[serde(default)]
    pub next_set_key: String,
    /// Key that switches to the previous set. Empty means unbound.
    #[serde(default)]
    pub prev_set_key: String,
    /// All action sets.
    pub sets: Vec<ActionSet>,
}

impl Default for QuickActionConfig {
    fn default() -> Self {
        Self {
            next_set_key: "Tab".into(),
            prev_set_key: "Q".into(),
            sets: vec![
                ActionSet {
                    name: "Set 1".into(),
                    opacity: 1.0,
                    input_override: false,
                    entries: vec![
                        SetEntry::Action(QuickAction {
                            name: "Quick Attack".into(),
                            key: "X".into(),
                            command: "attack".into(),
                            ..default()
                        }),
                        SetEntry::Action(QuickAction {
                            name: "Heal".into(),
                            key: "B".into(),
                            icon: "✚".into(),
                            command: "heal".into(),
                            ..default()
                        }),
                        SetEntry::Action(QuickAction { name: "Action 3".into(), ..default() }),
                        SetEntry::Wheel(Wheel::new("Wheel 1", 6)),
                    ],
                },
                ActionSet {
                    name: "Set 2".into(),
                    opacity: 1.0,
                    input_override: false,
                    entries: vec![
                        SetEntry::Action(QuickAction {
                            name: "Quick Attack".into(),
                            key: "X".into(),
                            command: "attack".into(),
                            ..default()
                        }),
                        SetEntry::Action(QuickAction {
                            name: "Heal".into(),
                            key: "B".into(),
                            icon: "✚".into(),
                            command: "heal".into(),
                            ..default()
                        }),
                        SetEntry::WheelSet(WheelSet {
                            name: "Wheel Set 1".into(),
                            wheels: vec![
                                Wheel::new("Wheel 1", 6),
                                Wheel::new("Wheel 2", 6),
                                Wheel::new("Wheel 3", 6),
                            ],
                        }),
                    ],
                },
            ],
        }
    }
}

// ─── selection & runtime state ──────────────────────────────────────────────────

/// What the canvas is currently focused on.
#[derive(Clone, Copy, PartialEq, Debug, Default)]
pub enum Selection {
    /// Nothing selected — the canvas shows its placeholder.
    #[default]
    None,
    /// A quick action, addressed by set + entry index.
    Action { set: usize, entry: usize },
    /// A wheel. `wheel` is `None` for a standalone [`SetEntry::Wheel`] or
    /// `Some(i)` for the `i`-th wheel of a [`SetEntry::WheelSet`].
    Wheel { set: usize, entry: usize, wheel: Option<usize> },
    /// An action set's own settings submenu (name / opacity / input override).
    Set { set: usize },
    /// The set-switching shortcut submenu pinned to the top of the tree.
    SetSwitch,
}

/// Which inline text/key field is currently capturing input.
#[derive(Clone, Copy, PartialEq, Debug, Default)]
pub enum EditFocus {
    /// Nothing is capturing input.
    #[default]
    None,
    /// The selected action's name is being typed.
    Name,
    /// The next key press is captured as the selected action's binding.
    Key,
    /// The selected set's name is being typed.
    SetName,
    /// The selected wheel's name is being typed.
    WheelName,
    /// The selected wheel's slot label at this index is being typed.
    SlotName(usize),
    /// The next key press is captured as the "next set" switch shortcut.
    NextSetKey,
    /// The next key press is captured as the "previous set" switch shortcut.
    PrevSetKey,
}

/// Transient UI state. `dirty` triggers a sidebar + canvas rebuild.
#[derive(Resource)]
pub struct EditorUiState {
    /// When `true`, the sidebar and canvas are rebuilt next frame.
    pub dirty: bool,
    /// Current canvas focus.
    pub selection: Selection,
    /// Which inline field (if any) is capturing keyboard input.
    pub editing: EditFocus,
    /// Path used by Save / Load.
    pub config_path: String,
}

impl Default for EditorUiState {
    fn default() -> Self {
        Self {
            dirty: true,
            selection: Selection::None,
            editing: EditFocus::None,
            config_path: "quickactions_config.ron".into(),
        }
    }
}

/// Marker for the root of the sidebar UI (despawned on rebuild).
#[derive(Component)]
pub struct EditorRoot;

/// Marker for the root of the canvas / preview (despawned on rebuild).
#[derive(Component)]
pub struct EditorCanvasRoot;

/// A clickable editor control.
#[derive(Component, Clone)]
pub struct EditorButton {
    /// What pressing it does.
    pub action: EditorAction,
    /// Idle background color, restored when not hovered.
    pub base: Color,
}

/// Every mutation the editor UI can request. Handled by [`handle_editor_buttons`].
#[derive(Clone, Debug)]
pub enum EditorAction {
    /// Append a new, empty action set.
    AddSet,
    /// Append a quick action to a set.
    AddAction { set: usize },
    /// Append a standalone wheel to a set.
    AddWheel { set: usize },
    /// Append a wheel set (seeded with one wheel) to a set.
    AddWheelSet { set: usize },
    /// Append a wheel to the wheel set at `(set, entry)`.
    AddWheelToSet { set: usize, entry: usize },
    /// Select a quick action.
    SelectAction { set: usize, entry: usize },
    /// Select a wheel (`wheel` indexes into a wheel set, or `None` for a
    /// standalone wheel).
    SelectWheel { set: usize, entry: usize, wheel: Option<usize> },
    /// Focus the selected action's name field for typing.
    EditName { set: usize, entry: usize },
    /// Capture the next key press as the action's binding.
    CaptureKey { set: usize, entry: usize },
    /// Cycle the action's icon glyph.
    CycleIcon { set: usize, entry: usize },
    /// Cycle the action's assigned command.
    CycleCommand { set: usize, entry: usize },
    /// Toggle hold-to-activate.
    ToggleHold { set: usize, entry: usize },
    /// Toggle show-on-menu.
    ToggleShowOnMenu { set: usize, entry: usize },
    /// Change the action opacity.
    OpacityDelta { set: usize, entry: usize, delta: f32 },
    /// Change the action radius.
    RadiusDelta { set: usize, entry: usize, delta: f32 },
    /// Cycle the placement reference.
    CyclePosition { set: usize, entry: usize },
    /// Cycle the button shape.
    CycleShape { set: usize, entry: usize },
    /// Select an action set's settings submenu.
    SelectSet { set: usize },
    /// Focus the selected set's name field for typing.
    EditSetName { set: usize },
    /// Change a set's opacity.
    SetOpacityDelta { set: usize, delta: f32 },
    /// Toggle a set's input-override flag.
    ToggleInputOverride { set: usize },
    /// Open the top-of-tree set-switching submenu.
    SelectSetSwitch,
    /// Capture the next key press as the "next set" shortcut.
    CaptureNextSetKey,
    /// Capture the next key press as the "previous set" shortcut.
    CapturePrevSetKey,
    /// Focus the selected wheel's name field for typing.
    EditWheelName,
    /// Change the selected wheel's cooldown.
    WheelCooldownDelta { delta: f32 },
    /// Append a slot to the selected wheel.
    AddSlot,
    /// Remove the last slot from the selected wheel.
    RemoveSlot,
    /// Focus the selected wheel's slot label at `slot` for typing.
    EditSlotName { slot: usize },
    /// Delete an action set.
    DeleteSet { set: usize },
    /// Delete an entry from a set.
    DeleteEntry { set: usize, entry: usize },
    /// Remove a wheel from a wheel-set entry.
    DeleteWheelFromSet { set: usize, entry: usize, wheel: usize },
    /// Move an entry one position earlier in its set.
    MoveEntryUp { set: usize, entry: usize },
    /// Move an entry one position later in its set.
    MoveEntryDown { set: usize, entry: usize },
    /// Save the document to disk.
    Save,
    /// Load the document from disk.
    Load,
}

// ─── plugin ──────────────────────────────────────────────────────────────────────

/// Plugin that adds the in-app Quick Action Menu editor.
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
                    editor_text_input,
                    editor_button_feedback,
                    rebuild_editor,
                )
                    .chain(),
            );
    }
}

// ─── palette ────────────────────────────────────────────────────────────────────

const BG_SIDEBAR: Color = Color::srgb(0.043, 0.055, 0.075);
const BG_MAIN: Color = Color::srgb(0.055, 0.067, 0.086);
const BG_SETSBAR: Color = Color::srgb(0.063, 0.078, 0.10);
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

// ─── bsn! visual builders ────────────────────────────────────────────────────────

/// A single line of styled text.
fn text(s: &str, size: f32, color: Color) -> impl Scene {
    bsn! {
        Text({s.to_string()})
        TextFont { font_size: {FontSize::Px(size)} }
        TextColor({color})
    }
}

/// A horizontal cluster that vertically centers its children.
fn hcluster() -> impl Scene {
    bsn! {
        Node {
            flex_direction: FlexDirection::Row,
            align_items: AlignItems::Center,
            column_gap: {px(7.)},
        }
    }
}

/// A clickable tree-row container with `space-between` layout.
fn row_button(bg: Color) -> impl Scene {
    bsn! {
        Node {
            width: {percent(100.)},
            height: {px(24.)},
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

/// The bordered box of a key-binding badge (its text is added as a child).
fn key_badge_box() -> impl Scene {
    bsn! {
        Node {
            width: {px(18.)},
            height: {px(16.)},
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            border: {UiRect::all(px(1.))},
            border_radius: {BorderRadius::all(px(3.))},
        }
        BorderColor::all(BADGE_BORDER)
    }
}

/// A full-width "add" affordance (solid accent border stands in for dashed).
fn add_button(icon: &str, icon_color: Color, label: &str, accent: Color) -> impl Scene {
    bsn! {
        Node {
            width: {percent(100.)},
            height: {px(26.)},
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

/// A clickable `▾ Set N … ⚙` group header row.
fn set_header_row(bg: Color) -> impl Scene {
    bsn! {
        Node {
            width: {percent(100.)},
            height: {px(24.)},
            margin: {UiRect::top(px(6.))},
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

/// A column that holds a group's children.
fn col() -> impl Scene {
    bsn! {
        Node { flex_direction: FlexDirection::Column, row_gap: {px(1.)} }
    }
}

/// A column indented under a group header.
fn indent_col() -> impl Scene {
    bsn! {
        Node {
            flex_direction: FlexDirection::Column,
            padding: {UiRect::left(px(14.))},
            row_gap: {px(1.)},
        }
    }
}

/// The fixed-width sidebar shell.
fn sidebar() -> impl Scene {
    bsn! {
        Node {
            width: {px(258.)},
            height: {percent(100.)},
            flex_direction: FlexDirection::Column,
            border: {UiRect::right(px(1.))},
        }
        BackgroundColor({BG_SIDEBAR})
        BorderColor::all(SIDEBAR_BORDER)
    }
}

/// The title block.
fn header() -> impl Scene {
    bsn! {
        Node {
            flex_direction: FlexDirection::Row,
            justify_content: JustifyContent::SpaceBetween,
            align_items: AlignItems::FlexStart,
            padding: {UiRect::all(px(16.))},
        }
        Children [
            (
                Node { flex_direction: FlexDirection::Column, row_gap: {px(2.)} }
                Children [
                    text("QUICK ACTION", 13., GREEN),
                    text("MENU EDITOR", 10., DIM),
                ]
            ),
            text("⚙", 12., DIM),
        ]
    }
}

/// The `SETS … + Set` bar (its `+ Set` button is made clickable imperatively).
fn sets_bar() -> impl Scene {
    bsn! {
        Node {
            flex_direction: FlexDirection::Row,
            justify_content: JustifyContent::SpaceBetween,
            align_items: AlignItems::Center,
            padding: {UiRect::axes(px(16.), px(8.))},
        }
        BackgroundColor({BG_SETSBAR})
    }
}

/// The `+ Set` pill button.
fn set_pill() -> impl Scene {
    bsn! {
        Node {
            padding: {UiRect::axes(px(8.), px(3.))},
            border: {UiRect::all(px(1.))},
            border_radius: {BorderRadius::all(px(4.))},
        }
        BorderColor::all(GREEN)
        BackgroundColor({GREEN_BG})
        Button
        Children [ text("+ Set", 10., GREEN) ]
    }
}

/// The scrollable tree column.
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

/// A footer action button (`SAVE` / `LOAD`).
fn footer_button(label: &str, accent: Color, filled: bool) -> impl Scene {
    let (bg, border) = if filled {
        (GREEN_BG, Color::NONE)
    } else {
        (Color::NONE, BADGE_BORDER)
    };
    bsn! {
        Node {
            flex_grow: 1.,
            height: {px(30.)},
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            border: {UiRect::all(px(1.))},
            border_radius: {BorderRadius::all(px(4.))},
        }
        BorderColor::all(border)
        BackgroundColor({bg})
        Button
        Children [ text(label, 11., accent) ]
    }
}

// ─── imperative spawn helpers ────────────────────────────────────────────────────

/// Spawn `scene` as a child of `parent`, returning the new entity.
fn child(commands: &mut Commands, parent: Entity, scene: impl Scene) -> Entity {
    let e = commands.spawn_scene(scene).id();
    commands.entity(parent).add_child(e);
    e
}

/// Spawn a clickable `scene` (carrying an [`EditorButton`]) under `parent`.
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

/// A small × delete button.
fn del_btn() -> impl Scene {
    bsn! {
        Node {
            width: {px(15.)},
            height: {px(15.)},
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            border_radius: {BorderRadius::all(px(3.))},
            margin: {UiRect::left(px(2.))},
        }
        Button
    }
}

/// What trails a tree row on the right.
enum Badge {
    /// No trailing badge.
    None,
    /// A bordered key-binding badge.
    Key(String),
    /// Dim text (cooldown / count).
    Dim(String),
}

/// Spawn a clickable entry row with an icon, label, optional trailing badge and optional delete button.
#[allow(clippy::too_many_arguments)]
fn spawn_entry_row(
    commands: &mut Commands,
    parent: Entity,
    selected: bool,
    action: EditorAction,
    icon: &str,
    icon_color: Color,
    label: &str,
    label_color: Color,
    badge: Badge,
    delete: Option<EditorAction>,
) {
    let base = if selected { ROW_SEL } else { Color::NONE };
    let row = clickable(commands, parent, row_button(base), action, base);
    let cluster = child(commands, row, hcluster());
    child(commands, cluster, text(icon, 10., icon_color));
    child(commands, cluster, text(label, 11., label_color));
    // Right cluster: badge + optional delete button grouped together.
    let right = child(commands, row, hcluster());
    match badge {
        Badge::None => {}
        Badge::Key(k) => {
            let box_e = child(commands, right, key_badge_box());
            child(commands, box_e, text(&k, 9., DIM));
        }
        Badge::Dim(s) => {
            child(commands, right, text(&s, 9., DIM));
        }
    }
    if let Some(del) = delete {
        let d = clickable(commands, right, del_btn(), del, Color::NONE);
        child(commands, d, text("×", 10., DIMMER));
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

    build_sidebar(&mut commands, &cfg, &ui);

    let win_w = windows.iter().next().map(|w| w.width()).unwrap_or(1229.0);
    build_canvas(&mut commands, &cfg, &ui, win_w);
}

/// Builds the whole sidebar tree from the document.
fn build_sidebar(commands: &mut Commands, cfg: &QuickActionConfig, ui: &EditorUiState) {
    let root = commands.spawn_scene(sidebar()).insert(EditorRoot).id();

    child(commands, root, header());

    // SETS bar with its + Set pill.
    let bar = child(commands, root, sets_bar());
    let left = child(commands, bar, hcluster());
    child(commands, left, text("≣", 11., DIM));
    child(commands, left, text("SETS", 11., DIM));
    clickable(commands, bar, set_pill(), EditorAction::AddSet, GREEN_BG);

    // Scrollable tree.
    let tree = child(commands, root, tree());
    build_set_switch(commands, tree, ui, cfg);
    for (si, set) in cfg.sets.iter().enumerate() {
        build_set(commands, tree, ui, si, set);
    }

    // Footer.
    build_footer(commands, root, &ui.config_path);
}

/// Builds one action-set group.
fn build_set(
    commands: &mut Commands,
    parent: Entity,
    ui: &EditorUiState,
    si: usize,
    set: &ActionSet,
) {
    let group = child(commands, parent, col());

    // Clickable header that opens the set's settings submenu.
    let set_selected = ui.selection == Selection::Set { set: si };
    let hb = if set_selected { ROW_SEL } else { Color::NONE };
    let header = clickable(
        commands, group, set_header_row(hb),
        EditorAction::SelectSet { set: si }, hb,
    );
    let hc = child(commands, header, hcluster());
    child(commands, hc, text("▾", 9., DIM));
    child(commands, hc, text(&set.name, 11., TEXT));
    let right = child(commands, header, hcluster());
    if set.opacity < 0.999 {
        child(commands, right, text(&format!("{:.0}%", set.opacity * 100.0), 9., DIM));
    }
    if set.input_override {
        child(commands, right, text("⊘", 9., AMBER));
    }
    // ⚙ gear opens settings; × deletes the set.
    child(commands, right, text("⚙", 10., DIM));
    let dx = clickable(commands, right, del_btn(), EditorAction::DeleteSet { set: si }, Color::NONE);
    child(commands, dx, text("×", 10., DIMMER));

    let body = child(commands, group, indent_col());

    if set_selected {
        spawn_set_editor(commands, body, ui, si, set);
    }

    let n_entries = set.entries.len();
    for (ei, entry) in set.entries.iter().enumerate() {
        match entry {
            SetEntry::Action(qa) => {
                let selected = ui.selection == (Selection::Action { set: si, entry: ei });
                let badge = if qa.key.is_empty() {
                    Badge::None
                } else {
                    Badge::Key(qa.key.clone())
                };
                spawn_entry_row(
                    commands, body, selected,
                    EditorAction::SelectAction { set: si, entry: ei },
                    &qa.icon, ICON, &qa.name, TEXT, badge,
                    Some(EditorAction::DeleteEntry { set: si, entry: ei }),
                );
                if selected {
                    spawn_action_editor(commands, body, ui, si, ei, qa);
                }
            }
            SetEntry::Wheel(w) => {
                let selected =
                    ui.selection == (Selection::Wheel { set: si, entry: ei, wheel: None });
                spawn_entry_row(
                    commands, body, selected,
                    EditorAction::SelectWheel { set: si, entry: ei, wheel: None },
                    "○", ICON, &w.name, TEXT,
                    Badge::Dim(format!("{:.0}s", w.cooldown_secs)),
                    Some(EditorAction::DeleteEntry { set: si, entry: ei }),
                );
                if selected {
                    spawn_wheel_editor(commands, body, ui, w);
                }
            }
            SetEntry::WheelSet(ws) => {
                build_wheel_set(commands, body, ui, si, ei, ws);
            }
        }
        // ↑↓ reorder buttons (shown between add-buttons and entries, not on first/last).
        let _ = n_entries; // suppress unused warning
    }

    // Per-set "add" affordances.
    clickable(
        commands, body, add_button("◆", AMBER, "Quick Action", AMBER),
        EditorAction::AddAction { set: si }, Color::srgba(1., 1., 1., 0.015),
    );
    clickable(
        commands, body, add_button("○", BLUE, "Wheel", BLUE),
        EditorAction::AddWheel { set: si }, Color::srgba(1., 1., 1., 0.015),
    );
    clickable(
        commands, body, add_button("◈", BLUE, "Wheel Set", BLUE),
        EditorAction::AddWheelSet { set: si }, Color::srgba(1., 1., 1., 0.015),
    );
}

/// Builds a nested wheel-set group (header + wheel rows + add link).
fn build_wheel_set(
    commands: &mut Commands,
    parent: Entity,
    ui: &EditorUiState,
    si: usize,
    ei: usize,
    ws: &WheelSet,
) {
    let group = child(commands, parent, col());

    // Header row: `▾ ◈ Name … Nw`.
    let header = child(commands, group, row_button(Color::NONE));
    let cluster = child(commands, header, hcluster());
    child(commands, cluster, text("▾", 9., DIM));
    child(commands, cluster, text("◈", 10., BLUE));
    child(commands, cluster, text(&ws.name, 11., TEXT));
    child(commands, header, text(&format!("{}w", ws.wheels.len()), 9., DIM));

    let body = child(commands, group, indent_col());
    for (wi, w) in ws.wheels.iter().enumerate() {
        let selected =
            ui.selection == (Selection::Wheel { set: si, entry: ei, wheel: Some(wi) });
        spawn_entry_row(
            commands, body, selected,
            EditorAction::SelectWheel { set: si, entry: ei, wheel: Some(wi) },
            "○", ICON, &w.name, TEAL,
            Badge::Dim(format!("{:.0}s", w.cooldown_secs)),
            Some(EditorAction::DeleteWheelFromSet { set: si, entry: ei, wheel: wi }),
        );
        if selected {
            spawn_wheel_editor(commands, body, ui, w);
        }
    }
    // `+ add wheel` link.
    let link = clickable(
        commands, body, row_button(Color::NONE),
        EditorAction::AddWheelToSet { set: si, entry: ei }, Color::NONE,
    );
    child(commands, link, text("+ add wheel", 10., DIM));
}

/// Builds the pinned save/load footer.
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
        commands, footer,
        bsn! { Node { flex_direction: FlexDirection::Row, column_gap: {px(8.)} } },
    );
    clickable(commands, row, footer_button("SAVE", GREEN, true), EditorAction::Save, GREEN_BG);
    clickable(commands, row, footer_button("LOAD", DIM, false), EditorAction::Load, Color::NONE);

    let cap = child(
        commands, footer,
        bsn! { Node { justify_content: JustifyContent::Center } },
    );
    child(commands, cap, text(path, 9., DIMMER));
}

// ─── canvas / preview ────────────────────────────────────────────────────────────

/// Builds the right-hand canvas.
///
/// When a wheel is selected the canvas shows a full-screen radial preview of
/// that wheel. All other times it shows the live visual overview — action
/// buttons rendered at their configured shape/size, and every wheel as a
/// compact radial render — so any sidebar edit is immediately visible.
fn build_canvas(commands: &mut Commands, cfg: &QuickActionConfig, ui: &EditorUiState, win_w: f32) {
    if let Selection::Wheel { set, entry, wheel } = ui.selection {
        let w = cfg.sets.get(set).and_then(|s| s.entries.get(entry)).and_then(|e| match (e, wheel) {
            (SetEntry::Wheel(w), None) => Some(w),
            (SetEntry::WheelSet(ws), Some(i)) => ws.wheels.get(i),
            _ => None,
        });
        if let Some(w) = w {
            build_wheel_preview(commands, w, win_w);
            return;
        }
    }
    build_overview(commands, cfg);
}

/// A full-screen canvas root anchored to the right of the sidebar.
fn canvas_root() -> impl Scene {
    bsn! {
        Node {
            position_type: PositionType::Absolute,
            left: {px(258.)},
            top: {px(0.)},
            right: {px(0.)},
            bottom: {px(0.)},
            flex_direction: FlexDirection::Column,
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            row_gap: {px(10.)},
        }
        BackgroundColor({BG_MAIN})
    }
}

/// The scrollable overview root (top-left anchored).
fn overview_root() -> impl Scene {
    bsn! {
        Node {
            position_type: PositionType::Absolute,
            left: {px(258.)},
            top: {px(0.)},
            right: {px(0.)},
            bottom: {px(0.)},
            flex_direction: FlexDirection::Column,
            row_gap: {px(14.)},
            padding: {UiRect::all(px(24.))},
            overflow: {Overflow::scroll_y()},
        }
        BackgroundColor({BG_MAIN})
    }
}

/// Returns `key` or an em dash when unbound.
fn label_or(key: &str) -> String {
    if key.is_empty() { "—".into() } else { key.into() }
}

/// A color with its alpha multiplied.
fn with_alpha(c: Color, a: f32) -> Color {
    c.with_alpha(a)
}

/// Slice background color used in the canvas wheel previews.
const CANVAS_SLICE_BG: Color = Color::srgb(0.15, 0.20, 0.32);

/// Renders a quick action as a styled button widget on the canvas.
fn spawn_canvas_action(commands: &mut Commands, parent: Entity, set_opacity: f32, qa: &QuickAction) {
    let eff    = (set_opacity * qa.opacity).clamp(0.05, 1.0);
    let sz     = (qa.radius * 0.75).clamp(36., 72.);
    let corner = match qa.shape {
        ActionShape::Round   => sz / 2.0,
        ActionShape::Rounded => sz * 0.25,
        ActionShape::Square  => 3.0,
        ActionShape::Diamond => 3.0,  // diamond uses Transform rotation below
    };
    let bg      = with_alpha(Color::srgb(0.14, 0.20, 0.33), eff);
    let bord    = with_alpha(BADGE_BORDER, (eff * 1.4).min(1.0));
    let icon_c  = with_alpha(AMBER, eff);
    let text_c  = with_alpha(if qa.show_on_menu { TEXT } else { DIMMER }, (eff * 0.8).min(1.0));
    let icon_sz = (sz * 0.36).max(10.);
    let is_diamond = qa.shape == ActionShape::Diamond;

    let col = child(commands, parent, bsn! {
        Node {
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Center,
            row_gap: {px(4.)},
        }
    });

    if is_diamond {
        // Diamond: fixed-size container, rotated bg square, unrotated icon overlay.
        let inner_sz = sz * 0.72;  // side length of rotated square; fits in sz when rotated 45°.
        let container = child(commands, col, bsn! {
            Node {
                width: {px(sz)},
                height: {px(sz)},
            }
        });
        // Rotated background square (absolute, centered in container).
        let bg_node = child(commands, container, bsn! {
            Node {
                position_type: PositionType::Absolute,
                left: {px((sz - inner_sz) / 2.)},
                top: {px((sz - inner_sz) / 2.)},
                width: {px(inner_sz)},
                height: {px(inner_sz)},
                border: {UiRect::all(px(1.))},
                border_radius: {BorderRadius::all(px(3.))},
            }
            BackgroundColor({bg})
            BorderColor::all(bord)
        });
        commands.entity(bg_node).insert(
            Transform::from_rotation(Quat::from_rotation_z(std::f32::consts::FRAC_PI_4))
        );
        // Unrotated icon overlay (absolute, covers full container, centered).
        let overlay = child(commands, container, bsn! {
            Node {
                position_type: PositionType::Absolute,
                left: {px(0.)},
                top: {px(0.)},
                width: {px(sz)},
                height: {px(sz)},
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
            }
        });
        child(commands, overlay, text(&qa.icon, icon_sz, icon_c));
    } else {
        let btn = child(commands, col, bsn! {
            Node {
                width: {px(sz)},
                height: {px(sz)},
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                border: {UiRect::all(px(1.))},
                border_radius: {BorderRadius::all(px(corner))},
            }
            BackgroundColor({bg})
            BorderColor::all(bord)
        });
        child(commands, btn, text(&qa.icon, icon_sz, icon_c));
    }
    if !qa.key.is_empty() {
        let kb = child(commands, col, key_badge_box());
        child(commands, kb, text(&qa.key, 8., DIM));
    }
    child(commands, col, text(&qa.name, 8., text_c));
}

/// Renders a wheel as a compact live radial preview on the canvas.
fn spawn_canvas_wheel(commands: &mut Commands, parent: Entity, w: &Wheel) {
    let radius = 88.0_f32;
    let inner  = 26.0_f32;
    let n = w.slots.len().max(1);
    let menu = WheelMenu {
        slices: n,
        radius,
        inner_radius: inner,
        deadzone: 0.3,
        gap: 0.03,
        arc_span: std::f32::consts::TAU,
        arc_offset: 0.0,
        overlap: false,
    };
    let container_size = radius * 2.2;
    let slice_sz = (radius - inner) * 0.82;
    let corner   = slice_sz * 0.2;
    let disc_r   = (inner - 4.).max(6.);

    let col = child(commands, parent, bsn! {
        Node {
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Center,
            row_gap: {px(4.)},
        }
    });
    let container = child(commands, col, bsn! {
        Node {
            width: {px(container_size)},
            height: {px(container_size)},
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
        }
    });
    let hub = child(commands, container, wheel_hub());
    let disc = child(commands, hub, wheel_center_disc(disc_r, PANEL_CARD));
    child(commands, disc, wheel_slice_label(w.name.clone(), (disc_r * 0.38).max(7.), DIM));
    for (i, slot) in w.slots.iter().enumerate() {
        if i >= n { break; }
        let panel = child(
            commands, hub,
            wheel_slice_panel_styled(&menu, i, slice_sz, CANVAS_SLICE_BG, corner),
        );
        child(commands, panel, wheel_slice_label(slot.clone(), 9., TEXT));
    }
    child(commands, col, text(&format!("{:.1}s", w.cooldown_secs), 9., DIM));
}

/// Builds the live visual overview: action buttons and radial wheel previews.
fn build_overview(commands: &mut Commands, cfg: &QuickActionConfig) {
    let root = commands.spawn_scene(overview_root()).insert(EditorCanvasRoot).id();

    // Title + set-switch hint.
    let head = child(
        commands, root,
        bsn! { Node { flex_direction: FlexDirection::Column, row_gap: {px(2.)} } },
    );
    child(commands, head, text("MENU PREVIEW", 13., GREEN));
    child(
        commands, head,
        text(
            &format!(
                "switch sets   ·   next {}   ·   prev {}",
                label_or(&cfg.next_set_key),
                label_or(&cfg.prev_set_key),
            ),
            10., DIM,
        ),
    );

    if cfg.sets.is_empty() {
        child(commands, root, text("No sets configured — add one in the sidebar.", 12., DIMMER));
        return;
    }

    for set in &cfg.sets {
        // Per-set section card.
        let section = child(commands, root, bsn! {
            Node {
                flex_direction: FlexDirection::Column,
                row_gap: {px(10.)},
                padding: {UiRect::all(px(12.))},
                border: {UiRect::all(px(1.))},
                border_radius: {BorderRadius::all(px(8.))},
            }
            BorderColor::all(SIDEBAR_BORDER)
            BackgroundColor({PANEL_CARD})
        });

        // Section header row: name + opacity / override tags.
        let hrow = child(commands, section, bsn! {
            Node {
                flex_direction: FlexDirection::Row,
                justify_content: JustifyContent::SpaceBetween,
                align_items: AlignItems::Center,
            }
        });
        child(commands, hrow, text(&set.name, 11., TEXT));
        let tags = child(commands, hrow, hcluster());
        if set.opacity < 0.999 {
            child(commands, tags, text(&format!("{:.0}%", set.opacity * 100.0), 9., DIM));
        }
        if set.input_override {
            child(commands, tags, text("⊘ override", 9., AMBER));
        }

        if set.entries.is_empty() {
            child(commands, section, text("empty set", 10., DIMMER));
            continue;
        }

        // Wrapping content row: buttons and wheels rendered side-by-side.
        let content = child(commands, section, bsn! {
            Node {
                flex_direction: FlexDirection::Row,
                flex_wrap: FlexWrap::Wrap,
                column_gap: {px(10.)},
                row_gap: {px(10.)},
                align_items: AlignItems::FlexEnd,
            }
        });

        for entry in &set.entries {
            match entry {
                SetEntry::Action(qa) => spawn_canvas_action(commands, content, set.opacity, qa),
                SetEntry::Wheel(w)   => spawn_canvas_wheel(commands, content, w),
                SetEntry::WheelSet(ws) => {
                    let grp = child(commands, content, bsn! {
                        Node {
                            flex_direction: FlexDirection::Column,
                            row_gap: {px(6.)},
                            padding: {UiRect::all(px(8.))},
                            border: {UiRect::all(px(1.))},
                            border_radius: {BorderRadius::all(px(6.))},
                        }
                        BorderColor::all(BADGE_BORDER)
                    });
                    let gh = child(commands, grp, hcluster());
                    child(commands, gh, text("◈", 10., BLUE));
                    child(commands, gh, text(&ws.name, 10., TEXT));
                    let wrow = child(commands, grp, bsn! {
                        Node {
                            flex_direction: FlexDirection::Row,
                            flex_wrap: FlexWrap::Wrap,
                            column_gap: {px(8.)},
                            row_gap: {px(8.)},
                            align_items: AlignItems::FlexEnd,
                        }
                    });
                    for w in &ws.wheels {
                        spawn_canvas_wheel(commands, wrow, w);
                    }
                }
            }
        }
    }
}


/// A live radial preview of the selected wheel, centered in the canvas.
fn build_wheel_preview(commands: &mut Commands, wheel: &Wheel, _win_w: f32) {
    let root = commands.spawn_scene(canvas_root()).insert(EditorCanvasRoot).id();

    // Hub is the radial origin; absolutely-positioned slices hang off it.
    let hub = child(commands, root, wheel_hub());
    let menu = wheel.to_menu();

    let disc = (menu.inner_radius - 6.0).max(1.0);
    let center = child(commands, hub, wheel_center_disc(disc, PANEL_CARD));
    child(commands, center, wheel_slice_label(wheel.name.clone(), 13., TEXT));

    let size = (menu.radius - menu.inner_radius) * 0.78;
    let corner = size * 0.18;
    for (i, slot) in wheel.slots.iter().enumerate() {
        if i >= menu.slices {
            break;
        }
        let panel = child(
            commands, hub,
            wheel_slice_panel_styled(&menu, i, size, Color::srgb(0.16, 0.20, 0.30), corner),
        );
        child(commands, panel, wheel_slice_label(slot.clone(), 12., TEXT));
    }

    // Caption under the wheel.
    let caption = child(
        commands, root,
        bsn! {
            Node {
                position_type: PositionType::Absolute,
                bottom: {px(40.)},
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                row_gap: {px(4.)},
            }
        },
    );
    child(commands, caption, text(&wheel.name, 14., TEXT));
    child(
        commands, caption,
        text(&format!("{} slots · {:.0}s cooldown", wheel.slots.len(), wheel.cooldown_secs), 11., DIM),
    );
}

// ─── interaction ─────────────────────────────────────────────────────────────────

/// Hover color feedback for clickable controls.
fn editor_button_feedback(
    mut q: Query<(&Interaction, &EditorButton, &mut BackgroundColor), Changed<Interaction>>,
) {
    for (interaction, button, mut bg) in &mut q {
        bg.0 = match *interaction {
            Interaction::Hovered | Interaction::Pressed => {
                if button.base == ROW_SEL { ROW_SEL } else { ROW_HOVER }
            }
            Interaction::None => button.base,
        };
    }
}

/// Applies the action of a pressed control to the document / selection.
fn handle_editor_buttons(
    mut cfg: ResMut<QuickActionConfig>,
    mut ui: ResMut<EditorUiState>,
    q: Query<(&Interaction, &EditorButton), Changed<Interaction>>,
) {
    for (interaction, button) in &q {
        if *interaction != Interaction::Pressed {
            continue;
        }
        apply_action(&button.action, &mut cfg, &mut ui);
        ui.dirty = true;
    }
}

fn apply_action(action: &EditorAction, cfg: &mut QuickActionConfig, ui: &mut EditorUiState) {
    match *action {
        EditorAction::AddSet => {
            let n = cfg.sets.len() + 1;
            cfg.sets.push(ActionSet {
                name: format!("Set {}", n),
                opacity: 1.0,
                input_override: false,
                entries: Vec::new(),
            });
        }
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
        EditorAction::SelectAction { set, entry } => {
            ui.selection = Selection::Action { set, entry };
            ui.editing = EditFocus::None;
        }
        EditorAction::SelectWheel { set, entry, wheel } => {
            ui.selection = Selection::Wheel { set, entry, wheel };
            ui.editing = EditFocus::None;
        }
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
        EditorAction::SelectSetSwitch => {
            ui.selection = Selection::SetSwitch;
            ui.editing = EditFocus::None;
        }
        EditorAction::CaptureNextSetKey => {
            ui.selection = Selection::SetSwitch;
            ui.editing = EditFocus::NextSetKey;
        }
        EditorAction::CapturePrevSetKey => {
            ui.selection = Selection::SetSwitch;
            ui.editing = EditFocus::PrevSetKey;
        }
        EditorAction::EditWheelName => {
            ui.editing = EditFocus::WheelName;
        }
        EditorAction::WheelCooldownDelta { delta } => {
            if let Some(w) = wheel_at(cfg, ui.selection) {
                w.cooldown_secs = (w.cooldown_secs + delta).clamp(0.0, 60.0);
            }
        }
        EditorAction::AddSlot => {
            if let Some(w) = wheel_at(cfg, ui.selection) {
                let n = w.slots.len() + 1;
                w.slots.push(format!("Slot {}", n));
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
        EditorAction::DeleteSet { set } => {
            if set < cfg.sets.len() {
                cfg.sets.remove(set);
            }
            let clear = match ui.selection {
                Selection::Action { set: s, .. } => s == set,
                Selection::Wheel { set: s, .. } => s == set,
                Selection::Set { set: s } => s == set,
                _ => false,
            };
            if clear {
                ui.selection = Selection::None;
                ui.editing = EditFocus::None;
            }
        }
        EditorAction::DeleteEntry { set, entry } => {
            if let Some(s) = cfg.sets.get_mut(set) {
                if entry < s.entries.len() {
                    s.entries.remove(entry);
                }
            }
            let clear = match ui.selection {
                Selection::Action { set: s, entry: e } => s == set && e == entry,
                Selection::Wheel { set: s, entry: e, .. } => s == set && e == entry,
                _ => false,
            };
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
            let clear = matches!(
                ui.selection,
                Selection::Wheel { set: s, entry: e, wheel: Some(w) }
                    if s == set && e == entry && w == wheel
            );
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
                    Selection::Action { set: s, entry: e } if s == set && e == entry =>
                        Selection::Action { set, entry: entry - 1 },
                    Selection::Wheel { set: s, entry: e, wheel: w } if s == set && e == entry =>
                        Selection::Wheel { set, entry: entry - 1, wheel: w },
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
                    Selection::Action { set: s, entry: e } if s == set && e == entry =>
                        Selection::Action { set, entry: entry + 1 },
                    Selection::Wheel { set: s, entry: e, wheel: w } if s == set && e == entry =>
                        Selection::Wheel { set, entry: entry + 1, wheel: w },
                    other => other,
                };
            }
        }
        EditorAction::Save => save_config(cfg, &ui.config_path),
        EditorAction::Load => {
            if let Some(loaded) = load_config(&ui.config_path) {
                *cfg = loaded;
                ui.selection = Selection::None;
            }
        }
    }
}

// ─── persistence ─────────────────────────────────────────────────────────────────

fn save_config(cfg: &QuickActionConfig, path: &str) {
    match ron::ser::to_string_pretty(cfg, ron::ser::PrettyConfig::default()) {
        Ok(text) => {
            if let Err(e) = std::fs::write(path, text) {
                warn!("quick action editor: failed to save '{}': {}", path, e);
            } else {
                info!("quick action editor: saved configuration to '{}'", path);
            }
        }
        Err(e) => warn!("quick action editor: failed to serialize configuration: {}", e),
    }
}

fn load_config(path: &str) -> Option<QuickActionConfig> {
    match std::fs::read_to_string(path) {
        Ok(text) => match ron::from_str::<QuickActionConfig>(&text) {
            Ok(cfg) => {
                info!("quick action editor: loaded configuration from '{}'", path);
                Some(cfg)
            }
            Err(e) => {
                warn!("quick action editor: failed to parse '{}': {}", path, e);
                None
            }
        },
        Err(e) => {
            warn!("quick action editor: failed to read '{}': {}", path, e);
            None
        }
    }
}

// ─── inline quick-action editor ──────────────────────────────────────────────────

const CTRL_BG: Color = Color::srgb(0.11, 0.14, 0.19);

/// The card that holds the inline editor fields, indented under its row.
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

/// A labelled field row.
fn field_row() -> impl Scene {
    bsn! {
        Node {
            width: {percent(100.)},
            min_height: {px(22.)},
            flex_direction: FlexDirection::Row,
            align_items: AlignItems::Center,
            column_gap: {px(6.)},
        }
    }
}

/// The fixed-width label cell at the start of a field row.
fn label_cell(s: &str) -> impl Scene {
    bsn! {
        Node { width: {px(70.)}, flex_direction: FlexDirection::Row, align_items: AlignItems::Center }
        Children [ text(s, 10., DIM) ]
    }
}

/// A bordered, clickable value box (grows to fill the row).
fn ctrl_box(accent: Color) -> impl Scene {
    bsn! {
        Node {
            flex_grow: 1.,
            height: {px(20.)},
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

/// A small square button (stepper − / +).
fn mini_box() -> impl Scene {
    bsn! {
        Node {
            width: {px(22.)},
            height: {px(20.)},
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            border_radius: {BorderRadius::all(px(4.))},
        }
        BackgroundColor({CTRL_BG})
        Button
    }
}

/// A centered, growing value cell (between stepper buttons).
fn val_cell() -> impl Scene {
    bsn! {
        Node {
            flex_grow: 1.,
            height: {px(20.)},
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
        }
    }
}

/// A toggle pill.
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

/// Spawn a labelled row and return it for the control to be appended.
fn spawn_field(commands: &mut Commands, parent: Entity, label: &str) -> Entity {
    let row = child(commands, parent, field_row());
    child(commands, row, label_cell(label));
    row
}

/// A clickable value-box field (name, key, icon, command, cycles).
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

/// A boolean toggle field.
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

/// A `− value +` stepper field.
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

/// Builds the inline editor for the selected quick action.
fn spawn_action_editor(
    commands: &mut Commands,
    parent: Entity,
    ui: &EditorUiState,
    set: usize,
    entry: usize,
    qa: &QuickAction,
) {
    let card = child(commands, parent, editor_card());

    // Name (typed).
    let name_focus = ui.editing == EditFocus::Name;
    let name_disp = if name_focus { format!("{}|", qa.name) } else { qa.name.clone() };
    spawn_box_field(
        commands, card, "Name", &name_disp, TEXT,
        if name_focus { AMBER } else { BADGE_BORDER },
        EditorAction::EditName { set, entry },
    );

    // Input (captured key).
    let key_focus = ui.editing == EditFocus::Key;
    let (key_disp, key_col) = if key_focus {
        ("press a key…".to_string(), AMBER)
    } else if qa.key.is_empty() {
        ("unbound".to_string(), DIM)
    } else {
        (qa.key.clone(), TEXT)
    };
    spawn_box_field(
        commands, card, "Input", &key_disp, key_col,
        if key_focus { AMBER } else { BADGE_BORDER },
        EditorAction::CaptureKey { set, entry },
    );

    // Icon / command (cycled).
    spawn_box_field(commands, card, "Icon", &qa.icon, AMBER, BADGE_BORDER, EditorAction::CycleIcon { set, entry });
    spawn_box_field(commands, card, "Command", &qa.command, TEAL, BADGE_BORDER, EditorAction::CycleCommand { set, entry });

    // Options.
    spawn_toggle_field(commands, card, "Hold", qa.hold, EditorAction::ToggleHold { set, entry });
    spawn_toggle_field(commands, card, "On menu", qa.show_on_menu, EditorAction::ToggleShowOnMenu { set, entry });
    spawn_stepper_field(
        commands, card, "Opacity", &format!("{:.0}%", qa.opacity * 100.0),
        EditorAction::OpacityDelta { set, entry, delta: -0.1 },
        EditorAction::OpacityDelta { set, entry, delta: 0.1 },
    );
    spawn_box_field(commands, card, "Position", qa.position.label(), TEXT, BADGE_BORDER, EditorAction::CyclePosition { set, entry });
    spawn_stepper_field(
        commands, card, "Radius", &format!("{:.0}", qa.radius),
        EditorAction::RadiusDelta { set, entry, delta: -4.0 },
        EditorAction::RadiusDelta { set, entry, delta: 4.0 },
    );
    spawn_box_field(commands, card, "Shape", qa.shape.label(), TEXT, BADGE_BORDER, EditorAction::CycleShape { set, entry });
}

/// Display text + color for a (possibly focused) key field.
fn key_display(focus: bool, key: &str) -> (String, Color) {
    if focus {
        ("press a key…".to_string(), AMBER)
    } else if key.is_empty() {
        ("unbound".to_string(), DIM)
    } else {
        (key.to_string(), TEXT)
    }
}

/// Builds the inline settings submenu for a selected action set.
fn spawn_set_editor(
    commands: &mut Commands,
    parent: Entity,
    ui: &EditorUiState,
    si: usize,
    set: &ActionSet,
) {
    let card = child(commands, parent, editor_card());

    // Name (typed).
    let name_focus = ui.editing == EditFocus::SetName;
    let name_disp = if name_focus { format!("{}|", set.name) } else { set.name.clone() };
    spawn_box_field(
        commands, card, "Name", &name_disp, TEXT,
        if name_focus { AMBER } else { BADGE_BORDER },
        EditorAction::EditSetName { set: si },
    );

    // Opacity (stepper).
    spawn_stepper_field(
        commands, card, "Opacity", &format!("{:.0}%", set.opacity * 100.0),
        EditorAction::SetOpacityDelta { set: si, delta: -0.1 },
        EditorAction::SetOpacityDelta { set: si, delta: 0.1 },
    );

    // Input override for children (toggle).
    spawn_toggle_field(
        commands, card, "Override in", set.input_override,
        EditorAction::ToggleInputOverride { set: si },
    );
}

/// Builds the inline config editor for a selected wheel.
fn spawn_wheel_editor(commands: &mut Commands, parent: Entity, ui: &EditorUiState, w: &Wheel) {
    let card = child(commands, parent, editor_card());

    // Name (typed).
    let name_focus = ui.editing == EditFocus::WheelName;
    let name_disp = if name_focus { format!("{}|", w.name) } else { w.name.clone() };
    spawn_box_field(
        commands, card, "Name", &name_disp, TEXT,
        if name_focus { AMBER } else { BADGE_BORDER },
        EditorAction::EditWheelName,
    );

    // Cooldown (stepper).
    spawn_stepper_field(
        commands, card, "Cooldown", &format!("{:.1}s", w.cooldown_secs),
        EditorAction::WheelCooldownDelta { delta: -0.5 },
        EditorAction::WheelCooldownDelta { delta: 0.5 },
    );

    // Slot count (stepper).
    spawn_stepper_field(
        commands, card, "Slots", &format!("{}", w.slots.len()),
        EditorAction::RemoveSlot, EditorAction::AddSlot,
    );

    // Slot labels (typed).
    for (i, slot) in w.slots.iter().enumerate() {
        let focus = ui.editing == EditFocus::SlotName(i);
        let disp = if focus { format!("{}|", slot) } else { slot.clone() };
        spawn_box_field(
            commands, card, &format!("Slot {}", i + 1), &disp, TEAL,
            if focus { AMBER } else { BADGE_BORDER },
            EditorAction::EditSlotName { slot: i },
        );
    }
}

/// Builds the top-of-tree set-switching shortcut submenu.
fn build_set_switch(
    commands: &mut Commands,
    parent: Entity,
    ui: &EditorUiState,
    cfg: &QuickActionConfig,
) {
    let group = child(commands, parent, col());

    let selected = ui.selection == Selection::SetSwitch;
    let hb = if selected { ROW_SEL } else { Color::NONE };
    let header = clickable(
        commands, group, set_header_row(hb), EditorAction::SelectSetSwitch, hb,
    );
    let hc = child(commands, header, hcluster());
    child(commands, hc, text("⇄", 10., TEAL));
    child(commands, hc, text("Set Switching", 11., TEXT));
    child(commands, header, text("⚙", 10., DIM));

    if !selected {
        return;
    }

    let body = child(commands, group, indent_col());
    let card = child(commands, body, editor_card());

    let nf = ui.editing == EditFocus::NextSetKey;
    let (nd, nc) = key_display(nf, &cfg.next_set_key);
    spawn_box_field(
        commands, card, "Next set", &nd, nc,
        if nf { AMBER } else { BADGE_BORDER }, EditorAction::CaptureNextSetKey,
    );

    let pf = ui.editing == EditFocus::PrevSetKey;
    let (pd, pc) = key_display(pf, &cfg.prev_set_key);
    spawn_box_field(
        commands, card, "Prev set", &pd, pc,
        if pf { AMBER } else { BADGE_BORDER }, EditorAction::CapturePrevSetKey,
    );
}

// ─── keyboard input ──────────────────────────────────────────────────────────────

/// Borrow the [`QuickAction`] at `(set, entry)`, if that entry is an action.
fn action_at<'a>(cfg: &'a mut QuickActionConfig, set: usize, entry: usize) -> Option<&'a mut QuickAction> {
    match cfg.sets.get_mut(set).and_then(|s| s.entries.get_mut(entry)) {
        Some(SetEntry::Action(a)) => Some(a),
        _ => None,
    }
}

/// Borrow the [`Wheel`] referenced by a wheel selection, if any.
fn wheel_at(cfg: &mut QuickActionConfig, sel: Selection) -> Option<&mut Wheel> {
    let Selection::Wheel { set, entry, wheel } = sel else {
        return None;
    };
    match cfg.sets.get_mut(set).and_then(|s| s.entries.get_mut(entry)) {
        Some(SetEntry::Wheel(w)) if wheel.is_none() => Some(w),
        Some(SetEntry::WheelSet(ws)) => wheel.and_then(move |i| ws.wheels.get_mut(i)),
        _ => None,
    }
}

/// Borrow the name string currently focused for typing, if any.
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
        EditFocus::SlotName(i) => wheel_at(cfg, ui.selection).and_then(move |w| w.slots.get_mut(i)),
        _ => None,
    }
}

/// Types into the focused name (action / set / wheel / slot) while a name field
/// is active.
fn editor_text_input(
    mut messages: MessageReader<KeyboardInput>,
    mut cfg: ResMut<QuickActionConfig>,
    mut ui: ResMut<EditorUiState>,
) {
    if !matches!(
        ui.editing,
        EditFocus::Name | EditFocus::SetName | EditFocus::WheelName | EditFocus::SlotName(_)
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

/// Captures the next key press into the focused key field (action binding or a
/// set-switch shortcut) while a key field is active.
fn editor_capture_key(
    keys: Res<ButtonInput<KeyCode>>,
    mut cfg: ResMut<QuickActionConfig>,
    mut ui: ResMut<EditorUiState>,
) {
    let focus = ui.editing;
    if !matches!(focus, EditFocus::Key | EditFocus::NextSetKey | EditFocus::PrevSetKey) {
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
                EditFocus::NextSetKey => cfg.next_set_key = label,
                EditFocus::PrevSetKey => cfg.prev_set_key = label,
                _ => {}
            }
        }
        ui.editing = EditFocus::None;
        ui.dirty = true;
        return;
    }
}

/// A short, human-readable label for a key code.
fn key_label(k: KeyCode) -> String {
    let dbg = format!("{:?}", k);
    let s = dbg.strip_prefix("Key").unwrap_or(&dbg);
    let s = s.strip_prefix("Digit").unwrap_or(s);
    s.to_string()
}

/// Whether a key code is a bare modifier (ignored during capture).
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
