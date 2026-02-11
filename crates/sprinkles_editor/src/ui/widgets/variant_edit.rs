use bevy::prelude::*;
use bevy::reflect::{TypeInfo, Typed, VariantInfo};

use crate::ui::components::binding::FieldBinding;
use crate::ui::components::inspector::utils::field_from_type_path;
use crate::ui::components::inspector::{FieldKind, VariantField, name_to_label, path_to_label};

use crate::ui::tokens::{BORDER_COLOR, FONT_PATH, TEXT_BODY_COLOR, TEXT_MUTED_COLOR, TEXT_SIZE_SM};
use crate::ui::widgets::button::{
    ButtonClickEvent, ButtonProps, ButtonVariant, EditorButton, button, set_button_variant,
};
use crate::ui::widgets::checkbox::{CheckboxProps, checkbox};
use crate::ui::widgets::color_picker::{ColorPickerProps, color_picker};
use crate::ui::widgets::combobox::{
    ComboBoxChangeEvent, ComboBoxOptionData, combobox, combobox_with_selected,
};
use crate::ui::widgets::gradient_edit::{GradientEditProps, gradient_edit};
use crate::ui::widgets::popover::{
    EditorPopover, PopoverHeaderProps, PopoverPlacement, PopoverProps, popover, popover_content,
    popover_header,
};
use crate::ui::widgets::text_edit::{TextEditProps, text_edit};

use crate::ui::icons::ICON_MORE;
use crate::ui::widgets::vector_edit::{VectorEditProps, vector_edit};

use sprinkles::textures::preset::{PresetTexture, TextureRef};

#[derive(Clone, Default)]
pub enum VariantContentMode {
    #[default]
    AutoFields,
    CustomContent,
}

pub struct VariantDefinition {
    pub name: String,
    pub aliases: Vec<String>,
    pub icon: Option<String>,
    pub rows: Vec<Vec<VariantField>>,
    default_value: Option<Box<dyn PartialReflect>>,
}

impl Clone for VariantDefinition {
    fn clone(&self) -> Self {
        let cloned_default =
            self.default_value
                .as_ref()
                .and_then(|v| match v.as_ref().reflect_clone() {
                    Ok(cloned) => Some(cloned.into_partial_reflect()),
                    Err(err) => {
                        warn!(
                            "VariantDefinition::clone: reflect_clone failed for variant '{}': {:?}",
                            self.name, err
                        );
                        None
                    }
                });

        Self {
            name: self.name.clone(),
            aliases: self.aliases.clone(),
            icon: self.icon.clone(),
            rows: self.rows.clone(),
            default_value: cloned_default,
        }
    }
}

impl std::fmt::Debug for VariantDefinition {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("VariantDefinition")
            .field("name", &self.name)
            .field("aliases", &self.aliases)
            .field("icon", &self.icon)
            .field("rows", &self.rows)
            .field("default_value", &self.default_value.is_some())
            .finish()
    }
}

impl VariantDefinition {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            aliases: Vec::new(),
            icon: None,
            rows: Vec::new(),
            default_value: None,
        }
    }

    pub fn with_aliases(mut self, aliases: Vec<impl Into<String>>) -> Self {
        self.aliases = aliases.into_iter().map(Into::into).collect();
        self
    }

    pub fn with_icon(mut self, icon: impl Into<String>) -> Self {
        self.icon = Some(icon.into());
        self
    }

    pub fn with_rows(mut self, rows: Vec<Vec<VariantField>>) -> Self {
        self.rows = rows;
        self
    }

    pub fn with_default<T: PartialReflect + Clone + 'static>(mut self, value: T) -> Self {
        self.default_value = Some(Box::new(value));
        self
    }

    pub fn with_default_boxed(mut self, value: Box<dyn PartialReflect>) -> Self {
        self.default_value = Some(value);
        self
    }

    pub fn create_default(&self) -> Option<Box<dyn PartialReflect>> {
        let Some(default_value) = self.default_value.as_ref() else {
            warn!(
                "VariantDefinition::create_default: no default_value stored for variant '{}'",
                self.name
            );
            return None;
        };

        match default_value.as_ref().reflect_clone() {
            Ok(cloned) => Some(cloned.into_partial_reflect()),
            Err(err) => {
                warn!(
                    "VariantDefinition::create_default: reflect_clone failed for variant '{}': {:?}",
                    self.name, err
                );
                None
            }
        }
    }

    pub fn from_reflect<T: Typed>(variant_name: &str) -> Option<Self> {
        let TypeInfo::Enum(enum_info) = T::type_info() else {
            return None;
        };

        let variant_info = (0..enum_info.variant_len())
            .filter_map(|i| enum_info.variant_at(i))
            .find(|v| v.name() == variant_name)?;

        let mut def = Self::new(variant_name);
        def.rows = Self::rows_from_variant_info(variant_info);
        Some(def)
    }

    pub fn with_inferred_rows<T: Typed>(mut self) -> Self {
        let TypeInfo::Enum(enum_info) = T::type_info() else {
            return self;
        };

        let variant_info = (0..enum_info.variant_len())
            .filter_map(|i| enum_info.variant_at(i))
            .find(|v| v.name() == self.name);

        if let Some(variant_info) = variant_info {
            self.rows = Self::rows_from_variant_info(variant_info);
        }
        self
    }

    fn rows_from_variant_info(variant_info: &VariantInfo) -> Vec<Vec<VariantField>> {
        match variant_info {
            VariantInfo::Struct(struct_info) => struct_info
                .iter()
                .filter_map(|field| {
                    let name = field.name();
                    let type_path = field.type_path();
                    field_from_type_path(name, type_path, None).map(|f| vec![f])
                })
                .collect(),
            VariantInfo::Unit(_) | VariantInfo::Tuple(_) => Vec::new(),
        }
    }
}

pub fn plugin(app: &mut App) {
    app.add_observer(handle_variant_edit_click)
        .add_observer(handle_variant_combobox_change)
        .add_systems(
            Update,
            (
                setup_variant_edit,
                sync_variant_edit_button,
                handle_popover_closed,
            ),
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
    pub popover_width: Option<f32>,
    pub content_mode: VariantContentMode,
    pub show_swatch_slot: bool,
    initialized: bool,
}

#[derive(Component)]
struct VariantEditPopover(Entity);

#[derive(Component)]
struct VariantEditLeftIcon(Entity);

#[derive(Component)]
pub struct VariantEditSwatchSlot(pub Entity);

#[derive(Component)]
pub struct VariantFieldsContainer(pub Entity);

#[derive(Component)]
pub struct VariantComboBox(pub Entity);

#[derive(Component, Default)]
struct VariantEditState {
    popover: Option<Entity>,
    last_synced_index: Option<usize>,
}

pub struct VariantEditProps {
    pub path: String,
    pub label: Option<String>,
    pub popover_title: Option<String>,
    pub variants: Vec<VariantDefinition>,
    pub selected_index: usize,
    pub popover_width: Option<f32>,
    pub content_mode: VariantContentMode,
    pub show_swatch_slot: bool,
}

impl VariantEditProps {
    pub fn new(path: impl Into<String>) -> Self {
        Self {
            path: path.into(),
            label: None,
            popover_title: None,
            variants: Vec::new(),
            selected_index: 0,
            popover_width: Some(256.0),
            content_mode: VariantContentMode::default(),
            show_swatch_slot: false,
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

    pub fn with_popover_width(mut self, width: Option<f32>) -> Self {
        self.popover_width = width;
        self
    }

    pub fn with_content_mode(mut self, mode: VariantContentMode) -> Self {
        self.content_mode = mode;
        self
    }

    pub fn with_swatch_slot(mut self, show: bool) -> Self {
        self.show_swatch_slot = show;
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
        popover_width,
        content_mode,
        show_swatch_slot,
    } = props;

    (
        EditorVariantEdit,
        VariantEditConfig {
            path,
            label,
            popover_title,
            variants,
            selected_index,
            popover_width,
            content_mode,
            show_swatch_slot,
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

const SWATCH_SIZE: f32 = 16.0;

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
            .map(|v| name_to_label(&v.name))
            .unwrap_or_default();
        let icon = selected_variant.and_then(|v| v.icon.clone());

        let button_props = ButtonProps::new(&value)
            .align_left()
            .with_right_icon(ICON_MORE);

        let button_entity = commands
            .spawn((VariantEditButton, button(button_props)))
            .id();

        if config.show_swatch_slot {
            let swatch_slot = commands
                .spawn((
                    VariantEditSwatchSlot(entity),
                    Node {
                        width: px(SWATCH_SIZE),
                        height: px(SWATCH_SIZE),
                        border_radius: BorderRadius::all(px(4.0)),
                        overflow: Overflow::clip(),
                        ..default()
                    },
                ))
                .id();
            commands
                .entity(button_entity)
                .insert_children(0, &[swatch_slot]);
        } else {
            let has_icon = icon.is_some();
            let icon_path = icon.unwrap_or_else(|| ICON_MORE.to_string());
            let left_icon_entity = commands
                .spawn((
                    VariantEditLeftIcon(entity),
                    ImageNode::new(asset_server.load(&icon_path))
                        .with_color(Color::Srgba(TEXT_BODY_COLOR)),
                    Node {
                        width: px(16.0),
                        height: px(16.0),
                        display: if has_icon {
                            Display::Flex
                        } else {
                            Display::None
                        },
                        ..default()
                    },
                ))
                .id();
            commands
                .entity(button_entity)
                .insert_children(0, &[left_icon_entity]);
        }

        commands.entity(entity).add_child(button_entity);
    }
}

fn sync_variant_edit_button(
    asset_server: Res<AssetServer>,
    mut variant_edits: Query<
        (Entity, &VariantEditConfig, &mut VariantEditState, &Children),
        With<EditorVariantEdit>,
    >,
    children_query: Query<&Children>,
    mut texts: Query<&mut Text>,
    mut left_icons: Query<(&VariantEditLeftIcon, &mut ImageNode, &mut Node)>,
) {
    for (entity, config, mut state, children) in &mut variant_edits {
        if state.last_synced_index == Some(config.selected_index) {
            continue;
        }

        let Some(selected_variant) = config.variants.get(config.selected_index) else {
            continue;
        };

        let Some(&button_entity) = children.last() else {
            continue;
        };
        let Ok(button_children) = children_query.get(button_entity) else {
            continue;
        };

        let mut text_updated = false;
        for child in button_children.iter() {
            if let Ok(mut text) = texts.get_mut(child) {
                **text = name_to_label(&selected_variant.name);
                text_updated = true;
                break;
            }
        }

        if !config.show_swatch_slot {
            for (left_icon, mut image, mut node) in &mut left_icons {
                if left_icon.0 != entity {
                    continue;
                }
                if let Some(ref icon_path) = selected_variant.icon {
                    image.image = asset_server.load(icon_path);
                    node.display = Display::Flex;
                } else {
                    node.display = Display::None;
                }
                break;
            }
        }

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

    let variant_edit_entity =
        if let Ok(button_child_of) = variant_edit_buttons.get(child_of.parent()) {
            button_child_of.parent()
        } else {
            child_of.parent()
        };

    let Ok((entity, mut state, config, children)) = variant_edits.get_mut(variant_edit_entity)
    else {
        return;
    };

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
            let mut opt = ComboBoxOptionData::new(name_to_label(&v.name)).with_value(&v.name);
            if let Some(ref icon) = v.icon {
                opt = opt.with_icon(icon);
            }
            opt
        })
        .collect();

    let selected_variant = config.variants.get(config.selected_index);
    let has_auto_fields = matches!(config.content_mode, VariantContentMode::AutoFields)
        && selected_variant
            .map(|v| !v.rows.is_empty())
            .unwrap_or(false);
    let has_custom_content = matches!(config.content_mode, VariantContentMode::CustomContent);
    let show_fields_container = has_auto_fields || has_custom_content;

    let default_width = 256.0;
    let popover_props = PopoverProps::new(trigger.entity)
        .with_placement(PopoverPlacement::Right)
        .with_padding(0.0);

    let popover_props = if let Some(width) = config.popover_width {
        popover_props.with_node(Node {
            width: px(width),
            min_width: px(default_width),
            ..default()
        })
    } else {
        popover_props.with_node(Node {
            min_width: px(default_width),
            ..default()
        })
    };

    let popover_entity = commands
        .spawn((VariantEditPopover(entity), popover(popover_props)))
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
                        border: if show_fields_container {
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

            if show_fields_container {
                let fields_container = parent
                    .spawn((VariantFieldsContainer(entity), popover_content()))
                    .id();

                if has_auto_fields {
                    if let Some(variant) = selected_variant {
                        let mut cmds = parent.commands();
                        spawn_variant_fields_for_entity(
                            &mut cmds,
                            fields_container,
                            entity,
                            &config.path,
                            &variant.rows,
                            &asset_server,
                        );
                    }
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
    mut left_icons: Query<(&VariantEditLeftIcon, &mut ImageNode, &mut Node)>,
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

    if let Ok(children) = variant_edit_children.get(variant_edit_entity) {
        if let Some(&button_entity) = children.last() {
            if let Ok(button_children) = children_query.get(button_entity) {
                for child in button_children.iter() {
                    if let Ok(mut text) = texts.get_mut(child) {
                        **text = name_to_label(&selected_variant.name);
                        break;
                    }
                }
            }
        }
    }

    if !config.show_swatch_slot {
        for (left_icon, mut image, mut node) in &mut left_icons {
            if left_icon.0 != variant_edit_entity {
                continue;
            }
            if let Some(ref icon_path) = selected_variant.icon {
                image.image = asset_server.load(icon_path);
                node.display = Display::Flex;
            } else {
                node.display = Display::None;
            }
            break;
        }
    }

    if matches!(config.content_mode, VariantContentMode::AutoFields) {
        for (container_entity, container) in &fields_containers {
            if container.0 != variant_edit_entity {
                continue;
            }

            if let Ok(children) = children_query.get(container_entity) {
                for child in children.iter() {
                    commands.entity(child).try_despawn();
                }
            }

            spawn_variant_fields_for_entity(
                &mut commands,
                container_entity,
                variant_edit_entity,
                &config.path,
                &selected_variant.rows,
                &asset_server,
            );

            break;
        }
    }
}

fn spawn_variant_fields_for_entity(
    commands: &mut Commands,
    container: Entity,
    variant_edit: Entity,
    path: &str,
    rows: &[Vec<VariantField>],
    asset_server: &AssetServer,
) {
    for row_fields in rows {
        let row_entity = commands.spawn(fields_row()).id();
        commands.entity(container).add_child(row_entity);

        for field in row_fields {
            let label = path_to_label(&field.name);
            let binding = FieldBinding::emitter_variant(
                path,
                &field.name,
                field.kind.clone(),
                variant_edit,
            );

            let field_entity =
                spawn_field_widget(commands, asset_server, &field.kind, label, binding);
            commands.entity(row_entity).add_child(field_entity);
        }
    }
}

fn spawn_field_widget(
    commands: &mut Commands,
    asset_server: &AssetServer,
    kind: &FieldKind,
    label: String,
    binding: FieldBinding,
) -> Entity {
    match kind {
        FieldKind::F32 | FieldKind::F32Percent => commands
            .spawn((
                binding,
                text_edit(TextEditProps::default().with_label(label).numeric_f32()),
            ))
            .id(),

        FieldKind::U32 | FieldKind::U32OrEmpty | FieldKind::OptionalU32 => commands
            .spawn((
                binding,
                text_edit(TextEditProps::default().with_label(label).numeric_i32()),
            ))
            .id(),

        FieldKind::Bool => commands
            .spawn((binding, checkbox(CheckboxProps::new(label), asset_server)))
            .id(),

        FieldKind::Vector(suffixes) => commands
            .spawn((
                binding,
                vector_edit(
                    VectorEditProps::default()
                        .with_label(label)
                        .with_size(suffixes.vector_size())
                        .with_suffixes(*suffixes),
                ),
            ))
            .id(),

        FieldKind::ComboBox { options } => {
            let combobox_options: Vec<ComboBoxOptionData> =
                options.iter().map(|o| ComboBoxOptionData::new(o)).collect();
            spawn_labeled_field(
                commands,
                asset_server,
                &label,
                binding,
                combobox(combobox_options),
            )
        }

        FieldKind::Color => spawn_labeled_field(
            commands,
            asset_server,
            &label,
            binding,
            color_picker(ColorPickerProps::new()),
        ),

        FieldKind::Gradient => spawn_labeled_field(
            commands,
            asset_server,
            &label,
            binding,
            gradient_edit(GradientEditProps::new().inline()),
        ),

        FieldKind::TextureRef => {
            let field_name = binding.field_name().unwrap_or_default().to_string();
            let props = VariantEditProps::new(&field_name)
                .with_label(label)
                .with_variants(texture_ref_variants())
                .with_content_mode(VariantContentMode::CustomContent);
            commands.spawn((binding, variant_edit(props))).id()
        }

        FieldKind::Curve | FieldKind::AnimatedVelocity => commands.spawn_empty().id(),
    }
}

fn spawn_labeled_field(
    commands: &mut Commands,
    asset_server: &AssetServer,
    label: &str,
    binding: FieldBinding,
    widget: impl Bundle,
) -> Entity {
    let font: Handle<Font> = asset_server.load(FONT_PATH);

    commands
        .spawn(labeled_field_wrapper())
        .with_child((
            Text::new(label),
            TextFont {
                font,
                font_size: TEXT_SIZE_SM,
                weight: FontWeight::MEDIUM,
                ..default()
            },
            TextColor(TEXT_MUTED_COLOR.into()),
        ))
        .with_child((binding, widget))
        .id()
}

fn labeled_field_wrapper() -> impl Bundle {
    Node {
        flex_direction: FlexDirection::Column,
        row_gap: px(3.0),
        flex_grow: 1.0,
        flex_shrink: 1.0,
        flex_basis: px(0.0),
        ..default()
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

fn texture_ref_variants() -> Vec<VariantDefinition> {
    vec![
        VariantDefinition::new("None")
            .with_icon("icons/blender-texture.png")
            .with_default(Option::<TextureRef>::None),
        VariantDefinition::new("Preset")
            .with_icon("icons/blender-texture.png")
            .with_default(Some(TextureRef::Preset(PresetTexture::Circle1))),
        VariantDefinition::new("Custom")
            .with_icon("icons/blender-texture.png")
            .with_aliases(vec!["Asset", "Local"])
            .with_default(Some(TextureRef::Asset(String::new()))),
    ]
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
