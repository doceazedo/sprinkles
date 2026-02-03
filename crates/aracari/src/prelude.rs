//! Prelude module for convenient imports.
//!
//! ```rust,ignore
//! use aracari::prelude::*;
//! ```

// core plugin
pub use crate::AracariPlugin;

// asset types
pub use crate::asset::{
    AnimatedVelocity, DrawOrder, DrawPassMaterial, EmissionShape, EmitterData, EmitterDrawPass,
    EmitterTime, Gradient as ParticleGradient, GradientInterpolation, GradientStop, ParticleFlags,
    ParticleMesh, ParticleProcessAccelerations, ParticleProcessAnimVelocities,
    ParticleProcessCollision, ParticleProcessCollisionMode, ParticleProcessConfig,
    ParticleProcessDisplay, ParticleProcessDisplayColor, ParticleProcessDisplayScale,
    ParticleProcessSpawn, ParticleProcessSpawnPosition, ParticleProcessSpawnVelocity,
    ParticleProcessTurbulence, ParticleSystemAsset, ParticleSystemDimension,
    ParticlesColliderShape3D, QuadOrientation, Range as ParticleRange, SerializableAlphaMode,
    SolidOrGradientColor, SplineCurve, SplineCurveConfig, StandardParticleMaterial,
};

// runtime types
pub use crate::runtime::{
    EmitterEntity, EmitterRuntime, ParticleMaterial, ParticleMaterialHandle, ParticleSystem2D,
    ParticleSystem3D, ParticleSystemRuntime, ParticlesCollider3D,
};
