use aracari::prelude::*;
use bevy::prelude::*;

use crate::ui::widgets::inspector_field::InspectorFieldProps;
use crate::ui::widgets::variant_edit::{VariantDefinition, VariantEditProps};

use super::utils::VariantConfig;
use super::{InspectorItem, InspectorSection, inspector_section};

pub fn plugin(_app: &mut App) {}

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
    inspector_section(
        InspectorSection::new(
            "Colors",
            vec![
                vec![InspectorItem::Variant {
                    path: "colors.initial_color".into(),
                    props: VariantEditProps::new("colors.initial_color")
                        .with_variants(color_variants())
                        .with_swatch_slot(true),
                }],
                vec![
                    InspectorFieldProps::new("colors.alpha_curve")
                        .curve()
                        .with_label("Alpha Curve")
                        .into(),
                    InspectorFieldProps::new("colors.emission_curve")
                        .curve()
                        .with_label("Emission Curve")
                        .into(),
                ],
            ],
        ),
        asset_server,
    )
}
