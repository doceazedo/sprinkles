use std::env;
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;

use bevy::prelude::*;
use bevy::tasks::IoTaskPool;
use serde::{Deserialize, Serialize};

pub fn plugin(app: &mut App) {
    ensure_data_dirs();
    crate::assets::extract_examples(&examples_dir());
    let editor_data = load_editor_data();
    app.register_type::<EditorSettings>()
        .register_type::<EditorTonemapping>()
        .register_type::<EditorBloom>()
        .register_type::<EditorSmaaPreset>()
        .insert_resource(editor_data);
}

#[derive(Resource, Serialize, Deserialize, Default)]
pub struct EditorData {
    pub cache: EditorCache,
    #[serde(default)]
    pub settings: EditorSettings,
}

#[derive(Serialize, Deserialize, Reflect, Clone)]
pub struct EditorSettings {
    #[serde(default = "default_show_fps")]
    pub show_fps: bool,
    #[serde(default = "default_vsync")]
    pub vsync: bool,
    #[serde(default = "default_tonemapping")]
    pub tonemapping: Option<EditorTonemapping>,
    #[serde(default = "default_bloom")]
    pub bloom: Option<EditorBloom>,
    #[serde(default = "default_anti_aliasing")]
    pub anti_aliasing: Option<EditorSmaaPreset>,
}

fn default_show_fps() -> bool {
    true
}

fn default_vsync() -> bool {
    true
}

fn default_tonemapping() -> Option<EditorTonemapping> {
    Some(EditorTonemapping::TonyMcMapface)
}

fn default_bloom() -> Option<EditorBloom> {
    Some(EditorBloom::Natural)
}

fn default_anti_aliasing() -> Option<EditorSmaaPreset> {
    Some(EditorSmaaPreset::High)
}

impl Default for EditorSettings {
    fn default() -> Self {
        Self {
            show_fps: default_show_fps(),
            vsync: default_vsync(),
            tonemapping: default_tonemapping(),
            bloom: default_bloom(),
            anti_aliasing: default_anti_aliasing(),
        }
    }
}

#[derive(Serialize, Deserialize, Reflect, Clone, Default, PartialEq)]
pub enum EditorTonemapping {
    #[default]
    Reinhard,
    ReinhardLuminance,
    AcesFitted,
    AgX,
    SomewhatBoringDisplayTransform,
    TonyMcMapface,
    BlenderFilmic,
}

#[derive(Serialize, Deserialize, Reflect, Clone, Default, PartialEq)]
pub enum EditorBloom {
    #[default]
    Natural,
    Anamorphic,
    OldSchool,
    ScreenBlur,
}

#[derive(Serialize, Deserialize, Reflect, Clone, Default, PartialEq)]
pub enum EditorSmaaPreset {
    Low,
    Medium,
    #[default]
    High,
    Ultra,
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
    #[cfg(unix)]
    let home = env::var_os("HOME").map(PathBuf::from).unwrap_or_default();
    #[cfg(not(unix))]
    let home = env::var_os("USERPROFILE").map(PathBuf::from).unwrap_or_default();
    home.join(".sprinkles")
}

pub fn projects_dir() -> PathBuf {
    data_dir().join("projects")
}

pub fn examples_dir() -> PathBuf {
    data_dir().join("examples")
}

pub fn is_example_path(path: &std::path::Path) -> bool {
    path.starts_with(examples_dir())
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
    if relative_path.starts_with("~/") {
        #[cfg(unix)]
        let home = env::var_os("HOME").map(PathBuf::from);
        #[cfg(not(unix))]
        let home = env::var_os("USERPROFILE").map(PathBuf::from);
        home.map(|h| h.join(&relative_path[2..]))
            .unwrap_or_else(|| PathBuf::from(relative_path))
    } else {
        PathBuf::from(relative_path)
    }
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
