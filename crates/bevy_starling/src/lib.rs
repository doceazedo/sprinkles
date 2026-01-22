pub mod asset;
pub mod core;
pub mod render;
pub mod runtime;
pub mod systems;

use bevy::{
    asset::embedded_asset,
    pbr::MaterialPlugin,
    prelude::*,
    render::{ExtractSchedule, RenderApp},
};

use asset::{ParticleSystemAsset, ParticleSystemAssetLoader};
use render::{
    compute::ParticleComputePlugin,
    extract::extract_particle_systems,
    sort::ParticleSortPlugin,
};
use systems::{cleanup_particle_entities, setup_particle_systems, update_particle_time, ParticleMaterial};

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

        // register the extended material for particle rendering
        app.add_plugins(MaterialPlugin::<ParticleMaterial>::default());

        // main world systems
        app.add_systems(
            Update,
            (setup_particle_systems, update_particle_time, cleanup_particle_entities),
        );

        // render plugins
        app.add_plugins((ParticleComputePlugin, ParticleSortPlugin));

        // extract systems
        if let Some(render_app) = app.get_sub_app_mut(RenderApp) {
            render_app.add_systems(ExtractSchedule, extract_particle_systems);
        }
    }
}

// re-exports for convenience
pub use asset::{
    DrawOrder, DrawPassConfig, EmitterData, ParticleMesh, ParticleProcessConfig,
    ParticleSystemDimension,
};
pub use core::{ParticleData, ParticleSystem2D, ParticleSystem3D};
pub use render::material::ParticleMaterialExtension;
pub use runtime::{ParticleBufferHandle, ParticleEntity, ParticleSystemRef, ParticleSystemRuntime};
