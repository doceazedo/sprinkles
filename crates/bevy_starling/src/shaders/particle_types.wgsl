#define_import_path aracari::particle_types

struct Particle {
    position: vec4<f32>,  // xyz, scale
    velocity: vec4<f32>,  // xyz, lifetime
    color: vec4<f32>,
    custom: vec4<f32>,    // age, spawn_index, seed, flags
}

struct SplineCurve {
    enabled: u32,
    min_value: f32,
    max_value: f32,
    _pad: u32,
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

    emission_shape: u32,
    emission_sphere_radius: f32,
    emission_ring_height: f32,
    emission_ring_radius: f32,

    emission_ring_inner_radius: f32,
    spread: f32,
    flatness: f32,
    initial_velocity_min: f32,

    initial_velocity_max: f32,
    inherit_velocity_ratio: f32,
    explosiveness: f32,
    randomness: f32,

    emission_shape_offset: vec3<f32>,
    _pad1: f32,

    emission_shape_scale: vec3<f32>,
    _pad2: f32,

    emission_box_extents: vec3<f32>,
    _pad3: f32,

    emission_ring_axis: vec3<f32>,
    _pad4: f32,

    direction: vec3<f32>,
    _pad5: f32,

    velocity_pivot: vec3<f32>,
    _pad6: f32,

    draw_order: u32,
    clear_particles: u32,
    scale_min: f32,
    scale_max: f32,

    scale_curve: SplineCurve,

    use_initial_color_gradient: u32,
    turbulence_enabled: u32,
    particle_flags: u32,
    _pad7: u32,

    initial_color: vec4<f32>,

    alpha_curve: SplineCurve,
    emission_curve: SplineCurve,

    // turbulence
    turbulence_noise_strength: f32,
    turbulence_noise_scale: f32,
    turbulence_noise_speed_random: f32,
    turbulence_influence_min: f32,

    turbulence_noise_speed: vec3<f32>,
    turbulence_influence_max: f32,

    turbulence_influence_curve: SplineCurve,
}

// per-particle flags (stored in particle.custom.w)
const PARTICLE_FLAG_ACTIVE: u32 = 1u;

// emitter-level particle flags (from EmitterParams.particle_flags)
const EMITTER_FLAG_ALIGN_Y_TO_VELOCITY: u32 = 1u;
const EMITTER_FLAG_DISABLE_Z: u32 = 4u;
