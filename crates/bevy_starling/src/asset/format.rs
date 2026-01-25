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
    #[serde(default)]
    pub seed: u32,
    #[serde(default)]
    pub use_fixed_seed: bool,
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
            seed: 0,
            use_fixed_seed: false,
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

/// A knot point on a spline curve.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct Knot {
    /// Position along the curve (0.0 to 1.0)
    pub position: f32,
    /// Value at this position
    pub value: f32,
}

impl Knot {
    pub fn new(position: f32, value: f32) -> Self {
        Self { position, value }
    }
}

/// Spline curve for animating particle properties over lifetime.
/// Custom curves use knots, presets are converted to knots for texture baking.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub enum SplineCurve {
    /// Custom curve defined by knot points
    Custom(Vec<Knot>),

    /// Constant value (always 1.0)
    #[default]
    Constant,

    // Increase curves (0 -> 1)
    LinearIn,
    SineIn,
    SineOut,
    SineInOut,
    QuadIn,
    QuadOut,
    QuadInOut,
    CubicIn,
    CubicOut,
    CubicInOut,
    QuartIn,
    QuartOut,
    QuartInOut,
    QuintIn,
    QuintOut,
    QuintInOut,
    ExpoIn,
    ExpoOut,
    ExpoInOut,
    CircIn,
    CircOut,
    CircInOut,
    BackIn,
    BackOut,
    BackInOut,
    ElasticIn,
    ElasticOut,
    ElasticInOut,
    BounceIn,
    BounceOut,
    BounceInOut,

    // Decrease curves (1 -> 0)
    LinearReverse,
    SineInReverse,
    SineOutReverse,
    SineInOutReverse,
    QuadInReverse,
    QuadOutReverse,
    QuadInOutReverse,
    CubicInReverse,
    CubicOutReverse,
    CubicInOutReverse,
    QuartInReverse,
    QuartOutReverse,
    QuartInOutReverse,
    QuintInReverse,
    QuintOutReverse,
    QuintInOutReverse,
    ExpoInReverse,
    ExpoOutReverse,
    ExpoInOutReverse,
    CircInReverse,
    CircOutReverse,
    CircInOutReverse,
    BackInReverse,
    BackOutReverse,
    BackInOutReverse,
    ElasticInReverse,
    ElasticOutReverse,
    ElasticInOutReverse,
    BounceInReverse,
    BounceOutReverse,
    BounceInOutReverse,
}

impl SplineCurve {
    /// Returns a unique cache key for this curve.
    /// For presets, uses the discriminant. For custom curves, hashes the knots.
    pub fn cache_key(&self) -> u64 {
        use std::hash::{Hash, Hasher};

        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        match self {
            Self::Custom(knots) => {
                0u8.hash(&mut hasher); // marker for custom
                for knot in knots {
                    knot.position.to_bits().hash(&mut hasher);
                    knot.value.to_bits().hash(&mut hasher);
                }
            }
            // for presets, use the discriminant
            _ => {
                std::mem::discriminant(self).hash(&mut hasher);
            }
        }
        hasher.finish()
    }

    /// Returns true if this is a constant curve (no animation).
    pub fn is_constant(&self) -> bool {
        matches!(self, Self::Constant)
    }

    /// Converts this curve to its knot representation for texture baking.
    /// Presets generate knots by sampling the easing function.
    pub fn to_knots(&self) -> Vec<Knot> {
        match self {
            Self::Custom(knots) => knots.clone(),
            Self::Constant => vec![Knot::new(0.0, 1.0), Knot::new(1.0, 1.0)],
            _ => {
                // for presets, sample the easing function to generate knots
                const KNOT_COUNT: usize = 32;
                let mut knots = Vec::with_capacity(KNOT_COUNT);
                for i in 0..KNOT_COUNT {
                    let t = i as f32 / (KNOT_COUNT - 1) as f32;
                    let value = self.sample_preset(t);
                    knots.push(Knot::new(t, value));
                }
                knots
            }
        }
    }

    /// Samples a preset curve at the given t value.
    /// Only valid for preset variants, not Custom.
    fn sample_preset(&self, t: f32) -> f32 {
        use std::f32::consts::{FRAC_PI_2, PI};

        match self {
            Self::Custom(_) => 1.0, // should not be called for Custom
            Self::Constant => 1.0,

            // linear
            Self::LinearIn => t,
            Self::LinearReverse => 1.0 - t,

            // sine increase
            Self::SineIn => 1.0 - (t * FRAC_PI_2).cos(),
            Self::SineOut => (t * FRAC_PI_2).sin(),
            Self::SineInOut => -(PI * t).cos() * 0.5 + 0.5,

            // sine decrease
            Self::SineInReverse => 1.0 - (1.0 - (t * FRAC_PI_2).cos()),
            Self::SineOutReverse => 1.0 - (t * FRAC_PI_2).sin(),
            Self::SineInOutReverse => 1.0 - (-(PI * t).cos() * 0.5 + 0.5),

            // quad increase
            Self::QuadIn => t * t,
            Self::QuadOut => -t * (t - 2.0),
            Self::QuadInOut => {
                let t2 = t * 2.0;
                if t2 < 1.0 {
                    0.5 * t2 * t2
                } else {
                    -0.5 * ((t2 - 1.0) * (t2 - 3.0) - 1.0)
                }
            }

            // quad decrease
            Self::QuadInReverse => 1.0 - t * t,
            Self::QuadOutReverse => 1.0 - (-t * (t - 2.0)),
            Self::QuadInOutReverse => {
                let t2 = t * 2.0;
                1.0 - if t2 < 1.0 {
                    0.5 * t2 * t2
                } else {
                    -0.5 * ((t2 - 1.0) * (t2 - 3.0) - 1.0)
                }
            }

            // cubic increase
            Self::CubicIn => t * t * t,
            Self::CubicOut => {
                let t1 = t - 1.0;
                t1 * t1 * t1 + 1.0
            }
            Self::CubicInOut => {
                let t2 = t * 2.0;
                if t2 < 1.0 {
                    0.5 * t2 * t2 * t2
                } else {
                    let t3 = t2 - 2.0;
                    0.5 * (t3 * t3 * t3 + 2.0)
                }
            }

            // cubic decrease
            Self::CubicInReverse => 1.0 - t * t * t,
            Self::CubicOutReverse => {
                let t1 = t - 1.0;
                1.0 - (t1 * t1 * t1 + 1.0)
            }
            Self::CubicInOutReverse => {
                let t2 = t * 2.0;
                1.0 - if t2 < 1.0 {
                    0.5 * t2 * t2 * t2
                } else {
                    let t3 = t2 - 2.0;
                    0.5 * (t3 * t3 * t3 + 2.0)
                }
            }

            // quart increase
            Self::QuartIn => t * t * t * t,
            Self::QuartOut => {
                let t1 = t - 1.0;
                -(t1 * t1 * t1 * t1 - 1.0)
            }
            Self::QuartInOut => {
                let t2 = t * 2.0;
                if t2 < 1.0 {
                    0.5 * t2 * t2 * t2 * t2
                } else {
                    let t3 = t2 - 2.0;
                    -0.5 * (t3 * t3 * t3 * t3 - 2.0)
                }
            }

            // quart decrease
            Self::QuartInReverse => 1.0 - t * t * t * t,
            Self::QuartOutReverse => {
                let t1 = t - 1.0;
                1.0 - (-(t1 * t1 * t1 * t1 - 1.0))
            }
            Self::QuartInOutReverse => {
                let t2 = t * 2.0;
                1.0 - if t2 < 1.0 {
                    0.5 * t2 * t2 * t2 * t2
                } else {
                    let t3 = t2 - 2.0;
                    -0.5 * (t3 * t3 * t3 * t3 - 2.0)
                }
            }

            // quint increase
            Self::QuintIn => t * t * t * t * t,
            Self::QuintOut => {
                let t1 = t - 1.0;
                t1 * t1 * t1 * t1 * t1 + 1.0
            }
            Self::QuintInOut => {
                let t2 = t * 2.0;
                if t2 < 1.0 {
                    0.5 * t2 * t2 * t2 * t2 * t2
                } else {
                    let t3 = t2 - 2.0;
                    0.5 * (t3 * t3 * t3 * t3 * t3 + 2.0)
                }
            }

            // quint decrease
            Self::QuintInReverse => 1.0 - t * t * t * t * t,
            Self::QuintOutReverse => {
                let t1 = t - 1.0;
                1.0 - (t1 * t1 * t1 * t1 * t1 + 1.0)
            }
            Self::QuintInOutReverse => {
                let t2 = t * 2.0;
                1.0 - if t2 < 1.0 {
                    0.5 * t2 * t2 * t2 * t2 * t2
                } else {
                    let t3 = t2 - 2.0;
                    0.5 * (t3 * t3 * t3 * t3 * t3 + 2.0)
                }
            }

            // expo increase
            Self::ExpoIn => {
                if t == 0.0 {
                    0.0
                } else {
                    2.0_f32.powf(10.0 * (t - 1.0)) - 0.001
                }
            }
            Self::ExpoOut => {
                if t == 1.0 {
                    1.0
                } else {
                    1.001 * (1.0 - 2.0_f32.powf(-10.0 * t))
                }
            }
            Self::ExpoInOut => {
                if t == 0.0 {
                    return 0.0;
                }
                if t == 1.0 {
                    return 1.0;
                }
                let t2 = t * 2.0;
                if t2 < 1.0 {
                    0.5 * 2.0_f32.powf(10.0 * (t2 - 1.0)) - 0.0005
                } else {
                    0.5 * 1.0005 * (2.0 - 2.0_f32.powf(-10.0 * (t2 - 1.0)))
                }
            }

            // expo decrease
            Self::ExpoInReverse => {
                1.0 - if t == 0.0 {
                    0.0
                } else {
                    2.0_f32.powf(10.0 * (t - 1.0)) - 0.001
                }
            }
            Self::ExpoOutReverse => {
                1.0 - if t == 1.0 {
                    1.0
                } else {
                    1.001 * (1.0 - 2.0_f32.powf(-10.0 * t))
                }
            }
            Self::ExpoInOutReverse => {
                if t == 0.0 {
                    return 1.0;
                }
                if t == 1.0 {
                    return 0.0;
                }
                let t2 = t * 2.0;
                1.0 - if t2 < 1.0 {
                    0.5 * 2.0_f32.powf(10.0 * (t2 - 1.0)) - 0.0005
                } else {
                    0.5 * 1.0005 * (2.0 - 2.0_f32.powf(-10.0 * (t2 - 1.0)))
                }
            }

            // circ increase
            Self::CircIn => -(1.0 - t * t).sqrt() + 1.0,
            Self::CircOut => {
                let t1 = t - 1.0;
                (1.0 - t1 * t1).sqrt()
            }
            Self::CircInOut => {
                let t2 = t * 2.0;
                if t2 < 1.0 {
                    -0.5 * ((1.0 - t2 * t2).sqrt() - 1.0)
                } else {
                    let t3 = t2 - 2.0;
                    0.5 * ((1.0 - t3 * t3).sqrt() + 1.0)
                }
            }

            // circ decrease
            Self::CircInReverse => 1.0 - (-(1.0 - t * t).sqrt() + 1.0),
            Self::CircOutReverse => {
                let t1 = t - 1.0;
                1.0 - (1.0 - t1 * t1).sqrt()
            }
            Self::CircInOutReverse => {
                let t2 = t * 2.0;
                1.0 - if t2 < 1.0 {
                    -0.5 * ((1.0 - t2 * t2).sqrt() - 1.0)
                } else {
                    let t3 = t2 - 2.0;
                    0.5 * ((1.0 - t3 * t3).sqrt() + 1.0)
                }
            }

            // back increase
            Self::BackIn => {
                const S: f32 = 1.70158;
                t * t * ((S + 1.0) * t - S)
            }
            Self::BackOut => {
                const S: f32 = 1.70158;
                let t1 = t - 1.0;
                t1 * t1 * ((S + 1.0) * t1 + S) + 1.0
            }
            Self::BackInOut => {
                const S: f32 = 1.70158 * 1.525;
                let t2 = t * 2.0;
                if t2 < 1.0 {
                    0.5 * (t2 * t2 * ((S + 1.0) * t2 - S))
                } else {
                    let t3 = t2 - 2.0;
                    0.5 * (t3 * t3 * ((S + 1.0) * t3 + S) + 2.0)
                }
            }

            // back decrease
            Self::BackInReverse => {
                const S: f32 = 1.70158;
                1.0 - t * t * ((S + 1.0) * t - S)
            }
            Self::BackOutReverse => {
                const S: f32 = 1.70158;
                let t1 = t - 1.0;
                1.0 - (t1 * t1 * ((S + 1.0) * t1 + S) + 1.0)
            }
            Self::BackInOutReverse => {
                const S: f32 = 1.70158 * 1.525;
                let t2 = t * 2.0;
                1.0 - if t2 < 1.0 {
                    0.5 * (t2 * t2 * ((S + 1.0) * t2 - S))
                } else {
                    let t3 = t2 - 2.0;
                    0.5 * (t3 * t3 * ((S + 1.0) * t3 + S) + 2.0)
                }
            }

            // elastic increase
            Self::ElasticIn => {
                if t == 0.0 {
                    return 0.0;
                }
                if t == 1.0 {
                    return 1.0;
                }
                let p = 0.3;
                let s = p / 4.0;
                let t1 = t - 1.0;
                let a = 2.0_f32.powf(10.0 * t1);
                -(a * ((t1 - s) * (2.0 * PI) / p).sin())
            }
            Self::ElasticOut => {
                if t == 0.0 {
                    return 0.0;
                }
                if t == 1.0 {
                    return 1.0;
                }
                let p = 0.3;
                let s = p / 4.0;
                2.0_f32.powf(-10.0 * t) * ((t - s) * (2.0 * PI) / p).sin() + 1.0
            }
            Self::ElasticInOut => {
                if t == 0.0 {
                    return 0.0;
                }
                if t == 1.0 {
                    return 1.0;
                }
                let p = 0.3 * 1.5;
                let s = p / 4.0;
                let t2 = t * 2.0;
                if t2 < 1.0 {
                    let t3 = t2 - 1.0;
                    let a = 2.0_f32.powf(10.0 * t3);
                    -0.5 * (a * ((t3 - s) * (2.0 * PI) / p).sin())
                } else {
                    let t3 = t2 - 1.0;
                    let a = 2.0_f32.powf(-10.0 * t3);
                    a * ((t3 - s) * (2.0 * PI) / p).sin() * 0.5 + 1.0
                }
            }

            // elastic decrease
            Self::ElasticInReverse => {
                if t == 0.0 {
                    return 1.0;
                }
                if t == 1.0 {
                    return 0.0;
                }
                let p = 0.3;
                let s = p / 4.0;
                let t1 = t - 1.0;
                let a = 2.0_f32.powf(10.0 * t1);
                1.0 - (-(a * ((t1 - s) * (2.0 * PI) / p).sin()))
            }
            Self::ElasticOutReverse => {
                if t == 0.0 {
                    return 1.0;
                }
                if t == 1.0 {
                    return 0.0;
                }
                let p = 0.3;
                let s = p / 4.0;
                1.0 - (2.0_f32.powf(-10.0 * t) * ((t - s) * (2.0 * PI) / p).sin() + 1.0)
            }
            Self::ElasticInOutReverse => {
                if t == 0.0 {
                    return 1.0;
                }
                if t == 1.0 {
                    return 0.0;
                }
                let p = 0.3 * 1.5;
                let s = p / 4.0;
                let t2 = t * 2.0;
                1.0 - if t2 < 1.0 {
                    let t3 = t2 - 1.0;
                    let a = 2.0_f32.powf(10.0 * t3);
                    -0.5 * (a * ((t3 - s) * (2.0 * PI) / p).sin())
                } else {
                    let t3 = t2 - 1.0;
                    let a = 2.0_f32.powf(-10.0 * t3);
                    a * ((t3 - s) * (2.0 * PI) / p).sin() * 0.5 + 1.0
                }
            }

            // bounce increase
            Self::BounceIn => 1.0 - Self::bounce_out_value(1.0 - t),
            Self::BounceOut => Self::bounce_out_value(t),
            Self::BounceInOut => {
                if t < 0.5 {
                    (1.0 - Self::bounce_out_value(1.0 - t * 2.0)) * 0.5
                } else {
                    Self::bounce_out_value(t * 2.0 - 1.0) * 0.5 + 0.5
                }
            }

            // bounce decrease
            Self::BounceInReverse => Self::bounce_out_value(1.0 - t),
            Self::BounceOutReverse => 1.0 - Self::bounce_out_value(t),
            Self::BounceInOutReverse => {
                1.0 - if t < 0.5 {
                    (1.0 - Self::bounce_out_value(1.0 - t * 2.0)) * 0.5
                } else {
                    Self::bounce_out_value(t * 2.0 - 1.0) * 0.5 + 0.5
                }
            }
        }
    }

    fn bounce_out_value(t: f32) -> f32 {
        const N1: f32 = 7.5625;
        const D1: f32 = 2.75;

        if t < 1.0 / D1 {
            N1 * t * t
        } else if t < 2.0 / D1 {
            let t1 = t - 1.5 / D1;
            N1 * t1 * t1 + 0.75
        } else if t < 2.5 / D1 {
            let t1 = t - 2.25 / D1;
            N1 * t1 * t1 + 0.9375
        } else {
            let t1 = t - 2.625 / D1;
            N1 * t1 * t1 + 0.984375
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
    pub curve: Option<SplineCurve>,
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub alpha_curve: Option<SplineCurve>,
}

impl Default for ParticleProcessDisplayColor {
    fn default() -> Self {
        Self {
            initial_color: SolidOrGradientColor::default(),
            alpha_curve: None,
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

fn default_turbulence_noise_strength() -> f32 {
    1.0
}

fn default_turbulence_noise_scale() -> f32 {
    2.5
}

fn default_turbulence_influence() -> Range {
    Range { min: 0.0, max: 0.1 }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParticleProcessTurbulence {
    #[serde(default)]
    pub enabled: bool,

    #[serde(default = "default_turbulence_noise_strength")]
    pub noise_strength: f32,

    #[serde(default = "default_turbulence_noise_scale")]
    pub noise_scale: f32,

    #[serde(default)]
    pub noise_speed: Vec3,

    #[serde(default)]
    pub noise_speed_random: f32,

    #[serde(default = "default_turbulence_influence")]
    pub influence: Range,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub influence_curve: Option<SplineCurve>,
}

impl Default for ParticleProcessTurbulence {
    fn default() -> Self {
        Self {
            enabled: false,
            noise_strength: default_turbulence_noise_strength(),
            noise_scale: default_turbulence_noise_scale(),
            noise_speed: Vec3::ZERO,
            noise_speed_random: 0.0,
            influence: default_turbulence_influence(),
            influence_curve: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParticleProcessConfig {
    #[serde(default)]
    pub spawn: ParticleProcessSpawn,
    #[serde(default)]
    pub display: ParticleProcessDisplay,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub turbulence: Option<ParticleProcessTurbulence>,
}

impl Default for ParticleProcessConfig {
    fn default() -> Self {
        Self {
            spawn: ParticleProcessSpawn::default(),
            display: ParticleProcessDisplay::default(),
            turbulence: None,
        }
    }
}

#[derive(Asset, TypePath, Debug, Serialize, Deserialize)]
pub struct ParticleSystemAsset {
    pub name: String,
    pub dimension: ParticleSystemDimension,
    pub emitters: Vec<EmitterData>,
}
