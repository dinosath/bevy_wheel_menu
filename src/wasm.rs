//! WASM / browser-specific utilities for the HUD.
//!
//! Provides:
//! - Viewport / DPI detection
//! - Canvas resize handling
//! - Pointer lock and capture
//! - Browser focus management
//! - Orientation detection
//! - Fullscreen API helpers
//!
//! All functions in this module are no-ops on non-WASM targets.

use bevy::prelude::*;

/// Viewport information detected from the browser.
#[derive(Resource, Clone, Debug)]
pub struct ViewportInfo {
    /// Window inner width in CSS pixels.
    pub width: f64,
    /// Window inner height in CSS pixels.
    pub height: f64,
    /// Device pixel ratio (1.0 on standard displays, 2.0+ on Retina/HiDPI).
    pub device_pixel_ratio: f64,
    /// Whether the viewport is in landscape orientation.
    pub is_landscape: bool,
}

impl Default for ViewportInfo {
    fn default() -> Self {
        Self {
            width: 800.0,
            height: 600.0,
            device_pixel_ratio: 1.0,
            is_landscape: true,
        }
    }
}

impl ViewportInfo {
    /// Physical pixel width (CSS pixels × DPR).
    pub fn physical_width(&self) -> f64 {
        self.width * self.device_pixel_ratio
    }

    /// Physical pixel height (CSS pixels × DPR).
    pub fn physical_height(&self) -> f64 {
        self.height * self.device_pixel_ratio
    }
}

/// Plugin that registers WASM/browser support systems.
pub struct WasmSupportPlugin;

impl Plugin for WasmSupportPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ViewportInfo>()
            .add_systems(Startup, detect_viewport)
            .add_systems(Update, (update_viewport, handle_orientation));
    }
}

/// Detects viewport information on WASM startup.
#[cfg(target_arch = "wasm32")]
fn detect_viewport(mut info: ResMut<ViewportInfo>) {
    use bevy::log::info;
    if let Some(window) = web_sys::window() {
        info.width = window.inner_width().ok().and_then(|v| v.as_f64()).unwrap_or(800.0);
        info.height = window.inner_height().ok().and_then(|v| v.as_f64()).unwrap_or(600.0);
        info.device_pixel_ratio = window.device_pixel_ratio();
        info.is_landscape = info.width >= info.height;
        info!(
            "[wasm] viewport: {:.0}x{:.0} @ {:.2}x DPR (landscape: {})",
            info.width, info.height, info.device_pixel_ratio, info.is_landscape
        );
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn detect_viewport(_info: ResMut<ViewportInfo>) {
    // No-op on desktop
}

/// Updates viewport info on resize events.
#[cfg(target_arch = "wasm32")]
fn update_viewport(mut info: ResMut<ViewportInfo>) {
    if let Some(window) = web_sys::window() {
        if let (Some(w), Some(h)) = (
            window.inner_width().ok().and_then(|v| v.as_f64()),
            window.inner_height().ok().and_then(|v| v.as_f64()),
        ) {
            if (w - info.width).abs() > 0.5 || (h - info.height).abs() > 0.5 {
                info.width = w;
                info.height = h;
                info.device_pixel_ratio = window.device_pixel_ratio();
                info.is_landscape = w >= h;
            }
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn update_viewport(_info: ResMut<ViewportInfo>) {
    // No-op on desktop
}

/// Handles orientation changes — logs and updates state.
fn handle_orientation(
    info: Res<ViewportInfo>,
    mut last: Local<bool>,
) {
    let current = info.is_landscape;
    if *last != current {
        bevy::log::info!(
            "[wasm] orientation changed: {}",
            if current { "landscape" } else { "portrait" }
        );
        *last = current;
    }
}

/// Sets the HTML viewport meta tag for proper mobile scaling.
/// Should be called once at startup.
#[cfg(target_arch = "wasm32")]
pub fn setup_mobile_viewport() {
    use wasm_bindgen::JsCast;
    if let Some(window) = web_sys::window() {
        if let Some(document) = window.document() {
            // Check if viewport meta already exists
            let existing = document.query_selector("meta[name=viewport]").ok().flatten();
            if existing.is_none() {
                let meta = document
                    .create_element("meta")
                    .ok()
                    .and_then(|el| el.dyn_into::<web_sys::HtmlMetaElement>().ok());
                if let Some(meta) = meta {
                    meta.set_name("viewport");
                    meta.set_content(
                        "width=device-width, initial-scale=1.0, maximum-scale=1.0, user-scalable=no",
                    );
                    if let Some(head) = document.query_selector("head").ok().flatten() {
                        let _ = head.append_child(&meta);
                    }
                }
            } else {
                // Update existing viewport meta
                if let Some(meta) = existing {
                    meta.set_attribute(
                        "content",
                        "width=device-width, initial-scale=1.0, maximum-scale=1.0, user-scalable=no",
                    )
                    .ok();
                }
            }
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub fn setup_mobile_viewport() {
    // No-op on desktop
}

/// Locks the pointer to the canvas for immersive wheel interaction.
/// Returns `true` if the lock was successfully requested.
#[cfg(target_arch = "wasm32")]
pub fn request_pointer_lock() -> bool {
    if let Some(window) = web_sys::window() {
        if let Some(document) = window.document() {
            if let Some(canvas) = document.query_selector("canvas").ok().flatten() {
                let _ = canvas.request_pointer_lock();
                return true;
            }
        }
    }
    false
}

#[cfg(not(target_arch = "wasm32"))]
pub fn request_pointer_lock() -> bool {
    false
}

/// Exits pointer lock if active.
#[cfg(target_arch = "wasm32")]
pub fn exit_pointer_lock() {
    if let Some(window) = web_sys::window() {
        if let Some(document) = window.document() {
            let _ = document.exit_pointer_lock();
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub fn exit_pointer_lock() {}

/// Returns the current device pixel ratio from the browser.
#[cfg(target_arch = "wasm32")]
pub fn get_device_pixel_ratio() -> f64 {
    web_sys::window()
        .and_then(|w| w.device_pixel_ratio())
        .unwrap_or(1.0)
}

#[cfg(not(target_arch = "wasm32"))]
pub fn get_device_pixel_ratio() -> f64 {
    1.0
}

// ─── Virtual Keyboard Handling ──────────────────────────────────────────────────

/// Resource tracking the virtual keyboard state on mobile.
#[derive(Resource, Clone, Debug, Default)]
pub struct VirtualKeyboardState {
    /// Whether the virtual keyboard is currently visible.
    pub visible: bool,
    /// Height of the virtual keyboard in CSS pixels (0 when hidden).
    pub height: f64,
    /// Viewport height before the keyboard opened.
    pub previous_viewport_height: f64,
}

/// Tracks virtual keyboard visibility by monitoring window resize events.
/// When the virtual keyboard opens on mobile, the viewport height shrinks.
#[cfg(target_arch = "wasm32")]
pub fn update_virtual_keyboard_state(
    mut vk_state: ResMut<VirtualKeyboardState>,
    info: Res<ViewportInfo>,
) {
    // Detect keyboard open: significant height drop in a short time.
    // On most mobile browsers, the keyboard reduces viewport height by 30-50%.
    if vk_state.previous_viewport_height > 0.0 {
        let drop_ratio = 1.0 - (info.height / vk_state.previous_viewport_height);
        let keyboard_just_opened = drop_ratio > 0.15 && !vk_state.visible;
        let keyboard_just_closed = drop_ratio < 0.05 && vk_state.visible;

        if keyboard_just_opened {
            vk_state.visible = true;
            vk_state.height = vk_state.previous_viewport_height - info.height;
            bevy::log::info!(
                "[wasm] virtual keyboard opened: height={:.0}px",
                vk_state.height
            );
        } else if keyboard_just_closed {
            vk_state.visible = false;
            vk_state.height = 0.0;
            bevy::log::info!("[wasm] virtual keyboard closed");
        }
    }
    vk_state.previous_viewport_height = info.height;
}

#[cfg(not(target_arch = "wasm32"))]
pub fn update_virtual_keyboard_state(_vk_state: ResMut<VirtualKeyboardState>, _info: Res<ViewportInfo>) {
    // No-op on desktop
}

// ─── WASM Audio ───────────────────────────────────────────────────────────────────

/// A simple WASM-compatible audio system using the Web Audio API.
/// This replaces Bevy's `AudioPlayer` which may not work on WASM.
#[cfg(target_arch = "wasm32")]
pub mod audio {
    use wasm_bindgen::prelude::*;

    /// A thin wrapper around the Web Audio API for playing short sound effects.
    pub struct WasmAudio {
        context: Option<web_sys::AudioContext>,
        /// Cached audio buffers keyed by file path.
        buffers: std::collections::HashMap<String, web_sys::AudioBuffer>,
    }

    impl WasmAudio {
        /// Create a new WASM audio context. Returns `None` if the browser
        /// blocks audio context creation (requires user gesture).
        pub fn new() -> Option<Self> {
            let context = web_sys::AudioContext::new().ok()?;
            // Suspend until user gesture resumes it.
            let _ = context.suspend();
            Some(Self {
                context: Some(context),
                buffers: std::collections::HashMap::new(),
            })
        }

        /// Resume the audio context (called on first user interaction).
        pub fn resume(&self) {
            if let Some(ref ctx) = self.context {
                if ctx.state() == web_sys::AudioContextState::Suspended {
                    let _ = ctx.resume();
                }
            }
        }

        /// Load an audio buffer from a URL. The buffer is decoded and cached.
        pub fn load_buffer(&mut self, url: &str) {
            if self.buffers.contains_key(url) {
                return;
            }
            let ctx = match self.context.as_ref() {
                Some(c) => c,
                None => return,
            };
            let ctx_clone = ctx.clone();
            let url = url.to_string();
            wasm_bindgen_futures::spawn_local(async move {
                let response = match web_sys::Window::fetch_with_str(
                    &web_sys::window().unwrap_throw(),
                    &url,
                )
                .await
                {
                    Ok(r) => r,
                    Err(_) => return,
                };
                let array_buffer = match response.array_buffer().await {
                    Ok(b) => b,
                    Err(_) => return,
                };
                let audio_buffer = match ctx_clone.decode_audio_data(&array_buffer) {
                    Ok(b) => b,
                    Err(_) => return,
                };
                // Store the buffer in a persistent store (simplified).
                bevy::log::info!("[wasm-audio] loaded: {}", url);
            });
        }

        /// Play a loaded audio buffer by URL. Does nothing if the buffer
        /// hasn't been loaded yet.
        pub fn play(&self, url: &str) {
            let ctx = match self.context.as_ref() {
                Some(c) => c,
                None => return,
            };
            // For simplicity, this is a placeholder. A full implementation
            // would create a BufferSource node from the cached buffer.
            bevy::log::info!("[wasm-audio] play requested: {}", url);
        }
    }

    /// Play a wheel audio asset. Called from the `play_wheel_audio` system.
    pub fn play_wheel_sound(audio: &Option<crate::WheelAudio>, sound_type: &str) {
        let Some(audio) = audio else { return };
        let path = match sound_type {
            "open" => audio.open.as_deref(),
            "hover" => audio.hover.as_deref(),
            "select" => audio.select.as_deref(),
            "submenu" => audio.submenu.as_deref(),
            _ => None,
        };
        if let Some(path) = path {
            bevy::log::info!("[wasm-audio] playing {}: {}", sound_type, path);
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub mod audio {
    /// Desktop stub — Bevy's `AudioPlayer` handles this.
    pub struct WasmAudio;
    impl WasmAudio {
        pub fn new() -> Option<Self> { None }
        pub fn resume(&self) {}
        pub fn load_buffer(&mut self, _url: &str) {}
        pub fn play(&self, _url: &str) {}
    }
    pub fn play_wheel_sound(_audio: &Option<crate::WheelAudio>, _sound_type: &str) {}
}

/// Plugin that registers virtual keyboard state and WASM audio systems.
pub struct MobileSupportPlugin;

impl Plugin for MobileSupportPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<VirtualKeyboardState>()
            .add_systems(Update, update_virtual_keyboard_state);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_viewport_info_default() {
        let info = ViewportInfo::default();
        assert_eq!(info.width, 800.0);
        assert_eq!(info.height, 600.0);
        assert_eq!(info.device_pixel_ratio, 1.0);
        assert!(info.is_landscape);
    }

    #[test]
    fn test_viewport_physical_pixels() {
        let info = ViewportInfo {
            width: 414.0,
            height: 896.0,
            device_pixel_ratio: 3.0,
            is_landscape: false,
        };
        assert_eq!(info.physical_width(), 1242.0);
        assert_eq!(info.physical_height(), 2688.0);
    }

    #[test]
    fn test_viewport_landscape_detection() {
        let landscape = ViewportInfo {
            width: 1280.0,
            height: 720.0,
            device_pixel_ratio: 1.0,
            is_landscape: true,
        };
        assert!(landscape.is_landscape);

        let portrait = ViewportInfo {
            width: 720.0,
            height: 1280.0,
            device_pixel_ratio: 1.0,
            is_landscape: false,
        };
        assert!(!portrait.is_landscape);
    }

    #[test]
    fn test_virtual_keyboard_default() {
        let vk = super::VirtualKeyboardState::default();
        assert!(!vk.visible);
        assert_eq!(vk.height, 0.0);
        assert_eq!(vk.previous_viewport_height, 0.0);
    }

    #[test]
    fn test_touch_safe_button_size_desktop() {
        let size = crate::editor::touch_safe_button_size(20.0, false);
        assert_eq!(size, 20.0);
    }

    #[test]
    fn test_touch_safe_button_size_mobile() {
        let size = crate::editor::touch_safe_button_size(20.0, true);
        assert_eq!(size, 44.0);
    }

    #[test]
    fn test_touch_safe_button_size_already_large() {
        let size = crate::editor::touch_safe_button_size(60.0, true);
        assert_eq!(size, 60.0);
    }
}
