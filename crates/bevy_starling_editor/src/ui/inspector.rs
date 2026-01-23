use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts};
use bevy_starling::asset::{
    DrawOrder, EasingCurve, EmissionShape, EmitterData, EmitterDrawPass, EmitterDrawing,
    EmitterTime, ParticleMesh, ParticleProcessConfig, ParticleProcessDisplay,
    ParticleProcessDisplayColor, ParticleProcessDisplayScale, ParticleProcessSpawnAccelerations,
    ParticleProcessSpawnPosition, ParticleProcessSpawnVelocity, ParticleSystemAsset, Range,
};
use egui_remixicon::icons;
use inflector::Inflector;

use crate::state::{EditorState, InspectorState};
use crate::ui::modals::ConfirmDeleteModal;
use crate::ui::styles::{colors, icon_button, styled_checkbox, ICON_BUTTON_SIZE, TEXT_BASE, TEXT_SM};
use crate::viewport::ViewportLayout;

const ROW_HEIGHT: f32 = 24.0;
const EMITTER_HEADER_HEIGHT: f32 = 28.0;
const PANEL_WIDTH: f32 = 500.0;
const ACRONYMS: &[&str] = &["fps"];

#[derive(Event)]
pub struct AddEmitterEvent;

#[derive(Event)]
pub struct RemoveEmitterEvent {
    pub index: usize,
}

#[derive(Event)]
pub struct AddDrawPassEvent {
    pub emitter_index: usize,
}

#[derive(Event)]
pub struct RemoveDrawPassEvent {
    pub emitter_index: usize,
    pub pass_index: usize,
}

fn field_label(name: &str) -> String {
    let sentence = name.to_sentence_case();
    sentence
        .split_whitespace()
        .map(|word| {
            if ACRONYMS.contains(&word.to_lowercase().as_str()) {
                word.to_uppercase()
            } else {
                word.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

const INDENT_WIDTH: f32 = 12.0;

fn inspector_row(
    ui: &mut egui::Ui,
    label: &str,
    indent_level: u8,
    add_contents: impl FnOnce(&mut egui::Ui, f32),
) {
    let available_width = ui.available_width();
    let indent_compensation = indent_level as f32 * INDENT_WIDTH;
    let label_width = (available_width - indent_compensation) / 2.0;
    let value_width = available_width - label_width;

    egui::Grid::new(label)
        .num_columns(2)
        .spacing([0.0, 0.0])
        .min_col_width(label_width)
        .max_col_width(label_width)
        .show(ui, |ui| {
            ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                ui.set_height(ROW_HEIGHT);
                ui.label(egui::RichText::new(label).color(colors::TEXT_MUTED));
            });

            ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                ui.set_height(ROW_HEIGHT);
                ui.set_max_width(value_width);
                add_contents(ui, value_width);
            });

            ui.end_row();
        });
}

fn inspector_category(
    ui: &mut egui::Ui,
    id: &str,
    label: &str,
    indent_level: u8,
    add_contents: impl FnOnce(&mut egui::Ui, u8),
) {
    let header_id = ui.make_persistent_id(id);
    let mut state = egui::collapsing_header::CollapsingState::load_with_default_open(
        ui.ctx(),
        header_id,
        true,
    );

    let available_width = ui.available_width();

    let (rect, header_response) = ui.allocate_exact_size(
        egui::vec2(available_width, ROW_HEIGHT),
        egui::Sense::click(),
    );

    if header_response.clicked() {
        state.toggle(ui);
    }

    if ui.is_rect_visible(rect) {
        ui.painter().rect_filled(
            rect,
            egui::CornerRadius::same(4),
            colors::CATEGORY_HEADER_BG,
        );

        let chevron = if state.is_open() {
            icons::ARROW_DOWN_S_LINE
        } else {
            icons::ARROW_RIGHT_S_LINE
        };

        let chevron_pos = egui::pos2(rect.left() + 12.0, rect.center().y + 1.0);
        ui.painter().text(
            chevron_pos,
            egui::Align2::CENTER_CENTER,
            chevron,
            egui::FontId::proportional(TEXT_BASE),
            colors::TEXT_MUTED,
        );

        let text_pos = egui::pos2(rect.left() + 24.0, rect.center().y);
        ui.painter().text(
            text_pos,
            egui::Align2::LEFT_CENTER,
            label,
            egui::FontId::proportional(TEXT_BASE),
            colors::TEXT_MUTED,
        );
    }

    state.show_body_indented(&header_response, ui, |ui| {
        ui.add_space(4.0);
        add_contents(ui, indent_level + 1);
        ui.add_space(4.0);
    });

    state.store(ui.ctx());
}

struct EmitterHeaderResponse {
    toggled: bool,
    should_edit: bool,
    should_remove: bool,
}

fn emitter_collapsible_header(
    ui: &mut egui::Ui,
    name: &str,
    is_expanded: bool,
) -> EmitterHeaderResponse {
    let mut response = EmitterHeaderResponse {
        toggled: false,
        should_edit: false,
        should_remove: false,
    };

    let available_width = ui.available_width();
    let action_buttons_width = ICON_BUTTON_SIZE * 2.0 + 4.0;
    let main_button_width = available_width - action_buttons_width - 4.0;

    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 4.0;

        let (rect, main_response) = ui.allocate_exact_size(
            egui::vec2(main_button_width, EMITTER_HEADER_HEIGHT),
            egui::Sense::click(),
        );

        if main_response.clicked() {
            response.toggled = true;
        }

        if ui.is_rect_visible(rect) {
            ui.painter().rect_filled(
                rect,
                egui::CornerRadius::same(4),
                colors::EMITTER_HEADER_BG,
            );

            let icon_pos = egui::pos2(rect.left() + 16.0, rect.center().y + 1.0);
            ui.painter().text(
                icon_pos,
                egui::Align2::CENTER_CENTER,
                icons::SHOWERS_FILL,
                egui::FontId::proportional(TEXT_BASE),
                colors::TEXT_MUTED,
            );

            let text_pos = egui::pos2(rect.left() + 32.0, rect.center().y);
            ui.painter().text(
                text_pos,
                egui::Align2::LEFT_CENTER,
                name,
                egui::FontId::proportional(TEXT_BASE),
                colors::TEXT_LIGHT,
            );

            let chevron = if is_expanded {
                icons::ARROW_DOWN_S_LINE
            } else {
                icons::ARROW_RIGHT_S_LINE
            };
            let chevron_pos = egui::pos2(rect.right() - 16.0, rect.center().y + 1.0);
            ui.painter().text(
                chevron_pos,
                egui::Align2::CENTER_CENTER,
                chevron,
                egui::FontId::proportional(TEXT_BASE),
                colors::TEXT_MUTED,
            );
        }

        if icon_button(ui, icons::EDIT_2_FILL).clicked() {
            response.should_edit = true;
        }
        if icon_button(ui, icons::DELETE_BIN_2_FILL).clicked() {
            response.should_remove = true;
        }
    });

    response
}

fn inspect_f32_positive(ui: &mut egui::Ui, label: &str, value: &mut f32, indent_level: u8) -> bool {
    let mut changed = false;
    inspector_row(ui, label, indent_level, |ui, width| {
        let response = ui.add_sized(
            egui::vec2(width, ROW_HEIGHT),
            egui::DragValue::new(value).speed(0.01).range(0.0..=f32::MAX),
        );
        changed = response.changed();
    });
    changed
}

fn inspect_f32_clamped(
    ui: &mut egui::Ui,
    label: &str,
    value: &mut f32,
    min: f32,
    max: f32,
    indent_level: u8,
) -> bool {
    let mut changed = false;
    inspector_row(ui, label, indent_level, |ui, width| {
        let response = ui.add_sized(
            egui::vec2(width, ROW_HEIGHT),
            egui::DragValue::new(value).speed(0.01).range(min..=max),
        );
        changed = response.changed();
    });
    changed
}

fn inspect_u32(ui: &mut egui::Ui, label: &str, value: &mut u32, indent_level: u8) -> bool {
    let mut changed = false;
    inspector_row(ui, label, indent_level, |ui, width| {
        let response = ui.add_sized(
            egui::vec2(width, ROW_HEIGHT),
            egui::DragValue::new(value).speed(0.5).range(0..=u32::MAX),
        );
        changed = response.changed();
    });
    changed
}

fn inspect_bool(ui: &mut egui::Ui, label: &str, value: &mut bool, indent_level: u8) -> bool {
    let old_value = *value;
    inspector_row(ui, label, indent_level, |ui, _width| {
        styled_checkbox(ui, value);
    });
    *value != old_value
}

const DEFAULT_FIELD_COLORS: [egui::Color32; 3] = [colors::AXIS_X, colors::AXIS_Y, colors::AXIS_Z];

fn inspect_vector_fields<const N: usize>(
    ui: &mut egui::Ui,
    label: &str,
    values: &mut [f32; N],
    labels: &[&str; N],
    indent_level: u8,
) -> bool {
    let mut changed = false;

    let font_id = egui::FontId::proportional(TEXT_SM);
    let label_input_spacing = 4.0;
    let field_spacing = 8.0;

    inspector_row(ui, label, indent_level, |ui, width| {
        // calculate each label's width
        let label_widths: [f32; N] = std::array::from_fn(|i| {
            let galley = ui.painter().layout_no_wrap(labels[i].to_string(), font_id.clone(), egui::Color32::WHITE);
            galley.size().x
        });

        let total_label_width: f32 = label_widths.iter().sum();
        let total_label_input_spacing = N as f32 * label_input_spacing;
        let total_field_spacing = (N - 1) as f32 * field_spacing;
        let total_input_width = width - total_label_width - total_label_input_spacing - total_field_spacing;
        let input_width = total_input_width / N as f32;

        ui.spacing_mut().item_spacing.x = 0.0;

        for i in 0..N {
            let color = DEFAULT_FIELD_COLORS.get(i).copied().unwrap_or(colors::TEXT_MUTED);

            // paint label
            let (label_rect, _) = ui.allocate_exact_size(
                egui::vec2(label_widths[i], ROW_HEIGHT),
                egui::Sense::hover(),
            );
            ui.painter().text(
                label_rect.center(),
                egui::Align2::CENTER_CENTER,
                labels[i],
                font_id.clone(),
                color,
            );

            // spacing between label and input
            ui.add_space(label_input_spacing);

            // text input
            ui.scope(|ui| {
                ui.set_max_width(input_width);
                let (rect, _) = ui.allocate_exact_size(
                    egui::vec2(input_width, ROW_HEIGHT),
                    egui::Sense::hover(),
                );
                let mut text = format!("{:.2}", values[i]);
                let response = ui.put(
                    rect,
                    egui::TextEdit::singleline(&mut text)
                        .horizontal_align(egui::Align::Center),
                );
                if response.changed() {
                    if let Ok(new_value) = text.parse::<f32>() {
                        values[i] = new_value;
                        changed = true;
                    }
                }
            });

            // spacing between fields
            if i < N - 1 {
                ui.add_space(field_spacing);
            }
        }
    });

    changed
}

fn inspect_vec3(ui: &mut egui::Ui, label: &str, value: &mut Vec3, indent_level: u8) -> bool {
    let mut values = [value.x, value.y, value.z];
    let changed = inspect_vector_fields(ui, label, &mut values, &["X", "Y", "Z"], indent_level);
    if changed {
        value.x = values[0];
        value.y = values[1];
        value.z = values[2];
    }
    changed
}

fn inspect_draw_order(ui: &mut egui::Ui, label: &str, value: &mut DrawOrder, indent_level: u8) -> bool {
    let mut changed = false;
    inspector_row(ui, label, indent_level, |ui, width| {
        let current_text = match value {
            DrawOrder::Index => "Index",
            DrawOrder::Lifetime => "Lifetime",
            DrawOrder::ReverseLifetime => "ReverseLifetime",
            DrawOrder::ViewDepth => "ViewDepth",
        };

        egui::ComboBox::from_id_salt(label)
            .selected_text(current_text)
            .width(width)
            .show_ui(ui, |ui| {
                if ui
                    .selectable_value(value, DrawOrder::Index, "Index")
                    .changed()
                {
                    changed = true;
                }
                if ui
                    .selectable_value(value, DrawOrder::Lifetime, "Lifetime")
                    .changed()
                {
                    changed = true;
                }
                if ui
                    .selectable_value(value, DrawOrder::ReverseLifetime, "ReverseLifetime")
                    .changed()
                {
                    changed = true;
                }
                if ui
                    .selectable_value(value, DrawOrder::ViewDepth, "ViewDepth")
                    .changed()
                {
                    changed = true;
                }
            });
    });
    changed
}

fn inspect_particle_mesh(ui: &mut egui::Ui, label: &str, value: &mut ParticleMesh, indent_level: u8) -> bool {
    let mut changed = false;

    let current_variant = match value {
        ParticleMesh::Quad => "Quad",
        ParticleMesh::Sphere { .. } => "Sphere",
        ParticleMesh::Cuboid { .. } => "Cuboid",
    };

    inspector_row(ui, label, indent_level, |ui, width| {
        egui::ComboBox::from_id_salt(label)
            .selected_text(current_variant)
            .width(width)
            .show_ui(ui, |ui| {
                if ui
                    .selectable_label(matches!(value, ParticleMesh::Quad), "Quad")
                    .clicked()
                {
                    *value = ParticleMesh::Quad;
                    changed = true;
                }
                if ui
                    .selectable_label(matches!(value, ParticleMesh::Sphere { .. }), "Sphere")
                    .clicked()
                {
                    *value = ParticleMesh::Sphere { radius: 1.0 };
                    changed = true;
                }
                if ui
                    .selectable_label(matches!(value, ParticleMesh::Cuboid { .. }), "Cuboid")
                    .clicked()
                {
                    *value = ParticleMesh::Cuboid {
                        half_size: Vec3::splat(0.5),
                    };
                    changed = true;
                }
            });
    });

    match value {
        ParticleMesh::Quad => {}
        ParticleMesh::Sphere { radius } => {
            if inspect_f32_positive(ui, &field_label("radius"), radius, indent_level) {
                changed = true;
            }
        }
        ParticleMesh::Cuboid { half_size } => {
            if inspect_vec3(ui, &field_label("half_size"), half_size, indent_level) {
                changed = true;
            }
        }
    }

    changed
}

fn inspect_emitter_time(ui: &mut egui::Ui, id: &str, time: &mut EmitterTime, indent_level: u8) -> bool {
    let mut changed = false;
    inspector_category(ui, id, "Time", indent_level, |ui, indent| {
        changed |= inspect_f32_positive(ui, &field_label("lifetime"), &mut time.lifetime, indent);
        changed |= inspect_f32_clamped(
            ui,
            &field_label("lifetime_randomness"),
            &mut time.lifetime_randomness,
            0.0,
            1.0,
            indent,
        );
        changed |= inspect_bool(ui, &field_label("one_shot"), &mut time.one_shot, indent);
        changed |= inspect_f32_clamped(
            ui,
            &field_label("explosiveness"),
            &mut time.explosiveness,
            0.0,
            1.0,
            indent,
        );
        changed |= inspect_f32_clamped(
            ui,
            &field_label("randomness"),
            &mut time.randomness,
            0.0,
            1.0,
            indent,
        );
        changed |= inspect_u32(ui, &field_label("fixed_fps"), &mut time.fixed_fps, indent);
    });
    changed
}

fn inspect_emitter_drawing(ui: &mut egui::Ui, id: &str, drawing: &mut EmitterDrawing, indent_level: u8) -> bool {
    let mut changed = false;
    inspector_category(ui, id, "Drawing", indent_level, |ui, indent| {
        changed |= inspect_draw_order(ui, &field_label("draw_order"), &mut drawing.draw_order, indent);
    });
    changed
}

struct DrawPassesActions {
    add_pass: bool,
    remove_pass: Option<usize>,
    changed: bool,
}

fn inspect_draw_passes(
    ui: &mut egui::Ui,
    id: &str,
    passes: &mut Vec<EmitterDrawPass>,
    indent_level: u8,
) -> DrawPassesActions {
    let mut actions = DrawPassesActions {
        add_pass: false,
        remove_pass: None,
        changed: false,
    };

    inspector_category(ui, id, "Draw passes", indent_level, |ui, indent| {
        let can_remove = passes.len() > 1;

        for (pass_idx, pass) in passes.iter_mut().enumerate() {
            ui.push_id(pass_idx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(format!("Pass {}", pass_idx + 1));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if can_remove && icon_button(ui, icons::DELETE_BIN_LINE).clicked() {
                            actions.remove_pass = Some(pass_idx);
                        }
                    });
                });
                ui.indent(pass_idx, |ui| {
                    actions.changed |= inspect_particle_mesh(ui, &field_label("mesh"), &mut pass.mesh, indent + 1);
                });
            });
        }

        ui.add_space(4.0);
        if ui
            .add_sized(
                egui::vec2(ui.available_width(), 24.0),
                egui::Button::new(format!("{} Add pass", icons::ADD_LINE)),
            )
            .clicked()
        {
            actions.add_pass = true;
        }
    });

    actions
}

fn inspect_range(ui: &mut egui::Ui, label: &str, value: &mut Range, indent_level: u8) -> bool {
    let mut values = [value.min, value.max];
    let changed = inspect_vector_fields(ui, label, &mut values, &["min", "max"], indent_level);
    if changed {
        value.min = values[0];
        value.max = values[1];
    }
    changed
}

fn inspect_emission_shape(
    ui: &mut egui::Ui,
    label: &str,
    value: &mut EmissionShape,
    indent_level: u8,
) -> bool {
    let mut changed = false;

    let current_variant = match value {
        EmissionShape::Point => "Point",
        EmissionShape::Sphere { .. } => "Sphere",
        EmissionShape::SphereSurface { .. } => "Sphere surface",
        EmissionShape::Box { .. } => "Box",
        EmissionShape::Ring { .. } => "Ring",
    };

    inspector_row(ui, label, indent_level, |ui, width| {
        egui::ComboBox::from_id_salt(label)
            .selected_text(current_variant)
            .width(width)
            .show_ui(ui, |ui| {
                if ui
                    .selectable_label(matches!(value, EmissionShape::Point), "Point")
                    .clicked()
                {
                    *value = EmissionShape::Point;
                    changed = true;
                }
                if ui
                    .selectable_label(matches!(value, EmissionShape::Sphere { .. }), "Sphere")
                    .clicked()
                {
                    *value = EmissionShape::Sphere { radius: 1.0 };
                    changed = true;
                }
                if ui
                    .selectable_label(
                        matches!(value, EmissionShape::SphereSurface { .. }),
                        "Sphere surface",
                    )
                    .clicked()
                {
                    *value = EmissionShape::SphereSurface { radius: 1.0 };
                    changed = true;
                }
                if ui
                    .selectable_label(matches!(value, EmissionShape::Box { .. }), "Box")
                    .clicked()
                {
                    *value = EmissionShape::Box {
                        extents: Vec3::ONE,
                    };
                    changed = true;
                }
                if ui
                    .selectable_label(matches!(value, EmissionShape::Ring { .. }), "Ring")
                    .clicked()
                {
                    *value = EmissionShape::Ring {
                        axis: Vec3::Z,
                        height: 1.0,
                        radius: 2.0,
                        inner_radius: 0.0,
                    };
                    changed = true;
                }
            });
    });

    // show shape-specific fields
    match value {
        EmissionShape::Point => {}
        EmissionShape::Sphere { radius } | EmissionShape::SphereSurface { radius } => {
            if inspect_f32_positive(ui, &field_label("radius"), radius, indent_level) {
                changed = true;
            }
        }
        EmissionShape::Box { extents } => {
            if inspect_vec3(ui, &field_label("extents"), extents, indent_level) {
                changed = true;
            }
        }
        EmissionShape::Ring {
            axis,
            height,
            radius,
            inner_radius,
        } => {
            if inspect_vec3(ui, &field_label("axis"), axis, indent_level) {
                changed = true;
            }
            if inspect_f32_positive(ui, &field_label("height"), height, indent_level) {
                changed = true;
            }
            if inspect_f32_positive(ui, &field_label("radius"), radius, indent_level) {
                changed = true;
            }
            if inspect_f32_positive(ui, &field_label("inner_radius"), inner_radius, indent_level) {
                changed = true;
            }
        }
    }

    changed
}

fn inspect_spawn_position(
    ui: &mut egui::Ui,
    id: &str,
    position: &mut ParticleProcessSpawnPosition,
    indent_level: u8,
) -> bool {
    let mut changed = false;
    inspector_category(ui, id, "Position", indent_level, |ui, indent| {
        changed |= inspect_vec3(
            ui,
            &field_label("emission_shape_offset"),
            &mut position.emission_shape_offset,
            indent,
        );
        changed |= inspect_vec3(
            ui,
            &field_label("emission_shape_scale"),
            &mut position.emission_shape_scale,
            indent,
        );
        changed |= inspect_emission_shape(
            ui,
            &field_label("emission_shape"),
            &mut position.emission_shape,
            indent,
        );
    });
    changed
}

fn inspect_spawn_velocity(
    ui: &mut egui::Ui,
    id: &str,
    velocity: &mut ParticleProcessSpawnVelocity,
    indent_level: u8,
) -> bool {
    let mut changed = false;
    inspector_category(ui, id, "Velocity", indent_level, |ui, indent| {
        changed |= inspect_vec3(ui, &field_label("direction"), &mut velocity.direction, indent);
        changed |= inspect_f32_clamped(
            ui,
            &field_label("spread"),
            &mut velocity.spread,
            0.0,
            180.0,
            indent,
        );
        changed |= inspect_f32_clamped(
            ui,
            &field_label("flatness"),
            &mut velocity.flatness,
            0.0,
            1.0,
            indent,
        );
        changed |= inspect_range(
            ui,
            &field_label("initial_velocity"),
            &mut velocity.initial_velocity,
            indent,
        );
        changed |= inspect_f32_clamped(
            ui,
            &field_label("inherit_velocity_ratio"),
            &mut velocity.inherit_velocity_ratio,
            0.0,
            1.0,
            indent,
        );
        changed |= inspect_vec3(
            ui,
            &field_label("velocity_pivot"),
            &mut velocity.velocity_pivot,
            indent,
        );
    });
    changed
}

fn inspect_spawn_accelerations(
    ui: &mut egui::Ui,
    id: &str,
    accelerations: &mut ParticleProcessSpawnAccelerations,
    indent_level: u8,
) -> bool {
    let mut changed = false;
    inspector_category(ui, id, "Accelerations", indent_level, |ui, indent| {
        changed |= inspect_vec3(ui, &field_label("gravity"), &mut accelerations.gravity, indent);
    });
    changed
}

fn easing_curve_label(curve: Option<EasingCurve>) -> &'static str {
    // TODO: add more easing curve labels when implemented
    match curve {
        None => "Constant",
        Some(EasingCurve::LinearIn) => "Linear In",
        Some(EasingCurve::LinearOut) => "Linear Out",
    }
}

fn all_easing_options() -> Vec<(Option<EasingCurve>, &'static str)> {
    // TODO: add more easing curve options when implemented
    vec![
        (None, "Constant"),
        (Some(EasingCurve::LinearIn), "Linear In"),
        (Some(EasingCurve::LinearOut), "Linear Out"),
    ]
}

fn inspect_easing_curve(
    ui: &mut egui::Ui,
    label: &str,
    value: &mut Option<EasingCurve>,
    indent_level: u8,
) -> bool {
    let mut changed = false;
    inspector_row(ui, label, indent_level, |ui, width| {
        let current_text = easing_curve_label(*value);

        egui::ComboBox::from_id_salt(label)
            .selected_text(current_text)
            .width(width)
            .show_ui(ui, |ui| {
                for (option_value, option_label) in all_easing_options() {
                    if ui
                        .selectable_value(value, option_value, option_label)
                        .changed()
                    {
                        changed = true;
                    }
                }
            });
    });
    changed
}

fn inspect_color_rgba(
    ui: &mut egui::Ui,
    label: &str,
    value: &mut [f32; 4],
    indent_level: u8,
) -> bool {
    let mut changed = false;
    inspector_row(ui, label, indent_level, |ui, _width| {
        let mut color = egui::Color32::from_rgba_unmultiplied(
            (value[0] * 255.0) as u8,
            (value[1] * 255.0) as u8,
            (value[2] * 255.0) as u8,
            (value[3] * 255.0) as u8,
        );
        if ui.color_edit_button_srgba(&mut color).changed() {
            value[0] = color.r() as f32 / 255.0;
            value[1] = color.g() as f32 / 255.0;
            value[2] = color.b() as f32 / 255.0;
            value[3] = color.a() as f32 / 255.0;
            changed = true;
        }
    });
    changed
}

fn inspect_display_color(
    ui: &mut egui::Ui,
    id: &str,
    color_curves: &mut ParticleProcessDisplayColor,
    indent_level: u8,
) -> bool {
    let mut changed = false;
    inspector_category(ui, id, "Color curves", indent_level, |ui, indent| {
        changed |= inspect_color_rgba(
            ui,
            &field_label("initial_color"),
            &mut color_curves.initial_color,
            indent,
        );
    });
    changed
}

fn inspect_display_scale(
    ui: &mut egui::Ui,
    id: &str,
    scale: &mut ParticleProcessDisplayScale,
    indent_level: u8,
) -> bool {
    let mut changed = false;
    inspector_category(ui, id, "Scale", indent_level, |ui, indent| {
        changed |= inspect_range(ui, &field_label("initial range"), &mut scale.range, indent);
        changed |= inspect_easing_curve(ui, &field_label("scale over lifetime"), &mut scale.curve, indent);
    });
    changed
}

fn inspect_process_display(
    ui: &mut egui::Ui,
    id: &str,
    display: &mut ParticleProcessDisplay,
    indent_level: u8,
) -> bool {
    let mut changed = false;
    inspector_category(ui, id, "Display", indent_level, |ui, indent| {
        changed |= inspect_display_scale(
            ui,
            &format!("{}_scale", id),
            &mut display.scale,
            indent,
        );
        changed |= inspect_display_color(
            ui,
            &format!("{}_color", id),
            &mut display.color_curves,
            indent,
        );
    });
    changed
}

fn inspect_process_config(
    ui: &mut egui::Ui,
    id: &str,
    config: &mut ParticleProcessConfig,
    indent_level: u8,
) -> bool {
    let mut changed = false;
    inspector_category(ui, id, "Process", indent_level, |ui, indent| {
        changed |= inspect_spawn_position(
            ui,
            &format!("{}_position", id),
            &mut config.spawn.position,
            indent,
        );
        changed |= inspect_spawn_velocity(
            ui,
            &format!("{}_velocity", id),
            &mut config.spawn.velocity,
            indent,
        );
        changed |= inspect_spawn_accelerations(
            ui,
            &format!("{}_accelerations", id),
            &mut config.spawn.accelerations,
            indent,
        );
        changed |= inspect_process_display(
            ui,
            &format!("{}_display", id),
            &mut config.display,
            indent,
        );
    });
    changed
}


pub fn draw_inspector(
    mut contexts: EguiContexts,
    mut layout: ResMut<ViewportLayout>,
    mut editor_state: ResMut<EditorState>,
    mut inspector_state: ResMut<InspectorState>,
    mut confirm_delete_modal: ResMut<ConfirmDeleteModal>,
    mut assets: ResMut<Assets<ParticleSystemAsset>>,
    mut commands: Commands,
) -> Result {
    let ctx = contexts.ctx_mut()?;

    let response = egui::SidePanel::left("inspector")
        .resizable(true)
        .default_width(PANEL_WIDTH)
        .min_width(PANEL_WIDTH)
        .frame(
            egui::Frame::NONE
                .fill(colors::PANEL_BG)
                .inner_margin(egui::Margin::same(12)),
        )
        .show(ctx, |ui| {
            let Some(handle) = &editor_state.current_project else {
                ui.label("No project loaded");
                return;
            };

            let Some(asset) = assets.get_mut(handle.id()) else {
                ui.label("Loading...");
                return;
            };

            let mut should_add_emitter = false;
            let mut should_remove_emitter: Option<(usize, String)> = None;
            let mut should_add_pass: Option<usize> = None;
            let mut should_remove_pass: Option<(usize, usize)> = None;

            let mut toggle_emitter: Option<usize> = None;
            let mut start_editing_emitter: Option<usize> = None;
            let mut any_changed = false;

            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    ui.add_space(0.0);
                    let scroll_content_width = ui.available_width() - 12.0;
                    ui.set_width(scroll_content_width);

                    for (idx, emitter) in asset.emitters.iter_mut().enumerate() {
                        let emitter_id = format!("emitter_{}", idx);
                        let is_editing = inspector_state.editing_emitter_name == Some(idx);
                        let is_expanded = inspector_state.is_emitter_expanded(idx);

                        if is_editing {
                            ui.horizontal(|ui| {
                                let available_width = ui.available_width();
                                let response = ui.add_sized(
                                    egui::vec2(available_width, EMITTER_HEADER_HEIGHT),
                                    egui::TextEdit::singleline(&mut emitter.name),
                                );
                                if response.changed() {
                                    any_changed = true;
                                }
                                if response.lost_focus()
                                    || ui.input(|i| i.key_pressed(egui::Key::Enter))
                                {
                                    inspector_state.editing_emitter_name = None;
                                }
                                response.request_focus();
                            });
                        } else {
                            let header_response =
                                emitter_collapsible_header(ui, &emitter.name, is_expanded);

                            if header_response.toggled {
                                toggle_emitter = Some(idx);
                            }
                            if header_response.should_edit {
                                start_editing_emitter = Some(idx);
                            }
                            if header_response.should_remove {
                                should_remove_emitter = Some((idx, emitter.name.clone()));
                            }
                        }

                        if is_expanded {
                            ui.add_space(4.0);
                            let base_indent: u8 = 1;
                            ui.indent(&emitter_id, |ui| {
                                any_changed |= inspect_bool(ui, &field_label("enabled"), &mut emitter.enabled, base_indent);
                                any_changed |= inspect_u32(ui, &field_label("amount"), &mut emitter.amount, base_indent);

                                any_changed |= inspect_emitter_time(
                                    ui,
                                    &format!("{}_time", emitter_id),
                                    &mut emitter.time,
                                    base_indent,
                                );
                                any_changed |= inspect_emitter_drawing(
                                    ui,
                                    &format!("{}_drawing", emitter_id),
                                    &mut emitter.drawing,
                                    base_indent,
                                );

                                let pass_actions = inspect_draw_passes(
                                    ui,
                                    &format!("{}_passes", emitter_id),
                                    &mut emitter.draw_passes,
                                    base_indent,
                                );

                                any_changed |= pass_actions.changed;
                                if pass_actions.add_pass {
                                    should_add_pass = Some(idx);
                                }
                                if let Some(pass_idx) = pass_actions.remove_pass {
                                    should_remove_pass = Some((idx, pass_idx));
                                }

                                any_changed |= inspect_process_config(
                                    ui,
                                    &format!("{}_process", emitter_id),
                                    &mut emitter.process,
                                    base_indent,
                                );
                            });
                        }

                        ui.add_space(8.0);
                    }

                    ui.add_space(8.0);
                    if ui
                        .add_sized(
                            egui::vec2(ui.available_width(), 24.0),
                            egui::Button::new(format!("{} Add emitter", icons::ADD_LINE)),
                        )
                        .clicked()
                    {
                        should_add_emitter = true;
                    }
                });

            if any_changed {
                editor_state.mark_unsaved();
            }

            if let Some(idx) = toggle_emitter {
                inspector_state.toggle_emitter(idx);
            }
            if let Some(idx) = start_editing_emitter {
                inspector_state.editing_emitter_name = Some(idx);
            }

            if should_add_emitter {
                commands.trigger(AddEmitterEvent);
            }

            if let Some((idx, name)) = should_remove_emitter {
                confirm_delete_modal.open_for_emitter(idx, &name);
            }

            if let Some(emitter_idx) = should_add_pass {
                commands.trigger(AddDrawPassEvent {
                    emitter_index: emitter_idx,
                });
            }

            if let Some((emitter_idx, pass_idx)) = should_remove_pass {
                confirm_delete_modal.open_for_draw_pass(emitter_idx, pass_idx);
            }
        });

    layout.left_panel_width = response.response.rect.width();

    Ok(())
}

fn generate_unique_emitter_name(emitters: &[EmitterData]) -> String {
    let base_name = "Emitter";
    let existing_names: Vec<&str> = emitters.iter().map(|e| e.name.as_str()).collect();

    if !existing_names.contains(&base_name) {
        return base_name.to_string();
    }

    let mut counter = 2;
    loop {
        let candidate = format!("{} {}", base_name, counter);
        if !existing_names.contains(&candidate.as_str()) {
            return candidate;
        }
        counter += 1;
    }
}

pub fn on_add_emitter(
    _trigger: On<AddEmitterEvent>,
    mut editor_state: ResMut<EditorState>,
    mut assets: ResMut<Assets<ParticleSystemAsset>>,
) {
    let Some(handle) = &editor_state.current_project else {
        return;
    };

    let Some(asset) = assets.get_mut(handle.id()) else {
        return;
    };

    let name = generate_unique_emitter_name(&asset.emitters);
    let mut new_emitter = EmitterData::default();
    new_emitter.name = name;
    asset.emitters.push(new_emitter);
    editor_state.mark_unsaved();
}

pub fn on_remove_emitter(
    trigger: On<RemoveEmitterEvent>,
    mut editor_state: ResMut<EditorState>,
    mut inspector_state: ResMut<InspectorState>,
    mut assets: ResMut<Assets<ParticleSystemAsset>>,
) {
    let event = trigger.event();

    let Some(handle) = &editor_state.current_project else {
        return;
    };

    let Some(asset) = assets.get_mut(handle.id()) else {
        return;
    };

    if event.index < asset.emitters.len() {
        asset.emitters.remove(event.index);

        if inspector_state.editing_emitter_name == Some(event.index) {
            inspector_state.editing_emitter_name = None;
        }

        inspector_state.collapsed_emitters.remove(&event.index);

        let updated_collapsed: std::collections::HashSet<usize> = inspector_state
            .collapsed_emitters
            .iter()
            .map(|&idx| if idx > event.index { idx - 1 } else { idx })
            .collect();
        inspector_state.collapsed_emitters = updated_collapsed;

        editor_state.mark_unsaved();
    }
}

pub fn on_add_draw_pass(
    trigger: On<AddDrawPassEvent>,
    mut editor_state: ResMut<EditorState>,
    mut assets: ResMut<Assets<ParticleSystemAsset>>,
) {
    let event = trigger.event();

    let Some(handle) = &editor_state.current_project else {
        return;
    };

    let Some(asset) = assets.get_mut(handle.id()) else {
        return;
    };

    if let Some(emitter) = asset.emitters.get_mut(event.emitter_index) {
        emitter.draw_passes.push(EmitterDrawPass::default());
        editor_state.mark_unsaved();
    }
}

pub fn on_remove_draw_pass(
    trigger: On<RemoveDrawPassEvent>,
    mut editor_state: ResMut<EditorState>,
    mut assets: ResMut<Assets<ParticleSystemAsset>>,
) {
    let event = trigger.event();

    let Some(handle) = &editor_state.current_project else {
        return;
    };

    let Some(asset) = assets.get_mut(handle.id()) else {
        return;
    };

    if let Some(emitter) = asset.emitters.get_mut(event.emitter_index) {
        if event.pass_index < emitter.draw_passes.len() && emitter.draw_passes.len() > 1 {
            emitter.draw_passes.remove(event.pass_index);
            editor_state.mark_unsaved();
        }
    }
}
