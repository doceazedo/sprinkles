use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use bevy::input_focus::InputFocus;
use bevy::picking::hover::Hovered;
use bevy::prelude::*;
use bevy::tasks::IoTaskPool;
use bevy_ui_text_input::{
    TextInputBuffer, TextInputPrompt, TextInputQueue,
    actions::{TextInputAction, TextInputEdit},
};

use aracari::prelude::*;

use crate::io::{EditorData, project_path, save_editor_data, working_dir};
use crate::project::{
    BrowseOpenProjectEvent, OpenProjectEvent, SaveResult, load_project_from_path,
    save_project_to_path,
};
use crate::state::{DirtyState, EditorState, Inspectable, Inspecting};
use crate::ui::icons::{
    ICON_ARROW_DOWN, ICON_CLOSE, ICON_FILE_ADD, ICON_FOLDER_IMAGE, ICON_FOLDER_OPEN,
};
use crate::ui::tokens::{
    BORDER_COLOR, FONT_PATH, TEXT_BODY_COLOR, TEXT_MUTED_COLOR, TEXT_SIZE, TEXT_SIZE_SM,
};
use crate::ui::widgets::button::{
    ButtonClickEvent, ButtonProps, ButtonSize, ButtonVariant, IconButtonProps, button, icon_button,
};
use crate::ui::widgets::dialog::{
    DialogActionEvent, DialogChildrenSlot, EditorDialog, OpenDialogEvent,
};
use crate::ui::widgets::popover::{EditorPopover, PopoverPlacement, PopoverProps, popover};
use crate::ui::widgets::text_edit::{EditorTextEdit, TextEditProps, text_edit};
use crate::ui::widgets::utils::is_descendant_of;

pub fn plugin(app: &mut App) {
    app.add_observer(handle_trigger_click)
        .add_observer(handle_new_project_click)
        .add_observer(handle_open_project_click)
        .add_observer(handle_recent_project_click)
        .add_observer(handle_remove_recent_project_click)
        .add_observer(handle_popover_option_click)
        .add_observer(handle_create_project)
        .add_observer(handle_browse_location_click)
        .add_systems(
            Update,
            (
                setup_project_selector,
                update_project_label,
                handle_popover_closed,
                setup_new_project_dialog_content,
                focus_new_project_name,
                update_location_placeholder,
                poll_browse_location_result,
                cleanup_new_project_state,
                update_remove_button_visibility,
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
struct NewProjectButton;

#[derive(Component)]
struct OpenProjectButton;

#[derive(Component)]
struct RecentProjectButton(String);

#[derive(Component)]
struct RecentProjectRow;

#[derive(Component)]
struct RemoveRecentProjectButton(String);

#[derive(Component)]
struct NewProjectNameInput;

#[derive(Component)]
struct NewProjectLocationInput;

#[derive(Component)]
struct BrowseLocationButton;

#[derive(Resource)]
struct BrowseLocationResult(Arc<Mutex<Option<PathBuf>>>);

#[derive(Resource)]
struct NewProjectDialogState {
    default_name: String,
    default_slug: String,
    name_entity: Option<Entity>,
    location_entity: Option<Entity>,
    focused: bool,
}

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
                        .with_right_icon(ICON_ARROW_DOWN),
                ),
            ))
            .id();

        commands.entity(entity).add_child(trigger);
    }
}

fn update_project_label(
    editor_state: Res<EditorState>,
    dirty_state: Res<DirtyState>,
    assets: Res<Assets<ParticleSystemAsset>>,
    triggers: Query<&Children, With<ProjectSelectorTrigger>>,
    mut texts: Query<&mut Text>,
) {
    if !editor_state.is_changed() && !dirty_state.is_changed() && !assets.is_changed() {
        return;
    }

    let project_name = editor_state
        .current_project
        .as_ref()
        .and_then(|handle| assets.get(handle))
        .map(|asset| asset.name.as_str())
        .unwrap_or("Untitled");

    let prefix = if dirty_state.has_unsaved_changes {
        "* "
    } else {
        ""
    };

    for children in &triggers {
        for child in children.iter() {
            if let Ok(mut text) = texts.get_mut(child) {
                **text = format!("{prefix}{project_name}");
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
            popover(
                PopoverProps::new(trigger.entity)
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
        .with_child((
            NewProjectButton,
            button(
                ButtonProps::new("New project...")
                    .with_variant(ButtonVariant::Ghost)
                    .align_left()
                    .with_left_icon(ICON_FILE_ADD),
            ),
        ))
        .with_child((
            OpenProjectButton,
            button(
                ButtonProps::new("Open...")
                    .with_variant(ButtonVariant::Ghost)
                    .align_left()
                    .with_left_icon(ICON_FOLDER_OPEN),
            ),
        ))
        .with_child(button(
            ButtonProps::new("Examples")
                .with_variant(ButtonVariant::Ghost)
                .align_left()
                .with_left_icon(ICON_FOLDER_IMAGE),
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

    let recent_wrapper_id = commands
        .spawn(Node {
            flex_direction: FlexDirection::Column,
            ..default()
        })
        .id();

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

        let project_button = commands
            .spawn((
                RecentProjectButton(path_str.clone()),
                button(
                    ButtonProps::new(name)
                        .with_variant(ButtonVariant::Ghost)
                        .align_left()
                        .with_direction(FlexDirection::Column)
                        .with_subtitle(path_str),
                ),
            ))
            .id();

        commands
            .entity(project_button)
            .entry::<Node>()
            .and_modify(|mut node| {
                node.flex_grow = 1.0;
            });

        let remove_button = commands
            .spawn((
                RemoveRecentProjectButton(path_str.clone()),
                icon_button(
                    IconButtonProps::new(ICON_CLOSE)
                        .variant(ButtonVariant::Ghost)
                        .with_size(ButtonSize::IconSM),
                    &asset_server,
                ),
                Visibility::Hidden,
            ))
            .id();

        commands
            .entity(remove_button)
            .entry::<Node>()
            .and_modify(|mut node| {
                node.flex_shrink = 0.0;
            });

        let row = commands
            .spawn((
                RecentProjectRow,
                Hovered::default(),
                Node {
                    align_items: AlignItems::Center,
                    column_gap: px(6.0),
                    ..default()
                },
            ))
            .add_children(&[project_button, remove_button])
            .id();

        commands.entity(recent_wrapper_id).add_child(row);
    }

    commands.entity(popover_entity).add_child(recent_wrapper_id);
}

fn handle_new_project_click(
    trigger: On<ButtonClickEvent>,
    buttons: Query<(), With<NewProjectButton>>,
    mut commands: Commands,
) {
    if buttons.get(trigger.entity).is_err() {
        return;
    }

    let (default_name, default_slug) = next_untitled_name();
    commands.insert_resource(NewProjectDialogState {
        default_name,
        default_slug,
        name_entity: None,
        location_entity: None,
        focused: false,
    });

    commands.trigger(OpenDialogEvent::new("New project", "Create"));
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

fn handle_remove_recent_project_click(
    trigger: On<ButtonClickEvent>,
    buttons: Query<&RemoveRecentProjectButton>,
    mut editor_data: ResMut<EditorData>,
    parents: Query<&ChildOf>,
    mut commands: Commands,
) {
    let Ok(remove_btn) = buttons.get(trigger.entity) else {
        return;
    };

    editor_data.cache.remove_recent_project(&remove_btn.0);
    save_editor_data(&editor_data);

    if let Ok(child_of) = parents.get(trigger.entity) {
        commands.entity(child_of.parent()).try_despawn();
    }
}

fn update_remove_button_visibility(
    rows: Query<(&Children, &Hovered), (With<RecentProjectRow>, Changed<Hovered>)>,
    mut remove_buttons: Query<&mut Visibility, With<RemoveRecentProjectButton>>,
) {
    for (children, hovered) in &rows {
        let visible = hovered.get();
        for child in children.iter() {
            if let Ok(mut visibility) = remove_buttons.get_mut(child) {
                *visibility = if visible {
                    Visibility::Inherited
                } else {
                    Visibility::Hidden
                };
            }
        }
    }
}

fn handle_popover_option_click(
    trigger: On<ButtonClickEvent>,
    mut commands: Commands,
    triggers: Query<(), With<ProjectSelectorTrigger>>,
    remove_buttons: Query<(), With<RemoveRecentProjectButton>>,
    popovers: Query<Entity, With<ProjectSelectorPopover>>,
    parents: Query<&ChildOf>,
    mut states: Query<&mut ProjectSelectorState>,
) {
    if triggers.get(trigger.entity).is_ok() {
        return;
    }
    if remove_buttons.get(trigger.entity).is_ok() {
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

fn next_untitled_name() -> (String, String) {
    let projects_dir = project_path("projects");
    if !projects_dir.join("untitled-project.ron").exists() {
        return (
            "Untitled project".to_string(),
            "untitled-project".to_string(),
        );
    }

    let mut n = 2u32;
    while projects_dir
        .join(format!("untitled-project-{n}.ron"))
        .exists()
    {
        n += 1;
    }
    (
        format!("Untitled project {n}"),
        format!("untitled-project-{n}"),
    )
}

fn slugify(name: &str) -> String {
    let mut result = String::new();
    let mut prev_hyphen = true;
    for c in name.chars() {
        if c.is_alphanumeric() {
            result.push(c.to_ascii_lowercase());
            prev_hyphen = false;
        } else if !prev_hyphen {
            result.push('-');
            prev_hyphen = true;
        }
    }
    result.trim_end_matches('-').to_string()
}

fn resolve_location_path(raw: &str, slug: &str) -> PathBuf {
    let expanded = if raw.starts_with("~/") {
        #[cfg(unix)]
        {
            std::env::var_os("HOME")
                .map(|home| PathBuf::from(home).join(&raw[2..]))
                .unwrap_or_else(|| PathBuf::from(raw))
        }
        #[cfg(not(unix))]
        {
            PathBuf::from(raw)
        }
    } else if raw.starts_with("./") || raw.starts_with("../") {
        working_dir().join(raw)
    } else if PathBuf::from(raw).is_absolute() {
        PathBuf::from(raw)
    } else {
        working_dir().join(raw)
    };

    let filename = format!("{slug}.ron");
    expanded.join(filename)
}

fn find_inner_text_edit(
    entity: Entity,
    children_query: &Query<&Children>,
    text_edits: &Query<Entity, With<EditorTextEdit>>,
) -> Option<Entity> {
    if text_edits.get(entity).is_ok() {
        return Some(entity);
    }
    let Ok(children) = children_query.get(entity) else {
        return None;
    };
    for child in children.iter() {
        if let Some(found) = find_inner_text_edit(child, children_query, text_edits) {
            return Some(found);
        }
    }
    None
}

fn setup_new_project_dialog_content(
    state: Option<ResMut<NewProjectDialogState>>,
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    slots: Query<Entity, With<DialogChildrenSlot>>,
) {
    let Some(mut state) = state else { return };
    if state.name_entity.is_some() {
        return;
    }
    let Ok(slot_entity) = slots.single() else {
        return;
    };

    let font: Handle<Font> = asset_server.load(FONT_PATH);
    let location_placeholder = format!("./projects/{}", state.default_slug);

    let name_input = commands
        .spawn((
            NewProjectNameInput,
            text_edit(TextEditProps::default().with_placeholder(&state.default_name)),
        ))
        .id();

    let location_text_edit = commands
        .spawn(text_edit(
            TextEditProps::default().with_placeholder(&location_placeholder),
        ))
        .id();

    let browse_button = commands
        .spawn((
            BrowseLocationButton,
            icon_button(
                IconButtonProps::new(ICON_FOLDER_OPEN)
                    .variant(ButtonVariant::Ghost)
                    .with_size(ButtonSize::IconSM),
                &asset_server,
            ),
        ))
        .id();

    commands
        .entity(browse_button)
        .entry::<Node>()
        .and_modify(|mut node| {
            node.position_type = PositionType::Absolute;
            node.right = px(2);
            node.top = px(2);
        });

    let location_text_edit_wrapper = commands
        .spawn(Node {
            flex_grow: 1.0,
            flex_shrink: 1.0,
            flex_basis: px(0),
            ..default()
        })
        .id();

    commands
        .entity(location_text_edit_wrapper)
        .add_children(&[location_text_edit, browse_button]);

    let ron_label = commands
        .spawn((
            Text::new(".ron"),
            TextFont {
                font: font.clone(),
                font_size: TEXT_SIZE,
                ..default()
            },
            TextColor(TEXT_MUTED_COLOR.into()),
        ))
        .id();

    let location_input = commands
        .spawn((
            NewProjectLocationInput,
            Node {
                align_items: AlignItems::Center,
                column_gap: px(6),
                ..default()
            },
        ))
        .id();

    commands
        .entity(location_input)
        .add_children(&[location_text_edit_wrapper, ron_label]);

    let mut grid = commands.spawn(Node {
        display: Display::Grid,
        grid_template_columns: vec![GridTrack::max_content(), GridTrack::fr(1.0)],
        column_gap: px(12),
        row_gap: px(6),
        align_items: AlignItems::Center,
        ..default()
    });

    grid.with_child((
        Text::new("Project name"),
        TextFont {
            font: font.clone(),
            font_size: TEXT_SIZE,
            weight: FontWeight::MEDIUM,
            ..default()
        },
        TextColor(TEXT_BODY_COLOR.into()),
    ));
    grid.add_child(name_input);
    grid.with_child((
        Text::new("Location"),
        TextFont {
            font,
            font_size: TEXT_SIZE,
            weight: FontWeight::MEDIUM,
            ..default()
        },
        TextColor(TEXT_BODY_COLOR.into()),
    ));
    grid.add_child(location_input);

    let grid_id = grid.id();
    commands.entity(slot_entity).add_child(grid_id);

    state.name_entity = Some(name_input);
    state.location_entity = Some(location_input);
}

fn focus_new_project_name(
    state: Option<ResMut<NewProjectDialogState>>,
    mut focus: ResMut<InputFocus>,
    children_query: Query<&Children>,
    text_edits: Query<Entity, With<EditorTextEdit>>,
) {
    let Some(mut state) = state else { return };
    if state.focused {
        return;
    }
    let Some(name_entity) = state.name_entity else {
        return;
    };

    if let Some(inner) = find_inner_text_edit(name_entity, &children_query, &text_edits) {
        focus.0 = Some(inner);
        state.focused = true;
    }
}

fn update_location_placeholder(
    state: Option<Res<NewProjectDialogState>>,
    children_query: Query<&Children>,
    text_edits: Query<Entity, With<EditorTextEdit>>,
    buffers: Query<&TextInputBuffer>,
    mut prompts: Query<&mut TextInputPrompt>,
) {
    let Some(state) = state else { return };
    let Some(name_entity) = state.name_entity else {
        return;
    };
    let Some(location_entity) = state.location_entity else {
        return;
    };

    let Some(name_inner) = find_inner_text_edit(name_entity, &children_query, &text_edits) else {
        return;
    };
    let Some(location_inner) = find_inner_text_edit(location_entity, &children_query, &text_edits)
    else {
        return;
    };

    let Ok(buffer) = buffers.get(name_inner) else {
        return;
    };
    let name_text = buffer.get_text();

    let slug = if name_text.is_empty() {
        state.default_slug.clone()
    } else {
        let s = slugify(&name_text);
        if s.is_empty() {
            state.default_slug.clone()
        } else {
            s
        }
    };

    let new_placeholder = format!("./projects/{}", slug);
    if let Ok(mut prompt) = prompts.get_mut(location_inner) {
        if prompt.text != new_placeholder {
            prompt.text = new_placeholder;
        }
    }
}

fn handle_create_project(
    _event: On<DialogActionEvent>,
    state: Option<Res<NewProjectDialogState>>,
    mut editor_state: ResMut<EditorState>,
    mut editor_data: ResMut<EditorData>,
    mut assets: ResMut<Assets<ParticleSystemAsset>>,
    mut dirty_state: ResMut<DirtyState>,
    children_query: Query<&Children>,
    text_edits: Query<Entity, With<EditorTextEdit>>,
    buffers: Query<&TextInputBuffer>,
    mut commands: Commands,
) {
    let Some(state) = state else { return };
    let Some(name_entity) = state.name_entity else {
        return;
    };
    let Some(location_entity) = state.location_entity else {
        return;
    };

    let name = find_inner_text_edit(name_entity, &children_query, &text_edits)
        .and_then(|e| buffers.get(e).ok())
        .map(|b| b.get_text().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| state.default_name.clone());

    let location_raw = find_inner_text_edit(location_entity, &children_query, &text_edits)
        .and_then(|e| buffers.get(e).ok())
        .map(|b| b.get_text().to_string())
        .filter(|s| !s.is_empty());

    let slug = slugify(&name);
    let slug = if slug.is_empty() {
        &state.default_slug
    } else {
        &slug
    };

    let path = match location_raw {
        Some(raw) => resolve_location_path(&raw, slug),
        None => project_path(&format!("projects/{slug}.ron")),
    };
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }

    let asset = aracari::asset::ParticleSystemAsset {
        name,
        dimension: aracari::asset::ParticleSystemDimension::D3,
        emitters: vec![aracari::asset::EmitterData {
            name: "Emitter 1".to_string(),
            ..Default::default()
        }],
        colliders: vec![],
    };

    let result = Arc::new(Mutex::new(None));
    save_project_to_path(path.clone(), &asset, result.clone());
    commands.insert_resource(SaveResult(result));

    let handle = assets.add(asset);
    editor_state.current_project = Some(handle);
    editor_state.current_project_path = Some(path.clone());
    editor_state.inspecting = Some(Inspecting {
        kind: Inspectable::Emitter,
        index: 0,
    });
    dirty_state.has_unsaved_changes = false;

    let location = path
        .strip_prefix(working_dir())
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|_| path.to_string_lossy().to_string());
    editor_data.cache.add_recent_project(location);
    save_editor_data(&editor_data);

    commands.remove_resource::<NewProjectDialogState>();
}

fn handle_browse_location_click(
    trigger: On<ButtonClickEvent>,
    buttons: Query<(), With<BrowseLocationButton>>,
    mut commands: Commands,
) {
    if buttons.get(trigger.entity).is_err() {
        return;
    }

    let projects_dir = project_path("projects");

    let path_result = Arc::new(Mutex::new(None));
    let path_result_clone = path_result.clone();

    let task = rfd::AsyncFileDialog::new()
        .set_title("Select Location")
        .set_directory(&projects_dir)
        .pick_folder();

    IoTaskPool::get()
        .spawn(async move {
            if let Some(handle) = task.await {
                let path = handle.path().to_path_buf();
                if let Ok(mut guard) = path_result_clone.lock() {
                    *guard = Some(path);
                }
            }
        })
        .detach();

    commands.insert_resource(BrowseLocationResult(path_result));
}

fn simplify_path(path: &PathBuf) -> String {
    let cwd = working_dir();
    if let Ok(relative) = path.strip_prefix(&cwd) {
        return format!("./{}", relative.to_string_lossy());
    }

    #[cfg(unix)]
    if let Some(home) = std::env::var_os("HOME") {
        let home_path = PathBuf::from(home);
        if let Ok(relative) = path.strip_prefix(&home_path) {
            return format!("~/{}", relative.to_string_lossy());
        }
    }

    path.to_string_lossy().to_string()
}

fn poll_browse_location_result(
    result: Option<Res<BrowseLocationResult>>,
    state: Option<Res<NewProjectDialogState>>,
    children_query: Query<&Children>,
    text_edits: Query<Entity, With<EditorTextEdit>>,
    buffers: Query<&TextInputBuffer>,
    mut queues: Query<&mut TextInputQueue>,
    mut commands: Commands,
) {
    let Some(result) = result else { return };

    let path = {
        let Ok(mut guard) = result.0.lock() else {
            return;
        };
        guard.take()
    };

    let Some(path) = path else { return };

    commands.remove_resource::<BrowseLocationResult>();

    let Some(state) = state else { return };
    let Some(name_entity) = state.name_entity else {
        return;
    };
    let Some(location_entity) = state.location_entity else {
        return;
    };

    let slug = find_inner_text_edit(name_entity, &children_query, &text_edits)
        .and_then(|e| buffers.get(e).ok())
        .map(|b| slugify(&b.get_text()))
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| state.default_slug.clone());

    let Some(inner) = find_inner_text_edit(location_entity, &children_query, &text_edits) else {
        return;
    };
    let Ok(mut queue) = queues.get_mut(inner) else {
        return;
    };

    let dir_path = simplify_path(&path);
    let display_path = format!("{dir_path}/{slug}");
    queue.add(TextInputAction::Edit(TextInputEdit::SelectAll));
    queue.add(TextInputAction::Edit(TextInputEdit::Paste(display_path)));
}

fn cleanup_new_project_state(
    state: Option<Res<NewProjectDialogState>>,
    dialogs: Query<(), With<EditorDialog>>,
    mut commands: Commands,
) {
    if state.is_some() && dialogs.is_empty() {
        commands.remove_resource::<NewProjectDialogState>();
    }
}
