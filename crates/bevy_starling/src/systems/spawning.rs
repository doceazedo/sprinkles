use bevy::{
    asset::RenderAssetUsages,
    mesh::{Indices, PrimitiveTopology, VertexAttributeValues},
    pbr::ExtendedMaterial,
    prelude::*,
    render::storage::ShaderStorageBuffer,
};

use crate::{
    asset::{ParticleMesh, ParticleSystemAsset},
    core::{ParticleData, ParticleSystem3D},
    render::material::ParticleMaterialExtension,
    runtime::{
        CurrentMeshConfig, EmitterEntity, EmitterMeshEntity, EmitterRuntime, ParticleBufferHandle,
        ParticleMaterial, ParticleMaterialHandle, ParticleMeshHandle, ParticleSystemRuntime,
    },
};

/// creates a base mesh from the particle mesh configuration
fn create_base_mesh(config: &ParticleMesh) -> Mesh {
    match config {
        ParticleMesh::Quad => Mesh::from(Rectangle::new(1.0, 1.0)),
        ParticleMesh::Sphere { radius } => Mesh::from(Sphere::new(*radius)),
        ParticleMesh::Cuboid { half_size } => {
            Mesh::from(Cuboid::new(half_size.x * 2.0, half_size.y * 2.0, half_size.z * 2.0))
        }
    }
}

/// creates a merged mesh containing `particle_count` copies of the base mesh,
/// each with its particle index encoded in the UV_1 (uv_b) attribute.
/// this eliminates reliance on instance_index.
fn create_particle_mesh(
    config: &ParticleMesh,
    particle_count: u32,
    meshes: &mut Assets<Mesh>,
) -> Handle<Mesh> {
    let base_mesh = create_base_mesh(config);

    // extract base mesh data
    let base_positions: Vec<[f32; 3]> = base_mesh
        .attribute(Mesh::ATTRIBUTE_POSITION)
        .and_then(|attr| match attr {
            VertexAttributeValues::Float32x3(v) => Some(v.clone()),
            _ => None,
        })
        .unwrap_or_default();

    let base_normals: Vec<[f32; 3]> = base_mesh
        .attribute(Mesh::ATTRIBUTE_NORMAL)
        .and_then(|attr| match attr {
            VertexAttributeValues::Float32x3(v) => Some(v.clone()),
            _ => None,
        })
        .unwrap_or_else(|| vec![[0.0, 0.0, 1.0]; base_positions.len()]);

    let base_uvs: Vec<[f32; 2]> = base_mesh
        .attribute(Mesh::ATTRIBUTE_UV_0)
        .and_then(|attr| match attr {
            VertexAttributeValues::Float32x2(v) => Some(v.clone()),
            _ => None,
        })
        .unwrap_or_else(|| vec![[0.0, 0.0]; base_positions.len()]);

    let base_indices: Vec<u32> = base_mesh
        .indices()
        .map(|indices| indices.iter().map(|i| i as u32).collect())
        .unwrap_or_else(|| (0..base_positions.len() as u32).collect());

    let vertices_per_mesh = base_positions.len();
    let indices_per_mesh = base_indices.len();

    let total_vertices = particle_count as usize * vertices_per_mesh;
    let total_indices = particle_count as usize * indices_per_mesh;

    let mut positions: Vec<[f32; 3]> = Vec::with_capacity(total_vertices);
    let mut normals: Vec<[f32; 3]> = Vec::with_capacity(total_vertices);
    let mut uvs: Vec<[f32; 2]> = Vec::with_capacity(total_vertices);
    let mut uv_bs: Vec<[f32; 2]> = Vec::with_capacity(total_vertices);
    let mut indices: Vec<u32> = Vec::with_capacity(total_indices);

    for particle_idx in 0..particle_count {
        let base_vertex = (particle_idx as usize * vertices_per_mesh) as u32;
        let particle_index_f32 = particle_idx as f32;

        // copy all vertices from base mesh
        for i in 0..vertices_per_mesh {
            positions.push(base_positions[i]);
            normals.push(base_normals[i]);
            uvs.push(base_uvs[i]);
            uv_bs.push([particle_index_f32, 0.0]); // particle index in uv_b.x
        }

        // copy indices with offset
        for &idx in &base_indices {
            indices.push(base_vertex + idx);
        }
    }

    let mut mesh = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::default());
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_1, uv_bs);
    mesh.insert_indices(Indices::U32(indices));

    meshes.add(mesh)
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

            // create merged particle mesh with particle indices encoded in UV_1
            let particle_mesh_handle = create_particle_mesh(&current_mesh, amount, &mut meshes);

            // create a single shared material for all particles in this emitter
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
            let fixed_seed = if emitter.time.use_fixed_seed {
                Some(emitter.time.seed)
            } else {
                None
            };
            let emitter_entity = commands
                .spawn((
                    EmitterEntity {
                        parent_system: system_entity,
                    },
                    EmitterRuntime::new(emitter_index, fixed_seed),
                    ParticleBufferHandle {
                        particle_buffer: particle_buffer_handle.clone(),
                        indices_buffer: indices_buffer_handle.clone(),
                        sorted_particles_buffer: sorted_particles_buffer_handle.clone(),
                        max_particles: amount,
                    },
                    CurrentMeshConfig(current_mesh),
                    ParticleMeshHandle(particle_mesh_handle.clone()),
                    ParticleMaterialHandle(material_handle.clone()),
                    Transform::default(),
                    Visibility::default(),
                ))
                .id();

            commands.entity(system_entity).add_child(emitter_entity);

            // spawn single mesh entity for this emitter (contains all particles in one mesh)
            commands.spawn((
                Mesh3d(particle_mesh_handle),
                MeshMaterial3d(material_handle),
                Transform::default(),
                Visibility::default(),
                EmitterMeshEntity { emitter_entity },
            ));
        }
    }
}

/// cleanup mesh entities when their parent emitter or particle system is despawned
pub fn cleanup_particle_entities(
    mut commands: Commands,
    mut removed_systems: RemovedComponents<ParticleSystem3D>,
    mut removed_emitters: RemovedComponents<EmitterEntity>,
    mesh_entities: Query<(Entity, &EmitterMeshEntity)>,
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

        // despawn all mesh entities that belong to emitters of this system
        for (mesh_entity, emitter_mesh) in mesh_entities.iter() {
            if let Ok(emitter) = emitter_parent_query.get(emitter_mesh.emitter_entity) {
                if emitter.parent_system == removed_system {
                    commands.entity(mesh_entity).despawn();
                }
            }
        }
    }

    // cleanup when emitter is removed
    for removed_emitter in removed_emitters.read() {
        for (mesh_entity, emitter_mesh) in mesh_entities.iter() {
            if emitter_mesh.emitter_entity == removed_emitter {
                commands.entity(mesh_entity).despawn();
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
        &ParticleBufferHandle,
        &mut CurrentMeshConfig,
        &mut ParticleMeshHandle,
    )>,
    mut mesh_entities: Query<(&EmitterMeshEntity, &mut Mesh3d)>,
    assets: Res<Assets<ParticleSystemAsset>>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    for (emitter_entity, emitter, runtime, buffer_handle, mut current_config, mut mesh_handle) in
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
            // regenerate particle mesh with updated configuration
            let new_mesh_handle =
                create_particle_mesh(&new_mesh, buffer_handle.max_particles, &mut meshes);

            // update the mesh entity for this emitter
            for (emitter_mesh, mut mesh3d) in mesh_entities.iter_mut() {
                if emitter_mesh.emitter_entity == emitter_entity {
                    mesh3d.0 = new_mesh_handle.clone();
                }
            }

            current_config.0 = new_mesh;
            mesh_handle.0 = new_mesh_handle;
        }
    }
}
