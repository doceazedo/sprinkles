use bevy::prelude::*;

use crate::ui::widgets::inspector_field::InspectorFieldProps;
use crate::ui::widgets::vector_edit::VectorSuffixes;

use super::{InspectorSection, inspector_section};

pub fn plugin(_app: &mut App) {}

pub fn angle_section(asset_server: &AssetServer) -> impl Bundle {
    inspector_section(
        InspectorSection::new(
            "Angle",
            vec![
                vec![
                    InspectorFieldProps::new("angle.range")
                        .vector(VectorSuffixes::Range)
                        .with_label("Initial angle")
                        .with_suffix("°")
                        .with_min(-360.0)
                        .with_max(360.0)
                        .into(),
                ],
                vec![
                    InspectorFieldProps::new("angle.angle_over_lifetime")
                        .curve()
                        .into(),
                ],
            ],
        ),
        asset_server,
    )
}
