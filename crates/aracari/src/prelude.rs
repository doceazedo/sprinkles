//! Prelude module for convenient imports.
//!
//! ```rust,ignore
//! use aracari::prelude::*;
//! ```

// core plugin
pub use crate::AracariPlugin;

// asset types
pub use crate::asset::{
    AnimatedVelocity, ColliderData, DrawOrder, DrawPassMaterial, EmissionShape,
    EmitterAccelerations, EmitterCollision, EmitterCollisionMode, EmitterColors, EmitterData,
    EmitterDrawPass, EmitterEmission, EmitterScale, EmitterTime, EmitterTurbulence,
    EmitterVelocities, Gradient as ParticleGradient, GradientInterpolation, GradientStop,
    ParticleFlags, ParticleMesh, ParticleSystemAsset, ParticleSystemDimension,
    ParticlesColliderShape3D, QuadOrientation, Range as ParticleRange, SerializableAlphaMode,
    SolidOrGradientColor, CurveEasing, CurveMode, CurvePoint, CurveTexture,
    StandardParticleMaterial,
};

// runtime types
pub use crate::runtime::{
    ColliderEntity, EmitterEntity, EmitterRuntime, ParticleMaterial, ParticleMaterialHandle,
    ParticleSystem2D, ParticleSystem3D, ParticleSystemRuntime, ParticlesCollider3D,
};
