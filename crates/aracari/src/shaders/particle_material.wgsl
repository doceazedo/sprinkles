// particle data structure matching ParticleData in Rust
struct Particle {
    position: vec4<f32>,  // xyz + scale
    velocity: vec4<f32>,  // xyz + lifetime_remaining
    color: vec4<f32>,     // rgba
    custom: vec4<f32>,    // age, spawn_index, seed, flags
}


// per-particle flags (stored in particle.custom.w)
const PARTICLE_FLAG_ACTIVE: u32 = 1u;

// emitter-level particle flags (from particle_flags uniform)
const EMITTER_FLAG_ALIGN_Y_TO_VELOCITY: u32 = 1u;
const EMITTER_FLAG_DISABLE_Z: u32 = 4u;

// standard material flags (from bevy_pbr::pbr_types)
const STANDARD_MATERIAL_FLAGS_UNLIT_BIT: u32 = 1u << 5u;

#import bevy_pbr::{
    mesh_functions,
    view_transformations::position_world_to_clip,
}

#ifdef PREPASS_PIPELINE
#import bevy_pbr::{
    prepass_io::{Vertex, VertexOutput, FragmentOutput},
    pbr_deferred_functions::deferred_output,
}
#else
#import bevy_pbr::{
    forward_io::{Vertex, VertexOutput, FragmentOutput},
    pbr_fragment::pbr_input_from_standard_material,
    pbr_functions::{apply_pbr_lighting, main_pass_post_lighting_processing, alpha_discard},
}
#endif

// sorted particle data buffer - particles are written here in draw order by the sort compute shader
// instance 0 contains the first particle to render (back-most), instance N is the last (front-most)
@group(#{MATERIAL_BIND_GROUP}) @binding(100) var<storage, read> sorted_particles: array<Particle>;
@group(#{MATERIAL_BIND_GROUP}) @binding(101) var<uniform> max_particles: u32;
@group(#{MATERIAL_BIND_GROUP}) @binding(102) var<uniform> particle_flags: u32;

// helper function to compute a rotation matrix that aligns Y axis to a direction
fn align_y_to_direction(dir: vec3<f32>) -> mat3x3<f32> {
    let y_axis = normalize(dir);

    // find a perpendicular axis for X
    var up = vec3(0.0, 1.0, 0.0);
    // if Y is nearly parallel to world up, use a different reference
    if abs(dot(y_axis, up)) > 0.999 {
        up = vec3(0.0, 0.0, 1.0);
    }

    let x_axis = normalize(cross(y_axis, up));
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

    // get velocity for ALIGN_Y_TO_VELOCITY
    let velocity = particle.velocity.xyz;

    // compute particle rotation based on flags
    var rotated_position = vertex.position;
    var rotated_normal = vertex.normal;

    if (particle_flags & EMITTER_FLAG_ALIGN_Y_TO_VELOCITY) != 0u {
        let vel_length = length(velocity);
        if vel_length > 0.0001 {
            let rotation_matrix = align_y_to_direction(velocity);
            rotated_position = rotation_matrix * vertex.position;
            rotated_normal = rotation_matrix * vertex.normal;
        }
    }

    // scale vertex position by particle scale
    let scaled_position = rotated_position * particle_scale;

    // translate to particle position
    let local_position = scaled_position + particle_position;

    // get world transform matrix (single mesh entity per emitter, so index 0)
    var world_from_local = mesh_functions::get_world_from_local(0u);

    // compute world position
    out.world_position = mesh_functions::mesh_position_local_to_world(world_from_local, vec4(local_position, 1.0));

    // compute clip position
    out.position = position_world_to_clip(out.world_position.xyz);

    // transform normal to world space (use rotated normal if ALIGN_Y_TO_VELOCITY)
#ifdef VERTEX_NORMALS
    out.world_normal = mesh_functions::mesh_normal_local_to_world(rotated_normal, 0u);
#endif

#ifdef VERTEX_UVS_A
    out.uv = vertex.uv;
#endif

#ifdef VERTEX_UVS_B
    out.uv_b = vertex.uv_b;
#endif

#ifdef VERTEX_TANGENTS
    out.world_tangent = mesh_functions::mesh_tangent_local_to_world(world_from_local, vertex.tangent, 0u);
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

@fragment
fn fragment(
    in: VertexOutput,
    @builtin(front_facing) is_front: bool,
) -> FragmentOutput {
    // read particle index from interpolated uv_b.x
    // since all vertices of each quad have the same uv_b.x, interpolation yields the same value
#ifdef VERTEX_UVS_B
    let particle_index = u32(in.uv_b.x);
    let particle = sorted_particles[particle_index];
#else
    let particle = sorted_particles[0u];
#endif

    // check if particle is active
    let flags = bitcast<u32>(particle.custom.w);
    let is_active = (flags & PARTICLE_FLAG_ACTIVE) != 0u;

    // discard inactive particles
    if (!is_active || particle.color.a < 0.001) {
        discard;
    }

#ifdef PREPASS_PIPELINE
    var pbr_input = pbr_input_from_standard_material(in, is_front);
    pbr_input.material.base_color = pbr_input.material.base_color * particle.color;
    let out = deferred_output(in, pbr_input);
#else
    // generate PbrInput from StandardMaterial bindings
    var pbr_input = pbr_input_from_standard_material(in, is_front);

    // multiply base color by particle color
    pbr_input.material.base_color = pbr_input.material.base_color * particle.color;

    // alpha discard
    pbr_input.material.base_color = alpha_discard(pbr_input.material, pbr_input.material.base_color);

    // store the alpha before PBR lighting (which may overwrite it)
    let particle_alpha = pbr_input.material.base_color.a;

    var out: FragmentOutput;

    // check if material is unlit
    let is_unlit = (pbr_input.material.flags & STANDARD_MATERIAL_FLAGS_UNLIT_BIT) != 0u;

    if is_unlit {
        // for unlit materials, use base color + emissive directly
        out.color = pbr_input.material.base_color + pbr_input.material.emissive;
    } else {
        // apply PBR lighting for lit materials
        out.color = apply_pbr_lighting(pbr_input);
    }

    // apply post-processing (fog, tonemapping, etc.)
    out.color = main_pass_post_lighting_processing(pbr_input, out.color);

    // restore particle alpha for proper blending
    out.color.a = particle_alpha;
#endif

    return out;
}
