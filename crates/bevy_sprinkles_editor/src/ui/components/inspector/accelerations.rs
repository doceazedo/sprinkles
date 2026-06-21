use bevy::prelude::*;

use crate::ui::widgets::inspector_field::InspectorFieldProps;
use crate::ui::widgets::vector_edit::VectorSuffixes;

use super::InspectorSection;

pub fn plugin(_app: &mut App) {}

pub fn accelerations_section() -> (impl Bundle, InspectorSection) {
    (
        (),
        InspectorSection::new(
            "Accelerations",
            vec![vec![
                InspectorFieldProps::new("accelerations.gravity")
                    .vector(VectorSuffixes::XYZ)
                    .into(),
            ]],
        ),
    )
}
