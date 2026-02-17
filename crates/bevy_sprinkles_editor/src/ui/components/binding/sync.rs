use bevy::prelude::*;
use bevy::reflect::{PartialReflect, ReflectRef};
use bevy_sprinkles::prelude::*;
use bevy_ui_text_input::TextInputQueue;

use crate::state::EditorState;
use crate::ui::components::inspector::FieldKind;
use crate::ui::widgets::checkbox::CheckboxState;
use crate::ui::widgets::color_picker::{
    ColorPickerState, EditorColorPicker, TriggerSwatchMaterial,
};
use crate::ui::widgets::combobox::ComboBoxConfig;
use crate::ui::widgets::curve_edit::{CurveEditState, EditorCurveEdit};
use crate::ui::widgets::gradient_edit::{EditorGradientEdit, GradientEditState};
use crate::ui::widgets::text_edit::{EditorTextEdit, set_text_input_value};
use crate::ui::widgets::variant_edit::{EditorVariantEdit, VariantDefinition, VariantEditConfig};

use super::{
    BoundTo, FieldBinding, FieldValue, InspectedEmitterTracker, format_f32, get_inspected_data,
    get_variant_index_by_reflection,
};

pub(super) fn bind_text_inputs(
    editor_state: Res<EditorState>,
    assets: Res<Assets<ParticleSystemAsset>>,
    tracker: Res<InspectedEmitterTracker>,
    new_bindings: Query<Entity, Added<FieldBinding>>,
    new_bound: Query<Entity, Added<BoundTo>>,
    bindings: Query<&FieldBinding>,
    mut text_edits: Query<(&BoundTo, &mut TextInputQueue), With<EditorTextEdit>>,
) {
    if !tracker.is_changed() && new_bindings.is_empty() && new_bound.is_empty() {
        return;
    }

    let Some(data) = get_inspected_data(&editor_state, &assets) else {
        return;
    };

    for (bound, mut queue) in &mut text_edits {
        let Ok(binding) = bindings.get(bound.binding) else {
            continue;
        };

        let value = binding.read_value(data);

        if let Some(idx) = bound.component_index {
            if let FieldKind::Vector(suffixes) = &binding.kind {
                if let Some(v) = get_field_value_component(&value, idx) {
                    let text = if suffixes.is_integer() {
                        (v as i32).to_string()
                    } else {
                        format_f32(v)
                    };
                    set_text_input_value(&mut queue, text);
                }
                continue;
            }
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
    new_bound: Query<Entity, Added<BoundTo>>,
    bindings: Query<&FieldBinding>,
    mut checkbox_states: Query<(&FieldBinding, &mut CheckboxState)>,
    mut curve_edits: Query<(&FieldBinding, &mut CurveEditState), With<EditorCurveEdit>>,
    mut gradient_edits: Query<(&FieldBinding, &mut GradientEditState), With<EditorGradientEdit>>,
    mut colocated_comboboxes: Query<(&FieldBinding, &mut ComboBoxConfig)>,
    mut bound_comboboxes: Query<(&BoundTo, &mut ComboBoxConfig), Without<FieldBinding>>,
    mut variant_edits: Query<(&FieldBinding, &mut VariantEditConfig), With<EditorVariantEdit>>,
) {
    if !tracker.is_changed()
        && new_bindings.is_empty()
        && new_variant_edits.is_empty()
        && new_bound.is_empty()
    {
        return;
    }

    let Some(data) = get_inspected_data(&editor_state, &assets) else {
        return;
    };

    for (binding, mut state) in &mut checkbox_states {
        let value = binding.read_value(data);
        if let Some(checked) = value.to_bool() {
            state.checked = checked;
        }
    }

    for (binding, mut state) in &mut curve_edits {
        if binding.kind != FieldKind::Curve {
            continue;
        }
        let Some(reflected) = binding.read_reflected(data) else {
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

    for (binding, mut state) in &mut gradient_edits {
        if binding.kind != FieldKind::Gradient {
            continue;
        }
        let Some(reflected) = binding.read_reflected(data) else {
            continue;
        };
        if let Some(gradient) = reflected.try_downcast_ref::<ParticleGradient>() {
            state.gradient = gradient.clone();
        }
    }

    for (binding, mut config) in &mut colocated_comboboxes {
        if !matches!(binding.kind, FieldKind::ComboBox { .. }) {
            continue;
        }
        let value = binding.read_value(data);
        if let FieldValue::U32(index) = value {
            config.selected = index as usize;
        }
    }

    for (bound, mut config) in &mut bound_comboboxes {
        let Ok(binding) = bindings.get(bound.binding) else {
            continue;
        };
        if !matches!(binding.kind, FieldKind::ComboBox { .. }) {
            continue;
        }
        let value = binding.read_value(data);
        if let FieldValue::U32(index) = value {
            config.selected = index as usize;
        }
    }

    for (binding, mut config) in &mut variant_edits {
        let new_index = if binding.is_variant() {
            let Some(reflected) = binding.read_reflected(data) else {
                continue;
            };
            get_nested_variant_index(reflected, &config.variants)
        } else {
            let Some(idx) = get_variant_index_by_reflection(data, binding.path(), &config.variants)
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
    let Some(data) = get_inspected_data(&editor_state, &assets) else {
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

        let value = binding.read_value(data);
        let Some(color) = value.to_color() else {
            continue;
        };

        state.set_from_rgba(color);
        commands.entity(entity).try_insert(BindingInitialized);
    }
}

#[derive(Component)]
pub(super) struct BindingInitialized;

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

fn get_field_value_component(value: &FieldValue, index: usize) -> Option<f32> {
    match value {
        FieldValue::Vec2(vec) => match index {
            0 => Some(vec.x),
            1 => Some(vec.y),
            _ => None,
        },
        FieldValue::Vec3(vec) => match index {
            0 => Some(vec.x),
            1 => Some(vec.y),
            2 => Some(vec.z),
            _ => None,
        },
        FieldValue::Range(min, max) => match index {
            0 => Some(*min),
            1 => Some(*max),
            _ => None,
        },
        _ => None,
    }
}
