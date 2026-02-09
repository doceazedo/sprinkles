use bevy::color::palettes::tailwind;
use bevy::input::mouse::{MouseMotion, MouseScrollUnit, MouseWheel};
use bevy::picking::hover::{HoverMap, Hovered};
use bevy::prelude::*;
use bevy::ui::UiGlobalTransform;
use bevy::window::SystemCursorIcon;

use crate::ui::tokens::{BACKGROUND_COLOR, BORDER_COLOR};
use crate::ui::widgets::cursor::{ActiveCursor, HoverCursor};

const RESIZE_HANDLE_WIDTH: u32 = 12;

const SCROLLBAR_SPEED: f32 = 24.0;
const SCROLLBAR_MIN_HEIGHT: f32 = 24.0;
const SCROLLBAR_WIDTH: f32 = 3.0;
const SCROLLBAR_MARGIN: f32 = 3.0;

pub fn plugin(app: &mut App) {
    app.add_systems(
        Update,
        (
            spawn_resize_handles,
            sync_resize_handle_positions,
            handle_resize_drag,
            send_scroll_events,
            update_scrollbar,
        ),
    )
    .add_observer(on_scroll_handler);
}

#[derive(EntityEvent, Debug)]
#[entity_event(propagate, auto_propagate)]
struct Scroll {
    entity: Entity,
    delta: Vec2,
}

fn send_scroll_events(
    mut mouse_wheel_reader: MessageReader<MouseWheel>,
    hover_map: Res<HoverMap>,
    mut commands: Commands,
) {
    for mouse_wheel in mouse_wheel_reader.read() {
        let mut delta = -Vec2::new(mouse_wheel.x, mouse_wheel.y);

        if mouse_wheel.unit == MouseScrollUnit::Line {
            delta *= SCROLLBAR_SPEED;
        }

        for pointer_map in hover_map.values() {
            for entity in pointer_map.keys().copied() {
                commands.trigger(Scroll { entity, delta });
            }
        }
    }
}

fn on_scroll_handler(
    mut scroll: On<Scroll>,
    mut query: Query<(&mut ScrollPosition, &Node, &ComputedNode)>,
) {
    let Ok((mut scroll_position, node, computed)) = query.get_mut(scroll.entity) else {
        return;
    };

    let max_offset = (computed.content_size() - computed.size()) * computed.inverse_scale_factor();
    let max_offset = max_offset.max(Vec2::ZERO);

    let delta = &mut scroll.delta;
    if node.overflow.x == OverflowAxis::Scroll && delta.x != 0. {
        let old_x = scroll_position.x;
        scroll_position.x = (scroll_position.x + delta.x).clamp(0., max_offset.x);
        if scroll_position.x != old_x {
            delta.x = 0.;
        }
    }

    if node.overflow.y == OverflowAxis::Scroll && delta.y != 0. {
        let old_y = scroll_position.y;
        scroll_position.y = (scroll_position.y + delta.y).clamp(0., max_offset.y);
        if scroll_position.y != old_y {
            delta.y = 0.;
        }
    }

    if *delta == Vec2::ZERO {
        scroll.propagate(false);
    }
}

#[derive(Component)]
pub struct EditorPanel;

#[derive(Component, Default, Clone, Copy, PartialEq, Eq)]
pub enum PanelDirection {
    #[default]
    Left,
    Right,
}

#[derive(Component)]
pub struct PanelWidth {
    pub current: u32,
    pub min: u32,
    pub max: u32,
}

#[derive(Component)]
pub struct PanelResizeHandle {
    pub panel: Entity,
    pub direction: PanelDirection,
}

#[derive(Component, Default)]
pub struct ResizeDragState {
    pub dragging: bool,
    pub accumulated_delta: f32,
}

pub struct PanelProps {
    pub direction: PanelDirection,
    pub width: u32,
    pub min_width: u32,
    pub max_width: u32,
}

impl Default for PanelProps {
    fn default() -> Self {
        Self {
            direction: PanelDirection::default(),
            width: 250,
            min_width: 100,
            max_width: 500,
        }
    }
}

impl PanelProps {
    pub fn new(direction: PanelDirection) -> Self {
        Self {
            direction,
            ..default()
        }
    }

    pub fn with_width(mut self, width: u32) -> Self {
        self.width = width;
        self
    }

    pub fn with_min_width(mut self, min_width: u32) -> Self {
        self.min_width = min_width;
        self
    }

    pub fn with_max_width(mut self, max_width: u32) -> Self {
        self.max_width = max_width;
        self
    }
}

pub fn panel(props: PanelProps) -> impl Bundle {
    let PanelProps {
        direction,
        width,
        min_width,
        max_width,
    } = props;

    let border = match direction {
        PanelDirection::Left => UiRect::right(px(1)),
        PanelDirection::Right => UiRect::left(px(1)),
    };
    let margin = match direction {
        PanelDirection::Left => UiRect::ZERO,
        PanelDirection::Right => UiRect::left(Val::Auto),
    };

    (
        EditorPanel,
        direction,
        PanelWidth {
            current: width,
            min: min_width,
            max: max_width,
        },
        Hovered::default(),
        Node {
            width: px(width),
            height: percent(100),
            min_height: px(0.0),
            flex_direction: FlexDirection::Column,
            border,
            margin,
            position_type: PositionType::Relative,
            overflow: Overflow::scroll_y(),
            ..default()
        },
        BackgroundColor(BACKGROUND_COLOR.into()),
        BorderColor::all(BORDER_COLOR),
    )
}

#[derive(Component)]
pub struct PanelScrollbar {
    pub panel: Entity,
}

pub fn panel_scrollbar(panel: Entity) -> impl Bundle {
    (
        PanelScrollbar { panel },
        Node {
            position_type: PositionType::Absolute,
            width: px(SCROLLBAR_WIDTH),
            right: px(SCROLLBAR_MARGIN),
            top: px(SCROLLBAR_MARGIN),
            border_radius: BorderRadius::all(px(SCROLLBAR_WIDTH / 2.0)),
            ..default()
        },
        IgnoreScroll(BVec2::new(false, true)),
        BackgroundColor(tailwind::ZINC_600.into()),
        Visibility::Hidden,
    )
}

fn update_scrollbar(
    panels: Query<(&Hovered, &ScrollPosition, &ComputedNode), With<EditorPanel>>,
    mut scrollbars: Query<(&PanelScrollbar, &mut Node, &mut Visibility)>,
) {
    for (scrollbar, mut node, mut visibility) in &mut scrollbars {
        let Ok((hovered, scroll_position, computed)) = panels.get(scrollbar.panel) else {
            continue;
        };

        let content_height = computed.content_size().y * computed.inverse_scale_factor();
        let visible_height = computed.size().y * computed.inverse_scale_factor();
        let has_scroll = content_height > visible_height;

        let should_show = hovered.get() && has_scroll;
        let new_visibility = if should_show {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };

        if *visibility != new_visibility {
            *visibility = new_visibility;
        }

        if !has_scroll {
            continue;
        }

        let track_height = visible_height - (SCROLLBAR_MARGIN * 2.0);
        let thumb_ratio = visible_height / content_height;
        let thumb_height = (track_height * thumb_ratio).max(SCROLLBAR_MIN_HEIGHT);

        let max_scroll = content_height - visible_height;
        let scroll_ratio = if max_scroll > 0.0 {
            scroll_position.y / max_scroll
        } else {
            0.0
        };
        let thumb_offset = scroll_ratio * (track_height - thumb_height);

        node.top = px(SCROLLBAR_MARGIN + thumb_offset);
        node.height = px(thumb_height);
    }
}

fn spawn_resize_handles(
    mut commands: Commands,
    panels: Query<(Entity, &PanelDirection, &ChildOf), Added<EditorPanel>>,
) {
    for (panel_entity, &direction, child_of) in &panels {
        let handle = commands
            .spawn((
                PanelResizeHandle {
                    panel: panel_entity,
                    direction,
                },
                ResizeDragState::default(),
                Hovered::default(),
                HoverCursor(SystemCursorIcon::ColResize),
                Node {
                    position_type: PositionType::Absolute,
                    width: px(RESIZE_HANDLE_WIDTH),
                    ..default()
                },
                ZIndex(10),
                Pickable {
                    should_block_lower: true,
                    is_hoverable: true,
                },
            ))
            .id();
        commands.entity(child_of.parent()).add_child(handle);
    }
}

fn sync_resize_handle_positions(
    panels: Query<(&PanelDirection, &UiGlobalTransform, &ComputedNode), With<EditorPanel>>,
    parents: Query<(&UiGlobalTransform, &ComputedNode), Without<EditorPanel>>,
    mut handles: Query<(&PanelResizeHandle, &ChildOf, &mut Node)>,
) {
    let half = (RESIZE_HANDLE_WIDTH / 2) as f32;

    for (handle, child_of, mut node) in &mut handles {
        let Ok((direction, panel_transform, panel_computed)) = panels.get(handle.panel) else {
            continue;
        };
        let Ok((parent_transform, parent_computed)) = parents.get(child_of.parent()) else {
            continue;
        };

        let scale = panel_computed.inverse_scale_factor();
        let panel_center = panel_transform.translation.x * scale;
        let parent_center = parent_transform.translation.x * scale;
        let panel_half_w = panel_computed.size().x * scale / 2.0;
        let parent_half_w = parent_computed.size().x * scale / 2.0;

        let parent_left = parent_center - parent_half_w;

        let panel_edge = match direction {
            PanelDirection::Left => panel_center + panel_half_w,
            PanelDirection::Right => panel_center - panel_half_w,
        };

        node.left = px(panel_edge - parent_left - half);
        node.top = px(0.0);
        node.height = percent(100);
    }
}

fn handle_resize_drag(
    mut commands: Commands,
    mut handles: Query<(Entity, &PanelResizeHandle, &mut ResizeDragState, &Hovered)>,
    mut panels: Query<(&mut Node, &mut PanelWidth), With<EditorPanel>>,
    mouse: Res<ButtonInput<MouseButton>>,
    mut mouse_motion: MessageReader<MouseMotion>,
) {
    let cursor_delta: f32 = mouse_motion.read().map(|e| e.delta.x).sum();

    for (entity, handle, mut drag_state, hovered) in &mut handles {
        if mouse.just_pressed(MouseButton::Left) && hovered.get() {
            drag_state.dragging = true;
            drag_state.accumulated_delta = 0.0;
            commands
                .entity(entity)
                .insert(ActiveCursor(SystemCursorIcon::ColResize));
        }

        if mouse.just_released(MouseButton::Left) {
            drag_state.dragging = false;
            commands.entity(entity).remove::<ActiveCursor>();
        }

        if drag_state.dragging && cursor_delta != 0.0 {
            if let Ok((mut node, mut panel_width)) = panels.get_mut(handle.panel) {
                let delta = match handle.direction {
                    PanelDirection::Left => cursor_delta,
                    PanelDirection::Right => -cursor_delta,
                };

                drag_state.accumulated_delta += delta;
                let new_width = ((panel_width.current as f32) + drag_state.accumulated_delta)
                    .clamp(panel_width.min as f32, panel_width.max as f32)
                    as u32;

                if new_width != panel_width.current {
                    drag_state.accumulated_delta = 0.0;
                    panel_width.current = new_width;
                    node.width = px(new_width);
                }
            }
        }
    }
}
