use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use bevy::reflect::{PartialReflect, ReflectRef};
use sprinkles::prelude::*;

use crate::state::{DirtyState, EditorState};
use crate::ui::components::inspector::FieldKind;
use crate::ui::widgets::checkbox::CheckboxCommitEvent;
use crate::ui::widgets::color_picker::ColorPickerCommitEvent;
use crate::ui::widgets::combobox::ComboBoxChangeEvent;
use crate::ui::widgets::curve_edit::CurveEditCommitEvent;
use crate::ui::widgets::gradient_edit::GradientEditCommitEvent;
use crate::ui::widgets::text_edit::TextEditCommitEvent;
use crate::ui::widgets::texture_edit::TextureEditCommitEvent;
use crate::ui::widgets::variant_edit::{VariantComboBox, VariantEditConfig};
use crate::viewport::RespawnEmittersEvent;

use super::{
    BoundTo, FieldBinding, FieldValue, get_inspected_data_mut, mark_dirty_and_restart,
    parse_field_value, read_fixed_seed,
};

#[derive(SystemParam)]
pub(super) struct CommitContext<'w, 's> {
    editor_state: Res<'w, EditorState>,
    assets: ResMut<'w, Assets<ParticleSystemAsset>>,
    dirty_state: ResMut<'w, DirtyState>,
    bindings: Query<'w, 's, &'static FieldBinding>,
    bound_query: Query<'w, 's, &'static BoundTo>,
    emitter_runtimes: Query<'w, 's, &'static mut EmitterRuntime>,
}

impl CommitContext<'_, '_> {
    fn resolve_binding(&self, entity: Entity) -> Option<FieldBinding> {
        if let Ok(binding) = self.bindings.get(entity) {
            return Some(binding.clone());
        }
        let bound = self.bound_query.get(entity).ok()?;
        self.bindings.get(bound.binding).ok().cloned()
    }

    fn commit_reflected(&mut self, entity: Entity, apply_fn: impl FnOnce(&mut dyn PartialReflect)) {
        let Some(binding) = self.resolve_binding(entity) else {
            return;
        };
        let Some(data) = get_inspected_data_mut(&self.editor_state, &mut self.assets) else {
            return;
        };
        let fixed_seed = read_fixed_seed(&*data);
        if binding.write_reflected(data, apply_fn) {
            mark_dirty_and_restart(
                &mut self.dirty_state,
                &mut self.emitter_runtimes,
                fixed_seed,
            );
        }
    }

    fn commit_field_value(&mut self, entity: Entity, value: FieldValue) -> bool {
        let Some(binding) = self.resolve_binding(entity) else {
            return false;
        };
        let should_respawn = requires_respawn_binding(&binding);
        let Some(data) = get_inspected_data_mut(&self.editor_state, &mut self.assets) else {
            return false;
        };
        let fixed_seed = read_fixed_seed(&*data);
        if binding.write_value(data, &value) {
            mark_dirty_and_restart(
                &mut self.dirty_state,
                &mut self.emitter_runtimes,
                fixed_seed,
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

pub(super) fn handle_text_commit(
    trigger: On<TextEditCommitEvent>,
    mut commands: Commands,
    mut ctx: CommitContext,
) {
    let Some(binding) = ctx.resolve_binding(trigger.entity) else {
        return;
    };

    let component_index = ctx
        .bound_query
        .get(trigger.entity)
        .ok()
        .and_then(|b| b.component_index);

    let Some(data) = get_inspected_data_mut(&ctx.editor_state, &mut ctx.assets) else {
        return;
    };

    if let Some(idx) = component_index {
        if let FieldKind::Vector(_) = &binding.kind {
            let Ok(v) = trigger.text.trim().parse::<f32>() else {
                return;
            };

            let current_value = binding.read_value(&*data);
            let new_value = set_field_value_component(&current_value, idx, v);
            let fixed_seed = read_fixed_seed(&*data);

            if binding.write_value(data, &new_value) {
                mark_dirty_and_restart(&mut ctx.dirty_state, &mut ctx.emitter_runtimes, fixed_seed);
                if requires_respawn_binding(&binding) {
                    commands.trigger(RespawnEmittersEvent);
                }
            }
            return;
        }
    }

    let value = parse_field_value(&trigger.text, &binding.kind);
    if matches!(value, FieldValue::None) {
        return;
    }

    let fixed_seed = read_fixed_seed(&*data);
    if binding.write_value(data, &value) {
        mark_dirty_and_restart(&mut ctx.dirty_state, &mut ctx.emitter_runtimes, fixed_seed);
        if requires_respawn_binding(&binding) {
            commands.trigger(RespawnEmittersEvent);
        }
    }
}

pub(super) fn handle_checkbox_commit(
    trigger: On<CheckboxCommitEvent>,
    mut commands: Commands,
    mut ctx: CommitContext,
) {
    let value = FieldValue::Bool(trigger.checked);
    if ctx.commit_field_value(trigger.entity, value) {
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

    let Some(binding) = ctx.resolve_binding(trigger.entity) else {
        return;
    };

    let is_optional = matches!(binding.kind, FieldKind::ComboBox { optional: true, .. });

    let Some(data) = get_inspected_data_mut(&ctx.editor_state, &mut ctx.assets) else {
        return;
    };

    let fixed_seed = read_fixed_seed(&*data);
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
        binding.set_optional_enum(data, inner_variant.as_deref())
    } else {
        let variant_name = trigger
            .value
            .clone()
            .unwrap_or_else(|| trigger.label.split_whitespace().collect());
        binding.set_enum_by_name(data, &variant_name)
    };

    if changed {
        mark_dirty_and_restart(&mut ctx.dirty_state, &mut ctx.emitter_runtimes, fixed_seed);
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

    let Some(data) = get_inspected_data_mut(&ctx.editor_state, &mut ctx.assets) else {
        return;
    };

    let Some(default_value) = variant_def.create_default() else {
        return;
    };

    if !binding.is_variant() {
        if let Some(current) = binding.read_reflected(&*data) {
            if let ReflectRef::Enum(current) = current.reflect_ref() {
                if current.variant_name() == variant_def.name {
                    return;
                }
            }
        }
    }

    let fixed_seed = read_fixed_seed(&*data);
    if binding.write_reflected(data, |field| {
        field.apply(default_value.as_ref());
    }) {
        mark_dirty_and_restart(&mut ctx.dirty_state, &mut ctx.emitter_runtimes, fixed_seed);
    }
}

fn set_field_value_component(value: &FieldValue, index: usize, v: f32) -> FieldValue {
    match value {
        FieldValue::Vec2(vec) => {
            let mut vec = *vec;
            match index {
                0 => vec.x = v,
                1 => vec.y = v,
                _ => {}
            }
            FieldValue::Vec2(vec)
        }
        FieldValue::Vec3(vec) => {
            let mut vec = *vec;
            match index {
                0 => vec.x = v,
                1 => vec.y = v,
                2 => vec.z = v,
                _ => {}
            }
            FieldValue::Vec3(vec)
        }
        FieldValue::Range(min, max) => match index {
            0 => FieldValue::Range(v, *max),
            1 => FieldValue::Range(*min, v),
            _ => value.clone(),
        },
        _ => value.clone(),
    }
}
