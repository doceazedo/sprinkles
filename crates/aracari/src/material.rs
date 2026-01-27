use bevy::{
    mesh::MeshVertexBufferLayoutRef,
    pbr::{MaterialExtension, MaterialExtensionKey, MaterialExtensionPipeline, MeshPipelineKey},
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

const SHADER_ASSET_PATH: &str = "embedded://aracari/shaders/particle_material.wgsl";

#[derive(Asset, AsBindGroup, Reflect, Debug, Clone)]
pub struct ParticleMaterialExtension {
    /// sorted particle data buffer (written in draw order by the sort compute shader)
    #[storage(100, read_only)]
    pub sorted_particles: Handle<ShaderStorageBuffer>,
    /// maximum number of particles
    #[uniform(101)]
    pub max_particles: u32,
    /// particle flags (emitter-level flags that affect all particles)
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
        // check if material uses a transparent blend mode
        let is_transparent = key.mesh_key.contains(MeshPipelineKey::BLEND_ALPHA)
            || key.mesh_key.contains(MeshPipelineKey::BLEND_PREMULTIPLIED_ALPHA)
            || key.mesh_key.contains(MeshPipelineKey::BLEND_MULTIPLY)
            || key.mesh_key.contains(MeshPipelineKey::BLEND_ALPHA_TO_COVERAGE);

        if let Some(depth_stencil) = &mut descriptor.depth_stencil {
            // opaque particles write to depth for proper 3D occlusion within emitter.
            // transparent particles skip depth write for cross-emitter transparency.
            depth_stencil.depth_write_enabled = !is_transparent;
            depth_stencil.depth_compare = CompareFunction::GreaterEqual;
        }

        // only enable alpha blending for transparent materials
        if is_transparent {
            if let Some(fragment) = &mut descriptor.fragment {
                for target in &mut fragment.targets {
                    if let Some(target) = target {
                        target.blend = Some(BlendState::ALPHA_BLENDING);
                    }
                }
            }
        }

        Ok(())
    }
}
