use std::path::PathBuf;

use aracari::prelude::*;
use bevy::prelude::*;
use bevy::window::PrimaryWindow;

pub fn plugin(app: &mut App) {
    app.init_resource::<EditorState>()
        .init_resource::<DirtyState>()
        .add_systems(PostStartup, update_window_title)
        .add_systems(Update, update_window_title);
}

#[derive(Resource, Default)]
pub struct EditorState {
    pub current_project: Option<Handle<ParticleSystemAsset>>,
    pub current_project_path: Option<PathBuf>,
    pub inspecting: Option<Inspecting>,
}

#[derive(Resource, Default)]
pub struct DirtyState {
    pub has_unsaved_changes: bool,
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

#[derive(Event)]
pub struct PlaybackResetEvent;

#[derive(Event)]
pub struct PlaybackPlayEvent;

#[derive(Event)]
pub struct PlaybackSeekEvent(pub f32);

fn update_window_title(
    editor_state: Res<EditorState>,
    dirty_state: Res<DirtyState>,
    assets: Res<Assets<ParticleSystemAsset>>,
    mut window: Query<&mut Window, With<PrimaryWindow>>,
) {
    if !editor_state.is_changed() && !dirty_state.is_changed() {
        return;
    }

    let Ok(mut window) = window.single_mut() else {
        return;
    };

    let project_name = editor_state
        .current_project
        .as_ref()
        .and_then(|handle| assets.get(handle))
        .map(|asset| asset.name.as_str())
        .unwrap_or("Untitled");

    let prefix = if dirty_state.has_unsaved_changes {
        "* "
    } else {
        ""
    };

    window.title = format!("{prefix}{project_name} - Aracari Editor");
}
