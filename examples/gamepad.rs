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

#[derive(Component)]
struct SliceVisual {
    index: usize,
}

#[derive(Component)]
struct SliceIcon {
    index: usize,
}

#[derive(Component)]
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
        ))
        .run();
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut mats: ResMut<Assets<ColorMaterial>>,
    config: Res<WheelConfig>,
) {
    commands.spawn(Camera2d);
    spawn_diablo_wheel(&mut commands, &mut meshes, &mut mats, &config);
}

fn spawn_diablo_wheel(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    mats: &mut Assets<ColorMaterial>,
    config: &WheelConfig,
) {
    let menu = WheelMenu {
        slices: config.slices,
        radius: 180.0,
        inner_radius: 60.0,
        deadzone: 0.25,
        gap: config.gap,
    };

    // Spawn wheel root with the menu logic
    let root = commands.spawn((
        WheelRoot,
        menu.clone(),
        WheelState::default(),
        Transform::default(),
        Visibility::Visible,
    )).id();

    // Spawn visual slices
    for i in 0..menu.slices {
        let (a0, a1) = slice_angles(&menu, i);
        let mesh_handle = meshes.add(mesh::wedge(menu.inner_radius, menu.radius, a0, a1));
        let mat_handle = mats.add(ColorMaterial::from_color(diablo_theme::SLICE_BASE));

        // Spawn slice mesh
        let slice_entity = commands.spawn((
            WheelSlice { index: i },
            SliceVisual { index: i },
            Mesh2d(mesh_handle),
            MeshMaterial2d(mat_handle),
            Transform::from_translation(Vec3::Z * 0.0),
        )).id();
        
        commands.entity(root).add_child(slice_entity);

        // Get center position for icon and label
        let center = slice_center(&menu, i);
        let skill = SKILLS.get(i % SKILLS.len()).unwrap_or(&("?", "Unknown"));

        // Spawn icon (using text as icon placeholder)
        let icon_entity = commands.spawn((
            SliceIcon { index: i },
            Text2d::new(skill.0),
            TextFont {
                font_size: 28.0,
                ..default()
            },
            TextColor(diablo_theme::ICON_NORMAL),
            Transform::from_translation(Vec3::new(center.x, center.y + 10.0, 1.0)),
        )).id();
        
        commands.entity(root).add_child(icon_entity);

        // Spawn label
        let label_entity = commands.spawn((
            SliceLabel { index: i },
            Text2d::new(skill.1),
            TextFont {
                font_size: 12.0,
                ..default()
            },
            TextColor(diablo_theme::TEXT_NORMAL),
            Transform::from_translation(Vec3::new(center.x, center.y - 14.0, 1.0)),
        )).id();
        
        commands.entity(root).add_child(label_entity);
    }

    // Spawn center decoration (dark circle)
    let center_mesh = meshes.add(Circle::new(menu.inner_radius - 5.0));
    let center_mat = mats.add(ColorMaterial::from_color(diablo_theme::BACKGROUND));
    let center_entity = commands.spawn((
        Mesh2d(center_mesh),
        MeshMaterial2d(center_mat),
        Transform::from_translation(Vec3::Z * 2.0),
    )).id();
    
    commands.entity(root).add_child(center_entity);
}

fn despawn_wheel(commands: &mut Commands, wheel_query: &Query<Entity, With<WheelRoot>>) {
    for entity in wheel_query.iter() {
        commands.entity(entity).despawn();
    }
}

fn adjust_wheel_config(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut mats: ResMut<Assets<ColorMaterial>>,
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
        spawn_diablo_wheel(&mut commands, &mut meshes, &mut mats, &config);
        
        info!("Wheel: {} slices, gap: {:.2}", config.slices, config.gap);
    }
}

fn on_hover_changed(
    mut hover_events: MessageReader<WheelMenuHoverChanged>,
    mut slice_visuals: Query<(&SliceVisual, &MeshMaterial2d<ColorMaterial>)>,
    mut slice_icons: Query<(&SliceIcon, &mut TextColor), Without<SliceLabel>>,
    mut slice_labels: Query<(&SliceLabel, &mut TextColor), Without<SliceIcon>>,
    mut mats: ResMut<Assets<ColorMaterial>>,
) {
    for event in hover_events.read() {
        // Update slice colors
        for (visual, mat_handle) in &mut slice_visuals {
            if let Some(mat) = mats.get_mut(&mat_handle.0) {
                let is_hovered = event.current == Some(visual.index);
                mat.color = if is_hovered {
                    diablo_theme::SLICE_HOVER
                } else {
                    diablo_theme::SLICE_BASE
                };
            }
        }
        
        // Update icon colors
        for (icon, mut color) in &mut slice_icons {
            let is_hovered = event.current == Some(icon.index);
            color.0 = if is_hovered {
                diablo_theme::ICON_HOVER
            } else {
                diablo_theme::ICON_NORMAL
            };
        }
        
        // Update label colors
        for (label, mut color) in &mut slice_labels {
            let is_hovered = event.current == Some(label.index);
            color.0 = if is_hovered {
                diablo_theme::TEXT_HOVER
            } else {
                diablo_theme::TEXT_NORMAL
            };
        }
    }
}

fn on_select(mut select_events: MessageReader<WheelMenuSelected>) {
    for event in select_events.read() {
        let skill = SKILLS.get(event.index % SKILLS.len()).unwrap_or(&("?", "Unknown"));
        info!("Selected skill: {} ({})", skill.1, skill.0);
    }
}
