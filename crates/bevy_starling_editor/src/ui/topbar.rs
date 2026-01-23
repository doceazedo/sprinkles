use std::path::Path;

use bevy::prelude::*;
use bevy_egui::egui::{self, RichText};
use bevy_egui::EguiContexts;
use bevy_starling::asset::ParticleSystemAsset;
use egui_remixicon::icons;

use crate::state::{EditorData, EditorState};
use crate::ui::modals::NewProjectModal;
use crate::ui::styles::{self, colors, ghost_button_with_icon, icon_button, icon_button_colored, icon_toggle};

pub fn draw_topbar(
    mut contexts: EguiContexts,
    mut editor_state: ResMut<EditorState>,
    mut new_project_modal: ResMut<NewProjectModal>,
    editor_data: Res<EditorData>,
    particle_systems: Res<Assets<ParticleSystemAsset>>,
) -> Result {
    let ctx = contexts.ctx_mut()?;

    egui::TopBottomPanel::top("topbar")
        .frame(styles::topbar_frame())
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                let project_name = editor_state.project_name(&particle_systems);

                let button_response =
                    ghost_button_with_icon(ui, &project_name, icons::ARROW_DOWN_S_LINE);

                egui::Popup::menu(&button_response)
                    .width(180.0)
                    .show(|ui| {
                        if ui
                            .button(format!("{} New project...", icons::FILE_ADD_LINE))
                            .clicked()
                        {
                            new_project_modal.open = true;
                        }
                        if ui
                            .button(format!("{} Open...", icons::FOLDER_OPEN_LINE))
                            .clicked()
                        {
                            // TODO: implement file open dialog
                        }

                        ui.separator();

                        ui.label(RichText::new("Recent projects").strong().size(12.0));
                        if editor_data.cache.recent_projects.is_empty() {
                            ui.weak("No recent projects");
                        } else {
                            for file_name in &editor_data.cache.recent_projects {
                                if let Some(name) = Path::new(file_name).file_stem().and_then(|s| s.to_str()) {
                                    if ui.button(name).clicked() {
                                        // TODO: load the project
                                    }
                                }
                            }
                        }
                    });

                ui.separator();

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
                        editor_state.current_frame = 0;
                    }

                    let play_pause_icon = if editor_state.is_playing {
                        icons::PAUSE_FILL
                    } else {
                        icons::PLAY_FILL
                    };
                    if icon_button_colored(ui, play_pause_icon, colors::GREEN, colors::green_hover())
                        .clicked()
                    {
                        editor_state.is_playing = !editor_state.is_playing;
                    }

                    let progress_text = format!(
                        "{}/{}",
                        editor_state.current_frame, editor_state.total_frames
                    );
                    ui.label(progress_text);
                });
            });
        });

    Ok(())
}
