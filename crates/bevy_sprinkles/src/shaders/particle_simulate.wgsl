#import bevy_render::maths::PI
#import bevy_sprinkles::common::{
    Particle,
    CurveUniform,
    SubEmissionEntry,
    PARTICLE_FLAG_ACTIVE,
    EMITTER_FLAG_DISABLE_Z,
    EMISSION_FLAG_HAS_POSITION,
    EMISSION_FLAG_HAS_VELOCITY,
    SUB_EMITTER_MODE_DISABLED,
    SUB_EMITTER_MODE_CONSTANT,
    SUB_EMITTER_MODE_AT_END,
    SUB_EMITTER_MODE_AT_COLLISION,
    SUB_EMITTER_MODE_AT_START,
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

    turbulence_influence_over_lifetime: CurveUniform,

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

    // angle
    angle_min: f32,
    angle_max: f32,
    _angle_pad0: f32,
    _angle_pad1: f32,

    angle_over_lifetime: CurveUniform,

    angular_velocity: AnimatedVelocity,

    // sub emitter
    sub_emitter_mode: u32,
    sub_emitter_frequency: f32,
    sub_emitter_amount: u32,
    sub_emitter_keep_velocity: u32,

    is_sub_emitter_target: u32,
    _sub_emitter_pad0: u32,
    _sub_emitter_pad1: u32,
    _sub_emitter_pad2: u32,
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
@group(0) @binding(10) var turbulence_influence_over_lifetime_texture: texture_2d<f32>;
@group(0) @binding(11) var turbulence_influence_over_lifetime_sampler: sampler;
@group(0) @binding(12) var radial_velocity_curve_texture: texture_2d<f32>;
@group(0) @binding(13) var radial_velocity_curve_sampler: sampler;
@group(0) @binding(14) var angle_over_lifetime_texture: texture_2d<f32>;
@group(0) @binding(15) var angle_over_lifetime_sampler: sampler;
@group(0) @binding(16) var angular_velocity_curve_texture: texture_2d<f32>;
@group(0) @binding(17) var angular_velocity_curve_sampler: sampler;
@group(0) @binding(18) var color_over_lifetime_texture: texture_2d<f32>;
@group(0) @binding(19) var color_over_lifetime_sampler: sampler;
@group(0) @binding(20) var<storage, read> colliders: ColliderArray;

struct SubEmissionBuffer {
    particle_count: atomic<i32>,
    particle_max: u32,
    data: array<SubEmissionEntry>,
}

@group(0) @binding(21) var<storage, read_write> dst_emission_buffer: SubEmissionBuffer;
@group(0) @binding(22) var<storage, read_write> src_emission_buffer: SubEmissionBuffer;

@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let idx = global_id.x;
    if (idx >= params.amount) {
        return;
    }

    var p = particles[idx];

    // fully reset particle data, then continue to emission/update
    // (clear does not skip the rest of the step)
    if (params.clear_particles != 0u) {
        p.position = vec4(0.0, 0.0, 0.0, 1.0);
        p.velocity = vec4(0.0);
        p.color = vec4(1.0);
        p.custom = vec4(0.0);
        p.alignment_dir = vec4(0.0, 1.0, 0.0, 0.0);
    }

    // when delta is zero (paused), write cleared data and stop
    if (params.delta_time <= 0.0) {
        particles[idx] = p;
        return;
    }

    let flags = bitcast<u32>(p.custom.w);
    let is_active = (flags & PARTICLE_FLAG_ACTIVE) != 0u;

    // phase-based emission: each particle has a phase (0-1) based on its index
    let base_phase = f32(idx) / f32(params.amount);
    let phase = base_phase + hash_to_float(idx) * params.spawn_time_randomness;
    let adjusted_phase = fract(phase * (1.0 - params.explosiveness));

    if (params.is_sub_emitter_target != 0u) {
        // sub-emitter target mode: inactive particles consume from source buffer
        if (!is_active) {
            let src_index = atomicAdd(&src_emission_buffer.particle_count, -1) - 1;

            if (src_index >= 0) {
                let entry = src_emission_buffer.data[src_index];
                p = spawn_particle(idx);

                if ((entry.flags & EMISSION_FLAG_HAS_POSITION) != 0u) {
                    p.position = vec4(entry.position.xyz, p.position.w);
                }

                if ((entry.flags & EMISSION_FLAG_HAS_VELOCITY) != 0u) {
                    p.velocity = vec4(entry.velocity.xyz, p.velocity.w);
                    if length(entry.velocity.xyz) > 0.0 {
                        p.alignment_dir = vec4(normalize(entry.velocity.xyz), p.alignment_dir.w);
                    }
                }
            }
        } else {
            p = update_particle(p);
        }
    } else {
        // normal phase-based emission
        var should_restart = false;
        if (params.emitting != 0u) {
            if (params.system_phase < params.prev_system_phase) {
                // phase wrapped around
                should_restart = adjusted_phase >= params.prev_system_phase ||
                               adjusted_phase < params.system_phase;
            } else {
                should_restart = adjusted_phase >= params.prev_system_phase &&
                               adjusted_phase < params.system_phase;
            }
        }

        if (should_restart) {
            p = spawn_particle(idx);
        } else if (is_active) {
            p = update_particle(p);
        }
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
            // uniform distribution inside sphere
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
            let u = hash_to_float(seed);
            let v = hash_to_float(seed + 1u);
            let h = hash_to_float(seed + 2u);

            let theta = 2.0 * PI * u;
            let r_range = params.emission_ring_radius - params.emission_ring_inner_radius;
            let r = params.emission_ring_inner_radius + sqrt(v) * r_range;
            let height_offset = (h - 0.5) * params.emission_ring_height;

            // ring local space (ring lies in XY plane, axis is Z)
            let local_pos = vec3(r * cos(theta), r * sin(theta), height_offset);
            pos = rotate_to_axis(local_pos, params.emission_ring_axis);
        }
        default: {
            pos = vec3(0.0);
        }
    }

    var result = pos * params.emission_scale + params.emission_offset;

    // disable z for 2d mode
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

    let rot_axis = normalize(cross(z_axis, target_axis));
    let cos_angle = dot_val;
    let sin_angle = sqrt(1.0 - cos_angle * cos_angle);

    // rodrigues rotation formula
    return v * cos_angle + cross(rot_axis, v) * sin_angle + rot_axis * dot(rot_axis, v) * (1.0 - cos_angle);
}

fn get_emission_velocity(seed: u32) -> vec3<f32> {
    var dir = normalize(params.direction);
    if (length(params.direction) < 0.0001) {
        dir = vec3(1.0, 0.0, 0.0);
    }

    // randomize direction within a cone based on spread angle
    let spread_rad = radians(params.spread);
    if (spread_rad > 0.0001) {
        let u = hash_to_float(seed);
        let v = hash_to_float(seed + 1u);

        let phi = 2.0 * PI * u;
        let theta = spread_rad * sqrt(v);

        let cos_theta = cos(theta);
        let sin_theta = sin(theta);

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

    let vel_t = hash_to_float(seed + 2u);
    let speed = mix(params.initial_velocity_min, params.initial_velocity_max, vel_t);

    var result = dir * speed;

    if ((params.particle_flags & EMITTER_FLAG_DISABLE_Z) != 0u) {
        result.z = 0.0;
    }

    return result;
}

fn get_initial_scale(seed: u32) -> f32 {
    let t = hash_to_float(seed);
    return mix(params.scale_min, params.scale_max, t);
}

fn get_initial_angle(seed: u32) -> f32 {
    let t = hash_to_float(seed);
    return mix(params.angle_min, params.angle_max, t);
}

fn get_initial_angular_velocity(seed: u32) -> f32 {
    let t = hash_to_float(seed);
    return mix(params.angular_velocity.min, params.angular_velocity.max, t);
}

// computes the final angle in radians from initial angle, angular velocity,
// and lifetime curves
fn compute_angle(seed: u32, age: f32, lifetime: f32) -> f32 {
    let lifetime_frac = clamp(age / lifetime, 0.0, 1.0);

    var base_angle = get_initial_angle(seed);

    // curve applies as a multiplier to the initial angle
    if (params.angle_over_lifetime.enabled != 0u) {
        let curve_value = sample_spline_curve(
            angle_over_lifetime_texture,
            angle_over_lifetime_sampler,
            params.angle_over_lifetime,
            lifetime_frac
        );
        base_angle *= curve_value;
    }

    let angular_vel = get_initial_angular_velocity(seed + 1u);
    if (abs(angular_vel) > 0.0001) {
        if (params.angular_velocity.curve.enabled != 0u) {
            let vel_curve = sample_spline_curve(
                angular_velocity_curve_texture,
                angular_velocity_curve_sampler,
                params.angular_velocity.curve,
                lifetime_frac
            );
            base_angle += age * angular_vel * vel_curve;
        } else {
            base_angle += age * angular_vel;
        }
    }

    return radians(base_angle);
}

// 3d noise / turbulence functions
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
    // domain rotation to improve the look of xyz slices + animation patterns
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
    if (params.turbulence_influence_over_lifetime.enabled == 0u) {
        return base_influence;
    }
    let t = clamp(age / lifetime, 0.0, 1.0);
    let curve_value = sample_spline_curve(
        turbulence_influence_over_lifetime_texture,
        turbulence_influence_over_lifetime_sampler,
        params.turbulence_influence_over_lifetime,
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

fn get_color_over_lifetime(age: f32, lifetime: f32) -> vec4<f32> {
    let t = clamp(age / lifetime, 0.0, 1.0);
    return textureSampleLevel(color_over_lifetime_texture, color_over_lifetime_sampler, vec2(t, 0.5), 0.0);
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
fn get_radial_displacement(
    position: vec3<f32>,
    radial_velocity: f32,
    age: f32,
    lifetime: f32,
    dt: f32,
    seed: u32
) -> vec3<f32> {
    var radial_displacement = vec3(0.0);

    if (dt < 0.001) {
        return radial_displacement;
    }

    let curve_multiplier = get_radial_velocity_curve_multiplier(age, lifetime);
    let effective_velocity = radial_velocity * curve_multiplier;

    if (abs(effective_velocity) < 0.0001) {
        return radial_displacement;
    }

    let pivot = params.velocity_pivot;
    let to_particle = position - pivot;
    let distance_to_pivot = length(to_particle);

    // minimum distance threshold to avoid singularity
    let min_distance = 0.01;

    if (distance_to_pivot > min_distance) {
        let direction = normalize(to_particle);
        radial_displacement = direction * effective_velocity;

        // clamp inward velocity to prevent overshooting pivot
        if (effective_velocity < 0.0) {
            let max_inward_speed = distance_to_pivot / dt;
            let clamped_speed = min(abs(effective_velocity), max_inward_speed);
            radial_displacement = direction * (-clamped_speed);
        }
    } else {
        // particle at pivot - use random direction to avoid singularity
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

    let local_pos = (collider.inverse_transform * vec4(particle_pos, 1.0)).xyz;
    let collider_radius = collider.extents.x;

    let dist = length(local_pos);
    let penetration = dist - (particle_radius + collider_radius);

    if (penetration <= COLLISION_EPSILON) {
        result.collided = true;
        result.depth = -penetration;

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

    let local_pos = (collider.inverse_transform * vec4(particle_pos, 1.0)).xyz;
    let extents = collider.extents;

    let abs_pos = abs(local_pos);
    let sgn_pos = sign(local_pos);

    // point outside box
    if (any(abs_pos > extents)) {
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
        // point inside box
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
                final_result = col_result;
            } else {
                // accumulate multiple collisions
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

fn emit_subparticle(position: vec3<f32>, scale: f32, velocity: vec3<f32>, flags: u32) -> bool {
    if (params.sub_emitter_mode == SUB_EMITTER_MODE_DISABLED) {
        return false;
    }

    let dst_index = atomicAdd(&dst_emission_buffer.particle_count, 1);
    if (dst_index >= i32(dst_emission_buffer.particle_max)) {
        atomicAdd(&dst_emission_buffer.particle_count, -1);
        return false;
    }

    dst_emission_buffer.data[dst_index].position = vec4(position, scale);
    dst_emission_buffer.data[dst_index].velocity = vec4(velocity, 0.0);
    dst_emission_buffer.data[dst_index].flags = flags;

    return true;
}

fn emit_sub_particles(position: vec3<f32>, scale: f32, velocity: vec3<f32>) {
    var flags = EMISSION_FLAG_HAS_POSITION;
    if (params.sub_emitter_keep_velocity != 0u) {
        flags |= EMISSION_FLAG_HAS_VELOCITY;
    }
    for (var i = 0u; i < params.sub_emitter_amount; i++) {
        emit_subparticle(position, scale, velocity, flags);
    }
}

fn spawn_particle(idx: u32) -> Particle {
    var p: Particle;
    // per-particle seed: base_seed + 1 + index + (cycle * amount)
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

    let initial_alpha = p.color.a;
    p.color.a = get_alpha_at_lifetime(initial_alpha, 0.0, 1.0);

    let emission = get_emission_at_lifetime(0.0, 1.0);
    let col_life = get_color_over_lifetime(0.0, 1.0);
    p.color = vec4(p.color.rgb * emission * col_life.rgb, p.color.a * col_life.a);

    // spawn_index tracks total spawns across all cycles for depth ordering
    var spawn_index = 0.0;
    if (params.draw_order == DRAW_ORDER_INDEX) {
        spawn_index = f32(params.cycle * params.amount + idx);
    }
    p.custom = vec4(0.0, spawn_index, bitcast<f32>(seed), bitcast<f32>(PARTICLE_FLAG_ACTIVE));

    let angle = compute_angle(seed + 70u, 0.0, lifetime);

    // initialize alignment direction from velocity, w stores angle in radians
    if length(vel) > 0.0 {
        p.alignment_dir = vec4(normalize(vel), angle);
    } else {
        p.alignment_dir = vec4(0.0, 1.0, 0.0, angle);
    }

    // sub emitter: at start trigger
    if (params.sub_emitter_mode == SUB_EMITTER_MODE_AT_START) {
        emit_sub_particles(p.position.xyz, p.position.w, vel);
    }

    return p;
}

fn update_particle(p_in: Particle) -> Particle {
    var p = p_in;
    let dt = params.delta_time;
    let age = p.custom.x + dt;
    p.custom.x = age;

    let lifetime = p.velocity.w;

    // sub emitter: constant mode - emit once per frequency interval
    if (params.sub_emitter_mode == SUB_EMITTER_MODE_CONSTANT) {
        let prev_age = age - dt;
        let interval = params.sub_emitter_frequency;
        if (interval > 0.0) {
            let interval_rem = interval - fract(prev_age / interval) * interval;
            if (dt >= interval_rem) {
                emit_subparticle(
                    p.position.xyz,
                    p.position.w,
                    p.velocity.xyz,
                    EMISSION_FLAG_HAS_POSITION | select(0u, EMISSION_FLAG_HAS_VELOCITY, params.sub_emitter_keep_velocity != 0u)
                );
            }
        }
    }

    if (age >= lifetime) {
        // sub emitter: at end trigger
        if (params.sub_emitter_mode == SUB_EMITTER_MODE_AT_END) {
            emit_sub_particles(p.position.xyz, p.position.w, p.velocity.xyz);
        }
        p.custom.w = bitcast<f32>(0u); // deactivate
        return p;
    }

    let seed = bitcast<u32>(p.custom.z);
    let initial_radial_velocity = get_initial_radial_velocity(seed + 60u);

    // stored velocity includes previous radial displacement, extract pure physics velocity
    let stored_velocity = p.velocity.xyz;

    // on first frame, stored velocity is pure physics (no radial yet)
    var physics_velocity = stored_velocity;
    if (age > dt) {
        let prev_position = p.position.xyz - stored_velocity * dt;
        let prev_age = age - dt;

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

    var gravity = params.gravity;
    if ((params.particle_flags & EMITTER_FLAG_DISABLE_Z) != 0u) {
        gravity.z = 0.0;
    }
    physics_velocity = physics_velocity + gravity * dt;

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

    // turbulence
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

    if ((params.particle_flags & EMITTER_FLAG_DISABLE_Z) != 0u) {
        physics_velocity.z = 0.0;
    }

    // combine physics velocity with controlled displacements
    let effective_velocity = physics_velocity + radial_displacement;

    p.velocity = vec4(effective_velocity, lifetime);

    let angle = compute_angle(seed + 70u, age, lifetime);

    // update alignment direction from velocity, preserve existing direction if zero
    if length(effective_velocity) > 0.0 {
        p.alignment_dir = vec4(normalize(effective_velocity), angle);
    } else {
        p.alignment_dir.w = angle;
    }

    var new_position = p.position.xyz + effective_velocity * dt;

    if ((params.particle_flags & EMITTER_FLAG_DISABLE_Z) != 0u) {
        new_position.z = 0.0;
    }

    let initial_scale = get_initial_scale(seed + 20u);
    let scale = get_scale_at_lifetime(initial_scale, age, lifetime);

    p.position = vec4(new_position, scale);

    // collision handling
    if (params.collision_mode != COLLISION_MODE_DISABLED && params.collider_count > 0u) {
        let particle_radius = get_particle_collision_size(scale);
        let collision = process_collisions(p.position.xyz, particle_radius);

        if (collision.collided) {
            // sub emitter: at collision trigger
            if (params.sub_emitter_mode == SUB_EMITTER_MODE_AT_COLLISION) {
                emit_sub_particles(p.position.xyz, p.position.w, p.velocity.xyz);
            }

            if (params.collision_mode == COLLISION_MODE_HIDE_ON_CONTACT) {
                p.custom.w = bitcast<f32>(0u);
                return p;
            }

            // COLLISION_MODE_RIGID
            var velocity = p.velocity.xyz;
            let collision_response = dot(collision.normal, velocity);

            // adaptive bounce threshold
            let bounce_threshold = 2.0 / clamp(params.collision_bounce + 1.0, 1.0, 2.0);
            let should_bounce = step(bounce_threshold, abs(collision_response));

            var col_position = p.position.xyz + collision.normal * collision.depth;

            // remove velocity components not tangential to collision normal
            var col_velocity = velocity - collision.normal * collision_response;

            // apply friction to velocity along the surface
            col_velocity = mix(col_velocity, vec3(0.0), clamp(params.collision_friction, 0.0, 1.0));

            // add bounce velocity
            col_velocity -= collision.normal * collision_response * params.collision_bounce * should_bounce;
            if ((params.particle_flags & EMITTER_FLAG_DISABLE_Z) != 0u) {
                col_position.z = 0.0;
                col_velocity.z = 0.0;
            }

            p.position = vec4(col_position, scale);
            p.velocity = vec4(col_velocity, lifetime);

            // update alignment direction from velocity
            if length(col_velocity) > 0.0 {
                p.alignment_dir = vec4(normalize(col_velocity), p.alignment_dir.w);
            }
        }
    }

    let initial_alpha = get_initial_alpha(seed);
    p.color.a = get_alpha_at_lifetime(initial_alpha, age, lifetime);

    let initial_rgb = get_initial_color_rgb(seed);
    let emission = get_emission_at_lifetime(age, lifetime);
    let col_life = get_color_over_lifetime(age, lifetime);
    p.color = vec4(initial_rgb * emission * col_life.rgb, p.color.a * col_life.a);

    return p;
}
