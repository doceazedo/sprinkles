mod commit;
mod swatch;
mod sync;

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use bevy::reflect::{
    DynamicEnum, DynamicTuple, DynamicVariant, PartialReflect, ReflectMut, ReflectRef,
};
use sprinkles::prelude::*;

use crate::state::{DirtyState, EditorState, Inspectable};
use crate::ui::widgets::combobox::ComboBoxConfig;
use crate::ui::widgets::text_edit::EditorTextEdit;
use crate::ui::widgets::variant_edit::VariantDefinition;
use crate::ui::widgets::vector_edit::VectorComponentIndex;

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

pub(super) fn get_inspected_data<'a>(
    editor_state: &EditorState,
    assets: &'a Assets<ParticleSystemAsset>,
) -> Option<&'a dyn Reflect> {
    let inspecting = editor_state.inspecting.as_ref()?;
    let handle = editor_state.current_project.as_ref()?;
    let asset = assets.get(handle)?;
    match inspecting.kind {
        Inspectable::Emitter => {
            let emitter = asset.emitters.get(inspecting.index as usize)?;
            Some(emitter)
        }
        Inspectable::Collider => {
            let collider = asset.colliders.get(inspecting.index as usize)?;
            Some(collider)
        }
    }
}

pub(super) fn get_inspected_data_mut<'a>(
    editor_state: &EditorState,
    assets: &'a mut Assets<ParticleSystemAsset>,
) -> Option<&'a mut dyn Reflect> {
    let inspecting = editor_state.inspecting.as_ref()?;
    let handle = editor_state.current_project.as_ref()?;
    let asset = assets.get_mut(handle)?;
    match inspecting.kind {
        Inspectable::Emitter => {
            let emitter = asset.emitters.get_mut(inspecting.index as usize)?;
            Some(emitter)
        }
        Inspectable::Collider => {
            let collider = asset.colliders.get_mut(inspecting.index as usize)?;
            Some(collider)
        }
    }
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
    app.add_observer(commit::handle_text_commit)
        .add_observer(commit::handle_checkbox_commit)
        .add_observer(commit::handle_combobox_change)
        .add_observer(commit::handle_curve_commit)
        .add_observer(commit::handle_gradient_commit)
        .add_observer(commit::handle_color_commit)
        .add_observer(commit::handle_texture_commit)
        .add_observer(commit::handle_variant_change)
        .add_observer(swatch::sync_variant_swatch_from_color)
        .add_systems(
            Update,
            (
                propagate_bindings,
                (
                    sync::bind_text_inputs,
                    sync::bind_widget_values,
                    sync::bind_color_pickers,
                    swatch::setup_variant_swatch,
                    swatch::sync_variant_swatch_from_gradient,
                ),
            )
                .chain()
                .after(super::inspector::update_inspected_emitter_tracker),
        );
}

#[derive(Component)]
pub(super) struct BoundTo {
    pub binding: Entity,
    pub component_index: Option<usize>,
}

fn propagate_bindings(
    new_text_edits: Query<Entity, Added<EditorTextEdit>>,
    new_comboboxes: Query<Entity, (Added<ComboBoxConfig>, Without<FieldBinding>)>,
    parents: Query<&ChildOf>,
    bindings: Query<Entity, With<FieldBinding>>,
    vector_indices: Query<&VectorComponentIndex>,
    mut commands: Commands,
) {
    for widget_entity in new_text_edits.iter().chain(new_comboboxes.iter()) {
        let mut entity = widget_entity;
        let mut component_index = None;
        for _ in 0..MAX_ANCESTOR_DEPTH {
            if let Ok(vi) = vector_indices.get(entity) {
                component_index = Some(vi.0);
            }
            if bindings.get(entity).is_ok() {
                commands.entity(widget_entity).try_insert(BoundTo {
                    binding: entity,
                    component_index,
                });
                break;
            }
            let Ok(child_of) = parents.get(entity) else {
                break;
            };
            entity = child_of.parent();
        }
    }
}

#[derive(Clone)]
pub enum FieldAccessor {
    Direct(String),
    VariantField { path: String, field_name: String },
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
            accessor: FieldAccessor::Direct(path.into()),
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
            accessor: FieldAccessor::VariantField {
                path: path.into(),
                field_name: field_name.into(),
            },
            kind,
            variant_edit: Some(variant_edit),
        }
    }

    pub fn emitter_variant_field(
        path: impl Into<String>,
        field_name: impl Into<String>,
        kind: FieldKind,
    ) -> Self {
        Self {
            accessor: FieldAccessor::VariantField {
                path: path.into(),
                field_name: field_name.into(),
            },
            kind,
            variant_edit: None,
        }
    }

    pub(super) fn resolve_ref<'a>(&self, data: &'a dyn Reflect) -> Option<&'a dyn PartialReflect> {
        let path = ReflectPath::new(self.path());
        let value = match data.reflect_path(path.as_str()) {
            Ok(v) => v,
            Err(e) => {
                warn!("binding: failed to resolve '{}': {}", self.path(), e);
                return None;
            }
        };
        match &self.accessor {
            FieldAccessor::Direct(_) => Some(value),
            FieldAccessor::VariantField { field_name, .. } => {
                resolve_variant_field_ref(value, field_name)
            }
        }
    }

    pub(super) fn with_resolved_mut<R>(
        &self,
        data: &mut dyn Reflect,
        f: impl FnOnce(&mut dyn PartialReflect) -> R,
    ) -> Option<R> {
        let path = ReflectPath::new(self.path());
        let target = match data.reflect_path_mut(path.as_str()) {
            Ok(v) => v,
            Err(e) => {
                warn!("binding: failed to resolve_mut '{}': {}", self.path(), e);
                return None;
            }
        };
        match &self.accessor {
            FieldAccessor::Direct(_) => Some(f(target)),
            FieldAccessor::VariantField { field_name, .. } => {
                with_variant_field_mut(target, field_name, f)
            }
        }
    }

    pub(super) fn read_value(&self, data: &dyn Reflect) -> FieldValue {
        let Some(value) = self.resolve_ref(data) else {
            return FieldValue::None;
        };
        reflect_to_field_value(value, &self.kind)
    }

    pub(super) fn write_value(&self, data: &mut dyn Reflect, value: &FieldValue) -> bool {
        self.with_resolved_mut(data, |target| apply_field_value_to_reflect(target, value))
            .unwrap_or(false)
    }

    pub fn set_enum_by_name(&self, data: &mut dyn Reflect, variant_name: &str) -> bool {
        self.with_resolved_mut(data, |target| {
            set_enum_variant_by_name(target, variant_name)
        })
        .unwrap_or(false)
    }

    pub fn set_optional_enum(
        &self,
        data: &mut dyn Reflect,
        inner_variant_name: Option<&str>,
    ) -> bool {
        self.with_resolved_mut(data, |target| {
            set_optional_enum_by_name(target, inner_variant_name)
        })
        .unwrap_or(false)
    }

    pub fn read_reflected<'a>(&self, data: &'a dyn Reflect) -> Option<&'a dyn PartialReflect> {
        self.resolve_ref(data)
    }

    pub fn write_reflected(
        &self,
        data: &mut dyn Reflect,
        f: impl FnOnce(&mut dyn PartialReflect),
    ) -> bool {
        self.with_resolved_mut(data, |target| {
            f(target);
        })
        .is_some()
    }

    pub fn path(&self) -> &str {
        match &self.accessor {
            FieldAccessor::Direct(path) => path,
            FieldAccessor::VariantField { path, .. } => path,
        }
    }

    pub fn field_name(&self) -> Option<&str> {
        match &self.accessor {
            FieldAccessor::Direct(_) => None,
            FieldAccessor::VariantField { field_name, .. } => Some(field_name),
        }
    }

    pub fn is_variant(&self) -> bool {
        matches!(self.accessor, FieldAccessor::VariantField { .. })
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
            FieldValue::F32(v) => match kind {
                FieldKind::F32Percent => {
                    let display = (v * 100.0 * 100.0).round() / 100.0;
                    Some(format_f32(display))
                }
                _ => Some(format_f32(*v)),
            },
            FieldValue::U32(v) => match kind {
                FieldKind::U32OrEmpty if *v == 0 => None,
                _ => Some(v.to_string()),
            },
            FieldValue::OptionalU32(v) => match (v, kind) {
                (None, _) => None,
                (Some(0), FieldKind::OptionalU32) => None,
                (Some(v), _) => Some(v.to_string()),
            },
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

fn apply_with_change_check(target: &mut dyn PartialReflect, value: &dyn PartialReflect) -> bool {
    if let Some(true) = target.reflect_partial_eq(value) {
        return false;
    }
    target.apply(value);
    true
}

pub(super) fn format_f32(v: f32) -> String {
    let mut text = v.to_string();
    if !text.contains('.') {
        text.push_str(".0");
    }
    text
}

pub(super) fn parse_field_value(text: &str, kind: &FieldKind) -> FieldValue {
    let text = text.trim();
    match kind {
        FieldKind::F32 | FieldKind::F32Percent => {
            let parsed: Option<f32> = match kind {
                FieldKind::F32Percent => text
                    .trim_end_matches('%')
                    .trim()
                    .parse()
                    .ok()
                    .map(|v: f32| v / 100.0),
                _ => text.trim_end_matches('s').trim().parse().ok(),
            };
            parsed.map(FieldValue::F32).unwrap_or(FieldValue::None)
        }
        FieldKind::U32 | FieldKind::U32OrEmpty => {
            let parsed: Option<u32> = if text.is_empty() && matches!(kind, FieldKind::U32OrEmpty) {
                Some(0)
            } else {
                text.parse().ok()
            };
            parsed.map(FieldValue::U32).unwrap_or(FieldValue::None)
        }
        FieldKind::OptionalU32 => {
            let parsed: Option<Option<u32>> = if text.is_empty() {
                Some(None)
            } else {
                text.parse::<u32>()
                    .ok()
                    .map(|v| if v == 0 { None } else { Some(v) })
            };
            parsed
                .map(FieldValue::OptionalU32)
                .unwrap_or(FieldValue::None)
        }
        _ => FieldValue::None,
    }
}

fn reflect_to_field_value(value: &dyn PartialReflect, kind: &FieldKind) -> FieldValue {
    if let FieldKind::ComboBox { optional: true, .. } = kind {
        if let Some(index) = read_optional_enum_index(value) {
            return FieldValue::U32(index as u32);
        }
        return FieldValue::None;
    }
    if let Some(v) = value.try_downcast_ref::<f32>() {
        return FieldValue::F32(*v);
    }
    if let Some(v) = value.try_downcast_ref::<u32>() {
        return FieldValue::U32(*v);
    }
    if let Some(v) = value.try_downcast_ref::<bool>() {
        return FieldValue::Bool(*v);
    }
    if let Some(v) = value.try_downcast_ref::<Vec2>() {
        return FieldValue::Vec2(*v);
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
    if let Some(v) = value.try_downcast_ref::<ParticleRange>() {
        return FieldValue::Range(v.min, v.max);
    }
    if let ReflectRef::Enum(enum_ref) = value.reflect_ref() {
        return FieldValue::U32(enum_ref.variant_index() as u32);
    }
    FieldValue::None
}

fn read_optional_enum_index(value: &dyn PartialReflect) -> Option<usize> {
    let ReflectRef::Enum(enum_ref) = value.reflect_ref() else {
        return None;
    };
    if enum_ref.variant_name() == "None" {
        return Some(0);
    }
    let inner = enum_ref.field_at(0)?;
    if let ReflectRef::Enum(inner_enum) = inner.reflect_ref() {
        Some(inner_enum.variant_index() + 1)
    } else {
        None
    }
}

fn apply_field_value_to_reflect(target: &mut dyn PartialReflect, value: &FieldValue) -> bool {
    match value {
        FieldValue::F32(v) => apply_with_change_check(target, v),
        FieldValue::U32(v) => apply_with_change_check(target, v),
        FieldValue::OptionalU32(v) => apply_with_change_check(target, v),
        FieldValue::Bool(v) => apply_with_change_check(target, v),
        FieldValue::Vec2(v) => apply_with_change_check(target, v),
        FieldValue::Vec3(v) => apply_with_change_check(target, v),
        FieldValue::Range(min, max) => apply_with_change_check(
            target,
            &ParticleRange {
                min: *min,
                max: *max,
            },
        ),
        FieldValue::Color(c) => apply_with_change_check(target, c),
        FieldValue::None => false,
    }
}

pub(super) fn get_variant_index_by_reflection(
    data: &dyn Reflect,
    path: &str,
    variants: &[VariantDefinition],
) -> Option<usize> {
    let reflect_path = ReflectPath::new(path);
    let value = data.reflect_path(reflect_path.as_str()).ok()?;

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

pub(super) fn find_ancestor_entity(
    entity: Entity,
    target: Entity,
    parents: &Query<&ChildOf>,
) -> bool {
    find_ancestor(entity, parents, MAX_ANCESTOR_DEPTH, |e| e == target).is_some()
}

pub(super) fn read_fixed_seed(data: &dyn Reflect) -> Option<u32> {
    let path = ReflectPath::new("time.fixed_seed");
    data.reflect_path(path.as_str())
        .ok()
        .and_then(|v| v.try_downcast_ref::<Option<u32>>().copied())
        .flatten()
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

#[derive(SystemParam)]
pub(crate) struct EmitterWriter<'w, 's> {
    editor_state: Res<'w, EditorState>,
    assets: ResMut<'w, Assets<ParticleSystemAsset>>,
    dirty_state: ResMut<'w, DirtyState>,
    emitter_runtimes: Query<'w, 's, &'static mut EmitterRuntime>,
}

impl EmitterWriter<'_, '_> {
    pub(crate) fn modify_emitter(&mut self, f: impl FnOnce(&mut EmitterData) -> bool) {
        let Some((_, emitter)) = get_inspecting_emitter_mut(&self.editor_state, &mut self.assets)
        else {
            return;
        };
        let fixed_seed = emitter.time.fixed_seed;
        if f(emitter) {
            mark_dirty_and_restart(
                &mut self.dirty_state,
                &mut self.emitter_runtimes,
                fixed_seed,
            );
        }
    }

    pub(crate) fn emitter(&self) -> Option<&EmitterData> {
        get_inspecting_emitter(&self.editor_state, &self.assets).map(|(_, e)| e)
    }
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

fn set_optional_enum_by_name(
    target: &mut dyn PartialReflect,
    inner_variant_name: Option<&str>,
) -> bool {
    let ReflectMut::Enum(enum_mut) = target.reflect_mut() else {
        return false;
    };

    let is_none = enum_mut.variant_name() == "None";

    match inner_variant_name {
        None => {
            if is_none {
                return false;
            }
            let dynamic_enum = DynamicEnum::new("None", DynamicVariant::Unit);
            target.apply(&dynamic_enum);
            true
        }
        Some(variant_name) => {
            if !is_none {
                if let ReflectRef::Enum(enum_ref) = target.reflect_ref() {
                    if let Some(inner) = enum_ref.field_at(0) {
                        if let ReflectRef::Enum(inner_enum) = inner.reflect_ref() {
                            if inner_enum.variant_name() == variant_name {
                                return false;
                            }
                        }
                    }
                }
            }
            let inner = DynamicEnum::new(variant_name, DynamicVariant::Unit);
            let mut tuple = DynamicTuple::default();
            tuple.insert(inner);
            let some = DynamicEnum::new("Some", DynamicVariant::Tuple(tuple));
            target.apply(&some);
            true
        }
    }
}
