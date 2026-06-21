use bevy::prelude::*;

use crate::ui::widgets::inspector_field::InspectorFieldProps;
use crate::ui::widgets::vector_edit::VectorSuffixes;

use super::InspectorSection;

pub fn plugin(_app: &mut App) {}

pub fn scale_section() -> (impl Bundle, InspectorSection) {
    (
        (),
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
                        .into(),
                ],
            ],
        ),
    )
}
