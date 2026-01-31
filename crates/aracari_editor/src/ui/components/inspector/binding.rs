use std::collections::HashMap;

use aracari::prelude::*;
use bevy::ecs::system::ParamSet;
use bevy::input_focus::InputFocus;
use bevy::prelude::*;
use bevy::reflect::GetPath;
use bevy_ui_text_input::{
    TextInputBuffer, TextInputQueue,
    actions::{TextInputAction, TextInputEdit},
};

use crate::state::{DirtyState, EditorState, Inspectable};
use crate::ui::widgets::checkbox::CheckboxState;
use crate::ui::widgets::text_edit::EditorTextEdit;

pub fn plugin(app: &mut App) {
    app.init_resource::<BoundEmitter>().add_systems(
        Update,
        (
            bind_values_to_inputs,
            sync_input_on_blur,
            sync_checkbox_changes_to_asset,
        ),
    );
}

#[derive(Resource, Default)]
struct BoundEmitter(Option<u8>);

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum FieldKind {
    #[default]
    F32,
    F32Percent,
    U32,
    U32OrEmpty,
    OptionalU32,
    Bool,
    VariantEdit,
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

    fn reflect_path(&self) -> String {
        format!(".{}", self.path)
    }
}

fn get_emitter_field_value(emitter: &EmitterData, field: &Field) -> FieldValue {
    let path = field.reflect_path();

    let value = match field.kind {
        FieldKind::F32 | FieldKind::F32Percent => emitter
            .path::<f32>(path.as_str())
            .ok()
            .map(|v| FieldValue::F32(*v))
            .unwrap_or(FieldValue::None),
        FieldKind::U32 | FieldKind::U32OrEmpty => emitter
            .path::<u32>(path.as_str())
            .ok()
            .map(|v| FieldValue::U32(*v))
            .unwrap_or(FieldValue::None),
        FieldKind::OptionalU32 => emitter
            .path::<Option<u32>>(path.as_str())
            .ok()
            .map(|v| FieldValue::OptionalU32(*v))
            .unwrap_or(FieldValue::None),
        FieldKind::Bool => emitter
            .path::<bool>(path.as_str())
            .ok()
            .map(|v| FieldValue::Bool(*v))
            .unwrap_or(FieldValue::None),
        FieldKind::VariantEdit => FieldValue::None,
    };

    value.with_kind(field.kind)
}

fn set_emitter_field_value(emitter: &mut EmitterData, field: &Field, value: FieldValue) -> bool {
    let path = field.reflect_path();

    match value {
        FieldValue::F32(v) => {
            if let Ok(current) = emitter.path::<f32>(path.as_str()) {
                if (*current - v).abs() > f32::EPSILON {
                    if let Ok(field_mut) = emitter.path_mut::<f32>(path.as_str()) {
                        *field_mut = v;
                        return true;
                    }
                }
            }
        }
        FieldValue::U32(v) => {
            if let Ok(current) = emitter.path::<u32>(path.as_str()) {
                if *current != v {
                    if let Ok(field_mut) = emitter.path_mut::<u32>(path.as_str()) {
                        *field_mut = v;
                        return true;
                    }
                }
            }
        }
        FieldValue::OptionalU32(v) => {
            if let Ok(current) = emitter.path::<Option<u32>>(path.as_str()) {
                if *current != v {
                    if let Ok(field_mut) = emitter.path_mut::<Option<u32>>(path.as_str()) {
                        *field_mut = v;
                        return true;
                    }
                }
            }
        }
        FieldValue::Bool(v) => {
            if let Ok(current) = emitter.path::<bool>(path.as_str()) {
                if *current != v {
                    if let Ok(field_mut) = emitter.path_mut::<bool>(path.as_str()) {
                        *field_mut = v;
                        return true;
                    }
                }
            }
        }
        FieldValue::None => {}
    }
    false
}

#[derive(Clone)]
enum FieldValue {
    None,
    F32(f32),
    U32(u32),
    OptionalU32(Option<u32>),
    Bool(bool),
}

impl FieldValue {
    fn with_kind(self, kind: FieldKind) -> Self {
        match (self, kind) {
            (FieldValue::F32(v), FieldKind::F32Percent) => {
                FieldValue::F32((v * 100.0 * 100.0).round() / 100.0)
            }
            (FieldValue::U32(v), FieldKind::U32OrEmpty) if v == 0 => FieldValue::None,
            (FieldValue::OptionalU32(None), _) => FieldValue::None,
            (FieldValue::OptionalU32(Some(0)), FieldKind::OptionalU32) => FieldValue::None,
            (FieldValue::OptionalU32(Some(v)), _) => FieldValue::U32(v),
            (other, _) => other,
        }
    }

    fn to_display_string(&self) -> Option<String> {
        match self {
            FieldValue::None => None,
            FieldValue::F32(v) => {
                let mut text = v.to_string();
                if !text.contains('.') {
                    text.push_str(".0");
                }
                Some(text)
            }
            FieldValue::U32(v) => Some(v.to_string()),
            FieldValue::OptionalU32(Some(v)) => Some(v.to_string()),
            FieldValue::OptionalU32(None) => None,
            FieldValue::Bool(_) => None,
        }
    }

    fn to_bool(&self) -> Option<bool> {
        match self {
            FieldValue::Bool(v) => Some(*v),
            _ => None,
        }
    }
}

fn parse_field_value(text: &str, kind: FieldKind) -> FieldValue {
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
        FieldKind::U32 => text
            .parse::<u32>()
            .map(FieldValue::U32)
            .unwrap_or(FieldValue::None),
        FieldKind::U32OrEmpty => {
            if text.is_empty() {
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
        FieldKind::Bool => FieldValue::None,
        FieldKind::VariantEdit => FieldValue::None,
    }
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
) {
    let inspecting = match &editor_state.inspecting {
        Some(i) if i.kind == Inspectable::Emitter => i,
        _ => {
            bound_emitter.0 = None;
            return;
        }
    };

    let emitter_changed = bound_emitter.0 != Some(inspecting.index);
    let has_new_checkboxes = !checkbox_set.p0().is_empty();
    let has_new_fields = !new_fields.is_empty() || !new_text_edits.is_empty();
    let should_rebind = emitter_changed || has_new_fields || has_new_checkboxes;

    if !should_rebind {
        return;
    }

    let Some(handle) = &editor_state.current_project else {
        return;
    };
    let Some(asset) = assets.get(handle) else {
        return;
    };
    let Some(emitter) = asset.emitters.get(inspecting.index as usize) else {
        return;
    };

    bound_emitter.0 = Some(inspecting.index);

    for (entity, child_of, mut queue) in &mut text_edits {
        let Some(field) = find_ancestor_field(child_of.parent(), &fields, &parents) else {
            continue;
        };

        let value = get_emitter_field_value(emitter, field);
        let text = value.to_display_string().unwrap_or_default();

        queue.add(TextInputAction::Edit(TextInputEdit::SelectAll));
        queue.add(TextInputAction::Edit(TextInputEdit::Paste(text)));
        commands.entity(entity).insert(FieldBound);
    }

    for (entity, mut state) in &mut checkbox_set.p1() {
        let field = if let Ok(f) = fields.get(entity) {
            f
        } else if let Ok(child_of) = parents.get(entity) {
            match find_ancestor_field(child_of.parent(), &fields, &parents) {
                Some(f) => f,
                None => continue,
            }
        } else {
            continue;
        };

        let value = get_emitter_field_value(emitter, field);
        if let Some(checked) = value.to_bool() {
            state.checked = checked;
        }
        commands.entity(entity).insert(CheckboxBound);
    }
}

fn find_ancestor_field<'a>(
    mut entity: Entity,
    fields: &'a Query<&Field>,
    parents: &Query<&ChildOf>,
) -> Option<&'a Field> {
    for _ in 0..10 {
        if let Ok(field) = fields.get(entity) {
            return Some(field);
        }
        entity = parents.get(entity).ok()?.parent();
    }
    None
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

    let inspecting = match &editor_state.inspecting {
        Some(i) if i.kind == Inspectable::Emitter => i,
        _ => return,
    };

    let Some(handle) = &editor_state.current_project else {
        return;
    };
    let Some(asset) = assets.get_mut(handle) else {
        return;
    };
    let Some(emitter) = asset.emitters.get_mut(inspecting.index as usize) else {
        return;
    };

    let Some(field) = find_ancestor_field(child_of.parent(), &fields, &parents) else {
        return;
    };

    let text = buffer.get_text();
    let value = parse_field_value(&text, field.kind);

    if matches!(value, FieldValue::None) {
        return;
    }

    if set_emitter_field_value(emitter, field, value) {
        dirty_state.has_unsaved_changes = true;
        for mut runtime in &mut emitter_runtimes {
            runtime.restart(emitter.time.fixed_seed);
        }
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
    let inspecting = match &editor_state.inspecting {
        Some(i) if i.kind == Inspectable::Emitter => i,
        _ => return,
    };

    let Some(handle) = &editor_state.current_project else {
        return;
    };
    let Some(asset) = assets.get_mut(handle) else {
        return;
    };
    let Some(emitter) = asset.emitters.get_mut(inspecting.index as usize) else {
        return;
    };

    for (entity, state) in &changed_checkboxes {
        let last_state = last_checkbox_states.get(&entity).copied();
        last_checkbox_states.insert(entity, state.checked);
        if last_state.is_none() {
            continue;
        }

        let field = if let Ok(f) = fields.get(entity) {
            f
        } else if let Ok(child_of) = parents.get(entity) {
            match find_ancestor_field(child_of.parent(), &fields, &parents) {
                Some(f) => f,
                None => continue,
            }
        } else {
            continue;
        };

        if set_emitter_field_value(emitter, field, FieldValue::Bool(state.checked)) {
            dirty_state.has_unsaved_changes = true;
            for mut runtime in &mut emitter_runtimes {
                runtime.restart(emitter.time.fixed_seed);
            }
        }
    }
}
