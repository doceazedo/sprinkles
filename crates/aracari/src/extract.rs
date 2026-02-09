use bevy::{
    prelude::*,
    render::{render_resource::ShaderType, storage::ShaderStorageBuffer, Extract},
};
use bytemuck::{Pod, Zeroable};

use crate::{
    asset::{
        DrawOrder, EmissionShape, EmitterCollisionMode, ParticleSystemAsset,
        ParticlesColliderShape3D, SolidOrGradientColor,
    },
    runtime::{
        compute_phase, is_past_delay, EmitterEntity, EmitterRuntime, ParticleBufferHandle,
        ParticleSystem3D, ParticleSystemRuntime, ParticlesCollider3D,
    },
    textures::{CurveTextureCache, GradientTextureCache},
};

// emission shape constants
pub const EMISSION_SHAPE_POINT: u32 = 0;
pub const EMISSION_SHAPE_SPHERE: u32 = 1;
pub const EMISSION_SHAPE_SPHERE_SURFACE: u32 = 2;
pub const EMISSION_SHAPE_BOX: u32 = 3;
pub const EMISSION_SHAPE_RING: u32 = 4;

// collision constants
pub const COLLIDER_TYPE_SPHERE: u32 = 0;
pub const COLLIDER_TYPE_BOX: u32 = 1;
pub const MAX_COLLIDERS: usize = 32;

pub const COLLISION_MODE_DISABLED: u32 = 0;
pub const COLLISION_MODE_RIGID: u32 = 1;
pub const COLLISION_MODE_HIDE_ON_CONTACT: u32 = 2;

#[derive(Clone, Copy, Default, Pod, Zeroable, ShaderType)]
#[repr(C)]
pub struct CurveUniform {
    pub enabled: u32,
    pub min_value: f32,
    pub max_value: f32,
    pub _pad: u32,
}

impl CurveUniform {
    pub fn disabled() -> Self {
        Self {
            enabled: 0,
            min_value: 0.0,
            max_value: 1.0,
            _pad: 0,
        }
    }

    pub fn enabled(min_value: f32, max_value: f32) -> Self {
        Self {
            enabled: 1,
            min_value,
            max_value,
            _pad: 0,
        }
    }
}

#[derive(Clone, Copy, Default, Pod, Zeroable, ShaderType)]
#[repr(C)]
pub struct AnimatedVelocityUniform {
    pub min: f32,
    pub max: f32,
    pub _pad0: f32,
    pub _pad1: f32,
    pub curve: CurveUniform,
}

#[derive(Clone, Copy, Default, Pod, Zeroable, ShaderType)]
#[repr(C)]
pub struct ColliderUniform {
    pub transform: [f32; 16],
    pub inverse_transform: [f32; 16],
    pub extents: [f32; 3],
    pub collider_type: u32,
}

#[derive(Clone, Copy, Default, Pod, Zeroable, ShaderType)]
#[repr(C)]
pub struct EmitterUniforms {
    pub delta_time: f32,
    pub system_phase: f32,
    pub prev_system_phase: f32,
    pub cycle: u32,

    pub amount: u32,
    pub lifetime: f32,
    pub lifetime_randomness: f32,
    pub emitting: u32,

    pub gravity: [f32; 3],
    pub random_seed: u32,

    pub emission_shape: u32,
    pub emission_sphere_radius: f32,
    pub emission_ring_height: f32,
    pub emission_ring_radius: f32,

    pub emission_ring_inner_radius: f32,
    pub spread: f32,
    pub flatness: f32,
    pub initial_velocity_min: f32,

    pub initial_velocity_max: f32,
    pub inherit_velocity_ratio: f32,
    pub explosiveness: f32,
    pub spawn_time_randomness: f32,

    pub emission_offset: [f32; 3],
    pub _pad1: f32,

    pub emission_scale: [f32; 3],
    pub _pad2: f32,

    pub emission_box_extents: [f32; 3],
    pub _pad3: f32,

    pub emission_ring_axis: [f32; 3],
    pub _pad4: f32,

    pub direction: [f32; 3],
    pub _pad5: f32,

    pub velocity_pivot: [f32; 3],
    pub _pad6: f32,

    pub draw_order: u32,
    pub clear_particles: u32,
    pub scale_min: f32,
    pub scale_max: f32,

    pub scale_over_lifetime: CurveUniform,

    pub use_initial_color_gradient: u32,
    pub turbulence_enabled: u32,
    pub particle_flags: u32,
    pub _pad7: u32,

    pub initial_color: [f32; 4],

    pub alpha_over_lifetime: CurveUniform,
    pub emission_over_lifetime: CurveUniform,

    pub turbulence_noise_strength: f32,
    pub turbulence_noise_scale: f32,
    pub turbulence_noise_speed_random: f32,
    pub turbulence_influence_min: f32,

    pub turbulence_noise_speed: [f32; 3],
    pub turbulence_influence_max: f32,

    pub turbulence_influence_over_lifetime: CurveUniform,

    pub radial_velocity: AnimatedVelocityUniform,

    // collision
    pub collision_mode: u32,
    pub collision_base_size: f32,
    pub collision_use_scale: u32,
    pub collision_friction: f32,

    pub collision_bounce: f32,
    pub collider_count: u32,
    pub _collision_pad0: f32,
    pub _collision_pad1: f32,
}

#[derive(Resource, Default)]
pub struct ExtractedColliders {
    pub colliders: Vec<ColliderUniform>,
}

#[derive(Resource, Default)]
pub struct ExtractedParticleSystem {
    pub emitters: Vec<(Entity, ExtractedEmitterData)>,
}

pub struct ExtractedEmitterData {
    pub uniform_steps: Vec<EmitterUniforms>,
    pub particle_buffer_handle: Handle<ShaderStorageBuffer>,
    pub indices_buffer_handle: Handle<ShaderStorageBuffer>,
    pub sorted_particles_buffer_handle: Handle<ShaderStorageBuffer>,
    pub amount: u32,
    pub draw_order: u32,
    pub camera_position: [f32; 3],
    pub camera_forward: [f32; 3],
    pub emitter_transform: Mat4,
    pub gradient_texture_handle: Option<Handle<Image>>,
    pub scale_over_lifetime_texture_handle: Option<Handle<Image>>,
    pub alpha_over_lifetime_texture_handle: Option<Handle<Image>>,
    pub emission_over_lifetime_texture_handle: Option<Handle<Image>>,
    pub turbulence_influence_over_lifetime_texture_handle: Option<Handle<Image>>,
    pub radial_velocity_curve_texture_handle: Option<Handle<Image>>,
}

pub fn extract_particle_systems(
    mut commands: Commands,
    emitter_query: Extract<
        Query<(
            Entity,
            &EmitterEntity,
            &EmitterRuntime,
            &ParticleBufferHandle,
            &GlobalTransform,
        )>,
    >,
    system_query: Extract<Query<(&ParticleSystem3D, &ParticleSystemRuntime)>>,
    camera_query: Extract<Query<&GlobalTransform, With<Camera3d>>>,
    assets: Extract<Res<Assets<ParticleSystemAsset>>>,
    gradient_cache: Extract<Res<GradientTextureCache>>,
    curve_cache: Extract<Res<CurveTextureCache>>,
) {
    let mut extracted = ExtractedParticleSystem::default();

    let (camera_position, camera_forward) = camera_query
        .iter()
        .next()
        .map(|t| (t.translation(), t.forward().as_vec3()))
        .unwrap_or((Vec3::ZERO, Vec3::NEG_Z));

    for (entity, emitter_entity, runtime, buffer_handle, global_transform) in emitter_query.iter() {
        let Ok((particle_system, _system_runtime)) = system_query.get(emitter_entity.parent_system)
        else {
            continue;
        };

        let Some(asset) = assets.get(&particle_system.handle) else {
            continue;
        };

        let Some(emitter) = asset.emitters.get(runtime.emitter_index) else {
            continue;
        };

        if !emitter.enabled {
            continue;
        }

        let draw_order = match emitter.draw_pass.draw_order {
            DrawOrder::Index => 0,
            DrawOrder::Lifetime => 1,
            DrawOrder::ReverseLifetime => 2,
            DrawOrder::ViewDepth => 3,
        };

        let (emission_shape, emission_sphere_radius, emission_box_extents, emission_ring_axis, emission_ring_height, emission_ring_radius, emission_ring_inner_radius) =
            match emitter.emission.shape {
                EmissionShape::Point => {
                    (EMISSION_SHAPE_POINT, 0.0, Vec3::ZERO, Vec3::Z, 0.0, 0.0, 0.0)
                }
                EmissionShape::Sphere { radius } => {
                    (EMISSION_SHAPE_SPHERE, radius, Vec3::ZERO, Vec3::Z, 0.0, 0.0, 0.0)
                }
                EmissionShape::SphereSurface { radius } => {
                    (EMISSION_SHAPE_SPHERE_SURFACE, radius, Vec3::ZERO, Vec3::Z, 0.0, 0.0, 0.0)
                }
                EmissionShape::Box { extents } => {
                    (EMISSION_SHAPE_BOX, 0.0, extents, Vec3::Z, 0.0, 0.0, 0.0)
                }
                EmissionShape::Ring { axis, height, radius, inner_radius } => {
                    (EMISSION_SHAPE_RING, 0.0, Vec3::ZERO, axis, height, radius, inner_radius)
                }
            };

        let turbulence = &emitter.turbulence;

        // build base uniform with timing fields zeroed (filled per step below)
        let base_uniforms = EmitterUniforms {
            delta_time: 0.0,
            system_phase: 0.0,
            prev_system_phase: 0.0,
            cycle: 0,

            amount: emitter.emission.particles_amount,
            lifetime: emitter.time.lifetime,
            lifetime_randomness: emitter.time.lifetime_randomness,
            emitting: 0,

            gravity: emitter.accelerations.gravity.into(),
            random_seed: runtime.random_seed,

            emission_shape,
            emission_sphere_radius,
            emission_ring_height,
            emission_ring_radius,

            emission_ring_inner_radius,
            spread: emitter.velocities.spread,
            flatness: emitter.velocities.flatness,
            initial_velocity_min: emitter.velocities.initial_velocity.min,

            initial_velocity_max: emitter.velocities.initial_velocity.max,
            inherit_velocity_ratio: emitter.velocities.inherit_ratio,
            explosiveness: emitter.time.explosiveness,
            spawn_time_randomness: emitter.time.spawn_time_randomness,

            emission_offset: emitter.emission.offset.into(),
            _pad1: 0.0,

            emission_scale: emitter.emission.scale.into(),
            _pad2: 0.0,

            emission_box_extents: emission_box_extents.into(),
            _pad3: 0.0,

            emission_ring_axis: emission_ring_axis.into(),
            _pad4: 0.0,

            direction: emitter.velocities.initial_direction.into(),
            _pad5: 0.0,

            velocity_pivot: emitter.velocities.pivot.into(),
            _pad6: 0.0,

            draw_order,
            clear_particles: 0,
            scale_min: emitter.scale.range.min,
            scale_max: emitter.scale.range.max,

            scale_over_lifetime: match &emitter.scale.scale_over_lifetime {
                Some(c) if !c.is_constant() => {
                    CurveUniform::enabled(c.range.min, c.range.max)
                }
                _ => CurveUniform::disabled(),
            },

            use_initial_color_gradient: match &emitter.colors.initial_color {
                SolidOrGradientColor::Solid { .. } => 0,
                SolidOrGradientColor::Gradient { .. } => 1,
            },
            turbulence_enabled: if turbulence.enabled { 1 } else { 0 },
            particle_flags: emitter.particle_flags.bits(),
            _pad7: 0,

            initial_color: match &emitter.colors.initial_color {
                SolidOrGradientColor::Solid { color } => *color,
                SolidOrGradientColor::Gradient { .. } => [1.0, 1.0, 1.0, 1.0],
            },

            alpha_over_lifetime: match &emitter.colors.alpha_over_lifetime {
                Some(c) if !c.is_constant() => {
                    CurveUniform::enabled(c.range.min, c.range.max)
                }
                _ => CurveUniform::disabled(),
            },
            emission_over_lifetime: match &emitter.colors.emission_over_lifetime {
                Some(c) if !c.is_constant() => {
                    CurveUniform::enabled(c.range.min, c.range.max)
                }
                _ => CurveUniform::disabled(),
            },

            turbulence_noise_strength: turbulence.noise_strength,
            turbulence_noise_scale: turbulence.noise_scale,
            turbulence_noise_speed_random: turbulence.noise_speed_random,
            turbulence_influence_min: turbulence.influence.min,

            turbulence_noise_speed: turbulence.noise_speed.into(),
            turbulence_influence_max: turbulence.influence.max,

            turbulence_influence_over_lifetime: turbulence
                .influence_over_lifetime
                .as_ref()
                .filter(|c| !c.is_constant())
                .map_or(CurveUniform::disabled(), |c| {
                    CurveUniform::enabled(c.range.min, c.range.max)
                }),

            radial_velocity: AnimatedVelocityUniform {
                min: emitter.velocities.radial_velocity.velocity.min,
                max: emitter.velocities.radial_velocity.velocity.max,
                _pad0: 0.0,
                _pad1: 0.0,
                curve: match &emitter.velocities.radial_velocity.velocity_over_lifetime {
                    Some(c) if !c.is_constant() => {
                        CurveUniform::enabled(c.range.min, c.range.max)
                    }
                    _ => CurveUniform::disabled(),
                },
            },

            collision_mode: match &emitter.collision.mode {
                Some(EmitterCollisionMode::Rigid { .. }) => COLLISION_MODE_RIGID,
                Some(EmitterCollisionMode::HideOnContact) => COLLISION_MODE_HIDE_ON_CONTACT,
                None => COLLISION_MODE_DISABLED,
            },
            collision_base_size: emitter.collision.base_size,
            collision_use_scale: emitter.collision.use_scale as u32,
            collision_friction: match &emitter.collision.mode {
                Some(EmitterCollisionMode::Rigid { friction, .. }) => *friction,
                _ => 0.0,
            },
            collision_bounce: match &emitter.collision.mode {
                Some(EmitterCollisionMode::Rigid { bounce, .. }) => *bounce,
                _ => 0.0,
            },
            collider_count: 0,
            _collision_pad0: 0.0,
            _collision_pad1: 0.0,
        };

        let uniform_steps: Vec<EmitterUniforms> = runtime
            .simulation_steps
            .iter()
            .map(|step| {
                let should_emit =
                    runtime.emitting && is_past_delay(step.system_time, &emitter.time);
                EmitterUniforms {
                    delta_time: step.delta_time,
                    system_phase: compute_phase(step.system_time, &emitter.time),
                    prev_system_phase: compute_phase(step.prev_system_time, &emitter.time),
                    cycle: step.cycle,
                    emitting: if should_emit { 1 } else { 0 },
                    clear_particles: if step.clear_requested { 1 } else { 0 },
                    ..base_uniforms
                }
            })
            .collect();

        let gradient_texture_handle = match &emitter.colors.initial_color {
            SolidOrGradientColor::Gradient { gradient } => gradient_cache.get(gradient),
            SolidOrGradientColor::Solid { .. } => None,
        };

        let scale_over_lifetime_texture_handle = emitter
            .scale
            .scale_over_lifetime
            .as_ref()
            .filter(|c| !c.is_constant())
            .and_then(|c| curve_cache.get(c));

        let alpha_over_lifetime_texture_handle = emitter
            .colors
            .alpha_over_lifetime
            .as_ref()
            .filter(|c| !c.is_constant())
            .and_then(|c| curve_cache.get(c));

        let emission_over_lifetime_texture_handle = emitter
            .colors
            .emission_over_lifetime
            .as_ref()
            .filter(|c| !c.is_constant())
            .and_then(|c| curve_cache.get(c));

        let turbulence_influence_over_lifetime_texture_handle = turbulence
            .influence_over_lifetime
            .as_ref()
            .filter(|c| !c.is_constant())
            .and_then(|c| curve_cache.get(c));

        let radial_velocity_curve_texture_handle = emitter
            .velocities
            .radial_velocity
            .velocity_over_lifetime
            .as_ref()
            .filter(|c| !c.is_constant())
            .and_then(|c| curve_cache.get(c));

        extracted.emitters.push((
            entity,
            ExtractedEmitterData {
                uniform_steps,
                particle_buffer_handle: buffer_handle.particle_buffer.clone(),
                indices_buffer_handle: buffer_handle.indices_buffer.clone(),
                sorted_particles_buffer_handle: buffer_handle.sorted_particles_buffer.clone(),
                amount: emitter.emission.particles_amount,
                draw_order,
                camera_position: camera_position.into(),
                camera_forward: camera_forward.into(),
                emitter_transform: global_transform.to_matrix(),
                gradient_texture_handle,
                scale_over_lifetime_texture_handle,
                alpha_over_lifetime_texture_handle,
                emission_over_lifetime_texture_handle,
                turbulence_influence_over_lifetime_texture_handle,
                radial_velocity_curve_texture_handle,
            },
        ));
    }

    commands.insert_resource(extracted);
}

pub fn extract_colliders(
    mut commands: Commands,
    colliders_query: Extract<Query<(&GlobalTransform, &ParticlesCollider3D)>>,
) {
    let mut colliders = Vec::new();

    for (global_transform, collider) in colliders_query.iter() {
        let transform = global_transform.to_matrix();
        let offset_transform = transform * Mat4::from_translation(collider.position);
        let inverse = offset_transform.inverse();

        let (extents, collider_type) = match &collider.shape {
            ParticlesColliderShape3D::Sphere { radius } => {
                ([*radius, 0.0, 0.0], COLLIDER_TYPE_SPHERE)
            }
            ParticlesColliderShape3D::Box { size } => {
                ((*size * 0.5).to_array(), COLLIDER_TYPE_BOX)
            }
        };

        colliders.push(ColliderUniform {
            transform: offset_transform.to_cols_array(),
            inverse_transform: inverse.to_cols_array(),
            extents,
            collider_type,
        });

        if colliders.len() >= MAX_COLLIDERS {
            break;
        }
    }

    commands.insert_resource(ExtractedColliders { colliders });
}
