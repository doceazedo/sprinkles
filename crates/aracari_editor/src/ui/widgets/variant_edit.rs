use bevy::prelude::*;
use inflector::Inflector;

use crate::ui::tokens::{BORDER_COLOR, FONT_PATH, TEXT_MUTED_COLOR, TEXT_SIZE_SM};
use crate::ui::widgets::button::{
    ButtonClickEvent, ButtonProps, ButtonVariant, EditorButton, button, set_button_variant,
};
use crate::ui::widgets::checkbox::{CheckboxProps, checkbox};
use crate::ui::widgets::combobox::{
    ComboBoxChangeEvent, ComboBoxOptionData, combobox, combobox_with_selected,
};
use crate::ui::widgets::popover::{
    EditorPopover, PopoverHeaderProps, PopoverPlacement, PopoverProps, popover, popover_content,
    popover_header,
};
use crate::ui::widgets::text_edit::{TextEditProps, text_edit};
use crate::ui::widgets::vector_edit::{VectorEditProps, VectorSuffixes, vector_edit};

const ICON_MORE: &str = "icons/ri-more-fill.png";

#[derive(Clone, Debug)]
pub enum VariantFieldKind {
    F32,
    U32,
    Bool,
    Vec3(VectorSuffixes),
    ComboBox { options: Vec<String> },
}

#[derive(Clone, Debug)]
pub struct VariantField {
    pub name: String,
    pub kind: VariantFieldKind,
}

impl VariantField {
    pub fn f32(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            kind: VariantFieldKind::F32,
        }
    }

    pub fn u32(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            kind: VariantFieldKind::U32,
        }
    }

    pub fn bool(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            kind: VariantFieldKind::Bool,
        }
    }

    pub fn vec3(name: impl Into<String>, suffixes: VectorSuffixes) -> Self {
        Self {
            name: name.into(),
            kind: VariantFieldKind::Vec3(suffixes),
        }
    }

    pub fn combobox(name: impl Into<String>, options: Vec<impl Into<String>>) -> Self {
        Self {
            name: name.into(),
            kind: VariantFieldKind::ComboBox {
                options: options.into_iter().map(Into::into).collect(),
            },
        }
    }
}

pub struct VariantDefinition {
    pub name: String,
    pub icon: Option<String>,
    pub fields: Vec<VariantField>,
    default_value: Option<Box<dyn PartialReflect>>,
}

impl Clone for VariantDefinition {
    fn clone(&self) -> Self {
        Self {
            name: self.name.clone(),
            icon: self.icon.clone(),
            fields: self.fields.clone(),
            default_value: self
                .default_value
                .as_ref()
                .and_then(|v| v.as_ref().reflect_clone().ok())
                .map(|v| v.into_partial_reflect()),
        }
    }
}

impl std::fmt::Debug for VariantDefinition {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("VariantDefinition")
            .field("name", &self.name)
            .field("icon", &self.icon)
            .field("fields", &self.fields)
            .field("default_value", &self.default_value.is_some())
            .finish()
    }
}

impl VariantDefinition {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            icon: None,
            fields: Vec::new(),
            default_value: None,
        }
    }

    pub fn with_icon(mut self, icon: impl Into<String>) -> Self {
        self.icon = Some(icon.into());
        self
    }

    pub fn with_fields(mut self, fields: Vec<VariantField>) -> Self {
        self.fields = fields;
        self
    }

    pub fn with_default<T: PartialReflect + Clone + 'static>(mut self, value: T) -> Self {
        self.default_value = Some(Box::new(value));
        self
    }

    pub fn create_default(&self) -> Option<Box<dyn PartialReflect>> {
        self.default_value
            .as_ref()
            .and_then(|v| v.as_ref().reflect_clone().ok())
            .map(|v| v.into_partial_reflect())
    }
}

pub fn plugin(app: &mut App) {
    app.add_observer(handle_variant_edit_click)
        .add_observer(handle_variant_combobox_change)
        .add_systems(
            Update,
            (setup_variant_edit, sync_variant_edit_button, handle_popover_closed),
        );
}

#[derive(Component)]
pub struct EditorVariantEdit;

#[derive(Component, Clone)]
pub struct VariantEditConfig {
    pub path: String,
    pub label: Option<String>,
    pub popover_title: Option<String>,
    pub variants: Vec<VariantDefinition>,
    pub selected_index: usize,
    initialized: bool,
}

#[derive(Component)]
struct VariantEditPopover(Entity);

#[derive(Component)]
struct VariantFieldsContainer(Entity);

#[derive(Component)]
pub struct VariantComboBox(pub Entity);

#[derive(Component)]
pub struct VariantFieldBinding {
    pub variant_edit: Entity,
    pub field_name: String,
    pub field_kind: VariantFieldKind,
}

#[derive(Component, Default)]
struct VariantEditState {
    popover: Option<Entity>,
    last_synced_index: Option<usize>,
}

const UPPERCASE_ACRONYMS: &[&str] = &["fps"];

fn path_to_label(path: &str) -> String {
    let field_name = path.split('.').last().unwrap_or(path);
    let sentence = field_name.to_sentence_case();

    sentence
        .split_whitespace()
        .map(|word| {
            let lower = word.to_lowercase();
            if UPPERCASE_ACRONYMS.contains(&lower.as_str()) {
                lower.to_uppercase()
            } else {
                word.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

pub struct VariantEditProps {
    pub path: String,
    pub label: Option<String>,
    pub popover_title: Option<String>,
    pub variants: Vec<VariantDefinition>,
    pub selected_index: usize,
}

impl VariantEditProps {
    pub fn new(path: impl Into<String>) -> Self {
        Self {
            path: path.into(),
            label: None,
            popover_title: None,
            variants: Vec::new(),
            selected_index: 0,
        }
    }

    pub fn with_label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    pub fn with_popover_title(mut self, title: impl Into<String>) -> Self {
        self.popover_title = Some(title.into());
        self
    }

    pub fn with_variants(mut self, variants: Vec<VariantDefinition>) -> Self {
        self.variants = variants;
        self
    }

    pub fn with_selected(mut self, index: usize) -> Self {
        self.selected_index = index;
        self
    }
}

pub fn variant_edit(props: VariantEditProps) -> impl Bundle {
    let VariantEditProps {
        path,
        label,
        popover_title,
        variants,
        selected_index,
    } = props;

    (
        EditorVariantEdit,
        VariantEditConfig {
            path,
            label,
            popover_title,
            variants,
            selected_index,
            initialized: false,
        },
        VariantEditState::default(),
        Node {
            flex_direction: FlexDirection::Column,
            row_gap: px(3.0),
            flex_grow: 1.0,
            flex_shrink: 1.0,
            flex_basis: px(0.0),
            ..default()
        },
    )
}

#[derive(Component)]
struct VariantEditButton;

fn setup_variant_edit(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut configs: Query<(Entity, &mut VariantEditConfig)>,
) {
    let font: Handle<Font> = asset_server.load(FONT_PATH);

    for (entity, mut config) in &mut configs {
        if config.initialized {
            continue;
        }
        config.initialized = true;

        let label = config
            .label
            .clone()
            .unwrap_or_else(|| path_to_label(&config.path));

        let label_entity = commands
            .spawn((
                Text::new(&label),
                TextFont {
                    font: font.clone(),
                    font_size: TEXT_SIZE_SM,
                    weight: FontWeight::MEDIUM,
                    ..default()
                },
                TextColor(TEXT_MUTED_COLOR.into()),
            ))
            .id();
        commands.entity(entity).add_child(label_entity);

        let selected_variant = config.variants.get(config.selected_index);
        let value = selected_variant
            .map(|v| v.name.clone())
            .unwrap_or_default();
        let icon = selected_variant.and_then(|v| v.icon.clone());

        let mut button_props = ButtonProps::new(&value)
            .align_left()
            .with_right_icon(ICON_MORE);

        if let Some(ref icon_path) = icon {
            button_props = button_props.with_left_icon(icon_path);
        }

        let button_entity = commands
            .spawn((VariantEditButton, button(button_props)))
            .id();

        commands.entity(entity).add_child(button_entity);
    }
}

fn sync_variant_edit_button(
    asset_server: Res<AssetServer>,
    mut variant_edits: Query<
        (&VariantEditConfig, &mut VariantEditState, &Children),
        With<EditorVariantEdit>,
    >,
    children_query: Query<&Children>,
    mut texts: Query<&mut Text>,
    mut images: Query<&mut ImageNode>,
) {
    for (config, mut state, children) in &mut variant_edits {
        if state.last_synced_index == Some(config.selected_index) {
            continue;
        }

        let Some(selected_variant) = config.variants.get(config.selected_index) else {
            continue;
        };

        // the button is the last child (label is first)
        let Some(&button_entity) = children.last() else {
            continue;
        };
        let Ok(button_children) = children_query.get(button_entity) else {
            continue;
        };

        // only mark as synced after we've confirmed the button children exist
        let mut text_updated = false;
        let mut icon_updated = false;
        for child in button_children.iter() {
            if let Ok(mut text) = texts.get_mut(child) {
                **text = selected_variant.name.clone();
                text_updated = true;
            }
            if !icon_updated {
                if let Ok(mut image) = images.get_mut(child) {
                    if let Some(ref icon_path) = selected_variant.icon {
                        image.image = asset_server.load(icon_path);
                        icon_updated = true;
                    }
                }
            }
        }

        // only update last_synced_index if we actually updated the button text
        if text_updated {
            state.last_synced_index = Some(config.selected_index);
        }
    }
}

fn handle_variant_edit_click(
    trigger: On<ButtonClickEvent>,
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    buttons: Query<&ChildOf, With<EditorButton>>,
    variant_edit_buttons: Query<&ChildOf, With<VariantEditButton>>,
    mut variant_edits: Query<
        (Entity, &mut VariantEditState, &VariantEditConfig, &Children),
        With<EditorVariantEdit>,
    >,
    existing_popovers: Query<Entity, With<VariantEditPopover>>,
    all_popovers: Query<Entity, With<EditorPopover>>,
    mut button_styles: Query<(&mut BackgroundColor, &mut BorderColor, &mut ButtonVariant)>,
    parents: Query<&ChildOf>,
) {
    let Ok(child_of) = buttons.get(trigger.entity) else {
        return;
    };

    let variant_edit_entity = if let Ok(button_child_of) = variant_edit_buttons.get(child_of.parent()) {
        button_child_of.parent()
    } else {
        child_of.parent()
    };

    let Ok((entity, mut state, config, children)) = variant_edits.get_mut(variant_edit_entity) else {
        return;
    };

    // toggle behavior: if this variant edit's popover is already open, close it
    if let Some(popover_entity) = state.popover {
        if existing_popovers.get(popover_entity).is_ok() {
            commands.entity(popover_entity).try_despawn();
            state.popover = None;
            if let Some(&button_entity) = children.last() {
                if let Ok((mut bg, mut border, mut variant)) = button_styles.get_mut(button_entity)
                {
                    *variant = ButtonVariant::Default;
                    set_button_variant(ButtonVariant::Default, &mut bg, &mut border);
                }
            }
            return;
        }
    }

    // check if any other popover is open - if so, don't open a new one unless nested
    let any_popover_open = !all_popovers.is_empty();
    if any_popover_open {
        let is_nested = all_popovers
            .iter()
            .any(|popover| is_descendant_of(entity, popover, &parents));
        if !is_nested {
            return;
        }
    }

    if let Some(&button_entity) = children.last() {
        if let Ok((mut bg, mut border, mut variant)) = button_styles.get_mut(button_entity) {
            *variant = ButtonVariant::ActiveAlt;
            set_button_variant(ButtonVariant::ActiveAlt, &mut bg, &mut border);
        }
    }

    let popover_title = config
        .popover_title
        .clone()
        .or_else(|| config.label.clone())
        .unwrap_or_else(|| path_to_label(&config.path));

    let options: Vec<ComboBoxOptionData> = config
        .variants
        .iter()
        .map(|v| {
            let mut opt = ComboBoxOptionData::new(&v.name);
            if let Some(ref icon) = v.icon {
                opt = opt.with_icon(icon);
            }
            opt
        })
        .collect();

    let selected_variant = config.variants.get(config.selected_index);
    let has_fields = selected_variant
        .map(|v| !v.fields.is_empty())
        .unwrap_or(false);

    let popover_entity = commands
        .spawn((
            VariantEditPopover(entity),
            popover(
                PopoverProps::new(trigger.entity)
                    .with_placement(PopoverPlacement::Left)
                    .with_padding(0.0)
                    .with_node(Node {
                        width: px(256.0),
                        ..default()
                    }),
            ),
        ))
        .id();

    commands
        .entity(popover_entity)
        .with_child(popover_header(
            PopoverHeaderProps::new(popover_title, popover_entity),
            &asset_server,
        ))
        .with_children(|parent| {
            parent
                .spawn((
                    Node {
                        width: percent(100),
                        padding: UiRect::all(px(12.0)),
                        border: if has_fields {
                            UiRect::bottom(px(1.0))
                        } else {
                            UiRect::ZERO
                        },
                        ..default()
                    },
                    BorderColor::all(BORDER_COLOR),
                ))
                .with_child((
                    VariantComboBox(entity),
                    combobox_with_selected(options, config.selected_index),
                ));

            if has_fields {
                let fields_container = parent
                    .spawn((
                        VariantFieldsContainer(entity),
                        popover_content(),
                    ))
                    .id();

                if let Some(variant) = selected_variant {
                    let mut cmds = parent.commands();
                    spawn_variant_fields_for_entity(
                        &mut cmds,
                        fields_container,
                        entity,
                        &variant.fields,
                        &asset_server,
                    );
                }
            }
        });

    state.popover = Some(popover_entity);
}

fn handle_variant_combobox_change(
    trigger: On<ComboBoxChangeEvent>,
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    variant_comboboxes: Query<&VariantComboBox>,
    mut variant_edits: Query<&mut VariantEditConfig, With<EditorVariantEdit>>,
    fields_containers: Query<(Entity, &VariantFieldsContainer)>,
    variant_edit_children: Query<&Children, With<EditorVariantEdit>>,
    mut texts: Query<&mut Text>,
    mut images: Query<&mut ImageNode>,
    children_query: Query<&Children>,
) {
    let combobox_entity = trigger.entity;

    let Ok(variant_combobox) = variant_comboboxes.get(combobox_entity) else {
        return;
    };

    let variant_edit_entity = variant_combobox.0;
    let Ok(mut config) = variant_edits.get_mut(variant_edit_entity) else {
        return;
    };

    let new_index = trigger.selected;
    if new_index == config.selected_index {
        return;
    }

    config.selected_index = new_index;

    let Some(selected_variant) = config.variants.get(new_index).cloned() else {
        return;
    };

    // update the button text and icon
    if let Ok(children) = variant_edit_children.get(variant_edit_entity) {
        if let Some(&button_entity) = children.last() {
            if let Ok(button_children) = children_query.get(button_entity) {
                let mut icon_updated = false;
                for child in button_children.iter() {
                    if let Ok(mut text) = texts.get_mut(child) {
                        **text = selected_variant.name.clone();
                    }
                    if !icon_updated {
                        if let Ok(mut image) = images.get_mut(child) {
                            if let Some(ref icon_path) = selected_variant.icon {
                                image.image = asset_server.load(icon_path);
                                icon_updated = true;
                            }
                        }
                    }
                }
            }
        }
    }

    // find and update the fields container
    for (container_entity, container) in &fields_containers {
        if container.0 != variant_edit_entity {
            continue;
        }

        // despawn old children
        if let Ok(children) = children_query.get(container_entity) {
            for child in children.iter() {
                commands.entity(child).try_despawn();
            }
        }

        // spawn new fields
        spawn_variant_fields_for_entity(
            &mut commands,
            container_entity,
            variant_edit_entity,
            &selected_variant.fields,
            &asset_server,
        );

        break;
    }
}

fn spawn_variant_fields_for_entity(
    commands: &mut Commands,
    container: Entity,
    variant_edit: Entity,
    fields: &[VariantField],
    asset_server: &AssetServer,
) {
    let font: Handle<Font> = asset_server.load(FONT_PATH);

    for field in fields {
        let label = path_to_label(&field.name);
        let binding = VariantFieldBinding {
            variant_edit,
            field_name: field.name.clone(),
            field_kind: field.kind.clone(),
        };

        match &field.kind {
            VariantFieldKind::F32 => {
                let row = commands
                    .spawn(fields_row())
                    .with_child((
                        binding,
                        text_edit(TextEditProps::default().with_label(label).numeric_f32()),
                    ))
                    .id();
                commands.entity(container).add_child(row);
            }
            VariantFieldKind::U32 => {
                let row = commands
                    .spawn(fields_row())
                    .with_child((
                        binding,
                        text_edit(TextEditProps::default().with_label(label).numeric_i32()),
                    ))
                    .id();
                commands.entity(container).add_child(row);
            }
            VariantFieldKind::Bool => {
                let row = commands
                    .spawn(fields_row())
                    .with_child((binding, checkbox(CheckboxProps::new(label), asset_server)))
                    .id();
                commands.entity(container).add_child(row);
            }
            VariantFieldKind::Vec3(suffixes) => {
                let row = commands
                    .spawn((
                        binding,
                        vector_edit(
                            VectorEditProps::default()
                                .with_label(label)
                                .with_suffixes(*suffixes),
                        ),
                    ))
                    .id();
                commands.entity(container).add_child(row);
            }
            VariantFieldKind::ComboBox { options } => {
                let combobox_options: Vec<ComboBoxOptionData> =
                    options.iter().map(|o| ComboBoxOptionData::new(o)).collect();
                let row = commands
                    .spawn(Node {
                        flex_direction: FlexDirection::Column,
                        row_gap: px(3.0),
                        ..default()
                    })
                    .with_child((
                        Text::new(label),
                        TextFont {
                            font: font.clone(),
                            font_size: TEXT_SIZE_SM,
                            weight: FontWeight::MEDIUM,
                            ..default()
                        },
                        TextColor(TEXT_MUTED_COLOR.into()),
                    ))
                    .with_child((binding, combobox(combobox_options)))
                    .id();
                commands.entity(container).add_child(row);
            }
        }
    }
}

fn fields_row() -> impl Bundle {
    Node {
        width: Val::Percent(100.0),
        column_gap: Val::Px(8.0),
        ..default()
    }
}

fn is_descendant_of(entity: Entity, ancestor: Entity, parents: &Query<&ChildOf>) -> bool {
    let mut current = entity;
    for _ in 0..50 {
        if current == ancestor {
            return true;
        }
        if let Ok(child_of) = parents.get(current) {
            current = child_of.parent();
        } else {
            return false;
        }
    }
    false
}

fn handle_popover_closed(
    mut variant_edits: Query<(&mut VariantEditState, &Children), With<EditorVariantEdit>>,
    popovers: Query<Entity, With<EditorPopover>>,
    mut button_styles: Query<(&mut BackgroundColor, &mut BorderColor, &mut ButtonVariant)>,
) {
    for (mut state, children) in &mut variant_edits {
        let Some(popover_entity) = state.popover else {
            continue;
        };

        if popovers.get(popover_entity).is_ok() {
            continue;
        }

        state.popover = None;

        if let Some(&button_entity) = children.last() {
            if let Ok((mut bg, mut border, mut variant)) = button_styles.get_mut(button_entity) {
                *variant = ButtonVariant::Default;
                set_button_variant(ButtonVariant::Default, &mut bg, &mut border);
            }
        }
    }
}
