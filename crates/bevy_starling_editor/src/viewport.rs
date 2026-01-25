use std::f32::consts::{FRAC_PI_2, FRAC_PI_4, PI};
use std::ops::Range;

use bevy::camera::SubCameraView;
use bevy::color::palettes::tailwind;
use bevy::core_pipeline::oit::OrderIndependentTransparencySettings;
use bevy::input::mouse::{AccumulatedMouseMotion, AccumulatedMouseScroll};
use bevy::prelude::*;
use bevy_starling::{
    asset::ParticleSystemAsset,
    core::ParticleSystem3D,
    runtime::{EmitterEntity, EmitterRuntime, ParticleSystemRuntime},
};

use crate::state::EditorState;

const MIN_ZOOM_DISTANCE: f32 = 0.1;
const MAX_ZOOM_DISTANCE: f32 = 20.0;
const ZOOM_SPEED: f32 = 0.5;
const INITIAL_ORBIT_DISTANCE: f32 = 8.66;

#[derive(Component)]
pub struct EditorCamera;

#[derive(Debug, Resource)]
pub struct CameraSettings {
    pub orbit_distance: f32,
    pub pitch_speed: f32,
    pub pitch_range: Range<f32>,
    pub yaw_speed: f32,
}

impl Default for CameraSettings {
    fn default() -> Self {
        let pitch_limit = FRAC_PI_2 - 0.01;
        Self {
            orbit_distance: INITIAL_ORBIT_DISTANCE,
            pitch_speed: 0.003,
            pitch_range: -pitch_limit..pitch_limit,
            yaw_speed: 0.004,
        }
    }
}

#[derive(Resource, Default)]
pub struct ViewportLayout {
    pub left_panel_width: f32,
}

pub fn setup_camera(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn((
        EditorCamera,
        Name::new("Camera"),
        Camera3d::default(),
        Transform::from_xyz(5.0, 5.0, 5.0).looking_at(Vec3::ZERO, Vec3::Y),
        EnvironmentMapLight {
            diffuse_map: asset_server.load("pisa_diffuse_rgb9e5_zstd.ktx2"),
            specular_map: asset_server.load("pisa_specular_rgb9e5_zstd.ktx2"),
            ..default()
        },
        // enable order-independent transparency for proper particle rendering
        // this handles overlapping transparent particles correctly regardless of GPU render order
        OrderIndependentTransparencySettings::default(),
        // MSAA is incompatible with OIT
        Msaa::Off,
    ));

    commands.spawn((
        DirectionalLight::default(),
        Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, -FRAC_PI_4, 0.0, -FRAC_PI_4)),
    ));
}

pub fn orbit_camera(
    mut camera: Single<&mut Transform, With<EditorCamera>>,
    camera_settings: Res<CameraSettings>,
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    mouse_motion: Res<AccumulatedMouseMotion>,
) {
    let orbiting = mouse_buttons.pressed(MouseButton::Left)
        || mouse_buttons.pressed(MouseButton::Right);

    if !orbiting {
        return;
    }

    let delta = -mouse_motion.delta;
    let delta_pitch = delta.y * camera_settings.pitch_speed;
    let delta_yaw = delta.x * camera_settings.yaw_speed;

    let (yaw, pitch, roll) = camera.rotation.to_euler(EulerRot::YXZ);

    let pitch = (pitch + delta_pitch).clamp(
        camera_settings.pitch_range.start,
        camera_settings.pitch_range.end,
    );
    let yaw = yaw + delta_yaw;
    camera.rotation = Quat::from_euler(EulerRot::YXZ, yaw, pitch, roll);

    let target = Vec3::ZERO;
    camera.translation = target - camera.forward() * camera_settings.orbit_distance;
}

pub fn zoom_camera(
    mut camera: Single<&mut Transform, With<EditorCamera>>,
    mut camera_settings: ResMut<CameraSettings>,
    mouse_scroll: Res<AccumulatedMouseScroll>,
) {
    let delta = mouse_scroll.delta.y;
    if delta == 0.0 {
        return;
    }

    let zoom_delta = -delta * ZOOM_SPEED;
    camera_settings.orbit_distance =
        (camera_settings.orbit_distance + zoom_delta).clamp(MIN_ZOOM_DISTANCE, MAX_ZOOM_DISTANCE);

    let target = Vec3::ZERO;
    camera.translation = target - camera.forward() * camera_settings.orbit_distance;
}

pub fn update_camera_viewport(
    mut camera: Single<&mut Camera, With<EditorCamera>>,
    layout: Res<ViewportLayout>,
    windows: Query<&Window>,
) {
    let Ok(window) = windows.single() else {
        return;
    };

    let window_width = window.width();
    let window_height = window.height();
    let panel_width = layout.left_panel_width;

    if panel_width <= 0.0 || panel_width >= window_width {
        camera.sub_camera_view = None;
        return;
    }

    // use SubCameraView to offset the projection so the origin appears centered
    // in the available viewport area (to the right of the panel)
    camera.sub_camera_view = Some(SubCameraView {
        full_size: UVec2::new((window_width + panel_width) as u32, window_height as u32),
        offset: Vec2::ZERO,
        size: UVec2::new(window_width as u32, window_height as u32),
    });
}

pub fn draw_grid(mut gizmos: Gizmos) {
    gizmos.grid(
        Quat::from_rotation_x(PI / 2.0),
        UVec2::splat(100),
        Vec2::new(1.0, 1.0),
        tailwind::ZINC_700,
    );
}

#[derive(Component)]
pub struct EditorParticlePreview;

pub fn spawn_preview_particle_system(
    mut commands: Commands,
    editor_state: Res<EditorState>,
    assets: Res<Assets<ParticleSystemAsset>>,
    existing: Query<Entity, With<EditorParticlePreview>>,
) {
    let Some(handle) = &editor_state.current_project else {
        for entity in existing.iter() {
            commands.entity(entity).despawn();
        }
        return;
    };

    if assets.get(handle).is_none() {
        return;
    }

    if !existing.is_empty() {
        return;
    }

    commands.spawn((
        ParticleSystem3D {
            handle: handle.clone(),
        },
        Transform::default(),
        Visibility::default(),
        EditorParticlePreview,
        Name::new("Particle Preview"),
    ));
}

pub fn despawn_preview_on_project_change(
    mut commands: Commands,
    editor_state: Res<EditorState>,
    existing: Query<(Entity, &ParticleSystem3D), With<EditorParticlePreview>>,
) {
    if !editor_state.is_changed() {
        return;
    }

    for (entity, particle_system) in existing.iter() {
        let should_despawn = match &editor_state.current_project {
            Some(handle) => particle_system.handle != *handle,
            None => true,
        };

        if should_despawn {
            commands.entity(entity).despawn();
        }
    }
}

pub fn sync_playback_state(
    mut editor_state: ResMut<EditorState>,
    assets: Res<Assets<ParticleSystemAsset>>,
    mut system_query: Query<
        (Entity, &ParticleSystem3D, &mut ParticleSystemRuntime),
        With<EditorParticlePreview>,
    >,
    mut emitter_query: Query<(&EmitterEntity, &mut EmitterRuntime)>,
) {
    for (system_entity, particle_system, mut system_runtime) in system_query.iter_mut() {
        let Some(asset) = assets.get(&particle_system.handle) else {
            continue;
        };

        // calculate duration from the longest emitter total duration (delay + lifetime)
        let max_duration = asset
            .emitters
            .iter()
            .map(|e| e.time.delay + e.time.lifetime)
            .fold(0.0_f32, |a, b| a.max(b));
        editor_state.duration_ms = max_duration * 1000.0;

        // handle stop button - apply to all emitters
        if editor_state.should_reset {
            system_runtime.paused = false;
            for (emitter, mut runtime) in emitter_query.iter_mut() {
                if emitter.parent_system == system_entity {
                    let fixed_seed = asset
                        .emitters
                        .get(runtime.emitter_index)
                        .filter(|e| e.time.use_fixed_seed)
                        .map(|e| e.time.seed);
                    runtime.stop(fixed_seed);
                }
            }
            editor_state.elapsed_ms = 0.0;
            editor_state.should_reset = false;
            continue;
        }

        // check if all one-shot emitters have completed
        let all_one_shots_completed = asset.emitters.iter().enumerate().all(|(idx, emitter_data)| {
            if !emitter_data.time.one_shot {
                return true;
            }
            emitter_query.iter().any(|(emitter, runtime)| {
                emitter.parent_system == system_entity
                    && runtime.emitter_index == idx
                    && runtime.one_shot_completed
            })
        });

        let has_one_shot = asset.emitters.iter().any(|e| e.time.one_shot);

        // handle one-shot emitters completion
        if has_one_shot && all_one_shots_completed {
            if editor_state.is_looping || editor_state.play_requested {
                // looping mode or user clicked play: restart all emitters with new seed
                for (emitter, mut runtime) in emitter_query.iter_mut() {
                    if emitter.parent_system == system_entity {
                        runtime.restart(None);
                    }
                }
                editor_state.play_requested = false;
            } else {
                // one_shot finished, not looping: stop and reset progress
                editor_state.elapsed_ms = 0.0;
                editor_state.is_playing = false;
            }
            continue;
        }

        // clear play_requested if we get here (normal playback)
        editor_state.play_requested = false;

        // sync playback state from editor to system
        if editor_state.is_playing {
            if system_runtime.paused {
                system_runtime.resume();
            }
            // ensure non-completed emitters are emitting
            // (don't restart one-shot emitters that have already completed)
            for (emitter, mut runtime) in emitter_query.iter_mut() {
                if emitter.parent_system == system_entity
                    && !runtime.emitting
                    && !runtime.one_shot_completed
                {
                    runtime.play();
                }
            }
        } else {
            if !system_runtime.paused {
                system_runtime.pause();
            }
        }

        // track elapsed time as the maximum system_time across all emitters
        // this prevents the progress bar from resetting when shorter emitters wrap
        let mut max_elapsed = 0.0_f32;
        for (emitter, runtime) in emitter_query.iter() {
            if emitter.parent_system == system_entity {
                max_elapsed = max_elapsed.max(runtime.system_time);
            }
        }
        editor_state.elapsed_ms = max_elapsed * 1000.0;
    }
}
