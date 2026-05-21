use bevy::{
    light::NotShadowCaster, pbr::ExtendedMaterial, prelude::*, render::storage::ShaderStorageBuffer,
};

use crate::{
    asset::{DrawPassMaterial, EmitterData, EmitterTrail, ParticlesAsset},
    material::{ParticleEmitterUniforms, ParticleMaterialExtension, TRAIL_THICKNESS_CURVE_SAMPLES},
    mesh::ParticleMeshCache,
    runtime::{
        ColliderEntity, CurrentMaterialConfig, CurrentMeshConfig, EditorMode, EmitterEntity,
        EmitterRuntime, ParticleBufferHandle, ParticleData, ParticleMaterial,
        ParticleMaterialHandle, ParticleMeshHandle, ParticleSystemRuntime, Particles3d,
        ParticlesCollider3D, SimulationStep, SubEmitterBufferHandle, TrailHistoryEntry,
    },
};

const MAX_FRAME_DELTA: f32 = 0.1;
const INACTIVE_GRACE_FACTOR: f32 = 1.2;
const MAX_TRAIL_HISTORY_FPS: f32 = 240.0;
fn create_trail_history_buffer(
    amount: u32,
    frames: u32,
    buffers: &mut Assets<ShaderStorageBuffer>,
) -> Option<Handle<ShaderStorageBuffer>> {
    if frames > 0 {
        let data = vec![TrailHistoryEntry::default(); (amount * frames) as usize];
        Some(buffers.add(ShaderStorageBuffer::from(data)))
    } else {
        None
    }
}

fn compute_trail_history_frames(emitter: &EmitterData) -> u32 {
    let trail_size = emitter.trail_size();
    if trail_size <= 1 {
        return 0;
    }
    let effective_fps = if emitter.time.fixed_fps > 0 {
        emitter.time.fixed_fps as f32
    } else {
        MAX_TRAIL_HISTORY_FPS
    };
    let from_stretch = (emitter.trail.stretch_time * effective_fps).ceil() as u32;
    trail_size.max(from_stretch).max(2)
}

fn get_particle_asset<'a>(
    parent_system: Entity,
    particle_systems: &Query<&Particles3d>,
    assets: &'a Assets<ParticlesAsset>,
) -> Option<&'a ParticlesAsset> {
    let particle_system = particle_systems.get(parent_system).ok()?;
    assets.get(particle_system)
}

fn get_emitter_data<'a>(
    parent_system: Entity,
    emitter_index: usize,
    particle_systems: &Query<&Particles3d>,
    assets: &'a Assets<ParticlesAsset>,
) -> Option<&'a EmitterData> {
    get_particle_asset(parent_system, particle_systems, assets)
        .and_then(|asset| asset.emitters.get(emitter_index))
}

fn get_editor_assets_folders<'a>(
    parent_system: Entity,
    is_editor: bool,
    particle_systems: &Query<&Particles3d>,
    assets: &'a Assets<ParticlesAsset>,
) -> &'a [String] {
    if !is_editor {
        return &[];
    }
    get_particle_asset(parent_system, particle_systems, assets)
        .map(|a| a.sprinkles_editor.assets_folder.as_slice())
        .unwrap_or(&[])
}

pub fn update_particle_time(
    time: Res<Time>,
    assets: Res<Assets<ParticlesAsset>>,
    system_query: Query<(&Particles3d, &ParticleSystemRuntime)>,
    mut emitter_query: Query<(&EmitterEntity, &mut EmitterRuntime)>,
) {
    for (emitter, mut runtime) in emitter_query.iter_mut() {
        let Ok((particle_system, system_runtime)) = system_query.get(emitter.parent_system) else {
            continue;
        };

        let Some(asset) = assets.get(particle_system) else {
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
                    trail_history_write_index: runtime.trail_history_write_index,
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
                    trail_history_write_index: runtime.trail_history_write_index,
                };
                runtime.advance_trail_history();
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
                trail_history_write_index: runtime.trail_history_write_index,
            };
            runtime.advance_trail_history();
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

fn transform_align_to_u32(align: Option<crate::asset::TransformAlign>) -> u32 {
    use crate::asset::TransformAlign;
    match align {
        None => 0,
        Some(TransformAlign::Billboard) => 1,
        Some(TransformAlign::YToVelocity) => 2,
        Some(TransformAlign::BillboardYToVelocity) => 3,
        Some(TransformAlign::BillboardFixedY) => 4,
    }
}

fn create_particle_material_from_config(
    config: &DrawPassMaterial,
    sorted_particles_buffer: Handle<ShaderStorageBuffer>,
    emitter_uniforms_buffer: Handle<ShaderStorageBuffer>,
    asset_server: &AssetServer,
    assets_folders: &[String],
) -> ParticleMaterial {
    let base = match config {
        DrawPassMaterial::Standard(mat) => mat.to_standard_material(asset_server, assets_folders),
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

fn bake_thickness_curve(trail: &EmitterTrail) -> [f32; TRAIL_THICKNESS_CURVE_SAMPLES] {
    let mut samples = [1.0f32; TRAIL_THICKNESS_CURVE_SAMPLES];
    if let Some(ref curve) = trail.thickness_curve {
        for (i, sample) in samples.iter_mut().enumerate() {
            let t = i as f32 / (TRAIL_THICKNESS_CURVE_SAMPLES - 1) as f32;
            *sample = curve.sample(t);
        }
    }
    samples
}

pub fn setup_particle_systems(
    mut commands: Commands,
    query: Query<(Entity, &Particles3d, Has<EditorMode>), Without<ParticleSystemRuntime>>,
    assets: Res<Assets<ParticlesAsset>>,
    asset_server: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut mesh_cache: ResMut<ParticleMeshCache>,
    mut buffers: ResMut<Assets<ShaderStorageBuffer>>,
    mut materials: ResMut<Assets<ParticleMaterial>>,
) {
    for (system_entity, particle_system, is_editor) in query.iter() {
        let Some(asset) = assets.get(particle_system) else {
            continue;
        };

        if asset.emitters.is_empty() {
            continue;
        }

        let assets_folders = if is_editor {
            asset.sprinkles_editor.assets_folder.as_slice()
        } else {
            &[]
        };

        let initial_transform = asset.initial_transform.to_transform();

        commands
            .entity(system_entity)
            .queue_silenced(move |mut entity: EntityWorldMut| {
                entity
                    .insert(ParticleSystemRuntime::default())
                    .insert_if_new((initial_transform, Visibility::default()));
            });

        let mut emitter_entities: Vec<Entity> = Vec::new();

        for (emitter_index, emitter) in asset.emitters.iter().enumerate() {
            let amount = emitter.emission.particles_amount;
            let trail_size = emitter.trail_size();
            let total_slots = amount * trail_size;

            let particles: Vec<ParticleData> =
                (0..total_slots).map(|_| ParticleData::default()).collect();

            let particle_buffer_handle = buffers.add(ShaderStorageBuffer::from(particles.clone()));

            let indices: Vec<u32> = (0..total_slots).collect();
            let indices_buffer_handle = buffers.add(ShaderStorageBuffer::from(indices));

            let sorted_particles_buffer_handle = buffers.add(ShaderStorageBuffer::from(particles));

            let trail_history_frames = compute_trail_history_frames(emitter);
            let trail_history_buffer =
                create_trail_history_buffer(amount, trail_history_frames, &mut buffers);

            let emitter_uniforms = ParticleEmitterUniforms {
                emitter_transform: Mat4::IDENTITY,
                max_particles: total_slots,
                particle_flags: emitter.particle_flags.bits(),
                trail_size,
                transform_align: transform_align_to_u32(emitter.draw_pass.transform_align),
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
                assets_folders,
            ));

            let mut runtime = EmitterRuntime::new(emitter_index, emitter.time.fixed_seed);
            runtime.trail_history_frames = trail_history_frames;

            let mut emitter_cmds = commands.spawn((
                EmitterEntity {
                    parent_system: system_entity,
                },
                runtime,
                ParticleBufferHandle {
                    particle_buffer: particle_buffer_handle.clone(),
                    indices_buffer: indices_buffer_handle.clone(),
                    sorted_particles_buffer: sorted_particles_buffer_handle.clone(),
                    emitter_uniforms_buffer: emitter_uniforms_buffer_handle,
                    max_particles: total_slots,
                    amount,
                    trail_size,
                    trail_history_buffer,
                    trail_history_frames,
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
            commands
                .entity(system_entity)
                .queue_silenced(move |mut entity: EntityWorldMut| {
                    entity.add_child(emitter_entity);
                });
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

            commands
                .entity(system_entity)
                .queue_silenced(move |mut entity: EntityWorldMut| {
                    entity.add_child(collider_entity);
                });
        }
    }
}

pub fn cleanup_particle_entities(
    mut commands: Commands,
    mut removed_systems: RemovedComponents<Particles3d>,
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
    particle_systems: Query<&Particles3d>,
    assets: Res<Assets<ParticlesAsset>>,
    mut collider_query: Query<(&ColliderEntity, &mut ParticlesCollider3D, &mut Transform)>,
) {
    if !assets.is_changed() {
        return;
    }

    for (collider, mut collider3d, mut transform) in collider_query.iter_mut() {
        let Some(collider_data) =
            get_particle_asset(collider.parent_system, &particle_systems, &assets)
                .and_then(|asset| asset.colliders.get(collider.collider_index))
        else {
            continue;
        };

        collider3d.enabled = collider_data.enabled;
        collider3d.shape = collider_data.shape.clone();
        *transform = collider_data.initial_transform.to_transform();
    }
}

pub fn sync_particle_mesh(
    particle_systems: Query<&Particles3d>,
    mut emitter_query: Query<(
        &EmitterEntity,
        &EmitterRuntime,
        &ParticleBufferHandle,
        &mut CurrentMeshConfig,
        &mut ParticleMeshHandle,
        &mut Mesh3d,
    )>,
    assets: Res<Assets<ParticlesAsset>>,
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
                mesh_cache.get_or_create(&new_mesh, buffer_handle.amount, &mut meshes);
            mesh3d.0 = new_mesh_handle.clone();
            current_config.0 = new_mesh;
            mesh_handle.0 = new_mesh_handle;
        }
    }
}

pub(crate) fn sync_particle_buffers(
    particle_systems: Query<&Particles3d>,
    editor_modes: Query<Has<EditorMode>>,
    mut emitter_query: Query<(
        &EmitterEntity,
        &mut EmitterRuntime,
        &mut ParticleBufferHandle,
        &mut ParticleMeshHandle,
        &mut Mesh3d,
        &mut CurrentMeshConfig,
        &mut ParticleMaterialHandle,
        &mut MeshMaterial3d<ParticleMaterial>,
        &mut CurrentMaterialConfig,
    )>,
    assets: Res<Assets<ParticlesAsset>>,
    asset_server: Res<AssetServer>,
    mut buffers: ResMut<Assets<ShaderStorageBuffer>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut mesh_cache: ResMut<ParticleMeshCache>,
    mut materials: ResMut<Assets<ParticleMaterial>>,
) {
    for (
        emitter,
        mut runtime,
        mut buffer_handle,
        mut mesh_handle,
        mut mesh3d,
        mut current_config,
        mut material_handle,
        mut material3d,
        mut current_material_config,
    ) in emitter_query.iter_mut()
    {
        let Some(emitter_data) = get_emitter_data(
            emitter.parent_system,
            runtime.emitter_index,
            &particle_systems,
            &assets,
        ) else {
            continue;
        };

        let new_amount = emitter_data.emission.particles_amount;
        let new_trail_size = emitter_data.trail_size();
        let new_trail_history_frames = compute_trail_history_frames(emitter_data);

        if buffer_handle.amount == new_amount
            && buffer_handle.trail_size == new_trail_size
            && buffer_handle.trail_history_frames == new_trail_history_frames
        {
            continue;
        }

        let new_total = new_amount * new_trail_size;
        let particles: Vec<ParticleData> =
            (0..new_total).map(|_| ParticleData::default()).collect();

        let new_particle_buf = buffers.add(ShaderStorageBuffer::from(particles.clone()));
        let new_indices_buf = buffers.add(ShaderStorageBuffer::from(
            (0..new_total).collect::<Vec<u32>>(),
        ));
        let new_sorted_buf = buffers.add(ShaderStorageBuffer::from(particles));

        let emitter_uniforms = ParticleEmitterUniforms {
            max_particles: new_total,
            particle_flags: emitter_data.particle_flags.bits(),
            trail_size: new_trail_size,
            transform_align: transform_align_to_u32(emitter_data.draw_pass.transform_align),
            trail_thickness_curve: bake_thickness_curve(&emitter_data.trail),
            ..default()
        };
        let mut emitter_uniforms_ssbo = ShaderStorageBuffer::default();
        emitter_uniforms_ssbo.set_data(emitter_uniforms);
        let new_uniforms_buf = buffers.add(emitter_uniforms_ssbo);

        buffer_handle.particle_buffer = new_particle_buf;
        buffer_handle.indices_buffer = new_indices_buf;
        buffer_handle.sorted_particles_buffer = new_sorted_buf.clone();
        buffer_handle.emitter_uniforms_buffer = new_uniforms_buf.clone();
        buffer_handle.max_particles = new_total;
        buffer_handle.amount = new_amount;
        buffer_handle.trail_size = new_trail_size;

        buffer_handle.trail_history_buffer =
            create_trail_history_buffer(new_amount, new_trail_history_frames, &mut buffers);
        buffer_handle.trail_history_frames = new_trail_history_frames;
        runtime.trail_history_write_index = 0;
        runtime.trail_history_frames = new_trail_history_frames;

        let is_editor = editor_modes.get(emitter.parent_system).unwrap_or(false);
        let assets_folders =
            get_editor_assets_folders(emitter.parent_system, is_editor, &particle_systems, &assets);

        let new_material = materials.add(create_particle_material_from_config(
            &emitter_data.draw_pass.material,
            new_sorted_buf,
            new_uniforms_buf,
            &asset_server,
            assets_folders,
        ));
        material3d.0 = new_material.clone();
        material_handle.0 = new_material;
        current_material_config.0 = emitter_data.draw_pass.material.clone();

        let new_mesh_handle =
            mesh_cache.get_or_create(&emitter_data.draw_pass.mesh, new_amount, &mut meshes);
        mesh3d.0 = new_mesh_handle.clone();
        mesh_handle.0 = new_mesh_handle.clone();
        current_config.0 = emitter_data.draw_pass.mesh.clone();
    }
}

pub fn write_emitter_uniforms(
    particle_systems: Query<&Particles3d>,
    emitter_query: Query<(
        &EmitterEntity,
        &EmitterRuntime,
        &ParticleBufferHandle,
        &GlobalTransform,
    )>,
    assets: Res<Assets<ParticlesAsset>>,
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

        let trail_size = emitter_data.trail_size();
        let trail_thickness_curve = bake_thickness_curve(&emitter_data.trail);

        let uniforms = ParticleEmitterUniforms {
            emitter_transform: global_transform.to_matrix(),
            max_particles: buffer_handle.max_particles,
            particle_flags: emitter_data.particle_flags.bits(),
            use_local_coords: emitter_data.draw_pass.use_local_coords as u32,
            trail_size,
            transform_align: transform_align_to_u32(emitter_data.draw_pass.transform_align),
            trail_thickness_curve,
        };

        if let Some(buffer) = buffers.get_mut(&buffer_handle.emitter_uniforms_buffer) {
            buffer.set_data(uniforms);
        }
    }
}

pub fn sync_particle_material(
    particle_systems: Query<&Particles3d>,
    editor_modes: Query<Has<EditorMode>>,
    mut emitter_query: Query<(
        &EmitterEntity,
        &EmitterRuntime,
        &mut CurrentMaterialConfig,
        &mut ParticleMaterialHandle,
        &mut MeshMaterial3d<ParticleMaterial>,
    )>,
    assets: Res<Assets<ParticlesAsset>>,
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

            let is_editor = editor_modes.get(emitter.parent_system).unwrap_or(false);
            let assets_folders = get_editor_assets_folders(
                emitter.parent_system,
                is_editor,
                &particle_systems,
                &assets,
            );

            let new_material_handle = materials.add(create_particle_material_from_config(
                &new_material,
                sorted_particles_handle,
                emitter_uniforms_handle,
                &asset_server,
                assets_folders,
            ));

            material3d.0 = new_material_handle.clone();
            current_config.0 = new_material;
            material_handle.0 = new_material_handle;
        }
    }
}
