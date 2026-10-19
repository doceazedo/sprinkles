use bevy::prelude::*;
use bevy::reflect::TypePath;
use bevy::render::render_resource::*;
use bevy::shader::ShaderRef;

const SHADER_HSV_RECT_PATH: &str = "embedded://sprinkles/assets/shaders/color_picker_hsv_rect.wesl";
const SHADER_HUE_PATH: &str = "embedded://sprinkles/assets/shaders/color_picker_hue.wesl";
const SHADER_ALPHA_PATH: &str = "embedded://sprinkles/assets/shaders/color_picker_alpha.wesl";
const SHADER_CHECKERBOARD_PATH: &str =
    "embedded://sprinkles/assets/shaders/color_picker_checkerboard.wesl";

#[derive(AsBindGroup, Asset, TypePath, Debug, Clone)]
pub struct HsvRectMaterial {
    #[uniform(0)]
    pub hue: f32,
    #[uniform(0)]
    pub border_radius: f32,
}

impl UiMaterial for HsvRectMaterial {
    fn fragment_shader() -> ShaderRef {
        SHADER_HSV_RECT_PATH.into()
    }
}

#[derive(AsBindGroup, Asset, TypePath, Debug, Clone)]
pub struct HueSliderMaterial {
    #[uniform(0)]
    pub border_radius: f32,
}

impl UiMaterial for HueSliderMaterial {
    fn fragment_shader() -> ShaderRef {
        SHADER_HUE_PATH.into()
    }
}

#[derive(AsBindGroup, Asset, TypePath, Debug, Clone)]
pub struct AlphaSliderMaterial {
    #[uniform(0)]
    pub color: Vec4,
    #[uniform(0)]
    pub checkerboard_size: f32,
    #[uniform(0)]
    pub border_radius: f32,
}

impl UiMaterial for AlphaSliderMaterial {
    fn fragment_shader() -> ShaderRef {
        SHADER_ALPHA_PATH.into()
    }
}

#[derive(AsBindGroup, Asset, TypePath, Debug, Clone)]
pub struct CheckerboardMaterial {
    #[uniform(0)]
    pub color: Vec4,
    #[uniform(0)]
    pub size: f32,
    #[uniform(0)]
    pub border_radius: f32,
}

impl UiMaterial for CheckerboardMaterial {
    fn fragment_shader() -> ShaderRef {
        SHADER_CHECKERBOARD_PATH.into()
    }
}
