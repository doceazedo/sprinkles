use aracari::prelude::*;
use bevy::prelude::*;
use bevy_ui_text_input::TextInputQueue;

use crate::state::{DirtyState, EditorState};
use crate::ui::components::inspector::utils::{VariantConfig, variants_from_reflect};
use crate::ui::widgets::combobox::ComboBoxChangeEvent;
use crate::ui::widgets::inspector_field::fields_row;
use crate::ui::widgets::text_edit::{TextEditCommitEvent, set_text_input_value};
use crate::ui::widgets::variant_edit::{
    VariantComboBox, VariantDefinition, VariantEditConfig, VariantEditProps, VariantFieldBinding,
    variant_edit,
};
use crate::ui::widgets::vector_edit::{EditorVectorEdit, VectorEditProps, VectorSuffixes, vector_edit};

use crate::ui::components::binding::{
    find_ancestor, find_ancestor_entity, format_f32, get_inspecting_collider,
    get_inspecting_collider_mut,
};
use super::{InspectorSection, inspector_section, section_needs_setup};

#[derive(Component)]
struct ColliderPropertiesSection;

#[derive(Component)]
struct ColliderPropertiesContent;

#[derive(Component)]
struct ColliderShapeVariantEdit;

#[derive(Component)]
struct ColliderPositionEdit;

#[derive(Component)]
struct ColliderFieldBound;

pub fn plugin(app: &mut App) {
    app.add_observer(handle_collider_shape_variant_change)
        .add_observer(handle_collider_text_commit)
        .add_systems(
            Update,
            (
                setup_collider_content,
                cleanup_collider_on_switch,
                sync_collider_shape_variant,
                bind_collider_shape_fields,
            )
                .after(super::update_inspected_collider_tracker),
        );
}

pub fn collider_properties_section(asset_server: &AssetServer) -> impl Bundle {
    (
        ColliderPropertiesSection,
        inspector_section(
            InspectorSection::new("Properties", vec![]),
            asset_server,
        ),
    )
}

fn shape_index(shape: &ParticlesColliderShape3D) -> usize {
    match shape {
        ParticlesColliderShape3D::Box { .. } => 0,
        ParticlesColliderShape3D::Sphere { .. } => 1,
    }
}

fn shape_variants() -> Vec<VariantDefinition> {
    variants_from_reflect::<ParticlesColliderShape3D>(&[
        (
            "Box",
            VariantConfig::default()
                .default_value(ParticlesColliderShape3D::Box { size: Vec3::ONE }),
        ),
        (
            "Sphere",
            VariantConfig::default()
                .default_value(ParticlesColliderShape3D::Sphere { radius: 1.0 }),
        ),
    ])
}

fn setup_collider_content(
    mut commands: Commands,
    editor_state: Res<EditorState>,
    assets: Res<Assets<ParticleSystemAsset>>,
    sections: Query<(Entity, &InspectorSection), With<ColliderPropertiesSection>>,
    existing: Query<Entity, With<ColliderPropertiesContent>>,
) {
    let Some(entity) = section_needs_setup(&sections, &existing) else {
        return;
    };

    let collider = get_inspecting_collider(&editor_state, &assets).map(|(_, c)| c);
    let shape = collider.map(|c| &c.shape).cloned().unwrap_or_default();
    let position = collider.map(|c| c.position).unwrap_or(Vec3::ZERO);
    let selected = shape_index(&shape);

    let content = commands
        .spawn((
            ColliderPropertiesContent,
            Node {
                width: percent(100),
                flex_direction: FlexDirection::Column,
                row_gap: px(12.0),
                ..default()
            },
        ))
        .with_children(|parent| {
            parent.spawn(fields_row()).with_children(|row| {
                row.spawn((
                    ColliderShapeVariantEdit,
                    variant_edit(
                        VariantEditProps::new("shape")
                            .with_label("Shape")
                            .with_variants(shape_variants())
                            .with_selected(selected),
                    ),
                ));
            });
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

fn cleanup_collider_on_switch(
    mut commands: Commands,
    tracker: Res<super::InspectedColliderTracker>,
    existing: Query<Entity, With<ColliderPropertiesContent>>,
) {
    if !tracker.is_changed() {
        return;
    }

    for entity in &existing {
        commands.entity(entity).try_despawn();
    }
}

fn handle_collider_shape_variant_change(
    trigger: On<ComboBoxChangeEvent>,
    variant_comboboxes: Query<&VariantComboBox>,
    shape_variant_edits: Query<(), With<ColliderShapeVariantEdit>>,
    editor_state: Res<EditorState>,
    mut assets: ResMut<Assets<ParticleSystemAsset>>,
    mut dirty_state: ResMut<DirtyState>,
) {
    let Ok(variant_combobox) = variant_comboboxes.get(trigger.entity) else {
        return;
    };

    if shape_variant_edits.get(variant_combobox.0).is_err() {
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
}

#[allow(clippy::too_many_arguments)]
fn handle_collider_text_commit(
    trigger: On<TextEditCommitEvent>,
    parents: Query<&ChildOf>,
    variant_field_bindings: Query<&VariantFieldBinding>,
    shape_variant_edits: Query<(), With<ColliderShapeVariantEdit>>,
    position_edits: Query<(), With<ColliderPositionEdit>>,
    children_query: Query<&Children>,
    editor_state: Res<EditorState>,
    mut assets: ResMut<Assets<ParticleSystemAsset>>,
    mut dirty_state: ResMut<DirtyState>,
) {
    if let Some(binding_entity) = find_ancestor(
        trigger.entity,
        &parents,
        10,
        |e| variant_field_bindings.get(e).is_ok(),
    ) {
        let Ok(binding) = variant_field_bindings.get(binding_entity) else {
            return;
        };

        if shape_variant_edits.get(binding.variant_edit).is_ok() {
            let Ok(value) = trigger.text.parse::<f32>() else {
                return;
            };

            let Some((_, collider)) =
                get_inspecting_collider_mut(&editor_state, &mut assets)
            else {
                return;
            };

            let changed = match (binding.field_name.as_str(), &mut collider.shape) {
                ("radius", ParticlesColliderShape3D::Sphere { radius }) => {
                    *radius = value;
                    true
                }
                ("size", ParticlesColliderShape3D::Box { size }) => {
                    if let Ok(vec_children) = children_query.get(binding_entity) {
                        let mut applied = false;
                        for (idx, child) in vec_children.iter().enumerate().take(3) {
                            if find_ancestor_entity(trigger.entity, child, &parents) {
                                match idx {
                                    0 => size.x = value,
                                    1 => size.y = value,
                                    2 => size.z = value,
                                    _ => {}
                                }
                                applied = true;
                                break;
                            }
                        }
                        applied
                    } else {
                        false
                    }
                }
                _ => false,
            };

            if changed {
                dirty_state.has_unsaved_changes = true;
            }
            return;
        }
    }

    if let Some(position_entity) = find_ancestor(
        trigger.entity,
        &parents,
        10,
        |e| position_edits.get(e).is_ok(),
    ) {
        let Ok(value) = trigger.text.parse::<f32>() else {
            return;
        };

        let Ok(vec_children) = children_query.get(position_entity) else {
            return;
        };

        let Some((_, collider)) = get_inspecting_collider_mut(&editor_state, &mut assets) else {
            return;
        };

        for (idx, child) in vec_children.iter().enumerate().take(3) {
            if find_ancestor_entity(trigger.entity, child, &parents) {
                match idx {
                    0 => collider.position.x = value,
                    1 => collider.position.y = value,
                    2 => collider.position.z = value,
                    _ => {}
                }
                dirty_state.has_unsaved_changes = true;
                break;
            }
        }
    }
}

fn sync_collider_shape_variant(
    editor_state: Res<EditorState>,
    assets: Res<Assets<ParticleSystemAsset>>,
    mut variant_edits: Query<&mut VariantEditConfig, With<ColliderShapeVariantEdit>>,
) {
    if !editor_state.is_changed() && !assets.is_changed() {
        return;
    }

    let Some((_, collider)) = get_inspecting_collider(&editor_state, &assets) else {
        return;
    };

    let new_index = shape_index(&collider.shape);
    for mut config in &mut variant_edits {
        if config.selected_index != new_index {
            config.selected_index = new_index;
        }
    }
}

fn bind_collider_shape_fields(
    mut commands: Commands,
    editor_state: Res<EditorState>,
    assets: Res<Assets<ParticleSystemAsset>>,
    variant_field_bindings: Query<(Entity, &VariantFieldBinding), Without<ColliderFieldBound>>,
    shape_variant_edits: Query<(), With<ColliderShapeVariantEdit>>,
    mut text_edits: Query<
        (Entity, &ChildOf, &mut TextInputQueue),
        (
            With<crate::ui::widgets::text_edit::EditorTextEdit>,
            Without<ColliderFieldBound>,
        ),
    >,
    parents: Query<&ChildOf>,
    vector_edit_children: Query<&Children, With<EditorVectorEdit>>,
) {
    let Some((_, collider)) = get_inspecting_collider(&editor_state, &assets) else {
        return;
    };

    for (binding_entity, binding) in &variant_field_bindings {
        if shape_variant_edits.get(binding.variant_edit).is_err() {
            continue;
        }

        match binding.field_name.as_str() {
            "radius" => {
                if let ParticlesColliderShape3D::Sphere { radius } = &collider.shape {
                    let text = format_f32(*radius);
                    let mut found = false;
                    for (text_edit_entity, text_edit_parent, mut queue) in &mut text_edits {
                        if find_ancestor_entity(
                            text_edit_parent.parent(),
                            binding_entity,
                            &parents,
                        ) {
                            set_text_input_value(&mut queue, text.clone());
                            commands
                                .entity(text_edit_entity)
                                .try_insert(ColliderFieldBound);
                            found = true;
                            break;
                        }
                    }
                    if found {
                        commands
                            .entity(binding_entity)
                            .try_insert(ColliderFieldBound);
                    }
                }
            }
            "size" => {
                if let ParticlesColliderShape3D::Box { size } = &collider.shape {
                    if let Ok(vec_children) = vector_edit_children.get(binding_entity) {
                        let mut bound_count = 0;
                        for (idx, vec_child) in vec_children.iter().enumerate().take(3) {
                            let val = match idx {
                                0 => size.x,
                                1 => size.y,
                                2 => size.z,
                                _ => continue,
                            };
                            let text = format_f32(val);
                            for (text_edit_entity, text_edit_parent, mut queue) in
                                &mut text_edits
                            {
                                if find_ancestor_entity(
                                    text_edit_parent.parent(),
                                    vec_child,
                                    &parents,
                                ) {
                                    set_text_input_value(&mut queue, text.clone());
                                    commands
                                        .entity(text_edit_entity)
                                        .try_insert(ColliderFieldBound);
                                    bound_count += 1;
                                    break;
                                }
                            }
                        }
                        if bound_count == 3 {
                            commands
                                .entity(binding_entity)
                                .try_insert(ColliderFieldBound);
                        }
                    }
                }
            }
            _ => {}
        }
    }
}
