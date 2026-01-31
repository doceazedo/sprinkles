use bevy::prelude::*;
use inflector::Inflector;

use crate::ui::tokens::{BORDER_COLOR, FONT_PATH, TEXT_BODY_COLOR, TEXT_MUTED_COLOR, TEXT_SIZE, TEXT_SIZE_SM};
use crate::ui::widgets::button::{
    ButtonClickEvent, ButtonProps, ButtonVariant, EditorButton, button, set_button_variant,
};
use crate::ui::widgets::combobox::{ComboBoxOptionData, combobox};
use crate::ui::widgets::popover::{
    EditorPopover, PopoverHeaderProps, PopoverPlacement, PopoverProps, popover, popover_content,
    popover_header,
};

const ICON_MORE: &str = "icons/ri-more-fill.png";

pub fn plugin(app: &mut App) {
    app.add_observer(handle_variant_edit_click)
        .add_systems(Update, (setup_variant_edit, handle_popover_closed));
}

#[derive(Component)]
pub struct EditorVariantEdit;

#[derive(Component)]
struct VariantEditConfig {
    icon: String,
    value: String,
    label: Option<String>,
    popover_title: Option<String>,
    options: Vec<ComboBoxOptionData>,
    initialized: bool,
}

#[derive(Component)]
struct VariantEditPopover(Entity);

#[derive(Component, Default)]
struct VariantEditState {
    popover: Option<Entity>,
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
    pub icon: String,
    pub value: String,
    pub label: Option<String>,
    pub popover_title: Option<String>,
    pub options: Vec<ComboBoxOptionData>,
}

impl VariantEditProps {
    pub fn new(icon: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            icon: icon.into(),
            value: value.into(),
            label: None,
            popover_title: None,
            options: Vec::new(),
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

    pub fn with_options(mut self, options: Vec<impl Into<ComboBoxOptionData>>) -> Self {
        self.options = options.into_iter().map(Into::into).collect();
        self
    }
}

pub fn variant_edit(props: VariantEditProps) -> impl Bundle {
    let VariantEditProps {
        icon,
        value,
        label,
        popover_title,
        options,
    } = props;

    (
        EditorVariantEdit,
        VariantEditConfig {
            icon,
            value,
            label,
            popover_title,
            options,
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

        if let Some(ref label) = config.label {
            let label_entity = commands
                .spawn((
                    Text::new(label),
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
        }

        let button_entity = commands
            .spawn(button(
                ButtonProps::new(&config.value)
                    .align_left()
                    .with_left_icon(&config.icon)
                    .with_right_icon(ICON_MORE),
            ))
            .id();

        commands.entity(entity).add_child(button_entity);
    }
}

fn handle_variant_edit_click(
    trigger: On<ButtonClickEvent>,
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    buttons: Query<&ChildOf, With<EditorButton>>,
    mut variant_edits: Query<
        (&mut VariantEditState, &VariantEditConfig, &Children),
        With<EditorVariantEdit>,
    >,
    existing_popovers: Query<Entity, With<VariantEditPopover>>,
    mut button_styles: Query<(&mut BackgroundColor, &mut BorderColor, &mut ButtonVariant)>,
) {
    let Ok(child_of) = buttons.get(trigger.entity) else {
        return;
    };

    let Ok((mut state, config, children)) = variant_edits.get_mut(child_of.parent()) else {
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

    if let Some(&button_entity) = children.last() {
        if let Ok((mut bg, mut border, mut variant)) = button_styles.get_mut(button_entity) {
            *variant = ButtonVariant::ActiveAlt;
            set_button_variant(ButtonVariant::ActiveAlt, &mut bg, &mut border);
        }
    }

    let font: Handle<Font> = asset_server.load(FONT_PATH);
    let popover_title = config
        .popover_title
        .clone()
        .or_else(|| config.label.clone())
        .unwrap_or_else(|| config.value.clone());

    let popover_entity = commands
        .spawn((
            VariantEditPopover(child_of.parent()),
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
                        border: UiRect::bottom(px(1.0)),
                        ..default()
                    },
                    BorderColor::all(BORDER_COLOR),
                ))
                .with_child(combobox(config.options.clone()));
        })
        .with_child((
            popover_content(),
            children![(
                Text::new("TODO"),
                TextFont {
                    font: font.clone(),
                    font_size: TEXT_SIZE,
                    ..default()
                },
                TextColor(TEXT_BODY_COLOR.into()),
            )],
        ));

    state.popover = Some(popover_entity);
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
