use bevy::prelude::*;

use crate::ui::widgets::inspector_field::InspectorFieldProps;
use crate::ui::widgets::vector_edit::VectorSuffixes;

use super::{InspectorSection, inspector_section};

pub fn plugin(_app: &mut App) {}

pub fn scale_section(asset_server: &AssetServer) -> impl Bundle {
    inspector_section(
        InspectorSection::new(
            "Scale",
            vec![
                vec![
                    InspectorFieldProps::new("scale.range")
                        .vector(VectorSuffixes::Range)
                        .with_label("Initial scale ratio")
                        .into(),
                ],
                vec![
                    InspectorFieldProps::new("scale.scale_over_lifetime")
                        .curve()
                        .with_label("Scale Over Lifetime")
                        .into(),
                ],
            ],
        ),
        asset_server,
    )
}
