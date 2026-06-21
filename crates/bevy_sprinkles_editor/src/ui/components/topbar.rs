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

#[derive(Component, Default, Clone)]
pub struct EditorTopbar;

pub fn topbar() -> impl Scene {
    bsn! {
        EditorTopbar
        Node {
            width: percent(100),
            height: px(52),
            padding: { UiRect::all(px(12)) },
            border: { UiRect::bottom(px(1)) },
            justify_content: { JustifyContent::SpaceBetween },
            align_items: { AlignItems::Center },
        }
        BackgroundColor(BACKGROUND_COLOR)
        template_value(BorderColor::all(BORDER_COLOR))
    }
}

fn topbar_right() -> impl Scene {
    bsn! {
        Node {
            column_gap: px(12),
            align_items: { AlignItems::Center },
        }
    }
}

pub fn spawn_topbar(commands: &mut Commands, asset_server: &AssetServer, parent: Entity) {
    let bar = commands.spawn_scene(topbar()).insert(ChildOf(parent)).id();

    let selector = commands.spawn(project_selector()).id();
    commands.entity(bar).add_children(&[selector]);

    let right = commands.spawn_scene(topbar_right()).insert(ChildOf(bar)).id();
    commands.spawn(seekbar(asset_server)).insert(ChildOf(right));
    commands
        .spawn_scene(playback_controls())
        .insert(ChildOf(right));
    commands
        .spawn_scene(EditorSeparator::vertical())
        .insert(ChildOf(right));
    commands
        .spawn_scene(button(
            ButtonProps::new("Save").with_variant(ButtonVariant::Primary),
        ))
        .insert(SaveButton)
        .insert(ChildOf(right));
}
