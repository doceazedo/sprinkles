#import bevy_ui::ui_vertex_output::UiVertexOutput
#import bevy_sprinkles_editor::common::rounded_box_sdf

const CURVE_WIDTH: f32 = 2.0;
const GRID_CURVE_WIDTH: f32 = 1.0;
const GRID_COLOR: vec4<f32> = vec4<f32>(0.3, 0.3, 0.3, 0.5);
const CURVE_COLOR: vec4<f32> = vec4<f32>(0.076, 0.215, 0.854, 0.8);
const FILL_COLOR: vec4<f32> = vec4<f32>(0.076, 0.215, 0.854, 0.2);

struct CurveUniforms {
    border_radius: f32,
    point_count: u32,
    range_min: f32,
    range_max: f32,
    positions_low: vec4<f32>,
    positions_high: vec4<f32>,
    values_low: vec4<f32>,
    values_high: vec4<f32>,
    modes_low: vec4<u32>,
    modes_high: vec4<u32>,
    tensions_low: vec4<f32>,
    tensions_high: vec4<f32>,
    easings_low: vec4<u32>,
    easings_high: vec4<u32>,
}

@group(1) @binding(0)
var<uniform> uniforms: CurveUniforms;

fn get_position(i: u32) -> f32 {
    if i < 4u { return uniforms.positions_low[i]; }
    return uniforms.positions_high[i - 4u];
}

fn get_value(i: u32) -> f32 {
    if i < 4u { return uniforms.values_low[i]; }
    return uniforms.values_high[i - 4u];
}

fn get_mode(i: u32) -> u32 {
    if i < 4u { return uniforms.modes_low[i]; }
    return uniforms.modes_high[i - 4u];
}

fn get_tension(i: u32) -> f32 {
    if i < 4u { return uniforms.tensions_low[i]; }
    return uniforms.tensions_high[i - 4u];
}

fn get_easing(i: u32) -> u32 {
    if i < 4u { return uniforms.easings_low[i]; }
    return uniforms.easings_high[i - 4u];
}

const PI: f32 = 3.14159265359;

fn apply_power(t: f32, tension: f32) -> f32 {
    if abs(tension) < 0.001 {
        return t;
    }
    let exp = 1.0 / (1.0 - abs(tension) * 0.999);
    if tension > 0.0 {
        return pow(t, exp);
    } else {
        return 1.0 - pow(1.0 - t, exp);
    }
}

fn apply_sine(t: f32, tension: f32) -> f32 {
    let intensity = abs(tension);
    if intensity < 0.001 {
        return t;
    }
    var eased: f32;
    if tension >= 0.0 {
        eased = 1.0 - cos(t * PI * 0.5);
    } else {
        eased = sin(t * PI * 0.5);
    }
    return t + (eased - t) * intensity;
}

fn apply_expo(t: f32, tension: f32) -> f32 {
    let intensity = abs(tension);
    if intensity < 0.001 {
        return t;
    }
    var eased: f32;
    if tension >= 0.0 {
        if t <= 0.0 { eased = 0.0; } else { eased = pow(2.0, 10.0 * (t - 1.0)); }
    } else {
        if t >= 1.0 { eased = 1.0; } else { eased = 1.0 - pow(2.0, -10.0 * t); }
    }
    return t + (eased - t) * intensity;
}

fn apply_circ(t: f32, tension: f32) -> f32 {
    let intensity = abs(tension);
    if intensity < 0.001 {
        return t;
    }
    var eased: f32;
    if tension >= 0.0 {
        eased = 1.0 - sqrt(1.0 - t * t);
    } else {
        eased = sqrt(1.0 - (1.0 - t) * (1.0 - t));
    }
    return t + (eased - t) * intensity;
}

fn apply_easing(t: f32, easing: u32, tension: f32) -> f32 {
    switch easing {
        case 0u: { return apply_power(t, tension); }
        case 1u: { return apply_sine(t, tension); }
        case 2u: { return apply_expo(t, tension); }
        case 3u: { return apply_circ(t, tension); }
        default: { return apply_power(t, tension); }
    }
}

fn apply_curve(t: f32, mode: u32, easing: u32, tension: f32) -> f32 {
    switch mode {
        case 0u: {
            return apply_easing(t, easing, tension);
        }
        case 1u: {
            if t < 0.5 {
                let local_t = t * 2.0;
                return apply_easing(local_t, easing, tension) * 0.5;
            } else {
                let local_t = (t - 0.5) * 2.0;
                return 0.5 + apply_easing(local_t, easing, -tension) * 0.5;
            }
        }
        case 2u: {
            return 0.0;
        }
        case 3u: {
            let steps = u32(2.0 + 64.0 * clamp(tension, 0.0, 1.0));
            let step_f = f32(steps);
            return floor(t * step_f) / max(step_f - 1.0, 1.0);
        }
        case 4u: {
            let steps = u32(2.0 + 64.0 * clamp(tension, 0.0, 1.0));
            let step_f = f32(steps);
            let step_size = 1.0 / step_f;
            let current_step = floor(t / step_size);
            let local_t = (t - current_step * step_size) / step_size;
            let smooth_t = local_t * local_t * (3.0 - 2.0 * local_t);
            let start = current_step / max(step_f - 1.0, 1.0);
            let end = min(current_step + 1.0, step_f - 1.0) / max(step_f - 1.0, 1.0);
            return start + (end - start) * smooth_t;
        }
        default: {
            return t;
        }
    }
}

fn sample_curve(x: f32) -> f32 {
    if uniforms.point_count == 0u {
        return 1.0;
    }
    if uniforms.point_count == 1u {
        return get_value(0u);
    }

    let t = clamp(x, 0.0, 1.0);

    var left_idx = 0u;
    var right_idx = uniforms.point_count - 1u;

    for (var i = 0u; i < uniforms.point_count; i++) {
        if get_position(i) <= t {
            left_idx = i;
        }
    }
    for (var i = 0u; i < uniforms.point_count; i++) {
        if get_position(i) >= t {
            right_idx = i;
            break;
        }
    }

    let left_pos = get_position(left_idx);
    let left_val = get_value(left_idx);
    let right_pos = get_position(right_idx);
    let right_val = get_value(right_idx);
    let right_mode = get_mode(right_idx);
    let right_tension = get_tension(right_idx);
    let right_easing = get_easing(right_idx);

    if left_idx == right_idx {
        return left_val;
    }

    let segment_range = right_pos - left_pos;
    if segment_range <= 0.0 {
        return left_val;
    }

    let local_t = (t - left_pos) / segment_range;

    // adjust tension based on slope direction so positive tension always bends down
    let slope_sign = sign(right_val - left_val);
    let effective_tension = right_tension * slope_sign;
    let curved_t = apply_curve(local_t, right_mode, right_easing, effective_tension);

    return left_val + (right_val - left_val) * curved_t;
}

fn normalize_value(value: f32) -> f32 {
    let range_span = uniforms.range_max - uniforms.range_min;
    if abs(range_span) < 0.001 {
        return 0.5;
    }
    return (value - uniforms.range_min) / range_span;
}

// distance from point p to line segment from a to b
fn dist_to_segment(p: vec2<f32>, a: vec2<f32>, b: vec2<f32>) -> f32 {
    let pa = p - a;
    let ba = b - a;
    let h = clamp(dot(pa, ba) / dot(ba, ba), 0.0, 1.0);
    return length(pa - ba * h);
}

@fragment
fn fragment(in: UiVertexOutput) -> @location(0) vec4<f32> {
    let pixel_pos = (in.uv - 0.5) * in.size;
    let half_size = in.size * 0.5;
    let d = rounded_box_sdf(pixel_pos, half_size, uniforms.border_radius);

    if d > 0.0 {
        return vec4<f32>(0.0);
    }

    var color = vec4<f32>(0.0);

    // draw grid
    let grid_divisions = 4.0;
    let grid_x = fract(in.uv.x * grid_divisions);
    let grid_y = fract(in.uv.y * grid_divisions);
    let grid_line_x = min(grid_x, 1.0 - grid_x) * in.size.x / grid_divisions;
    let grid_line_y = min(grid_y, 1.0 - grid_y) * in.size.y / grid_divisions;

    if grid_line_x < GRID_CURVE_WIDTH || grid_line_y < GRID_CURVE_WIDTH {
        color = mix(color, GRID_COLOR, GRID_COLOR.a);
    }

    // draw zero line
    let center_y = 1.0 - normalize_value(0.0);
    let center_dist = abs(in.uv.y - center_y) * in.size.y;
    if center_dist < GRID_CURVE_WIDTH * 0.5 && uniforms.range_min <= 0.0 && uniforms.range_max >= 0.0 {
        color = mix(color, vec4<f32>(0.5, 0.5, 0.5, 0.8), 0.8);
    }

    // compute distance to curve using polyline segments sampled at pixel intervals
    let px = vec2<f32>(in.uv.x * in.size.x, in.uv.y * in.size.y);
    var min_dist = 1000.0;
    let num_samples = i32(in.size.x);
    var prev_pos = vec2<f32>(0.0, (1.0 - normalize_value(sample_curve(0.0))) * in.size.y);

    for (var i = 1; i <= num_samples; i++) {
        let t = f32(i) / f32(num_samples);
        let curr_y = 1.0 - normalize_value(sample_curve(t));
        let curr_pos = vec2<f32>(t * in.size.x, curr_y * in.size.y);

        let seg_dist = dist_to_segment(px, prev_pos, curr_pos);
        min_dist = min(min_dist, seg_dist);

        prev_pos = curr_pos;
    }

    // fill below curve
    let curve_y = (1.0 - normalize_value(sample_curve(in.uv.x))) * in.size.y;
    if px.y > curve_y {
        color = mix(color, FILL_COLOR, FILL_COLOR.a);
    }

    // draw curve with anti-aliasing
    if min_dist < CURVE_WIDTH {
        let line_alpha = 1.0 - smoothstep(CURVE_WIDTH * 0.5, CURVE_WIDTH, min_dist);
        color = mix(color, CURVE_COLOR, line_alpha);
    }

    let edge_alpha = 1.0 - smoothstep(-1.0, 1.0, d);
    color.a *= edge_alpha;

    return color;
}
