use bevy::{
    asset::{io::Reader, AssetLoader, LoadContext},
    prelude::*,
    render::alpha::AlphaMode,
};
use bitflags::bitflags;
use serde::{Deserialize, Serialize};
use std::hash::{Hash, Hasher};
use thiserror::Error;

// serde skip helpers

fn is_false(v: &bool) -> bool {
    !*v
}

fn is_true(v: &bool) -> bool {
    *v
}

fn is_zero_f32(v: &f32) -> bool {
    *v == 0.0
}

fn is_zero_u32(v: &u32) -> bool {
    *v == 0
}

fn is_zero_vec3(v: &Vec3) -> bool {
    *v == Vec3::ZERO
}

fn is_one_vec3(v: &Vec3) -> bool {
    *v == Vec3::ONE
}

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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default, Reflect)]
pub enum ParticleSystemDimension {
    #[default]
    D3,
    D2,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default, Reflect)]
pub enum DrawOrder {
    #[default]
    Index,
    Lifetime,
    ReverseLifetime,
    ViewDepth,
}

impl DrawOrder {
    fn is_default(&self) -> bool {
        *self == Self::default()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Reflect)]
pub struct EmitterTime {
    #[serde(default = "default_lifetime")]
    pub lifetime: f32,
    #[serde(default, skip_serializing_if = "is_zero_f32")]
    pub lifetime_randomness: f32,
    #[serde(default, skip_serializing_if = "is_zero_f32")]
    pub delay: f32,
    #[serde(default, skip_serializing_if = "is_false")]
    pub one_shot: bool,
    #[serde(default, skip_serializing_if = "is_zero_f32")]
    pub explosiveness: f32,
    #[serde(default, skip_serializing_if = "is_zero_f32")]
    pub spawn_time_randomness: f32,
    #[serde(default, skip_serializing_if = "is_zero_u32")]
    pub fixed_fps: u32,
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
    pub fn total_duration(&self) -> f32 {
        self.delay + self.lifetime
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Reflect)]
pub struct EmitterData {
    pub name: String,
    #[serde(default = "default_enabled", skip_serializing_if = "is_true")]
    pub enabled: bool,

    #[serde(default, skip_serializing_if = "is_zero_vec3")]
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
    pub colors: EmitterColors,

    #[serde(default)]
    pub velocities: EmitterVelocities,

    #[serde(default)]
    pub accelerations: EmitterAccelerations,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub turbulence: Option<EmitterTurbulence>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub collision: Option<EmitterCollision>,

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
            colors: EmitterColors::default(),
            velocities: EmitterVelocities::default(),
            accelerations: EmitterAccelerations::default(),
            turbulence: None,
            collision: None,
            particle_flags: ParticleFlags::empty(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Reflect)]
pub struct EmitterDrawPass {
    #[serde(default, skip_serializing_if = "DrawOrder::is_default")]
    pub draw_order: DrawOrder,
    pub mesh: ParticleMesh,
    #[serde(default)]
    pub material: DrawPassMaterial,
    #[serde(default = "default_shadow_caster", skip_serializing_if = "is_true")]
    pub shadow_caster: bool,
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
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default, PartialEq, Reflect)]
pub enum QuadOrientation {
    FaceX,
    FaceY,
    #[default]
    FaceZ,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Reflect)]
pub enum ParticleMesh {
    Quad {
        #[serde(default)]
        orientation: QuadOrientation,
    },
    Sphere {
        #[serde(default = "default_sphere_radius")]
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
    Prism {
        #[serde(default = "default_prism_left_to_right")]
        left_to_right: f32,
        #[serde(default = "default_prism_size")]
        size: Vec3,
        #[serde(default, skip_serializing_if = "is_zero_vec3")]
        subdivide: Vec3,
    },
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
        Self::Sphere {
            radius: 1.0,
        }
    }
}

// material types

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default, PartialEq, Reflect)]
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

fn default_fog_enabled() -> bool {
    true
}

fn is_default_emissive(v: &[f32; 4]) -> bool {
    *v == [0.0, 0.0, 0.0, 1.0]
}

#[derive(Debug, Clone, Serialize, Deserialize, Reflect)]
pub struct StandardParticleMaterial {
    #[serde(default = "default_base_color")]
    pub base_color: [f32; 4],

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub base_color_texture: Option<String>,

    #[serde(default, skip_serializing_if = "is_default_emissive")]
    pub emissive: [f32; 4],

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub emissive_texture: Option<String>,

    #[serde(default = "default_alpha_mode")]
    pub alpha_mode: SerializableAlphaMode,

    #[serde(default = "default_perceptual_roughness")]
    pub perceptual_roughness: f32,

    #[serde(default, skip_serializing_if = "is_zero_f32")]
    pub metallic: f32,

    #[serde(default = "default_reflectance")]
    pub reflectance: f32,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metallic_roughness_texture: Option<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub normal_map_texture: Option<String>,

    #[serde(default, skip_serializing_if = "is_false")]
    pub double_sided: bool,

    #[serde(default, skip_serializing_if = "is_false")]
    pub unlit: bool,

    #[serde(default = "default_fog_enabled", skip_serializing_if = "is_true")]
    pub fog_enabled: bool,
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

#[derive(Debug, Clone, Serialize, Deserialize, Reflect)]
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

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Reflect)]
pub struct Range {
    #[serde(default, skip_serializing_if = "is_zero_f32")]
    pub min: f32,
    #[serde(default, skip_serializing_if = "is_zero_f32")]
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

    fn is_zero(&self) -> bool {
        self.min == 0.0 && self.max == 0.0
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default, PartialEq, Reflect)]
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

#[derive(Debug, Clone, Serialize, Deserialize, Reflect)]
pub struct EmitterEmission {
    #[serde(default, skip_serializing_if = "is_zero_vec3")]
    pub offset: Vec3,
    #[serde(default = "default_emission_scale", skip_serializing_if = "is_one_vec3")]
    pub scale: Vec3,
    #[serde(default, skip_serializing_if = "EmissionShape::is_default")]
    pub shape: EmissionShape,
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

#[derive(Debug, Clone, Serialize, Deserialize, Reflect)]
pub struct EmitterScale {
    #[serde(default = "default_scale_range")]
    pub range: Range,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub curve: Option<CurveConfig>,
}

impl Default for EmitterScale {
    fn default() -> Self {
        Self {
            range: default_scale_range(),
            curve: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Reflect)]
pub struct EmitterColors {
    #[serde(default)]
    pub initial_color: SolidOrGradientColor,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub alpha_curve: Option<CurveConfig>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub emission_curve: Option<CurveConfig>,
}

impl Default for EmitterColors {
    fn default() -> Self {
        Self {
            initial_color: SolidOrGradientColor::default(),
            alpha_curve: None,
            emission_curve: None,
        }
    }
}

fn default_direction() -> Vec3 {
    Vec3::X
}

fn default_spread() -> f32 {
    45.0
}

#[derive(Debug, Clone, Serialize, Deserialize, Reflect)]
pub struct AnimatedVelocity {
    #[serde(default, skip_serializing_if = "Range::is_zero")]
    pub value: Range,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub curve: Option<CurveConfig>,
}

impl Default for AnimatedVelocity {
    fn default() -> Self {
        Self {
            value: Range::default(),
            curve: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Reflect)]
pub struct EmitterVelocities {
    #[serde(default = "default_direction")]
    pub direction: Vec3,
    #[serde(default = "default_spread")]
    pub spread: f32,
    #[serde(default, skip_serializing_if = "is_zero_f32")]
    pub flatness: f32,
    #[serde(default, skip_serializing_if = "Range::is_zero")]
    pub initial_velocity: Range,
    #[serde(default)]
    pub radial_velocity: AnimatedVelocity,
    #[serde(default, skip_serializing_if = "is_zero_vec3")]
    pub velocity_pivot: Vec3,
    #[serde(default, skip_serializing_if = "is_zero_f32")]
    pub inherit_velocity_ratio: f32,
}

impl Default for EmitterVelocities {
    fn default() -> Self {
        Self {
            direction: Vec3::X,
            spread: 45.0,
            flatness: 0.0,
            initial_velocity: Range::default(),
            radial_velocity: AnimatedVelocity::default(),
            velocity_pivot: Vec3::ZERO,
            inherit_velocity_ratio: 0.0,
        }
    }
}

fn default_gravity() -> Vec3 {
    Vec3::new(0.0, -9.8, 0.0)
}

#[derive(Debug, Clone, Serialize, Deserialize, Reflect)]
pub struct EmitterAccelerations {
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

#[derive(Debug, Clone, Serialize, Deserialize, Reflect)]
pub struct EmitterTurbulence {
    #[serde(default, skip_serializing_if = "is_false")]
    pub enabled: bool,
    #[serde(default = "default_turbulence_noise_strength")]
    pub noise_strength: f32,
    #[serde(default = "default_turbulence_noise_scale")]
    pub noise_scale: f32,
    #[serde(default, skip_serializing_if = "is_zero_vec3")]
    pub noise_speed: Vec3,
    #[serde(default, skip_serializing_if = "is_zero_f32")]
    pub noise_speed_random: f32,
    #[serde(default = "default_turbulence_influence")]
    pub influence: Range,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub influence_curve: Option<CurveConfig>,
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
            influence_curve: None,
        }
    }
}

fn default_collision_base_size() -> f32 {
    0.01
}

#[derive(Debug, Clone, Serialize, Deserialize, Reflect)]
pub enum EmitterCollisionMode {
    Rigid { friction: f32, bounce: f32 },
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

#[derive(Debug, Clone, Serialize, Deserialize, Reflect)]
pub struct EmitterCollision {
    pub mode: EmitterCollisionMode,
    #[serde(default = "default_collision_base_size")]
    pub base_size: f32,
    #[serde(default, skip_serializing_if = "is_false")]
    pub use_scale: bool,
}

impl Default for EmitterCollision {
    fn default() -> Self {
        Self {
            mode: EmitterCollisionMode::default(),
            base_size: default_collision_base_size(),
            use_scale: false,
        }
    }
}


#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Default, Reflect)]
pub enum CurveMode {
    #[default]
    SingleCurve,
    DoubleCurve,
    Hold,
    Stairs,
    SmoothStairs,
}

fn default_tension() -> f64 {
    0.0
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Reflect)]
pub struct CurvePoint {
    pub position: f32,
    pub value: f64,
    #[serde(default)]
    pub mode: CurveMode,
    #[serde(default = "default_tension")]
    pub tension: f64,
}

impl CurvePoint {
    pub fn new(position: f32, value: f64) -> Self {
        Self {
            position,
            value,
            mode: CurveMode::default(),
            tension: 0.0,
        }
    }

    pub fn with_mode(mut self, mode: CurveMode) -> Self {
        self.mode = mode;
        self
    }

    pub fn with_tension(mut self, tension: f64) -> Self {
        self.tension = tension;
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Reflect)]
pub struct CurveTexture {
    pub points: Vec<CurvePoint>,
    #[serde(default)]
    pub range: Range,
}

impl Default for CurveTexture {
    fn default() -> Self {
        Self {
            points: vec![
                CurvePoint::new(0.0, 1.0),
                CurvePoint::new(1.0, 1.0),
            ],
            range: Range::default(),
        }
    }
}

impl CurveTexture {
    pub fn new(points: Vec<CurvePoint>) -> Self {
        Self {
            points,
            range: Range::default(),
        }
    }

    pub fn with_range(mut self, range: Range) -> Self {
        self.range = range;
        self
    }

    pub fn cache_key(&self) -> u64 {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        for point in &self.points {
            point.position.to_bits().hash(&mut hasher);
            (point.value as f32).to_bits().hash(&mut hasher);
            std::mem::discriminant(&point.mode).hash(&mut hasher);
            (point.tension as f32).to_bits().hash(&mut hasher);
        }
        self.range.min.to_bits().hash(&mut hasher);
        self.range.max.to_bits().hash(&mut hasher);
        hasher.finish()
    }

    pub fn is_constant(&self) -> bool {
        if self.points.len() < 2 {
            return true;
        }
        let first_value = self.points[0].value;
        self.points.iter().all(|p| (p.value - first_value).abs() < f64::EPSILON)
    }

    pub fn sample(&self, t: f32) -> f32 {
        if self.points.is_empty() {
            return 1.0;
        }
        if self.points.len() == 1 {
            return self.points[0].value as f32;
        }

        let t = t.clamp(0.0, 1.0);

        let mut left_idx = 0;
        let mut right_idx = self.points.len() - 1;

        for (i, point) in self.points.iter().enumerate() {
            if point.position <= t {
                left_idx = i;
            }
        }
        for (i, point) in self.points.iter().enumerate() {
            if point.position >= t {
                right_idx = i;
                break;
            }
        }

        let left = &self.points[left_idx];
        let right = &self.points[right_idx];

        if left_idx == right_idx {
            return left.value as f32;
        }

        let segment_range = right.position - left.position;
        if segment_range <= 0.0 {
            return left.value as f32;
        }

        let local_t = (t - left.position) / segment_range;
        let curved_t = apply_curve(local_t, right.mode, right.tension as f32);

        (left.value + (right.value - left.value) * curved_t as f64) as f32
    }
}

fn apply_curve(t: f32, mode: CurveMode, tension: f32) -> f32 {
    match mode {
        CurveMode::SingleCurve => apply_tension(t, tension),
        CurveMode::DoubleCurve => {
            if t < 0.5 {
                let local_t = t * 2.0;
                apply_tension(local_t, tension) * 0.5
            } else {
                let local_t = (t - 0.5) * 2.0;
                0.5 + apply_tension(local_t, -tension) * 0.5
            }
        }
        CurveMode::Hold => 0.0,
        CurveMode::Stairs => {
            let steps = tension_to_steps(tension);
            (t * steps as f32).floor() / (steps - 1).max(1) as f32
        }
        CurveMode::SmoothStairs => {
            let steps = tension_to_steps(tension);
            let step_size = 1.0 / steps as f32;
            let current_step = (t / step_size).floor();
            let local_t = (t - current_step * step_size) / step_size;
            let smooth_t = local_t * local_t * (3.0 - 2.0 * local_t);
            let start = current_step / (steps - 1).max(1) as f32;
            let end = (current_step + 1.0).min(steps as f32 - 1.0) / (steps - 1).max(1) as f32;
            start + (end - start) * smooth_t
        }
    }
}

fn apply_tension(t: f32, tension: f32) -> f32 {
    if tension.abs() < f32::EPSILON {
        return t;
    }
    let exp = 1.0 / (1.0 - tension.abs() * 0.999);
    if tension > 0.0 {
        t.powf(exp)
    } else {
        1.0 - (1.0 - t).powf(exp)
    }
}

fn tension_to_steps(tension: f32) -> u32 {
    let tension = tension.clamp(0.0, 1.0);
    let min_steps = 1u32;
    let max_steps = 64u32;
    min_steps + ((max_steps - min_steps) as f32 * tension) as u32
}

fn default_curve_min() -> f32 {
    0.0
}

fn default_curve_max() -> f32 {
    1.0
}

#[derive(Debug, Clone, Serialize, Deserialize, Reflect)]
pub struct CurveConfig {
    pub curve: CurveTexture,
    #[serde(default = "default_curve_min")]
    pub min_value: f32,
    #[serde(default = "default_curve_max")]
    pub max_value: f32,
}

impl Default for CurveConfig {
    fn default() -> Self {
        Self {
            curve: CurveTexture::default(),
            min_value: 0.0,
            max_value: 1.0,
        }
    }
}

impl CurveConfig {
    pub fn is_constant(&self) -> bool {
        self.curve.is_constant()
    }

    pub fn cache_key(&self) -> u64 {
        self.curve.cache_key()
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default, PartialEq, Eq, Hash, Reflect)]
pub enum GradientInterpolation {
    Steps,
    #[default]
    Linear,
    Smoothstep,
}

impl GradientInterpolation {
    fn is_default(&self) -> bool {
        *self == Self::default()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Reflect)]
pub struct GradientStop {
    pub color: [f32; 4],
    pub position: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Reflect)]
pub struct Gradient {
    pub stops: Vec<GradientStop>,
    #[serde(default, skip_serializing_if = "GradientInterpolation::is_default")]
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

#[derive(Debug, Clone, Serialize, Deserialize, Reflect)]
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

// collision shapes

#[derive(Debug, Clone, Serialize, Deserialize, Reflect)]
pub enum ParticlesColliderShape3D {
    Box { size: Vec3 },
    Sphere { radius: f32 },
}

impl Default for ParticlesColliderShape3D {
    fn default() -> Self {
        Self::Sphere { radius: 1.0 }
    }
}

#[derive(Asset, TypePath, Debug, Clone, Serialize, Deserialize)]
pub struct ParticleSystemAsset {
    pub name: String,
    pub dimension: ParticleSystemDimension,
    pub emitters: Vec<EmitterData>,
}
