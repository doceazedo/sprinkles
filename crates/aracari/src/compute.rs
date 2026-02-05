use bevy::{
    prelude::*,
    render::{
        render_asset::RenderAssets,
        render_graph::{self, RenderGraph, RenderLabel},
        render_resource::{
            binding_types::{sampler, storage_buffer, storage_buffer_read_only, texture_2d, uniform_buffer},
            BindGroup, BindGroupEntries, BindGroupLayoutDescriptor, BindGroupLayoutEntries,
            BufferUsages, CachedComputePipelineId, CachedPipelineState, ComputePassDescriptor,
            ComputePipelineDescriptor, PipelineCache, SamplerBindingType, SamplerDescriptor,
            ShaderStages, TextureSampleType,
        },
        renderer::{RenderContext, RenderDevice, RenderQueue},
        storage::GpuShaderStorageBuffer,
        texture::GpuImage,
        Render, RenderApp, RenderStartup, RenderSystems,
    },
};
use std::borrow::Cow;

use bevy::render::render_resource::ShaderType;

use crate::extract::{ColliderUniform, EmitterUniforms, ExtractedColliders, ExtractedParticleSystem, MAX_COLLIDERS};
use crate::runtime::ParticleData;
use crate::textures::{FallbackCurveTexture, FallbackGradientTexture};

#[derive(Clone, Copy, Default, bytemuck::Pod, bytemuck::Zeroable, ShaderType)]
#[repr(C)]
pub struct ColliderArray {
    pub colliders: [ColliderUniform; MAX_COLLIDERS],
}

const SHADER_ASSET_PATH: &str = "embedded://aracari/shaders/particle_simulate.wgsl";
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
                // colliders storage buffer (read-only)
                storage_buffer_read_only::<ColliderArray>(false),
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

    commands.insert_resource(ParticleComputePipeline {
        bind_group_layout,
        simulate_pipeline,
    });
    commands.insert_resource(GradientSampler(gradient_sampler));
    commands.insert_resource(CurveSampler(curve_sampler));
}

#[derive(Resource)]
pub struct GradientSampler(pub bevy::render::render_resource::Sampler);

#[derive(Resource)]
pub struct CurveSampler(pub bevy::render::render_resource::Sampler);

#[derive(Resource, Default)]
pub struct ParticleComputeBindGroups {
    pub bind_groups: Vec<(Entity, BindGroup)>,
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

    // prepare colliders array
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

    for (entity, emitter_data) in &extracted_systems.emitters {
        let Some(gpu_buffer) = gpu_storage_buffers.get(&emitter_data.particle_buffer_handle) else {
            continue;
        };

        let gradient_gpu_image = emitter_data
            .gradient_texture_handle
            .as_ref()
            .and_then(|h| gpu_images.get(h))
            .or(fallback_gradient_gpu_image);

        let scale_over_lifetime_gpu_image = emitter_data
            .scale_over_lifetime_texture_handle
            .as_ref()
            .and_then(|h| gpu_images.get(h))
            .or(fallback_curve_gpu_image);

        let alpha_curve_gpu_image = emitter_data
            .alpha_curve_texture_handle
            .as_ref()
            .and_then(|h| gpu_images.get(h))
            .or(fallback_curve_gpu_image);

        let emission_curve_gpu_image = emitter_data
            .emission_curve_texture_handle
            .as_ref()
            .and_then(|h| gpu_images.get(h))
            .or(fallback_curve_gpu_image);

        let turbulence_influence_curve_gpu_image = emitter_data
            .turbulence_influence_curve_texture_handle
            .as_ref()
            .and_then(|h| gpu_images.get(h))
            .or(fallback_curve_gpu_image);

        let radial_velocity_curve_gpu_image = emitter_data
            .radial_velocity_curve_texture_handle
            .as_ref()
            .and_then(|h| gpu_images.get(h))
            .or(fallback_curve_gpu_image);

        let Some(gradient_image) = gradient_gpu_image else {
            continue;
        };

        let Some(scale_over_lifetime_image) = scale_over_lifetime_gpu_image else {
            continue;
        };

        let Some(alpha_curve_image) = alpha_curve_gpu_image else {
            continue;
        };

        let Some(turbulence_influence_curve_image) = turbulence_influence_curve_gpu_image else {
            continue;
        };

        let Some(emission_curve_image) = emission_curve_gpu_image else {
            continue;
        };

        let Some(radial_velocity_curve_image) = radial_velocity_curve_gpu_image else {
            continue;
        };

        // update collider_count in uniforms
        let mut uniforms = emitter_data.uniforms;
        uniforms.collider_count = collider_count;

        let uniform_buffer = render_device.create_buffer_with_data(
            &bevy::render::render_resource::BufferInitDescriptor {
                label: Some("emitter_uniform_buffer"),
                contents: bytemuck::bytes_of(&uniforms),
                usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
            },
        );

        let bind_group = render_device.create_bind_group(
            Some("particle_compute_bind_group"),
            &pipeline_cache.get_bind_group_layout(&pipeline.bind_group_layout),
            &BindGroupEntries::sequential((
                uniform_buffer.as_entire_binding(),
                gpu_buffer.buffer.as_entire_binding(),
                &gradient_image.texture_view,
                &gradient_sampler.0,
                &scale_over_lifetime_image.texture_view,
                &curve_sampler.0,
                &alpha_curve_image.texture_view,
                &curve_sampler.0,
                &emission_curve_image.texture_view,
                &curve_sampler.0,
                &turbulence_influence_curve_image.texture_view,
                &curve_sampler.0,
                &radial_velocity_curve_image.texture_view,
                &curve_sampler.0,
                colliders_buffer.as_entire_binding(),
            )),
        );

        bind_groups.push((*entity, bind_group));
    }

    commands.insert_resource(ParticleComputeBindGroups { bind_groups });
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

        let Some(compute_pipeline) =
            pipeline_cache.get_compute_pipeline(pipeline.simulate_pipeline)
        else {
            return Ok(());
        };

        let mut pass = render_context
            .command_encoder()
            .begin_compute_pass(&ComputePassDescriptor {
                label: Some("particle_compute_pass"),
                ..default()
            });

        pass.set_pipeline(compute_pipeline);

        for (entity, bind_group) in &bind_groups.bind_groups {
            if let Some(emitter_data) = extracted.emitters.iter().find(|(e, _)| e == entity) {
                let amount = emitter_data.1.uniforms.amount;
                let workgroups = (amount + WORKGROUP_SIZE - 1) / WORKGROUP_SIZE;

                pass.set_bind_group(0, bind_group, &[]);
                pass.dispatch_workgroups(workgroups, 1, 1);
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
