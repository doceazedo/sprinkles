use bevy::picking::hover::Hovered;
use bevy::prelude::*;
use bevy::ui::widget::ViewportNode;

use crate::viewport::EditorCamera;

#[derive(Component)]
pub struct EditorViewportContainer;

#[derive(Component)]
pub struct EditorViewport;

pub fn viewport_container() -> impl Bundle {
    (
        EditorViewportContainer,
        Node {
            flex_grow: 1.0,
            height: percent(100),
            ..default()
        },
        Hovered::default(),
    )
}

pub fn setup_viewport(
    mut commands: Commands,
    camera: Query<Entity, Added<EditorCamera>>,
    container: Query<Entity, (With<EditorViewportContainer>, Without<EditorViewport>)>,
) {
    let Ok(camera_entity) = camera.single() else {
        return;
    };

    let Ok(container_entity) = container.single() else {
        return;
    };

    commands.entity(container_entity).insert((
        EditorViewport,
        ViewportNode::new(camera_entity),
    ));
}
