use bevy::{
    pbr::ExtendedMaterial,
    prelude::*,
    render::storage::ShaderStorageBuffer,
};

use crate::{
    asset::{DrawOrder, ParticleMesh, ParticleSystemAsset},
    core::{ParticleData, ParticleSystem3D},
    render::material::ParticleMaterialExtension,
    runtime::{ParticleBufferHandle, ParticleEntity, ParticleSystemRef, ParticleSystemRuntime},
};

pub type ParticleMaterial = ExtendedMaterial<StandardMaterial, ParticleMaterialExtension>;

pub fn setup_particle_systems(
    mut commands: Commands,
    query: Query<(Entity, &ParticleSystem3D), Without<ParticleSystemRuntime>>,
    assets: Res<Assets<ParticleSystemAsset>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut buffers: ResMut<Assets<ShaderStorageBuffer>>,
    mut materials: ResMut<Assets<ParticleMaterial>>,
) {
    for (entity, particle_system) in query.iter() {
        let Some(asset) = assets.get(&particle_system.handle) else {
            continue;
        };

        let Some(emitter) = asset.emitters.first() else {
            continue;
        };

        let amount = emitter.amount;

        // initialize particle data buffer (all particles start inactive)
        let particles: Vec<ParticleData> = (0..amount).map(|_| ParticleData::default()).collect();

        // create ShaderStorageBuffer asset for the particle data
        let particle_buffer_handle = buffers.add(ShaderStorageBuffer::from(particles));

        // initialize particle indices buffer (identity mapping)
        let indices: Vec<u32> = (0..amount).collect();
        let indices_buffer_handle = buffers.add(ShaderStorageBuffer::from(indices));

        // create mesh based on draw pass configuration
        let mesh_handle = if let Some(draw_pass) = emitter.draw_passes.first() {
            match &draw_pass.mesh {
                ParticleMesh::Quad => meshes.add(Rectangle::new(1.0, 1.0)),
                ParticleMesh::Sphere {
                    radius,
                    rings,
                    sectors,
                } => meshes.add(Sphere::new(*radius).mesh().uv(*sectors, *rings)),
                ParticleMesh::Cube { size } => meshes.add(Cuboid::new(*size, *size, *size)),
            }
        } else {
            meshes.add(Rectangle::new(1.0, 1.0))
        };

        let use_index_draw_order = emitter.draw_order == DrawOrder::Index;

        // add runtime components to the particle system entity
        commands.entity(entity).insert((
            ParticleSystemRuntime::default(),
            ParticleBufferHandle {
                particle_buffer: particle_buffer_handle.clone(),
                indices_buffer: indices_buffer_handle.clone(),
                max_particles: amount,
            },
            Transform::default(),
            Visibility::default(),
        ));

        // spawn individual particle entities
        for i in 0..amount {
            let depth_bias = if use_index_draw_order { i as f32 } else { 0.0 };

            let material_handle = materials.add(ExtendedMaterial {
                base: StandardMaterial {
                    base_color: Color::WHITE,
                    alpha_mode: AlphaMode::Blend,
                    depth_bias,
                    ..default()
                },
                extension: ParticleMaterialExtension {
                    particles: particle_buffer_handle.clone(),
                    indices: indices_buffer_handle.clone(),
                },
            });

            commands.spawn((
                Mesh3d(mesh_handle.clone()),
                MeshMaterial3d(material_handle),
                bevy::mesh::MeshTag(i),
                Transform::default(),
                Visibility::default(),
                ParticleEntity,
                ParticleSystemRef(entity),
            ));
        }
    }
}

/// cleanup particle entities when their parent particle system is despawned
pub fn cleanup_particle_entities(
    mut commands: Commands,
    mut removed_systems: RemovedComponents<ParticleSystem3D>,
    particle_entities: Query<(Entity, &ParticleSystemRef), With<ParticleEntity>>,
) {
    for removed_entity in removed_systems.read() {
        for (entity, system_ref) in particle_entities.iter() {
            if system_ref.0 == removed_entity {
                commands.entity(entity).despawn();
            }
        }
    }
}
