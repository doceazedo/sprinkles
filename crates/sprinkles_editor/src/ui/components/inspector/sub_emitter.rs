use sprinkles::prelude::*;
use bevy::prelude::*;

use crate::state::{DirtyState, EditorState, Inspectable};
use crate::ui::components::inspector::utils::name_to_label;
use crate::ui::tokens::{FONT_PATH, TEXT_MUTED_COLOR, TEXT_SIZE_SM};
use crate::ui::widgets::alert::{AlertSpan, AlertVariant, alert};
use crate::ui::widgets::checkbox::{CheckboxCommitEvent, CheckboxProps, checkbox};
use crate::ui::widgets::combobox::{
    ComboBoxChangeEvent, ComboBoxConfig, ComboBoxOptionData, combobox_with_selected,
};
use crate::ui::widgets::inspector_field::fields_row;
use crate::ui::widgets::text_edit::{TextEditCommitEvent, TextEditProps, text_edit};

use super::{InspectorSection, inspector_section, section_needs_setup};
use crate::ui::components::binding::{
    find_ancestor, get_inspecting_emitter, get_inspecting_emitter_mut, mark_dirty_and_restart,
};

#[derive(Component)]
struct SubEmitterSection;

#[derive(Component)]
struct SubEmitterModeComboBox;

#[derive(Component)]
struct SubEmitterContent;

#[derive(Component)]
struct SubEmitterTargetComboBox;

#[derive(Component)]
struct SubEmitterFieldsContainer;

#[derive(Component)]
struct SubEmitterFrequencyField;

#[derive(Component)]
struct SubEmitterAmountField;

#[derive(Component)]
struct SubEmitterKeepVelocityCheckbox;

#[derive(Component)]
struct SubEmitterFieldInput {
    field_name: String,
}

pub fn plugin(app: &mut App) {
    app.add_observer(handle_sub_emitter_mode_change)
        .add_observer(handle_sub_emitter_target_change)
        .add_observer(handle_sub_emitter_field_commit)
        .add_observer(handle_keep_velocity_change)
        .add_systems(
            Update,
            (
                setup_sub_emitter_content,
                cleanup_sub_emitter_on_emitter_change,
                sync_sub_emitter_ui,
            )
                .after(super::update_inspected_emitter_tracker),
        );
}

pub fn sub_emitter_section(asset_server: &AssetServer) -> impl Bundle {
    (
        SubEmitterSection,
        inspector_section(InspectorSection::new("Sub-emitter", vec![]), asset_server),
    )
}

fn mode_index(config: &Option<SubEmitterConfig>) -> usize {
    match config {
        None => 0,
        Some(c) => match c.mode {
            SubEmitterMode::Constant => 1,
            SubEmitterMode::AtEnd => 2,
            SubEmitterMode::AtCollision => 3,
            SubEmitterMode::AtStart => 4,
        },
    }
}

fn mode_options() -> Vec<ComboBoxOptionData> {
    vec![
        ComboBoxOptionData::new(name_to_label("None")).with_value("None"),
        ComboBoxOptionData::new(name_to_label("Constant")).with_value("Constant"),
        ComboBoxOptionData::new(name_to_label("AtEnd")).with_value("AtEnd"),
        ComboBoxOptionData::new(name_to_label("AtCollision")).with_value("AtCollision"),
        ComboBoxOptionData::new(name_to_label("AtStart")).with_value("AtStart"),
    ]
}

fn target_options(
    asset: &ParticleSystemAsset,
    current_emitter_index: usize,
) -> Vec<ComboBoxOptionData> {
    asset
        .emitters
        .iter()
        .enumerate()
        .filter(|(i, _)| *i != current_emitter_index)
        .map(|(i, e)| ComboBoxOptionData::new(name_to_label(&e.name)).with_value(&i.to_string()))
        .collect()
}

fn target_combo_index(
    config: &Option<SubEmitterConfig>,
    asset: &ParticleSystemAsset,
    current_emitter_index: usize,
) -> usize {
    let target = match config {
        Some(c) => c.target_emitter,
        None => return 0,
    };

    asset
        .emitters
        .iter()
        .enumerate()
        .filter(|(i, _)| *i != current_emitter_index)
        .position(|(i, _)| i == target)
        .unwrap_or(0)
}

fn spawn_mode_combobox(parent: &mut ChildSpawnerCommands, font: &Handle<Font>, mode_idx: usize) {
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
                SubEmitterModeComboBox,
                combobox_with_selected(mode_options(), mode_idx),
            ));
        });
    });
}

fn spawn_target_combobox(
    parent: &mut ChildSpawnerCommands,
    font: &Handle<Font>,
    asset: &ParticleSystemAsset,
    current_emitter_index: usize,
    target_idx: usize,
) {
    let options = target_options(asset, current_emitter_index);
    if options.is_empty() {
        return;
    }

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
                Text::new("Target"),
                TextFont {
                    font: font.clone(),
                    font_size: TEXT_SIZE_SM,
                    weight: FontWeight::MEDIUM,
                    ..default()
                },
                TextColor(TEXT_MUTED_COLOR.into()),
            ));
            wrapper.spawn((
                SubEmitterTargetComboBox,
                combobox_with_selected(options, target_idx),
            ));
        });
    });
}

fn spawn_fields(
    parent: &mut ChildSpawnerCommands,
    config: &Option<SubEmitterConfig>,
    asset: &ParticleSystemAsset,
    current_emitter_index: usize,
    font: &Handle<Font>,
    asset_server: &AssetServer,
) {
    let has_mode = config.is_some();
    let is_constant = matches!(config, Some(c) if c.mode == SubEmitterMode::Constant);
    let is_event = matches!(
        config,
        Some(c) if matches!(c.mode, SubEmitterMode::AtEnd | SubEmitterMode::AtCollision | SubEmitterMode::AtStart)
    );

    parent
        .spawn((
            SubEmitterFieldsContainer,
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
        .with_children(|fields| {
            let target_idx = target_combo_index(config, asset, current_emitter_index);
            spawn_target_combobox(fields, font, asset, current_emitter_index, target_idx);

            let freq_val = config.as_ref().map(|c| c.frequency).unwrap_or(4.0);
            fields
                .spawn((
                    SubEmitterFrequencyField,
                    Node {
                        width: percent(100),
                        flex_direction: FlexDirection::Column,
                        row_gap: px(12.0),
                        display: if is_constant {
                            Display::Flex
                        } else {
                            Display::None
                        },
                        ..default()
                    },
                ))
                .with_children(|freq| {
                    freq.spawn(fields_row()).with_children(|row| {
                        row.spawn((
                            SubEmitterFieldInput {
                                field_name: "frequency".into(),
                            },
                            text_edit(
                                TextEditProps::default()
                                    .with_label("Frequency (Hz)")
                                    .with_default_value(&freq_val.to_string())
                                    .numeric_f32(),
                            ),
                        ));
                    });
                });

            let amount_val = config.as_ref().map(|c| c.amount).unwrap_or(1);
            fields
                .spawn((
                    SubEmitterAmountField,
                    Node {
                        width: percent(100),
                        flex_direction: FlexDirection::Column,
                        row_gap: px(12.0),
                        display: if is_event {
                            Display::Flex
                        } else {
                            Display::None
                        },
                        ..default()
                    },
                ))
                .with_children(|amt| {
                    amt.spawn(fields_row()).with_children(|row| {
                        row.spawn((
                            SubEmitterFieldInput {
                                field_name: "amount".into(),
                            },
                            text_edit(
                                TextEditProps::default()
                                    .with_label("Amount")
                                    .with_default_value(&amount_val.to_string())
                                    .numeric_f32(),
                            ),
                        ));
                    });
                });

            let keep_vel = config.as_ref().map(|c| c.keep_velocity).unwrap_or(false);
            fields.spawn(fields_row()).with_children(|row| {
                row.spawn((
                    SubEmitterKeepVelocityCheckbox,
                    checkbox(
                        CheckboxProps::new("Keep velocity").checked(keep_vel),
                        asset_server,
                    ),
                ));
            });

            let target_amount = config
                .as_ref()
                .and_then(|c| asset.emitters.get(c.target_emitter))
                .map(|e| e.emission.particles_amount)
                .unwrap_or(0);

            fields.spawn(alert(
                AlertVariant::Important,
                vec![
                    AlertSpan::Text("A total of up to ".into()),
                    AlertSpan::Bold(format!("{target_amount}")),
                    AlertSpan::Text(
                        " particles can be spawned at once, limited by the sub-emitter's ".into(),
                    ),
                    AlertSpan::Bold("Particles amount".into()),
                    AlertSpan::Text(".".into()),
                ],
            ));
        });
}

fn setup_sub_emitter_content(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    editor_state: Res<EditorState>,
    assets: Res<Assets<ParticleSystemAsset>>,
    sections: Query<(Entity, &InspectorSection), With<SubEmitterSection>>,
    existing: Query<Entity, With<SubEmitterContent>>,
) {
    let Some(entity) = section_needs_setup(&sections, &existing) else {
        return;
    };

    let font: Handle<Font> = asset_server.load(FONT_PATH);

    let inspecting = editor_state
        .inspecting
        .as_ref()
        .filter(|i| i.kind == Inspectable::Emitter);
    let emitter_index = inspecting.map(|i| i.index as usize).unwrap_or(0);

    let config = get_inspecting_emitter(&editor_state, &assets)
        .map(|(_, emitter)| emitter.sub_emitter.clone())
        .unwrap_or(None);

    let asset_ref = editor_state
        .current_project
        .as_ref()
        .and_then(|h| assets.get(h));

    let mode_idx = mode_index(&config);

    let content = commands
        .spawn((
            SubEmitterContent,
            Node {
                width: percent(100),
                flex_direction: FlexDirection::Column,
                row_gap: px(12.0),
                ..default()
            },
        ))
        .with_children(|parent| {
            spawn_mode_combobox(parent, &font, mode_idx);

            if let Some(asset) = asset_ref {
                spawn_fields(parent, &config, asset, emitter_index, &font, &asset_server);
            }
        })
        .id();

    commands.entity(entity).add_child(content);
}

fn cleanup_sub_emitter_on_emitter_change(
    mut commands: Commands,
    tracker: Res<super::InspectedEmitterTracker>,
    existing: Query<Entity, With<SubEmitterContent>>,
) {
    if !tracker.is_changed() {
        return;
    }

    for entity in &existing {
        commands.entity(entity).try_despawn();
    }
}

fn handle_sub_emitter_mode_change(
    trigger: On<ComboBoxChangeEvent>,
    mode_comboboxes: Query<(), With<SubEmitterModeComboBox>>,
    editor_state: Res<EditorState>,
    mut assets: ResMut<Assets<ParticleSystemAsset>>,
    mut dirty_state: ResMut<DirtyState>,
    mut emitter_runtimes: Query<&mut EmitterRuntime>,
) {
    if mode_comboboxes.get(trigger.entity).is_err() {
        return;
    }

    let Some((_, emitter)) = get_inspecting_emitter_mut(&editor_state, &mut assets) else {
        return;
    };

    let new_config = match trigger.value.as_deref().unwrap_or(&trigger.label) {
        "None" => None,
        label => {
            let mode = match label {
                "Constant" => SubEmitterMode::Constant,
                "AtEnd" => SubEmitterMode::AtEnd,
                "AtCollision" => SubEmitterMode::AtCollision,
                "AtStart" => SubEmitterMode::AtStart,
                _ => return,
            };
            let prev = emitter.sub_emitter.clone().unwrap_or_default();
            Some(SubEmitterConfig {
                mode,
                target_emitter: find_first_other_emitter_index(&editor_state, emitter),
                frequency: prev.frequency,
                amount: prev.amount,
                keep_velocity: prev.keep_velocity,
            })
        }
    };

    if mode_index(&emitter.sub_emitter) == mode_index(&new_config) {
        return;
    }

    emitter.sub_emitter = new_config;
    mark_dirty_and_restart(
        &mut dirty_state,
        &mut emitter_runtimes,
        emitter.time.fixed_seed,
    );
}

fn find_first_other_emitter_index(editor_state: &EditorState, emitter: &EmitterData) -> usize {
    let current_index = editor_state
        .inspecting
        .as_ref()
        .filter(|i| i.kind == Inspectable::Emitter)
        .map(|i| i.index as usize)
        .unwrap_or(0);

    if let Some(ref config) = emitter.sub_emitter {
        return config.target_emitter;
    }

    if current_index == 0 { 1 } else { 0 }
}

fn handle_sub_emitter_target_change(
    trigger: On<ComboBoxChangeEvent>,
    target_comboboxes: Query<(), With<SubEmitterTargetComboBox>>,
    editor_state: Res<EditorState>,
    mut assets: ResMut<Assets<ParticleSystemAsset>>,
    mut dirty_state: ResMut<DirtyState>,
    mut emitter_runtimes: Query<&mut EmitterRuntime>,
) {
    if target_comboboxes.get(trigger.entity).is_err() {
        return;
    }

    let value_str = trigger.value.as_deref().unwrap_or(&trigger.label);
    let Ok(target_index) = value_str.parse::<usize>() else {
        return;
    };

    let Some((_, emitter)) = get_inspecting_emitter_mut(&editor_state, &mut assets) else {
        return;
    };

    let Some(ref mut config) = emitter.sub_emitter else {
        return;
    };

    if config.target_emitter == target_index {
        return;
    }

    config.target_emitter = target_index;
    mark_dirty_and_restart(
        &mut dirty_state,
        &mut emitter_runtimes,
        emitter.time.fixed_seed,
    );
}

fn handle_sub_emitter_field_commit(
    trigger: On<TextEditCommitEvent>,
    field_inputs: Query<&SubEmitterFieldInput>,
    parents: Query<&ChildOf>,
    editor_state: Res<EditorState>,
    mut assets: ResMut<Assets<ParticleSystemAsset>>,
    mut dirty_state: ResMut<DirtyState>,
    mut emitter_runtimes: Query<&mut EmitterRuntime>,
) {
    let Some(input_entity) = find_ancestor(trigger.entity, &parents, 10, |e| {
        field_inputs.get(e).is_ok()
    }) else {
        return;
    };
    let Ok(input) = field_inputs.get(input_entity) else {
        return;
    };

    let Some((_, emitter)) = get_inspecting_emitter_mut(&editor_state, &mut assets) else {
        return;
    };

    let Some(ref mut config) = emitter.sub_emitter else {
        return;
    };

    let changed = match input.field_name.as_str() {
        "frequency" => {
            if let Ok(value) = trigger.text.parse::<f32>() {
                if config.frequency != value {
                    config.frequency = value.max(0.01);
                    true
                } else {
                    false
                }
            } else {
                false
            }
        }
        "amount" => {
            if let Ok(value) = trigger.text.parse::<u32>() {
                let clamped = value.clamp(1, 32);
                if config.amount != clamped {
                    config.amount = clamped;
                    true
                } else {
                    false
                }
            } else {
                false
            }
        }
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

fn handle_keep_velocity_change(
    trigger: On<CheckboxCommitEvent>,
    checkboxes: Query<(), With<SubEmitterKeepVelocityCheckbox>>,
    editor_state: Res<EditorState>,
    mut assets: ResMut<Assets<ParticleSystemAsset>>,
    mut dirty_state: ResMut<DirtyState>,
    mut emitter_runtimes: Query<&mut EmitterRuntime>,
) {
    if checkboxes.get(trigger.entity).is_err() {
        return;
    }

    let Some((_, emitter)) = get_inspecting_emitter_mut(&editor_state, &mut assets) else {
        return;
    };

    let Some(ref mut config) = emitter.sub_emitter else {
        return;
    };

    if config.keep_velocity != trigger.checked {
        config.keep_velocity = trigger.checked;
        mark_dirty_and_restart(
            &mut dirty_state,
            &mut emitter_runtimes,
            emitter.time.fixed_seed,
        );
    }
}

fn sync_sub_emitter_ui(
    editor_state: Res<EditorState>,
    assets: Res<Assets<ParticleSystemAsset>>,
    mut mode_comboboxes: Query<&mut ComboBoxConfig, With<SubEmitterModeComboBox>>,
    mut target_comboboxes: Query<
        &mut ComboBoxConfig,
        (
            With<SubEmitterTargetComboBox>,
            Without<SubEmitterModeComboBox>,
        ),
    >,
    mut fields_container: Query<
        &mut Node,
        (
            With<SubEmitterFieldsContainer>,
            Without<SubEmitterFrequencyField>,
            Without<SubEmitterAmountField>,
        ),
    >,
    mut freq_field: Query<
        &mut Node,
        (
            With<SubEmitterFrequencyField>,
            Without<SubEmitterFieldsContainer>,
            Without<SubEmitterAmountField>,
        ),
    >,
    mut amount_field: Query<
        &mut Node,
        (
            With<SubEmitterAmountField>,
            Without<SubEmitterFieldsContainer>,
            Without<SubEmitterFrequencyField>,
        ),
    >,
) {
    if !editor_state.is_changed() && !assets.is_changed() {
        return;
    }

    let emitter = get_inspecting_emitter(&editor_state, &assets).map(|(_, e)| e);
    let config = emitter.map(|e| &e.sub_emitter);

    if editor_state.is_changed() {
        let new_index = config.map(mode_index).unwrap_or(0);
        for mut cb in &mut mode_comboboxes {
            if cb.selected != new_index {
                cb.selected = new_index;
            }
        }
    }

    if assets.is_changed() {
        let emitter_index = editor_state
            .inspecting
            .as_ref()
            .filter(|i| i.kind == Inspectable::Emitter)
            .map(|i| i.index as usize)
            .unwrap_or(0);

        if let Some(asset) = editor_state
            .current_project
            .as_ref()
            .and_then(|h| assets.get(h))
        {
            let options = target_options(asset, emitter_index);
            let sub_emitter = emitter.and_then(|e| e.sub_emitter.as_ref());
            let target_idx = sub_emitter
                .map(|c| {
                    asset
                        .emitters
                        .iter()
                        .enumerate()
                        .filter(|(i, _)| *i != emitter_index)
                        .position(|(i, _)| i == c.target_emitter)
                        .unwrap_or(0)
                })
                .unwrap_or(0);
            for mut cb in &mut target_comboboxes {
                cb.set_options(options.clone());
                cb.selected = target_idx;
            }
        }
    }

    let has_mode = matches!(config, Some(Some(_)));
    let is_constant = matches!(
        config,
        Some(Some(c)) if c.mode == SubEmitterMode::Constant
    );
    let is_event = matches!(
        config,
        Some(Some(c)) if matches!(c.mode, SubEmitterMode::AtEnd | SubEmitterMode::AtCollision | SubEmitterMode::AtStart)
    );

    fn set_visible(
        query: &mut Query<&mut Node, impl bevy::ecs::query::QueryFilter>,
        visible: bool,
    ) {
        let display = if visible {
            Display::Flex
        } else {
            Display::None
        };
        for mut node in query {
            if node.display != display {
                node.display = display;
            }
        }
    }

    set_visible(&mut fields_container, has_mode);
    set_visible(&mut freq_field, is_constant);
    set_visible(&mut amount_field, is_event);
}
