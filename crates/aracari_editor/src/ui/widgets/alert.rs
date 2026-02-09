use bevy::color::palettes::tailwind;
use bevy::picking::hover::Hovered;
use bevy::prelude::*;
use bevy::text::TextLayoutInfo;
use bevy::window::SystemCursorIcon;

use crate::ui::tokens::{CORNER_RADIUS, FONT_PATH, TEXT_SIZE};
use crate::ui::widgets::cursor::HoverCursor;

#[derive(Component)]
pub struct EditorAlert;

#[derive(Default, Clone, Copy)]
pub enum AlertVariant {
    #[default]
    Info,
    Warning,
    Important,
}

impl AlertVariant {
    fn border_color(&self) -> Srgba {
        match self {
            Self::Info => tailwind::BLUE_500,
            Self::Warning => tailwind::YELLOW_500,
            Self::Important => tailwind::VIOLET_500,
        }
    }

    fn bg_color(&self) -> Color {
        match self {
            Self::Info => tailwind::BLUE_500.with_alpha(0.1).into(),
            Self::Warning => tailwind::YELLOW_500.with_alpha(0.1).into(),
            Self::Important => tailwind::VIOLET_500.with_alpha(0.1).into(),
        }
    }

    fn text_color(&self) -> Srgba {
        match self {
            Self::Info => tailwind::BLUE_400,
            Self::Warning => tailwind::YELLOW_400,
            Self::Important => tailwind::VIOLET_400,
        }
    }
}

const LINK_HIT_PADDING: f32 = 2.0;
const TEXT_ALPHA: f32 = 0.8;
const BOLD_ALPHA: f32 = 1.0;

#[derive(Clone)]
pub enum AlertSpan {
    Text(String),
    Bold(String),
    Link { text: String, url: String },
}

#[derive(Component)]
struct AlertConfig {
    variant: AlertVariant,
    spans: Vec<AlertSpan>,
}

#[derive(Component)]
struct AlertLinkHitbox {
    text_entity: Entity,
    link_span_index: usize,
    link_span_entity: Entity,
    url: String,
    base_color: Color,
}

pub fn plugin(app: &mut App) {
    app.add_systems(
        Update,
        (setup_alert, position_link_hitboxes, handle_link_click, update_link_hover),
    );
}

pub fn alert(variant: AlertVariant, spans: Vec<AlertSpan>) -> impl Bundle {
    (
        EditorAlert,
        AlertConfig { variant, spans },
        Node {
            width: percent(100),
            padding: UiRect::all(px(12.0)),
            border: UiRect::all(px(1.0)),
            border_radius: BorderRadius::all(CORNER_RADIUS),
            position_type: PositionType::Relative,
            ..default()
        },
        BackgroundColor(variant.bg_color()),
        BorderColor::all(variant.border_color()),
    )
}

fn setup_alert(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    alerts: Query<(Entity, &AlertConfig), Added<AlertConfig>>,
) {
    let font: Handle<Font> = asset_server.load(FONT_PATH);

    for (entity, config) in &alerts {
        let text_color = config.variant.text_color();

        let Some(first) = config.spans.first() else {
            continue;
        };

        let (first_text, first_weight, first_alpha) = span_props(first);
        let text_id = commands
            .spawn((
                Text::new(first_text),
                TextFont {
                    font: font.clone(),
                    font_size: TEXT_SIZE,
                    weight: first_weight,
                    ..default()
                },
                TextColor(text_color.with_alpha(first_alpha).into()),
            ))
            .id();

        let mut link_info = None;

        for (i, span) in config.spans.iter().skip(1).enumerate() {
            let span_index = i + 1;
            let (text, weight, alpha) = span_props(span);
            let color: Color = text_color.with_alpha(alpha).into();
            let mut span_cmd = commands.spawn((
                TextSpan::new(text),
                TextFont {
                    font: font.clone(),
                    font_size: TEXT_SIZE,
                    weight,
                    ..default()
                },
                TextColor(color),
            ));
            if let AlertSpan::Link { url, .. } = span {
                span_cmd.insert(Underline);
                link_info = Some((url.clone(), span_index, span_cmd.id(), color));
            }
            let span_id = span_cmd.id();
            commands.entity(text_id).add_child(span_id);
        }

        if let Some((url, link_span_index, link_span_entity, base_color)) = link_info {
            let wrapper = commands
                .spawn(Node {
                    width: percent(100),
                    position_type: PositionType::Relative,
                    ..default()
                })
                .id();

            let hitbox = commands
                .spawn((
                    Button,
                    Hovered::default(),
                    HoverCursor(SystemCursorIcon::Pointer),
                    AlertLinkHitbox {
                        text_entity: text_id,
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
                .id();

            commands.entity(wrapper).add_child(text_id);
            commands.entity(wrapper).add_child(hitbox);
            commands.entity(entity).add_child(wrapper);
        } else {
            commands.entity(entity).add_child(text_id);
        }
    }
}

fn position_link_hitboxes(
    mut hitboxes: Query<(&AlertLinkHitbox, &mut Node)>,
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

fn span_props(span: &AlertSpan) -> (&str, FontWeight, f32) {
    match span {
        AlertSpan::Text(t) => (t.as_str(), FontWeight::NORMAL, TEXT_ALPHA),
        AlertSpan::Bold(t) => (t.as_str(), FontWeight::MEDIUM, BOLD_ALPHA),
        AlertSpan::Link { text, .. } => (text.as_str(), FontWeight::MEDIUM, BOLD_ALPHA),
    }
}

fn handle_link_click(
    interactions: Query<(&Interaction, &AlertLinkHitbox), Changed<Interaction>>,
) {
    for (interaction, hitbox) in &interactions {
        if *interaction == Interaction::Pressed {
            let _ = open::that(&hitbox.url);
        }
    }
}

fn update_link_hover(
    hitboxes: Query<(&AlertLinkHitbox, &Hovered), Changed<Hovered>>,
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
