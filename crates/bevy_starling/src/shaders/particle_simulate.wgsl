// particle types are inlined here since shader imports from embedded assets can be tricky

struct Particle {
    position: vec4<f32>,  // xyz, scale
    velocity: vec4<f32>,  // xyz, lifetime
    color: vec4<f32>,
    custom: vec4<f32>,    // age, phase, seed, flags
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

    draw_order: u32,
    clear_particles: u32,
    _pad3_a: u32,
    _pad3_b: u32,
}

const DRAW_ORDER_INDEX: u32 = 0u;

const PARTICLE_FLAG_ACTIVE: u32 = 1u;

@group(0) @binding(0) var<uniform> params: EmitterParams;
@group(0) @binding(1) var<storage, read_write> particles: array<Particle>;

@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let idx = global_id.x;
    if (idx >= params.amount) {
        return;
    }

    // handle clear request - deactivate all particles
    if (params.clear_particles != 0u) {
        var p = particles[idx];
        p.custom.w = bitcast<f32>(0u);
        particles[idx] = p;
        return;
    }

    var p = particles[idx];
    let flags = bitcast<u32>(p.custom.w);
    let is_active = (flags & PARTICLE_FLAG_ACTIVE) != 0u;

    // phase-based emission: each particle has a phase (0-1) based on its index
    let base_phase = f32(idx) / f32(params.amount);
    let phase = base_phase + hash_to_float(idx) * params.randomness;
    let adjusted_phase = fract(phase * (1.0 - params.explosiveness));

    // check if this particle should restart this frame
    var should_restart = false;
    if (params.emitting != 0u) {
        if (params.system_phase < params.prev_system_phase) {
            // wrapped around: check if phase is in [prev, 1) or [0, current)
            should_restart = adjusted_phase >= params.prev_system_phase ||
                           adjusted_phase < params.system_phase;
        } else {
            // normal case: check if phase is in [prev, current)
            should_restart = adjusted_phase >= params.prev_system_phase &&
                           adjusted_phase < params.system_phase;
        }
    }

    if (should_restart) {
        p = spawn_particle(idx);
    } else if (is_active) {
        p = update_particle(p);
    }

    particles[idx] = p;
}

fn spawn_particle(idx: u32) -> Particle {
    var p: Particle;
    let seed = hash(params.random_seed + idx + params.cycle * 1000u);

    let scale = params.initial_scale + random_range(seed, params.initial_scale_randomness);
    p.position = vec4(0.0, 0.0, 0.0, scale);

    let vel = params.initial_velocity + random_vec3(seed + 1u, params.initial_velocity_randomness);
    let lifetime = params.lifetime * (1.0 + random_range(seed + 4u, params.lifetime_randomness));
    p.velocity = vec4(vel, lifetime);

    p.color = vec4(1.0, 1.0, 1.0, 1.0);

    // spawn_index tracks total spawns across all cycles for depth ordering
    // only set when draw_order is Index, otherwise use 0
    var spawn_index = 0.0;
    if (params.draw_order == DRAW_ORDER_INDEX) {
        spawn_index = f32(params.cycle * params.amount + idx);
    }
    p.custom = vec4(0.0, spawn_index, bitcast<f32>(seed), bitcast<f32>(PARTICLE_FLAG_ACTIVE));

    return p;
}

fn update_particle(p_in: Particle) -> Particle {
    var p = p_in;
    let dt = params.delta_time;

    // update age
    let age = p.custom.x + dt;
    p.custom.x = age;

    // check if lifetime exceeded
    if (age >= p.velocity.w) {
        p.custom.w = bitcast<f32>(0u); // deactivate
        return p;
    }

    // apply gravity and update velocity
    let velocity = p.velocity.xyz + params.gravity * dt;
    p.velocity = vec4(velocity, p.velocity.w);

    // update position
    p.position = vec4(p.position.xyz + velocity * dt, p.position.w);

    return p;
}

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
