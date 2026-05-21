use bevy::prelude::*;
use bevy_sprinkles::prelude::*;

use crate::state::EditorState;
use crate::ui::icons::ICON_TIME;
use crate::ui::widgets::alert::{AlertSpan, AlertVariant, alert};
use crate::ui::widgets::inspector_field::{InspectorFieldProps, fields_row, spawn_inspector_field};

use super::{InspectorSection, inspector_section, section_needs_setup};
use crate::ui::components::binding::get_inspecting_emitter;

#[derive(Component)]
struct TrailSection;

#[derive(Component)]
struct TrailOptions;

#[derive(Component)]
struct TrailNoMeshAlert;

pub fn plugin(app: &mut App) {
    app.add_systems(
        Update,
        (
            setup_trail_options,
            toggle_trail_options,
            sync_trail_no_mesh_alert,
        ),
    );
}

pub fn trail_section(asset_server: &AssetServer) -> impl Bundle {
    (
        TrailSection,
        inspector_section(
            InspectorSection::new(
                "Trail",
                vec![vec![
                    InspectorFieldProps::new("trail.enabled").bool().into(),
                ]],
            ),
            asset_server,
        ),
    )
}

fn setup_trail_options(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    editor_state: Res<EditorState>,
    assets: Res<Assets<ParticlesAsset>>,
    sections: Query<(Entity, &InspectorSection), With<TrailSection>>,
    existing: Query<Entity, With<TrailOptions>>,
) {
    let Some(entity) = section_needs_setup(&sections, &existing) else {
        return;
    };

    let enabled = get_inspecting_emitter(&editor_state, &assets)
        .map(|(_, e)| e.trail.enabled)
        .unwrap_or(false);

    let display = if enabled {
        Display::Flex
    } else {
        Display::None
    };

    let options = commands
        .spawn((
            TrailOptions,
            Node {
                width: percent(100),
                flex_direction: FlexDirection::Column,
                row_gap: px(12.0),
                display,
                ..default()
            },
        ))
        .with_children(|parent| {
            let rows = vec![vec![
                InspectorFieldProps::new("trail.stretch_time")
                    .with_icon(ICON_TIME)
                    .with_suffix("s"),
                InspectorFieldProps::new("trail.thickness_curve").curve(),
            ]];

            for fields in rows {
                parent.spawn(fields_row()).with_children(|row| {
                    for props in fields {
                        spawn_inspector_field(row, props, &asset_server);
                    }
                });
            }

            parent
                .spawn((
                    TrailNoMeshAlert,
                    Node {
                        width: percent(100),
                        display: Display::None,
                        ..default()
                    },
                ))
                .with_child(alert(
                    AlertVariant::Warning,
                    vec![
                        AlertSpan::Text("Select a different ".into()),
                        AlertSpan::Bold("Mesh".into()),
                        AlertSpan::Text(" for trails to work correctly.".into()),
                    ],
                ));
        })
        .id();

    commands.entity(entity).add_child(options);
}

fn toggle_trail_options(
    editor_state: Res<EditorState>,
    assets: Res<Assets<ParticlesAsset>>,
    mut options: Query<&mut Node, With<TrailOptions>>,
) {
    let Ok(mut node) = options.single_mut() else {
        return;
    };

    let enabled = get_inspecting_emitter(&editor_state, &assets)
        .map(|(_, e)| e.trail.enabled)
        .unwrap_or(false);

    super::set_display_visible(&mut node, enabled);
}

fn sync_trail_no_mesh_alert(
    editor_state: Res<EditorState>,
    assets: Res<Assets<ParticlesAsset>>,
    mut alert_nodes: Query<&mut Node, With<TrailNoMeshAlert>>,
    new_alerts: Query<Entity, Added<TrailNoMeshAlert>>,
) {
    if !editor_state.is_changed() && !assets.is_changed() && new_alerts.is_empty() {
        return;
    }

    let should_show = get_inspecting_emitter(&editor_state, &assets)
        .map(|(_, e)| {
            e.trail.enabled
                && !matches!(
                    e.draw_pass.mesh,
                    ParticleMesh::TubeTrail { .. } | ParticleMesh::RibbonTrail { .. }
                )
        })
        .unwrap_or(false);

    for mut node in &mut alert_nodes {
        super::set_display_visible(&mut node, should_show);
    }
}
