use sprinkles::prelude::*;
use bevy::prelude::*;

use crate::state::{DirtyState, EditorState};
use crate::ui::tokens::{FONT_PATH, TEXT_MUTED_COLOR, TEXT_SIZE_SM};
use crate::ui::widgets::combobox::{
    ComboBoxChangeEvent, ComboBoxConfig, ComboBoxOptionData, combobox_with_selected,
};
use crate::ui::widgets::inspector_field::{InspectorFieldProps, fields_row};
use crate::ui::widgets::variant_edit::{VariantDefinition, VariantEditProps};
use crate::ui::widgets::vector_edit::VectorSuffixes;

use super::utils::{VariantConfig, combobox_options_from_reflect, variants_from_reflect};
use super::{InspectorItem, InspectorSection, inspector_section, section_needs_setup};
use crate::ui::components::binding::{
    get_inspecting_emitter, get_inspecting_emitter_mut, mark_dirty_and_restart,
};
use crate::ui::icons::{
    ICON_CONE, ICON_CUBE, ICON_MESH_CYLINDER, ICON_MESH_PLANE, ICON_MESH_UVSPHERE,
};

#[derive(Component)]
struct DrawPassSection;

#[derive(Component)]
struct TransformAlignComboBox;

#[derive(Component)]
struct TransformAlignContent;

pub fn plugin(app: &mut App) {
    app.add_observer(handle_transform_align_change)
        .add_systems(
            Update,
            (setup_transform_align_content, sync_transform_align_ui)
                .after(super::update_inspected_emitter_tracker),
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
                        InspectorFieldProps::new("draw_pass.shadow_caster")
                            .bool()
                            .into(),
                    ],
                ],
            ),
            asset_server,
        ),
    )
}

fn transform_align_index(mode: &Option<TransformAlign>) -> usize {
    match mode {
        None => 0,
        Some(TransformAlign::YToVelocity) => 1,
        Some(TransformAlign::Billboard) => 2,
        Some(TransformAlign::BillboardFixedY) => 3,
        Some(TransformAlign::BillboardYToVelocity) => 4,
    }
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

fn setup_transform_align_content(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    editor_state: Res<EditorState>,
    assets: Res<Assets<ParticleSystemAsset>>,
    sections: Query<(Entity, &InspectorSection), With<DrawPassSection>>,
    existing: Query<Entity, With<TransformAlignContent>>,
) {
    let Some(entity) = section_needs_setup(&sections, &existing) else {
        return;
    };

    let emitter = get_inspecting_emitter(&editor_state, &assets).map(|(_, e)| e);
    let mode = emitter.map(|e| &e.draw_pass.transform_align);
    let mode_index = mode.map(transform_align_index).unwrap_or(0);

    let font: Handle<Font> = asset_server.load(FONT_PATH);

    let content = commands
        .spawn((
            TransformAlignContent,
            Node {
                width: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(12.0),
                ..default()
            },
        ))
        .with_children(|parent| {
            parent.spawn(fields_row()).with_children(|row| {
                row.spawn(Node {
                    flex_direction: FlexDirection::Column,
                    row_gap: Val::Px(3.0),
                    flex_grow: 1.0,
                    flex_shrink: 1.0,
                    flex_basis: Val::Px(0.0),
                    ..default()
                })
                .with_children(|wrapper| {
                    wrapper.spawn((
                        Text::new("Transform align"),
                        TextFont {
                            font: font.clone(),
                            font_size: TEXT_SIZE_SM,
                            weight: FontWeight::MEDIUM,
                            ..default()
                        },
                        TextColor(TEXT_MUTED_COLOR.into()),
                    ));
                    wrapper.spawn((
                        TransformAlignComboBox,
                        combobox_with_selected(transform_align_options(), mode_index),
                    ));
                });
            });
        })
        .id();

    commands.entity(entity).add_child(content);
}

fn handle_transform_align_change(
    trigger: On<ComboBoxChangeEvent>,
    comboboxes: Query<(), With<TransformAlignComboBox>>,
    editor_state: Res<EditorState>,
    mut assets: ResMut<Assets<ParticleSystemAsset>>,
    mut dirty_state: ResMut<DirtyState>,
    mut emitter_runtimes: Query<&mut EmitterRuntime>,
) {
    if comboboxes.get(trigger.entity).is_err() {
        return;
    }

    let Some((_, emitter)) = get_inspecting_emitter_mut(&editor_state, &mut assets) else {
        return;
    };

    let new_mode = match trigger.value.as_deref().unwrap_or(&trigger.label) {
        "Disabled" => None,
        "Billboard" => Some(TransformAlign::Billboard),
        "YToVelocity" => Some(TransformAlign::YToVelocity),
        "BillboardYToVelocity" => Some(TransformAlign::BillboardYToVelocity),
        "BillboardFixedY" => Some(TransformAlign::BillboardFixedY),
        _ => return,
    };

    if transform_align_index(&emitter.draw_pass.transform_align) == transform_align_index(&new_mode)
    {
        return;
    }

    emitter.draw_pass.transform_align = new_mode;
    mark_dirty_and_restart(
        &mut dirty_state,
        &mut emitter_runtimes,
        emitter.time.fixed_seed,
    );
}

fn sync_transform_align_ui(
    editor_state: Res<EditorState>,
    assets: Res<Assets<ParticleSystemAsset>>,
    mut comboboxes: Query<&mut ComboBoxConfig, With<TransformAlignComboBox>>,
) {
    if !editor_state.is_changed() && !assets.is_changed() {
        return;
    }

    let emitter = get_inspecting_emitter(&editor_state, &assets).map(|(_, e)| e);
    let mode = emitter.map(|e| &e.draw_pass.transform_align);
    let new_index = mode.map(transform_align_index).unwrap_or(0);

    for mut config in &mut comboboxes {
        if config.selected != new_index {
            config.selected = new_index;
        }
    }
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
                .default_value(ParticleMesh::Quad {
                    orientation: QuadOrientation::default(),
                    size: Vec2::ONE,
                    subdivide: Vec2::ZERO,
                }),
        ),
        (
            "Sphere",
            VariantConfig::default()
                .icon(ICON_MESH_UVSPHERE)
                .default_value(ParticleMesh::Sphere { radius: 1.0 }),
        ),
        (
            "Cuboid",
            VariantConfig::default()
                .icon(ICON_CUBE)
                .default_value(ParticleMesh::Cuboid {
                    half_size: Vec3::splat(0.5),
                }),
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
                .default_value(ParticleMesh::Cylinder {
                    top_radius: 0.5,
                    bottom_radius: 0.5,
                    height: 1.0,
                    radial_segments: 16,
                    rings: 1,
                    cap_top: true,
                    cap_bottom: true,
                }),
        ),
        (
            "Prism",
            VariantConfig::default()
                .icon(ICON_CONE)
                .override_suffixes("subdivide", VectorSuffixes::WHD)
                .default_value(ParticleMesh::Prism {
                    left_to_right: 0.5,
                    size: Vec3::splat(1.0),
                    subdivide: Vec3::ZERO,
                }),
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
                .override_rows(vec![
                    vec!["base_color", "base_color_texture"],
                    vec!["emissive", "emissive_texture"],
                    vec!["alpha_mode"],
                    vec!["perceptual_roughness"],
                    vec!["metallic"],
                    vec!["reflectance"],
                    vec!["metallic_roughness_texture", "normal_map_texture"],
                    vec!["double_sided"],
                    vec!["unlit"],
                    vec!["fog_enabled"],
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
