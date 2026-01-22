pub mod compute;
pub mod extract;
pub mod material;
pub mod sort;

use bevy::render::render_resource::ShaderType;
use bytemuck::{Pod, Zeroable};

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

    pub initial_velocity: [f32; 3],
    pub _pad1: f32,
    pub initial_velocity_randomness: [f32; 3],
    pub _pad2: f32,

    pub initial_scale: f32,
    pub initial_scale_randomness: f32,
    pub explosiveness: f32,
    pub randomness: f32,

    pub draw_order: u32,
    pub _pad3: [u32; 3],
}
