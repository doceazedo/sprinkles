use bevy::color::palettes::tailwind::ZINC_950;
use bevy::prelude::*;
use bevy_egui::{
    egui, input::egui_wants_any_pointer_input, EguiContexts, EguiPlugin, EguiPrimaryContextPass,
};
use aracari::prelude::*;

use crate::state::{load_editor_data, load_project_from_path, project_path, save_editor_data, EditorData, EditorState, InspectorState};
use crate::ui::modals::{
    draw_confirm_delete_modal, draw_new_project_modal, on_create_project_event,
    on_open_file_dialog_event, on_open_project_event, on_save_project_event,
    poll_open_file_dialog, ConfirmDeleteModal, NewProjectModal, OpenFileDialogState,
};
use crate::ui::{
    configure_style, draw_inspector, draw_topbar, on_add_draw_pass, on_add_emitter,
    on_remove_draw_pass, on_remove_emitter,
};
use crate::viewport::{
    configure_floor_texture, despawn_preview_on_project_change, orbit_camera, setup_camera,
    setup_floor, spawn_preview_particle_system, sync_playback_state, update_camera_viewport,
    zoom_camera, CameraSettings, ViewportLayout,
};

pub struct AracariEditorPlugin;

impl Plugin for AracariEditorPlugin {
    fn build(&self, app: &mut App) {
        let editor_data = load_editor_data();

        app.add_plugins(AracariPlugin)
            .add_plugins(EguiPlugin::default())
            .init_resource::<EditorState>()
            .init_resource::<InspectorState>()
            .init_resource::<CameraSettings>()
            .init_resource::<ViewportLayout>()
            .init_resource::<NewProjectModal>()
            .init_resource::<ConfirmDeleteModal>()
            .init_resource::<OpenFileDialogState>()
            .insert_resource(editor_data)
            .insert_resource(EguiConfigured(false))
            .insert_resource(ClearColor(ZINC_950.into()))
            .add_observer(on_create_project_event)
            .add_observer(on_save_project_event)
            .add_observer(on_open_project_event)
            .add_observer(on_open_file_dialog_event)
            .add_observer(on_add_emitter)
            .add_observer(on_remove_emitter)
            .add_observer(on_add_draw_pass)
            .add_observer(on_remove_draw_pass)
            .add_systems(Startup, (setup_camera, setup_floor, load_initial_project))
            .add_systems(
                Update,
                (
                    orbit_camera.run_if(not(egui_wants_any_pointer_input)),
                    zoom_camera.run_if(not(egui_wants_any_pointer_input)),
                    update_camera_viewport,
                    configure_floor_texture,
                    spawn_preview_particle_system,
                    despawn_preview_on_project_change,
                    sync_playback_state,
                    poll_open_file_dialog,
                ),
            )
            .add_systems(
                EguiPrimaryContextPass,
                (
                    setup_egui.run_if(not(egui_configured)),
                    (draw_topbar, draw_inspector).chain(),
                    draw_new_project_modal,
                    draw_confirm_delete_modal,
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
    mut editor_data: ResMut<EditorData>,
    mut assets: ResMut<Assets<ParticleSystemAsset>>,
) {
    // try to load last opened project
    if let Some(location) = &editor_data.cache.last_opened_project.clone() {
        let path = project_path(location);
        if path.exists() {
            if let Some(asset) = load_project_from_path(&path) {
                let handle = assets.add(asset);
                editor_state.current_project = Some(handle);
                editor_state.current_project_path = Some(path);
                return;
            }
        }
    }

    // only load demo on first run (when no recent projects exist)
    let is_first_run = editor_data.cache.recent_projects.is_empty();

    if is_first_run {
        let demo_file = "examples/3d_explosion.ron";
        let demo_path = project_path(demo_file);
        if demo_path.exists() {
            if let Some(asset) = load_project_from_path(&demo_path) {
                let handle = assets.add(asset);
                editor_state.current_project = Some(handle);
                editor_state.current_project_path = Some(demo_path);

                // add demo to recent projects
                editor_data.cache.add_recent_project(demo_file.to_string());
                save_editor_data(&editor_data);
                return;
            }
        }
    }

    // fallback: create a default empty project
    let asset = aracari::asset::ParticleSystemAsset {
        name: "New project".to_string(),
        dimension: aracari::asset::ParticleSystemDimension::D3,
        emitters: vec![aracari::asset::EmitterData {
            name: "Emitter 1".to_string(),
            ..Default::default()
        }],
    };
    let handle = assets.add(asset);
    editor_state.current_project = Some(handle);
}

