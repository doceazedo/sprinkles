pub mod asset;
mod compute;
mod extract;
pub mod material;
pub mod prelude;
pub mod runtime;
mod sort;
pub mod spawning;
pub mod textures;

use bevy::{
    asset::{embedded_asset, load_internal_asset, uuid_handle},
    pbr::MaterialPlugin,
    prelude::*,
    render::{ExtractSchedule, RenderApp, extract_resource::ExtractResourcePlugin},
};

const SHADER_COMMON: Handle<Shader> = uuid_handle!("10b6a301-2396-4ce0-906a-b3e38aaddddf");

use asset::{ParticleSystemAsset, ParticleSystemAssetLoader};
use compute::ParticleComputePlugin;
use extract::{extract_colliders, extract_particle_systems};
use sort::ParticleSortPlugin;
use spawning::{
    cleanup_particle_entities, setup_particle_systems, sync_collider_data,
    sync_emitter_mesh_transforms, sync_particle_material, sync_particle_mesh, update_particle_time,
};
use textures::{
    CurveTextureCache, FallbackCurveTexture, FallbackGradientTexture, GradientTextureCache,
    create_fallback_curve_texture, create_fallback_gradient_texture, prepare_curve_textures,
    prepare_gradient_textures,
};

pub struct SprinklesPlugin;

impl Plugin for SprinklesPlugin {
    fn build(&self, app: &mut App) {
        load_internal_asset!(app, SHADER_COMMON, "shaders/common.wgsl", Shader::from_wgsl);
        embedded_asset!(app, "shaders/particle_simulate.wgsl");
        embedded_asset!(app, "shaders/particle_material.wgsl");
        embedded_asset!(app, "shaders/particle_sort.wgsl");

        #[cfg(feature = "preset-textures")]
        textures::preset::register_preset_textures(app);

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
                sync_collider_data,
                update_particle_time,
                cleanup_particle_entities,
            ),
        );

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

pub use asset::{
    ColliderData, DrawOrder, DrawPassMaterial, EmitterAccelerations, EmitterCollision,
    EmitterCollisionMode, EmitterColors, EmitterData, EmitterDrawPass, EmitterEmission,
    EmitterScale, EmitterTime, EmitterTurbulence, EmitterVelocities, ParticleFlags, ParticleMesh,
    ParticleSystemDimension, ParticlesColliderShape3D, QuadOrientation, SerializableAlphaMode,
    StandardParticleMaterial, TransformAlign,
};
pub use material::ParticleMaterialExtension;
pub use runtime::{
    ColliderEntity, EmitterEntity, EmitterMeshEntity, EmitterRuntime, ParticleBufferHandle,
    ParticleData, ParticleMaterial, ParticleMaterialHandle, ParticleSystem2D, ParticleSystem3D,
    ParticleSystemRuntime, ParticlesCollider3D,
};
#[cfg(feature = "preset-textures")]
pub use textures::preset::PresetTexture;
pub use textures::preset::TextureRef;
