use bevy::math::Rot2;
use bevy::prelude::*;

use crate::ui::widgets::button::{
    ButtonClickEvent, ButtonProps, ButtonSize, ButtonVariant, button, icon_button, IconButtonProps,
    set_button_variant,
};
use crate::ui::widgets::popover::{EditorPopover, PopoverPlacement, PopoverProps, popover};

const ICON_CHEVRON_DOWN: &str = "icons/ri-arrow-down-s-line.png";
const ICON_MORE: &str = "icons/ri-more-fill.png";

pub fn plugin(app: &mut App) {
    app.add_observer(handle_trigger_click)
        .add_observer(handle_option_click)
        .add_systems(Update, (setup_combobox, handle_combobox_popover_closed));
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
}

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

    // check if popover already exists for this combobox (toggle behavior)
    for (popover_entity, popover_ref) in &existing_popovers {
        if popover_ref.0 == combo_trigger.0 {
            commands.entity(popover_entity).try_despawn();
            state.popover = None;
            reset_combobox_trigger_style(
                trigger.entity,
                &mut button_styles,
                &children_query,
                &mut transforms,
                &images,
                &mut commands,
            );
            return;
        }
    }

    // check if any other popover is open - if so, don't open a new one unless nested
    let any_popover_open = !all_popovers.is_empty();
    if any_popover_open {
        // check if this combobox is nested inside an open popover
        let is_nested = all_popovers
            .iter()
            .any(|popover| is_descendant_of(combo_trigger.0, popover, &parents));
        if !is_nested {
            return;
        }
    }

    let combobox_entity = combo_trigger.0;

    // set button to active state and rotate chevron
    if let Ok((mut bg, mut border, mut variant)) = button_styles.get_mut(trigger.entity) {
        *variant = ButtonVariant::ActiveAlt;
        set_button_variant(ButtonVariant::ActiveAlt, &mut bg, &mut border);
    }

    // rotate chevron icon (last ImageNode child of the button)
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
        let variant = if config.style == ComboBoxStyle::Default && index == config.selected {
            ButtonVariant::Active
        } else {
            ButtonVariant::Ghost
        };

        let mut button_props = ButtonProps::new(&option.label).with_variant(variant).align_left();

        if let Some(ref icon_path) = option.icon {
            button_props = button_props.with_left_icon(icon_path);
        }

        commands.entity(popover_entity).with_child((
            ComboBoxOption {
                combobox: combobox_entity,
                index,
                label: option.label.clone(),
            },
            button(button_props),
        ));
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

fn reset_combobox_trigger_style(
    trigger_entity: Entity,
    button_styles: &mut Query<(&mut BackgroundColor, &mut BorderColor, &mut ButtonVariant)>,
    children_query: &Query<&Children>,
    transforms: &mut Query<&mut UiTransform>,
    images: &Query<(), With<ImageNode>>,
    commands: &mut Commands,
) {
    if let Ok((mut bg, mut border, mut variant)) = button_styles.get_mut(trigger_entity) {
        *variant = ButtonVariant::Default;
        set_button_variant(ButtonVariant::Default, &mut bg, &mut border);
    }

    // reset chevron rotation
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
    mut states: Query<(&mut ComboBoxState, &Children), With<EditorComboBox>>,
    popovers: Query<Entity, With<EditorPopover>>,
    triggers: Query<Entity, With<ComboBoxTrigger>>,
    mut button_styles: Query<(&mut BackgroundColor, &mut BorderColor, &mut ButtonVariant)>,
    children_query: Query<&Children>,
    mut transforms: Query<&mut UiTransform>,
    images: Query<(), With<ImageNode>>,
) {
    for (mut state, combobox_children) in &mut states {
        let Some(popover_entity) = state.popover else {
            continue;
        };

        if popovers.get(popover_entity).is_ok() {
            continue;
        }

        state.popover = None;

        // find the trigger button entity (first child of combobox)
        for child in combobox_children.iter() {
            if triggers.get(child).is_ok() {
                reset_combobox_trigger_style(
                    child,
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
