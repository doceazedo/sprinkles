pub mod components;
pub mod tokens;
pub mod widgets;

use bevy::prelude::*;
use widgets::button::{button, ButtonProps, ButtonVariant};

pub struct EditorUiPlugin;

impl Plugin for EditorUiPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(widgets::button::plugin)
            .add_systems(Startup, setup_ui);
    }
}

fn setup_ui(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn(button(
        ButtonProps::new("Primary").variant(ButtonVariant::Primary),
        &asset_server,
    ));
}
