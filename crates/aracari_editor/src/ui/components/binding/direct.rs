use aracari::prelude::*;
use bevy::ecs::system::ParamSet;
use bevy::prelude::*;
use bevy_ui_text_input::TextInputQueue;

use crate::state::{DirtyState, EditorState};
use crate::viewport::RespawnEmittersEvent;
use crate::ui::widgets::checkbox::{CheckboxCommitEvent, CheckboxState};
use crate::ui::widgets::combobox::ComboBoxChangeEvent;
use crate::ui::widgets::curve_edit::{CurveEditCommitEvent, CurveEditState, EditorCurveEdit};
use crate::ui::widgets::text_edit::{EditorTextEdit, TextEditCommitEvent, set_text_input_value};
use crate::ui::widgets::variant_edit::{
    EditorVariantEdit, VariantComboBox, VariantEditConfig, VariantFieldBinding,
};
use crate::ui::widgets::vector_edit::EditorVectorEdit;

use super::{
    Bound, Field, FieldKind, FieldValue, InspectedEmitterTracker, MAX_ANCESTOR_DEPTH,
    ReflectPath, find_ancestor, find_ancestor_entity, find_ancestor_field, find_field_for_entity,
    format_f32, get_field_value_by_reflection, get_inspecting_emitter, get_inspecting_emitter_mut,
    get_vec3_component, label_to_variant_name, mark_dirty_and_restart, parse_field_value,
    set_field_enum_by_name, set_field_value_by_reflection, set_variant_field_enum_by_name,
    set_vec3_component,
};

fn is_descendant_of_variant_edit(
    entity: Entity,
    variant_edit_query: &Query<(), With<EditorVariantEdit>>,
    parents: &Query<&ChildOf>,
) -> bool {
    find_ancestor(entity, parents, MAX_ANCESTOR_DEPTH * 2, |e| {
        variant_edit_query.get(e).is_ok()
    })
    .is_some()
}

pub(super) fn bind_values_to_inputs(
    mut commands: Commands,
    editor_state: Res<EditorState>,
    assets: Res<Assets<ParticleSystemAsset>>,
    tracker: Res<InspectedEmitterTracker>,
    new_fields: Query<Entity, Added<Field>>,
    new_text_edits: Query<Entity, Added<EditorTextEdit>>,
    fields: Query<&Field>,
    mut text_edits: Query<(Entity, &ChildOf, &mut TextInputQueue), With<EditorTextEdit>>,
    mut checkbox_set: ParamSet<(
        Query<Entity, Added<CheckboxState>>,
        Query<(Entity, &mut CheckboxState)>,
    )>,
    parents: Query<&ChildOf>,
    variant_edit_query: Query<(), With<EditorVariantEdit>>,
    vector_edit_children: Query<&Children, With<EditorVectorEdit>>,
) {
    let has_new_checkboxes = !checkbox_set.p0().is_empty();
    let has_new_widgets = !new_fields.is_empty() || !new_text_edits.is_empty() || has_new_checkboxes;
    if !tracker.is_changed() && !has_new_widgets {
        return;
    }

    let Some((_, emitter)) = get_inspecting_emitter(&editor_state, &assets) else {
        return;
    };

    for (entity, child_of, mut queue) in &mut text_edits {
        let Some((field_entity, field)) = find_ancestor_field(child_of.parent(), &fields, &parents) else {
            continue;
        };

        if is_descendant_of_variant_edit(child_of.parent(), &variant_edit_query, &parents) {
            continue;
        }

        // handle Vector fields by finding the component index
        if let FieldKind::Vector(suffixes) = &field.kind {
            let value = get_field_value_by_reflection(emitter, &field.path, &field.kind);
            let component_count = suffixes.vector_size().count();

            // find the vector edit ancestor and determine component index
            if let Some(vec_edit_entity) =
                find_ancestor(child_of.parent(), &parents, MAX_ANCESTOR_DEPTH, |e| {
                    vector_edit_children.get(e).is_ok()
                })
            {
                if let Ok(vec_children) = vector_edit_children.get(vec_edit_entity) {
                    for (idx, vec_child) in vec_children.iter().enumerate().take(component_count) {
                        if find_ancestor_entity(child_of.parent(), vec_child, &parents) {
                            let component_value = match &value {
                                FieldValue::Vec3(vec) => Some(get_vec3_component(*vec, idx)),
                                FieldValue::Range(min, max) => match idx {
                                    0 => Some(*min),
                                    1 => Some(*max),
                                    _ => None,
                                },
                                _ => None,
                            };
                            if let Some(v) = component_value {
                                set_text_input_value(&mut queue, format_f32(v));
                                commands.entity(entity).try_insert(Bound::direct(field_entity));
                            }
                            break;
                        }
                    }
                }
            }
            continue;
        }

        let value = get_field_value_by_reflection(emitter, &field.path, &field.kind);
        let text = value.to_display_string(&field.kind).unwrap_or_default();

        set_text_input_value(&mut queue, text);
        commands.entity(entity).try_insert(Bound::direct(field_entity));
    }

    for (entity, mut state) in &mut checkbox_set.p1() {
        if let Ok(child_of) = parents.get(entity) {
            if is_descendant_of_variant_edit(child_of.parent(), &variant_edit_query, &parents) {
                continue;
            }
        }

        let Some((field_entity, field)) = find_field_for_entity(entity, &fields, &parents) else {
            continue;
        };

        let value = get_field_value_by_reflection(emitter, &field.path, &field.kind);
        if let Some(checked) = value.to_bool() {
            state.checked = checked;
        }
        commands.entity(entity).try_insert(Bound::direct(field_entity));
    }
}

pub(super) fn handle_combobox_change(
    trigger: On<ComboBoxChangeEvent>,
    editor_state: Res<EditorState>,
    mut assets: ResMut<Assets<ParticleSystemAsset>>,
    mut dirty_state: ResMut<DirtyState>,
    variant_field_bindings: Query<&VariantFieldBinding>,
    variant_edit_configs: Query<&VariantEditConfig>,
    fields: Query<&Field>,
    mut emitter_runtimes: Query<&mut EmitterRuntime>,
    parents: Query<&ChildOf>,
    variant_comboboxes: Query<(), With<VariantComboBox>>,
) {
    if variant_comboboxes.get(trigger.entity).is_ok() {
        return;
    }

    let Some((_, emitter)) = get_inspecting_emitter_mut(&editor_state, &mut assets) else {
        return;
    };

    let variant_name = trigger
        .value
        .clone()
        .unwrap_or_else(|| label_to_variant_name(&trigger.label));
    let mut changed = false;

    let binding = find_ancestor(trigger.entity, &parents, MAX_ANCESTOR_DEPTH, |e| {
        variant_field_bindings.get(e).is_ok()
    })
    .and_then(|e| variant_field_bindings.get(e).ok());

    if let Some(binding) = binding {
        if let Ok(config) = variant_edit_configs.get(binding.variant_edit) {
            changed = set_variant_field_enum_by_name(
                emitter,
                &config.path,
                &binding.field_name,
                &variant_name,
            );
        }
    } else if let Some((_, field)) = find_ancestor_field(trigger.entity, &fields, &parents) {
        changed = set_field_enum_by_name(emitter, &field.path, &variant_name);
    }

    if changed {
        mark_dirty_and_restart(&mut dirty_state, &mut emitter_runtimes, emitter.time.fixed_seed);
    }
}

pub(super) fn bind_curve_edit_values(
    mut commands: Commands,
    editor_state: Res<EditorState>,
    assets: Res<Assets<ParticleSystemAsset>>,
    tracker: Res<InspectedEmitterTracker>,
    new_curve_edits: Query<Entity, Added<EditorCurveEdit>>,
    mut curve_edits: Query<(Entity, Option<&ChildOf>, &mut CurveEditState), With<EditorCurveEdit>>,
    fields: Query<&Field>,
    parents: Query<&ChildOf>,
    variant_edit_query: Query<(), With<EditorVariantEdit>>,
) {
    if !tracker.is_changed() && new_curve_edits.is_empty() {
        return;
    }

    let Some((_, emitter)) = get_inspecting_emitter(&editor_state, &assets) else {
        return;
    };

    for (entity, child_of, mut state) in &mut curve_edits {
        let Some((field_entity, field)) = find_field_for_entity(entity, &fields, &parents) else {
            continue;
        };

        if let Some(child_of) = child_of {
            if is_descendant_of_variant_edit(child_of.parent(), &variant_edit_query, &parents) {
                continue;
            }
        }

        if field.kind != FieldKind::Curve {
            continue;
        }

        let reflect_path = ReflectPath::new(&field.path);
        let Ok(value) = emitter.reflect_path(reflect_path.as_str()) else {
            continue;
        };

        // try binding directly to CurveTexture
        if let Some(curve_texture) = value.try_downcast_ref::<CurveTexture>() {
            state.set_curve(curve_texture.clone());
            commands.entity(entity).try_insert(Bound::direct(field_entity));
            continue;
        }

        // try binding to Option<CurveTexture>
        if let Some(curve_opt) = value.try_downcast_ref::<Option<CurveTexture>>() {
            if let Some(curve) = curve_opt {
                state.set_curve(curve.clone());
            }
            // mark as bound even if None so we can create the curve on commit
            commands.entity(entity).try_insert(Bound::direct(field_entity));
        }
    }
}

pub(super) fn handle_curve_edit_commit(
    trigger: On<CurveEditCommitEvent>,
    editor_state: Res<EditorState>,
    mut assets: ResMut<Assets<ParticleSystemAsset>>,
    mut dirty_state: ResMut<DirtyState>,
    curve_edits: Query<&Bound, With<EditorCurveEdit>>,
    fields: Query<&Field>,
    parents: Query<&ChildOf>,
    mut emitter_runtimes: Query<&mut EmitterRuntime>,
) {
    let Ok(bound) = curve_edits.get(trigger.entity) else {
        return;
    };

    if bound.is_variant_field {
        return;
    }

    let Some((_, field)) = find_field_for_entity(trigger.entity, &fields, &parents) else {
        return;
    };

    let Some((_, emitter)) = get_inspecting_emitter_mut(&editor_state, &mut assets) else {
        return;
    };

    let reflect_path = ReflectPath::new(&field.path);
    let Ok(target) = emitter.reflect_path_mut(reflect_path.as_str()) else {
        return;
    };

    // handle direct CurveTexture binding
    if let Some(curve_texture) = target.try_downcast_mut::<CurveTexture>() {
        *curve_texture = trigger.curve.clone();
        mark_dirty_and_restart(&mut dirty_state, &mut emitter_runtimes, emitter.time.fixed_seed);
        return;
    }

    // handle Option<CurveTexture> binding
    if let Some(curve_opt) = target.try_downcast_mut::<Option<CurveTexture>>() {
        match curve_opt {
            Some(curve) => {
                *curve = trigger.curve.clone();
            }
            None => {
                *curve_opt = Some(trigger.curve.clone());
            }
        }
        mark_dirty_and_restart(&mut dirty_state, &mut emitter_runtimes, emitter.time.fixed_seed);
    }
}

pub(super) fn handle_text_edit_commit(
    trigger: On<TextEditCommitEvent>,
    editor_state: Res<EditorState>,
    mut assets: ResMut<Assets<ParticleSystemAsset>>,
    mut dirty_state: ResMut<DirtyState>,
    bound_widgets: Query<&Bound>,
    fields: Query<&Field>,
    variant_field_bindings: Query<(&VariantFieldBinding, &ChildOf)>,
    variant_edit_configs: Query<&VariantEditConfig>,
    parents: Query<&ChildOf>,
    mut emitter_runtimes: Query<&mut EmitterRuntime>,
    vector_edit_children: Query<&Children, With<EditorVectorEdit>>,
) {
    let Ok(bound) = bound_widgets.get(trigger.entity) else {
        return;
    };

    let Some((_, emitter)) = get_inspecting_emitter_mut(&editor_state, &mut assets) else {
        return;
    };

    if bound.is_variant_field {
        super::variant::handle_variant_text_commit(
            trigger.entity,
            &trigger.text,
            emitter,
            &variant_field_bindings,
            &variant_edit_configs,
            &parents,
            &vector_edit_children,
            &mut dirty_state,
            &mut emitter_runtimes,
        );
    } else {
        handle_direct_text_commit(
            trigger.entity,
            &trigger.text,
            emitter,
            &fields,
            &parents,
            &vector_edit_children,
            &mut dirty_state,
            &mut emitter_runtimes,
        );
    }
}

fn handle_direct_text_commit(
    entity: Entity,
    text: &str,
    emitter: &mut EmitterData,
    fields: &Query<&Field>,
    parents: &Query<&ChildOf>,
    vector_edit_children: &Query<&Children, With<EditorVectorEdit>>,
    dirty_state: &mut DirtyState,
    emitter_runtimes: &mut Query<&mut EmitterRuntime>,
) {
    let Some(child_of) = parents.get(entity).ok() else {
        return;
    };

    let Some((_, field)) = find_ancestor_field(child_of.parent(), fields, parents) else {
        return;
    };

    // handle Vector fields
    if let FieldKind::Vector(suffixes) = &field.kind {
        let current_value = get_field_value_by_reflection(emitter, &field.path, &field.kind);
        let component_count = suffixes.vector_size().count();

        let vec_edit_entity = find_ancestor(child_of.parent(), parents, MAX_ANCESTOR_DEPTH, |e| {
            vector_edit_children.get(e).is_ok()
        });
        let Some(vec_edit_entity) = vec_edit_entity else {
            return;
        };
        let Ok(vec_children) = vector_edit_children.get(vec_edit_entity) else {
            return;
        };

        let mut component_idx = None;
        for (idx, vec_child) in vec_children.iter().enumerate().take(component_count) {
            if find_ancestor_entity(child_of.parent(), vec_child, parents) {
                component_idx = Some(idx);
                break;
            }
        }

        let Some(idx) = component_idx else {
            return;
        };

        let Ok(v) = text.trim().parse::<f32>() else {
            return;
        };

        let new_value = match current_value {
            FieldValue::Vec3(mut vec) => {
                set_vec3_component(&mut vec, idx, v);
                FieldValue::Vec3(vec)
            }
            FieldValue::Range(min, max) => match idx {
                0 => FieldValue::Range(v, max),
                1 => FieldValue::Range(min, v),
                _ => return,
            },
            _ => return,
        };

        if set_field_value_by_reflection(emitter, &field.path, &new_value) {
            mark_dirty_and_restart(dirty_state, emitter_runtimes, emitter.time.fixed_seed);
        }
        return;
    }

    let value = parse_field_value(text, &field.kind);

    if matches!(value, FieldValue::None) {
        return;
    }

    if set_field_value_by_reflection(emitter, &field.path, &value) {
        mark_dirty_and_restart(dirty_state, emitter_runtimes, emitter.time.fixed_seed);
    }
}

pub(super) fn handle_checkbox_commit(
    trigger: On<CheckboxCommitEvent>,
    mut commands: Commands,
    editor_state: Res<EditorState>,
    mut assets: ResMut<Assets<ParticleSystemAsset>>,
    mut dirty_state: ResMut<DirtyState>,
    bound_widgets: Query<&Bound>,
    fields: Query<&Field>,
    variant_field_bindings: Query<&VariantFieldBinding>,
    variant_edit_configs: Query<&VariantEditConfig>,
    parents: Query<&ChildOf>,
    mut emitter_runtimes: Query<&mut EmitterRuntime>,
) {
    let Ok(bound) = bound_widgets.get(trigger.entity) else {
        return;
    };

    let Some((_, emitter)) = get_inspecting_emitter_mut(&editor_state, &mut assets) else {
        return;
    };

    let value = FieldValue::Bool(trigger.checked);

    if bound.is_variant_field {
        let Ok(binding) = variant_field_bindings.get(trigger.entity) else {
            return;
        };
        let Ok(config) = variant_edit_configs.get(binding.variant_edit) else {
            return;
        };
        if super::set_variant_field_value_by_reflection(
            emitter,
            &config.path,
            &binding.field_name,
            &value,
        ) {
            mark_dirty_and_restart(&mut dirty_state, &mut emitter_runtimes, emitter.time.fixed_seed);
        }
    } else {
        let Some((_, field)) = find_field_for_entity(trigger.entity, &fields, &parents) else {
            return;
        };
        if super::set_field_value_by_reflection(emitter, &field.path, &value) {
            mark_dirty_and_restart(&mut dirty_state, &mut emitter_runtimes, emitter.time.fixed_seed);
            if field.path == "enabled" {
                commands.trigger(RespawnEmittersEvent);
            }
        }
    }
}
