use bevy::math::Rot2;
use bevy::prelude::*;

use crate::ui::icons::{ICON_ARROW_DOWN, ICON_MORE};
use crate::ui::widgets::button::{
    ButtonClickEvent, ButtonProps, ButtonSize, ButtonVariant, IconButtonProps, button, icon_button,
    set_button_variant,
};
use crate::ui::widgets::popover::{EditorPopover, PopoverPlacement, PopoverProps, popover};
use crate::ui::widgets::utils::is_descendant_of;

pub fn plugin(app: &mut App) {
    app.add_observer(handle_trigger_click)
        .add_observer(handle_option_click)
        .add_systems(
            Update,
            (
                setup_combobox,
                handle_combobox_popover_closed,
                sync_combobox_selection,
            ),
        );
}

#[derive(Component)]
pub struct EditorComboBox;

#[derive(Component)]
pub struct ComboBoxTrigger(pub Entity);

#[derive(Component)]
pub struct ComboBoxPopover(pub Entity);

#[derive(Component, Default)]
struct ComboBoxState {
    popover: Option<Entity>,
    last_synced_selected: Option<usize>,
}

#[derive(Component, Clone)]
struct ComboBoxOption {
    combobox: Entity,
    index: usize,
    label: String,
    value: Option<String>,
}

#[derive(Clone)]
pub struct ComboBoxOptionData {
    pub label: String,
    pub value: Option<String>,
    pub icon: Option<String>,
}

impl ComboBoxOptionData {
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            value: None,
            icon: None,
        }
    }

    pub fn with_value(mut self, value: impl Into<String>) -> Self {
        self.value = Some(value.into());
        self
    }

    pub fn with_icon(mut self, icon: impl Into<String>) -> Self {
        self.icon = Some(icon.into());
        self
    }
}

impl<T: Into<String>> From<T> for ComboBoxOptionData {
    fn from(label: T) -> Self {
        Self::new(label)
    }
}

#[derive(Clone, Copy, Default, PartialEq)]
enum ComboBoxStyle {
    #[default]
    Default,
    IconOnly,
}

#[derive(Component)]
pub(crate) struct ComboBoxConfig {
    options: Vec<ComboBoxOptionData>,
    pub(crate) selected: usize,
    icon: Option<String>,
    style: ComboBoxStyle,
    label_override: Option<String>,
    highlight_selected: bool,
    initialized: bool,
}

#[derive(EntityEvent)]
pub struct ComboBoxChangeEvent {
    pub entity: Entity,
    pub selected: usize,
    pub label: String,
    pub value: Option<String>,
}

pub fn combobox(options: Vec<impl Into<ComboBoxOptionData>>) -> impl Bundle {
    combobox_with_selected(options, 0)
}

pub fn combobox_with_selected(
    options: Vec<impl Into<ComboBoxOptionData>>,
    selected: usize,
) -> impl Bundle {
    (
        EditorComboBox,
        ComboBoxConfig {
            options: options.into_iter().map(Into::into).collect(),
            selected,
            icon: None,
            style: ComboBoxStyle::Default,
            label_override: None,
            highlight_selected: true,
            initialized: false,
        },
        ComboBoxState::default(),
        Node {
            width: percent(100),
            ..default()
        },
    )
}

pub fn combobox_with_label(
    options: Vec<impl Into<ComboBoxOptionData>>,
    label: impl Into<String>,
) -> impl Bundle {
    (
        EditorComboBox,
        ComboBoxConfig {
            options: options.into_iter().map(Into::into).collect(),
            selected: 0,
            icon: None,
            style: ComboBoxStyle::Default,
            label_override: Some(label.into()),
            highlight_selected: false,
            initialized: false,
        },
        ComboBoxState::default(),
        Node {
            width: percent(100),
            ..default()
        },
    )
}

pub fn combobox_icon(options: Vec<impl Into<ComboBoxOptionData>>) -> impl Bundle {
    (
        EditorComboBox,
        ComboBoxConfig {
            options: options.into_iter().map(Into::into).collect(),
            selected: 0,
            icon: None,
            style: ComboBoxStyle::IconOnly,
            label_override: None,
            highlight_selected: false,
            initialized: false,
        },
        ComboBoxState::default(),
        Node::default(),
    )
}

pub fn combobox_icon_with_selected(
    options: Vec<impl Into<ComboBoxOptionData>>,
    selected: usize,
) -> impl Bundle {
    (
        EditorComboBox,
        ComboBoxConfig {
            options: options.into_iter().map(Into::into).collect(),
            selected,
            icon: None,
            style: ComboBoxStyle::IconOnly,
            label_override: None,
            highlight_selected: true,
            initialized: false,
        },
        ComboBoxState::default(),
        Node::default(),
    )
}

fn setup_combobox(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut configs: Query<(Entity, &mut ComboBoxConfig)>,
) {
    for (entity, mut config) in &mut configs {
        if config.initialized {
            continue;
        }
        config.initialized = true;

        let trigger_entity = match config.style {
            ComboBoxStyle::IconOnly => commands
                .spawn((
                    ComboBoxTrigger(entity),
                    icon_button(
                        IconButtonProps::new(ICON_MORE).variant(ButtonVariant::Ghost),
                        &asset_server,
                    ),
                ))
                .id(),
            ComboBoxStyle::Default => {
                let selected_option = config.options.get(config.selected);
                let label = config
                    .label_override
                    .clone()
                    .or_else(|| selected_option.map(|o| o.label.clone()))
                    .unwrap_or_default();
                let selected_icon = selected_option.and_then(|o| o.icon.clone());
                let icon_to_show = config.icon.clone().or(selected_icon);

                let mut button_props = ButtonProps::new(label)
                    .with_size(ButtonSize::MD)
                    .align_left()
                    .with_right_icon(ICON_ARROW_DOWN);

                if let Some(icon_path) = icon_to_show {
                    button_props = button_props.with_left_icon(icon_path);
                }

                commands
                    .spawn((ComboBoxTrigger(entity), button(button_props)))
                    .id()
            }
        };

        commands.entity(entity).add_child(trigger_entity);
    }
}

fn handle_trigger_click(
    trigger: On<ButtonClickEvent>,
    mut commands: Commands,
    triggers: Query<&ComboBoxTrigger>,
    configs: Query<&ComboBoxConfig>,
    mut states: Query<&mut ComboBoxState>,
    existing_popovers: Query<(Entity, &ComboBoxPopover)>,
    all_popovers: Query<Entity, With<EditorPopover>>,
    mut button_styles: Query<(&mut BackgroundColor, &mut BorderColor, &mut ButtonVariant)>,
    children_query: Query<&Children>,
    mut transforms: Query<&mut UiTransform>,
    images: Query<(), With<ImageNode>>,
    parents: Query<&ChildOf>,
) {
    let Ok(combo_trigger) = triggers.get(trigger.entity) else {
        return;
    };
    let Ok(config) = configs.get(combo_trigger.0) else {
        return;
    };
    let Ok(mut state) = states.get_mut(combo_trigger.0) else {
        return;
    };

    for (popover_entity, popover_ref) in &existing_popovers {
        if popover_ref.0 == combo_trigger.0 {
            commands.entity(popover_entity).try_despawn();
            state.popover = None;
            let base = if config.style == ComboBoxStyle::IconOnly {
                ButtonVariant::Ghost
            } else {
                ButtonVariant::Default
            };
            reset_combobox_trigger_style(
                trigger.entity,
                base,
                &mut button_styles,
                &children_query,
                &mut transforms,
                &images,
                &mut commands,
            );
            return;
        }
    }

    let any_popover_open = !all_popovers.is_empty();
    if any_popover_open {
        let is_nested = all_popovers
            .iter()
            .any(|popover| is_descendant_of(combo_trigger.0, popover, &parents));
        if !is_nested {
            return;
        }
    }

    let combobox_entity = combo_trigger.0;

    if let Ok((mut bg, mut border, mut variant)) = button_styles.get_mut(trigger.entity) {
        *variant = ButtonVariant::ActiveAlt;
        set_button_variant(ButtonVariant::ActiveAlt, &mut bg, &mut border);
    }

    if let Ok(button_children) = children_query.get(trigger.entity) {
        for child in button_children.iter().rev() {
            if images.get(child).is_ok() {
                if let Ok(mut transform) = transforms.get_mut(child) {
                    transform.rotation = Rot2::degrees(180.0);
                } else {
                    commands.entity(child).insert(UiTransform {
                        rotation: Rot2::degrees(180.0),
                        ..default()
                    });
                }
                break;
            }
        }
    }

    let popover_entity = commands
        .spawn((
            ComboBoxPopover(combobox_entity),
            popover(
                PopoverProps::new(trigger.entity)
                    .with_placement(PopoverPlacement::BottomStart)
                    .with_padding(4.0)
                    .with_z_index(200)
                    .with_node(Node {
                        min_width: px(120.0),
                        ..default()
                    }),
            ),
        ))
        .id();

    state.popover = Some(popover_entity);

    for (index, option) in config.options.iter().enumerate() {
        let variant = if config.highlight_selected && index == config.selected {
            ButtonVariant::Active
        } else {
            ButtonVariant::Ghost
        };

        let mut button_props = ButtonProps::new(&option.label)
            .with_variant(variant)
            .align_left();

        if let Some(ref icon_path) = option.icon {
            button_props = button_props.with_left_icon(icon_path);
        }

        commands.entity(popover_entity).with_child((
            ComboBoxOption {
                combobox: combobox_entity,
                index,
                label: option.label.clone(),
                value: option.value.clone(),
            },
            button(button_props),
        ));
    }
}

fn reset_combobox_trigger_style(
    trigger_entity: Entity,
    base_variant: ButtonVariant,
    button_styles: &mut Query<(&mut BackgroundColor, &mut BorderColor, &mut ButtonVariant)>,
    children_query: &Query<&Children>,
    transforms: &mut Query<&mut UiTransform>,
    images: &Query<(), With<ImageNode>>,
    commands: &mut Commands,
) {
    if let Ok((mut bg, mut border, mut variant)) = button_styles.get_mut(trigger_entity) {
        *variant = base_variant;
        set_button_variant(base_variant, &mut bg, &mut border);
    }

    if let Ok(button_children) = children_query.get(trigger_entity) {
        for child in button_children.iter().rev() {
            if images.get(child).is_ok() {
                if let Ok(mut transform) = transforms.get_mut(child) {
                    transform.rotation = Rot2::degrees(0.0);
                } else {
                    commands.entity(child).insert(UiTransform::default());
                }
                break;
            }
        }
    }
}

fn handle_combobox_popover_closed(
    mut commands: Commands,
    mut states: Query<(&mut ComboBoxState, &ComboBoxConfig, &Children), With<EditorComboBox>>,
    popovers: Query<Entity, With<EditorPopover>>,
    triggers: Query<Entity, With<ComboBoxTrigger>>,
    mut button_styles: Query<(&mut BackgroundColor, &mut BorderColor, &mut ButtonVariant)>,
    children_query: Query<&Children>,
    mut transforms: Query<&mut UiTransform>,
    images: Query<(), With<ImageNode>>,
) {
    for (mut state, config, combobox_children) in &mut states {
        let Some(popover_entity) = state.popover else {
            continue;
        };

        if popovers.get(popover_entity).is_ok() {
            continue;
        }

        state.popover = None;

        let base = if config.style == ComboBoxStyle::IconOnly {
            ButtonVariant::Ghost
        } else {
            ButtonVariant::Default
        };

        for child in combobox_children.iter() {
            if triggers.get(child).is_ok() {
                reset_combobox_trigger_style(
                    child,
                    base,
                    &mut button_styles,
                    &children_query,
                    &mut transforms,
                    &images,
                    &mut commands,
                );
                break;
            }
        }
    }
}

fn handle_option_click(
    trigger: On<ButtonClickEvent>,
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    options: Query<&ComboBoxOption>,
    mut configs: Query<&mut ComboBoxConfig>,
    popovers: Query<(Entity, &ComboBoxPopover)>,
    triggers: Query<(Entity, &ComboBoxTrigger, &Children)>,
    mut texts: Query<&mut Text>,
    mut images: Query<&mut ImageNode>,
) {
    let Ok(option) = options.get(trigger.entity) else {
        return;
    };

    let Ok(mut config) = configs.get_mut(option.combobox) else {
        return;
    };

    let is_icon_only = config.style == ComboBoxStyle::IconOnly;
    let has_label_override = config.label_override.is_some();
    let selected_option = config.options.get(option.index).cloned();
    let should_update_icon = config.icon.is_none();
    config.selected = option.index;

    commands.trigger(ComboBoxChangeEvent {
        entity: option.combobox,
        selected: option.index,
        label: option.label.clone(),
        value: option.value.clone(),
    });

    if !is_icon_only && !has_label_override {
        for (_trigger_entity, combo_trigger, children) in &triggers {
            if combo_trigger.0 != option.combobox {
                continue;
            }
            let mut icon_updated = false;
            for child in children.iter() {
                if let Ok(mut text) = texts.get_mut(child) {
                    **text = option.label.clone();
                }
                if should_update_icon && !icon_updated {
                    if let Ok(mut image) = images.get_mut(child) {
                        if let Some(ref opt) = selected_option {
                            if let Some(ref icon_path) = opt.icon {
                                image.image = asset_server.load(icon_path);
                                icon_updated = true;
                            }
                        }
                    }
                }
            }
        }
    }

    for (popover_entity, popover_ref) in &popovers {
        if popover_ref.0 == option.combobox {
            commands.entity(popover_entity).try_despawn();
        }
    }
}

fn sync_combobox_selection(
    mut combos: Query<(Entity, &ComboBoxConfig, &mut ComboBoxState)>,
    triggers: Query<(&ComboBoxTrigger, &Children)>,
    mut texts: Query<&mut Text>,
) {
    for (entity, config, mut state) in &mut combos {
        if !config.initialized {
            continue;
        }
        let Some(option) = config.options.get(config.selected) else {
            continue;
        };
        let index_changed = state.last_synced_selected != Some(config.selected);
        for (trigger, children) in &triggers {
            if trigger.0 != entity {
                continue;
            }
            for child in children.iter() {
                if let Ok(mut text) = texts.get_mut(child) {
                    if index_changed || **text != option.label {
                        **text = option.label.clone();
                        state.last_synced_selected = Some(config.selected);
                    }
                    break;
                }
            }
            break;
        }
    }
}
