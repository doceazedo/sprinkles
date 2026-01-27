use std::collections::HashSet;
use std::env;
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use bevy::prelude::*;
use bevy::tasks::IoTaskPool;
use serde::{Deserialize, Serialize};

use aracari::prelude::*;

#[derive(Resource)]
pub struct EditorState {
    pub current_project: Option<Handle<ParticleSystemAsset>>,
    pub current_project_path: Option<PathBuf>,
    pub is_playing: bool,
    pub is_looping: bool,
    /// elapsed time in milliseconds
    pub elapsed_ms: f32,
    /// duration based on the longest emitter lifetime, in milliseconds
    pub duration_ms: f32,
    /// set to true when stop button is clicked, cleared after reset
    pub should_reset: bool,
    /// set to true when play button is clicked, cleared after processed
    pub play_requested: bool,
    /// tracks whether there are unsaved changes
    pub has_unsaved_changes: bool,
    /// true while save operation is in progress
    pub is_saving: bool,
    /// timestamp when save completed, used for "Saved!" label animation
    pub save_completed_at: Option<f64>,
    /// flag set by async save task when complete
    pub save_complete_flag: Option<Arc<AtomicBool>>,
}

impl EditorState {
    pub fn mark_unsaved(&mut self) {
        self.has_unsaved_changes = true;
    }

    pub fn check_save_completed(&mut self, current_time: f64) -> bool {
        if let Some(flag) = &self.save_complete_flag {
            if flag.load(Ordering::Relaxed) {
                self.is_saving = false;
                self.has_unsaved_changes = false;
                self.save_completed_at = Some(current_time);
                self.save_complete_flag = None;
                return true;
            }
        }
        false
    }

    pub fn project_name(&self, assets: &Assets<ParticleSystemAsset>) -> String {
        self.current_project
            .as_ref()
            .and_then(|handle| assets.get(handle))
            .map(|asset| asset.name.clone())
            .unwrap_or_else(|| "Untitled project".to_string())
    }
}

impl Default for EditorState {
    fn default() -> Self {
        Self {
            current_project: None,
            current_project_path: None,
            is_playing: true,
            is_looping: true,
            elapsed_ms: 0.0,
            duration_ms: 1000.0,
            should_reset: false,
            play_requested: false,
            has_unsaved_changes: false,
            is_saving: false,
            save_completed_at: None,
            save_complete_flag: None,
        }
    }
}

#[derive(Resource)]
pub struct InspectorState {
    pub editing_emitter_name: Option<usize>,
    pub collapsed_emitters: HashSet<usize>,
}

impl Default for InspectorState {
    fn default() -> Self {
        Self {
            editing_emitter_name: None,
            collapsed_emitters: HashSet::new(),
        }
    }
}

impl InspectorState {
    pub fn is_emitter_expanded(&self, index: usize) -> bool {
        !self.collapsed_emitters.contains(&index)
    }

    pub fn toggle_emitter(&mut self, index: usize) {
        if self.collapsed_emitters.contains(&index) {
            self.collapsed_emitters.remove(&index);
        } else {
            self.collapsed_emitters.insert(index);
        }
    }
}

pub const DEFAULT_PROJECTS_DIR: &str = "./projects";

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

    pub fn remove_recent_project(&mut self, path: &str) {
        self.recent_projects.retain(|p| p != path);
        if self.last_opened_project.as_deref() == Some(path) {
            self.last_opened_project = self.recent_projects.first().cloned();
        }
    }
}

pub fn format_display_path(path: &str) -> String {
    if path.starts_with("./") || path.starts_with(".\\") {
        return path.to_string();
    }

    let working_dir = working_dir();
    let full_path = working_dir.join(path);

    if let Ok(canonical) = full_path.canonicalize() {
        if let Ok(working_canonical) = working_dir.canonicalize() {
            if let Ok(relative) = canonical.strip_prefix(&working_canonical) {
                return format!("./{}", relative.display());
            }
        }
    }

    if let Ok(home) = env::var("HOME") {
        let home_dir = PathBuf::from(&home);
        let path_buf = PathBuf::from(path);
        if let Ok(stripped) = path_buf.strip_prefix(&home_dir) {
            return format!("~/{}", stripped.display());
        }
        if let Ok(stripped) = full_path.strip_prefix(&home_dir) {
            return format!("~/{}", stripped.display());
        }
    }

    path.to_string()
}

pub fn working_dir() -> PathBuf {
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
