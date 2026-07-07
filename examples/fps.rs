//! FPS-style wheel menu example — driven by `QuickActionHudPlugin`.
//!
//! Controls (gamepad):
//!   L2 (hold)          — open the HUD wheel overlay
//!   Right stick        — navigate / highlight wheel segments
//!   Release to centre  — confirm selection (equip weapon / use ability)
//!   L1 / R1            — previous / next action set (while HUD is open)
//!   Left stick         — move player (only when wheel is closed)
//!   RT                 — shoot (only when wheel is closed)
//!   ⚙ Edit button      — open in-app config editor (while HUD is open)

use bevy::prelude::*;
use bevy_quick_action_hud::{
    HudOpenMode, HudSegmentSelected, QuickActionConfig, QuickActionHudPlugin, WheelHudState,
};

// ─── FPS colour theme ────────────────────────────────────────────────────────

mod fps_theme {
    use bevy::prelude::*;
    pub const HUD_TEXT: Color = Color::srgba(0.85, 0.85, 0.9, 1.0);
    pub const AMMO_TEXT: Color = Color::srgba(0.9, 0.7, 0.2, 1.0);
    pub const CROSSHAIR: Color = Color::srgba(0.9, 0.9, 0.9, 0.8);
}

// ─── Weapon definitions ───────────────────────────────────────────────────────

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
    fn icon(self) -> &'static str {
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
    fn name(self) -> &'static str {
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
    fn max_ammo(self) -> u32 {
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

// ─── Ability definitions ──────────────────────────────────────────────────────

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
    fn icon(self) -> &'static str {
        match self {
            Self::Grenade => "💣",
            Self::Flashbang => "✨",
            Self::Smoke => "💨",
            Self::Heal => "💊",
            Self::Sprint => "👟",
            Self::Shield => "🛡",
        }
    }
    fn name(self) -> &'static str {
        match self {
            Self::Grenade => "Grenade",
            Self::Flashbang => "Flashbang",
            Self::Smoke => "Smoke",
            Self::Heal => "Heal",
            Self::Sprint => "Sprint",
            Self::Shield => "Shield",
        }
    }
    fn cooldown(self) -> f32 {
        match self {
            Self::Grenade => 15.0,
            Self::Flashbang => 12.0,
            Self::Smoke => 10.0,
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

// ─── Game state ───────────────────────────────────────────────────────────────

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
        for &w in WEAPONS {
            ammo.insert(w, w.max_ammo());
        }
        let mut ability_cooldowns = std::collections::HashMap::new();
        for &a in ABILITIES {
            ability_cooldowns.insert(a, 0.0);
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

// ─── Components ───────────────────────────────────────────────────────────────

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

// ─── Main ─────────────────────────────────────────────────────────────────────

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "FPS + Quick Action HUD".into(),
                resolution: (1280, 720).into(),
                ..default()
            }),
            ..default()
        }))
        .add_plugins(QuickActionHudPlugin::with_editor())
        .init_resource::<PlayerState>()
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (
                handle_wheel_toggle,
                handle_player_movement,
                handle_shooting,
                update_ability_cooldowns,
                update_hud,
                on_segment_selected,
            ),
        )
        .run();
}

// ─── Startup ──────────────────────────────────────────────────────────────────

fn setup(mut commands: Commands) {
    commands.spawn(Camera2d);
    spawn_hud(&mut commands);
    spawn_crosshair(&mut commands);
    spawn_player_marker(&mut commands);
}

fn spawn_hud(commands: &mut Commands) {
    // Top-left: health + shield
    commands.spawn((
        HudElement,
        HealthDisplay,
        Text2d::new("HP: 100 | Shield: 50"),
        TextFont {
            font_size: FontSize::Px(20.0),
            ..default()
        },
        TextColor(fps_theme::HUD_TEXT),
        Transform::from_translation(Vec3::new(-550.0, 320.0, 10.0)),
    ));
    // Bottom-right: current weapon
    commands.spawn((
        HudElement,
        WeaponDisplay,
        Text2d::new("🎯 AR-15"),
        TextFont {
            font_size: FontSize::Px(24.0),
            ..default()
        },
        TextColor(fps_theme::HUD_TEXT),
        Transform::from_translation(Vec3::new(480.0, -300.0, 10.0)),
    ));
    // Bottom-right: ammo
    commands.spawn((
        HudElement,
        AmmoDisplay,
        Text2d::new("30/30"),
        TextFont {
            font_size: FontSize::Px(18.0),
            ..default()
        },
        TextColor(fps_theme::AMMO_TEXT),
        Transform::from_translation(Vec3::new(480.0, -330.0, 10.0)),
    ));
    // Bottom center: hints
    commands.spawn((
        HudElement,
        Text2d::new("L2: Wheel  |  L1/R1: Prev/Next Set  |  RT: Shoot  |  ⚙ Edit"),
        TextFont {
            font_size: FontSize::Px(13.0),
            ..default()
        },
        TextColor(Color::srgba(0.6, 0.6, 0.6, 0.8)),
        Transform::from_translation(Vec3::new(0.0, -340.0, 10.0)),
    ));
}

fn spawn_crosshair(commands: &mut Commands) {
    commands.spawn((
        Crosshair,
        Text2d::new("+"),
        TextFont {
            font_size: FontSize::Px(32.0),
            ..default()
        },
        TextColor(fps_theme::CROSSHAIR),
        Transform::from_translation(Vec3::new(0.0, 0.0, 5.0)),
    ));
}

fn spawn_player_marker(commands: &mut Commands) {
    commands.spawn((
        PlayerMarker,
        Text2d::new("▲"),
        TextFont {
            font_size: FontSize::Px(24.0),
            ..default()
        },
        TextColor(Color::srgba(0.3, 0.8, 0.3, 0.9)),
        Transform::from_translation(Vec3::new(0.0, -200.0, 5.0)),
    ));
}

// ─── Systems ──────────────────────────────────────────────────────────────────

/// Manages HUD visibility based on L2 input and the configured open mode.
///
/// - **Hold** (default): HUD is open while L2 is held; releasing closes it.
/// - **Toggle**: first L2 press opens, second press closes.
///
/// R1/L1 are intentionally free here — they are handled by `apply_set_shortcuts`
/// in editor.rs, which only fires when `hud.open` is true.
fn handle_wheel_toggle(
    gamepads: Query<&Gamepad>,
    mut hud: ResMut<WheelHudState>,
    cfg: Res<QuickActionConfig>,
) {
    match cfg.hud_open_mode {
        HudOpenMode::Hold => {
            let mut l2 = false;
            for gp in &gamepads {
                l2 |= gp.pressed(GamepadButton::LeftTrigger2);
            }
            // Keep alive while the editor sidebar is open so the user doesn't
            // lose the editor by releasing the button.
            let want_open = l2 || hud.editor_open;
            if hud.open != want_open {
                if !want_open {
                    hud.highlighted = None;
                }
                hud.open = want_open;
                hud.dirty = true;
            }
        }
        HudOpenMode::Toggle => {
            let mut just_l2 = false;
            for gp in &gamepads {
                just_l2 |= gp.just_pressed(GamepadButton::LeftTrigger2);
            }
            if just_l2 {
                if hud.open && !hud.editor_open {
                    // Don't close while editor is open.
                    hud.highlighted = None;
                    hud.open = false;
                    hud.dirty = true;
                } else if !hud.open {
                    hud.open = true;
                    hud.dirty = true;
                }
            }
        }
    }
}

/// Left-stick movement + right-stick look (disabled while wheel is open).
fn handle_player_movement(
    gamepads: Query<&Gamepad>,
    time: Res<Time>,
    mut player_state: ResMut<PlayerState>,
    hud: Res<WheelHudState>,
    mut player_marker: Query<&mut Transform, With<PlayerMarker>>,
) {
    if hud.open {
        return;
    }
    for gp in &gamepads {
        let mx = gp.get(GamepadAxis::LeftStickX).unwrap_or(0.0);
        let my = gp.get(GamepadAxis::LeftStickY).unwrap_or(0.0);
        if mx.abs() > 0.1 || my.abs() > 0.1 {
            let speed = 200.0;
            player_state.position.x += mx * speed * time.delta_secs();
            player_state.position.y += my * speed * time.delta_secs();
        }
        let lx = gp.get(GamepadAxis::RightStickX).unwrap_or(0.0);
        let ly = gp.get(GamepadAxis::RightStickY).unwrap_or(0.0);
        if lx.abs() > 0.2 || ly.abs() > 0.2 {
            player_state.look_angle = ly.atan2(lx);
        }
    }
    for mut t in &mut player_marker {
        t.translation.x = player_state.position.x;
        t.translation.y = player_state.position.y - 200.0;
        t.rotation = Quat::from_rotation_z(player_state.look_angle - std::f32::consts::FRAC_PI_2);
    }
}

/// RT shoots (disabled while wheel is open).
fn handle_shooting(
    gamepads: Query<&Gamepad>,
    mut player_state: ResMut<PlayerState>,
    hud: Res<WheelHudState>,
) {
    if hud.open {
        return;
    }
    for gp in &gamepads {
        if gp.get(GamepadAxis::RightZ).unwrap_or(0.0) > 0.5 {
            let w = player_state.current_weapon;
            if w.max_ammo() > 0 {
                if let Some(ammo) = player_state.ammo.get_mut(&w) {
                    if *ammo > 0 {
                        *ammo -= 1;
                    }
                }
            }
        }
    }
}

fn update_ability_cooldowns(time: Res<Time>, mut player_state: ResMut<PlayerState>) {
    for cd in player_state.ability_cooldowns.values_mut() {
        if *cd > 0.0 {
            *cd = (*cd - time.delta_secs()).max(0.0);
        }
    }
}

fn update_hud(
    player_state: Res<PlayerState>,
    mut health_q: Query<
        &mut Text2d,
        (
            With<HealthDisplay>,
            Without<WeaponDisplay>,
            Without<AmmoDisplay>,
        ),
    >,
    mut weapon_q: Query<
        &mut Text2d,
        (
            With<WeaponDisplay>,
            Without<HealthDisplay>,
            Without<AmmoDisplay>,
        ),
    >,
    mut ammo_q: Query<
        &mut Text2d,
        (
            With<AmmoDisplay>,
            Without<HealthDisplay>,
            Without<WeaponDisplay>,
        ),
    >,
) {
    for mut t in &mut health_q {
        t.0 = format!(
            "HP: {:.0} | Shield: {:.0}",
            player_state.health, player_state.shield
        );
    }
    for mut t in &mut weapon_q {
        let w = player_state.current_weapon;
        t.0 = format!("{} {}", w.icon(), w.name());
    }
    for mut t in &mut ammo_q {
        let w = player_state.current_weapon;
        let ammo = player_state.ammo.get(&w).copied().unwrap_or(0);
        t.0 = if w.max_ammo() > 0 {
            format!("{}/{}", ammo, w.max_ammo())
        } else {
            "∞".to_string()
        };
    }
}

/// Reacts to the release-to-use selection emitted by `hud_stick_nav`.
/// Set 0 = weapons, Set 1 = abilities.
fn on_segment_selected(
    mut events: bevy::ecs::message::MessageReader<HudSegmentSelected>,
    mut player_state: ResMut<PlayerState>,
) {
    for ev in events.read() {
        match ev.set {
            0 => {
                if let Some(&weapon) = WEAPONS.get(ev.slot) {
                    player_state.current_weapon = weapon;
                    info!("Equipped: {} {}", weapon.icon(), weapon.name());
                }
            }
            1 => {
                if let Some(&ability) = ABILITIES.get(ev.slot) {
                    let cd = player_state
                        .ability_cooldowns
                        .get(&ability)
                        .copied()
                        .unwrap_or(0.0);
                    if cd <= 0.0 {
                        player_state
                            .ability_cooldowns
                            .insert(ability, ability.cooldown());
                        info!("Used ability: {} {}", ability.icon(), ability.name());
                    } else {
                        info!(
                            "Ability {} on cooldown: {:.1}s remaining",
                            ability.name(),
                            cd
                        );
                    }
                }
            }
            _ => {}
        }
    }
}
