//! In-app Quick Action Menu editor example.
//!
//! Run with: `cargo run --example editor`
//!
//! A left-hand sidebar (built entirely with `bsn!` macros) lets you author a
//! document of **action sets**. Each set holds quick actions, standalone wheels
//! and wheel sets:
//! - `+ Set` adds a new action set
//! - `+ Quick Action` / `+ Wheel` / `+ Wheel Set` add entries to a set
//! - `+ add wheel` adds a wheel to a wheel set
//! - click a quick action to edit it, or a wheel to preview it on the canvas
//! - `SAVE` / `LOAD` persist the whole document as RON (`quickactions_config.ron`)

use bevy::prelude::*;
use bevy_wheel_menu::editor::QuickActionEditorPlugin;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Quick Action Menu Editor".into(),
                resolution: (1229, 768).into(),
                ..default()
            }),
            ..default()
        }))
        .add_plugins(QuickActionEditorPlugin)
        .add_systems(Startup, setup)
        .run();
}

fn setup(mut commands: Commands) {
    commands.spawn(Camera2d);
}
