pub mod compute;
pub mod extract;
pub mod gradient_texture;
pub mod material;
pub mod sort;

use bevy::render::render_resource::ShaderType;
use bytemuck::{Pod, Zeroable};

pub const EMISSION_SHAPE_POINT: u32 = 0;
pub const EMISSION_SHAPE_SPHERE: u32 = 1;
pub const EMISSION_SHAPE_SPHERE_SURFACE: u32 = 2;
pub const EMISSION_SHAPE_BOX: u32 = 3;
pub const EMISSION_SHAPE_RING: u32 = 4;

// scale curve constants (0 = constant/no curve, 1+ = easing curve type)
// TODO: implement more easing curves
pub const SCALE_CURVE_CONSTANT: u32 = 0;
pub const SCALE_CURVE_LINEAR_IN: u32 = 1;
pub const SCALE_CURVE_LINEAR_OUT: u32 = 2;

#[derive(Clone, Copy, Default, Pod, Zeroable, ShaderType)]
#[repr(C)]
pub struct EmitterUniforms {
    pub delta_time: f32,
    pub system_phase: f32,
    pub prev_system_phase: f32,
    pub cycle: u32,

    pub amount: u32,
    pub lifetime: f32,
    pub lifetime_randomness: f32,
    pub emitting: u32,

    pub gravity: [f32; 3],
    pub random_seed: u32,

    // emission shape
    pub emission_shape: u32,
    pub emission_sphere_radius: f32,
    pub emission_ring_height: f32,
    pub emission_ring_radius: f32,

    pub emission_ring_inner_radius: f32,
    pub spread: f32,
    pub flatness: f32,
    pub initial_velocity_min: f32,

    pub initial_velocity_max: f32,
    pub inherit_velocity_ratio: f32,
    pub explosiveness: f32,
    pub randomness: f32,

    pub emission_shape_offset: [f32; 3],
    pub _pad1: f32,

    pub emission_shape_scale: [f32; 3],
    pub _pad2: f32,

    pub emission_box_extents: [f32; 3],
    pub _pad3: f32,

    pub emission_ring_axis: [f32; 3],
    pub _pad4: f32,

    pub direction: [f32; 3],
    pub _pad5: f32,

    pub velocity_pivot: [f32; 3],
    pub _pad6: f32,

    pub draw_order: u32,
    pub clear_particles: u32,
    pub scale_min: f32,
    pub scale_max: f32,

    pub scale_curve: u32,
    pub use_initial_color_gradient: u32,
    pub _pad7: [u32; 2],

    pub initial_color: [f32; 4],
}
