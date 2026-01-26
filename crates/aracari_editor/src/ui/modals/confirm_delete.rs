use bevy::prelude::*;
use bevy_egui::egui::{self, RichText};
use bevy_egui::EguiContexts;
use egui_remixicon::icons;

use crate::ui::inspector::{RemoveDrawPassEvent, RemoveEmitterEvent};
use crate::ui::styles::{
    close_button, colors, danger_button, draw_modal_backdrop, modal_frame, modal_title_frame,
    MODAL_FOOTER_PADDING, TEXT_LG,
};

#[derive(Default, Clone, Copy, PartialEq, Eq)]
pub enum DeleteTarget {
    #[default]
    None,
    Emitter {
        index: usize,
    },
    DrawPass {
        emitter_index: usize,
        pass_index: usize,
    },
}

#[derive(Resource, Default)]
pub struct ConfirmDeleteModal {
    pub open: bool,
    pub target: DeleteTarget,
    pub item_name: String,
}

impl ConfirmDeleteModal {
    pub fn open_for_emitter(&mut self, index: usize, name: &str) {
        self.open = true;
        self.target = DeleteTarget::Emitter { index };
        self.item_name = name.to_string();
    }

    pub fn open_for_draw_pass(&mut self, emitter_index: usize, pass_index: usize) {
        self.open = true;
        self.target = DeleteTarget::DrawPass {
            emitter_index,
            pass_index,
        };
        self.item_name = format!("Pass {}", pass_index + 1);
    }

    fn reset(&mut self) {
        self.open = false;
        self.target = DeleteTarget::None;
        self.item_name.clear();
    }
}

const MODAL_PADDING: i8 = 12;

pub fn draw_confirm_delete_modal(
    mut contexts: EguiContexts,
    mut modal: ResMut<ConfirmDeleteModal>,
    mut commands: Commands,
) -> Result {
    if !modal.open {
        return Ok(());
    }

    let ctx = contexts.ctx_mut()?;

    let mut should_close = false;
    let mut should_delete = false;

    if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
        should_close = true;
    }

    let backdrop_response = egui::Area::new(egui::Id::new("confirm_delete_backdrop"))
        .fixed_pos(egui::pos2(0.0, 0.0))
        .order(egui::Order::Background)
        .show(ctx, |ui| {
            draw_modal_backdrop(ui);
            ui.allocate_response(
                ui.ctx().input(|i| i.viewport_rect().size()),
                egui::Sense::click(),
            )
        });

    if backdrop_response.inner.clicked() {
        should_close = true;
    }

    let (title, message) = match modal.target {
        DeleteTarget::None => ("Delete", "Are you sure you want to delete this item?".to_string()),
        DeleteTarget::Emitter { .. } => (
            "Delete emitter",
            format!(
                "Are you sure you want to delete \"{}\"?",
                modal.item_name
            ),
        ),
        DeleteTarget::DrawPass { .. } => (
            "Delete draw pass",
            format!(
                "Are you sure you want to delete \"{}\"?",
                modal.item_name
            ),
        ),
    };

    egui::Window::new("Confirm delete")
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .title_bar(false)
        .frame(modal_frame())
        .show(ctx, |ui| {
            modal_title_frame().show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new(title)
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
                    ui.label(message);
                });

            ui.separator();

            ui.add_space(MODAL_FOOTER_PADDING as f32);
            ui.horizontal(|ui| {
                ui.add_space(MODAL_PADDING as f32);
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.add_space(MODAL_PADDING as f32);
                    if danger_button(ui, "Delete").clicked() {
                        should_delete = true;
                    }
                    ui.add_space(8.0);
                    if ui.button("Cancel").clicked() {
                        should_close = true;
                    }
                });
            });
            ui.add_space(MODAL_FOOTER_PADDING as f32);
        });

    if should_close {
        modal.reset();
    }

    if should_delete {
        match modal.target {
            DeleteTarget::None => {}
            DeleteTarget::Emitter { index } => {
                commands.trigger(RemoveEmitterEvent { index });
            }
            DeleteTarget::DrawPass {
                emitter_index,
                pass_index,
            } => {
                commands.trigger(RemoveDrawPassEvent {
                    emitter_index,
                    pass_index,
                });
            }
        }
        modal.reset();
    }

    Ok(())
}
