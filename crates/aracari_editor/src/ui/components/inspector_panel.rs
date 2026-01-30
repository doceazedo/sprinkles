use aracari::prelude::*;
use bevy::prelude::*;

use crate::state::{EditorState, Inspectable};
use crate::ui::tokens::{BORDER_COLOR, FONT_PATH, TEXT_BODY_COLOR, TEXT_SIZE, TEXT_SIZE_LG};
use crate::ui::widgets::button::ButtonClickEvent;
use crate::ui::widgets::button::{ButtonVariant, IconButtonProps, icon_button};
use crate::ui::widgets::checkbox::{CheckboxProps, checkbox};
use crate::ui::widgets::panel::{PanelDirection, PanelProps, panel, panel_resize_handle};
use crate::ui::widgets::panel_section::{PanelSectionProps, PanelSectionSize, panel_section};
use crate::ui::widgets::text_edit::{TextEditProps, text_edit};

pub fn plugin(app: &mut App) {
    app.add_systems(Update, (setup_inspector_panel, update_panel_title));
}

#[derive(Component)]
pub struct EditorInspectorPanel;

#[derive(Component)]
struct InspectorPanelContent;

#[derive(Component)]
struct PanelTitleText;

#[derive(Component)]
struct PanelTitleIcon;

pub fn inspector_panel(_asset_server: &AssetServer) -> impl Bundle {
    (
        EditorInspectorPanel,
        panel(
            PanelProps::new(PanelDirection::Right)
                .with_width(320)
                .with_min_width(320)
                .with_max_width(512),
        ),
    )
}

fn setup_inspector_panel(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    panels: Query<Entity, Added<EditorInspectorPanel>>,
) {
    let font: Handle<Font> = asset_server.load(FONT_PATH);

    for panel_entity in &panels {
        commands
            .entity(panel_entity)
            .with_child(panel_resize_handle(panel_entity, PanelDirection::Right))
            .with_children(|parent| {
                parent.spawn(panel_title(&asset_server));

                parent
                    .spawn((
                        InspectorPanelContent,
                        Node {
                            width: percent(100),
                            flex_direction: FlexDirection::Column,
                            ..default()
                        },
                    ))
                    .with_children(|content| {
                        content
                            .spawn(panel_section(
                                PanelSectionProps::new("Collapsible")
                                    .with_size(PanelSectionSize::XL)
                                    .collapsible(),
                                &asset_server,
                            ))
                            .with_children(|section| {
                                section.spawn(test_label("Content 1", font.clone()));
                                section.spawn(text_edit(
                                    TextEditProps::default()
                                        .with_label("Text Input")
                                        .with_placeholder("Type here..."),
                                ));
                                section.spawn(text_edit(
                                    TextEditProps::default()
                                        .numeric()
                                        .with_label("Numeric Input")
                                        .with_placeholder("0.0")
                                        .with_default_value("45")
                                        .with_suffix("%"),
                                ));
                            });

                        content
                            .spawn(panel_section(
                                PanelSectionProps::new("Add & Collapsible")
                                    .with_size(PanelSectionSize::XL)
                                    .with_add_button()
                                    .collapsible(),
                                &asset_server,
                            ))
                            .observe(|_: On<ButtonClickEvent>| {
                                println!("Add & Collapsible");
                            })
                            .with_children(|section| {
                                section.spawn(test_label("Content A", font.clone()));
                                section.spawn(test_label("Content B", font.clone()));
                            });
                    });
            });
    }
}

fn panel_title(asset_server: &AssetServer) -> impl Bundle {
    let font: Handle<Font> = asset_server.load(FONT_PATH);

    (
        Node {
            width: percent(100),
            align_items: AlignItems::Center,
            column_gap: px(12.0),
            padding: UiRect::axes(px(24.0), px(20.0)),
            border: UiRect::bottom(px(1.0)),
            ..default()
        },
        BorderColor::all(BORDER_COLOR),
        children![
            (
                Node {
                    align_items: AlignItems::Center,
                    column_gap: px(6.0),
                    flex_grow: 1.0,
                    ..default()
                },
                children![
                    (
                        PanelTitleIcon,
                        ImageNode::new(asset_server.load("icons/ri-showers-fill.png"))
                            .with_color(Color::Srgba(TEXT_BODY_COLOR)),
                        Node {
                            width: px(16.0),
                            height: px(16.0),
                            ..default()
                        },
                    ),
                    (
                        PanelTitleText,
                        Text::new(""),
                        TextFont {
                            font: font.into(),
                            font_size: TEXT_SIZE_LG,
                            weight: FontWeight::SEMIBOLD,
                            ..default()
                        },
                        TextColor(TEXT_BODY_COLOR.into()),
                    ),
                ],
            ),
            checkbox(CheckboxProps::new("Enabled").checked(true), asset_server),
            icon_button(
                IconButtonProps::new("icons/ri-more-fill.png").variant(ButtonVariant::Ghost),
                asset_server,
            ),
        ],
    )
}

fn update_panel_title(
    editor_state: Res<EditorState>,
    assets: Res<Assets<ParticleSystemAsset>>,
    mut title_text: Query<&mut Text, With<PanelTitleText>>,
    mut title_icon: Query<&mut ImageNode, With<PanelTitleIcon>>,
    asset_server: Res<AssetServer>,
    new_titles: Query<Entity, Added<PanelTitleText>>,
) {
    let should_update = editor_state.is_changed() || !new_titles.is_empty();
    if !should_update {
        return;
    }

    let Some(inspecting) = &editor_state.inspecting else {
        return;
    };

    let Some(handle) = &editor_state.current_project else {
        return;
    };

    let Some(asset) = assets.get(handle) else {
        return;
    };

    let (name, icon_path) = match inspecting.kind {
        Inspectable::Emitter => {
            let emitter = asset.emitters.get(inspecting.index as usize);
            let name = emitter.map(|e| e.name.clone()).unwrap_or_default();
            (name, "icons/ri-showers-fill.png")
        }
        Inspectable::Collider => ("Collider".to_string(), "icons/ri-box-2-fill.png"),
    };

    for mut text in &mut title_text {
        **text = name.clone();
    }

    for mut icon in &mut title_icon {
        icon.image = asset_server.load(icon_path);
    }
}

fn test_label(content: &str, font: Handle<Font>) -> impl Bundle {
    (
        Text::new(content),
        TextFont {
            font,
            font_size: TEXT_SIZE,
            ..default()
        },
        TextColor(TEXT_BODY_COLOR.into()),
    )
}
