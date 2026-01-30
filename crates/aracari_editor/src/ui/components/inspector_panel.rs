use bevy::prelude::*;

use crate::ui::tokens::{FONT_PATH, TEXT_BODY_COLOR, TEXT_SIZE};
use crate::ui::widgets::button::ButtonClickEvent;
use crate::ui::widgets::panel::{PanelDirection, PanelProps, panel, panel_resize_handle};
use crate::ui::widgets::panel_section::{PanelSectionProps, panel_section};

pub fn plugin(app: &mut App) {
    app.add_systems(Update, setup_inspector_panel);
}

#[derive(Component)]
pub struct EditorInspectorPanel;

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
                // Collapsible section
                parent
                    .spawn(panel_section(
                        PanelSectionProps::new("Collapsible").collapsible(),
                        &asset_server,
                    ))
                    .with_child(test_label("Content 1", font.clone()));

                // Add & Collapsible section
                parent
                    .spawn(panel_section(
                        PanelSectionProps::new("Add & Collapsible")
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
