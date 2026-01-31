use std::fs::File;
use std::io::Write;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use aracari::prelude::*;
use bevy::prelude::*;
use bevy::tasks::IoTaskPool;

use crate::io::{project_path, save_editor_data, working_dir, EditorData};
use crate::state::{DirtyState, EditorState};

pub fn plugin(app: &mut App) {
    app.add_observer(on_project_save_event)
        .add_observer(on_project_save_as_event)
        .add_systems(Update, (handle_save_keyboard_shortcut, poll_save_as_result));
}

#[derive(Event)]
pub struct ProjectSaveEvent;

#[derive(Event)]
pub struct ProjectSaveAsEvent;

#[derive(Resource, Clone)]
pub struct SaveAsResult(pub Arc<Mutex<Option<PathBuf>>>);

pub fn load_project_from_path(
    path: &std::path::Path,
) -> Option<aracari::asset::ParticleSystemAsset> {
    let contents = std::fs::read_to_string(path).ok()?;
    ron::from_str(&contents).ok()
}

pub fn save_project_to_path(path: PathBuf, asset: &aracari::asset::ParticleSystemAsset) {
    let Ok(contents) = ron::ser::to_string_pretty(asset, ron::ser::PrettyConfig::default()) else {
        println!("TODO: implement error toast - failed to serialize project");
        return;
    };

    IoTaskPool::get()
        .spawn(async move {
            match File::create(&path) {
                Ok(mut file) => {
                    if file.write_all(contents.as_bytes()).is_ok() {
                        println!("TODO: implement saved successfully toast");
                    } else {
                        println!("TODO: implement error toast - failed to write project file");
                    }
                }
                Err(_) => {
                    println!("TODO: implement error toast - failed to create project file");
                }
            }
        })
        .detach();
}

fn on_project_save_event(
    _event: On<ProjectSaveEvent>,
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
        save_project_to_path(path.clone(), asset);
        dirty_state.has_unsaved_changes = false;
    } else {
        commands.trigger(ProjectSaveAsEvent);
    }
}

fn on_project_save_as_event(
    _event: On<ProjectSaveAsEvent>,
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

    let projects_dir = project_path("projects");
    let default_name = format!("{}.ron", asset.name);
    let asset_clone = asset.clone();

    let result = Arc::new(Mutex::new(None));
    let result_clone = result.clone();

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
                save_project_to_path(path.clone(), &asset_clone);
                if let Ok(mut guard) = result_clone.lock() {
                    *guard = Some(path);
                }
            }
        })
        .detach();

    commands.insert_resource(SaveAsResult(result));
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

        let relative = path
            .strip_prefix(working_dir())
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|_| path.to_string_lossy().to_string());

        editor_data.cache.add_recent_project(relative);
        save_editor_data(&editor_data);
        dirty_state.has_unsaved_changes = false;
        commands.remove_resource::<SaveAsResult>();
    }
}

fn handle_save_keyboard_shortcut(keyboard: Res<ButtonInput<KeyCode>>, mut commands: Commands) {
    let ctrl_or_cmd = keyboard.pressed(KeyCode::SuperLeft)
        || keyboard.pressed(KeyCode::SuperRight)
        || keyboard.pressed(KeyCode::ControlLeft)
        || keyboard.pressed(KeyCode::ControlRight);

    if ctrl_or_cmd && keyboard.just_pressed(KeyCode::KeyS) {
        commands.trigger(ProjectSaveEvent);
    }
}
