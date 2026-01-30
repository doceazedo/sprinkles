use std::env;
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;

use bevy::prelude::*;
use bevy::tasks::IoTaskPool;
use serde::{Deserialize, Serialize};

use aracari::prelude::*;

#[derive(Resource, Default)]
pub struct EditorState {
    pub current_project: Option<Handle<ParticleSystemAsset>>,
    pub current_project_path: Option<PathBuf>,
    pub inspecting: Option<Inspecting>,
}

#[derive(Clone, Copy)]
pub struct Inspecting {
    pub kind: Inspectable,
    pub index: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Inspectable {
    Emitter,
    Collider,
}

/// triggered to reset playback (stop and return to beginning)
#[derive(Event)]
pub struct PlaybackResetEvent;

/// triggered to request play (used to restart completed one-shot emitters)
#[derive(Event)]
pub struct PlaybackPlayEvent;

/// triggered to seek to a specific time in seconds
#[derive(Event)]
pub struct PlaybackSeekEvent(pub f32);

#[derive(Resource, Serialize, Deserialize, Default)]
pub struct EditorData {
    pub cache: EditorCache,
}

#[derive(Serialize, Deserialize, Default)]
pub struct EditorCache {
    pub last_opened_project: Option<String>,
    pub recent_projects: Vec<String>,
}

impl EditorCache {
    const MAX_RECENT_PROJECTS: usize = 10;

    pub fn add_recent_project(&mut self, path: String) {
        let new_canonical = canonicalize_path(&path);
        self.recent_projects
            .retain(|p| canonicalize_path(p) != new_canonical);
        self.recent_projects.insert(0, path.clone());
        self.recent_projects.truncate(Self::MAX_RECENT_PROJECTS);
        self.last_opened_project = Some(path);
    }
}

fn working_dir() -> PathBuf {
    env::current_dir().unwrap_or_default()
}

fn canonicalize_path(path: &str) -> PathBuf {
    let path_buf = if path.starts_with("./") || path.starts_with(".\\") {
        working_dir().join(&path[2..])
    } else {
        PathBuf::from(path)
    };
    path_buf.canonicalize().unwrap_or(path_buf)
}

fn editor_data_path() -> PathBuf {
    working_dir().join("editor.ron")
}

pub fn project_path(relative_path: &str) -> PathBuf {
    working_dir().join(relative_path)
}

pub fn load_editor_data() -> EditorData {
    let path = editor_data_path();
    if path.exists() {
        std::fs::read_to_string(&path)
            .ok()
            .and_then(|contents| ron::from_str(&contents).ok())
            .unwrap_or_default()
    } else {
        EditorData::default()
    }
}

pub fn save_editor_data(data: &EditorData) {
    let path = editor_data_path();
    let Ok(contents) = ron::ser::to_string_pretty(data, ron::ser::PrettyConfig::default()) else {
        return;
    };

    IoTaskPool::get()
        .spawn(async move {
            let mut file = File::create(&path).expect("failed to create editor data file");
            file.write_all(contents.as_bytes())
                .expect("failed to write editor data");
        })
        .detach();
}

pub fn load_project_from_path(
    path: &std::path::Path,
) -> Option<aracari::asset::ParticleSystemAsset> {
    let contents = std::fs::read_to_string(path).ok()?;
    ron::from_str(&contents).ok()
}
