use bevy::{
    prelude::*,
    render::{render_resource::ShaderType, storage::ShaderStorageBuffer, Extract},
};
use bytemuck::{Pod, Zeroable};

use crate::{
    asset::{DrawOrder, EmissionShape, ParticleSystemAsset, SolidOrGradientColor},
    runtime::{
        EmitterEntity, EmitterRuntime, ParticleBufferHandle, ParticleSystem3D, ParticleSystemRuntime,
    },
    textures::{CurveTextureCache, GradientTextureCache},
};

// emission shape constants
pub const EMISSION_SHAPE_POINT: u32 = 0;
pub const EMISSION_SHAPE_SPHERE: u32 = 1;
pub const EMISSION_SHAPE_SPHERE_SURFACE: u32 = 2;
pub const EMISSION_SHAPE_BOX: u32 = 3;
pub const EMISSION_SHAPE_RING: u32 = 4;

#[derive(Clone, Copy, Default, Pod, Zeroable, ShaderType)]
#[repr(C)]
pub struct SplineCurveUniform {
    pub enabled: u32,
    pub min_value: f32,
    pub max_value: f32,
    pub _pad: u32,
}

impl SplineCurveUniform {
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
    pub randomness: f32,

    pub emission_shape_offset: [f32; 3],
    pub _pad1: f32,

    pub emission_shape_scale: [f32; 3],
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

    pub scale_curve: SplineCurveUniform,

    pub use_initial_color_gradient: u32,
    pub turbulence_enabled: u32,
    pub particle_flags: u32,
    pub _pad7: u32,

    pub initial_color: [f32; 4],

    pub alpha_curve: SplineCurveUniform,
    pub emission_curve: SplineCurveUniform,

    pub turbulence_noise_strength: f32,
    pub turbulence_noise_scale: f32,
    pub turbulence_noise_speed_random: f32,
    pub turbulence_influence_min: f32,

    pub turbulence_noise_speed: [f32; 3],
    pub turbulence_influence_max: f32,

    pub turbulence_influence_curve: SplineCurveUniform,

    pub radial_velocity_min: f32,
    pub radial_velocity_max: f32,
    pub _pad8: f32,
    pub _pad9: f32,

    pub radial_velocity_curve: SplineCurveUniform,
}

#[derive(Resource, Default)]
pub struct ExtractedParticleSystem {
    pub emitters: Vec<(Entity, ExtractedEmitterData)>,
}

pub struct ExtractedEmitterData {
    pub uniforms: EmitterUniforms,
    pub particle_buffer_handle: Handle<ShaderStorageBuffer>,
    pub indices_buffer_handle: Handle<ShaderStorageBuffer>,
    pub sorted_particles_buffer_handle: Handle<ShaderStorageBuffer>,
    pub amount: u32,
    pub draw_order: u32,
    pub camera_position: [f32; 3],
    pub camera_forward: [f32; 3],
    pub emitter_transform: Mat4,
    pub gradient_texture_handle: Option<Handle<Image>>,
    pub curve_texture_handle: Option<Handle<Image>>,
    pub alpha_curve_texture_handle: Option<Handle<Image>>,
    pub emission_curve_texture_handle: Option<Handle<Image>>,
    pub turbulence_influence_curve_texture_handle: Option<Handle<Image>>,
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
    time: Extract<Res<Time>>,
) {
    let mut extracted = ExtractedParticleSystem::default();

    let (camera_position, camera_forward) = camera_query
        .iter()
        .next()
        .map(|t| (t.translation(), t.forward().as_vec3()))
        .unwrap_or((Vec3::ZERO, Vec3::NEG_Z));

    for (entity, emitter_entity, runtime, buffer_handle, global_transform) in emitter_query.iter() {
        let Ok((particle_system, system_runtime)) = system_query.get(emitter_entity.parent_system)
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

        let lifetime = emitter.time.lifetime;
        let delay = emitter.time.delay;
        let delta_time = if system_runtime.paused {
            0.0
        } else {
            time.delta_secs()
        };

        let should_emit = runtime.emitting && runtime.is_past_delay(lifetime, delay);

        let draw_order = match emitter.drawing.draw_order {
            DrawOrder::Index => 0,
            DrawOrder::Lifetime => 1,
            DrawOrder::ReverseLifetime => 2,
            DrawOrder::ViewDepth => 3,
        };

        let spawn = &emitter.process.spawn;
        let position = &spawn.position;
        let velocity = &spawn.velocity;
        let accelerations = &emitter.process.accelerations;
        let display = &emitter.process.display;

        let (emission_shape, emission_sphere_radius, emission_box_extents, emission_ring_axis, emission_ring_height, emission_ring_radius, emission_ring_inner_radius) =
            match position.emission_shape {
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

        let uniforms = EmitterUniforms {
            delta_time,
            system_phase: runtime.system_phase(lifetime, delay),
            prev_system_phase: runtime.prev_system_phase(lifetime, delay),
            cycle: runtime.cycle,

            amount: emitter.amount,
            lifetime: emitter.time.lifetime,
            lifetime_randomness: emitter.time.lifetime_randomness,
            emitting: if should_emit { 1 } else { 0 },

            gravity: accelerations.gravity.into(),
            random_seed: runtime.random_seed,

            emission_shape,
            emission_sphere_radius,
            emission_ring_height,
            emission_ring_radius,

            emission_ring_inner_radius,
            spread: velocity.spread,
            flatness: velocity.flatness,
            initial_velocity_min: velocity.initial_velocity.min,

            initial_velocity_max: velocity.initial_velocity.max,
            inherit_velocity_ratio: velocity.inherit_velocity_ratio,
            explosiveness: emitter.time.explosiveness,
            randomness: emitter.time.randomness,

            emission_shape_offset: position.emission_shape_offset.into(),
            _pad1: 0.0,

            emission_shape_scale: position.emission_shape_scale.into(),
            _pad2: 0.0,

            emission_box_extents: emission_box_extents.into(),
            _pad3: 0.0,

            emission_ring_axis: emission_ring_axis.into(),
            _pad4: 0.0,

            direction: velocity.direction.into(),
            _pad5: 0.0,

            velocity_pivot: velocity.velocity_pivot.into(),
            _pad6: 0.0,

            draw_order,
            clear_particles: if runtime.clear_requested { 1 } else { 0 },
            scale_min: display.scale.range.min,
            scale_max: display.scale.range.max,

            scale_curve: match &display.scale.curve {
                Some(c) if !c.is_constant() => {
                    SplineCurveUniform::enabled(c.min_value, c.max_value)
                }
                _ => SplineCurveUniform::disabled(),
            },

            use_initial_color_gradient: match &display.color_curves.initial_color {
                SolidOrGradientColor::Solid { .. } => 0,
                SolidOrGradientColor::Gradient { .. } => 1,
            },
            turbulence_enabled: match &emitter.process.turbulence {
                Some(t) if t.enabled => 1,
                _ => 0,
            },
            particle_flags: emitter.process.particle_flags.bits(),
            _pad7: 0,

            initial_color: match &display.color_curves.initial_color {
                SolidOrGradientColor::Solid { color } => *color,
                SolidOrGradientColor::Gradient { .. } => [1.0, 1.0, 1.0, 1.0],
            },

            alpha_curve: match &display.color_curves.alpha_curve {
                Some(c) if !c.is_constant() => {
                    SplineCurveUniform::enabled(c.min_value, c.max_value)
                }
                _ => SplineCurveUniform::disabled(),
            },
            emission_curve: match &display.color_curves.emission_curve {
                Some(c) if !c.is_constant() => {
                    SplineCurveUniform::enabled(c.min_value, c.max_value)
                }
                _ => SplineCurveUniform::disabled(),
            },

            turbulence_noise_strength: emitter
                .process
                .turbulence
                .as_ref()
                .map(|t| t.noise_strength)
                .unwrap_or(1.0),
            turbulence_noise_scale: emitter
                .process
                .turbulence
                .as_ref()
                .map(|t| t.noise_scale)
                .unwrap_or(2.5),
            turbulence_noise_speed_random: emitter
                .process
                .turbulence
                .as_ref()
                .map(|t| t.noise_speed_random)
                .unwrap_or(0.0),
            turbulence_influence_min: emitter
                .process
                .turbulence
                .as_ref()
                .map(|t| t.influence.min)
                .unwrap_or(0.0),

            turbulence_noise_speed: emitter
                .process
                .turbulence
                .as_ref()
                .map(|t| t.noise_speed.into())
                .unwrap_or([0.0, 0.0, 0.0]),
            turbulence_influence_max: emitter
                .process
                .turbulence
                .as_ref()
                .map(|t| t.influence.max)
                .unwrap_or(0.1),

            turbulence_influence_curve: match &emitter.process.turbulence {
                Some(t) => match &t.influence_curve {
                    Some(c) if !c.is_constant() => {
                        SplineCurveUniform::enabled(c.min_value, c.max_value)
                    }
                    _ => SplineCurveUniform::disabled(),
                },
                None => SplineCurveUniform::disabled(),
            },

            radial_velocity_min: emitter.process.animated_velocity.radial_velocity.value.min,
            radial_velocity_max: emitter.process.animated_velocity.radial_velocity.value.max,
            _pad8: 0.0,
            _pad9: 0.0,

            radial_velocity_curve: match &emitter.process.animated_velocity.radial_velocity.curve {
                Some(c) if !c.is_constant() => {
                    SplineCurveUniform::enabled(c.min_value, c.max_value)
                }
                _ => SplineCurveUniform::disabled(),
            },
        };

        let gradient_texture_handle = match &display.color_curves.initial_color {
            SolidOrGradientColor::Gradient { gradient } => gradient_cache.get(gradient),
            SolidOrGradientColor::Solid { .. } => None,
        };

        let curve_texture_handle = display
            .scale
            .curve
            .as_ref()
            .filter(|c| !c.is_constant())
            .and_then(|c| curve_cache.get(&c.curve));

        let alpha_curve_texture_handle = display
            .color_curves
            .alpha_curve
            .as_ref()
            .filter(|c| !c.is_constant())
            .and_then(|c| curve_cache.get(&c.curve));

        let emission_curve_texture_handle = display
            .color_curves
            .emission_curve
            .as_ref()
            .filter(|c| !c.is_constant())
            .and_then(|c| curve_cache.get(&c.curve));

        let turbulence_influence_curve_texture_handle = emitter
            .process
            .turbulence
            .as_ref()
            .and_then(|t| t.influence_curve.as_ref())
            .filter(|c| !c.is_constant())
            .and_then(|c| curve_cache.get(&c.curve));

        let radial_velocity_curve_texture_handle = emitter
            .process
            .animated_velocity
            .radial_velocity
            .curve
            .as_ref()
            .filter(|c| !c.is_constant())
            .and_then(|c| curve_cache.get(&c.curve));

        extracted.emitters.push((
            entity,
            ExtractedEmitterData {
                uniforms,
                particle_buffer_handle: buffer_handle.particle_buffer.clone(),
                indices_buffer_handle: buffer_handle.indices_buffer.clone(),
                sorted_particles_buffer_handle: buffer_handle.sorted_particles_buffer.clone(),
                amount: emitter.amount,
                draw_order,
                camera_position: camera_position.into(),
                camera_forward: camera_forward.into(),
                emitter_transform: global_transform.to_matrix(),
                gradient_texture_handle,
                curve_texture_handle,
                alpha_curve_texture_handle,
                emission_curve_texture_handle,
                turbulence_influence_curve_texture_handle,
                radial_velocity_curve_texture_handle,
            },
        ));
    }

    commands.insert_resource(extracted);
}
