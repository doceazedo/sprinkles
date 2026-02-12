use bevy::prelude::*;
use sprinkles::prelude::*;

use crate::state::EditorState;
use crate::ui::tokens::BORDER_COLOR;
use crate::ui::widgets::button::{
    ButtonClickEvent, ButtonProps, ButtonVariant, EditorButton, IconButtonProps, button,
    icon_button, set_button_variant,
};
use crate::ui::widgets::inspector_field::{InspectorFieldProps, fields_row, spawn_inspector_field};
use crate::ui::widgets::panel_section::{
    PanelSectionAddButton, PanelSectionProps, PanelSectionSize, panel_section,
};
use crate::ui::widgets::popover::{
    PopoverHeaderProps, PopoverPlacement, PopoverProps, popover, popover_content, popover_header,
};
use crate::ui::widgets::vector_edit::VectorSuffixes;

use super::utils::name_to_label;
use super::{DynamicSectionContent, InspectorSection};
use crate::ui::components::binding::{EmitterWriter, get_inspecting_emitter};
use crate::ui::icons::{ICON_CLOSE, ICON_MORE};

const ANIMATED_VELOCITY_FIELDS: &[&str] = &["radial_velocity", "angular_velocity"];

pub fn plugin(app: &mut App) {
    app.add_systems(
        Update,
        (setup_velocity_list, update_add_button_state)
            .after(super::update_inspected_emitter_tracker),
    )
    .add_observer(handle_add_button_click)
    .add_observer(handle_add_velocity_option)
    .add_observer(handle_velocity_delete)
    .add_observer(handle_velocity_edit);
}

#[derive(Component)]
struct VelocityList {
    added: Vec<String>,
    initialized: bool,
}

#[derive(Component)]
struct VelocitySeparator;

#[derive(Component)]
struct VelocityListContainer;

#[derive(Component)]
struct VelocityItemRow(String);

#[derive(Component)]
struct VelocityDeleteButton(String);

#[derive(Component)]
struct VelocityEditButton(String);

#[derive(Component)]
struct VelocityEditPopover(Entity);

#[derive(Component)]
struct AddVelocityPopover;

#[derive(Component)]
struct AddVelocityOption(String);

pub fn velocities_section(asset_server: &AssetServer) -> impl Bundle {
    (
        VelocityList {
            added: Vec::new(),
            initialized: false,
        },
        InspectorSection::new(
            "Velocities",
            vec![
                vec![
                    InspectorFieldProps::new("velocities.initial_velocity")
                        .vector(VectorSuffixes::Range)
                        .into(),
                ],
                vec![
                    InspectorFieldProps::new("velocities.initial_direction")
                        .vector(VectorSuffixes::XYZ)
                        .into(),
                ],
                vec![
                    InspectorFieldProps::new("velocities.pivot")
                        .vector(VectorSuffixes::XYZ)
                        .into(),
                ],
                vec![
                    InspectorFieldProps::new("velocities.inherit_ratio").into(),
                    InspectorFieldProps::new("velocities.spread").into(),
                    InspectorFieldProps::new("velocities.flatness").into(),
                ],
            ],
        ),
        panel_section(
            PanelSectionProps::new("Velocities")
                .with_add_button()
                .collapsible()
                .with_size(PanelSectionSize::XL),
            asset_server,
        ),
    )
}

fn get_active_animated_velocities(
    editor_state: &EditorState,
    assets: &Assets<ParticleSystemAsset>,
) -> Vec<String> {
    let Some((_, emitter)) = get_inspecting_emitter(editor_state, assets) else {
        return Vec::new();
    };

    let mut active = Vec::new();
    for field_name in ANIMATED_VELOCITY_FIELDS {
        let value_path = format!(".velocities.{}.velocity", field_name);
        if let Ok(value) = emitter.reflect_path(value_path.as_str()) {
            if let Some(range) = value.try_downcast_ref::<ParticleRange>() {
                if range.min.abs() > f32::EPSILON || range.max.abs() > f32::EPSILON {
                    active.push(field_name.to_string());
                    continue;
                }
            }
        }
        let curve_path = format!(".velocities.{}.velocity_over_lifetime", field_name);
        if let Ok(value) = emitter.reflect_path(curve_path.as_str()) {
            if let Some(curve_opt) = value.try_downcast_ref::<Option<CurveTexture>>() {
                if curve_opt.is_some() {
                    active.push(field_name.to_string());
                }
            }
        }
    }
    active
}

fn remaining_velocity_fields(added: &[String]) -> Vec<&'static str> {
    ANIMATED_VELOCITY_FIELDS
        .iter()
        .filter(|f| !added.contains(&f.to_string()))
        .copied()
        .collect()
}

fn setup_velocity_list(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    editor_state: Res<EditorState>,
    assets: Res<Assets<ParticleSystemAsset>>,
    mut lists: Query<(Entity, &mut VelocityList, &InspectorSection)>,
    existing_containers: Query<Entity, With<VelocityListContainer>>,
) {
    for (entity, mut list, section) in &mut lists {
        if list.initialized && existing_containers.is_empty() {
            list.initialized = false;
        }

        if list.initialized || !section.initialized {
            continue;
        }
        list.initialized = true;

        let active = get_active_animated_velocities(&editor_state, &assets);
        list.added = active.clone();

        let has_items = !active.is_empty();

        let separator = commands
            .spawn((
                VelocitySeparator,
                DynamicSectionContent,
                Node {
                    width: percent(100),
                    height: px(1.0),
                    margin: UiRect::vertical(px(4.0)),
                    display: if has_items {
                        Display::Flex
                    } else {
                        Display::None
                    },
                    ..default()
                },
                BackgroundColor(BORDER_COLOR.into()),
            ))
            .id();
        commands.entity(entity).add_child(separator);

        let container = commands
            .spawn((
                VelocityListContainer,
                DynamicSectionContent,
                Node {
                    flex_direction: FlexDirection::Column,
                    width: percent(100),
                    row_gap: px(4.0),
                    ..default()
                },
            ))
            .id();
        for field_name in &active {
            let item = spawn_velocity_item(&mut commands, field_name, &asset_server);
            commands.entity(container).add_child(item);
        }
        commands.entity(entity).add_child(container);
    }
}

fn spawn_velocity_item(
    commands: &mut Commands,
    field_name: &str,
    asset_server: &AssetServer,
) -> Entity {
    let label = name_to_label(field_name);

    let row = commands
        .spawn((
            VelocityItemRow(field_name.to_string()),
            Node {
                width: percent(100),
                column_gap: px(4.0),
                align_items: AlignItems::Center,
                ..default()
            },
        ))
        .id();

    let edit_btn = commands
        .spawn((
            VelocityEditButton(field_name.to_string()),
            button(
                ButtonProps::new(&label)
                    .align_left()
                    .with_right_icon(ICON_MORE),
            ),
        ))
        .id();
    commands.entity(row).add_child(edit_btn);

    let delete_btn = commands
        .spawn((
            VelocityDeleteButton(field_name.to_string()),
            icon_button(
                IconButtonProps::new(ICON_CLOSE).variant(ButtonVariant::Ghost),
                asset_server,
            ),
        ))
        .id();
    commands.entity(row).add_child(delete_btn);

    row
}

fn update_separator(has_items: bool, separators: &mut Query<&mut Node, With<VelocitySeparator>>) {
    for mut node in separators.iter_mut() {
        node.display = if has_items {
            Display::Flex
        } else {
            Display::None
        };
    }
}

fn set_add_button_disabled(
    commands: &mut Commands,
    btn_entity: Entity,
    disabled: bool,
    button_styles: &mut Query<
        (&mut ButtonVariant, &mut BackgroundColor, &mut BorderColor),
        With<EditorButton>,
    >,
    children_query: &Query<&Children>,
    images: &mut Query<&mut ImageNode>,
) {
    let new_variant = if disabled {
        ButtonVariant::Disabled
    } else {
        ButtonVariant::Ghost
    };

    if let Ok((mut variant, mut bg, mut border)) = button_styles.get_mut(btn_entity) {
        if *variant == new_variant {
            return;
        }
        *variant = new_variant;
        set_button_variant(new_variant, &mut bg, &mut border);
    }

    let icon_color = new_variant.text_color();
    if let Ok(children) = children_query.get(btn_entity) {
        for child in children.iter() {
            if let Ok(mut image) = images.get_mut(child) {
                image.color = icon_color.into();
            }
        }
    }

    if disabled {
        commands.entity(btn_entity).remove::<Interaction>();
    } else {
        commands.entity(btn_entity).insert(Interaction::None);
    }
}

fn update_add_button_state(
    mut commands: Commands,
    lists: Query<(Entity, &VelocityList), Changed<VelocityList>>,
    add_buttons: Query<(Entity, &PanelSectionAddButton)>,
    mut button_styles: Query<
        (&mut ButtonVariant, &mut BackgroundColor, &mut BorderColor),
        With<EditorButton>,
    >,
    children_query: Query<&Children>,
    mut images: Query<&mut ImageNode>,
) {
    let Ok((entity, list)) = lists.single() else {
        return;
    };

    let Some(btn_entity) = add_buttons
        .iter()
        .find(|(_, btn)| btn.0 == entity)
        .map(|(e, _)| e)
    else {
        return;
    };

    let remaining = remaining_velocity_fields(&list.added);
    set_add_button_disabled(
        &mut commands,
        btn_entity,
        remaining.is_empty(),
        &mut button_styles,
        &children_query,
        &mut images,
    );
}

fn handle_add_button_click(
    trigger: On<ButtonClickEvent>,
    mut commands: Commands,
    lists: Query<(Entity, &VelocityList)>,
    add_buttons: Query<(Entity, &PanelSectionAddButton)>,
    existing_popovers: Query<(Entity, &AddVelocityPopover)>,
) {
    let Ok((section_entity, list)) = lists.get(trigger.entity) else {
        return;
    };

    let add_btn_entity = add_buttons
        .iter()
        .find(|(_, btn)| btn.0 == section_entity)
        .map(|(e, _)| e);
    let Some(anchor) = add_btn_entity else { return };

    for (popover_entity, _) in &existing_popovers {
        commands.entity(popover_entity).try_despawn();
        return;
    }

    let remaining = remaining_velocity_fields(&list.added);
    if remaining.is_empty() {
        return;
    }

    let popover_entity = commands
        .spawn((
            AddVelocityPopover,
            popover(
                PopoverProps::new(anchor)
                    .with_placement(PopoverPlacement::BottomEnd)
                    .with_padding(4.0)
                    .with_node(Node {
                        min_width: px(120.0),
                        ..default()
                    }),
            ),
        ))
        .id();

    for field_name in remaining {
        let label = name_to_label(field_name);
        commands.entity(popover_entity).with_child((
            AddVelocityOption(field_name.to_string()),
            button(
                ButtonProps::new(&label)
                    .with_variant(ButtonVariant::Ghost)
                    .align_left(),
            ),
        ));
    }
}

#[allow(clippy::too_many_arguments)]
fn handle_add_velocity_option(
    trigger: On<ButtonClickEvent>,
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut ew: EmitterWriter,
    options: Query<&AddVelocityOption>,
    mut lists: Query<&mut VelocityList>,
    list_containers: Query<Entity, With<VelocityListContainer>>,
    mut separators: Query<&mut Node, With<VelocitySeparator>>,
    popovers: Query<Entity, With<AddVelocityPopover>>,
) {
    let Ok(option) = options.get(trigger.entity) else {
        return;
    };
    let field_name = option.0.clone();

    ew.modify_emitter(|emitter| {
        let path = format!(".velocities.{}", field_name);
        if let Ok(target) = emitter.reflect_path_mut(path.as_str()) {
            if let Some(av) = target.try_downcast_mut::<AnimatedVelocity>() {
                *av = AnimatedVelocity {
                    velocity: ParticleRange::new(0.0, 1.0),
                    velocity_over_lifetime: None,
                };
                return true;
            }
        }
        false
    });

    for mut list in &mut lists {
        if !list.added.contains(&field_name) {
            list.added.push(field_name.clone());
        }
    }

    for container_entity in &list_containers {
        let item = spawn_velocity_item(&mut commands, &field_name, &asset_server);
        commands.entity(container_entity).add_child(item);
    }

    update_separator(true, &mut separators);

    for popover_entity in &popovers {
        commands.entity(popover_entity).try_despawn();
    }
}

fn handle_velocity_delete(
    trigger: On<ButtonClickEvent>,
    mut commands: Commands,
    mut ew: EmitterWriter,
    delete_buttons: Query<&VelocityDeleteButton>,
    mut lists: Query<&mut VelocityList>,
    item_rows: Query<(Entity, &VelocityItemRow)>,
    mut separators: Query<&mut Node, With<VelocitySeparator>>,
) {
    let Ok(delete_button) = delete_buttons.get(trigger.entity) else {
        return;
    };
    let field_name = delete_button.0.clone();

    ew.modify_emitter(|emitter| {
        let path = format!(".velocities.{}", field_name);
        if let Ok(target) = emitter.reflect_path_mut(path.as_str()) {
            if let Some(av) = target.try_downcast_mut::<AnimatedVelocity>() {
                *av = AnimatedVelocity::default();
                return true;
            }
        }
        false
    });

    for (row_entity, row) in &item_rows {
        if row.0 == field_name {
            commands.entity(row_entity).try_despawn();
        }
    }

    for mut list in &mut lists {
        list.added.retain(|n| *n != field_name);
        update_separator(!list.added.is_empty(), &mut separators);
    }
}

fn handle_velocity_edit(
    trigger: On<ButtonClickEvent>,
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    edit_buttons: Query<&VelocityEditButton>,
    existing_popovers: Query<(Entity, &VelocityEditPopover)>,
) {
    let Ok(edit_button) = edit_buttons.get(trigger.entity) else {
        return;
    };
    let field_name = edit_button.0.clone();

    for (popover_entity, popover_ref) in &existing_popovers {
        if popover_ref.0 == trigger.entity {
            commands.entity(popover_entity).try_despawn();
            return;
        }
    }

    let popover_title = name_to_label(&field_name);
    let value_path = format!("velocities.{}.velocity", field_name);
    let curve_path = format!("velocities.{}.velocity_over_lifetime", field_name);

    let popover_entity = commands
        .spawn((
            VelocityEditPopover(trigger.entity),
            popover(
                PopoverProps::new(trigger.entity)
                    .with_placement(PopoverPlacement::Right)
                    .with_padding(0.0)
                    .with_node(Node {
                        width: px(256.0),
                        min_width: px(256.0),
                        ..default()
                    }),
            ),
        ))
        .id();

    commands
        .entity(popover_entity)
        .with_child(popover_header(
            PopoverHeaderProps::new(&popover_title, popover_entity),
            &asset_server,
        ))
        .with_children(|parent| {
            parent.spawn(popover_content()).with_children(|content| {
                content.spawn(fields_row()).with_children(|row| {
                    spawn_inspector_field(
                        row,
                        InspectorFieldProps::new(&value_path).vector(VectorSuffixes::Range),
                        &asset_server,
                    );
                });
                content.spawn(fields_row()).with_children(|row| {
                    spawn_inspector_field(
                        row,
                        InspectorFieldProps::new(&curve_path).curve(),
                        &asset_server,
                    );
                });
            });
        });
}
