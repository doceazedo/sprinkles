use std::env;
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;

use bevy::prelude::*;
use bevy::tasks::IoTaskPool;
use serde::{Deserialize, Serialize};

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
        self.recent_projects.retain(|p| p != &path);
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

/// Formats a path for display in the UI.
/// - Paths within the working directory are shown as relative (./path)
/// - Home directory paths are shortened to ~/path
/// - Other absolute paths are shown as-is
pub fn format_display_path(path: &str) -> String {
    // if it starts with ./ it's already relative to working dir
    if path.starts_with("./") || path.starts_with(".\\") {
        return path.to_string();
    }

    let working_dir = working_dir();
    let full_path = working_dir.join(path);

    // check if it's within the working directory
    if let Ok(canonical) = full_path.canonicalize() {
        if let Ok(working_canonical) = working_dir.canonicalize() {
            if let Ok(relative) = canonical.strip_prefix(&working_canonical) {
                return format!("./{}", relative.display());
            }
        }
    }

    // try to shorten home directory to ~/
    if let Ok(home) = env::var("HOME") {
        let home_dir = PathBuf::from(&home);
        let path_buf = PathBuf::from(path);
        if let Ok(stripped) = path_buf.strip_prefix(&home_dir) {
            return format!("~/{}", stripped.display());
        }
        // also check the full path
        if let Ok(stripped) = full_path.strip_prefix(&home_dir) {
            return format!("~/{}", stripped.display());
        }
    }

    path.to_string()
}

pub fn working_dir() -> PathBuf {
    env::current_dir().unwrap_or_default()
}

pub fn editor_data_path() -> PathBuf {
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
