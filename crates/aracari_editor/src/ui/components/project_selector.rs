use bevy::prelude::*;

use aracari::prelude::*;

use crate::io::{EditorData, working_dir};
use crate::project::{BrowseOpenProjectEvent, OpenProjectEvent, load_project_from_path};
use crate::state::EditorState;
use crate::ui::tokens::{BORDER_COLOR, FONT_PATH, TEXT_MUTED_COLOR, TEXT_SIZE_SM};
use crate::ui::widgets::button::{ButtonClickEvent, ButtonProps, ButtonVariant, button};
use crate::ui::widgets::popover::{EditorPopover, PopoverPlacement, PopoverProps, popover};
use crate::ui::widgets::utils::is_descendant_of;

const ICON_CHEVRON_DOWN: &str = "icons/ri-arrow-down-s-line.png";
const ICON_NEW: &str = "icons/ri-file-add-line.png";
const ICON_OPEN: &str = "icons/ri-folder-open-line.png";
const ICON_EXAMPLES: &str = "icons/ri-folder-image-line.png";

pub fn plugin(app: &mut App) {
    app.add_observer(handle_trigger_click)
        .add_observer(handle_open_project_click)
        .add_observer(handle_recent_project_click)
        .add_observer(handle_popover_option_click)
        .add_systems(
        Update,
        (
            setup_project_selector,
            update_project_label,
            handle_popover_closed,
        ),
    );
}

#[derive(Component)]
pub struct ProjectSelector;

#[derive(Component)]
struct ProjectSelectorTrigger(Entity);

#[derive(Component, Default)]
struct ProjectSelectorState {
    popover: Option<Entity>,
    initialized: bool,
}

#[derive(Component)]
struct ProjectSelectorPopover;

#[derive(Component)]
struct OpenProjectButton;

#[derive(Component)]
struct RecentProjectButton(String);

pub fn project_selector() -> impl Bundle {
    (
        ProjectSelector,
        ProjectSelectorState::default(),
        Node::default(),
    )
}

fn setup_project_selector(
    mut commands: Commands,
    mut selectors: Query<(Entity, &mut ProjectSelectorState)>,
) {
    for (entity, mut state) in &mut selectors {
        if state.initialized {
            continue;
        }
        state.initialized = true;

        let trigger = commands
            .spawn((
                ProjectSelectorTrigger(entity),
                button(
                    ButtonProps::new("Untitled")
                        .with_variant(ButtonVariant::Ghost)
                        .with_right_icon(ICON_CHEVRON_DOWN),
                ),
            ))
            .id();

        commands.entity(entity).add_child(trigger);
    }
}

fn update_project_label(
    editor_state: Res<EditorState>,
    assets: Res<Assets<ParticleSystemAsset>>,
    triggers: Query<&Children, With<ProjectSelectorTrigger>>,
    mut texts: Query<&mut Text>,
) {
    if !editor_state.is_changed() && !assets.is_changed() {
        return;
    }

    let project_name = editor_state
        .current_project
        .as_ref()
        .and_then(|handle| assets.get(handle))
        .map(|asset| asset.name.as_str())
        .unwrap_or("Untitled");

    for children in &triggers {
        for child in children.iter() {
            if let Ok(mut text) = texts.get_mut(child) {
                **text = project_name.to_string();
                return;
            }
        }
    }
}

fn handle_trigger_click(
    trigger: On<ButtonClickEvent>,
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    editor_data: Res<EditorData>,
    triggers: Query<&ProjectSelectorTrigger>,
    mut states: Query<&mut ProjectSelectorState>,
    all_popovers: Query<Entity, With<EditorPopover>>,
) {
    let Ok(selector_trigger) = triggers.get(trigger.entity) else {
        return;
    };
    let Ok(mut state) = states.get_mut(selector_trigger.0) else {
        return;
    };

    if let Some(popover_entity) = state.popover {
        commands.entity(popover_entity).try_despawn();
        state.popover = None;
        return;
    }

    if !all_popovers.is_empty() {
        return;
    }

    let font: Handle<Font> = asset_server.load(FONT_PATH);

    let popover_entity = commands
        .spawn((
            ProjectSelectorPopover,
            popover(PopoverProps::new(trigger.entity)
                .with_placement(PopoverPlacement::BottomStart)
                .with_padding(6.0)
                .with_gap(6.0)
                .with_z_index(200)
                .with_node(Node {
                    min_width: px(200.0),
                    ..default()
                }),
            ),
        ))
        .id();

    state.popover = Some(popover_entity);

    let actions_wrapper = commands
        .spawn(Node {
            flex_direction: FlexDirection::Column,
            ..default()
        })
        .with_child(button(
            ButtonProps::new("New project...")
                .with_variant(ButtonVariant::Ghost)
                .align_left()
                .with_left_icon(ICON_NEW),
        ))
        .with_child((
            OpenProjectButton,
            button(
                ButtonProps::new("Open...")
                    .with_variant(ButtonVariant::Ghost)
                    .align_left()
                    .with_left_icon(ICON_OPEN),
            ),
        ))
        .with_child(button(
            ButtonProps::new("Examples")
                .with_variant(ButtonVariant::Ghost)
                .align_left()
                .with_left_icon(ICON_EXAMPLES),
        ))
        .id();

    commands.entity(popover_entity).add_child(actions_wrapper);

    commands.entity(popover_entity).with_child((
        Node {
            width: percent(100),
            height: px(1),
            ..default()
        },
        BackgroundColor(BORDER_COLOR.into()),
    ));

    commands.entity(popover_entity).with_child((
        Text::new("Recent projects"),
        TextFont {
            font,
            font_size: TEXT_SIZE_SM,
            weight: FontWeight::MEDIUM,
            ..default()
        },
        TextColor(TEXT_MUTED_COLOR.into()),
        Node::default(),
    ));

    let mut recent_wrapper = commands.spawn(Node {
        flex_direction: FlexDirection::Column,
        ..default()
    });

    for path_str in &editor_data.cache.recent_projects {
        let full_path = working_dir().join(path_str);
        let name = load_project_from_path(&full_path)
            .map(|asset| asset.name)
            .unwrap_or_else(|| {
                full_path
                    .file_stem()
                    .map(|s| s.to_string_lossy().to_string())
                    .unwrap_or_else(|| path_str.clone())
            });

        recent_wrapper.with_child((
            RecentProjectButton(path_str.clone()),
            button(
                ButtonProps::new(name)
                    .with_variant(ButtonVariant::Ghost)
                    .align_left()
                    .with_direction(FlexDirection::Column)
                    .with_subtitle(path_str),
            ),
        ));
    }

    let recent_wrapper_id = recent_wrapper.id();
    commands.entity(popover_entity).add_child(recent_wrapper_id);
}

fn handle_open_project_click(
    trigger: On<ButtonClickEvent>,
    buttons: Query<(), With<OpenProjectButton>>,
    mut commands: Commands,
) {
    if buttons.get(trigger.entity).is_err() {
        return;
    }
    commands.trigger(BrowseOpenProjectEvent);
}

fn handle_recent_project_click(
    trigger: On<ButtonClickEvent>,
    buttons: Query<&RecentProjectButton>,
    mut commands: Commands,
) {
    let Ok(recent) = buttons.get(trigger.entity) else {
        return;
    };
    commands.trigger(OpenProjectEvent(recent.0.clone()));
}

fn handle_popover_option_click(
    trigger: On<ButtonClickEvent>,
    mut commands: Commands,
    triggers: Query<(), With<ProjectSelectorTrigger>>,
    popovers: Query<Entity, With<ProjectSelectorPopover>>,
    parents: Query<&ChildOf>,
    mut states: Query<&mut ProjectSelectorState>,
) {
    if triggers.get(trigger.entity).is_ok() {
        return;
    }
    for popover_entity in &popovers {
        if is_descendant_of(trigger.entity, popover_entity, &parents) {
            commands.entity(popover_entity).try_despawn();
            for mut state in &mut states {
                state.popover = None;
            }
            return;
        }
    }
}

fn handle_popover_closed(
    mut states: Query<&mut ProjectSelectorState>,
    popovers: Query<Entity, With<EditorPopover>>,
) {
    for mut state in &mut states {
        let Some(popover_entity) = state.popover else {
            continue;
        };
        if popovers.get(popover_entity).is_err() {
            state.popover = None;
        }
    }
}
