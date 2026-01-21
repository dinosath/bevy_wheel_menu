
//! Headless wheel menu library for Bevy.
//! 
//! This library provides the logic and data structures for wheel menus.
//! Rendering is left to the application.

pub mod mesh;

use bevy::prelude::*;

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
}

impl Default for WheelMenu {
    fn default() -> Self {
        Self {
            slices: 8,
            radius: 120.0,
            inner_radius: 40.0,
            deadzone: 0.25,
            gap: 0.02,
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

/// Plugin that provides headless wheel menu logic.
pub struct WheelMenuPlugin;

impl Plugin for WheelMenuPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_message::<WheelMenuSelected>()
            .add_message::<WheelMenuHoverChanged>()
            .add_systems(Update, (
                update_wheel_input,
                update_wheel_hover,
                emit_selection,
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

/// System that determines which slice is hovered based on input direction.
pub fn update_wheel_hover(
    mut q: Query<(Entity, &WheelMenu, &mut WheelState)>,
    mut hover_ev: MessageWriter<WheelMenuHoverChanged>,
) {
    for (entity, menu, mut state) in &mut q {
        let previous = state.hovered;
        
        if state.dir.length() < menu.deadzone {
            state.hovered = None;
        } else {
            let mut a = state.dir.y.atan2(state.dir.x);
            if a < 0.0 { a += std::f32::consts::TAU; }
            let idx = ((a / std::f32::consts::TAU) * menu.slices as f32).floor() as usize;
            state.hovered = Some(idx.min(menu.slices - 1));
        }
        
        if previous != state.hovered {
            hover_ev.write(WheelMenuHoverChanged {
                previous,
                current: state.hovered,
                menu_entity: entity,
            });
        }
    }
}

/// System that emits selection events on button press.
pub fn emit_selection(
    gamepads: Query<&Gamepad>,
    q: Query<(Entity, &WheelState)>,
    mut ev: MessageWriter<WheelMenuSelected>,
) {
    for gamepad in &gamepads {
        if gamepad.just_pressed(GamepadButton::South) {
            for (entity, state) in &q {
                if let Some(i) = state.hovered {
                    ev.write(WheelMenuSelected { index: i, menu_entity: entity });
                }
            }
        }
    }
}

/// Helper to calculate slice angles with gap.
pub fn slice_angles(menu: &WheelMenu, index: usize) -> (f32, f32) {
    let slice_angle = std::f32::consts::TAU / menu.slices as f32;
    let half_gap = menu.gap / 2.0;
    let a0 = index as f32 * slice_angle + half_gap;
    let a1 = (index + 1) as f32 * slice_angle - half_gap;
    (a0, a1)
}

/// Helper to get the center position of a slice (for placing icons/text).
pub fn slice_center(menu: &WheelMenu, index: usize) -> Vec2 {
    let (a0, a1) = slice_angles(menu, index);
    let center_angle = (a0 + a1) / 2.0;
    let center_radius = (menu.inner_radius + menu.radius) / 2.0;
    Vec2::new(center_angle.cos() * center_radius, center_angle.sin() * center_radius)
}
