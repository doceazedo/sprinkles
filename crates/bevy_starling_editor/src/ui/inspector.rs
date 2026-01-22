use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts};

use crate::ui::styles::colors;

pub fn draw_inspector(mut contexts: EguiContexts) -> Result {
    let ctx = contexts.ctx_mut()?;

    egui::SidePanel::left("inspector")
        .resizable(true)
        .default_width(384.0)
        .min_width(200.0)
        .frame(
            egui::Frame::NONE
                .fill(colors::PANEL_BG)
                .inner_margin(egui::Margin::same(8)),
        )
        .show(ctx, |ui| {
            ui.heading("Inspector");
            ui.separator();
        });

    Ok(())
}
