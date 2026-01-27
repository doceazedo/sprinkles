use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts};
use aracari::prelude::*;
use egui_remixicon::icons;
use inflector::Inflector;

use crate::state::{EditorState, InspectorState};
use crate::ui::color_picker::{color_picker, color_picker_with_id, gradient_picker};
use crate::ui::curve_picker::spline_curve_config_picker;
use crate::ui::modals::ConfirmDeleteModal;
use crate::ui::styles::{
    colors, icon_button, styled_checkbox, styled_f32_input, styled_labeled_f32_input,
    styled_u32_input, ICON_BUTTON_SIZE, TEXT_BASE, TEXT_SM,
};
use crate::viewport::ViewportLayout;

const ROW_HEIGHT: f32 = 24.0;
const EMITTER_HEADER_HEIGHT: f32 = 28.0;
const PANEL_WIDTH: f32 = 500.0;
const ACRONYMS: &[&str] = &["fps"];

fn rand_seed() -> u32 {
    use std::time::{SystemTime, UNIX_EPOCH};
    let duration = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    (duration.as_nanos() & 0xFFFFFFFF) as u32
}

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

const MORE_BUTTON_SPACING: f32 = 4.0;

/// Renders a row for instantiable fields: Label | [ComboBox] [More button]
/// Returns the width available for content inside the combobox dropdown.
fn instantiable_row(
    ui: &mut egui::Ui,
    label: &str,
    indent_level: u8,
    selected_text: &str,
    add_combobox_contents: impl FnOnce(&mut egui::Ui) -> bool,
) -> bool {
    let mut changed = false;
    let available_width = ui.available_width();
    let indent_compensation = indent_level as f32 * INDENT_WIDTH;
    let label_width = (available_width - indent_compensation) / 2.0;
    let value_width = available_width - label_width;
    let combobox_width = value_width - ICON_BUTTON_SIZE - MORE_BUTTON_SPACING;

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
                ui.spacing_mut().item_spacing.x = MORE_BUTTON_SPACING;

                egui::ComboBox::from_id_salt(label)
                    .selected_text(selected_text)
                    .width(combobox_width)
                    .show_ui(ui, |ui| {
                        if add_combobox_contents(ui) {
                            changed = true;
                        }
                    });

                // more button (does nothing for now)
                icon_button(ui, icons::MORE_2_FILL);
            });

            ui.end_row();
        });

    changed
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
        changed = styled_f32_input(ui, value, width, ROW_HEIGHT, Some(0.0), None);
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
        changed = styled_f32_input(ui, value, width, ROW_HEIGHT, Some(min), Some(max));
    });
    changed
}

fn inspect_u32(ui: &mut egui::Ui, label: &str, value: &mut u32, indent_level: u8) -> bool {
    let mut changed = false;
    inspector_row(ui, label, indent_level, |ui, width| {
        changed = styled_u32_input(ui, value, width, ROW_HEIGHT);
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
    let label_padding = 6.0;
    let field_spacing = 4.0;

    inspector_row(ui, label, indent_level, |ui, width| {
        // calculate each label box width (text + padding)
        let label_box_widths: [f32; N] = std::array::from_fn(|i| {
            let galley = ui.painter().layout_no_wrap(labels[i].to_string(), font_id.clone(), egui::Color32::WHITE);
            galley.size().x + label_padding * 2.0
        });

        let total_label_box_width: f32 = label_box_widths.iter().sum();
        let total_field_spacing = (N - 1) as f32 * field_spacing;
        let total_input_width = width - total_label_box_width - total_field_spacing;
        let input_width = total_input_width / N as f32;

        ui.spacing_mut().item_spacing.x = 0.0;

        for i in 0..N {
            let color = DEFAULT_FIELD_COLORS.get(i).copied().unwrap_or(colors::TEXT_MUTED);

            if styled_labeled_f32_input(
                ui,
                labels[i],
                color,
                &mut values[i],
                input_width,
                ROW_HEIGHT,
                None,
                None,
            ) {
                changed = true;
            }

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

fn inspect_particle_mesh(
    ui: &mut egui::Ui,
    label: &str,
    value: &mut ParticleMesh,
    indent_level: u8,
) -> bool {
    let mut changed = false;

    let current_variant = match value {
        ParticleMesh::Quad { .. } => "Quad",
        ParticleMesh::Sphere { .. } => "Sphere",
        ParticleMesh::Cuboid { .. } => "Cuboid",
        ParticleMesh::Cylinder { .. } => "Cylinder",
    };

    changed |= instantiable_row(ui, label, indent_level, current_variant, |ui| {
        let mut inner_changed = false;
        if ui
            .selectable_label(matches!(value, ParticleMesh::Quad { .. }), "Quad")
            .clicked()
        {
            *value = ParticleMesh::Quad {
                orientation: QuadOrientation::default(),
            };
            inner_changed = true;
        }
        if ui
            .selectable_label(matches!(value, ParticleMesh::Sphere { .. }), "Sphere")
            .clicked()
        {
            *value = ParticleMesh::Sphere { radius: 1.0 };
            inner_changed = true;
        }
        if ui
            .selectable_label(matches!(value, ParticleMesh::Cuboid { .. }), "Cuboid")
            .clicked()
        {
            *value = ParticleMesh::Cuboid {
                half_size: Vec3::splat(0.5),
            };
            inner_changed = true;
        }
        if ui
            .selectable_label(matches!(value, ParticleMesh::Cylinder { .. }), "Cylinder")
            .clicked()
        {
            *value = ParticleMesh::Cylinder {
                top_radius: 0.5,
                bottom_radius: 0.5,
                height: 2.0,
                radial_segments: 64,
                rings: 4,
                cap_top: true,
                cap_bottom: true,
            };
            inner_changed = true;
        }
        inner_changed
    });

    let inner_indent = indent_level + 1;
    ui.spacing_mut().indent = INDENT_WIDTH;
    ui.indent(label, |ui| match value {
        ParticleMesh::Quad { orientation } => {
            changed |= inspect_quad_orientation(ui, "Orientation", orientation, inner_indent);
        }
        ParticleMesh::Sphere { radius } => {
            changed |= inspect_f32_positive(ui, &field_label("radius"), radius, inner_indent);
        }
        ParticleMesh::Cuboid { half_size } => {
            changed |= inspect_vec3(ui, &field_label("half_size"), half_size, inner_indent);
        }
        ParticleMesh::Cylinder {
            top_radius,
            bottom_radius,
            height,
            radial_segments,
            rings,
            cap_top,
            cap_bottom,
        } => {
            changed |= inspect_f32_positive(
                ui,
                &field_label("top_radius"),
                top_radius,
                inner_indent,
            );
            changed |= inspect_f32_positive(
                ui,
                &field_label("bottom_radius"),
                bottom_radius,
                inner_indent,
            );
            changed |= inspect_f32_positive(ui, &field_label("height"), height, inner_indent);
            changed |=
                inspect_u32(ui, &field_label("radial_segments"), radial_segments, inner_indent);
            changed |= inspect_u32(ui, &field_label("rings"), rings, inner_indent);
            changed |= inspect_bool(ui, &field_label("cap_top"), cap_top, inner_indent);
            changed |= inspect_bool(ui, &field_label("cap_bottom"), cap_bottom, inner_indent);
        }
    });

    changed
}

fn inspect_quad_orientation(
    ui: &mut egui::Ui,
    label: &str,
    value: &mut QuadOrientation,
    indent_level: u8,
) -> bool {
    let mut changed = false;

    let current_text = match value {
        QuadOrientation::FaceX => "Face X",
        QuadOrientation::FaceY => "Face Y",
        QuadOrientation::FaceZ => "Face Z",
    };

    inspector_row(ui, label, indent_level, |ui, width| {
        egui::ComboBox::from_id_salt(label)
            .selected_text(current_text)
            .width(width)
            .show_ui(ui, |ui| {
                if ui
                    .selectable_value(value, QuadOrientation::FaceX, "Face X")
                    .changed()
                {
                    changed = true;
                }
                if ui
                    .selectable_value(value, QuadOrientation::FaceY, "Face Y")
                    .changed()
                {
                    changed = true;
                }
                if ui
                    .selectable_value(value, QuadOrientation::FaceZ, "Face Z")
                    .changed()
                {
                    changed = true;
                }
            });
    });

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
        changed |= inspect_f32_positive(ui, &field_label("delay"), &mut time.delay, indent);
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
        let was_fixed_seed = time.use_fixed_seed;
        changed |= inspect_bool(ui, &field_label("use_fixed_seed"), &mut time.use_fixed_seed, indent);
        if time.use_fixed_seed && !was_fixed_seed && time.seed == 0 {
            time.seed = rand_seed();
            changed = true;
        }
        if time.use_fixed_seed {
            ui.spacing_mut().indent = INDENT_WIDTH;
            ui.indent("seed_indent", |ui| {
                changed |= inspect_u32(ui, &field_label("seed"), &mut time.seed, indent + 1);
            });
        }
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

fn inspect_alpha_mode(
    ui: &mut egui::Ui,
    label: &str,
    value: &mut SerializableAlphaMode,
    indent_level: u8,
) -> bool {
    let mut changed = false;

    let current_text = match value {
        SerializableAlphaMode::Opaque => "Opaque",
        SerializableAlphaMode::Mask { .. } => "Mask",
        SerializableAlphaMode::Blend => "Blend",
        SerializableAlphaMode::Premultiplied => "Premultiplied",
        SerializableAlphaMode::Add => "Add",
        SerializableAlphaMode::Multiply => "Multiply",
        SerializableAlphaMode::AlphaToCoverage => "Alpha to coverage",
    };

    inspector_row(ui, label, indent_level, |ui, width| {
        egui::ComboBox::from_id_salt(label)
            .selected_text(current_text)
            .width(width)
            .show_ui(ui, |ui| {
                if ui
                    .selectable_value(value, SerializableAlphaMode::Opaque, "Opaque")
                    .changed()
                {
                    changed = true;
                }
                if ui
                    .selectable_value(
                        value,
                        SerializableAlphaMode::Mask { cutoff: 0.5 },
                        "Mask",
                    )
                    .changed()
                {
                    changed = true;
                }
                if ui
                    .selectable_value(value, SerializableAlphaMode::Blend, "Blend")
                    .changed()
                {
                    changed = true;
                }
                if ui
                    .selectable_value(value, SerializableAlphaMode::Premultiplied, "Premultiplied")
                    .changed()
                {
                    changed = true;
                }
                if ui
                    .selectable_value(value, SerializableAlphaMode::Add, "Add")
                    .changed()
                {
                    changed = true;
                }
                if ui
                    .selectable_value(value, SerializableAlphaMode::Multiply, "Multiply")
                    .changed()
                {
                    changed = true;
                }
                if ui
                    .selectable_value(
                        value,
                        SerializableAlphaMode::AlphaToCoverage,
                        "Alpha to coverage",
                    )
                    .changed()
                {
                    changed = true;
                }
            });
    });

    if let SerializableAlphaMode::Mask { cutoff } = value {
        let inner_indent = indent_level + 1;
        ui.spacing_mut().indent = INDENT_WIDTH;
        ui.indent(label, |ui| {
            changed |= inspect_f32_clamped(ui, "Cutoff", cutoff, 0.0, 1.0, inner_indent);
        });
    }

    changed
}

fn inspect_standard_material(
    ui: &mut egui::Ui,
    id: &str,
    mat: &mut StandardParticleMaterial,
    indent_level: u8,
    panel_right_edge: Option<f32>,
) -> bool {
    let mut changed = false;

    inspector_row(ui, "Base color", indent_level, |ui, width| {
        if color_picker_with_id(
            ui,
            format!("{}_base_color", id),
            &mut mat.base_color,
            width,
            panel_right_edge,
        )
        .changed()
        {
            changed = true;
        }
    });

    inspector_row(ui, "Base color texture", indent_level, |ui, width| {
        let mut texture_path = mat.base_color_texture.clone().unwrap_or_default();
        let response = ui.add_sized(
            egui::vec2(width, ROW_HEIGHT),
            egui::TextEdit::singleline(&mut texture_path).hint_text(
                egui::RichText::new("Path to texture...").color(colors::placeholder_text()),
            ),
        );
        if response.changed() {
            mat.base_color_texture = if texture_path.is_empty() {
                None
            } else {
                Some(texture_path)
            };
            changed = true;
        }
    });

    inspector_row(ui, "Emissive", indent_level, |ui, width| {
        if color_picker_with_id(
            ui,
            format!("{}_emissive", id),
            &mut mat.emissive,
            width,
            panel_right_edge,
        )
        .changed()
        {
            changed = true;
        }
    });

    inspector_row(ui, "Emissive texture", indent_level, |ui, width| {
        let mut texture_path = mat.emissive_texture.clone().unwrap_or_default();
        let response = ui.add_sized(
            egui::vec2(width, ROW_HEIGHT),
            egui::TextEdit::singleline(&mut texture_path).hint_text(
                egui::RichText::new("Path to texture...").color(colors::placeholder_text()),
            ),
        );
        if response.changed() {
            mat.emissive_texture = if texture_path.is_empty() {
                None
            } else {
                Some(texture_path)
            };
            changed = true;
        }
    });

    const MIN_ROUGHNESS: f32 = 0.089;
    changed |= inspect_f32_clamped(
        ui,
        "Perceptual roughness",
        &mut mat.perceptual_roughness,
        MIN_ROUGHNESS,
        1.0,
        indent_level,
    );
    changed |= inspect_f32_clamped(ui, "Metallic", &mut mat.metallic, 0.0, 1.0, indent_level);

    inspector_row(ui, "Metallic roughness texture", indent_level, |ui, width| {
        let mut texture_path = mat.metallic_roughness_texture.clone().unwrap_or_default();
        let response = ui.add_sized(
            egui::vec2(width, ROW_HEIGHT),
            egui::TextEdit::singleline(&mut texture_path).hint_text(
                egui::RichText::new("Path to texture...").color(colors::placeholder_text()),
            ),
        );
        if response.changed() {
            mat.metallic_roughness_texture = if texture_path.is_empty() {
                None
            } else {
                Some(texture_path)
            };
            changed = true;
        }
    });

    inspector_row(ui, "Normal map texture", indent_level, |ui, width| {
        let mut texture_path = mat.normal_map_texture.clone().unwrap_or_default();
        let response = ui.add_sized(
            egui::vec2(width, ROW_HEIGHT),
            egui::TextEdit::singleline(&mut texture_path).hint_text(
                egui::RichText::new("Path to texture...").color(colors::placeholder_text()),
            ),
        );
        if response.changed() {
            mat.normal_map_texture = if texture_path.is_empty() {
                None
            } else {
                Some(texture_path)
            };
            changed = true;
        }
    });

    changed |= inspect_alpha_mode(ui, "Alpha mode", &mut mat.alpha_mode, indent_level);
    changed |= inspect_f32_clamped(
        ui,
        "Reflectance",
        &mut mat.reflectance,
        0.0,
        1.0,
        indent_level,
    );
    changed |= inspect_bool(ui, "Unlit", &mut mat.unlit, indent_level);
    changed |= inspect_bool(ui, "Double sided", &mut mat.double_sided, indent_level);
    changed |= inspect_bool(ui, "Fog enabled", &mut mat.fog_enabled, indent_level);

    changed
}

fn inspect_draw_pass_material(
    ui: &mut egui::Ui,
    id: &str,
    material: &mut DrawPassMaterial,
    indent_level: u8,
    panel_right_edge: Option<f32>,
) -> bool {
    let mut changed = false;

    let current_variant = match material {
        DrawPassMaterial::Standard(_) => "Standard",
        DrawPassMaterial::CustomShader { .. } => "Custom shader",
    };

    changed |= instantiable_row(ui, "Material", indent_level, current_variant, |ui| {
        let mut inner_changed = false;
        if ui
            .selectable_label(
                matches!(material, DrawPassMaterial::Standard(_)),
                "Standard",
            )
            .clicked()
        {
            *material = DrawPassMaterial::Standard(StandardParticleMaterial::default());
            inner_changed = true;
        }
        if ui
            .selectable_label(
                matches!(material, DrawPassMaterial::CustomShader { .. }),
                "Custom shader",
            )
            .clicked()
        {
            *material = DrawPassMaterial::CustomShader {
                vertex_shader: None,
                fragment_shader: None,
            };
            inner_changed = true;
        }
        inner_changed
    });

    let inner_indent = indent_level + 1;
    ui.spacing_mut().indent = INDENT_WIDTH;
    ui.indent(id, |ui| match material {
        DrawPassMaterial::Standard(mat) => {
            changed |= inspect_standard_material(
                ui,
                &format!("{}_standard", id),
                mat,
                inner_indent,
                panel_right_edge,
            );
        }
        DrawPassMaterial::CustomShader {
            vertex_shader,
            fragment_shader,
        } => {
            ui.label(
                egui::RichText::new("Custom shaders coming soon")
                    .color(colors::TEXT_MUTED)
                    .italics(),
            );

            ui.add_enabled_ui(false, |ui| {
                inspector_row(ui, "Vertex shader", inner_indent, |ui, width| {
                    let mut path = vertex_shader.clone().unwrap_or_default();
                    ui.add_sized(
                        egui::vec2(width, ROW_HEIGHT),
                        egui::TextEdit::singleline(&mut path).hint_text(
                            egui::RichText::new("Path to shader...")
                                .color(colors::placeholder_text()),
                        ),
                    );
                });
                inspector_row(ui, "Fragment shader", inner_indent, |ui, width| {
                    let mut path = fragment_shader.clone().unwrap_or_default();
                    ui.add_sized(
                        egui::vec2(width, ROW_HEIGHT),
                        egui::TextEdit::singleline(&mut path).hint_text(
                            egui::RichText::new("Path to shader...")
                                .color(colors::placeholder_text()),
                        ),
                    );
                });
            });
        }
    });

    changed
}

fn inspect_draw_passes(
    ui: &mut egui::Ui,
    id: &str,
    passes: &mut Vec<EmitterDrawPass>,
    indent_level: u8,
    panel_right_edge: Option<f32>,
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
                    actions.changed |=
                        inspect_particle_mesh(ui, &field_label("mesh"), &mut pass.mesh, indent + 1);
                    actions.changed |= inspect_draw_pass_material(
                        ui,
                        &format!("{}_material_{}", id, pass_idx),
                        &mut pass.material,
                        indent + 1,
                        panel_right_edge,
                    );
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

fn inspect_range(ui: &mut egui::Ui, label: &str, value: &mut ParticleRange, indent_level: u8) -> bool {
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

    changed |= instantiable_row(ui, label, indent_level, current_variant, |ui| {
        let mut inner_changed = false;
        if ui
            .selectable_label(matches!(value, EmissionShape::Point), "Point")
            .clicked()
        {
            *value = EmissionShape::Point;
            inner_changed = true;
        }
        if ui
            .selectable_label(matches!(value, EmissionShape::Sphere { .. }), "Sphere")
            .clicked()
        {
            *value = EmissionShape::Sphere { radius: 1.0 };
            inner_changed = true;
        }
        if ui
            .selectable_label(
                matches!(value, EmissionShape::SphereSurface { .. }),
                "Sphere surface",
            )
            .clicked()
        {
            *value = EmissionShape::SphereSurface { radius: 1.0 };
            inner_changed = true;
        }
        if ui
            .selectable_label(matches!(value, EmissionShape::Box { .. }), "Box")
            .clicked()
        {
            *value = EmissionShape::Box {
                extents: Vec3::ONE,
            };
            inner_changed = true;
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
            inner_changed = true;
        }
        inner_changed
    });

    let has_extra_settings = !matches!(value, EmissionShape::Point);
    if has_extra_settings {
        let inner_indent = indent_level + 1;
        ui.spacing_mut().indent = INDENT_WIDTH;
        ui.indent(label, |ui| match value {
            EmissionShape::Point => {}
            EmissionShape::Sphere { radius } | EmissionShape::SphereSurface { radius } => {
                changed |=
                    inspect_f32_positive(ui, &field_label("radius"), radius, inner_indent);
            }
            EmissionShape::Box { extents } => {
                changed |= inspect_vec3(ui, &field_label("extents"), extents, inner_indent);
            }
            EmissionShape::Ring {
                axis,
                height,
                radius,
                inner_radius,
            } => {
                changed |= inspect_vec3(ui, &field_label("axis"), axis, inner_indent);
                changed |= inspect_f32_positive(ui, &field_label("height"), height, inner_indent);
                changed |=
                    inspect_f32_positive(ui, &field_label("radius"), radius, inner_indent);
                changed |= inspect_f32_positive(
                    ui,
                    &field_label("inner_radius"),
                    inner_radius,
                    inner_indent,
                );
            }
        });
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

fn inspect_accelerations(
    ui: &mut egui::Ui,
    id: &str,
    accelerations: &mut ParticleProcessAccelerations,
    indent_level: u8,
) -> bool {
    let mut changed = false;
    inspector_category(ui, id, "Accelerations", indent_level, |ui, indent| {
        changed |= inspect_vec3(ui, &field_label("gravity"), &mut accelerations.gravity, indent);
    });
    changed
}

fn inspect_spline_curve(
    ui: &mut egui::Ui,
    label: &str,
    value: &mut Option<SplineCurveConfig>,
    indent_level: u8,
) -> bool {
    let mut changed = false;
    let available_width = ui.available_width();
    let indent_compensation = indent_level as f32 * INDENT_WIDTH;
    let label_width = (available_width - indent_compensation) / 2.0;
    let value_width = available_width - label_width;
    let combobox_width = value_width - ICON_BUTTON_SIZE - MORE_BUTTON_SPACING;

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
                ui.spacing_mut().item_spacing.x = MORE_BUTTON_SPACING;

                // the spline curve picker uses its own combobox with submenus,
                // so we pass the reduced width to it
                if spline_curve_config_picker(ui, label, value, combobox_width) {
                    changed = true;
                }

                // more button (does nothing for now)
                icon_button(ui, icons::MORE_2_FILL);
            });

            ui.end_row();
        });

    // show min/max fields when a curve is selected
    if let Some(config) = value {
        let inner_indent = indent_level + 1;
        ui.spacing_mut().indent = INDENT_WIDTH;
        ui.indent(format!("{}_range", label), |ui| {
            let mut values = [config.min_value, config.max_value];
            if inspect_vector_fields(ui, "Range", &mut values, &["min", "max"], inner_indent) {
                config.min_value = values[0];
                config.max_value = values[1];
                changed = true;
            }
        });
    }

    changed
}

fn inspect_solid_or_gradient_color(
    ui: &mut egui::Ui,
    id: &str,
    value: &mut SolidOrGradientColor,
    indent_level: u8,
    panel_right_edge: Option<f32>,
) -> bool {
    let mut changed = false;

    let current_variant = match value {
        SolidOrGradientColor::Solid { .. } => "Solid",
        SolidOrGradientColor::Gradient { .. } => "Gradient",
    };

    changed |= instantiable_row(ui, "Initial color", indent_level, current_variant, |ui| {
        let mut inner_changed = false;

        if ui
            .selectable_label(matches!(value, SolidOrGradientColor::Solid { .. }), "Solid")
            .clicked()
        {
            if !matches!(value, SolidOrGradientColor::Solid { .. }) {
                let color = match value {
                    SolidOrGradientColor::Gradient { gradient } => gradient
                        .stops
                        .first()
                        .map(|s| s.color)
                        .unwrap_or([1.0, 1.0, 1.0, 1.0]),
                    SolidOrGradientColor::Solid { color } => *color,
                };
                *value = SolidOrGradientColor::Solid { color };
                inner_changed = true;
            }
        }

        if ui
            .selectable_label(
                matches!(value, SolidOrGradientColor::Gradient { .. }),
                "Gradient",
            )
            .clicked()
        {
            if !matches!(value, SolidOrGradientColor::Gradient { .. }) {
                let color = match value {
                    SolidOrGradientColor::Solid { color } => *color,
                    SolidOrGradientColor::Gradient { gradient } => gradient
                        .stops
                        .first()
                        .map(|s| s.color)
                        .unwrap_or([1.0, 1.0, 1.0, 1.0]),
                };
                let inverted_color = [1.0 - color[0], 1.0 - color[1], 1.0 - color[2], color[3]];
                *value = SolidOrGradientColor::Gradient {
                    gradient: ParticleGradient {
                        stops: vec![
                            GradientStop {
                                color,
                                position: 0.0,
                            },
                            GradientStop {
                                color: inverted_color,
                                position: 1.0,
                            },
                        ],
                        interpolation: GradientInterpolation::Linear,
                    },
                };
                inner_changed = true;
            }
        }

        inner_changed
    });

    let inner_indent = indent_level + 1;
    ui.spacing_mut().indent = INDENT_WIDTH;
    ui.indent(id, |ui| match value {
        SolidOrGradientColor::Solid { color } => {
            inspector_row(ui, "Color", inner_indent, |ui, width| {
                if color_picker(ui, color, width, panel_right_edge).changed() {
                    changed = true;
                }
            });
        }
        SolidOrGradientColor::Gradient { gradient } => {
            inspector_row(ui, "Gradient", inner_indent, |ui, width| {
                if gradient_picker(ui, gradient, width, panel_right_edge).changed() {
                    changed = true;
                }
            });
            changed |= inspect_gradient_interpolation(
                ui,
                &field_label("interpolation"),
                &mut gradient.interpolation,
                inner_indent,
            );
        }
    });

    changed
}

fn inspect_gradient_interpolation(
    ui: &mut egui::Ui,
    label: &str,
    value: &mut GradientInterpolation,
    indent_level: u8,
) -> bool {
    let mut changed = false;

    let current_text = match value {
        GradientInterpolation::Steps => "Steps",
        GradientInterpolation::Linear => "Linear",
        GradientInterpolation::Smoothstep => "Smoothstep",
    };

    inspector_row(ui, label, indent_level, |ui, width| {
        egui::ComboBox::from_id_salt(label)
            .selected_text(current_text)
            .width(width)
            .show_ui(ui, |ui| {
                if ui
                    .selectable_value(value, GradientInterpolation::Steps, "Steps")
                    .changed()
                {
                    changed = true;
                }
                if ui
                    .selectable_value(value, GradientInterpolation::Linear, "Linear")
                    .changed()
                {
                    changed = true;
                }
                if ui
                    .selectable_value(value, GradientInterpolation::Smoothstep, "Smoothstep")
                    .changed()
                {
                    changed = true;
                }
            });
    });

    changed
}

fn inspect_display_color(
    ui: &mut egui::Ui,
    id: &str,
    color_curves: &mut ParticleProcessDisplayColor,
    indent_level: u8,
    panel_right_edge: Option<f32>,
) -> bool {
    let mut changed = false;
    inspector_category(ui, id, "Color curves", indent_level, |ui, indent| {
        changed |= inspect_solid_or_gradient_color(
            ui,
            &format!("{}_initial_color", id),
            &mut color_curves.initial_color,
            indent,
            panel_right_edge,
        );
        changed |= inspect_spline_curve(
            ui,
            &field_label("alpha over lifetime"),
            &mut color_curves.alpha_curve,
            indent,
        );
        changed |= inspect_spline_curve(
            ui,
            &field_label("emission over lifetime"),
            &mut color_curves.emission_curve,
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
        changed |=
            inspect_spline_curve(ui, &field_label("scale over lifetime"), &mut scale.curve, indent);
    });
    changed
}

fn inspect_process_display(
    ui: &mut egui::Ui,
    id: &str,
    display: &mut ParticleProcessDisplay,
    indent_level: u8,
    panel_right_edge: Option<f32>,
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
            panel_right_edge,
        );
    });
    changed
}

fn inspect_turbulence(
    ui: &mut egui::Ui,
    id: &str,
    turbulence: &mut Option<ParticleProcessTurbulence>,
    indent_level: u8,
) -> bool {
    let mut changed = false;

    inspector_category(ui, id, "Turbulence", indent_level, |ui, indent| {
        let mut enabled = turbulence.as_ref().is_some_and(|t| t.enabled);
        if inspect_bool(ui, "Enabled", &mut enabled, indent) {
            if enabled {
                if turbulence.is_none() {
                    *turbulence = Some(ParticleProcessTurbulence::default());
                }
                turbulence.as_mut().unwrap().enabled = true;
            } else if let Some(t) = turbulence {
                t.enabled = false;
            }
            changed = true;
        }

        if let Some(turb) = turbulence {
            if turb.enabled {
                changed |= inspect_f32_clamped(
                    ui,
                    &field_label("noise_strength"),
                    &mut turb.noise_strength,
                    0.0,
                    20.0,
                    indent,
                );
                changed |= inspect_f32_clamped(
                    ui,
                    &field_label("noise_scale"),
                    &mut turb.noise_scale,
                    0.0,
                    10.0,
                    indent,
                );
                changed |= inspect_vec3(ui, &field_label("noise_speed"), &mut turb.noise_speed, indent);
                changed |= inspect_f32_clamped(
                    ui,
                    &field_label("noise_speed_random"),
                    &mut turb.noise_speed_random,
                    0.0,
                    4.0,
                    indent,
                );
                changed |= inspect_range(ui, &field_label("influence"), &mut turb.influence, indent);
                changed |= inspect_spline_curve(
                    ui,
                    &field_label("influence over lifetime"),
                    &mut turb.influence_curve,
                    indent,
                );
            }
        }
    });

    changed
}

fn inspect_particle_flags(
    ui: &mut egui::Ui,
    id: &str,
    flags: &mut ParticleFlags,
    indent_level: u8,
) -> bool {
    let mut changed = false;

    inspector_category(ui, id, "Particle flags", indent_level, |ui, indent| {
        let mut align_y = flags.contains(ParticleFlags::ALIGN_Y_TO_VELOCITY);
        if inspect_bool(ui, "Align Y to velocity", &mut align_y, indent) {
            flags.set(ParticleFlags::ALIGN_Y_TO_VELOCITY, align_y);
            changed = true;
        }

        let mut disable_z = flags.contains(ParticleFlags::DISABLE_Z);
        if inspect_bool(ui, "Disable Z movement", &mut disable_z, indent) {
            flags.set(ParticleFlags::DISABLE_Z, disable_z);
            changed = true;
        }
    });

    changed
}

fn inspect_process_spawn(
    ui: &mut egui::Ui,
    id: &str,
    spawn: &mut ParticleProcessSpawn,
    indent_level: u8,
) -> bool {
    let mut changed = false;
    inspector_category(ui, id, "Spawn", indent_level, |ui, indent| {
        changed |= inspect_spawn_position(
            ui,
            &format!("{}_position", id),
            &mut spawn.position,
            indent,
        );
        changed |= inspect_spawn_velocity(
            ui,
            &format!("{}_velocity", id),
            &mut spawn.velocity,
            indent,
        );
    });
    changed
}

fn inspect_animated_velocity(
    ui: &mut egui::Ui,
    id: &str,
    label: &str,
    velocity: &mut AnimatedVelocity,
    indent_level: u8,
) -> bool {
    let mut changed = false;
    inspector_category(ui, id, label, indent_level, |ui, indent| {
        changed |= inspect_range(ui, &field_label("value"), &mut velocity.value, indent);
        changed |= inspect_spline_curve(
            ui,
            "Curve",
            &mut velocity.curve,
            indent,
        );
    });
    changed
}

fn inspect_animated_velocities(
    ui: &mut egui::Ui,
    id: &str,
    animated_velocity: &mut ParticleProcessAnimVelocities,
    indent_level: u8,
) -> bool {
    let mut changed = false;
    inspector_category(ui, id, "Animated velocities", indent_level, |ui, indent| {
        changed |= inspect_animated_velocity(
            ui,
            &format!("{}_radial", id),
            "Radial velocity",
            &mut animated_velocity.radial_velocity,
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
    panel_right_edge: Option<f32>,
) -> bool {
    let mut changed = false;
    inspector_category(ui, id, "Process", indent_level, |ui, indent| {
        changed |= inspect_particle_flags(
            ui,
            &format!("{}_flags", id),
            &mut config.particle_flags,
            indent,
        );
        changed |= inspect_process_spawn(
            ui,
            &format!("{}_spawn", id),
            &mut config.spawn,
            indent,
        );
        changed |= inspect_animated_velocities(
            ui,
            &format!("{}_animated_velocity", id),
            &mut config.animated_velocity,
            indent,
        );
        changed |= inspect_accelerations(
            ui,
            &format!("{}_accelerations", id),
            &mut config.accelerations,
            indent,
        );
        changed |= inspect_process_display(
            ui,
            &format!("{}_display", id),
            &mut config.display,
            indent,
            panel_right_edge,
        );
        changed |= inspect_turbulence(
            ui,
            &format!("{}_turbulence", id),
            &mut config.turbulence,
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

            // calculate panel right edge for color picker positioning
            let panel_rect = ui.max_rect();
            let panel_right_edge = Some(panel_rect.right());

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
                                any_changed |= inspect_vec3(ui, &field_label("position"), &mut emitter.position, base_indent);

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
                                    panel_right_edge,
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
                                    panel_right_edge,
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
                editor_state.should_reset = true;
                editor_state.is_playing = true;
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
    editor_state.should_reset = true;
    editor_state.is_playing = true;
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
        editor_state.should_reset = true;
        editor_state.is_playing = true;
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
        editor_state.should_reset = true;
        editor_state.is_playing = true;
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
            editor_state.should_reset = true;
            editor_state.is_playing = true;
        }
    }
}
