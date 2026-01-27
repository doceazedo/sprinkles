use bevy::{
    asset::{io::Reader, AssetLoader, LoadContext},
    prelude::*,
    render::alpha::AlphaMode,
};
use bitflags::bitflags;
use serde::{Deserialize, Serialize};
use std::hash::{Hash, Hasher};
use thiserror::Error;

// asset loader

#[derive(Default, TypePath)]
pub struct ParticleSystemAssetLoader;

#[non_exhaustive]
#[derive(Debug, Error)]
pub enum ParticleSystemAssetLoaderError {
    #[error("Could not load asset: {0}")]
    Io(#[from] std::io::Error),
    #[error("Could not parse RON: {0}")]
    Ron(#[from] ron::error::SpannedError),
}

impl AssetLoader for ParticleSystemAssetLoader {
    type Asset = ParticleSystemAsset;
    type Settings = ();
    type Error = ParticleSystemAssetLoaderError;

    async fn load(
        &self,
        reader: &mut dyn Reader,
        _settings: &(),
        _load_context: &mut LoadContext<'_>,
    ) -> Result<Self::Asset, Self::Error> {
        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes).await?;
        let asset = ron::de::from_bytes::<ParticleSystemAsset>(&bytes)?;
        Ok(asset)
    }

    fn extensions(&self) -> &[&str] {
        &["ron"]
    }
}

// asset format

bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
    #[serde(transparent)]
    pub struct ParticleFlags: u32 {
        const ALIGN_Y_TO_VELOCITY = 1 << 0;

        // TODO: requires implementing angular velocity
        // const ROTATE_Y = 1 << 1;

        const DISABLE_Z = 1 << 2;

        // TODO: requires implementing damping
        // const DAMPING_AS_FRICTION = 1 << 3;
    }
}

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

    #[serde(default)]
    pub position: Vec3,

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
            position: Vec3::ZERO,
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
    #[serde(default)]
    pub material: DrawPassMaterial,
    #[serde(default)]
    pub shadow_caster: bool,
}

impl Default for EmitterDrawPass {
    fn default() -> Self {
        Self {
            mesh: ParticleMesh::default(),
            material: DrawPassMaterial::default(),
            shadow_caster: true,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default, PartialEq)]
pub enum QuadOrientation {
    FaceX,
    FaceY,
    #[default]
    FaceZ,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ParticleMesh {
    Quad {
        orientation: QuadOrientation,
    },
    Sphere {
        #[serde(default)]
        radius: f32,
    },
    Cuboid {
        half_size: Vec3,
    },
    Cylinder {
        top_radius: f32,
        bottom_radius: f32,
        height: f32,
        radial_segments: u32,
        rings: u32,
        cap_top: bool,
        cap_bottom: bool,
    },
}

impl Default for ParticleMesh {
    fn default() -> Self {
        Self::Sphere {
            radius: 1.0,
        }
    }
}

// material types

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default, PartialEq)]
pub enum SerializableAlphaMode {
    Opaque,
    Mask { cutoff: f32 },
    #[default]
    Blend,
    Premultiplied,
    Add,
    Multiply,
    AlphaToCoverage,
}

impl From<SerializableAlphaMode> for AlphaMode {
    fn from(mode: SerializableAlphaMode) -> Self {
        match mode {
            SerializableAlphaMode::Opaque => AlphaMode::Opaque,
            SerializableAlphaMode::Mask { cutoff } => AlphaMode::Mask(cutoff),
            SerializableAlphaMode::Blend => AlphaMode::Blend,
            SerializableAlphaMode::Premultiplied => AlphaMode::Premultiplied,
            SerializableAlphaMode::Add => AlphaMode::Add,
            SerializableAlphaMode::Multiply => AlphaMode::Multiply,
            SerializableAlphaMode::AlphaToCoverage => AlphaMode::AlphaToCoverage,
        }
    }
}

impl From<AlphaMode> for SerializableAlphaMode {
    fn from(mode: AlphaMode) -> Self {
        match mode {
            AlphaMode::Opaque => SerializableAlphaMode::Opaque,
            AlphaMode::Mask(cutoff) => SerializableAlphaMode::Mask { cutoff },
            AlphaMode::Blend => SerializableAlphaMode::Blend,
            AlphaMode::Premultiplied => SerializableAlphaMode::Premultiplied,
            AlphaMode::Add => SerializableAlphaMode::Add,
            AlphaMode::Multiply => SerializableAlphaMode::Multiply,
            AlphaMode::AlphaToCoverage => SerializableAlphaMode::AlphaToCoverage,
        }
    }
}

fn default_base_color() -> [f32; 4] {
    [1.0, 1.0, 1.0, 1.0]
}

fn default_perceptual_roughness() -> f32 {
    0.5
}

fn default_alpha_mode() -> SerializableAlphaMode {
    SerializableAlphaMode::Opaque
}

fn default_reflectance() -> f32 {
    0.5
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StandardParticleMaterial {
    #[serde(default = "default_base_color")]
    pub base_color: [f32; 4],

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub base_color_texture: Option<String>,

    #[serde(default)]
    pub emissive: [f32; 4],

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub emissive_texture: Option<String>,

    #[serde(default = "default_perceptual_roughness")]
    pub perceptual_roughness: f32,

    #[serde(default)]
    pub metallic: f32,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metallic_roughness_texture: Option<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub normal_map_texture: Option<String>,

    #[serde(default = "default_alpha_mode")]
    pub alpha_mode: SerializableAlphaMode,

    #[serde(default)]
    pub double_sided: bool,

    #[serde(default)]
    pub unlit: bool,

    #[serde(default)]
    pub fog_enabled: bool,

    #[serde(default = "default_reflectance")]
    pub reflectance: f32,
}

impl Default for StandardParticleMaterial {
    fn default() -> Self {
        Self {
            base_color: default_base_color(),
            base_color_texture: None,
            emissive: [0.0, 0.0, 0.0, 1.0],
            emissive_texture: None,
            perceptual_roughness: default_perceptual_roughness(),
            metallic: 0.0,
            metallic_roughness_texture: None,
            normal_map_texture: None,
            alpha_mode: default_alpha_mode(),
            double_sided: false,
            unlit: false,
            fog_enabled: true,
            reflectance: default_reflectance(),
        }
    }
}

impl StandardParticleMaterial {
    pub fn to_standard_material(&self, asset_server: &AssetServer) -> StandardMaterial {
        let base_color = Color::linear_rgba(
            self.base_color[0],
            self.base_color[1],
            self.base_color[2],
            self.base_color[3],
        );

        let emissive = Color::linear_rgba(
            self.emissive[0],
            self.emissive[1],
            self.emissive[2],
            self.emissive[3],
        );

        StandardMaterial {
            base_color,
            base_color_texture: self
                .base_color_texture
                .as_ref()
                .map(|path| asset_server.load(path)),
            emissive: emissive.into(),
            emissive_texture: self
                .emissive_texture
                .as_ref()
                .map(|path| asset_server.load(path)),
            perceptual_roughness: self.perceptual_roughness,
            metallic: self.metallic,
            metallic_roughness_texture: self
                .metallic_roughness_texture
                .as_ref()
                .map(|path| asset_server.load(path)),
            normal_map_texture: self
                .normal_map_texture
                .as_ref()
                .map(|path| asset_server.load(path)),
            alpha_mode: self.alpha_mode.into(),
            double_sided: self.double_sided,
            unlit: self.unlit,
            fog_enabled: self.fog_enabled,
            reflectance: self.reflectance,
            ..default()
        }
    }

    /// Creates a StandardParticleMaterial from a StandardMaterial.
    /// Note: texture paths cannot be recovered from handles and will be set to None.
    pub fn from_standard_material(material: &StandardMaterial) -> Self {
        let base_color = material.base_color.to_linear();
        let emissive = material.emissive;

        Self {
            base_color: [base_color.red, base_color.green, base_color.blue, base_color.alpha],
            base_color_texture: None,
            emissive: [emissive.red, emissive.green, emissive.blue, emissive.alpha],
            emissive_texture: None,
            perceptual_roughness: material.perceptual_roughness,
            metallic: material.metallic,
            metallic_roughness_texture: None,
            normal_map_texture: None,
            alpha_mode: material.alpha_mode.into(),
            double_sided: material.double_sided,
            unlit: material.unlit,
            fog_enabled: material.fog_enabled,
            reflectance: material.reflectance,
        }
    }

    pub fn cache_key(&self) -> u64 {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        for c in self.base_color {
            c.to_bits().hash(&mut hasher);
        }
        self.base_color_texture.hash(&mut hasher);
        for c in self.emissive {
            c.to_bits().hash(&mut hasher);
        }
        self.emissive_texture.hash(&mut hasher);
        self.perceptual_roughness.to_bits().hash(&mut hasher);
        self.metallic.to_bits().hash(&mut hasher);
        self.metallic_roughness_texture.hash(&mut hasher);
        self.normal_map_texture.hash(&mut hasher);
        std::mem::discriminant(&self.alpha_mode).hash(&mut hasher);
        if let SerializableAlphaMode::Mask { cutoff } = self.alpha_mode {
            cutoff.to_bits().hash(&mut hasher);
        }
        self.double_sided.hash(&mut hasher);
        self.unlit.hash(&mut hasher);
        self.fog_enabled.hash(&mut hasher);
        self.reflectance.to_bits().hash(&mut hasher);
        hasher.finish()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DrawPassMaterial {
    Standard(StandardParticleMaterial),
    CustomShader {
        vertex_shader: Option<String>,
        fragment_shader: Option<String>,
    },
}

impl Default for DrawPassMaterial {
    fn default() -> Self {
        Self::Standard(StandardParticleMaterial::default())
    }
}

impl DrawPassMaterial {
    pub fn cache_key(&self) -> u64 {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        match self {
            Self::Standard(mat) => {
                0u8.hash(&mut hasher);
                mat.cache_key().hash(&mut hasher);
            }
            Self::CustomShader {
                vertex_shader,
                fragment_shader,
            } => {
                1u8.hash(&mut hasher);
                vertex_shader.hash(&mut hasher);
                fragment_shader.hash(&mut hasher);
            }
        }
        hasher.finish()
    }
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
pub struct ParticleProcessAccelerations {
    #[serde(default = "default_gravity")]
    pub gravity: Vec3,
}

fn default_gravity() -> Vec3 {
    Vec3::new(0.0, -9.8, 0.0)
}

impl Default for ParticleProcessAccelerations {
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
}

impl Default for ParticleProcessSpawn {
    fn default() -> Self {
        Self {
            position: ParticleProcessSpawnPosition::default(),
            velocity: ParticleProcessSpawnVelocity::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnimatedVelocity {
    #[serde(default)]
    pub value: Range,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub curve: Option<SplineCurveConfig>,
}

impl Default for AnimatedVelocity {
    fn default() -> Self {
        Self {
            value: Range::default(),
            curve: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParticleProcessAnimVelocities {
    // TODO: angular_velocity: AnimatedVelocity,
    #[serde(default)]
    pub radial_velocity: AnimatedVelocity,
    // TODO: directional_velocity: AnimatedVelocity,
    // TODO: orbit_velocity: AnimatedVelocity,
    // TODO: velocity_limit: Option<SplineCurve>,
}

impl Default for ParticleProcessAnimVelocities {
    fn default() -> Self {
        Self {
            radial_velocity: AnimatedVelocity::default(),
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct Knot {
    pub position: f32,
    pub value: f32,
}

impl Knot {
    pub fn new(position: f32, value: f32) -> Self {
        Self { position, value }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub enum SplineCurve {
    Custom(Vec<Knot>),

    #[default]
    Constant,

    // increase curves (0 -> 1)
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

    // decrease curves (1 -> 0)
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
    pub fn cache_key(&self) -> u64 {
        use std::hash::{Hash, Hasher};

        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        match self {
            Self::Custom(knots) => {
                0u8.hash(&mut hasher);
                for knot in knots {
                    knot.position.to_bits().hash(&mut hasher);
                    knot.value.to_bits().hash(&mut hasher);
                }
            }
            _ => {
                std::mem::discriminant(self).hash(&mut hasher);
            }
        }
        hasher.finish()
    }

    pub fn is_constant(&self) -> bool {
        matches!(self, Self::Constant)
    }
}

fn default_curve_min() -> f32 {
    0.0
}

fn default_curve_max() -> f32 {
    1.0
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SplineCurveConfig {
    pub curve: SplineCurve,
    #[serde(default = "default_curve_min")]
    pub min_value: f32,
    #[serde(default = "default_curve_max")]
    pub max_value: f32,
}

impl Default for SplineCurveConfig {
    fn default() -> Self {
        Self {
            curve: SplineCurve::default(),
            min_value: 0.0,
            max_value: 1.0,
        }
    }
}

impl SplineCurveConfig {
    pub fn is_constant(&self) -> bool {
        self.curve.is_constant()
    }

    pub fn cache_key(&self) -> u64 {
        self.curve.cache_key()
    }

    pub fn to_knots(&self) -> Vec<Knot> {
        self.curve.to_knots()
    }
}

impl SplineCurve {
    pub fn to_knots(&self) -> Vec<Knot> {
        match self {
            Self::Custom(knots) => knots.clone(),
            Self::Constant => vec![Knot::new(0.0, 1.0), Knot::new(1.0, 1.0)],
            _ => {
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

    fn sample_preset(&self, t: f32) -> f32 {
        use std::f32::consts::{FRAC_PI_2, PI};

        match self {
            Self::Custom(_) => 1.0,
            Self::Constant => 1.0,

            Self::LinearIn => t,
            Self::LinearReverse => 1.0 - t,

            Self::SineIn => 1.0 - (t * FRAC_PI_2).cos(),
            Self::SineOut => (t * FRAC_PI_2).sin(),
            Self::SineInOut => -(PI * t).cos() * 0.5 + 0.5,

            Self::SineInReverse => 1.0 - (1.0 - (t * FRAC_PI_2).cos()),
            Self::SineOutReverse => 1.0 - (t * FRAC_PI_2).sin(),
            Self::SineInOutReverse => 1.0 - (-(PI * t).cos() * 0.5 + 0.5),

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

            Self::BounceIn => 1.0 - Self::bounce_out_value(1.0 - t),
            Self::BounceOut => Self::bounce_out_value(t),
            Self::BounceInOut => {
                if t < 0.5 {
                    (1.0 - Self::bounce_out_value(1.0 - t * 2.0)) * 0.5
                } else {
                    Self::bounce_out_value(t * 2.0 - 1.0) * 0.5 + 0.5
                }
            }

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
    pub curve: Option<SplineCurveConfig>,
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
    pub alpha_curve: Option<SplineCurveConfig>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub emission_curve: Option<SplineCurveConfig>,
}

impl Default for ParticleProcessDisplayColor {
    fn default() -> Self {
        Self {
            initial_color: SolidOrGradientColor::default(),
            alpha_curve: None,
            emission_curve: None,
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
    pub influence_curve: Option<SplineCurveConfig>,
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
    pub particle_flags: ParticleFlags,
    #[serde(default)]
    pub spawn: ParticleProcessSpawn,
    #[serde(default)]
    pub animated_velocity: ParticleProcessAnimVelocities,
    #[serde(default)]
    pub accelerations: ParticleProcessAccelerations,
    #[serde(default)]
    pub display: ParticleProcessDisplay,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub turbulence: Option<ParticleProcessTurbulence>,
}

impl Default for ParticleProcessConfig {
    fn default() -> Self {
        Self {
            particle_flags: ParticleFlags::empty(),
            spawn: ParticleProcessSpawn::default(),
            animated_velocity: ParticleProcessAnimVelocities::default(),
            accelerations: ParticleProcessAccelerations::default(),
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
