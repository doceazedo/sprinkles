#import bevy_sprinkles::common::{
    Particle,
    ParticleEmitterUniforms,
    PARTICLE_FLAG_ACTIVE,
    EMITTER_FLAG_ROTATE_Y,
    TRANSFORM_ALIGN_BILLBOARD,
    TRANSFORM_ALIGN_Y_TO_VELOCITY,
    TRANSFORM_ALIGN_BILLBOARD_Y_TO_VELOCITY,
    TRANSFORM_ALIGN_BILLBOARD_FIXED_Y,
    TRAIL_THICKNESS_CURVE_SAMPLES,
}
#import bevy_pbr::{
    mesh_functions,
    mesh_view_bindings::view,
    view_transformations::position_world_to_clip,
}

#ifdef PREPASS_PIPELINE
#import bevy_pbr::prepass_io::{Vertex, VertexOutput}
#ifdef PREPASS_FRAGMENT
#import bevy_pbr::{
    prepass_io::FragmentOutput,
    pbr_deferred_functions::deferred_output,
    pbr_fragment::pbr_input_from_standard_material,
}
#endif
#else
#import bevy_pbr::{
    forward_io::{Vertex, VertexOutput, FragmentOutput},
    pbr_fragment::pbr_input_from_standard_material,
    pbr_functions::{apply_pbr_lighting, main_pass_post_lighting_processing, alpha_discard},
}
#endif

const STANDARD_MATERIAL_FLAGS_UNLIT_BIT: u32 = 1u << 5u;

// sorted particle data, written in draw order by the sort compute shader
@group(#{MATERIAL_BIND_GROUP}) @binding(100) var<storage, read> sorted_particles: array<Particle>;
@group(#{MATERIAL_BIND_GROUP}) @binding(101) var<storage, read> emitter_uniforms: ParticleEmitterUniforms;

// computes a shortest-arc rotation matrix that aligns the Y axis to a direction
fn align_y_to_direction(dir: vec3<f32>) -> mat3x3<f32> {
    let to = normalize(dir);
    let d = to.y; // dot((0,1,0), to)

    // near-opposite: 180° rotation around X
    if d < -0.9999 {
        return mat3x3<f32>(
            vec3(1.0, 0.0, 0.0),
            vec3(0.0, -1.0, 0.0),
            vec3(0.0, 0.0, -1.0),
        );
    }

    // shortest-arc quaternion from (0,1,0) to `to`, simplified to matrix form
    let k = 1.0 / (1.0 + d);
    return mat3x3<f32>(
        vec3(1.0 - to.x * to.x * k, -to.x, -to.x * to.z * k),
        to,
        vec3(-to.x * to.z * k, -to.z, 1.0 - to.z * to.z * k),
    );
}

// aligns Y axis to a direction using a parallel-transported reference "up" vector
fn align_y_with_ref(dir: vec3<f32>, ref_up: vec3<f32>) -> mat3x3<f32> {
    let y_axis = normalize(dir);
    var x_axis = cross(y_axis, ref_up);
    let x_len = length(x_axis);

    // fallback to shortest-arc if ref_up is degenerate
    if x_len < 0.001 {
        return align_y_to_direction(dir);
    }

    x_axis = x_axis / x_len;
    let z_axis = cross(x_axis, y_axis);
    return mat3x3<f32>(x_axis, y_axis, z_axis);
}

@vertex
fn vertex(vertex: Vertex) -> VertexOutput {
    var out: VertexOutput;

    // particle index encoded in uv_b.x (instance_index doesn't guarantee particle order)
    let particle_index = u32(vertex.uv_b.x);
    let trail_size = emitter_uniforms.trail_size;
    var world_from_local = mesh_functions::get_world_from_local(vertex.instance_index);

#ifdef VERTEX_UVS_A
    out.uv = vertex.uv;
#endif

#ifdef VERTEX_UVS_B
    out.uv_b = vertex.uv_b;
#endif

#ifdef VERTEX_TANGENTS
    out.world_tangent = mesh_functions::mesh_tangent_local_to_world(world_from_local, vertex.tangent, vertex.instance_index);
#endif

#ifdef VERTEX_OUTPUT_INSTANCE_INDEX
    out.instance_index = vertex.instance_index;
#endif

    // trail rendering: position vertices along the trail path
    if (trail_size > 1u) {
        let head_slot = particle_index * trail_size;
        let head_particle = sorted_particles[head_slot];
        let head_flags = bitcast<u32>(head_particle.custom.w);
        let is_active = (head_flags & PARTICLE_FLAG_ACTIVE) != 0u;

        let section_frac = vertex.uv_b.y;

        // per-segment interpolation for curved trails
        let last_seg = trail_size - 1u;
        let section_f = section_frac * f32(last_seg);
        let seg_lo = min(u32(section_f), last_seg);
        let seg_hi = min(seg_lo + 1u, last_seg);
        let seg_t = section_f - f32(seg_lo);

        let idx_lo = head_slot + seg_lo;
        let idx_hi = head_slot + seg_hi;
        let pos_lo = sorted_particles[idx_lo].position.xyz;
        let pos_hi = sorted_particles[idx_hi].position.xyz;
        let scale_lo = sorted_particles[idx_lo].position.w;
        let scale_hi = sorted_particles[idx_hi].position.w;

        let trail_pos = mix(pos_lo, pos_hi, seg_t);
        let base_scale = mix(scale_lo, scale_hi, seg_t);
        let particle_scale = select(0.0, base_scale, is_active);

        // sample thickness curve LUT (index 0 = tail, index N = head)
        let last_curve_idx = TRAIL_THICKNESS_CURVE_SAMPLES - 1u;
        let curve_idx_f = (1.0 - section_frac) * f32(last_curve_idx);
        let curve_lo = min(u32(curve_idx_f), last_curve_idx);
        let curve_hi = min(curve_lo + 1u, last_curve_idx);
        let curve_t = curve_idx_f - f32(curve_lo);
        let thickness = mix(
            emitter_uniforms.trail_thickness_curve[curve_lo],
            emitter_uniforms.trail_thickness_curve[curve_hi],
            curve_t
        );

        // derive trail direction from adjacent segment positions
        var trail_dir: vec3<f32>;
        if (seg_lo != seg_hi) {
            trail_dir = pos_hi - pos_lo;
        } else {
            // at the tail end, use direction from previous segment
            let prev_pos = sorted_particles[head_slot + max(seg_lo, 1u) - 1u].position.xyz;
            trail_dir = pos_lo - prev_pos;
        }
        let dir_len = length(trail_dir);
        if (dir_len > 0.001) {
            trail_dir = trail_dir / dir_len;
        } else {
            trail_dir = -normalize(head_particle.velocity.xyz);
        }

        let orient = align_y_to_direction(trail_dir);

        // scale cross-section by width curve, flatten Y (position from trail_pos)
        var cross_section = vertex.position;
        cross_section.x *= thickness;
        cross_section.z *= thickness;
        cross_section.y = 0.0;

        let is_local = emitter_uniforms.use_local_coords != 0u;

        if (is_local) {
            let offset = orient * (cross_section * particle_scale);
            let local_pos = trail_pos + offset;
            out.world_position = mesh_functions::mesh_position_local_to_world(
                world_from_local, vec4(local_pos, 1.0)
            );
        } else {
            let emitter_scale = vec3(
                length(world_from_local[0].xyz),
                length(world_from_local[1].xyz),
                length(world_from_local[2].xyz),
            );
            let offset = orient * (cross_section * particle_scale * emitter_scale);
            out.world_position = vec4(trail_pos + offset, 1.0);
        }

        out.position = position_world_to_clip(out.world_position.xyz);

#ifdef VERTEX_NORMALS
        let rotated_normal = orient * vertex.normal;
        if (is_local) {
            out.world_normal = mesh_functions::mesh_normal_local_to_world(rotated_normal, vertex.instance_index);
        } else {
            out.world_normal = rotated_normal;
        }
#endif

#ifdef VERTEX_COLORS
        out.color = vertex.color * head_particle.color;
#endif

        return out;
    }

    let particle = sorted_particles[particle_index];

    let flags = bitcast<u32>(particle.custom.w);
    let is_active = (flags & PARTICLE_FLAG_ACTIVE) != 0u;

    let particle_position = particle.position.xyz;
    let particle_scale = select(0.0, particle.position.w, is_active);
    let is_local = emitter_uniforms.use_local_coords != 0u;

    var rotated_position = vertex.position;
#ifdef VERTEX_NORMALS
    var rotated_normal = vertex.normal;
#endif

    let transform_align = emitter_uniforms.transform_align;

    if transform_align == TRANSFORM_ALIGN_Y_TO_VELOCITY {
        let alignment_dir = particle.alignment_dir.xyz;
        let dir_length = length(alignment_dir);
        if dir_length > 0.0 {
            let rotation_matrix = align_y_with_ref(alignment_dir, particle.ref_up.xyz);
            rotated_position = rotation_matrix * vertex.position;
#ifdef VERTEX_NORMALS
            rotated_normal = rotation_matrix * vertex.normal;
#endif
        }
    }

    // angle rotation from alignment_dir.w (radians)
    let angle = particle.alignment_dir.w;
    if abs(angle) > 0.0001 {
        let cos_a = cos(angle);
        let sin_a = sin(angle);
        if (emitter_uniforms.particle_flags & EMITTER_FLAG_ROTATE_Y) != 0u {
            let angle_matrix = mat3x3<f32>(
                vec3(cos_a, 0.0, sin_a),
                vec3(0.0, 1.0, 0.0),
                vec3(-sin_a, 0.0, cos_a),
            );
            rotated_position = angle_matrix * rotated_position;
#ifdef VERTEX_NORMALS
            rotated_normal = angle_matrix * rotated_normal;
#endif
        } else {
            let angle_matrix = mat3x3<f32>(
                vec3(cos_a, sin_a, 0.0),
                vec3(-sin_a, cos_a, 0.0),
                vec3(0.0, 0.0, 1.0),
            );
            rotated_position = angle_matrix * rotated_position;
#ifdef VERTEX_NORMALS
            rotated_normal = angle_matrix * rotated_normal;
#endif
        }
    }

    let emitter_scale = vec3(
        length(world_from_local[0].xyz),
        length(world_from_local[1].xyz),
        length(world_from_local[2].xyz),
    );

    if transform_align == TRANSFORM_ALIGN_BILLBOARD || transform_align == TRANSFORM_ALIGN_BILLBOARD_Y_TO_VELOCITY || transform_align == TRANSFORM_ALIGN_BILLBOARD_FIXED_Y {
        let cam_right = normalize(view.world_from_view[0].xyz);
        let cam_up = normalize(view.world_from_view[1].xyz);
        let cam_forward = normalize(view.world_from_view[2].xyz);

        var particle_world_pos: vec3<f32>;
        if is_local {
            particle_world_pos = (world_from_local * vec4(particle_position, 1.0)).xyz;
        } else {
            particle_world_pos = particle_position;
        }
        let scale = vec3(particle_scale) * emitter_scale;

        if transform_align == TRANSFORM_ALIGN_BILLBOARD_Y_TO_VELOCITY {
            var v = particle.alignment_dir.xyz;
            if is_local {
                let emitter_rotation = mat3x3<f32>(
                    normalize(world_from_local[0].xyz),
                    normalize(world_from_local[1].xyz),
                    normalize(world_from_local[2].xyz)
                );
                v = emitter_rotation * v;
            }

            // project velocity onto the screen plane
            var sv = v - cam_forward * dot(cam_forward, v);
            if length(sv) < 0.001 {
                sv = cam_up;
            }
            sv = normalize(sv);

            let right = normalize(cross(sv, cam_forward));

            let scaled_vertex = rotated_position * scale;
            let pos = particle_world_pos
                + right * scaled_vertex.x
                + sv * scaled_vertex.y
                + cam_forward * scaled_vertex.z;

            out.world_position = vec4(pos, 1.0);
            out.position = position_world_to_clip(pos);

#ifdef VERTEX_NORMALS
            out.world_normal = right * rotated_normal.x
                + sv * rotated_normal.y
                + cam_forward * rotated_normal.z;
#endif
        } else if transform_align == TRANSFORM_ALIGN_BILLBOARD_FIXED_Y {
            // y-axis locked to world up, rotates around vertical axis to face camera
            let world_up = vec3(0.0, 1.0, 0.0);
            let right = normalize(cross(world_up, cam_forward));
            let forward = cross(right, world_up);

            let scaled_vertex = rotated_position * scale;
            let pos = particle_world_pos
                + right * scaled_vertex.x
                + world_up * scaled_vertex.y
                + forward * scaled_vertex.z;

            out.world_position = vec4(pos, 1.0);
            out.position = position_world_to_clip(pos);

#ifdef VERTEX_NORMALS
            out.world_normal = right * rotated_normal.x
                + world_up * rotated_normal.y
                + forward * rotated_normal.z;
#endif
        } else {
            // standard billboard
            let scaled_vertex = rotated_position * scale;
            let billboard_pos = particle_world_pos
                + cam_right * scaled_vertex.x
                + cam_up * scaled_vertex.y
                + cam_forward * scaled_vertex.z;

            out.world_position = vec4(billboard_pos, 1.0);
            out.position = position_world_to_clip(billboard_pos);

#ifdef VERTEX_NORMALS
            out.world_normal = cam_right * rotated_normal.x
                + cam_up * rotated_normal.y
                + cam_forward * rotated_normal.z;
#endif
        }
    } else {
        // non-billboard rendering
        let emitter_rotation = mat3x3(
            normalize(world_from_local[0].xyz),
            normalize(world_from_local[1].xyz),
            normalize(world_from_local[2].xyz)
        );

        if is_local {
            let offset = rotated_position * particle_scale;
            let local_position = offset + particle_position;
            out.world_position = mesh_functions::mesh_position_local_to_world(world_from_local, vec4(local_position, 1.0));
        } else {
            let offset = emitter_rotation * (rotated_position * particle_scale * emitter_scale);
            out.world_position = vec4(particle_position + offset, 1.0);
        }

        out.position = position_world_to_clip(out.world_position.xyz);

#ifdef VERTEX_NORMALS
        if is_local {
            out.world_normal = mesh_functions::mesh_normal_local_to_world(rotated_normal, vertex.instance_index);
        } else {
            out.world_normal = emitter_rotation * rotated_normal;
        }
#endif
    }

#ifdef VERTEX_COLORS
    out.color = vertex.color * particle.color;
#endif

    return out;
}

fn get_head_particle(particle_index: u32) -> Particle {
    let trail_size = emitter_uniforms.trail_size;
    let head_slot = particle_index * max(trail_size, 1u);
    return sorted_particles[head_slot];
}

// depth-only prepass fragment - discard inactive, no output needed
#ifdef PREPASS_PIPELINE
#ifndef PREPASS_FRAGMENT
@fragment
fn fragment(
    in: VertexOutput,
    @builtin(front_facing) is_front: bool,
) {
#ifdef VERTEX_UVS_B
    let particle = get_head_particle(u32(round(in.uv_b.x)));
#else
    let particle = sorted_particles[0u];
#endif

    let flags = bitcast<u32>(particle.custom.w);
    let is_active = (flags & PARTICLE_FLAG_ACTIVE) != 0u;

    if (!is_active || particle.color.a < 0.001) {
        discard;
    }
}
#endif
#endif

// deferred prepass fragment (normal/motion vector/deferred passes)
#ifdef PREPASS_PIPELINE
#ifdef PREPASS_FRAGMENT
@fragment
fn fragment(
    in: VertexOutput,
    @builtin(front_facing) is_front: bool,
) -> FragmentOutput {
#ifdef VERTEX_UVS_B
    let particle = get_head_particle(u32(round(in.uv_b.x)));
#else
    let particle = sorted_particles[0u];
#endif

    let flags = bitcast<u32>(particle.custom.w);
    let is_active = (flags & PARTICLE_FLAG_ACTIVE) != 0u;

    if (!is_active || particle.color.a < 0.001) {
        discard;
    }

    var pbr_input = pbr_input_from_standard_material(in, is_front);
    pbr_input.material.base_color = pbr_input.material.base_color * particle.color;
    let out = deferred_output(in, pbr_input);

    return out;
}
#endif
#endif

// forward rendering fragment
#ifndef PREPASS_PIPELINE
@fragment
fn fragment(
    in: VertexOutput,
    @builtin(front_facing) is_front: bool,
) -> FragmentOutput {
#ifdef VERTEX_UVS_B
    let particle = get_head_particle(u32(round(in.uv_b.x)));
#else
    let particle = sorted_particles[0u];
#endif

    let flags = bitcast<u32>(particle.custom.w);
    let is_active = (flags & PARTICLE_FLAG_ACTIVE) != 0u;

    if (!is_active || particle.color.a < 0.001) {
        discard;
    }

    var pbr_input = pbr_input_from_standard_material(in, is_front);
    pbr_input.material.base_color = pbr_input.material.base_color * particle.color;
    pbr_input.material.base_color = alpha_discard(pbr_input.material, pbr_input.material.base_color);

    let particle_alpha = pbr_input.material.base_color.a;

    var out: FragmentOutput;

    let is_unlit = (pbr_input.material.flags & STANDARD_MATERIAL_FLAGS_UNLIT_BIT) != 0u;

    if is_unlit {
        out.color = pbr_input.material.base_color + pbr_input.material.emissive;
    } else {
        out.color = apply_pbr_lighting(pbr_input);
    }

    out.color = main_pass_post_lighting_processing(pbr_input, out.color);
#ifndef PREMULTIPLY_ALPHA
    out.color.a = particle_alpha;
#endif

    return out;
}
#endif
