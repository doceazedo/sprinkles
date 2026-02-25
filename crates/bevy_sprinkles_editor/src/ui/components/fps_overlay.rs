use std::time::Duration;

use bevy::diagnostic::{DiagnosticsStore, FrameTimeDiagnosticsPlugin};
use bevy::prelude::*;
use bevy::text::{FontFeatureTag, FontFeatures};
use bevy::time::common_conditions::on_timer;

use crate::io::EditorData;
use crate::ui::tokens::{FONT_PATH, TEXT_BODY_COLOR, TEXT_SIZE};

use super::viewport::EditorViewportContainer;

const REFRESH_INTERVAL: Duration = Duration::from_millis(100);

#[derive(Component)]
struct FpsOverlay;

#[derive(Component)]
struct FpsText;

pub fn plugin(app: &mut App) {
    app.add_systems(
        Update,
        (
            setup_fps_overlay,
            toggle_fps_overlay,
            update_fps_text.run_if(on_timer(REFRESH_INTERVAL)),
        ),
    );
}

fn setup_fps_overlay(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    containers: Query<Entity, Added<EditorViewportContainer>>,
) {
    for container in &containers {
        let font: Handle<Font> = asset_server.load(FONT_PATH);

        let overlay = commands
            .spawn((
                FpsOverlay,
                Node {
                    position_type: PositionType::Absolute,
                    top: px(12),
                    right: px(12),
                    ..default()
                },
            ))
            .with_children(|parent| {
                parent.spawn((
                    FpsText,
                    Text::new(""),
                    TextFont {
                        font,
                        font_size: TEXT_SIZE,
                        font_features: FontFeatures::builder()
                            .enable(FontFeatureTag::TABULAR_FIGURES)
                            .build(),
                        ..default()
                    },
                    TextColor(TEXT_BODY_COLOR.into()),
                    TextShadow {
                        offset: Vec2::new(-1.0, 1.0),
                        color: Color::BLACK.with_alpha(0.8),
                    },
                ));
            })
            .id();

        commands.entity(container).add_child(overlay);
    }
}

fn update_fps_text(diagnostics: Res<DiagnosticsStore>, mut text: Query<&mut Text, With<FpsText>>) {
    for mut t in &mut text {
        if let Some(fps) = diagnostics.get(&FrameTimeDiagnosticsPlugin::FPS)
            && let Some(fps_value) = fps.smoothed()
            && let Some(frame_time) = diagnostics.get(&FrameTimeDiagnosticsPlugin::FRAME_TIME)
            && let Some(ft_value) = frame_time.smoothed()
        {
            **t = format!("{fps_value:.0} FPS ({ft_value:.1} ms)");
        }
    }
}

fn toggle_fps_overlay(
    editor_data: Res<EditorData>,
    mut overlays: Query<&mut Node, With<FpsOverlay>>,
) {
    if !editor_data.is_changed() {
        return;
    }

    let display = if editor_data.settings.show_fps {
        Display::Flex
    } else {
        Display::None
    };

    for mut node in &mut overlays {
        if node.display != display {
            node.display = display;
        }
    }
}
