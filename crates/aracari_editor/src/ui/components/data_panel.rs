use bevy::prelude::*;

use crate::ui::widgets::button::{
    ButtonClickEvent, ButtonMoreEvent, ButtonProps, ButtonVariant, button,
};
use crate::ui::widgets::panel::{PanelDirection, PanelProps, panel, panel_resize_handle};
use crate::ui::widgets::panel_section::{PanelSectionProps, panel_section};

pub fn plugin(app: &mut App) {
    app.add_systems(Update, setup_data_panel);
}

#[derive(Component)]
pub struct EditorDataPanel;

pub fn data_panel(_asset_server: &AssetServer) -> impl Bundle {
    (
        EditorDataPanel,
        panel(
            PanelProps::new(PanelDirection::Left)
                .with_width(224)
                .with_min_width(160)
                .with_max_width(320),
        ),
    )
}

fn setup_data_panel(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    panels: Query<Entity, Added<EditorDataPanel>>,
) {
    for panel_entity in &panels {
        commands
            .entity(panel_entity)
            .with_child(panel_resize_handle(panel_entity, PanelDirection::Left))
            .with_children(|parent| {
                // Emitters section
                parent
                    .spawn(panel_section(
                        PanelSectionProps::new("Emitters").with_add_button(),
                        &asset_server,
                    ))
                    .observe(|_: On<ButtonClickEvent>| {
                        println!("TODO: add emitter");
                    })
                    .with_children(|section| {
                        section
                            .spawn(button(
                                ButtonProps::new("Emitter 1")
                                    .variant(ButtonVariant::Active)
                                    .align_left()
                                    .with_more(),
                                &asset_server,
                            ))
                            .observe(|_: On<ButtonClickEvent>| {
                                println!("Emitter 1");
                            })
                            .observe(|_: On<ButtonMoreEvent>| {
                                println!("More Emitter 1...");
                            });

                        section
                            .spawn(button(
                                ButtonProps::new("Emitter 2")
                                    .variant(ButtonVariant::Ghost)
                                    .align_left()
                                    .with_more(),
                                &asset_server,
                            ))
                            .observe(|_: On<ButtonClickEvent>| {
                                println!("Emitter 2");
                            })
                            .observe(|_: On<ButtonMoreEvent>| {
                                println!("More Emitter 2...");
                            });
                    });

                // Colliders section
                parent
                    .spawn(panel_section(
                        PanelSectionProps::new("Colliders").with_add_button(),
                        &asset_server,
                    ))
                    .observe(|_: On<ButtonClickEvent>| {
                        println!("TODO: add collider");
                    });

                // Attractors section
                parent
                    .spawn(panel_section(
                        PanelSectionProps::new("Attractors").with_add_button(),
                        &asset_server,
                    ))
                    .observe(|_: On<ButtonClickEvent>| {
                        println!("TODO: add attractor");
                    });
            });
    }
}
