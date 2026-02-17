use std::env;
use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};

use bevy::prelude::*;
use bevy::tasks::IoTaskPool;
use serde::{Deserialize, Serialize};

pub fn plugin(app: &mut App) {
    ensure_data_dirs();
    crate::assets::extract_examples(&examples_dir());
    let editor_data = load_editor_data();
    app.insert_resource(editor_data);
}

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
        let canonical = canonicalize_path(path);
        self.recent_projects
            .retain(|p| canonicalize_path(p) != canonical);
    }
}

pub fn data_dir() -> PathBuf {
    let home = env::var_os("HOME").map(PathBuf::from).unwrap_or_default();
    home.join(".sprinkles")
}

pub fn projects_dir() -> PathBuf {
    data_dir().join("projects")
}

pub fn examples_dir() -> PathBuf {
    data_dir().join("examples")
}

pub fn working_dir() -> PathBuf {
    env::current_dir().unwrap_or_default()
}

fn ensure_data_dirs() {
    let _ = std::fs::create_dir_all(projects_dir());
    let _ = std::fs::create_dir_all(examples_dir());
}

fn canonicalize_path(path: &str) -> PathBuf {
    let stripped = path
        .strip_prefix("./")
        .or_else(|| path.strip_prefix(".\\"))
        .unwrap_or(path);
    let path_buf = project_path(stripped);
    path_buf.canonicalize().unwrap_or(path_buf)
}

fn editor_data_path() -> PathBuf {
    data_dir().join("editor.ron")
}

pub fn project_path(relative_path: &str) -> PathBuf {
    let p = PathBuf::from(relative_path);
    if p.is_absolute() {
        p
    } else {
        data_dir().join(relative_path)
    }
}

pub fn simplify_path(path: &Path) -> String {
    if let Ok(relative) = path.strip_prefix(data_dir()) {
        return relative.to_string_lossy().to_string();
    }

    #[cfg(unix)]
    if let Some(home) = env::var_os("HOME") {
        let home_path = PathBuf::from(home);
        if let Ok(relative) = path.strip_prefix(&home_path) {
            return format!("~/{}", relative.to_string_lossy());
        }
    }

    path.to_string_lossy().to_string()
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
