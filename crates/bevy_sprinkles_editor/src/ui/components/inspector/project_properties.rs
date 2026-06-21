use std::path::PathBuf;

use bevy::prelude::*;

use crate::state::EditorState;
use crate::ui::icons::ICON_FOLDER_OPEN;
use crate::ui::tokens::{BORDER_COLOR, FONT_PATH, TEXT_MUTED_COLOR, TEXT_SIZE, TEXT_SIZE_SM};
use crate::ui::widgets::button::{
    ButtonClickEvent, ButtonSize, ButtonVariant, IconButtonProps, icon_button,
};
use crate::ui::widgets::checkbox::{CheckboxProps, checkbox};
use crate::ui::widgets::inspector_field::fields_row;
use crate::ui::widgets::text_edit::{TextEditProps, text_edit};
use crate::utils::{MAX_DISPLAY_PATH_LEN, truncate_path};

use super::{DynamicSectionContent, InspectorSection, inspector_section, section_needs_setup};
use crate::ui::components::binding::FieldBinding;
use crate::ui::components::inspector::FieldKind;

#[derive(Component)]
struct ProjectPropertiesSection;

#[derive(Component)]
struct ProjectPropertiesContent;

#[derive(Component)]
struct ProjectRuntimeSection;

#[derive(Component)]
struct ProjectRuntimeContent;

#[derive(Component)]
struct RevealFileButton(PathBuf);

pub fn plugin(app: &mut App) {
    app.add_systems(Update, (setup_properties_content, setup_runtime_content))
        .add_observer(handle_reveal_file_click);
}

pub fn project_properties_section(asset_server: &AssetServer) -> impl Bundle {
    (
        ProjectPropertiesSection,
        inspector_section(InspectorSection::new("Properties", vec![]), asset_server),
    )
}

pub fn project_runtime_section(asset_server: &AssetServer) -> impl Bundle {
    (
        ProjectRuntimeSection,
        inspector_section(InspectorSection::new("Runtime", vec![]), asset_server),
    )
}

fn setup_properties_content(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    editor_state: Res<EditorState>,
    sections: Query<(Entity, &InspectorSection), With<ProjectPropertiesSection>>,
    existing: Query<Entity, With<ProjectPropertiesContent>>,
) {
    let Some(entity) = section_needs_setup(&sections, &existing) else {
        return;
    };

    if editor_state.current_project.is_none() {
        return;
    }

    let font: Handle<Font> = asset_server.load(FONT_PATH);
    let file_path = editor_state.current_project_path.clone();

    let content = commands
        .spawn((
            ProjectPropertiesContent,
            DynamicSectionContent,
            Node {
                width: percent(100),
                flex_direction: FlexDirection::Column,
                row_gap: px(12.0),
                ..default()
            },
        ))
        .with_children(|parent| {
            parent.spawn(fields_row()).with_children(|row| {
                row.spawn((
                    FieldBinding::asset("name", FieldKind::String),
                    text_edit(TextEditProps::default().with_label("Project name")),
                ));
            });

            parent.spawn(fields_row()).with_children(|row| {
                row.spawn((
                    FieldBinding::asset("authors.submitted_by", FieldKind::String),
                    text_edit(TextEditProps::default().with_label("Submitted by")),
                ));
                row.spawn((
                    FieldBinding::asset("authors.inspired_by", FieldKind::String),
                    text_edit(TextEditProps::default().with_label("Inspired by")),
                ));
            });

            if let Some(ref path) = file_path {
                spawn_file_path_field(parent, path, &font, &asset_server);
            }
        })
        .id();

    commands.entity(entity).add_child(content);
}

fn handle_reveal_file_click(trigger: On<ButtonClickEvent>, buttons: Query<&RevealFileButton>) {
    let Ok(button) = buttons.get(trigger.entity) else {
        return;
    };
    if let Some(parent) = button.0.parent() {
        let _ = open::that(parent);
    }
}

fn spawn_file_path_field(
    parent: &mut ChildSpawnerCommands,
    path: &PathBuf,
    font: &Handle<Font>,
    _asset_server: &AssetServer,
) {
    let display_path = truncate_path(&path.display().to_string(), MAX_DISPLAY_PATH_LEN);

    parent.spawn(fields_row()).with_children(|row| {
        row.spawn(Node {
            flex_direction: FlexDirection::Column,
            row_gap: px(3.0),
            flex_grow: 1.0,
            flex_shrink: 1.0,
            flex_basis: px(0.0),
            ..default()
        })
        .with_children(|col| {
            col.spawn((
                Text::new("File path"),
                TextFont {
                    font: font.clone().into(),
                    font_size: TEXT_SIZE_SM.into(),
                    weight: FontWeight::MEDIUM,
                    ..default()
                },
                TextColor(TEXT_MUTED_COLOR.into()),
            ));

            col.spawn(Node {
                width: percent(100),
                height: px(28.0),
                padding: UiRect::new(px(6.0), px(30.0), px(6.0), px(6.0)),
                border: UiRect::all(px(1.0)),
                border_radius: BorderRadius::all(px(2.0)),
                align_items: AlignItems::Center,
                overflow: Overflow::clip(),
                ..default()
            })
            .insert(BorderColor::all(BORDER_COLOR))
            .with_children(|wrapper| {
                wrapper.spawn((
                    Text::new(display_path),
                    TextFont {
                        font: font.clone().into(),
                        font_size: TEXT_SIZE.into(),
                        ..default()
                    },
                    TextColor(TEXT_MUTED_COLOR.into()),
                    Node {
                        flex_shrink: 0.0,
                        ..default()
                    },
                ));

                let wrapper_target = wrapper.target_entity();
                wrapper
                    .commands()
                    .spawn_scene(icon_button(
                        IconButtonProps::new(ICON_FOLDER_OPEN)
                            .variant(ButtonVariant::Ghost)
                            .with_size(ButtonSize::IconSM),
                    ))
                    .insert(RevealFileButton(path.clone()))
                    .insert(ChildOf(wrapper_target))
                    .entry::<Node>()
                    .and_modify(|mut node| {
                        node.position_type = PositionType::Absolute;
                        node.right = px(2.0);
                        node.top = px(2.0);
                    });
            });
        });
    });
}

fn setup_runtime_content(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    editor_state: Res<EditorState>,
    sections: Query<(Entity, &InspectorSection), With<ProjectRuntimeSection>>,
    existing: Query<Entity, With<ProjectRuntimeContent>>,
) {
    let Some(entity) = section_needs_setup(&sections, &existing) else {
        return;
    };

    if editor_state.current_project.is_none() {
        return;
    }

    let content = commands
        .spawn((
            ProjectRuntimeContent,
            DynamicSectionContent,
            Node {
                width: percent(100),
                flex_direction: FlexDirection::Column,
                row_gap: px(12.0),
                ..default()
            },
        ))
        .with_children(|parent| {
            parent.spawn(fields_row()).with_children(|row| {
                row.spawn((
                    FieldBinding::asset("despawn_on_finish", FieldKind::Bool),
                    checkbox(CheckboxProps::new("Despawn on finish"), &asset_server),
                ));
            });
        })
        .id();

    commands.entity(entity).add_child(content);
}
