use bevy::prelude::*;

use crate::ui::widgets::inspector_field::InspectorFieldProps;
use crate::ui::widgets::vector_edit::VectorSuffixes;

use super::{InspectorSection, inspector_section};

pub fn plugin(_app: &mut App) {}

pub fn accelerations_section(asset_server: &AssetServer) -> impl Bundle {
    inspector_section(
        InspectorSection::new(
            "Accelerations",
            vec![vec![
                InspectorFieldProps::new("accelerations.gravity")
                    .vector(VectorSuffixes::XYZ)
                    .into(),
            ]],
        ),
        asset_server,
    )
}
