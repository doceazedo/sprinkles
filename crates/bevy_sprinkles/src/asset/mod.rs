mod curve;
mod gradient;
mod particle_material;
pub(crate) mod serde_helpers;
/// Asset format versioning, validation, and migration.
pub mod versions;

pub use curve::{Curve, CurveEasing, CurveMode, CurvePoint, CurveTexture};
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
use versions::current_format_version;

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
    /// The asset file could not be parsed or migrated.
    #[error("{0}")]
    Migration(#[from] versions::MigrationError),
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

        let result = versions::migrate(&bytes)?;

        if result.was_migrated {
            let path = load_context.path();
            warn!(
                "{path:?}: migrated asset to sprinkles_version \"{}\"",
                current_format_version()
            );
        }

        Ok(result.asset)
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
#[serde(default)]
pub struct EmitterTime {
    /// The amount of time each particle will exist, in seconds.
    ///
    /// The effective emission rate is `particles_amount / lifetime` particles per second.
    /// Defaults to `1.0`.
    pub lifetime: f32,
    /// Particle lifetime randomness ratio.
    ///
    /// The actual lifetime of each particle is `lifetime * (1.0 - rand() * lifetime_randomness)`.
    /// For example, a value of `0.4` scales each particle's lifetime between 60% and 100% of
    /// the configured [`lifetime`](Self::lifetime). Defaults to `0.0`.
    #[serde(skip_serializing_if = "is_zero_f32")]
    pub lifetime_randomness: f32,
    /// Time in seconds to wait before the emitter starts spawning particles.
    ///
    /// Defaults to `0.0`.
    #[serde(skip_serializing_if = "is_zero_f32")]
    pub delay: f32,
    /// If `true`, only one emission cycle will occur: exactly `particles_amount` particles
    /// will be emitted, and then the emitter stops.
    ///
    /// Defaults to `false`.
    #[serde(skip_serializing_if = "is_false")]
    pub one_shot: bool,
    /// Time ratio between each emission, from `0.0` to `1.0`.
    ///
    /// If `0.0`, particles are emitted continuously over the lifetime. If `1.0`, all
    /// particles are emitted simultaneously at the start of each cycle. Defaults to `0.0`.
    #[serde(skip_serializing_if = "is_zero_f32")]
    pub explosiveness: f32,
    /// Emission randomness ratio.
    ///
    /// Adds randomness to the timing of individual particle spawns within each cycle.
    /// Defaults to `0.0`.
    #[serde(skip_serializing_if = "is_zero_f32")]
    pub spawn_time_randomness: f32,
    /// Fixed frame rate for the particle simulation, in frames per second.
    ///
    /// When set to a non-zero value, the particle system updates at this fixed rate
    /// instead of every frame. This does not slow down the simulation itself, only
    /// how often it is evaluated. Defaults to `0` (updates every frame).
    #[serde(skip_serializing_if = "is_zero_u32")]
    pub fixed_fps: u32,
    /// Optional fixed random seed for deterministic particle behavior.
    ///
    /// When set, the particle system will produce the same visual result across
    /// replays, which is useful for cinematics or testing. Defaults to `None`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fixed_seed: Option<u32>,
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

/// The initial transform applied when spawning a particle system, emitter, or collider.
///
/// Used only during spawning if no [`Transform`] component is already present on the entity. To change the transform at runtime, modify the entity's [`Transform`] directly.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
#[serde(default)]
pub struct InitialTransform {
    /// Position of the entity.
    #[serde(skip_serializing_if = "is_zero_vec3")]
    pub translation: Vec3,
    /// Rotation of the entity.
    ///
    /// Expects `EulerRot::ZYX` where X = roll, Y = pitch, and Z = yaw.
    #[serde(skip_serializing_if = "is_zero_vec3")]
    pub rotation: Vec3,
    /// Scale factors.
    #[serde(skip_serializing_if = "is_one_vec3")]
    pub scale: Vec3,
}

impl Default for InitialTransform {
    fn default() -> Self {
        Self {
            translation: Vec3::ZERO,
            rotation: Vec3::ZERO,
            scale: Vec3::ONE,
        }
    }
}

impl InitialTransform {
    pub(crate) fn should_skip(t: &Self) -> bool {
        t.translation == Vec3::ZERO && t.rotation == Vec3::ZERO && t.scale == Vec3::ONE
    }

    /// Converts this initial transform into a Bevy [`Transform`].
    pub fn to_transform(&self) -> Transform {
        let roll = self.rotation.x.to_radians();
        let pitch = self.rotation.y.to_radians();
        let yaw = self.rotation.z.to_radians();
        Transform {
            translation: self.translation,
            rotation: Quat::from_euler(EulerRot::ZYX, yaw, pitch, roll),
            scale: self.scale,
        }
    }
}

/// Complete configuration for a single particle emitter.
///
/// An emitter is the source that creates particles. It controls how, where, and when
/// particles are spawned, as well as their visual properties and physical behavior
/// over their lifetime.
#[derive(Debug, Clone, Serialize, Deserialize, Reflect)]
#[serde(default)]
pub struct EmitterData {
    /// Display name for this emitter.
    pub name: String,
    /// Whether this emitter is active. Disabled emitters do not spawn particles.
    ///
    /// Defaults to `true`.
    #[serde(skip_serializing_if = "is_true")]
    pub enabled: bool,

    /// Initial transform applied when spawning this emitter.
    ///
    /// Only used during spawning if no [`Transform`] is already present.
    /// To change the transform at runtime, modify the emitter entity's [`Transform`] directly.
    #[serde(skip_serializing_if = "InitialTransform::should_skip")]
    pub initial_transform: InitialTransform,

    /// Timing and lifecycle settings (lifetime, delay, one-shot, etc.).
    pub time: EmitterTime,

    /// Draw pass configuration (mesh, material, draw order).
    pub draw_pass: EmitterDrawPass,

    /// Emission shape and particle count settings.
    pub emission: EmitterEmission,

    /// Particle scale range and scale-over-lifetime curve.
    pub scale: EmitterScale,

    /// Initial particle rotation angle and angle-over-lifetime curve.
    #[serde(skip_serializing_if = "EmitterAngle::should_skip")]
    pub angle: EmitterAngle,

    /// Color and alpha settings, including gradients and curves over lifetime.
    pub colors: EmitterColors,

    /// Velocity settings (direction, spread, radial/angular velocity, etc.).
    pub velocities: EmitterVelocities,

    /// Acceleration settings (gravity).
    pub accelerations: EmitterAccelerations,

    /// Turbulence noise settings for varying particle movement.
    #[serde(skip_serializing_if = "EmitterTurbulence::should_skip")]
    pub turbulence: EmitterTurbulence,

    /// Collision behavior settings.
    pub collision: EmitterCollision,

    /// Optional sub-emitter configuration for spawning secondary particles.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sub_emitter: Option<SubEmitterConfig>,

    /// Trail configuration for this emitter.
    #[serde(skip_serializing_if = "EmitterTrail::should_skip")]
    pub trail: EmitterTrail,

    /// Bitflags controlling per-particle behavior (Y rotation, Z-axis disable, etc.).
    #[reflect(ignore)]
    pub particle_flags: ParticleFlags,
}

impl Default for EmitterData {
    fn default() -> Self {
        Self {
            name: "Emitter".to_string(),
            enabled: true,
            initial_transform: InitialTransform::default(),
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
            trail: EmitterTrail::default(),
            particle_flags: ParticleFlags::empty(),
        }
    }
}

impl EmitterData {
    pub(crate) fn trail_size(&self) -> u32 {
        if !self.trail.enabled {
            return 1;
        }
        self.draw_pass.mesh.trail_sections().unwrap_or(1)
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
#[serde(default)]
pub struct EmitterDrawPass {
    /// The order in which particles are drawn. Defaults to [`DrawOrder::Index`].
    #[serde(skip_serializing_if = "DrawOrder::is_default")]
    pub draw_order: DrawOrder,
    /// The mesh shape used to render each particle.
    pub mesh: ParticleMesh,
    /// The material applied to the particle mesh. Defaults to a standard PBR material.
    pub material: DrawPassMaterial,
    /// Whether particles cast shadows. Defaults to `true`.
    #[serde(skip_serializing_if = "is_true")]
    pub shadow_caster: bool,
    /// Optional transform alignment mode for particles. When `None`, no special
    /// alignment is applied.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transform_align: Option<TransformAlign>,
    /// Whether particles use local coordinates and follow the emitter's transform.
    ///
    /// When `false` (default), particles are emitted into world space and remain
    /// at their world position even when the emitter moves. When `true`, particles
    /// are simulated in the emitter's local space and follow the emitter.
    ///
    /// Defaults to `false`.
    #[serde(skip_serializing_if = "is_false")]
    pub use_local_coords: bool,
}

impl Default for EmitterDrawPass {
    fn default() -> Self {
        Self {
            draw_order: DrawOrder::default(),
            mesh: ParticleMesh::default(),
            material: DrawPassMaterial::default(),
            shadow_caster: true,
            transform_align: None,
            use_local_coords: false,
        }
    }
}

/// The axis a quad particle mesh faces by default.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default, PartialEq, Eq, Hash, Reflect)]
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
        /// Number of longitudinal segments around the sphere. Defaults to `32`.
        #[serde(default = "default_sphere_segments")]
        segments: u32,
        /// Number of latitudinal rings on the sphere. Defaults to `16`.
        #[serde(default = "default_sphere_rings")]
        rings: u32,
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
    /// A tube-shaped trail mesh.
    TubeTrail {
        /// Radius of the tube cross-section. Defaults to `0.5`.
        radius: f32,
        /// Number of radial segments around the tube. Defaults to `8`.
        radial_steps: u32,
        /// Number of trail sections along the tube length. Defaults to `8`.
        sections: u32,
        /// Number of ring subdivisions within each section. Defaults to `1`.
        #[serde(default = "default_section_rings")]
        section_rings: u32,
    },
    /// A ribbon-shaped trail mesh.
    RibbonTrail {
        /// Half-width of the ribbon. Defaults to `1.0`.
        size: f32,
        /// Number of trail sections along the ribbon length. Defaults to `8`.
        sections: u32,
        /// Number of ring subdivisions within each section. Defaults to `1`.
        #[serde(default = "default_section_rings")]
        section_rings: u32,
        /// The ribbon cross-section shape. Defaults to [`RibbonTrailShape::Flat`].
        #[serde(default)]
        shape: RibbonTrailShape,
    },
}

impl ParticleMesh {
    /// Returns `true` if this is a trail mesh variant.
    pub fn is_trail(&self) -> bool {
        matches!(self, Self::TubeTrail { .. } | Self::RibbonTrail { .. })
    }

    /// Returns the number of trail sections for trail mesh variants, or `None` for other meshes.
    pub fn trail_sections(&self) -> Option<u32> {
        match self {
            Self::TubeTrail { sections, .. } | Self::RibbonTrail { sections, .. } => {
                Some(*sections)
            }
            _ => None,
        }
    }
}

fn default_quad_size() -> Vec2 {
    Vec2::ONE
}

fn default_sphere_radius() -> f32 {
    1.0
}

fn default_sphere_segments() -> u32 {
    32
}

fn default_sphere_rings() -> u32 {
    16
}

fn default_section_rings() -> u32 {
    1
}

fn default_prism_left_to_right() -> f32 {
    0.5
}

fn default_prism_size() -> Vec3 {
    Vec3::splat(1.0)
}

impl Eq for ParticleMesh {}

impl std::hash::Hash for ParticleMesh {
    fn hash<H: std::hash::Hasher>(&self, hasher: &mut H) {
        std::mem::discriminant(self).hash(hasher);
        match self {
            Self::Quad {
                orientation,
                size,
                subdivide,
            } => {
                orientation.hash(hasher);
                size.x.to_bits().hash(hasher);
                size.y.to_bits().hash(hasher);
                subdivide.x.to_bits().hash(hasher);
                subdivide.y.to_bits().hash(hasher);
            }
            Self::Sphere {
                radius,
                segments,
                rings,
            } => {
                radius.to_bits().hash(hasher);
                segments.hash(hasher);
                rings.hash(hasher);
            }
            Self::Cuboid { half_size } => {
                half_size.x.to_bits().hash(hasher);
                half_size.y.to_bits().hash(hasher);
                half_size.z.to_bits().hash(hasher);
            }
            Self::Cylinder {
                top_radius,
                bottom_radius,
                height,
                radial_segments,
                rings,
                cap_top,
                cap_bottom,
            } => {
                top_radius.to_bits().hash(hasher);
                bottom_radius.to_bits().hash(hasher);
                height.to_bits().hash(hasher);
                radial_segments.hash(hasher);
                rings.hash(hasher);
                cap_top.hash(hasher);
                cap_bottom.hash(hasher);
            }
            Self::Prism {
                left_to_right,
                size,
                subdivide,
            } => {
                left_to_right.to_bits().hash(hasher);
                size.x.to_bits().hash(hasher);
                size.y.to_bits().hash(hasher);
                size.z.to_bits().hash(hasher);
                subdivide.x.to_bits().hash(hasher);
                subdivide.y.to_bits().hash(hasher);
                subdivide.z.to_bits().hash(hasher);
            }
            Self::TubeTrail {
                radius,
                radial_steps,
                sections,
                section_rings,
            } => {
                radius.to_bits().hash(hasher);
                radial_steps.hash(hasher);
                sections.hash(hasher);
                section_rings.hash(hasher);
            }
            Self::RibbonTrail {
                size,
                sections,
                section_rings,
                shape,
            } => {
                size.to_bits().hash(hasher);
                sections.hash(hasher);
                section_rings.hash(hasher);
                shape.hash(hasher);
            }
        }
    }
}

impl Default for ParticleMesh {
    fn default() -> Self {
        Self::default_sphere()
    }
}

impl ParticleMesh {
    /// Returns a default [`Quad`](Self::Quad) mesh.
    pub fn default_quad() -> Self {
        Self::Quad {
            orientation: QuadOrientation::default(),
            size: Vec2::ONE,
            subdivide: Vec2::ZERO,
        }
    }

    /// Returns a default [`Sphere`](Self::Sphere) mesh.
    pub fn default_sphere() -> Self {
        Self::Sphere {
            radius: 1.0,
            segments: 32,
            rings: 16,
        }
    }

    /// Returns a default [`Cuboid`](Self::Cuboid) mesh.
    pub fn default_cuboid() -> Self {
        Self::Cuboid {
            half_size: Vec3::splat(0.5),
        }
    }

    /// Returns a default [`Cylinder`](Self::Cylinder) mesh.
    pub fn default_cylinder() -> Self {
        Self::Cylinder {
            top_radius: 0.5,
            bottom_radius: 0.5,
            height: 1.0,
            radial_segments: 16,
            rings: 1,
            cap_top: true,
            cap_bottom: true,
        }
    }

    /// Returns a default [`Prism`](Self::Prism) mesh.
    pub fn default_prism() -> Self {
        Self::Prism {
            left_to_right: 0.5,
            size: Vec3::splat(1.0),
            subdivide: Vec3::ZERO,
        }
    }

    /// Returns a default [`TubeTrail`](Self::TubeTrail) mesh.
    pub fn default_tube_trail() -> Self {
        Self::TubeTrail {
            radius: 0.5,
            radial_steps: 8,
            sections: 8,
            section_rings: 1,
        }
    }

    /// Returns a default [`RibbonTrail`](Self::RibbonTrail) mesh.
    pub fn default_ribbon_trail() -> Self {
        Self::RibbonTrail {
            size: 1.0,
            sections: 8,
            section_rings: 1,
            shape: RibbonTrailShape::default(),
        }
    }
}

/// A minimum/maximum range of `f32` values, used to randomize particle properties.
///
/// When a particle is spawned, a random value between [`min`](Self::min) and
/// [`max`](Self::max) is selected. Defaults to `0.0..1.0`.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Reflect)]
#[serde(default)]
pub struct Range {
    /// Lower bound of the range. Defaults to `0.0`.
    pub min: f32,
    /// Upper bound of the range. Defaults to `1.0`.
    pub max: f32,
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

    /// Returns a default [`Sphere`](Self::Sphere) shape.
    pub fn default_sphere() -> Self {
        Self::Sphere { radius: 1.0 }
    }

    /// Returns a default [`SphereSurface`](Self::SphereSurface) shape.
    pub fn default_sphere_surface() -> Self {
        Self::SphereSurface { radius: 1.0 }
    }

    /// Returns a default [`Box`](Self::Box) shape.
    pub fn default_box() -> Self {
        Self::Box { extents: Vec3::ONE }
    }

    /// Returns a default [`Ring`](Self::Ring) shape.
    pub fn default_ring() -> Self {
        Self::Ring {
            axis: Vec3::Y,
            height: 0.0,
            radius: 1.0,
            inner_radius: 0.0,
        }
    }
}

/// Emission configuration: shape, offset, scale, and particle count.
#[derive(Debug, Clone, Serialize, Deserialize, Reflect)]
#[serde(default)]
pub struct EmitterEmission {
    /// Position offset of the emission shape in local space. Defaults to [`Vec3::ZERO`].
    #[serde(skip_serializing_if = "is_zero_vec3")]
    pub offset: Vec3,
    /// Scale of the emission shape in local space. Defaults to [`Vec3::ONE`].
    #[serde(skip_serializing_if = "is_one_vec3")]
    pub scale: Vec3,
    /// The shape of the emission region. Defaults to [`EmissionShape::Point`].
    #[serde(skip_serializing_if = "EmissionShape::is_default")]
    pub shape: EmissionShape,
    /// The number of particles to emit in one emission cycle.
    ///
    /// Higher values will increase GPU load. Defaults to `8`.
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

/// Particle scale configuration.
#[derive(Debug, Clone, Serialize, Deserialize, Reflect)]
#[serde(default)]
pub struct EmitterScale {
    /// The initial scale range applied to each particle.
    ///
    /// A random value between `min` and `max` is selected at spawn time.
    /// Defaults to `1.0..1.0`.
    pub range: Range,
    /// Optional curve that modulates each particle's scale over its lifetime.
    ///
    /// The curve value is multiplied with the initial scale.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scale_over_lifetime: Option<CurveTexture>,
}

impl Default for EmitterScale {
    fn default() -> Self {
        Self {
            range: Range { min: 1.0, max: 1.0 },
            scale_over_lifetime: None,
        }
    }
}

/// Color and alpha configuration for particles.
#[derive(Debug, Clone, Serialize, Deserialize, Reflect)]
#[serde(default)]
pub struct EmitterColors {
    /// Each particle's initial color. Can be a solid color or a gradient from which a random
    /// color is sampled at spawn time. Defaults to opaque white.
    pub initial_color: SolidOrGradientColor,
    /// Gradient that modulates each particle's color over its lifetime.
    ///
    /// The particle's initial color is multiplied by the gradient value at the
    /// corresponding lifetime position. Defaults to a constant white gradient.
    pub color_over_lifetime: Gradient,
    /// Optional curve that modulates each particle's alpha over its lifetime.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub alpha_over_lifetime: Option<CurveTexture>,
    /// Optional curve that modulates the emissive intensity over each particle's lifetime.
    #[serde(skip_serializing_if = "Option::is_none")]
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

/// A velocity value with an optional curve for animation over a particle's lifetime.
#[derive(Debug, Clone, Serialize, Deserialize, Reflect)]
#[serde(default)]
pub struct AnimatedVelocity {
    /// The initial velocity range. A random value between `min` and `max` is
    /// selected at spawn time. Defaults to zero.
    #[serde(skip_serializing_if = "Range::is_zero")]
    pub velocity: Range,
    /// Optional curve that modulates the velocity over each particle's lifetime.
    #[serde(skip_serializing_if = "Option::is_none")]
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
#[serde(default)]
pub struct EmitterAngle {
    /// The initial rotation angle range in degrees. A random value between `min` and
    /// `max` is applied to each particle at spawn time. Defaults to zero.
    #[serde(skip_serializing_if = "Range::is_zero")]
    pub range: Range,
    /// Optional curve that animates each particle's rotation over its lifetime.
    #[serde(skip_serializing_if = "Option::is_none")]
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
#[serde(default)]
pub struct EmitterVelocities {
    /// Unit vector specifying the base emission direction. Defaults to `Vec3::X`.
    pub initial_direction: Vec3,
    /// The angular spread in degrees. Each particle's initial direction varies from
    /// +spread to -spread relative to [`initial_direction`](Self::initial_direction).
    /// Defaults to `45.0`.
    pub spread: f32,
    /// Amount of spread flattening along the Y axis.
    ///
    /// A value of `0.0` means uniform conical spread; `1.0` flattens it into a disc.
    /// Defaults to `0.0`.
    #[serde(skip_serializing_if = "is_zero_f32")]
    pub flatness: f32,
    /// The initial velocity magnitude range. Each particle receives a random speed
    /// between `min` and `max`, applied in its emission direction. Defaults to zero.
    #[serde(skip_serializing_if = "Range::is_zero")]
    pub initial_velocity: Range,
    /// Radial velocity that pushes particles away from (or toward, if negative) the
    /// [`pivot`](Self::pivot) point.
    pub radial_velocity: AnimatedVelocity,
    /// Angular (rotation) velocity applied to each particle, in degrees per second.
    ///
    /// Only applied when [`ParticleFlags::DISABLE_Z`] or [`ParticleFlags::ROTATE_Y`] are set,
    /// or when using billboard rendering.
    pub angular_velocity: AnimatedVelocity,
    /// Orbital velocity that makes particles orbit around the [`pivot`](Self::pivot)
    /// point, in revolutions per second.
    pub orbit_velocity: AnimatedVelocity,
    /// Velocity along an arbitrary direction over each particle's lifetime.
    ///
    /// When a curve is set, the curve's XYZ channels provide the direction
    /// vector and the velocity range controls the magnitude. Without a curve,
    /// particles move along their initial emission direction.
    pub directional_velocity: AnimatedVelocity,
    /// The pivot point used to calculate radial and orbital velocity.
    ///
    /// Defaults to [`Vec3::ZERO`].
    #[serde(skip_serializing_if = "is_zero_vec3")]
    pub pivot: Vec3,
    /// Percentage of the emitter entity's velocity inherited by each particle when spawning.
    ///
    /// Defaults to `0.0`.
    #[serde(skip_serializing_if = "is_zero_f32")]
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
            orbit_velocity: AnimatedVelocity::default(),
            directional_velocity: AnimatedVelocity::default(),
            pivot: Vec3::ZERO,
            inherit_ratio: 0.0,
        }
    }
}

/// Acceleration forces applied to every particle.
#[derive(Debug, Clone, Serialize, Deserialize, Reflect)]
#[serde(default)]
pub struct EmitterAccelerations {
    /// Gravity vector applied to every particle, in units per second squared.
    ///
    /// Defaults to `(0.0, -9.8, 0.0)`.
    pub gravity: Vec3,
}

impl Default for EmitterAccelerations {
    fn default() -> Self {
        Self {
            gravity: Vec3::new(0.0, -9.8, 0.0),
        }
    }
}

/// Turbulence noise settings for varying particle movement based on position.
///
/// Turbulence uses a 3D noise pattern to displace particles, creating organic,
/// wind-like motion. Enabling turbulence has a significant performance cost on
/// the GPU.
#[derive(Debug, Clone, Serialize, Deserialize, Reflect)]
#[serde(default)]
pub struct EmitterTurbulence {
    /// Whether turbulence is enabled. Defaults to `false`.
    #[serde(skip_serializing_if = "is_false")]
    pub enabled: bool,
    /// The turbulence noise strength. Higher values produce a stronger, more
    /// contrasting flow pattern. Defaults to `1.0`.
    pub noise_strength: f32,
    /// Overall scale/frequency of the turbulence noise pattern.
    ///
    /// A small scale produces smaller features with more detail, while a large
    /// scale produces smoother noise with larger features. Defaults to `2.5`.
    pub noise_scale: f32,
    /// Scrolling velocity for the turbulence field, setting a directional trend
    /// for the noise pattern over time. Defaults to [`Vec3::ZERO`] (no scrolling).
    #[serde(skip_serializing_if = "is_zero_vec3")]
    pub noise_speed: Vec3,
    /// The in-place rate of change of the turbulence field.
    ///
    /// Controls how quickly the noise pattern varies over time. A value of `0.0`
    /// results in a fixed pattern. Defaults to `0.0`.
    #[serde(skip_serializing_if = "is_zero_f32")]
    pub noise_speed_random: f32,
    /// The range of turbulence influence applied to each particle.
    ///
    /// A random value between `min` and `max` is selected per particle, then
    /// multiplied by [`influence_over_lifetime`](Self::influence_over_lifetime)
    /// if provided. Defaults to `0.0..0.1`.
    pub influence: Range,
    /// Optional curve that modulates turbulence influence over each particle's lifetime.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub influence_over_lifetime: Option<CurveTexture>,
}

impl Default for EmitterTurbulence {
    fn default() -> Self {
        Self {
            enabled: false,
            noise_strength: 1.0,
            noise_scale: 2.5,
            noise_speed: Vec3::ZERO,
            noise_speed_random: 0.0,
            influence: Range { min: 0.0, max: 0.1 },
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
#[serde(default)]
pub struct EmitterCollision {
    /// The collision mode. When `None`, collision is disabled and particles pass
    /// through colliders. Defaults to `None`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mode: Option<EmitterCollisionMode>,
    /// If `true`, [`base_size`](Self::base_size) is multiplied by the particle's
    /// effective scale. Defaults to `false`.
    #[serde(skip_serializing_if = "is_false")]
    pub use_scale: bool,
    /// The base diameter for particle collision, in meters.
    ///
    /// If particles appear to sink into the ground, increase this value. If they
    /// appear to float above surfaces, decrease it. Particles always use a spherical
    /// collision shape. Defaults to `0.01`.
    pub base_size: f32,
}

impl Default for EmitterCollision {
    fn default() -> Self {
        Self {
            mode: None,
            base_size: 0.01,
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

/// Configuration for a sub-emitter that spawns secondary particles from parent particles.
///
/// Sub-emitters can be used to achieve effects such as fireworks, sparks on collision,
/// or bubbles popping into water drops.
#[derive(Debug, Clone, Serialize, Deserialize, Reflect)]
#[serde(default)]
pub struct SubEmitterConfig {
    /// When the sub-emitter triggers.
    pub mode: SubEmitterMode,
    /// Index of the target emitter (within the same [`ParticleSystemAsset`]) to spawn from.
    pub target_emitter: usize,
    /// How often particles are emitted from the sub-emitter, in seconds.
    ///
    /// Only used when [`mode`](Self::mode) is [`SubEmitterMode::Constant`]. Defaults to `4.0`.
    pub frequency: f32,
    /// The number of particles to spawn per trigger event. Defaults to `1`.
    pub amount: u32,
    /// If `true`, the sub-emitted particles inherit the parent particle's velocity.
    ///
    /// Defaults to `false`.
    #[serde(skip_serializing_if = "is_false")]
    pub keep_velocity: bool,
}

impl Default for SubEmitterConfig {
    fn default() -> Self {
        Self {
            mode: SubEmitterMode::Constant,
            target_emitter: 0,
            frequency: 4.0,
            amount: 1,
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
        Self::default_sphere()
    }
}

impl ParticlesColliderShape3D {
    /// Returns a default [`Sphere`](Self::Sphere) collider.
    pub fn default_sphere() -> Self {
        Self::Sphere { radius: 1.0 }
    }

    /// Returns a default [`Box`](Self::Box) collider.
    pub fn default_box() -> Self {
        Self::Box { size: Vec3::ONE }
    }
}

/// Serializable data for a particle collider.
#[derive(Debug, Clone, Serialize, Deserialize, Reflect)]
#[serde(default)]
pub struct ColliderData {
    /// Display name for this collider.
    pub name: String,
    /// Whether this collider is active. Defaults to `true`.
    #[serde(skip_serializing_if = "is_true")]
    pub enabled: bool,
    /// The collision shape.
    pub shape: ParticlesColliderShape3D,
    /// Initial transform applied when spawning this collider.
    ///
    /// Only used during spawning if no [`Transform`] is already present.
    /// To change the transform at runtime, modify the collider entity's [`Transform`] directly.
    #[serde(skip_serializing_if = "InitialTransform::should_skip")]
    pub initial_transform: InitialTransform,
}

impl Default for ColliderData {
    fn default() -> Self {
        Self {
            name: "Collider".to_string(),
            enabled: true,
            shape: ParticlesColliderShape3D::default(),
            initial_transform: InitialTransform::default(),
        }
    }
}

/// Trail configuration for an emitter.
///
/// When enabled, each particle leaves a visible trail behind it as it moves.
#[derive(Debug, Clone, Serialize, Deserialize, Reflect)]
#[serde(default)]
pub struct EmitterTrail {
    /// Whether trails are enabled. Defaults to `false`.
    #[serde(skip_serializing_if = "is_false")]
    pub enabled: bool,
    /// The amount of time the particle's trail should represent, in seconds.
    ///
    /// Defaults to `0.3`.
    pub stretch_time: f32,
    /// Optional curve that controls trail thickness from head (0) to tail (1).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thickness_curve: Option<CurveTexture>,
}

impl Default for EmitterTrail {
    fn default() -> Self {
        Self {
            enabled: false,
            stretch_time: 0.3,
            thickness_curve: None,
        }
    }
}

impl EmitterTrail {
    pub(crate) fn should_skip(&self) -> bool {
        if self.enabled {
            return false;
        }
        let d = Self::default();
        self.stretch_time == d.stretch_time && self.thickness_curve.is_none()
    }
}

/// The shape of a ribbon trail mesh cross-section.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default, PartialEq, Eq, Hash, Reflect)]
pub enum RibbonTrailShape {
    /// A single flat quad strip.
    #[default]
    Flat,
    /// Two perpendicular quad strips forming a cross.
    Cross,
}

/// Editor-specific metadata.
#[derive(Debug, Clone, Default, Serialize, Deserialize, Reflect)]
pub struct SprinklesEditorData {
    /// Known asset folder paths for resolving [`TextureRef::Asset`] references.
    ///
    /// Multiple entries allow different users or devices to open the same
    /// project from different locations. At load time the first path that
    /// exists on disk is used.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub assets_folder: Vec<String>,
}

impl SprinklesEditorData {
    /// Returns `true` if no editor metadata has been recorded.
    pub fn is_empty(&self) -> bool {
        self.assets_folder.is_empty()
    }
}

/// Attribution information for a particle system.
#[derive(Debug, Clone, Default, Serialize, Deserialize, Reflect)]
pub struct ParticleSystemAuthors {
    /// The original creator this effect was inspired by.
    #[serde(default, skip_serializing_if = "is_empty_string")]
    pub inspired_by: String,
    /// The person who submitted or ported this effect.
    #[serde(default, skip_serializing_if = "is_empty_string")]
    pub submitted_by: String,
}

impl ParticleSystemAuthors {
    /// Returns `true` if both fields are empty
    pub fn is_empty(&self) -> bool {
        self.inspired_by.is_empty() && self.submitted_by.is_empty()
    }
}

/// A complete particle system asset, loadable from RON files.
///
/// Contains one or more emitters and optional colliders that together define a
/// particle effect. Load this asset and reference it from a [`ParticleSystem3D`](crate::ParticleSystem3D)
/// or [`ParticleSystem2D`](crate::ParticleSystem2D) component to render the effect.
#[derive(Asset, Debug, Clone, Serialize, Deserialize, Reflect)]
pub struct ParticleSystemAsset {
    sprinkles_version: String,
    /// Display name for this particle system.
    pub name: String,
    /// Whether this is a 3D or 2D particle system.
    pub dimension: ParticleSystemDimension,
    /// Initial transform applied when spawning this particle system.
    ///
    /// Only used during spawning if no [`Transform`] is already present on the entity.
    /// To change the transform at runtime, modify the entity's [`Transform`] directly.
    #[serde(default, skip_serializing_if = "InitialTransform::should_skip")]
    pub initial_transform: InitialTransform,
    /// The list of emitters that make up this particle system.
    pub emitters: Vec<EmitterData>,
    /// Optional colliders that particles can interact with.
    #[serde(default)]
    pub colliders: Vec<ColliderData>,
    /// Whether to despawn the particle system entity when all one-shot emitters finish.
    ///
    /// Defaults to `false`.
    #[serde(default, skip_serializing_if = "is_false")]
    pub despawn_on_finish: bool,
    /// Attribution information.
    #[serde(default, skip_serializing_if = "ParticleSystemAuthors::is_empty")]
    pub authors: ParticleSystemAuthors,
    /// Editor-specific metadata.
    #[serde(default, skip_serializing_if = "SprinklesEditorData::is_empty")]
    pub sprinkles_editor: SprinklesEditorData,
}

impl ParticleSystemAsset {
    /// Creates a new particle system asset with the current format version.
    pub fn new(
        name: String,
        dimension: ParticleSystemDimension,
        initial_transform: InitialTransform,
        emitters: Vec<EmitterData>,
        colliders: Vec<ColliderData>,
        despawn_on_finish: bool,
        authors: ParticleSystemAuthors,
    ) -> Self {
        Self {
            sprinkles_version: current_format_version().to_string(),
            name,
            dimension,
            initial_transform,
            emitters,
            colliders,
            despawn_on_finish,
            authors,
            sprinkles_editor: SprinklesEditorData::default(),
        }
    }
}
