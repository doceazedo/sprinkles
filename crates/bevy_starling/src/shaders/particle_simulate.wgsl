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

    use_scale_curve: u32,
    use_initial_color_gradient: u32,
    use_alpha_curve: u32,
    _pad7: u32,

    initial_color: vec4<f32>,
}

const EMISSION_SHAPE_POINT: u32 = 0u;
const EMISSION_SHAPE_SPHERE: u32 = 1u;
const EMISSION_SHAPE_SPHERE_SURFACE: u32 = 2u;
const EMISSION_SHAPE_BOX: u32 = 3u;
const EMISSION_SHAPE_RING: u32 = 4u;

const DRAW_ORDER_INDEX: u32 = 0u;
const PI: f32 = 3.14159265359;

const PARTICLE_FLAG_ACTIVE: u32 = 1u;

@group(0) @binding(0) var<uniform> params: EmitterParams;
@group(0) @binding(1) var<storage, read_write> particles: array<Particle>;
@group(0) @binding(2) var gradient_texture: texture_2d<f32>;
@group(0) @binding(3) var gradient_sampler: sampler;
@group(0) @binding(4) var curve_texture: texture_2d<f32>;
@group(0) @binding(5) var curve_sampler: sampler;
@group(0) @binding(6) var alpha_curve_texture: texture_2d<f32>;
@group(0) @binding(7) var alpha_curve_sampler: sampler;

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

fn get_emission_position(seed: u32) -> vec3<f32> {
    var pos = vec3(0.0);

    switch params.emission_shape {
        case EMISSION_SHAPE_POINT: {
            pos = vec3(0.0);
        }
        case EMISSION_SHAPE_SPHERE: {
            // uniform distribution inside sphere using rejection sampling approximation
            let u = hash_to_float(seed);
            let v = hash_to_float(seed + 1u);
            let w = hash_to_float(seed + 2u);

            let theta = 2.0 * PI * u;
            let phi = acos(2.0 * v - 1.0);
            let r = pow(w, 1.0 / 3.0) * params.emission_sphere_radius;

            pos = vec3(
                r * sin(phi) * cos(theta),
                r * sin(phi) * sin(theta),
                r * cos(phi)
            );
        }
        case EMISSION_SHAPE_SPHERE_SURFACE: {
            // uniform distribution on sphere surface
            let u = hash_to_float(seed);
            let v = hash_to_float(seed + 1u);

            let theta = 2.0 * PI * u;
            let phi = acos(2.0 * v - 1.0);
            let r = params.emission_sphere_radius;

            pos = vec3(
                r * sin(phi) * cos(theta),
                r * sin(phi) * sin(theta),
                r * cos(phi)
            );
        }
        case EMISSION_SHAPE_BOX: {
            // uniform distribution inside box
            let u = hash_to_float(seed) * 2.0 - 1.0;
            let v = hash_to_float(seed + 1u) * 2.0 - 1.0;
            let w = hash_to_float(seed + 2u) * 2.0 - 1.0;
            pos = vec3(u, v, w) * params.emission_box_extents;
        }
        case EMISSION_SHAPE_RING: {
            // ring emission with configurable axis, height, radius, and inner radius
            let u = hash_to_float(seed);
            let v = hash_to_float(seed + 1u);
            let h = hash_to_float(seed + 2u);

            let theta = 2.0 * PI * u;
            let r_range = params.emission_ring_radius - params.emission_ring_inner_radius;
            let r = params.emission_ring_inner_radius + sqrt(v) * r_range;
            let height_offset = (h - 0.5) * params.emission_ring_height;

            // create position in ring local space (ring lies in XY plane, axis is Z)
            let local_pos = vec3(r * cos(theta), r * sin(theta), height_offset);

            // rotate to align with the configured axis
            pos = rotate_to_axis(local_pos, params.emission_ring_axis);
        }
        default: {
            pos = vec3(0.0);
        }
    }

    // apply offset and scale
    return pos * params.emission_shape_scale + params.emission_shape_offset;
}

fn rotate_to_axis(v: vec3<f32>, axis: vec3<f32>) -> vec3<f32> {
    let z_axis = vec3(0.0, 0.0, 1.0);
    let target_axis = normalize(axis);

    // if axis is already Z (or close), no rotation needed
    let dot_val = dot(z_axis, target_axis);
    if (abs(dot_val) > 0.9999) {
        if (dot_val < 0.0) {
            return vec3(v.x, -v.y, -v.z);
        }
        return v;
    }

    // compute rotation axis and angle
    let rot_axis = normalize(cross(z_axis, target_axis));
    let cos_angle = dot_val;
    let sin_angle = sqrt(1.0 - cos_angle * cos_angle);

    // rodrigues rotation formula
    return v * cos_angle + cross(rot_axis, v) * sin_angle + rot_axis * dot(rot_axis, v) * (1.0 - cos_angle);
}

fn get_emission_velocity(seed: u32) -> vec3<f32> {
    // base direction
    var dir = normalize(params.direction);
    if (length(params.direction) < 0.0001) {
        dir = vec3(1.0, 0.0, 0.0);
    }

    // apply spread angle to randomize direction within a cone
    let spread_rad = radians(params.spread);
    if (spread_rad > 0.0001) {
        let u = hash_to_float(seed);
        let v = hash_to_float(seed + 1u);

        // random angle around the cone
        let phi = 2.0 * PI * u;
        // random angle from center (0 to spread)
        let theta = spread_rad * sqrt(v);

        // create a random direction within the cone
        let cos_theta = cos(theta);
        let sin_theta = sin(theta);

        // find perpendicular vectors to direction
        var perp1: vec3<f32>;
        if (abs(dir.x) < 0.9) {
            perp1 = normalize(cross(dir, vec3(1.0, 0.0, 0.0)));
        } else {
            perp1 = normalize(cross(dir, vec3(0.0, 1.0, 0.0)));
        }
        let perp2 = cross(dir, perp1);

        // apply flatness: 0.0 = sphere cone, 1.0 = flat disc
        let flat_cos_phi = cos(phi);
        let flat_sin_phi = sin(phi) * (1.0 - params.flatness);
        let flat_angle = atan2(flat_sin_phi, flat_cos_phi);

        dir = dir * cos_theta + (perp1 * cos(flat_angle) + perp2 * sin(flat_angle)) * sin_theta;
        dir = normalize(dir);
    }

    // interpolate between min and max velocity
    let vel_t = hash_to_float(seed + 2u);
    let speed = mix(params.initial_velocity_min, params.initial_velocity_max, vel_t);

    return dir * speed;
}

fn get_initial_scale(seed: u32) -> f32 {
    let t = hash_to_float(seed);
    return mix(params.scale_min, params.scale_max, t);
}

fn get_scale_at_lifetime(initial_scale: f32, age: f32, lifetime: f32) -> f32 {
    if (params.use_scale_curve == 0u) {
        return initial_scale;
    }
    let t = clamp(age / lifetime, 0.0, 1.0);
    let curve_value = textureSampleLevel(curve_texture, curve_sampler, vec2(t, 0.5), 0.0).r;
    return initial_scale * curve_value;
}

fn get_initial_alpha(seed: u32) -> f32 {
    if (params.use_initial_color_gradient == 0u) {
        return params.initial_color.a;
    } else {
        let t = hash_to_float(seed + 30u);
        return textureSampleLevel(gradient_texture, gradient_sampler, vec2(t, 0.5), 0.0).a;
    }
}

fn get_alpha_at_lifetime(initial_alpha: f32, age: f32, lifetime: f32) -> f32 {
    if (params.use_alpha_curve == 0u) {
        return initial_alpha;
    }
    let t = clamp(age / lifetime, 0.0, 1.0);
    let curve_value = textureSampleLevel(alpha_curve_texture, alpha_curve_sampler, vec2(t, 0.5), 0.0).r;
    return initial_alpha * curve_value;
}

fn spawn_particle(idx: u32) -> Particle {
    var p: Particle;
    let seed = hash(params.random_seed + idx + params.cycle * 1000u);

    let emission_pos = get_emission_position(seed);
    let initial_scale = get_initial_scale(seed + 20u);
    // for constant curve, use initial scale directly; for curves, start at eased t=0
    let scale = get_scale_at_lifetime(initial_scale, 0.0, 1.0);
    p.position = vec4(emission_pos, scale);

    let vel = get_emission_velocity(seed + 10u);
    let lifetime = params.lifetime * (1.0 + random_range(seed + 4u, params.lifetime_randomness));
    p.velocity = vec4(vel, lifetime);

    if (params.use_initial_color_gradient == 0u) {
        p.color = params.initial_color;
    } else {
        let t = hash_to_float(seed + 30u);
        p.color = textureSampleLevel(gradient_texture, gradient_sampler, vec2(t, 0.5), 0.0);
    }

    // apply alpha curve at spawn (t=0)
    let initial_alpha = p.color.a;
    p.color.a = get_alpha_at_lifetime(initial_alpha, 0.0, 1.0);

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

    let lifetime = p.velocity.w;

    // check if lifetime exceeded
    if (age >= lifetime) {
        p.custom.w = bitcast<f32>(0u); // deactivate
        return p;
    }

    // apply gravity and update velocity
    let velocity = p.velocity.xyz + params.gravity * dt;
    p.velocity = vec4(velocity, lifetime);

    // update position
    let new_position = p.position.xyz + velocity * dt;

    // update scale based on lifetime progress
    let seed = bitcast<u32>(p.custom.z);
    let initial_scale = get_initial_scale(seed + 20u);
    let scale = get_scale_at_lifetime(initial_scale, age, lifetime);

    p.position = vec4(new_position, scale);

    // update alpha based on lifetime progress
    let initial_alpha = get_initial_alpha(seed);
    p.color.a = get_alpha_at_lifetime(initial_alpha, age, lifetime);

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
