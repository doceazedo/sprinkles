use sprinkles::prelude::*;
use bevy::prelude::*;

use crate::state::{DirtyState, EditorState};
use crate::ui::components::inspector::utils::name_to_label;
use crate::ui::tokens::{FONT_PATH, TEXT_MUTED_COLOR, TEXT_SIZE_SM};
use crate::ui::widgets::combobox::{
    ComboBoxChangeEvent, ComboBoxConfig, ComboBoxOptionData, combobox_with_selected,
};
use crate::ui::widgets::inspector_field::{InspectorFieldProps, fields_row, spawn_inspector_field};
use crate::ui::widgets::text_edit::{TextEditCommitEvent, TextEditProps, text_edit};

use super::{InspectorSection, inspector_section, section_needs_setup};
use crate::ui::components::binding::{
    find_ancestor, get_inspecting_emitter, get_inspecting_emitter_mut, mark_dirty_and_restart,
};

#[derive(Component)]
struct CollisionSection;

#[derive(Component)]
struct CollisionModeComboBox;

#[derive(Component)]
struct CollisionContent;

#[derive(Component)]
struct CollisionRigidFields;

#[derive(Component)]
struct CollisionCommonFields;

#[derive(Component)]
struct CollisionFieldInput {
    field_name: String,
}

pub fn plugin(app: &mut App) {
    app.add_observer(handle_collision_mode_change)
        .add_observer(handle_collision_field_commit)
        .add_systems(
            Update,
            (
                setup_collision_content,
                cleanup_collision_on_emitter_change,
                sync_collision_ui,
            )
                .after(super::update_inspected_emitter_tracker),
        );
}

pub fn collision_section(asset_server: &AssetServer) -> impl Bundle {
    (
        CollisionSection,
        inspector_section(InspectorSection::new("Collision", vec![]), asset_server),
    )
}

fn collision_mode_index(mode: &Option<EmitterCollisionMode>) -> usize {
    match mode {
        None => 0,
        Some(EmitterCollisionMode::Rigid { .. }) => 1,
        Some(EmitterCollisionMode::HideOnContact) => 2,
    }
}

fn collision_mode_options() -> Vec<ComboBoxOptionData> {
    vec![
        ComboBoxOptionData::new(name_to_label("None")).with_value("None"),
        ComboBoxOptionData::new(name_to_label("Rigid")).with_value("Rigid"),
        ComboBoxOptionData::new(name_to_label("HideOnContact")).with_value("HideOnContact"),
    ]
}

fn spawn_mode_combobox(parent: &mut ChildSpawnerCommands, font: &Handle<Font>, mode_index: usize) {
    parent.spawn(fields_row()).with_children(|row| {
        row.spawn(Node {
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(3.0),
            flex_grow: 1.0,
            flex_shrink: 1.0,
            flex_basis: Val::Px(0.0),
            ..default()
        })
        .with_children(|wrapper| {
            wrapper.spawn((
                Text::new("Mode"),
                TextFont {
                    font: font.clone(),
                    font_size: TEXT_SIZE_SM,
                    weight: FontWeight::MEDIUM,
                    ..default()
                },
                TextColor(TEXT_MUTED_COLOR.into()),
            ));
            wrapper.spawn((
                CollisionModeComboBox,
                combobox_with_selected(collision_mode_options(), mode_index),
            ));
        });
    });
}

fn spawn_common_fields(
    parent: &mut ChildSpawnerCommands,
    has_mode: bool,
    asset_server: &AssetServer,
) {
    parent
        .spawn((
            CollisionCommonFields,
            Node {
                width: percent(100),
                flex_direction: FlexDirection::Column,
                row_gap: px(12.0),
                display: if has_mode {
                    Display::Flex
                } else {
                    Display::None
                },
                ..default()
            },
        ))
        .with_children(|common| {
            common.spawn(fields_row()).with_children(|row| {
                spawn_inspector_field(
                    row,
                    InspectorFieldProps::new("collision.use_scale").bool(),
                    asset_server,
                );
            });
            common.spawn(fields_row()).with_children(|row| {
                spawn_inspector_field(
                    row,
                    InspectorFieldProps::new("collision.base_size"),
                    asset_server,
                );
            });
        });
}

fn spawn_rigid_fields(
    parent: &mut ChildSpawnerCommands,
    is_rigid: bool,
    friction_val: &str,
    bounce_val: &str,
) {
    parent
        .spawn((
            CollisionRigidFields,
            Node {
                width: percent(100),
                flex_direction: FlexDirection::Column,
                row_gap: px(12.0),
                display: if is_rigid {
                    Display::Flex
                } else {
                    Display::None
                },
                ..default()
            },
        ))
        .with_children(|rigid| {
            rigid.spawn(fields_row()).with_children(|row| {
                row.spawn((
                    CollisionFieldInput {
                        field_name: "friction".into(),
                    },
                    text_edit(
                        TextEditProps::default()
                            .with_label("Friction")
                            .with_default_value(friction_val)
                            .numeric_f32(),
                    ),
                ));
                row.spawn((
                    CollisionFieldInput {
                        field_name: "bounce".into(),
                    },
                    text_edit(
                        TextEditProps::default()
                            .with_label("Bounce")
                            .with_default_value(bounce_val)
                            .numeric_f32(),
                    ),
                ));
            });
        });
}

fn setup_collision_content(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    editor_state: Res<EditorState>,
    assets: Res<Assets<ParticleSystemAsset>>,
    sections: Query<(Entity, &InspectorSection), With<CollisionSection>>,
    existing: Query<Entity, With<CollisionContent>>,
) {
    let Some(entity) = section_needs_setup(&sections, &existing) else {
        return;
    };

    let emitter = get_inspecting_emitter(&editor_state, &assets).map(|(_, e)| e);
    let mode = emitter.map(|e| &e.collision.mode);
    let mode_index = mode.map(collision_mode_index).unwrap_or(0);
    let is_rigid = matches!(mode, Some(Some(EmitterCollisionMode::Rigid { .. })));
    let has_mode = mode.map(|m| m.is_some()).unwrap_or(false);

    let (friction_val, bounce_val) = match mode {
        Some(Some(EmitterCollisionMode::Rigid { friction, bounce })) => {
            (friction.to_string(), bounce.to_string())
        }
        _ => ("0".to_string(), "0".to_string()),
    };

    let font: Handle<Font> = asset_server.load(FONT_PATH);

    let content = commands
        .spawn((
            CollisionContent,
            Node {
                width: percent(100),
                flex_direction: FlexDirection::Column,
                row_gap: px(12.0),
                ..default()
            },
        ))
        .with_children(|parent| {
            spawn_mode_combobox(parent, &font, mode_index);
            spawn_common_fields(parent, has_mode, &asset_server);
            spawn_rigid_fields(parent, is_rigid, &friction_val, &bounce_val);
        })
        .id();

    commands.entity(entity).add_child(content);
}

fn cleanup_collision_on_emitter_change(
    mut commands: Commands,
    tracker: Res<super::InspectedEmitterTracker>,
    existing: Query<Entity, With<CollisionContent>>,
) {
    if !tracker.is_changed() {
        return;
    }

    for entity in &existing {
        commands.entity(entity).try_despawn();
    }
}

fn handle_collision_mode_change(
    trigger: On<ComboBoxChangeEvent>,
    collision_comboboxes: Query<(), With<CollisionModeComboBox>>,
    editor_state: Res<EditorState>,
    mut assets: ResMut<Assets<ParticleSystemAsset>>,
    mut dirty_state: ResMut<DirtyState>,
    mut emitter_runtimes: Query<&mut EmitterRuntime>,
) {
    if collision_comboboxes.get(trigger.entity).is_err() {
        return;
    }

    let Some((_, emitter)) = get_inspecting_emitter_mut(&editor_state, &mut assets) else {
        return;
    };

    let new_mode = match trigger.value.as_deref().unwrap_or(&trigger.label) {
        "None" => None,
        "Rigid" => Some(EmitterCollisionMode::Rigid {
            friction: 0.0,
            bounce: 0.0,
        }),
        "HideOnContact" => Some(EmitterCollisionMode::HideOnContact),
        _ => return,
    };

    if collision_mode_index(&emitter.collision.mode) == collision_mode_index(&new_mode) {
        return;
    }

    emitter.collision.mode = new_mode;
    mark_dirty_and_restart(
        &mut dirty_state,
        &mut emitter_runtimes,
        emitter.time.fixed_seed,
    );
}

fn handle_collision_field_commit(
    trigger: On<TextEditCommitEvent>,
    collision_fields: Query<&CollisionFieldInput>,
    parents: Query<&ChildOf>,
    editor_state: Res<EditorState>,
    mut assets: ResMut<Assets<ParticleSystemAsset>>,
    mut dirty_state: ResMut<DirtyState>,
    mut emitter_runtimes: Query<&mut EmitterRuntime>,
) {
    let Some(input_entity) = find_ancestor(trigger.entity, &parents, 10, |e| {
        collision_fields.get(e).is_ok()
    }) else {
        return;
    };
    let Ok(input) = collision_fields.get(input_entity) else {
        return;
    };

    let Ok(value) = trigger.text.parse::<f32>() else {
        return;
    };

    let Some((_, emitter)) = get_inspecting_emitter_mut(&editor_state, &mut assets) else {
        return;
    };

    let Some(EmitterCollisionMode::Rigid {
        ref mut friction,
        ref mut bounce,
    }) = emitter.collision.mode
    else {
        return;
    };

    let set_if_changed = |field: &mut f32| {
        if *field != value {
            *field = value;
            true
        } else {
            false
        }
    };

    let changed = match input.field_name.as_str() {
        "friction" => set_if_changed(friction),
        "bounce" => set_if_changed(bounce),
        _ => false,
    };

    if changed {
        mark_dirty_and_restart(
            &mut dirty_state,
            &mut emitter_runtimes,
            emitter.time.fixed_seed,
        );
    }
}

fn sync_collision_ui(
    editor_state: Res<EditorState>,
    assets: Res<Assets<ParticleSystemAsset>>,
    mut comboboxes: Query<&mut ComboBoxConfig, With<CollisionModeComboBox>>,
    mut rigid_fields: Query<
        &mut Node,
        (With<CollisionRigidFields>, Without<CollisionCommonFields>),
    >,
    mut common_fields: Query<
        &mut Node,
        (With<CollisionCommonFields>, Without<CollisionRigidFields>),
    >,
) {
    if !editor_state.is_changed() && !assets.is_changed() {
        return;
    }

    let emitter = get_inspecting_emitter(&editor_state, &assets).map(|(_, e)| e);
    let mode = emitter.map(|e| &e.collision.mode);

    if editor_state.is_changed() {
        let new_index = mode.map(collision_mode_index).unwrap_or(0);
        for mut config in &mut comboboxes {
            if config.selected != new_index {
                config.selected = new_index;
            }
        }
    }

    let is_rigid = matches!(mode, Some(Some(EmitterCollisionMode::Rigid { .. })));
    let has_mode = mode.map(|m| m.is_some()).unwrap_or(false);

    for mut node in &mut rigid_fields {
        let display = if is_rigid {
            Display::Flex
        } else {
            Display::None
        };
        if node.display != display {
            node.display = display;
        }
    }

    for mut node in &mut common_fields {
        let display = if has_mode {
            Display::Flex
        } else {
            Display::None
        };
        if node.display != display {
            node.display = display;
        }
    }
}
