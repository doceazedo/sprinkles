use std::f32::consts::{FRAC_PI_2, FRAC_PI_4};
use std::ops::Range;

use sprinkles::prelude::*;
use bevy::asset::RenderAssetUsages;
use bevy::camera::RenderTarget;
use bevy::color::palettes::tailwind::{ZINC_200, ZINC_950};
use bevy::image::{ImageAddressMode, ImageSamplerDescriptor};
use bevy::input::mouse::{AccumulatedMouseMotion, AccumulatedMouseScroll};
use bevy::math::Affine2;
use bevy::picking::hover::Hovered;
use bevy::post_process::bloom::Bloom;
use bevy::prelude::*;
use bevy::render::render_resource::{TextureDimension, TextureFormat, TextureUsages};

use crate::state::{
    EditorState, Inspectable, PlaybackPlayEvent, PlaybackResetEvent, PlaybackSeekEvent,
};
use crate::ui::components::seekbar::SeekbarDragState;
use crate::ui::components::viewport::EditorViewport;
use crate::ui::tokens::PRIMARY_COLOR;

const MIN_ZOOM_DISTANCE: f32 = 0.1;
const MAX_ZOOM_DISTANCE: f32 = 20.0;
const ZOOM_SPEED: f32 = 0.5;
const INITIAL_ORBIT_DISTANCE: f32 = 8.0;
const ORBIT_OFFSET: Vec3 = Vec3::new(1.0, 0.75, 1.0);
const ORBIT_TARGET: Vec3 = Vec3::ZERO;

const FLOOR_SIZE: f32 = 192.0;
const FLOOR_TILE_SIZE: f32 = 2.0;

#[derive(Component)]
pub struct EditorCamera;

#[derive(Default, Resource)]
pub struct ViewportInputState {
    pub dragging: bool,
}

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

pub fn setup_camera(mut commands: Commands, mut images: ResMut<Assets<Image>>) {
    let mut image = Image::new_uninit(
        default(),
        TextureDimension::D2,
        TextureFormat::Bgra8UnormSrgb,
        RenderAssetUsages::all(),
    );
    image.texture_descriptor.usage =
        TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST | TextureUsages::RENDER_ATTACHMENT;
    let image_handle = images.add(image);

    commands.spawn((Name::new("UiCamera"), Camera2d));

    let initial_position = ORBIT_TARGET + ORBIT_OFFSET.normalize() * INITIAL_ORBIT_DISTANCE;
    commands.spawn((
        EditorCamera,
        Name::new("ViewportCamera"),
        Camera3d::default(),
        Camera {
            order: -1,
            clear_color: ClearColorConfig::Custom(ZINC_950.into()),
            ..default()
        },
        RenderTarget::Image(image_handle.into()),
        Transform::from_translation(initial_position).looking_at(ORBIT_TARGET, Vec3::Y),
        Bloom::NATURAL,
        DistanceFog {
            color: ZINC_950.into(),
            falloff: FogFalloff::Linear {
                start: 24.0,
                end: 96.0,
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
    viewport: Single<&Hovered, With<EditorViewport>>,
    mut input_state: ResMut<ViewportInputState>,
    camera_settings: Res<CameraSettings>,
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    mouse_motion: Res<AccumulatedMouseMotion>,
) {
    let pressing =
        mouse_buttons.pressed(MouseButton::Left) || mouse_buttons.pressed(MouseButton::Right);

    if !pressing {
        input_state.dragging = false;
        return;
    }

    let just_pressed = mouse_buttons.just_pressed(MouseButton::Left)
        || mouse_buttons.just_pressed(MouseButton::Right);

    if just_pressed && viewport.get() {
        input_state.dragging = true;
    }

    if !input_state.dragging {
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
    viewport: Single<&Hovered, With<EditorViewport>>,
    mut camera_settings: ResMut<CameraSettings>,
    mouse_scroll: Res<AccumulatedMouseScroll>,
) {
    if !viewport.get() {
        return;
    }

    let delta = mouse_scroll.delta.y;
    if delta == 0.0 {
        return;
    }

    let zoom_delta = -delta * ZOOM_SPEED;
    camera_settings.orbit_distance =
        (camera_settings.orbit_distance + zoom_delta).clamp(MIN_ZOOM_DISTANCE, MAX_ZOOM_DISTANCE);

    camera.translation = ORBIT_TARGET - camera.forward() * camera_settings.orbit_distance;
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

#[derive(Event)]
pub struct RespawnEmittersEvent;

#[derive(Event)]
pub struct RespawnCollidersEvent;

pub fn handle_respawn_emitters(
    _trigger: On<RespawnEmittersEvent>,
    mut commands: Commands,
    preview_systems: Query<Entity, With<EditorParticlePreview>>,
    emitter_entities: Query<(Entity, &EmitterEntity)>,
) {
    for system_entity in &preview_systems {
        for (emitter_entity, emitter) in &emitter_entities {
            if emitter.parent_system == system_entity {
                commands.entity(emitter_entity).despawn();
            }
        }
        commands
            .entity(system_entity)
            .remove::<ParticleSystemRuntime>();
    }
}

pub fn handle_respawn_colliders(
    _trigger: On<RespawnCollidersEvent>,
    mut commands: Commands,
    preview_systems: Query<Entity, With<EditorParticlePreview>>,
    collider_entities: Query<(Entity, &ColliderEntity)>,
) {
    for system_entity in &preview_systems {
        for (collider_entity, collider) in &collider_entities {
            if collider.parent_system == system_entity {
                commands.entity(collider_entity).despawn();
            }
        }
        commands
            .entity(system_entity)
            .remove::<ParticleSystemRuntime>();
    }
}

pub fn respawn_preview_on_emitter_change(
    _trigger: On<PlaybackResetEvent>,
    mut commands: Commands,
    editor_state: Res<EditorState>,
    assets: Res<Assets<ParticleSystemAsset>>,
    preview_query: Query<Entity, (With<EditorParticlePreview>, With<ParticleSystemRuntime>)>,
    emitter_query: Query<&EmitterEntity>,
) {
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

pub fn handle_playback_reset_event(
    _trigger: On<PlaybackResetEvent>,
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

        system_runtime.paused = true;
        for (emitter, mut runtime) in emitter_query.iter_mut() {
            if emitter.parent_system == system_entity {
                let fixed_seed = asset
                    .emitters
                    .get(runtime.emitter_index)
                    .and_then(|e| e.time.fixed_seed);
                runtime.stop(fixed_seed);
            }
        }
    }
}

pub fn handle_playback_play_event(
    _trigger: On<PlaybackPlayEvent>,
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

        let sub_target_indices: Vec<usize> = asset
            .emitters
            .iter()
            .filter_map(|e| e.sub_emitter.as_ref().map(|s| s.target_emitter))
            .collect();

        // check if all one-shot emitters have completed (ignoring subemitter targets)
        let all_one_shots_completed =
            asset
                .emitters
                .iter()
                .enumerate()
                .filter(|(idx, _)| !sub_target_indices.contains(idx))
                .all(|(idx, emitter_data)| {
                    if !emitter_data.time.one_shot {
                        return true;
                    }
                    emitter_query.iter().any(|(emitter, runtime)| {
                        emitter.parent_system == system_entity
                            && runtime.emitter_index == idx
                            && runtime.one_shot_completed
                    })
                });

        let has_one_shot = asset
            .emitters
            .iter()
            .enumerate()
            .filter(|(idx, _)| !sub_target_indices.contains(idx))
            .any(|(_, e)| e.time.one_shot);

        // restart completed one-shot emitters when play is requested
        if has_one_shot && all_one_shots_completed {
            for (emitter, mut runtime) in emitter_query.iter_mut() {
                if emitter.parent_system == system_entity {
                    runtime.restart(None);
                }
            }
            system_runtime.resume();
        }
    }
}

pub fn handle_playback_seek_event(
    trigger: On<PlaybackSeekEvent>,
    system_query: Query<Entity, With<EditorParticlePreview>>,
    mut emitter_query: Query<(&EmitterEntity, &mut EmitterRuntime)>,
) {
    let seek_time = trigger.0;

    for system_entity in system_query.iter() {
        for (emitter, mut runtime) in emitter_query.iter_mut() {
            if emitter.parent_system == system_entity {
                runtime.seek(seek_time);
            }
        }
    }
}

pub fn draw_collider_gizmos(
    mut gizmos: Gizmos,
    colliders: Query<(&ParticlesCollider3D, &ColliderEntity, &Transform)>,
    editor_state: Res<EditorState>,
) {
    let inspected_index = editor_state
        .inspecting
        .as_ref()
        .filter(|i| i.kind == Inspectable::Collider)
        .map(|i| i.index as usize);

    for (collider, collider_entity, transform) in &colliders {
        if inspected_index != Some(collider_entity.collider_index) {
            continue;
        }

        let color = if collider.enabled {
            PRIMARY_COLOR
        } else {
            ZINC_200
        };

        match &collider.shape {
            ParticlesColliderShape3D::Box { size } => {
                let collider_transform = Transform {
                    translation: transform.translation,
                    rotation: transform.rotation,
                    scale: *size,
                };
                gizmos.cube(collider_transform, color);
            }
            ParticlesColliderShape3D::Sphere { radius } => {
                let isometry = Isometry3d::from_translation(transform.translation);
                gizmos.sphere(isometry, *radius, color);
            }
        }
    }
}

pub fn sync_playback_state(
    assets: Res<Assets<ParticleSystemAsset>>,
    drag_state: Query<&SeekbarDragState>,
    mut system_query: Query<
        (Entity, &ParticleSystem3D, &mut ParticleSystemRuntime),
        With<EditorParticlePreview>,
    >,
    mut emitter_query: Query<(&EmitterEntity, &mut EmitterRuntime)>,
) {
    let is_seeking = drag_state.iter().any(|s| s.dragging);

    for (system_entity, particle_system, mut system_runtime) in system_query.iter_mut() {
        let Some(asset) = assets.get(&particle_system.handle) else {
            continue;
        };

        // while seeking, pause and skip normal updates
        if is_seeking {
            if !system_runtime.paused {
                system_runtime.pause();
            }
            continue;
        }

        // ignore subemitter targets for completion checks
        let sub_target_indices: Vec<usize> = asset
            .emitters
            .iter()
            .filter_map(|e| e.sub_emitter.as_ref().map(|s| s.target_emitter))
            .collect();

        // check if all one-shot emitters have completed
        let all_one_shots_completed =
            asset
                .emitters
                .iter()
                .enumerate()
                .filter(|(idx, _)| !sub_target_indices.contains(idx))
                .all(|(idx, emitter_data)| {
                    if !emitter_data.time.one_shot {
                        return true;
                    }
                    emitter_query.iter().any(|(emitter, runtime)| {
                        emitter.parent_system == system_entity
                            && runtime.emitter_index == idx
                            && runtime.one_shot_completed
                    })
                });

        let has_one_shot = asset
            .emitters
            .iter()
            .enumerate()
            .filter(|(idx, _)| !sub_target_indices.contains(idx))
            .any(|(_, e)| e.time.one_shot);

        // handle one-shot emitters completion
        if has_one_shot && all_one_shots_completed {
            if system_runtime.force_loop {
                for (emitter, mut runtime) in emitter_query.iter_mut() {
                    if emitter.parent_system == system_entity {
                        let fixed_seed = asset
                            .emitters
                            .get(runtime.emitter_index)
                            .and_then(|e| e.time.fixed_seed);
                        runtime.restart(fixed_seed);
                    }
                }
            } else {
                // pause and reset elapsed time to 0
                system_runtime.pause();
                for (emitter, mut runtime) in emitter_query.iter_mut() {
                    if emitter.parent_system == system_entity {
                        runtime.seek(0.0);
                    }
                }
            }
            continue;
        }

        // sync playback state
        if !system_runtime.paused {
            for (emitter, mut runtime) in emitter_query.iter_mut() {
                if emitter.parent_system == system_entity
                    && !runtime.emitting
                    && !runtime.one_shot_completed
                {
                    runtime.play();
                }
            }
        }
    }
}
