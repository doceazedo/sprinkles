pub mod components;
pub mod tokens;
pub mod widgets;

use bevy::prelude::*;

use components::data_panel::data_panel;
use components::inspector_panel::inspector_panel;
use components::sidebar::sidebar;
use components::topbar::topbar;
use components::viewport::{setup_viewport, viewport_container};

pub struct EditorUiPlugin;

impl Plugin for EditorUiPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(widgets::button::plugin)
            .add_plugins(widgets::checkbox::plugin)
            .add_plugins(widgets::panel::plugin)
            .add_plugins(widgets::panel_section::plugin)
            .add_plugins(widgets::text_edit::plugin)
            .add_plugins(components::data_panel::plugin)
            .add_plugins(components::inspector_panel::plugin)
            .add_plugins(components::seekbar::plugin)
            .add_plugins(components::playback_controls::plugin)
            .add_systems(Startup, setup_ui)
            .add_systems(Update, setup_viewport);
    }
}

fn setup_ui(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn((
        Node {
            width: percent(100),
            height: percent(100),
            flex_direction: FlexDirection::Column,
            ..default()
        },
        children![
            topbar(&asset_server),
            (
                Node {
                    width: percent(100),
                    height: percent(100),
                    ..default()
                },
                children![
                    sidebar(&asset_server),
                    data_panel(&asset_server),
                    viewport_container(),
                    inspector_panel(&asset_server),
                ],
            ),
        ],
    ));
}
