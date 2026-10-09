use bevy::color::palettes::tailwind;
use bevy::prelude::*;
use bevy::text::{FontFeatureTag, FontFeatures, FontSourceTemplate};
use bevy_sprinkles::prelude::*;

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

#[derive(Component, Default, Clone)]
pub struct EditorSeekbar;

#[derive(Component, Default, Clone)]
pub struct SeekbarElapsed;

#[derive(Component, Default, Clone)]
pub struct SeekbarDuration;

#[derive(Component, Default, Clone)]
pub struct SeekbarHitbox;

#[derive(Component, Default, Clone)]
pub struct SeekbarTrack;

#[derive(Component, Default, Clone)]
pub struct SeekbarFill;

#[derive(Component, Default, Clone)]
pub struct SeekbarDragState {
    pub dragging: bool,
    pub drag_time: f32,
}

#[derive(EntityEvent)]
pub struct SeekbarDragEvent {
    pub entity: Entity,
    pub value: f32,
}

pub fn seekbar() -> impl Scene {
    let tabular_figures: FontFeatures = [FontFeatureTag::TABULAR_FIGURES].into();

    bsn! {
        EditorSeekbar
        Node {
            align_items: { AlignItems::Center },
            column_gap: px(6),
        }
        Children [
            SeekbarElapsed
            Text("0.00")
            TextFont {
                font: { FontSourceTemplate::Handle(FONT_PATH.into()) },
                font_size: LABEL_SIZE,
                font_features: { tabular_figures.clone() },
                weight: { FontWeight::MEDIUM },
            }
            TextColor({ TEXT_MUTED_COLOR })
            --
            Node {
                width: px(SEEKBAR_WIDTH),
                height: px(SEEKBAR_HEIGHT),
            }
            Children [
                SeekbarTrack
                Node {
                    width: percent(100),
                    height: percent(100),
                    border_radius: { BorderRadius::all(Val::Percent(100.0)) },
                    overflow: { Overflow::clip() },
                }
                BackgroundColor({ tailwind::ZINC_700 })
                Children [
                    SeekbarFill
                    Node {
                        width: percent(0),
                        height: percent(100),
                        border_radius: { BorderRadius::all(Val::Percent(100.0)) },
                    }
                    BackgroundColor({ tailwind::ZINC_200 })
                ]
                --
                SeekbarHitbox
                SeekbarDragState
                Node {
                    position_type: { PositionType::Absolute },
                    width: px(SEEKBAR_WIDTH),
                    height: px(SEEKBAR_HEIGHT * 3.),
                    top: px(-SEEKBAR_HEIGHT),
                    justify_content: { JustifyContent::Center },
                    align_items: { AlignItems::Center },
                }
            ]
            --
            SeekbarDuration
            Text("0.00s")
            TextFont {
                font: { FontSourceTemplate::Handle(FONT_PATH.into()) },
                font_size: LABEL_SIZE,
                font_features: { tabular_figures },
                weight: { FontWeight::MEDIUM },
            }
            TextColor({ TEXT_MUTED_COLOR })
        ]
    }
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
    assets: Res<Assets<ParticlesAsset>>,
    system_query: Query<(Entity, &Particles3d), With<EditorParticlePreview>>,
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

    let Some(asset) = assets.get(particle_system) else {
        return;
    };

    let sub_target_indices: Vec<usize> = asset
        .emitters
        .iter()
        .filter_map(|e| e.sub_emitter.as_ref().map(|s| s.target_emitter))
        .collect();

    let duration = asset
        .emitters
        .iter()
        .enumerate()
        .filter(|(idx, _)| !sub_target_indices.contains(idx))
        .map(|(_, e)| e.time.total_duration())
        .fold(0.0_f32, |a, b| a.max(b));

    let elapsed = if drag.dragging {
        drag.drag_time
    } else {
        emitter_query
            .iter()
            .filter(|(e, r)| {
                e.parent_system == system_entity && !sub_target_indices.contains(&r.emitter_index)
            })
            .map(|(_, r)| r.system_time)
            .fold(0.0_f32, |a, b| a.max(b))
    };

    for mut text in &mut elapsed_label {
        **text = format_time(elapsed);
    }

    for mut text in &mut duration_label {
        **text = format_duration(duration);
    }

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
    event: On<PointerDragStart>,
    mut hitboxes: Query<&mut SeekbarDragState, With<SeekbarHitbox>>,
) {
    let Ok(mut drag_state) = hitboxes.get_mut(event.entity) else {
        return;
    };
    drag_state.dragging = true;
}

fn on_drag(
    event: On<PointerDrag>,
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

    let pointer_x = event.pointer.position.x;
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
    event: On<PointerDragEnd>,
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
    assets: Res<Assets<ParticlesAsset>>,
    system_query: Query<&Particles3d, With<EditorParticlePreview>>,
    mut hitboxes: Query<&mut SeekbarDragState, With<SeekbarHitbox>>,
) {
    let Some(particle_system) = system_query.iter().next() else {
        return;
    };

    let Some(asset) = assets.get(particle_system) else {
        return;
    };

    let sub_target_indices: Vec<usize> = asset
        .emitters
        .iter()
        .filter_map(|e| e.sub_emitter.as_ref().map(|s| s.target_emitter))
        .collect();

    let duration = asset
        .emitters
        .iter()
        .enumerate()
        .filter(|(idx, _)| !sub_target_indices.contains(idx))
        .map(|(_, e)| e.time.total_duration())
        .fold(0.0_f32, |a, b| a.max(b));

    let seek_time = event.value * duration;

    if let Ok(mut drag_state) = hitboxes.get_mut(event.entity) {
        drag_state.drag_time = seek_time;
    }

    commands.trigger(PlaybackSeekEvent(seek_time));
}
