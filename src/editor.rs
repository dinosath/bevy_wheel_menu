//! BSN-macro Quick Action Menu editor — sidebar UI only.
//!
//! The full-screen HUD canvas (wheel preview, floating buttons, set tabs) is
//! now rendered by [`WheelHudPlugin`] in `lib.rs`.  This module contains only
//! the left sidebar editor UI.
//!
//! ## Layout
//! * **Left sidebar** is context-sensitive:
//!   - Default: **navigation view** — wheel-set tree and button list for the
//!     active set, plus a set-switch key summary at the bottom.
//!   - When an item is selected: **editor panel** for that item (wheel, button,
//!     or wheel-set) with a `‹ Back` breadcrumb header.
//! * **Right canvas** is handled by [`WheelHudPlugin`].

use crate::*;

use bevy::ecs::message::MessageReader;
use bevy::input::keyboard::{Key, KeyboardInput};
use bevy::input::ButtonState;
use bevy::prelude::*;
use bevy::ui_widgets::{ControlOrientation, Scrollbar, ScrollbarThumb};

// ─── selection & edit-focus ──────────────────────────────────────────────────────

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
    SetName,
    WheelName,
    SlotName(usize),
    NextSetKey,
    PrevSetKey,
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
    /// Capturing a key/button for the global edit shortcut.
    EditShortcut,
    /// Editing the bg_image path of a set.
    SetBgImage(usize),
    /// Capturing the next-wheel shortcut for a set.
    NextWheelKey(usize),
    /// Capturing the prev-wheel shortcut for a set.
    PrevWheelKey(usize),
}

// ─── editor state ────────────────────────────────────────────────────────────────

#[derive(Resource)]
pub struct EditorUiState {
    pub dirty: bool,
    pub selection: Selection,
    pub editing: EditFocus,
    pub config_path: String,
    /// Persisted vertical scroll offset for the wheel editor panel.
    pub wheel_scroll_y: f32,
    // active_set and editor_open moved to WheelHudState in lib.rs
}
impl Default for EditorUiState {
    fn default() -> Self {
        Self {
            dirty: true,
            selection: Selection::None,
            editing: EditFocus::None,
            config_path: crate::CONFIG_FILE.into(),
            wheel_scroll_y: 0.0,
        }
    }
}

// ─── components ──────────────────────────────────────────────────────────────────

#[derive(Component)]
pub struct EditorRoot;

/// Marks the scrollable content entity inside the wheel editor panel.
/// Used by `rebuild_editor` to persist the vertical scroll offset across rebuilds.
#[derive(Component)]
pub struct EditorScrollArea;

#[derive(Component)]
pub struct SegmentHoverColor(pub Color);

#[derive(Component, Clone)]
pub struct EditorButton {
    pub action: EditorAction,
    pub base: Color,
}

// ─── editor actions ───────────────────────────────────────────────────────────────

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
    /// Capture the key/button binding for a quick action (keyboard or gamepad).
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
    ToggleWheelShowLabels,
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
    // ── persistence ────────────────────────────────────────────────────────────
    Save,
    Load,
    /// Toggle `QuickActionConfig::show_set_bar`.
    ToggleShowSetBar,
    /// Toggle `QuickActionConfig::cycle_sets`.
    ToggleCycleSets,
    /// Begin capturing the global edit-sidebar shortcut.
    CaptureEditShortcut,
    CycleHudOpenMode,
    /// Nudge `QuickActionConfig::hud_bg_opacity` by `delta`.
    HudBgOpacityDelta {
        delta: f32,
    },
    // ── per-set config ──────────────────────────────────────────────────────────
    EditSetBgImage {
        set: usize,
    },
    SetBgImageOpacityDelta {
        set: usize,
        delta: f32,
    },
    CaptureNextWheelKey {
        set: usize,
    },
    CapturePrevWheelKey {
        set: usize,
    },
    ToggleCycleWheels {
        set: usize,
    },
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
    /// Step the wheel's overall opacity up or down.
    WheelOpacityDelta {
        delta: f32,
    },
    /// Cycle the inner-border ring color (empty = no border).
    CycleInnerBorderColor,
    /// Cycle the outer-border ring color (empty = no border).
    CycleOuterBorderColor,
    /// Cycle the wheel background color.
    CycleWheelBgColor,
    /// Adjust wheel background opacity.
    WheelBgOpacityDelta {
        delta: f32,
    },
    /// Adjust outer border ring width.
    WheelOuterBorderWidthDelta {
        delta: f32,
    },
    /// Cycle the hub (inner circle) background color.
    CycleWheelHubColor,
    /// Adjust hub (inner circle) background opacity.
    WheelHubOpacityDelta {
        delta: f32,
    },
    /// Adjust inner border ring width.
    WheelInnerBorderWidthDelta {
        delta: f32,
    },
    // ── segment input / gamepad binding ─────────────────────────────────────────
    /// Capture a key or gamepad button as the input binding for segment `slot`.
    CaptureSlotInput {
        slot: usize,
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

/// Registers all editor UI resources and systems into `app`.
/// Called by [`QuickActionHudPlugin`] when `editor: true`.
pub(crate) fn register_editor_systems(app: &mut App) {
    app.init_resource::<EditorUiState>().add_systems(
        Update,
        (
            process_hud_buttons,
            handle_editor_buttons,
            editor_capture_key,
            editor_capture_gamepad,
            editor_text_input,
            editor_button_feedback,
            apply_set_shortcuts,
            hud_wheel_nav,
            check_edit_shortcut,
            rebuild_editor,
        )
            .chain(),
    );
}

/// Convenience plugin — equivalent to `QuickActionHudPlugin::with_editor()`.
///
/// Adds core wheel logic, the HUD canvas, **and** the editor sidebar in one call.
pub struct QuickActionEditorPlugin;
impl Plugin for QuickActionEditorPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(QuickActionHudPlugin::with_editor());
    }
}

// ─── palette ─────────────────────────────────────────────────────────────────────

const BG_SIDEBAR: Color = Color::srgb(0.043, 0.055, 0.075);
#[allow(dead_code)]
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
            position_type: PositionType::Absolute,
            left: {px(0.)}, top: {px(0.)}, bottom: {px(0.)},
            width: {px(260.)},
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

/// A scrollable content column with an attached thin vertical scrollbar.
/// Returns the scrollable content entity to use as the layout parent.
fn scrolled_tree(commands: &mut Commands, parent: Entity) -> Entity {
    // Flex-row wrapper: [content column | scrollbar track]
    let wrapper = commands
        .spawn(Node {
            flex_grow: 1.0,
            flex_direction: FlexDirection::Row,
            min_height: Val::Px(0.0),
            ..default()
        })
        .id();
    commands.entity(parent).add_child(wrapper);

    // Scrollable content area (mouse-wheel + draggable scrollbar both work)
    let scroll_area = commands
        .spawn((
            Node {
                flex_grow: 1.0,
                flex_direction: FlexDirection::Column,
                padding: UiRect::axes(Val::Px(12.0), Val::Px(8.0)),
                row_gap: Val::Px(2.0),
                overflow: Overflow::scroll_y(),
                min_height: Val::Px(0.0),
                ..default()
            },
            EditorScrollArea,
        ))
        .id();
    commands.entity(wrapper).add_child(scroll_area);

    // Scrollbar track (6 px wide strip on the right)
    let track = commands
        .spawn((
            Node {
                min_width: Val::Px(6.0),
                ..default()
            },
            BackgroundColor(Color::srgba(0.06, 0.08, 0.12, 0.70)),
            Scrollbar {
                target: scroll_area,
                orientation: ControlOrientation::Vertical,
                min_thumb_length: 20.0,
            },
        ))
        .id();
    commands.entity(wrapper).add_child(track);

    // Scrollbar thumb — NO Node; the Scrollbar system owns its geometry
    let thumb = commands
        .spawn((
            ScrollbarThumb {
                border_radius: BorderRadius::all(Val::Px(3.0)),
                border: UiRect::all(Val::Px(0.0)),
            },
            BackgroundColor(Color::srgba(0.48, 0.50, 0.55, 0.85)),
        ))
        .id();
    commands.entity(track).add_child(thumb);

    scroll_area
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

/// Spawn a [`WheelHudButton`] child — handled by `process_hud_buttons`.
fn hud_clickable(
    commands: &mut Commands,
    parent: Entity,
    scene: impl Scene,
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
    icons: &Icons<'_>,
) {
    let bg = if selected { ROW_SEL } else { Color::NONE };
    let row = clickable(commands, parent, row_button(bg), select_action, bg);
    let left = child(commands, row, hcluster());
    child(commands, left, text(icon, 10., icon_col));
    child(commands, left, text(name, 11., name_col));
    let right = child(commands, row, hcluster());
    match &badge {
        Badge::Key(k) => {
            spawn_input_badge(commands, right, k, icons);
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

// ─── icons context ─────────────────────────────────────────────────────────────

/// Lightweight context bundle threaded through builder functions to enable
/// controller button icon display.
struct Icons<'a> {
    srv: &'a AssetServer,
    set: GamepadIconSet,
}

/// Renders a key/button input badge.
/// - `"GP:A"`, `"GP:LB"` etc. → loads and shows the matching PNG icon (20×20 px).
/// - Keyboard key → text badge (existing behaviour).
fn spawn_input_badge(commands: &mut Commands, parent: Entity, key: &str, icons: &Icons<'_>) {
    if let Some(label) = key.strip_prefix("GP:") {
        if let Some(path) = icons.set.icon_path(label) {
            let handle = icons.srv.load::<Image>(path);
            let e = commands
                .spawn((
                    Node {
                        width: Val::Px(20.0),
                        height: Val::Px(20.0),
                        ..default()
                    },
                    ImageNode::new(handle),
                ))
                .id();
            commands.entity(parent).add_child(e);
            return;
        }
    }
    // Keyboard fallback — keep the existing bordered text badge
    let kb = child(commands, parent, key_badge_box());
    child(commands, kb, text(key, 8., DIM));
}

/// Like [`spawn_box_field`] but renders a controller icon inside the clickable
/// box when `raw_key` is a `"GP:…"` binding and the field is not in capture mode.
fn spawn_key_capture_field(
    commands: &mut Commands,
    parent: Entity,
    label: &str,
    display: &str,
    display_color: Color,
    accent: Color,
    action: EditorAction,
    raw_key: &str,
    focused: bool,
    icons: &Icons<'_>,
) {
    let row = spawn_field(commands, parent, label);
    let b = clickable(commands, row, ctrl_box(accent), action, Color::NONE);
    if !focused && !raw_key.is_empty() {
        if let Some(btn_label) = raw_key.strip_prefix("GP:") {
            if let Some(path) = icons.set.icon_path(btn_label) {
                let handle = icons.srv.load::<Image>(path);
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
                commands.entity(b).add_child(e);
                return;
            }
        }
    }
    child(commands, b, text(display, 11., display_color));
}

// ─── rebuild ─────────────────────────────────────────────────────────────────────

fn rebuild_editor(
    mut commands: Commands,
    mut ui: ResMut<EditorUiState>,
    hud: Res<WheelHudState>,
    cfg: Res<QuickActionConfig>,
    asset_server: Res<AssetServer>,
    icon_set: Res<GamepadIconSet>,
    old_sidebar: Query<Entity, With<EditorRoot>>,
    scroll_q: Query<&ScrollPosition, With<EditorScrollArea>>,
) {
    if !ui.dirty {
        return;
    }
    ui.dirty = false;

    // Persist the current vertical scroll offset so we can restore it after
    // the UI is rebuilt (despawn + respawn resets ScrollPosition to zero).
    if let Ok(sp) = scroll_q.single() {
        ui.wheel_scroll_y = sp.0.y;
    }

    for e in &old_sidebar {
        commands.entity(e).despawn();
    }

    if hud.editor_open {
        let icons = Icons {
            srv: &*asset_server,
            set: *icon_set,
        };
        if let Some(scroll_area) = build_sidebar(&mut commands, &cfg, &ui, &hud, &icons) {
            // Restore saved offset. The insert command runs after the spawn
            // commands, so the entity is guaranteed to exist by then.
            if ui.wheel_scroll_y > 0.0 {
                commands
                    .entity(scroll_area)
                    .insert(ScrollPosition(Vec2::new(0.0, ui.wheel_scroll_y)));
            }
        } else {
            // Navigated away from wheel editor — reset so the next wheel
            // editor opens at the top.
            ui.wheel_scroll_y = 0.0;
        }
    }
}

// ─── sidebar ─────────────────────────────────────────────────────────────────────

fn build_sidebar(
    commands: &mut Commands,
    cfg: &QuickActionConfig,
    ui: &EditorUiState,
    hud: &WheelHudState,
    icons: &Icons<'_>,
) -> Option<Entity> {
    let root = commands.spawn_scene(sidebar()).insert(EditorRoot).id();

    match ui.selection {
        // Root: ActionSets list ───────────────────────────────────────────
        Selection::None => {
            build_root_sidebar(commands, root, cfg, ui, icons);
            None
        }

        // Set detail: wheels + buttons for one set ──────────────────────────────
        Selection::Set { .. } | Selection::SetSwitch => {
            let si = if let Selection::Set { set } = ui.selection {
                set
            } else {
                0
            };
            build_nav_sidebar(commands, root, cfg, ui, hud, si, icons);
            None
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
                let set_name = cfg.sets.get(set).map(|s| s.name.as_str()).unwrap_or("Set");
                build_editor_header(
                    commands,
                    root,
                    Some(set_name),
                    &qa.name.clone(),
                    EditorAction::NavBack,
                );
                let scroll = child(commands, root, tree());
                spawn_action_editor(commands, scroll, ui, set, entry, qa, icons);
                build_footer(commands, root, &ui.config_path);
            } else {
                build_nav_sidebar(commands, root, cfg, ui, hud, set, icons);
            }
            None
        }

        // Wheel editor ────────────────────────────────────────────────────────
        Selection::Wheel { set, entry, wheel } => {
            let w_ref = cfg
                .sets
                .get(set)
                .and_then(|s| s.entries.get(entry))
                .and_then(|e| match (e, wheel) {
                    (SetEntry::Wheel(w), None) => Some(w as &WheelData),
                    (SetEntry::WheelSet(ws), Some(i)) => ws.wheels.get(i).map(|w| w as &WheelData),
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
                let scroll = scrolled_tree(commands, root);
                spawn_wheel_editor(commands, scroll, ui, w, set, entry, wheel, icons);
                build_footer(commands, root, &ui.config_path);
                Some(scroll)
            } else {
                build_nav_sidebar(commands, root, cfg, ui, hud, set, icons);
                None
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
                let set_name2 = cfg.sets.get(set).map(|s| s.name.as_str()).unwrap_or("Set");
                build_editor_header(
                    commands,
                    root,
                    Some(set_name2),
                    &wname,
                    EditorAction::NavBack,
                );
                let scroll = child(commands, root, tree());
                spawn_wheelset_entry_editor(commands, scroll, ui, set, entry, ws, icons);
                build_footer(commands, root, &ui.config_path);
            } else {
                build_nav_sidebar(commands, root, cfg, ui, hud, set, icons);
            }
            None
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
                    (SetEntry::Wheel(w), None) => Some(w as &WheelData),
                    (SetEntry::WheelSet(ws), Some(i)) => ws.wheels.get(i).map(|w| w as &WheelData),
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
                spawn_segment_editor(commands, scroll, ui, slot, w, icons);
                build_footer(commands, root, &ui.config_path);
            } else {
                build_nav_sidebar(commands, root, cfg, ui, hud, set, icons);
            }
            None
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

/// Set-detail sidebar: wheel-set tree + button list for one specific set.
fn build_nav_sidebar(
    commands: &mut Commands,
    root: Entity,
    cfg: &QuickActionConfig,
    ui: &EditorUiState,
    _hud: &WheelHudState,
    si: usize,
    icons: &Icons<'_>,
) {
    // Header with breadcrumb: ‹ Action Sets | Set Name
    let set_name = cfg.sets.get(si).map(|s| s.name.as_str()).unwrap_or("—");
    let header = commands
        .spawn_scene(bsn! {
            Node {
                flex_direction: FlexDirection::Row,
                justify_content: JustifyContent::SpaceBetween,
                align_items: AlignItems::Center,
                padding: {UiRect::all(px(12.))},
                column_gap: {px(6.)},
                border: {UiRect::bottom(px(1.))},
            }
            BorderColor::all(SIDEBAR_BORDER)
        })
        .id();
    commands.entity(root).add_child(header);

    // Back button → root
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
        EditorAction::NavBack,
        Color::NONE,
    );
    child(commands, back, text("‹", 15., DIM));

    // Breadcrumb: "Action Sets | Set Name"
    child(commands, header, text("Action Sets", 10., DIM));
    child(commands, header, text("|", 10., DIMMER));

    // Editable set name
    let nf = ui.editing == EditFocus::SetName && ui.selection == (Selection::Set { set: si });
    let nd = if nf {
        format!("{}|", set_name)
    } else {
        set_name.to_string()
    };
    let name_btn = clickable(
        commands,
        header,
        bsn! {
            Node { flex_grow: 1., padding: {UiRect::axes(px(3.), px(2.))} }
            Button
        },
        EditorAction::EditSetName { set: si },
        Color::NONE,
    );
    child(
        commands,
        name_btn,
        text(&nd, 11., if nf { AMBER } else { TEXT }),
    );

    // Close button
    let close = hud_clickable(
        commands,
        header,
        bsn! {
            Node {
                width: {px(20.)}, height: {px(20.)},
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                border: {UiRect::all(px(1.))},
                border_radius: {BorderRadius::all(px(3.))},
            }
            BorderColor::all(BADGE_BORDER)
            Button
        },
        WheelHudAction::ToggleEditor,
        Color::NONE,
    );
    child(commands, close, text("×", 13., DIM));

    // Scrollable body
    let scroll = child(commands, root, tree());

    if let Some(set) = cfg.sets.get(si) {
        // Per-set opacity control
        let scard = child(commands, scroll, editor_card());
        spawn_stepper_field(
            commands,
            scard,
            "Opacity",
            &format!("{:.0}%", set.opacity * 100.0),
            EditorAction::SetOpacityDelta {
                set: si,
                delta: -0.05,
            },
            EditorAction::SetOpacityDelta {
                set: si,
                delta: 0.05,
            },
        );

        // ── SET CONFIG ────────────────────────────────────────────────────────────────
        section_label(commands, scroll, "SET CONFIG");
        let cfg_card = child(commands, scroll, editor_card());

        // Background image path
        let bg_f = ui.editing == EditFocus::SetBgImage(si);
        let bg_d = if bg_f {
            format!("{}|", set.bg_image)
        } else if set.bg_image.is_empty() {
            "none".to_string()
        } else {
            set.bg_image.clone()
        };
        spawn_box_field(
            commands,
            cfg_card,
            "BG image",
            &bg_d,
            if bg_f {
                AMBER
            } else if set.bg_image.is_empty() {
                DIMMER
            } else {
                TEXT
            },
            if bg_f { AMBER } else { BADGE_BORDER },
            EditorAction::EditSetBgImage { set: si },
        );

        // Background image opacity
        spawn_stepper_field(
            commands,
            cfg_card,
            "BG opacity",
            &format!("{:.0}%", set.bg_image_opacity * 100.0),
            EditorAction::SetBgImageOpacityDelta {
                set: si,
                delta: -0.05,
            },
            EditorAction::SetBgImageOpacityDelta {
                set: si,
                delta: 0.05,
            },
        );

        // Next wheel shortcut
        let nwf = ui.editing == EditFocus::NextWheelKey(si);
        let (nwd, nwc) = key_display(nwf, &set.next_wheel_key);
        spawn_key_capture_field(
            commands,
            cfg_card,
            "Next wheel",
            &nwd,
            nwc,
            if nwf { AMBER } else { BADGE_BORDER },
            EditorAction::CaptureNextWheelKey { set: si },
            &set.next_wheel_key,
            nwf,
            icons,
        );

        // Prev wheel shortcut
        let pwf = ui.editing == EditFocus::PrevWheelKey(si);
        let (pwd, pwc) = key_display(pwf, &set.prev_wheel_key);
        spawn_key_capture_field(
            commands,
            cfg_card,
            "Prev wheel",
            &pwd,
            pwc,
            if pwf { AMBER } else { BADGE_BORDER },
            EditorAction::CapturePrevWheelKey { set: si },
            &set.prev_wheel_key,
            pwf,
            icons,
        );

        // Cycle wheels toggle
        spawn_toggle_field(
            commands,
            cfg_card,
            "Cycle wheels",
            set.cycle_wheels,
            EditorAction::ToggleCycleWheels { set: si },
        );
        // ────────────────────────────────────────────────────────────────────────────

        build_nav_wheel_section(commands, scroll, ui, set, si, icons);
        build_nav_button_section(commands, scroll, ui, set, si, icons);
    } else {
        child(commands, scroll, text("No entries.", 10., DIMMER));
    }

    build_footer(commands, root, &ui.config_path);
}

/// Root sidebar: list of all ActionSets + global config.
fn build_root_sidebar(
    commands: &mut Commands,
    root: Entity,
    cfg: &QuickActionConfig,
    ui: &EditorUiState,
    icons: &Icons<'_>,
) {
    // ── header ───────────────────────────────────────────────────────────────────────
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
    child(commands, header, text("Action Sets", 13., TEXT));
    let close = hud_clickable(
        commands,
        header,
        bsn! {
            Node {
                width: {px(20.)}, height: {px(20.)},
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                border: {UiRect::all(px(1.))},
                border_radius: {BorderRadius::all(px(3.))},
            }
            BorderColor::all(BADGE_BORDER)
            Button
        },
        WheelHudAction::ToggleEditor,
        Color::NONE,
    );
    child(commands, close, text("×", 13., DIM));

    // ── scrollable set list ──────────────────────────────────────────────────────
    let scroll = child(commands, root, tree());

    for (si, set) in cfg.sets.iter().enumerate() {
        let sel = ui.selection == (Selection::Set { set: si });
        let row_bg = if sel { ROW_SEL } else { Color::NONE };
        let row = child(
            commands,
            scroll,
            bsn! {
                Node {
                    width: {percent(100.)}, height: {px(30.)},
                    flex_direction: FlexDirection::Row,
                    align_items: AlignItems::Center,
                    column_gap: {px(4.)},
                    padding: {UiRect::horizontal(px(4.))},
                    border_radius: {BorderRadius::all(px(3.))},
                }
                BackgroundColor({row_bg})
            },
        );

        // Clickable set name → navigate into set
        let name_btn = clickable(
            commands,
            row,
            bsn! {
                Node {
                    flex_grow: 1.,
                    flex_direction: FlexDirection::Row,
                    align_items: AlignItems::Center,
                    column_gap: {px(6.)},
                    padding: {UiRect::axes(px(4.), px(3.))},
                    border_radius: {BorderRadius::all(px(3.))},
                }
                Button
            },
            EditorAction::SelectSet { set: si },
            Color::NONE,
        );
        child(commands, name_btn, text("›", 11., DIMMER));
        child(commands, name_btn, text(&set.name, 11., TEXT));

        // Delete button
        let dx = clickable(
            commands,
            row,
            del_btn(),
            EditorAction::DeleteSet { set: si },
            Color::NONE,
        );
        child(commands, dx, text("×", 10., DIMMER));
    }

    // + Add Set
    let add_row = clickable(
        commands,
        scroll,
        bsn! {
            Node {
                width: {percent(100.)}, height: {px(28.)},
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                column_gap: {px(6.)},
                margin: {UiRect::top(px(4.))},
                border: {UiRect::all(px(1.))},
                border_radius: {BorderRadius::all(px(4.))},
            }
            BorderColor::all(TEAL)
            Button
        },
        EditorAction::AddSet,
        Color::NONE,
    );
    child(commands, add_row, text("+", 12., TEAL));
    child(commands, add_row, text("Add Set", 10., TEAL));

    // ── config section ───────────────────────────────────────────────────────────────
    let config_area = commands
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
    commands.entity(root).add_child(config_area);

    // Section label
    let lrow = child(commands, config_area, hcluster());
    child(commands, lrow, text("~", 9., TEAL));
    child(commands, lrow, text("SET CONFIG", 9., DIM));

    let card = child(commands, config_area, editor_card());

    // Next set key
    let nf = ui.editing == EditFocus::NextSetKey;
    let (nd, nc) = key_display(nf, &cfg.next_set_key);
    spawn_key_capture_field(
        commands,
        card,
        "Next set key",
        &nd,
        nc,
        if nf { AMBER } else { BADGE_BORDER },
        EditorAction::CaptureNextSetKey,
        &cfg.next_set_key,
        nf,
        icons,
    );

    // Prev set key
    let pf = ui.editing == EditFocus::PrevSetKey;
    let (pd, pc) = key_display(pf, &cfg.prev_set_key);
    spawn_key_capture_field(
        commands,
        card,
        "Prev set key",
        &pd,
        pc,
        if pf { AMBER } else { BADGE_BORDER },
        EditorAction::CapturePrevSetKey,
        &cfg.prev_set_key,
        pf,
        icons,
    );

    // Show set bar toggle
    spawn_toggle_field(
        commands,
        card,
        "Show set bar",
        cfg.show_set_bar,
        EditorAction::ToggleShowSetBar,
    );

    // Cycle sets toggle
    spawn_toggle_field(
        commands,
        card,
        "Cycle sets",
        cfg.cycle_sets,
        EditorAction::ToggleCycleSets,
    );

    // Edit shortcut
    let ef = ui.editing == EditFocus::EditShortcut;
    let (ed, ec) = key_display(ef, &cfg.edit_shortcut);
    spawn_key_capture_field(
        commands,
        card,
        "Edit shortcut",
        &ed,
        ec,
        if ef { AMBER } else { BADGE_BORDER },
        EditorAction::CaptureEditShortcut,
        &cfg.edit_shortcut,
        ef,
        icons,
    );

    // HUD open mode (Hold / Toggle)
    spawn_box_field(
        commands,
        card,
        "HUD open mode",
        cfg.hud_open_mode.label(),
        TEXT,
        BADGE_BORDER,
        EditorAction::CycleHudOpenMode,
    );

    // HUD background opacity
    spawn_stepper_field(
        commands,
        card,
        "HUD bg opacity",
        &format!("{:.0}%", cfg.hud_bg_opacity * 100.0),
        EditorAction::HudBgOpacityDelta { delta: -0.05 },
        EditorAction::HudBgOpacityDelta { delta: 0.05 },
    );

    // ── footer ─────────────────────────────────────────────────────────────────────
    build_footer(commands, root, &ui.config_path);
}

/// "~ WHEEL SET" navigation section.
fn build_nav_wheel_section(
    commands: &mut Commands,
    parent: Entity,
    ui: &EditorUiState,
    set: &ActionSet,
    si: usize,
    icons: &Icons<'_>,
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
        EditorAction::AddWheel { set: si },
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
                let badge = Badge::None;
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
                    icons,
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
                    let badge = Badge::None;
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
                        icons,
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
}

/// "~ BUTTONS" navigation section.
fn build_nav_button_section(
    commands: &mut Commands,
    parent: Entity,
    ui: &EditorUiState,
    set: &ActionSet,
    si: usize,
    icons: &Icons<'_>,
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
                icons,
            );
        }
    }
    if !has_any {
        child(commands, body, text("No buttons yet.", 10., DIMMER));
    }
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
    mut hud: ResMut<WheelHudState>,
) {
    for (btn, interaction) in &buttons {
        if *interaction == Interaction::Pressed {
            apply_action(&btn.action, &mut cfg, &mut ui, &mut hud);
            ui.dirty = true;
            hud.dirty = true;
        }
    }
}

/// Handles [`WheelHudButton`] clicks spawned by the HUD (set tabs, toggle, etc.).
fn process_hud_buttons(
    buttons: Query<(&WheelHudButton, &Interaction), Changed<Interaction>>,
    mut hud: ResMut<WheelHudState>,
    mut ui: ResMut<EditorUiState>,
    qcfg: Res<QuickActionConfig>,
) {
    for (btn, interaction) in &buttons {
        if *interaction == Interaction::Pressed {
            match &btn.action {
                WheelHudAction::SetActiveSet(i) => {
                    hud.active_set = *i;
                    hud.active_wheel_entry = 0;
                    hud.dirty = true;
                    ui.dirty = true;
                }
                WheelHudAction::PrevSet => {
                    if hud.active_set > 0 {
                        hud.active_set -= 1;
                    } else if qcfg.cycle_sets && !qcfg.sets.is_empty() {
                        hud.active_set = qcfg.sets.len() - 1;
                    }
                    hud.active_wheel_entry = 0;
                    hud.dirty = true;
                    ui.dirty = true;
                }
                WheelHudAction::NextSet => {
                    let max = qcfg.sets.len().saturating_sub(1);
                    if hud.active_set < max {
                        hud.active_set += 1;
                    } else if qcfg.cycle_sets {
                        hud.active_set = 0;
                    }
                    hud.active_wheel_entry = 0;
                    hud.dirty = true;
                    ui.dirty = true;
                }
                WheelHudAction::ToggleEditor => {
                    hud.editor_open = !hud.editor_open;
                    if !hud.editor_open {
                        ui.selection = Selection::None;
                        ui.editing = EditFocus::None;
                    }
                    hud.dirty = true;
                    ui.dirty = true;
                }
            }
        }
    }
}

// ─── action application ──────────────────────────────────────────────────────────

fn apply_action(
    action: &EditorAction,
    cfg: &mut QuickActionConfig,
    ui: &mut EditorUiState,
    hud: &mut WheelHudState,
) {
    match *action {
        // ── sets ──────────────────────────────────────────────────────────────
        EditorAction::AddSet => {
            let n = cfg.sets.len() + 1;
            cfg.sets.push(ActionSet {
                name: format!("Set {}", n),
                opacity: 1.0,
                input_override: false,
                entries: Vec::new(),
                bg_image: String::new(),
                bg_image_opacity: 1.0,
                next_wheel_key: String::new(),
                prev_wheel_key: String::new(),
                cycle_wheels: false,
            });
            hud.active_set = cfg.sets.len() - 1;
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
                hud.active_set = hud.active_set.min(cfg.sets.len() - 1);
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
                s.entries
                    .push(SetEntry::Wheel(WheelData::new("New Wheel", 6)));
            }
        }
        EditorAction::AddWheelSet { set } => {
            if let Some(s) = cfg.sets.get_mut(set) {
                s.entries.push(SetEntry::WheelSet(WheelSetData {
                    name: "New Wheel Set".into(),
                    wheels: vec![WheelData::new("Wheel 1", 6)],
                    switch_key: String::new(),
                }));
            }
        }
        EditorAction::AddWheelToSet { set, entry } => {
            if let Some(SetEntry::WheelSet(ws)) =
                cfg.sets.get_mut(set).and_then(|s| s.entries.get_mut(entry))
            {
                let n = ws.wheels.len() + 1;
                ws.wheels.push(WheelData::new(format!("Wheel {}", n), 6));
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
            hud.highlighted = None;
        }
        EditorAction::SelectWheel { set, entry, wheel } => {
            ui.selection = Selection::Wheel { set, entry, wheel };
            ui.editing = EditFocus::None;
            hud.highlighted = None;
        }
        EditorAction::SelectWheelSetEntry { set, entry } => {
            ui.selection = Selection::WheelSetEntry { set, entry };
            ui.editing = EditFocus::None;
            hud.highlighted = None;
        }
        EditorAction::SelectSetSwitch => {
            ui.selection = Selection::SetSwitch;
            ui.editing = EditFocus::None;
            hud.highlighted = None;
        }
        EditorAction::NavBack => {
            ui.editing = EditFocus::None;
            hud.highlighted = None;
            ui.selection = match ui.selection {
                // Segment → back to its wheel
                Selection::Segment {
                    set, entry, wheel, ..
                } => Selection::Wheel { set, entry, wheel },
                // Wheel / Action / WheelSetEntry → back to the set
                Selection::Wheel { set, .. }
                | Selection::Action { set, .. }
                | Selection::WheelSetEntry { set, .. } => Selection::Set { set },
                // Set / SetSwitch / root → root
                _ => Selection::None,
            };
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
                a.icon = cycle_palette(ICON_PALETTE, &a.icon).into();
            }
        }
        EditorAction::CycleCommand { set, entry } => {
            if let Some(a) = action_at(cfg, set, entry) {
                a.command = cycle_palette(COMMAND_PALETTE, &a.command).into();
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
        EditorAction::ToggleWheelShowLabels => {
            if let Some(w) = wheel_at(cfg, ui.selection) {
                w.show_labels = !w.show_labels;
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
        // ── set-switch shortcuts ──────────────────────────────────────────
        EditorAction::CaptureNextSetKey => {
            ui.editing = EditFocus::NextSetKey;
        }
        EditorAction::CapturePrevSetKey => {
            ui.editing = EditFocus::PrevSetKey;
        }
        EditorAction::CaptureEditShortcut => {
            ui.editing = EditFocus::EditShortcut;
        }
        // ── persistence ───────────────────────────────────────────────────────────
        EditorAction::Save => save_config(cfg, &ui.config_path),
        EditorAction::Load => {
            if let Some(loaded) = load_config(&ui.config_path) {
                *cfg = loaded;
                ui.selection = Selection::None;
                ui.editing = EditFocus::None;
                hud.active_set = 0;
                hud.highlighted = None;
                hud.dirty = true;
                ui.dirty = true;
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
            hud.highlighted = Some((set, entry, wheel, slot));
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
                w.highlight_color = cycle_palette(COLORS, &w.highlight_color).into();
            }
        }
        EditorAction::SegmentScaleDelta { delta } => {
            if let Some(w) = wheel_at(cfg, ui.selection) {
                w.segment_scale = (w.segment_scale + delta).clamp(0.5, 2.0);
            }
        }
        EditorAction::WheelOpacityDelta { delta } => {
            if let Some(w) = wheel_at(cfg, ui.selection) {
                w.opacity = (w.opacity + delta).clamp(0.0, 1.0);
            }
        }
        EditorAction::CycleInnerBorderColor => {
            const COLORS: &[&str] = &[
                "", "#f59e0b", "#3b82f6", "#14b8a6", "#8b5cf6", "#22c55e", "#ef4444",
            ];
            if let Some(w) = wheel_at(cfg, ui.selection) {
                w.inner_border = cycle_palette(COLORS, &w.inner_border).into();
            }
        }
        EditorAction::CycleOuterBorderColor => {
            const COLORS: &[&str] = &[
                "", "#f59e0b", "#3b82f6", "#14b8a6", "#8b5cf6", "#22c55e", "#ef4444",
            ];
            if let Some(w) = wheel_at(cfg, ui.selection) {
                w.outer_border = cycle_palette(COLORS, &w.outer_border).into();
            }
        }
        EditorAction::CycleWheelBgColor => {
            const COLORS: &[&str] = &[
                "", "#0d1520", "#111827", "#1a1a2e", "#0f172a", "#1c1c1c", "#0a0f1e",
            ];
            if let Some(w) = wheel_at(cfg, ui.selection) {
                w.bg_color = cycle_palette(COLORS, &w.bg_color).into();
            }
        }
        EditorAction::WheelBgOpacityDelta { delta } => {
            if let Some(w) = wheel_at(cfg, ui.selection) {
                w.bg_opacity = (w.bg_opacity + delta).clamp(0.0, 1.0);
            }
        }
        EditorAction::WheelOuterBorderWidthDelta { delta } => {
            if let Some(w) = wheel_at(cfg, ui.selection) {
                w.outer_border_width = (w.outer_border_width + delta).clamp(0.0, 12.0);
            }
        }
        EditorAction::CycleWheelHubColor => {
            const COLORS: &[&str] = &[
                "", "#0d1520", "#111827", "#1a1a2e", "#0f172a", "#1c1c1c", "#142030",
            ];
            if let Some(w) = wheel_at(cfg, ui.selection) {
                w.hub_color = cycle_palette(COLORS, &w.hub_color).into();
            }
        }
        EditorAction::WheelInnerBorderWidthDelta { delta } => {
            if let Some(w) = wheel_at(cfg, ui.selection) {
                w.inner_border_width = (w.inner_border_width + delta).clamp(0.0, 12.0);
            }
        }
        EditorAction::WheelHubOpacityDelta { delta } => {
            if let Some(w) = wheel_at(cfg, ui.selection) {
                w.hub_opacity = (w.hub_opacity + delta).clamp(0.0, 1.0);
            }
        }
        // ── segment input / gamepad binding ─────────────────────────────────────────
        EditorAction::CaptureSlotInput { slot } => {
            ui.editing = EditFocus::SlotInput(slot);
        }
        // ── per-slot items ─────────────────────────────────────────────────────────────────────────
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
        EditorAction::ToggleShowSetBar => {
            cfg.show_set_bar = !cfg.show_set_bar;
        }
        EditorAction::ToggleCycleSets => {
            cfg.cycle_sets = !cfg.cycle_sets;
        }
        EditorAction::CycleHudOpenMode => {
            cfg.hud_open_mode = cfg.hud_open_mode.next();
        }
        EditorAction::HudBgOpacityDelta { delta } => {
            cfg.hud_bg_opacity = (cfg.hud_bg_opacity + delta).clamp(0.0, 1.0);
        }
        // ── per-set config ──────────────────────────────────────────────────────────
        EditorAction::EditSetBgImage { set } => {
            ui.editing = EditFocus::SetBgImage(set);
        }
        EditorAction::SetBgImageOpacityDelta { set, delta } => {
            if let Some(s) = cfg.sets.get_mut(set) {
                s.bg_image_opacity = (s.bg_image_opacity + delta).clamp(0.0, 1.0);
                hud.dirty = true;
                ui.dirty = true;
            }
        }
        EditorAction::CaptureNextWheelKey { set } => {
            ui.editing = EditFocus::NextWheelKey(set);
        }
        EditorAction::CapturePrevWheelKey { set } => {
            ui.editing = EditFocus::PrevWheelKey(set);
        }
        EditorAction::ToggleCycleWheels { set } => {
            if let Some(s) = cfg.sets.get_mut(set) {
                s.cycle_wheels = !s.cycle_wheels;
                ui.dirty = true;
            }
        }
    }
}

// ─── persistence ─────────────────────────────────────────────────────────────────

fn save_config(cfg: &QuickActionConfig, path: &str) {
    match ron::ser::to_string_pretty(cfg, ron::ser::PrettyConfig::default()) {
        Ok(s) => {
            if let Err(e) = std::fs::write(path, &s) {
                error!("[editor] write failed ({path}): {e}");
            } else {
                info!("[editor] saved to {path}");
            }
        }
        Err(e) => error!("[editor] serialize failed: {e}"),
    }
}

fn load_config(path: &str) -> Option<QuickActionConfig> {
    let s = match std::fs::read_to_string(path) {
        Ok(s) => s,
        Err(e) => {
            error!("[editor] read failed ({path}): {e}");
            return None;
        }
    };
    match ron::from_str(&s) {
        Ok(c) => {
            info!("[editor] loaded from {path}");
            Some(c)
        }
        Err(e) => {
            error!("[editor] parse failed ({path}): {e}");
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
    icons: &Icons<'_>,
) {
    // Panel header: "BUTTON" label + delete
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

    // Unified input field (keyboard key or gamepad button, same as set shortcuts)
    {
        let row = spawn_field(commands, card, "Input");
        let kf = ui.editing == EditFocus::Key;
        let (kd, kc) = if kf {
            ("press key or button\u{2026}".to_string(), AMBER)
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
                    width: {px(56.)}, height: {px(20.)},
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
        if !kf && !qa.key.is_empty() {
            if let Some(btn_label) = qa.key.strip_prefix("GP:") {
                if let Some(path) = icons.set.icon_path(btn_label) {
                    let handle = icons.srv.load::<Image>(path);
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
                    commands.entity(kb).add_child(e);
                } else {
                    child(commands, kb, text(&kd, 10., kc));
                }
            } else {
                child(commands, kb, text(&kd, 10., kc));
            }
        } else {
            child(commands, kb, text(&kd, 10., kc));
        }

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
        ("press key or button\u{2026}".to_string(), AMBER)
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
    w: &WheelData,
    set: usize,
    entry: usize,
    w_idx: Option<usize>,
    icons: &Icons<'_>,
) {
    // ── WHEEL (basic) ──────────────────────────────────────────────────────────
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

    // Cooldown
    spawn_stepper_field(
        commands,
        card,
        "Cooldown (s)",
        &format!("{:.1}", w.cooldown_secs),
        EditorAction::WheelCooldownDelta { delta: -0.5 },
        EditorAction::WheelCooldownDelta { delta: 0.5 },
    );

    // ── APPEARANCE ─────────────────────────────────────────────────────────────
    section_label(commands, parent, "APPEARANCE");
    let app_card = child(commands, parent, editor_card());

    // Opacity
    spawn_stepper_field(
        commands,
        app_card,
        "Opacity",
        &format!("{:.0}%", w.opacity * 100.0),
        EditorAction::WheelOpacityDelta { delta: -0.05 },
        EditorAction::WheelOpacityDelta { delta: 0.05 },
    );

    // Show labels
    spawn_toggle_field(
        commands,
        app_card,
        "Show labels",
        w.show_labels,
        EditorAction::ToggleWheelShowLabels,
    );

    // Show icons
    spawn_toggle_field(
        commands,
        app_card,
        "Show icons",
        w.show_icon,
        EditorAction::ToggleWheelShowIcon,
    );

    // Segment Shape
    spawn_box_field(
        commands,
        app_card,
        "Seg shape",
        w.segment_shape.label(),
        TEXT,
        BADGE_BORDER,
        EditorAction::CycleSegmentShape,
    );

    // Segment Scale
    spawn_stepper_field(
        commands,
        app_card,
        "Seg scale",
        &format!("{:.1}", w.segment_scale),
        EditorAction::SegmentScaleDelta { delta: -0.1 },
        EditorAction::SegmentScaleDelta { delta: 0.1 },
    );

    // Highlight color
    {
        let hcol = parse_hex_color(&w.highlight_color, 1.0);
        let hex = w.highlight_color.clone();
        let hrow = spawn_field(commands, app_card, "Highlight");
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

    // ── BACKGROUND ─────────────────────────────────────────────────────────────
    section_label(commands, parent, "BACKGROUND");
    let bg_card = child(commands, parent, editor_card());

    // Bg color
    {
        let label = if w.bg_color.is_empty() {
            "Theme".to_string()
        } else {
            w.bg_color.clone()
        };
        let col = if w.bg_color.is_empty() { DIMMER } else { TEXT };
        spawn_box_field(
            commands,
            bg_card,
            "Color",
            &label,
            col,
            BADGE_BORDER,
            EditorAction::CycleWheelBgColor,
        );
    }

    // Bg opacity
    spawn_stepper_field(
        commands,
        bg_card,
        "Opacity",
        &format!("{:.0}%", w.bg_opacity * 100.0),
        EditorAction::WheelBgOpacityDelta { delta: -0.05 },
        EditorAction::WheelBgOpacityDelta { delta: 0.05 },
    );

    // ── OUTER CIRCLE ───────────────────────────────────────────────────────────
    section_label(commands, parent, "OUTER CIRCLE");
    let out_card = child(commands, parent, editor_card());

    // Outer Radius
    spawn_stepper_field(
        commands,
        out_card,
        "Radius",
        &format!("{:.0}", w.outer_radius),
        EditorAction::WheelOuterRadiusDelta { delta: -5.0 },
        EditorAction::WheelOuterRadiusDelta { delta: 5.0 },
    );

    // Outer border color
    {
        let label = if w.outer_border.is_empty() {
            "None".to_string()
        } else {
            w.outer_border.clone()
        };
        let col = if w.outer_border.is_empty() {
            DIMMER
        } else {
            TEXT
        };
        spawn_box_field(
            commands,
            out_card,
            "Border color",
            &label,
            col,
            BADGE_BORDER,
            EditorAction::CycleOuterBorderColor,
        );
    }

    // Outer border width
    spawn_stepper_field(
        commands,
        out_card,
        "Border width",
        &format!("{:.0}px", w.outer_border_width),
        EditorAction::WheelOuterBorderWidthDelta { delta: -0.5 },
        EditorAction::WheelOuterBorderWidthDelta { delta: 0.5 },
    );

    // ── INNER CIRCLE ───────────────────────────────────────────────────────────
    section_label(commands, parent, "INNER CIRCLE");
    let inn_card = child(commands, parent, editor_card());

    // Inner Radius
    spawn_stepper_field(
        commands,
        inn_card,
        "Radius",
        &format!("{:.0}", w.inner_radius),
        EditorAction::WheelInnerRadiusDelta { delta: -2.0 },
        EditorAction::WheelInnerRadiusDelta { delta: 2.0 },
    );

    // Inner border color
    {
        let label = if w.inner_border.is_empty() {
            "None".to_string()
        } else {
            w.inner_border.clone()
        };
        let col = if w.inner_border.is_empty() {
            DIMMER
        } else {
            TEXT
        };
        spawn_box_field(
            commands,
            inn_card,
            "Border color",
            &label,
            col,
            BADGE_BORDER,
            EditorAction::CycleInnerBorderColor,
        );
    }

    // Inner border width
    spawn_stepper_field(
        commands,
        inn_card,
        "Border width",
        &format!("{:.0}px", w.inner_border_width),
        EditorAction::WheelInnerBorderWidthDelta { delta: -0.5 },
        EditorAction::WheelInnerBorderWidthDelta { delta: 0.5 },
    );

    // Hub background color
    {
        let label = if w.hub_color.is_empty() {
            "Theme".to_string()
        } else {
            w.hub_color.clone()
        };
        let col = if w.hub_color.is_empty() { DIMMER } else { TEXT };
        spawn_box_field(
            commands,
            inn_card,
            "Hub color",
            &label,
            col,
            BADGE_BORDER,
            EditorAction::CycleWheelHubColor,
        );
    }

    // Hub opacity
    spawn_stepper_field(
        commands,
        inn_card,
        "Hub opacity",
        &format!("{:.0}%", w.hub_opacity * 100.0),
        EditorAction::WheelHubOpacityDelta { delta: -0.05 },
        EditorAction::WheelHubOpacityDelta { delta: 0.05 },
    );

    // ── SEGMENTS ───────────────────────────────────────────────────────────────
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
        if !slot.items.is_empty() {
            child(
                commands,
                left,
                text(&format!("[{}]", slot.items.len()), 9., TEAL),
            );
        }
        if !slot.input.is_empty() {
            let right = child(commands, row, hcluster());
            spawn_input_badge(commands, right, &slot.input, icons);
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
    w: &WheelData,
    icons: &Icons<'_>,
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
    spawn_key_capture_field(
        commands,
        card,
        "Input",
        &inp_d,
        inp_c,
        if inp_kf { AMBER } else { BADGE_BORDER },
        EditorAction::CaptureSlotInput { slot },
        slot_input,
        inp_kf,
        icons,
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
    ws: &WheelSetData,
    icons: &Icons<'_>,
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
    spawn_key_capture_field(
        commands,
        card,
        "Switch Key",
        &kd,
        kc,
        if kf { AMBER } else { BADGE_BORDER },
        EditorAction::CaptureWheelSetSwitchKey { set, entry },
        &ws.switch_key,
        kf,
        icons,
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
        let badge = Badge::None;
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
            icons,
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

fn wheel_at(cfg: &mut QuickActionConfig, sel: Selection) -> Option<&mut WheelData> {
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
        EditFocus::SetBgImage(set) => cfg.sets.get_mut(set).map(|s| &mut s.bg_image),
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
            | EditFocus::WheelSetSwitchKey
            | EditFocus::SlotInput(_)
            | EditFocus::EditShortcut
            | EditFocus::NextWheelKey(_)
            | EditFocus::PrevWheelKey(_)
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
                EditFocus::EditShortcut => cfg.edit_shortcut = label,
                EditFocus::NextWheelKey(set) => {
                    if let Some(s) = cfg.sets.get_mut(set) {
                        s.next_wheel_key = label;
                    }
                }
                EditFocus::PrevWheelKey(set) => {
                    if let Some(s) = cfg.sets.get_mut(set) {
                        s.prev_wheel_key = label;
                    }
                }
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
    if !matches!(
        focus,
        EditFocus::Key
            | EditFocus::SlotInput(_)
            | EditFocus::NextSetKey
            | EditFocus::PrevSetKey
            | EditFocus::EditShortcut
            | EditFocus::WheelSetSwitchKey
            | EditFocus::NextWheelKey(_)
            | EditFocus::PrevWheelKey(_)
    ) {
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
        GamepadButton::DPadUp,
        GamepadButton::DPadDown,
        GamepadButton::DPadLeft,
        GamepadButton::DPadRight,
    ];
    for gamepad in &gamepads {
        for &btn in BUTTONS {
            if gamepad.just_pressed(btn) {
                let label = gamepad_btn_label(btn);
                let gp = format!("GP:{}", label);
                match focus {
                    EditFocus::Key => {
                        if let Selection::Action { set, entry } = ui.selection {
                            if let Some(a) = action_at(&mut cfg, set, entry) {
                                a.key = gp;
                            }
                        }
                    }
                    EditFocus::WheelSetSwitchKey => {
                        if let Selection::WheelSetEntry { set, entry } = ui.selection {
                            if let Some(SetEntry::WheelSet(ws)) =
                                cfg.sets.get_mut(set).and_then(|s| s.entries.get_mut(entry))
                            {
                                ws.switch_key = gp;
                            }
                        }
                    }
                    EditFocus::SlotInput(slot) => {
                        if let Some(w) = wheel_at(&mut cfg, ui.selection) {
                            if let Some(s) = w.slots.get_mut(slot) {
                                s.input = gp;
                            }
                        }
                    }
                    EditFocus::NextSetKey => cfg.next_set_key = gp,
                    EditFocus::PrevSetKey => cfg.prev_set_key = gp,
                    EditFocus::EditShortcut => cfg.edit_shortcut = gp,
                    EditFocus::NextWheelKey(set) => {
                        if let Some(s) = cfg.sets.get_mut(set) {
                            s.next_wheel_key = gp;
                        }
                    }
                    EditFocus::PrevWheelKey(set) => {
                        if let Some(s) = cfg.sets.get_mut(set) {
                            s.prev_wheel_key = gp;
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
        GamepadButton::DPadUp => "DUp".into(),
        GamepadButton::DPadDown => "DDown".into(),
        GamepadButton::DPadLeft => "DLeft".into(),
        GamepadButton::DPadRight => "DRight".into(),
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

/// Watches for the configured edit shortcut and toggles the editor sidebar.
// ─── shortcut helper ────────────────────────────────────────────────────────────

/// Returns `true` if `shortcut` was just pressed this frame.
/// Shortcut format: plain key label (e.g. `"Tab"`) or `"GP:{label}"` for gamepad.
fn shortcut_just_pressed(
    shortcut: &str,
    keys: &ButtonInput<KeyCode>,
    gamepads: &Query<&Gamepad>,
) -> bool {
    if shortcut.is_empty() {
        return false;
    }
    if let Some(btn_name) = shortcut.strip_prefix("GP:") {
        const BTNS: &[GamepadButton] = &[
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
            GamepadButton::DPadUp,
            GamepadButton::DPadDown,
            GamepadButton::DPadLeft,
            GamepadButton::DPadRight,
        ];
        for &btn in BTNS {
            if gamepad_btn_label(btn) == btn_name {
                if gamepads.iter().any(|gp| gp.just_pressed(btn)) {
                    return true;
                }
            }
        }
        false
    } else {
        keys.get_just_pressed()
            .any(|&k| !is_modifier(k) && key_label(k) == shortcut)
    }
}

/// Applies Next Set / Prev Set shortcuts — only while the HUD overlay is open.
/// Gameplay code can freely use the same buttons when the HUD is closed.
fn apply_set_shortcuts(
    keys: Res<ButtonInput<KeyCode>>,
    gamepads: Query<&Gamepad>,
    cfg: Res<QuickActionConfig>,
    mut hud: ResMut<WheelHudState>,
    ui: Res<EditorUiState>,
) {
    if !hud.open || ui.editing != EditFocus::None {
        return;
    }
    if shortcut_just_pressed(&cfg.next_set_key, &keys, &gamepads) {
        let max = cfg.sets.len().saturating_sub(1);
        if hud.active_set < max {
            hud.active_set += 1;
        } else if cfg.cycle_sets {
            hud.active_set = 0;
        }
        hud.active_wheel_entry = 0;
        hud.dirty = true;
    }
    if shortcut_just_pressed(&cfg.prev_set_key, &keys, &gamepads) {
        if hud.active_set > 0 {
            hud.active_set -= 1;
        } else if cfg.cycle_sets && !cfg.sets.is_empty() {
            hud.active_set = cfg.sets.len() - 1;
        }
        hud.active_wheel_entry = 0;
        hud.dirty = true;
    }
}

/// Navigates between wheel entries within the active set using the per-set
/// `next_wheel_key` / `prev_wheel_key` shortcuts.
fn hud_wheel_nav(
    keys: Res<ButtonInput<KeyCode>>,
    gamepads: Query<&Gamepad>,
    cfg: Res<QuickActionConfig>,
    mut hud: ResMut<WheelHudState>,
    ui: Res<EditorUiState>,
) {
    if !hud.open || ui.editing != EditFocus::None {
        return;
    }
    let Some(set) = cfg.sets.get(hud.active_set) else {
        return;
    };
    let n = count_wheel_entries(set);
    if n < 2 {
        return;
    }
    // Clamp in case the set shrank since last frame.
    if hud.active_wheel_entry >= n {
        hud.active_wheel_entry = 0;
        hud.dirty = true;
    }
    if shortcut_just_pressed(&set.next_wheel_key, &keys, &gamepads) {
        if hud.active_wheel_entry + 1 < n {
            hud.active_wheel_entry += 1;
        } else if set.cycle_wheels {
            hud.active_wheel_entry = 0;
        }
        hud.dirty = true;
    }
    if shortcut_just_pressed(&set.prev_wheel_key, &keys, &gamepads) {
        if hud.active_wheel_entry > 0 {
            hud.active_wheel_entry -= 1;
        } else if set.cycle_wheels && n > 0 {
            hud.active_wheel_entry = n - 1;
        }
        hud.dirty = true;
    }
}

/// Toggles the editor sidebar when the configured edit shortcut is pressed.
/// Only fires while the HUD overlay is open — gameplay can reuse the same
/// buttons without conflict.
fn check_edit_shortcut(
    keys: Res<ButtonInput<KeyCode>>,
    gamepads: Query<&Gamepad>,
    cfg: Res<QuickActionConfig>,
    mut hud: ResMut<WheelHudState>,
    mut ui: ResMut<EditorUiState>,
) {
    // Only active when the HUD overlay is visible and no capture is in progress.
    if !hud.open || ui.editing != EditFocus::None || cfg.edit_shortcut.is_empty() {
        return;
    }
    if shortcut_just_pressed(&cfg.edit_shortcut, &keys, &gamepads) {
        hud.editor_open = !hud.editor_open;
        // When closing the editor, reset selection state.
        if !hud.editor_open {
            ui.selection = Selection::None;
            ui.editing = EditFocus::None;
        }
        hud.dirty = true;
        ui.dirty = true;
    }
}
