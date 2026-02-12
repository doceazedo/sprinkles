use bevy::picking::prelude::Pickable;
use bevy::prelude::*;
use bevy_sprinkles::prelude::*;

use crate::state::EditorState;
use crate::ui::components::binding::{FieldBinding, get_inspecting_emitter};
use crate::ui::tokens::BACKGROUND_COLOR;
use crate::ui::widgets::alert::{AlertSpan, AlertVariant, alert};
use crate::ui::widgets::inspector_field::InspectorFieldProps;
use crate::ui::widgets::variant_edit::{VariantDefinition, VariantEditProps};

use super::utils::VariantConfig;
use super::{InspectorItem, InspectorSection, inspector_section, section_needs_setup};

#[derive(Component)]
struct ColorsSection;

#[derive(Component)]
struct AlphaOpaqueAlert;

#[derive(Component)]
struct AlphaDisabledOverlay;

pub fn plugin(app: &mut App) {
    app.add_systems(
        Update,
        (setup_alpha_alert, sync_alpha_disabled).after(super::update_inspected_emitter_tracker),
    );
}

fn color_variants() -> Vec<VariantDefinition> {
    super::utils::variants_from_reflect::<SolidOrGradientColor>(&[
        (
            "Solid",
            VariantConfig::default().default_value(SolidOrGradientColor::Solid {
                color: [1.0, 1.0, 1.0, 1.0],
            }),
        ),
        (
            "Gradient",
            VariantConfig::default().default_value(SolidOrGradientColor::Gradient {
                gradient: ParticleGradient::default(),
            }),
        ),
    ])
}

pub fn colors_section(asset_server: &AssetServer) -> impl Bundle {
    (
        ColorsSection,
        inspector_section(
            InspectorSection::new(
                "Colors",
                vec![
                    vec![
                        InspectorItem::Variant {
                            path: "colors.initial_color".into(),
                            props: VariantEditProps::new("colors.initial_color")
                                .with_variants(color_variants())
                                .with_swatch_slot(true),
                        },
                        InspectorFieldProps::new("colors.color_over_lifetime")
                            .gradient()
                            .into(),
                    ],
                    vec![
                        InspectorFieldProps::new("colors.alpha_over_lifetime")
                            .curve()
                            .into(),
                        InspectorFieldProps::new("colors.emission_over_lifetime")
                            .curve()
                            .into(),
                    ],
                ],
            ),
            asset_server,
        ),
    )
}

fn setup_alpha_alert(
    mut commands: Commands,
    sections: Query<(Entity, &InspectorSection), With<ColorsSection>>,
    existing: Query<Entity, With<AlphaOpaqueAlert>>,
) {
    let Some(entity) = section_needs_setup(&sections, &existing) else {
        return;
    };

    let alert_entity = commands
        .spawn((
            AlphaOpaqueAlert,
            Node {
                width: percent(100),
                display: Display::None,
                ..default()
            },
        ))
        .with_child(alert(
            AlertVariant::Warning,
            vec![
                AlertSpan::Text("To use ".into()),
                AlertSpan::Bold("Alpha over lifetime".into()),
                AlertSpan::Text(", set the material to a different ".into()),
                AlertSpan::Bold("Alpha mode".into()),
                AlertSpan::Text(".".into()),
            ],
        ))
        .id();

    commands.entity(entity).add_child(alert_entity);
}

fn sync_alpha_disabled(
    mut commands: Commands,
    editor_state: Res<EditorState>,
    assets: Res<Assets<ParticleSystemAsset>>,
    mut alert_nodes: Query<&mut Node, With<AlphaOpaqueAlert>>,
    new_alerts: Query<Entity, Added<AlphaOpaqueAlert>>,
    fields: Query<(Entity, &FieldBinding)>,
    overlays: Query<Entity, With<AlphaDisabledOverlay>>,
) {
    if !editor_state.is_changed() && !assets.is_changed() && new_alerts.is_empty() {
        return;
    }

    let is_opaque = get_inspecting_emitter(&editor_state, &assets)
        .map(|(_, emitter)| {
            matches!(
                emitter.draw_pass.material,
                DrawPassMaterial::Standard(ref mat)
                    if matches!(mat.alpha_mode, SerializableAlphaMode::Opaque)
            )
        })
        .unwrap_or(false);

    for mut node in &mut alert_nodes {
        let display = if is_opaque {
            Display::Flex
        } else {
            Display::None
        };
        if node.display != display {
            node.display = display;
        }
    }

    let alpha_field = fields
        .iter()
        .find(|(_, f)| f.path() == "colors.alpha_over_lifetime")
        .map(|(e, _)| e);

    let Some(field_entity) = alpha_field else {
        return;
    };

    let has_overlay = !overlays.is_empty();

    if is_opaque && !has_overlay {
        let overlay = commands
            .spawn((
                AlphaDisabledOverlay,
                Pickable::default(),
                Button,
                Node {
                    position_type: PositionType::Absolute,
                    width: percent(100),
                    height: percent(100),
                    ..default()
                },
                BackgroundColor(BACKGROUND_COLOR.with_alpha(0.7).into()),
            ))
            .id();
        commands.entity(field_entity).add_child(overlay);
    } else if !is_opaque && has_overlay {
        for entity in &overlays {
            commands.entity(entity).try_despawn();
        }
    }
}
