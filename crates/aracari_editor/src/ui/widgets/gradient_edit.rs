use aracari::prelude::{GradientStop, ParticleGradient};
use bevy::picking::events::Click;
use bevy::picking::hover::Hovered;
use bevy::picking::pointer::PointerButton;
use bevy::picking::prelude::Pickable;
use bevy::prelude::*;
use bevy::reflect::TypePath;
use bevy::render::render_resource::*;
use bevy::shader::ShaderRef;
use bevy::ui::UiGlobalTransform;
use bevy::window::{CursorIcon, SystemCursorIcon};

use crate::ui::tokens::{BORDER_COLOR, PRIMARY_COLOR};
use crate::ui::widgets::button::{
    ButtonClickEvent, ButtonProps, ButtonVariant, EditorButton, IconButtonProps, button, icon_button,
};
use crate::ui::widgets::color_picker::{
    ColorPickerChangeEvent, ColorPickerCommitEvent, ColorPickerProps, EditorColorPicker,
    color_picker,
};
use crate::ui::widgets::panel_section::{PanelSectionProps, panel_section};
use crate::ui::widgets::popover::{EditorPopover, PopoverPlacement, PopoverProps, popover};
use crate::ui::widgets::text_edit::{TextEditCommitEvent, TextEditProps, text_edit};
use bevy_ui_text_input::TextInputQueue;
use bevy_ui_text_input::actions::{TextInputAction, TextInputEdit};

const SHADER_GRADIENT_PATH: &str = "shaders/gradient_edit.wgsl";
const ICON_CLOSE: &str = "icons/ri-close-fill.png";
const BAR_HEIGHT: f32 = 24.0;
const HANDLE_SIZE: f32 = 24.0;
const HANDLE_ARROW_WIDTH: f32 = 8.0;
const HANDLE_ARROW_HEIGHT: f32 = 6.0;
const BORDER_RADIUS: f32 = 4.0;
const CHECKERBOARD_SIZE: f32 = 6.0;
const BAR_PADDING: f32 = 6.0;
pub(crate) const MAX_STOPS: usize = 8;

pub(crate) fn pack_gradient_stops(gradient: &ParticleGradient) -> (u32, [Vec4; 2], [Vec4; MAX_STOPS]) {
    let stop_count = gradient.stops.len().min(MAX_STOPS) as u32;
    let mut positions = [Vec4::ZERO; 2];
    let mut colors = [Vec4::ZERO; MAX_STOPS];

    for (i, stop) in gradient.stops.iter().take(MAX_STOPS).enumerate() {
        positions[i / 4][i % 4] = stop.position;
        colors[i] = Vec4::new(stop.color[0], stop.color[1], stop.color[2], stop.color[3]);
    }

    (stop_count, positions, colors)
}


pub fn plugin(app: &mut App) {
    app.add_plugins(UiMaterialPlugin::<GradientMaterial>::default())
        .add_observer(handle_add_stop_click)
        .add_observer(handle_delete_stop_click)
        .add_observer(handle_stop_position_commit)
        .add_observer(handle_stop_color_change)
        .add_observer(handle_stop_color_commit)
        .add_observer(handle_redistribute_click)
        .add_observer(handle_delete_menu_click)
        .add_observer(handle_handle_color_change)
        .add_observer(handle_handle_color_commit)
        .add_systems(
            Update,
            (
                setup_gradient_edit,
                fix_stop_row_sizing,
                update_gradient_visuals,
                update_handle_positions,
                update_handle_cursors,
                update_handle_colors,
                update_stop_position_inputs,
                handle_bar_right_click,
                handle_handle_right_click,
                respawn_stops_on_change,
            ),
        );
}

#[derive(Component)]
pub struct EditorGradientEdit;

#[derive(Component, Clone, Default)]
pub struct GradientEditState {
    pub gradient: ParticleGradient,
}

impl GradientEditState {
    pub fn from_gradient(gradient: ParticleGradient) -> Self {
        Self { gradient }
    }
}

#[derive(EntityEvent)]
pub struct GradientEditChangeEvent {
    pub entity: Entity,
    pub gradient: ParticleGradient,
}

#[derive(EntityEvent)]
pub struct GradientEditCommitEvent {
    pub entity: Entity,
    pub gradient: ParticleGradient,
}

fn trigger_gradient_events(commands: &mut Commands, entity: Entity, gradient: &ParticleGradient) {
    commands.trigger(GradientEditChangeEvent {
        entity,
        gradient: gradient.clone(),
    });
    commands.trigger(GradientEditCommitEvent {
        entity,
        gradient: gradient.clone(),
    });
}

#[derive(Default)]
pub struct GradientEditProps {
    pub gradient: Option<ParticleGradient>,
}

impl GradientEditProps {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_gradient(mut self, gradient: ParticleGradient) -> Self {
        self.gradient = Some(gradient);
        self
    }
}

pub fn gradient_edit(props: GradientEditProps) -> impl Bundle {
    let state = props
        .gradient
        .map(GradientEditState::from_gradient)
        .unwrap_or_default();

    (
        EditorGradientEdit,
        state,
        Node {
            flex_direction: FlexDirection::Column,
            row_gap: px(12.0),
            width: percent(100),
            ..default()
        },
    )
}

#[derive(Clone, Copy)]
struct StopRef {
    gradient_edit: Entity,
    index: usize,
}

impl StopRef {
    fn new(gradient_edit: Entity, index: usize) -> Self {
        Self {
            gradient_edit,
            index,
        }
    }
}

macro_rules! stop_ref_component {
    ($name:ident) => {
        #[derive(Component)]
        struct $name(StopRef);

        impl std::ops::Deref for $name {
            type Target = StopRef;
            fn deref(&self) -> &StopRef {
                &self.0
            }
        }
    };
}

#[derive(Component)]
struct GradientBar(Entity);

#[derive(Component)]
struct GradientMaterialNode(Entity);

#[derive(Component)]
struct HandleArea(Entity);

stop_ref_component!(StopHandle);

#[derive(Component)]
struct StopHandleSquare;

#[derive(Component)]
struct StopHandleArrow;

#[derive(Component)]
struct StopsSection(Entity);

#[derive(Component)]
struct StopRowsContainer(Entity);

stop_ref_component!(StopRow);
stop_ref_component!(StopPositionInput);
stop_ref_component!(StopColorPicker);
stop_ref_component!(DeleteStopButton);

#[derive(Component)]
struct HandleMenu;

stop_ref_component!(HandleColorPopover);
stop_ref_component!(HandleColorPicker);

#[derive(Component)]
struct RedistributeOption(Entity);

stop_ref_component!(DeleteMenuOption);

#[derive(Component, Default)]
struct Dragging;

#[derive(Component)]
struct JustDragged;

#[derive(AsBindGroup, Asset, TypePath, Debug, Clone)]
pub struct GradientMaterial {
    #[uniform(0)]
    pub border_radius: f32,
    #[uniform(0)]
    pub checkerboard_size: f32,
    #[uniform(0)]
    pub stop_count: u32,
    #[uniform(0)]
    pub interpolation: u32,
    #[uniform(0)]
    pub positions: [Vec4; 2],
    #[uniform(0)]
    pub colors: [Vec4; MAX_STOPS],
}

const SWATCH_CHECKERBOARD_SIZE: f32 = 4.0;
const SWATCH_BORDER_RADIUS: f32 = 4.0;
const LINEAR_INTERPOLATION: u32 = 1;

impl GradientMaterial {
    pub fn from_gradient(gradient: &ParticleGradient) -> Self {
        let (stop_count, positions, colors) = pack_gradient_stops(gradient);
        Self {
            border_radius: BORDER_RADIUS,
            checkerboard_size: CHECKERBOARD_SIZE,
            stop_count,
            interpolation: gradient.interpolation as u32,
            positions,
            colors,
        }
    }

    pub fn swatch(gradient: &ParticleGradient) -> Self {
        let (stop_count, positions, colors) = pack_gradient_stops(gradient);
        Self {
            border_radius: SWATCH_BORDER_RADIUS,
            checkerboard_size: SWATCH_CHECKERBOARD_SIZE,
            stop_count,
            interpolation: LINEAR_INTERPOLATION,
            positions,
            colors,
        }
    }
}

impl UiMaterial for GradientMaterial {
    fn fragment_shader() -> ShaderRef {
        SHADER_GRADIENT_PATH.into()
    }
}

fn setup_gradient_edit(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut gradient_materials: ResMut<Assets<GradientMaterial>>,
    gradient_edits: Query<(Entity, &GradientEditState), Added<EditorGradientEdit>>,
) {
    for (entity, state) in &gradient_edits {
        let bar_entity = commands
            .spawn((
                GradientBar(entity),
                Hovered::default(),
                Node {
                    height: px(BAR_HEIGHT),
                    ..default()
                },
            ))
            .id();

        commands.entity(bar_entity).with_children(|bar_parent| {
            bar_parent.spawn((
                GradientMaterialNode(entity),
                Pickable::IGNORE,
                MaterialNode(
                    gradient_materials.add(GradientMaterial::from_gradient(&state.gradient)),
                ),
                Node {
                    position_type: PositionType::Absolute,
                    left: px(0.0),
                    right: px(0.0),
                    height: percent(100),
                    ..default()
                },
            ));

            bar_parent
                .spawn((
                    HandleArea(entity),
                    Pickable::IGNORE,
                    Node {
                        position_type: PositionType::Absolute,
                        left: px(BAR_PADDING),
                        right: px(BAR_PADDING),
                        top: px(0.0),
                        bottom: px(0.0),
                        ..default()
                    },
                ))
                .with_children(|handle_parent| {
                    spawn_stop_handles(handle_parent, entity, &state.gradient);
                });
        });

        commands.entity(entity).add_child(bar_entity);

        let section_entity = commands
            .spawn((
                StopsSection(entity),
                panel_section(
                    PanelSectionProps::new("Stops").with_add_button(),
                    &asset_server,
                ),
            ))
            .id();

        // override panel_section's default padding/border
        commands.entity(section_entity).insert(Node {
            width: percent(100),
            flex_direction: FlexDirection::Column,
            row_gap: px(12),
            padding: UiRect::ZERO,
            border: UiRect::ZERO,
            ..default()
        });

        commands
            .entity(section_entity)
            .with_children(|section_parent| {
                section_parent
                    .spawn((
                        StopRowsContainer(entity),
                        Node {
                            flex_direction: FlexDirection::Column,
                            row_gap: px(6.0),
                            margin: UiRect::top(px(-6.0)),
                            width: percent(100),
                            ..default()
                        },
                    ))
                    .with_children(|rows_parent| {
                        spawn_stop_rows(rows_parent, entity, &state.gradient, &asset_server);
                    });
            });

        commands.entity(entity).add_child(section_entity);
    }
}

#[derive(Component)]
struct StopSizingApplied;

fn fix_stop_row_sizing(
    mut commands: Commands,
    mut position_inputs: Query<
        (Entity, &mut Node),
        (With<StopPositionInput>, Without<StopSizingApplied>, Without<StopColorPicker>),
    >,
    mut color_pickers: Query<
        (Entity, &mut Node, Option<&Children>),
        (With<StopColorPicker>, Without<StopSizingApplied>, Without<EditorButton>, Without<StopPositionInput>),
    >,
    mut button_nodes: Query<&mut Node, (With<EditorButton>, Without<StopColorPicker>, Without<StopPositionInput>)>,
) {
    for (entity, mut node) in &mut position_inputs {
        node.flex_grow = 0.0;
        node.flex_shrink = 0.0;
        node.flex_basis = Val::Auto;
        node.width = px(72.0);
        commands.entity(entity).insert(StopSizingApplied);
    }

    for (entity, mut node, children) in &mut color_pickers {
        node.flex_grow = 1.0;

        let trigger_fixed = children
            .iter()
            .flat_map(|c| c.iter())
            .any(|child| {
                if let Ok(mut button_node) = button_nodes.get_mut(child) {
                    button_node.flex_grow = 1.0;
                    true
                } else {
                    false
                }
            });

        if trigger_fixed {
            commands.entity(entity).insert(StopSizingApplied);
        }
    }
}

fn spawn_handle_square(parent: &mut ChildSpawnerCommands, color: [f32; 4]) {
    parent
        .spawn((
            StopHandleSquare,
            Pickable::IGNORE,
            Node {
                width: px(HANDLE_SIZE),
                height: px(HANDLE_SIZE),
                border_radius: BorderRadius::all(px(4.0)),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
            BackgroundColor(BORDER_COLOR.into()),
        ))
        .with_children(|square| {
            square.spawn((
                Pickable::IGNORE,
                Node {
                    width: px(HANDLE_SIZE - 6.0),
                    height: px(HANDLE_SIZE - 6.0),
                    border_radius: BorderRadius::all(px(2.0)),
                    ..default()
                },
                BackgroundColor(Color::linear_rgba(color[0], color[1], color[2], color[3])),
            ));
        });
}

fn spawn_handle_arrow(parent: &mut ChildSpawnerCommands) {
    let arrow_square_size = HANDLE_ARROW_WIDTH * 0.8;
    let arrow_offset = (HANDLE_ARROW_HEIGHT - arrow_square_size) / 2.0;
    parent
        .spawn((
            Pickable::IGNORE,
            Node {
                width: px(HANDLE_ARROW_WIDTH),
                height: px(HANDLE_ARROW_HEIGHT),
                overflow: Overflow::clip(),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
        ))
        .with_children(|arrow_container| {
            arrow_container.spawn((
                StopHandleArrow,
                Pickable::IGNORE,
                Node {
                    width: px(arrow_square_size),
                    height: px(arrow_square_size),
                    margin: UiRect::bottom(px(-arrow_square_size + arrow_offset)),
                    border_radius: BorderRadius::all(px(2.0)),
                    ..default()
                },
                UiTransform {
                    rotation: Rot2::degrees(45.0),
                    ..default()
                },
                BackgroundColor(BORDER_COLOR.into()),
            ));
        });
}

fn spawn_stop_handles(
    parent: &mut ChildSpawnerCommands,
    gradient_edit: Entity,
    gradient: &ParticleGradient,
) {
    for (i, stop) in gradient.stops.iter().enumerate() {
        parent
            .spawn((
                StopHandle(StopRef::new(gradient_edit, i)),
                Pickable::default(),
                Hovered::default(),
                Interaction::None,
                Node {
                    position_type: PositionType::Absolute,
                    width: px(HANDLE_SIZE),
                    height: px(HANDLE_SIZE + HANDLE_ARROW_HEIGHT),
                    left: percent(stop.position * 100.0),
                    margin: UiRect::left(px(-HANDLE_SIZE / 2.0)),
                    top: px(BAR_HEIGHT / 2.0 - HANDLE_ARROW_HEIGHT),
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::FlexStart,
                    flex_direction: FlexDirection::Column,
                    ..default()
                },
            ))
            .with_children(|handle| {
                spawn_handle_arrow(handle);
                spawn_handle_square(handle, stop.color);
            })
            .observe(on_handle_click)
            .observe(on_handle_drag_start)
            .observe(on_handle_drag)
            .observe(on_handle_drag_end);
    }
}

fn spawn_stop_rows(
    parent: &mut ChildSpawnerCommands,
    gradient_edit: Entity,
    gradient: &ParticleGradient,
    asset_server: &AssetServer,
) {
    for (i, stop) in gradient.stops.iter().enumerate() {
        let can_delete = gradient.stops.len() > 1;
        let position_percent = (stop.position * 100.0).round() as i32;

        parent
            .spawn((
                StopRow(StopRef::new(gradient_edit, i)),
                Node {
                    width: percent(100),
                    column_gap: px(8.0),
                    align_items: AlignItems::Center,
                    ..default()
                },
            ))
            .with_children(|row| {
                row.spawn((
                    StopPositionInput(StopRef::new(gradient_edit, i)),
                    text_edit(
                        TextEditProps::default()
                            .numeric_i32()
                            .with_min(0.0)
                            .with_max(100.0)
                            .with_suffix("%")
                            .with_default_value(position_percent.to_string()),
                    ),
                ));

                row.spawn((
                    StopColorPicker(StopRef::new(gradient_edit, i)),
                    color_picker(ColorPickerProps::new().with_color(stop.color)),
                ));

                let delete_variant = if can_delete {
                    ButtonVariant::Ghost
                } else {
                    ButtonVariant::Disabled
                };

                row.spawn((
                    DeleteStopButton(StopRef::new(gradient_edit, i)),
                    icon_button(
                        IconButtonProps::new(ICON_CLOSE).variant(delete_variant),
                        asset_server,
                    ),
                ));
            });
    }
}

fn on_handle_click(
    event: On<Pointer<Click>>,
    mut commands: Commands,
    handles: Query<(&StopHandle, Has<Dragging>, Has<JustDragged>)>,
    states: Query<&GradientEditState>,
    existing_popovers: Query<Entity, With<HandleColorPopover>>,
    all_popovers: Query<Entity, With<EditorPopover>>,
) {
    if event.button != PointerButton::Primary {
        return;
    }

    let Ok((handle, is_dragging, just_dragged)) = handles.get(event.event_target()) else {
        return;
    };

    if is_dragging || just_dragged {
        commands
            .entity(event.event_target())
            .remove::<JustDragged>();
        return;
    }

    for popover_entity in &existing_popovers {
        commands.entity(popover_entity).try_despawn();
    }

    if !all_popovers.is_empty() {
        return;
    }

    let Ok(state) = states.get(handle.gradient_edit) else {
        return;
    };

    let Some(stop) = state.gradient.stops.get(handle.index) else {
        return;
    };

    let popover_entity = commands
        .spawn((
            HandleColorPopover(StopRef::new(handle.gradient_edit, handle.index)),
            popover(
                PopoverProps::new(event.event_target())
                    .with_placement(PopoverPlacement::Top)
                    .with_padding(12.0)
                    .with_z_index(300),
            ),
        ))
        .id();

    commands.entity(popover_entity).with_children(|parent| {
        parent.spawn((
            HandleColorPicker(StopRef::new(handle.gradient_edit, handle.index)),
            color_picker(ColorPickerProps::new().with_color(stop.color).inline()),
        ));
    });
}

fn on_handle_drag_start(
    event: On<Pointer<DragStart>>,
    mut commands: Commands,
    handles: Query<&StopHandle>,
) {
    if event.button != PointerButton::Primary {
        return;
    }
    let Ok(_handle) = handles.get(event.event_target()) else {
        return;
    };
    commands.entity(event.event_target()).insert(Dragging);
}

fn bar_position_from_normalized(normalized_x: f32, bar_width: f32) -> f32 {
    let content_width = bar_width - BAR_PADDING * 2.0;
    if content_width > 0.0 {
        let pixel_x = (normalized_x + 0.5) * bar_width;
        ((pixel_x - BAR_PADDING) / content_width).clamp(0.0, 1.0)
    } else {
        0.0
    }
}

fn on_handle_drag(
    event: On<Pointer<Drag>>,
    mut commands: Commands,
    handles: Query<&StopHandle, With<Dragging>>,
    bars: Query<(&GradientBar, &ComputedNode, &UiGlobalTransform)>,
    mut states: Query<&mut GradientEditState>,
) {
    if event.button != PointerButton::Primary {
        return;
    }
    let Ok(handle) = handles.get(event.event_target()) else {
        return;
    };

    let Some((_, computed, ui_transform)) = bars
        .iter()
        .find(|(bar, _, _)| bar.0 == handle.gradient_edit)
    else {
        return;
    };

    let cursor_pos = event.pointer_location.position / computed.inverse_scale_factor;
    let Some(normalized) = computed.normalize_point(*ui_transform, cursor_pos) else {
        return;
    };

    let Ok(mut state) = states.get_mut(handle.gradient_edit) else {
        return;
    };

    let new_pos = bar_position_from_normalized(normalized.x, computed.size.x);

    let prev_pos = if handle.index > 0 {
        state.gradient.stops[handle.index - 1].position + 0.001
    } else {
        0.0
    };
    let next_pos = if handle.index < state.gradient.stops.len() - 1 {
        state.gradient.stops[handle.index + 1].position - 0.001
    } else {
        1.0
    };
    let clamped_pos = new_pos.clamp(prev_pos, next_pos);

    state.gradient.stops[handle.index].position = clamped_pos;

    commands.trigger(GradientEditChangeEvent {
        entity: handle.gradient_edit,
        gradient: state.gradient.clone(),
    });
}

fn on_handle_drag_end(
    event: On<Pointer<DragEnd>>,
    mut commands: Commands,
    handles: Query<&StopHandle>,
    states: Query<&GradientEditState>,
) {
    if event.button != PointerButton::Primary {
        return;
    }
    let Ok(handle) = handles.get(event.event_target()) else {
        return;
    };

    commands
        .entity(event.event_target())
        .remove::<Dragging>()
        .insert(JustDragged);

    if let Ok(state) = states.get(handle.gradient_edit) {
        commands.trigger(GradientEditCommitEvent {
            entity: handle.gradient_edit,
            gradient: state.gradient.clone(),
        });
    }
}

fn update_handle_stop_color(
    commands: &mut Commands,
    handle_pickers: &Query<&HandleColorPicker>,
    states: &mut Query<&mut GradientEditState>,
    trigger_entity: Entity,
    color: [f32; 4],
    commit: bool,
) {
    let Ok(picker) = handle_pickers.get(trigger_entity) else {
        return;
    };

    let Ok(mut state) = states.get_mut(picker.gradient_edit) else {
        return;
    };

    if picker.index >= state.gradient.stops.len() {
        return;
    }

    state.gradient.stops[picker.index].color = color;

    if commit {
        trigger_gradient_events(commands, picker.gradient_edit, &state.gradient);
    } else {
        commands.trigger(GradientEditChangeEvent {
            entity: picker.gradient_edit,
            gradient: state.gradient.clone(),
        });
    }
}

fn handle_handle_color_change(
    trigger: On<ColorPickerChangeEvent>,
    mut commands: Commands,
    handle_pickers: Query<&HandleColorPicker>,
    mut states: Query<&mut GradientEditState>,
) {
    update_handle_stop_color(&mut commands, &handle_pickers, &mut states, trigger.entity, trigger.color, false);
}

fn handle_handle_color_commit(
    trigger: On<ColorPickerCommitEvent>,
    mut commands: Commands,
    handle_pickers: Query<&HandleColorPicker>,
    mut states: Query<&mut GradientEditState>,
) {
    update_handle_stop_color(&mut commands, &handle_pickers, &mut states, trigger.entity, trigger.color, true);
}

fn update_gradient_visuals(
    states: Query<(Entity, &GradientEditState), Changed<GradientEditState>>,
    material_nodes: Query<(&GradientMaterialNode, &MaterialNode<GradientMaterial>)>,
    mut gradient_materials: ResMut<Assets<GradientMaterial>>,
) {
    for (gradient_edit_entity, state) in &states {
        for (mat_node, material_node) in &material_nodes {
            if mat_node.0 != gradient_edit_entity {
                continue;
            }
            if let Some(material) = gradient_materials.get_mut(&material_node.0) {
                *material = GradientMaterial::from_gradient(&state.gradient);
            }
        }
    }
}

fn update_handle_positions(
    states: Query<(Entity, &GradientEditState), Changed<GradientEditState>>,
    mut handles: Query<(&StopHandle, &mut Node, &Children)>,
    children_query: Query<&Children>,
    mut bg_colors: Query<&mut BackgroundColor>,
) {
    for (gradient_edit_entity, state) in &states {
        for (handle, mut node, children) in &mut handles {
            if handle.gradient_edit != gradient_edit_entity {
                continue;
            }
            let Some(stop) = state.gradient.stops.get(handle.index) else {
                continue;
            };

            node.left = percent(stop.position * 100.0);

            // handle children: [arrow, square]; square -> color_indicator
            if let Some(&square_entity) = children.get(1) {
                if let Ok(square_children) = children_query.get(square_entity) {
                    if let Some(&color_indicator) = square_children.first() {
                        if let Ok(mut bg) = bg_colors.get_mut(color_indicator) {
                            *bg = BackgroundColor(Color::linear_rgba(
                                stop.color[0],
                                stop.color[1],
                                stop.color[2],
                                stop.color[3],
                            ));
                        }
                    }
                }
            }
        }
    }
}

fn update_stop_position_inputs(
    states: Query<(Entity, &GradientEditState), Changed<GradientEditState>>,
    position_inputs: Query<(&StopPositionInput, &Children)>,
    children_query: Query<&Children>,
    mut text_queues: Query<&mut TextInputQueue>,
) {
    for (gradient_edit_entity, state) in &states {
        for (input, input_children) in &position_inputs {
            if input.gradient_edit != gradient_edit_entity {
                continue;
            }

            let Some(stop) = state.gradient.stops.get(input.index) else {
                continue;
            };

            let position_percent = (stop.position * 100.0).round() as i32;
            let text = position_percent.to_string();

            // hierarchy: StopPositionInput -> wrapper -> text_input (with TextInputQueue)
            for wrapper_entity in input_children.iter() {
                let Ok(wrapper_children) = children_query.get(wrapper_entity) else {
                    continue;
                };
                for text_input_entity in wrapper_children.iter() {
                    if let Ok(mut queue) = text_queues.get_mut(text_input_entity) {
                        queue.add(TextInputAction::Edit(TextInputEdit::SelectAll));
                        queue.add(TextInputAction::Edit(TextInputEdit::Paste(text.clone())));
                        break;
                    }
                }
            }
        }
    }
}

fn update_handle_cursors(
    mut commands: Commands,
    window: Single<(Entity, Option<&CursorIcon>), With<Window>>,
    handles: Query<(&Hovered, Has<Dragging>), With<StopHandle>>,
) {
    let (window_entity, current_cursor) = *window;

    let mut new_cursor: Option<SystemCursorIcon> = None;

    for (hovered, is_dragging) in &handles {
        if is_dragging {
            new_cursor = Some(SystemCursorIcon::Grabbing);
            break;
        } else if hovered.get() && new_cursor.is_none() {
            new_cursor = Some(SystemCursorIcon::Grab);
        }
    }

    if let Some(cursor) = new_cursor {
        commands
            .entity(window_entity)
            .insert(CursorIcon::from(cursor));
    } else if current_cursor.is_some_and(|c| {
        matches!(
            c,
            CursorIcon::System(SystemCursorIcon::Grab | SystemCursorIcon::Grabbing)
        )
    }) {
        commands.entity(window_entity).remove::<CursorIcon>();
    }
}

fn update_handle_colors(
    mut removed_dragging: RemovedComponents<Dragging>,
    handles: Query<
        (Entity, &Hovered, Has<Dragging>),
        (With<StopHandle>, Or<(Changed<Hovered>, Added<Dragging>)>),
    >,
    handles_all: Query<(Entity, &Hovered), With<StopHandle>>,
    mut squares: Query<(&ChildOf, &mut BackgroundColor), With<StopHandleSquare>>,
    mut arrows: Query<
        (&ChildOf, &mut BackgroundColor),
        (With<StopHandleArrow>, Without<StopHandleSquare>),
    >,
    children_query: Query<&Children>,
) {
    let removed: Vec<Entity> = removed_dragging.read().collect();

    // collect entities that need color updates
    let mut updates: Vec<(Entity, Srgba)> = Vec::new();

    for (entity, hovered, is_dragging) in &handles {
        if removed.contains(&entity) {
            continue;
        }
        let color = if is_dragging {
            PRIMARY_COLOR
        } else if hovered.get() {
            PRIMARY_COLOR.lighter(0.1)
        } else {
            BORDER_COLOR
        };
        updates.push((entity, color));
    }

    for entity in removed {
        if let Ok((_, hovered)) = handles_all.get(entity) {
            let color = if hovered.get() {
                PRIMARY_COLOR.lighter(0.1)
            } else {
                BORDER_COLOR
            };
            updates.push((entity, color));
        }
    }

    // apply color updates to squares
    for (child_of, mut bg) in &mut squares {
        let handle_entity = child_of.parent();
        if let Some((_, color)) = updates.iter().find(|(e, _)| *e == handle_entity) {
            *bg = BackgroundColor((*color).into());
        }
    }

    // apply color updates to arrows (arrows are grandchildren: handle -> container -> arrow)
    for (arrow_child_of, mut bg) in &mut arrows {
        let container_entity = arrow_child_of.parent();
        // find the handle that owns this container
        for (handle_entity, color) in &updates {
            if let Ok(handle_children) = children_query.get(*handle_entity) {
                if handle_children.contains(&container_entity) {
                    *bg = BackgroundColor((*color).into());
                    break;
                }
            }
        }
    }
}

fn handle_bar_right_click(
    mut commands: Commands,
    mouse: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window>,
    bars: Query<(&GradientBar, &ComputedNode, &UiGlobalTransform, &Hovered)>,
    mut states: Query<&mut GradientEditState>,
    handles: Query<&Hovered, With<StopHandle>>,
) {
    if !mouse.just_pressed(MouseButton::Right) {
        return;
    }

    let handle_hovered = handles.iter().any(|h| h.get());
    if handle_hovered {
        return;
    }

    let Ok(window) = windows.single() else {
        return;
    };
    let Some(cursor_position) = window.cursor_position() else {
        return;
    };

    for (bar, computed, ui_transform, hovered) in &bars {
        if !hovered.get() {
            continue;
        }

        let Ok(mut state) = states.get_mut(bar.0) else {
            continue;
        };

        if state.gradient.stops.len() >= MAX_STOPS {
            continue;
        }

        let cursor_pos = cursor_position / computed.inverse_scale_factor;
        let Some(normalized) = computed.normalize_point(*ui_transform, cursor_pos) else {
            continue;
        };

        let position = bar_position_from_normalized(normalized.x, computed.size.x);

        let left_color = state
            .gradient
            .stops
            .iter()
            .rev()
            .find(|s| s.position <= position)
            .map(|s| s.color)
            .unwrap_or([1.0, 1.0, 1.0, 1.0]);

        let right_color = state
            .gradient
            .stops
            .iter()
            .find(|s| s.position >= position)
            .map(|s| s.color)
            .unwrap_or([1.0, 1.0, 1.0, 1.0]);

        let t = 0.5;
        let color = [
            left_color[0] + (right_color[0] - left_color[0]) * t,
            left_color[1] + (right_color[1] - left_color[1]) * t,
            left_color[2] + (right_color[2] - left_color[2]) * t,
            left_color[3] + (right_color[3] - left_color[3]) * t,
        ];

        let new_stop = GradientStop { color, position };

        let insert_idx = state
            .gradient
            .stops
            .iter()
            .position(|s| s.position > position)
            .unwrap_or(state.gradient.stops.len());

        state.gradient.stops.insert(insert_idx, new_stop);
        trigger_gradient_events(&mut commands, bar.0, &state.gradient);

        break;
    }
}

fn handle_handle_right_click(
    mut commands: Commands,
    mouse: Res<ButtonInput<MouseButton>>,
    handles: Query<(Entity, &StopHandle, &Hovered)>,
    states: Query<&GradientEditState>,
    existing_menus: Query<Entity, With<HandleMenu>>,
) {
    if !mouse.just_pressed(MouseButton::Right) {
        return;
    }

    for menu_entity in &existing_menus {
        commands.entity(menu_entity).try_despawn();
    }

    for (handle_entity, handle, hovered) in &handles {
        if !hovered.get() {
            continue;
        }

        let Ok(state) = states.get(handle.gradient_edit) else {
            continue;
        };

        let can_delete = state.gradient.stops.len() > 1;

        let popover_entity = commands
            .spawn((
                HandleMenu,
                popover(
                    PopoverProps::new(handle_entity)
                        .with_placement(PopoverPlacement::BottomStart)
                        .with_padding(4.0)
                        .with_z_index(300),
                ),
            ))
            .id();

        commands.entity(popover_entity).with_children(|parent| {
            parent.spawn((
                RedistributeOption(handle.gradient_edit),
                button(
                    ButtonProps::new("Redistribute stops")
                        .with_variant(ButtonVariant::Ghost)
                        .align_left(),
                ),
            ));

            parent.spawn((
                Node {
                    width: percent(100),
                    height: px(1.0),
                    margin: UiRect::vertical(px(4.0)),
                    ..default()
                },
                BackgroundColor(BORDER_COLOR.into()),
            ));

            let delete_variant = if can_delete {
                ButtonVariant::Ghost
            } else {
                ButtonVariant::Disabled
            };

            parent.spawn((
                DeleteMenuOption(StopRef::new(handle.gradient_edit, handle.index)),
                button(
                    ButtonProps::new("Delete")
                        .with_variant(delete_variant)
                        .align_left(),
                ),
            ));
        });

        break;
    }
}

fn handle_add_stop_click(
    trigger: On<ButtonClickEvent>,
    mut commands: Commands,
    stops_sections: Query<&StopsSection>,
    mut states: Query<&mut GradientEditState>,
) {
    let Ok(section) = stops_sections.get(trigger.entity) else {
        return;
    };

    let Ok(mut state) = states.get_mut(section.0) else {
        return;
    };

    if state.gradient.stops.len() >= MAX_STOPS {
        return;
    }

    let position: f32;
    let color: [f32; 4];

    if state.gradient.stops.len() == 1 {
        let existing = &state.gradient.stops[0];
        if existing.position < 0.5 {
            position = 1.0;
        } else {
            position = 0.0;
        }
        color = existing.color;
    } else {
        let last = &state.gradient.stops[state.gradient.stops.len() - 1];
        let second_last = &state.gradient.stops[state.gradient.stops.len() - 2];
        position = (second_last.position + last.position) / 2.0;
        color = [
            (second_last.color[0] + last.color[0]) / 2.0,
            (second_last.color[1] + last.color[1]) / 2.0,
            (second_last.color[2] + last.color[2]) / 2.0,
            (second_last.color[3] + last.color[3]) / 2.0,
        ];
    }

    let new_stop = GradientStop { color, position };

    let insert_idx = state
        .gradient
        .stops
        .iter()
        .position(|s| s.position > position)
        .unwrap_or(state.gradient.stops.len());

    state.gradient.stops.insert(insert_idx, new_stop);
    trigger_gradient_events(&mut commands, section.0, &state.gradient);
}

fn handle_delete_stop_click(
    trigger: On<ButtonClickEvent>,
    mut commands: Commands,
    delete_buttons: Query<&DeleteStopButton>,
    mut states: Query<&mut GradientEditState>,
) {
    let Ok(delete_button) = delete_buttons.get(trigger.entity) else {
        return;
    };

    let Ok(mut state) = states.get_mut(delete_button.gradient_edit) else {
        return;
    };

    if state.gradient.stops.len() <= 1 {
        return;
    }

    state.gradient.stops.remove(delete_button.index);
    trigger_gradient_events(&mut commands, delete_button.gradient_edit, &state.gradient);
}

fn handle_redistribute_click(
    trigger: On<ButtonClickEvent>,
    mut commands: Commands,
    redistribute_options: Query<&RedistributeOption>,
    mut states: Query<&mut GradientEditState>,
    menus: Query<Entity, With<HandleMenu>>,
) {
    let Ok(option) = redistribute_options.get(trigger.entity) else {
        return;
    };

    let Ok(mut state) = states.get_mut(option.0) else {
        return;
    };

    let count = state.gradient.stops.len();
    if count < 2 {
        return;
    }

    for (i, stop) in state.gradient.stops.iter_mut().enumerate() {
        stop.position = i as f32 / (count - 1) as f32;
    }

    trigger_gradient_events(&mut commands, option.0, &state.gradient);

    for menu in &menus {
        commands.entity(menu).try_despawn();
    }
}

fn handle_delete_menu_click(
    trigger: On<ButtonClickEvent>,
    mut commands: Commands,
    delete_options: Query<&DeleteMenuOption>,
    mut states: Query<&mut GradientEditState>,
    menus: Query<Entity, With<HandleMenu>>,
) {
    let Ok(option) = delete_options.get(trigger.entity) else {
        return;
    };

    let Ok(mut state) = states.get_mut(option.gradient_edit) else {
        return;
    };

    if state.gradient.stops.len() <= 1 {
        return;
    }

    state.gradient.stops.remove(option.index);
    trigger_gradient_events(&mut commands, option.gradient_edit, &state.gradient);

    for menu in &menus {
        commands.entity(menu).try_despawn();
    }
}

fn handle_stop_position_commit(
    trigger: On<TextEditCommitEvent>,
    mut commands: Commands,
    position_inputs: Query<&StopPositionInput>,
    mut states: Query<&mut GradientEditState>,
    parents: Query<&ChildOf>,
) {
    // hierarchy: StopPositionInput -> wrapper -> text_input (trigger.entity)
    let wrapper_entity = parents
        .get(trigger.entity)
        .map(|p| p.parent())
        .unwrap_or(trigger.entity);
    let input_entity = parents
        .get(wrapper_entity)
        .map(|p| p.parent())
        .unwrap_or(wrapper_entity);
    let Ok(input) = position_inputs.get(input_entity) else {
        return;
    };

    let Ok(mut state) = states.get_mut(input.gradient_edit) else {
        return;
    };

    let Ok(value) = trigger.text.trim().trim_end_matches('%').parse::<f32>() else {
        return;
    };

    let position = (value / 100.0).clamp(0.0, 1.0);

    if input.index >= state.gradient.stops.len() {
        return;
    }

    state.gradient.stops[input.index].position = position;

    state.gradient.stops.sort_by(|a, b| {
        a.position
            .partial_cmp(&b.position)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    trigger_gradient_events(&mut commands, input.gradient_edit, &state.gradient);
}

fn update_stop_color(
    commands: &mut Commands,
    color_pickers: &Query<&StopColorPicker>,
    states: &mut Query<&mut GradientEditState>,
    trigger_entity: Entity,
    color: [f32; 4],
    commit: bool,
) {
    let Ok(picker) = color_pickers.get(trigger_entity) else {
        return;
    };

    let Ok(mut state) = states.get_mut(picker.gradient_edit) else {
        return;
    };

    if picker.index >= state.gradient.stops.len() {
        return;
    }

    state.gradient.stops[picker.index].color = color;

    if commit {
        trigger_gradient_events(commands, picker.gradient_edit, &state.gradient);
    } else {
        commands.trigger(GradientEditChangeEvent {
            entity: picker.gradient_edit,
            gradient: state.gradient.clone(),
        });
    }
}

fn handle_stop_color_change(
    trigger: On<ColorPickerChangeEvent>,
    mut commands: Commands,
    color_pickers: Query<&StopColorPicker>,
    mut states: Query<&mut GradientEditState>,
) {
    update_stop_color(&mut commands, &color_pickers, &mut states, trigger.entity, trigger.color, false);
}

fn handle_stop_color_commit(
    trigger: On<ColorPickerCommitEvent>,
    mut commands: Commands,
    color_pickers: Query<&StopColorPicker>,
    mut states: Query<&mut GradientEditState>,
) {
    update_stop_color(&mut commands, &color_pickers, &mut states, trigger.entity, trigger.color, true);
}

fn respawn_stops_on_change(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    states: Query<(Entity, &GradientEditState), Changed<GradientEditState>>,
    containers: Query<(Entity, &StopRowsContainer)>,
    stop_rows: Query<(Entity, &StopRow)>,
    stop_handles: Query<(Entity, &StopHandle)>,
    handle_areas: Query<(Entity, &HandleArea)>,
) {
    for (gradient_edit_entity, state) in &states {
        let current_stop_count = state.gradient.stops.len();

        let row_count = stop_rows
            .iter()
            .filter(|(_, r)| r.gradient_edit == gradient_edit_entity)
            .count();

        if row_count != current_stop_count {
            for (container_entity, container) in &containers {
                if container.0 != gradient_edit_entity {
                    continue;
                }

                for (row_entity, row) in &stop_rows {
                    if row.gradient_edit == gradient_edit_entity {
                        commands.entity(row_entity).despawn();
                    }
                }

                commands.entity(container_entity).with_children(|parent| {
                    spawn_stop_rows(parent, gradient_edit_entity, &state.gradient, &asset_server);
                });

                break;
            }
        }

        let handle_count = stop_handles
            .iter()
            .filter(|(_, h)| h.gradient_edit == gradient_edit_entity)
            .count();

        if handle_count != current_stop_count {
            for (area_entity, area) in &handle_areas {
                if area.0 != gradient_edit_entity {
                    continue;
                }

                for (handle_entity, handle) in &stop_handles {
                    if handle.gradient_edit == gradient_edit_entity {
                        commands.entity(handle_entity).despawn();
                    }
                }

                commands.entity(area_entity).with_children(|parent| {
                    spawn_stop_handles(parent, gradient_edit_entity, &state.gradient);
                });

                break;
            }
        }
    }
}
