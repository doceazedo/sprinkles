use bevy::prelude::*;
use serde::Deserialize;

use super::super::curve::CurvePoint;
use super::super::{
    AnimatedVelocity, ColliderData as CurrentColliderData, Curve, CurveTexture,
    EmitterAccelerations, EmitterAngle as CurrentEmitterAngle, EmitterCollision,
    EmitterColors as CurrentEmitterColors, EmitterData as CurrentEmitterData, EmitterDrawPass,
    EmitterEmission, EmitterScale as CurrentEmitterScale, EmitterTime,
    EmitterTrail as CurrentEmitterTrail, EmitterTurbulence as CurrentEmitterTurbulence,
    EmitterVelocities as CurrentEmitterVelocities, Gradient, InitialTransform, ParticleFlags,
    ParticleSystemAsset as CurrentParticleSystemAsset,
    ParticleSystemAuthors as CurrentParticleSystemAuthors, ParticleSystemDimension,
    ParticlesColliderShape3D, Range, SolidOrGradientColor, SprinklesEditorData, SubEmitterConfig,
};

#[derive(Debug, Clone, Deserialize)]
struct OldCurveTexture {
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    points: Vec<CurvePoint>,
    #[serde(default)]
    range: Range,
}

impl From<OldCurveTexture> for CurveTexture {
    fn from(old: OldCurveTexture) -> Self {
        Self {
            name: old.name,
            x: Curve {
                points: old.points,
                range: old.range,
            },
            y: None,
            z: None,
        }
    }
}

fn migrate_curve(old: Option<OldCurveTexture>) -> Option<CurveTexture> {
    old.map(Into::into)
}

#[derive(Debug, Clone, Deserialize)]
struct ParticleSystemAuthors {
    #[serde(default)]
    inspired_by: Option<String>,
    submitted_by: String,
}

impl From<ParticleSystemAuthors> for CurrentParticleSystemAuthors {
    fn from(old: ParticleSystemAuthors) -> Self {
        Self {
            inspired_by: old.inspired_by.unwrap_or_default(),
            submitted_by: old.submitted_by,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub(super) struct ParticleSystemAsset {
    #[allow(dead_code)]
    sprinkles_version: String,
    name: String,
    dimension: ParticleSystemDimension,
    #[serde(default)]
    initial_transform: InitialTransform,
    emitters: Vec<EmitterData>,
    #[serde(default)]
    colliders: Vec<ColliderData>,
    #[serde(default)]
    despawn_on_finish: bool,
    #[serde(default)]
    authors: Option<ParticleSystemAuthors>,
    #[serde(default)]
    sprinkles_editor: SprinklesEditorData,
}

#[derive(Debug, Clone, Deserialize)]
pub(super) struct EmitterData {
    name: String,
    #[serde(default = "default_enabled")]
    enabled: bool,
    #[serde(default)]
    position: Vec3,
    #[serde(default)]
    time: EmitterTime,
    #[serde(default)]
    draw_pass: EmitterDrawPass,
    #[serde(default)]
    emission: EmitterEmission,
    #[serde(default)]
    scale: EmitterScale,
    #[serde(default)]
    angle: EmitterAngle,
    #[serde(default)]
    colors: EmitterColors,
    #[serde(default)]
    velocities: EmitterVelocities,
    #[serde(default)]
    accelerations: EmitterAccelerations,
    #[serde(default)]
    turbulence: EmitterTurbulence,
    #[serde(default)]
    collision: EmitterCollision,
    #[serde(default)]
    sub_emitter: Option<SubEmitterConfig>,
    #[serde(default)]
    trail: EmitterTrail,
    #[serde(default)]
    particle_flags: ParticleFlags,
}

fn default_enabled() -> bool {
    true
}

fn default_scale_range() -> Range {
    Range { min: 1.0, max: 1.0 }
}

#[derive(Debug, Clone, Deserialize)]
struct EmitterScale {
    #[serde(default = "default_scale_range")]
    range: Range,
    #[serde(default)]
    scale_over_lifetime: Option<OldCurveTexture>,
}

impl Default for EmitterScale {
    fn default() -> Self {
        Self {
            range: default_scale_range(),
            scale_over_lifetime: None,
        }
    }
}

impl From<EmitterScale> for CurrentEmitterScale {
    fn from(old: EmitterScale) -> Self {
        Self {
            range: old.range,
            scale_over_lifetime: migrate_curve(old.scale_over_lifetime),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
struct EmitterColors {
    #[serde(default)]
    initial_color: SolidOrGradientColor,
    #[serde(default = "Gradient::white")]
    color_over_lifetime: Gradient,
    #[serde(default)]
    alpha_over_lifetime: Option<OldCurveTexture>,
    #[serde(default)]
    emission_over_lifetime: Option<OldCurveTexture>,
}

impl Default for EmitterColors {
    fn default() -> Self {
        Self {
            initial_color: SolidOrGradientColor::default(),
            color_over_lifetime: Gradient::white(),
            alpha_over_lifetime: None,
            emission_over_lifetime: None,
        }
    }
}

impl From<EmitterColors> for CurrentEmitterColors {
    fn from(old: EmitterColors) -> Self {
        Self {
            initial_color: old.initial_color,
            color_over_lifetime: old.color_over_lifetime,
            alpha_over_lifetime: migrate_curve(old.alpha_over_lifetime),
            emission_over_lifetime: migrate_curve(old.emission_over_lifetime),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
struct OldAnimatedVelocity {
    #[serde(default = "Range::zero")]
    velocity: Range,
    #[serde(default)]
    velocity_over_lifetime: Option<OldCurveTexture>,
}

impl Default for OldAnimatedVelocity {
    fn default() -> Self {
        Self {
            velocity: Range::zero(),
            velocity_over_lifetime: None,
        }
    }
}

impl From<OldAnimatedVelocity> for AnimatedVelocity {
    fn from(old: OldAnimatedVelocity) -> Self {
        Self {
            velocity: old.velocity,
            velocity_over_lifetime: migrate_curve(old.velocity_over_lifetime),
        }
    }
}

fn default_direction() -> Vec3 {
    Vec3::X
}

fn default_spread() -> f32 {
    45.0
}

#[derive(Debug, Clone, Deserialize)]
struct EmitterVelocities {
    #[serde(default = "default_direction")]
    initial_direction: Vec3,
    #[serde(default = "default_spread")]
    spread: f32,
    #[serde(default)]
    flatness: f32,
    #[serde(default = "Range::zero")]
    initial_velocity: Range,
    #[serde(default)]
    radial_velocity: OldAnimatedVelocity,
    #[serde(default)]
    angular_velocity: OldAnimatedVelocity,
    #[serde(default)]
    orbit_velocity: OldAnimatedVelocity,
    #[serde(default)]
    directional_velocity: OldAnimatedVelocity,
    #[serde(default)]
    pivot: Vec3,
    #[serde(default)]
    inherit_ratio: f32,
}

impl Default for EmitterVelocities {
    fn default() -> Self {
        Self {
            initial_direction: Vec3::X,
            spread: 45.0,
            flatness: 0.0,
            initial_velocity: Range::zero(),
            radial_velocity: OldAnimatedVelocity::default(),
            angular_velocity: OldAnimatedVelocity::default(),
            orbit_velocity: OldAnimatedVelocity::default(),
            directional_velocity: OldAnimatedVelocity::default(),
            pivot: Vec3::ZERO,
            inherit_ratio: 0.0,
        }
    }
}

impl From<EmitterVelocities> for CurrentEmitterVelocities {
    fn from(old: EmitterVelocities) -> Self {
        Self {
            initial_direction: old.initial_direction,
            spread: old.spread,
            flatness: old.flatness,
            initial_velocity: old.initial_velocity,
            radial_velocity: old.radial_velocity.into(),
            angular_velocity: old.angular_velocity.into(),
            orbit_velocity: old.orbit_velocity.into(),
            directional_velocity: old.directional_velocity.into(),
            pivot: old.pivot,
            inherit_ratio: old.inherit_ratio,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
struct EmitterAngle {
    #[serde(default = "Range::zero")]
    range: Range,
    #[serde(default)]
    angle_over_lifetime: Option<OldCurveTexture>,
}

impl Default for EmitterAngle {
    fn default() -> Self {
        Self {
            range: Range::zero(),
            angle_over_lifetime: None,
        }
    }
}

impl From<EmitterAngle> for CurrentEmitterAngle {
    fn from(old: EmitterAngle) -> Self {
        Self {
            range: old.range,
            angle_over_lifetime: migrate_curve(old.angle_over_lifetime),
        }
    }
}

fn default_turbulence_noise_strength() -> f32 {
    1.0
}

fn default_turbulence_noise_scale() -> f32 {
    2.5
}

fn default_turbulence_influence() -> Range {
    Range { min: 0.0, max: 0.1 }
}

#[derive(Debug, Clone, Deserialize)]
struct EmitterTurbulence {
    #[serde(default)]
    enabled: bool,
    #[serde(default = "default_turbulence_noise_strength")]
    noise_strength: f32,
    #[serde(default = "default_turbulence_noise_scale")]
    noise_scale: f32,
    #[serde(default)]
    noise_speed: Vec3,
    #[serde(default)]
    noise_speed_random: f32,
    #[serde(default = "default_turbulence_influence")]
    influence: Range,
    #[serde(default)]
    influence_over_lifetime: Option<OldCurveTexture>,
}

impl Default for EmitterTurbulence {
    fn default() -> Self {
        Self {
            enabled: false,
            noise_strength: default_turbulence_noise_strength(),
            noise_scale: default_turbulence_noise_scale(),
            noise_speed: Vec3::ZERO,
            noise_speed_random: 0.0,
            influence: default_turbulence_influence(),
            influence_over_lifetime: None,
        }
    }
}

impl From<EmitterTurbulence> for CurrentEmitterTurbulence {
    fn from(old: EmitterTurbulence) -> Self {
        Self {
            enabled: old.enabled,
            noise_strength: old.noise_strength,
            noise_scale: old.noise_scale,
            noise_speed: old.noise_speed,
            noise_speed_random: old.noise_speed_random,
            influence: old.influence,
            influence_over_lifetime: migrate_curve(old.influence_over_lifetime),
        }
    }
}

fn default_trail_stretch_time() -> f32 {
    0.3
}

#[derive(Debug, Clone, Deserialize)]
struct EmitterTrail {
    #[serde(default)]
    enabled: bool,
    #[serde(default = "default_trail_stretch_time")]
    stretch_time: f32,
    #[serde(default)]
    thickness_curve: Option<OldCurveTexture>,
}

impl Default for EmitterTrail {
    fn default() -> Self {
        Self {
            enabled: false,
            stretch_time: default_trail_stretch_time(),
            thickness_curve: None,
        }
    }
}

impl From<EmitterTrail> for CurrentEmitterTrail {
    fn from(old: EmitterTrail) -> Self {
        Self {
            enabled: old.enabled,
            stretch_time: old.stretch_time,
            thickness_curve: migrate_curve(old.thickness_curve),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub(super) struct ColliderData {
    name: String,
    #[serde(default = "default_enabled")]
    enabled: bool,
    shape: ParticlesColliderShape3D,
    #[serde(default)]
    position: Vec3,
}

impl From<ParticleSystemAsset> for CurrentParticleSystemAsset {
    fn from(old: ParticleSystemAsset) -> Self {
        let authors = old.authors.map(Into::into).unwrap_or_default();
        let mut asset = CurrentParticleSystemAsset::new(
            old.name,
            old.dimension,
            old.initial_transform,
            old.emitters.into_iter().map(Into::into).collect(),
            old.colliders.into_iter().map(Into::into).collect(),
            old.despawn_on_finish,
            authors,
        );
        asset.sprinkles_editor = old.sprinkles_editor;
        asset
    }
}

fn migrate_position(position: Vec3) -> InitialTransform {
    InitialTransform {
        translation: position,
        ..Default::default()
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
            scale: old.scale.into(),
            angle: old.angle.into(),
            colors: old.colors.into(),
            velocities: old.velocities.into(),
            accelerations: old.accelerations,
            turbulence: old.turbulence.into(),
            collision: old.collision,
            sub_emitter: old.sub_emitter,
            trail: old.trail.into(),
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
