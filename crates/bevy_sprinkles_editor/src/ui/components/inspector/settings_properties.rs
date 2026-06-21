use bevy::prelude::*;

use crate::io::{EditorBloom, EditorSmaaPreset, EditorTonemapping};
use crate::ui::tokens::BORDER_COLOR;
use crate::ui::widgets::checkbox::{CheckboxProps, checkbox};
use crate::ui::widgets::combobox::ComboBoxOptionData;
use crate::ui::widgets::inspector_field::{combobox_field, fields_row};

use crate::ui::components::binding::FieldBinding;
use crate::ui::components::inspector::utils::{
    combobox_options_from_reflect, combobox_options_from_reflect_raw, combobox_options_to_combobox,
};
use crate::ui::components::inspector::{FieldKind, path_to_label};

fn optional_combobox_options(mut options: Vec<ComboBoxOptionData>) -> Vec<ComboBoxOptionData> {
    options.insert(
        0,
        ComboBoxOptionData::new("Disabled").with_value("Disabled"),
    );
    options
}

fn settings_combobox(
    path: &str,
    label: Option<&str>,
    combobox_data: Vec<ComboBoxOptionData>,
) -> impl Bundle {
    let combobox_data = optional_combobox_options(combobox_data);
    let field_options = combobox_options_to_combobox(&combobox_data);
    let label = label
        .map(String::from)
        .unwrap_or_else(|| path_to_label(path));
    (
        FieldBinding::editor_settings(
            path,
            FieldKind::ComboBox {
                options: field_options,
                optional: true,
            },
        ),
        combobox_field(label, combobox_data),
    )
}

pub fn spawn_settings_properties_section(commands: &mut Commands, parent: Entity) {
    let section = commands
        .spawn((
            Node {
                width: percent(100),
                flex_direction: FlexDirection::Column,
                row_gap: px(12),
                padding: UiRect::all(px(24)),
                border: UiRect::bottom(px(1)),
                ..default()
            },
            BorderColor::all(BORDER_COLOR),
            ChildOf(parent),
        ))
        .id();

    let row = commands.spawn(fields_row()).insert(ChildOf(section)).id();
    commands
        .spawn_scene(checkbox(CheckboxProps::new(path_to_label("show_fps"))))
        .insert(FieldBinding::editor_settings("show_fps", FieldKind::Bool))
        .insert(ChildOf(row));

    let row = commands.spawn(fields_row()).insert(ChildOf(section)).id();
    commands
        .spawn_scene(checkbox(CheckboxProps::new("V-Sync").checked(true)))
        .insert(FieldBinding::editor_settings("vsync", FieldKind::Bool))
        .insert(ChildOf(row));

    let row = commands.spawn(fields_row()).insert(ChildOf(section)).id();
    commands
        .spawn(settings_combobox(
            "tonemapping",
            None,
            combobox_options_from_reflect_raw::<EditorTonemapping>(),
        ))
        .insert(ChildOf(row));

    let row = commands.spawn(fields_row()).insert(ChildOf(section)).id();
    commands
        .spawn(settings_combobox(
            "bloom",
            None,
            combobox_options_from_reflect::<EditorBloom>(),
        ))
        .insert(ChildOf(row));

    let row = commands.spawn(fields_row()).insert(ChildOf(section)).id();
    commands
        .spawn(settings_combobox(
            "anti_aliasing",
            Some("Anti-aliasing (SMAA)"),
            combobox_options_from_reflect::<EditorSmaaPreset>(),
        ))
        .insert(ChildOf(row));
}
