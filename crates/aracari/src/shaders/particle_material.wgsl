#import aracari::common::{
    Particle,
    PARTICLE_FLAG_ACTIVE,
    EMITTER_FLAG_ALIGN_Y_TO_VELOCITY,
}
#import bevy_pbr::{
    mesh_functions,
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

// sorted particle data buffer - particles are written here in draw order by the sort compute shader
// instance 0 contains the first particle to render (back-most), instance N is the last (front-most)
@group(#{MATERIAL_BIND_GROUP}) @binding(100) var<storage, read> sorted_particles: array<Particle>;
@group(#{MATERIAL_BIND_GROUP}) @binding(101) var<uniform> max_particles: u32;
@group(#{MATERIAL_BIND_GROUP}) @binding(102) var<uniform> particle_flags: u32;

// helper function to compute a rotation matrix that aligns Y axis to a direction
// based on godot's TRANSFORM_ALIGN_Y_TO_VELOCITY implementation
fn align_y_to_direction(dir: vec3<f32>) -> mat3x3<f32> {
    let y_axis = normalize(dir);

    // use world Z as reference (like godot does)
    var z_ref = vec3(0.0, 0.0, 1.0);

    // compute X axis from Y cross Z
    var x_axis = cross(y_axis, z_ref);
    let x_len = length(x_axis);

    // if Y is nearly parallel to Z, use world X as reference instead
    if x_len < 0.001 {
        x_axis = normalize(cross(y_axis, vec3(1.0, 0.0, 0.0)));
    } else {
        x_axis = x_axis / x_len;
    }

    // compute Z axis from X cross Y to ensure orthonormal basis
    let z_axis = cross(x_axis, y_axis);

    return mat3x3<f32>(x_axis, y_axis, z_axis);
}

@vertex
fn vertex(vertex: Vertex) -> VertexOutput {
    var out: VertexOutput;

    // read particle index from uv_b.x (encoded during mesh generation)
    // this eliminates reliance on instance_index which isn't guaranteed to match particle order
    let particle_index = u32(vertex.uv_b.x);
    let particle = sorted_particles[particle_index];

    // check if particle is active
    let flags = bitcast<u32>(particle.custom.w);
    let is_active = (flags & PARTICLE_FLAG_ACTIVE) != 0u;

    // get particle transform data
    let particle_position = particle.position.xyz;
    let particle_scale = select(0.0, particle.position.w, is_active);

    // compute particle rotation based on flags
    var rotated_position = vertex.position;
#ifdef VERTEX_NORMALS
    var rotated_normal = vertex.normal;
#endif

    // use alignment_dir for ALIGN_Y_TO_VELOCITY (like godot)
    // alignment_dir is updated only when velocity > 0, preserving direction when stopped
    if (particle_flags & EMITTER_FLAG_ALIGN_Y_TO_VELOCITY) != 0u {
        let alignment_dir = particle.alignment_dir.xyz;
        let dir_length = length(alignment_dir);
        if dir_length > 0.0 {
            let rotation_matrix = align_y_to_direction(alignment_dir);
            rotated_position = rotation_matrix * vertex.position;
#ifdef VERTEX_NORMALS
            rotated_normal = rotation_matrix * vertex.normal;
#endif
        }
    }

    // scale vertex position by particle scale
    let scaled_position = rotated_position * particle_scale;

    // translate to particle position
    let local_position = scaled_position + particle_position;

    // get world transform matrix using the mesh's instance index
    var world_from_local = mesh_functions::get_world_from_local(vertex.instance_index);

    // compute world position
    out.world_position = mesh_functions::mesh_position_local_to_world(world_from_local, vec4(local_position, 1.0));

    // compute clip position
    out.position = position_world_to_clip(out.world_position.xyz);

    // transform normal to world space (use rotated normal if ALIGN_Y_TO_VELOCITY)
#ifdef VERTEX_NORMALS
    out.world_normal = mesh_functions::mesh_normal_local_to_world(rotated_normal, vertex.instance_index);
#endif

#ifdef VERTEX_UVS_A
    out.uv = vertex.uv;
#endif

#ifdef VERTEX_UVS_B
    out.uv_b = vertex.uv_b;
#endif

#ifdef VERTEX_TANGENTS
    out.world_tangent = mesh_functions::mesh_tangent_local_to_world(world_from_local, vertex.tangent, vertex.instance_index);
#endif

#ifdef VERTEX_COLORS
    out.color = vertex.color * particle.color;
#else
    // store particle color for fragment shader (requires VERTEX_COLORS)
#endif

#ifdef VERTEX_OUTPUT_INSTANCE_INDEX
    out.instance_index = vertex.instance_index;
#endif

    return out;
}

// depth-only prepass fragment (shadow/depth pass) - just discard inactive, no output needed
#ifdef PREPASS_PIPELINE
#ifndef PREPASS_FRAGMENT
@fragment
fn fragment(
    in: VertexOutput,
    @builtin(front_facing) is_front: bool,
) {
#ifdef VERTEX_UVS_B
    let particle_index = u32(in.uv_b.x);
    let particle = sorted_particles[particle_index];
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
    let particle_index = u32(in.uv_b.x);
    let particle = sorted_particles[particle_index];
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
    let particle_index = u32(in.uv_b.x);
    let particle = sorted_particles[particle_index];
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
    out.color.a = particle_alpha;

    return out;
}
#endif
