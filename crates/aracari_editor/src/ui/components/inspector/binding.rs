use std::collections::HashMap;

use aracari::prelude::*;
use bevy::ecs::system::ParamSet;
use bevy::input_focus::InputFocus;
use bevy::prelude::*;
use bevy::reflect::{DynamicEnum, DynamicVariant, PartialReflect, ReflectMut, ReflectRef};
use bevy_ui_text_input::{
    TextInputBuffer, TextInputQueue,
    actions::{TextInputAction, TextInputEdit},
};

use crate::state::{DirtyState, EditorState, Inspectable};
use crate::ui::widgets::checkbox::CheckboxState;
use crate::ui::widgets::color_picker::{
    ColorPickerCommitEvent, ColorPickerState, EditorColorPicker, TriggerSwatchMaterial,
};
use crate::ui::widgets::combobox::ComboBoxChangeEvent;
use crate::ui::widgets::text_edit::EditorTextEdit;
use crate::ui::widgets::variant_edit::{
    EditorVariantEdit, VariantComboBox, VariantDefinition, VariantEditConfig, VariantFieldBinding,
    VariantFieldKind,
};
use crate::ui::widgets::vector_edit::EditorVectorEdit;

const MAX_ANCESTOR_DEPTH: usize = 10;

fn get_inspecting_emitter<'a>(
    editor_state: &EditorState,
    assets: &'a Assets<ParticleSystemAsset>,
) -> Option<(u8, &'a EmitterData)> {
    let inspecting = match &editor_state.inspecting {
        Some(i) if i.kind == Inspectable::Emitter => i,
        _ => return None,
    };
    let handle = editor_state.current_project.as_ref()?;
    let asset = assets.get(handle)?;
    let emitter = asset.emitters.get(inspecting.index as usize)?;
    Some((inspecting.index, emitter))
}

fn get_inspecting_emitter_mut<'a>(
    editor_state: &EditorState,
    assets: &'a mut Assets<ParticleSystemAsset>,
) -> Option<(u8, &'a mut EmitterData)> {
    let inspecting = match &editor_state.inspecting {
        Some(i) if i.kind == Inspectable::Emitter => i,
        _ => return None,
    };
    let handle = editor_state.current_project.as_ref()?;
    let asset = assets.get_mut(handle)?;
    let emitter = asset.emitters.get_mut(inspecting.index as usize)?;
    Some((inspecting.index, emitter))
}

fn find_ancestor<F>(
    mut entity: Entity,
    parents: &Query<&ChildOf>,
    max_depth: usize,
    mut predicate: F,
) -> Option<Entity>
where
    F: FnMut(Entity) -> bool,
{
    for _ in 0..max_depth {
        if predicate(entity) {
            return Some(entity);
        }
        entity = parents.get(entity).ok()?.parent();
    }
    None
}

pub fn plugin(app: &mut App) {
    app.init_resource::<BoundEmitter>()
        .add_observer(handle_variant_change)
        .add_observer(handle_combobox_change)
        .add_observer(handle_variant_color_commit)
        .add_systems(
            Update,
            (
                bind_values_to_inputs,
                bind_variant_edits,
                bind_variant_field_values,
                bind_variant_color_pickers,
                sync_input_on_blur,
                sync_checkbox_changes_to_asset,
                sync_variant_field_on_blur,
                sync_variant_checkbox_changes,
            ),
        );
}

#[derive(Resource, Default)]
struct BoundEmitter(Option<u8>);

#[derive(Debug, Clone, PartialEq, Default)]
pub enum FieldKind {
    #[default]
    F32,
    F32Percent,
    U32,
    U32OrEmpty,
    OptionalU32,
    Bool,
    ComboBox { options: Vec<String> },
    Color,
}

#[derive(Component, Clone)]
pub struct Field {
    pub path: String,
    pub kind: FieldKind,
}

#[derive(Component)]
struct FieldBound;

#[derive(Component)]
struct CheckboxBound;

#[derive(Component)]
struct VariantFieldBound;

#[derive(Component)]
struct ColorPickerBound;

impl Field {
    pub fn new(path: impl Into<String>) -> Self {
        Self {
            path: path.into(),
            kind: FieldKind::default(),
        }
    }

    pub fn with_kind(mut self, kind: FieldKind) -> Self {
        self.kind = kind;
        self
    }
}

#[derive(Debug, Clone)]
struct ReflectPath(String);

impl ReflectPath {
    fn new(path: &str) -> Self {
        Self(format!(".{}", path))
    }

    fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone)]
enum FieldValue {
    None,
    F32(f32),
    U32(u32),
    OptionalU32(Option<u32>),
    Bool(bool),
    Vec3(Vec3),
    Color([f32; 4]),
}

impl FieldValue {
    fn to_display_string(&self, kind: &FieldKind) -> Option<String> {
        match (self, kind) {
            (FieldValue::F32(v), FieldKind::F32Percent) => {
                let display = (v * 100.0 * 100.0).round() / 100.0;
                Some(format_f32(display))
            }
            (FieldValue::F32(v), _) => Some(format_f32(*v)),
            (FieldValue::U32(0), FieldKind::U32OrEmpty) => None,
            (FieldValue::U32(v), _) => Some(v.to_string()),
            (FieldValue::OptionalU32(None), _) => None,
            (FieldValue::OptionalU32(Some(0)), FieldKind::OptionalU32) => None,
            (FieldValue::OptionalU32(Some(v)), _) => Some(v.to_string()),
            _ => None,
        }
    }

    fn to_vec3(&self) -> Option<Vec3> {
        match self {
            FieldValue::Vec3(v) => Some(*v),
            _ => None,
        }
    }

    fn to_bool(&self) -> Option<bool> {
        match self {
            FieldValue::Bool(v) => Some(*v),
            _ => None,
        }
    }

    fn to_color(&self) -> Option<[f32; 4]> {
        match self {
            FieldValue::Color(c) => Some(*c),
            _ => None,
        }
    }
}

fn format_f32(v: f32) -> String {
    let mut text = v.to_string();
    if !text.contains('.') {
        text.push_str(".0");
    }
    text
}

fn set_vec3_component(vec: &mut Vec3, index: usize, value: f32) {
    match index {
        0 => vec.x = value,
        1 => vec.y = value,
        2 => vec.z = value,
        _ => {}
    }
}

fn get_vec3_component(vec: Vec3, index: usize) -> f32 {
    match index {
        0 => vec.x,
        1 => vec.y,
        2 => vec.z,
        _ => 0.0,
    }
}

fn get_field_value_by_reflection(
    emitter: &EmitterData,
    path: &str,
    kind: &FieldKind,
) -> FieldValue {
    let reflect_path = ReflectPath::new(path);
    let Ok(value) = emitter.reflect_path(reflect_path.as_str()) else {
        return FieldValue::None;
    };
    reflect_to_field_value(value, kind)
}

fn set_field_value_by_reflection(
    emitter: &mut EmitterData,
    path: &str,
    value: &FieldValue,
) -> bool {
    let reflect_path = ReflectPath::new(path);
    let Ok(target) = emitter.reflect_path_mut(reflect_path.as_str()) else {
        return false;
    };
    apply_field_value_to_reflect(target, value)
}

fn parse_field_value(text: &str, kind: &FieldKind) -> FieldValue {
    let text = text.trim();

    match kind {
        FieldKind::F32 => text
            .trim_end_matches('s')
            .trim()
            .parse::<f32>()
            .map(FieldValue::F32)
            .unwrap_or(FieldValue::None),
        FieldKind::F32Percent => text
            .trim_end_matches('%')
            .trim()
            .parse::<f32>()
            .map(|v| FieldValue::F32(v / 100.0))
            .unwrap_or(FieldValue::None),
        FieldKind::U32 | FieldKind::U32OrEmpty => {
            if text.is_empty() && matches!(kind, FieldKind::U32OrEmpty) {
                FieldValue::U32(0)
            } else {
                text.parse::<u32>()
                    .map(FieldValue::U32)
                    .unwrap_or(FieldValue::None)
            }
        }
        FieldKind::OptionalU32 => {
            if text.is_empty() {
                FieldValue::OptionalU32(None)
            } else {
                text.parse::<u32>()
                    .ok()
                    .map(|v| {
                        if v == 0 {
                            FieldValue::OptionalU32(None)
                        } else {
                            FieldValue::OptionalU32(Some(v))
                        }
                    })
                    .unwrap_or(FieldValue::None)
            }
        }
        FieldKind::Bool | FieldKind::ComboBox { .. } | FieldKind::Color => FieldValue::None,
    }
}

fn reflect_to_field_value(value: &dyn PartialReflect, kind: &FieldKind) -> FieldValue {
    if let Some(v) = value.try_downcast_ref::<f32>() {
        return FieldValue::F32(*v);
    }
    if let Some(v) = value.try_downcast_ref::<u32>() {
        return FieldValue::U32(*v);
    }
    if let Some(v) = value.try_downcast_ref::<bool>() {
        return FieldValue::Bool(*v);
    }
    if let Some(v) = value.try_downcast_ref::<Vec3>() {
        return FieldValue::Vec3(*v);
    }
    if let Some(v) = value.try_downcast_ref::<Option<u32>>() {
        return FieldValue::OptionalU32(*v);
    }
    if let Some(v) = value.try_downcast_ref::<[f32; 4]>() {
        return FieldValue::Color(*v);
    }
    if let ReflectRef::Enum(enum_ref) = value.reflect_ref() {
        return FieldValue::U32(enum_ref.variant_index() as u32);
    }
    let _ = kind;
    FieldValue::None
}

fn apply_field_value_to_reflect(target: &mut dyn PartialReflect, value: &FieldValue) -> bool {
    match value {
        FieldValue::F32(v) => {
            if let Some(field) = target.try_downcast_mut::<f32>() {
                if (*field - v).abs() > f32::EPSILON {
                    *field = *v;
                    return true;
                }
            }
        }
        FieldValue::U32(v) => {
            if let Some(field) = target.try_downcast_mut::<u32>() {
                if *field != *v {
                    *field = *v;
                    return true;
                }
            }
        }
        FieldValue::OptionalU32(v) => {
            if let Some(field) = target.try_downcast_mut::<Option<u32>>() {
                if *field != *v {
                    *field = *v;
                    return true;
                }
            }
        }
        FieldValue::Bool(v) => {
            if let Some(field) = target.try_downcast_mut::<bool>() {
                if *field != *v {
                    *field = *v;
                    return true;
                }
            }
        }
        FieldValue::Vec3(v) => {
            if let Some(field) = target.try_downcast_mut::<Vec3>() {
                if (*field - *v).length() > f32::EPSILON {
                    *field = *v;
                    return true;
                }
            }
        }
        FieldValue::Color(c) => {
            if let Some(field) = target.try_downcast_mut::<[f32; 4]>() {
                if *field != *c {
                    *field = *c;
                    return true;
                }
            }
        }
        FieldValue::None => {}
    }
    false
}

fn get_variant_index_by_reflection(
    emitter: &EmitterData,
    path: &str,
    variants: &[VariantDefinition],
) -> Option<usize> {
    let reflect_path = ReflectPath::new(path);
    let value = emitter.reflect_path(reflect_path.as_str()).ok()?;

    let ReflectRef::Enum(enum_ref) = value.reflect_ref() else {
        return None;
    };

    let variant_name = enum_ref.variant_name();
    variants.iter().position(|v| v.name == variant_name)
}

fn get_variant_field_value_by_reflection(
    emitter: &EmitterData,
    path: &str,
    field_name: &str,
    kind: &VariantFieldKind,
) -> Option<FieldValue> {
    let reflect_path = ReflectPath::new(path);
    let value = emitter.reflect_path(reflect_path.as_str()).ok()?;

    let ReflectRef::Enum(enum_ref) = value.reflect_ref() else {
        return None;
    };

    let field_kind = FieldKind::from(kind);

    if let Some(field) = enum_ref.field(field_name) {
        return Some(reflect_to_field_value(field, &field_kind));
    }

    if let Some(inner) = enum_ref.field_at(0) {
        if let ReflectRef::Struct(struct_ref) = inner.reflect_ref() {
            if let Some(field) = struct_ref.field(field_name) {
                return Some(reflect_to_field_value(field, &field_kind));
            }
        }
    }

    None
}

fn set_variant_field_value_by_reflection(
    emitter: &mut EmitterData,
    path: &str,
    field_name: &str,
    value: &FieldValue,
) -> bool {
    let reflect_path = ReflectPath::new(path);
    let Ok(target) = emitter.reflect_path_mut(reflect_path.as_str()) else {
        return false;
    };

    let ReflectMut::Enum(enum_mut) = target.reflect_mut() else {
        return false;
    };

    if let Some(field) = enum_mut.field_mut(field_name) {
        return apply_field_value_to_reflect(field, value);
    }

    if let Some(inner) = enum_mut.field_at_mut(0) {
        if let ReflectMut::Struct(struct_mut) = inner.reflect_mut() {
            if let Some(field) = struct_mut.field_mut(field_name) {
                return apply_field_value_to_reflect(field, value);
            }
        }
    }

    false
}

fn create_variant_from_definition(
    emitter: &mut EmitterData,
    path: &str,
    variant_def: &VariantDefinition,
) -> bool {
    let Some(default_value) = variant_def.create_default() else {
        return false;
    };

    let reflect_path = ReflectPath::new(path);
    let Ok(target) = emitter.reflect_path_mut(reflect_path.as_str()) else {
        return false;
    };

    if let ReflectRef::Enum(current) = target.reflect_ref() {
        if current.variant_name() == variant_def.name {
            return false;
        }
    }

    target.apply(default_value.as_ref());
    true
}

impl From<&VariantFieldKind> for FieldKind {
    fn from(kind: &VariantFieldKind) -> Self {
        match kind {
            VariantFieldKind::F32 => FieldKind::F32,
            VariantFieldKind::U32 => FieldKind::U32,
            VariantFieldKind::Bool => FieldKind::Bool,
            VariantFieldKind::Vec3(_) => FieldKind::F32,
            VariantFieldKind::ComboBox { options } => FieldKind::ComboBox {
                options: options.clone(),
            },
            VariantFieldKind::Color => FieldKind::Color,
        }
    }
}

fn bind_variant_edits(
    editor_state: Res<EditorState>,
    assets: Res<Assets<ParticleSystemAsset>>,
    mut variant_edits: Query<(&Field, &mut VariantEditConfig), With<EditorVariantEdit>>,
    new_variant_edits: Query<Entity, Added<EditorVariantEdit>>,
    mut last_bound_emitter: Local<Option<u8>>,
) {
    let Some((index, emitter)) = get_inspecting_emitter(&editor_state, &assets) else {
        *last_bound_emitter = None;
        return;
    };

    let emitter_changed = *last_bound_emitter != Some(index);
    let has_new_variant_edits = !new_variant_edits.is_empty();

    if !emitter_changed && !has_new_variant_edits {
        return;
    }

    *last_bound_emitter = Some(index);

    for (field, mut config) in &mut variant_edits {
        let Some(new_index) =
            get_variant_index_by_reflection(emitter, &field.path, &config.variants)
        else {
            continue;
        };

        config.selected_index = new_index;
    }
}

fn bind_variant_field_values(
    mut commands: Commands,
    editor_state: Res<EditorState>,
    assets: Res<Assets<ParticleSystemAsset>>,
    variant_field_bindings: Query<(Entity, &VariantFieldBinding), Without<VariantFieldBound>>,
    variant_edit_configs: Query<&VariantEditConfig>,
    mut text_edits: Query<
        (Entity, &ChildOf, &mut TextInputQueue),
        (With<EditorTextEdit>, Without<VariantFieldBound>),
    >,
    mut checkbox_states: Query<&mut CheckboxState>,
    parents: Query<&ChildOf>,
    vector_edit_children: Query<&Children, With<EditorVectorEdit>>,
) {
    let Some((_, emitter)) = get_inspecting_emitter(&editor_state, &assets) else {
        return;
    };

    for (binding_entity, binding) in &variant_field_bindings {
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

        let field_kind = FieldKind::from(&binding.field_kind);
        let mut bound = false;

        if let Some(text) = value.to_display_string(&field_kind) {
            for (text_edit_entity, text_edit_parent, mut queue) in &mut text_edits {
                if find_ancestor_entity(text_edit_parent.parent(), binding_entity, &parents) {
                    queue.add(TextInputAction::Edit(TextInputEdit::SelectAll));
                    queue.add(TextInputAction::Edit(TextInputEdit::Paste(text.clone())));
                    commands.entity(text_edit_entity).insert(VariantFieldBound);
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

        if let Some(vec) = value.to_vec3() {
            if let Ok(vec_children) = vector_edit_children.get(binding_entity) {
                let mut component_index = 0;

                for vec_child in vec_children.iter().take(3) {
                    for (text_edit_entity, text_edit_parent, mut queue) in &mut text_edits {
                        if find_ancestor_entity(text_edit_parent.parent(), vec_child, &parents) {
                            let text = format_f32(get_vec3_component(vec, component_index));
                            queue.add(TextInputAction::Edit(TextInputEdit::SelectAll));
                            queue.add(TextInputAction::Edit(TextInputEdit::Paste(text)));
                            commands.entity(text_edit_entity).insert(VariantFieldBound);
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

        if bound {
            commands.entity(binding_entity).insert(VariantFieldBound);
        }
    }
}

fn find_ancestor_entity(entity: Entity, target: Entity, parents: &Query<&ChildOf>) -> bool {
    find_ancestor(entity, parents, MAX_ANCESTOR_DEPTH, |e| e == target).is_some()
}

fn handle_variant_change(
    trigger: On<ComboBoxChangeEvent>,
    editor_state: Res<EditorState>,
    mut assets: ResMut<Assets<ParticleSystemAsset>>,
    mut dirty_state: ResMut<DirtyState>,
    variant_comboboxes: Query<&VariantComboBox>,
    variant_edit_configs: Query<&VariantEditConfig>,
    mut emitter_runtimes: Query<&mut EmitterRuntime>,
) {
    let combobox_entity = trigger.entity;

    let Ok(variant_combobox) = variant_comboboxes.get(combobox_entity) else {
        return;
    };

    let Ok(config) = variant_edit_configs.get(variant_combobox.0) else {
        return;
    };

    let Some(variant_def) = config.variants.get(trigger.selected) else {
        return;
    };

    let Some((_, emitter)) = get_inspecting_emitter_mut(&editor_state, &mut assets) else {
        return;
    };

    if create_variant_from_definition(emitter, &config.path, variant_def) {
        mark_dirty_and_restart(&mut dirty_state, &mut emitter_runtimes, emitter.time.fixed_seed);
    }
}

fn sync_variant_field_on_blur(
    input_focus: Res<InputFocus>,
    mut last_focus: Local<Option<Entity>>,
    editor_state: Res<EditorState>,
    mut assets: ResMut<Assets<ParticleSystemAsset>>,
    mut dirty_state: ResMut<DirtyState>,
    text_inputs: Query<(&TextInputBuffer, &ChildOf), With<VariantFieldBound>>,
    variant_field_bindings: Query<(&VariantFieldBinding, &ChildOf)>,
    variant_edit_configs: Query<&VariantEditConfig>,
    parents: Query<&ChildOf>,
    mut emitter_runtimes: Query<&mut EmitterRuntime>,
    vector_edits: Query<&Children, With<EditorVectorEdit>>,
) {
    let current_focus = input_focus.0;
    let previous_focus = *last_focus;
    *last_focus = current_focus;

    let Some(blurred_entity) = previous_focus else {
        return;
    };
    if current_focus == Some(blurred_entity) {
        return;
    }

    let Ok((buffer, text_input_parent)) = text_inputs.get(blurred_entity) else {
        return;
    };

    let binding = find_variant_field_binding(
        text_input_parent.parent(),
        &variant_field_bindings,
        &parents,
    );
    let Some((binding, binding_entity)) = binding else {
        return;
    };

    let Ok(config) = variant_edit_configs.get(binding.variant_edit) else {
        return;
    };

    let Some((_, emitter)) = get_inspecting_emitter_mut(&editor_state, &mut assets) else {
        return;
    };

    let text = buffer.get_text();
    let value = match &binding.field_kind {
        VariantFieldKind::F32 => text
            .trim()
            .parse::<f32>()
            .map(FieldValue::F32)
            .unwrap_or(FieldValue::None),
        VariantFieldKind::U32 => text
            .trim()
            .parse::<u32>()
            .map(FieldValue::U32)
            .unwrap_or(FieldValue::None),
        VariantFieldKind::Bool => FieldValue::None,
        VariantFieldKind::Vec3(suffixes) => {
            let Ok(vec_children) = vector_edits.get(binding_entity) else {
                return;
            };
            let kind = VariantFieldKind::Vec3(*suffixes);
            let Some(FieldValue::Vec3(mut vec)) = get_variant_field_value_by_reflection(
                emitter,
                &config.path,
                &binding.field_name,
                &kind,
            ) else {
                return;
            };

            for (idx, vec_child) in vec_children.iter().enumerate().take(3) {
                if find_ancestor_entity(text_input_parent.parent(), vec_child, &parents) {
                    if let Ok(v) = text.trim().parse::<f32>() {
                        set_vec3_component(&mut vec, idx, v);
                    }
                    break;
                }
            }
            FieldValue::Vec3(vec)
        }
        VariantFieldKind::ComboBox { .. } | VariantFieldKind::Color => FieldValue::None,
    };

    if matches!(value, FieldValue::None) {
        return;
    }

    let changed =
        set_variant_field_value_by_reflection(emitter, &config.path, &binding.field_name, &value);

    if changed {
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

fn bind_values_to_inputs(
    mut commands: Commands,
    editor_state: Res<EditorState>,
    mut bound_emitter: ResMut<BoundEmitter>,
    assets: Res<Assets<ParticleSystemAsset>>,
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
) {
    let Some((index, emitter)) = get_inspecting_emitter(&editor_state, &assets) else {
        bound_emitter.0 = None;
        return;
    };

    let emitter_changed = bound_emitter.0 != Some(index);
    let has_new_checkboxes = !checkbox_set.p0().is_empty();
    let has_new_fields = !new_fields.is_empty() || !new_text_edits.is_empty();
    let should_rebind = emitter_changed || has_new_fields || has_new_checkboxes;

    if !should_rebind {
        return;
    }

    bound_emitter.0 = Some(index);

    for (entity, child_of, mut queue) in &mut text_edits {
        let Some(field) = find_ancestor_field(child_of.parent(), &fields, &parents) else {
            continue;
        };

        if is_descendant_of_variant_edit(child_of.parent(), &variant_edit_query, &parents) {
            continue;
        }

        let value = get_field_value_by_reflection(emitter, &field.path, &field.kind);
        let text = value.to_display_string(&field.kind).unwrap_or_default();

        queue.add(TextInputAction::Edit(TextInputEdit::SelectAll));
        queue.add(TextInputAction::Edit(TextInputEdit::Paste(text)));
        commands.entity(entity).insert(FieldBound);
    }

    for (entity, mut state) in &mut checkbox_set.p1() {
        if let Ok(child_of) = parents.get(entity) {
            if is_descendant_of_variant_edit(child_of.parent(), &variant_edit_query, &parents) {
                continue;
            }
        }

        let Some(field) = find_field_for_entity(entity, &fields, &parents) else {
            continue;
        };

        let value = get_field_value_by_reflection(emitter, &field.path, &field.kind);
        if let Some(checked) = value.to_bool() {
            state.checked = checked;
        }
        commands.entity(entity).insert(CheckboxBound);
    }
}

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

fn find_ancestor_field<'a>(
    entity: Entity,
    fields: &'a Query<&Field>,
    parents: &Query<&ChildOf>,
) -> Option<&'a Field> {
    find_ancestor(entity, parents, MAX_ANCESTOR_DEPTH, |e| fields.get(e).is_ok())
        .and_then(|e| fields.get(e).ok())
}

fn find_field_for_entity<'a>(
    entity: Entity,
    fields: &'a Query<&Field>,
    parents: &Query<&ChildOf>,
) -> Option<&'a Field> {
    if let Ok(field) = fields.get(entity) {
        return Some(field);
    }
    if let Ok(child_of) = parents.get(entity) {
        return find_ancestor_field(child_of.parent(), fields, parents);
    }
    None
}

fn mark_dirty_and_restart(
    dirty_state: &mut DirtyState,
    emitter_runtimes: &mut Query<&mut EmitterRuntime>,
    fixed_seed: Option<u32>,
) {
    dirty_state.has_unsaved_changes = true;
    for mut runtime in emitter_runtimes.iter_mut() {
        runtime.restart(fixed_seed);
    }
}

fn sync_input_on_blur(
    input_focus: Res<InputFocus>,
    mut last_focus: Local<Option<Entity>>,
    editor_state: Res<EditorState>,
    mut assets: ResMut<Assets<ParticleSystemAsset>>,
    mut dirty_state: ResMut<DirtyState>,
    text_inputs: Query<(&TextInputBuffer, &ChildOf), With<FieldBound>>,
    fields: Query<&Field>,
    parents: Query<&ChildOf>,
    mut emitter_runtimes: Query<&mut EmitterRuntime>,
) {
    let current_focus = input_focus.0;
    let previous_focus = *last_focus;
    *last_focus = current_focus;

    let Some(blurred_entity) = previous_focus else {
        return;
    };
    if current_focus == Some(blurred_entity) {
        return;
    }

    let Ok((buffer, child_of)) = text_inputs.get(blurred_entity) else {
        return;
    };

    let Some(field) = find_ancestor_field(child_of.parent(), &fields, &parents) else {
        return;
    };

    let Some((_, emitter)) = get_inspecting_emitter_mut(&editor_state, &mut assets) else {
        return;
    };

    let text = buffer.get_text();
    let value = parse_field_value(&text, &field.kind);

    if matches!(value, FieldValue::None) {
        return;
    }

    if set_field_value_by_reflection(emitter, &field.path, &value) {
        mark_dirty_and_restart(&mut dirty_state, &mut emitter_runtimes, emitter.time.fixed_seed);
    }
}

fn sync_checkbox_changes_to_asset(
    editor_state: Res<EditorState>,
    mut assets: ResMut<Assets<ParticleSystemAsset>>,
    mut dirty_state: ResMut<DirtyState>,
    changed_checkboxes: Query<
        (Entity, &CheckboxState),
        (Changed<CheckboxState>, With<CheckboxBound>),
    >,
    fields: Query<&Field>,
    parents: Query<&ChildOf>,
    mut emitter_runtimes: Query<&mut EmitterRuntime>,
    mut last_checkbox_states: Local<HashMap<Entity, bool>>,
) {
    let Some((_, emitter)) = get_inspecting_emitter_mut(&editor_state, &mut assets) else {
        return;
    };

    for (entity, state) in &changed_checkboxes {
        let last_state = last_checkbox_states.get(&entity).copied();
        last_checkbox_states.insert(entity, state.checked);
        if last_state.is_none() {
            continue;
        }

        let Some(field) = find_field_for_entity(entity, &fields, &parents) else {
            continue;
        };

        let value = FieldValue::Bool(state.checked);
        if set_field_value_by_reflection(emitter, &field.path, &value) {
            mark_dirty_and_restart(&mut dirty_state, &mut emitter_runtimes, emitter.time.fixed_seed);
        }
    }
}

fn sync_variant_checkbox_changes(
    editor_state: Res<EditorState>,
    mut assets: ResMut<Assets<ParticleSystemAsset>>,
    mut dirty_state: ResMut<DirtyState>,
    changed_checkboxes: Query<
        (Entity, &CheckboxState, &VariantFieldBinding),
        (Changed<CheckboxState>, With<VariantFieldBound>),
    >,
    variant_edit_configs: Query<&VariantEditConfig>,
    mut emitter_runtimes: Query<&mut EmitterRuntime>,
    mut last_states: Local<HashMap<Entity, bool>>,
) {
    let Some((_, emitter)) = get_inspecting_emitter_mut(&editor_state, &mut assets) else {
        return;
    };

    for (entity, state, binding) in &changed_checkboxes {
        let last_state = last_states.get(&entity).copied();
        last_states.insert(entity, state.checked);
        if last_state.is_none() {
            continue;
        }

        let Ok(config) = variant_edit_configs.get(binding.variant_edit) else {
            continue;
        };

        let value = FieldValue::Bool(state.checked);

        let changed =
        set_variant_field_value_by_reflection(emitter, &config.path, &binding.field_name, &value);

        if changed {
            mark_dirty_and_restart(&mut dirty_state, &mut emitter_runtimes, emitter.time.fixed_seed);
        }
    }
}

fn handle_combobox_change(
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

    if let Some(binding) = find_variant_field_binding_from_entity(
        trigger.entity,
        &variant_field_bindings,
        &parents,
    ) {
        if let Ok(config) = variant_edit_configs.get(binding.variant_edit) {
            changed = set_variant_field_enum_by_name(
                emitter,
                &config.path,
                &binding.field_name,
                &variant_name,
            );
        }
    } else if let Some(field) = find_ancestor_field(trigger.entity, &fields, &parents) {
        changed = set_field_enum_by_name(emitter, &field.path, &variant_name);
    }

    if changed {
        mark_dirty_and_restart(&mut dirty_state, &mut emitter_runtimes, emitter.time.fixed_seed);
    }
}

fn find_variant_field_binding_from_entity<'a>(
    entity: Entity,
    bindings: &'a Query<&VariantFieldBinding>,
    parents: &Query<&ChildOf>,
) -> Option<&'a VariantFieldBinding> {
    find_ancestor(entity, parents, MAX_ANCESTOR_DEPTH, |e| bindings.get(e).is_ok())
        .and_then(|e| bindings.get(e).ok())
}

fn label_to_variant_name(label: &str) -> String {
    label.split_whitespace().collect()
}

fn set_variant_field_enum_by_name(
    emitter: &mut EmitterData,
    path: &str,
    field_name: &str,
    variant_name: &str,
) -> bool {
    let reflect_path = ReflectPath::new(path);
    let Ok(target) = emitter.reflect_path_mut(reflect_path.as_str()) else {
        return false;
    };

    let ReflectMut::Enum(enum_mut) = target.reflect_mut() else {
        return false;
    };

    if let Some(field) = enum_mut.field_mut(field_name) {
        return set_enum_variant_by_name(field, variant_name);
    }

    if let Some(inner) = enum_mut.field_at_mut(0) {
        if let ReflectMut::Struct(struct_mut) = inner.reflect_mut() {
            if let Some(field) = struct_mut.field_mut(field_name) {
                return set_enum_variant_by_name(field, variant_name);
            }
        }
    }

    false
}

fn set_field_enum_by_name(emitter: &mut EmitterData, path: &str, variant_name: &str) -> bool {
    let reflect_path = ReflectPath::new(path);
    let Ok(target) = emitter.reflect_path_mut(reflect_path.as_str()) else {
        return false;
    };
    set_enum_variant_by_name(target, variant_name)
}

fn set_enum_variant_by_name(target: &mut dyn PartialReflect, variant_name: &str) -> bool {
    let ReflectMut::Enum(enum_mut) = target.reflect_mut() else {
        return false;
    };

    if enum_mut.variant_name() == variant_name {
        return false;
    }

    let dynamic_enum = DynamicEnum::new(variant_name, DynamicVariant::Unit);
    target.apply(&dynamic_enum);
    true
}

fn bind_variant_color_pickers(
    mut commands: Commands,
    editor_state: Res<EditorState>,
    assets: Res<Assets<ParticleSystemAsset>>,
    mut color_pickers: Query<
        (Entity, &mut ColorPickerState, &VariantFieldBinding),
        (With<EditorColorPicker>, Without<ColorPickerBound>),
    >,
    variant_edit_configs: Query<&VariantEditConfig>,
    trigger_swatches: Query<&TriggerSwatchMaterial>,
) {
    let Some((_, emitter)) = get_inspecting_emitter(&editor_state, &assets) else {
        return;
    };

    for (picker_entity, mut picker_state, binding) in &mut color_pickers {
        if !matches!(binding.field_kind, VariantFieldKind::Color) {
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
        commands.entity(picker_entity).insert(ColorPickerBound);
    }
}

fn handle_variant_color_commit(
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
