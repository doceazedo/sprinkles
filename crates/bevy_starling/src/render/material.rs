use bevy::{
    mesh::MeshVertexBufferLayoutRef,
    pbr::{MaterialExtension, MaterialExtensionKey, MaterialExtensionPipeline},
    prelude::*,
    render::{
        render_resource::{
            AsBindGroup, BlendState, CompareFunction, RenderPipelineDescriptor,
            SpecializedMeshPipelineError,
        },
        storage::ShaderStorageBuffer,
    },
    shader::ShaderRef,
};

const SHADER_ASSET_PATH: &str = "embedded://bevy_starling/shaders/particle_material.wgsl";

#[derive(Asset, AsBindGroup, Reflect, Debug, Clone)]
pub struct ParticleMaterialExtension {
    /// sorted particle data buffer (written in draw order by the sort compute shader)
    #[storage(100, read_only)]
    pub sorted_particles: Handle<ShaderStorageBuffer>,
    /// maximum number of particles
    #[uniform(101)]
    pub max_particles: u32,
}

impl MaterialExtension for ParticleMaterialExtension {
    fn vertex_shader() -> ShaderRef {
        SHADER_ASSET_PATH.into()
    }

    fn fragment_shader() -> ShaderRef {
        SHADER_ASSET_PATH.into()
    }

    fn specialize(
        _pipeline: &MaterialExtensionPipeline,
        descriptor: &mut RenderPipelineDescriptor,
        _layout: &MeshVertexBufferLayoutRef,
        _key: MaterialExtensionKey<Self>,
    ) -> Result<(), SpecializedMeshPipelineError> {
        // enable depth writing for proper 3D depth testing between particles.
        // this allows particles from different emitters to correctly occlude
        // each other based on their actual world-space depth, matching Godot's
        // default behavior. the draw_order sorting within each emitter still
        // controls submission order for alpha blending correctness.
        if let Some(depth_stencil) = &mut descriptor.depth_stencil {
            depth_stencil.depth_write_enabled = true;
            depth_stencil.depth_compare = CompareFunction::GreaterEqual;
        }

        // enable alpha blending for particle transparency
        if let Some(fragment) = &mut descriptor.fragment {
            for target in &mut fragment.targets {
                if let Some(target) = target {
                    target.blend = Some(BlendState::ALPHA_BLENDING);
                }
            }
        }

        Ok(())
    }
}
