use bevy::prelude::*;

use crate::ui::widgets::inspector_field::InspectorFieldProps;
use crate::ui::widgets::vector_edit::VectorSuffixes;

use super::InspectorSection;

pub fn plugin(_app: &mut App) {}

pub fn angle_section() -> (impl Bundle, InspectorSection) {
    (
        (),
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
    )
}
