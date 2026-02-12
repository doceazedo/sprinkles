use bevy::prelude::*;
use bevy_sprinkles::prelude::*;

use crate::ui::components::inspector::utils::name_to_label;
use crate::ui::tokens::FONT_PATH;
use crate::ui::widgets::combobox::{ComboBoxChangeEvent, ComboBoxOptionData};
use crate::ui::widgets::inspector_field::{InspectorFieldProps, fields_row, spawn_inspector_field};
use crate::ui::widgets::text_edit::{TextEditProps, text_edit};

use super::{
    DynamicSectionContent, InspectorSection, inspector_section, section_needs_setup,
    spawn_labeled_combobox,
};
use crate::ui::components::binding::{EmitterWriter, FieldBinding};
use crate::ui::components::inspector::FieldKind;

#[derive(Component)]
struct CollisionSection;

#[derive(Component)]
struct CollisionModeComboBox;

#[derive(Component)]
struct CollisionContent;

pub fn plugin(app: &mut App) {
    app.add_observer(handle_collision_mode_change).add_systems(
        Update,
        setup_collision_content.after(super::update_inspected_emitter_tracker),
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

fn setup_collision_content(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    ew: EmitterWriter,
    sections: Query<(Entity, &InspectorSection), With<CollisionSection>>,
    existing: Query<Entity, With<CollisionContent>>,
) {
    let Some(entity) = section_needs_setup(&sections, &existing) else {
        return;
    };

    let emitter = ew.emitter();
    let mode = emitter.map(|e| &e.collision.mode);
    let mode_index = mode.map(collision_mode_index).unwrap_or(0);
    let is_rigid = matches!(mode, Some(Some(EmitterCollisionMode::Rigid { .. })));
    let has_mode = mode.map(|m| m.is_some()).unwrap_or(false);

    let font: Handle<Font> = asset_server.load(FONT_PATH);

    let content = commands
        .spawn((
            CollisionContent,
            DynamicSectionContent,
            Node {
                width: percent(100),
                flex_direction: FlexDirection::Column,
                row_gap: px(12.0),
                ..default()
            },
        ))
        .with_children(|parent| {
            spawn_labeled_combobox(
                parent,
                &font,
                "Mode",
                collision_mode_options(),
                mode_index,
                CollisionModeComboBox,
            );

            if has_mode {
                parent.spawn(fields_row()).with_children(|row| {
                    spawn_inspector_field(
                        row,
                        InspectorFieldProps::new("collision.use_scale").bool(),
                        &asset_server,
                    );
                });
                parent.spawn(fields_row()).with_children(|row| {
                    spawn_inspector_field(
                        row,
                        InspectorFieldProps::new("collision.base_size"),
                        &asset_server,
                    );
                });
            }

            if is_rigid {
                parent.spawn(fields_row()).with_children(|row| {
                    row.spawn((
                        FieldBinding::emitter_variant_field(
                            "collision.mode",
                            "friction",
                            FieldKind::F32,
                        ),
                        text_edit(
                            TextEditProps::default()
                                .with_label("Friction")
                                .numeric_f32(),
                        ),
                    ));
                    row.spawn((
                        FieldBinding::emitter_variant_field(
                            "collision.mode",
                            "bounce",
                            FieldKind::F32,
                        ),
                        text_edit(TextEditProps::default().with_label("Bounce").numeric_f32()),
                    ));
                });
            }
        })
        .id();

    commands.entity(entity).add_child(content);
}

fn handle_collision_mode_change(
    trigger: On<ComboBoxChangeEvent>,
    mut commands: Commands,
    collision_comboboxes: Query<(), With<CollisionModeComboBox>>,
    mut ew: EmitterWriter,
    existing: Query<Entity, With<CollisionContent>>,
) {
    if collision_comboboxes.get(trigger.entity).is_err() {
        return;
    }

    let new_mode = match trigger.value.as_deref().unwrap_or(&trigger.label) {
        "None" => None,
        "Rigid" => Some(EmitterCollisionMode::Rigid {
            friction: 0.0,
            bounce: 0.0,
        }),
        "HideOnContact" => Some(EmitterCollisionMode::HideOnContact),
        _ => return,
    };

    ew.modify_emitter(|emitter| {
        if collision_mode_index(&emitter.collision.mode) == collision_mode_index(&new_mode) {
            return false;
        }
        emitter.collision.mode = new_mode;
        true
    });

    for entity in &existing {
        commands.entity(entity).try_despawn();
    }
}
