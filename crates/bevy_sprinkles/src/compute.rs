use bevy::{
    prelude::*,
    render::{
        Render, RenderApp, RenderStartup, RenderSystems,
        render_asset::RenderAssets,
        render_graph::{self, RenderGraph, RenderLabel},
        render_resource::{
            BindGroup, BindGroupEntries, BindGroupLayoutDescriptor, BindGroupLayoutEntries, Buffer,
            BufferUsages, CachedComputePipelineId, CachedPipelineState, ComputePassDescriptor,
            ComputePipelineDescriptor, PipelineCache, SamplerBindingType, SamplerDescriptor,
            ShaderStages, TextureSampleType,
            binding_types::{
                sampler, storage_buffer, storage_buffer_read_only, storage_buffer_sized,
                texture_2d, uniform_buffer,
            },
        },
        renderer::{RenderContext, RenderDevice, RenderQueue},
        storage::GpuShaderStorageBuffer,
        texture::GpuImage,
    },
};
use std::borrow::Cow;

use bevy::render::render_resource::ShaderType;

use crate::extract::{
    ColliderUniform, EmitterUniforms, ExtractedColliders, ExtractedEmitterData,
    ExtractedParticleSystem, MAX_COLLIDERS,
};
use crate::runtime::ParticleData;
use crate::textures::{FallbackCurveTexture, FallbackGradientTexture};

#[derive(Clone, Copy, Default, bytemuck::Pod, bytemuck::Zeroable, ShaderType)]
#[repr(C)]
pub struct ColliderArray {
    pub colliders: [ColliderUniform; MAX_COLLIDERS],
}

const SHADER_ASSET_PATH: &str = "embedded://bevy_sprinkles/shaders/particle_simulate.wgsl";
const WORKGROUP_SIZE: u32 = 64;

#[derive(Debug, Hash, PartialEq, Eq, Clone, RenderLabel)]
pub struct ParticleComputeLabel;

#[derive(Resource)]
pub struct ParticleComputePipeline {
    pub bind_group_layout: BindGroupLayoutDescriptor,
    pub simulate_pipeline: CachedComputePipelineId,
}

pub fn init_particle_compute_pipeline(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    pipeline_cache: Res<PipelineCache>,
    render_device: Res<RenderDevice>,
) {
    let bind_group_layout = BindGroupLayoutDescriptor::new(
        "ParticleComputeBindGroup",
        &BindGroupLayoutEntries::sequential(
            ShaderStages::COMPUTE,
            (
                uniform_buffer::<EmitterUniforms>(false),
                storage_buffer::<ParticleData>(false),
                texture_2d(TextureSampleType::Float { filterable: true }),
                sampler(SamplerBindingType::Filtering),
                texture_2d(TextureSampleType::Float { filterable: true }),
                sampler(SamplerBindingType::Filtering),
                texture_2d(TextureSampleType::Float { filterable: true }),
                sampler(SamplerBindingType::Filtering),
                texture_2d(TextureSampleType::Float { filterable: true }),
                sampler(SamplerBindingType::Filtering),
                texture_2d(TextureSampleType::Float { filterable: true }),
                sampler(SamplerBindingType::Filtering),
                texture_2d(TextureSampleType::Float { filterable: true }),
                sampler(SamplerBindingType::Filtering),
                texture_2d(TextureSampleType::Float { filterable: true }),
                sampler(SamplerBindingType::Filtering),
                texture_2d(TextureSampleType::Float { filterable: true }),
                sampler(SamplerBindingType::Filtering),
                texture_2d(TextureSampleType::Float { filterable: true }),
                sampler(SamplerBindingType::Filtering),
                storage_buffer_read_only::<ColliderArray>(false),
                storage_buffer_sized(false, None),
                storage_buffer_sized(false, None),
            ),
        ),
    );

    let shader = asset_server.load(SHADER_ASSET_PATH);
    let simulate_pipeline = pipeline_cache.queue_compute_pipeline(ComputePipelineDescriptor {
        label: Some("particle_simulate_pipeline".into()),
        layout: vec![bind_group_layout.clone()],
        shader,
        entry_point: Some(Cow::from("main")),
        ..default()
    });

    let gradient_sampler = render_device.create_sampler(&SamplerDescriptor {
        label: Some("gradient_sampler"),
        address_mode_u: bevy::render::render_resource::AddressMode::ClampToEdge,
        address_mode_v: bevy::render::render_resource::AddressMode::ClampToEdge,
        mag_filter: bevy::render::render_resource::FilterMode::Linear,
        min_filter: bevy::render::render_resource::FilterMode::Linear,
        ..default()
    });

    let curve_sampler = render_device.create_sampler(&SamplerDescriptor {
        label: Some("curve_sampler"),
        address_mode_u: bevy::render::render_resource::AddressMode::ClampToEdge,
        address_mode_v: bevy::render::render_resource::AddressMode::ClampToEdge,
        mag_filter: bevy::render::render_resource::FilterMode::Linear,
        min_filter: bevy::render::render_resource::FilterMode::Linear,
        ..default()
    });

    let fallback_emission_buffer = render_device.create_buffer_with_data(
        &bevy::render::render_resource::BufferInitDescriptor {
            label: Some("fallback_emission_buffer"),
            contents: &[0u8; 64],
            usage: BufferUsages::STORAGE | BufferUsages::COPY_DST,
        },
    );

    commands.insert_resource(ParticleComputePipeline {
        bind_group_layout,
        simulate_pipeline,
    });
    commands.insert_resource(GradientSampler(gradient_sampler));
    commands.insert_resource(CurveSampler(curve_sampler));
    commands.insert_resource(FallbackEmissionBuffer(fallback_emission_buffer));
}

#[derive(Resource)]
pub struct GradientSampler(pub bevy::render::render_resource::Sampler);

#[derive(Resource)]
pub struct CurveSampler(pub bevy::render::render_resource::Sampler);

#[derive(Resource)]
pub struct FallbackEmissionBuffer(pub Buffer);

#[derive(Resource, Default)]
pub struct EmissionBufferClearList {
    pub buffers: Vec<Buffer>,
}

#[derive(Resource, Default)]
pub struct ParticleComputeBindGroups {
    pub bind_groups: Vec<(Entity, Vec<BindGroup>)>,
}

pub fn prepare_particle_compute_bind_groups(
    mut commands: Commands,
    pipeline: Res<ParticleComputePipeline>,
    pipeline_cache: Res<PipelineCache>,
    render_device: Res<RenderDevice>,
    _render_queue: Res<RenderQueue>,
    extracted_systems: Res<ExtractedParticleSystem>,
    extracted_colliders: Option<Res<ExtractedColliders>>,
    gpu_storage_buffers: Res<RenderAssets<GpuShaderStorageBuffer>>,
    gpu_images: Res<RenderAssets<GpuImage>>,
    fallback_gradient_texture: Option<Res<FallbackGradientTexture>>,
    fallback_curve_texture: Option<Res<FallbackCurveTexture>>,
    fallback_emission_buffer: Res<FallbackEmissionBuffer>,
    gradient_sampler: Res<GradientSampler>,
    curve_sampler: Res<CurveSampler>,
) {
    let mut bind_groups = Vec::new();

    let fallback_gradient_gpu_image = fallback_gradient_texture
        .as_ref()
        .and_then(|ft| gpu_images.get(&ft.handle));

    let fallback_curve_gpu_image = fallback_curve_texture
        .as_ref()
        .and_then(|ft| gpu_images.get(&ft.handle));

    let mut collider_array = ColliderArray::default();
    let collider_count = if let Some(ref colliders) = extracted_colliders {
        for (i, collider) in colliders.colliders.iter().enumerate() {
            if i >= MAX_COLLIDERS {
                break;
            }
            collider_array.colliders[i] = *collider;
        }
        colliders.colliders.len().min(MAX_COLLIDERS) as u32
    } else {
        0
    };

    let colliders_buffer = render_device.create_buffer_with_data(
        &bevy::render::render_resource::BufferInitDescriptor {
            label: Some("colliders_buffer"),
            contents: bytemuck::bytes_of(&collider_array),
            usage: BufferUsages::STORAGE | BufferUsages::COPY_DST,
        },
    );

    let mut emission_clear_list = Vec::new();

    for (entity, emitter_data) in &extracted_systems.emitters {
        let Some(gpu_buffer) = gpu_storage_buffers.get(&emitter_data.particle_buffer_handle) else {
            continue;
        };

        let gradient_gpu_image = emitter_data
            .gradient_texture_handle
            .as_ref()
            .and_then(|h| gpu_images.get(h))
            .or(fallback_gradient_gpu_image);

        let color_over_lifetime_gpu_image = emitter_data
            .color_over_lifetime_texture_handle
            .as_ref()
            .and_then(|h| gpu_images.get(h))
            .or(fallback_gradient_gpu_image);

        let scale_over_lifetime_gpu_image = emitter_data
            .scale_over_lifetime_texture_handle
            .as_ref()
            .and_then(|h| gpu_images.get(h))
            .or(fallback_curve_gpu_image);

        let alpha_over_lifetime_gpu_image = emitter_data
            .alpha_over_lifetime_texture_handle
            .as_ref()
            .and_then(|h| gpu_images.get(h))
            .or(fallback_curve_gpu_image);

        let emission_over_lifetime_gpu_image = emitter_data
            .emission_over_lifetime_texture_handle
            .as_ref()
            .and_then(|h| gpu_images.get(h))
            .or(fallback_curve_gpu_image);

        let turbulence_influence_over_lifetime_gpu_image = emitter_data
            .turbulence_influence_over_lifetime_texture_handle
            .as_ref()
            .and_then(|h| gpu_images.get(h))
            .or(fallback_curve_gpu_image);

        let radial_velocity_curve_gpu_image = emitter_data
            .radial_velocity_curve_texture_handle
            .as_ref()
            .and_then(|h| gpu_images.get(h))
            .or(fallback_curve_gpu_image);

        let angle_over_lifetime_gpu_image = emitter_data
            .angle_over_lifetime_texture_handle
            .as_ref()
            .and_then(|h| gpu_images.get(h))
            .or(fallback_curve_gpu_image);

        let angular_velocity_curve_gpu_image = emitter_data
            .angular_velocity_curve_texture_handle
            .as_ref()
            .and_then(|h| gpu_images.get(h))
            .or(fallback_curve_gpu_image);

        let Some(gradient_image) = gradient_gpu_image else {
            continue;
        };

        let Some(color_over_lifetime_image) = color_over_lifetime_gpu_image else {
            continue;
        };

        let Some(scale_over_lifetime_image) = scale_over_lifetime_gpu_image else {
            continue;
        };

        let Some(alpha_over_lifetime_image) = alpha_over_lifetime_gpu_image else {
            continue;
        };

        let Some(turbulence_influence_over_lifetime_image) =
            turbulence_influence_over_lifetime_gpu_image
        else {
            continue;
        };

        let Some(emission_over_lifetime_image) = emission_over_lifetime_gpu_image else {
            continue;
        };

        let Some(radial_velocity_curve_image) = radial_velocity_curve_gpu_image else {
            continue;
        };

        let Some(angle_over_lifetime_image) = angle_over_lifetime_gpu_image else {
            continue;
        };

        let Some(angular_velocity_curve_image) = angular_velocity_curve_gpu_image else {
            continue;
        };

        let bind_group_layout = pipeline_cache.get_bind_group_layout(&pipeline.bind_group_layout);

        let dst_buffer = emitter_data
            .emission_buffer_handle
            .as_ref()
            .and_then(|h| gpu_storage_buffers.get(h))
            .map(|b| &b.buffer);

        let src_buffer = emitter_data
            .source_buffer_handle
            .as_ref()
            .and_then(|h| gpu_storage_buffers.get(h))
            .map(|b| &b.buffer);

        let dst_binding = dst_buffer.unwrap_or(&fallback_emission_buffer.0);
        let src_binding = src_buffer.unwrap_or(&fallback_emission_buffer.0);

        if let Some(buf) = dst_buffer {
            emission_clear_list.push(buf.clone());
        }

        let step_bind_groups: Vec<BindGroup> = emitter_data
            .uniform_steps
            .iter()
            .map(|step_uniforms| {
                let mut uniforms = *step_uniforms;
                uniforms.collider_count = collider_count;

                let uniform_buffer = render_device.create_buffer_with_data(
                    &bevy::render::render_resource::BufferInitDescriptor {
                        label: Some("emitter_uniform_buffer"),
                        contents: bytemuck::bytes_of(&uniforms),
                        usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
                    },
                );

                render_device.create_bind_group(
                    Some("particle_compute_bind_group"),
                    &bind_group_layout,
                    &BindGroupEntries::sequential((
                        uniform_buffer.as_entire_binding(),
                        gpu_buffer.buffer.as_entire_binding(),
                        &gradient_image.texture_view,
                        &gradient_sampler.0,
                        &scale_over_lifetime_image.texture_view,
                        &curve_sampler.0,
                        &alpha_over_lifetime_image.texture_view,
                        &curve_sampler.0,
                        &emission_over_lifetime_image.texture_view,
                        &curve_sampler.0,
                        &turbulence_influence_over_lifetime_image.texture_view,
                        &curve_sampler.0,
                        &radial_velocity_curve_image.texture_view,
                        &curve_sampler.0,
                        &angle_over_lifetime_image.texture_view,
                        &curve_sampler.0,
                        &angular_velocity_curve_image.texture_view,
                        &curve_sampler.0,
                        &color_over_lifetime_image.texture_view,
                        &gradient_sampler.0,
                        colliders_buffer.as_entire_binding(),
                        dst_binding.as_entire_binding(),
                        src_binding.as_entire_binding(),
                    )),
                )
            })
            .collect();

        bind_groups.push((*entity, step_bind_groups));
    }

    let mut unique_buffers: Vec<Buffer> = Vec::new();
    for buf in emission_clear_list {
        if !unique_buffers.iter().any(|b| b.id() == buf.id()) {
            unique_buffers.push(buf);
        }
    }

    commands.insert_resource(ParticleComputeBindGroups { bind_groups });
    commands.insert_resource(EmissionBufferClearList {
        buffers: unique_buffers,
    });
}

pub struct ParticleComputeNode {
    ready: bool,
}

impl Default for ParticleComputeNode {
    fn default() -> Self {
        Self { ready: false }
    }
}

impl render_graph::Node for ParticleComputeNode {
    fn update(&mut self, world: &mut World) {
        let pipeline = world.resource::<ParticleComputePipeline>();
        let pipeline_cache = world.resource::<PipelineCache>();

        match pipeline_cache.get_compute_pipeline_state(pipeline.simulate_pipeline) {
            CachedPipelineState::Ok(_) => {
                self.ready = true;
            }
            CachedPipelineState::Queued | CachedPipelineState::Creating(_) => {}
            CachedPipelineState::Err(err) => {
                panic!("Failed to load particle compute shader: {err}")
            }
        }
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
        let pipeline = world.resource::<ParticleComputePipeline>();
        let bind_groups = world.resource::<ParticleComputeBindGroups>();
        let extracted = world.resource::<ExtractedParticleSystem>();
        let emission_clear_list = world.resource::<EmissionBufferClearList>();

        let Some(compute_pipeline) =
            pipeline_cache.get_compute_pipeline(pipeline.simulate_pipeline)
        else {
            return Ok(());
        };

        let max_steps = bind_groups
            .bind_groups
            .iter()
            .map(|(_, steps)| steps.len())
            .max()
            .unwrap_or(0);

        let emitter_map: std::collections::HashMap<Entity, &ExtractedEmitterData> = extracted
            .emitters
            .iter()
            .map(|(e, data)| (*e, data))
            .collect();

        let has_sub_emitter_targets = emitter_map.values().any(|data| data.is_sub_emitter_target);
        let pass_labels: &[&str] = if has_sub_emitter_targets {
            &["particle_compute_pass", "particle_sub_emitter_pass"]
        } else {
            &["particle_compute_pass"]
        };

        for step_index in 0..max_steps {
            for buf in &emission_clear_list.buffers {
                render_context
                    .command_encoder()
                    .clear_buffer(buf, 0, Some(4));
            }

            for (pass_index, label) in pass_labels.iter().enumerate() {
                let is_target_pass = pass_index == 1;

                let mut pass =
                    render_context
                        .command_encoder()
                        .begin_compute_pass(&ComputePassDescriptor {
                            label: Some(label),
                            ..default()
                        });

                pass.set_pipeline(compute_pipeline);

                for (entity, step_bind_groups) in &bind_groups.bind_groups {
                    let Some(bind_group) = step_bind_groups.get(step_index) else {
                        continue;
                    };

                    let Some(emitter_data) = emitter_map.get(entity) else {
                        continue;
                    };

                    if emitter_data.is_sub_emitter_target != is_target_pass {
                        continue;
                    }

                    let workgroups = (emitter_data.amount + WORKGROUP_SIZE - 1) / WORKGROUP_SIZE;
                    pass.set_bind_group(0, bind_group, &[]);
                    pass.dispatch_workgroups(workgroups, 1, 1);
                }
            }
        }

        Ok(())
    }
}

pub struct ParticleComputePlugin;

impl Plugin for ParticleComputePlugin {
    fn build(&self, app: &mut App) {
        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };

        render_app
            .init_resource::<ParticleComputeBindGroups>()
            .init_resource::<EmissionBufferClearList>()
            .add_systems(RenderStartup, init_particle_compute_pipeline)
            .add_systems(
                Render,
                prepare_particle_compute_bind_groups.in_set(RenderSystems::PrepareBindGroups),
            );

        let mut render_graph = render_app.world_mut().resource_mut::<RenderGraph>();
        render_graph.add_node(ParticleComputeLabel, ParticleComputeNode::default());
        render_graph.add_node_edge(ParticleComputeLabel, bevy::render::graph::CameraDriverLabel);
    }
}
