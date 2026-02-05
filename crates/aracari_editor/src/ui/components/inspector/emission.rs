use aracari::prelude::*;
use bevy::prelude::*;

use crate::ui::widgets::inspector_field::InspectorFieldProps;
use crate::ui::widgets::variant_edit::{VariantDefinition, VariantEditProps};
use crate::ui::widgets::vector_edit::VectorSuffixes;

use super::utils::{VariantConfig, variants_from_reflect};
use super::{InspectorItem, InspectorSection, inspector_section};

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
                vec![InspectorFieldProps::new("emission.particles_amount").u32().into()],
            ],
        ),
        asset_server,
    )
}

const ICON_POINT: &str = "icons/blender_empty_axis.png";
const ICON_SPHERE: &str = "icons/blender_sphere.png";
const ICON_SPHERE_SURFACE: &str = "icons/blender_mesh_uvsphere.png";
const ICON_BOX: &str = "icons/blender_cube.png";
const ICON_RING: &str = "icons/blender_mesh_torus.png";

fn emission_shape_variants() -> Vec<VariantDefinition> {
    variants_from_reflect::<EmissionShape>(&[
        (
            "Point",
            VariantConfig::default()
                .icon(ICON_POINT)
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
                .icon(ICON_SPHERE_SURFACE)
                .default_value(EmissionShape::SphereSurface { radius: 1.0 }),
        ),
        (
            "Box",
            VariantConfig::default()
                .icon(ICON_BOX)
                .default_value(EmissionShape::Box { extents: Vec3::ONE }),
        ),
        (
            "Ring",
            VariantConfig::default()
                .icon(ICON_RING)
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
