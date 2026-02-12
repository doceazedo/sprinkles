use bevy::{
    prelude::*,
    render::{
        Render, RenderApp, RenderStartup, RenderSystems,
        render_asset::RenderAssets,
        render_graph::{self, RenderGraph, RenderLabel},
        render_resource::{
            BindGroupEntries, BindGroupLayoutDescriptor, BindGroupLayoutEntries, Buffer,
            CachedComputePipelineId, CachedPipelineState, ComputePassDescriptor,
            ComputePipelineDescriptor, PipelineCache, ShaderStages, ShaderType, UniformBuffer,
            binding_types::{storage_buffer, uniform_buffer},
        },
        renderer::{RenderContext, RenderDevice, RenderQueue},
        storage::GpuShaderStorageBuffer,
    },
};
use std::borrow::Cow;

use crate::compute::ParticleComputeLabel;
use crate::extract::ExtractedParticleSystem;
use crate::runtime::ParticleData;

const SHADER_ASSET_PATH: &str = "embedded://bevy_sprinkles/shaders/particle_sort.wgsl";
const WORKGROUP_SIZE: u32 = 256;

#[derive(Clone, Copy, Default, ShaderType)]
pub struct SortParams {
    pub amount: u32,
    pub draw_order: u32,
    pub stage: u32,
    pub step: u32,
    pub camera_position: Vec3,
    pub _pad1: f32,
    pub camera_forward: Vec3,
    pub _pad2: f32,
    pub emitter_transform: Mat4,
}

#[derive(Debug, Hash, PartialEq, Eq, Clone, RenderLabel)]
pub struct ParticleSortLabel;

#[derive(Resource)]
pub struct ParticleSortPipeline {
    pub bind_group_layout: BindGroupLayoutDescriptor,
    pub init_pipeline: CachedComputePipelineId,
    pub sort_pipeline: CachedComputePipelineId,
    pub copy_pipeline: CachedComputePipelineId,
}

pub fn init_particle_sort_pipeline(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    pipeline_cache: Res<PipelineCache>,
) {
    let bind_group_layout = BindGroupLayoutDescriptor::new(
        "ParticleSortBindGroup",
        &BindGroupLayoutEntries::sequential(
            ShaderStages::COMPUTE,
            (
                uniform_buffer::<SortParams>(false),
                storage_buffer::<ParticleData>(false),
                storage_buffer::<u32>(false),
                storage_buffer::<ParticleData>(false),
            ),
        ),
    );

    let shader = asset_server.load(SHADER_ASSET_PATH);

    let init_pipeline = pipeline_cache.queue_compute_pipeline(ComputePipelineDescriptor {
        label: Some("particle_sort_init_pipeline".into()),
        layout: vec![bind_group_layout.clone()],
        shader: shader.clone(),
        entry_point: Some(Cow::from("init_indices")),
        ..default()
    });

    let sort_pipeline = pipeline_cache.queue_compute_pipeline(ComputePipelineDescriptor {
        label: Some("particle_sort_pipeline".into()),
        layout: vec![bind_group_layout.clone()],
        shader: shader.clone(),
        entry_point: Some(Cow::from("sort")),
        ..default()
    });

    let copy_pipeline = pipeline_cache.queue_compute_pipeline(ComputePipelineDescriptor {
        label: Some("particle_sort_copy_pipeline".into()),
        layout: vec![bind_group_layout.clone()],
        shader,
        entry_point: Some(Cow::from("copy_sorted")),
        ..default()
    });

    commands.insert_resource(ParticleSortPipeline {
        bind_group_layout,
        init_pipeline,
        sort_pipeline,
        copy_pipeline,
    });
}

#[derive(Resource, Default)]
pub struct ParticleSortData {
    pub emitters: Vec<SortEmitterData>,
}

pub struct SortEmitterData {
    pub entity: Entity,
    pub particle_buffer: Buffer,
    pub indices_buffer: Buffer,
    pub sorted_particles_buffer: Buffer,
    pub amount: u32,
    pub draw_order: u32,
    pub camera_position: Vec3,
    pub camera_forward: Vec3,
    pub emitter_transform: Mat4,
}

pub fn prepare_particle_sort_data(
    mut commands: Commands,
    extracted_systems: Res<ExtractedParticleSystem>,
    gpu_storage_buffers: Res<RenderAssets<GpuShaderStorageBuffer>>,
) {
    let mut emitters = Vec::new();

    for (entity, emitter_data) in &extracted_systems.emitters {
        let Some(particle_buffer) = gpu_storage_buffers.get(&emitter_data.particle_buffer_handle)
        else {
            continue;
        };

        let Some(indices_buffer) = gpu_storage_buffers.get(&emitter_data.indices_buffer_handle)
        else {
            continue;
        };

        let Some(sorted_particles_buffer) =
            gpu_storage_buffers.get(&emitter_data.sorted_particles_buffer_handle)
        else {
            continue;
        };

        emitters.push(SortEmitterData {
            entity: *entity,
            particle_buffer: particle_buffer.buffer.clone(),
            indices_buffer: indices_buffer.buffer.clone(),
            sorted_particles_buffer: sorted_particles_buffer.buffer.clone(),
            amount: emitter_data.amount,
            draw_order: emitter_data.draw_order,
            camera_position: Vec3::from_array(emitter_data.camera_position),
            camera_forward: Vec3::from_array(emitter_data.camera_forward),
            emitter_transform: emitter_data.emitter_transform,
        });
    }

    commands.insert_resource(ParticleSortData { emitters });
}

pub struct ParticleSortNode {
    ready: bool,
}

impl Default for ParticleSortNode {
    fn default() -> Self {
        Self { ready: false }
    }
}

impl render_graph::Node for ParticleSortNode {
    fn update(&mut self, world: &mut World) {
        let pipeline = world.resource::<ParticleSortPipeline>();
        let pipeline_cache = world.resource::<PipelineCache>();

        let init_ready = matches!(
            pipeline_cache.get_compute_pipeline_state(pipeline.init_pipeline),
            CachedPipelineState::Ok(_)
        );
        let sort_ready = matches!(
            pipeline_cache.get_compute_pipeline_state(pipeline.sort_pipeline),
            CachedPipelineState::Ok(_)
        );
        let copy_ready = matches!(
            pipeline_cache.get_compute_pipeline_state(pipeline.copy_pipeline),
            CachedPipelineState::Ok(_)
        );

        self.ready = init_ready && sort_ready && copy_ready;
    }

    fn run(
        &self,
        _graph: &mut render_graph::RenderGraphContext,
        render_context: &mut RenderContext,
        world: &World,
    ) -> Result<(), render_graph::NodeRunError> {
        if !self.ready {
            return Ok(());
        }

        let pipeline_cache = world.resource::<PipelineCache>();
        let pipeline = world.resource::<ParticleSortPipeline>();
        let sort_data = world.resource::<ParticleSortData>();
        let render_device = world.resource::<RenderDevice>();
        let render_queue = world.resource::<RenderQueue>();

        let Some(init_pipeline) = pipeline_cache.get_compute_pipeline(pipeline.init_pipeline)
        else {
            return Ok(());
        };

        let Some(sort_pipeline) = pipeline_cache.get_compute_pipeline(pipeline.sort_pipeline)
        else {
            return Ok(());
        };

        let Some(copy_pipeline) = pipeline_cache.get_compute_pipeline(pipeline.copy_pipeline)
        else {
            return Ok(());
        };

        let bind_group_layout = pipeline_cache.get_bind_group_layout(&pipeline.bind_group_layout);

        for data in &sort_data.emitters {
            let workgroups = (data.amount + WORKGROUP_SIZE - 1) / WORKGROUP_SIZE;

            {
                let sort_params = SortParams {
                    amount: data.amount,
                    draw_order: data.draw_order,
                    stage: 0,
                    step: 0,
                    camera_position: data.camera_position,
                    _pad1: 0.0,
                    camera_forward: data.camera_forward,
                    _pad2: 0.0,
                    emitter_transform: data.emitter_transform,
                };

                let mut uniform_buffer = UniformBuffer::from(sort_params);
                uniform_buffer.write_buffer(render_device, render_queue);

                let bind_group = render_device.create_bind_group(
                    Some("particle_sort_init_bind_group"),
                    &bind_group_layout,
                    &BindGroupEntries::sequential((
                        &uniform_buffer,
                        data.particle_buffer.as_entire_binding(),
                        data.indices_buffer.as_entire_binding(),
                        data.sorted_particles_buffer.as_entire_binding(),
                    )),
                );

                let mut pass =
                    render_context
                        .command_encoder()
                        .begin_compute_pass(&ComputePassDescriptor {
                            label: Some("particle_sort_init_pass"),
                            ..default()
                        });

                pass.set_pipeline(init_pipeline);
                pass.set_bind_group(0, &bind_group, &[]);
                pass.dispatch_workgroups(workgroups, 1, 1);
            }

            if data.draw_order != 0 {
                let n = data.amount.next_power_of_two();
                let num_stages = (n as f32).log2().ceil() as u32;

                for stage in 0..num_stages {
                    for step in (0..=stage).rev() {
                        let sort_params = SortParams {
                            amount: data.amount,
                            draw_order: data.draw_order,
                            stage,
                            step,
                            camera_position: data.camera_position,
                            _pad1: 0.0,
                            camera_forward: data.camera_forward,
                            _pad2: 0.0,
                            emitter_transform: data.emitter_transform,
                        };

                        let mut uniform_buffer = UniformBuffer::from(sort_params);
                        uniform_buffer.write_buffer(render_device, render_queue);

                        let bind_group = render_device.create_bind_group(
                            Some("particle_sort_bind_group"),
                            &bind_group_layout,
                            &BindGroupEntries::sequential((
                                &uniform_buffer,
                                data.particle_buffer.as_entire_binding(),
                                data.indices_buffer.as_entire_binding(),
                                data.sorted_particles_buffer.as_entire_binding(),
                            )),
                        );

                        let mut pass = render_context.command_encoder().begin_compute_pass(
                            &ComputePassDescriptor {
                                label: Some("particle_sort_pass"),
                                ..default()
                            },
                        );

                        pass.set_pipeline(sort_pipeline);
                        pass.set_bind_group(0, &bind_group, &[]);
                        pass.dispatch_workgroups(workgroups, 1, 1);
                    }
                }
            }

            {
                let sort_params = SortParams {
                    amount: data.amount,
                    draw_order: data.draw_order,
                    stage: 0,
                    step: 0,
                    camera_position: data.camera_position,
                    _pad1: 0.0,
                    camera_forward: data.camera_forward,
                    _pad2: 0.0,
                    emitter_transform: data.emitter_transform,
                };

                let mut uniform_buffer = UniformBuffer::from(sort_params);
                uniform_buffer.write_buffer(render_device, render_queue);

                let bind_group = render_device.create_bind_group(
                    Some("particle_sort_copy_bind_group"),
                    &bind_group_layout,
                    &BindGroupEntries::sequential((
                        &uniform_buffer,
                        data.particle_buffer.as_entire_binding(),
                        data.indices_buffer.as_entire_binding(),
                        data.sorted_particles_buffer.as_entire_binding(),
                    )),
                );

                let mut pass =
                    render_context
                        .command_encoder()
                        .begin_compute_pass(&ComputePassDescriptor {
                            label: Some("particle_sort_copy_pass"),
                            ..default()
                        });

                pass.set_pipeline(copy_pipeline);
                pass.set_bind_group(0, &bind_group, &[]);
                pass.dispatch_workgroups(workgroups, 1, 1);
            }
        }

        Ok(())
    }
}

pub struct ParticleSortPlugin;

impl Plugin for ParticleSortPlugin {
    fn build(&self, app: &mut App) {
        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };

        render_app
            .init_resource::<ParticleSortData>()
            .add_systems(RenderStartup, init_particle_sort_pipeline)
            .add_systems(
                Render,
                prepare_particle_sort_data.in_set(RenderSystems::PrepareBindGroups),
            );

        let mut render_graph = render_app.world_mut().resource_mut::<RenderGraph>();
        render_graph.add_node(ParticleSortLabel, ParticleSortNode::default());
        render_graph.add_node_edge(ParticleComputeLabel, ParticleSortLabel);
        render_graph.add_node_edge(ParticleSortLabel, bevy::render::graph::CameraDriverLabel);
    }
}
