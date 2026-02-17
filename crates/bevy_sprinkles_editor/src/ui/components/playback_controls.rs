use bevy::color::palettes::tailwind;
use bevy::prelude::*;
use bevy_sprinkles::prelude::*;

use crate::state::{PlaybackPlayEvent, PlaybackResetEvent};
use crate::ui::tokens::{PRIMARY_COLOR, TEXT_BODY_COLOR};
use crate::ui::widgets::button::{
    ButtonSize, ButtonVariant, IconButtonProps, icon_button, set_button_variant,
};
use crate::ui::icons::{ICON_PAUSE, ICON_PLAY, ICON_REPEAT, ICON_STOP};
use crate::viewport::EditorParticlePreview;

pub fn plugin(app: &mut App) {
    app.add_systems(
        Update,
        (
            handle_play_pause_click,
            handle_stop_click,
            handle_loop_click,
            update_play_pause_icon,
            update_loop_button_style,
        ),
    );
}

#[derive(Component)]
pub struct EditorPlaybackControls;

#[derive(Component)]
pub struct PlayPauseButton;

#[derive(Component)]
pub struct StopButton;

#[derive(Component)]
pub struct LoopButton;

pub fn playback_controls(asset_server: &AssetServer) -> impl Bundle {
    (
        EditorPlaybackControls,
        Node {
            align_items: AlignItems::Center,
            column_gap: px(6),
            ..default()
        },
        children![
            play_pause_button(asset_server),
            stop_button(asset_server),
            loop_button(asset_server),
        ],
    )
}

fn play_pause_button(asset_server: &AssetServer) -> impl Bundle {
    (
        PlayPauseButton,
        icon_button(
            IconButtonProps::new(ICON_PAUSE)
                .color(tailwind::GREEN_500)
                .variant(ButtonVariant::Ghost)
                .with_size(ButtonSize::Icon),
            asset_server,
        ),
    )
}

fn stop_button(asset_server: &AssetServer) -> impl Bundle {
    (
        StopButton,
        icon_button(
            IconButtonProps::new(ICON_STOP)
                .color(TEXT_BODY_COLOR)
                .variant(ButtonVariant::Ghost)
                .with_size(ButtonSize::Icon),
            asset_server,
        ),
    )
}

fn loop_button(asset_server: &AssetServer) -> impl Bundle {
    (
        LoopButton,
        icon_button(
            IconButtonProps::new(ICON_REPEAT)
                .color(PRIMARY_COLOR)
                .variant(ButtonVariant::Active)
                .with_size(ButtonSize::Icon),
            asset_server,
        ),
    )
}

fn handle_play_pause_click(
    mut commands: Commands,
    mut runtime_query: Query<&mut ParticleSystemRuntime, With<EditorParticlePreview>>,
    button_query: Query<&Interaction, (Changed<Interaction>, With<PlayPauseButton>)>,
) {
    for interaction in &button_query {
        if *interaction == Interaction::Pressed {
            for mut runtime in &mut runtime_query {
                runtime.toggle();
                if !runtime.paused {
                    commands.trigger(PlaybackPlayEvent);
                }
            }
        }
    }
}

fn handle_stop_click(
    mut commands: Commands,
    button_query: Query<&Interaction, (Changed<Interaction>, With<StopButton>)>,
) {
    for interaction in &button_query {
        if *interaction == Interaction::Pressed {
            commands.trigger(PlaybackResetEvent);
        }
    }
}

fn handle_loop_click(
    mut runtime_query: Query<&mut ParticleSystemRuntime, With<EditorParticlePreview>>,
    button_query: Query<&Interaction, (Changed<Interaction>, With<LoopButton>)>,
) {
    for interaction in &button_query {
        if *interaction == Interaction::Pressed {
            for mut runtime in &mut runtime_query {
                runtime.force_loop = !runtime.force_loop;
            }
        }
    }
}

fn update_play_pause_icon(
    asset_server: Res<AssetServer>,
    runtime_query: Query<
        &ParticleSystemRuntime,
        (Changed<ParticleSystemRuntime>, With<EditorParticlePreview>),
    >,
    button_query: Query<&Children, With<PlayPauseButton>>,
    mut image_query: Query<&mut ImageNode>,
) {
    let Some(runtime) = runtime_query.iter().next() else {
        return;
    };

    let icon_path = if runtime.paused {
        ICON_PLAY
    } else {
        ICON_PAUSE
    };

    for children in &button_query {
        for child in children.iter() {
            if let Ok(mut image) = image_query.get_mut(child) {
                image.image = asset_server.load(icon_path);
            }
        }
    }
}

fn update_loop_button_style(
    runtime_query: Query<
        &ParticleSystemRuntime,
        (Changed<ParticleSystemRuntime>, With<EditorParticlePreview>),
    >,
    mut button_query: Query<
        (
            &Children,
            &mut ButtonVariant,
            &mut BackgroundColor,
            &mut BorderColor,
        ),
        With<LoopButton>,
    >,
    mut image_query: Query<&mut ImageNode>,
) {
    let Some(runtime) = runtime_query.iter().next() else {
        return;
    };

    let variant = if runtime.force_loop {
        ButtonVariant::Active
    } else {
        ButtonVariant::Ghost
    };

    for (children, mut current_variant, mut bg, mut border) in &mut button_query {
        if *current_variant != variant {
            *current_variant = variant;
            set_button_variant(variant, &mut bg, &mut border);
        }

        for child in children.iter() {
            if let Ok(mut image) = image_query.get_mut(child) {
                image.color = variant.text_color().into();
            }
        }
    }
}
