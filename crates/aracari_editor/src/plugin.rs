use aracari::prelude::*;
use bevy::color::palettes::tailwind::ZINC_950;
use bevy::prelude::*;

use crate::io::{project_path, save_editor_data, EditorData};
use crate::project::load_project_from_path;
use crate::state::{EditorState, Inspectable, Inspecting};
use crate::viewport::{
    CameraSettings, ViewportInputState, configure_floor_texture, despawn_preview_on_project_change,
    handle_playback_play_event, handle_playback_reset_event, handle_playback_seek_event,
    orbit_camera, respawn_preview_on_emitter_change, setup_camera, setup_floor,
    spawn_preview_particle_system, sync_playback_state, zoom_camera,
};

pub struct AracariEditorPlugin;

impl Plugin for AracariEditorPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(AracariPlugin)
            .add_plugins(crate::io::plugin)
            .add_plugins(crate::state::plugin)
            .add_plugins(crate::project::plugin)
            .init_resource::<CameraSettings>()
            .init_resource::<ViewportInputState>()
            .insert_resource(ClearColor(ZINC_950.into()))
            .add_observer(respawn_preview_on_emitter_change)
            .add_observer(handle_playback_play_event)
            .add_observer(handle_playback_reset_event)
            .add_observer(handle_playback_seek_event)
            .add_systems(Startup, (setup_camera, setup_floor, load_initial_project))
            .add_systems(
                Update,
                (
                    orbit_camera,
                    zoom_camera,
                    configure_floor_texture,
                    spawn_preview_particle_system,
                    despawn_preview_on_project_change,
                    sync_playback_state,
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
                let has_emitters = !asset.emitters.is_empty();
                let handle = assets.add(asset);
                editor_state.current_project = Some(handle);
                editor_state.current_project_path = Some(path);
                if has_emitters {
                    editor_state.inspecting = Some(Inspecting {
                        kind: Inspectable::Emitter,
                        index: 0,
                    });
                }
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
                let has_emitters = !asset.emitters.is_empty();
                let handle = assets.add(asset);
                editor_state.current_project = Some(handle);
                editor_state.current_project_path = Some(demo_path);
                if has_emitters {
                    editor_state.inspecting = Some(Inspecting {
                        kind: Inspectable::Emitter,
                        index: 0,
                    });
                }

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
    editor_state.inspecting = Some(Inspecting {
        kind: Inspectable::Emitter,
        index: 0,
    });
}
