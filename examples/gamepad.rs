//! Diablo-style wheel menu example with icons and labels.
//!
//! Controls:
//! - Left Stick: Navigate the wheel
//! - A/South Button: Select
//! - D-Pad Up/Down: Increase/decrease number of slices
//! - D-Pad Left/Right: Increase/decrease gap between slices

use bevy::prelude::*;
use bevy_wheel_menu::*;

// Diablo-style color theme
mod diablo_theme {
    use bevy::prelude::*;
    
    pub const BACKGROUND: Color = Color::srgba(0.05, 0.02, 0.02, 0.95);
    pub const SLICE_BASE: Color = Color::srgba(0.12, 0.08, 0.06, 0.9);
    pub const SLICE_HOVER: Color = Color::srgba(0.6, 0.15, 0.05, 0.95);
    pub const TEXT_NORMAL: Color = Color::srgba(0.8, 0.7, 0.5, 1.0);
    pub const TEXT_HOVER: Color = Color::srgba(1.0, 0.85, 0.4, 1.0);
    pub const ICON_NORMAL: Color = Color::srgba(0.7, 0.6, 0.4, 1.0);
    pub const ICON_HOVER: Color = Color::srgba(1.0, 0.8, 0.3, 1.0);
}

// Skill definitions for the wheel
const SKILLS: &[(&str, &str)] = &[
    ("⚔", "Attack"),
    ("🛡", "Defend"),
    ("✨", "Magic"),
    ("💊", "Potion"),
    ("🏃", "Dodge"),
    ("🔥", "Fire"),
    ("❄", "Ice"),
    ("⚡", "Lightning"),
    ("💀", "Death"),
    ("💚", "Heal"),
    ("🌀", "Vortex"),
    ("👁", "Vision"),
    ("", "Stealth"),
    ("🗡", "Backstab"),
    ("🕯", "Light"),
    ("🌪", "Wind"),
];

#[derive(Resource)]
struct WheelConfig {
    slices: usize,
    gap: f32,
}

impl Default for WheelConfig {
    fn default() -> Self {
        Self {
            slices: 8,
            gap: 0.04,
        }
    }
}

#[derive(Component, Clone, Default)]
struct SliceVisual {
    index: usize,
}

#[derive(Component, Clone, Default)]
struct SliceIcon {
    index: usize,
}

#[derive(Component, Clone, Default)]
struct SliceLabel {
    index: usize,
}

#[derive(Component)]
struct WheelRoot;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Diablo Wheel Menu".into(),
                resolution: (800, 600).into(),
                ..default()
            }),
            ..default()
        }))
        .add_plugins(WheelMenuPlugin)
        .init_resource::<WheelConfig>()
        .add_systems(Startup, setup)
        .add_systems(Update, (
            adjust_wheel_config,
            on_hover_changed,
            on_select,
            on_low_count,
        ))
        .run();
}

fn setup(mut commands: Commands, config: Res<WheelConfig>) {
    commands.spawn(Camera2d);
    spawn_diablo_wheel(&mut commands, &config);
}

fn spawn_diablo_wheel(commands: &mut Commands, config: &WheelConfig) {
    let menu = WheelMenu {
        slices: config.slices,
        radius: 180.0,
        inner_radius: 60.0,
        deadzone: 0.25,
        gap: config.gap,
        ..default()
    };

    // Full-screen UI overlay (from the library, authored with `bsn!`) that also
    // carries the wheel-menu logic components.
    // release_to_use: navigate to a skill, release the stick → fires selection.
    let root = commands
        .spawn_scene(wheel_overlay())
        .insert((
            WheelRoot,
            menu.clone(),
            WheelState::default(),
            WheelMenuConfig { casting_mode: CastingMode::ReleaseToUse, auto_snap: true, ..default() },
        ))
        .id();

    // Zero-size hub at the screen center; absolutely-positioned slices are laid
    // out relative to it.
    let hub = commands.spawn_scene(wheel_hub()).id();
    commands.entity(root).add_child(hub);

    // Center decoration disc.
    let disc = (menu.inner_radius - 5.0).max(1.0);
    let center = commands.spawn_scene(wheel_center_disc(disc, diablo_theme::BACKGROUND)).id();
    commands.entity(hub).add_child(center);

    // One rounded UI panel per slice, placed radially around the hub.
    let size = 84.0_f32;
    for i in 0..menu.slices {
        let skill = SKILLS.get(i % SKILLS.len()).copied().unwrap_or(("?", "Unknown"));
        let icon = skill.0.to_string();
        let label = skill.1.to_string();

        // Library builds the positioned, rounded panel via `bsn!`.
        // Each skill has 10 uses; WheelSliceCount feeds the library's low-count system.
        let slice = commands
            .spawn_scene(wheel_slice_panel(&menu, i, size, diablo_theme::SLICE_BASE))
            .insert((
                SliceVisual { index: i },
                WheelSlice { index: i },
                WheelSliceCount { current: 10, max: 10, low_threshold: 3, ..default() },
            ))
            .id();

        let icon_entity = commands
            .spawn_scene(wheel_slice_icon(icon, 30.0, diablo_theme::ICON_NORMAL))
            .insert(SliceIcon { index: i })
            .id();
        let label_entity = commands
            .spawn_scene(wheel_slice_label(label, 13.0, diablo_theme::TEXT_NORMAL))
            .insert(SliceLabel { index: i })
            .id();

        commands.entity(slice).add_children(&[icon_entity, label_entity]);
        commands.entity(hub).add_child(slice);
    }
}

fn despawn_wheel(commands: &mut Commands, wheel_query: &Query<Entity, With<WheelRoot>>) {
    for entity in wheel_query.iter() {
        commands.entity(entity).despawn();
    }
}

fn adjust_wheel_config(
    mut commands: Commands,
    gamepads: Query<&Gamepad>,
    wheel_query: Query<Entity, With<WheelRoot>>,
    mut config: ResMut<WheelConfig>,
) {
    let mut slice_delta: i32 = 0;
    let mut gap_delta: f32 = 0.0;
    
    for gamepad in &gamepads {
        if gamepad.just_pressed(GamepadButton::DPadUp) {
            slice_delta += 1;
        }
        if gamepad.just_pressed(GamepadButton::DPadDown) {
            slice_delta -= 1;
        }
        if gamepad.just_pressed(GamepadButton::DPadRight) {
            gap_delta += 0.02;
        }
        if gamepad.just_pressed(GamepadButton::DPadLeft) {
            gap_delta -= 0.02;
        }
    }
    
    let new_slices = (config.slices as i32 + slice_delta).max(2) as usize;
    let new_gap = (config.gap + gap_delta).clamp(0.0, 0.2);
    
    if new_slices != config.slices || (new_gap - config.gap).abs() > 0.001 {
        config.slices = new_slices;
        config.gap = new_gap;
        
        despawn_wheel(&mut commands, &wheel_query);
        spawn_diablo_wheel(&mut commands, &config);
        
        info!("Wheel: {} slices, gap: {:.2}", config.slices, config.gap);
    }
}

fn on_hover_changed(
    mut hover_events: MessageReader<WheelMenuHoverChanged>,
    mut slice_visuals: Query<(&SliceVisual, &mut BackgroundColor)>,
    mut slice_icons: Query<(&SliceIcon, &mut TextColor), Without<SliceLabel>>,
    mut slice_labels: Query<(&SliceLabel, &mut TextColor), Without<SliceIcon>>,
) {
    for event in hover_events.read() {
        // Update slice background colors
        for (visual, mut bg) in &mut slice_visuals {
            bg.0 = if event.current == Some(visual.index) {
                diablo_theme::SLICE_HOVER
            } else {
                diablo_theme::SLICE_BASE
            };
        }

        // Update icon colors
        for (icon, mut color) in &mut slice_icons {
            color.0 = if event.current == Some(icon.index) {
                diablo_theme::ICON_HOVER
            } else {
                diablo_theme::ICON_NORMAL
            };
        }

        // Update label colors
        for (label, mut color) in &mut slice_labels {
            color.0 = if event.current == Some(label.index) {
                diablo_theme::TEXT_HOVER
            } else {
                diablo_theme::TEXT_NORMAL
            };
        }
    }
}

fn on_select(
    mut select_events: MessageReader<WheelMenuSelected>,
    mut slice_counts: Query<(&WheelSlice, &mut WheelSliceCount)>,
) {
    for event in select_events.read() {
        let skill = SKILLS.get(event.index % SKILLS.len()).unwrap_or(&("?", "Unknown"));
        info!("Selected skill: {} ({})", skill.1, skill.0);
        // Decrement the use count; the library will emit WheelMenuLowCount
        // when it crosses the threshold.
        for (slice, mut count) in &mut slice_counts {
            if slice.index == event.index && count.current > 0 {
                count.current -= 1;
                info!("  {} uses remaining", count.current);
            }
        }
    }
}

/// Tints a slice amber when the library emits a low-count warning.
fn on_low_count(
    mut low_events: MessageReader<WheelMenuLowCount>,
    mut slice_visuals: Query<(&SliceVisual, &mut BackgroundColor)>,
) {
    for event in low_events.read() {
        for (visual, mut bg) in &mut slice_visuals {
            if visual.index == event.index {
                bg.0 = Color::srgba(0.55, 0.22, 0.02, 0.95);
                info!("Skill {} is running low ({} uses left)", event.index, event.current);
            }
        }
    }
}
