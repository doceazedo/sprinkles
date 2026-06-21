use std::time::Duration;

use bevy::color::palettes::tailwind;
use bevy::prelude::*;
use bevy::text::FontSourceTemplate;

use crate::ui::icons::{ICON_CHECKBOX_CIRCLE, ICON_CLOSE, ICON_CLOSE_CIRCLE, ICON_INFORMATION};
use crate::ui::tokens::{CORNER_RADIUS, FONT_PATH, TEXT_BODY_COLOR, TEXT_SIZE};
use crate::ui::widgets::button::{ButtonVariant, IconButtonProps, icon_button};

pub const TOAST_BOTTOM_OFFSET: f32 = 12.0;
pub const DEFAULT_TOAST_DURATION: Duration = Duration::from_millis(3000);

#[derive(Component, Default, Clone)]
pub struct EditorToast;

#[derive(Component)]
pub struct ToastCloseButton(pub Entity);

#[derive(Component, Default, Clone)]
pub struct ToastIcon;

#[derive(Component, Default, Clone)]
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

#[derive(Component, Default, Clone)]
pub struct ToastDuration(pub Timer);

pub fn toast(
    variant: ToastVariant,
    content: impl Into<String>,
    duration: Duration,
) -> impl Scene {
    let content: String = content.into();
    let icon = variant.icon();
    let bg = variant.bg_color().with_alpha(0.);

    bsn! {
        EditorToast
        template_value(variant)
        Interaction
        template_value(ToastDuration(Timer::new(duration, TimerMode::Once)))
        Node {
            position_type: { PositionType::Absolute },
            left: percent(50),
            bottom: px(TOAST_BOTTOM_OFFSET),
            column_gap: px(12),
            padding: { UiRect::axes(px(12), px(6)) },
            border: { UiRect::all(px(1)) },
            border_radius: { BorderRadius::all(CORNER_RADIUS) },
            box_sizing: { BoxSizing::BorderBox },
            align_items: { AlignItems::Center },
        }
        template_value(UiTransform {
            translation: Val2 {
                x: percent(-50),
                y: px(24),
            },
            scale: Vec2::splat(0.75),
            ..default()
        })
        BackgroundColor({ bg })
        template_value(BorderColor::all(TEXT_BODY_COLOR.with_alpha(0.)))
        Children [
            (
                ToastIcon
                ImageNode {
                    image: { icon },
                    color: { TEXT_BODY_COLOR.with_alpha(0.) },
                }
                Node {
                    width: px(18),
                    height: px(18),
                }
            ),
            (
                ToastText
                Text({ content })
                TextFont {
                    font: { FontSourceTemplate::Handle(FONT_PATH.into()) },
                    font_size: TEXT_SIZE,
                }
                TextColor({ TEXT_BODY_COLOR.with_alpha(0.) })
            ),
            (
                Node {
                    column_gap: px(6),
                    align_items: { AlignItems::Center },
                }
                Children [
                    Node {
                        width: px(1),
                    },
                    icon_button(
                        IconButtonProps::new(ICON_CLOSE)
                            .variant(ButtonVariant::Ghost)
                            .with_alpha(0.),
                    ),
                ]
            ),
        ]
    }
}
