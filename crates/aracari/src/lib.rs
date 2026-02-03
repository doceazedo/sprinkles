pub mod asset;
mod compute;
mod extract;
pub mod material;
pub mod prelude;
pub mod runtime;
mod sort;
mod spawning;
pub mod textures;

use bevy::{
    asset::embedded_asset,
    pbr::MaterialPlugin,
    prelude::*,
    render::{extract_resource::ExtractResourcePlugin, ExtractSchedule, RenderApp},
};

use asset::{ParticleSystemAsset, ParticleSystemAssetLoader};
use compute::ParticleComputePlugin;
use extract::{extract_colliders, extract_particle_systems};
use sort::ParticleSortPlugin;
use spawning::{
    cleanup_particle_entities, clear_particle_clear_requests, setup_particle_systems,
    sync_emitter_mesh_transforms, sync_particle_material, sync_particle_mesh, update_particle_time,
};
use textures::{
    create_fallback_curve_texture, create_fallback_gradient_texture, prepare_curve_textures,
    prepare_gradient_textures, CurveTextureCache, FallbackCurveTexture, FallbackGradientTexture,
    GradientTextureCache,
};

pub struct AracariPlugin;

impl Plugin for AracariPlugin {
    fn build(&self, app: &mut App) {
        embedded_asset!(app, "shaders/particle_types.wgsl");
        embedded_asset!(app, "shaders/particle_simulate.wgsl");
        embedded_asset!(app, "shaders/particle_material.wgsl");
        embedded_asset!(app, "shaders/particle_sort.wgsl");

        app.init_asset::<ParticleSystemAsset>()
            .init_asset_loader::<ParticleSystemAssetLoader>();

        app.init_resource::<GradientTextureCache>()
            .add_systems(Startup, create_fallback_gradient_texture)
            .add_systems(PostUpdate, prepare_gradient_textures);

        app.init_resource::<CurveTextureCache>()
            .add_systems(Startup, create_fallback_curve_texture)
            .add_systems(PostUpdate, prepare_curve_textures);

        app.add_plugins(MaterialPlugin::<runtime::ParticleMaterial>::default());

        app.add_systems(
            Update,
            (
                setup_particle_systems,
                sync_particle_mesh,
                sync_particle_material,
                sync_emitter_mesh_transforms,
                update_particle_time,
                cleanup_particle_entities,
            ),
        );

        app.add_systems(First, clear_particle_clear_requests);

        app.add_plugins((
            ParticleComputePlugin,
            ParticleSortPlugin,
            ExtractResourcePlugin::<FallbackGradientTexture>::default(),
            ExtractResourcePlugin::<FallbackCurveTexture>::default(),
        ));

        if let Some(render_app) = app.get_sub_app_mut(RenderApp) {
            render_app.add_systems(
                ExtractSchedule,
                (extract_particle_systems, extract_colliders),
            );
        }
    }
}

// re-exports
pub use asset::{
    DrawOrder, DrawPassMaterial, EmitterData, EmitterDrawPass, EmitterTime, ParticleFlags,
    ParticleMesh, ParticleProcessCollision, ParticleProcessCollisionMode, ParticleProcessConfig,
    ParticleSystemDimension, ParticlesColliderShape3D, QuadOrientation, SerializableAlphaMode,
    StandardParticleMaterial,
};
pub use material::ParticleMaterialExtension;
pub use runtime::{
    EmitterEntity, EmitterMeshEntity, EmitterRuntime, ParticleBufferHandle, ParticleData,
    ParticleMaterial, ParticleMaterialHandle, ParticleSystem2D, ParticleSystem3D,
    ParticleSystemRuntime, ParticlesCollider3D,
};
