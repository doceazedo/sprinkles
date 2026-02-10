use bevy::asset::UnapprovedPathMode;
use bevy::prelude::*;
use bevy::window::WindowResolution;

use sprinkles_editor::plugin::SprinklesEditorPlugin;
use sprinkles_editor::ui::EditorUiPlugin;

fn main() {
    App::new()
        .add_plugins(
            DefaultPlugins
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title: "Sprinkles Editor".into(),
                        resolution: WindowResolution::new(1366, 768),
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
        .run();
}
