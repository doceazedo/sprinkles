use aracari::prelude::*;
use bevy::prelude::*;

use crate::state::EditorState;
use crate::ui::widgets::inspector_field::{InspectorFieldProps, fields_row, spawn_inspector_field};
use crate::ui::widgets::vector_edit::VectorSuffixes;

use super::binding::get_inspecting_emitter;
use super::{InspectorSection, inspector_section};

#[derive(Component)]
struct TurbulenceSection;

#[derive(Component)]
struct TurbulenceOptions;

pub fn plugin(app: &mut App) {
    app.add_systems(Update, (setup_turbulence_options, toggle_turbulence_options));
}

pub fn turbulence_section(asset_server: &AssetServer) -> impl Bundle {
    (
        TurbulenceSection,
        inspector_section(
            InspectorSection::new(
                "Turbulence",
                vec![vec![
                    InspectorFieldProps::new("turbulence.enabled").bool().into(),
                ]],
            ),
            asset_server,
        ),
    )
}

fn setup_turbulence_options(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    editor_state: Res<EditorState>,
    assets: Res<Assets<ParticleSystemAsset>>,
    sections: Query<(Entity, &InspectorSection), With<TurbulenceSection>>,
    existing: Query<Entity, With<TurbulenceOptions>>,
) {
    let Ok((entity, section)) = sections.single() else {
        return;
    };

    if !section.initialized || !existing.is_empty() {
        return;
    }

    let enabled = get_inspecting_emitter(&editor_state, &assets)
        .map(|(_, e)| e.turbulence.enabled)
        .unwrap_or(false);

    let display = if enabled {
        Display::Flex
    } else {
        Display::None
    };

    let options = commands
        .spawn((
            TurbulenceOptions,
            Node {
                width: percent(100),
                flex_direction: FlexDirection::Column,
                row_gap: px(12.0),
                display,
                ..default()
            },
        ))
        .with_children(|parent| {
            let rows: Vec<(Vec<InspectorFieldProps>,)> = vec![
                (vec![
                    InspectorFieldProps::new("turbulence.noise_strength"),
                    InspectorFieldProps::new("turbulence.noise_scale"),
                ],),
                (vec![InspectorFieldProps::new("turbulence.noise_speed")
                    .vector(VectorSuffixes::XYZ)],),
                (vec![InspectorFieldProps::new(
                    "turbulence.noise_speed_random",
                )],),
                (vec![InspectorFieldProps::new("turbulence.influence")
                    .vector(VectorSuffixes::Range)],),
                (vec![InspectorFieldProps::new("turbulence.influence_over_lifetime").curve()],),
            ];

            for (fields,) in rows {
                parent.spawn(fields_row()).with_children(|row| {
                    for props in fields {
                        spawn_inspector_field(row, props, &asset_server);
                    }
                });
            }
        })
        .id();

    commands.entity(entity).add_child(options);
}

fn toggle_turbulence_options(
    editor_state: Res<EditorState>,
    assets: Res<Assets<ParticleSystemAsset>>,
    mut options: Query<&mut Node, With<TurbulenceOptions>>,
) {
    let Ok(mut node) = options.single_mut() else {
        return;
    };

    let enabled = get_inspecting_emitter(&editor_state, &assets)
        .map(|(_, e)| e.turbulence.enabled)
        .unwrap_or(false);

    let display = if enabled {
        Display::Flex
    } else {
        Display::None
    };

    if node.display != display {
        node.display = display;
    }
}
