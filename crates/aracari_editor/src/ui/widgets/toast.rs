use std::time::Duration;

use bevy::color::palettes::tailwind;
use bevy::prelude::*;

use crate::ui::icons::{ICON_CHECKBOX_CIRCLE, ICON_CLOSE, ICON_CLOSE_CIRCLE, ICON_INFORMATION};
use crate::ui::tokens::{CORNER_RADIUS, FONT_PATH, TEXT_BODY_COLOR, TEXT_SIZE};
use crate::ui::widgets::button::{ButtonVariant, IconButtonProps, icon_button};
use crate::ui::widgets::separator::{SeparatorProps, separator};

pub const TOAST_BOTTOM_OFFSET: f32 = 12.0;
pub const DEFAULT_TOAST_DURATION: Duration = Duration::from_millis(3000);

#[derive(Component)]
pub struct EditorToast;

#[derive(Component)]
pub struct ToastCloseButton(pub Entity);

#[derive(Component)]
pub struct ToastIcon;

#[derive(Component)]
pub struct ToastText;

#[derive(Component, Default, Clone, Copy)]
pub enum ToastVariant {
    #[default]
    Info,
    Success,
    Error,
}

impl ToastVariant {
    pub fn bg_color(&self) -> Srgba {
        match self {
            Self::Info => tailwind::ZINC_700,
            Self::Success => tailwind::GREEN_800,
            Self::Error => tailwind::RED_800,
        }
    }

    pub fn icon(&self) -> &'static str {
        match self {
            Self::Info => ICON_INFORMATION,
            Self::Success => ICON_CHECKBOX_CIRCLE,
            Self::Error => ICON_CLOSE_CIRCLE,
        }
    }
}

#[derive(Component)]
pub struct ToastDuration(pub Timer);

pub fn toast(
    variant: ToastVariant,
    content: impl Into<String>,
    duration: Duration,
    asset_server: &AssetServer,
) -> impl Bundle {
    let font: Handle<Font> = asset_server.load(FONT_PATH);

    (
        EditorToast,
        variant,
        Interaction::None,
        ToastDuration(Timer::new(duration, TimerMode::Once)),
        Node {
            position_type: PositionType::Absolute,
            left: percent(50),
            bottom: px(TOAST_BOTTOM_OFFSET),
            column_gap: px(12),
            padding: UiRect::axes(px(12), px(6)),
            border: UiRect::all(px(1)),
            border_radius: BorderRadius::all(CORNER_RADIUS),
            box_sizing: BoxSizing::BorderBox,
            align_items: AlignItems::Center,
            ..default()
        },
        UiTransform {
            translation: Val2 {
                x: percent(-50),
                y: px(24),
            },
            scale: Vec2::splat(0.75),
            ..default()
        },
        BackgroundColor(variant.bg_color().with_alpha(0.).into()),
        BorderColor::all(TEXT_BODY_COLOR.with_alpha(0.)),
        children![
            (
                ToastIcon,
                ImageNode::new(asset_server.load(variant.icon()))
                    .with_color(TEXT_BODY_COLOR.with_alpha(0.).into()),
                Node {
                    width: px(18),
                    height: px(18),
                    ..default()
                },
            ),
            (
                ToastText,
                Text::new(content),
                TextFont {
                    font: font.into(),
                    font_size: TEXT_SIZE,
                    ..default()
                },
                TextColor(TEXT_BODY_COLOR.with_alpha(0.).into()),
            ),
            (
                Node {
                    column_gap: px(6),
                    align_items: AlignItems::Center,
                    ..default()
                },
                children![
                    separator(SeparatorProps::vertical().with_alpha(0.)),
                    icon_button(
                        IconButtonProps::new(ICON_CLOSE)
                            .variant(ButtonVariant::Ghost)
                            .with_alpha(0.),
                        asset_server
                    ),
                ],
            ),
        ],
    )
}
