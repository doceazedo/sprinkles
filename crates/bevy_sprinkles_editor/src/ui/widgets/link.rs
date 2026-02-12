use bevy::picking::hover::Hovered;
use bevy::prelude::*;
use bevy::text::TextLayoutInfo;
use bevy::window::SystemCursorIcon;

use crate::ui::widgets::cursor::HoverCursor;

const LINK_HIT_PADDING: f32 = 2.0;

#[derive(Component)]
pub struct LinkHitbox {
    pub text_entity: Entity,
    pub link_span_index: usize,
    pub link_span_entity: Entity,
    pub url: String,
    pub base_color: Color,
}

pub fn plugin(app: &mut App) {
    app.add_systems(
        Update,
        (position_link_hitboxes, handle_link_click, update_link_hover),
    );
}

pub fn spawn_link_hitbox(
    commands: &mut Commands,
    text_entity: Entity,
    link_span_index: usize,
    link_span_entity: Entity,
    url: String,
    base_color: Color,
) -> Entity {
    commands
        .spawn((
            Button,
            Hovered::default(),
            HoverCursor(SystemCursorIcon::Pointer),
            LinkHitbox {
                text_entity,
                link_span_index,
                link_span_entity,
                url,
                base_color,
            },
            Node {
                position_type: PositionType::Absolute,
                ..default()
            },
        ))
        .id()
}

fn position_link_hitboxes(
    mut hitboxes: Query<(&LinkHitbox, &mut Node)>,
    text_layouts: Query<(&TextLayoutInfo, &ComputedNode)>,
) {
    for (hitbox, mut node) in &mut hitboxes {
        let Ok((layout, computed)) = text_layouts.get(hitbox.text_entity) else {
            continue;
        };

        let scale = computed.inverse_scale_factor();
        let mut min_x = f32::MAX;
        let mut min_y = f32::MAX;
        let mut max_x = f32::MIN;
        let mut max_y = f32::MIN;
        let mut found = false;

        for glyph in &layout.glyphs {
            if glyph.span_index == hitbox.link_span_index {
                let w = glyph.size.x * scale;
                let h = glyph.size.y * scale;
                let x = glyph.position.x * scale - w / 2.0;
                let y = glyph.position.y * scale - h / 2.0;
                min_x = min_x.min(x);
                min_y = min_y.min(y);
                max_x = max_x.max(x + w);
                max_y = max_y.max(y + h);
                found = true;
            }
        }

        if !found {
            continue;
        }

        node.left = px(min_x - LINK_HIT_PADDING);
        node.top = px(min_y - LINK_HIT_PADDING);
        node.width = px(max_x - min_x + LINK_HIT_PADDING * 2.0);
        node.height = px(max_y - min_y + LINK_HIT_PADDING * 2.0);
    }
}

fn handle_link_click(interactions: Query<(&Interaction, &LinkHitbox), Changed<Interaction>>) {
    for (interaction, hitbox) in &interactions {
        if *interaction == Interaction::Pressed {
            let _ = open::that(&hitbox.url);
        }
    }
}

fn update_link_hover(
    hitboxes: Query<(&LinkHitbox, &Hovered), Changed<Hovered>>,
    mut text_colors: Query<&mut TextColor>,
) {
    for (hitbox, hovered) in &hitboxes {
        let Ok(mut color) = text_colors.get_mut(hitbox.link_span_entity) else {
            continue;
        };
        color.0 = if hovered.get() {
            hitbox.base_color.lighter(0.1)
        } else {
            hitbox.base_color
        };
    }
}
