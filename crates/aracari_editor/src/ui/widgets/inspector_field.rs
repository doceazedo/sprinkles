use bevy::prelude::*;

use super::checkbox::{CheckboxProps, checkbox};
use super::combobox::{ComboBoxOptionData, combobox};
use super::curve_edit::{CurveEditProps, curve_edit};
use super::text_edit::{TextEditPrefix, TextEditProps, text_edit};
use super::vector_edit::{VectorEditProps, VectorSuffixes, vector_edit};
use crate::ui::components::binding::Field;
use crate::ui::components::inspector::{FieldKind, path_to_label};

pub fn plugin(app: &mut App) {
    app.add_systems(Update, setup_combobox_fields);
}

pub struct InspectorFieldProps {
    path: String,
    kind: FieldKind,
    label: Option<String>,
    icon: Option<String>,
    suffix: Option<String>,
    placeholder: Option<String>,
    min: Option<f32>,
    max: Option<f32>,
    combobox_options: Option<Vec<ComboBoxOptionData>>,
}

impl InspectorFieldProps {
    pub fn new(path: impl Into<String>) -> Self {
        Self {
            path: path.into(),
            kind: FieldKind::F32,
            label: None,
            icon: None,
            suffix: None,
            placeholder: None,
            min: None,
            max: None,
            combobox_options: None,
        }
    }

    pub fn percent(mut self) -> Self {
        self.kind = FieldKind::F32Percent;
        self
    }

    pub fn u32(mut self) -> Self {
        self.kind = FieldKind::U32;
        self
    }

    pub fn u32_or_empty(mut self) -> Self {
        self.kind = FieldKind::U32OrEmpty;
        self
    }

    pub fn optional_u32(mut self) -> Self {
        self.kind = FieldKind::OptionalU32;
        self
    }

    pub fn bool(mut self) -> Self {
        self.kind = FieldKind::Bool;
        self
    }

    pub fn vector(mut self, suffixes: VectorSuffixes) -> Self {
        self.kind = FieldKind::Vector(suffixes);
        self
    }

    pub fn curve(mut self) -> Self {
        self.kind = FieldKind::Curve;
        self
    }

    pub fn combobox(mut self, options: Vec<ComboBoxOptionData>) -> Self {
        let option_labels: Vec<String> = options.iter().map(|o| o.label.clone()).collect();
        self.kind = FieldKind::ComboBox { options: option_labels };
        self.combobox_options = Some(options);
        self
    }

    pub fn with_icon(mut self, path: impl Into<String>) -> Self {
        self.icon = Some(path.into());
        self
    }

    pub fn with_label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    pub fn with_suffix(mut self, suffix: impl Into<String>) -> Self {
        self.suffix = Some(suffix.into());
        self
    }

    pub fn with_placeholder(mut self, placeholder: impl Into<String>) -> Self {
        self.placeholder = Some(placeholder.into());
        self
    }

    pub fn with_min(mut self, min: f32) -> Self {
        self.min = Some(min);
        self
    }

    pub fn with_max(mut self, max: f32) -> Self {
        self.max = Some(max);
        self
    }

    fn inferred_label(&self) -> String {
        self.label
            .clone()
            .unwrap_or_else(|| path_to_label(&self.path))
    }

    fn inferred_suffix(&self) -> Option<&str> {
        if self.suffix.is_some() {
            return self.suffix.as_deref();
        }
        match self.kind {
            FieldKind::F32Percent => Some("%"),
            _ => None,
        }
    }

    fn inferred_min(&self) -> Option<f32> {
        if self.min.is_some() {
            return self.min;
        }
        match self.kind {
            FieldKind::F32Percent | FieldKind::U32 | FieldKind::U32OrEmpty | FieldKind::OptionalU32 => Some(0.0),
            _ => None,
        }
    }

    fn inferred_max(&self) -> Option<f32> {
        if self.max.is_some() {
            return self.max;
        }
        match self.kind {
            FieldKind::F32Percent => Some(100.0),
            _ => None,
        }
    }

    fn should_allow_empty(&self) -> bool {
        matches!(self.kind, FieldKind::U32OrEmpty | FieldKind::OptionalU32)
    }

    fn is_integer(&self) -> bool {
        matches!(
            self.kind,
            FieldKind::U32 | FieldKind::U32OrEmpty | FieldKind::OptionalU32
        )
    }
}

pub fn spawn_inspector_field(
    spawner: &mut ChildSpawnerCommands,
    props: InspectorFieldProps,
    asset_server: &AssetServer,
) {
    let field = Field::new(&props.path).with_kind(props.kind.clone());
    let label = props.inferred_label();

    if props.kind == FieldKind::Bool {
        spawner.spawn((field, checkbox(CheckboxProps::new(label), asset_server)));
        return;
    }

    if let FieldKind::Vector(suffixes) = props.kind {
        spawner.spawn((
            field,
            vector_edit(
                VectorEditProps::default()
                    .with_label(label)
                    .with_size(suffixes.vector_size())
                    .with_suffixes(suffixes),
            ),
        ));
        return;
    }

    if props.kind == FieldKind::Curve {
        spawner.spawn((
            field,
            curve_edit(CurveEditProps::new().with_label(label)),
        ));
        return;
    }

    if let Some(options) = props.combobox_options {
        spawner.spawn((
            field,
            combobox_field(label, options),
        ));
        return;
    }

    let mut text_props = TextEditProps::default().with_label(label);

    if props.is_integer() {
        text_props = text_props.numeric_i32();
    } else {
        text_props = text_props.numeric_f32();
    }

    if let Some(suffix) = props.inferred_suffix() {
        text_props = text_props.with_suffix(suffix);
    }

    if let Some(ref placeholder) = props.placeholder {
        text_props = text_props.with_placeholder(placeholder);
    }

    if let Some(ref icon) = props.icon {
        text_props = text_props.with_prefix(TextEditPrefix::Icon { path: icon.clone() });
    }

    if let Some(min) = props.inferred_min() {
        text_props = text_props.with_min(min as f64);
    }

    if let Some(max) = props.inferred_max() {
        text_props = text_props.with_max(max as f64);
    }

    if props.should_allow_empty() {
        text_props = text_props.allow_empty();
    }

    spawner.spawn((field, text_edit(text_props)));
}

#[derive(Component)]
struct ComboBoxFieldConfig {
    label: String,
    options: Vec<ComboBoxOptionData>,
    initialized: bool,
}

fn combobox_field(label: String, options: Vec<ComboBoxOptionData>) -> impl Bundle {
    (
        ComboBoxFieldConfig {
            label,
            options,
            initialized: false,
        },
        Node {
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(3.0),
            flex_grow: 1.0,
            flex_shrink: 1.0,
            flex_basis: Val::Px(0.0),
            ..default()
        },
    )
}

pub fn setup_combobox_fields(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut configs: Query<(Entity, &mut ComboBoxFieldConfig)>,
) {
    let font: Handle<Font> = asset_server.load(crate::ui::tokens::FONT_PATH);

    for (entity, mut config) in &mut configs {
        if config.initialized {
            continue;
        }
        config.initialized = true;

        let label_entity = commands
            .spawn((
                Text::new(&config.label),
                TextFont {
                    font: font.clone(),
                    font_size: 11.0,
                    weight: FontWeight::MEDIUM,
                    ..default()
                },
                TextColor(crate::ui::tokens::TEXT_MUTED_COLOR.into()),
            ))
            .id();

        let combobox_entity = commands.spawn(combobox(config.options.clone())).id();

        commands
            .entity(entity)
            .add_children(&[label_entity, combobox_entity]);
    }
}

pub fn fields_row() -> impl Bundle {
    Node {
        width: Val::Percent(100.0),
        column_gap: Val::Px(12.0),
        ..default()
    }
}
