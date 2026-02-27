use bevy::{
    light::NotShadowCaster, pbr::ExtendedMaterial, prelude::*, render::storage::ShaderStorageBuffer,
};

use crate::{
    asset::{DrawPassMaterial, EmitterData, ParticleSystemAsset},
    material::{ParticleEmitterUniforms, ParticleMaterialExtension},
    mesh::ParticleMeshCache,
    runtime::{
        ColliderEntity, CurrentMaterialConfig, CurrentMeshConfig, EmitterEntity, EmitterRuntime,
        ParticleBufferHandle, ParticleData, ParticleMaterial, ParticleMaterialHandle,
        ParticleMeshHandle, ParticleSystem3D, ParticleSystemRuntime, ParticlesCollider3D,
        SimulationStep, SubEmitterBufferHandle,
    },
};

const MAX_FRAME_DELTA: f32 = 0.1;
const INACTIVE_GRACE_FACTOR: f32 = 1.2;

fn get_particle_asset<'a>(
    parent_system: Entity,
    particle_systems: &Query<&ParticleSystem3D>,
    assets: &'a Assets<ParticleSystemAsset>,
) -> Option<&'a ParticleSystemAsset> {
    let particle_system = particle_systems.get(parent_system).ok()?;
    assets.get(&particle_system.handle)
}

fn get_emitter_data<'a>(
    parent_system: Entity,
    emitter_index: usize,
    particle_systems: &Query<&ParticleSystem3D>,
    assets: &'a Assets<ParticleSystemAsset>,
) -> Option<&'a EmitterData> {
    get_particle_asset(parent_system, particle_systems, assets)
        .and_then(|asset| asset.emitters.get(emitter_index))
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

        let Some(asset) = assets.get(&particle_system.handle) else {
            continue;
        };

        let Some(emitter_data) = asset.emitters.get(runtime.emitter_index) else {
            continue;
        };

        runtime.simulation_steps.clear();

        let clear_requested = runtime.clear_requested;
        runtime.clear_requested = false;

        if runtime.inactive || system_runtime.paused {
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
            runtime.set_emitting(false);
            runtime.one_shot_completed = true;
        }

        if !runtime.emitting {
            runtime.inactive_time += time.delta_secs();
            let grace = emitter_data.time.lifetime * INACTIVE_GRACE_FACTOR;
            if runtime.inactive_time > grace {
                runtime.inactive = true;
            }
        } else {
            runtime.inactive_time = 0.0;
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
    emitter_uniforms_buffer: Handle<ShaderStorageBuffer>,
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
            emitter_uniforms: emitter_uniforms_buffer,
        },
    }
}

pub fn setup_particle_systems(
    mut commands: Commands,
    query: Query<(Entity, &ParticleSystem3D), Without<ParticleSystemRuntime>>,
    assets: Res<Assets<ParticleSystemAsset>>,
    asset_server: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut mesh_cache: ResMut<ParticleMeshCache>,
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

        commands
            .entity(system_entity)
            .insert(ParticleSystemRuntime::default())
            .insert_if_new((
                asset.initial_transform.to_transform(),
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

            let emitter_uniforms = ParticleEmitterUniforms {
                emitter_transform: Mat4::IDENTITY,
                max_particles: amount,
                particle_flags: combined_particle_flags(emitter),
                ..default()
            };
            let mut emitter_uniforms_ssbo = ShaderStorageBuffer::default();
            emitter_uniforms_ssbo.set_data(emitter_uniforms);
            let emitter_uniforms_buffer_handle = buffers.add(emitter_uniforms_ssbo);

            let current_mesh = emitter.draw_pass.mesh.clone();
            let current_material = emitter.draw_pass.material.clone();
            let shadow_caster = emitter.draw_pass.shadow_caster;

            let particle_mesh_handle = mesh_cache.get_or_create(&current_mesh, amount, &mut meshes);

            let material_handle = materials.add(create_particle_material_from_config(
                &current_material,
                sorted_particles_buffer_handle.clone(),
                emitter_uniforms_buffer_handle.clone(),
                &asset_server,
            ));

            let mut emitter_cmds = commands.spawn((
                EmitterEntity {
                    parent_system: system_entity,
                },
                EmitterRuntime::new(emitter_index, emitter.time.fixed_seed),
                ParticleBufferHandle {
                    particle_buffer: particle_buffer_handle.clone(),
                    indices_buffer: indices_buffer_handle.clone(),
                    sorted_particles_buffer: sorted_particles_buffer_handle.clone(),
                    emitter_uniforms_buffer: emitter_uniforms_buffer_handle,
                    max_particles: amount,
                },
                Mesh3d(particle_mesh_handle.clone()),
                MeshMaterial3d(material_handle.clone()),
                CurrentMeshConfig(current_mesh),
                CurrentMaterialConfig(current_material),
                ParticleMeshHandle(particle_mesh_handle),
                ParticleMaterialHandle(material_handle),
                emitter.initial_transform.to_transform(),
                Visibility::default(),
            ));

            if !shadow_caster {
                emitter_cmds.insert(NotShadowCaster);
            }

            let emitter_entity = emitter_cmds.id();

            emitter_entities.push(emitter_entity);
            commands.entity(system_entity).add_child(emitter_entity);
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
                    },
                    collider_data.initial_transform.to_transform(),
                    Name::new(collider_data.name.clone()),
                ))
                .id();

            commands.entity(system_entity).add_child(collider_entity);
        }
    }
}

pub fn cleanup_particle_entities(
    mut commands: Commands,
    mut removed_systems: RemovedComponents<ParticleSystem3D>,
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

        for (entity, collider) in collider_entities.iter() {
            if collider.parent_system == removed_system {
                commands.entity(entity).despawn();
            }
        }
    }
}

pub fn sync_collider_data(
    particle_systems: Query<&ParticleSystem3D>,
    assets: Res<Assets<ParticleSystemAsset>>,
    mut collider_query: Query<(&ColliderEntity, &mut ParticlesCollider3D)>,
) {
    if !assets.is_changed() {
        return;
    }

    for (collider, mut collider3d) in collider_query.iter_mut() {
        let Some(collider_data) =
            get_particle_asset(collider.parent_system, &particle_systems, &assets)
                .and_then(|asset| asset.colliders.get(collider.collider_index))
        else {
            continue;
        };

        collider3d.enabled = collider_data.enabled;
        collider3d.shape = collider_data.shape.clone();
    }
}

pub fn sync_particle_mesh(
    particle_systems: Query<&ParticleSystem3D>,
    mut emitter_query: Query<(
        &EmitterEntity,
        &EmitterRuntime,
        &ParticleBufferHandle,
        &mut CurrentMeshConfig,
        &mut ParticleMeshHandle,
        &mut Mesh3d,
    )>,
    assets: Res<Assets<ParticleSystemAsset>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut mesh_cache: ResMut<ParticleMeshCache>,
) {
    for (emitter, runtime, buffer_handle, mut current_config, mut mesh_handle, mut mesh3d) in
        emitter_query.iter_mut()
    {
        let Some(emitter_data) = get_emitter_data(
            emitter.parent_system,
            runtime.emitter_index,
            &particle_systems,
            &assets,
        ) else {
            continue;
        };

        let new_mesh = emitter_data.draw_pass.mesh.clone();

        if current_config.0 != new_mesh {
            let new_mesh_handle =
                mesh_cache.get_or_create(&new_mesh, buffer_handle.max_particles, &mut meshes);
            mesh3d.0 = new_mesh_handle.clone();
            current_config.0 = new_mesh;
            mesh_handle.0 = new_mesh_handle;
        }
    }
}

pub fn write_emitter_uniforms(
    particle_systems: Query<&ParticleSystem3D>,
    emitter_query: Query<(
        &EmitterEntity,
        &EmitterRuntime,
        &ParticleBufferHandle,
        &GlobalTransform,
    )>,
    assets: Res<Assets<ParticleSystemAsset>>,
    mut buffers: ResMut<Assets<ShaderStorageBuffer>>,
) {
    for (emitter, runtime, buffer_handle, global_transform) in emitter_query.iter() {
        let Some(emitter_data) = get_emitter_data(
            emitter.parent_system,
            runtime.emitter_index,
            &particle_systems,
            &assets,
        ) else {
            continue;
        };

        let uniforms = ParticleEmitterUniforms {
            emitter_transform: global_transform.to_matrix(),
            max_particles: buffer_handle.max_particles,
            particle_flags: combined_particle_flags(emitter_data),
            use_local_coords: emitter_data.draw_pass.use_local_coords as u32,
            ..default()
        };

        if let Some(buffer) = buffers.get_mut(&buffer_handle.emitter_uniforms_buffer) {
            buffer.set_data(uniforms);
        }
    }
}

pub fn sync_particle_material(
    particle_systems: Query<&ParticleSystem3D>,
    mut emitter_query: Query<(
        &EmitterEntity,
        &EmitterRuntime,
        &mut CurrentMaterialConfig,
        &mut ParticleMaterialHandle,
        &mut MeshMaterial3d<ParticleMaterial>,
    )>,
    assets: Res<Assets<ParticleSystemAsset>>,
    asset_server: Res<AssetServer>,
    mut materials: ResMut<Assets<ParticleMaterial>>,
) {
    for (emitter, runtime, mut current_config, mut material_handle, mut material3d) in
        emitter_query.iter_mut()
    {
        let Some(emitter_data) = get_emitter_data(
            emitter.parent_system,
            runtime.emitter_index,
            &particle_systems,
            &assets,
        ) else {
            continue;
        };

        let new_material = emitter_data.draw_pass.material.clone();

        if current_config.0.cache_key() != new_material.cache_key() {
            let (sorted_particles_handle, emitter_uniforms_handle) = {
                let Some(existing_material) = materials.get(&material_handle.0) else {
                    continue;
                };
                (
                    existing_material.extension.sorted_particles.clone(),
                    existing_material.extension.emitter_uniforms.clone(),
                )
            };

            let new_material_handle = materials.add(create_particle_material_from_config(
                &new_material,
                sorted_particles_handle,
                emitter_uniforms_handle,
                &asset_server,
            ));

            material3d.0 = new_material_handle.clone();
            current_config.0 = new_material;
            material_handle.0 = new_material_handle;
        }
    }
}
