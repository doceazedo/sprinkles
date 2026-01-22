use std::fs::File;
use std::io::Write;
use std::path::Path;

use bevy::asset::io::file::FileAssetReader;
use bevy::prelude::*;
use bevy::tasks::IoTaskPool;
use bevy_egui::egui::{self, RichText};
use bevy_egui::EguiContexts;
use bevy_starling::asset::{EmitterData, ParticleSystemAsset, ParticleSystemDimension};

use crate::state::{save_editor_data, EditorData, EditorState};
use egui_remixicon::icons;

use crate::ui::styles::{
    close_button, colors, draw_modal_backdrop, modal_frame, modal_title_frame, primary_button,
    styled_radio, MODAL_FOOTER_PADDING,
};

#[derive(Event)]
pub struct CreateProjectEvent {
    pub project_name: String,
    pub file_name: String,
    pub dimension: ParticleSystemDimension,
}

const DEFAULT_PROJECT_NAME: &str = "Untitled project";

#[derive(Resource)]
pub struct NewProjectModal {
    pub open: bool,
    pub project_name: String,
    pub file_name: String,
    pub dimension: ParticleSystemDimension,
    pub file_name_edited: bool,
    pub untitled_counter: u32,
    pub focus_requested: bool,
}

impl Default for NewProjectModal {
    fn default() -> Self {
        Self {
            open: false,
            project_name: String::new(),
            file_name: String::new(),
            dimension: ParticleSystemDimension::D3,
            file_name_edited: false,
            untitled_counter: 1,
            focus_requested: false,
        }
    }
}

impl NewProjectModal {
    fn reset(&mut self) {
        self.project_name.clear();
        self.file_name.clear();
        self.dimension = ParticleSystemDimension::D3;
        self.file_name_edited = false;
        self.focus_requested = false;
    }

    fn default_name(&self) -> String {
        if self.untitled_counter == 1 {
            DEFAULT_PROJECT_NAME.to_string()
        } else {
            format!("{} {}", DEFAULT_PROJECT_NAME, self.untitled_counter)
        }
    }

    fn effective_project_name(&self) -> String {
        if self.project_name.trim().is_empty() {
            self.default_name()
        } else {
            self.project_name.clone()
        }
    }

    fn effective_file_name(&self) -> String {
        if self.file_name.trim().is_empty() {
            to_file_name(&self.default_name())
        } else {
            self.file_name.clone()
        }
    }
}

const LABEL_WIDTH: f32 = 100.0;
const MODAL_PADDING: i8 = 12;
const INPUT_WIDTH: f32 = 384.0;

pub fn draw_new_project_modal(
    mut contexts: EguiContexts,
    mut modal: ResMut<NewProjectModal>,
    mut commands: Commands,
) -> Result {
    if !modal.open {
        return Ok(());
    }

    let ctx = contexts.ctx_mut()?;

    let mut should_close = false;
    let mut should_create = false;

    if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
        should_close = true;
    }

    let backdrop_response = egui::Area::new(egui::Id::new("modal_backdrop"))
        .fixed_pos(egui::pos2(0.0, 0.0))
        .order(egui::Order::Background)
        .show(ctx, |ui| {
            draw_modal_backdrop(ui);
            ui.allocate_response(ui.ctx().input(|i| i.viewport_rect().size()), egui::Sense::click())
        });

    if backdrop_response.inner.clicked() {
        should_close = true;
    }

    egui::Window::new("New project")
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .title_bar(false)
        .frame(modal_frame())
        .show(ctx, |ui| {
            modal_title_frame().show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new("New project")
                            .strong()
                            .size(18.0)
                            .color(colors::ZINC_200),
                    );

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if close_button(ui, icons::CLOSE_LINE).clicked() {
                            should_close = true;
                        }
                    });
                });
            });

            egui::Frame::NONE
                .inner_margin(egui::Margin::same(MODAL_PADDING))
                .show(ui, |ui| {
                    let default_name = modal.default_name();
                    let default_file_name = to_file_name(&default_name);
                    let placeholder_color = colors::placeholder_text();

                    ui.horizontal(|ui| {
                        ui.allocate_ui_with_layout(
                            egui::vec2(LABEL_WIDTH, 24.0),
                            egui::Layout::right_to_left(egui::Align::Center),
                            |ui| {
                                ui.label("Project name:");
                            },
                        );
                        ui.add_space(8.0);
                        let response = ui.add(
                            egui::TextEdit::singleline(&mut modal.project_name)
                                .desired_width(INPUT_WIDTH)
                                .hint_text(RichText::new(&default_name).color(placeholder_color)),
                        );
                        if response.changed() && !modal.file_name_edited {
                            modal.file_name = to_file_name(&modal.project_name);
                        }
                        if !modal.focus_requested {
                            response.request_focus();
                            modal.focus_requested = true;
                        }
                    });

                    ui.add_space(8.0);

                    ui.horizontal(|ui| {
                        ui.allocate_ui_with_layout(
                            egui::vec2(LABEL_WIDTH, 24.0),
                            egui::Layout::right_to_left(egui::Align::Center),
                            |ui| {
                                ui.label("File name:");
                            },
                        );
                        ui.add_space(8.0);
                        let response = ui.add(
                            egui::TextEdit::singleline(&mut modal.file_name)
                                .desired_width(INPUT_WIDTH - 70.0)
                                .hint_text(RichText::new(&default_file_name).color(placeholder_color)),
                        );
                        if response.changed() {
                            modal.file_name_edited = true;
                        }
                        ui.label(".starling");
                    });

                    ui.add_space(8.0);

                    ui.horizontal(|ui| {
                        ui.allocate_ui_with_layout(
                            egui::vec2(LABEL_WIDTH, 24.0),
                            egui::Layout::right_to_left(egui::Align::Center),
                            |ui| {
                                ui.label("Dimension:");
                            },
                        );
                        ui.add_space(8.0);

                        if styled_radio(ui, modal.dimension == ParticleSystemDimension::D3, "3D")
                            .clicked()
                        {
                            modal.dimension = ParticleSystemDimension::D3;
                        }
                        if styled_radio(ui, modal.dimension == ParticleSystemDimension::D2, "2D")
                            .clicked()
                        {
                            modal.dimension = ParticleSystemDimension::D2;
                        }
                    });
                });

            ui.separator();

            ui.add_space(MODAL_FOOTER_PADDING as f32);
            ui.horizontal(|ui| {
                ui.add_space(MODAL_PADDING as f32);
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.add_space(MODAL_PADDING as f32);
                    if primary_button(ui, "Create").clicked() {
                        should_create = true;
                    }
                });
            });
            ui.add_space(MODAL_FOOTER_PADDING as f32);
        });

    if should_close {
        modal.open = false;
        modal.reset();
    }

    if should_create {
        commands.trigger(CreateProjectEvent {
            project_name: modal.effective_project_name(),
            file_name: modal.effective_file_name(),
            dimension: modal.dimension,
        });
    }

    Ok(())
}

pub fn on_create_project_event(
    trigger: On<CreateProjectEvent>,
    mut modal: ResMut<NewProjectModal>,
    mut editor_state: ResMut<EditorState>,
    mut editor_data: ResMut<EditorData>,
    asset_server: Res<AssetServer>,
) {
    let event = trigger.event();
    let file_name = format!("{}.starling", event.file_name);

    let asset = ParticleSystemAsset {
        name: event.project_name.clone(),
        dimension: event.dimension,
        emitters: vec![EmitterData {
            name: "Emitter 1".to_string(),
        }],
    };

    let contents = match ron::ser::to_string_pretty(&asset, ron::ser::PrettyConfig::default()) {
        Ok(contents) => contents,
        Err(_) => return,
    };

    let path = Path::join(
        &FileAssetReader::get_base_path(),
        Path::join(Path::new("assets"), Path::new(&file_name)),
    );

    let write_path = path.clone();
    IoTaskPool::get()
        .spawn(async move {
            let mut file = File::create(&write_path).expect("failed to create file");
            file.write_all(contents.as_bytes())
                .expect("failed to write to file");
        })
        .detach();

    editor_data.cache.add_recent_project(path.clone());
    save_editor_data(&editor_data);

    editor_state.current_project = Some(asset_server.load(file_name.clone()));
    editor_state.current_project_path = Some(path);

    modal.untitled_counter += 1;
    modal.open = false;
    modal.reset();
}

fn to_file_name(name: &str) -> String {
    name.to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '_' })
        .collect::<String>()
        .trim_matches('_')
        .to_string()
}

