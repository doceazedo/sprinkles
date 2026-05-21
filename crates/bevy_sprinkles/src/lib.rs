#![deny(missing_docs)]
#![doc(
    html_logo_url = "https://raw.githubusercontent.com/doceazedo/sprinkles/refs/heads/main/assets/icon.svg"
)]
//! **Sprinkles** is a GPU-accelerated particle system for the
//! [Bevy game engine](https://bevyengine.org/), inspired by
//! [Godot's particle system](https://docs.godotengine.org/en/stable/tutorials/3d/particles/index.html).
//!
//! # Getting started
//!
//! ## Add the dependency
//!
//! First, add `bevy_sprinkles` to the dependencies in your `Cargo.toml`:
//!
//! ```toml
//! [dependencies]
//! bevy_sprinkles = "0.2"
//! ```
//!
//! ## Add the plugin
//!
//! Add [`SprinklesPlugin`] to your Bevy app:
//!
//! ```no_run
//! use bevy::prelude::*;
//! use bevy_sprinkles::prelude::*;
//!
//! fn main() {
//!     App::new()
//!         .add_plugins((DefaultPlugins, SprinklesPlugin))
//!         // ...your other plugins, systems and resources
//!         .run();
//! }
//! ```
//!
//! Now you can use all of Sprinkles' components and resources to build particle effects!
//!
//! ## Spawning a particle system
//!
//! A particle system is defined by a [`ParticlesAsset`] containing one or more
//! [`EmitterData`] entries. Spawn a [`Particles3d`] component to render the effect.
//!
//! ### Loading from a file
//!
//! Particle systems can be loaded from RON asset files:
//!
//! ```
//! use bevy::prelude::*;
//! use bevy_sprinkles::prelude::*;
//!
//! fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
//!     commands.spawn(Particles3d(asset_server.load("my_effect.ron")));
//! }
//! ```
//!
//! ### Building in code
//!
//! You can also build a [`ParticlesAsset`] directly:
//!
//! ```
//! use bevy::prelude::*;
//! use bevy_sprinkles::prelude::*;
//!
//! fn setup(mut commands: Commands, mut assets: ResMut<Assets<ParticlesAsset>>) {
//!     let handle = assets.add(ParticlesAsset::new(
//!         "My Effect".into(),
//!         ParticlesDimension::D3,
//!         Default::default(),
//!         vec![EmitterData {
//!             emission: EmitterEmission {
//!                 particles_amount: 32,
//!                 ..default()
//!             },
//!             velocities: EmitterVelocities {
//!                 initial_velocity: ParticleRange::new(1.0, 5.0),
//!                 spread: 90.0,
//!                 ..default()
//!             },
//!             ..default()
//!         }],
//!         vec![],
//!         false,
//!         Default::default(),
//!     ));
//!
//!     commands.spawn(Particles3d(handle));
//! }
//! ```
//!
//! # Feature flags
//!
//! - `preset-textures` - Bundles a library of built-in particle
//!   textures, see [`PresetTexture`] (enabled by default)
//!
//! # Table of contents
//!
//! ## Particle systems
//!
//! A particle system is the top-level container for one or more emitters and optional colliders.
//!
//! - [Spawning a system](Particles3d) with a handle to a [`ParticlesAsset`]
//! - [Playback control](ParticleSystemRuntime) (pause, resume, restart)
//! - [Per-emitter runtime state](EmitterRuntime)
//!
//! ## Emitters
//!
//! An [emitter](EmitterData) is the source that creates particles. It controls how, where,
//! and when particles are spawned, as well as their behavior over their lifetime.
//!
//! - [Timing](EmitterTime): controls when and how particles are spawned
//! - [Emission](EmitterEmission): particle count and
//!   [emission shape](asset::EmissionShape)
//! - [Rendering](EmitterDrawPass): [mesh](ParticleMesh),
//!   [material](DrawPassMaterial), [draw order](DrawOrder), and
//!   [transform alignment](TransformAlign)
//!
//! See [`EmitterData`] for the full list of emitter settings.
//!
//! ## Particle properties
//!
//! Each particle's appearance and motion can be configured and animated over its lifetime.
//!
//! - [Velocity](EmitterVelocities): particle speed and direction
//! - [Acceleration](EmitterAccelerations): constant forces applied to particles
//! - [Scale](EmitterScale): particle size over lifetime
//! - [Color](EmitterColors): particle color over lifetime
//! - [Turbulence](EmitterTurbulence): noise-based displacement
//!
//! See the [`asset`] module for more details about particle properties.
//!
//! ## Collision
//!
//! Particles can interact with [collider](ParticlesCollider3D) entities in the scene.
//!
//! - [Emitter settings](EmitterCollision): collision behavior on the emitter side
//! - [Collision mode](EmitterCollisionMode): how particles react to colliders
//! - [Collider shapes](ParticlesColliderShape3D): the collision surface geometry
//! - [Collider data](ColliderData): per-collider configuration
//!
//! ## Sub-emitters
//!
//! [Sub-emitters](asset::SubEmitterConfig) spawn secondary particles from parent particles,
//! enabling effects like fireworks, sparks on collision, or bubbles popping into water drops.
//!
//! - [Trigger modes](asset::SubEmitterMode): when sub-emitters activate
//! - [Configuration](asset::SubEmitterConfig): which emitter to spawn and how
//!
//! ## Textures
//!
//! Sprinkles bakes gradients and curves into GPU textures for efficient sampling in shaders.
//!
//! - [Gradient textures](asset::Gradient): color ramps baked into 1D images
//! - [Curve textures](asset::CurveTexture): value curves baked into 1D images
//! - [Preset textures](PresetTexture): built-in particle textures bundled with the crate
//!
//! See the [`textures::baked`] module for more details about texture baking and caching.

/// Particle system asset definitions, emitter data, and serialization types.
pub mod asset;
mod compute;
mod extract;
/// Particle material extension for GPU-driven particle rendering.
pub mod material;
mod mesh;
/// Convenience re-exports for common particle system types.
pub mod prelude;
/// Runtime components and state for active particle systems.
pub mod runtime;
mod sort;
mod spawning;
/// Texture baking and caching for gradients and curves.
pub mod textures;

use bevy::{
    asset::{embedded_asset, load_internal_asset, uuid_handle},
    pbr::MaterialPlugin,
    prelude::*,
    render::{ExtractSchedule, RenderApp, extract_resource::ExtractResourcePlugin},
};

const SHADER_COMMON: Handle<Shader> = uuid_handle!("10b6a301-2396-4ce0-906a-b3e38aaddddf");

use asset::{ParticlesAsset, ParticlesAssetLoader};
use compute::ParticleComputePlugin;
use extract::{extract_colliders, extract_particle_systems};
use mesh::ParticleMeshCache;
use runtime::check_particle_system_finished;
use sort::ParticleSortPlugin;
use spawning::{
    cleanup_particle_entities, setup_particle_systems, sync_collider_data, sync_particle_buffers,
    sync_particle_material, sync_particle_mesh, update_particle_time, write_emitter_uniforms,
};
use textures::{
    CurveTextureCache, FallbackCurveTexture, FallbackGradientTexture, GradientTextureCache,
    create_fallback_curve_texture, create_fallback_gradient_texture, prepare_curve_textures,
    prepare_gradient_textures,
};

/// Plugin that adds GPU particle system support to a Bevy app.
///
/// Registers asset loaders, compute pipelines, material plugins, texture caches,
/// and all the systems needed to simulate and render particles.
pub struct SprinklesPlugin;

impl Plugin for SprinklesPlugin {
    fn build(&self, app: &mut App) {
        load_internal_asset!(app, SHADER_COMMON, "shaders/common.wgsl", Shader::from_wgsl);
        embedded_asset!(app, "shaders/particle_simulate.wgsl");
        embedded_asset!(app, "shaders/particle_material.wgsl");
        embedded_asset!(app, "shaders/particle_sort.wgsl");

        #[cfg(feature = "preset-textures")]
        textures::preset::register_preset_textures(app);

        app.init_asset::<ParticlesAsset>()
            .init_asset_loader::<ParticlesAssetLoader>();

        app.init_resource::<GradientTextureCache>()
            .add_systems(Startup, create_fallback_gradient_texture)
            .add_systems(PostUpdate, prepare_gradient_textures);

        app.init_resource::<CurveTextureCache>()
            .add_systems(Startup, create_fallback_curve_texture)
            .add_systems(PostUpdate, prepare_curve_textures);

        app.init_resource::<ParticleMeshCache>();

        app.add_plugins(MaterialPlugin::<runtime::ParticleMaterial>::default());

        app.add_systems(
            Update,
            (
                setup_particle_systems,
                sync_particle_buffers.after(setup_particle_systems),
                sync_particle_mesh.after(sync_particle_buffers),
                sync_particle_material,
                sync_collider_data,
                update_particle_time,
                check_particle_system_finished.after(update_particle_time),
                cleanup_particle_entities,
            ),
        );

        app.add_systems(PostUpdate, write_emitter_uniforms);

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
    EmitterScale, EmitterTime, EmitterTrail, EmitterTurbulence, EmitterVelocities, ParticleFlags,
    ParticleMesh, ParticlesColliderShape3D, ParticlesDimension, QuadOrientation, RibbonTrailShape,
    SerializableAlphaMode, StandardParticleMaterial, TransformAlign,
};
pub use material::ParticleMaterialExtension;
pub use runtime::{
    ColliderEntity, EmitterEntity, EmitterRuntime, Finished, ParticleBufferHandle, ParticleData,
    ParticleMaterial, ParticleMaterialHandle, ParticleSystemRuntime, Particles2d, Particles3d,
    ParticlesCollider3D,
};
#[cfg(feature = "preset-textures")]
pub use textures::preset::PresetTexture;
pub use textures::preset::TextureRef;
