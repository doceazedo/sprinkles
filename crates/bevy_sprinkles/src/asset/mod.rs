mod curve;
mod gradient;
mod particle_material;
pub(crate) mod serde_helpers;
/// Asset format version tracking and compatibility validation.
pub mod versioning;

pub use curve::{CurveEasing, CurveMode, CurvePoint, CurveTexture};
pub use gradient::{Gradient, GradientInterpolation, GradientStop, SolidOrGradientColor};
pub use particle_material::{
    DrawPassMaterial, SerializableAlphaMode, SerializableFace, StandardParticleMaterial,
};

use bevy::{
    asset::{AssetLoader, LoadContext, io::Reader},
    prelude::*,
};
use bitflags::bitflags;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use serde_helpers::*;
use versioning::{VersionStatus, current_format_version};

/// Asset loader for [`ParticleSystemAsset`] files in RON format.
#[derive(Default, TypePath)]
pub struct ParticleSystemAssetLoader;

/// Errors that can occur when loading a [`ParticleSystemAsset`].
#[non_exhaustive]
#[derive(Debug, Error)]
pub enum ParticleSystemAssetLoaderError {
    /// An I/O error occurred while reading the asset file.
    #[error("Could not load asset: {0}")]
    Io(#[from] std::io::Error),
    /// The asset file contained invalid RON syntax.
    #[error("Could not parse RON: {0}")]
    Ron(#[from] ron::error::SpannedError),
    /// The asset file has an unknown format version, likely from a newer Sprinkles.
    #[error("Unknown sprinkles_version. You may need a newer version of Sprinkles.")]
    UnknownVersion,
    /// The asset file has a version that requires breaking changes to upgrade.
    #[error(
        "Asset version \"{found}\" is incompatible with current version \"{current}\". Manual migration is required."
    )]
    IncompatibleVersion {
        /// The version found in the asset file.
        found: String,
        /// The current format version.
        current: String,
    },
}

impl AssetLoader for ParticleSystemAssetLoader {
    type Asset = ParticleSystemAsset;
    type Settings = ();
    type Error = ParticleSystemAssetLoaderError;

    async fn load(
        &self,
        reader: &mut dyn Reader,
        _settings: &(),
        load_context: &mut LoadContext<'_>,
    ) -> Result<Self::Asset, Self::Error> {
        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes).await?;
        let mut asset = ron::de::from_bytes::<ParticleSystemAsset>(&bytes)?;

        match asset.try_upgrade_version() {
            VersionStatus::Current => {}
            VersionStatus::Outdated { found, current } => {
                let path = load_context.path();
                warn!(
                    "{path:?}: loaded asset with sprinkles_version \"{found}\", current is \"{current}\""
                );
            }
            VersionStatus::Incompatible { found, current } => {
                return Err(ParticleSystemAssetLoaderError::IncompatibleVersion {
                    found,
                    current: current.to_string(),
                });
            }
            VersionStatus::Unknown => {
                return Err(ParticleSystemAssetLoaderError::UnknownVersion);
            }
        }

        Ok(asset)
    }

    fn extensions(&self) -> &[&str] {
        &["ron"]
    }
}

bitflags! {
    /// Bitflags that control per-particle behavior.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
    #[serde(transparent)]
    pub struct ParticleFlags: u32 {
        /// If set, particles rotate around the Y axis by the configured angle.
        const ROTATE_Y = 1 << 1;
        /// If set, particles will not move on the Z axis, confining them to a 2D plane.
        const DISABLE_Z = 1 << 2;

        // TODO: requires implementing damping
    }
}

/// Whether the particle system operates in 3D or 2D space.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default, Reflect)]
pub enum ParticleSystemDimension {
    /// 3D particle system.
    #[default]
    D3,
    /// 2D particle system.
    D2,
}

/// Controls the order in which particles are drawn.
///
/// Draw order can affect visual quality depending on the blending mode used.
/// [`DrawOrder::Index`] is the only option that supports motion vectors for
/// effects like TAA, making it the best choice for opaque particles.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default, Reflect)]
pub enum DrawOrder {
    /// Particles are drawn in the order they were emitted.
    #[default]
    Index,
    /// Particles are drawn by remaining lifetime, highest first.
    Lifetime,
    /// Particles are drawn by remaining lifetime, lowest first.
    ReverseLifetime,
    /// Particles are drawn by depth relative to the camera.
    ViewDepth,
}

impl DrawOrder {
    fn is_default(&self) -> bool {
        *self == Self::default()
    }
}

/// Timing and lifecycle configuration for an emitter.
#[derive(Debug, Clone, Serialize, Deserialize, Reflect)]
pub struct EmitterTime {
    /// The amount of time each particle will exist, in seconds.
    ///
    /// The effective emission rate is `particles_amount / lifetime` particles per second.
    /// Defaults to `1.0`.
    #[serde(default = "default_lifetime")]
    pub lifetime: f32,
    /// Particle lifetime randomness ratio.
    ///
    /// The actual lifetime of each particle is `lifetime * (1.0 - rand() * lifetime_randomness)`.
    /// For example, a value of `0.4` scales each particle's lifetime between 60% and 100% of
    /// the configured [`lifetime`](Self::lifetime). Defaults to `0.0`.
    #[serde(default, skip_serializing_if = "is_zero_f32")]
    pub lifetime_randomness: f32,
    /// Time in seconds to wait before the emitter starts spawning particles.
    ///
    /// Defaults to `0.0`.
    #[serde(default, skip_serializing_if = "is_zero_f32")]
    pub delay: f32,
    /// If `true`, only one emission cycle will occur: exactly `particles_amount` particles
    /// will be emitted, and then the emitter stops.
    ///
    /// Defaults to `false`.
    #[serde(default, skip_serializing_if = "is_false")]
    pub one_shot: bool,
    /// Time ratio between each emission, from `0.0` to `1.0`.
    ///
    /// If `0.0`, particles are emitted continuously over the lifetime. If `1.0`, all
    /// particles are emitted simultaneously at the start of each cycle. Defaults to `0.0`.
    #[serde(default, skip_serializing_if = "is_zero_f32")]
    pub explosiveness: f32,
    /// Emission randomness ratio.
    ///
    /// Adds randomness to the timing of individual particle spawns within each cycle.
    /// Defaults to `0.0`.
    #[serde(default, skip_serializing_if = "is_zero_f32")]
    pub spawn_time_randomness: f32,
    /// Fixed frame rate for the particle simulation, in frames per second.
    ///
    /// When set to a non-zero value, the particle system updates at this fixed rate
    /// instead of every frame. This does not slow down the simulation itself, only
    /// how often it is evaluated. Defaults to `0` (updates every frame).
    #[serde(default, skip_serializing_if = "is_zero_u32")]
    pub fixed_fps: u32,
    /// Optional fixed random seed for deterministic particle behavior.
    ///
    /// When set, the particle system will produce the same visual result across
    /// replays, which is useful for cinematics or testing. Defaults to `None`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fixed_seed: Option<u32>,
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
            spawn_time_randomness: 0.0,
            fixed_fps: 0,
            fixed_seed: None,
        }
    }
}

impl EmitterTime {
    /// Returns the total duration of one emission cycle, including the delay.
    pub fn total_duration(&self) -> f32 {
        self.delay + self.lifetime
    }
}

/// Complete configuration for a single particle emitter.
///
/// An emitter is the source that creates particles. It controls how, where, and when
/// particles are spawned, as well as their visual properties and physical behavior
/// over their lifetime.
#[derive(Debug, Clone, Serialize, Deserialize, Reflect)]
pub struct EmitterData {
    /// Display name for this emitter.
    pub name: String,
    /// Whether this emitter is active. Disabled emitters do not spawn particles.
    ///
    /// Defaults to `true`.
    #[serde(default = "default_enabled", skip_serializing_if = "is_true")]
    pub enabled: bool,

    /// Position offset of this emitter relative to the particle system entity.
    ///
    /// Defaults to [`Vec3::ZERO`].
    #[serde(default, skip_serializing_if = "is_zero_vec3")]
    pub position: Vec3,

    /// Timing and lifecycle settings (lifetime, delay, one-shot, etc.).
    #[serde(default)]
    pub time: EmitterTime,

    /// Draw pass configuration (mesh, material, draw order).
    #[serde(default)]
    pub draw_pass: EmitterDrawPass,

    /// Emission shape and particle count settings.
    #[serde(default)]
    pub emission: EmitterEmission,

    /// Particle scale range and scale-over-lifetime curve.
    #[serde(default)]
    pub scale: EmitterScale,

    /// Initial particle rotation angle and angle-over-lifetime curve.
    #[serde(default, skip_serializing_if = "EmitterAngle::should_skip")]
    pub angle: EmitterAngle,

    /// Color and alpha settings, including gradients and curves over lifetime.
    #[serde(default)]
    pub colors: EmitterColors,

    /// Velocity settings (direction, spread, radial/angular velocity, etc.).
    #[serde(default)]
    pub velocities: EmitterVelocities,

    /// Acceleration settings (gravity).
    #[serde(default)]
    pub accelerations: EmitterAccelerations,

    /// Turbulence noise settings for varying particle movement.
    #[serde(default, skip_serializing_if = "EmitterTurbulence::should_skip")]
    pub turbulence: EmitterTurbulence,

    /// Collision behavior settings.
    #[serde(default)]
    pub collision: EmitterCollision,

    /// Optional sub-emitter configuration for spawning secondary particles.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sub_emitter: Option<SubEmitterConfig>,

    /// Bitflags controlling per-particle behavior (Y rotation, Z-axis disable, etc.).
    #[serde(default)]
    #[reflect(ignore)]
    pub particle_flags: ParticleFlags,
}

fn default_enabled() -> bool {
    true
}

impl Default for EmitterData {
    fn default() -> Self {
        Self {
            name: "Emitter".to_string(),
            enabled: true,
            position: Vec3::ZERO,
            time: EmitterTime::default(),
            draw_pass: EmitterDrawPass::default(),
            emission: EmitterEmission::default(),
            scale: EmitterScale::default(),
            angle: EmitterAngle::default(),
            colors: EmitterColors::default(),
            velocities: EmitterVelocities::default(),
            accelerations: EmitterAccelerations::default(),
            turbulence: EmitterTurbulence::default(),
            collision: EmitterCollision::default(),
            sub_emitter: None,
            particle_flags: ParticleFlags::empty(),
        }
    }
}

/// Controls how each particle's transform is aligned relative to the camera or its velocity.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default, Reflect)]
pub enum TransformAlign {
    /// Particles always face the camera (Z-billboard).
    #[default]
    Billboard,
    /// Particles align their Y axis to the direction of their velocity.
    YToVelocity,
    /// Particles face the camera and additionally align their Y axis to velocity.
    BillboardYToVelocity,
    /// Particles face the camera with a fixed world-space Y axis.
    BillboardFixedY,
}

/// Configuration for how particles are rendered in a single draw pass.
#[derive(Debug, Clone, Serialize, Deserialize, Reflect)]
pub struct EmitterDrawPass {
    /// The order in which particles are drawn. Defaults to [`DrawOrder::Index`].
    #[serde(default, skip_serializing_if = "DrawOrder::is_default")]
    pub draw_order: DrawOrder,
    /// The mesh shape used to render each particle.
    pub mesh: ParticleMesh,
    /// The material applied to the particle mesh. Defaults to a standard PBR material.
    #[serde(default)]
    pub material: DrawPassMaterial,
    /// Whether particles cast shadows. Defaults to `true`.
    #[serde(default = "default_shadow_caster", skip_serializing_if = "is_true")]
    pub shadow_caster: bool,
    /// Optional transform alignment mode for particles. When `None`, no special
    /// alignment is applied.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub transform_align: Option<TransformAlign>,
}

fn default_shadow_caster() -> bool {
    true
}

impl Default for EmitterDrawPass {
    fn default() -> Self {
        Self {
            draw_order: DrawOrder::default(),
            mesh: ParticleMesh::default(),
            material: DrawPassMaterial::default(),
            shadow_caster: true,
            transform_align: None,
        }
    }
}

/// The axis a quad particle mesh faces by default.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default, PartialEq, Reflect)]
pub enum QuadOrientation {
    /// The quad faces along the X axis.
    FaceX,
    /// The quad faces along the Y axis.
    FaceY,
    /// The quad faces along the Z axis.
    #[default]
    FaceZ,
}

/// The mesh shape used to render each particle.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Reflect)]
pub enum ParticleMesh {
    /// A flat quadrilateral. Commonly used for billboard particles like sparks and smoke.
    Quad {
        /// Which axis the quad faces. Defaults to [`QuadOrientation::FaceZ`].
        #[serde(default)]
        orientation: QuadOrientation,
        /// Size of the quad in world units. Defaults to `Vec2::ONE`.
        #[serde(default = "default_quad_size")]
        size: Vec2,
        /// Number of subdivisions along each axis. Defaults to `Vec2::ZERO` (no subdivision).
        #[serde(default, skip_serializing_if = "is_zero_vec2")]
        subdivide: Vec2,
    },
    /// A sphere mesh.
    Sphere {
        /// Radius of the sphere. Defaults to `1.0`.
        #[serde(default = "default_sphere_radius")]
        radius: f32,
    },
    /// An axis-aligned box mesh.
    Cuboid {
        /// Half-extents of the box along each axis.
        half_size: Vec3,
    },
    /// A cylinder or cone mesh.
    Cylinder {
        /// Radius of the top cap.
        top_radius: f32,
        /// Radius of the bottom cap.
        bottom_radius: f32,
        /// Height of the cylinder along the Y axis.
        height: f32,
        /// Number of radial segments around the circumference.
        radial_segments: u32,
        /// Number of vertical ring subdivisions.
        rings: u32,
        /// Whether to generate the top cap.
        cap_top: bool,
        /// Whether to generate the bottom cap.
        cap_bottom: bool,
    },
    /// A triangular prism mesh.
    Prism {
        /// Ratio controlling the position of the apex, from `0.0` (left) to `1.0` (right).
        /// Defaults to `0.5` (centered).
        #[serde(default = "default_prism_left_to_right")]
        left_to_right: f32,
        /// Size of the prism along each axis. Defaults to `Vec3::splat(1.0)`.
        #[serde(default = "default_prism_size")]
        size: Vec3,
        /// Number of subdivisions along each axis. Defaults to `Vec3::ZERO`.
        #[serde(default, skip_serializing_if = "is_zero_vec3")]
        subdivide: Vec3,
    },
}

fn default_quad_size() -> Vec2 {
    Vec2::ONE
}

fn default_sphere_radius() -> f32 {
    1.0
}

fn default_prism_left_to_right() -> f32 {
    0.5
}

fn default_prism_size() -> Vec3 {
    Vec3::splat(1.0)
}

impl Default for ParticleMesh {
    fn default() -> Self {
        Self::Sphere { radius: 1.0 }
    }
}

/// A minimum/maximum range of `f32` values, used to randomize particle properties.
///
/// When a particle is spawned, a random value between [`min`](Self::min) and
/// [`max`](Self::max) is selected. Defaults to `0.0..1.0`.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Reflect)]
pub struct Range {
    /// Lower bound of the range. Defaults to `0.0`.
    #[serde(default)]
    pub min: f32,
    /// Upper bound of the range. Defaults to `1.0`.
    #[serde(default = "default_one_f32")]
    pub max: f32,
}

fn default_one_f32() -> f32 {
    1.0
}

impl Default for Range {
    fn default() -> Self {
        Self { min: 0.0, max: 1.0 }
    }
}

impl Range {
    /// Creates a new range with the given bounds.
    pub fn new(min: f32, max: f32) -> Self {
        Self { min, max }
    }

    /// Returns the span of this range (`max - min`), or `1.0` if the span is effectively zero.
    pub fn span(&self) -> f32 {
        let span = self.max - self.min;
        if span.abs() < f32::EPSILON { 1.0 } else { span }
    }

    fn is_zero(&self) -> bool {
        self.min == 0.0 && self.max == 0.0
    }

    fn zero() -> Self {
        Self { min: 0.0, max: 0.0 }
    }
}

/// The region in which particles are spawned.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default, PartialEq, Reflect)]
pub enum EmissionShape {
    /// All particles are emitted from a single point.
    #[default]
    Point,
    /// Particles are emitted within the volume of a sphere.
    Sphere {
        /// Radius of the emission sphere.
        radius: f32,
    },
    /// Particles are emitted on the surface of a sphere.
    SphereSurface {
        /// Radius of the emission sphere surface.
        radius: f32,
    },
    /// Particles are emitted within the volume of a box.
    ///
    /// The extents define the half-size along each axis. The actual box is twice as large.
    Box {
        /// Half-extents of the emission box along each axis.
        extents: Vec3,
    },
    /// Particles are emitted within a ring or cylinder shape.
    Ring {
        /// The axis the ring is oriented around.
        axis: Vec3,
        /// The height of the ring (cylinder) along the axis.
        height: f32,
        /// The outer radius of the ring.
        radius: f32,
        /// The inner radius of the ring. A value of `0.0` fills the entire disc.
        inner_radius: f32,
    },
}

impl EmissionShape {
    fn is_default(&self) -> bool {
        *self == Self::default()
    }
}

fn default_emission_scale() -> Vec3 {
    Vec3::ONE
}

fn default_particles_amount() -> u32 {
    8
}

/// Emission configuration: shape, offset, scale, and particle count.
#[derive(Debug, Clone, Serialize, Deserialize, Reflect)]
pub struct EmitterEmission {
    /// Position offset of the emission shape in local space. Defaults to [`Vec3::ZERO`].
    #[serde(default, skip_serializing_if = "is_zero_vec3")]
    pub offset: Vec3,
    /// Scale of the emission shape in local space. Defaults to [`Vec3::ONE`].
    #[serde(
        default = "default_emission_scale",
        skip_serializing_if = "is_one_vec3"
    )]
    pub scale: Vec3,
    /// The shape of the emission region. Defaults to [`EmissionShape::Point`].
    #[serde(default, skip_serializing_if = "EmissionShape::is_default")]
    pub shape: EmissionShape,
    /// The number of particles to emit in one emission cycle.
    ///
    /// Higher values will increase GPU load. Defaults to `8`.
    #[serde(default = "default_particles_amount")]
    pub particles_amount: u32,
}

impl Default for EmitterEmission {
    fn default() -> Self {
        Self {
            offset: Vec3::ZERO,
            scale: Vec3::ONE,
            shape: EmissionShape::default(),
            particles_amount: 8,
        }
    }
}

fn default_scale_range() -> Range {
    Range { min: 1.0, max: 1.0 }
}

/// Particle scale configuration.
#[derive(Debug, Clone, Serialize, Deserialize, Reflect)]
pub struct EmitterScale {
    /// The initial scale range applied to each particle.
    ///
    /// A random value between `min` and `max` is selected at spawn time.
    /// Defaults to `1.0..1.0`.
    #[serde(default = "default_scale_range")]
    pub range: Range,
    /// Optional curve that modulates each particle's scale over its lifetime.
    ///
    /// The curve value is multiplied with the initial scale.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scale_over_lifetime: Option<CurveTexture>,
}

impl Default for EmitterScale {
    fn default() -> Self {
        Self {
            range: default_scale_range(),
            scale_over_lifetime: None,
        }
    }
}

/// Color and alpha configuration for particles.
#[derive(Debug, Clone, Serialize, Deserialize, Reflect)]
pub struct EmitterColors {
    /// Each particle's initial color. Can be a solid color or a gradient from which a random
    /// color is sampled at spawn time. Defaults to opaque white.
    #[serde(default)]
    pub initial_color: SolidOrGradientColor,
    /// Gradient that modulates each particle's color over its lifetime.
    ///
    /// The particle's initial color is multiplied by the gradient value at the
    /// corresponding lifetime position. Defaults to a constant white gradient.
    #[serde(default = "Gradient::white")]
    pub color_over_lifetime: Gradient,
    /// Optional curve that modulates each particle's alpha over its lifetime.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub alpha_over_lifetime: Option<CurveTexture>,
    /// Optional curve that modulates the emissive intensity over each particle's lifetime.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub emission_over_lifetime: Option<CurveTexture>,
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

fn default_direction() -> Vec3 {
    Vec3::X
}

fn default_spread() -> f32 {
    45.0
}

/// A velocity value with an optional curve for animation over a particle's lifetime.
#[derive(Debug, Clone, Serialize, Deserialize, Reflect)]
pub struct AnimatedVelocity {
    /// The initial velocity range. A random value between `min` and `max` is
    /// selected at spawn time. Defaults to zero.
    #[serde(default = "Range::zero", skip_serializing_if = "Range::is_zero")]
    pub velocity: Range,
    /// Optional curve that modulates the velocity over each particle's lifetime.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub velocity_over_lifetime: Option<CurveTexture>,
}

impl Default for AnimatedVelocity {
    fn default() -> Self {
        Self {
            velocity: Range::zero(),
            velocity_over_lifetime: None,
        }
    }
}

/// Initial rotation angle and animated rotation for particles.
///
/// Only applied when [`ParticleFlags::DISABLE_Z`] or [`ParticleFlags::ROTATE_Y`] are set,
/// or when using billboard rendering.
#[derive(Debug, Clone, Serialize, Deserialize, Reflect)]
pub struct EmitterAngle {
    /// The initial rotation angle range in degrees. A random value between `min` and
    /// `max` is applied to each particle at spawn time. Defaults to zero.
    #[serde(default = "Range::zero", skip_serializing_if = "Range::is_zero")]
    pub range: Range,
    /// Optional curve that animates each particle's rotation over its lifetime.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub angle_over_lifetime: Option<CurveTexture>,
}

impl Default for EmitterAngle {
    fn default() -> Self {
        Self {
            range: Range::zero(),
            angle_over_lifetime: None,
        }
    }
}

impl EmitterAngle {
    fn should_skip(&self) -> bool {
        self.range.is_zero() && self.angle_over_lifetime.is_none()
    }
}

/// Velocity settings for particles, including direction, spread, and animated velocities.
#[derive(Debug, Clone, Serialize, Deserialize, Reflect)]
pub struct EmitterVelocities {
    /// Unit vector specifying the base emission direction. Defaults to `Vec3::X`.
    #[serde(default = "default_direction")]
    pub initial_direction: Vec3,
    /// The angular spread in degrees. Each particle's initial direction varies from
    /// +spread to -spread relative to [`initial_direction`](Self::initial_direction).
    /// Defaults to `45.0`.
    #[serde(default = "default_spread")]
    pub spread: f32,
    /// Amount of spread flattening along the Y axis.
    ///
    /// A value of `0.0` means uniform conical spread; `1.0` flattens it into a disc.
    /// Defaults to `0.0`.
    #[serde(default, skip_serializing_if = "is_zero_f32")]
    pub flatness: f32,
    /// The initial velocity magnitude range. Each particle receives a random speed
    /// between `min` and `max`, applied in its emission direction. Defaults to zero.
    #[serde(default = "Range::zero", skip_serializing_if = "Range::is_zero")]
    pub initial_velocity: Range,
    /// Radial velocity that pushes particles away from (or toward, if negative) the
    /// [`pivot`](Self::pivot) point.
    #[serde(default)]
    pub radial_velocity: AnimatedVelocity,
    /// Angular (rotation) velocity applied to each particle, in degrees per second.
    ///
    /// Only applied when [`ParticleFlags::DISABLE_Z`] or [`ParticleFlags::ROTATE_Y`] are set,
    /// or when using billboard rendering.
    #[serde(default)]
    pub angular_velocity: AnimatedVelocity,
    /// The pivot point used to calculate radial and orbital velocity.
    ///
    /// Defaults to [`Vec3::ZERO`].
    #[serde(default, skip_serializing_if = "is_zero_vec3")]
    pub pivot: Vec3,
    /// Percentage of the emitter entity's velocity inherited by each particle when spawning.
    ///
    /// Defaults to `0.0`.
    #[serde(default, skip_serializing_if = "is_zero_f32")]
    pub inherit_ratio: f32,
}

impl Default for EmitterVelocities {
    fn default() -> Self {
        Self {
            initial_direction: Vec3::X,
            spread: 45.0,
            flatness: 0.0,
            initial_velocity: Range::zero(),
            radial_velocity: AnimatedVelocity::default(),
            angular_velocity: AnimatedVelocity::default(),
            pivot: Vec3::ZERO,
            inherit_ratio: 0.0,
        }
    }
}

fn default_gravity() -> Vec3 {
    Vec3::new(0.0, -9.8, 0.0)
}

/// Acceleration forces applied to every particle.
#[derive(Debug, Clone, Serialize, Deserialize, Reflect)]
pub struct EmitterAccelerations {
    /// Gravity vector applied to every particle, in units per second squared.
    ///
    /// Defaults to `(0.0, -9.8, 0.0)`.
    #[serde(default = "default_gravity")]
    pub gravity: Vec3,
}

impl Default for EmitterAccelerations {
    fn default() -> Self {
        Self {
            gravity: Vec3::new(0.0, -9.8, 0.0),
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

/// Turbulence noise settings for varying particle movement based on position.
///
/// Turbulence uses a 3D noise pattern to displace particles, creating organic,
/// wind-like motion. Enabling turbulence has a significant performance cost on
/// the GPU.
#[derive(Debug, Clone, Serialize, Deserialize, Reflect)]
pub struct EmitterTurbulence {
    /// Whether turbulence is enabled. Defaults to `false`.
    #[serde(default, skip_serializing_if = "is_false")]
    pub enabled: bool,
    /// The turbulence noise strength. Higher values produce a stronger, more
    /// contrasting flow pattern. Defaults to `1.0`.
    #[serde(default = "default_turbulence_noise_strength")]
    pub noise_strength: f32,
    /// Overall scale/frequency of the turbulence noise pattern.
    ///
    /// A small scale produces smaller features with more detail, while a large
    /// scale produces smoother noise with larger features. Defaults to `2.5`.
    #[serde(default = "default_turbulence_noise_scale")]
    pub noise_scale: f32,
    /// Scrolling velocity for the turbulence field, setting a directional trend
    /// for the noise pattern over time. Defaults to [`Vec3::ZERO`] (no scrolling).
    #[serde(default, skip_serializing_if = "is_zero_vec3")]
    pub noise_speed: Vec3,
    /// The in-place rate of change of the turbulence field.
    ///
    /// Controls how quickly the noise pattern varies over time. A value of `0.0`
    /// results in a fixed pattern. Defaults to `0.0`.
    #[serde(default, skip_serializing_if = "is_zero_f32")]
    pub noise_speed_random: f32,
    /// The range of turbulence influence applied to each particle.
    ///
    /// A random value between `min` and `max` is selected per particle, then
    /// multiplied by [`influence_over_lifetime`](Self::influence_over_lifetime)
    /// if provided. Defaults to `0.0..0.1`.
    #[serde(default = "default_turbulence_influence")]
    pub influence: Range,
    /// Optional curve that modulates turbulence influence over each particle's lifetime.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub influence_over_lifetime: Option<CurveTexture>,
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

impl EmitterTurbulence {
    fn should_skip(&self) -> bool {
        if self.enabled {
            return false;
        }
        let d = Self::default();
        self.noise_strength == d.noise_strength
            && self.noise_scale == d.noise_scale
            && self.noise_speed == d.noise_speed
            && self.noise_speed_random == d.noise_speed_random
            && self.influence.min == d.influence.min
            && self.influence.max == d.influence.max
            && self.influence_over_lifetime.is_none()
    }
}

fn default_collision_base_size() -> f32 {
    0.01
}

/// How particles behave when they collide with a [`ParticlesCollider3D`](crate::ParticlesCollider3D).
#[derive(Debug, Clone, Serialize, Deserialize, Reflect)]
pub enum EmitterCollisionMode {
    /// Rigid-body style collision. Particles bounce off surfaces.
    Rigid {
        /// Friction factor from `0.0` (frictionless) to `1.0` (maximum friction).
        friction: f32,
        /// Bounciness from `0.0` (no bounce) to `1.0` (full bounce).
        bounce: f32,
    },
    /// Particles are hidden instantly on contact with a collider.
    ///
    /// This can be combined with a sub-emitter to replace the parent particle
    /// with secondary particles on impact.
    HideOnContact,
}

impl Default for EmitterCollisionMode {
    fn default() -> Self {
        Self::Rigid {
            friction: 0.0,
            bounce: 0.0,
        }
    }
}

/// Particle collision configuration.
#[derive(Debug, Clone, Serialize, Deserialize, Reflect)]
pub struct EmitterCollision {
    /// The collision mode. When `None`, collision is disabled and particles pass
    /// through colliders. Defaults to `None`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mode: Option<EmitterCollisionMode>,
    /// If `true`, [`base_size`](Self::base_size) is multiplied by the particle's
    /// effective scale. Defaults to `false`.
    #[serde(default, skip_serializing_if = "is_false")]
    pub use_scale: bool,
    /// The base diameter for particle collision, in meters.
    ///
    /// If particles appear to sink into the ground, increase this value. If they
    /// appear to float above surfaces, decrease it. Particles always use a spherical
    /// collision shape. Defaults to `0.01`.
    #[serde(default = "default_collision_base_size")]
    pub base_size: f32,
}

impl Default for EmitterCollision {
    fn default() -> Self {
        Self {
            mode: None,
            base_size: default_collision_base_size(),
            use_scale: false,
        }
    }
}

/// When a sub-emitter spawns its particles.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub enum SubEmitterMode {
    /// Continuously emit from each parent particle at a fixed frequency.
    Constant,
    /// Emit when the parent particle reaches the end of its lifetime.
    AtEnd,
    /// Emit when the parent particle collides with a surface.
    AtCollision,
    /// Emit once when the parent particle is first spawned.
    AtStart,
}

fn default_sub_emitter_frequency() -> f32 {
    4.0
}

fn default_sub_emitter_amount() -> u32 {
    1
}

/// Configuration for a sub-emitter that spawns secondary particles from parent particles.
///
/// Sub-emitters can be used to achieve effects such as fireworks, sparks on collision,
/// or bubbles popping into water drops.
#[derive(Debug, Clone, Serialize, Deserialize, Reflect)]
pub struct SubEmitterConfig {
    /// When the sub-emitter triggers.
    pub mode: SubEmitterMode,
    /// Index of the target emitter (within the same [`ParticleSystemAsset`]) to spawn from.
    pub target_emitter: usize,
    /// How often particles are emitted from the sub-emitter, in seconds.
    ///
    /// Only used when [`mode`](Self::mode) is [`SubEmitterMode::Constant`]. Defaults to `4.0`.
    #[serde(default = "default_sub_emitter_frequency")]
    pub frequency: f32,
    /// The number of particles to spawn per trigger event. Defaults to `1`.
    #[serde(default = "default_sub_emitter_amount")]
    pub amount: u32,
    /// If `true`, the sub-emitted particles inherit the parent particle's velocity.
    ///
    /// Defaults to `false`.
    #[serde(default, skip_serializing_if = "is_false")]
    pub keep_velocity: bool,
}

impl Default for SubEmitterConfig {
    fn default() -> Self {
        Self {
            mode: SubEmitterMode::Constant,
            target_emitter: 0,
            frequency: default_sub_emitter_frequency(),
            amount: default_sub_emitter_amount(),
            keep_velocity: false,
        }
    }
}

/// The 3D shape of a particle collider.
#[derive(Debug, Clone, Serialize, Deserialize, Reflect)]
pub enum ParticlesColliderShape3D {
    /// An axis-aligned box collider.
    Box {
        /// Full size of the box along each axis.
        size: Vec3,
    },
    /// A sphere collider.
    Sphere {
        /// Radius of the sphere. Defaults to `1.0`.
        radius: f32,
    },
}

impl Default for ParticlesColliderShape3D {
    fn default() -> Self {
        Self::Sphere { radius: 1.0 }
    }
}

/// Serializable data for a particle collider.
#[derive(Debug, Clone, Serialize, Deserialize, Reflect)]
pub struct ColliderData {
    /// Display name for this collider.
    pub name: String,
    /// Whether this collider is active. Defaults to `true`.
    #[serde(default = "default_enabled", skip_serializing_if = "is_true")]
    pub enabled: bool,
    /// The collision shape.
    pub shape: ParticlesColliderShape3D,
    /// Position offset relative to the particle system entity. Defaults to [`Vec3::ZERO`].
    #[serde(default)]
    pub position: Vec3,
}

impl Default for ColliderData {
    fn default() -> Self {
        Self {
            name: "Collider".to_string(),
            enabled: true,
            shape: ParticlesColliderShape3D::default(),
            position: Vec3::ZERO,
        }
    }
}

/// Attribution information for a particle system.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParticleSystemAuthors {
    /// The original creator this effect was inspired by.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub inspired_by: Option<String>,
    /// The person who submitted or ported this effect.
    pub submitted_by: String,
}

/// A complete particle system asset, loadable from RON files.
///
/// Contains one or more emitters and optional colliders that together define a
/// particle effect. Load this asset and reference it from a [`ParticleSystem3D`](crate::ParticleSystem3D)
/// or [`ParticleSystem2D`](crate::ParticleSystem2D) component to render the effect.
#[derive(Asset, TypePath, Debug, Clone, Serialize, Deserialize)]
pub struct ParticleSystemAsset {
    sprinkles_version: String,
    /// Display name for this particle system.
    pub name: String,
    /// Whether this is a 3D or 2D particle system.
    pub dimension: ParticleSystemDimension,
    /// The list of emitters that make up this particle system.
    pub emitters: Vec<EmitterData>,
    /// Optional colliders that particles can interact with.
    #[serde(default)]
    pub colliders: Vec<ColliderData>,
    /// Optional attribution information.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub authors: Option<ParticleSystemAuthors>,
}

impl ParticleSystemAsset {
    /// Creates a new particle system asset with the current format version.
    pub fn new(
        name: String,
        dimension: ParticleSystemDimension,
        emitters: Vec<EmitterData>,
        colliders: Vec<ColliderData>,
        authors: Option<ParticleSystemAuthors>,
    ) -> Self {
        Self {
            sprinkles_version: current_format_version().to_string(),
            name,
            dimension,
            emitters,
            colliders,
            authors,
        }
    }

    /// Validates this asset's `sprinkles_version` against the current format version.
    ///
    /// If the version is outdated but compatible, it is automatically upgraded.
    /// Returns the original [`VersionStatus`] so the caller can react accordingly.
    pub fn try_upgrade_version(&mut self) -> VersionStatus {
        let status = versioning::validate_version(&self.sprinkles_version);
        if matches!(status, VersionStatus::Outdated { .. }) {
            self.sprinkles_version = current_format_version().to_string();
        }
        status
    }
}
