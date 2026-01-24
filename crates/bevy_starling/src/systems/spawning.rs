use bevy::{
    pbr::ExtendedMaterial,
    prelude::*,
    render::storage::ShaderStorageBuffer,
};

use crate::{
    asset::{ParticleMesh, ParticleSystemAsset},
    core::{ParticleData, ParticleSystem3D},
    render::material::ParticleMaterialExtension,
    runtime::{
        CurrentMeshConfig, EmitterEntity, EmitterRuntime, ParticleBufferHandle, ParticleEntity,
        ParticleMaterial, ParticleMaterialHandle, ParticleMeshHandle, ParticleSystemRef,
        ParticleSystemRuntime,
    },
};

fn create_mesh_from_config(config: &ParticleMesh, meshes: &mut Assets<Mesh>) -> Handle<Mesh> {
    match config {
        ParticleMesh::Quad => meshes.add(Rectangle::new(1.0, 1.0)),
        ParticleMesh::Sphere { radius } => meshes.add(Sphere::new(*radius)),
        ParticleMesh::Cuboid { half_size } => {
            meshes.add(Cuboid::new(half_size.x * 2.0, half_size.y * 2.0, half_size.z * 2.0))
        }
    }
}

pub fn setup_particle_systems(
    mut commands: Commands,
    query: Query<(Entity, &ParticleSystem3D), Without<ParticleSystemRuntime>>,
    assets: Res<Assets<ParticleSystemAsset>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut buffers: ResMut<Assets<ShaderStorageBuffer>>,
    mut materials: ResMut<Assets<ParticleMaterial>>,
) {
    for (system_entity, particle_system) in query.iter() {
        let Some(asset) = assets.get(&particle_system.handle) else {
            continue;
        };

        if asset.emitters.is_empty() {
            continue;
        }

        // add system-wide runtime to the particle system entity
        commands.entity(system_entity).insert((
            ParticleSystemRuntime::default(),
            Transform::default(),
            Visibility::default(),
        ));

        // spawn an emitter entity for each emitter in the asset
        for (emitter_index, emitter) in asset.emitters.iter().enumerate() {
            let amount = emitter.amount;

            // initialize particle data buffer (all particles start inactive)
            let particles: Vec<ParticleData> =
                (0..amount).map(|_| ParticleData::default()).collect();

            // create ShaderStorageBuffer asset for the particle data
            let particle_buffer_handle = buffers.add(ShaderStorageBuffer::from(particles.clone()));

            // initialize particle indices buffer (identity mapping)
            let indices: Vec<u32> = (0..amount).collect();
            let indices_buffer_handle = buffers.add(ShaderStorageBuffer::from(indices));

            // create sorted particles buffer (same size, written in sorted order for rendering)
            let sorted_particles_buffer_handle = buffers.add(ShaderStorageBuffer::from(particles));

            // create mesh based on draw pass configuration
            let current_mesh = if let Some(draw_pass) = emitter.draw_passes.first() {
                draw_pass.mesh.clone()
            } else {
                ParticleMesh::Quad
            };

            let mesh_handle = create_mesh_from_config(&current_mesh, &mut meshes);

            // create a single shared material for all particles in this emitter (enables automatic instancing)
            let material_handle = materials.add(ExtendedMaterial {
                base: StandardMaterial {
                    base_color: Color::WHITE,
                    alpha_mode: AlphaMode::Blend,
                    ..default()
                },
                extension: ParticleMaterialExtension {
                    sorted_particles: sorted_particles_buffer_handle.clone(),
                    max_particles: amount,
                },
            });

            // spawn emitter entity as child of particle system
            let emitter_entity = commands
                .spawn((
                    EmitterEntity {
                        parent_system: system_entity,
                    },
                    EmitterRuntime::new(emitter_index),
                    ParticleBufferHandle {
                        particle_buffer: particle_buffer_handle.clone(),
                        indices_buffer: indices_buffer_handle.clone(),
                        sorted_particles_buffer: sorted_particles_buffer_handle.clone(),
                        max_particles: amount,
                    },
                    CurrentMeshConfig(current_mesh),
                    ParticleMeshHandle(mesh_handle.clone()),
                    ParticleMaterialHandle(material_handle.clone()),
                    Transform::default(),
                    Visibility::default(),
                ))
                .id();

            commands.entity(system_entity).add_child(emitter_entity);

            // spawn individual particle entities with shared mesh and material (automatic instancing)
            for i in 0..amount {
                commands.spawn((
                    Mesh3d(mesh_handle.clone()),
                    MeshMaterial3d(material_handle.clone()),
                    bevy::mesh::MeshTag(i),
                    Transform::default(),
                    Visibility::default(),
                    ParticleEntity,
                    ParticleSystemRef {
                        system_entity,
                        emitter_entity,
                    },
                ));
            }
        }
    }
}

/// cleanup particle entities when their parent particle system is despawned
pub fn cleanup_particle_entities(
    mut commands: Commands,
    mut removed_systems: RemovedComponents<ParticleSystem3D>,
    mut removed_emitters: RemovedComponents<EmitterEntity>,
    particle_entities: Query<(Entity, &ParticleSystemRef), With<ParticleEntity>>,
    emitter_entities: Query<Entity, With<EmitterEntity>>,
    emitter_parent_query: Query<&EmitterEntity>,
) {
    // cleanup when particle system is removed
    for removed_system in removed_systems.read() {
        // despawn all emitter entities that belong to this system
        for emitter_entity in emitter_entities.iter() {
            if let Ok(emitter) = emitter_parent_query.get(emitter_entity) {
                if emitter.parent_system == removed_system {
                    commands.entity(emitter_entity).despawn();
                }
            }
        }

        // despawn all particle entities that belong to this system
        for (entity, system_ref) in particle_entities.iter() {
            if system_ref.system_entity == removed_system {
                commands.entity(entity).despawn();
            }
        }
    }

    // cleanup when emitter is removed
    for removed_emitter in removed_emitters.read() {
        for (entity, system_ref) in particle_entities.iter() {
            if system_ref.emitter_entity == removed_emitter {
                commands.entity(entity).despawn();
            }
        }
    }
}

/// sync particle mesh when asset configuration changes
pub fn sync_particle_mesh(
    particle_systems: Query<&ParticleSystem3D>,
    mut emitter_query: Query<(
        Entity,
        &EmitterEntity,
        &EmitterRuntime,
        &mut CurrentMeshConfig,
        &mut ParticleMeshHandle,
    )>,
    mut particle_entities: Query<(&ParticleSystemRef, &mut Mesh3d), With<ParticleEntity>>,
    assets: Res<Assets<ParticleSystemAsset>>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    for (emitter_entity, emitter, runtime, mut current_config, mut mesh_handle) in
        emitter_query.iter_mut()
    {
        let Ok(particle_system) = particle_systems.get(emitter.parent_system) else {
            continue;
        };

        let Some(asset) = assets.get(&particle_system.handle) else {
            continue;
        };

        let Some(emitter_data) = asset.emitters.get(runtime.emitter_index) else {
            continue;
        };

        let new_mesh = if let Some(draw_pass) = emitter_data.draw_passes.first() {
            draw_pass.mesh.clone()
        } else {
            ParticleMesh::Quad
        };

        if current_config.0 != new_mesh {
            let new_mesh_handle = create_mesh_from_config(&new_mesh, &mut meshes);

            for (system_ref, mut mesh3d) in particle_entities.iter_mut() {
                if system_ref.emitter_entity == emitter_entity {
                    mesh3d.0 = new_mesh_handle.clone();
                }
            }

            current_config.0 = new_mesh;
            mesh_handle.0 = new_mesh_handle;
        }
    }
}
