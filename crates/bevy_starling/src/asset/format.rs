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
pub struct EmitterTime {
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
    #[serde(default)]
    pub fixed_fps: u32,
}

fn default_lifetime() -> f32 {
    1.0
}

impl Default for EmitterTime {
    fn default() -> Self {
        Self {
            lifetime: 1.0,
            lifetime_randomness: 0.0,
            one_shot: false,
            explosiveness: 0.0,
            randomness: 0.0,
            fixed_fps: 0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmitterDrawing {
    #[serde(default)]
    pub draw_order: DrawOrder,
}

impl Default for EmitterDrawing {
    fn default() -> Self {
        Self {
            draw_order: DrawOrder::Index,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmitterData {
    pub name: String,
    #[serde(default = "default_enabled")]
    pub enabled: bool,

    #[serde(default = "default_amount")]
    pub amount: u32,

    #[serde(default)]
    pub time: EmitterTime,

    #[serde(default)]
    pub drawing: EmitterDrawing,

    #[serde(default)]
    pub draw_passes: Vec<EmitterDrawPass>,

    #[serde(default)]
    pub process: ParticleProcessConfig,
}

fn default_enabled() -> bool {
    true
}

fn default_amount() -> u32 {
    8
}

impl Default for EmitterData {
    fn default() -> Self {
        Self {
            name: "Emitter".to_string(),
            enabled: true,
            amount: 8,
            time: EmitterTime::default(),
            drawing: EmitterDrawing::default(),
            draw_passes: vec![EmitterDrawPass::default()],
            process: ParticleProcessConfig::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmitterDrawPass {
    pub mesh: ParticleMesh,
}

impl Default for EmitterDrawPass {
    fn default() -> Self {
        Self {
            mesh: ParticleMesh::Quad,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub enum ParticleMesh {
    #[default]
    Quad,
    Sphere {
        radius: f32,
    },
    Cuboid {
        half_size: Vec3,
    },
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
            gravity: Vec3::new(0., -9.8, 0.),
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
