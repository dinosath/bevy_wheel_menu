# bevy_wheel_menu

A headless, gamepad-driven radial (wheel) menu library for [Bevy](https://bevyengine.org/) 0.19.

**Headless** means the library handles all logic — input, hover detection, casting modes, time scaling, slot cycling — and emits events your app reacts to for rendering.  You own the visuals.

---

## Features

| Feature | Description |
|---|---|
| **Casting modes** | `Vanilla` (button press), `ReleaseToUse` (stick release), `HoldToActivate` (dwell), `Direct` (instant on hover) |
| **Time scaling** | `Normal`, `Slow(scale)`, or `Pause` virtual time while the wheel is open |
| **Multi-item slots** | Each slice holds a `Vec<ActionItem>`; player cycles with thumbstick buttons |
| **Action data model** | `ActionItem` enum (`Weapon`, `Spell`, `Consumable`, `Shout`, `Custom`) + `ActionBehavior` trait |
| **Wheel sets** | Cycle between multiple wheels with shoulder buttons |
| **Hold-to-activate** | Per-wheel dwell timer with progress events for UI feedback |
| **Low-count warnings** | Emit once when a slice's count drops below a threshold |
| **Edit mode** | D-pad reorder slices at runtime |
| **Lifecycle events** | `WheelOpened` / `WheelClosed` on hover transitions |
| **Conflict-free config** | Enum variants make invalid combinations impossible |

---

## Quick start

Add the dependency (local path or once published, crates.io):

```toml
[dependencies]
bevy_wheel_menu = { path = "../bevy_wheel_menu" }
bevy = "0.19"
```

### Minimal example

```rust
use bevy::prelude::*;
use bevy_wheel_menu::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(WheelMenuPlugin)   // or ActionWheelPlugin
        .add_systems(Startup, spawn_wheel)
        .add_systems(Update, on_select)
        .run();
}

fn spawn_wheel(mut commands: Commands) {
    commands.spawn((
        WheelMenu { slices: 8, radius: 160.0, inner_radius: 45.0, ..default() },
        WheelState::default(),
        WheelMenuConfig {
            casting_mode: CastingMode::ReleaseToUse,
            ..default()
        },
    ));
}

fn on_select(mut events: MessageReader<WheelMenuSelected>) {
    for ev in events.read() {
        println!("Selected slice {}", ev.index);
    }
}
```

---

## Core types

### `WheelMenu`

The shape descriptor — attach to any entity.

```rust
WheelMenu {
    slices: 8,
    radius: 160.0,       // outer radius (px)
    inner_radius: 45.0,  // hole radius (px)
    deadzone: 0.25,      // stick deadzone (0–1)
    gap: 0.04,           // gap between slices (radians)
}
```

### `WheelMenuConfig`

All behaviour flags replaced by mutually-exclusive **enums** — no invalid combinations.

```rust
WheelMenuConfig {
    time_mode:            TimeMode::Slow(0.2),
    casting_mode:         CastingMode::HoldToActivate { duration: 0.8 },
    toggle_mode:          WheelToggleMode::Hold,
    auto_snap:            true,
    block_gameplay_input: false,
}
```

### `TimeMode`

```rust
TimeMode::Normal       // no time manipulation
TimeMode::Slow(0.15)   // slow Time<Virtual> to 15 %
TimeMode::Pause        // fully freeze virtual time
```

### `CastingMode`

```rust
CastingMode::Vanilla                         // South/A button confirms
CastingMode::ReleaseToUse                    // release stick to confirm
CastingMode::HoldToActivate { duration: 0.8} // dwell for N seconds
CastingMode::Direct                          // fire immediately on hover
```

### `WheelSlot` + `ActionItem`

Attach `WheelSlot` alongside `WheelSlice` to store multiple items per slice.
The player cycles through them with right/left thumbstick press.

```rust
commands.spawn((
    WheelSlice { index: 0 },
    WheelSlot::new(vec![
        ActionItem::Weapon { name: "Sword".into(), icon: "⚔️".into() },
        ActionItem::Weapon { name: "Bow".into(),   icon: "🏹".into() },
    ]),
));
```

Implement `ActionBehavior` for fully custom actions:

```rust
struct HealPotion;

impl ActionBehavior for HealPotion {
    fn execute(&self, commands: &mut Commands) { /* apply healing */ }
    fn label(&self) -> &str { "Heal Potion" }
    fn icon(&self)  -> &str { "🧪" }
}

ActionItem::Custom(Box::new(HealPotion))
```

---

## Events (messages)

| Event | When |
|---|---|
| `WheelMenuSelected { index, menu_entity }` | Slice confirmed (all modes) |
| `WheelMenuHoverChanged { previous, current, menu_entity }` | Hover changes |
| `WheelOpened { menu_entity }` | First slice hovered this session |
| `WheelClosed { menu_entity }` | Stick returns to centre |
| `SlotSelected { slot_index, menu_entity }` | Normalised selection signal |
| `ActionTriggered { slot_index, menu_entity }` | Prompt to call `ActionBehavior::execute` |
| `WheelSlotItemChanged { slot_index, previous_item, current_item, menu_entity }` | Slot item cycled |
| `WheelMenuHoldProgress { index, progress, menu_entity }` | Hold progress 0–1 each frame |
| `WheelMenuHoldActivated { index, menu_entity }` | Hold threshold reached |
| `WheelMenuLowCount { index, current, threshold, slice_entity }` | Count crossed low threshold |
| `WheelSwitched { previous, current, menu_entity }` | Active wheel in a set changed |
| `WheelEditModeChanged { active, menu_entity }` | Edit mode toggled |
| `WheelSliceReorder { from_index, to_index, menu_entity }` | Reorder requested |

---

## Optional components

| Component | Purpose |
|---|---|
| `WheelHoldState` | Required for `CastingMode::HoldToActivate`; tracks dwell progress |
| `WheelSliceCount { current, max, low_threshold }` | Drives low-count warnings |
| `WheelSet { count, prev_button, next_button }` | Multi-wheel cycling |
| `WheelEditMode { toggle_button }` | Runtime slice reorder |

---

## UI helpers (BSN)

The library ships three `bsn!`-authored scene builders for `bevy_ui`:

```rust
// Full-screen centered overlay — attach WheelMenu + WheelState here.
commands.spawn_scene(wheel_overlay()).insert((menu, WheelState::default(), config));

// Zero-size hub at screen center — parent slices to this.
let hub = commands.spawn_scene(wheel_hub()).id();

// Absolutely-positioned rounded panel for slice `i`.
let slice = commands.spawn_scene(wheel_slice_panel(&menu, i, 96.0, Color::srgba(0.1, 0.1, 0.1, 0.9))).id();
```

---

## Examples

```sh
# Diablo-style skill wheel (Release-to-Use, low-count warnings)
cargo run --example gamepad

# FPS weapon / ability wheel (slow-time, hold-to-activate, ammo tracking)
cargo run --example fps
```

---

## Bevy compatibility

| `bevy_wheel_menu` | Bevy |
|---|---|
| `0.1` | `0.19` |

---

## License

Licensed under either of [Apache 2.0](LICENSE-APACHE) or [MIT](LICENSE-MIT) at your option.
