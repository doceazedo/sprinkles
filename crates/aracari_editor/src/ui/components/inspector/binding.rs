use aracari::prelude::*;
use bevy::ecs::system::ParamSet;
use bevy::prelude::*;
use bevy::reflect::{DynamicEnum, DynamicVariant, PartialReflect, ReflectMut, ReflectRef};
use bevy_ui_text_input::{
    TextInputQueue,
    actions::{TextInputAction, TextInputEdit},
};

use crate::state::{DirtyState, EditorState, Inspectable};
use crate::ui::widgets::checkbox::{CheckboxCommitEvent, CheckboxState};
use crate::ui::widgets::color_picker::{
    CheckerboardMaterial, ColorPickerChangeEvent, ColorPickerCommitEvent, ColorPickerState,
    EditorColorPicker, TriggerSwatchMaterial,
};
use crate::ui::widgets::combobox::ComboBoxChangeEvent;
use crate::ui::widgets::curve_edit::{CurveEditCommitEvent, CurveEditState, EditorCurveEdit};
use crate::ui::widgets::gradient_edit::{
    EditorGradientEdit, GradientEditCommitEvent, GradientEditState, GradientMaterial,
};
use crate::ui::widgets::text_edit::{EditorTextEdit, TextEditCommitEvent};
use crate::ui::widgets::variant_edit::{
    EditorVariantEdit, VariantComboBox, VariantDefinition, VariantEditConfig, VariantEditSwatchSlot,
    VariantFieldBinding,
};
use crate::ui::widgets::vector_edit::EditorVectorEdit;

use super::types::FieldKind;

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
    app.add_observer(handle_text_edit_commit)
        .add_observer(handle_checkbox_commit)
        .add_observer(handle_variant_change)
        .add_observer(handle_combobox_change)
        .add_observer(handle_variant_color_commit)
        .add_observer(handle_curve_edit_commit)
        .add_observer(handle_variant_gradient_commit)
        .add_observer(sync_variant_swatch_from_color)
        .add_systems(
            Update,
            (
                bind_values_to_inputs,
                bind_curve_edit_values,
                bind_variant_edits,
                bind_variant_field_values,
                bind_variant_color_pickers,
                bind_variant_gradient_edits,
                setup_variant_swatch,
                sync_variant_swatch_from_gradient,
                respawn_variant_swatch_on_switch,
            ),
        );
}

#[derive(Component, Clone)]
pub struct Field {
    pub path: String,
    pub kind: FieldKind,
}

#[derive(Component)]
struct Bound {
    field_entity: Entity,
    is_variant_field: bool,
}

impl Bound {
    fn direct(field_entity: Entity) -> Self {
        Self {
            field_entity,
            is_variant_field: false,
        }
    }

    fn variant(field_entity: Entity) -> Self {
        Self {
            field_entity,
            is_variant_field: true,
        }
    }
}

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
    Range(f32, f32),
    Color([f32; 4]),
}

impl FieldValue {
    fn to_display_string(&self, kind: &FieldKind) -> Option<String> {
        match self {
            FieldValue::F32(v) => f32::to_display_string(*v, kind),
            FieldValue::U32(v) => u32::to_display_string(*v, kind),
            FieldValue::OptionalU32(v) => Option::<u32>::to_display_string(*v, kind),
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

trait Bindable: Sized {
    fn try_from_reflected(value: &dyn PartialReflect) -> Option<Self>;
    fn apply_to_reflect(&self, target: &mut dyn PartialReflect) -> bool;
    fn to_display_string(value: Self, kind: &FieldKind) -> Option<String>;
    fn parse(text: &str, kind: &FieldKind) -> Option<Self>;
}

impl Bindable for f32 {
    fn try_from_reflected(value: &dyn PartialReflect) -> Option<Self> {
        value.try_downcast_ref::<f32>().copied()
    }

    fn apply_to_reflect(&self, target: &mut dyn PartialReflect) -> bool {
        if let Some(field) = target.try_downcast_mut::<f32>() {
            if (*field - self).abs() > f32::EPSILON {
                *field = *self;
                return true;
            }
        }
        false
    }

    fn to_display_string(value: Self, kind: &FieldKind) -> Option<String> {
        match kind {
            FieldKind::F32Percent => {
                let display = (value * 100.0 * 100.0).round() / 100.0;
                Some(format_f32(display))
            }
            _ => Some(format_f32(value)),
        }
    }

    fn parse(text: &str, kind: &FieldKind) -> Option<Self> {
        let text = text.trim();
        match kind {
            FieldKind::F32Percent => text
                .trim_end_matches('%')
                .trim()
                .parse()
                .ok()
                .map(|v: f32| v / 100.0),
            _ => text.trim_end_matches('s').trim().parse().ok(),
        }
    }
}

impl Bindable for u32 {
    fn try_from_reflected(value: &dyn PartialReflect) -> Option<Self> {
        value.try_downcast_ref::<u32>().copied()
    }

    fn apply_to_reflect(&self, target: &mut dyn PartialReflect) -> bool {
        if let Some(field) = target.try_downcast_mut::<u32>() {
            if *field != *self {
                *field = *self;
                return true;
            }
        }
        false
    }

    fn to_display_string(value: Self, kind: &FieldKind) -> Option<String> {
        match kind {
            FieldKind::U32OrEmpty if value == 0 => None,
            _ => Some(value.to_string()),
        }
    }

    fn parse(text: &str, kind: &FieldKind) -> Option<Self> {
        let text = text.trim();
        if text.is_empty() && matches!(kind, FieldKind::U32OrEmpty) {
            Some(0)
        } else {
            text.parse().ok()
        }
    }
}

impl Bindable for Option<u32> {
    fn try_from_reflected(value: &dyn PartialReflect) -> Option<Self> {
        value.try_downcast_ref::<Option<u32>>().copied()
    }

    fn apply_to_reflect(&self, target: &mut dyn PartialReflect) -> bool {
        if let Some(field) = target.try_downcast_mut::<Option<u32>>() {
            if *field != *self {
                *field = *self;
                return true;
            }
        }
        false
    }

    fn to_display_string(value: Self, kind: &FieldKind) -> Option<String> {
        match (value, kind) {
            (None, _) => None,
            (Some(0), FieldKind::OptionalU32) => None,
            (Some(v), _) => Some(v.to_string()),
        }
    }

    fn parse(text: &str, kind: &FieldKind) -> Option<Self> {
        let text = text.trim();
        let _ = kind;
        if text.is_empty() {
            Some(None)
        } else {
            text.parse::<u32>()
                .ok()
                .map(|v| if v == 0 { None } else { Some(v) })
        }
    }
}

impl Bindable for bool {
    fn try_from_reflected(value: &dyn PartialReflect) -> Option<Self> {
        value.try_downcast_ref::<bool>().copied()
    }

    fn apply_to_reflect(&self, target: &mut dyn PartialReflect) -> bool {
        if let Some(field) = target.try_downcast_mut::<bool>() {
            if *field != *self {
                *field = *self;
                return true;
            }
        }
        false
    }

    fn to_display_string(_value: Self, _kind: &FieldKind) -> Option<String> {
        None
    }

    fn parse(_text: &str, _kind: &FieldKind) -> Option<Self> {
        None
    }
}

impl Bindable for Vec3 {
    fn try_from_reflected(value: &dyn PartialReflect) -> Option<Self> {
        value.try_downcast_ref::<Vec3>().copied()
    }

    fn apply_to_reflect(&self, target: &mut dyn PartialReflect) -> bool {
        if let Some(field) = target.try_downcast_mut::<Vec3>() {
            if (*field - *self).length() > f32::EPSILON {
                *field = *self;
                return true;
            }
        }
        false
    }

    fn to_display_string(_value: Self, _kind: &FieldKind) -> Option<String> {
        None
    }

    fn parse(_text: &str, _kind: &FieldKind) -> Option<Self> {
        None
    }
}

impl Bindable for [f32; 4] {
    fn try_from_reflected(value: &dyn PartialReflect) -> Option<Self> {
        value.try_downcast_ref::<[f32; 4]>().copied()
    }

    fn apply_to_reflect(&self, target: &mut dyn PartialReflect) -> bool {
        if let Some(field) = target.try_downcast_mut::<[f32; 4]>() {
            if *field != *self {
                *field = *self;
                return true;
            }
        }
        false
    }

    fn to_display_string(_value: Self, _kind: &FieldKind) -> Option<String> {
        None
    }

    fn parse(_text: &str, _kind: &FieldKind) -> Option<Self> {
        None
    }
}

impl Bindable for ParticleRange {
    fn try_from_reflected(value: &dyn PartialReflect) -> Option<Self> {
        value.try_downcast_ref::<ParticleRange>().copied()
    }

    fn apply_to_reflect(&self, target: &mut dyn PartialReflect) -> bool {
        if let Some(field) = target.try_downcast_mut::<ParticleRange>() {
            if (field.min - self.min).abs() > f32::EPSILON
                || (field.max - self.max).abs() > f32::EPSILON
            {
                field.min = self.min;
                field.max = self.max;
                return true;
            }
        }
        false
    }

    fn to_display_string(_value: Self, _kind: &FieldKind) -> Option<String> {
        None
    }

    fn parse(_text: &str, _kind: &FieldKind) -> Option<Self> {
        None
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
    match kind {
        FieldKind::F32 | FieldKind::F32Percent => f32::parse(text, kind)
            .map(FieldValue::F32)
            .unwrap_or(FieldValue::None),
        FieldKind::U32 | FieldKind::U32OrEmpty => u32::parse(text, kind)
            .map(FieldValue::U32)
            .unwrap_or(FieldValue::None),
        FieldKind::OptionalU32 => Option::<u32>::parse(text, kind)
            .map(FieldValue::OptionalU32)
            .unwrap_or(FieldValue::None),
        FieldKind::Bool
        | FieldKind::Vector(_)
        | FieldKind::ComboBox { .. }
        | FieldKind::Color
        | FieldKind::Gradient
        | FieldKind::Curve => FieldValue::None,
    }
}

fn reflect_to_field_value(value: &dyn PartialReflect, _kind: &FieldKind) -> FieldValue {
    if let Some(v) = f32::try_from_reflected(value) {
        return FieldValue::F32(v);
    }
    if let Some(v) = u32::try_from_reflected(value) {
        return FieldValue::U32(v);
    }
    if let Some(v) = bool::try_from_reflected(value) {
        return FieldValue::Bool(v);
    }
    if let Some(v) = Vec3::try_from_reflected(value) {
        return FieldValue::Vec3(v);
    }
    if let Some(v) = Option::<u32>::try_from_reflected(value) {
        return FieldValue::OptionalU32(v);
    }
    if let Some(v) = <[f32; 4]>::try_from_reflected(value) {
        return FieldValue::Color(v);
    }
    if let Some(v) = ParticleRange::try_from_reflected(value) {
        return FieldValue::Range(v.min, v.max);
    }
    if let ReflectRef::Enum(enum_ref) = value.reflect_ref() {
        return FieldValue::U32(enum_ref.variant_index() as u32);
    }
    FieldValue::None
}

fn apply_field_value_to_reflect(target: &mut dyn PartialReflect, value: &FieldValue) -> bool {
    match value {
        FieldValue::F32(v) => v.apply_to_reflect(target),
        FieldValue::U32(v) => v.apply_to_reflect(target),
        FieldValue::OptionalU32(v) => v.apply_to_reflect(target),
        FieldValue::Bool(v) => v.apply_to_reflect(target),
        FieldValue::Vec3(v) => v.apply_to_reflect(target),
        FieldValue::Range(min, max) => {
            ParticleRange { min: *min, max: *max }.apply_to_reflect(target)
        }
        FieldValue::Color(c) => c.apply_to_reflect(target),
        FieldValue::None => false,
    }
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

fn resolve_variant_field_ref<'a>(
    value: &'a dyn PartialReflect,
    field_name: &str,
) -> Option<&'a dyn PartialReflect> {
    let ReflectRef::Enum(enum_ref) = value.reflect_ref() else {
        return None;
    };
    if let Some(field) = enum_ref.field(field_name) {
        return Some(field);
    }
    if let Some(inner) = enum_ref.field_at(0) {
        if let ReflectRef::Struct(struct_ref) = inner.reflect_ref() {
            return struct_ref.field(field_name);
        }
    }
    None
}

fn with_variant_field_mut<F, R>(
    value: &mut dyn PartialReflect,
    field_name: &str,
    f: F,
) -> Option<R>
where
    F: FnOnce(&mut dyn PartialReflect) -> R,
{
    let ReflectMut::Enum(enum_mut) = value.reflect_mut() else {
        return None;
    };
    if let Some(field) = enum_mut.field_mut(field_name) {
        return Some(f(field));
    }
    if let Some(inner) = enum_mut.field_at_mut(0) {
        if let ReflectMut::Struct(struct_mut) = inner.reflect_mut() {
            if let Some(field) = struct_mut.field_mut(field_name) {
                return Some(f(field));
            }
        }
    }
    None
}

fn get_variant_field_value_by_reflection(
    emitter: &EmitterData,
    path: &str,
    field_name: &str,
    kind: &FieldKind,
) -> Option<FieldValue> {
    let reflect_path = ReflectPath::new(path);
    let value = emitter.reflect_path(reflect_path.as_str()).ok()?;
    let field = resolve_variant_field_ref(value, field_name)?;
    Some(reflect_to_field_value(field, kind))
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
    with_variant_field_mut(target, field_name, |field| {
        apply_field_value_to_reflect(field, value)
    })
    .unwrap_or(false)
}

fn create_variant_from_definition(
    emitter: &mut EmitterData,
    path: &str,
    variant_def: &VariantDefinition,
) -> bool {
    let Some(default_value) = variant_def.create_default() else {
        warn!(
            "create_variant_from_definition: create_default() returned None for variant '{}' at path '{}'",
            variant_def.name, path
        );
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


fn bind_variant_edits(
    editor_state: Res<EditorState>,
    assets: Res<Assets<ParticleSystemAsset>>,
    mut variant_edits: Query<(&Field, &mut VariantEditConfig), With<EditorVariantEdit>>,
    new_variant_edits: Query<Entity, Added<EditorVariantEdit>>,
    mut last_bound_emitter: Local<Option<u8>>,
) {
    let emitter_info = get_inspecting_emitter(&editor_state, &assets);
    if !should_rebind(
        &mut last_bound_emitter,
        emitter_info.map(|(i, _)| i),
        !new_variant_edits.is_empty(),
    ) {
        return;
    }

    let Some((_, emitter)) = emitter_info else {
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

fn bind_variant_field_values(
    mut commands: Commands,
    editor_state: Res<EditorState>,
    assets: Res<Assets<ParticleSystemAsset>>,
    variant_field_bindings: Query<(Entity, &VariantFieldBinding), Without<Bound>>,
    variant_edit_configs: Query<&VariantEditConfig>,
    mut text_edits: Query<
        (Entity, &ChildOf, &mut TextInputQueue),
        (With<EditorTextEdit>, Without<Bound>),
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

        let mut bound = false;

        if let Some(text) = value.to_display_string(&binding.field_kind) {
            for (text_edit_entity, text_edit_parent, mut queue) in &mut text_edits {
                if find_ancestor_entity(text_edit_parent.parent(), binding_entity, &parents) {
                    queue.add(TextInputAction::Edit(TextInputEdit::SelectAll));
                    queue.add(TextInputAction::Edit(TextInputEdit::Paste(text.clone())));
                    commands
                        .entity(text_edit_entity)
                        .insert(Bound::variant(binding_entity));
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
                            commands
                                .entity(text_edit_entity)
                                .insert(Bound::variant(binding_entity));
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
            commands
                .entity(binding_entity)
                .insert(Bound::variant(binding_entity));
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
    vector_edit_children: Query<&Children, With<EditorVectorEdit>>,
    mut last_bound_emitter: Local<Option<u8>>,
) {
    let emitter_info = get_inspecting_emitter(&editor_state, &assets);
    let has_new_checkboxes = !checkbox_set.p0().is_empty();
    let has_new_fields = !new_fields.is_empty() || !new_text_edits.is_empty();
    if !should_rebind(
        &mut last_bound_emitter,
        emitter_info.map(|(i, _)| i),
        has_new_fields || has_new_checkboxes,
    ) {
        return;
    }

    let Some((_, emitter)) = emitter_info else {
        return;
    };

    for (entity, child_of, mut queue) in &mut text_edits {
        let Some(field) = find_ancestor_field(child_of.parent(), &fields, &parents) else {
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
                                let text = format_f32(v);
                                queue.add(TextInputAction::Edit(TextInputEdit::SelectAll));
                                queue.add(TextInputAction::Edit(TextInputEdit::Paste(text)));
                                if let Some(field_entity) =
                                    find_ancestor_field_entity(child_of.parent(), &fields, &parents)
                                {
                                    commands.entity(entity).insert(Bound::direct(field_entity));
                                }
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

        queue.add(TextInputAction::Edit(TextInputEdit::SelectAll));
        queue.add(TextInputAction::Edit(TextInputEdit::Paste(text)));
        if let Some(field_entity) =
            find_ancestor_field_entity(child_of.parent(), &fields, &parents)
        {
            commands.entity(entity).insert(Bound::direct(field_entity));
        }
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
        if let Some(field_entity) = find_field_entity_for_entity(entity, &fields, &parents) {
            commands.entity(entity).insert(Bound::direct(field_entity));
        }
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

fn find_ancestor_field_entity(
    entity: Entity,
    fields: &Query<&Field>,
    parents: &Query<&ChildOf>,
) -> Option<Entity> {
    find_ancestor(entity, parents, MAX_ANCESTOR_DEPTH, |e| fields.get(e).is_ok())
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

fn find_field_entity_for_entity(
    entity: Entity,
    fields: &Query<&Field>,
    parents: &Query<&ChildOf>,
) -> Option<Entity> {
    if fields.get(entity).is_ok() {
        return Some(entity);
    }
    if let Ok(child_of) = parents.get(entity) {
        return find_ancestor_field_entity(child_of.parent(), fields, parents);
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

fn should_rebind(
    last_bound: &mut Option<u8>,
    current_index: Option<u8>,
    has_new_widgets: bool,
) -> bool {
    let Some(index) = current_index else {
        *last_bound = None;
        return false;
    };
    let changed = *last_bound != Some(index);
    *last_bound = Some(index);
    changed || has_new_widgets
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
    with_variant_field_mut(target, field_name, |field| {
        set_enum_variant_by_name(field, variant_name)
    })
    .unwrap_or(false)
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
            .insert(Bound::variant(picker_entity));
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

fn bind_curve_edit_values(
    mut commands: Commands,
    editor_state: Res<EditorState>,
    assets: Res<Assets<ParticleSystemAsset>>,
    new_curve_edits: Query<Entity, Added<EditorCurveEdit>>,
    mut curve_edits: Query<(Entity, Option<&ChildOf>, &mut CurveEditState), With<EditorCurveEdit>>,
    fields: Query<&Field>,
    parents: Query<&ChildOf>,
    variant_edit_query: Query<(), With<EditorVariantEdit>>,
    mut last_bound_emitter: Local<Option<u8>>,
) {
    let emitter_info = get_inspecting_emitter(&editor_state, &assets);
    if !should_rebind(
        &mut last_bound_emitter,
        emitter_info.map(|(i, _)| i),
        !new_curve_edits.is_empty(),
    ) {
        return;
    }

    let Some((_, emitter)) = emitter_info else {
        return;
    };

    for (entity, child_of, mut state) in &mut curve_edits {
        let Some(field) = find_field_for_entity(entity, &fields, &parents) else {
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

        let field_entity =
            find_field_entity_for_entity(entity, &fields, &parents).unwrap_or(entity);

        // try binding directly to CurveTexture
        if let Some(curve_texture) = value.try_downcast_ref::<CurveTexture>() {
            state.set_curve(curve_texture.clone());
            commands.entity(entity).insert(Bound::direct(field_entity));
            continue;
        }

        // try binding to Option<CurveTexture>
        if let Some(curve_opt) = value.try_downcast_ref::<Option<CurveTexture>>() {
            if let Some(curve) = curve_opt {
                state.set_curve(curve.clone());
            }
            // mark as bound even if None so we can create the curve on commit
            commands.entity(entity).insert(Bound::direct(field_entity));
        }
    }
}

fn handle_curve_edit_commit(
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

    let Some(field) = find_field_for_entity(trigger.entity, &fields, &parents) else {
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

fn handle_text_edit_commit(
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
        handle_variant_text_commit(
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

    let Some(field) = find_ancestor_field(child_of.parent(), fields, parents) else {
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

#[allow(clippy::too_many_arguments)]
fn handle_variant_text_commit(
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
        FieldKind::ComboBox { .. } | FieldKind::Color | FieldKind::Gradient | FieldKind::Curve => FieldValue::None,
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

fn handle_checkbox_commit(
    trigger: On<CheckboxCommitEvent>,
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
        if set_variant_field_value_by_reflection(
            emitter,
            &config.path,
            &binding.field_name,
            &value,
        ) {
            mark_dirty_and_restart(&mut dirty_state, &mut emitter_runtimes, emitter.time.fixed_seed);
        }
    } else {
        let Some(field) = find_field_for_entity(trigger.entity, &fields, &parents) else {
            return;
        };
        if set_field_value_by_reflection(emitter, &field.path, &value) {
            mark_dirty_and_restart(&mut dirty_state, &mut emitter_runtimes, emitter.time.fixed_seed);
        }
    }
}

fn bind_variant_gradient_edits(
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
                .insert(Bound::variant(entity));
        }
    }
}

fn handle_variant_gradient_commit(
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

#[derive(Component)]
struct VariantSwatchOwner(Entity);

#[derive(Component)]
struct SolidSwatchMaterial(Entity);

#[derive(Component)]
struct GradientSwatchNode(Entity);

fn swatch_fill_node() -> Node {
    Node {
        position_type: PositionType::Absolute,
        width: percent(100),
        height: percent(100),
        ..default()
    }
}

fn spawn_swatch_material(
    commands: &mut Commands,
    variant_edit_entity: Entity,
    color_value: &SolidOrGradientColor,
    checkerboard_materials: &mut Assets<CheckerboardMaterial>,
    gradient_materials: &mut Assets<GradientMaterial>,
) -> Entity {
    match color_value {
        SolidOrGradientColor::Solid { color } => commands
            .spawn((
                SolidSwatchMaterial(variant_edit_entity),
                MaterialNode(checkerboard_materials.add(CheckerboardMaterial {
                    color: Vec4::new(color[0], color[1], color[2], color[3]),
                    size: 4.0,
                    border_radius: 4.0,
                })),
                swatch_fill_node(),
            ))
            .id(),
        SolidOrGradientColor::Gradient { gradient } => commands
            .spawn((
                GradientSwatchNode(variant_edit_entity),
                MaterialNode(gradient_materials.add(GradientMaterial::swatch(gradient))),
                swatch_fill_node(),
            ))
            .id(),
    }
}

fn setup_variant_swatch(
    mut commands: Commands,
    editor_state: Res<EditorState>,
    assets: Res<Assets<ParticleSystemAsset>>,
    swatch_slots: Query<(Entity, &VariantEditSwatchSlot), Added<VariantEditSwatchSlot>>,
    variant_edit_configs: Query<(&VariantEditConfig, &Field), With<EditorVariantEdit>>,
    mut checkerboard_materials: ResMut<Assets<CheckerboardMaterial>>,
    mut gradient_materials: ResMut<Assets<GradientMaterial>>,
) {
    if swatch_slots.is_empty() {
        return;
    }

    let emitter = get_inspecting_emitter(&editor_state, &assets).map(|(_, e)| e);

    for (slot_entity, slot) in &swatch_slots {
        let variant_edit_entity = slot.0;
        let Ok((_config, field)) = variant_edit_configs.get(variant_edit_entity) else {
            continue;
        };

        let Some(emitter) = emitter else {
            continue;
        };

        let reflect_path = ReflectPath::new(&field.path);
        let Ok(value) = emitter.reflect_path(reflect_path.as_str()) else {
            continue;
        };

        let Some(color_value) = value.try_downcast_ref::<SolidOrGradientColor>() else {
            continue;
        };

        commands
            .entity(slot_entity)
            .insert(VariantSwatchOwner(variant_edit_entity));

        let material_entity = spawn_swatch_material(
            &mut commands,
            variant_edit_entity,
            color_value,
            &mut checkerboard_materials,
            &mut gradient_materials,
        );
        commands.entity(slot_entity).add_child(material_entity);
    }
}

fn sync_variant_swatch_from_color(
    trigger: On<ColorPickerChangeEvent>,
    variant_field_bindings: Query<&VariantFieldBinding>,
    solid_swatches: Query<(&SolidSwatchMaterial, &MaterialNode<CheckerboardMaterial>)>,
    mut checkerboard_materials: ResMut<Assets<CheckerboardMaterial>>,
    parents: Query<&ChildOf>,
) {
    let binding = find_ancestor(trigger.entity, &parents, MAX_ANCESTOR_DEPTH, |e| {
        variant_field_bindings.get(e).is_ok()
    })
    .and_then(|e| variant_field_bindings.get(e).ok());

    let Some(binding) = binding else {
        return;
    };

    if !matches!(binding.field_kind, FieldKind::Color) {
        return;
    }

    let variant_edit = binding.variant_edit;

    for (solid, mat_node) in &solid_swatches {
        if solid.0 != variant_edit {
            continue;
        }
        if let Some(mat) = checkerboard_materials.get_mut(&mat_node.0) {
            let c = trigger.color;
            mat.color = Vec4::new(c[0], c[1], c[2], c[3]);
        }
    }
}

fn sync_variant_swatch_from_gradient(
    mut commands: Commands,
    gradient_edits: Query<
        (Entity, &GradientEditState, &VariantFieldBinding),
        (With<EditorGradientEdit>, Changed<GradientEditState>),
    >,
    swatches: Query<(Entity, &VariantSwatchOwner, &Children)>,
    gradient_nodes: Query<Entity, With<GradientSwatchNode>>,
    mut gradient_materials: ResMut<Assets<GradientMaterial>>,
) {
    for (_, state, binding) in &gradient_edits {
        if !matches!(binding.field_kind, FieldKind::Gradient) {
            continue;
        }

        let variant_edit = binding.variant_edit;

        let Some((swatch_entity, _, swatch_children)) = swatches
            .iter()
            .find(|(_, owner, _)| owner.0 == variant_edit)
        else {
            continue;
        };

        for child in swatch_children.iter() {
            if gradient_nodes.get(child).is_ok() {
                commands.entity(child).try_despawn();
            }
        }

        let material_entity = commands
            .spawn((
                GradientSwatchNode(variant_edit),
                MaterialNode(
                    gradient_materials
                        .add(GradientMaterial::swatch(&state.gradient)),
                ),
                swatch_fill_node(),
            ))
            .id();
        commands.entity(swatch_entity).add_child(material_entity);
    }
}

fn respawn_variant_swatch_on_switch(
    mut commands: Commands,
    editor_state: Res<EditorState>,
    assets: Res<Assets<ParticleSystemAsset>>,
    changed_configs: Query<(Entity, &VariantEditConfig, &Field), Changed<VariantEditConfig>>,
    swatches: Query<(Entity, &VariantSwatchOwner, &Children)>,
    mut checkerboard_materials: ResMut<Assets<CheckerboardMaterial>>,
    mut gradient_materials: ResMut<Assets<GradientMaterial>>,
) {
    let Some((_, emitter)) = get_inspecting_emitter(&editor_state, &assets) else {
        return;
    };

    for (variant_edit_entity, config, field) in &changed_configs {
        if !config.show_swatch_slot {
            continue;
        }

        let Some((swatch_entity, _, swatch_children)) = swatches
            .iter()
            .find(|(_, owner, _)| owner.0 == variant_edit_entity)
        else {
            continue;
        };

        for child in swatch_children.iter() {
            commands.entity(child).try_despawn();
        }

        let reflect_path = ReflectPath::new(&field.path);
        let Ok(value) = emitter.reflect_path(reflect_path.as_str()) else {
            continue;
        };

        let Some(color_value) = value.try_downcast_ref::<SolidOrGradientColor>() else {
            continue;
        };

        let material_entity = spawn_swatch_material(
            &mut commands,
            variant_edit_entity,
            color_value,
            &mut checkerboard_materials,
            &mut gradient_materials,
        );
        commands.entity(swatch_entity).add_child(material_entity);
    }
}
