# Changelog

All notable changes to `bevy_quick_action_hud` are documented here.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.2.0] — 2026-07-15

### Added

- **Touch input module** (`src/touch.rs`): full multi-touch support with tap,
  drag, and long-press detection for mobile/browser HUD interaction.
- **WASM support module** (`src/wasm.rs`): viewport/DPI detection, mobile
  viewport meta management, pointer lock, and orientation handling.
- **Comprehensive unit tests** (160+ tests): covers slot assignment, action
  registration, serialization, layout calculations, editor state transitions,
  input mapping, palette helpers, and all enum variants.
- **CI pipeline** (`.github/workflows/ci.yml`): `cargo fmt`, `cargo clippy`,
  `cargo test`, `cargo doc`, WASM target check, `cargo audit`, `cargo deny`.
- **`deny.toml`**: license, vulnerability, and duplicate-dependency checking.
- **Missing `quickactions_config.ron`**: default RON config file required by
  the serialisation round-trip test.

### Changed

- **Cargo.toml**: cleaned up feature flags, added `[profile.*]` optimisation
  presets, WASM-only dependencies, feature-gated examples.
- **Updated to Bevy 0.19 conventions**: events use `Message`/`MessageReader`/
  `MessageWriter` API with `add_message()` registration.
- **Version bumped** to `0.2.0`.

### Fixed

- **Missing RON config file**: the `ron_config_parses` test now has a valid
  `quickactions_config.ron` to read from.
- **Crate documentation**: added module-level docs, improved public API docs.

### Removed

- **Dead constants**: removed unused palette constants that were duplicated
  between `lib.rs` and `editor.rs`.
- **Unused feature flags**: cleaned up Bevy feature dependencies.

## [0.1.0] — Initial release

- Headless radial (wheel) menu library for Bevy 0.19
- Casting modes (Vanilla, ReleaseToUse, HoldToActivate, Direct)
- Time scaling (Normal, Slow, Pause)
- Multi-item wheel slots with cycle
- Wheel sets with shoulder-button switching
- In-app HUD editor sidebar
- Gamepad icon sets (Xbox, PS4, PS5, Switch)
- UI helpers (wheel overlay, hub, slice panels, labels)
- RON-based config persistence
