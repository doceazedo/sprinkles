use aracari::prelude::*;
use bevy::prelude::*;

use crate::ui::widgets::inspector_field::InspectorFieldProps;
use crate::ui::widgets::variant_edit::{VariantDefinition, VariantEditProps};
use crate::ui::widgets::vector_edit::VectorSuffixes;

use super::utils::{VariantConfig, combobox_options_from_reflect, variants_from_reflect};
use super::{InspectorItem, InspectorSection, inspector_section};
use crate::ui::icons::{
    ICON_CONE, ICON_CUBE, ICON_MESH_CYLINDER, ICON_MESH_PLANE, ICON_MESH_UVSPHERE,
};

pub fn plugin(_app: &mut App) {}

pub fn draw_pass_section(asset_server: &AssetServer) -> impl Bundle {
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
    )
}

fn mesh_variants() -> Vec<VariantDefinition> {
    variants_from_reflect::<ParticleMesh>(&[
        (
            "Quad",
            VariantConfig::default()
                .icon(ICON_MESH_PLANE)
                .override_combobox::<QuadOrientation>("orientation")
                .default_value(ParticleMesh::Quad {
                    orientation: QuadOrientation::default(),
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
