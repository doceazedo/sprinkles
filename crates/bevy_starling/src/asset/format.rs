use bevy::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum ParticleSystemDimension {
    #[default]
    D3,
    D2,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum DrawOrder {
    #[default]
    Index,
    Lifetime,
    ReverseLifetime,
    ViewDepth,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmitterData {
    pub name: String,
    #[serde(default = "default_enabled")]
    pub enabled: bool,

    // emission
    #[serde(default = "default_amount")]
    pub amount: u32,
    #[serde(default = "default_lifetime")]
    pub lifetime: f32,
    #[serde(default)]
    pub lifetime_randomness: f32,
    #[serde(default)]
    pub one_shot: bool,
    #[serde(default)]
    pub explosiveness: f32,
    #[serde(default)]
    pub randomness: f32,

    // timing
    #[serde(default = "default_fixed_fps")]
    pub fixed_fps: u32,

    // draw
    #[serde(default)]
    pub draw_order: DrawOrder,
    #[serde(default)]
    pub draw_passes: Vec<DrawPassConfig>,

    // process
    #[serde(default)]
    pub process: ParticleProcessConfig,
}

fn default_enabled() -> bool {
    true
}

fn default_amount() -> u32 {
    8
}

fn default_lifetime() -> f32 {
    1.0
}

fn default_fixed_fps() -> u32 {
    0
}

impl Default for EmitterData {
    fn default() -> Self {
        Self {
            name: "Emitter".to_string(),
            enabled: true,
            amount: 8,
            lifetime: 1.0,
            lifetime_randomness: 0.0,
            one_shot: false,
            explosiveness: 0.0,
            randomness: 0.0,
            fixed_fps: 0,
            draw_order: DrawOrder::Index,
            draw_passes: vec![DrawPassConfig::default()],
            process: ParticleProcessConfig::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DrawPassConfig {
    pub mesh: ParticleMesh,
}

impl Default for DrawPassConfig {
    fn default() -> Self {
        Self {
            mesh: ParticleMesh::Quad,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum ParticleMesh {
    #[default]
    Quad,
    Sphere {
        radius: f32,
        #[serde(default = "default_sphere_rings")]
        rings: u32,
        #[serde(default = "default_sphere_sectors")]
        sectors: u32,
    },
    Cube {
        size: f32,
    },
}

fn default_sphere_rings() -> u32 {
    16
}

fn default_sphere_sectors() -> u32 {
    32
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParticleProcessConfig {
    #[serde(default)]
    pub gravity: Vec3,
    #[serde(default)]
    pub initial_velocity: Vec3,
    #[serde(default)]
    pub initial_velocity_randomness: Vec3,
    #[serde(default = "default_initial_scale")]
    pub initial_scale: f32,
    #[serde(default)]
    pub initial_scale_randomness: f32,
}

fn default_initial_scale() -> f32 {
    1.0
}

impl Default for ParticleProcessConfig {
    fn default() -> Self {
        Self {
            gravity: Vec3::ZERO,
            initial_velocity: Vec3::ZERO,
            initial_velocity_randomness: Vec3::ZERO,
            initial_scale: 1.0,
            initial_scale_randomness: 0.0,
        }
    }
}

#[derive(Asset, TypePath, Debug, Serialize, Deserialize)]
pub struct ParticleSystemAsset {
    pub name: String,
    pub dimension: ParticleSystemDimension,
    pub emitters: Vec<EmitterData>,
}
