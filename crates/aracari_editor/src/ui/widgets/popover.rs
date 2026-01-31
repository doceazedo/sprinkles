use bevy::picking::hover::Hovered;
use bevy::prelude::*;
use bevy::ui::UiGlobalTransform;
use bevy::window::PrimaryWindow;

use crate::ui::tokens::{BACKGROUND_COLOR, BORDER_COLOR, CORNER_RADIUS_LG};

const POPOVER_GAP: f32 = 4.0;

pub fn plugin(app: &mut App) {
    app.add_systems(Update, (handle_popover_position, handle_popover_dismiss));
}

#[derive(Component)]
pub struct EditorPopover;

#[derive(Component)]
pub struct PopoverAnchor(pub Entity);

#[derive(Component, Default, Clone, Copy, PartialEq)]
pub enum PopoverPlacement {
    TopStart,
    Top,
    TopEnd,
    RightStart,
    Right,
    RightEnd,
    #[default]
    BottomStart,
    Bottom,
    BottomEnd,
    LeftStart,
    Left,
    LeftEnd,
}

impl PopoverPlacement {
    fn offset(&self, anchor_size: Vec2, popover_size: Vec2) -> Vec2 {
        match self {
            Self::TopStart => Vec2::new(0.0, -popover_size.y - POPOVER_GAP),
            Self::Top => Vec2::new(
                (anchor_size.x - popover_size.x) / 2.0,
                -popover_size.y - POPOVER_GAP,
            ),
            Self::TopEnd => Vec2::new(
                anchor_size.x - popover_size.x,
                -popover_size.y - POPOVER_GAP,
            ),
            Self::RightStart => Vec2::new(anchor_size.x + POPOVER_GAP, 0.0),
            Self::Right => Vec2::new(
                anchor_size.x + POPOVER_GAP,
                (anchor_size.y - popover_size.y) / 2.0,
            ),
            Self::RightEnd => {
                Vec2::new(anchor_size.x + POPOVER_GAP, anchor_size.y - popover_size.y)
            }
            Self::BottomStart => Vec2::new(0.0, anchor_size.y + POPOVER_GAP),
            Self::Bottom => Vec2::new(
                (anchor_size.x - popover_size.x) / 2.0,
                anchor_size.y + POPOVER_GAP,
            ),
            Self::BottomEnd => {
                Vec2::new(anchor_size.x - popover_size.x, anchor_size.y + POPOVER_GAP)
            }
            Self::LeftStart => Vec2::new(-popover_size.x - POPOVER_GAP, 0.0),
            Self::Left => Vec2::new(
                -popover_size.x - POPOVER_GAP,
                (anchor_size.y - popover_size.y) / 2.0,
            ),
            Self::LeftEnd => Vec2::new(
                -popover_size.x - POPOVER_GAP,
                anchor_size.y - popover_size.y,
            ),
        }
    }

    fn flip(&self) -> Self {
        match self {
            Self::TopStart => Self::BottomStart,
            Self::Top => Self::Bottom,
            Self::TopEnd => Self::BottomEnd,
            Self::RightStart => Self::LeftStart,
            Self::Right => Self::Left,
            Self::RightEnd => Self::LeftEnd,
            Self::BottomStart => Self::TopStart,
            Self::Bottom => Self::Top,
            Self::BottomEnd => Self::TopEnd,
            Self::LeftStart => Self::RightStart,
            Self::Left => Self::Right,
            Self::LeftEnd => Self::RightEnd,
        }
    }
}

pub struct PopoverProps {
    pub placement: PopoverPlacement,
    pub anchor: Entity,
    pub node: Option<Node>,
    pub padding: f32,
}

impl PopoverProps {
    pub fn new(anchor: Entity) -> Self {
        Self {
            placement: PopoverPlacement::default(),
            anchor,
            node: None,
            padding: 6.0,
        }
    }

    pub fn with_placement(mut self, placement: PopoverPlacement) -> Self {
        self.placement = placement;
        self
    }

    pub fn with_node(mut self, node: Node) -> Self {
        self.node = Some(node);
        self
    }

    pub fn with_padding(mut self, padding: f32) -> Self {
        self.padding = padding;
        self
    }
}

pub fn popover(props: PopoverProps) -> impl Bundle {
    let PopoverProps {
        placement,
        anchor,
        node,
        padding,
    } = props;

    let base_node = node.unwrap_or_default();

    (
        EditorPopover,
        PopoverAnchor(anchor),
        placement,
        Hovered::default(),
        Interaction::None,
        Node {
            position_type: PositionType::Absolute,
            padding: UiRect::all(px(padding)),
            border: UiRect::all(px(1.0)),
            border_radius: BorderRadius::all(CORNER_RADIUS_LG),
            flex_direction: FlexDirection::Column,
            ..base_node
        },
        Visibility::Hidden,
        BackgroundColor(BACKGROUND_COLOR.into()),
        BorderColor::all(BORDER_COLOR),
        ZIndex(100),
    )
}

fn handle_popover_position(
    mut popovers: Query<
        (
            &PopoverAnchor,
            &PopoverPlacement,
            &ComputedNode,
            &mut Node,
            &mut Visibility,
        ),
        With<EditorPopover>,
    >,
    anchors: Query<(&ComputedNode, &UiGlobalTransform)>,
    windows: Query<&Window, With<PrimaryWindow>>,
) {
    let Ok(window) = windows.single() else {
        return;
    };
    let window_size = Vec2::new(window.width(), window.height());

    for (anchor_ref, placement, popover_computed, mut popover_node, mut visibility) in &mut popovers
    {
        let Ok((anchor_computed, anchor_transform)) = anchors.get(anchor_ref.0) else {
            continue;
        };

        let scale = anchor_computed.inverse_scale_factor();
        let anchor_center = anchor_transform.translation * scale;
        let anchor_size = anchor_computed.size() * scale;
        let popover_size = popover_computed.size() * popover_computed.inverse_scale_factor();

        if popover_size.x == 0.0 || popover_size.y == 0.0 {
            continue;
        }

        let anchor_top_left = Vec2::new(
            anchor_center.x - anchor_size.x * 0.5,
            anchor_center.y - anchor_size.y * 0.5,
        );

        let mut pos = anchor_top_left + placement.offset(anchor_size, popover_size);

        if pos.x < 0.0
            || pos.x + popover_size.x > window_size.x
            || pos.y < 0.0
            || pos.y + popover_size.y > window_size.y
        {
            let flipped = placement.flip();
            let flipped_pos = anchor_top_left + flipped.offset(anchor_size, popover_size);

            if flipped_pos.x >= 0.0
                && flipped_pos.x + popover_size.x <= window_size.x
                && flipped_pos.y >= 0.0
                && flipped_pos.y + popover_size.y <= window_size.y
            {
                pos = flipped_pos;
            }
        }

        pos.x = pos.x.clamp(0.0, (window_size.x - popover_size.x).max(0.0));
        pos.y = pos.y.clamp(0.0, (window_size.y - popover_size.y).max(0.0));

        popover_node.left = px(pos.x);
        popover_node.top = px(pos.y);
        *visibility = Visibility::Visible;
    }
}

fn handle_popover_dismiss(
    mut commands: Commands,
    popovers: Query<(Entity, &Hovered), With<EditorPopover>>,
    keyboard: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
) {
    let esc_pressed = keyboard.just_pressed(KeyCode::Escape);
    let clicked = mouse.get_just_pressed().next().is_some();

    if !esc_pressed && !clicked {
        return;
    }

    for (entity, hovered) in &popovers {
        let should_dismiss = esc_pressed || (clicked && !hovered.get());
        if should_dismiss {
            commands.entity(entity).try_despawn();
        }
    }
}
