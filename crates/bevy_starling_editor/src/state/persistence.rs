use std::fs::File;
use std::io::Write;
use std::path::PathBuf;

use bevy::asset::io::file::FileAssetReader;
use bevy::prelude::*;
use bevy::tasks::IoTaskPool;
use serde::{Deserialize, Serialize};

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

    pub fn add_recent_project(&mut self, file_name: String) {
        self.recent_projects.retain(|p| p != &file_name);
        self.recent_projects.insert(0, file_name.clone());
        self.recent_projects.truncate(Self::MAX_RECENT_PROJECTS);
        self.last_opened_project = Some(file_name);
    }
}

pub fn editor_data_dir() -> PathBuf {
    FileAssetReader::get_base_path()
}

pub fn editor_data_path() -> PathBuf {
    editor_data_dir().join("editor.ron")
}

pub fn assets_dir() -> PathBuf {
    editor_data_dir().join("assets")
}

pub fn project_path(file_name: &str) -> PathBuf {
    assets_dir().join(file_name)
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
