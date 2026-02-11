mod swatch;
mod unified;

use bevy::prelude::*;
use bevy::reflect::{DynamicEnum, DynamicVariant, PartialReflect, ReflectMut, ReflectRef};
use sprinkles::prelude::*;

use crate::state::{DirtyState, EditorState, Inspectable};
use crate::ui::widgets::variant_edit::VariantDefinition;

pub(super) use super::inspector::FieldKind;
pub(super) use super::inspector::InspectedEmitterTracker;

pub(super) const MAX_ANCESTOR_DEPTH: usize = 10;

pub(crate) fn get_inspecting_emitter<'a>(
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

pub(super) fn get_inspecting_emitter_mut<'a>(
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

pub(super) fn get_inspecting_collider<'a>(
    editor_state: &EditorState,
    assets: &'a Assets<ParticleSystemAsset>,
) -> Option<(u8, &'a ColliderData)> {
    let inspecting = match &editor_state.inspecting {
        Some(i) if i.kind == Inspectable::Collider => i,
        _ => return None,
    };
    let handle = editor_state.current_project.as_ref()?;
    let asset = assets.get(handle)?;
    let collider = asset.colliders.get(inspecting.index as usize)?;
    Some((inspecting.index, collider))
}

pub(super) fn get_inspecting_collider_mut<'a>(
    editor_state: &EditorState,
    assets: &'a mut Assets<ParticleSystemAsset>,
) -> Option<(u8, &'a mut ColliderData)> {
    let inspecting = match &editor_state.inspecting {
        Some(i) if i.kind == Inspectable::Collider => i,
        _ => return None,
    };
    let handle = editor_state.current_project.as_ref()?;
    let asset = assets.get_mut(handle)?;
    let collider = asset.colliders.get_mut(inspecting.index as usize)?;
    Some((inspecting.index, collider))
}

pub(super) fn find_ancestor<F>(
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
    app.add_observer(unified::handle_text_commit)
        .add_observer(unified::handle_checkbox_commit)
        .add_observer(unified::handle_combobox_change)
        .add_observer(unified::handle_curve_commit)
        .add_observer(unified::handle_gradient_commit)
        .add_observer(unified::handle_color_commit)
        .add_observer(unified::handle_texture_commit)
        .add_observer(unified::handle_variant_change)
        .add_observer(swatch::sync_variant_swatch_from_color)
        .add_systems(
            Update,
            (
                unified::bind_text_inputs,
                unified::bind_checkboxes,
                unified::bind_curve_edits,
                unified::bind_gradient_edits,
                unified::bind_color_pickers,
                unified::bind_combobox_fields,
                unified::bind_variant_edits,
                unified::bind_nested_variant_edits,
                swatch::setup_variant_swatch,
                swatch::sync_variant_swatch_from_gradient,
                swatch::respawn_variant_swatch_on_switch,
            )
                .after(super::inspector::update_inspected_emitter_tracker),
        );
}

#[derive(Clone)]
pub enum FieldAccessor {
    Emitter(String),
    EmitterVariant { path: String, field_name: String },
}

#[derive(Component, Clone)]
pub struct FieldBinding {
    pub accessor: FieldAccessor,
    pub kind: FieldKind,
    pub variant_edit: Option<Entity>,
}

impl FieldBinding {
    pub fn emitter(path: impl Into<String>, kind: FieldKind) -> Self {
        Self {
            accessor: FieldAccessor::Emitter(path.into()),
            kind,
            variant_edit: None,
        }
    }

    pub fn emitter_variant(
        path: impl Into<String>,
        field_name: impl Into<String>,
        kind: FieldKind,
        variant_edit: Entity,
    ) -> Self {
        Self {
            accessor: FieldAccessor::EmitterVariant {
                path: path.into(),
                field_name: field_name.into(),
            },
            kind,
            variant_edit: Some(variant_edit),
        }
    }

    pub(super) fn read_value(&self, emitter: &EmitterData) -> FieldValue {
        match &self.accessor {
            FieldAccessor::Emitter(path) => get_field_value_by_reflection(emitter, path, &self.kind),
            FieldAccessor::EmitterVariant { path, field_name } => {
                get_variant_field_value_by_reflection(emitter, path, field_name, &self.kind)
                    .unwrap_or(FieldValue::None)
            }
        }
    }

    pub(super) fn write_value(&self, emitter: &mut EmitterData, value: &FieldValue) -> bool {
        match &self.accessor {
            FieldAccessor::Emitter(path) => set_field_value_by_reflection(emitter, path, value),
            FieldAccessor::EmitterVariant { path, field_name } => {
                set_variant_field_value_by_reflection(emitter, path, field_name, value)
            }
        }
    }

    pub fn set_enum_by_name(&self, emitter: &mut EmitterData, variant_name: &str) -> bool {
        match &self.accessor {
            FieldAccessor::Emitter(path) => set_field_enum_by_name(emitter, path, variant_name),
            FieldAccessor::EmitterVariant { path, field_name } => {
                set_variant_field_enum_by_name(emitter, path, field_name, variant_name)
            }
        }
    }

    pub fn read_reflected<'a>(
        &self,
        emitter: &'a EmitterData,
    ) -> Option<&'a dyn PartialReflect> {
        match &self.accessor {
            FieldAccessor::Emitter(path) => {
                let reflect_path = ReflectPath::new(path);
                emitter.reflect_path(reflect_path.as_str()).ok()
            }
            FieldAccessor::EmitterVariant { path, field_name } => {
                let reflect_path = ReflectPath::new(path);
                let value = emitter.reflect_path(reflect_path.as_str()).ok()?;
                resolve_variant_field_ref(value, field_name)
            }
        }
    }

    pub fn write_reflected(
        &self,
        emitter: &mut EmitterData,
        f: impl FnOnce(&mut dyn PartialReflect),
    ) -> bool {
        match &self.accessor {
            FieldAccessor::Emitter(path) => {
                let reflect_path = ReflectPath::new(path);
                if let Ok(target) = emitter.reflect_path_mut(reflect_path.as_str()) {
                    f(target);
                    true
                } else {
                    false
                }
            }
            FieldAccessor::EmitterVariant { path, field_name } => {
                let reflect_path = ReflectPath::new(path);
                let Ok(target) = emitter.reflect_path_mut(reflect_path.as_str()) else {
                    return false;
                };
                with_variant_field_mut(target, field_name, f).is_some()
            }
        }
    }

    pub fn path(&self) -> &str {
        match &self.accessor {
            FieldAccessor::Emitter(path) => path,
            FieldAccessor::EmitterVariant { path, .. } => path,
        }
    }

    pub fn field_name(&self) -> Option<&str> {
        match &self.accessor {
            FieldAccessor::Emitter(_) => None,
            FieldAccessor::EmitterVariant { field_name, .. } => Some(field_name),
        }
    }

    pub fn is_variant(&self) -> bool {
        matches!(self.accessor, FieldAccessor::EmitterVariant { .. })
    }
}

#[derive(Debug, Clone)]
pub(super) struct ReflectPath(String);

impl ReflectPath {
    pub(super) fn new(path: &str) -> Self {
        Self(format!(".{}", path))
    }

    pub(super) fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone)]
pub(super) enum FieldValue {
    None,
    F32(f32),
    U32(u32),
    OptionalU32(Option<u32>),
    Bool(bool),
    Vec2(Vec2),
    Vec3(Vec3),
    Range(f32, f32),
    Color([f32; 4]),
}

impl FieldValue {
    pub(super) fn to_display_string(&self, kind: &FieldKind) -> Option<String> {
        match self {
            FieldValue::F32(v) => f32::to_display_string(*v, kind),
            FieldValue::U32(v) => u32::to_display_string(*v, kind),
            FieldValue::OptionalU32(v) => Option::<u32>::to_display_string(*v, kind),
            _ => None,
        }
    }

    pub(super) fn to_bool(&self) -> Option<bool> {
        match self {
            FieldValue::Bool(v) => Some(*v),
            _ => None,
        }
    }

    pub(super) fn to_color(&self) -> Option<[f32; 4]> {
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

impl Bindable for Vec2 {
    fn try_from_reflected(value: &dyn PartialReflect) -> Option<Self> {
        value.try_downcast_ref::<Vec2>().copied()
    }

    fn apply_to_reflect(&self, target: &mut dyn PartialReflect) -> bool {
        if let Some(field) = target.try_downcast_mut::<Vec2>() {
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

pub(super) fn format_f32(v: f32) -> String {
    let mut text = v.to_string();
    if !text.contains('.') {
        text.push_str(".0");
    }
    text
}

pub(super) fn set_vec2_component(vec: &mut Vec2, index: usize, value: f32) {
    match index {
        0 => vec.x = value,
        1 => vec.y = value,
        _ => {}
    }
}

pub(super) fn get_vec2_component(vec: Vec2, index: usize) -> f32 {
    match index {
        0 => vec.x,
        1 => vec.y,
        _ => 0.0,
    }
}

pub(super) fn set_vec3_component(vec: &mut Vec3, index: usize, value: f32) {
    match index {
        0 => vec.x = value,
        1 => vec.y = value,
        2 => vec.z = value,
        _ => {}
    }
}

pub(super) fn get_vec3_component(vec: Vec3, index: usize) -> f32 {
    match index {
        0 => vec.x,
        1 => vec.y,
        2 => vec.z,
        _ => 0.0,
    }
}

pub(super) fn get_field_value_by_reflection(
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

pub(super) fn set_field_value_by_reflection(
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

pub(super) fn parse_field_value(text: &str, kind: &FieldKind) -> FieldValue {
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
        | FieldKind::Curve
        | FieldKind::AnimatedVelocity
        | FieldKind::TextureRef => FieldValue::None,
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
    if let Some(v) = Vec2::try_from_reflected(value) {
        return FieldValue::Vec2(v);
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
        FieldValue::Vec2(v) => v.apply_to_reflect(target),
        FieldValue::Vec3(v) => v.apply_to_reflect(target),
        FieldValue::Range(min, max) => ParticleRange {
            min: *min,
            max: *max,
        }
        .apply_to_reflect(target),
        FieldValue::Color(c) => c.apply_to_reflect(target),
        FieldValue::None => false,
    }
}

pub(super) fn get_variant_index_by_reflection(
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

pub(crate) fn resolve_variant_field_ref<'a>(
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
        match inner.reflect_ref() {
            ReflectRef::Struct(struct_ref) => {
                return struct_ref.field(field_name);
            }
            ReflectRef::Enum(inner_enum) => {
                return inner_enum.field(field_name);
            }
            _ => {}
        }
    }
    None
}

pub(super) fn with_variant_field_mut<F, R>(
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
        match inner.reflect_mut() {
            ReflectMut::Struct(struct_mut) => {
                if let Some(field) = struct_mut.field_mut(field_name) {
                    return Some(f(field));
                }
            }
            ReflectMut::Enum(inner_enum) => {
                if let Some(field) = inner_enum.field_mut(field_name) {
                    return Some(f(field));
                }
            }
            _ => {}
        }
    }
    None
}

pub(super) fn get_variant_field_value_by_reflection(
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

pub(super) fn set_variant_field_value_by_reflection(
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

pub(super) fn create_variant_from_definition(
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

pub(super) fn find_ancestor_entity(
    entity: Entity,
    target: Entity,
    parents: &Query<&ChildOf>,
) -> bool {
    find_ancestor(entity, parents, MAX_ANCESTOR_DEPTH, |e| e == target).is_some()
}

pub(super) fn mark_dirty_and_restart(
    dirty_state: &mut DirtyState,
    emitter_runtimes: &mut Query<&mut EmitterRuntime>,
    fixed_seed: Option<u32>,
) {
    dirty_state.has_unsaved_changes = true;
    for mut runtime in emitter_runtimes.iter_mut() {
        runtime.restart(fixed_seed);
    }
}

pub(super) fn label_to_variant_name(label: &str) -> String {
    label.split_whitespace().collect()
}

pub(super) fn set_variant_field_enum_by_name(
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

pub(super) fn set_field_enum_by_name(
    emitter: &mut EmitterData,
    path: &str,
    variant_name: &str,
) -> bool {
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
