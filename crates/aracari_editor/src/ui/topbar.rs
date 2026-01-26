use std::path::PathBuf;

use bevy::prelude::*;
use bevy_egui::egui::{self, Color32, CornerRadius, FontId, Pos2, RichText, Vec2};
use bevy_egui::EguiContexts;
use aracari::prelude::*;
use egui_remixicon::icons;

use crate::state::{format_display_path, project_path, save_editor_data, EditorData, EditorState};
use crate::ui::modals::{NewProjectModal, OpenFileDialogEvent, OpenProjectEvent, SaveProjectEvent};
use crate::ui::styles::{self, colors, ghost_button_with_icon, icon_button, icon_button_colored, icon_toggle, ICON_BUTTON_SIZE, TEXT_BASE, TEXT_SM};

const BADGE_SIZE: f32 = 8.0;
const BADGE_OFFSET: f32 = 6.0;
const SAVED_LABEL_VISIBLE_DURATION: f64 = 1.0;
const SAVED_LABEL_FADE_DURATION: f64 = 0.8;

pub fn draw_topbar(
    mut contexts: EguiContexts,
    mut editor_state: ResMut<EditorState>,
    mut new_project_modal: ResMut<NewProjectModal>,
    mut editor_data: ResMut<EditorData>,
    particle_systems: Res<Assets<ParticleSystemAsset>>,
    mut commands: Commands,
    time: Res<Time<Real>>,
) -> Result {
    let ctx = contexts.ctx_mut()?;
    let current_time = time.elapsed_secs_f64();

    // check if save completed
    editor_state.check_save_completed(current_time);

    // handle Ctrl/Cmd + S keyboard shortcut
    let modifiers = ctx.input(|i| i.modifiers);
    let save_shortcut_pressed = ctx.input(|i| i.key_pressed(egui::Key::S))
        && (modifiers.command || modifiers.ctrl);

    if save_shortcut_pressed && !editor_state.is_saving {
        commands.trigger(SaveProjectEvent);
    }

    egui::TopBottomPanel::top("topbar")
        .frame(styles::topbar_frame())
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                let project_name = editor_state.project_name(&particle_systems);

                let button_response =
                    ghost_button_with_icon(ui, &project_name, icons::ARROW_DOWN_S_LINE);

                let mut open_file_dialog = false;
                let mut open_project_path: Option<PathBuf> = None;

                egui::Popup::menu(&button_response).show(|ui| {
                        let new_project_response = ui
                            .button(format!("{} New project...", icons::FILE_ADD_LINE));
                        if new_project_response.clicked() {
                            new_project_modal.open = true;
                        }

                        // use the width of the first button as reference for recent project rows
                        let menu_item_width = new_project_response.rect.width();

                        if ui
                            .button(format!("{} Open...", icons::FOLDER_OPEN_LINE))
                            .clicked()
                        {
                            open_file_dialog = true;
                        }

                        ui.separator();

                        ui.label(RichText::new("Recent projects").strong().size(TEXT_SM));
                        if editor_data.cache.recent_projects.is_empty() {
                            ui.weak("No recent projects");
                        } else {
                            let mut path_to_remove: Option<String> = None;

                            for recent_path in editor_data.cache.recent_projects.clone() {
                                let display_path = format_display_path(&recent_path);

                                let row_response =
                                    draw_recent_project_row(ui, &display_path, menu_item_width);

                                if row_response.clicked {
                                    open_project_path = Some(project_path(&recent_path));
                                }
                                if row_response.remove_clicked {
                                    path_to_remove = Some(recent_path.clone());
                                }
                            }

                            if let Some(path) = path_to_remove {
                                editor_data.cache.remove_recent_project(&path);
                                save_editor_data(&editor_data);
                            }
                        }
                    });

                if open_file_dialog {
                    commands.trigger(OpenFileDialogEvent);
                }
                if let Some(path) = open_project_path {
                    commands.trigger(OpenProjectEvent { path });
                }

                ui.separator();

                // save button with badge and "Saved!" label
                let save_response = draw_save_button(ui, &editor_state, current_time);
                if save_response.clicked() && !editor_state.is_saving {
                    commands.trigger(SaveProjectEvent);
                }

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if icon_toggle(
                        ui,
                        icons::REPEAT_FILL,
                        editor_state.is_looping,
                        colors::BLUE,
                        colors::blue_semi(),
                        colors::blue_hover(),
                    )
                    .clicked()
                    {
                        editor_state.is_looping = !editor_state.is_looping;
                    }

                    if icon_button(ui, icons::STOP_FILL).clicked() {
                        editor_state.is_playing = false;
                        editor_state.should_reset = true;
                    }

                    let play_pause_icon = if editor_state.is_playing {
                        icons::PAUSE_FILL
                    } else {
                        icons::PLAY_FILL
                    };
                    if icon_button_colored(ui, play_pause_icon, colors::GREEN, colors::green_hover())
                        .clicked()
                    {
                        if !editor_state.is_playing {
                            editor_state.play_requested = true;
                        }
                        editor_state.is_playing = !editor_state.is_playing;
                    }

                    // progress bar (right-to-left layout, so this appears on the right)
                    let progress_width = 192.0;
                    let progress_height = 8.0;
                    let progress_ratio = if editor_state.duration_ms > 0.0 {
                        (editor_state.elapsed_ms / editor_state.duration_ms).clamp(0.0, 1.0)
                    } else {
                        0.0
                    };

                    let (rect, _) = ui.allocate_exact_size(
                        egui::vec2(progress_width, progress_height),
                        egui::Sense::hover(),
                    );

                    if ui.is_rect_visible(rect) {
                        // background
                        ui.painter().rect_filled(
                            rect,
                            egui::CornerRadius::same(4),
                            colors::ZINC_600,
                        );

                        // progress fill (fill from left side)
                        if progress_ratio > 0.0 {
                            let fill_rect = egui::Rect::from_min_size(
                                rect.min,
                                egui::vec2(progress_width * progress_ratio, progress_height),
                            );
                            ui.painter().rect_filled(
                                fill_rect,
                                egui::CornerRadius::same(4),
                                colors::ZINC_300,
                            );
                        }
                    }

                    ui.add_space(8.0);

                    let elapsed_s = editor_state.elapsed_ms / 1000.0;
                    let duration_s = editor_state.duration_ms / 1000.0;
                    let progress_text = format!("{:.1}/{:.1}s", elapsed_s, duration_s);
                    ui.label(progress_text);
                });
            });
        });

    Ok(())
}

fn draw_save_button(ui: &mut egui::Ui, editor_state: &EditorState, current_time: f64) -> egui::Response {
    // calculate "Saved!" label opacity
    let saved_label_opacity = if let Some(completed_at) = editor_state.save_completed_at {
        let elapsed = current_time - completed_at;
        if elapsed < SAVED_LABEL_VISIBLE_DURATION {
            1.0
        } else {
            let fade_progress = (elapsed - SAVED_LABEL_VISIBLE_DURATION) / SAVED_LABEL_FADE_DURATION;
            (1.0 - fade_progress).max(0.0) as f32
        }
    } else {
        0.0
    };

    // request repaint during animation
    if saved_label_opacity > 0.0 {
        ui.ctx().request_repaint();
    }

    // also request repaint while saving (for spinner animation)
    if editor_state.is_saving {
        ui.ctx().request_repaint();
    }

    // calculate total width needed
    let saved_label_text = "Saved!";
    let label_galley = ui.painter().layout_no_wrap(
        saved_label_text.to_string(),
        FontId::proportional(TEXT_SM),
        colors::TEXT_MUTED,
    );
    let label_spacing = 4.0;
    let label_width = if saved_label_opacity > 0.0 {
        label_galley.size().x + label_spacing
    } else {
        0.0
    };

    let total_width = ICON_BUTTON_SIZE + label_width;
    let (total_rect, response) = ui.allocate_exact_size(
        Vec2::new(total_width, ICON_BUTTON_SIZE),
        egui::Sense::click(),
    );

    if ui.is_rect_visible(total_rect) {
        let button_rect = egui::Rect::from_min_size(
            total_rect.min,
            Vec2::splat(ICON_BUTTON_SIZE),
        );

        // draw button background
        let bg_color = if response.hovered() && !editor_state.is_saving {
            colors::hover_bg()
        } else {
            Color32::TRANSPARENT
        };
        ui.painter().rect_filled(button_rect, CornerRadius::same(4), bg_color);

        // draw icon (save or loader)
        let icon_pos = button_rect.center() + Vec2::new(0.0, 1.0);
        if editor_state.is_saving {
            // rotating loader icon
            let rotation = (current_time * 4.0) as f32;
            let _icon_galley = ui.painter().layout_no_wrap(
                icons::LOADER_FILL.to_string(),
                FontId::proportional(TEXT_BASE),
                colors::TEXT_MUTED,
            );

            // use a transform to rotate around center
            let painter = ui.painter();
            let icon_center = icon_pos;

            // egui doesn't have built-in rotation for text, so we'll simulate with position offset
            // for a simple spinning effect, we can just use the loader icon which already looks good
            painter.text(
                Pos2::new(
                    icon_center.x + rotation.sin() * 0.5,
                    icon_center.y + rotation.cos() * 0.5,
                ),
                egui::Align2::CENTER_CENTER,
                icons::LOADER_FILL,
                FontId::proportional(TEXT_BASE),
                colors::TEXT_MUTED,
            );
        } else {
            ui.painter().text(
                icon_pos,
                egui::Align2::CENTER_CENTER,
                icons::SAVE_3_FILL,
                FontId::proportional(TEXT_BASE),
                ui.visuals().text_color(),
            );
        }

        // draw unsaved changes badge
        if editor_state.has_unsaved_changes && !editor_state.is_saving {
            let badge_center = Pos2::new(
                button_rect.center().x + BADGE_OFFSET,
                button_rect.center().y - BADGE_OFFSET,
            );
            ui.painter().circle_filled(
                badge_center,
                BADGE_SIZE / 2.0,
                colors::RED_400,
            );
        }

        // draw "Saved!" label
        if saved_label_opacity > 0.0 {
            let label_color = Color32::from_rgba_unmultiplied(
                colors::TEXT_MUTED.r(),
                colors::TEXT_MUTED.g(),
                colors::TEXT_MUTED.b(),
                (saved_label_opacity * 255.0) as u8,
            );
            let label_pos = Pos2::new(
                button_rect.right() + label_spacing,
                button_rect.center().y,
            );
            ui.painter().text(
                label_pos,
                egui::Align2::LEFT_CENTER,
                saved_label_text,
                FontId::proportional(TEXT_SM),
                label_color,
            );
        }
    }

    response
}

struct RecentProjectRowResponse {
    clicked: bool,
    remove_clicked: bool,
}

fn draw_recent_project_row(
    ui: &mut egui::Ui,
    display_path: &str,
    row_width: f32,
) -> RecentProjectRowResponse {
    let mut response = RecentProjectRowResponse {
        clicked: false,
        remove_clicked: false,
    };

    let item_spacing = 4.0;
    let close_button_size = 24.0;
    let button_width = row_width - close_button_size - item_spacing;

    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = item_spacing;

        // draw clickable button that fills the row width
        let text_response = ui.add_sized(
            Vec2::new(button_width, close_button_size),
            egui::Button::new(display_path).right_text(""),
        );

        if text_response.clicked() {
            response.clicked = true;
        }

        // remove button - only show icon on row hover
        let row_hovered = text_response.hovered() || ui.rect_contains_pointer(ui.max_rect());

        // check if pointer is over the remove button area before drawing
        let button_pos = ui.cursor().min;
        let button_rect = egui::Rect::from_min_size(button_pos, Vec2::splat(close_button_size));
        let button_hovered = ui.rect_contains_pointer(button_rect);

        let icon_color = if button_hovered {
            colors::TEXT_MUTED
        } else if row_hovered {
            colors::ZINC_500
        } else {
            Color32::TRANSPARENT
        };

        let remove_button = egui::Button::new(
            RichText::new(icons::CLOSE_FILL)
                .size(14.0)
                .color(icon_color),
        )
        .min_size(Vec2::splat(close_button_size));

        let remove_response = ui.add(remove_button);

        if row_hovered && remove_response.clicked() {
            response.remove_clicked = true;
        }
    });

    response
}
