//! FPS-style wheel menu example with weapon selection and abilities.
//!
//! This example demonstrates using bevy_wheel_menu in a first-person shooter context
//! with gamepad support for weapon switching, abilities, and quick actions.
//!
//! Controls:
//! - Left Stick: Move player
//! - Right Stick: Look around
//! - Left Bumper (Hold): Open weapon wheel
//! - Right Bumper (Hold): Open ability wheel
//! - Left Stick (while wheel open): Navigate wheel
//! - A/South Button: Select item from wheel
//! - Right Trigger: Shoot

use bevy::prelude::*;
use bevy_wheel_menu::*;

// Modern FPS color theme (Apex Legends / Destiny inspired)
mod fps_theme {
    use bevy::prelude::*;

    pub const BACKGROUND: Color = Color::srgba(0.02, 0.02, 0.05, 0.92);
    pub const SLICE_BASE: Color = Color::srgba(0.08, 0.12, 0.18, 0.85);
    pub const SLICE_HOVER: Color = Color::srgba(0.2, 0.5, 0.9, 0.95);
    pub const SLICE_EQUIPPED: Color = Color::srgba(0.1, 0.7, 0.4, 0.9);
    pub const TEXT_NORMAL: Color = Color::srgba(0.7, 0.75, 0.8, 1.0);
    pub const TEXT_HOVER: Color = Color::srgba(1.0, 1.0, 1.0, 1.0);
    pub const ICON_NORMAL: Color = Color::srgba(0.6, 0.65, 0.7, 1.0);
    pub const ICON_HOVER: Color = Color::srgba(1.0, 0.95, 0.8, 1.0);
    pub const AMMO_TEXT: Color = Color::srgba(0.9, 0.7, 0.2, 1.0);
    pub const CROSSHAIR: Color = Color::srgba(0.9, 0.9, 0.9, 0.8);
    pub const HUD_TEXT: Color = Color::srgba(0.85, 0.85, 0.9, 1.0);
}

// Weapon definitions
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
enum WeaponType {
    Pistol,
    AssaultRifle,
    Shotgun,
    Sniper,
    RocketLauncher,
    SMG,
    LMG,
    Knife,
}

impl WeaponType {
    fn icon(&self) -> &'static str {
        match self {
            Self::Pistol => "🔫",
            Self::AssaultRifle => "🎯",
            Self::Shotgun => "💥",
            Self::Sniper => "🔭",
            Self::RocketLauncher => "🚀",
            Self::SMG => "⚡",
            Self::LMG => "🔥",
            Self::Knife => "🗡",
        }
    }

    fn name(&self) -> &'static str {
        match self {
            Self::Pistol => "Pistol",
            Self::AssaultRifle => "AR-15",
            Self::Shotgun => "Shotgun",
            Self::Sniper => "Sniper",
            Self::RocketLauncher => "RPG",
            Self::SMG => "SMG",
            Self::LMG => "LMG",
            Self::Knife => "Knife",
        }
    }

    fn max_ammo(&self) -> u32 {
        match self {
            Self::Pistol => 15,
            Self::AssaultRifle => 30,
            Self::Shotgun => 8,
            Self::Sniper => 5,
            Self::RocketLauncher => 3,
            Self::SMG => 25,
            Self::LMG => 100,
            Self::Knife => 0,
        }
    }
}

const WEAPONS: &[WeaponType] = &[
    WeaponType::Pistol,
    WeaponType::AssaultRifle,
    WeaponType::Shotgun,
    WeaponType::Sniper,
    WeaponType::RocketLauncher,
    WeaponType::SMG,
    WeaponType::LMG,
    WeaponType::Knife,
];

// Ability definitions
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
enum AbilityType {
    Grenade,
    Flashbang,
    Smoke,
    Heal,
    Sprint,
    Shield,
}

impl AbilityType {
    fn icon(&self) -> &'static str {
        match self {
            Self::Grenade => "💣",
            Self::Flashbang => "💡",
            Self::Smoke => "🌫",
            Self::Heal => "💚",
            Self::Sprint => "🏃",
            Self::Shield => "🛡",
        }
    }

    fn name(&self) -> &'static str {
        match self {
            Self::Grenade => "Frag",
            Self::Flashbang => "Flash",
            Self::Smoke => "Smoke",
            Self::Heal => "Med Kit",
            Self::Sprint => "Sprint",
            Self::Shield => "Shield",
        }
    }

    fn cooldown(&self) -> f32 {
        match self {
            Self::Grenade => 15.0,
            Self::Flashbang => 12.0,
            Self::Smoke => 18.0,
            Self::Heal => 20.0,
            Self::Sprint => 8.0,
            Self::Shield => 25.0,
        }
    }
}

const ABILITIES: &[AbilityType] = &[
    AbilityType::Grenade,
    AbilityType::Flashbang,
    AbilityType::Smoke,
    AbilityType::Heal,
    AbilityType::Sprint,
    AbilityType::Shield,
];

// Player state
#[derive(Resource)]
struct PlayerState {
    current_weapon: WeaponType,
    ammo: std::collections::HashMap<WeaponType, u32>,
    health: f32,
    shield: f32,
    ability_cooldowns: std::collections::HashMap<AbilityType, f32>,
    position: Vec2,
    look_angle: f32,
}

impl Default for PlayerState {
    fn default() -> Self {
        let mut ammo = std::collections::HashMap::new();
        for weapon in WEAPONS {
            ammo.insert(*weapon, weapon.max_ammo());
        }

        let mut ability_cooldowns = std::collections::HashMap::new();
        for ability in ABILITIES {
            ability_cooldowns.insert(*ability, 0.0);
        }

        Self {
            current_weapon: WeaponType::AssaultRifle,
            ammo,
            health: 100.0,
            shield: 50.0,
            ability_cooldowns,
            position: Vec2::ZERO,
            look_angle: 0.0,
        }
    }
}

// Wheel types
#[derive(Clone, Copy, PartialEq, Eq)]
enum WheelType {
    Weapon,
    Ability,
}

#[derive(Resource, Default)]
struct ActiveWheel {
    wheel_type: Option<WheelType>,
}

// Components
#[derive(Component)]
struct WheelRoot {
    wheel_type: WheelType,
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
struct SliceAmmoText {
    index: usize,
}

#[derive(Component)]
struct Crosshair;

#[derive(Component)]
struct HudElement;

#[derive(Component)]
struct WeaponDisplay;

#[derive(Component)]
struct HealthDisplay;

#[derive(Component)]
struct AmmoDisplay;

#[derive(Component)]
struct PlayerMarker;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "FPS Wheel Menu".into(),
                resolution: (1280, 720).into(),
                ..default()
            }),
            ..default()
        }))
        .add_plugins(WheelMenuPlugin)
        .init_resource::<PlayerState>()
        .init_resource::<ActiveWheel>()
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (
                handle_wheel_toggle,
                handle_player_movement,
                handle_shooting,
                update_ability_cooldowns,
                update_hud,
                on_hover_changed,
                on_wheel_select,
            ),
        )
        .run();
}

fn setup(mut commands: Commands) {
    commands.spawn(Camera2d);

    // Spawn HUD elements
    spawn_hud(&mut commands);

    // Spawn crosshair
    spawn_crosshair(&mut commands);

    // Spawn player marker
    spawn_player_marker(&mut commands);
}

fn spawn_hud(commands: &mut Commands) {
    // Top-left: Health and Shield
    commands.spawn((
        HudElement,
        HealthDisplay,
        Text2d::new("HP: 100 | Shield: 50"),
        TextFont {
            font_size: 20.0,
            ..default()
        },
        TextColor(fps_theme::HUD_TEXT),
        Transform::from_translation(Vec3::new(-550.0, 320.0, 10.0)),
    ));

    // Bottom-right: Current weapon and ammo
    commands.spawn((
        HudElement,
        WeaponDisplay,
        Text2d::new("🎯 AR-15"),
        TextFont {
            font_size: 24.0,
            ..default()
        },
        TextColor(fps_theme::HUD_TEXT),
        Transform::from_translation(Vec3::new(480.0, -300.0, 10.0)),
    ));

    commands.spawn((
        HudElement,
        AmmoDisplay,
        Text2d::new("30/30"),
        TextFont {
            font_size: 18.0,
            ..default()
        },
        TextColor(fps_theme::AMMO_TEXT),
        Transform::from_translation(Vec3::new(480.0, -330.0, 10.0)),
    ));

    // Bottom center: Controls hint
    commands.spawn((
        HudElement,
        Text2d::new("LB: Weapons | RB: Abilities | RT: Shoot"),
        TextFont {
            font_size: 14.0,
            ..default()
        },
        TextColor(Color::srgba(0.6, 0.6, 0.6, 0.8)),
        Transform::from_translation(Vec3::new(0.0, -340.0, 10.0)),
    ));
}

fn spawn_crosshair(commands: &mut Commands) {
    // Simple crosshair using text
    commands.spawn((
        Crosshair,
        Text2d::new("+"),
        TextFont {
            font_size: 32.0,
            ..default()
        },
        TextColor(fps_theme::CROSSHAIR),
        Transform::from_translation(Vec3::new(0.0, 0.0, 5.0)),
    ));
}

fn spawn_player_marker(commands: &mut Commands) {
    // Player direction indicator
    commands.spawn((
        PlayerMarker,
        Text2d::new("▲"),
        TextFont {
            font_size: 24.0,
            ..default()
        },
        TextColor(Color::srgba(0.3, 0.8, 0.3, 0.9)),
        Transform::from_translation(Vec3::new(0.0, -200.0, 5.0)),
    ));
}

fn spawn_weapon_wheel(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    mats: &mut Assets<ColorMaterial>,
    player_state: &PlayerState,
) {
    let menu = WheelMenu {
        slices: WEAPONS.len(),
        radius: 200.0,
        inner_radius: 70.0,
        deadzone: 0.3,
        gap: 0.03,
    };

    let root = commands
        .spawn((
            WheelRoot {
                wheel_type: WheelType::Weapon,
            },
            menu.clone(),
            WheelState::default(),
            Transform::default(),
            Visibility::Visible,
        ))
        .id();

    for (i, weapon) in WEAPONS.iter().enumerate() {
        let (a0, a1) = slice_angles(&menu, i);
        let mesh_handle = meshes.add(mesh::wedge(menu.inner_radius, menu.radius, a0, a1));

        // Highlight currently equipped weapon
        let base_color = if *weapon == player_state.current_weapon {
            fps_theme::SLICE_EQUIPPED
        } else {
            fps_theme::SLICE_BASE
        };
        let mat_handle = mats.add(ColorMaterial::from_color(base_color));

        let slice_entity = commands
            .spawn((
                WheelSlice { index: i },
                SliceVisual { index: i },
                Mesh2d(mesh_handle),
                MeshMaterial2d(mat_handle),
                Transform::from_translation(Vec3::Z * 0.0),
            ))
            .id();

        commands.entity(root).add_child(slice_entity);

        let center = slice_center(&menu, i);

        // Icon
        let icon_entity = commands
            .spawn((
                SliceIcon { index: i },
                Text2d::new(weapon.icon()),
                TextFont {
                    font_size: 26.0,
                    ..default()
                },
                TextColor(fps_theme::ICON_NORMAL),
                Transform::from_translation(Vec3::new(center.x, center.y + 15.0, 1.0)),
            ))
            .id();

        commands.entity(root).add_child(icon_entity);

        // Label
        let label_entity = commands
            .spawn((
                SliceLabel { index: i },
                Text2d::new(weapon.name()),
                TextFont {
                    font_size: 11.0,
                    ..default()
                },
                TextColor(fps_theme::TEXT_NORMAL),
                Transform::from_translation(Vec3::new(center.x, center.y - 5.0, 1.0)),
            ))
            .id();

        commands.entity(root).add_child(label_entity);

        // Ammo count
        let ammo = player_state.ammo.get(weapon).copied().unwrap_or(0);
        let ammo_text = if weapon.max_ammo() > 0 {
            format!("{}/{}", ammo, weapon.max_ammo())
        } else {
            "∞".to_string()
        };

        let ammo_entity = commands
            .spawn((
                SliceAmmoText { index: i },
                Text2d::new(ammo_text),
                TextFont {
                    font_size: 9.0,
                    ..default()
                },
                TextColor(fps_theme::AMMO_TEXT),
                Transform::from_translation(Vec3::new(center.x, center.y - 18.0, 1.0)),
            ))
            .id();

        commands.entity(root).add_child(ammo_entity);
    }

    // Center circle
    let center_mesh = meshes.add(Circle::new(menu.inner_radius - 8.0));
    let center_mat = mats.add(ColorMaterial::from_color(fps_theme::BACKGROUND));
    let center_entity = commands
        .spawn((
            Mesh2d(center_mesh),
            MeshMaterial2d(center_mat),
            Transform::from_translation(Vec3::Z * 2.0),
        ))
        .id();

    commands.entity(root).add_child(center_entity);

    // Center text
    let center_text = commands
        .spawn((
            Text2d::new("WEAPONS"),
            TextFont {
                font_size: 14.0,
                ..default()
            },
            TextColor(fps_theme::TEXT_NORMAL),
            Transform::from_translation(Vec3::Z * 3.0),
        ))
        .id();

    commands.entity(root).add_child(center_text);
}

fn spawn_ability_wheel(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    mats: &mut Assets<ColorMaterial>,
    player_state: &PlayerState,
) {
    let menu = WheelMenu {
        slices: ABILITIES.len(),
        radius: 180.0,
        inner_radius: 60.0,
        deadzone: 0.3,
        gap: 0.04,
    };

    let root = commands
        .spawn((
            WheelRoot {
                wheel_type: WheelType::Ability,
            },
            menu.clone(),
            WheelState::default(),
            Transform::default(),
            Visibility::Visible,
        ))
        .id();

    for (i, ability) in ABILITIES.iter().enumerate() {
        let (a0, a1) = slice_angles(&menu, i);
        let mesh_handle = meshes.add(mesh::wedge(menu.inner_radius, menu.radius, a0, a1));

        let cooldown = player_state
            .ability_cooldowns
            .get(ability)
            .copied()
            .unwrap_or(0.0);
        let base_color = if cooldown > 0.0 {
            Color::srgba(0.3, 0.3, 0.3, 0.7)
        } else {
            fps_theme::SLICE_BASE
        };
        let mat_handle = mats.add(ColorMaterial::from_color(base_color));

        let slice_entity = commands
            .spawn((
                WheelSlice { index: i },
                SliceVisual { index: i },
                Mesh2d(mesh_handle),
                MeshMaterial2d(mat_handle),
                Transform::from_translation(Vec3::Z * 0.0),
            ))
            .id();

        commands.entity(root).add_child(slice_entity);

        let center = slice_center(&menu, i);

        // Icon
        let icon_entity = commands
            .spawn((
                SliceIcon { index: i },
                Text2d::new(ability.icon()),
                TextFont {
                    font_size: 28.0,
                    ..default()
                },
                TextColor(if cooldown > 0.0 {
                    Color::srgba(0.4, 0.4, 0.4, 0.8)
                } else {
                    fps_theme::ICON_NORMAL
                }),
                Transform::from_translation(Vec3::new(center.x, center.y + 12.0, 1.0)),
            ))
            .id();

        commands.entity(root).add_child(icon_entity);

        // Label
        let label_entity = commands
            .spawn((
                SliceLabel { index: i },
                Text2d::new(ability.name()),
                TextFont {
                    font_size: 11.0,
                    ..default()
                },
                TextColor(fps_theme::TEXT_NORMAL),
                Transform::from_translation(Vec3::new(center.x, center.y - 8.0, 1.0)),
            ))
            .id();

        commands.entity(root).add_child(label_entity);

        // Cooldown indicator
        let cooldown_text = if cooldown > 0.0 {
            format!("{:.1}s", cooldown)
        } else {
            "Ready".to_string()
        };

        let cooldown_entity = commands
            .spawn((
                SliceAmmoText { index: i },
                Text2d::new(cooldown_text),
                TextFont {
                    font_size: 9.0,
                    ..default()
                },
                TextColor(if cooldown > 0.0 {
                    Color::srgba(1.0, 0.3, 0.3, 1.0)
                } else {
                    Color::srgba(0.3, 1.0, 0.3, 1.0)
                }),
                Transform::from_translation(Vec3::new(center.x, center.y - 20.0, 1.0)),
            ))
            .id();

        commands.entity(root).add_child(cooldown_entity);
    }

    // Center circle
    let center_mesh = meshes.add(Circle::new(menu.inner_radius - 5.0));
    let center_mat = mats.add(ColorMaterial::from_color(fps_theme::BACKGROUND));
    let center_entity = commands
        .spawn((
            Mesh2d(center_mesh),
            MeshMaterial2d(center_mat),
            Transform::from_translation(Vec3::Z * 2.0),
        ))
        .id();

    commands.entity(root).add_child(center_entity);

    // Center text
    let center_text = commands
        .spawn((
            Text2d::new("ABILITIES"),
            TextFont {
                font_size: 13.0,
                ..default()
            },
            TextColor(fps_theme::TEXT_NORMAL),
            Transform::from_translation(Vec3::Z * 3.0),
        ))
        .id();

    commands.entity(root).add_child(center_text);
}

fn despawn_wheels(commands: &mut Commands, wheel_query: &Query<Entity, With<WheelRoot>>) {
    for entity in wheel_query.iter() {
        commands.entity(entity).despawn();
    }
}

fn handle_wheel_toggle(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut mats: ResMut<Assets<ColorMaterial>>,
    gamepads: Query<&Gamepad>,
    wheel_query: Query<Entity, With<WheelRoot>>,
    mut active_wheel: ResMut<ActiveWheel>,
    player_state: Res<PlayerState>,
) {
    let mut lb_held = false;
    let mut rb_held = false;

    for gamepad in &gamepads {
        lb_held = lb_held || gamepad.pressed(GamepadButton::LeftTrigger);
        rb_held = rb_held || gamepad.pressed(GamepadButton::RightTrigger);
    }

    let desired_wheel = if lb_held {
        Some(WheelType::Weapon)
    } else if rb_held {
        Some(WheelType::Ability)
    } else {
        None
    };

    if desired_wheel != active_wheel.wheel_type {
        despawn_wheels(&mut commands, &wheel_query);

        match desired_wheel {
            Some(WheelType::Weapon) => {
                spawn_weapon_wheel(&mut commands, &mut meshes, &mut mats, &player_state);
                info!("Opened weapon wheel");
            }
            Some(WheelType::Ability) => {
                spawn_ability_wheel(&mut commands, &mut meshes, &mut mats, &player_state);
                info!("Opened ability wheel");
            }
            None => {
                if active_wheel.wheel_type.is_some() {
                    info!("Closed wheel");
                }
            }
        }

        active_wheel.wheel_type = desired_wheel;
    }
}

fn handle_player_movement(
    gamepads: Query<&Gamepad>,
    time: Res<Time>,
    mut player_state: ResMut<PlayerState>,
    active_wheel: Res<ActiveWheel>,
    mut player_marker: Query<&mut Transform, With<PlayerMarker>>,
) {
    // Don't move while wheel is open
    if active_wheel.wheel_type.is_some() {
        return;
    }

    for gamepad in &gamepads {
        // Movement with left stick
        let move_x = gamepad.get(GamepadAxis::LeftStickX).unwrap_or(0.0);
        let move_y = gamepad.get(GamepadAxis::LeftStickY).unwrap_or(0.0);

        if move_x.abs() > 0.1 || move_y.abs() > 0.1 {
            let speed = 200.0;
            player_state.position.x += move_x * speed * time.delta_secs();
            player_state.position.y += move_y * speed * time.delta_secs();
        }

        // Look with right stick
        let look_x = gamepad.get(GamepadAxis::RightStickX).unwrap_or(0.0);
        let look_y = gamepad.get(GamepadAxis::RightStickY).unwrap_or(0.0);

        if look_x.abs() > 0.2 || look_y.abs() > 0.2 {
            player_state.look_angle = look_y.atan2(look_x);
        }
    }

    // Update player marker position and rotation
    for mut transform in &mut player_marker {
        transform.translation.x = player_state.position.x;
        transform.translation.y = player_state.position.y - 200.0;
        transform.rotation = Quat::from_rotation_z(player_state.look_angle - std::f32::consts::FRAC_PI_2);
    }
}

fn handle_shooting(
    gamepads: Query<&Gamepad>,
    mut player_state: ResMut<PlayerState>,
    active_wheel: Res<ActiveWheel>,
) {
    // Don't shoot while wheel is open
    if active_wheel.wheel_type.is_some() {
        return;
    }

    for gamepad in &gamepads {
        let trigger = gamepad.get(GamepadAxis::RightZ).unwrap_or(0.0);

        if trigger > 0.5 {
            let weapon = player_state.current_weapon;
            if weapon.max_ammo() > 0 {
                if let Some(ammo) = player_state.ammo.get_mut(&weapon) {
                    if *ammo > 0 {
                        *ammo -= 1;
                        // Fire logic would go here
                    }
                }
            }
        }
    }
}

fn update_ability_cooldowns(time: Res<Time>, mut player_state: ResMut<PlayerState>) {
    for cooldown in player_state.ability_cooldowns.values_mut() {
        if *cooldown > 0.0 {
            *cooldown = (*cooldown - time.delta_secs()).max(0.0);
        }
    }
}

fn update_hud(
    player_state: Res<PlayerState>,
    mut health_query: Query<&mut Text2d, (With<HealthDisplay>, Without<WeaponDisplay>, Without<AmmoDisplay>)>,
    mut weapon_query: Query<&mut Text2d, (With<WeaponDisplay>, Without<HealthDisplay>, Without<AmmoDisplay>)>,
    mut ammo_query: Query<&mut Text2d, (With<AmmoDisplay>, Without<HealthDisplay>, Without<WeaponDisplay>)>,
) {
    for mut text in &mut health_query {
        text.0 = format!(
            "HP: {:.0} | Shield: {:.0}",
            player_state.health, player_state.shield
        );
    }

    for mut text in &mut weapon_query {
        text.0 = format!(
            "{} {}",
            player_state.current_weapon.icon(),
            player_state.current_weapon.name()
        );
    }

    for mut text in &mut ammo_query {
        let weapon = player_state.current_weapon;
        let ammo = player_state.ammo.get(&weapon).copied().unwrap_or(0);
        if weapon.max_ammo() > 0 {
            text.0 = format!("{}/{}", ammo, weapon.max_ammo());
        } else {
            text.0 = "∞".to_string();
        }
    }
}

fn on_hover_changed(
    mut hover_events: MessageReader<WheelMenuHoverChanged>,
    mut slice_visuals: Query<(&SliceVisual, &MeshMaterial2d<ColorMaterial>)>,
    mut slice_icons: Query<(&SliceIcon, &mut TextColor), Without<SliceLabel>>,
    mut slice_labels: Query<(&SliceLabel, &mut TextColor), Without<SliceIcon>>,
    mut mats: ResMut<Assets<ColorMaterial>>,
    active_wheel: Res<ActiveWheel>,
    player_state: Res<PlayerState>,
) {
    for event in hover_events.read() {
        for (visual, mat_handle) in &mut slice_visuals {
            if let Some(mat) = mats.get_mut(&mat_handle.0) {
                let is_hovered = event.current == Some(visual.index);

                // Check if this slice is the equipped weapon (for weapon wheel)
                let is_equipped = match active_wheel.wheel_type {
                    Some(WheelType::Weapon) => {
                        WEAPONS
                            .get(visual.index)
                            .map(|w| *w == player_state.current_weapon)
                            .unwrap_or(false)
                    }
                    _ => false,
                };

                mat.color = if is_hovered {
                    fps_theme::SLICE_HOVER
                } else if is_equipped {
                    fps_theme::SLICE_EQUIPPED
                } else {
                    fps_theme::SLICE_BASE
                };
            }
        }

        for (icon, mut color) in &mut slice_icons {
            let is_hovered = event.current == Some(icon.index);
            color.0 = if is_hovered {
                fps_theme::ICON_HOVER
            } else {
                fps_theme::ICON_NORMAL
            };
        }

        for (label, mut color) in &mut slice_labels {
            let is_hovered = event.current == Some(label.index);
            color.0 = if is_hovered {
                fps_theme::TEXT_HOVER
            } else {
                fps_theme::TEXT_NORMAL
            };
        }
    }
}

fn on_wheel_select(
    mut select_events: MessageReader<WheelMenuSelected>,
    active_wheel: Res<ActiveWheel>,
    mut player_state: ResMut<PlayerState>,
) {
    for event in select_events.read() {
        match active_wheel.wheel_type {
            Some(WheelType::Weapon) => {
                if let Some(weapon) = WEAPONS.get(event.index) {
                    player_state.current_weapon = *weapon;
                    info!("Equipped weapon: {} {}", weapon.icon(), weapon.name());
                }
            }
            Some(WheelType::Ability) => {
                if let Some(ability) = ABILITIES.get(event.index) {
                    let cooldown = player_state
                        .ability_cooldowns
                        .get(ability)
                        .copied()
                        .unwrap_or(0.0);

                    if cooldown <= 0.0 {
                        player_state
                            .ability_cooldowns
                            .insert(*ability, ability.cooldown());
                        info!("Used ability: {} {}", ability.icon(), ability.name());
                    } else {
                        info!(
                            "Ability {} on cooldown: {:.1}s remaining",
                            ability.name(),
                            cooldown
                        );
                    }
                }
            }
            None => {}
        }
    }
}
