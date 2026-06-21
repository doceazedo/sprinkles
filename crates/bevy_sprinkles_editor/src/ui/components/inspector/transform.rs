use bevy::prelude::*;

use crate::ui::components::binding::BindingTarget;
use crate::ui::widgets::inspector_field::InspectorFieldProps;
use crate::ui::widgets::vector_edit::VectorSuffixes;

use super::InspectorSection;

pub fn transform_section() -> (impl Bundle, InspectorSection) {
    transform_section_with_target(BindingTarget::Inspected)
}

pub fn asset_transform_section() -> (impl Bundle, InspectorSection) {
    transform_section_with_target(BindingTarget::Asset)
}

fn transform_section_with_target(target: BindingTarget) -> (impl Bundle, InspectorSection) {
    let field = |path| InspectorFieldProps::new(path).with_target(target);
    (
        (),
        InspectorSection::from_fields(
            "Initial transform",
            vec![
                field("initial_transform.translation")
                    .vector(VectorSuffixes::XYZ)
                    .into(),
                field("initial_transform.rotation")
                    .vector(VectorSuffixes::RollPitchYaw)
                    .with_suffix("°")
                    .into(),
                field("initial_transform.scale")
                    .vector(VectorSuffixes::XYZ)
                    .into(),
            ],
        ),
    )
}
