use std::fs::File;
use std::io::Write;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use bevy::prelude::*;
use bevy::tasks::IoTaskPool;
use bevy_egui::egui::{self, RichText};
use bevy_egui::EguiContexts;
use aracari::prelude::*;

use crate::state::{
    load_project_from_path, project_path, save_editor_data, EditorData, EditorState,
    DEFAULT_PROJECTS_DIR,
};
use egui_remixicon::icons;

use crate::ui::styles::{
    close_button, colors, draw_modal_backdrop, modal_frame, modal_title_frame, primary_button,
    styled_radio, MODAL_FOOTER_PADDING, TEXT_LG,
};

#[derive(Event)]
pub struct CreateProjectEvent {
    pub project_name: String,
    pub location: String,
    pub dimension: ParticleSystemDimension,
}

#[derive(Event)]
pub struct SaveProjectEvent;

#[derive(Event)]
pub struct OpenProjectEvent {
    pub path: PathBuf,
}

/// Event triggered when the "Open..." button is clicked
#[derive(Event)]
pub struct OpenFileDialogEvent;

/// Resource to track the async file dialog state
#[derive(Resource, Default)]
pub struct OpenFileDialogState {
    pub is_open: bool,
    pub completed: Option<Arc<AtomicBool>>,
    pub result: Option<Arc<Mutex<Option<PathBuf>>>>,
}

const DEFAULT_PROJECT_NAME: &str = "Untitled project";

#[derive(Resource)]
pub struct NewProjectModal {
    pub open: bool,
    pub project_name: String,
    pub location: String,
    pub dimension: ParticleSystemDimension,
    pub location_edited: bool,
    pub untitled_counter: u32,
    pub focus_requested: bool,
}

impl Default for NewProjectModal {
    fn default() -> Self {
        Self {
            open: false,
            project_name: String::new(),
            location: String::new(),
            dimension: ParticleSystemDimension::D3,
            location_edited: false,
            untitled_counter: 1,
            focus_requested: false,
        }
    }
}

impl NewProjectModal {
    fn reset(&mut self) {
        self.project_name.clear();
        self.location.clear();
        self.dimension = ParticleSystemDimension::D3;
        self.location_edited = false;
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

    fn effective_location(&self) -> String {
        if self.location.trim().is_empty() {
            format!("{}/{}", DEFAULT_PROJECTS_DIR, to_file_name(&self.default_name()))
        } else {
            self.location.clone()
        }
    }

    fn default_location(&self) -> String {
        format!("{}/{}", DEFAULT_PROJECTS_DIR, to_file_name(&self.default_name()))
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
                            .size(TEXT_LG)
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
                    let default_location = modal.default_location();
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
                        if response.changed() && !modal.location_edited {
                            modal.location = format!(
                                "{}/{}",
                                DEFAULT_PROJECTS_DIR,
                                to_file_name(&modal.project_name)
                            );
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
                                ui.label("Location:");
                            },
                        );
                        ui.add_space(8.0);
                        let response = ui.add(
                            egui::TextEdit::singleline(&mut modal.location)
                                .desired_width(INPUT_WIDTH)
                                .hint_text(RichText::new(&default_location).color(placeholder_color)),
                        );
                        if response.changed() {
                            modal.location_edited = true;
                        }
                        ui.label(".ron");
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
                        ui.add_enabled_ui(false, |ui| {
                            styled_radio(ui, modal.dimension == ParticleSystemDimension::D2, "2D (TODO)");
                        });
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
            location: modal.effective_location(),
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
    mut assets: ResMut<Assets<ParticleSystemAsset>>,
) {
    let event = trigger.event();
    let location_with_ext = format!("{}.ron", event.location);

    let asset = ParticleSystemAsset {
        name: event.project_name.clone(),
        dimension: event.dimension,
        emitters: vec![EmitterData {
            name: "Emitter 1".to_string(),
            ..Default::default()
        }],
    };

    let contents = match ron::ser::to_string_pretty(&asset, ron::ser::PrettyConfig::default()) {
        Ok(contents) => contents,
        Err(_) => return,
    };

    let path = project_path(&location_with_ext);
    let is_default_projects_dir = event.location.starts_with(DEFAULT_PROJECTS_DIR);

    let write_path = path.clone();
    IoTaskPool::get()
        .spawn(async move {
            if is_default_projects_dir {
                if let Some(parent) = write_path.parent() {
                    let _ = std::fs::create_dir_all(parent);
                }
            }
            let mut file = File::create(&write_path).expect("failed to create file");
            file.write_all(contents.as_bytes())
                .expect("failed to write to file");
        })
        .detach();

    editor_data.cache.add_recent_project(location_with_ext.clone());
    save_editor_data(&editor_data);

    let handle = assets.add(asset);
    editor_state.current_project = Some(handle);
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

pub fn on_save_project_event(
    _trigger: On<SaveProjectEvent>,
    mut editor_state: ResMut<EditorState>,
    particle_systems: Res<Assets<ParticleSystemAsset>>,
) {
    if editor_state.is_saving {
        return;
    }

    let Some(handle) = &editor_state.current_project else {
        return;
    };

    let Some(asset) = particle_systems.get(handle) else {
        return;
    };

    let Some(path) = &editor_state.current_project_path else {
        return;
    };

    let contents = match ron::ser::to_string_pretty(asset, ron::ser::PrettyConfig::default()) {
        Ok(contents) => contents,
        Err(_) => return,
    };

    let write_path = path.clone();
    editor_state.is_saving = true;
    editor_state.save_completed_at = None;

    let complete_flag = Arc::new(AtomicBool::new(false));
    let task_flag = complete_flag.clone();
    editor_state.save_complete_flag = Some(complete_flag);

    IoTaskPool::get()
        .spawn(async move {
            let mut file = File::create(&write_path).expect("failed to create file");
            file.write_all(contents.as_bytes())
                .expect("failed to write to file");
            task_flag.store(true, Ordering::Relaxed);
        })
        .detach();
}

pub fn on_open_project_event(
    trigger: On<OpenProjectEvent>,
    mut editor_state: ResMut<EditorState>,
    mut editor_data: ResMut<EditorData>,
    mut assets: ResMut<Assets<ParticleSystemAsset>>,
) {
    let event = trigger.event();
    let path = &event.path;

    let Some(asset) = load_project_from_path(path) else {
        warn!("failed to load project from {:?}", path);
        return;
    };

    let handle = assets.add(asset);
    editor_state.current_project = Some(handle);
    editor_state.current_project_path = Some(path.clone());
    editor_state.has_unsaved_changes = false;

    // add to recent projects using a path relative to working dir if possible
    let display_path = path
        .strip_prefix(std::env::current_dir().unwrap_or_default())
        .map(|p| format!("./{}", p.display()))
        .unwrap_or_else(|_| path.display().to_string());

    editor_data.cache.add_recent_project(display_path);
    save_editor_data(&editor_data);
}

pub fn on_open_file_dialog_event(
    _trigger: On<OpenFileDialogEvent>,
    mut dialog_state: ResMut<OpenFileDialogState>,
) {
    if dialog_state.is_open {
        return;
    }

    dialog_state.is_open = true;

    let completed = Arc::new(AtomicBool::new(false));
    let result = Arc::new(Mutex::new(None));

    let task_completed = completed.clone();
    let task_result = result.clone();

    dialog_state.completed = Some(completed);
    dialog_state.result = Some(result);

    IoTaskPool::get()
        .spawn(async move {
            let file = rfd::AsyncFileDialog::new()
                .add_filter("Aracari Project", &["ron"])
                .pick_file()
                .await;

            if let Some(file) = file {
                let path = file.path().to_path_buf();
                if let Ok(mut guard) = task_result.lock() {
                    *guard = Some(path);
                }
            }

            task_completed.store(true, Ordering::Release);
        })
        .detach();
}

pub fn poll_open_file_dialog(
    mut dialog_state: ResMut<OpenFileDialogState>,
    mut commands: Commands,
) {
    if !dialog_state.is_open {
        return;
    }

    let Some(completed) = &dialog_state.completed else {
        return;
    };

    if !completed.load(Ordering::Acquire) {
        return;
    }

    // dialog completed, check result
    if let Some(result) = &dialog_state.result {
        if let Ok(guard) = result.lock() {
            if let Some(path) = guard.clone() {
                commands.trigger(OpenProjectEvent { path });
            }
        }
    }

    // reset state
    dialog_state.is_open = false;
    dialog_state.completed = None;
    dialog_state.result = None;
}

