use bevy::{
    prelude::*,
    render::{Extract, storage::ShaderStorageBuffer},
};

use crate::{
    asset::{DrawOrder, EmissionShape, ParticleSystemAsset, SolidOrGradientColor},
    core::ParticleSystem3D,
    render::gradient_texture::GradientTextureCache,
    runtime::{EmitterEntity, EmitterRuntime, ParticleBufferHandle, ParticleSystemRuntime},
};

use super::{
    EmitterUniforms, EMISSION_SHAPE_BOX, EMISSION_SHAPE_POINT, EMISSION_SHAPE_RING,
    EMISSION_SHAPE_SPHERE, EMISSION_SHAPE_SPHERE_SURFACE,
};

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
    time: Extract<Res<Time>>,
) {
    let mut extracted = ExtractedParticleSystem::default();

    // get camera position and forward direction for view depth sorting
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
        // use actual frame delta for physics simulation, but freeze when paused
        // fixed_fps only affects emission timing via system_phase
        let delta_time = if system_runtime.paused {
            0.0
        } else {
            time.delta_secs()
        };

        // only emit when past the delay period
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
        let accelerations = &spawn.accelerations;
        let display = &emitter.process.display;

        // convert emission shape to u32 discriminant and extract shape-specific parameters
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

            scale_curve: display.scale.curve.map(|c| c.to_gpu_constant()).unwrap_or(0),
            use_initial_color_gradient: match &display.color_curves.initial_color {
                SolidOrGradientColor::Solid { .. } => 0,
                SolidOrGradientColor::Gradient { .. } => 1,
            },
            _pad7: [0; 2],

            initial_color: match &display.color_curves.initial_color {
                SolidOrGradientColor::Solid { color } => *color,
                SolidOrGradientColor::Gradient { .. } => [1.0, 1.0, 1.0, 1.0],
            },
        };

        let gradient_texture_handle = match &display.color_curves.initial_color {
            SolidOrGradientColor::Gradient { gradient } => gradient_cache.get(gradient),
            SolidOrGradientColor::Solid { .. } => None,
        };

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
            },
        ));
    }

    commands.insert_resource(extracted);
}
