mod plugin;
mod state;
mod ui;
mod viewport;

use bevy::prelude::*;
use bevy::window::WindowResolution;

use plugin::StarlingEditorPlugin;

fn main() {
    App::new()
        .add_plugins(
            DefaultPlugins
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title: "Starling Editor".into(),
                        resolution: WindowResolution::new(1280, 720),
                        ..default()
                    }),
                    ..default()
                }),
        )
        .add_plugins(StarlingEditorPlugin)
        .run();
}
