use aracari::prelude::*;
use bevy::color::palettes::tailwind::ZINC_950;
use bevy::prelude::*;

use crate::state::{
    EditorData, EditorState, load_editor_data, load_project_from_path, project_path,
    save_editor_data,
};
use crate::viewport::{
    CameraSettings, configure_floor_texture, despawn_preview_on_project_change, orbit_camera,
    respawn_preview_on_emitter_change, setup_camera, setup_floor, spawn_preview_particle_system,
    sync_playback_state, update_camera_viewport, zoom_camera,
};

pub struct AracariEditorPlugin;

impl Plugin for AracariEditorPlugin {
    fn build(&self, app: &mut App) {
        let editor_data = load_editor_data();

        app.add_plugins(AracariPlugin)
            .init_resource::<EditorState>()
            .init_resource::<CameraSettings>()
            .insert_resource(editor_data)
            .insert_resource(ClearColor(ZINC_950.into()))
            .add_systems(Startup, (setup_camera, setup_floor, load_initial_project))
            .add_systems(
                Update,
                (
                    orbit_camera,
                    zoom_camera,
                    update_camera_viewport,
                    configure_floor_texture,
                    spawn_preview_particle_system,
                    despawn_preview_on_project_change,
                    (respawn_preview_on_emitter_change, sync_playback_state).chain(),
                ),
            );
    }
}

fn load_initial_project(
    mut editor_state: ResMut<EditorState>,
    mut editor_data: ResMut<EditorData>,
    mut assets: ResMut<Assets<ParticleSystemAsset>>,
) {
    // try to load last opened project
    if let Some(location) = &editor_data.cache.last_opened_project.clone() {
        let path = project_path(location);
        if path.exists() {
            if let Some(asset) = load_project_from_path(&path) {
                let handle = assets.add(asset);
                editor_state.current_project = Some(handle);
                editor_state.current_project_path = Some(path);
                return;
            }
        }
    }

    // only load demo on first run (when no recent projects exist)
    let is_first_run = editor_data.cache.recent_projects.is_empty();

    if is_first_run {
        let demo_file = "examples/3d_explosion.ron";
        let demo_path = project_path(demo_file);
        if demo_path.exists() {
            if let Some(asset) = load_project_from_path(&demo_path) {
                let handle = assets.add(asset);
                editor_state.current_project = Some(handle);
                editor_state.current_project_path = Some(demo_path);

                // add demo to recent projects
                editor_data.cache.add_recent_project(demo_file.to_string());
                save_editor_data(&editor_data);
                return;
            }
        }
    }

    // fallback: create a default empty project
    let asset = aracari::asset::ParticleSystemAsset {
        name: "New project".to_string(),
        dimension: aracari::asset::ParticleSystemDimension::D3,
        emitters: vec![aracari::asset::EmitterData {
            name: "Emitter 1".to_string(),
            ..Default::default()
        }],
    };
    let handle = assets.add(asset);
    editor_state.current_project = Some(handle);
}
