#import bevy_render::maths::PI
#import aracari::common::{
    Particle,
    CurveUniform,
    PARTICLE_FLAG_ACTIVE,
    EMITTER_FLAG_DISABLE_Z,
    hash,
    hash_to_float,
}

struct AnimatedVelocity {
    min: f32,
    max: f32,
    _pad0: f32,
    _pad1: f32,
    curve: CurveUniform,
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
    spawn_time_randomness: f32,

    emission_offset: vec3<f32>,
    _pad1: f32,

    emission_scale: vec3<f32>,
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

    scale_over_lifetime: CurveUniform,

    use_initial_color_gradient: u32,
    turbulence_enabled: u32,
    particle_flags: u32,
    _pad7: u32,

    initial_color: vec4<f32>,

    alpha_over_lifetime: CurveUniform,
    emission_over_lifetime: CurveUniform,

    // turbulence
    turbulence_noise_strength: f32,
    turbulence_noise_scale: f32,
    turbulence_noise_speed_random: f32,
    turbulence_influence_min: f32,

    turbulence_noise_speed: vec3<f32>,
    turbulence_influence_max: f32,

    turbulence_influence_curve: CurveUniform,

    radial_velocity: AnimatedVelocity,

    // collision
    collision_mode: u32,
    collision_base_size: f32,
    collision_use_scale: u32,
    collision_friction: f32,

    collision_bounce: f32,
    collider_count: u32,
    _collision_pad0: f32,
    _collision_pad1: f32,
}

struct Collider {
    transform: mat4x4<f32>,
    inverse_transform: mat4x4<f32>,
    extents: vec3<f32>,
    collider_type: u32,
}

struct ColliderArray {
    colliders: array<Collider, 32>,
}

const EMISSION_SHAPE_POINT: u32 = 0u;
const EMISSION_SHAPE_SPHERE: u32 = 1u;
const EMISSION_SHAPE_SPHERE_SURFACE: u32 = 2u;
const EMISSION_SHAPE_BOX: u32 = 3u;
const EMISSION_SHAPE_RING: u32 = 4u;

const DRAW_ORDER_INDEX: u32 = 0u;

// collision constants
const COLLIDER_TYPE_SPHERE: u32 = 0u;
const COLLIDER_TYPE_BOX: u32 = 1u;
const COLLISION_MODE_DISABLED: u32 = 0u;
const COLLISION_MODE_RIGID: u32 = 1u;
const COLLISION_MODE_HIDE_ON_CONTACT: u32 = 2u;
const COLLISION_EPSILON: f32 = 0.001;

@group(0) @binding(0) var<uniform> params: EmitterParams;
@group(0) @binding(1) var<storage, read_write> particles: array<Particle>;
@group(0) @binding(2) var gradient_texture: texture_2d<f32>;
@group(0) @binding(3) var gradient_sampler: sampler;
@group(0) @binding(4) var scale_over_lifetime_texture: texture_2d<f32>;
@group(0) @binding(5) var scale_over_lifetime_sampler: sampler;
@group(0) @binding(6) var alpha_over_lifetime_texture: texture_2d<f32>;
@group(0) @binding(7) var alpha_over_lifetime_sampler: sampler;
@group(0) @binding(8) var emission_over_lifetime_texture: texture_2d<f32>;
@group(0) @binding(9) var emission_over_lifetime_sampler: sampler;
@group(0) @binding(10) var turbulence_influence_curve_texture: texture_2d<f32>;
@group(0) @binding(11) var turbulence_influence_curve_sampler: sampler;
@group(0) @binding(12) var radial_velocity_curve_texture: texture_2d<f32>;
@group(0) @binding(13) var radial_velocity_curve_sampler: sampler;
@group(0) @binding(14) var<storage, read> colliders: ColliderArray;

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
    let phase = base_phase + hash_to_float(idx) * params.spawn_time_randomness;
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

fn get_emission_offset(seed: u32) -> vec3<f32> {
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
    var result = pos * params.emission_scale + params.emission_offset;

    // disable Z for 2D mode
    if ((params.particle_flags & EMITTER_FLAG_DISABLE_Z) != 0u) {
        result.z = 0.0;
    }

    return result;
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

    var result = dir * speed;

    // disable Z for 2D mode
    if ((params.particle_flags & EMITTER_FLAG_DISABLE_Z) != 0u) {
        result.z = 0.0;
    }

    return result;
}

fn get_initial_scale(seed: u32) -> f32 {
    let t = hash_to_float(seed);
    return mix(params.scale_min, params.scale_max, t);
}

// turbulence noise functions (based on godot's implementation)
fn grad(p: vec4<f32>) -> vec4<f32> {
    let frac_p = fract(vec4(
        dot(p, vec4(0.143081, 0.001724, 0.280166, 0.262771)),
        dot(p, vec4(0.645401, -0.047791, -0.146698, 0.595016)),
        dot(p, vec4(-0.499665, -0.095734, 0.425674, -0.207367)),
        dot(p, vec4(-0.013596, -0.848588, 0.423736, 0.17044))
    ));
    return fract((frac_p.xyzw * frac_p.yzwx) * 2365.952041) * 2.0 - 1.0;
}

fn noise_4d(coord: vec4<f32>) -> f32 {
    // domain rotation for better xyz slices + animation patterns
    let rotated = vec4(
        coord.xyz + dot(coord, vec4(vec3(-0.1666667), -0.5)),
        dot(coord, vec4(0.5))
    );

    let base = floor(rotated);
    let delta = rotated - base;

    let grad_0000 = grad(base + vec4(0.0, 0.0, 0.0, 0.0));
    let grad_1000 = grad(base + vec4(1.0, 0.0, 0.0, 0.0));
    let grad_0100 = grad(base + vec4(0.0, 1.0, 0.0, 0.0));
    let grad_1100 = grad(base + vec4(1.0, 1.0, 0.0, 0.0));
    let grad_0010 = grad(base + vec4(0.0, 0.0, 1.0, 0.0));
    let grad_1010 = grad(base + vec4(1.0, 0.0, 1.0, 0.0));
    let grad_0110 = grad(base + vec4(0.0, 1.0, 1.0, 0.0));
    let grad_1110 = grad(base + vec4(1.0, 1.0, 1.0, 0.0));
    let grad_0001 = grad(base + vec4(0.0, 0.0, 0.0, 1.0));
    let grad_1001 = grad(base + vec4(1.0, 0.0, 0.0, 1.0));
    let grad_0101 = grad(base + vec4(0.0, 1.0, 0.0, 1.0));
    let grad_1101 = grad(base + vec4(1.0, 1.0, 0.0, 1.0));
    let grad_0011 = grad(base + vec4(0.0, 0.0, 1.0, 1.0));
    let grad_1011 = grad(base + vec4(1.0, 0.0, 1.0, 1.0));
    let grad_0111 = grad(base + vec4(0.0, 1.0, 1.0, 1.0));
    let grad_1111 = grad(base + vec4(1.0, 1.0, 1.0, 1.0));

    let result_0123 = vec4(
        dot(delta - vec4(0.0, 0.0, 0.0, 0.0), grad_0000),
        dot(delta - vec4(1.0, 0.0, 0.0, 0.0), grad_1000),
        dot(delta - vec4(0.0, 1.0, 0.0, 0.0), grad_0100),
        dot(delta - vec4(1.0, 1.0, 0.0, 0.0), grad_1100)
    );
    let result_4567 = vec4(
        dot(delta - vec4(0.0, 0.0, 1.0, 0.0), grad_0010),
        dot(delta - vec4(1.0, 0.0, 1.0, 0.0), grad_1010),
        dot(delta - vec4(0.0, 1.0, 1.0, 0.0), grad_0110),
        dot(delta - vec4(1.0, 1.0, 1.0, 0.0), grad_1110)
    );
    let result_89ab = vec4(
        dot(delta - vec4(0.0, 0.0, 0.0, 1.0), grad_0001),
        dot(delta - vec4(1.0, 0.0, 0.0, 1.0), grad_1001),
        dot(delta - vec4(0.0, 1.0, 0.0, 1.0), grad_0101),
        dot(delta - vec4(1.0, 1.0, 0.0, 1.0), grad_1101)
    );
    let result_cdef = vec4(
        dot(delta - vec4(0.0, 0.0, 1.0, 1.0), grad_0011),
        dot(delta - vec4(1.0, 0.0, 1.0, 1.0), grad_1011),
        dot(delta - vec4(0.0, 1.0, 1.0, 1.0), grad_0111),
        dot(delta - vec4(1.0, 1.0, 1.0, 1.0), grad_1111)
    );

    let fade = delta * delta * delta * (10.0 + delta * (-15.0 + delta * 6.0));
    let result_w0 = mix(result_0123, result_89ab, fade.w);
    let result_w1 = mix(result_4567, result_cdef, fade.w);
    let result_wz = mix(result_w0, result_w1, fade.z);
    let result_wzy = mix(result_wz.xy, result_wz.zw, fade.y);
    return mix(result_wzy.x, result_wzy.y, fade.x);
}

fn noise_3x(p: vec4<f32>) -> vec3<f32> {
    let s = noise_4d(p);
    let s1 = noise_4d(p + vec4(vec3(0.0), 1.7320508 * 2048.333333));
    let s2 = noise_4d(p - vec4(vec3(0.0), 1.7320508 * 2048.333333));
    return vec3(s, s1, s2);
}

fn curl_3d(p: vec4<f32>, c: f32) -> vec3<f32> {
    let epsilon = 0.001 + c;
    let dx = vec4(epsilon, 0.0, 0.0, 0.0);
    let dy = vec4(0.0, epsilon, 0.0, 0.0);
    let dz = vec4(0.0, 0.0, epsilon, 0.0);
    let x0 = noise_3x(p - dx);
    let x1 = noise_3x(p + dx);
    let y0 = noise_3x(p - dy);
    let y1 = noise_3x(p + dy);
    let z0 = noise_3x(p - dz);
    let z1 = noise_3x(p + dz);
    let curl_x = (y1.z - y0.z) - (z1.y - z0.y);
    let curl_y = (z1.x - z0.x) - (x1.z - x0.z);
    let curl_z = (x1.y - x0.y) - (y1.x - y0.x);
    return normalize(vec3(curl_x, curl_y, curl_z));
}

fn get_noise_direction(pos: vec3<f32>, time: f32, random_offset: f32) -> vec3<f32> {
    let adj_contrast = max((params.turbulence_noise_strength - 1.0), 0.0) * 70.0;
    let noise_time = time * vec4(params.turbulence_noise_speed, params.turbulence_noise_speed_random * random_offset);
    let noise_pos = vec4(pos * params.turbulence_noise_scale, 0.0);
    var noise_direction = curl_3d(noise_pos + noise_time, adj_contrast);
    noise_direction = mix(0.9 * noise_direction, noise_direction, params.turbulence_noise_strength - 9.0);
    return noise_direction;
}

fn get_turbulence_influence(seed: u32) -> f32 {
    let t = hash_to_float(seed);
    return mix(params.turbulence_influence_min, params.turbulence_influence_max, t);
}

fn sample_spline_curve(
    tex: texture_2d<f32>,
    samp: sampler,
    curve: CurveUniform,
    t: f32
) -> f32 {
    let raw = textureSampleLevel(tex, samp, vec2(t, 0.5), 0.0).r;
    return mix(curve.min_value, curve.max_value, raw);
}

fn get_turbulence_influence_at_lifetime(base_influence: f32, age: f32, lifetime: f32) -> f32 {
    if (params.turbulence_influence_curve.enabled == 0u) {
        return base_influence;
    }
    let t = clamp(age / lifetime, 0.0, 1.0);
    let curve_value = sample_spline_curve(
        turbulence_influence_curve_texture,
        turbulence_influence_curve_sampler,
        params.turbulence_influence_curve,
        t
    );
    return base_influence * curve_value;
}

fn get_scale_at_lifetime(initial_scale: f32, age: f32, lifetime: f32) -> f32 {
    if (params.scale_over_lifetime.enabled == 0u) {
        return initial_scale;
    }
    let t = clamp(age / lifetime, 0.0, 1.0);
    let curve_value = sample_spline_curve(
        scale_over_lifetime_texture,
        scale_over_lifetime_sampler,
        params.scale_over_lifetime,
        t
    );
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

fn get_initial_color_rgb(seed: u32) -> vec3<f32> {
    if (params.use_initial_color_gradient == 0u) {
        return params.initial_color.rgb;
    } else {
        let t = hash_to_float(seed + 30u);
        return textureSampleLevel(gradient_texture, gradient_sampler, vec2(t, 0.5), 0.0).rgb;
    }
}

fn get_alpha_at_lifetime(initial_alpha: f32, age: f32, lifetime: f32) -> f32 {
    if (params.alpha_over_lifetime.enabled == 0u) {
        return initial_alpha;
    }
    let t = clamp(age / lifetime, 0.0, 1.0);
    let curve_value = sample_spline_curve(
        alpha_over_lifetime_texture,
        alpha_over_lifetime_sampler,
        params.alpha_over_lifetime,
        t
    );
    return initial_alpha * curve_value;
}

fn get_emission_at_lifetime(age: f32, lifetime: f32) -> f32 {
    if (params.emission_over_lifetime.enabled == 0u) {
        return 1.0;
    }
    let t = clamp(age / lifetime, 0.0, 1.0);
    let curve_value = sample_spline_curve(
        emission_over_lifetime_texture,
        emission_over_lifetime_sampler,
        params.emission_over_lifetime,
        t
    );
    return 1.0 + curve_value;
}

fn get_initial_radial_velocity(seed: u32) -> f32 {
    let t = hash_to_float(seed);
    return mix(params.radial_velocity.min, params.radial_velocity.max, t);
}

fn get_radial_velocity_curve_multiplier(age: f32, lifetime: f32) -> f32 {
    if (params.radial_velocity.curve.enabled == 0u) {
        return 1.0;
    }
    let t = clamp(age / lifetime, 0.0, 1.0);
    return sample_spline_curve(
        radial_velocity_curve_texture,
        radial_velocity_curve_sampler,
        params.radial_velocity.curve,
        t
    );
}

// computes radial displacement (movement away from or toward velocity_pivot)
// based on godot's implementation
fn get_radial_displacement(
    position: vec3<f32>,
    radial_velocity: f32,
    age: f32,
    lifetime: f32,
    dt: f32,
    seed: u32
) -> vec3<f32> {
    var radial_displacement = vec3(0.0);

    // skip if delta time is too small
    if (dt < 0.001) {
        return radial_displacement;
    }

    // apply lifetime curve
    let curve_multiplier = get_radial_velocity_curve_multiplier(age, lifetime);
    let effective_velocity = radial_velocity * curve_multiplier;

    // skip if no radial velocity
    if (abs(effective_velocity) < 0.0001) {
        return radial_displacement;
    }

    let pivot = params.velocity_pivot;
    let to_particle = position - pivot;
    let distance_to_pivot = length(to_particle);

    // minimum distance threshold to avoid singularity
    let min_distance = 0.01;

    if (distance_to_pivot > min_distance) {
        // normal case: radiate away from pivot
        let direction = normalize(to_particle);
        radial_displacement = direction * effective_velocity;

        // for negative (inward) velocity, clamp to prevent overshooting pivot
        if (effective_velocity < 0.0) {
            let max_inward_speed = distance_to_pivot / dt;
            let clamped_speed = min(abs(effective_velocity), max_inward_speed);
            radial_displacement = direction * (-clamped_speed);
        }
    } else {
        // particle is at or very close to pivot - use random direction
        // this prevents singularity and creates natural spread
        let u = hash_to_float(seed + 50u);
        let v = hash_to_float(seed + 51u);
        let theta = 2.0 * PI * u;
        let phi = acos(2.0 * v - 1.0);
        let random_dir = vec3(
            sin(phi) * cos(theta),
            sin(phi) * sin(theta),
            cos(phi)
        );
        radial_displacement = random_dir * abs(effective_velocity);
    }

    return radial_displacement;
}

// collision detection

struct CollisionResult {
    collided: bool,
    normal: vec3<f32>,
    depth: f32,
}

fn get_particle_collision_size(scale: f32) -> f32 {
    var size = params.collision_base_size;
    if (params.collision_use_scale != 0u) {
        size *= scale;
    }
    return size * 0.5; // convert diameter to radius
}

fn check_sphere_collision(
    particle_pos: vec3<f32>,
    particle_radius: f32,
    collider: Collider,
) -> CollisionResult {
    var result: CollisionResult;
    result.collided = false;
    result.normal = vec3(0.0);
    result.depth = 0.0;

    // transform particle to collider local space
    let local_pos = (collider.inverse_transform * vec4(particle_pos, 1.0)).xyz;
    let collider_radius = collider.extents.x;

    let dist = length(local_pos);
    let penetration = dist - (particle_radius + collider_radius);

    if (penetration <= COLLISION_EPSILON) {
        result.collided = true;
        result.depth = -penetration;

        // normal in world space
        if (dist > COLLISION_EPSILON) {
            let local_normal = normalize(local_pos);
            result.normal = normalize((collider.transform * vec4(local_normal, 0.0)).xyz);
        } else {
            result.normal = vec3(0.0, 1.0, 0.0);
        }
    }

    return result;
}

fn check_box_collision(
    particle_pos: vec3<f32>,
    particle_radius: f32,
    collider: Collider,
) -> CollisionResult {
    var result: CollisionResult;
    result.collided = false;
    result.normal = vec3(0.0);
    result.depth = 0.0;

    // transform particle to collider local space
    let local_pos = (collider.inverse_transform * vec4(particle_pos, 1.0)).xyz;
    let extents = collider.extents;

    let abs_pos = abs(local_pos);
    let sgn_pos = sign(local_pos);

    // check if outside box
    if (any(abs_pos > extents)) {
        // find closest point on box surface
        let closest = min(abs_pos, extents);
        let rel = abs_pos - closest;
        let dist = length(rel);
        let penetration = dist - particle_radius;

        if (penetration <= COLLISION_EPSILON) {
            result.collided = true;
            result.depth = -penetration;

            if (dist > COLLISION_EPSILON) {
                let local_normal = normalize(rel) * sgn_pos;
                result.normal = normalize((collider.transform * vec4(local_normal, 0.0)).xyz);
            } else {
                result.normal = vec3(0.0, 1.0, 0.0);
            }
        }
    } else {
        // inside box - push out along shortest axis
        let axis_dist = extents - abs_pos;
        var local_normal: vec3<f32>;
        var min_dist: f32;

        if (axis_dist.x <= axis_dist.y && axis_dist.x <= axis_dist.z) {
            local_normal = vec3(1.0, 0.0, 0.0) * sgn_pos.x;
            min_dist = axis_dist.x;
        } else if (axis_dist.y <= axis_dist.z) {
            local_normal = vec3(0.0, 1.0, 0.0) * sgn_pos.y;
            min_dist = axis_dist.y;
        } else {
            local_normal = vec3(0.0, 0.0, 1.0) * sgn_pos.z;
            min_dist = axis_dist.z;
        }

        result.collided = true;
        result.depth = min_dist + particle_radius;
        result.normal = normalize((collider.transform * vec4(local_normal, 0.0)).xyz);
    }

    return result;
}

fn process_collisions(
    particle_pos: vec3<f32>,
    particle_radius: f32,
) -> CollisionResult {
    var final_result: CollisionResult;
    final_result.collided = false;
    final_result.normal = vec3(0.0);
    final_result.depth = 0.0;

    for (var i = 0u; i < params.collider_count; i++) {
        let collider = colliders.colliders[i];
        var col_result: CollisionResult;

        switch collider.collider_type {
            case COLLIDER_TYPE_SPHERE: {
                col_result = check_sphere_collision(particle_pos, particle_radius, collider);
            }
            case COLLIDER_TYPE_BOX: {
                col_result = check_box_collision(particle_pos, particle_radius, collider);
            }
            default: {
                continue;
            }
        }

        if (col_result.collided) {
            if (!final_result.collided) {
                // first collision
                final_result = col_result;
            } else {
                // accumulate multiple collisions (from godot)
                let c = final_result.normal * final_result.depth;
                let new_c = c + col_result.normal * max(0.0, col_result.depth - dot(col_result.normal, c));
                final_result.depth = length(new_c);
                if (final_result.depth > COLLISION_EPSILON) {
                    final_result.normal = normalize(new_c);
                }
            }
        }
    }

    return final_result;
}

fn spawn_particle(idx: u32) -> Particle {
    var p: Particle;
    // per-particle seed derivation:
    // base_seed + 1 + particle_index + (cycle * particle_count)
    let seed = hash(params.random_seed + 1u + idx + params.cycle * params.amount);

    let emission_pos = get_emission_offset(seed);
    let initial_scale = get_initial_scale(seed + 20u);
    // for constant curve, use initial scale directly; for curves, start at eased t=0
    let scale = get_scale_at_lifetime(initial_scale, 0.0, 1.0);
    p.position = vec4(emission_pos, scale);

    var vel = get_emission_velocity(seed + 10u);
    let lifetime = params.lifetime * (1.0 - hash_to_float(seed + 4u) * params.lifetime_randomness);

    // include radial velocity at spawn for correct initial alignment
    let initial_radial_velocity = get_initial_radial_velocity(seed + 60u);
    var radial_displacement = get_radial_displacement(
        emission_pos,
        initial_radial_velocity,
        0.0,  // age = 0 at spawn
        lifetime,
        params.delta_time,
        seed
    );
    if ((params.particle_flags & EMITTER_FLAG_DISABLE_Z) != 0u) {
        radial_displacement.z = 0.0;
    }
    vel = vel + radial_displacement;

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

    // apply emission curve at spawn (t=0)
    let emission = get_emission_at_lifetime(0.0, 1.0);
    p.color = vec4(p.color.rgb * emission, p.color.a);

    // spawn_index tracks total spawns across all cycles for depth ordering
    // only set when draw_order is Index, otherwise use 0
    var spawn_index = 0.0;
    if (params.draw_order == DRAW_ORDER_INDEX) {
        spawn_index = f32(params.cycle * params.amount + idx);
    }
    p.custom = vec4(0.0, spawn_index, bitcast<f32>(seed), bitcast<f32>(PARTICLE_FLAG_ACTIVE));

    // initialize alignment direction for ALIGN_Y_TO_VELOCITY (like godot)
    // if velocity > 0, use normalized velocity; otherwise use default up
    if length(vel) > 0.0 {
        p.alignment_dir = vec4(normalize(vel), 0.0);
    } else {
        p.alignment_dir = vec4(0.0, 1.0, 0.0, 0.0);
    }

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

    let seed = bitcast<u32>(p.custom.z);
    let initial_radial_velocity = get_initial_radial_velocity(seed + 60u);

    // the stored velocity includes the previous frame's radial displacement (for alignment)
    // we need to extract the pure physics velocity before applying gravity
    let stored_velocity = p.velocity.xyz;

    // extract physics velocity by removing previous radial component
    // on first frame (age <= dt), stored velocity is pure physics (no radial yet)
    var physics_velocity = stored_velocity;
    if (age > dt) {
        // compute previous position to calculate previous radial displacement
        let prev_position = p.position.xyz - stored_velocity * dt;
        let prev_age = age - dt;

        // compute what the radial displacement was last frame
        var prev_radial = get_radial_displacement(
            prev_position,
            initial_radial_velocity,
            prev_age,
            lifetime,
            dt,
            seed
        );
        if ((params.particle_flags & EMITTER_FLAG_DISABLE_Z) != 0u) {
            prev_radial.z = 0.0;
        }

        physics_velocity = stored_velocity - prev_radial;
    }

    // apply gravity (respect DISABLE_Z flag for 2D mode)
    var gravity = params.gravity;
    if ((params.particle_flags & EMITTER_FLAG_DISABLE_Z) != 0u) {
        gravity.z = 0.0;
    }
    physics_velocity = physics_velocity + gravity * dt;

    // compute current radial displacement
    var radial_displacement = get_radial_displacement(
        p.position.xyz,
        initial_radial_velocity,
        age,
        lifetime,
        dt,
        seed
    );
    if ((params.particle_flags & EMITTER_FLAG_DISABLE_Z) != 0u) {
        radial_displacement.z = 0.0;
    }

    // apply turbulence to physics velocity
    if (params.turbulence_enabled != 0u) {
        let base_influence = get_turbulence_influence(seed + 40u);
        let influence = get_turbulence_influence_at_lifetime(base_influence, age, lifetime);
        let random_offset = hash_to_float(seed + 41u);
        let noise_direction = get_noise_direction(p.position.xyz, age, random_offset);
        let vel_magnitude = length(physics_velocity);
        if (vel_magnitude > 0.0001) {
            physics_velocity = mix(physics_velocity, noise_direction * vel_magnitude, influence);
        }
    }

    // disable Z for 2D mode before storing velocity and updating position
    if ((params.particle_flags & EMITTER_FLAG_DISABLE_Z) != 0u) {
        physics_velocity.z = 0.0;
    }

    // combine physics velocity with controlled displacements (like radial velocity)
    let effective_velocity = physics_velocity + radial_displacement;

    p.velocity = vec4(effective_velocity, lifetime);

    // update alignment direction only when velocity > 0 (like godot)
    // if velocity is zero, keep the existing alignment direction
    if length(effective_velocity) > 0.0 {
        p.alignment_dir = vec4(normalize(effective_velocity), 0.0);
    }

    // update position
    var new_position = p.position.xyz + effective_velocity * dt;

    // force Z to 0 for 2D mode
    if ((params.particle_flags & EMITTER_FLAG_DISABLE_Z) != 0u) {
        new_position.z = 0.0;
    }

    // update scale based on lifetime progress
    let initial_scale = get_initial_scale(seed + 20u);
    let scale = get_scale_at_lifetime(initial_scale, age, lifetime);

    p.position = vec4(new_position, scale);

    // collision handling
    if (params.collision_mode != COLLISION_MODE_DISABLED && params.collider_count > 0u) {
        let particle_radius = get_particle_collision_size(scale);
        let collision = process_collisions(p.position.xyz, particle_radius);

        if (collision.collided) {
            if (params.collision_mode == COLLISION_MODE_HIDE_ON_CONTACT) {
                p.custom.w = bitcast<f32>(0u);
                return p;
            }

            // COLLISION_MODE_RIGID
            var velocity = p.velocity.xyz;
            let collision_response = dot(collision.normal, velocity);

            // adaptive bounce threshold (from godot)
            let bounce_threshold = 2.0 / clamp(params.collision_bounce + 1.0, 1.0, 2.0);
            let should_bounce = step(bounce_threshold, abs(collision_response));

            // push particle out of collision
            var col_position = p.position.xyz + collision.normal * collision.depth;

            // remove velocity component along normal
            var col_velocity = velocity - collision.normal * collision_response;

            // apply friction to remaining velocity
            col_velocity = mix(col_velocity, vec3(0.0), clamp(params.collision_friction, 0.0, 1.0));

            // apply bounce
            col_velocity -= collision.normal * collision_response * params.collision_bounce * should_bounce;

            // handle 2D mode
            if ((params.particle_flags & EMITTER_FLAG_DISABLE_Z) != 0u) {
                col_position.z = 0.0;
                col_velocity.z = 0.0;
            }

            p.position = vec4(col_position, scale);
            p.velocity = vec4(col_velocity, lifetime);

            // update alignment direction only when velocity > 0 (like godot)
            if length(col_velocity) > 0.0 {
                p.alignment_dir = vec4(normalize(col_velocity), 0.0);
            }
        }
    }

    // update alpha based on lifetime progress
    let initial_alpha = get_initial_alpha(seed);
    p.color.a = get_alpha_at_lifetime(initial_alpha, age, lifetime);

    // update color based on emission curve
    let initial_rgb = get_initial_color_rgb(seed);
    let emission = get_emission_at_lifetime(age, lifetime);
    p.color = vec4(initial_rgb * emission, p.color.a);

    return p;
}
