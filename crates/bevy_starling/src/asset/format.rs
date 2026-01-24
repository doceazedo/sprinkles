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
    pub delay: f32,
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
            delay: 0.0,
            one_shot: false,
            explosiveness: 0.0,
            randomness: 0.0,
            fixed_fps: 30,
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
            draw_order: DrawOrder::default(),
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

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Range {
    #[serde(default)]
    pub min: f32,
    #[serde(default)]
    pub max: f32,
}

impl Default for Range {
    fn default() -> Self {
        Self { min: 0.0, max: 0.0 }
    }
}

impl Range {
    pub fn new(min: f32, max: f32) -> Self {
        Self { min, max }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default, PartialEq)]
pub enum EmissionShape {
    #[default]
    Point,
    Sphere {
        radius: f32,
    },
    SphereSurface {
        radius: f32,
    },
    Box {
        extents: Vec3,
    },
    Ring {
        axis: Vec3,
        height: f32,
        radius: f32,
        inner_radius: f32,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParticleProcessSpawnPosition {
    #[serde(default)]
    pub emission_shape: EmissionShape,
    #[serde(default)]
    pub emission_shape_offset: Vec3,
    #[serde(default = "default_emission_shape_scale")]
    pub emission_shape_scale: Vec3,
}

fn default_emission_shape_scale() -> Vec3 {
    Vec3::ONE
}

impl Default for ParticleProcessSpawnPosition {
    fn default() -> Self {
        Self {
            emission_shape: EmissionShape::default(),
            emission_shape_offset: Vec3::ZERO,
            emission_shape_scale: Vec3::ONE,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParticleProcessSpawnVelocity {
    #[serde(default)]
    pub inherit_velocity_ratio: f32,
    #[serde(default)]
    pub velocity_pivot: Vec3,
    #[serde(default = "default_direction")]
    pub direction: Vec3,
    #[serde(default = "default_spread")]
    pub spread: f32,
    #[serde(default)]
    pub flatness: f32,
    #[serde(default)]
    pub initial_velocity: Range,
}

fn default_direction() -> Vec3 {
    Vec3::X
}

fn default_spread() -> f32 {
    45.0
}

impl Default for ParticleProcessSpawnVelocity {
    fn default() -> Self {
        Self {
            inherit_velocity_ratio: 0.0,
            velocity_pivot: Vec3::ZERO,
            direction: Vec3::X,
            spread: 45.0,
            flatness: 0.0,
            initial_velocity: Range::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParticleProcessSpawnAccelerations {
    #[serde(default = "default_gravity")]
    pub gravity: Vec3,
}

fn default_gravity() -> Vec3 {
    Vec3::new(0.0, -9.8, 0.0)
}

impl Default for ParticleProcessSpawnAccelerations {
    fn default() -> Self {
        Self {
            gravity: Vec3::new(0.0, -9.8, 0.0),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParticleProcessSpawn {
    #[serde(default)]
    pub position: ParticleProcessSpawnPosition,
    #[serde(default)]
    pub velocity: ParticleProcessSpawnVelocity,
    #[serde(default)]
    pub accelerations: ParticleProcessSpawnAccelerations,
}

impl Default for ParticleProcessSpawn {
    fn default() -> Self {
        Self {
            position: ParticleProcessSpawnPosition::default(),
            velocity: ParticleProcessSpawnVelocity::default(),
            accelerations: ParticleProcessSpawnAccelerations::default(),
        }
    }
}

// TODO: implement more easing curves (Sine, Quad, Cubic, Elastic, Bounce, etc.)
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Default)]
pub enum EasingCurve {
    #[default]
    LinearIn,
    LinearOut,
}

impl EasingCurve {
    pub fn to_gpu_constant(&self) -> u32 {
        match self {
            Self::LinearIn => 1,
            Self::LinearOut => 2,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default, PartialEq, Eq, Hash)]
pub enum GradientInterpolation {
    Steps,
    #[default]
    Linear,
    Smoothstep,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GradientStop {
    pub color: [f32; 4],
    pub position: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Gradient {
    pub stops: Vec<GradientStop>,
    #[serde(default)]
    pub interpolation: GradientInterpolation,
}

impl Default for Gradient {
    fn default() -> Self {
        Self {
            stops: vec![
                GradientStop {
                    color: [0.0, 0.0, 0.0, 1.0],
                    position: 0.0,
                },
                GradientStop {
                    color: [1.0, 1.0, 1.0, 1.0],
                    position: 1.0,
                },
            ],
            interpolation: GradientInterpolation::Linear,
        }
    }
}

impl Gradient {
    pub fn cache_key(&self) -> u64 {
        use std::hash::{Hash, Hasher};
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        for stop in &self.stops {
            for c in stop.color {
                c.to_bits().hash(&mut hasher);
            }
            stop.position.to_bits().hash(&mut hasher);
        }
        self.interpolation.hash(&mut hasher);
        hasher.finish()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SolidOrGradientColor {
    Solid { color: [f32; 4] },
    Gradient { gradient: Gradient },
}

impl Default for SolidOrGradientColor {
    fn default() -> Self {
        Self::Solid {
            color: [1.0, 1.0, 1.0, 1.0],
        }
    }
}

impl SolidOrGradientColor {
    pub fn solid(color: [f32; 4]) -> Self {
        Self::Solid { color }
    }

    pub fn is_solid(&self) -> bool {
        matches!(self, Self::Solid { .. })
    }

    pub fn is_gradient(&self) -> bool {
        matches!(self, Self::Gradient { .. })
    }

    pub fn as_solid_color(&self) -> Option<[f32; 4]> {
        match self {
            Self::Solid { color } => Some(*color),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParticleProcessDisplayScale {
    #[serde(default = "default_scale_range")]
    pub range: Range,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub curve: Option<EasingCurve>,
}

fn default_scale_range() -> Range {
    Range { min: 1.0, max: 1.0 }
}

impl Default for ParticleProcessDisplayScale {
    fn default() -> Self {
        Self {
            range: default_scale_range(),
            curve: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParticleProcessDisplayColor {
    #[serde(default)]
    pub initial_color: SolidOrGradientColor,
}

impl Default for ParticleProcessDisplayColor {
    fn default() -> Self {
        Self {
            initial_color: SolidOrGradientColor::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ParticleProcessDisplay {
    #[serde(default)]
    pub scale: ParticleProcessDisplayScale,
    #[serde(default)]
    pub color_curves: ParticleProcessDisplayColor,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParticleProcessConfig {
    #[serde(default)]
    pub spawn: ParticleProcessSpawn,
    #[serde(default)]
    pub display: ParticleProcessDisplay,
}

impl Default for ParticleProcessConfig {
    fn default() -> Self {
        Self {
            spawn: ParticleProcessSpawn::default(),
            display: ParticleProcessDisplay::default(),
        }
    }
}

#[derive(Asset, TypePath, Debug, Serialize, Deserialize)]
pub struct ParticleSystemAsset {
    pub name: String,
    pub dimension: ParticleSystemDimension,
    pub emitters: Vec<EmitterData>,
}
