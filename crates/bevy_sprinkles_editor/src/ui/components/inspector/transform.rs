use bevy::prelude::*;

use crate::ui::components::binding::BindingTarget;
use crate::ui::widgets::inspector_field::InspectorFieldProps;
use crate::ui::widgets::vector_edit::VectorSuffixes;

use super::{InspectorSection, inspector_section};

pub fn transform_section(asset_server: &AssetServer) -> impl Bundle {
    transform_section_with_target(asset_server, BindingTarget::Inspected)
}

pub fn asset_transform_section(asset_server: &AssetServer) -> impl Bundle {
    transform_section_with_target(asset_server, BindingTarget::Asset)
}

fn transform_section_with_target(asset_server: &AssetServer, target: BindingTarget) -> impl Bundle {
    let field = |path| InspectorFieldProps::new(path).with_target(target);
    inspector_section(
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
        asset_server,
    )
}
