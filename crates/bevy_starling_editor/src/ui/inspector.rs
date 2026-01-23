use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts};
use bevy_starling::asset::{
    DrawOrder, EmitterData, EmitterDrawPass, EmitterDrawing, EmitterTime, ParticleMesh,
    ParticleProcessConfig, ParticleSystemAsset,
};
use egui_remixicon::icons;
use inflector::Inflector;

use crate::state::{EditorState, InspectorState};
use crate::ui::styles::{colors, icon_button, styled_checkbox, ICON_BUTTON_SIZE};
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
            egui::FontId::proportional(16.0),
            colors::TEXT_MUTED,
        );

        let text_pos = egui::pos2(rect.left() + 24.0, rect.center().y);
        ui.painter().text(
            text_pos,
            egui::Align2::LEFT_CENTER,
            label,
            egui::FontId::proportional(14.0),
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
                egui::FontId::proportional(16.0),
                colors::TEXT_MUTED,
            );

            let text_pos = egui::pos2(rect.left() + 32.0, rect.center().y);
            ui.painter().text(
                text_pos,
                egui::Align2::LEFT_CENTER,
                name,
                egui::FontId::proportional(14.0),
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
                egui::FontId::proportional(16.0),
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

fn vec3_axis_input(
    ui: &mut egui::Ui,
    id: egui::Id,
    axis: &str,
    color: egui::Color32,
    value: &mut f32,
    width: f32,
) -> bool {
    let mut changed = false;
    let label_width = 16.0;
    let input_width = width - label_width;

    let (rect, _) = ui.allocate_exact_size(egui::vec2(width, ROW_HEIGHT), egui::Sense::hover());

    if ui.is_rect_visible(rect) {
        let label_rect = egui::Rect::from_min_size(rect.min, egui::vec2(label_width, ROW_HEIGHT));
        ui.painter().text(
            label_rect.center(),
            egui::Align2::CENTER_CENTER,
            axis,
            egui::FontId::proportional(12.0),
            color,
        );

        let input_rect = egui::Rect::from_min_size(
            egui::pos2(rect.min.x + label_width, rect.min.y),
            egui::vec2(input_width, ROW_HEIGHT),
        );

        let mut text = format!("{:.2}", value);
        let response = ui.put(
            input_rect,
            egui::TextEdit::singleline(&mut text)
                .id(id)
                .desired_width(input_width)
                .horizontal_align(egui::Align::Center),
        );

        if response.changed() {
            if let Ok(new_value) = text.parse::<f32>() {
                *value = new_value;
                changed = true;
            }
        }
    }

    changed
}

fn inspect_vec3(ui: &mut egui::Ui, label: &str, value: &mut Vec3, indent_level: u8) -> bool {
    let mut changed = false;
    let base_id = ui.id().with(label);

    inspector_row(ui, label, indent_level, |ui, width| {
        ui.spacing_mut().item_spacing.x = 2.0;
        let axis_width = (width - 4.0) / 3.0;

        if vec3_axis_input(ui, base_id.with("x"), "X", colors::AXIS_X, &mut value.x, axis_width) {
            changed = true;
        }
        if vec3_axis_input(ui, base_id.with("y"), "Y", colors::AXIS_Y, &mut value.y, axis_width) {
            changed = true;
        }
        if vec3_axis_input(ui, base_id.with("z"), "Z", colors::AXIS_Z, &mut value.z, axis_width) {
            changed = true;
        }
    });

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
                    *value = ParticleMesh::Sphere { radius: 0.5 };
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

fn inspect_emitter_time(ui: &mut egui::Ui, id: &str, time: &mut EmitterTime, indent_level: u8) {
    inspector_category(ui, id, "Time", indent_level, |ui, indent| {
        inspect_f32_positive(ui, &field_label("lifetime"), &mut time.lifetime, indent);
        inspect_f32_clamped(
            ui,
            &field_label("lifetime_randomness"),
            &mut time.lifetime_randomness,
            0.0,
            1.0,
            indent,
        );
        inspect_bool(ui, &field_label("one_shot"), &mut time.one_shot, indent);
        inspect_f32_clamped(
            ui,
            &field_label("explosiveness"),
            &mut time.explosiveness,
            0.0,
            1.0,
            indent,
        );
        inspect_f32_clamped(
            ui,
            &field_label("randomness"),
            &mut time.randomness,
            0.0,
            1.0,
            indent,
        );
        inspect_u32(ui, &field_label("fixed_fps"), &mut time.fixed_fps, indent);
    });
}

fn inspect_emitter_drawing(ui: &mut egui::Ui, id: &str, drawing: &mut EmitterDrawing, indent_level: u8) {
    inspector_category(ui, id, "Drawing", indent_level, |ui, indent| {
        inspect_draw_order(ui, &field_label("draw_order"), &mut drawing.draw_order, indent);
    });
}

struct DrawPassesActions {
    add_pass: bool,
    remove_pass: Option<usize>,
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
                    inspect_particle_mesh(ui, &field_label("mesh"), &mut pass.mesh, indent + 1);
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

fn inspect_process_config(ui: &mut egui::Ui, id: &str, config: &mut ParticleProcessConfig, indent_level: u8) {
    inspector_category(ui, id, "Process", indent_level, |ui, indent| {
        inspect_vec3(ui, &field_label("gravity"), &mut config.gravity, indent);
        inspect_vec3(
            ui,
            &field_label("initial_velocity"),
            &mut config.initial_velocity,
            indent,
        );
        inspect_vec3(
            ui,
            &field_label("initial_velocity_randomness"),
            &mut config.initial_velocity_randomness,
            indent,
        );
        inspect_f32_positive(ui, &field_label("initial_scale"), &mut config.initial_scale, indent);
        inspect_f32_clamped(
            ui,
            &field_label("initial_scale_randomness"),
            &mut config.initial_scale_randomness,
            0.0,
            1.0,
            indent,
        );
    });
}


pub fn draw_inspector(
    mut contexts: EguiContexts,
    mut layout: ResMut<ViewportLayout>,
    editor_state: Res<EditorState>,
    mut inspector_state: ResMut<InspectorState>,
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
            let mut should_remove_emitter: Option<usize> = None;
            let mut should_add_pass: Option<usize> = None;
            let mut should_remove_pass: Option<(usize, usize)> = None;

            let mut toggle_emitter: Option<usize> = None;
            let mut start_editing_emitter: Option<usize> = None;

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
                                should_remove_emitter = Some(idx);
                            }
                        }

                        if is_expanded {
                            ui.add_space(4.0);
                            let base_indent: u8 = 1;
                            ui.indent(&emitter_id, |ui| {
                                inspect_bool(ui, &field_label("enabled"), &mut emitter.enabled, base_indent);
                                inspect_u32(ui, &field_label("amount"), &mut emitter.amount, base_indent);

                                inspect_emitter_time(
                                    ui,
                                    &format!("{}_time", emitter_id),
                                    &mut emitter.time,
                                    base_indent,
                                );
                                inspect_emitter_drawing(
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

                                if pass_actions.add_pass {
                                    should_add_pass = Some(idx);
                                }
                                if let Some(pass_idx) = pass_actions.remove_pass {
                                    should_remove_pass = Some((idx, pass_idx));
                                }

                                inspect_process_config(
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

            if let Some(idx) = toggle_emitter {
                inspector_state.toggle_emitter(idx);
            }
            if let Some(idx) = start_editing_emitter {
                inspector_state.editing_emitter_name = Some(idx);
            }

            if should_add_emitter {
                commands.trigger(AddEmitterEvent);
            }

            if let Some(idx) = should_remove_emitter {
                commands.trigger(RemoveEmitterEvent { index: idx });
            }

            if let Some(emitter_idx) = should_add_pass {
                commands.trigger(AddDrawPassEvent {
                    emitter_index: emitter_idx,
                });
            }

            if let Some((emitter_idx, pass_idx)) = should_remove_pass {
                commands.trigger(RemoveDrawPassEvent {
                    emitter_index: emitter_idx,
                    pass_index: pass_idx,
                });
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
    editor_state: Res<EditorState>,
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
}

pub fn on_remove_emitter(
    trigger: On<RemoveEmitterEvent>,
    editor_state: Res<EditorState>,
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
    }
}

pub fn on_add_draw_pass(
    trigger: On<AddDrawPassEvent>,
    editor_state: Res<EditorState>,
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
    }
}

pub fn on_remove_draw_pass(
    trigger: On<RemoveDrawPassEvent>,
    editor_state: Res<EditorState>,
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
        }
    }
}
