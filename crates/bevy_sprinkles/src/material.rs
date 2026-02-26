use bevy::{
    mesh::MeshVertexBufferLayoutRef,
    pbr::{MaterialExtension, MaterialExtensionKey, MaterialExtensionPipeline, MeshPipelineKey},
    prelude::*,
    render::{
        render_resource::{
            AsBindGroup, CompareFunction, RenderPipelineDescriptor, ShaderType,
            SpecializedMeshPipelineError,
        },
        storage::ShaderStorageBuffer,
    },
    shader::ShaderRef,
};

const SHADER_ASSET_PATH: &str = "embedded://bevy_sprinkles/shaders/particle_material.wgsl";

/// GPU-side per-emitter uniforms passed to the particle material shader.
#[derive(Clone, Copy, Default, ShaderType)]
pub struct ParticleEmitterUniforms {
    /// World-space transform matrix for the emitter.
    pub emitter_transform: Mat4,
    /// Maximum number of particles this emitter can hold.
    pub max_particles: u32,
    /// Particle behavior flags (see [`ParticleFlags`](crate::ParticleFlags)).
    pub particle_flags: u32,
    /// Whether particles are simulated in local coordinates (1) or world coordinates (0).
    pub use_local_coords: u32,
    #[doc(hidden)]
    pub _pad: u32,
}

/// A material extension that binds particle data buffers for GPU particle rendering.
///
/// This extension provides the sorted particle buffer and per-emitter uniforms
/// to the vertex shader so it can read per-particle state (position, color,
/// scale, etc.) and transform each instanced mesh accordingly.
#[derive(Asset, AsBindGroup, Reflect, Debug, Clone)]
pub struct ParticleMaterialExtension {
    /// Handle to the sorted particle data buffer, read by the vertex shader.
    #[storage(100, read_only)]
    pub sorted_particles: Handle<ShaderStorageBuffer>,
    /// Handle to the per-emitter uniforms buffer (transform, flags, etc.).
    #[storage(101, read_only)]
    pub emitter_uniforms: Handle<ShaderStorageBuffer>,
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
