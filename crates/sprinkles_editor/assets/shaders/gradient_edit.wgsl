#import bevy_ui::ui_vertex_output::UiVertexOutput
#import bevy_ui_render::color_space::srgb_to_linear_rgb
#import sprinkles_editor::common::{checkerboard, rounded_box_sdf}

const MAX_STOPS: u32 = 8u;

struct GradientUniforms {
    border_radius: f32,
    checkerboard_size: f32,
    stop_count: u32,
    interpolation: u32,
    positions: array<vec4<f32>, 2>,
    colors: array<vec4<f32>, 8>,
}

@group(1) @binding(0)
var<uniform> uniforms: GradientUniforms;

fn get_stop_position(index: u32) -> f32 {
    let arr_idx = index / 4u;
    let vec_idx = index % 4u;
    if arr_idx == 0u {
        return uniforms.positions[0][vec_idx];
    }
    return uniforms.positions[1][vec_idx];
}

fn get_stop_color(index: u32) -> vec4<f32> {
    return uniforms.colors[index];
}

fn smoothstep_interp(edge0: f32, edge1: f32, x: f32) -> f32 {
    let t = clamp((x - edge0) / (edge1 - edge0), 0.0, 1.0);
    return t * t * (3.0 - 2.0 * t);
}

fn sample_gradient(t: f32) -> vec4<f32> {
    if uniforms.stop_count == 0u {
        return vec4<f32>(1.0);
    }
    if uniforms.stop_count == 1u {
        return get_stop_color(0u);
    }

    let clamped_t = clamp(t, 0.0, 1.0);

    var left_idx = 0u;
    var right_idx = uniforms.stop_count - 1u;

    for (var i = 0u; i < uniforms.stop_count; i = i + 1u) {
        if get_stop_position(i) <= clamped_t {
            left_idx = i;
        }
    }

    for (var i = 0u; i < uniforms.stop_count; i = i + 1u) {
        if get_stop_position(i) >= clamped_t {
            right_idx = i;
            break;
        }
    }

    let left_pos = get_stop_position(left_idx);
    let right_pos = get_stop_position(right_idx);
    let left_color = get_stop_color(left_idx);
    let right_color = get_stop_color(right_idx);

    if left_idx == right_idx {
        return left_color;
    }

    let segment_range = right_pos - left_pos;
    if segment_range <= 0.0 {
        return left_color;
    }

    let local_t = (clamped_t - left_pos) / segment_range;

    var interp_t: f32;
    switch uniforms.interpolation {
        case 0u: {
            interp_t = 0.0;
        }
        case 1u: {
            interp_t = local_t;
        }
        case 2u: {
            interp_t = smoothstep_interp(0.0, 1.0, local_t);
        }
        default: {
            interp_t = local_t;
        }
    }

    return mix(left_color, right_color, interp_t);
}

@fragment
fn fragment(in: UiVertexOutput) -> @location(0) vec4<f32> {
    let gradient_color = sample_gradient(in.uv.x);

    let checker_color_light = vec3<f32>(1.0, 1.0, 1.0);
    let checker_color_dark = srgb_to_linear_rgb(vec3<f32>(0.8, 0.8, 0.8));

    let cell_count = in.size.x / uniforms.checkerboard_size;
    let checker = checkerboard(in.uv, cell_count);
    let checker_rgb = mix(checker_color_dark, checker_color_light, checker);

    let final_rgb = mix(checker_rgb, srgb_to_linear_rgb(gradient_color.rgb), gradient_color.a);

    let pixel_pos = (in.uv - 0.5) * in.size;
    let half_size = in.size * 0.5;
    let d = rounded_box_sdf(pixel_pos, half_size, uniforms.border_radius);
    let mask_alpha = 1.0 - smoothstep(-1.0, 1.0, d);

    return vec4<f32>(final_rgb, mask_alpha);
}
