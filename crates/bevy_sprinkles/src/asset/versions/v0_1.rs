use bevy::prelude::*;
use serde::Deserialize;

use super::super::{
    ColliderData as CurrentColliderData, EmitterAccelerations, EmitterAngle, EmitterCollision,
    EmitterColors, EmitterData as CurrentEmitterData, EmitterDrawPass, EmitterEmission,
    EmitterScale, EmitterTime, EmitterTrail, EmitterTurbulence, EmitterVelocities,
    InitialTransform, ParticleFlags, ParticleSystemAsset as CurrentParticleSystemAsset,
    ParticleSystemAuthors, ParticleSystemDimension, ParticlesColliderShape3D, SprinklesEditorData,
    SubEmitterConfig,
};

#[derive(Debug, Clone, Deserialize)]
pub(super) struct ParticleSystemAsset {
    #[allow(dead_code)]
    sprinkles_version: String,
    pub name: String,
    pub dimension: ParticleSystemDimension,
    #[serde(default)]
    pub initial_transform: InitialTransform,
    pub emitters: Vec<EmitterData>,
    #[serde(default)]
    pub colliders: Vec<ColliderData>,
    #[serde(default)]
    pub despawn_on_finish: bool,
    #[serde(default)]
    pub authors: ParticleSystemAuthors,
    #[serde(default)]
    pub sprinkles_editor: SprinklesEditorData,
}

#[derive(Debug, Clone, Deserialize)]
pub(super) struct EmitterData {
    pub name: String,
    #[serde(default = "default_enabled")]
    pub enabled: bool,
    #[serde(default)]
    pub position: Vec3,
    #[serde(default)]
    pub time: EmitterTime,
    #[serde(default)]
    pub draw_pass: EmitterDrawPass,
    #[serde(default)]
    pub emission: EmitterEmission,
    #[serde(default)]
    pub scale: EmitterScale,
    #[serde(default)]
    pub angle: EmitterAngle,
    #[serde(default)]
    pub colors: EmitterColors,
    #[serde(default)]
    pub velocities: EmitterVelocities,
    #[serde(default)]
    pub accelerations: EmitterAccelerations,
    #[serde(default)]
    pub turbulence: EmitterTurbulence,
    #[serde(default)]
    pub collision: EmitterCollision,
    #[serde(default)]
    pub sub_emitter: Option<SubEmitterConfig>,
    #[serde(default)]
    pub trail: EmitterTrail,
    #[serde(default)]
    pub particle_flags: ParticleFlags,
}

fn default_enabled() -> bool {
    true
}

#[derive(Debug, Clone, Deserialize)]
pub(super) struct ColliderData {
    pub name: String,
    #[serde(default = "default_enabled")]
    pub enabled: bool,
    pub shape: ParticlesColliderShape3D,
    #[serde(default)]
    pub position: Vec3,
}

impl From<ParticleSystemAsset> for CurrentParticleSystemAsset {
    fn from(old: ParticleSystemAsset) -> Self {
        let mut asset = CurrentParticleSystemAsset::new(
            old.name,
            old.dimension,
            old.initial_transform,
            old.emitters.into_iter().map(Into::into).collect(),
            old.colliders.into_iter().map(Into::into).collect(),
            old.despawn_on_finish,
            old.authors,
        );
        asset.sprinkles_editor = old.sprinkles_editor;
        asset
    }
}

fn migrate_position(position: Vec3) -> InitialTransform {
    if position != Vec3::ZERO {
        InitialTransform {
            translation: position,
            ..Default::default()
        }
    } else {
        InitialTransform::default()
    }
}

impl From<EmitterData> for CurrentEmitterData {
    fn from(old: EmitterData) -> Self {
        Self {
            name: old.name,
            enabled: old.enabled,
            initial_transform: migrate_position(old.position),
            time: old.time,
            draw_pass: old.draw_pass,
            emission: old.emission,
            scale: old.scale,
            angle: old.angle,
            colors: old.colors,
            velocities: old.velocities,
            accelerations: old.accelerations,
            turbulence: old.turbulence,
            collision: old.collision,
            sub_emitter: old.sub_emitter,
            trail: old.trail,
            particle_flags: old.particle_flags,
        }
    }
}

impl From<ColliderData> for CurrentColliderData {
    fn from(old: ColliderData) -> Self {
        Self {
            name: old.name,
            enabled: old.enabled,
            shape: old.shape,
            initial_transform: migrate_position(old.position),
        }
    }
}
