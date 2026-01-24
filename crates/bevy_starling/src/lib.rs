pub mod asset;
pub mod core;
pub mod render;
pub mod runtime;
pub mod systems;

use bevy::{
    asset::embedded_asset,
    pbr::MaterialPlugin,
    prelude::*,
    render::{extract_resource::ExtractResourcePlugin, ExtractSchedule, RenderApp},
};

use asset::{ParticleSystemAsset, ParticleSystemAssetLoader};
use render::{
    compute::ParticleComputePlugin,
    extract::extract_particle_systems,
    gradient_texture::{
        create_fallback_gradient_texture, prepare_gradient_textures, FallbackGradientTexture,
        GradientTextureCache,
    },
    sort::ParticleSortPlugin,
};
use systems::{
    cleanup_particle_entities, clear_particle_clear_requests, setup_particle_systems,
    sync_particle_mesh, update_particle_time,
};

pub struct StarlingPlugin;

impl Plugin for StarlingPlugin {
    fn build(&self, app: &mut App) {
        // embed shaders
        embedded_asset!(app, "shaders/particle_types.wgsl");
        embedded_asset!(app, "shaders/particle_simulate.wgsl");
        embedded_asset!(app, "shaders/particle_material.wgsl");
        embedded_asset!(app, "shaders/particle_sort.wgsl");

        // asset loading
        app.init_asset::<ParticleSystemAsset>()
            .init_asset_loader::<ParticleSystemAssetLoader>();

        // gradient texture caching
        app.init_resource::<GradientTextureCache>()
            .add_systems(Startup, create_fallback_gradient_texture)
            .add_systems(PostUpdate, prepare_gradient_textures);

        // register the extended material for particle rendering
        app.add_plugins(MaterialPlugin::<runtime::ParticleMaterial>::default());

        // main world systems
        app.add_systems(
            Update,
            (
                setup_particle_systems,
                sync_particle_mesh,
                update_particle_time,
                cleanup_particle_entities,
            ),
        );

        // clear the clear_requested flag at the start of each frame
        // flags set in the previous frame will have been extracted by the render app
        app.add_systems(First, clear_particle_clear_requests);

        // render plugins
        app.add_plugins((
            ParticleComputePlugin,
            ParticleSortPlugin,
            ExtractResourcePlugin::<FallbackGradientTexture>::default(),
        ));

        // extract systems
        if let Some(render_app) = app.get_sub_app_mut(RenderApp) {
            render_app.add_systems(ExtractSchedule, extract_particle_systems);
        }
    }
}

// re-exports for convenience
pub use asset::{
    DrawOrder, EmitterData, EmitterDrawPass, EmitterDrawing, EmitterTime, ParticleMesh,
    ParticleProcessConfig, ParticleSystemDimension,
};
pub use core::{ParticleData, ParticleSystem2D, ParticleSystem3D};
pub use render::material::ParticleMaterialExtension;
pub use runtime::{
    EmitterEntity, EmitterMeshEntity, EmitterRuntime, ParticleBufferHandle, ParticleMaterial,
    ParticleMaterialHandle, ParticleSystemRuntime,
};
