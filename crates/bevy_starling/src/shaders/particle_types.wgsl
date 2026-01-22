#define_import_path starling::particle_types

struct Particle {
    position: vec4<f32>,  // xyz, scale
    velocity: vec4<f32>,  // xyz, lifetime
    color: vec4<f32>,
    custom: vec4<f32>,    // age, spawn_index, seed, flags
}

struct EmitterParams {
    delta_time: f32,
    system_phase: f32,
    prev_system_phase: f32,
    cycle: u32,

    amount: u32,
    lifetime: f32,
    lifetime_randomness: f32,
    emitting: u32,

    gravity: vec3<f32>,
    random_seed: u32,

    initial_velocity: vec3<f32>,
    _pad1: f32,
    initial_velocity_randomness: vec3<f32>,
    _pad2: f32,

    initial_scale: f32,
    initial_scale_randomness: f32,
    explosiveness: f32,
    randomness: f32,
}

const PARTICLE_FLAG_ACTIVE: u32 = 1u;
