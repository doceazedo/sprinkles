use aracari::prelude::*;
use bevy::prelude::*;

use crate::ui::widgets::combobox::ComboBoxOptionData;
use crate::ui::widgets::inspector_field::{InspectorFieldProps, fields_row, spawn_inspector_field};
use crate::ui::widgets::panel_section::{PanelSectionProps, PanelSectionSize, panel_section};
use crate::ui::widgets::variant_edit::{
    VariantDefinition, VariantEditProps, VariantField, variant_edit,
};
use crate::ui::widgets::vector_edit::VectorSuffixes;

use super::binding::Field;

pub fn plugin(app: &mut App) {
    app.add_systems(Update, setup_draw_pass_section_fields);
}

#[derive(Component)]
pub struct DrawPassSection;

#[derive(Component)]
struct DrawPassSectionInitialized;

pub fn draw_pass_section(asset_server: &AssetServer) -> impl Bundle {
    (
        DrawPassSection,
        panel_section(
            PanelSectionProps::new("Draw pass")
                .collapsible()
                .with_size(PanelSectionSize::XL),
            asset_server,
        ),
    )
}

const ICON_MESH_QUAD: &str = "icons/blender_mesh_plane.png";
const ICON_MESH_SPHERE: &str = "icons/blender_mesh_uvsphere.png";
const ICON_MESH_CUBOID: &str = "icons/blender_cube.png";
const ICON_MESH_CYLINDER: &str = "icons/blender_mesh_cylinder.png";
const ICON_MESH_PRISM: &str = "icons/blender_cone.png";

fn mesh_variants() -> Vec<VariantDefinition> {
    vec![
        VariantDefinition::new("Quad")
            .with_icon(ICON_MESH_QUAD)
            .with_fields(vec![VariantField::combobox(
                "orientation",
                vec!["Face X", "Face Y", "Face Z"],
            )])
            .with_default(ParticleMesh::Quad {
                orientation: QuadOrientation::default(),
            }),
        VariantDefinition::new("Sphere")
            .with_icon(ICON_MESH_SPHERE)
            .with_fields(vec![VariantField::f32("radius")])
            .with_default(ParticleMesh::Sphere { radius: 1.0 }),
        VariantDefinition::new("Cuboid")
            .with_icon(ICON_MESH_CUBOID)
            .with_fields(vec![VariantField::vec3("half_size", VectorSuffixes::XYZ)])
            .with_default(ParticleMesh::Cuboid {
                half_size: Vec3::splat(0.5),
            }),
        VariantDefinition::new("Cylinder")
            .with_icon(ICON_MESH_CYLINDER)
            .with_fields(vec![
                VariantField::f32("top_radius"),
                VariantField::f32("bottom_radius"),
                VariantField::f32("height"),
                VariantField::u32("radial_segments"),
                VariantField::u32("rings"),
                VariantField::bool("cap_top"),
                VariantField::bool("cap_bottom"),
            ])
            .with_default(ParticleMesh::Cylinder {
                top_radius: 0.5,
                bottom_radius: 0.5,
                height: 1.0,
                radial_segments: 16,
                rings: 1,
                cap_top: true,
                cap_bottom: true,
            }),
        VariantDefinition::new("Prism")
            .with_icon(ICON_MESH_PRISM)
            .with_fields(vec![
                VariantField::f32("left_to_right"),
                VariantField::vec3("size", VectorSuffixes::XYZ),
                VariantField::vec3("subdivide", VectorSuffixes::WHD),
            ])
            .with_default(ParticleMesh::Prism {
                left_to_right: 0.5,
                size: Vec3::splat(1.0),
                subdivide: Vec3::ZERO,
            }),
    ]
}

fn material_variants() -> Vec<VariantDefinition> {
    vec![
        VariantDefinition::new("Standard")
            .with_fields(vec![
                VariantField::f32("perceptual_roughness"),
                VariantField::f32("metallic"),
                VariantField::f32("reflectance"),
                VariantField::combobox(
                    "alpha_mode",
                    vec![
                        "Opaque",
                        "Mask",
                        "Blend",
                        "Premultiplied",
                        "Add",
                        "Multiply",
                        "Alpha To Coverage",
                    ],
                ),
                VariantField::bool("double_sided"),
                VariantField::bool("unlit"),
                VariantField::bool("fog_enabled"),
            ])
            .with_default(DrawPassMaterial::Standard(StandardParticleMaterial::default())),
        VariantDefinition::new("CustomShader").with_default(DrawPassMaterial::CustomShader {
            vertex_shader: None,
            fragment_shader: None,
        }),
    ]
}

fn draw_order_options() -> Vec<ComboBoxOptionData> {
    vec![
        ComboBoxOptionData::new("Index"),
        ComboBoxOptionData::new("Lifetime"),
        ComboBoxOptionData::new("Reverse Lifetime"),
        ComboBoxOptionData::new("View Depth"),
    ]
}

fn setup_draw_pass_section_fields(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    sections: Query<Entity, (With<DrawPassSection>, Without<DrawPassSectionInitialized>)>,
) {
    for entity in &sections {
        commands
            .entity(entity)
            .insert(DrawPassSectionInitialized)
            .with_children(|parent| {
                // row 1: mesh and material
                parent.spawn(fields_row()).with_children(|row| {
                    row.spawn((
                        Field::new("draw_pass.mesh"),
                        variant_edit(
                            VariantEditProps::new("draw_pass.mesh")
                                .with_variants(mesh_variants()),
                        ),
                    ));

                    row.spawn((
                        Field::new("draw_pass.material"),
                        variant_edit(
                            VariantEditProps::new("draw_pass.material")
                                .with_variants(material_variants()),
                        ),
                    ));
                });

                // row 2: draw order
                parent.spawn(fields_row()).with_children(|row| {
                    spawn_inspector_field(
                        row,
                        InspectorFieldProps::new("draw_pass.draw_order")
                            .combobox(draw_order_options()),
                        &asset_server,
                    );
                });

                // row 3: shadow caster
                parent.spawn(fields_row()).with_children(|row| {
                    spawn_inspector_field(
                        row,
                        InspectorFieldProps::new("draw_pass.shadow_caster").bool(),
                        &asset_server,
                    );
                });
            });
    }
}
