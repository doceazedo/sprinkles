use aracari::prelude::*;
use bevy::prelude::*;

use crate::state::{DirtyState, EditorState};
use crate::ui::components::inspector::utils::name_to_label;
use crate::ui::widgets::checkbox::{CheckboxCommitEvent, CheckboxProps, CheckboxState, checkbox};
use crate::ui::widgets::inspector_field::fields_row;

use super::{InspectorSection, inspector_section, section_needs_setup};
use crate::ui::components::binding::{
    get_inspecting_emitter, get_inspecting_emitter_mut, mark_dirty_and_restart,
};

#[derive(Component)]
struct ParticleFlagsSection;

#[derive(Component)]
struct ParticleFlagsContent;

#[derive(Component)]
struct ParticleFlagCheckbox {
    flag: ParticleFlags,
}

pub fn plugin(app: &mut App) {
    app.add_observer(handle_particle_flag_checkbox)
        .add_systems(Update, (setup_particle_flags_content, sync_particle_flags));
}

pub fn particle_flags_section(asset_server: &AssetServer) -> impl Bundle {
    (
        ParticleFlagsSection,
        inspector_section(
            InspectorSection::new("Particle Flags", vec![]),
            asset_server,
        ),
    )
}

fn setup_particle_flags_content(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    editor_state: Res<EditorState>,
    assets: Res<Assets<ParticleSystemAsset>>,
    sections: Query<(Entity, &InspectorSection), With<ParticleFlagsSection>>,
    existing: Query<Entity, With<ParticleFlagsContent>>,
) {
    let Some(entity) = section_needs_setup(&sections, &existing) else {
        return;
    };

    let emitter = get_inspecting_emitter(&editor_state, &assets).map(|(_, e)| e);
    let flags = emitter.map(|e| e.particle_flags).unwrap_or_default();

    let content = commands
        .spawn((
            ParticleFlagsContent,
            Node {
                width: percent(100),
                flex_direction: FlexDirection::Column,
                row_gap: px(12.0),
                ..default()
            },
        ))
        .with_children(|parent| {
            for (name, flag) in ParticleFlags::all().iter_names() {
                let label = name_to_label(name);
                let checked = flags.contains(flag);
                parent.spawn(fields_row()).with_children(|row| {
                    row.spawn((
                        ParticleFlagCheckbox { flag },
                        checkbox(CheckboxProps::new(label).checked(checked), &asset_server),
                    ));
                });
            }
        })
        .id();

    commands.entity(entity).add_child(content);
}

fn sync_particle_flags(
    editor_state: Res<EditorState>,
    assets: Res<Assets<ParticleSystemAsset>>,
    mut flag_checkboxes: Query<(&ParticleFlagCheckbox, &mut CheckboxState)>,
) {
    if !editor_state.is_changed() {
        return;
    }

    let Some((_, emitter)) = get_inspecting_emitter(&editor_state, &assets) else {
        return;
    };

    for (flag_checkbox, mut state) in &mut flag_checkboxes {
        let checked = emitter.particle_flags.contains(flag_checkbox.flag);
        if state.checked != checked {
            state.checked = checked;
        }
    }
}

fn handle_particle_flag_checkbox(
    trigger: On<CheckboxCommitEvent>,
    flag_checkboxes: Query<&ParticleFlagCheckbox>,
    editor_state: Res<EditorState>,
    mut assets: ResMut<Assets<ParticleSystemAsset>>,
    mut dirty_state: ResMut<DirtyState>,
    mut emitter_runtimes: Query<&mut EmitterRuntime>,
) {
    let Ok(flag_checkbox) = flag_checkboxes.get(trigger.entity) else {
        return;
    };

    let Some((_, emitter)) = get_inspecting_emitter_mut(&editor_state, &mut assets) else {
        return;
    };

    emitter
        .particle_flags
        .set(flag_checkbox.flag, trigger.checked);
    mark_dirty_and_restart(
        &mut dirty_state,
        &mut emitter_runtimes,
        emitter.time.fixed_seed,
    );
}
