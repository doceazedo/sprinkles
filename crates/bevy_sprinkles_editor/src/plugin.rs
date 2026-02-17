use bevy::color::palettes::tailwind::ZINC_950;
use bevy::prelude::*;
use bevy_sprinkles::prelude::*;

use crate::io::{EditorData, project_path, save_editor_data, working_dir};
use crate::project::load_project_from_path;
use crate::state::{DirtyState, EditorState, Inspectable, Inspecting};
use crate::viewport::{
    CameraSettings, ViewportInputState, configure_floor_texture, despawn_preview_on_project_change,
    draw_collider_gizmos, handle_playback_play_event, handle_playback_reset_event,
    handle_playback_seek_event, handle_respawn_colliders, handle_respawn_emitters, orbit_camera,
    respawn_preview_on_emitter_change, setup_camera, setup_floor, spawn_preview_particle_system,
    sync_playback_state, zoom_camera,
};

#[derive(Resource, Default)]
struct CliArgs {
    initial_file: Option<String>,
}

impl CliArgs {
    fn from_env() -> Self {
        Self {
            initial_file: std::env::args().nth(1),
        }
    }
}

pub struct SprinklesEditorPlugin;

impl Plugin for SprinklesEditorPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(CliArgs::from_env())
            .add_plugins(crate::assets::plugin)
            .add_plugins(SprinklesPlugin)
            .add_plugins(crate::io::plugin)
            .add_plugins(crate::state::plugin)
            .add_plugins(crate::project::plugin)
            .init_resource::<CameraSettings>()
            .init_resource::<ViewportInputState>()
            .insert_resource(ClearColor(ZINC_950.into()))
            .add_observer(respawn_preview_on_emitter_change)
            .add_observer(handle_respawn_emitters)
            .add_observer(handle_respawn_colliders)
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
                    draw_collider_gizmos,
                ),
            );
    }
}

fn load_initial_project(
    cli_args: Res<CliArgs>,
    mut editor_state: ResMut<EditorState>,
    mut editor_data: ResMut<EditorData>,
    mut dirty_state: ResMut<DirtyState>,
    mut assets: ResMut<Assets<ParticleSystemAsset>>,
) {
    if let Some(file) = &cli_args.initial_file {
        let cwd_path = working_dir().join(file);
        let path = if cwd_path.exists() {
            cwd_path
        } else {
            project_path(file)
        };
        if let Some(asset) = load_project_from_path(&path) {
            let has_emitters = !asset.emitters.is_empty();
            let handle = assets.add(asset);
            editor_state.open_project(handle, path, &mut dirty_state);
            if has_emitters {
                editor_state.inspecting = Some(Inspecting {
                    kind: Inspectable::Emitter,
                    index: 0,
                });
            }
            editor_data.cache.add_recent_project(file.clone());
            save_editor_data(&editor_data);
            return;
        }
    }

    if let Some(location) = &editor_data.cache.last_opened_project.clone() {
        let path = project_path(location);
        if path.exists() {
            if let Some(asset) = load_project_from_path(&path) {
                let has_emitters = !asset.emitters.is_empty();
                let handle = assets.add(asset);
                editor_state.open_project(handle, path, &mut dirty_state);
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

    let is_first_run = editor_data.cache.recent_projects.is_empty();

    if is_first_run {
        let demo_file = "examples/3d-explosion.ron";
        let demo_path = project_path(demo_file);
        if demo_path.exists() {
            if let Some(asset) = load_project_from_path(&demo_path) {
                let has_emitters = !asset.emitters.is_empty();
                let handle = assets.add(asset);
                editor_state.open_project(handle, demo_path, &mut dirty_state);
                if has_emitters {
                    editor_state.inspecting = Some(Inspecting {
                        kind: Inspectable::Emitter,
                        index: 0,
                    });
                }

                editor_data.cache.add_recent_project(demo_file.to_string());
                save_editor_data(&editor_data);
                return;
            }
        }
    }

    let asset = bevy_sprinkles::asset::ParticleSystemAsset::new(
        "New project".to_string(),
        bevy_sprinkles::asset::ParticleSystemDimension::D3,
        vec![bevy_sprinkles::asset::EmitterData {
            name: "Emitter 1".to_string(),
            ..Default::default()
        }],
        vec![],
        None,
    );
    let handle = assets.add(asset);
    editor_state.current_project = Some(handle);
    dirty_state.has_unsaved_changes = true;
    editor_state.inspecting = Some(Inspecting {
        kind: Inspectable::Emitter,
        index: 0,
    });
}
