use bevy::prelude::*;
use bevy_sprinkles::prelude::*;

use crate::ui::widgets::inspector_field::InspectorFieldProps;
use crate::ui::widgets::variant_edit::{VariantDefinition, VariantEditProps};
use crate::ui::widgets::vector_edit::VectorSuffixes;

use super::utils::{VariantConfig, variants_from_reflect};
use super::{InspectorItem, InspectorSection, inspector_section};
use crate::ui::icons::{
    ICON_CUBE, ICON_EMPTY_AXIS, ICON_MESH_TORUS, ICON_MESH_UVSPHERE, ICON_SPHERE,
};

pub fn plugin(_app: &mut App) {}

pub fn emission_section(asset_server: &AssetServer) -> impl Bundle {
    inspector_section(
        InspectorSection::new(
            "Emission",
            vec![
                vec![
                    InspectorFieldProps::new("emission.offset")
                        .vector(VectorSuffixes::XYZ)
                        .into(),
                ],
                vec![
                    InspectorFieldProps::new("emission.scale")
                        .vector(VectorSuffixes::XYZ)
                        .into(),
                ],
                vec![InspectorItem::Variant {
                    path: "emission.shape".into(),
                    props: VariantEditProps::new("emission.shape")
                        .with_variants(emission_shape_variants()),
                }],
                vec![
                    InspectorFieldProps::new("emission.particles_amount")
                        .u32()
                        .into(),
                ],
            ],
        ),
        asset_server,
    )
}

fn emission_shape_variants() -> Vec<VariantDefinition> {
    variants_from_reflect::<EmissionShape>(&[
        (
            "Point",
            VariantConfig::default()
                .icon(ICON_EMPTY_AXIS)
                .default_value(EmissionShape::Point),
        ),
        (
            "Sphere",
            VariantConfig::default()
                .icon(ICON_SPHERE)
                .default_value(EmissionShape::Sphere { radius: 1.0 }),
        ),
        (
            "SphereSurface",
            VariantConfig::default()
                .icon(ICON_MESH_UVSPHERE)
                .default_value(EmissionShape::SphereSurface { radius: 1.0 }),
        ),
        (
            "Box",
            VariantConfig::default()
                .icon(ICON_CUBE)
                .default_value(EmissionShape::Box { extents: Vec3::ONE }),
        ),
        (
            "Ring",
            VariantConfig::default()
                .icon(ICON_MESH_TORUS)
                .override_rows(vec![
                    vec!["axis"],
                    vec!["height"],
                    vec!["radius", "inner_radius"],
                ])
                .default_value(EmissionShape::Ring {
                    axis: Vec3::Y,
                    height: 0.0,
                    radius: 1.0,
                    inner_radius: 0.0,
                }),
        ),
    ])
}
