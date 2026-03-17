mod assets;
mod diagnostics;
mod io;
mod plugin;
mod project;
mod state;
mod ui;
mod utils;
mod viewport;

use bevy::asset::UnapprovedPathMode;
use bevy::prelude::*;
use bevy::window::{PresentMode, WindowResolution};

use plugin::SprinklesEditorPlugin;
use ui::EditorUiPlugin;

fn main() {
    let editor_data = io::load_editor_data();
    let present_mode = if editor_data.settings.vsync {
        PresentMode::AutoVsync
    } else {
        PresentMode::AutoNoVsync
    };

    App::new()
        .add_plugins(
            DefaultPlugins
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title: "Sprinkles Editor".into(),
                        resolution: WindowResolution::new(1366, 768),
                        present_mode,
                        ..default()
                    }),
                    ..default()
                })
                .set(AssetPlugin {
                    unapproved_path_mode: UnapprovedPathMode::Allow,
                    ..default()
                }),
        )
        .add_plugins(bevy_easings::EasingsPlugin::default())
        .add_plugins(SprinklesEditorPlugin)
        .add_plugins(EditorUiPlugin)
        .add_plugins(diagnostics::plugin)
        .run();
}
