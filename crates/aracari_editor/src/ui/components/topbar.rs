use bevy::prelude::*;

use crate::project::SaveProjectEvent;
use crate::ui::components::playback_controls::playback_controls;
use crate::ui::components::project_selector::project_selector;
use crate::ui::components::seekbar::seekbar;
use crate::ui::tokens::{BACKGROUND_COLOR, BORDER_COLOR};
use crate::ui::widgets::button::{ButtonClickEvent, ButtonProps, ButtonVariant, button};
use crate::ui::widgets::separator::EditorSeparator;

pub fn plugin(app: &mut App) {
    app.add_systems(Update, setup_save_button_observer);
}

#[derive(Component)]
pub struct SaveButton;

fn setup_save_button_observer(buttons: Query<Entity, Added<SaveButton>>, mut commands: Commands) {
    for entity in &buttons {
        commands.entity(entity).observe(on_save_button_click);
    }
}

fn on_save_button_click(_event: On<ButtonClickEvent>, mut commands: Commands) {
    commands.trigger(SaveProjectEvent);
}

#[derive(Component)]
pub struct EditorTopbar;

pub fn topbar(asset_server: &AssetServer) -> impl Bundle {
    (
        EditorTopbar,
        Node {
            width: percent(100),
            height: px(52),
            padding: UiRect::all(px(12)),
            border: UiRect::bottom(px(1)),
            justify_content: JustifyContent::SpaceBetween,
            align_items: AlignItems::Center,
            ..default()
        },
        BackgroundColor(BACKGROUND_COLOR.into()),
        BorderColor::all(BORDER_COLOR),
        children![
            project_selector(),
            (
                Node {
                    column_gap: px(12),
                    align_items: AlignItems::Center,
                    ..default()
                },
                children![
                    seekbar(asset_server),
                    playback_controls(asset_server),
                    EditorSeparator::vertical(),
                    (
                        SaveButton,
                        button(ButtonProps::new("Save").with_variant(ButtonVariant::Primary)),
                    ),
                ],
            ),
        ],
    )
}
