use bevy::{
    pbr::MaterialExtension,
    prelude::*,
    render::{render_resource::AsBindGroup, storage::ShaderStorageBuffer},
    shader::ShaderRef,
};

const SHADER_ASSET_PATH: &str = "embedded://bevy_starling/shaders/particle_material.wgsl";

#[derive(Asset, AsBindGroup, Reflect, Debug, Clone)]
pub struct ParticleMaterialExtension {
    #[storage(100, read_only)]
    pub particles: Handle<ShaderStorageBuffer>,
    #[storage(101, read_only)]
    pub indices: Handle<ShaderStorageBuffer>,
}

impl MaterialExtension for ParticleMaterialExtension {
    fn vertex_shader() -> ShaderRef {
        SHADER_ASSET_PATH.into()
    }

    fn fragment_shader() -> ShaderRef {
        SHADER_ASSET_PATH.into()
    }
}
