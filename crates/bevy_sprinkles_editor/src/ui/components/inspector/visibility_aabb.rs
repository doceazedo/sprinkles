use bevy::picking::prelude::Pickable;
use bevy::prelude::*;

use crate::state::{EditorState, GenerateAabbRequest, Inspectable};
use crate::ui::icons::ICON_PIVOT_BOUNDBOX;
use crate::ui::tokens::{BACKGROUND_COLOR, CORNER_RADIUS_LG};
use crate::ui::widgets::button::{ButtonClickEvent, ButtonProps, button};
use crate::ui::widgets::inspector_field::InspectorFieldProps;
use crate::ui::widgets::vector_edit::VectorSuffixes;
use crate::viewport::AabbGeneration;

use super::{InspectedEmitterTracker, InspectorSection};

#[derive(Component)]
struct VisibilityAabbSection;

#[derive(Component)]
struct GenerateAabbButton;

#[derive(Component)]
struct GenerateAabbOverlay;

pub fn plugin(app: &mut App) {
    app.add_systems(
        Update,
        (setup_generate_aabb_button, sync_generate_aabb_button),
    )
    .add_observer(handle_generate_aabb_click);
}

pub fn visibility_aabb_section() -> (impl Bundle, InspectorSection) {
    (
        VisibilityAabbSection,
        InspectorSection::from_fields(
            "Visibility AABB",
            vec![
                InspectorFieldProps::new("draw_pass.visibility_aabb.center")
                    .with_label("Position")
                    .vector(VectorSuffixes::XYZ)
                    .into(),
                InspectorFieldProps::new("draw_pass.visibility_aabb.half_extents")
                    .with_label("Size")
                    .vector(VectorSuffixes::WHD)
                    .into(),
            ],
        ),
    )
}

fn setup_generate_aabb_button(
    mut commands: Commands,
    sections: Query<(Entity, &Children), With<VisibilityAabbSection>>,
    existing: Query<(), With<GenerateAabbButton>>,
) {
    if !existing.is_empty() {
        return;
    }

    let Ok((section, children)) = sections.single() else {
        return;
    };

    if children.len() < 2 {
        return;
    }

    commands
        .spawn_scene(button(
            ButtonProps::new("Generate AABB")
                .with_left_icon(ICON_PIVOT_BOUNDBOX)
                .align_left(),
        ))
        .insert(GenerateAabbButton)
        .insert(ChildOf(section));
}

fn sync_generate_aabb_button(
    mut commands: Commands,
    generation: Res<AabbGeneration>,
    mut tracker: ResMut<InspectedEmitterTracker>,
    buttons: Query<(Entity, &Children), With<GenerateAabbButton>>,
    overlays: Query<Entity, With<GenerateAabbOverlay>>,
    mut texts: Query<&mut Text>,
) {
    if !generation.is_changed() {
        return;
    }

    let generating = generation.is_active();

    // When generation finishes, force the inspector fields to re-read the new
    // value (they only refresh when the inspected emitter tracker changes).
    if !generating {
        tracker.set_changed();
    }

    let label = if generating {
        "Generating AABB..."
    } else {
        "Generate AABB"
    };
    let has_overlay = !overlays.is_empty();

    for (button_entity, children) in &buttons {
        for child in children.iter() {
            if let Ok(mut text) = texts.get_mut(child) {
                **text = label.into();
            }
        }

        if generating && !has_overlay {
            let overlay = commands
                .spawn((
                    GenerateAabbOverlay,
                    Pickable::default(),
                    Button,
                    Node {
                        position_type: PositionType::Absolute,
                        left: px(0.0),
                        top: px(0.0),
                        width: percent(100),
                        height: percent(100),
                        border_radius: BorderRadius::all(CORNER_RADIUS_LG),
                        ..default()
                    },
                    BackgroundColor(BACKGROUND_COLOR.with_alpha(0.7).into()),
                ))
                .id();
            commands.entity(button_entity).add_child(overlay);
        }
    }

    if !generating && has_overlay {
        for entity in &overlays {
            commands.entity(entity).try_despawn();
        }
    }
}

fn handle_generate_aabb_click(
    trigger: On<ButtonClickEvent>,
    buttons: Query<(), With<GenerateAabbButton>>,
    editor_state: Res<EditorState>,
    mut commands: Commands,
) {
    if buttons.get(trigger.entity).is_err() {
        return;
    }

    let Some(index) = editor_state
        .inspecting
        .as_ref()
        .filter(|i| i.kind == Inspectable::Emitter)
        .map(|i| i.index as usize)
    else {
        return;
    };

    commands.trigger(GenerateAabbRequest(index));
}
