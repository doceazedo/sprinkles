use bevy::prelude::*;
use bevy::reflect::TypePath;
use bevy::render::render_resource::*;
use bevy::shader::ShaderRef;
use bevy_sprinkles::prelude::ParticleGradient;

use super::{MAX_STOPS, pack_gradient_stops};

const SHADER_GRADIENT_PATH: &str = "shaders/gradient_edit.wgsl";
const BORDER_RADIUS: f32 = 4.0;
const CHECKERBOARD_SIZE: f32 = 6.0;
const SWATCH_CHECKERBOARD_SIZE: f32 = 4.0;
const SWATCH_BORDER_RADIUS: f32 = 4.0;
const LINEAR_INTERPOLATION: u32 = 1;

#[derive(AsBindGroup, Asset, TypePath, Debug, Clone)]
pub struct GradientMaterial {
    #[uniform(0)]
    pub border_radius: f32,
    #[uniform(0)]
    pub checkerboard_size: f32,
    #[uniform(0)]
    pub stop_count: u32,
    #[uniform(0)]
    pub interpolation: u32,
    #[uniform(0)]
    pub positions: [Vec4; 2],
    #[uniform(0)]
    pub colors: [Vec4; MAX_STOPS],
}

impl GradientMaterial {
    pub fn from_gradient(gradient: &ParticleGradient) -> Self {
        let (stop_count, positions, colors) = pack_gradient_stops(gradient);
        Self {
            border_radius: BORDER_RADIUS,
            checkerboard_size: CHECKERBOARD_SIZE,
            stop_count,
            interpolation: gradient.interpolation as u32,
            positions,
            colors,
        }
    }

    pub fn swatch(gradient: &ParticleGradient) -> Self {
        let (stop_count, positions, colors) = pack_gradient_stops(gradient);
        Self {
            border_radius: SWATCH_BORDER_RADIUS,
            checkerboard_size: SWATCH_CHECKERBOARD_SIZE,
            stop_count,
            interpolation: LINEAR_INTERPOLATION,
            positions,
            colors,
        }
    }
}

impl UiMaterial for GradientMaterial {
    fn fragment_shader() -> ShaderRef {
        SHADER_GRADIENT_PATH.into()
    }
}
