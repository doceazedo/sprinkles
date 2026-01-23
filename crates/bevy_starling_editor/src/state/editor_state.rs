use std::path::PathBuf;

use bevy::prelude::*;
use bevy_starling::asset::ParticleSystemAsset;

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
