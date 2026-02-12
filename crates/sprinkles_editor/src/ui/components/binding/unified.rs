use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use bevy_ui_text_input::TextInputQueue;
use sprinkles::prelude::*;

use crate::state::{DirtyState, EditorState};
use crate::ui::widgets::checkbox::{CheckboxCommitEvent, CheckboxState};
use crate::ui::widgets::color_picker::{
    ColorPickerCommitEvent, ColorPickerState, EditorColorPicker, TriggerSwatchMaterial,
};
use crate::ui::widgets::combobox::{ComboBoxChangeEvent, ComboBoxConfig};
use crate::ui::widgets::curve_edit::{CurveEditCommitEvent, CurveEditState, EditorCurveEdit};
use crate::ui::widgets::gradient_edit::{
    EditorGradientEdit, GradientEditCommitEvent, GradientEditState,
};
use crate::ui::widgets::text_edit::{EditorTextEdit, TextEditCommitEvent, set_text_input_value};
use crate::ui::widgets::texture_edit::TextureEditCommitEvent;
use crate::ui::widgets::variant_edit::{EditorVariantEdit, VariantComboBox, VariantEditConfig};
use crate::ui::widgets::vector_edit::VectorComponentIndex;
use crate::viewport::RespawnEmittersEvent;

use super::{
    FieldBinding, FieldValue, InspectedEmitterTracker, MAX_ANCESTOR_DEPTH, find_ancestor,
    format_f32, get_inspecting_emitter, get_inspecting_emitter_mut,
    get_variant_index_by_reflection, get_vec2_component, get_vec3_component,
    mark_dirty_and_restart, parse_field_value, set_vec2_component, set_vec3_component,
};

use crate::ui::components::inspector::FieldKind;
use crate::ui::widgets::variant_edit::VariantDefinition;
use bevy::reflect::{PartialReflect, ReflectRef};

#[derive(SystemParam)]
pub(super) struct CommitContext<'w, 's> {
    editor_state: Res<'w, EditorState>,
    assets: ResMut<'w, Assets<ParticleSystemAsset>>,
    dirty_state: ResMut<'w, DirtyState>,
    bindings: Query<'w, 's, &'static FieldBinding>,
    parents: Query<'w, 's, &'static ChildOf>,
    emitter_runtimes: Query<'w, 's, &'static mut EmitterRuntime>,
}

impl CommitContext<'_, '_> {
    fn commit_reflected(&mut self, entity: Entity, apply_fn: impl FnOnce(&mut dyn PartialReflect)) {
        let Some((_, binding)) = find_binding_for_entity(entity, &self.bindings, &self.parents)
        else {
            return;
        };
        let Some((_, emitter)) = get_inspecting_emitter_mut(&self.editor_state, &mut self.assets)
        else {
            return;
        };
        if binding.write_reflected(emitter, apply_fn) {
            mark_dirty_and_restart(
                &mut self.dirty_state,
                &mut self.emitter_runtimes,
                emitter.time.fixed_seed,
            );
        }
    }

    fn commit_field_value(&mut self, entity: Entity, value: FieldValue) -> bool {
        let Some((_, binding)) = find_binding_for_entity(entity, &self.bindings, &self.parents)
        else {
            return false;
        };
        let should_respawn = requires_respawn_binding(binding);
        let Some((_, emitter)) = get_inspecting_emitter_mut(&self.editor_state, &mut self.assets)
        else {
            return false;
        };
        if binding.write_value(emitter, &value) {
            mark_dirty_and_restart(
                &mut self.dirty_state,
                &mut self.emitter_runtimes,
                emitter.time.fixed_seed,
            );
            return should_respawn;
        }
        false
    }
}

const RESPAWN_FIELD_PATHS: &[&str] = &[
    "enabled",
    "draw_pass.material.unlit",
    "draw_pass.shadow_caster",
    "emission.particles_amount",
];

fn requires_respawn(path: &str) -> bool {
    RESPAWN_FIELD_PATHS.contains(&path)
}

fn requires_respawn_binding(binding: &FieldBinding) -> bool {
    let path = binding.path();
    if requires_respawn(path) {
        return true;
    }
    if let Some(field_name) = binding.field_name() {
        let full = format!("{}.{}", path, field_name);
        return requires_respawn(&full);
    }
    false
}

// --- sync: data → UI ---

pub(super) fn bind_text_inputs(
    editor_state: Res<EditorState>,
    assets: Res<Assets<ParticleSystemAsset>>,
    tracker: Res<InspectedEmitterTracker>,
    new_bindings: Query<Entity, Added<FieldBinding>>,
    new_text_edits: Query<Entity, Added<EditorTextEdit>>,
    bindings: Query<(Entity, &FieldBinding)>,
    mut text_edits: Query<(Entity, &ChildOf, &mut TextInputQueue), With<EditorTextEdit>>,
    parents: Query<&ChildOf>,
    vector_indices: Query<&VectorComponentIndex>,
) {
    if !tracker.is_changed() && new_bindings.is_empty() && new_text_edits.is_empty() {
        return;
    }

    let Some((_, emitter)) = get_inspecting_emitter(&editor_state, &assets) else {
        return;
    };

    for (_text_entity, child_of, mut queue) in &mut text_edits {
        let Some((_binding_entity, binding)) =
            find_binding_ancestor(child_of.parent(), &bindings, &parents)
        else {
            continue;
        };

        let value = binding.read_value(emitter);

        if let FieldKind::Vector(suffixes) = &binding.kind {
            let idx = find_ancestor(child_of.parent(), &parents, MAX_ANCESTOR_DEPTH, |e| {
                vector_indices.get(e).is_ok()
            })
            .and_then(|e| vector_indices.get(e).ok())
            .map(|v| v.0);

            let Some(idx) = idx else {
                continue;
            };

            let component_value = match &value {
                FieldValue::Vec2(vec) => Some(get_vec2_component(*vec, idx)),
                FieldValue::Vec3(vec) => Some(get_vec3_component(*vec, idx)),
                FieldValue::Range(min, max) => match idx {
                    0 => Some(*min),
                    1 => Some(*max),
                    _ => None,
                },
                _ => None,
            };
            if let Some(v) = component_value {
                let text = if suffixes.is_integer() {
                    (v as i32).to_string()
                } else {
                    format_f32(v)
                };
                set_text_input_value(&mut queue, text);
            }
            continue;
        }

        let text = value.to_display_string(&binding.kind).unwrap_or_default();
        set_text_input_value(&mut queue, text);
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) fn bind_widget_values(
    editor_state: Res<EditorState>,
    assets: Res<Assets<ParticleSystemAsset>>,
    tracker: Res<InspectedEmitterTracker>,
    new_bindings: Query<Entity, Added<FieldBinding>>,
    new_variant_edits: Query<Entity, Added<EditorVariantEdit>>,
    bindings: Query<&FieldBinding>,
    parents: Query<&ChildOf>,
    mut checkbox_states: Query<(Entity, &mut CheckboxState)>,
    mut curve_edits: Query<(Entity, &mut CurveEditState), With<EditorCurveEdit>>,
    mut gradient_edits: Query<(Entity, &mut GradientEditState), With<EditorGradientEdit>>,
    mut combobox_bindings: Query<(&FieldBinding, &mut ComboBoxConfig)>,
    mut variant_edits: Query<(&FieldBinding, &mut VariantEditConfig), With<EditorVariantEdit>>,
) {
    if !tracker.is_changed() && new_bindings.is_empty() && new_variant_edits.is_empty() {
        return;
    }

    let Some((_, emitter)) = get_inspecting_emitter(&editor_state, &assets) else {
        return;
    };

    for (entity, mut state) in &mut checkbox_states {
        let Some((_, binding)) = find_binding_for_entity(entity, &bindings, &parents) else {
            continue;
        };
        let value = binding.read_value(emitter);
        if let Some(checked) = value.to_bool() {
            state.checked = checked;
        }
    }

    for (entity, mut state) in &mut curve_edits {
        let Some((_, binding)) = find_binding_for_entity(entity, &bindings, &parents) else {
            continue;
        };
        if binding.kind != FieldKind::Curve {
            continue;
        }
        let Some(reflected) = binding.read_reflected(emitter) else {
            continue;
        };
        if let Some(ct) = reflected.try_downcast_ref::<CurveTexture>() {
            state.set_curve(ct.clone());
        } else if let Some(opt) = reflected.try_downcast_ref::<Option<CurveTexture>>() {
            if let Some(curve) = opt {
                state.set_curve(curve.clone());
            }
        }
    }

    for (entity, mut state) in &mut gradient_edits {
        let Some((_, binding)) = find_binding_for_entity(entity, &bindings, &parents) else {
            continue;
        };
        if binding.kind != FieldKind::Gradient {
            continue;
        }
        let Some(reflected) = binding.read_reflected(emitter) else {
            continue;
        };
        if let Some(gradient) = reflected.try_downcast_ref::<ParticleGradient>() {
            state.gradient = gradient.clone();
        }
    }

    for (binding, mut config) in &mut combobox_bindings {
        if !matches!(binding.kind, FieldKind::ComboBox { .. }) {
            continue;
        }
        let value = binding.read_value(emitter);
        if let FieldValue::U32(index) = value {
            config.selected = index as usize;
        }
    }

    for (binding, mut config) in &mut variant_edits {
        let new_index = if binding.is_variant() {
            let Some(reflected) = binding.read_reflected(emitter) else {
                continue;
            };
            get_nested_variant_index(reflected, &config.variants)
        } else {
            let Some(idx) =
                get_variant_index_by_reflection(emitter, binding.path(), &config.variants)
            else {
                continue;
            };
            idx
        };
        config.selected_index = new_index;
    }
}

pub(super) fn bind_color_pickers(
    mut commands: Commands,
    editor_state: Res<EditorState>,
    assets: Res<Assets<ParticleSystemAsset>>,
    mut color_pickers: Query<
        (Entity, &mut ColorPickerState, &FieldBinding),
        (With<EditorColorPicker>, Without<BindingInitialized>),
    >,
    trigger_swatches: Query<&TriggerSwatchMaterial>,
) {
    let Some((_, emitter)) = get_inspecting_emitter(&editor_state, &assets) else {
        return;
    };

    for (entity, mut state, binding) in &mut color_pickers {
        if !matches!(binding.kind, FieldKind::Color) {
            continue;
        }

        let trigger_ready = trigger_swatches.iter().any(|swatch| swatch.0 == entity);
        if !trigger_ready {
            continue;
        }

        let value = binding.read_value(emitter);
        let Some(color) = value.to_color() else {
            continue;
        };

        state.set_from_rgba(color);
        commands.entity(entity).try_insert(BindingInitialized);
    }
}

// --- commit: UI → data ---

pub(super) fn handle_text_commit(
    trigger: On<TextEditCommitEvent>,
    mut commands: Commands,
    mut ctx: CommitContext,
    vector_indices: Query<&VectorComponentIndex>,
) {
    let Some((_, binding)) = find_binding_for_entity(trigger.entity, &ctx.bindings, &ctx.parents)
    else {
        return;
    };

    let Some((_, emitter)) = get_inspecting_emitter_mut(&ctx.editor_state, &mut ctx.assets) else {
        return;
    };

    if let FieldKind::Vector(_suffixes) = &binding.kind {
        let idx = find_ancestor(trigger.entity, &ctx.parents, MAX_ANCESTOR_DEPTH, |e| {
            vector_indices.get(e).is_ok()
        })
        .and_then(|e| vector_indices.get(e).ok())
        .map(|v| v.0);

        let Some(idx) = idx else {
            return;
        };

        let Ok(v) = trigger.text.trim().parse::<f32>() else {
            return;
        };

        let current_value = binding.read_value(emitter);
        let new_value = match current_value {
            FieldValue::Vec2(mut vec) => {
                set_vec2_component(&mut vec, idx, v);
                FieldValue::Vec2(vec)
            }
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

        if binding.write_value(emitter, &new_value) {
            mark_dirty_and_restart(
                &mut ctx.dirty_state,
                &mut ctx.emitter_runtimes,
                emitter.time.fixed_seed,
            );
            if requires_respawn_binding(binding) {
                commands.trigger(RespawnEmittersEvent);
            }
        }
        return;
    }

    let value = parse_field_value(&trigger.text, &binding.kind);
    if matches!(value, FieldValue::None) {
        return;
    }

    if binding.write_value(emitter, &value) {
        mark_dirty_and_restart(
            &mut ctx.dirty_state,
            &mut ctx.emitter_runtimes,
            emitter.time.fixed_seed,
        );
        if requires_respawn_binding(binding) {
            commands.trigger(RespawnEmittersEvent);
        }
    }
}

pub(super) fn handle_checkbox_commit(
    trigger: On<CheckboxCommitEvent>,
    mut commands: Commands,
    mut ctx: CommitContext,
) {
    if ctx.commit_field_value(trigger.entity, FieldValue::Bool(trigger.checked)) {
        commands.trigger(RespawnEmittersEvent);
    }
}

pub(super) fn handle_combobox_change(
    trigger: On<ComboBoxChangeEvent>,
    mut ctx: CommitContext,
    variant_comboboxes: Query<(), With<VariantComboBox>>,
) {
    if variant_comboboxes.get(trigger.entity).is_ok() {
        return;
    }

    let Some((_, binding)) = find_binding_for_entity(trigger.entity, &ctx.bindings, &ctx.parents)
    else {
        return;
    };

    let Some((_, emitter)) = get_inspecting_emitter_mut(&ctx.editor_state, &mut ctx.assets) else {
        return;
    };

    let is_optional = matches!(binding.kind, FieldKind::ComboBox { optional: true, .. });

    let changed = if is_optional {
        let inner_variant = if trigger.selected == 0 {
            None
        } else {
            Some(
                trigger
                    .value
                    .as_deref()
                    .unwrap_or(&trigger.label)
                    .split_whitespace()
                    .collect::<String>(),
            )
        };
        binding.set_optional_enum(emitter, inner_variant.as_deref())
    } else {
        let variant_name = trigger
            .value
            .clone()
            .unwrap_or_else(|| trigger.label.split_whitespace().collect());
        binding.set_enum_by_name(emitter, &variant_name)
    };

    if changed {
        mark_dirty_and_restart(
            &mut ctx.dirty_state,
            &mut ctx.emitter_runtimes,
            emitter.time.fixed_seed,
        );
    }
}

pub(super) fn handle_curve_commit(trigger: On<CurveEditCommitEvent>, mut ctx: CommitContext) {
    let curve = trigger.curve.clone();
    ctx.commit_reflected(trigger.entity, |target| {
        if let Some(ct) = target.try_downcast_mut::<CurveTexture>() {
            *ct = curve.clone();
        } else if let Some(opt) = target.try_downcast_mut::<Option<CurveTexture>>() {
            *opt = Some(curve);
        }
    });
}

pub(super) fn handle_gradient_commit(trigger: On<GradientEditCommitEvent>, mut ctx: CommitContext) {
    let gradient = trigger.gradient.clone();
    ctx.commit_reflected(trigger.entity, |target| {
        target.apply(&gradient);
    });
}

pub(super) fn handle_color_commit(trigger: On<ColorPickerCommitEvent>, mut ctx: CommitContext) {
    ctx.commit_field_value(trigger.entity, FieldValue::Color(trigger.color));
}

pub(super) fn handle_texture_commit(trigger: On<TextureEditCommitEvent>, mut ctx: CommitContext) {
    ctx.commit_reflected(trigger.entity, |target| {
        target.apply(&trigger.value);
    });
}

// --- variant edit combobox (switching which enum variant is selected) ---

pub(super) fn handle_variant_change(
    trigger: On<ComboBoxChangeEvent>,
    mut ctx: CommitContext,
    variant_comboboxes: Query<&VariantComboBox>,
    variant_edit_configs: Query<(&VariantEditConfig, &FieldBinding)>,
) {
    let Ok(variant_combobox) = variant_comboboxes.get(trigger.entity) else {
        return;
    };

    let variant_edit_entity = variant_combobox.0;
    let Ok((config, binding)) = variant_edit_configs.get(variant_edit_entity) else {
        return;
    };

    let Some(variant_def) = config.variants.get(trigger.selected) else {
        return;
    };

    let Some((_, emitter)) = get_inspecting_emitter_mut(&ctx.editor_state, &mut ctx.assets) else {
        return;
    };

    let Some(default_value) = variant_def.create_default() else {
        return;
    };

    if !binding.is_variant() {
        if let Some(current) = binding.read_reflected(emitter) {
            if let ReflectRef::Enum(current) = current.reflect_ref() {
                if current.variant_name() == variant_def.name {
                    return;
                }
            }
        }
    }

    if binding.write_reflected(emitter, |field| {
        field.apply(default_value.as_ref());
    }) {
        mark_dirty_and_restart(
            &mut ctx.dirty_state,
            &mut ctx.emitter_runtimes,
            emitter.time.fixed_seed,
        );
    }
}

// --- helpers ---

#[derive(Component)]
pub(super) struct BindingInitialized;

fn find_binding_ancestor<'a>(
    entity: Entity,
    bindings: &'a Query<(Entity, &FieldBinding)>,
    parents: &Query<&ChildOf>,
) -> Option<(Entity, &'a FieldBinding)> {
    find_ancestor(entity, parents, MAX_ANCESTOR_DEPTH, |e| {
        bindings.get(e).is_ok()
    })
    .and_then(|e| bindings.get(e).ok())
}

fn find_binding_for_entity<'a>(
    entity: Entity,
    bindings: &'a Query<&FieldBinding>,
    parents: &Query<&ChildOf>,
) -> Option<(Entity, &'a FieldBinding)> {
    if let Ok(binding) = bindings.get(entity) {
        return Some((entity, binding));
    }
    find_ancestor(entity, parents, MAX_ANCESTOR_DEPTH, |e| {
        bindings.get(e).is_ok()
    })
    .and_then(|e| bindings.get(e).ok().map(|b| (e, b)))
}

fn get_nested_variant_index(value: &dyn PartialReflect, variants: &[VariantDefinition]) -> usize {
    let ReflectRef::Enum(enum_ref) = value.reflect_ref() else {
        return 0;
    };

    let variant_name = enum_ref.variant_name();

    if let Some(pos) = find_variant_index_by_name(variant_name, variants) {
        return pos;
    }

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
    variants
        .iter()
        .position(|v| v.name == name || v.aliases.iter().any(|a| a == name))
}
