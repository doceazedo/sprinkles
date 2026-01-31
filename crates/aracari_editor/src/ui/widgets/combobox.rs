use bevy::prelude::*;

use crate::ui::widgets::button::{
    ButtonClickEvent, ButtonProps, ButtonSize, ButtonVariant, button, icon_button, IconButtonProps,
};
use crate::ui::widgets::popover::{PopoverPlacement, PopoverProps, popover};

const ICON_CHEVRON_DOWN: &str = "icons/ri-arrow-down-s-line.png";
const ICON_MORE: &str = "icons/ri-more-fill.png";

pub fn plugin(app: &mut App) {
    app.add_observer(handle_trigger_click)
        .add_observer(handle_option_click)
        .add_systems(Update, setup_combobox);
}

#[derive(Component)]
pub struct EditorComboBox;

#[derive(Component)]
pub struct ComboBoxTrigger(pub Entity);

#[derive(Component)]
pub struct ComboBoxPopover(pub Entity);

#[derive(Component, Clone)]
struct ComboBoxOption {
    combobox: Entity,
    index: usize,
    label: String,
}

#[derive(Clone)]
pub struct ComboBoxOptionData {
    pub label: String,
    pub icon: Option<String>,
}

impl ComboBoxOptionData {
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            icon: None,
        }
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
struct ComboBoxConfig {
    options: Vec<ComboBoxOptionData>,
    selected: usize,
    icon: Option<String>,
    style: ComboBoxStyle,
    initialized: bool,
}

#[derive(EntityEvent)]
pub struct ComboBoxChangeEvent {
    pub entity: Entity,
    pub selected: usize,
    pub label: String,
}

pub fn combobox(options: Vec<impl Into<ComboBoxOptionData>>) -> impl Bundle {
    (
        EditorComboBox,
        ComboBoxConfig {
            options: options.into_iter().map(Into::into).collect(),
            selected: 0,
            icon: None,
            style: ComboBoxStyle::Default,
            initialized: false,
        },
        Node::default(),
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
            initialized: false,
        },
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
                let label = selected_option
                    .map(|o| o.label.clone())
                    .unwrap_or_default();
                let selected_icon = selected_option.and_then(|o| o.icon.clone());
                let icon_to_show = config.icon.clone().or(selected_icon);

                let mut button_props = ButtonProps::new(label)
                    .with_size(ButtonSize::MD)
                    .align_left()
                    .with_right_icon(ICON_CHEVRON_DOWN);

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
    existing_popovers: Query<(Entity, &ComboBoxPopover)>,
) {
    let Ok(combo_trigger) = triggers.get(trigger.entity) else {
        return;
    };
    let Ok(config) = configs.get(combo_trigger.0) else {
        return;
    };

    for (popover_entity, popover_ref) in &existing_popovers {
        if popover_ref.0 == combo_trigger.0 {
            commands.entity(popover_entity).try_despawn();
            return;
        }
    }

    let combobox_entity = combo_trigger.0;

    let mut popover_cmd = commands.spawn((
        ComboBoxPopover(combobox_entity),
        popover(
            PopoverProps::new(trigger.entity)
                .with_placement(PopoverPlacement::BottomStart)
                .with_padding(4.0)
                .with_node(Node {
                    min_width: px(120.0),
                    ..default()
                }),
        ),
    ));

    for (index, option) in config.options.iter().enumerate() {
        let variant = if config.style == ComboBoxStyle::Default && index == config.selected {
            ButtonVariant::Active
        } else {
            ButtonVariant::Ghost
        };

        let mut button_props = ButtonProps::new(&option.label).with_variant(variant).align_left();

        if let Some(ref icon_path) = option.icon {
            button_props = button_props.with_left_icon(icon_path);
        }

        popover_cmd.with_child((
            ComboBoxOption {
                combobox: combobox_entity,
                index,
                label: option.label.clone(),
            },
            button(button_props),
        ));
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
    let selected_option = config.options.get(option.index).cloned();
    let should_update_icon = config.icon.is_none();
    config.selected = option.index;

    commands.trigger(ComboBoxChangeEvent {
        entity: option.combobox,
        selected: option.index,
        label: option.label.clone(),
    });

    if !is_icon_only {
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
