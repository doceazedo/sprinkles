use aracari::prelude::*;
use bevy::color::palettes::tailwind;
use bevy::prelude::*;
use bevy::text::{FontFeatureTag, FontFeatures};

use crate::state::PlaybackSeekEvent;
use crate::ui::tokens::{FONT_PATH, TEXT_MUTED_COLOR};
use crate::viewport::EditorParticlePreview;

const SEEKBAR_HEIGHT: f32 = 4.0;
const SEEKBAR_WIDTH: f32 = 192.0;
const LABEL_SIZE: f32 = 12.0;

pub fn plugin(app: &mut App) {
    app.add_systems(Update, (update_seekbar, setup_seekbar_observers))
        .add_observer(on_seekbar_drag);
}

#[derive(Component)]
pub struct EditorSeekbar;

#[derive(Component)]
pub struct SeekbarElapsed;

#[derive(Component)]
pub struct SeekbarDuration;

#[derive(Component)]
pub struct SeekbarHitbox;

#[derive(Component)]
pub struct SeekbarTrack;

#[derive(Component)]
pub struct SeekbarFill;

#[derive(Component, Default)]
pub struct SeekbarDragState {
    pub dragging: bool,
    pub drag_time: f32,
}

#[derive(EntityEvent)]
pub struct SeekbarDragEvent {
    pub entity: Entity,
    pub value: f32,
}

pub fn seekbar(asset_server: &AssetServer) -> impl Bundle {
    let font: Handle<Font> = asset_server.load(FONT_PATH);
    let tabular_figures: FontFeatures = [FontFeatureTag::TABULAR_FIGURES].into();

    (
        EditorSeekbar,
        Node {
            align_items: AlignItems::Center,
            column_gap: px(6),
            ..default()
        },
        children![
            (
                SeekbarElapsed,
                Text::new("0.00"),
                TextFont {
                    font: font.clone().into(),
                    font_size: LABEL_SIZE,
                    font_features: tabular_figures.clone(),
                    weight: FontWeight::MEDIUM,
                    ..default()
                },
                TextColor(TEXT_MUTED_COLOR.into()),
            ),
            (
                Node {
                    width: px(SEEKBAR_WIDTH),
                    height: px(SEEKBAR_HEIGHT),
                    ..default()
                },
                children![
                    (
                        SeekbarTrack,
                        Node {
                            width: percent(100),
                            height: percent(100),
                            border_radius: BorderRadius::all(Val::Percent(100.0)),
                            overflow: Overflow::clip(),
                            ..default()
                        },
                        BackgroundColor(tailwind::ZINC_700.into()),
                        children![(
                            SeekbarFill,
                            Node {
                                width: percent(0),
                                height: percent(100),
                                border_radius: BorderRadius::all(Val::Percent(100.0)),
                                ..default()
                            },
                            BackgroundColor(tailwind::ZINC_200.into()),
                        )],
                    ),
                    (
                        SeekbarHitbox,
                        SeekbarDragState::default(),
                        Node {
                            position_type: PositionType::Absolute,
                            width: px(SEEKBAR_WIDTH),
                            height: px(SEEKBAR_HEIGHT * 3.),
                            top: px(-SEEKBAR_HEIGHT),
                            justify_content: JustifyContent::Center,
                            align_items: AlignItems::Center,
                            ..default()
                        },
                    ),
                ],
            ),
            (
                SeekbarDuration,
                Text::new("0.00s"),
                TextFont {
                    font: font.into(),
                    font_size: LABEL_SIZE,
                    font_features: tabular_figures,
                    weight: FontWeight::MEDIUM,
                    ..default()
                },
                TextColor(TEXT_MUTED_COLOR.into()),
            ),
        ],
    )
}

fn format_time(seconds: f32) -> String {
    format!("{:.2}", seconds)
}

fn format_duration(seconds: f32) -> String {
    format!("{:.2}s", seconds)
}

fn setup_seekbar_observers(hitboxes: Query<Entity, Added<SeekbarHitbox>>, mut commands: Commands) {
    for entity in &hitboxes {
        commands
            .entity(entity)
            .observe(on_drag_start)
            .observe(on_drag)
            .observe(on_drag_end);
    }
}

fn update_seekbar(
    assets: Res<Assets<ParticleSystemAsset>>,
    system_query: Query<(Entity, &ParticleSystem3D), With<EditorParticlePreview>>,
    emitter_query: Query<(&EmitterEntity, &EmitterRuntime)>,
    mut elapsed_label: Query<&mut Text, (With<SeekbarElapsed>, Without<SeekbarDuration>)>,
    mut duration_label: Query<&mut Text, (With<SeekbarDuration>, Without<SeekbarElapsed>)>,
    mut fill: Query<&mut Node, With<SeekbarFill>>,
    drag_state: Query<&SeekbarDragState, With<SeekbarHitbox>>,
) {
    let Ok(drag) = drag_state.single() else {
        return;
    };

    let Some((system_entity, particle_system)) = system_query.iter().next() else {
        return;
    };

    let Some(asset) = assets.get(&particle_system.handle) else {
        return;
    };

    let duration = asset
        .emitters
        .iter()
        .map(|e| e.time.total_duration())
        .fold(0.0_f32, |a, b| a.max(b));

    let elapsed = if drag.dragging {
        drag.drag_time
    } else {
        emitter_query
            .iter()
            .filter(|(e, _)| e.parent_system == system_entity)
            .map(|(_, r)| r.system_time)
            .fold(0.0_f32, |a, b| a.max(b))
    };

    for mut text in &mut elapsed_label {
        **text = format_time(elapsed);
    }

    for mut text in &mut duration_label {
        **text = format_duration(duration);
    }

    // skip fill update while dragging since it's handled by the drag event
    if drag.dragging {
        return;
    }

    let progress = if duration > 0.0 {
        (elapsed / duration).clamp(0.0, 1.0)
    } else {
        0.0
    };

    for mut node in &mut fill {
        node.width = Val::Percent(progress * 100.0);
    }
}

fn on_drag_start(
    event: On<Pointer<DragStart>>,
    mut hitboxes: Query<&mut SeekbarDragState, With<SeekbarHitbox>>,
) {
    let Ok(mut drag_state) = hitboxes.get_mut(event.entity) else {
        return;
    };
    drag_state.dragging = true;
}

fn on_drag(
    event: On<Pointer<Drag>>,
    hitboxes: Query<(&SeekbarDragState, &ComputedNode, &UiGlobalTransform), With<SeekbarHitbox>>,
    mut fill: Query<&mut Node, With<SeekbarFill>>,
    mut commands: Commands,
) {
    let entity = event.entity;
    let Ok((drag_state, computed, transform)) = hitboxes.get(entity) else {
        return;
    };

    if !drag_state.dragging {
        return;
    }

    let pointer_x = event.pointer_location.position.x;
    let scale = computed.inverse_scale_factor;
    let center_x = transform.translation.x * scale;
    let width = computed.size.x * scale;
    let left_x = center_x - width * 0.5;
    let value = ((pointer_x - left_x) / width).clamp(0.0, 1.0);

    for mut node in &mut fill {
        node.width = Val::Percent(value * 100.0);
    }

    commands.trigger(SeekbarDragEvent { entity, value });
}

fn on_drag_end(
    event: On<Pointer<DragEnd>>,
    mut hitboxes: Query<&mut SeekbarDragState, With<SeekbarHitbox>>,
) {
    let entity = event.entity;
    let Ok(mut drag_state) = hitboxes.get_mut(entity) else {
        return;
    };

    drag_state.dragging = false;
}

fn on_seekbar_drag(
    event: On<SeekbarDragEvent>,
    mut commands: Commands,
    assets: Res<Assets<ParticleSystemAsset>>,
    system_query: Query<&ParticleSystem3D, With<EditorParticlePreview>>,
    mut hitboxes: Query<&mut SeekbarDragState, With<SeekbarHitbox>>,
) {
    let Some(particle_system) = system_query.iter().next() else {
        return;
    };

    let Some(asset) = assets.get(&particle_system.handle) else {
        return;
    };

    let duration = asset
        .emitters
        .iter()
        .map(|e| e.time.total_duration())
        .fold(0.0_f32, |a, b| a.max(b));

    let seek_time = event.value * duration;

    if let Ok(mut drag_state) = hitboxes.get_mut(event.entity) {
        drag_state.drag_time = seek_time;
    }

    commands.trigger(PlaybackSeekEvent(seek_time));
}
