use bevy::prelude::*;
use sprinkles::prelude::*;

use crate::state::{DirtyState, EditorState};
use crate::ui::tokens::FONT_PATH;
use crate::ui::widgets::combobox::{ComboBoxChangeEvent, ComboBoxOptionData};
use crate::ui::widgets::inspector_field::fields_row;
use crate::ui::widgets::text_edit::{TextEditCommitEvent, TextEditProps, text_edit};
use crate::ui::widgets::vector_edit::{VectorEditProps, VectorSuffixes, vector_edit};

use super::{
    DynamicSectionContent, InspectorSection, inspector_section, section_needs_setup,
    spawn_labeled_combobox,
};
use crate::ui::components::binding::{
    find_ancestor, find_ancestor_entity, format_f32, get_inspecting_collider,
    get_inspecting_collider_mut,
};

#[derive(Component)]
struct ColliderPropertiesSection;

#[derive(Component)]
struct ColliderPropertiesContent;

#[derive(Component)]
struct ColliderShapeComboBox;

#[derive(Component)]
struct ColliderShapeField(&'static str);

#[derive(Component)]
struct ColliderPositionEdit;

pub fn plugin(app: &mut App) {
    app.add_observer(handle_collider_shape_change)
        .add_observer(handle_collider_text_commit)
        .add_systems(
            Update,
            setup_collider_content.after(super::update_inspected_collider_tracker),
        );
}

pub fn collider_properties_section(asset_server: &AssetServer) -> impl Bundle {
    (
        ColliderPropertiesSection,
        inspector_section(InspectorSection::new("Properties", vec![]), asset_server),
    )
}

fn shape_index(shape: &ParticlesColliderShape3D) -> usize {
    match shape {
        ParticlesColliderShape3D::Box { .. } => 0,
        ParticlesColliderShape3D::Sphere { .. } => 1,
    }
}

fn shape_options() -> Vec<ComboBoxOptionData> {
    vec![
        ComboBoxOptionData::new("Box").with_value("Box"),
        ComboBoxOptionData::new("Sphere").with_value("Sphere"),
    ]
}

fn setup_collider_content(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    editor_state: Res<EditorState>,
    assets: Res<Assets<ParticleSystemAsset>>,
    sections: Query<(Entity, &InspectorSection), With<ColliderPropertiesSection>>,
    existing: Query<Entity, With<ColliderPropertiesContent>>,
) {
    let Some(entity) = section_needs_setup(&sections, &existing) else {
        return;
    };

    let Some((_, collider)) = get_inspecting_collider(&editor_state, &assets) else {
        return;
    };

    let shape = collider.shape.clone();
    let position = collider.position;
    let selected = shape_index(&shape);
    let font: Handle<Font> = asset_server.load(FONT_PATH);

    let content = commands
        .spawn((
            ColliderPropertiesContent,
            DynamicSectionContent,
            Node {
                width: percent(100),
                flex_direction: FlexDirection::Column,
                row_gap: px(12.0),
                ..default()
            },
        ))
        .with_children(|parent| {
            spawn_labeled_combobox(
                parent,
                &font,
                "Shape",
                shape_options(),
                selected,
                ColliderShapeComboBox,
            );

            match &shape {
                ParticlesColliderShape3D::Box { size } => {
                    parent.spawn(fields_row()).with_children(|row| {
                        row.spawn((
                            ColliderShapeField("size"),
                            vector_edit(
                                VectorEditProps::default()
                                    .with_label("Size")
                                    .with_suffixes(VectorSuffixes::XYZ)
                                    .with_default_values(vec![size.x, size.y, size.z]),
                            ),
                        ));
                    });
                }
                ParticlesColliderShape3D::Sphere { radius } => {
                    parent.spawn(fields_row()).with_children(|row| {
                        row.spawn((
                            ColliderShapeField("radius"),
                            text_edit(
                                TextEditProps::default()
                                    .with_label("Radius")
                                    .with_default_value(format_f32(*radius))
                                    .numeric_f32(),
                            ),
                        ));
                    });
                }
            }

            parent.spawn(fields_row()).with_children(|row| {
                row.spawn((
                    ColliderPositionEdit,
                    vector_edit(
                        VectorEditProps::default()
                            .with_label("Position")
                            .with_suffixes(VectorSuffixes::XYZ)
                            .with_default_values(vec![position.x, position.y, position.z]),
                    ),
                ));
            });
        })
        .id();

    commands.entity(entity).add_child(content);
}

fn handle_collider_shape_change(
    trigger: On<ComboBoxChangeEvent>,
    mut commands: Commands,
    shape_comboboxes: Query<(), With<ColliderShapeComboBox>>,
    editor_state: Res<EditorState>,
    mut assets: ResMut<Assets<ParticleSystemAsset>>,
    mut dirty_state: ResMut<DirtyState>,
    existing: Query<Entity, With<ColliderPropertiesContent>>,
) {
    if shape_comboboxes.get(trigger.entity).is_err() {
        return;
    }

    let Some((_, collider)) = get_inspecting_collider_mut(&editor_state, &mut assets) else {
        return;
    };

    let new_shape = match trigger.value.as_deref().unwrap_or(&trigger.label) {
        "Sphere" => ParticlesColliderShape3D::Sphere { radius: 1.0 },
        "Box" => ParticlesColliderShape3D::Box { size: Vec3::ONE },
        _ => return,
    };

    if shape_index(&collider.shape) == shape_index(&new_shape) {
        return;
    }

    collider.shape = new_shape;
    dirty_state.has_unsaved_changes = true;

    for entity in &existing {
        commands.entity(entity).try_despawn();
    }
}

fn handle_collider_text_commit(
    trigger: On<TextEditCommitEvent>,
    parents: Query<&ChildOf>,
    shape_fields: Query<(&ColliderShapeField, &Children)>,
    position_edits: Query<&Children, With<ColliderPositionEdit>>,
    editor_state: Res<EditorState>,
    mut assets: ResMut<Assets<ParticleSystemAsset>>,
    mut dirty_state: ResMut<DirtyState>,
) {
    let Ok(value) = trigger.text.parse::<f32>() else {
        return;
    };

    if let Some(shape_entity) = find_ancestor(trigger.entity, &parents, 10, |e| {
        shape_fields.get(e).is_ok()
    }) {
        let Ok((field, children)) = shape_fields.get(shape_entity) else {
            return;
        };

        let Some((_, collider)) = get_inspecting_collider_mut(&editor_state, &mut assets) else {
            return;
        };

        let changed = match (field.0, &mut collider.shape) {
            ("radius", ParticlesColliderShape3D::Sphere { radius }) => {
                *radius = value;
                true
            }
            ("size", ParticlesColliderShape3D::Box { size }) => {
                match find_vector_component(trigger.entity, children, &parents) {
                    Some(0) => size.x = value,
                    Some(1) => size.y = value,
                    Some(2) => size.z = value,
                    _ => return,
                }
                true
            }
            _ => false,
        };

        if changed {
            dirty_state.has_unsaved_changes = true;
        }
        return;
    }

    if let Some(position_entity) = find_ancestor(trigger.entity, &parents, 10, |e| {
        position_edits.get(e).is_ok()
    }) {
        let Ok(children) = position_edits.get(position_entity) else {
            return;
        };

        let Some((_, collider)) = get_inspecting_collider_mut(&editor_state, &mut assets) else {
            return;
        };

        match find_vector_component(trigger.entity, children, &parents) {
            Some(0) => collider.position.x = value,
            Some(1) => collider.position.y = value,
            Some(2) => collider.position.z = value,
            _ => return,
        }
        dirty_state.has_unsaved_changes = true;
    }
}

fn find_vector_component(
    entity: Entity,
    children: &Children,
    parents: &Query<&ChildOf>,
) -> Option<usize> {
    for (idx, child) in children.iter().enumerate().take(3) {
        if find_ancestor_entity(entity, child, parents) {
            return Some(idx);
        }
    }
    None
}
