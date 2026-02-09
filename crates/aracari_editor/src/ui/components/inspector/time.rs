use bevy::prelude::*;

use crate::ui::widgets::inspector_field::InspectorFieldProps;

use super::{InspectorSection, inspector_section};

pub fn plugin(_app: &mut App) {}

pub fn time_section(asset_server: &AssetServer) -> impl Bundle {
    inspector_section(
        InspectorSection::new(
            "Time",
            vec![
                vec![
                    InspectorFieldProps::new("time.lifetime")
                        .with_icon("icons/ri-time-line.png")
                        .with_suffix("s")
                        .into(),
                    InspectorFieldProps::new("time.lifetime_randomness")
                        .percent()
                        .with_icon("icons/ri-time-line.png")
                        .into(),
                ],
                vec![
                    InspectorFieldProps::new("time.delay")
                        .with_min(0.)
                        .with_icon("icons/ri-time-line.png")
                        .with_suffix("s")
                        .into(),
                ],
                vec![
                    InspectorFieldProps::new("time.explosiveness")
                        .percent()
                        .into(),
                    InspectorFieldProps::new("time.spawn_time_randomness")
                        .percent()
                        .into(),
                ],
                vec![
                    InspectorFieldProps::new("time.fixed_fps")
                        .u32_or_empty()
                        .with_placeholder("Unlimited")
                        .into(),
                    InspectorFieldProps::new("time.fixed_seed")
                        .optional_u32()
                        .with_icon("icons/ri-seedling-fill.png")
                        .with_placeholder("Random")
                        .into(),
                ],
                vec![InspectorFieldProps::new("time.one_shot").bool().into()],
            ],
        ),
        asset_server,
    )
}
