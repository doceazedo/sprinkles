use bevy::prelude::*;
use bevy_sprinkles::prelude::*;

use crate::state::EditorState;
use crate::ui::components::binding::{FieldBinding, get_inspecting_emitter};
use crate::ui::widgets::alert::{AlertSpan, AlertVariant, alert};
use crate::ui::widgets::combobox::ComboBoxOptionData;
use crate::ui::widgets::inspector_field::InspectorFieldProps;
use crate::ui::widgets::text_edit::{TextEditProps, text_edit};
use crate::ui::widgets::utils::find_ancestor;
use crate::ui::widgets::variant_edit::{
    VariantDefinition, VariantEditProps, VariantFieldsContainer,
};
use crate::ui::widgets::vector_edit::VectorSuffixes;

use super::types::{FieldKind, VariantField};
use super::utils::{VariantConfig, combobox_options_from_reflect, variants_from_reflect};
use super::{InspectorItem, InspectorSection, inspector_section};
use crate::ui::icons::{
    ICON_CONE, ICON_CUBE, ICON_MESH_CYLINDER, ICON_MESH_PLANE, ICON_MESH_UVSPHERE,
};

#[derive(Component)]
struct MaskCutoffRow;

#[derive(Component)]
pub struct TrailMeshAlert;

#[derive(Component)]
struct DrawPassSection;

pub fn plugin(app: &mut App) {
    app.add_systems(
        Update,
        (sync_mask_cutoff, sync_trail_mesh_alert).after(super::update_inspected_emitter_tracker),
    );
}

pub fn draw_pass_section(asset_server: &AssetServer) -> impl Bundle {
    (
        DrawPassSection,
        inspector_section(
            InspectorSection::new(
                "Draw pass",
                vec![
                    vec![
                        InspectorItem::Variant {
                            path: "draw_pass.mesh".into(),
                            props: VariantEditProps::new("draw_pass.mesh")
                                .with_variants(mesh_variants()),
                        },
                        InspectorItem::Variant {
                            path: "draw_pass.material".into(),
                            props: VariantEditProps::new("draw_pass.material")
                                .with_variants(material_variants()),
                        },
                    ],
                    vec![
                        InspectorFieldProps::new("draw_pass.draw_order")
                            .combobox(combobox_options_from_reflect::<DrawOrder>())
                            .into(),
                    ],
                    vec![
                        InspectorFieldProps::new("draw_pass.transform_align")
                            .optional_combobox(transform_align_options())
                            .into(),
                    ],
                    vec![
                        InspectorFieldProps::new("draw_pass.shadow_caster")
                            .bool()
                            .into(),
                    ],
                    vec![
                        InspectorFieldProps::new("draw_pass.use_local_coords")
                            .bool()
                            .into(),
                    ],
                ],
            ),
            asset_server,
        ),
    )
}

fn transform_align_options() -> Vec<ComboBoxOptionData> {
    vec![
        ComboBoxOptionData::new("Disabled").with_value("Disabled"),
        ComboBoxOptionData::new("Y to velocity").with_value("YToVelocity"),
        ComboBoxOptionData::new("Billboard").with_value("Billboard"),
        ComboBoxOptionData::new("Billboard (Fixed Y)").with_value("BillboardFixedY"),
        ComboBoxOptionData::new("Billboard (Y to velocity)").with_value("BillboardYToVelocity"),
    ]
}

fn mesh_variants() -> Vec<VariantDefinition> {
    variants_from_reflect::<ParticleMesh>(&[
        (
            "Quad",
            VariantConfig::default()
                .icon(ICON_MESH_PLANE)
                .override_combobox::<QuadOrientation>("orientation")
                .override_suffixes("size", VectorSuffixes::XY)
                .override_suffixes("subdivide", VectorSuffixes::WD)
                .default_value(ParticleMesh::default_quad()),
        ),
        (
            "Sphere",
            VariantConfig::default()
                .icon(ICON_MESH_UVSPHERE)
                .override_rows(vec![vec!["radius"], vec!["segments", "rings"]])
                .default_value(ParticleMesh::default_sphere()),
        ),
        (
            "Cuboid",
            VariantConfig::default()
                .icon(ICON_CUBE)
                .default_value(ParticleMesh::default_cuboid()),
        ),
        (
            "Cylinder",
            VariantConfig::default()
                .icon(ICON_MESH_CYLINDER)
                .override_rows(vec![
                    vec!["top_radius", "bottom_radius"],
                    vec!["height"],
                    vec!["radial_segments", "rings"],
                    vec!["cap_top"],
                    vec!["cap_bottom"],
                ])
                .default_value(ParticleMesh::default_cylinder()),
        ),
        (
            "Prism",
            VariantConfig::default()
                .icon(ICON_CONE)
                .override_suffixes("subdivide", VectorSuffixes::WHD)
                .default_value(ParticleMesh::default_prism()),
        ),
        (
            "TubeTrail",
            VariantConfig::default()
                .icon(ICON_MESH_CYLINDER)
                .override_rows(vec![
                    vec!["radius", "radial_steps"],
                    vec!["sections", "section_rings"],
                ])
                .default_value(ParticleMesh::default_tube_trail()),
        ),
        (
            "RibbonTrail",
            VariantConfig::default()
                .icon(ICON_MESH_PLANE)
                .override_combobox::<RibbonTrailShape>("shape")
                .override_rows(vec![
                    vec!["size"],
                    vec!["sections", "section_rings"],
                    vec!["shape"],
                ])
                .default_value(ParticleMesh::default_ribbon_trail()),
        ),
    ])
}

fn material_variants() -> Vec<VariantDefinition> {
    variants_from_reflect::<DrawPassMaterial>(&[
        (
            "Standard",
            VariantConfig::default()
                .fields_from::<StandardParticleMaterial>()
                .override_combobox::<SerializableAlphaMode>("alpha_mode")
                .override_optional_combobox::<SerializableFace>("cull_mode")
                .override_field(
                    "perceptual_roughness",
                    VariantField::f32("perceptual_roughness")
                        .with_min(0.089)
                        .with_max(1.0),
                )
                .override_field("metallic", VariantField::percent("metallic"))
                .override_field("reflectance", VariantField::percent("reflectance"))
                .override_field(
                    "attenuation_distance",
                    VariantField::new("attenuation_distance").with_kind(FieldKind::F32OrInfinity),
                )
                .override_rows(vec![
                    vec!["base_color", "base_color_texture"],
                    vec!["emissive", "emissive_texture"],
                    vec!["emissive_exposure_weight"],
                    vec!["alpha_mode"],
                    vec!["perceptual_roughness"],
                    vec!["metallic"],
                    vec!["reflectance"],
                    vec!["metallic_roughness_texture"],
                    vec!["normal_map_texture"],
                    vec!["flip_normal_map_y"],
                    vec!["occlusion_texture"],
                    vec!["specular_tint"],
                    vec!["diffuse_transmission"],
                    vec!["specular_transmission"],
                    vec!["thickness"],
                    vec!["ior"],
                    vec!["attenuation_distance"],
                    vec!["attenuation_color"],
                    vec!["clearcoat"],
                    vec!["clearcoat_perceptual_roughness"],
                    vec!["anisotropy_strength", "anisotropy_rotation"],
                    vec!["double_sided"],
                    vec!["cull_mode"],
                    vec!["unlit"],
                    vec!["fog_enabled"],
                    vec!["depth_bias"],
                ])
                .default_value(DrawPassMaterial::Standard(
                    StandardParticleMaterial::default(),
                )),
        ),
        (
            "CustomShader",
            VariantConfig::default().default_value(DrawPassMaterial::CustomShader {
                vertex_shader: None,
                fragment_shader: None,
            }),
        ),
    ])
}

fn find_ancestor_child_of(
    entity: Entity,
    target_parent: Entity,
    parents: &Query<&ChildOf>,
) -> Option<Entity> {
    let mut current = entity;
    for _ in 0..20 {
        let parent = parents.get(current).ok()?.parent();
        if parent == target_parent {
            return Some(current);
        }
        current = parent;
    }
    None
}

fn extract_mask_cutoff(editor_state: &EditorState, assets: &Assets<ParticlesAsset>) -> Option<f32> {
    let (_, emitter) = get_inspecting_emitter(editor_state, assets)?;
    let DrawPassMaterial::Standard(mat) = &emitter.draw_pass.material else {
        return None;
    };
    let SerializableAlphaMode::Mask { cutoff } = mat.alpha_mode else {
        return None;
    };
    Some(cutoff)
}

fn find_insertion_point(
    bindings: &Query<(Entity, &FieldBinding)>,
    parents: &Query<&ChildOf>,
    children_query: &Query<&Children>,
    containers: &Query<&VariantFieldsContainer>,
) -> Option<(Entity, usize, Entity)> {
    let (alpha_entity, _) = bindings
        .iter()
        .find(|(_, b)| b.path() == "draw_pass.material" && b.field_name() == Some("alpha_mode"))?;

    let (container, _) = find_ancestor(alpha_entity, containers, parents)?;
    let row = find_ancestor_child_of(alpha_entity, container, parents)?;
    let container_children = children_query.get(container).ok()?;
    let row_index = container_children.iter().position(|c| c == row)?;

    Some((container, row_index, alpha_entity))
}

fn spawn_cutoff_row(
    commands: &mut Commands,
    cutoff: f32,
    alpha_binding: &FieldBinding,
    container: Entity,
    row_index: usize,
) {
    let cutoff_binding = if let Some(ve) = alpha_binding.variant_edit {
        FieldBinding::emitter_variant(
            "draw_pass.material",
            "alpha_mode.cutoff",
            FieldKind::F32,
            ve,
        )
    } else {
        FieldBinding::emitter_variant_field(
            "draw_pass.material",
            "alpha_mode.cutoff",
            FieldKind::F32,
        )
    };

    let cutoff_row = commands
        .spawn((
            MaskCutoffRow,
            Node {
                width: Val::Percent(100.0),
                column_gap: Val::Px(8.0),
                ..default()
            },
        ))
        .with_child((
            cutoff_binding,
            text_edit(
                TextEditProps::default()
                    .with_label("Cutoff")
                    .with_default_value(crate::ui::components::binding::format_f32(cutoff))
                    .numeric_f32()
                    .with_min(0.0)
                    .with_max(1.0),
            ),
        ))
        .id();

    commands
        .entity(container)
        .insert_children(row_index + 1, &[cutoff_row]);
}

fn sync_mask_cutoff(
    mut commands: Commands,
    editor_state: Res<EditorState>,
    assets: Res<Assets<ParticlesAsset>>,
    existing: Query<Entity, With<MaskCutoffRow>>,
    bindings: Query<(Entity, &FieldBinding)>,
    parents: Query<&ChildOf>,
    children_query: Query<&Children>,
    containers: Query<&VariantFieldsContainer>,
) {
    if !editor_state.is_changed() && !assets.is_changed() {
        return;
    }

    let cutoff_value = extract_mask_cutoff(&editor_state, &assets);
    let has_row = !existing.is_empty();

    match (cutoff_value, has_row) {
        (Some(cutoff), false) => {
            let Some((container, row_index, alpha_entity)) =
                find_insertion_point(&bindings, &parents, &children_query, &containers)
            else {
                return;
            };
            let Some((_, alpha_binding)) = bindings.iter().find(|(e, _)| *e == alpha_entity) else {
                return;
            };
            spawn_cutoff_row(&mut commands, cutoff, alpha_binding, container, row_index);
        }
        (None, true) => {
            for entity in &existing {
                commands.entity(entity).try_despawn();
            }
        }
        _ => {}
    }
}

fn sync_trail_mesh_alert(
    mut commands: Commands,
    editor_state: Res<EditorState>,
    assets: Res<Assets<ParticlesAsset>>,
    sections: Query<(Entity, &InspectorSection, &Children), With<DrawPassSection>>,
    existing: Query<Entity, With<TrailMeshAlert>>,
    mut alert_nodes: Query<&mut Node, With<TrailMeshAlert>>,
    new_alerts: Query<Entity, Added<TrailMeshAlert>>,
) {
    if existing.is_empty() {
        if let Ok((section_entity, _, children)) = sections.single() {
            if children.len() > 1 {
                let alert_entity = commands
                    .spawn((
                        TrailMeshAlert,
                        Node {
                            width: Val::Percent(100.0),
                            display: Display::None,
                            ..default()
                        },
                    ))
                    .with_child(alert(
                        AlertVariant::Warning,
                        vec![
                            AlertSpan::Text("You need to enable ".into()),
                            AlertSpan::Bold("Trail".into()),
                            AlertSpan::Text(" to use this mesh correctly.".into()),
                        ],
                    ))
                    .id();
                commands
                    .entity(section_entity)
                    .insert_children(2, &[alert_entity]);
            }
        }
    }

    if !editor_state.is_changed() && !assets.is_changed() && new_alerts.is_empty() {
        return;
    }

    let should_show = get_inspecting_emitter(&editor_state, &assets)
        .map(|(_, e)| e.draw_pass.mesh.is_trail() && !e.trail.enabled)
        .unwrap_or(false);

    for mut node in &mut alert_nodes {
        super::set_display_visible(&mut node, should_show);
    }
}
