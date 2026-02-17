use std::fs::File;
use std::io::Write;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use bevy::prelude::*;
use bevy::tasks::IoTaskPool;
use bevy_sprinkles::asset::versioning::VersionStatus;
use bevy_sprinkles::prelude::*;
use inflector::Inflector;

use crate::io::{EditorData, project_path, projects_dir, save_editor_data, simplify_path};
use crate::state::{DirtyState, EditorState, Inspectable, Inspecting};
use crate::ui::components::toasts::ToastEvent;

pub fn plugin(app: &mut App) {
    app.add_observer(on_open_project_event)
        .add_observer(on_browse_open_project_event)
        .add_observer(on_save_project_event)
        .add_observer(on_save_project_as_event)
        .add_systems(
            Update,
            (
                handle_save_keyboard_shortcut,
                poll_browse_open_result,
                poll_save_as_result,
                poll_save_result,
            ),
        );
}

#[derive(Event)]
pub struct OpenProjectEvent(pub String);

#[derive(Event)]
pub struct BrowseOpenProjectEvent;

#[derive(Event)]
pub struct SaveProjectEvent;

#[derive(Event)]
pub struct SaveProjectAsEvent;

#[derive(Resource, Clone)]
pub struct BrowseOpenResult(pub Arc<Mutex<Option<PathBuf>>>);

#[derive(Resource, Clone)]
pub struct SaveAsResult(pub Arc<Mutex<Option<PathBuf>>>);

#[derive(Clone)]
pub enum SaveResultStatus {
    Success(String),
    SerializationError,
    WriteError(String),
    CreateError,
}

#[derive(Resource, Clone)]
pub struct SaveResult(pub Arc<Mutex<Option<SaveResultStatus>>>);

pub fn load_project_from_path(
    path: &std::path::Path,
) -> Option<bevy_sprinkles::asset::ParticleSystemAsset> {
    let contents = std::fs::read_to_string(path).ok()?;
    ron::from_str(&contents).ok()
}

fn on_open_project_event(
    event: On<OpenProjectEvent>,
    mut editor_state: ResMut<EditorState>,
    mut editor_data: ResMut<EditorData>,
    mut assets: ResMut<Assets<ParticleSystemAsset>>,
    mut dirty_state: ResMut<DirtyState>,
    mut commands: Commands,
) {
    let location = &event.0;
    let path = project_path(location);

    let Some(mut asset) = load_project_from_path(&path) else {
        commands.trigger(ToastEvent::error(format!(
            "Failed to open project: {location}"
        )));
        return;
    };

    let filename = path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| location.clone());

    match asset.try_upgrade_version() {
        VersionStatus::Current => {}
        VersionStatus::Outdated { current, .. } => {
            dirty_state.has_unsaved_changes = true;
            commands.trigger(ToastEvent::success(format!(
                "Project \"{filename}\" will be updated to {current} on the next save"
            )));
        }
        VersionStatus::Incompatible { found, current } => {
            commands.trigger(ToastEvent::error(format!(
                "Project \"{filename}\" (version {found}) is incompatible with Sprinkles {current}"
            )));
            return;
        }
        VersionStatus::Unknown => {
            commands.trigger(ToastEvent::error(format!(
                "Project \"{filename}\" has an unknown version. You may need to update Sprinkles."
            )));
            return;
        }
    }

    let has_emitters = !asset.emitters.is_empty();
    let handle = assets.add(asset);
    editor_state.open_project(handle, path, &mut dirty_state);
    editor_state.inspecting = if has_emitters {
        Some(Inspecting {
            kind: Inspectable::Emitter,
            index: 0,
        })
    } else {
        None
    };

    editor_data.cache.add_recent_project(location.clone());
    save_editor_data(&editor_data);
}

fn on_browse_open_project_event(_event: On<BrowseOpenProjectEvent>, mut commands: Commands) {
    let projects_dir = projects_dir();

    let path_result = Arc::new(Mutex::new(None));
    let path_result_clone = path_result.clone();

    let task = rfd::AsyncFileDialog::new()
        .set_title("Open Project")
        .set_directory(&projects_dir)
        .add_filter("RON files", &["ron"])
        .pick_file();

    IoTaskPool::get()
        .spawn(async move {
            if let Some(file_handle) = task.await {
                let path = file_handle.path().to_path_buf();
                if let Ok(mut guard) = path_result_clone.lock() {
                    *guard = Some(path);
                }
            }
        })
        .detach();

    commands.insert_resource(BrowseOpenResult(path_result));
}

fn poll_browse_open_result(result: Option<Res<BrowseOpenResult>>, mut commands: Commands) {
    let Some(result) = result else {
        return;
    };

    let path = {
        let Ok(mut guard) = result.0.lock() else {
            return;
        };
        guard.take()
    };

    if let Some(path) = path {
        commands.trigger(OpenProjectEvent(simplify_path(&path)));
        commands.remove_resource::<BrowseOpenResult>();
    }
}

pub fn save_project_to_path(
    path: PathBuf,
    asset: &bevy_sprinkles::asset::ParticleSystemAsset,
    result: Arc<Mutex<Option<SaveResultStatus>>>,
) {
    let Ok(contents) = ron::ser::to_string_pretty(asset, ron::ser::PrettyConfig::default()) else {
        if let Ok(mut guard) = result.lock() {
            *guard = Some(SaveResultStatus::SerializationError);
        }
        return;
    };

    IoTaskPool::get()
        .spawn(async move {
            let status = match File::create(&path) {
                Ok(mut file) => {
                    let filename = path
                        .file_name()
                        .map(|n| n.to_string_lossy().to_string())
                        .unwrap_or_else(|| "".to_string());
                    if file.write_all(contents.as_bytes()).is_ok() {
                        SaveResultStatus::Success(filename)
                    } else {
                        SaveResultStatus::WriteError(filename)
                    }
                }
                Err(_) => SaveResultStatus::CreateError,
            };
            if let Ok(mut guard) = result.lock() {
                *guard = Some(status);
            }
        })
        .detach();
}

fn on_save_project_event(
    _event: On<SaveProjectEvent>,
    editor_state: Res<EditorState>,
    assets: Res<Assets<ParticleSystemAsset>>,
    mut dirty_state: ResMut<DirtyState>,
    mut commands: Commands,
) {
    let Some(handle) = &editor_state.current_project else {
        return;
    };
    let Some(asset) = assets.get(handle) else {
        return;
    };

    if let Some(path) = &editor_state.current_project_path {
        let result = Arc::new(Mutex::new(None));
        save_project_to_path(path.clone(), asset, result.clone());
        commands.insert_resource(SaveResult(result));
        dirty_state.has_unsaved_changes = false;
    } else {
        commands.trigger(SaveProjectAsEvent);
    }
}

fn on_save_project_as_event(
    _event: On<SaveProjectAsEvent>,
    editor_state: Res<EditorState>,
    assets: Res<Assets<ParticleSystemAsset>>,
    mut commands: Commands,
) {
    let Some(handle) = &editor_state.current_project else {
        return;
    };
    let Some(asset) = assets.get(handle) else {
        return;
    };

    let projects_dir = projects_dir();
    let default_name = format!("{}.ron", asset.name.to_kebab_case());
    let asset_clone = asset.clone();

    let path_result = Arc::new(Mutex::new(None));
    let path_result_clone = path_result.clone();

    let save_result = Arc::new(Mutex::new(None));
    let save_result_clone = save_result.clone();

    let task = rfd::AsyncFileDialog::new()
        .set_title("Save Project As")
        .set_directory(&projects_dir)
        .set_file_name(&default_name)
        .add_filter("RON files", &["ron"])
        .save_file();

    IoTaskPool::get()
        .spawn(async move {
            if let Some(file_handle) = task.await {
                let path = file_handle.path().to_path_buf();
                save_project_to_path(path.clone(), &asset_clone, save_result_clone);
                if let Ok(mut guard) = path_result_clone.lock() {
                    *guard = Some(path);
                }
            }
        })
        .detach();

    commands.insert_resource(SaveAsResult(path_result));
    commands.insert_resource(SaveResult(save_result));
}

fn poll_save_as_result(
    result: Option<Res<SaveAsResult>>,
    mut editor_state: ResMut<EditorState>,
    mut editor_data: ResMut<EditorData>,
    mut dirty_state: ResMut<DirtyState>,
    mut commands: Commands,
) {
    let Some(result) = result else {
        return;
    };

    let path = {
        let Ok(mut guard) = result.0.lock() else {
            return;
        };
        guard.take()
    };

    if let Some(path) = path {
        editor_state.current_project_path = Some(path.clone());

        editor_data.cache.add_recent_project(simplify_path(&path));
        save_editor_data(&editor_data);
        dirty_state.has_unsaved_changes = false;
        commands.remove_resource::<SaveAsResult>();
    }
}

fn poll_save_result(result: Option<Res<SaveResult>>, mut commands: Commands) {
    let Some(result) = result else {
        return;
    };

    let status = {
        let Ok(mut guard) = result.0.lock() else {
            return;
        };
        guard.take()
    };

    if let Some(status) = status {
        match status {
            SaveResultStatus::Success(filename) => {
                commands.trigger(ToastEvent::success(format!("Saved \"{filename}\"")));
            }
            SaveResultStatus::SerializationError => {
                commands.trigger(ToastEvent::error("Cannot save project with invalid data"));
            }
            SaveResultStatus::WriteError(filename) => {
                commands.trigger(ToastEvent::error(format!(
                    "Failed to write to \"{filename}\""
                )));
            }
            SaveResultStatus::CreateError => {
                commands.trigger(ToastEvent::error("Failed to create project file"));
            }
        }
        commands.remove_resource::<SaveResult>();
    }
}

fn handle_save_keyboard_shortcut(keyboard: Res<ButtonInput<KeyCode>>, mut commands: Commands) {
    let ctrl_or_cmd = keyboard.pressed(KeyCode::SuperLeft)
        || keyboard.pressed(KeyCode::SuperRight)
        || keyboard.pressed(KeyCode::ControlLeft)
        || keyboard.pressed(KeyCode::ControlRight);

    if ctrl_or_cmd && keyboard.just_pressed(KeyCode::KeyS) {
        commands.trigger(SaveProjectEvent);
    }
}
