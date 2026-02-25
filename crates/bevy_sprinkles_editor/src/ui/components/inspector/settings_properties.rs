use bevy::prelude::*;

use crate::ui::tokens::BORDER_COLOR;
use crate::ui::widgets::checkbox::{CheckboxProps, checkbox};
use crate::ui::widgets::inspector_field::fields_row;

use crate::ui::components::binding::FieldBinding;
use crate::ui::components::inspector::{FieldKind, path_to_label};

pub fn settings_properties_section(asset_server: &AssetServer) -> impl Bundle {
    (
        Node {
            width: percent(100),
            flex_direction: FlexDirection::Column,
            row_gap: px(12),
            padding: UiRect::all(px(24)),
            border: UiRect::bottom(px(1)),
            ..default()
        },
        BorderColor::all(BORDER_COLOR),
        children![(
            fields_row(),
            children![(
                FieldBinding::editor_settings("show_fps", FieldKind::Bool),
                checkbox(CheckboxProps::new(path_to_label("show_fps")), asset_server,),
            )],
        )],
    )
}
