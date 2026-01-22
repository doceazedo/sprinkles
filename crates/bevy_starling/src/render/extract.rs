use bevy::{
    prelude::*,
    render::{Extract, storage::ShaderStorageBuffer},
};

use crate::{
    asset::{DrawOrder, ParticleSystemAsset},
    core::ParticleSystem3D,
    runtime::{ParticleBufferHandle, ParticleSystemRuntime},
};

use super::EmitterUniforms;

#[derive(Resource, Default)]
pub struct ExtractedParticleSystem {
    pub emitters: Vec<(Entity, ExtractedEmitterData)>,
}

pub struct ExtractedEmitterData {
    pub uniforms: EmitterUniforms,
    pub particle_buffer_handle: Handle<ShaderStorageBuffer>,
    pub indices_buffer_handle: Handle<ShaderStorageBuffer>,
    pub amount: u32,
    pub draw_order: u32,
    pub camera_position: [f32; 3],
    pub emitter_transform: Mat4,
}

pub fn extract_particle_systems(
    mut commands: Commands,
    query: Extract<
        Query<(
            Entity,
            &ParticleSystemRuntime,
            &ParticleBufferHandle,
            &ParticleSystem3D,
            &GlobalTransform,
        )>,
    >,
    camera_query: Extract<Query<&GlobalTransform, With<Camera3d>>>,
    assets: Extract<Res<Assets<ParticleSystemAsset>>>,
    time: Extract<Res<Time>>,
) {
    let mut extracted = ExtractedParticleSystem::default();

    // get camera position for view depth sorting
    let camera_position = camera_query
        .iter()
        .next()
        .map(|t| t.translation())
        .unwrap_or(Vec3::ZERO);

    for (entity, runtime, buffer_handle, particle_system, global_transform) in query.iter() {
        let Some(asset) = assets.get(&particle_system.handle) else {
            continue;
        };

        let Some(emitter) = asset.emitters.first() else {
            continue;
        };

        if !emitter.enabled {
            continue;
        }

        let lifetime = emitter.lifetime;
        // always use actual frame delta for physics simulation
        // fixed_fps only affects emission timing via system_phase
        let delta_time = time.delta_secs();

        let draw_order = match emitter.draw_order {
            DrawOrder::Index => 0,
            DrawOrder::Lifetime => 1,
            DrawOrder::ReverseLifetime => 2,
            DrawOrder::ViewDepth => 3,
        };

        let uniforms = EmitterUniforms {
            delta_time,
            system_phase: runtime.system_phase(lifetime),
            prev_system_phase: runtime.prev_system_phase(lifetime),
            cycle: runtime.cycle,

            amount: emitter.amount,
            lifetime: emitter.lifetime,
            lifetime_randomness: emitter.lifetime_randomness,
            emitting: if runtime.emitting { 1 } else { 0 },

            gravity: emitter.process.gravity.into(),
            random_seed: runtime.random_seed,

            initial_velocity: emitter.process.initial_velocity.into(),
            _pad1: 0.0,
            initial_velocity_randomness: emitter.process.initial_velocity_randomness.into(),
            _pad2: 0.0,

            initial_scale: emitter.process.initial_scale,
            initial_scale_randomness: emitter.process.initial_scale_randomness,
            explosiveness: emitter.explosiveness,
            randomness: emitter.randomness,

            draw_order,
            _pad3: [0; 3],
        };

        extracted.emitters.push((
            entity,
            ExtractedEmitterData {
                uniforms,
                particle_buffer_handle: buffer_handle.particle_buffer.clone(),
                indices_buffer_handle: buffer_handle.indices_buffer.clone(),
                amount: emitter.amount,
                draw_order,
                camera_position: camera_position.into(),
                emitter_transform: global_transform.to_matrix(),
            },
        ));
    }

    commands.insert_resource(extracted);
}
