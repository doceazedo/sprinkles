use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts, EguiPlugin, EguiPrimaryContextPass};
use bevy_starling::StarlingPlugin;

use crate::state::{load_editor_data, EditorData, EditorState};
use crate::ui::modals::{draw_new_project_modal, on_create_project_event, NewProjectModal};
use crate::ui::{configure_style, draw_inspector, draw_topbar};
use crate::viewport::{draw_grid, orbit_camera, setup_camera, OrbitCameraSettings};

pub struct StarlingEditorPlugin;

impl Plugin for StarlingEditorPlugin {
    fn build(&self, app: &mut App) {
        let editor_data = load_editor_data();

        app.add_plugins(StarlingPlugin)
            .add_plugins(EguiPlugin::default())
            .init_resource::<EditorState>()
            .init_resource::<OrbitCameraSettings>()
            .init_resource::<NewProjectModal>()
            .insert_resource(editor_data)
            .insert_resource(EguiConfigured(false))
            .add_observer(on_create_project_event)
            .add_systems(Startup, (setup_camera, setup_light, load_initial_project))
            .add_systems(Update, (orbit_camera, draw_grid))
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

fn setup_light(mut commands: Commands) {
    commands.spawn((
        DirectionalLight {
            shadows_enabled: true,
            ..default()
        },
        Transform::from_xyz(4.0, 8.0, 4.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
}

fn load_initial_project(
    mut editor_state: ResMut<EditorState>,
    editor_data: Res<EditorData>,
    asset_server: Res<AssetServer>,
) {
    if let Some(path) = &editor_data.cache.last_opened_project {
        if path.exists() {
            editor_state.current_project = Some(asset_server.load(path.clone()));
            editor_state.current_project_path = Some(path.clone());
            return;
        }
    }

    editor_state.current_project = Some(asset_server.load("demo.starling"));
}
