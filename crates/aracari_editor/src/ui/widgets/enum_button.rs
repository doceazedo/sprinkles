use bevy::prelude::*;

use crate::ui::tokens::{FONT_PATH, TEXT_BODY_COLOR, TEXT_SIZE};
use crate::ui::widgets::button::{
    ButtonClickEvent, ButtonProps, ButtonVariant, EditorButton, button, set_button_variant,
};
use crate::ui::widgets::popover::{EditorPopover, PopoverPlacement, PopoverProps, popover};

const ICON_MORE: &str = "icons/ri-more-fill.png";

pub fn plugin(app: &mut App) {
    app.add_observer(handle_enum_button_click)
        .add_systems(Update, (setup_enum_button, handle_popover_closed));
}

#[derive(Component)]
pub struct EditorEnumButton;

#[derive(Component)]
struct EnumButtonConfig {
    icon: String,
    label: String,
    initialized: bool,
}

#[derive(Component)]
struct EnumButtonPopover(Entity);

#[derive(Component, Default)]
struct EnumButtonState {
    popover: Option<Entity>,
}

pub struct EnumButtonProps {
    pub icon: String,
    pub label: String,
}

impl EnumButtonProps {
    pub fn new(icon: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            icon: icon.into(),
            label: label.into(),
        }
    }
}

pub fn enum_button(props: EnumButtonProps) -> impl Bundle {
    let EnumButtonProps { icon, label } = props;

    (
        EditorEnumButton,
        EnumButtonConfig {
            icon,
            label,
            initialized: false,
        },
        EnumButtonState::default(),
        Node::default(),
    )
}

fn setup_enum_button(mut commands: Commands, mut configs: Query<(Entity, &mut EnumButtonConfig)>) {
    for (entity, mut config) in &mut configs {
        if config.initialized {
            continue;
        }
        config.initialized = true;

        let button_entity = commands
            .spawn(button(
                ButtonProps::new(&config.label)
                    .align_left()
                    .with_left_icon(&config.icon)
                    .with_right_icon(ICON_MORE),
            ))
            .id();

        commands.entity(entity).add_child(button_entity);
    }
}

fn handle_enum_button_click(
    trigger: On<ButtonClickEvent>,
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    buttons: Query<&ChildOf, With<EditorButton>>,
    mut enum_buttons: Query<(&mut EnumButtonState, &Children), With<EditorEnumButton>>,
    existing_popovers: Query<Entity, With<EnumButtonPopover>>,
    mut button_styles: Query<(&mut BackgroundColor, &mut BorderColor, &mut ButtonVariant)>,
) {
    let Ok(child_of) = buttons.get(trigger.entity) else {
        return;
    };

    let Ok((mut state, children)) = enum_buttons.get_mut(child_of.parent()) else {
        return;
    };

    if let Some(popover_entity) = state.popover {
        if existing_popovers.get(popover_entity).is_ok() {
            commands.entity(popover_entity).try_despawn();
            state.popover = None;
            if let Some(&button_entity) = children.first() {
                if let Ok((mut bg, mut border, mut variant)) = button_styles.get_mut(button_entity)
                {
                    *variant = ButtonVariant::Default;
                    set_button_variant(ButtonVariant::Default, &mut bg, &mut border);
                }
            }
            return;
        }
    }

    if let Some(&button_entity) = children.first() {
        if let Ok((mut bg, mut border, mut variant)) = button_styles.get_mut(button_entity) {
            *variant = ButtonVariant::ActiveAlt;
            set_button_variant(ButtonVariant::ActiveAlt, &mut bg, &mut border);
        }
    }

    let font: Handle<Font> = asset_server.load(FONT_PATH);

    let popover_entity = commands
        .spawn((
            EnumButtonPopover(child_of.parent()),
            popover(
                PopoverProps::new(trigger.entity)
                    .with_placement(PopoverPlacement::Left)
                    .with_padding(0.0)
                    .with_node(Node {
                        width: px(256.0),
                        ..default()
                    }),
            ),
            children![(
                Text::new("TODO: EnumButton popover"),
                TextFont {
                    font: font.clone(),
                    font_size: TEXT_SIZE,
                    ..default()
                },
                TextColor(TEXT_BODY_COLOR.into()),
                Node {
                    padding: UiRect::all(px(12.0)),
                    ..default()
                },
            )],
        ))
        .id();

    state.popover = Some(popover_entity);
}

fn handle_popover_closed(
    mut enum_buttons: Query<(&mut EnumButtonState, &Children), With<EditorEnumButton>>,
    popovers: Query<Entity, With<EditorPopover>>,
    mut button_styles: Query<(&mut BackgroundColor, &mut BorderColor, &mut ButtonVariant)>,
) {
    for (mut state, children) in &mut enum_buttons {
        let Some(popover_entity) = state.popover else {
            continue;
        };

        if popovers.get(popover_entity).is_ok() {
            continue;
        }

        state.popover = None;

        if let Some(&button_entity) = children.first() {
            if let Ok((mut bg, mut border, mut variant)) = button_styles.get_mut(button_entity) {
                *variant = ButtonVariant::Default;
                set_button_variant(ButtonVariant::Default, &mut bg, &mut border);
            }
        }
    }
}
