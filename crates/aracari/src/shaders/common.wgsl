#define_import_path aracari::common

struct Particle {
    position: vec4<f32>,       // xyz, scale
    velocity: vec4<f32>,       // xyz, lifetime
    color: vec4<f32>,
    custom: vec4<f32>,         // age, spawn_index, seed, flags
    alignment_dir: vec4<f32>,  // xyz direction for ALIGN_Y_TO_VELOCITY, w unused
}

struct CurveUniform {
    enabled: u32,
    min_value: f32,
    max_value: f32,
    _pad: u32,
}

// per-particle flags (stored in particle.custom.w)
const PARTICLE_FLAG_ACTIVE: u32 = 1u;

// emitter-level particle flags (from EmitterParams.particle_flags)
const EMITTER_FLAG_ALIGN_Y_TO_VELOCITY: u32 = 1u;
const EMITTER_FLAG_DISABLE_Z: u32 = 4u;

fn hash(n: u32) -> u32 {
    var x = n;
    x = ((x >> 16u) ^ x) * 0x45d9f3bu;
    x = ((x >> 16u) ^ x) * 0x45d9f3bu;
    x = (x >> 16u) ^ x;
    return x;
}

fn hash_to_float(n: u32) -> f32 {
    return f32(hash(n)) / f32(0xFFFFFFFFu);
}

fn random_range(seed: u32, variation: f32) -> f32 {
    return (hash_to_float(seed) * 2.0 - 1.0) * variation;
}

fn random_vec3(seed: u32, variation: vec3<f32>) -> vec3<f32> {
    return vec3(
        random_range(seed, variation.x),
        random_range(seed + 1u, variation.y),
        random_range(seed + 2u, variation.z)
    );
}
