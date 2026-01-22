// particle data structure matching ParticleData in Rust
struct Particle {
    position: vec4<f32>,  // xyz + scale
    velocity: vec4<f32>,  // xyz + lifetime_remaining
    color: vec4<f32>,     // rgba
    custom: vec4<f32>,    // age, spawn_index, seed, flags
}


const PARTICLE_FLAG_ACTIVE: u32 = 1u;

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

// particle storage buffer at binding 100 to avoid conflict with StandardMaterial bindings
@group(#{MATERIAL_BIND_GROUP}) @binding(100) var<storage, read> particles: array<Particle>;
// particle indices for draw order sorting
@group(#{MATERIAL_BIND_GROUP}) @binding(101) var<storage, read> particle_indices: array<u32>;

@vertex
fn vertex(vertex: Vertex) -> VertexOutput {
    var out: VertexOutput;

    // get particle index from mesh tag through indirection buffer
    let tag = mesh_functions::get_tag(vertex.instance_index);
    let particle_index = particle_indices[tag];
    let particle = particles[particle_index];

    // check if particle is active
    let flags = bitcast<u32>(particle.custom.w);
    let is_active = (flags & PARTICLE_FLAG_ACTIVE) != 0u;

    // get particle transform data
    let particle_position = particle.position.xyz;
    let particle_scale = select(0.0, particle.position.w, is_active);

    // scale vertex position by particle scale
    let scaled_position = vertex.position * particle_scale;

    // translate to particle position
    let local_position = scaled_position + particle_position;

    // get world transform matrix
    var world_from_local = mesh_functions::get_world_from_local(vertex.instance_index);

    // compute world position
    out.world_position = mesh_functions::mesh_position_local_to_world(world_from_local, vec4(local_position, 1.0));

    // compute clip position
    out.position = position_world_to_clip(out.world_position.xyz);

    // transform normal to world space
#ifdef VERTEX_NORMALS
    out.world_normal = mesh_functions::mesh_normal_local_to_world(vertex.normal, vertex.instance_index);
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

@fragment
fn fragment(
    in: VertexOutput,
    @builtin(front_facing) is_front: bool,
) -> FragmentOutput {
    // get particle index from instance index through indirection buffer
#ifdef VERTEX_OUTPUT_INSTANCE_INDEX
    let tag = mesh_functions::get_tag(in.instance_index);
    let particle_index = particle_indices[tag];
#else
    let particle_index = 0u;
#endif
    let particle = particles[particle_index];

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

    var out: FragmentOutput;

    // apply PBR lighting
    out.color = apply_pbr_lighting(pbr_input);

    // apply post-processing (fog, tonemapping, etc.)
    out.color = main_pass_post_lighting_processing(pbr_input, out.color);
#endif

    return out;
}
