use bevy::{
    mesh::MeshVertexBufferLayoutRef,
    pbr::{MaterialExtension, MaterialExtensionKey, MaterialExtensionPipeline, MeshPipelineKey},
    prelude::*,
    render::{
        render_resource::{
            AsBindGroup, CompareFunction, RenderPipelineDescriptor, SpecializedMeshPipelineError,
        },
        storage::ShaderStorageBuffer,
    },
    shader::ShaderRef,
};

const SHADER_ASSET_PATH: &str = "embedded://bevy_sprinkles/shaders/particle_material.wgsl";

#[derive(Asset, AsBindGroup, Reflect, Debug, Clone)]
pub struct ParticleMaterialExtension {
    #[storage(100, read_only)]
    pub sorted_particles: Handle<ShaderStorageBuffer>,
    #[uniform(101)]
    pub max_particles: u32,
    #[uniform(102)]
    pub particle_flags: u32,
}

impl MaterialExtension for ParticleMaterialExtension {
    fn vertex_shader() -> ShaderRef {
        SHADER_ASSET_PATH.into()
    }

    fn fragment_shader() -> ShaderRef {
        SHADER_ASSET_PATH.into()
    }

    fn prepass_vertex_shader() -> ShaderRef {
        SHADER_ASSET_PATH.into()
    }

    fn prepass_fragment_shader() -> ShaderRef {
        SHADER_ASSET_PATH.into()
    }

    fn specialize(
        _pipeline: &MaterialExtensionPipeline,
        descriptor: &mut RenderPipelineDescriptor,
        _layout: &MeshVertexBufferLayoutRef,
        key: MaterialExtensionKey<Self>,
    ) -> Result<(), SpecializedMeshPipelineError> {
        let is_transparent = key.mesh_key.contains(MeshPipelineKey::BLEND_ALPHA)
            || key
                .mesh_key
                .contains(MeshPipelineKey::BLEND_PREMULTIPLIED_ALPHA)
            || key.mesh_key.contains(MeshPipelineKey::BLEND_MULTIPLY)
            || key
                .mesh_key
                .contains(MeshPipelineKey::BLEND_ALPHA_TO_COVERAGE);

        if let Some(depth_stencil) = &mut descriptor.depth_stencil {
            depth_stencil.depth_write_enabled = !is_transparent;
            depth_stencil.depth_compare = CompareFunction::GreaterEqual;
        }

        Ok(())
    }
}
