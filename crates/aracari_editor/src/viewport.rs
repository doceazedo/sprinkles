use std::f32::consts::{FRAC_PI_2, FRAC_PI_4};
use std::ops::Range;

use aracari::prelude::*;
use bevy::color::palettes::tailwind::ZINC_950;
use bevy::image::{ImageAddressMode, ImageSamplerDescriptor};
use bevy::math::Affine2;
use bevy::input::mouse::{AccumulatedMouseMotion, AccumulatedMouseScroll};
use bevy::post_process::bloom::Bloom;
use bevy::prelude::*;

use crate::state::EditorState;

const MIN_ZOOM_DISTANCE: f32 = 0.1;
const MAX_ZOOM_DISTANCE: f32 = 20.0;
const ZOOM_SPEED: f32 = 0.5;
const INITIAL_ORBIT_DISTANCE: f32 = 8.0;
const ORBIT_OFFSET: Vec3 = Vec3::new(1.0, 0.75, 1.0);
const ORBIT_TARGET: Vec3 = Vec3::ZERO;

const FLOOR_SIZE: f32 = 100.0;
const FLOOR_TILE_SIZE: f32 = 2.0;

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

pub fn setup_camera(mut commands: Commands) {
    let initial_position = ORBIT_TARGET + ORBIT_OFFSET.normalize() * INITIAL_ORBIT_DISTANCE;
    commands.spawn((
        EditorCamera,
        Name::new("Camera"),
        Camera3d::default(),
        Transform::from_translation(initial_position).looking_at(ORBIT_TARGET, Vec3::Y),
        Bloom::NATURAL,
        DistanceFog {
            color: ZINC_950.into(),
            falloff: FogFalloff::Linear {
                start: 24.0,
                end: 48.0,
            },
            ..default()
        },
    ));

    commands.spawn((
        DirectionalLight {
            shadows_enabled: true,
            ..default()
        },
        Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, -FRAC_PI_4, 0.0, -FRAC_PI_4)),
    ));
}

#[derive(Resource)]
pub struct FloorTexture(Handle<Image>);

pub fn setup_floor(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let floor_texture: Handle<Image> = asset_server.load("floor.png");
    commands.insert_resource(FloorTexture(floor_texture.clone()));

    let mesh = meshes.add(Plane3d::new(*Dir3::Y, Vec2::splat(FLOOR_SIZE / 2.)));

    let tile_count = FLOOR_SIZE / FLOOR_TILE_SIZE;
    let material = materials.add(StandardMaterial {
        base_color_texture: Some(floor_texture),
        uv_transform: Affine2::from_scale(Vec2::splat(tile_count)),
        perceptual_roughness: 1.0,
        ..default()
    });

    commands.spawn((
        Mesh3d(mesh),
        MeshMaterial3d(material),
        Name::new("Floor"),
        Transform::from_xyz(0.0, -2.0, 0.0),
        Visibility::default(),
    ));

    // particle collision box
    commands.spawn((
        ParticlesCollider3D {
            shape: ParticlesColliderShape3D::Box {
                size: Vec3::new(10.0, 0.1, 10.0),
            },
            position: Vec3::ZERO,
        },
        Transform::from_xyz(0.0, -2.01, 0.0),
        Name::new("Particle Collider"),
    ));
}

pub fn configure_floor_texture(
    mut commands: Commands,
    floor_texture: Option<Res<FloorTexture>>,
    mut images: ResMut<Assets<Image>>,
) {
    let Some(floor_texture) = floor_texture else {
        return;
    };
    let Some(image) = images.get_mut(&floor_texture.0) else {
        return;
    };

    image.sampler = bevy::image::ImageSampler::Descriptor(ImageSamplerDescriptor {
        address_mode_u: ImageAddressMode::Repeat,
        address_mode_v: ImageAddressMode::Repeat,
        address_mode_w: ImageAddressMode::Repeat,
        ..default()
    });

    commands.remove_resource::<FloorTexture>();
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

    camera.translation = ORBIT_TARGET - camera.forward() * camera_settings.orbit_distance;
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

    camera.translation = ORBIT_TARGET - camera.forward() * camera_settings.orbit_distance;
}

pub fn update_camera_viewport(
    mut camera: Single<&mut Camera, With<EditorCamera>>,
) {
    camera.sub_camera_view = None;
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

pub fn respawn_preview_on_emitter_change(
    mut commands: Commands,
    editor_state: Res<EditorState>,
    assets: Res<Assets<ParticleSystemAsset>>,
    preview_query: Query<Entity, (With<EditorParticlePreview>, With<ParticleSystemRuntime>)>,
    emitter_query: Query<&EmitterEntity>,
) {
    if !editor_state.should_reset {
        return;
    }

    let Some(handle) = &editor_state.current_project else {
        return;
    };

    let Some(asset) = assets.get(handle) else {
        return;
    };

    let Ok(preview_entity) = preview_query.single() else {
        return;
    };

    let current_emitter_count = emitter_query
        .iter()
        .filter(|e| e.parent_system == preview_entity)
        .count();

    let asset_emitter_count = asset.emitters.len();

    if current_emitter_count != asset_emitter_count {
        commands.entity(preview_entity).despawn();
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

        // calculate duration from the longest emitter total duration
        let max_duration = asset
            .emitters
            .iter()
            .map(|e| e.time.total_duration())
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
