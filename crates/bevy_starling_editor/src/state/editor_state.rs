use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use bevy::prelude::*;
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

impl EditorState {
    pub fn project_name(&self, assets: &Assets<ParticleSystemAsset>) -> String {
        self.current_project
            .as_ref()
            .and_then(|handle| assets.get(handle))
            .map(|asset| asset.name.clone())
            .unwrap_or_else(|| "Untitled project".to_string())
    }
}
