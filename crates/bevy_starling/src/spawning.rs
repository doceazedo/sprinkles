use bevy::{
    asset::RenderAssetUsages,
    mesh::{Indices, PrimitiveTopology, VertexAttributeValues},
    pbr::ExtendedMaterial,
    prelude::*,
    render::storage::ShaderStorageBuffer,
};

use crate::{
    asset::{DrawPassMaterial, ParticleMesh, ParticleSystemAsset, QuadOrientation},
    material::ParticleMaterialExtension,
    runtime::{
        CurrentMaterialConfig, CurrentMeshConfig, EmitterEntity, EmitterMeshEntity, EmitterRuntime,
        ParticleBufferHandle, ParticleData, ParticleMaterial, ParticleMaterialHandle,
        ParticleMeshHandle, ParticleSystem3D, ParticleSystemRuntime,
    },
};

// time systems

pub fn clear_particle_clear_requests(mut query: Query<&mut EmitterRuntime>) {
    for mut runtime in query.iter_mut() {
        if runtime.clear_requested {
            runtime.clear_requested = false;
        }
    }
}

pub fn update_particle_time(
    time: Res<Time>,
    assets: Res<Assets<ParticleSystemAsset>>,
    system_query: Query<(&ParticleSystem3D, &ParticleSystemRuntime)>,
    mut emitter_query: Query<(&EmitterEntity, &mut EmitterRuntime)>,
) {
    for (emitter, mut runtime) in emitter_query.iter_mut() {
        let Ok((particle_system, system_runtime)) = system_query.get(emitter.parent_system) else {
            continue;
        };

        if system_runtime.paused {
            continue;
        }

        let Some(asset) = assets.get(&particle_system.handle) else {
            continue;
        };

        let Some(emitter_data) = asset.emitters.get(runtime.emitter_index) else {
            continue;
        };

        let lifetime = emitter_data.time.lifetime;
        let delay = emitter_data.time.delay;
        let fixed_fps = emitter_data.time.fixed_fps;
        let total_duration = delay + lifetime;

        runtime.prev_system_time = runtime.system_time;

        if fixed_fps > 0 {
            let fixed_delta = 1.0 / fixed_fps as f32;
            runtime.accumulated_delta += time.delta_secs();

            while runtime.accumulated_delta >= fixed_delta {
                runtime.accumulated_delta -= fixed_delta;
                runtime.system_time += fixed_delta;

                if runtime.system_time >= total_duration && total_duration > 0.0 {
                    runtime.system_time = runtime.system_time % total_duration;
                    runtime.cycle += 1;
                }
            }
        } else {
            runtime.system_time += time.delta_secs();

            if runtime.system_time >= total_duration && total_duration > 0.0 {
                runtime.system_time = runtime.system_time % total_duration;
                runtime.cycle += 1;
            }
        }

        if emitter_data.time.one_shot && runtime.cycle > 0 && !runtime.one_shot_completed {
            runtime.emitting = false;
            runtime.one_shot_completed = true;
        }
    }
}

// mesh generation

fn create_cylinder_mesh(
    top_radius: f32,
    bottom_radius: f32,
    height: f32,
    radial_segments: u32,
    rings: u32,
    cap_top: bool,
    cap_bottom: bool,
) -> Mesh {
    let radial_segments = radial_segments.max(3);
    let rings = rings.max(1);
    let half_height = height / 2.0;

    let mut positions: Vec<[f32; 3]> = Vec::new();
    let mut normals: Vec<[f32; 3]> = Vec::new();
    let mut uvs: Vec<[f32; 2]> = Vec::new();
    let mut indices: Vec<u32> = Vec::new();

    let side_normal_y = (bottom_radius - top_radius) / height;
    let side_normal_scale = 1.0 / (1.0 + side_normal_y * side_normal_y).sqrt();

    for ring in 0..=rings {
        let v = ring as f32 / rings as f32;
        let y = half_height - height * v;
        let radius = top_radius + (bottom_radius - top_radius) * v;

        for segment in 0..=radial_segments {
            let u = segment as f32 / radial_segments as f32;
            let theta = u * std::f32::consts::TAU;
            let (sin_theta, cos_theta) = theta.sin_cos();

            let x = cos_theta * radius;
            let z = sin_theta * radius;

            positions.push([x, y, z]);

            let nx = cos_theta * side_normal_scale;
            let ny = side_normal_y * side_normal_scale;
            let nz = sin_theta * side_normal_scale;
            normals.push([nx, ny, nz]);

            uvs.push([u, v]);
        }
    }

    let verts_per_ring = radial_segments + 1;
    for ring in 0..rings {
        for segment in 0..radial_segments {
            let top_left = ring * verts_per_ring + segment;
            let top_right = ring * verts_per_ring + segment + 1;
            let bottom_left = (ring + 1) * verts_per_ring + segment;
            let bottom_right = (ring + 1) * verts_per_ring + segment + 1;

            indices.push(top_left);
            indices.push(top_right);
            indices.push(bottom_left);

            indices.push(top_right);
            indices.push(bottom_right);
            indices.push(bottom_left);
        }
    }

    if cap_top && top_radius > 0.0 {
        let center_index = positions.len() as u32;

        positions.push([0.0, half_height, 0.0]);
        normals.push([0.0, 1.0, 0.0]);
        uvs.push([0.5, 0.5]);

        for segment in 0..=radial_segments {
            let u = segment as f32 / radial_segments as f32;
            let theta = u * std::f32::consts::TAU;
            let (sin_theta, cos_theta) = theta.sin_cos();

            let x = cos_theta * top_radius;
            let z = sin_theta * top_radius;

            positions.push([x, half_height, z]);
            normals.push([0.0, 1.0, 0.0]);
            uvs.push([cos_theta * 0.5 + 0.5, sin_theta * 0.5 + 0.5]);
        }

        for segment in 0..radial_segments {
            let first = center_index + 1 + segment;
            let second = center_index + 1 + segment + 1;
            indices.push(center_index);
            indices.push(second);
            indices.push(first);
        }
    }

    if cap_bottom && bottom_radius > 0.0 {
        let center_index = positions.len() as u32;

        positions.push([0.0, -half_height, 0.0]);
        normals.push([0.0, -1.0, 0.0]);
        uvs.push([0.5, 0.5]);

        for segment in 0..=radial_segments {
            let u = segment as f32 / radial_segments as f32;
            let theta = u * std::f32::consts::TAU;
            let (sin_theta, cos_theta) = theta.sin_cos();

            let x = cos_theta * bottom_radius;
            let z = sin_theta * bottom_radius;

            positions.push([x, -half_height, z]);
            normals.push([0.0, -1.0, 0.0]);
            uvs.push([cos_theta * 0.5 + 0.5, sin_theta * 0.5 + 0.5]);
        }

        for segment in 0..radial_segments {
            let first = center_index + 1 + segment;
            let second = center_index + 1 + segment + 1;
            indices.push(center_index);
            indices.push(first);
            indices.push(second);
        }
    }

    let mut mesh = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::default());
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    mesh.insert_indices(Indices::U32(indices));
    mesh
}

fn create_base_mesh(config: &ParticleMesh) -> Mesh {
    match config {
        ParticleMesh::Quad { orientation } => {
            let mut mesh = Mesh::from(Rectangle::new(1.0, 1.0));

            let rotation = match orientation {
                QuadOrientation::FaceZ => None,
                QuadOrientation::FaceX => Some(Quat::from_rotation_y(std::f32::consts::FRAC_PI_2)),
                QuadOrientation::FaceY => Some(Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2)),
            };

            if let Some(rot) = rotation {
                if let Some(VertexAttributeValues::Float32x3(positions)) =
                    mesh.attribute_mut(Mesh::ATTRIBUTE_POSITION)
                {
                    for pos in positions.iter_mut() {
                        let v = rot * Vec3::from_array(*pos);
                        *pos = v.to_array();
                    }
                }
                if let Some(VertexAttributeValues::Float32x3(normals)) =
                    mesh.attribute_mut(Mesh::ATTRIBUTE_NORMAL)
                {
                    for normal in normals.iter_mut() {
                        let v = rot * Vec3::from_array(*normal);
                        *normal = v.to_array();
                    }
                }
            }

            mesh
        }
        ParticleMesh::Sphere { radius } => Mesh::from(Sphere::new(*radius)),
        ParticleMesh::Cuboid { half_size } => {
            Mesh::from(Cuboid::new(half_size.x * 2.0, half_size.y * 2.0, half_size.z * 2.0))
        }
        ParticleMesh::Cylinder {
            top_radius,
            bottom_radius,
            height,
            radial_segments,
            rings,
            cap_top,
            cap_bottom,
        } => create_cylinder_mesh(
            *top_radius,
            *bottom_radius,
            *height,
            *radial_segments,
            *rings,
            *cap_top,
            *cap_bottom,
        ),
    }
}

fn create_particle_mesh(
    config: &ParticleMesh,
    particle_count: u32,
    meshes: &mut Assets<Mesh>,
) -> Handle<Mesh> {
    let base_mesh = create_base_mesh(config);

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

        for i in 0..vertices_per_mesh {
            positions.push(base_positions[i]);
            normals.push(base_normals[i]);
            uvs.push(base_uvs[i]);
            uv_bs.push([particle_index_f32, 0.0]);
        }

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

// material creation

fn create_particle_material_from_config(
    config: &DrawPassMaterial,
    sorted_particles_buffer: Handle<ShaderStorageBuffer>,
    max_particles: u32,
    particle_flags: u32,
    asset_server: &AssetServer,
) -> ParticleMaterial {
    let base = match config {
        DrawPassMaterial::Standard(mat) => mat.to_standard_material(asset_server),
        DrawPassMaterial::CustomShader { .. } => {
            todo!("custom shader support not yet implemented")
        }
    };

    ExtendedMaterial {
        base,
        extension: ParticleMaterialExtension {
            sorted_particles: sorted_particles_buffer,
            max_particles,
            particle_flags,
        },
    }
}

// spawning systems

pub fn setup_particle_systems(
    mut commands: Commands,
    query: Query<(Entity, &ParticleSystem3D), Without<ParticleSystemRuntime>>,
    assets: Res<Assets<ParticleSystemAsset>>,
    asset_server: Res<AssetServer>,
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

        commands.entity(system_entity).insert((
            ParticleSystemRuntime::default(),
            Transform::default(),
            Visibility::default(),
        ));

        for (emitter_index, emitter) in asset.emitters.iter().enumerate() {
            let amount = emitter.amount;

            let particles: Vec<ParticleData> =
                (0..amount).map(|_| ParticleData::default()).collect();

            let particle_buffer_handle = buffers.add(ShaderStorageBuffer::from(particles.clone()));

            let indices: Vec<u32> = (0..amount).collect();
            let indices_buffer_handle = buffers.add(ShaderStorageBuffer::from(indices));

            let sorted_particles_buffer_handle = buffers.add(ShaderStorageBuffer::from(particles));

            let (current_mesh, current_material) = if let Some(draw_pass) =
                emitter.draw_passes.first()
            {
                (draw_pass.mesh.clone(), draw_pass.material.clone())
            } else {
                (ParticleMesh::default(), DrawPassMaterial::default())
            };

            let particle_mesh_handle = create_particle_mesh(&current_mesh, amount, &mut meshes);

            let material_handle = materials.add(create_particle_material_from_config(
                &current_material,
                sorted_particles_buffer_handle.clone(),
                amount,
                emitter.process.particle_flags.bits(),
                &asset_server,
            ));

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
                    CurrentMaterialConfig(current_material),
                    ParticleMeshHandle(particle_mesh_handle.clone()),
                    ParticleMaterialHandle(material_handle.clone()),
                    Transform::default(),
                    Visibility::default(),
                ))
                .id();

            commands.entity(system_entity).add_child(emitter_entity);

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

// small offset per emitter index to ensure consistent depth sorting for overlapping emitters
const EMITTER_DEPTH_OFFSET: f32 = 0.0001;

pub fn sync_emitter_mesh_transforms(
    camera_query: Query<&GlobalTransform, With<Camera3d>>,
    emitter_query: Query<(&GlobalTransform, &EmitterRuntime), With<EmitterEntity>>,
    mut mesh_query: Query<(&EmitterMeshEntity, &mut Transform)>,
) {
    let camera_forward = camera_query
        .iter()
        .next()
        .map(|t| t.forward().as_vec3())
        .unwrap_or(Vec3::NEG_Z);

    for (emitter_mesh, mut mesh_transform) in mesh_query.iter_mut() {
        if let Ok((emitter_global, runtime)) = emitter_query.get(emitter_mesh.emitter_entity) {
            // offset along camera forward based on emitter index for consistent ordering
            let depth_offset = camera_forward * (runtime.emitter_index as f32 * EMITTER_DEPTH_OFFSET);
            mesh_transform.translation = emitter_global.translation() + depth_offset;
        }
    }
}

pub fn cleanup_particle_entities(
    mut commands: Commands,
    mut removed_systems: RemovedComponents<ParticleSystem3D>,
    mut removed_emitters: RemovedComponents<EmitterEntity>,
    mesh_entities: Query<(Entity, &EmitterMeshEntity)>,
    emitter_entities: Query<Entity, With<EmitterEntity>>,
    emitter_parent_query: Query<&EmitterEntity>,
) {
    for removed_system in removed_systems.read() {
        for emitter_entity in emitter_entities.iter() {
            if let Ok(emitter) = emitter_parent_query.get(emitter_entity) {
                if emitter.parent_system == removed_system {
                    commands.entity(emitter_entity).despawn();
                }
            }
        }

        for (mesh_entity, emitter_mesh) in mesh_entities.iter() {
            if let Ok(emitter) = emitter_parent_query.get(emitter_mesh.emitter_entity) {
                if emitter.parent_system == removed_system {
                    commands.entity(mesh_entity).despawn();
                }
            }
        }
    }

    for removed_emitter in removed_emitters.read() {
        for (mesh_entity, emitter_mesh) in mesh_entities.iter() {
            if emitter_mesh.emitter_entity == removed_emitter {
                commands.entity(mesh_entity).despawn();
            }
        }
    }
}

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
            ParticleMesh::default()
        };

        if current_config.0 != new_mesh {
            let new_mesh_handle =
                create_particle_mesh(&new_mesh, buffer_handle.max_particles, &mut meshes);

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

pub fn sync_particle_material(
    particle_systems: Query<&ParticleSystem3D>,
    mut emitter_query: Query<(
        Entity,
        &EmitterEntity,
        &EmitterRuntime,
        &ParticleBufferHandle,
        &mut CurrentMaterialConfig,
        &mut ParticleMaterialHandle,
    )>,
    mut mesh_entities: Query<(&EmitterMeshEntity, &mut MeshMaterial3d<ParticleMaterial>)>,
    assets: Res<Assets<ParticleSystemAsset>>,
    asset_server: Res<AssetServer>,
    mut materials: ResMut<Assets<ParticleMaterial>>,
) {
    for (
        emitter_entity,
        emitter,
        runtime,
        buffer_handle,
        mut current_config,
        mut material_handle,
    ) in emitter_query.iter_mut()
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

        let new_material = if let Some(draw_pass) = emitter_data.draw_passes.first() {
            draw_pass.material.clone()
        } else {
            DrawPassMaterial::default()
        };

        if current_config.0.cache_key() != new_material.cache_key() {
            let sorted_particles_handle = {
                let Some(existing_material) = materials.get(&material_handle.0) else {
                    continue;
                };
                existing_material.extension.sorted_particles.clone()
            };

            let new_material_handle = materials.add(create_particle_material_from_config(
                &new_material,
                sorted_particles_handle,
                buffer_handle.max_particles,
                emitter_data.process.particle_flags.bits(),
                &asset_server,
            ));

            for (emitter_mesh, mut material3d) in mesh_entities.iter_mut() {
                if emitter_mesh.emitter_entity == emitter_entity {
                    material3d.0 = new_material_handle.clone();
                }
            }

            current_config.0 = new_material;
            material_handle.0 = new_material_handle;
        }
    }
}
