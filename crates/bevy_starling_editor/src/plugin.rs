use bevy::prelude::*;
use bevy_egui::{
    egui, input::egui_wants_any_pointer_input, EguiContexts, EguiPlugin, EguiPrimaryContextPass,
};
use bevy_starling::StarlingPlugin;

use crate::state::{load_editor_data, project_path, EditorData, EditorState, InspectorState};
use crate::ui::modals::{draw_new_project_modal, on_create_project_event, NewProjectModal};
use crate::ui::{
    configure_style, draw_inspector, draw_topbar, on_add_draw_pass, on_add_emitter,
    on_remove_draw_pass, on_remove_emitter,
};
use crate::viewport::{
    despawn_preview_on_project_change, draw_grid, orbit_camera, setup_camera,
    spawn_preview_particle_system, sync_playback_state, update_camera_viewport, zoom_camera,
    CameraSettings, ViewportLayout,
};

pub struct StarlingEditorPlugin;

impl Plugin for StarlingEditorPlugin {
    fn build(&self, app: &mut App) {
        let editor_data = load_editor_data();

        app.add_plugins(StarlingPlugin)
            .add_plugins(EguiPlugin::default())
            .init_resource::<EditorState>()
            .init_resource::<InspectorState>()
            .init_resource::<CameraSettings>()
            .init_resource::<ViewportLayout>()
            .init_resource::<NewProjectModal>()
            .insert_resource(editor_data)
            .insert_resource(EguiConfigured(false))
            .add_observer(on_create_project_event)
            .add_observer(on_add_emitter)
            .add_observer(on_remove_emitter)
            .add_observer(on_add_draw_pass)
            .add_observer(on_remove_draw_pass)
            .add_systems(Startup, (setup_camera, load_initial_project))
            .add_systems(
                Update,
                (
                    orbit_camera.run_if(not(egui_wants_any_pointer_input)),
                    zoom_camera.run_if(not(egui_wants_any_pointer_input)),
                    update_camera_viewport,
                    draw_grid,
                    spawn_preview_particle_system,
                    despawn_preview_on_project_change,
                    sync_playback_state,
                ),
            )
            .add_systems(
                EguiPrimaryContextPass,
                (
                    setup_egui.run_if(not(egui_configured)),
                    draw_topbar,
                    draw_inspector,
                    draw_new_project_modal,
                ),
            );
    }
}

#[derive(Resource)]
struct EguiConfigured(bool);

fn egui_configured(configured: Res<EguiConfigured>) -> bool {
    configured.0
}

fn setup_egui(mut contexts: EguiContexts, mut configured: ResMut<EguiConfigured>) -> Result {
    let ctx = contexts.ctx_mut()?;

    let mut fonts = egui::FontDefinitions::default();
    egui_remixicon::add_to_fonts(&mut fonts);
    ctx.set_fonts(fonts);

    configure_style(ctx);

    configured.0 = true;
    Ok(())
}

fn load_initial_project(
    mut editor_state: ResMut<EditorState>,
    editor_data: Res<EditorData>,
    asset_server: Res<AssetServer>,
) {
    if let Some(file_name) = &editor_data.cache.last_opened_project {
        let path = project_path(file_name);
        if path.exists() {
            editor_state.current_project = Some(asset_server.load(file_name));
            editor_state.current_project_path = Some(path);
            return;
        }
    }

    editor_state.current_project = Some(asset_server.load("demo.starling"));
}
