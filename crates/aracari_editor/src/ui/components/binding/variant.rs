use aracari::prelude::*;
use bevy::prelude::*;
use bevy::reflect::{PartialReflect, ReflectRef};
use bevy_ui_text_input::TextInputQueue;

use crate::state::{DirtyState, EditorState};
use crate::ui::widgets::checkbox::CheckboxState;
use crate::ui::widgets::color_picker::{
    ColorPickerCommitEvent, ColorPickerState, EditorColorPicker, TriggerSwatchMaterial,
};
use crate::ui::widgets::combobox::{ComboBoxChangeEvent, ComboBoxConfig};
use crate::ui::widgets::text_edit::set_text_input_value;
use crate::ui::widgets::gradient_edit::{
    EditorGradientEdit, GradientEditCommitEvent, GradientEditState,
};
use crate::ui::widgets::texture_edit::TextureEditCommitEvent;
use crate::ui::widgets::variant_edit::{
    EditorVariantEdit, VariantComboBox, VariantEditConfig, VariantFieldBinding,
    VariantDefinition,
};
use crate::ui::widgets::vector_edit::EditorVectorEdit;

use super::{
    Bound, FieldKind, FieldValue, InspectedEmitterTracker, MAX_ANCESTOR_DEPTH, ReflectPath,
    create_variant_from_definition, find_ancestor, find_ancestor_entity, format_f32,
    get_inspecting_emitter, get_inspecting_emitter_mut, get_variant_field_value_by_reflection,
    get_variant_index_by_reflection, get_vec3_component, mark_dirty_and_restart,
    resolve_variant_field_ref, set_variant_field_value_by_reflection, set_vec3_component,
    with_variant_field_mut, Field,
};

pub(super) fn bind_variant_edits(
    editor_state: Res<EditorState>,
    assets: Res<Assets<ParticleSystemAsset>>,
    tracker: Res<InspectedEmitterTracker>,
    mut variant_edits: Query<(&Field, &mut VariantEditConfig), With<EditorVariantEdit>>,
    new_variant_edits: Query<Entity, Added<EditorVariantEdit>>,
) {
    if !tracker.is_changed() && new_variant_edits.is_empty() {
        return;
    }

    let Some((_, emitter)) = get_inspecting_emitter(&editor_state, &assets) else {
        return;
    };

    for (field, mut config) in &mut variant_edits {
        let Some(new_index) =
            get_variant_index_by_reflection(emitter, &field.path, &config.variants)
        else {
            continue;
        };

        config.selected_index = new_index;
    }
}

pub(super) fn bind_variant_field_values(
    mut commands: Commands,
    editor_state: Res<EditorState>,
    assets: Res<Assets<ParticleSystemAsset>>,
    mut variant_field_bindings: Query<
        (Entity, &VariantFieldBinding, Option<&mut ComboBoxConfig>),
        Without<Bound>,
    >,
    variant_edit_configs: Query<&VariantEditConfig>,
    mut text_edits: Query<
        (Entity, &ChildOf, &mut TextInputQueue),
        (With<crate::ui::widgets::text_edit::EditorTextEdit>, Without<Bound>),
    >,
    mut checkbox_states: Query<&mut CheckboxState>,
    parents: Query<&ChildOf>,
    vector_edit_children: Query<&Children, With<EditorVectorEdit>>,
) {
    let Some((_, emitter)) = get_inspecting_emitter(&editor_state, &assets) else {
        return;
    };

    for (binding_entity, binding, mut combobox_config) in &mut variant_field_bindings {
        let Ok(config) = variant_edit_configs.get(binding.variant_edit) else {
            continue;
        };

        let value = get_variant_field_value_by_reflection(
            emitter,
            &config.path,
            &binding.field_name,
            &binding.field_kind,
        );

        let Some(value) = value else {
            continue;
        };

        let mut bound = false;

        if let Some(text) = value.to_display_string(&binding.field_kind) {
            for (text_edit_entity, text_edit_parent, mut queue) in &mut text_edits {
                if find_ancestor_entity(text_edit_parent.parent(), binding_entity, &parents) {
                    set_text_input_value(&mut queue, text.clone());
                    commands
                        .entity(text_edit_entity)
                        .try_insert(Bound::variant(binding_entity));
                    bound = true;
                    break;
                }
            }
        }

        if let Some(checked) = value.to_bool() {
            if let Ok(mut state) = checkbox_states.get_mut(binding_entity) {
                state.checked = checked;
                bound = true;
            }
        }

        if matches!(binding.field_kind, FieldKind::ComboBox { .. }) {
            if let FieldValue::U32(index) = &value {
                if let Some(ref mut config) = combobox_config {
                    config.selected = *index as usize;
                    bound = true;
                }
            }
        }

        if let Some(vec) = value.to_vec3() {
            if let Ok(vec_children) = vector_edit_children.get(binding_entity) {
                let mut component_index = 0;

                for vec_child in vec_children.iter().take(3) {
                    for (text_edit_entity, text_edit_parent, mut queue) in &mut text_edits {
                        if find_ancestor_entity(text_edit_parent.parent(), vec_child, &parents) {
                            let text = format_f32(get_vec3_component(vec, component_index));
                            set_text_input_value(&mut queue, text);
                            commands
                                .entity(text_edit_entity)
                                .try_insert(Bound::variant(binding_entity));
                            component_index += 1;
                            break;
                        }
                    }
                }

                if component_index == 3 {
                    bound = true;
                }
            }
        }
    }
}

pub(super) fn handle_variant_change(
    trigger: On<ComboBoxChangeEvent>,
    editor_state: Res<EditorState>,
    mut assets: ResMut<Assets<ParticleSystemAsset>>,
    mut dirty_state: ResMut<DirtyState>,
    variant_comboboxes: Query<&VariantComboBox>,
    variant_edit_configs: Query<&VariantEditConfig>,
    variant_field_bindings: Query<&VariantFieldBinding>,
    mut emitter_runtimes: Query<&mut EmitterRuntime>,
) {
    let combobox_entity = trigger.entity;

    let Ok(variant_combobox) = variant_comboboxes.get(combobox_entity) else {
        return;
    };

    let variant_edit_entity = variant_combobox.0;
    let Ok(config) = variant_edit_configs.get(variant_edit_entity) else {
        return;
    };

    let Some(variant_def) = config.variants.get(trigger.selected) else {
        return;
    };

    let Some((_, emitter)) = get_inspecting_emitter_mut(&editor_state, &mut assets) else {
        return;
    };

    // nested variant edit: write through parent's path + field resolution
    if let Ok(binding) = variant_field_bindings.get(variant_edit_entity) {
        let Ok(parent_config) = variant_edit_configs.get(binding.variant_edit) else {
            return;
        };

        let Some(default_value) = variant_def.create_default() else {
            return;
        };

        let reflect_path = ReflectPath::new(&parent_config.path);
        let Ok(target) = emitter.reflect_path_mut(reflect_path.as_str()) else {
            return;
        };

        let changed = with_variant_field_mut(target, &binding.field_name, |field| {
            field.apply(default_value.as_ref());
        })
        .is_some();

        if changed {
            mark_dirty_and_restart(
                &mut dirty_state,
                &mut emitter_runtimes,
                emitter.time.fixed_seed,
            );
        }
    } else if create_variant_from_definition(emitter, &config.path, variant_def) {
        mark_dirty_and_restart(&mut dirty_state, &mut emitter_runtimes, emitter.time.fixed_seed);
    }
}

fn find_variant_field_binding<'a>(
    entity: Entity,
    bindings: &'a Query<(&VariantFieldBinding, &ChildOf)>,
    parents: &Query<&ChildOf>,
) -> Option<(&'a VariantFieldBinding, Entity)> {
    find_ancestor(entity, parents, MAX_ANCESTOR_DEPTH, |e| bindings.get(e).is_ok())
        .and_then(|e| bindings.get(e).ok().map(|(binding, _)| (binding, e)))
}

#[allow(clippy::too_many_arguments)]
pub(super) fn handle_variant_text_commit(
    entity: Entity,
    text: &str,
    emitter: &mut EmitterData,
    variant_field_bindings: &Query<(&VariantFieldBinding, &ChildOf)>,
    variant_edit_configs: &Query<&VariantEditConfig>,
    parents: &Query<&ChildOf>,
    vector_edit_children: &Query<&Children, With<EditorVectorEdit>>,
    dirty_state: &mut DirtyState,
    emitter_runtimes: &mut Query<&mut EmitterRuntime>,
) {
    let Some(child_of) = parents.get(entity).ok() else {
        return;
    };

    let binding = find_variant_field_binding(child_of.parent(), variant_field_bindings, parents);
    let Some((binding, binding_entity)) = binding else {
        return;
    };

    let Ok(config) = variant_edit_configs.get(binding.variant_edit) else {
        return;
    };

    let value = match &binding.field_kind {
        FieldKind::F32 | FieldKind::F32Percent => text
            .trim()
            .parse::<f32>()
            .map(FieldValue::F32)
            .unwrap_or(FieldValue::None),
        FieldKind::U32 | FieldKind::U32OrEmpty | FieldKind::OptionalU32 => text
            .trim()
            .parse::<u32>()
            .map(FieldValue::U32)
            .unwrap_or(FieldValue::None),
        FieldKind::Bool => FieldValue::None,
        FieldKind::Vector(suffixes) => {
            let Ok(vec_children) = vector_edit_children.get(binding_entity) else {
                return;
            };
            let kind = FieldKind::Vector(*suffixes);
            let Some(FieldValue::Vec3(mut vec)) = get_variant_field_value_by_reflection(
                emitter,
                &config.path,
                &binding.field_name,
                &kind,
            ) else {
                return;
            };

            for (idx, vec_child) in vec_children.iter().enumerate().take(3) {
                if find_ancestor_entity(child_of.parent(), vec_child, parents) {
                    if let Ok(v) = text.trim().parse::<f32>() {
                        set_vec3_component(&mut vec, idx, v);
                    }
                    break;
                }
            }
            FieldValue::Vec3(vec)
        }
        FieldKind::ComboBox { .. } | FieldKind::Color | FieldKind::Gradient | FieldKind::Curve | FieldKind::AnimatedVelocity | FieldKind::TextureRef => FieldValue::None,
    };

    if matches!(value, FieldValue::None) {
        return;
    }

    let changed =
        set_variant_field_value_by_reflection(emitter, &config.path, &binding.field_name, &value);

    if changed {
        mark_dirty_and_restart(dirty_state, emitter_runtimes, emitter.time.fixed_seed);
    }
}

pub(super) fn bind_variant_color_pickers(
    mut commands: Commands,
    editor_state: Res<EditorState>,
    assets: Res<Assets<ParticleSystemAsset>>,
    mut color_pickers: Query<
        (Entity, &mut ColorPickerState, &VariantFieldBinding),
        (With<EditorColorPicker>, Without<Bound>),
    >,
    variant_edit_configs: Query<&VariantEditConfig>,
    trigger_swatches: Query<&TriggerSwatchMaterial>,
) {
    let Some((_, emitter)) = get_inspecting_emitter(&editor_state, &assets) else {
        return;
    };

    for (picker_entity, mut picker_state, binding) in &mut color_pickers {
        if !matches!(binding.field_kind, FieldKind::Color) {
            continue;
        }

        let trigger_ready = trigger_swatches
            .iter()
            .any(|swatch| swatch.0 == picker_entity);
        if !trigger_ready {
            continue;
        }

        let Ok(config) = variant_edit_configs.get(binding.variant_edit) else {
            continue;
        };

        let value = get_variant_field_value_by_reflection(
            emitter,
            &config.path,
            &binding.field_name,
            &binding.field_kind,
        );

        let Some(color) = value.and_then(|v| v.to_color()) else {
            continue;
        };

        picker_state.set_from_rgba(color);
        commands
            .entity(picker_entity)
            .try_insert(Bound::variant(picker_entity));
    }
}

pub(super) fn handle_variant_color_commit(
    trigger: On<ColorPickerCommitEvent>,
    editor_state: Res<EditorState>,
    mut assets: ResMut<Assets<ParticleSystemAsset>>,
    mut dirty_state: ResMut<DirtyState>,
    color_pickers: Query<&VariantFieldBinding, With<EditorColorPicker>>,
    variant_edit_configs: Query<&VariantEditConfig>,
    mut emitter_runtimes: Query<&mut EmitterRuntime>,
) {
    let Ok(binding) = color_pickers.get(trigger.entity) else {
        return;
    };

    let Ok(config) = variant_edit_configs.get(binding.variant_edit) else {
        return;
    };

    let Some((_, emitter)) = get_inspecting_emitter_mut(&editor_state, &mut assets) else {
        return;
    };

    let value = FieldValue::Color(trigger.color);
    let changed =
        set_variant_field_value_by_reflection(emitter, &config.path, &binding.field_name, &value);

    if changed {
        mark_dirty_and_restart(&mut dirty_state, &mut emitter_runtimes, emitter.time.fixed_seed);
    }
}

pub(super) fn bind_variant_gradient_edits(
    mut commands: Commands,
    editor_state: Res<EditorState>,
    assets: Res<Assets<ParticleSystemAsset>>,
    mut gradient_edits: Query<
        (Entity, &mut GradientEditState, &VariantFieldBinding),
        (With<EditorGradientEdit>, Without<Bound>),
    >,
    variant_edit_configs: Query<&VariantEditConfig>,
) {
    let Some((_, emitter)) = get_inspecting_emitter(&editor_state, &assets) else {
        return;
    };

    for (entity, mut state, binding) in &mut gradient_edits {
        if !matches!(binding.field_kind, FieldKind::Gradient) {
            continue;
        }

        let Ok(config) = variant_edit_configs.get(binding.variant_edit) else {
            continue;
        };

        let reflect_path = ReflectPath::new(&config.path);
        let Ok(value) = emitter.reflect_path(reflect_path.as_str()) else {
            continue;
        };

        let gradient = resolve_variant_field_ref(value, &binding.field_name)
            .and_then(|f| f.try_downcast_ref::<ParticleGradient>().cloned());

        if let Some(gradient) = gradient {
            state.gradient = gradient;
            commands
                .entity(entity)
                .try_insert(Bound::variant(entity));
        }
    }
}

pub(super) fn handle_variant_gradient_commit(
    trigger: On<GradientEditCommitEvent>,
    editor_state: Res<EditorState>,
    mut assets: ResMut<Assets<ParticleSystemAsset>>,
    mut dirty_state: ResMut<DirtyState>,
    gradient_edits: Query<&VariantFieldBinding, With<EditorGradientEdit>>,
    variant_edit_configs: Query<&VariantEditConfig>,
    mut emitter_runtimes: Query<&mut EmitterRuntime>,
) {
    let Ok(binding) = gradient_edits.get(trigger.entity) else {
        return;
    };

    let Ok(config) = variant_edit_configs.get(binding.variant_edit) else {
        return;
    };

    let Some((_, emitter)) = get_inspecting_emitter_mut(&editor_state, &mut assets) else {
        return;
    };

    let reflect_path = ReflectPath::new(&config.path);
    let Ok(target) = emitter.reflect_path_mut(reflect_path.as_str()) else {
        return;
    };

    let changed = with_variant_field_mut(target, &binding.field_name, |field| {
        field.apply(&trigger.gradient);
    })
    .is_some();

    if changed {
        mark_dirty_and_restart(&mut dirty_state, &mut emitter_runtimes, emitter.time.fixed_seed);
    }
}

pub(super) fn bind_nested_variant_edits(
    editor_state: Res<EditorState>,
    assets: Res<Assets<ParticleSystemAsset>>,
    tracker: Res<InspectedEmitterTracker>,
    mut nested_variant_edits: Query<
        (&VariantFieldBinding, &mut VariantEditConfig),
        With<EditorVariantEdit>,
    >,
    new_variant_edits: Query<Entity, Added<EditorVariantEdit>>,
    parent_configs: Query<&VariantEditConfig, Without<VariantFieldBinding>>,
) {
    if !tracker.is_changed() && new_variant_edits.is_empty() {
        return;
    }

    let Some((_, emitter)) = get_inspecting_emitter(&editor_state, &assets) else {
        return;
    };

    for (binding, mut config) in &mut nested_variant_edits {
        let Ok(parent_config) = parent_configs.get(binding.variant_edit) else {
            continue;
        };

        let reflect_path = ReflectPath::new(&parent_config.path);
        let Ok(value) = emitter.reflect_path(reflect_path.as_str()) else {
            continue;
        };

        let Some(field_value) = resolve_variant_field_ref(value, &binding.field_name) else {
            continue;
        };

        let new_index = get_nested_variant_index(field_value, &config.variants);
        config.selected_index = new_index;
    }
}

pub(super) fn handle_variant_texture_commit(
    trigger: On<TextureEditCommitEvent>,
    editor_state: Res<EditorState>,
    mut assets: ResMut<Assets<ParticleSystemAsset>>,
    mut dirty_state: ResMut<DirtyState>,
    variant_field_bindings: Query<&VariantFieldBinding>,
    variant_edit_configs: Query<&VariantEditConfig>,
    mut emitter_runtimes: Query<&mut EmitterRuntime>,
) {
    let variant_edit = trigger.entity;

    let Ok(binding) = variant_field_bindings.get(variant_edit) else {
        return;
    };

    let Ok(parent_config) = variant_edit_configs.get(binding.variant_edit) else {
        return;
    };

    let Some((_, emitter)) = get_inspecting_emitter_mut(&editor_state, &mut assets) else {
        return;
    };

    let reflect_path = ReflectPath::new(&parent_config.path);
    let Ok(target) = emitter.reflect_path_mut(reflect_path.as_str()) else {
        return;
    };

    let changed = with_variant_field_mut(target, &binding.field_name, |field| {
        field.apply(&trigger.value);
    })
    .is_some();

    if changed {
        mark_dirty_and_restart(&mut dirty_state, &mut emitter_runtimes, emitter.time.fixed_seed);
    }
}

fn get_nested_variant_index(
    value: &dyn PartialReflect,
    variants: &[VariantDefinition],
) -> usize {
    let ReflectRef::Enum(enum_ref) = value.reflect_ref() else {
        return 0;
    };

    let variant_name = enum_ref.variant_name();

    if let Some(pos) = find_variant_index_by_name(variant_name, variants) {
        return pos;
    }

    // for Option::Some, check the inner value's variant name
    if variant_name == "Some" {
        if let Some(inner) = enum_ref.field_at(0) {
            if let ReflectRef::Enum(inner_enum) = inner.reflect_ref() {
                let inner_name = inner_enum.variant_name();
                if let Some(pos) = find_variant_index_by_name(inner_name, variants) {
                    return pos;
                }
            }
        }
    }

    0
}

fn find_variant_index_by_name(name: &str, variants: &[VariantDefinition]) -> Option<usize> {
    variants.iter().position(|v| {
        v.name == name || v.aliases.iter().any(|a| a == name)
    })
}
