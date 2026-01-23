use std::path::PathBuf;

use bevy::prelude::*;
use bevy_starling::asset::ParticleSystemAsset;

#[derive(Resource)]
pub struct EditorState {
    pub current_project: Option<Handle<ParticleSystemAsset>>,
    pub current_project_path: Option<PathBuf>,
    pub is_playing: bool,
    pub is_looping: bool,
    pub current_frame: u32,
    pub total_frames: u32,
}

impl Default for EditorState {
    fn default() -> Self {
        Self {
            current_project: None,
            current_project_path: None,
            is_playing: true,
            is_looping: true,
            current_frame: 0,
            total_frames: 300,
        }
    }
}

impl EditorState {
    pub fn project_name(&self, assets: &Assets<ParticleSystemAsset>) -> String {
        self.current_project
            .as_ref()
            .and_then(|handle| assets.get(handle))
            .map(|asset| asset.name.clone())
            .unwrap_or_else(|| "Untitled project".to_string())
    }
}
