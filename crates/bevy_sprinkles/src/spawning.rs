use bevy::{
    light::NotShadowCaster,
    pbr::ExtendedMaterial,
    prelude::*,
    render::storage::ShaderStorageBuffer,
};

use crate::{
    asset::{DrawPassMaterial, EmitterData, ParticleSystemAsset},
    material::ParticleMaterialExtension,
    mesh::create_particle_mesh,
    runtime::{
        ColliderEntity, CurrentMaterialConfig, CurrentMeshConfig, EmitterEntity, EmitterMeshEntity,
        EmitterRuntime, ParticleBufferHandle, ParticleData, ParticleMaterial,
        ParticleMaterialHandle, ParticleMeshHandle, ParticleSystem3D, ParticleSystemRuntime,
        ParticlesCollider3D, SimulationStep, SubEmitterBufferHandle,
    },
};

const MAX_FRAME_DELTA: f32 = 0.1;

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

        let Some(asset) = assets.get(&particle_system.handle) else {
            continue;
        };

        let Some(emitter_data) = asset.emitters.get(runtime.emitter_index) else {
            continue;
        };

        runtime.simulation_steps.clear();

        let clear_requested = runtime.clear_requested;
        runtime.clear_requested = false;

        if system_runtime.paused {
            if clear_requested {
                let step = SimulationStep {
                    prev_system_time: runtime.system_time,
                    system_time: runtime.system_time,
                    cycle: runtime.cycle,
                    delta_time: 0.0,
                    clear_requested: true,
                };
                runtime.simulation_steps.push(step);
            }
            continue;
        }

        let fixed_fps = emitter_data.time.fixed_fps;
        let total_duration = emitter_data.time.total_duration();

        if fixed_fps > 0 {
            let fixed_delta = 1.0 / fixed_fps as f32;
            let frame_delta = time.delta_secs().min(MAX_FRAME_DELTA);
            runtime.accumulated_delta += frame_delta;

            while runtime.accumulated_delta >= fixed_delta
                || (clear_requested && runtime.simulation_steps.is_empty())
            {
                runtime.accumulated_delta -= fixed_delta;

                let prev_time = runtime.system_time;
                runtime.system_time += fixed_delta;

                if runtime.system_time >= total_duration && total_duration > 0.0 {
                    runtime.system_time = runtime.system_time % total_duration;
                    runtime.cycle += 1;
                }

                let step = SimulationStep {
                    prev_system_time: prev_time,
                    system_time: runtime.system_time,
                    cycle: runtime.cycle,
                    delta_time: fixed_delta,
                    clear_requested: if runtime.simulation_steps.is_empty() {
                        clear_requested
                    } else {
                        false
                    },
                };
                runtime.simulation_steps.push(step);
            }

            if !runtime.simulation_steps.is_empty() {
                runtime.prev_system_time = runtime.simulation_steps[0].prev_system_time;
            }
        } else {
            let delta = time.delta_secs();
            let prev_time = runtime.system_time;
            runtime.prev_system_time = runtime.system_time;
            runtime.system_time += delta;

            if runtime.system_time >= total_duration && total_duration > 0.0 {
                runtime.system_time = runtime.system_time % total_duration;
                runtime.cycle += 1;
            }

            let step = SimulationStep {
                prev_system_time: prev_time,
                system_time: runtime.system_time,
                cycle: runtime.cycle,
                delta_time: delta,
                clear_requested,
            };
            runtime.simulation_steps.push(step);
        }

        if emitter_data.time.one_shot && runtime.cycle > 0 && !runtime.one_shot_completed {
            runtime.emitting = false;
            runtime.one_shot_completed = true;
        }
    }
}

fn combined_particle_flags(emitter: &EmitterData) -> u32 {
    use crate::asset::TransformAlign;
    let mut flags = emitter.particle_flags.bits();
    let transform_align_bits = match emitter.draw_pass.transform_align {
        None => 0u32,
        Some(TransformAlign::Billboard) => 1,
        Some(TransformAlign::YToVelocity) => 2,
        Some(TransformAlign::BillboardYToVelocity) => 3,
        Some(TransformAlign::BillboardFixedY) => 4,
    };
    flags |= transform_align_bits << 3;
    flags
}

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

        let mut emitter_entities: Vec<Entity> = Vec::new();

        for (emitter_index, emitter) in asset.emitters.iter().enumerate() {
            let amount = emitter.emission.particles_amount;

            let particles: Vec<ParticleData> =
                (0..amount).map(|_| ParticleData::default()).collect();

            let particle_buffer_handle = buffers.add(ShaderStorageBuffer::from(particles.clone()));

            let indices: Vec<u32> = (0..amount).collect();
            let indices_buffer_handle = buffers.add(ShaderStorageBuffer::from(indices));

            let sorted_particles_buffer_handle = buffers.add(ShaderStorageBuffer::from(particles));

            let current_mesh = emitter.draw_pass.mesh.clone();
            let current_material = emitter.draw_pass.material.clone();
            let shadow_caster = emitter.draw_pass.shadow_caster;

            let particle_mesh_handle = create_particle_mesh(&current_mesh, amount, &mut meshes);

            let material_handle = materials.add(create_particle_material_from_config(
                &current_material,
                sorted_particles_buffer_handle.clone(),
                amount,
                combined_particle_flags(emitter),
                &asset_server,
            ));

            let emitter_entity = commands
                .spawn((
                    EmitterEntity {
                        parent_system: system_entity,
                    },
                    EmitterRuntime::new(emitter_index, emitter.time.fixed_seed),
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
                    Transform::from_translation(emitter.position),
                    Visibility::default(),
                ))
                .id();

            emitter_entities.push(emitter_entity);
            commands.entity(system_entity).add_child(emitter_entity);

            let mut mesh_entity = commands.spawn((
                Mesh3d(particle_mesh_handle),
                MeshMaterial3d(material_handle),
                Transform::default(),
                Visibility::default(),
                EmitterMeshEntity { emitter_entity },
            ));

            if !shadow_caster {
                mesh_entity.insert(NotShadowCaster);
            }
        }

        for (emitter_index, emitter) in asset.emitters.iter().enumerate() {
            if let Some(ref sub_config) = emitter.sub_emitter {
                let target_index = sub_config.target_emitter;
                if target_index == emitter_index || target_index >= asset.emitters.len() {
                    continue;
                }

                let target_amount = asset.emitters[target_index].emission.particles_amount;
                let buffer_len = 4 + 12 * target_amount as usize;
                let mut initial_data = vec![0u32; buffer_len];
                initial_data[1] = target_amount;
                let mut buffer = ShaderStorageBuffer::from(initial_data);
                buffer.buffer_description.usage |=
                    bevy::render::render_resource::BufferUsages::COPY_DST;

                let buffer_handle = buffers.add(buffer);
                let target_entity = emitter_entities[target_index];
                let parent_entity = emitter_entities[emitter_index];

                commands
                    .entity(parent_entity)
                    .insert(SubEmitterBufferHandle {
                        buffer: buffer_handle,
                        target_emitter: target_entity,
                        max_particles: target_amount,
                    });
            }
        }

        for (collider_index, collider_data) in asset.colliders.iter().enumerate() {
            let collider_entity = commands
                .spawn((
                    ColliderEntity {
                        parent_system: system_entity,
                        collider_index,
                    },
                    ParticlesCollider3D {
                        enabled: collider_data.enabled,
                        shape: collider_data.shape.clone(),
                        position: Vec3::ZERO,
                    },
                    Transform::from_translation(collider_data.position),
                    Name::new(collider_data.name.clone()),
                ))
                .id();

            commands.entity(system_entity).add_child(collider_entity);
        }
    }
}

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
            let depth_offset =
                camera_forward * (runtime.emitter_index as f32 * EMITTER_DEPTH_OFFSET);
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
    collider_entities: Query<(Entity, &ColliderEntity)>,
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

        for (entity, collider) in collider_entities.iter() {
            if collider.parent_system == removed_system {
                commands.entity(entity).despawn();
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

pub fn sync_collider_data(
    particle_systems: Query<&ParticleSystem3D>,
    assets: Res<Assets<ParticleSystemAsset>>,
    mut collider_query: Query<(&ColliderEntity, &mut ParticlesCollider3D, &mut Transform)>,
) {
    if !assets.is_changed() {
        return;
    }

    for (collider, mut collider3d, mut transform) in collider_query.iter_mut() {
        let Ok(particle_system) = particle_systems.get(collider.parent_system) else {
            continue;
        };
        let Some(asset) = assets.get(&particle_system.handle) else {
            continue;
        };
        let Some(collider_data) = asset.colliders.get(collider.collider_index) else {
            continue;
        };

        collider3d.enabled = collider_data.enabled;
        collider3d.shape = collider_data.shape.clone();
        transform.translation = collider_data.position;
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

        let new_mesh = emitter_data.draw_pass.mesh.clone();

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

        let new_material = emitter_data.draw_pass.material.clone();

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
                combined_particle_flags(emitter_data),
                &asset_server,
            ));

            for (emitter_mesh, mut material3d) in mesh_entities.iter_mut() {
                if emitter_mesh.emitter_entity == emitter_entity {
                    material3d.0 = new_material_handle.clone();
                }
            }

            current_config.0 = new_material;
            material_handle.0 = new_material_handle;
        } else {
            let new_flags = combined_particle_flags(emitter_data);
            if let Some(material) = materials.get_mut(&material_handle.0) {
                if material.extension.particle_flags != new_flags {
                    material.extension.particle_flags = new_flags;
                }
            }
        }
    }
}
