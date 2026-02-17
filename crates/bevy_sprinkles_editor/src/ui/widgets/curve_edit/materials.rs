use bevy::prelude::*;
use bevy::reflect::TypePath;
use bevy::render::render_resource::*;
use bevy::shader::ShaderRef;
use bevy_sprinkles::prelude::CurveTexture;

const SHADER_CURVE_PATH: &str = "embedded://sprinkles/assets/shaders/curve_edit.wgsl";
pub const MAX_POINTS: usize = 8;
const BORDER_RADIUS: f32 = 4.0;

fn pack_f32(values: &[f32; MAX_POINTS]) -> [Vec4; 2] {
    [
        Vec4::new(values[0], values[1], values[2], values[3]),
        Vec4::new(values[4], values[5], values[6], values[7]),
    ]
}

fn pack_u32(values: &[u32; MAX_POINTS]) -> [UVec4; 2] {
    [
        UVec4::new(values[0], values[1], values[2], values[3]),
        UVec4::new(values[4], values[5], values[6], values[7]),
    ]
}

#[derive(AsBindGroup, Asset, TypePath, Debug, Clone, Default)]
pub struct CurveMaterial {
    #[uniform(0)]
    border_radius: f32,
    #[uniform(0)]
    point_count: u32,
    #[uniform(0)]
    range_min: f32,
    #[uniform(0)]
    range_max: f32,
    #[uniform(0)]
    positions_low: Vec4,
    #[uniform(0)]
    positions_high: Vec4,
    #[uniform(0)]
    values_low: Vec4,
    #[uniform(0)]
    values_high: Vec4,
    #[uniform(0)]
    modes_low: UVec4,
    #[uniform(0)]
    modes_high: UVec4,
    #[uniform(0)]
    tensions_low: Vec4,
    #[uniform(0)]
    tensions_high: Vec4,
    #[uniform(0)]
    easings_low: UVec4,
    #[uniform(0)]
    easings_high: UVec4,
}

impl CurveMaterial {
    pub fn from_curve(curve: &CurveTexture) -> Self {
        let mut positions = [0.0f32; MAX_POINTS];
        let mut values = [0.0f32; MAX_POINTS];
        let mut modes = [0u32; MAX_POINTS];
        let mut tensions = [0.0f32; MAX_POINTS];
        let mut easings = [0u32; MAX_POINTS];

        for (i, point) in curve.points.iter().take(MAX_POINTS).enumerate() {
            positions[i] = point.position;
            values[i] = point.value as f32;
            modes[i] = point.mode as u32;
            tensions[i] = point.tension as f32;
            easings[i] = point.easing as u32;
        }

        let [positions_low, positions_high] = pack_f32(&positions);
        let [values_low, values_high] = pack_f32(&values);
        let [modes_low, modes_high] = pack_u32(&modes);
        let [tensions_low, tensions_high] = pack_f32(&tensions);
        let [easings_low, easings_high] = pack_u32(&easings);

        Self {
            border_radius: BORDER_RADIUS,
            point_count: curve.points.len().min(MAX_POINTS) as u32,
            range_min: curve.range.min,
            range_max: curve.range.max,
            positions_low,
            positions_high,
            values_low,
            values_high,
            modes_low,
            modes_high,
            tensions_low,
            tensions_high,
            easings_low,
            easings_high,
        }
    }
}

impl UiMaterial for CurveMaterial {
    fn fragment_shader() -> ShaderRef {
        SHADER_CURVE_PATH.into()
    }
}
