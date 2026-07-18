//! Touch input support for mobile/browser HUD interaction.
//!
//! Provides systems and utilities for touch-based interaction with the HUD
//! editor and wheel menus. Handles:
//! - Single-finger drag for repositioning HUD elements
//! - Tap for selection/activation
//! - Long-press for context actions
//! - Multi-touch for simultaneous interactions
//! - High-DPI / device pixel ratio scaling
//! - Coordinate conversion between screen space and UI space
//!
//! ## Browser compatibility
//!
//! Tested on:
//! - Chrome Desktop (mouse + touch emulation)
//! - Chrome Android
//! - Retroid Pocket 5 (Android + Chromium)
//! - Safari iOS

use bevy::input::touch::TouchPhase;
use bevy::prelude::*;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::JsCast;

/// Resource tracking active touch interactions with the HUD.
#[derive(Resource, Default)]
pub struct TouchState {
    /// Active drag operations keyed by touch ID.
    pub drags: Vec<TouchDrag>,
    /// Timestamp of the last tap for double-tap detection.
    pub last_tap_time: f64,
    /// Position of the last tap in logical (UI) coordinates.
    pub last_tap_pos: Vec2,
}

/// Represents an active drag operation.
#[derive(Clone, Debug)]
pub struct TouchDrag {
    /// The Bevy touch/finger ID.
    pub finger_id: u64,
    /// The HUD element entity being dragged, if any.
    pub target: Option<Entity>,
    /// The offset from the touch start to the element's origin.
    pub offset: Vec2,
    /// Current position in logical (UI) pixels.
    pub current_pos: Vec2,
    /// Start position in logical (UI) pixels.
    pub start_pos: Vec2,
    /// Time when the drag started.
    pub start_time: f64,
}

/// Event emitted when a HUD element is tapped via touch.
#[derive(Message, Clone, Debug)]
pub struct TouchTapEvent {
    /// Position of the tap in logical (UI) pixels.
    pub position: Vec2,
    /// Entity under the tap, if any.
    pub target: Option<Entity>,
}

/// Event emitted when a long press is detected.
#[derive(Message, Clone, Debug)]
pub struct TouchLongPressEvent {
    /// Position of the press in logical (UI) pixels.
    pub position: Vec2,
    /// Entity under the press, if any.
    pub target: Option<Entity>,
    /// Duration of the press in seconds.
    pub duration: f32,
}

/// Event emitted during a drag operation.
#[derive(Message, Clone, Debug)]
pub struct TouchDragEvent {
    /// The finger/touch ID.
    pub finger_id: u64,
    /// The entity being dragged, if any.
    pub target: Option<Entity>,
    /// Current position in logical (UI) pixels.
    pub position: Vec2,
    /// Delta from the last frame in logical (UI) pixels.
    pub delta: Vec2,
    /// Whether the drag just started this frame.
    pub started: bool,
    /// Whether the drag just ended this frame.
    pub ended: bool,
}

/// Configurable touch interaction parameters.
#[derive(Resource, Clone)]
pub struct TouchConfig {
    /// Maximum time (in seconds) for a touch to count as a tap.
    pub tap_max_duration: f32,
    /// Maximum movement (in logical pixels) for a touch to count as a tap.
    pub tap_max_distance: f32,
    /// Minimum hold duration (in seconds) for a long press.
    pub long_press_duration: f32,
    /// How far (in logical pixels) a finger must move before a drag starts.
    pub drag_threshold: f32,
    /// Minimum interval (in seconds) between taps for double-tap detection.
    pub double_tap_interval: f64,
    /// Device pixel ratio (set during startup based on the browser/window).
    pub device_pixel_ratio: f64,
}

impl Default for TouchConfig {
    fn default() -> Self {
        Self {
            tap_max_duration: 0.3,
            tap_max_distance: 10.0,
            long_press_duration: 0.8,
            drag_threshold: 5.0,
            double_tap_interval: 0.3,
            device_pixel_ratio: 1.0,
        }
    }
}

/// Plugin that registers touch-related systems and resources.
pub struct TouchInteractionPlugin;

impl Plugin for TouchInteractionPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<TouchState>()
            .init_resource::<TouchConfig>()
            .add_message::<TouchTapEvent>()
            .add_message::<TouchLongPressEvent>()
            .add_message::<TouchDragEvent>()
            .add_systems(
                Update,
                (handle_touches, detect_long_press, cleanup_touches).chain(),
            );
    }
}

/// Reads Bevy's `TouchInput` events and emits `TouchDragEvent`s.
fn handle_touches(
    mut touches: MessageReader<TouchInput>,
    mut touch_state: ResMut<TouchState>,
    time: Res<Time>,
    config: Res<TouchConfig>,
    mut drag_events: MessageWriter<TouchDragEvent>,
    mut tap_events: MessageWriter<TouchTapEvent>,
) {
    for touch in touches.read() {
        let logical_pos = touch.position; // Bevy already handles DPI scaling

        match touch.phase {
            TouchPhase::Started => {
                // Start tracking a potential drag
                touch_state.drags.push(TouchDrag {
                    finger_id: touch.id,
                    target: None,
                    offset: Vec2::ZERO,
                    current_pos: logical_pos,
                    start_pos: logical_pos,
                    start_time: time.elapsed_secs_f64(),
                });
                drag_events.write(TouchDragEvent {
                    finger_id: touch.id,
                    target: None,
                    position: logical_pos,
                    delta: Vec2::ZERO,
                    started: true,
                    ended: false,
                });
            }
            TouchPhase::Moved => {
                if let Some(drag) = touch_state
                    .drags
                    .iter_mut()
                    .find(|d| d.finger_id == touch.id)
                {
                    let delta = logical_pos - drag.current_pos;
                    drag.current_pos = logical_pos;
                    drag_events.write(TouchDragEvent {
                        finger_id: touch.id,
                        target: drag.target,
                        position: logical_pos,
                        delta,
                        started: false,
                        ended: false,
                    });
                }
            }
            TouchPhase::Ended | TouchPhase::Canceled => {
                let mut tap_fired = false;
                if let Some(idx) = touch_state
                    .drags
                    .iter()
                    .position(|d| d.finger_id == touch.id)
                {
                    let drag = touch_state.drags.swap_remove(idx);
                    let total_dist = drag.current_pos.distance(drag.start_pos);
                    let duration = (time.elapsed_secs_f64() - drag.start_time) as f32;

                    // Check if this was a tap (short duration, minimal movement)
                    if duration < config.tap_max_duration && total_dist < config.tap_max_distance {
                        tap_events.write(TouchTapEvent {
                            position: drag.current_pos,
                            target: drag.target,
                        });
                        tap_fired = true;
                        touch_state.last_tap_time = time.elapsed_secs_f64();
                        touch_state.last_tap_pos = drag.current_pos;
                    }

                    drag_events.write(TouchDragEvent {
                        finger_id: touch.id,
                        target: drag.target,
                        position: drag.current_pos,
                        delta: Vec2::ZERO,
                        started: false,
                        ended: true,
                    });
                }
                // Fallback: if no drag was tracked but we got an end event
                if !tap_fired {
                    // Could be from a touch that started before the system ran
                }
            }
        }
    }
}

/// Checks for active touches that have been held beyond the long-press
/// threshold and emits `TouchLongPressEvent`.
fn detect_long_press(
    time: Res<Time>,
    config: Res<TouchConfig>,
    touch_state: Res<TouchState>,
    mut long_press_events: MessageWriter<TouchLongPressEvent>,
) {
    for drag in &touch_state.drags {
        let elapsed = (time.elapsed_secs_f64() - drag.start_time) as f32;
        if elapsed >= config.long_press_duration {
            long_press_events.write(TouchLongPressEvent {
                position: drag.current_pos,
                target: drag.target,
                duration: elapsed,
            });
        }
    }
}

/// Removes stale drag entries (safety net for missed Canceled/Ended events).
fn cleanup_touches(
    mut touch_state: ResMut<TouchState>,
    time: Res<Time>,
    _config: Res<TouchConfig>,
) {
    touch_state.drags.retain(|drag| {
        let elapsed = (time.elapsed_secs_f64() - drag.start_time) as f32;
        elapsed < 10.0 // Remove any drag older than 10 seconds (safety net)
    });
}

/// Converts a screen-space position (physical pixels from the browser) to
/// logical UI pixels, accounting for the device pixel ratio.
pub fn screen_to_logical(pos: Vec2, dpr: f64) -> Vec2 {
    pos / dpr as f32
}

/// Converts logical UI pixels to physical screen pixels (for browser APIs).
pub fn logical_to_screen(pos: Vec2, dpr: f64) -> Vec2 {
    pos * dpr as f32
}

/// Detects and sets the device pixel ratio from the browser window.
/// Should be called once during startup or when the window resizes.
#[cfg(target_arch = "wasm32")]
pub fn detect_device_pixel_ratio(mut config: ResMut<TouchConfig>) {
    if let Some(window) = web_sys::window() {
        let dpr = window.device_pixel_ratio();
        config.device_pixel_ratio = dpr;
        bevy::log::info!("[touch] detected device pixel ratio: {:.2}", dpr);
    }
}

/// Desktop stub for device pixel ratio detection.
#[cfg(not(target_arch = "wasm32"))]
pub fn detect_device_pixel_ratio(_config: ResMut<TouchConfig>) {
    // Desktop always has 1.0 DPR
}

/// Prevents default browser touch actions (scroll, zoom) on the canvas
/// to avoid conflicts with HUD drag interactions.
#[cfg(target_arch = "wasm32")]
pub fn prevent_default_touch_actions() {
    if let Some(window) = web_sys::window() {
        if let Some(document) = window.document() {
            if let Ok(Some(canvas)) = document.query_selector("canvas") {
                let closure = wasm_bindgen::closure::Closure::<dyn FnMut(web_sys::TouchEvent)>::new(
                    move |event: web_sys::TouchEvent| {
                        event.prevent_default();
                    },
                );
                canvas
                    .add_event_listener_with_callback(
                        "touchstart",
                        closure.as_ref().unchecked_ref(),
                    )
                    .ok();
                canvas
                    .add_event_listener_with_callback("touchmove", closure.as_ref().unchecked_ref())
                    .ok();
                closure.forget();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_screen_to_logical() {
        let screen_pos = Vec2::new(200.0, 100.0);
        let logical = screen_to_logical(screen_pos, 2.0);
        assert_eq!(logical, Vec2::new(100.0, 50.0));
    }

    #[test]
    fn test_logical_to_screen() {
        let logical = Vec2::new(100.0, 50.0);
        let screen = logical_to_screen(logical, 2.0);
        assert_eq!(screen, Vec2::new(200.0, 100.0));
    }

    #[test]
    fn test_touch_config_default() {
        let config = TouchConfig::default();
        assert_eq!(config.tap_max_duration, 0.3);
        assert_eq!(config.long_press_duration, 0.8);
        assert_eq!(config.device_pixel_ratio, 1.0);
    }
}
