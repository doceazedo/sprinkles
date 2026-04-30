use bevy::{
    core_pipeline::schedule::camera_driver,
    prelude::*,
    render::{
        Render, RenderApp, RenderStartup, RenderSystems,
        render_asset::RenderAssets,
        render_resource::{
            BindGroup, BindGroupEntries, BindGroupLayoutDescriptor, BindGroupLayoutEntries, Buffer,
            CachedComputePipelineId, CachedPipelineState, ComputePassDescriptor,
            ComputePipelineDescriptor, DynamicUniformBuffer, PipelineCache, ShaderStages,
            ShaderType,
            binding_types::{storage_buffer, uniform_buffer},
        },
        renderer::{RenderContext, RenderDevice, RenderGraph, RenderGraphSystems, RenderQueue},
        storage::GpuShaderBuffer,
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
    pub trail_size: u32,
    pub _trail_pad0: u32,
    pub _trail_pad1: u32,
    pub _trail_pad2: u32,
}

#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemSet)]
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
                uniform_buffer::<SortParams>(true),
                storage_buffer::<ParticleData>(false),
                storage_buffer::<u32>(false),
                storage_buffer::<ParticleData>(false),
            ),
        ),
    );

    let shader = asset_server.load(SHADER_ASSET_PATH);

    let queue_pipeline = |label: &'static str, entry: &'static str| {
        pipeline_cache.queue_compute_pipeline(ComputePipelineDescriptor {
            label: Some(label.into()),
            layout: vec![bind_group_layout.clone()],
            shader: shader.clone(),
            entry_point: Some(Cow::from(entry)),
            ..default()
        })
    };

    let init_pipeline = queue_pipeline("particle_sort_init_pipeline", "init_indices");
    let sort_pipeline = queue_pipeline("particle_sort_pipeline", "sort");
    let copy_pipeline = queue_pipeline("particle_sort_copy_pipeline", "copy_sorted");

    commands.insert_resource(ParticleSortPipeline {
        bind_group_layout,
        init_pipeline,
        sort_pipeline,
        copy_pipeline,
    });
}

struct SortDispatch {
    emitter_index: usize,
    dynamic_offset: u32,
    workgroups: u32,
}

#[derive(Resource, Default)]
pub struct ParticleSortBindGroups {
    bind_groups: Vec<BindGroup>,
    init_dispatches: Vec<SortDispatch>,
    sort_levels: Vec<Vec<SortDispatch>>,
    copy_dispatches: Vec<SortDispatch>,
}

pub fn prepare_particle_sort_bind_groups(
    mut commands: Commands,
    pipeline: Res<ParticleSortPipeline>,
    pipeline_cache: Res<PipelineCache>,
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
    extracted_systems: Res<ExtractedParticleSystem>,
    gpu_storage_buffers: Res<RenderAssets<GpuShaderBuffer>>,
) {
    let mut result = ParticleSortBindGroups::default();
    let mut dynamic_uniform = DynamicUniformBuffer::<SortParams>::default();
    let mut emitter_buffers: Vec<(Buffer, Buffer, Buffer)> = Vec::new();

    for (_entity, emitter_data) in &extracted_systems.emitters {
        let Some(particle_buf) = gpu_storage_buffers.get(&emitter_data.particle_buffer_handle)
        else {
            continue;
        };
        let Some(indices_buf) = gpu_storage_buffers.get(&emitter_data.indices_buffer_handle) else {
            continue;
        };
        let Some(sorted_buf) =
            gpu_storage_buffers.get(&emitter_data.sorted_particles_buffer_handle)
        else {
            continue;
        };

        let emitter_idx = emitter_buffers.len();
        let trail_size = emitter_data.trail_size;
        let total_slots = emitter_data.amount * trail_size;
        let group_count = emitter_data.amount;
        let group_workgroups = (group_count + WORKGROUP_SIZE - 1) / WORKGROUP_SIZE;
        let total_workgroups = (total_slots + WORKGROUP_SIZE - 1) / WORKGROUP_SIZE;

        emitter_buffers.push((
            particle_buf.buffer.clone(),
            indices_buf.buffer.clone(),
            sorted_buf.buffer.clone(),
        ));

        let base_params = SortParams {
            amount: total_slots,
            draw_order: emitter_data.draw_order,
            stage: 0,
            step: 0,
            camera_position: Vec3::from_array(emitter_data.camera_position),
            _pad1: 0.0,
            camera_forward: Vec3::from_array(emitter_data.camera_forward),
            _pad2: 0.0,
            emitter_transform: emitter_data.emitter_transform,
            trail_size,
            _trail_pad0: 0,
            _trail_pad1: 0,
            _trail_pad2: 0,
        };

        let init_offset = dynamic_uniform.push(&base_params);
        result.init_dispatches.push(SortDispatch {
            emitter_index: emitter_idx,
            dynamic_offset: init_offset,
            workgroups: group_workgroups,
        });

        if emitter_data.draw_order != 0 {
            let n = group_count.next_power_of_two();
            let num_stages = (n as f32).log2().ceil() as u32;

            let mut level = 0usize;
            for stage in 0..num_stages {
                for step_val in (0..=stage).rev() {
                    let offset = dynamic_uniform.push(&SortParams {
                        stage,
                        step: step_val,
                        ..base_params
                    });

                    while result.sort_levels.len() <= level {
                        result.sort_levels.push(Vec::new());
                    }

                    result.sort_levels[level].push(SortDispatch {
                        emitter_index: emitter_idx,
                        dynamic_offset: offset,
                        workgroups: group_workgroups,
                    });

                    level += 1;
                }
            }
        }

        let copy_offset = dynamic_uniform.push(&base_params);
        result.copy_dispatches.push(SortDispatch {
            emitter_index: emitter_idx,
            dynamic_offset: copy_offset,
            workgroups: total_workgroups,
        });
    }

    dynamic_uniform.write_buffer(&render_device, &render_queue);

    if let Some(uniform_binding) = dynamic_uniform.binding() {
        let bind_group_layout = pipeline_cache.get_bind_group_layout(&pipeline.bind_group_layout);

        for (particle_buf, indices_buf, sorted_buf) in &emitter_buffers {
            let bind_group = render_device.create_bind_group(
                Some("particle_sort_bind_group"),
                &bind_group_layout,
                &BindGroupEntries::sequential((
                    uniform_binding.clone(),
                    particle_buf.as_entire_binding(),
                    indices_buf.as_entire_binding(),
                    sorted_buf.as_entire_binding(),
                )),
            );
            result.bind_groups.push(bind_group);
        }
    }

    commands.insert_resource(result);
}

pub fn run_particle_sort_node(
    pipeline: Res<ParticleSortPipeline>,
    pipeline_cache: Res<PipelineCache>,
    sort_bind_groups: Res<ParticleSortBindGroups>,
    mut ctx: RenderContext,
) {
    let is_ready = |id| {
        matches!(
            pipeline_cache.get_compute_pipeline_state(id),
            CachedPipelineState::Ok(_)
        )
    };

    if !(is_ready(pipeline.init_pipeline)
        && is_ready(pipeline.sort_pipeline)
        && is_ready(pipeline.copy_pipeline))
    {
        return;
    }

    let Some(init_pipeline) = pipeline_cache.get_compute_pipeline(pipeline.init_pipeline) else {
        return;
    };

    let Some(sort_pipeline) = pipeline_cache.get_compute_pipeline(pipeline.sort_pipeline) else {
        return;
    };

    let Some(copy_pipeline) = pipeline_cache.get_compute_pipeline(pipeline.copy_pipeline) else {
        return;
    };

    if sort_bind_groups.bind_groups.is_empty() {
        return;
    }

    let mut run_pass = |label, pipeline, dispatches: &[SortDispatch]| {
        let mut pass = ctx
            .command_encoder()
            .begin_compute_pass(&ComputePassDescriptor {
                label: Some(label),
                ..default()
            });
        pass.set_pipeline(pipeline);
        for dispatch in dispatches {
            pass.set_bind_group(
                0,
                &sort_bind_groups.bind_groups[dispatch.emitter_index],
                &[dispatch.dynamic_offset],
            );
            pass.dispatch_workgroups(dispatch.workgroups, 1, 1);
        }
    };

    run_pass(
        "particle_sort_init_pass",
        init_pipeline,
        &sort_bind_groups.init_dispatches,
    );

    for level in &sort_bind_groups.sort_levels {
        run_pass("particle_sort_pass", sort_pipeline, level);
    }

    run_pass(
        "particle_sort_copy_pass",
        copy_pipeline,
        &sort_bind_groups.copy_dispatches,
    );
}

pub struct ParticleSortPlugin;

impl Plugin for ParticleSortPlugin {
    fn build(&self, app: &mut App) {
        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };

        render_app
            .init_resource::<ParticleSortBindGroups>()
            .add_systems(RenderStartup, init_particle_sort_pipeline)
            .add_systems(
                Render,
                prepare_particle_sort_bind_groups.in_set(RenderSystems::PrepareBindGroups),
            )
            .add_systems(
                RenderGraph,
                run_particle_sort_node
                    .in_set(ParticleSortLabel)
                    .in_set(RenderGraphSystems::Render)
                    .after(ParticleComputeLabel)
                    .before(camera_driver),
            );
    }
}
