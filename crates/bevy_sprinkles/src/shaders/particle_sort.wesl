import super::common::{Particle, PARTICLE_FLAG_ACTIVE};

const DRAW_ORDER_INDEX: u32 = 0u;
const DRAW_ORDER_LIFETIME: u32 = 1u;
const DRAW_ORDER_REVERSE_LIFETIME: u32 = 2u;
const DRAW_ORDER_VIEW_DEPTH: u32 = 3u;

struct SortParams {
    amount: u32,
    draw_order: u32,
    stage: u32,
    step: u32,
    camera_position: vec3<f32>,
    _pad1: f32,
    camera_forward: vec3<f32>,
    _pad2: f32,
    emitter_transform: mat4x4<f32>,
    trail_size: u32,
    _trail_pad0: u32,
    _trail_pad1: u32,
    _trail_pad2: u32,
}

@group(0) @binding(0) var<uniform> params: SortParams;
@group(0) @binding(1) var<storage, read_write> particles: array<Particle>;
@group(0) @binding(2) var<storage, read_write> indices: array<u32>;
// output buffer: particle data written in sorted order for rendering
@group(0) @binding(3) var<storage, read_write> sorted_particles: array<Particle>;

fn get_sort_key(particle_index: u32) -> f32 {
    // for trail segments, use the head particle's sort key
    var head_index = particle_index;
    if (params.trail_size > 1u) {
        head_index = (particle_index / params.trail_size) * params.trail_size;
    }

    let particle = particles[head_index];
    let flags = bitcast<u32>(particle.custom.w);
    let is_active = (flags & PARTICLE_FLAG_ACTIVE) != 0u;

    // inactive particles sort to the back
    if (!is_active) {
        return -1e10;
    }

    switch (params.draw_order) {
        case DRAW_ORDER_INDEX: {
            // emission order (lowest index first, highest last = front)
            return f32(head_index);
        }
        case DRAW_ORDER_LIFETIME: {
            // highest remaining lifetime drawn at front
            let age = particle.custom.x;
            let lifetime = particle.velocity.w;
            let remaining = lifetime - age;
            return remaining;
        }
        case DRAW_ORDER_REVERSE_LIFETIME: {
            // lowest remaining lifetime drawn at front
            let age = particle.custom.x;
            let lifetime = particle.velocity.w;
            let remaining = lifetime - age;
            return -remaining;
        }
        case DRAW_ORDER_VIEW_DEPTH: {
            // depth along camera view axis (farthest first for transparency)
            // dot product with camera forward gives correct view-relative depth
            let local_pos = particle.position.xyz;
            let world_pos = (params.emitter_transform * vec4(local_pos, 1.0)).xyz;
            let to_particle = world_pos - params.camera_position;
            let depth = dot(to_particle, params.camera_forward);
            return -depth;
        }
        default: {
            return f32(head_index);
        }
    }
}

// bitonic sort: compare and swap based on current stage and step
// when trail_size > 1, sort operates on trail groups (indices store head slot indices)
@compute @workgroup_size(256)
fn sort(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let ts = params.trail_size;
    let group_count = params.amount / max(ts, 1u);
    let idx = global_id.x;
    if (idx >= group_count) {
        return;
    }

    let d = 1u << params.step;
    let block_2d = 2u * d;
    let within_block = idx % block_2d;
    if (within_block >= d) {
        return;
    }

    let partner = idx + d;
    if (partner >= group_count) {
        return;
    }

    let stage_block_size = 2u << params.stage;
    let stage_block_idx = idx / stage_block_size;
    let ascending = (stage_block_idx % 2u) == 0u;

    let idx_a = indices[idx];
    let idx_b = indices[partner];

    let key_a = get_sort_key(idx_a);
    let key_b = get_sort_key(idx_b);

    var should_swap = false;
    if (ascending) {
        should_swap = key_a > key_b;
    } else {
        should_swap = key_a < key_b;
    }

    if (should_swap) {
        indices[idx] = idx_b;
        indices[partner] = idx_a;
    }
}

// initialize indices: one entry per trail group, storing the head slot index
@compute @workgroup_size(256)
fn init_indices(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let idx = global_id.x;
    let ts = params.trail_size;
    let group_count = params.amount / max(ts, 1u);
    if (idx >= group_count) {
        return;
    }
    indices[idx] = idx * ts;
}

// copy particle data to sorted output buffer, expanding trail groups
@compute @workgroup_size(256)
fn copy_sorted(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let idx = global_id.x;
    if (idx >= params.amount) {
        return;
    }

    let ts = params.trail_size;
    if (ts <= 1u) {
        let particle_index = indices[idx];
        sorted_particles[idx] = particles[particle_index];
    } else {
        // map output slot to group and segment within group
        let group_idx = idx / ts;
        let within_group = idx % ts;
        let src_head = indices[group_idx];
        sorted_particles[idx] = particles[src_head + within_group];
    }
}
