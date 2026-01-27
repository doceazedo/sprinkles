use bevy::color::palettes::tailwind;
use bevy_egui::egui::{
    self, Color32, CornerRadius, FontId, Margin, RichText, Stroke, StrokeKind, Style, TextStyle,
    Vec2,
};

pub const BUTTON_HEIGHT: f32 = 24.0;
pub const BUTTON_PADDING: f32 = 12.0;
pub const ICON_BUTTON_SIZE: f32 = 24.0;
pub const MODAL_FOOTER_PADDING: i8 = 8;

pub const TEXT_SM: f32 = 12.0;
pub const TEXT_BASE: f32 = 14.0;
pub const TEXT_LG: f32 = 18.0;

pub mod colors {
    use super::*;

    pub const GREEN: Color32 = bevy_to_egui(tailwind::GREEN_500);
    pub const BLUE: Color32 = bevy_to_egui(tailwind::BLUE_500);

    pub const RED_400: Color32 = bevy_to_egui(tailwind::RED_400);
    pub const RED_800: Color32 = bevy_to_egui(tailwind::RED_800);

    pub const AXIS_X: Color32 = bevy_to_egui(tailwind::RED_400);
    pub const AXIS_X_BG: Color32 = bevy_to_egui(tailwind::RED_950);
    pub const AXIS_Y: Color32 = bevy_to_egui(tailwind::GREEN_400);
    pub const AXIS_Y_BG: Color32 = bevy_to_egui(tailwind::GREEN_950);
    pub const AXIS_Z: Color32 = bevy_to_egui(tailwind::BLUE_400);
    pub const AXIS_Z_BG: Color32 = bevy_to_egui(tailwind::BLUE_950);

    pub const ZINC_950: Color32 = bevy_to_egui(tailwind::ZINC_950);
    pub const ZINC_900: Color32 = bevy_to_egui(tailwind::ZINC_900);
    pub const ZINC_800: Color32 = bevy_to_egui(tailwind::ZINC_800);
    pub const ZINC_700: Color32 = bevy_to_egui(tailwind::ZINC_700);
    pub const ZINC_600: Color32 = bevy_to_egui(tailwind::ZINC_600);
    pub const ZINC_500: Color32 = bevy_to_egui(tailwind::ZINC_500);
    pub const ZINC_400: Color32 = bevy_to_egui(tailwind::ZINC_400);
    pub const ZINC_300: Color32 = bevy_to_egui(tailwind::ZINC_300);
    pub const ZINC_200: Color32 = bevy_to_egui(tailwind::ZINC_200);
    pub const ZINC_50: Color32 = bevy_to_egui(tailwind::ZINC_50);

    pub const TOPBAR_BG: Color32 = ZINC_800;
    pub const PANEL_BG: Color32 = ZINC_900;
    pub const WINDOW_BG: Color32 = ZINC_900;
    pub const MODAL_TITLE_BG: Color32 = ZINC_800;
    pub const INPUT_BG: Color32 = ZINC_800;
    pub const BORDER: Color32 = ZINC_700;
    pub const TEXT_MUTED: Color32 = ZINC_300;
    pub const TEXT_LIGHT: Color32 = ZINC_50;

    pub const EMITTER_HEADER_BG: Color32 = ZINC_700;
    pub const CATEGORY_HEADER_BG: Color32 = ZINC_800;

    const fn bevy_to_egui(color: bevy::color::Srgba) -> Color32 {
        Color32::from_rgb(
            (color.red * 255.0) as u8,
            (color.green * 255.0) as u8,
            (color.blue * 255.0) as u8,
        )
    }

    pub fn green_hover() -> Color32 {
        Color32::from_rgba_unmultiplied(34, 197, 94, 80)
    }

    pub fn blue_semi() -> Color32 {
        Color32::from_rgba_unmultiplied(59, 130, 246, 80)
    }

    pub fn blue_hover() -> Color32 {
        Color32::from_rgba_unmultiplied(59, 130, 246, 120)
    }

    pub fn hover_bg() -> Color32 {
        BORDER
    }

    pub fn active_bg() -> Color32 {
        Color32::from_white_alpha(25)
    }

    pub fn placeholder_text() -> Color32 {
        Color32::from_white_alpha(255 / 2)
    }
}

pub fn configure_style(ctx: &egui::Context) {
    let mut style = Style::default();

    style.text_styles = [
        (TextStyle::Small, FontId::proportional(TEXT_SM)),
        (TextStyle::Body, FontId::proportional(TEXT_BASE)),
        (TextStyle::Monospace, FontId::monospace(TEXT_BASE)),
        (TextStyle::Button, FontId::proportional(TEXT_BASE)),
        (TextStyle::Heading, FontId::proportional(TEXT_BASE)),
    ]
    .into();

    style.spacing.button_padding = Vec2::new(BUTTON_PADDING, (BUTTON_HEIGHT - TEXT_BASE) / 2.0);
    style.spacing.interact_size.y = BUTTON_HEIGHT;
    style.spacing.interact_size.x = 200.0;

    style.visuals.override_text_color = Some(colors::TEXT_MUTED);

    let widget_corner_radius = CornerRadius::same(2);

    style.visuals.widgets.inactive.bg_fill = Color32::TRANSPARENT;
    style.visuals.widgets.inactive.bg_stroke = Stroke::NONE;
    style.visuals.widgets.inactive.weak_bg_fill = Color32::TRANSPARENT;
    style.visuals.widgets.inactive.fg_stroke = Stroke::new(1.0, colors::TEXT_MUTED);
    style.visuals.widgets.inactive.corner_radius = widget_corner_radius;

    style.visuals.widgets.hovered.bg_fill = colors::hover_bg();
    style.visuals.widgets.hovered.bg_stroke = Stroke::NONE;
    style.visuals.widgets.hovered.weak_bg_fill = colors::hover_bg();
    style.visuals.widgets.hovered.fg_stroke = Stroke::new(1.0, colors::TEXT_MUTED);
    style.visuals.widgets.hovered.corner_radius = widget_corner_radius;

    style.visuals.widgets.active.bg_fill = colors::active_bg();
    style.visuals.widgets.active.bg_stroke = Stroke::NONE;
    style.visuals.widgets.active.weak_bg_fill = colors::active_bg();
    style.visuals.widgets.active.fg_stroke = Stroke::new(1.0, colors::TEXT_MUTED);
    style.visuals.widgets.active.corner_radius = widget_corner_radius;

    style.visuals.widgets.inactive.bg_fill = colors::INPUT_BG;
    style.visuals.widgets.inactive.bg_stroke = Stroke::new(1.0, colors::BORDER);
    style.visuals.widgets.noninteractive.bg_stroke = Stroke::new(1.0, colors::BORDER);
    style.visuals.widgets.noninteractive.fg_stroke = Stroke::new(1.0, colors::placeholder_text());
    style.visuals.widgets.noninteractive.corner_radius = widget_corner_radius;
    style.visuals.widgets.open.corner_radius = widget_corner_radius;
    style.visuals.extreme_bg_color = colors::INPUT_BG;

    style.visuals.panel_fill = colors::PANEL_BG;
    style.visuals.window_fill = colors::WINDOW_BG;

    style.visuals.window_corner_radius = CornerRadius::same(8);
    style.visuals.window_stroke = Stroke::new(1.0, colors::BORDER);

    ctx.set_style(style);
}

const ICON_BUTTON_CORNER_RADIUS: u8 = 4;
const ICON_VERTICAL_OFFSET: f32 = 1.0;

#[derive(Default)]
pub struct IconButtonOptions {
    pub color: Option<Color32>,
    pub active: Option<bool>,
    pub active_color: Option<Color32>,
    pub active_bg: Option<Color32>,
    pub hover_bg: Option<Color32>,
}

pub fn icon_button_ex(ui: &mut egui::Ui, icon: &str, options: IconButtonOptions) -> egui::Response {
    let (rect, response) =
        ui.allocate_exact_size(Vec2::splat(ICON_BUTTON_SIZE), egui::Sense::click());

    if ui.is_rect_visible(rect) {
        let is_active = options.active.unwrap_or(false);
        let default_hover_bg = colors::hover_bg();

        let bg_color = if is_active {
            if response.hovered() {
                options.hover_bg.unwrap_or(default_hover_bg)
            } else {
                options.active_bg.unwrap_or(Color32::TRANSPARENT)
            }
        } else if response.hovered() {
            options.hover_bg.unwrap_or(default_hover_bg)
        } else {
            Color32::TRANSPARENT
        };

        ui.painter()
            .rect_filled(rect, CornerRadius::same(ICON_BUTTON_CORNER_RADIUS), bg_color);

        let text_color = if is_active {
            options.active_color.unwrap_or_else(|| ui.visuals().text_color())
        } else {
            options.color.unwrap_or_else(|| ui.visuals().text_color())
        };

        let icon_pos = rect.center() + Vec2::new(0.0, ICON_VERTICAL_OFFSET);
        ui.painter().text(
            icon_pos,
            egui::Align2::CENTER_CENTER,
            icon,
            FontId::proportional(TEXT_BASE),
            text_color,
        );
    }

    response
}

pub fn icon_button(ui: &mut egui::Ui, icon: &str) -> egui::Response {
    icon_button_ex(ui, icon, IconButtonOptions::default())
}

pub fn icon_button_colored(
    ui: &mut egui::Ui,
    icon: &str,
    color: Color32,
    hover_color: Color32,
) -> egui::Response {
    icon_button_ex(ui, icon, IconButtonOptions {
        color: Some(color),
        hover_bg: Some(hover_color),
        ..Default::default()
    })
}

pub fn icon_toggle(
    ui: &mut egui::Ui,
    icon: &str,
    active: bool,
    active_color: Color32,
    active_bg: Color32,
    hover_bg: Color32,
) -> egui::Response {
    icon_button_ex(ui, icon, IconButtonOptions {
        active: Some(active),
        active_color: Some(active_color),
        active_bg: Some(active_bg),
        hover_bg: Some(hover_bg),
        ..Default::default()
    })
}

pub fn primary_button(ui: &mut egui::Ui, text: &str) -> egui::Response {
    let button = egui::Button::new(RichText::new(text).color(colors::TEXT_LIGHT))
        .fill(colors::BLUE)
        .stroke(Stroke::NONE);
    ui.add(button)
}

pub fn danger_button(ui: &mut egui::Ui, text: &str) -> egui::Response {
    let button = egui::Button::new(RichText::new(text).color(colors::TEXT_LIGHT))
        .fill(colors::RED_800)
        .stroke(Stroke::NONE);
    ui.add(button)
}

pub fn ghost_button_with_icon(
    ui: &mut egui::Ui,
    text: &str,
    icon: &str,
) -> egui::Response {
    let text_galley = ui.painter().layout_no_wrap(
        text.to_string(),
        FontId::proportional(TEXT_BASE),
        ui.visuals().text_color(),
    );
    let icon_galley = ui.painter().layout_no_wrap(
        icon.to_string(),
        FontId::proportional(TEXT_BASE),
        colors::ZINC_400,
    );

    let spacing = 8.0;
    let total_width = text_galley.size().x + spacing + icon_galley.size().x + BUTTON_PADDING * 2.0;
    let desired_size = Vec2::new(total_width, BUTTON_HEIGHT);

    let (rect, response) = ui.allocate_exact_size(desired_size, egui::Sense::click());

    if ui.is_rect_visible(rect) {
        let bg_color = if response.hovered() {
            colors::hover_bg()
        } else {
            Color32::TRANSPARENT
        };

        ui.painter()
            .rect_filled(rect, CornerRadius::same(4), bg_color);

        let text_pos = egui::pos2(rect.left() + BUTTON_PADDING, rect.center().y - text_galley.size().y / 2.0);
        ui.painter().galley(text_pos, text_galley, Color32::WHITE);

        let icon_pos = egui::pos2(
            rect.right() - BUTTON_PADDING - icon_galley.size().x,
            rect.center().y - icon_galley.size().y / 2.0,
        );
        ui.painter().galley(icon_pos, icon_galley, Color32::WHITE);
    }

    response
}

pub fn close_button(ui: &mut egui::Ui, icon: &str) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(Vec2::new(24.0, 24.0), egui::Sense::click());

    if ui.is_rect_visible(rect) {
        let bg_color = if response.hovered() {
            colors::hover_bg()
        } else {
            Color32::TRANSPARENT
        };

        ui.painter()
            .rect_filled(rect, CornerRadius::same(4), bg_color);

        ui.painter().text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            icon,
            FontId::proportional(TEXT_BASE),
            ui.visuals().text_color(),
        );
    }

    response
}

pub fn styled_radio(ui: &mut egui::Ui, selected: bool, text: &str) -> egui::Response {
    let text_width = text.len() as f32 * 10.0;
    let desired_size = Vec2::new(16.0 + 12.0 + text_width + 8.0, BUTTON_HEIGHT);
    let (rect, response) = ui.allocate_exact_size(desired_size, egui::Sense::click());

    if ui.is_rect_visible(rect) {
        let radio_size = 16.0;
        let radio_rect = egui::Rect::from_center_size(
            egui::pos2(rect.left() + radio_size / 2.0 + 4.0, rect.center().y),
            Vec2::splat(radio_size),
        );

        let (bg_color, stroke_color) = if selected {
            (colors::blue_semi(), colors::BLUE)
        } else {
            (Color32::TRANSPARENT, colors::ZINC_500)
        };

        ui.painter()
            .rect_filled(radio_rect, CornerRadius::same(8), bg_color);
        ui.painter().rect_stroke(
            radio_rect,
            CornerRadius::same(8),
            Stroke::new(1.0, stroke_color),
            StrokeKind::Inside,
        );

        if selected {
            let inner_rect = radio_rect.shrink(4.0);
            ui.painter()
                .rect_filled(inner_rect, CornerRadius::same(4), colors::BLUE);
        }

        let text_pos = egui::pos2(rect.left() + radio_size + 12.0, rect.center().y);
        ui.painter().text(
            text_pos,
            egui::Align2::LEFT_CENTER,
            text,
            FontId::proportional(TEXT_BASE),
            ui.visuals().text_color(),
        );
    }

    response
}

pub fn styled_checkbox(ui: &mut egui::Ui, checked: &mut bool) -> egui::Response {
    let checkbox_size = 18.0;
    let desired_size = Vec2::splat(checkbox_size);
    let (rect, response) = ui.allocate_exact_size(desired_size, egui::Sense::click());

    if response.clicked() {
        *checked = !*checked;
    }

    if ui.is_rect_visible(rect) {
        let (bg_color, stroke_color) = if *checked {
            (colors::blue_semi(), colors::BLUE)
        } else {
            (Color32::TRANSPARENT, colors::ZINC_500)
        };

        ui.painter()
            .rect_filled(rect, CornerRadius::same(4), bg_color);
        ui.painter().rect_stroke(
            rect,
            CornerRadius::same(4),
            Stroke::new(1.0, stroke_color),
            StrokeKind::Inside,
        );

        if *checked {
            ui.painter().text(
                rect.center(),
                egui::Align2::CENTER_CENTER,
                egui_remixicon::icons::CHECK_LINE,
                FontId::proportional(TEXT_BASE),
                colors::BLUE,
            );
        }
    }

    response
}

pub fn topbar_frame() -> egui::Frame {
    egui::Frame::NONE
        .fill(colors::TOPBAR_BG)
        .inner_margin(Margin::same(8))
}

pub fn modal_frame() -> egui::Frame {
    egui::Frame::NONE
        .fill(colors::WINDOW_BG)
        .corner_radius(CornerRadius::same(8))
        .stroke(Stroke::new(1.0, colors::BORDER))
}

pub fn modal_title_frame() -> egui::Frame {
    egui::Frame::NONE
        .fill(colors::MODAL_TITLE_BG)
        .inner_margin(Margin::same(12))
        .corner_radius(CornerRadius {
            nw: 8,
            ne: 8,
            sw: 0,
            se: 0,
        })
}

pub fn draw_modal_backdrop(ui: &mut egui::Ui) {
    let screen_rect = ui.ctx().input(|i| i.viewport_rect());
    ui.painter().rect_filled(
        screen_rect,
        CornerRadius::ZERO,
        Color32::from_black_alpha(180),
    );
}

const INPUT_CORNER_RADIUS: u8 = 2;

pub fn styled_f32_input(
    ui: &mut egui::Ui,
    value: &mut f32,
    width: f32,
    height: f32,
    min: Option<f32>,
    max: Option<f32>,
) -> bool {
    styled_f32_input_with_rounding(
        ui,
        value,
        width,
        height,
        min,
        max,
        CornerRadius::same(INPUT_CORNER_RADIUS),
    )
}

fn styled_f32_input_with_rounding(
    ui: &mut egui::Ui,
    value: &mut f32,
    width: f32,
    height: f32,
    min: Option<f32>,
    max: Option<f32>,
    corner_radius: CornerRadius,
) -> bool {
    let mut changed = false;
    let (rect, _) = ui.allocate_exact_size(egui::vec2(width, height), egui::Sense::hover());

    // draw background
    ui.painter().rect_filled(rect, corner_radius, colors::INPUT_BG);

    let mut text = format!("{:.2}", value);
    let response = ui.put(
        rect,
        egui::TextEdit::singleline(&mut text)
            .horizontal_align(egui::Align::Center)
            .font(FontId::proportional(TEXT_BASE))
            .text_color(colors::TEXT_MUTED)
            .background_color(Color32::TRANSPARENT)
            .frame(false),
    );

    // draw border after TextEdit so it's on top
    ui.painter().rect_stroke(
        rect,
        corner_radius,
        Stroke::new(1.0, colors::BORDER),
        StrokeKind::Inside,
    );

    if response.changed() {
        if let Ok(mut new_value) = text.parse::<f32>() {
            if let Some(min_val) = min {
                new_value = new_value.max(min_val);
            }
            if let Some(max_val) = max {
                new_value = new_value.min(max_val);
            }
            *value = new_value;
            changed = true;
        }
    }
    changed
}

pub fn styled_labeled_f32_input(
    ui: &mut egui::Ui,
    label: &str,
    label_color: Color32,
    value: &mut f32,
    input_width: f32,
    height: f32,
    min: Option<f32>,
    max: Option<f32>,
) -> bool {
    let font_id = FontId::proportional(TEXT_SM);
    let label_padding = 6.0;

    // calculate label width
    let label_galley = ui.painter().layout_no_wrap(
        label.to_string(),
        font_id.clone(),
        label_color,
    );
    let label_box_width = label_galley.size().x + label_padding * 2.0;

    // allocate space for both label box and input
    let total_width = label_box_width + input_width;
    let (total_rect, _) = ui.allocate_exact_size(egui::vec2(total_width, height), egui::Sense::hover());

    let label_rect = egui::Rect::from_min_size(
        total_rect.min,
        egui::vec2(label_box_width, height),
    );
    let input_rect = egui::Rect::from_min_size(
        egui::pos2(label_rect.right(), total_rect.top()),
        egui::vec2(input_width, height),
    );

    let corner_radius = CornerRadius::same(INPUT_CORNER_RADIUS);

    // draw label box background (rounded left only)
    let label_corner_radius = CornerRadius {
        nw: INPUT_CORNER_RADIUS,
        ne: 0,
        sw: INPUT_CORNER_RADIUS,
        se: 0,
    };
    ui.painter().rect_filled(label_rect, label_corner_radius, colors::BORDER);

    // draw input background (rounded right only)
    let input_corner_radius = CornerRadius {
        nw: 0,
        ne: INPUT_CORNER_RADIUS,
        sw: 0,
        se: INPUT_CORNER_RADIUS,
    };
    ui.painter().rect_filled(input_rect, input_corner_radius, colors::INPUT_BG);

    // draw label text
    ui.painter().text(
        label_rect.center(),
        egui::Align2::CENTER_CENTER,
        label,
        font_id,
        label_color,
    );

    let mut text = format!("{:.2}", value);
    let response = ui.put(
        input_rect,
        egui::TextEdit::singleline(&mut text)
            .horizontal_align(egui::Align::Center)
            .font(FontId::proportional(TEXT_BASE))
            .text_color(colors::TEXT_MUTED)
            .background_color(Color32::TRANSPARENT)
            .frame(false),
    );

    // draw outer border for the entire combined element (after TextEdit so it's on top)
    ui.painter().rect_stroke(
        total_rect,
        corner_radius,
        Stroke::new(1.0, colors::BORDER),
        StrokeKind::Inside,
    );

    let mut changed = false;
    if response.changed() {
        if let Ok(mut new_value) = text.parse::<f32>() {
            if let Some(min_val) = min {
                new_value = new_value.max(min_val);
            }
            if let Some(max_val) = max {
                new_value = new_value.min(max_val);
            }
            *value = new_value;
            changed = true;
        }
    }
    changed
}

pub fn styled_u32_input(
    ui: &mut egui::Ui,
    value: &mut u32,
    width: f32,
    height: f32,
) -> bool {
    let mut changed = false;
    let (rect, _) = ui.allocate_exact_size(egui::vec2(width, height), egui::Sense::hover());

    // draw background
    let corner_radius = CornerRadius::same(INPUT_CORNER_RADIUS);
    ui.painter().rect_filled(rect, corner_radius, colors::INPUT_BG);

    let mut text = value.to_string();
    let response = ui.put(
        rect,
        egui::TextEdit::singleline(&mut text)
            .horizontal_align(egui::Align::Center)
            .font(FontId::proportional(TEXT_BASE))
            .text_color(colors::TEXT_MUTED)
            .background_color(Color32::TRANSPARENT)
            .frame(false),
    );

    // draw border after TextEdit so it's on top
    ui.painter().rect_stroke(
        rect,
        corner_radius,
        Stroke::new(1.0, colors::BORDER),
        StrokeKind::Inside,
    );

    if response.changed() {
        if let Ok(new_value) = text.parse::<u32>() {
            *value = new_value;
            changed = true;
        }
    }
    changed
}
