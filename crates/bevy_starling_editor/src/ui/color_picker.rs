use bevy_egui::egui::{self, Color32, CornerRadius, FontId, Pos2, Rect, Response, Sense, Stroke, Vec2};
use bevy_starling::asset::{Gradient, GradientInterpolation, GradientStop};
use egui_remixicon::icons;

use super::styles::{colors, TEXT_BASE, TEXT_SM};

const POPOVER_WIDTH: f32 = 256.0;
const SPACING: f32 = 4.0;
const HUE_BAR_WIDTH: f32 = 16.0;
const CHANNEL_BAR_HEIGHT: f32 = 20.0;
const SELECTOR_CIRCLE_RADIUS: f32 = 4.0;
const SELECTOR_RECT_HEIGHT: f32 = 4.0;
const SELECTOR_RECT_OVERFLOW: f32 = 2.0;
const CHECKER_SIZE: f32 = 4.0;
const CORNER_RADIUS: f32 = 2.0;
const VALUE_INPUT_WIDTH: f32 = 36.0;

const GRADIENT_STOP_SIZE: f32 = 16.0;
const GRADIENT_STOP_PADDING: f32 = 2.0;
const GRADIENT_STOP_ARROW_SIZE: f32 = 4.0;
const GRADIENT_BAR_HEIGHT: f32 = 24.0;
const GRADIENT_BAR_PADDING: f32 = 4.0;

fn rgb_to_hsv(r: f32, g: f32, b: f32) -> (f32, f32, f32) {
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let delta = max - min;

    let h = if delta == 0.0 {
        0.0
    } else if max == r {
        60.0 * (((g - b) / delta) % 6.0)
    } else if max == g {
        60.0 * (((b - r) / delta) + 2.0)
    } else {
        60.0 * (((r - g) / delta) + 4.0)
    };

    let h = if h < 0.0 { h + 360.0 } else { h };

    let s = if max == 0.0 { 0.0 } else { delta / max };
    let v = max;

    (h, s, v)
}

fn hsv_to_rgb(h: f32, s: f32, v: f32) -> (f32, f32, f32) {
    let c = v * s;
    let h_prime = h / 60.0;
    let x = c * (1.0 - ((h_prime % 2.0) - 1.0).abs());
    let m = v - c;

    let (r, g, b) = if h_prime < 1.0 {
        (c, x, 0.0)
    } else if h_prime < 2.0 {
        (x, c, 0.0)
    } else if h_prime < 3.0 {
        (0.0, c, x)
    } else if h_prime < 4.0 {
        (0.0, x, c)
    } else if h_prime < 5.0 {
        (x, 0.0, c)
    } else {
        (c, 0.0, x)
    };

    (r + m, g + m, b + m)
}

fn hue_to_rgb(h: f32) -> Color32 {
    let (r, g, b) = hsv_to_rgb(h, 1.0, 1.0);
    Color32::from_rgb((r * 255.0) as u8, (g * 255.0) as u8, (b * 255.0) as u8)
}

fn draw_selector_circle(ui: &mut egui::Ui, center: Pos2, color: Color32) {
    let painter = ui.painter();
    // fill with current color
    painter.circle_filled(center, SELECTOR_CIRCLE_RADIUS, color);
    // outer white border
    painter.circle_stroke(center, SELECTOR_CIRCLE_RADIUS + 1.0, Stroke::new(1.0, Color32::WHITE));
    // inner black border
    painter.circle_stroke(center, SELECTOR_CIRCLE_RADIUS, Stroke::new(1.0, Color32::BLACK));
}

fn draw_selector_rect_horizontal(ui: &mut egui::Ui, center_y: f32, rect: Rect, color: Color32) {
    let painter = ui.painter();
    let corner_radius = CornerRadius::same(CORNER_RADIUS as u8);
    let selector_rect = Rect::from_center_size(
        Pos2::new(rect.center().x, center_y),
        Vec2::new(rect.width() + SELECTOR_RECT_OVERFLOW * 2.0, SELECTOR_RECT_HEIGHT),
    );
    // fill with current color
    painter.rect_filled(selector_rect, corner_radius, color);
    // outer white border
    painter.rect_stroke(selector_rect, corner_radius, Stroke::new(1.0, Color32::WHITE), egui::StrokeKind::Outside);
    // inner black border
    painter.rect_stroke(selector_rect, corner_radius, Stroke::new(1.0, Color32::BLACK), egui::StrokeKind::Inside);
}

fn draw_selector_rect_vertical(ui: &mut egui::Ui, center_x: f32, rect: Rect, color: Color32) {
    let painter = ui.painter();
    let corner_radius = CornerRadius::same(CORNER_RADIUS as u8);
    let selector_rect = Rect::from_center_size(
        Pos2::new(center_x, rect.center().y),
        Vec2::new(SELECTOR_RECT_HEIGHT, rect.height() + SELECTOR_RECT_OVERFLOW * 2.0),
    );
    // fill with current color
    painter.rect_filled(selector_rect, corner_radius, color);
    // outer white border
    painter.rect_stroke(selector_rect, corner_radius, Stroke::new(1.0, Color32::WHITE), egui::StrokeKind::Outside);
    // inner black border
    painter.rect_stroke(selector_rect, corner_radius, Stroke::new(1.0, Color32::BLACK), egui::StrokeKind::Inside);
}

pub fn rgba_to_color32(rgba: [f32; 4]) -> Color32 {
    Color32::from_rgba_unmultiplied(
        (rgba[0] * 255.0) as u8,
        (rgba[1] * 255.0) as u8,
        (rgba[2] * 255.0) as u8,
        (rgba[3] * 255.0) as u8,
    )
}

fn draw_checkerboard(ui: &mut egui::Ui, rect: Rect) {
    let painter = ui.painter();
    let cols = (rect.width() / CHECKER_SIZE).ceil() as i32;
    let rows = (rect.height() / CHECKER_SIZE).ceil() as i32;

    for row in 0..rows {
        for col in 0..cols {
            let is_light = (row + col) % 2 == 0;
            let color = if is_light {
                Color32::from_gray(180)
            } else {
                Color32::from_gray(120)
            };

            let cell_rect = Rect::from_min_size(
                Pos2::new(
                    rect.min.x + col as f32 * CHECKER_SIZE,
                    rect.min.y + row as f32 * CHECKER_SIZE,
                ),
                Vec2::splat(CHECKER_SIZE),
            );

            // clip to parent rect
            let clipped = cell_rect.intersect(rect);
            if clipped.width() > 0.0 && clipped.height() > 0.0 {
                painter.rect_filled(clipped, CornerRadius::ZERO, color);
            }
        }
    }
}

fn hsv_square(ui: &mut egui::Ui, hue: f32, saturation: &mut f32, value: &mut f32, size: f32) -> bool {
    let (rect, response) = ui.allocate_exact_size(Vec2::splat(size), Sense::click_and_drag());
    let mut changed = false;
    let corner_radius = CornerRadius::same(CORNER_RADIUS as u8);

    if ui.input(|i| i.pointer.any_down()) && response.contains_pointer() {
        if let Some(pos) = ui.input(|i| i.pointer.interact_pos()) {
            let local_pos = pos - rect.min;
            *saturation = (local_pos.x / size).clamp(0.0, 1.0);
            *value = 1.0 - (local_pos.y / size).clamp(0.0, 1.0);
            changed = true;
        }
    }

    if ui.is_rect_visible(rect) {
        let painter = ui.painter();
        let r = CORNER_RADIUS;

        // helper to get color at normalized position
        let color_at = |sx: f32, sy: f32| -> Color32 {
            let (rv, gv, bv) = hsv_to_rgb(hue, sx, 1.0 - sy);
            Color32::from_rgb((rv * 255.0) as u8, (gv * 255.0) as u8, (bv * 255.0) as u8)
        };

        // draw the HSV square using a mesh for smooth gradients
        let mut mesh = egui::Mesh::default();

        let steps = 32;
        for y in 0..steps {
            for x in 0..steps {
                let x0 = rect.min.x + (x as f32 / steps as f32) * size;
                let x1 = rect.min.x + ((x + 1) as f32 / steps as f32) * size;
                let y0 = rect.min.y + (y as f32 / steps as f32) * size;
                let y1 = rect.min.y + ((y + 1) as f32 / steps as f32) * size;

                let s0 = x as f32 / steps as f32;
                let s1 = (x + 1) as f32 / steps as f32;
                let v0 = y as f32 / steps as f32;
                let v1 = (y + 1) as f32 / steps as f32;

                let c00 = color_at(s0, v0);
                let c10 = color_at(s1, v0);
                let c01 = color_at(s0, v1);
                let c11 = color_at(s1, v1);

                let idx = mesh.vertices.len() as u32;
                mesh.vertices.push(egui::epaint::Vertex { pos: Pos2::new(x0, y0), uv: egui::epaint::WHITE_UV, color: c00 });
                mesh.vertices.push(egui::epaint::Vertex { pos: Pos2::new(x1, y0), uv: egui::epaint::WHITE_UV, color: c10 });
                mesh.vertices.push(egui::epaint::Vertex { pos: Pos2::new(x0, y1), uv: egui::epaint::WHITE_UV, color: c01 });
                mesh.vertices.push(egui::epaint::Vertex { pos: Pos2::new(x1, y1), uv: egui::epaint::WHITE_UV, color: c11 });

                mesh.indices.extend_from_slice(&[idx, idx + 1, idx + 2, idx + 1, idx + 3, idx + 2]);
            }
        }

        painter.add(egui::Shape::mesh(mesh));

        // draw corner covers with background color, then quarter circles with corner colors
        // mask size is corner radius + 1px offset to fully cover overflow
        let bg = colors::WINDOW_BG;
        let mask_r = r + 1.0;

        // corner centers and colors
        let corners = [
            (Pos2::new(rect.min.x + mask_r, rect.min.y + mask_r), color_at(0.0, 0.0)),  // top-left
            (Pos2::new(rect.max.x - mask_r, rect.min.y + mask_r), color_at(1.0, 0.0)),  // top-right
            (Pos2::new(rect.min.x + mask_r, rect.max.y - mask_r), color_at(0.0, 1.0)),  // bottom-left
            (Pos2::new(rect.max.x - mask_r, rect.max.y - mask_r), color_at(1.0, 1.0)),  // bottom-right
        ];

        let corner_rects = [
            Rect::from_min_size(rect.min, Vec2::splat(mask_r)),
            Rect::from_min_size(Pos2::new(rect.max.x - mask_r, rect.min.y), Vec2::splat(mask_r)),
            Rect::from_min_size(Pos2::new(rect.min.x, rect.max.y - mask_r), Vec2::splat(mask_r)),
            Rect::from_min_size(Pos2::new(rect.max.x - mask_r, rect.max.y - mask_r), Vec2::splat(mask_r)),
        ];

        for i in 0..4 {
            painter.rect_filled(corner_rects[i], CornerRadius::ZERO, bg);
            painter.circle_filled(corners[i].0, mask_r, corners[i].1);
        }

        // draw border
        painter.rect_stroke(rect, corner_radius, Stroke::new(1.0, colors::BORDER), egui::StrokeKind::Inside);

        // draw selector circle
        let circle_pos = Pos2::new(
            rect.min.x + *saturation * size,
            rect.min.y + (1.0 - *value) * size,
        );
        let (r, g, b) = hsv_to_rgb(hue, *saturation, *value);
        let selector_color = Color32::from_rgb((r * 255.0) as u8, (g * 255.0) as u8, (b * 255.0) as u8);
        draw_selector_circle(ui, circle_pos, selector_color);
    }

    changed
}

fn hue_bar(ui: &mut egui::Ui, hue: &mut f32, height: f32) -> bool {
    let (rect, response) = ui.allocate_exact_size(Vec2::new(HUE_BAR_WIDTH, height), Sense::click_and_drag());
    let mut changed = false;
    let corner_radius = CornerRadius::same(CORNER_RADIUS as u8);

    if ui.input(|i| i.pointer.any_down()) && response.contains_pointer() {
        if let Some(pos) = ui.input(|i| i.pointer.interact_pos()) {
            let local_y = pos.y - rect.min.y;
            *hue = (local_y / height).clamp(0.0, 1.0) * 360.0;
            changed = true;
        }
    }

    if ui.is_rect_visible(rect) {
        let painter = ui.painter();
        let r = CORNER_RADIUS;

        // draw hue gradient
        let mut mesh = egui::Mesh::default();
        let steps = 36;
        for i in 0..steps {
            let y0 = rect.min.y + (i as f32 / steps as f32) * height;
            let y1 = rect.min.y + ((i + 1) as f32 / steps as f32) * height;

            let h0 = (i as f32 / steps as f32) * 360.0;
            let h1 = ((i + 1) as f32 / steps as f32) * 360.0;

            let c0 = hue_to_rgb(h0);
            let c1 = hue_to_rgb(h1);

            let idx = mesh.vertices.len() as u32;
            mesh.vertices.push(egui::epaint::Vertex { pos: Pos2::new(rect.min.x, y0), uv: egui::epaint::WHITE_UV, color: c0 });
            mesh.vertices.push(egui::epaint::Vertex { pos: Pos2::new(rect.max.x, y0), uv: egui::epaint::WHITE_UV, color: c0 });
            mesh.vertices.push(egui::epaint::Vertex { pos: Pos2::new(rect.min.x, y1), uv: egui::epaint::WHITE_UV, color: c1 });
            mesh.vertices.push(egui::epaint::Vertex { pos: Pos2::new(rect.max.x, y1), uv: egui::epaint::WHITE_UV, color: c1 });
            mesh.indices.extend_from_slice(&[idx, idx + 1, idx + 2, idx + 1, idx + 3, idx + 2]);
        }
        painter.add(egui::Shape::mesh(mesh));

        // draw corner covers with background color, then quarter circles with corner colors
        // mask size is corner radius + 1px offset to fully cover overflow
        let bg = colors::WINDOW_BG;
        let mask_r = r + 1.0;
        let top_color = hue_to_rgb(0.0);
        let bottom_color = hue_to_rgb(360.0);

        let corners = [
            (Pos2::new(rect.min.x + mask_r, rect.min.y + mask_r), top_color),     // top-left
            (Pos2::new(rect.max.x - mask_r, rect.min.y + mask_r), top_color),     // top-right
            (Pos2::new(rect.min.x + mask_r, rect.max.y - mask_r), bottom_color),  // bottom-left
            (Pos2::new(rect.max.x - mask_r, rect.max.y - mask_r), bottom_color),  // bottom-right
        ];

        let corner_rects = [
            Rect::from_min_size(rect.min, Vec2::splat(mask_r)),
            Rect::from_min_size(Pos2::new(rect.max.x - mask_r, rect.min.y), Vec2::splat(mask_r)),
            Rect::from_min_size(Pos2::new(rect.min.x, rect.max.y - mask_r), Vec2::splat(mask_r)),
            Rect::from_min_size(Pos2::new(rect.max.x - mask_r, rect.max.y - mask_r), Vec2::splat(mask_r)),
        ];

        for i in 0..4 {
            painter.rect_filled(corner_rects[i], CornerRadius::ZERO, bg);
            painter.circle_filled(corners[i].0, mask_r, corners[i].1);
        }

        // draw border
        painter.rect_stroke(rect, corner_radius, Stroke::new(1.0, colors::BORDER), egui::StrokeKind::Inside);

        // draw selector
        let selector_y = rect.min.y + (*hue / 360.0) * height;
        let selector_color = hue_to_rgb(*hue);
        draw_selector_rect_horizontal(ui, selector_y, rect, selector_color);
    }

    changed
}

fn channel_bar(
    ui: &mut egui::Ui,
    value: &mut f32,
    width: f32,
    gradient_fn: impl Fn(f32) -> Color32,
    with_checkerboard: bool,
) -> bool {
    let (rect, response) = ui.allocate_exact_size(Vec2::new(width, CHANNEL_BAR_HEIGHT), Sense::click_and_drag());
    let mut changed = false;
    let corner_radius = CornerRadius::same(CORNER_RADIUS as u8);

    if ui.input(|i| i.pointer.any_down()) && response.contains_pointer() {
        if let Some(pos) = ui.input(|i| i.pointer.interact_pos()) {
            let local_x = pos.x - rect.min.x;
            *value = (local_x / width).clamp(0.0, 1.0);
            changed = true;
        }
    }

    if ui.is_rect_visible(rect) {
        let r = CORNER_RADIUS;

        // draw checkerboard for alpha
        if with_checkerboard {
            draw_checkerboard(ui, rect);
        }

        // draw gradient
        let mut mesh = egui::Mesh::default();
        let steps = 32;
        for i in 0..steps {
            let x0 = rect.min.x + (i as f32 / steps as f32) * width;
            let x1 = rect.min.x + ((i + 1) as f32 / steps as f32) * width;

            let t0 = i as f32 / steps as f32;
            let t1 = (i + 1) as f32 / steps as f32;

            let c0 = gradient_fn(t0);
            let c1 = gradient_fn(t1);

            let idx = mesh.vertices.len() as u32;
            mesh.vertices.push(egui::epaint::Vertex { pos: Pos2::new(x0, rect.min.y), uv: egui::epaint::WHITE_UV, color: c0 });
            mesh.vertices.push(egui::epaint::Vertex { pos: Pos2::new(x1, rect.min.y), uv: egui::epaint::WHITE_UV, color: c1 });
            mesh.vertices.push(egui::epaint::Vertex { pos: Pos2::new(x0, rect.max.y), uv: egui::epaint::WHITE_UV, color: c0 });
            mesh.vertices.push(egui::epaint::Vertex { pos: Pos2::new(x1, rect.max.y), uv: egui::epaint::WHITE_UV, color: c1 });
            mesh.indices.extend_from_slice(&[idx, idx + 1, idx + 2, idx + 1, idx + 3, idx + 2]);
        }
        ui.painter().add(egui::Shape::mesh(mesh));

        // draw corner covers with background color, then quarter circles with corner colors
        // mask size is corner radius + 1px offset to fully cover overflow
        let bg = colors::WINDOW_BG;
        let mask_r = r + 1.0;
        let left_color = gradient_fn(0.0);
        let right_color = gradient_fn(1.0);

        let corners = [
            (Pos2::new(rect.min.x + mask_r, rect.min.y + mask_r), left_color),   // top-left
            (Pos2::new(rect.max.x - mask_r, rect.min.y + mask_r), right_color),  // top-right
            (Pos2::new(rect.min.x + mask_r, rect.max.y - mask_r), left_color),   // bottom-left
            (Pos2::new(rect.max.x - mask_r, rect.max.y - mask_r), right_color),  // bottom-right
        ];

        let corner_rects = [
            Rect::from_min_size(rect.min, Vec2::splat(mask_r)),
            Rect::from_min_size(Pos2::new(rect.max.x - mask_r, rect.min.y), Vec2::splat(mask_r)),
            Rect::from_min_size(Pos2::new(rect.min.x, rect.max.y - mask_r), Vec2::splat(mask_r)),
            Rect::from_min_size(Pos2::new(rect.max.x - mask_r, rect.max.y - mask_r), Vec2::splat(mask_r)),
        ];

        for i in 0..4 {
            ui.painter().rect_filled(corner_rects[i], CornerRadius::ZERO, bg);
            ui.painter().circle_filled(corners[i].0, mask_r, corners[i].1);
        }

        // draw border
        ui.painter().rect_stroke(rect, corner_radius, Stroke::new(1.0, colors::BORDER), egui::StrokeKind::Inside);

        // draw selector
        let selector_x = rect.min.x + *value * width;
        let selector_color = gradient_fn(*value);
        draw_selector_rect_vertical(ui, selector_x, rect, selector_color);
    }

    changed
}

fn channel_value_input(ui: &mut egui::Ui, value: &mut f32) -> bool {
    let mut changed = false;
    let corner_radius = CornerRadius::same(CORNER_RADIUS as u8);
    let (rect, _) = ui.allocate_exact_size(Vec2::new(VALUE_INPUT_WIDTH, CHANNEL_BAR_HEIGHT), Sense::hover());

    // draw background
    ui.painter().rect_filled(rect, corner_radius, colors::INPUT_BG);

    let value_u8 = (*value * 255.0).round() as u8;
    let mut text = value_u8.to_string();
    let response = ui.put(
        rect,
        egui::TextEdit::singleline(&mut text)
            .horizontal_align(egui::Align::Center)
            .font(FontId::proportional(TEXT_BASE))
            .text_color(colors::TEXT_MUTED)
            .background_color(Color32::TRANSPARENT)
            .frame(false),
    );

    // draw border after TextEdit
    ui.painter().rect_stroke(rect, corner_radius, Stroke::new(1.0, colors::BORDER), egui::StrokeKind::Inside);

    if response.changed() {
        if let Ok(new_value) = text.parse::<u8>() {
            *value = new_value as f32 / 255.0;
            changed = true;
        }
    }

    changed
}

fn color_picker_popover_content(ui: &mut egui::Ui, rgba: &mut [f32; 4], initial_rgba: [f32; 4]) -> bool {
    let mut changed = false;
    ui.set_width(POPOVER_WIDTH);
    ui.spacing_mut().item_spacing = Vec2::splat(SPACING);

    // convert to HSV for editing
    let (mut hue, mut saturation, mut value) = rgb_to_hsv(rgba[0], rgba[1], rgba[2]);

    let square_size = POPOVER_WIDTH - HUE_BAR_WIDTH - SPACING;

    // HSV square and hue bar
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = SPACING;

        if hsv_square(ui, hue, &mut saturation, &mut value, square_size) {
            let (r, g, b) = hsv_to_rgb(hue, saturation, value);
            rgba[0] = r;
            rgba[1] = g;
            rgba[2] = b;
            changed = true;
        }

        if hue_bar(ui, &mut hue, square_size) {
            let (r, g, b) = hsv_to_rgb(hue, saturation, value);
            rgba[0] = r;
            rgba[1] = g;
            rgba[2] = b;
            changed = true;
        }
    });

    // previous/new color comparison
    let color_box_height = 24.0;
    let color_box_width = POPOVER_WIDTH / 2.0;
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 0.0;

        // previous color (left, rounded on left)
        let (prev_rect, _) = ui.allocate_exact_size(Vec2::new(color_box_width, color_box_height), Sense::hover());
        let prev_color = rgba_to_color32(initial_rgba);
        let left_radius = CornerRadius { nw: CORNER_RADIUS as u8, sw: CORNER_RADIUS as u8, ne: 0, se: 0 };
        draw_checkerboard(ui, prev_rect);
        ui.painter().rect_filled(prev_rect, left_radius, prev_color);

        // new color (right, rounded on right)
        let (new_rect, _) = ui.allocate_exact_size(Vec2::new(color_box_width, color_box_height), Sense::hover());
        let new_color = rgba_to_color32(*rgba);
        let right_radius = CornerRadius { nw: 0, sw: 0, ne: CORNER_RADIUS as u8, se: CORNER_RADIUS as u8 };
        draw_checkerboard(ui, new_rect);
        ui.painter().rect_filled(new_rect, right_radius, new_color);
    });

    let label_width = 12.0;
    let bar_width = POPOVER_WIDTH - label_width - SPACING - VALUE_INPUT_WIDTH - SPACING;

    // R channel
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = SPACING;
        ui.add_sized(Vec2::new(label_width, CHANNEL_BAR_HEIGHT), egui::Label::new(
            egui::RichText::new("R").size(TEXT_SM).color(colors::AXIS_X)
        ));
        let base_g = rgba[1];
        let base_b = rgba[2];
        let base_a = rgba[3];
        if channel_bar(ui, &mut rgba[0], bar_width, |t| {
            Color32::from_rgba_unmultiplied(
                (t * 255.0) as u8,
                (base_g * 255.0) as u8,
                (base_b * 255.0) as u8,
                (base_a * 255.0) as u8,
            )
        }, false) {
            changed = true;
        }
        if channel_value_input(ui, &mut rgba[0]) {
            changed = true;
        }
    });

    // G channel
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = SPACING;
        ui.add_sized(Vec2::new(label_width, CHANNEL_BAR_HEIGHT), egui::Label::new(
            egui::RichText::new("G").size(TEXT_SM).color(colors::AXIS_Y)
        ));
        let base_r = rgba[0];
        let base_b = rgba[2];
        let base_a = rgba[3];
        if channel_bar(ui, &mut rgba[1], bar_width, |t| {
            Color32::from_rgba_unmultiplied(
                (base_r * 255.0) as u8,
                (t * 255.0) as u8,
                (base_b * 255.0) as u8,
                (base_a * 255.0) as u8,
            )
        }, false) {
            changed = true;
        }
        if channel_value_input(ui, &mut rgba[1]) {
            changed = true;
        }
    });

    // B channel
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = SPACING;
        ui.add_sized(Vec2::new(label_width, CHANNEL_BAR_HEIGHT), egui::Label::new(
            egui::RichText::new("B").size(TEXT_SM).color(colors::AXIS_Z)
        ));
        let base_r = rgba[0];
        let base_g = rgba[1];
        let base_a = rgba[3];
        if channel_bar(ui, &mut rgba[2], bar_width, |t| {
            Color32::from_rgba_unmultiplied(
                (base_r * 255.0) as u8,
                (base_g * 255.0) as u8,
                (t * 255.0) as u8,
                (base_a * 255.0) as u8,
            )
        }, false) {
            changed = true;
        }
        if channel_value_input(ui, &mut rgba[2]) {
            changed = true;
        }
    });

    // A channel
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = SPACING;
        ui.add_sized(Vec2::new(label_width, CHANNEL_BAR_HEIGHT), egui::Label::new(
            egui::RichText::new("A").size(TEXT_SM).color(colors::TEXT_MUTED)
        ));
        let base_r = rgba[0];
        let base_g = rgba[1];
        let base_b = rgba[2];
        if channel_bar(ui, &mut rgba[3], bar_width, |t| {
            Color32::from_rgba_unmultiplied(
                (base_r * 255.0) as u8,
                (base_g * 255.0) as u8,
                (base_b * 255.0) as u8,
                (t * 255.0) as u8,
            )
        }, true) {
            changed = true;
        }
        if channel_value_input(ui, &mut rgba[3]) {
            changed = true;
        }
    });

    changed
}

fn draw_color_preview_button(ui: &mut egui::Ui, rect: Rect, color: Color32) {
    let corner_radius = CornerRadius::same(CORNER_RADIUS as u8);
    let mask_r = CORNER_RADIUS + 1.0;
    let bg = colors::PANEL_BG;

    ui.painter().rect_filled(rect, corner_radius, bg);
    draw_checkerboard(ui, rect);
    ui.painter().rect_filled(rect, corner_radius, color);

    // draw corner masks
    let corner_rects = [
        Rect::from_min_size(rect.min, Vec2::splat(mask_r)),
        Rect::from_min_size(Pos2::new(rect.max.x - mask_r, rect.min.y), Vec2::splat(mask_r)),
        Rect::from_min_size(Pos2::new(rect.min.x, rect.max.y - mask_r), Vec2::splat(mask_r)),
        Rect::from_min_size(Pos2::new(rect.max.x - mask_r, rect.max.y - mask_r), Vec2::splat(mask_r)),
    ];

    let corner_centers = [
        Pos2::new(rect.min.x + mask_r, rect.min.y + mask_r),
        Pos2::new(rect.max.x - mask_r, rect.min.y + mask_r),
        Pos2::new(rect.min.x + mask_r, rect.max.y - mask_r),
        Pos2::new(rect.max.x - mask_r, rect.max.y - mask_r),
    ];

    for i in 0..4 {
        ui.painter().rect_filled(corner_rects[i], CornerRadius::ZERO, bg);
        ui.painter().circle_filled(corner_centers[i], mask_r, color);
    }

    ui.painter().rect_stroke(
        rect,
        corner_radius,
        Stroke::new(1.0, colors::BORDER),
        egui::StrokeKind::Inside,
    );
}

pub fn color_picker(ui: &mut egui::Ui, rgba: &mut [f32; 4], width: f32, panel_right_edge: Option<f32>) -> Response {
    let color = rgba_to_color32(*rgba);

    let button_height = 24.0;
    let (button_rect, mut button_response) = ui.allocate_exact_size(Vec2::new(width, button_height), Sense::click());

    if ui.is_rect_visible(button_rect) {
        draw_color_preview_button(ui, button_rect, color);
    }

    let popup_id = ui.make_persistent_id("color_picker_popup");
    let initial_color_id = popup_id.with("initial_color");
    let mut is_open = ui.data(|d| d.get_temp::<bool>(popup_id).unwrap_or(false));

    if button_response.clicked() {
        is_open = !is_open;
        ui.data_mut(|d| d.insert_temp(popup_id, is_open));
        if is_open {
            ui.data_mut(|d| d.insert_temp(initial_color_id, *rgba));
        }
    }

    let initial_rgba = ui.data(|d| d.get_temp::<[f32; 4]>(initial_color_id).unwrap_or(*rgba));

    let mut changed = false;

    if is_open {
        // position popup to the right of the panel if specified, otherwise below the button
        let popup_pos = if let Some(panel_right) = panel_right_edge {
            Pos2::new(panel_right + SPACING, button_rect.min.y)
        } else {
            button_rect.left_bottom() + Vec2::new(0.0, 4.0)
        };

        let area_response = egui::Area::new(popup_id)
            .order(egui::Order::Foreground)
            .fixed_pos(popup_pos)
            .show(ui.ctx(), |ui| {
                egui::Frame::popup(ui.style()).show(ui, |ui| {
                    if color_picker_popover_content(ui, rgba, initial_rgba) {
                        changed = true;
                    }

                    // confirm button
                    ui.add_space(SPACING);
                    if ui.add_sized(
                        Vec2::new(POPOVER_WIDTH, 24.0),
                        egui::Button::new(format!("{} Confirm", icons::CHECK_FILL))
                    ).clicked() {
                        ui.data_mut(|d| d.insert_temp(popup_id, false));
                    }
                });
            });

        // close on click outside (check for new press, not release)
        let dominated = ui.input(|i| i.pointer.any_pressed());
        if dominated {
            if let Some(pos) = ui.input(|i| i.pointer.press_origin()) {
                let popup_rect = area_response.response.rect;
                if !popup_rect.contains(pos) && !button_rect.contains(pos) {
                    ui.data_mut(|d| d.insert_temp(popup_id, false));
                }
            }
        }
    }

    if changed {
        button_response.mark_changed();
    }

    button_response
}

/// A gradient picker widget that shows a gradient bar with editable stops.
/// Handles all popup interactions internally.
pub fn gradient_picker(
    ui: &mut egui::Ui,
    gradient: &mut Gradient,
    width: f32,
    panel_right_edge: Option<f32>,
) -> Response {
    let base_id = ui.make_persistent_id("gradient_picker");
    let stop_popup_id = base_id.with("stop_popup");

    let tooltip_height = GRADIENT_STOP_SIZE + GRADIENT_STOP_PADDING * 2.0 + GRADIENT_STOP_ARROW_SIZE;
    let total_height = (GRADIENT_BAR_HEIGHT / 2.) + tooltip_height;

    // note: gradient_picker_internal allocates the space, so we just need to track the rect
    let start_pos = ui.cursor().min;

    let mut changed = false;

    let result = gradient_picker_internal(ui, gradient, width, base_id);

    let total_rect = Rect::from_min_size(start_pos, Vec2::new(width, total_height));
    if result.changed {
        changed = true;
    }

    // handle opening color picker for a stop
    if let Some(stop_idx) = result.stop_to_open {
        let is_stop_open = ui.data(|d| d.get_temp::<Option<usize>>(stop_popup_id).unwrap_or(None));
        if is_stop_open == Some(stop_idx) {
            ui.data_mut(|d| d.insert_temp::<Option<usize>>(stop_popup_id, None));
        } else {
            ui.data_mut(|d| d.insert_temp(stop_popup_id, Some(stop_idx)));
            let initial_color = gradient.stops[stop_idx].color;
            ui.data_mut(|d| d.insert_temp(stop_popup_id.with("initial"), initial_color));
        }
    }

    // show stop color picker popup if open
    let open_stop_idx: Option<usize> = ui.data(|d| d.get_temp(stop_popup_id).unwrap_or(None));
    let mut stop_to_remove: Option<usize> = None;

    if let Some(stop_idx) = open_stop_idx {
        if stop_idx < gradient.stops.len() {
            let popup_pos = if let Some(panel_right) = panel_right_edge {
                Pos2::new(panel_right + SPACING, total_rect.min.y)
            } else {
                let inner_left = total_rect.min.x + GRADIENT_BAR_PADDING;
                let inner_width = width - GRADIENT_BAR_PADDING * 2.0;
                let stop_x = inner_left + gradient.stops[stop_idx].position * inner_width;
                Pos2::new(stop_x - POPOVER_WIDTH / 2.0, total_rect.max.y + SPACING)
            };

            let initial_color = ui.data(|d| {
                d.get_temp::<[f32; 4]>(stop_popup_id.with("initial"))
                    .unwrap_or(gradient.stops[stop_idx].color)
            });
            let can_remove = gradient.stops.len() > 1;

            let area_response = egui::Area::new(stop_popup_id.with("area"))
                .order(egui::Order::Foreground)
                .fixed_pos(popup_pos)
                .show(ui.ctx(), |ui| {
                    egui::Frame::popup(ui.style()).show(ui, |ui| {
                        if color_picker_popover_content(
                            ui,
                            &mut gradient.stops[stop_idx].color,
                            initial_color,
                        ) {
                            changed = true;
                        }

                        ui.add_space(SPACING);
                        let button_height = 24.0;
                        let confirm_width = POPOVER_WIDTH - button_height - SPACING;

                        let (row_rect, _) = ui.allocate_exact_size(
                            Vec2::new(POPOVER_WIDTH, button_height),
                            Sense::hover(),
                        );

                        let row_min_x = row_rect.min.x.round();
                        let row_min_y = row_rect.min.y.round();

                        // remove button
                        let remove_rect = Rect::from_min_size(
                            Pos2::new(row_min_x, row_min_y),
                            Vec2::splat(button_height),
                        );
                        let remove_response =
                            ui.interact(remove_rect, stop_popup_id.with("remove_btn"), Sense::click());

                        if ui.is_rect_visible(remove_rect) {
                            let visuals = if !can_remove {
                                ui.style().visuals.widgets.inactive
                            } else if remove_response.hovered() {
                                ui.style().visuals.widgets.hovered
                            } else {
                                ui.style().visuals.widgets.inactive
                            };
                            ui.painter()
                                .rect_filled(remove_rect, visuals.corner_radius, visuals.bg_fill);
                            ui.painter().rect_stroke(
                                remove_rect,
                                visuals.corner_radius,
                                visuals.bg_stroke,
                                egui::StrokeKind::Inside,
                            );
                            let text_color = if can_remove {
                                visuals.text_color()
                            } else {
                                ui.style().visuals.widgets.noninteractive.text_color()
                            };
                            ui.painter().text(
                                remove_rect.center(),
                                egui::Align2::CENTER_CENTER,
                                icons::SUBTRACT_FILL,
                                FontId::proportional(TEXT_BASE),
                                text_color,
                            );
                        }

                        if can_remove && remove_response.clicked() {
                            stop_to_remove = Some(stop_idx);
                            ui.data_mut(|d| d.insert_temp::<Option<usize>>(stop_popup_id, None));
                        }

                        // confirm button
                        let confirm_rect = Rect::from_min_size(
                            Pos2::new(row_min_x + button_height + SPACING, row_min_y),
                            Vec2::new(confirm_width, button_height),
                        );
                        let confirm_response =
                            ui.interact(confirm_rect, stop_popup_id.with("confirm_btn"), Sense::click());

                        if ui.is_rect_visible(confirm_rect) {
                            let visuals = if confirm_response.hovered() {
                                ui.style().visuals.widgets.hovered
                            } else {
                                ui.style().visuals.widgets.inactive
                            };
                            ui.painter()
                                .rect_filled(confirm_rect, visuals.corner_radius, visuals.bg_fill);
                            ui.painter().rect_stroke(
                                confirm_rect,
                                visuals.corner_radius,
                                visuals.bg_stroke,
                                egui::StrokeKind::Inside,
                            );
                            ui.painter().text(
                                confirm_rect.center(),
                                egui::Align2::CENTER_CENTER,
                                format!("{} Confirm", icons::CHECK_FILL),
                                FontId::proportional(TEXT_BASE),
                                visuals.text_color(),
                            );
                        }

                        if confirm_response.clicked() {
                            ui.data_mut(|d| d.insert_temp::<Option<usize>>(stop_popup_id, None));
                        }
                    });
                });

            // close on click outside
            let dominated = ui.input(|i| i.pointer.any_pressed());
            if dominated {
                if let Some(pos) = ui.input(|i| i.pointer.press_origin()) {
                    let popup_rect = area_response.response.rect;
                    if !popup_rect.contains(pos) {
                        ui.data_mut(|d| d.insert_temp::<Option<usize>>(stop_popup_id, None));
                    }
                }
            }
        }
    }

    if let Some(idx) = stop_to_remove {
        gradient.stops.remove(idx);
        changed = true;
    }

    let mut response = ui.interact(total_rect, base_id.with("response"), Sense::hover());
    if changed {
        response.mark_changed();
    }

    response
}

fn lerp_color(a: [f32; 4], b: [f32; 4], t: f32) -> [f32; 4] {
    [
        a[0] + (b[0] - a[0]) * t,
        a[1] + (b[1] - a[1]) * t,
        a[2] + (b[2] - a[2]) * t,
        a[3] + (b[3] - a[3]) * t,
    ]
}

fn sample_gradient_at(gradient: &Gradient, position: f32) -> [f32; 4] {
    if gradient.stops.is_empty() {
        return [1.0, 1.0, 1.0, 1.0];
    }

    let mut sorted_stops: Vec<&GradientStop> = gradient.stops.iter().collect();
    sorted_stops.sort_by(|a, b| a.position.partial_cmp(&b.position).unwrap());

    // find surrounding stops
    let mut left: Option<&GradientStop> = None;
    let mut right: Option<&GradientStop> = None;

    for stop in &sorted_stops {
        if stop.position <= position {
            left = Some(stop);
        }
        if stop.position >= position && right.is_none() {
            right = Some(stop);
        }
    }

    let left = left.unwrap_or(sorted_stops.first().unwrap());
    let right = right.unwrap_or(sorted_stops.last().unwrap());

    if left.position == right.position {
        return left.color;
    }

    let local_t = (position - left.position) / (right.position - left.position);

    match gradient.interpolation {
        GradientInterpolation::Steps => left.color,
        GradientInterpolation::Linear => lerp_color(left.color, right.color, local_t),
        GradientInterpolation::Smoothstep => {
            let smooth_t = local_t * local_t * (3.0 - 2.0 * local_t);
            lerp_color(left.color, right.color, smooth_t)
        }
    }
}

fn draw_gradient_bar(ui: &mut egui::Ui, rect: Rect, gradient: &Gradient) {
    let corner_radius = CornerRadius::same(CORNER_RADIUS as u8);
    let bg = colors::PANEL_BG;
    let mask_r = CORNER_RADIUS + 1.0;

    ui.painter().rect_filled(rect, corner_radius, bg);
    draw_checkerboard(ui, rect);

    if gradient.stops.is_empty() {
        return;
    }

    // build gradient mesh by sampling at regular intervals
    let mut mesh = egui::Mesh::default();
    let num_samples = 64;

    for i in 0..num_samples {
        let t0 = i as f32 / num_samples as f32;
        let t1 = (i + 1) as f32 / num_samples as f32;

        let x0 = rect.min.x + t0 * rect.width();
        let x1 = rect.min.x + t1 * rect.width();

        let c0 = rgba_to_color32(sample_gradient_at(gradient, t0));
        let c1 = rgba_to_color32(sample_gradient_at(gradient, t1));

        let idx = mesh.vertices.len() as u32;
        mesh.vertices.push(egui::epaint::Vertex { pos: Pos2::new(x0, rect.min.y), uv: egui::epaint::WHITE_UV, color: c0 });
        mesh.vertices.push(egui::epaint::Vertex { pos: Pos2::new(x1, rect.min.y), uv: egui::epaint::WHITE_UV, color: c1 });
        mesh.vertices.push(egui::epaint::Vertex { pos: Pos2::new(x0, rect.max.y), uv: egui::epaint::WHITE_UV, color: c0 });
        mesh.vertices.push(egui::epaint::Vertex { pos: Pos2::new(x1, rect.max.y), uv: egui::epaint::WHITE_UV, color: c1 });
        mesh.indices.extend_from_slice(&[idx, idx + 1, idx + 2, idx + 1, idx + 3, idx + 2]);
    }

    // draw corner covers
    let first_color = rgba_to_color32(sample_gradient_at(gradient, 0.0));
    let last_color = rgba_to_color32(sample_gradient_at(gradient, 1.0));

    let corners = [
        (Pos2::new(rect.min.x + mask_r, rect.min.y + mask_r), first_color),
        (Pos2::new(rect.max.x - mask_r, rect.min.y + mask_r), last_color),
        (Pos2::new(rect.min.x + mask_r, rect.max.y - mask_r), first_color),
        (Pos2::new(rect.max.x - mask_r, rect.max.y - mask_r), last_color),
    ];

    let corner_rects = [
        Rect::from_min_size(rect.min, Vec2::splat(mask_r)),
        Rect::from_min_size(Pos2::new(rect.max.x - mask_r, rect.min.y), Vec2::splat(mask_r)),
        Rect::from_min_size(Pos2::new(rect.min.x, rect.max.y - mask_r), Vec2::splat(mask_r)),
        Rect::from_min_size(Pos2::new(rect.max.x - mask_r, rect.max.y - mask_r), Vec2::splat(mask_r)),
    ];

    let painter = ui.painter();
    painter.add(egui::Shape::mesh(mesh));
    for i in 0..4 {
        painter.rect_filled(corner_rects[i], CornerRadius::ZERO, bg);
        painter.circle_filled(corners[i].0, mask_r, corners[i].1);
    }
    painter.rect_stroke(rect, corner_radius, Stroke::new(1.0, colors::BORDER), egui::StrokeKind::Inside);
}

fn draw_stop_tooltip(
    ui: &mut egui::Ui,
    center_x: f32,
    bar_center_y: f32,
    color: [f32; 4],
    stop_index: usize,
    base_id: egui::Id,
) -> Rect {
    let tooltip_width = GRADIENT_STOP_SIZE + GRADIENT_STOP_PADDING * 2.0;
    let body_height = GRADIENT_STOP_SIZE + GRADIENT_STOP_PADDING * 2.0;
    let tooltip_height = body_height + GRADIENT_STOP_ARROW_SIZE;

    // position tooltip so arrow tip touches bar center
    let arrow_tip_y = bar_center_y;
    let tooltip_rect = Rect::from_min_size(
        Pos2::new(center_x - tooltip_width / 2.0, arrow_tip_y),
        Vec2::new(tooltip_width, tooltip_height),
    );

    let corner_radius = CornerRadius::same(CORNER_RADIUS as u8);
    let color_corner_radius = CornerRadius::same(CORNER_RADIUS as u8);
    let bg = colors::PANEL_BG;

    // arrow at top pointing up
    let arrow_tip = Pos2::new(center_x, arrow_tip_y);
    let arrow_left = Pos2::new(center_x - GRADIENT_STOP_ARROW_SIZE, arrow_tip_y + GRADIENT_STOP_ARROW_SIZE);
    let arrow_right = Pos2::new(center_x + GRADIENT_STOP_ARROW_SIZE, arrow_tip_y + GRADIENT_STOP_ARROW_SIZE);

    // body rect below arrow
    let body_rect = Rect::from_min_size(
        Pos2::new(tooltip_rect.min.x, arrow_tip_y + GRADIENT_STOP_ARROW_SIZE),
        Vec2::new(tooltip_width, body_height),
    );

    // draw color preview square inside body
    let square_rect = Rect::from_center_size(
        body_rect.center(),
        Vec2::splat(GRADIENT_STOP_SIZE),
    );

    // full alpha color for border and left half
    let full_alpha_color = Color32::from_rgba_unmultiplied(
        (color[0] * 255.0) as u8,
        (color[1] * 255.0) as u8,
        (color[2] * 255.0) as u8,
        255,
    );
    let actual_color = rgba_to_color32(color);

    // split into left and right halves
    let right_half = Rect::from_min_max(
        Pos2::new(square_rect.center().x, square_rect.min.y),
        square_rect.max,
    );
    let left_half = Rect::from_min_max(
        square_rect.min,
        Pos2::new(square_rect.center().x, square_rect.max.y),
    );

    // draw checkerboard behind both halves for transparency
    draw_checkerboard(ui, square_rect);

    // use middle layer so tooltips render above the icon button but below popovers
    let layer_id = egui::LayerId::new(egui::Order::Middle, base_id.with(("tooltip_layer", stop_index)));
    let painter = ui.ctx().layer_painter(layer_id);

    // draw body with stroke first
    painter.rect_filled(body_rect, corner_radius, bg);
    painter.rect_stroke(body_rect, corner_radius, Stroke::new(1.0, colors::BORDER), egui::StrokeKind::Inside);

    // draw arrow on top to cover the body's top stroke where they connect
    painter.add(egui::Shape::convex_polygon(
        vec![arrow_left, arrow_tip, arrow_right],
        bg,
        Stroke::NONE,
    ));

    // draw arrow border (left and right edges only)
    painter.line_segment([arrow_left, arrow_tip], Stroke::new(1.0, colors::BORDER));
    painter.line_segment([arrow_tip, arrow_right], Stroke::new(1.0, colors::BORDER));

    // draw left half (full alpha) with left corner radius
    let left_corner_radius = CornerRadius { nw: color_corner_radius.nw, sw: color_corner_radius.sw, ne: 0, se: 0 };
    painter.rect_filled(left_half, left_corner_radius, full_alpha_color);

    // draw right half (actual alpha) with right corner radius
    let right_corner_radius = CornerRadius { nw: 0, sw: 0, ne: color_corner_radius.ne, se: color_corner_radius.se };
    painter.rect_filled(right_half, right_corner_radius, actual_color);

    // draw border around square with corner radius
    painter.rect_stroke(square_rect, color_corner_radius, Stroke::new(1.0, full_alpha_color), egui::StrokeKind::Inside);

    tooltip_rect
}

struct GradientPickerResult {
    changed: bool,
    stop_to_open: Option<usize>,
}

fn gradient_picker_internal(
    ui: &mut egui::Ui,
    gradient: &mut Gradient,
    width: f32,
    base_id: egui::Id,
) -> GradientPickerResult {
    let mut result = GradientPickerResult {
        changed: false,
        stop_to_open: None,
    };

    // calculate total height needed for bar + tooltip
    // tooltip arrow starts 4px up from bar bottom
    let tooltip_height = GRADIENT_STOP_SIZE + GRADIENT_STOP_PADDING * 2.0 + GRADIENT_STOP_ARROW_SIZE;
    let total_height = (GRADIENT_BAR_HEIGHT / 2.) + tooltip_height;

    let (total_rect, _) = ui.allocate_exact_size(Vec2::new(width, total_height), Sense::hover());

    // bar at top
    let bar_rect = Rect::from_min_size(
        total_rect.min,
        Vec2::new(width, GRADIENT_BAR_HEIGHT),
    );

    // draw gradient bar
    if ui.is_rect_visible(bar_rect) {
        draw_gradient_bar(ui, bar_rect, gradient);
    }

    // calculate the inner area for stop positioning (with padding)
    let inner_left = bar_rect.min.x + GRADIENT_BAR_PADDING;
    let inner_width = bar_rect.width() - GRADIENT_BAR_PADDING * 2.0;

    // helper to convert position (0.0-1.0) to screen x
    let position_to_x = |pos: f32| -> f32 {
        inner_left + pos * inner_width
    };

    // helper to convert screen x to position (0.0-1.0)
    let x_to_position = |x: f32| -> f32 {
        ((x - inner_left) / inner_width).clamp(0.0, 1.0)
    };

    // handle click on bar to add new stop
    let bar_response = ui.interact(bar_rect, base_id.with("bar"), Sense::click());
    if bar_response.clicked() {
        if let Some(pos) = ui.input(|i| i.pointer.interact_pos()) {
            let position = x_to_position(pos.x);

            // check if click is near an existing stop
            let near_stop = gradient.stops.iter().any(|stop| {
                (stop.position - position).abs() < 0.05
            });

            if !near_stop {
                // interpolate color at this position
                let color = sample_gradient_at(gradient, position);
                gradient.stops.push(GradientStop { color, position });
                result.changed = true;
            }
        }
    }

    // draw stop tooltips and handle interactions
    let dragging_stop_id = base_id.with("dragging_stop");
    let mut dragging_stop: Option<usize> = ui.data(|d| d.get_temp(dragging_stop_id));
    let mut stop_to_remove: Option<usize> = None;
    let can_remove_stop = gradient.stops.len() > 2;

    for (i, stop) in gradient.stops.iter_mut().enumerate() {
        let stop_x = position_to_x(stop.position);
        let tooltip_anchor_y = bar_rect.max.y - GRADIENT_BAR_HEIGHT / 2.;

        let tooltip_rect = draw_stop_tooltip(ui, stop_x, tooltip_anchor_y, stop.color, i, base_id);

        let stop_id = base_id.with(("stop", i));
        let stop_response = ui.interact(tooltip_rect, stop_id, Sense::click_and_drag());

        // handle click to open color picker
        if stop_response.clicked() {
            result.stop_to_open = Some(i);
        }

        // handle drag to reposition
        if stop_response.drag_started() {
            dragging_stop = Some(i);
            ui.data_mut(|d| d.insert_temp(dragging_stop_id, i));
        }

        if dragging_stop == Some(i) && stop_response.dragged() {
            if let Some(pos) = ui.input(|i| i.pointer.interact_pos()) {
                let new_position = x_to_position(pos.x);
                stop.position = new_position;
                result.changed = true;
            }
        }

        if stop_response.drag_stopped() {
            if dragging_stop == Some(i) {
                ui.data_mut(|d| d.remove::<usize>(dragging_stop_id));
                dragging_stop = None;
            }
        }

        // handle right-click to remove (if more than 2 stops)
        if stop_response.secondary_clicked() && can_remove_stop {
            stop_to_remove = Some(i);
        }
    }

    if let Some(idx) = stop_to_remove {
        gradient.stops.remove(idx);
        result.changed = true;
    }

    result
}
