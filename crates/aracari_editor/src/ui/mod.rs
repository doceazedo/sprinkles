pub mod components;
pub mod tokens;
pub mod widgets;

use bevy::asset::{load_internal_asset, uuid_handle};
use bevy::prelude::*;

use components::data_panel::data_panel;
use components::inspector::inspector_panel;
use components::sidebar::sidebar;
use components::topbar::topbar;
use components::viewport::{setup_viewport, viewport_container};

const SHADER_COMMON: Handle<Shader> = uuid_handle!("81dc1f0a-ec1e-4913-862a-1ec536a2a792");

pub struct EditorUiPlugin;

impl Plugin for EditorUiPlugin {
    fn build(&self, app: &mut App) {
        load_internal_asset!(
            app,
            SHADER_COMMON,
            "../../assets/shaders/common.wgsl",
            Shader::from_wgsl
        );

        app.add_plugins(widgets::button::plugin)
            .add_plugins(widgets::checkbox::plugin)
            .add_plugins(widgets::color_picker::plugin)
            .add_plugins(widgets::combobox::plugin)
            .add_plugins(widgets::curve_edit::plugin)
            .add_plugins(widgets::gradient_edit::plugin)
            .add_plugins(widgets::inspector_field::plugin)
            .add_plugins(widgets::variant_edit::plugin)
            .add_plugins(widgets::panel::plugin)
            .add_plugins(widgets::panel_section::plugin)
            .add_plugins(widgets::popover::plugin)
            .add_plugins(widgets::text_edit::plugin)
            .add_plugins(components::data_panel::plugin)
            .add_plugins(components::inspector::plugin)
            .add_plugins(components::seekbar::plugin)
            .add_plugins(components::playback_controls::plugin)
            .add_plugins(components::project_selector::plugin)
            .add_plugins(components::toasts::plugin)
            .add_plugins(components::topbar::plugin)
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
                    flex_grow: 1.0,
                    min_height: px(0.0),
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
