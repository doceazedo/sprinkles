use bevy::{
    pbr::ExtendedMaterial,
    prelude::*,
    render::storage::ShaderStorageBuffer,
};

use crate::{
    asset::{DrawOrder, ParticleMesh, ParticleSystemAsset},
    core::{ParticleData, ParticleSystem3D},
    render::material::ParticleMaterialExtension,
    runtime::{
        CurrentMeshConfig, ParticleBufferHandle, ParticleEntity, ParticleMeshHandle,
        ParticleSystemRef, ParticleSystemRuntime,
    },
};

pub type ParticleMaterial = ExtendedMaterial<StandardMaterial, ParticleMaterialExtension>;

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
        let current_mesh = if let Some(draw_pass) = emitter.draw_passes.first() {
            draw_pass.mesh.clone()
        } else {
            ParticleMesh::Quad
        };

        let mesh_handle = create_mesh_from_config(&current_mesh, &mut meshes);

        let use_index_draw_order = emitter.drawing.draw_order == DrawOrder::Index;

        // add runtime components to the particle system entity
        commands.entity(entity).insert((
            ParticleSystemRuntime::default(),
            ParticleBufferHandle {
                particle_buffer: particle_buffer_handle.clone(),
                indices_buffer: indices_buffer_handle.clone(),
                max_particles: amount,
            },
            CurrentMeshConfig(current_mesh),
            ParticleMeshHandle(mesh_handle.clone()),
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

/// sync particle mesh when asset configuration changes
pub fn sync_particle_mesh(
    mut particle_systems: Query<(
        Entity,
        &ParticleSystem3D,
        &mut CurrentMeshConfig,
        &mut ParticleMeshHandle,
    )>,
    mut particle_entities: Query<(&ParticleSystemRef, &mut Mesh3d), With<ParticleEntity>>,
    assets: Res<Assets<ParticleSystemAsset>>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    for (system_entity, particle_system, mut current_config, mut mesh_handle) in
        particle_systems.iter_mut()
    {
        let Some(asset) = assets.get(&particle_system.handle) else {
            continue;
        };

        let Some(emitter) = asset.emitters.first() else {
            continue;
        };

        let new_mesh = if let Some(draw_pass) = emitter.draw_passes.first() {
            draw_pass.mesh.clone()
        } else {
            ParticleMesh::Quad
        };

        if current_config.0 != new_mesh {
            let new_mesh_handle = create_mesh_from_config(&new_mesh, &mut meshes);

            for (system_ref, mut mesh3d) in particle_entities.iter_mut() {
                if system_ref.0 == system_entity {
                    mesh3d.0 = new_mesh_handle.clone();
                }
            }

            current_config.0 = new_mesh;
            mesh_handle.0 = new_mesh_handle;
        }
    }
}
