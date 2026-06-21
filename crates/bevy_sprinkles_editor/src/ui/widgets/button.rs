use bevy::color::palettes::tailwind;
use bevy::picking::hover::Hovered;
use bevy::prelude::*;
use bevy::text::FontSourceTemplate;

use crate::ui::tokens::{
    CORNER_RADIUS_LG, FONT_PATH, PRIMARY_COLOR, TEXT_BODY_COLOR, TEXT_DISPLAY_COLOR,
    TEXT_MUTED_COLOR, TEXT_SIZE, TEXT_SIZE_SM,
};

#[derive(EntityEvent)]
pub struct ButtonClickEvent {
    pub entity: Entity,
}

pub fn plugin(app: &mut App) {
    app.add_systems(Update, (handle_hover, handle_button_click));
}

#[derive(Component, Default, Clone)]
pub struct EditorButton;

#[derive(Component, Default, Clone, Copy, PartialEq)]
pub enum ButtonVariant {
    #[default]
    Default,
    Primary,
    Destructive,
    Ghost,
    Active,
    ActiveAlt,
    Disabled,
}

#[derive(Component, Default, Clone, Copy)]
pub enum ButtonSize {
    #[default]
    MD,
    Icon,
    IconSM,
}

impl ButtonVariant {
    pub fn bg_color(&self, hovered: bool) -> Srgba {
        match (self, hovered) {
            (Self::Default, _) => tailwind::ZINC_700,
            (Self::Ghost | Self::ActiveAlt | Self::Disabled, _) => TEXT_BODY_COLOR,
            (Self::Primary | Self::Active, _) => PRIMARY_COLOR,
            (Self::Destructive, false) => tailwind::RED_500,
            (Self::Destructive, true) => tailwind::RED_600,
        }
    }
    pub fn bg_opacity(&self, hovered: bool) -> f32 {
        match (self, hovered) {
            (Self::Ghost, false) | (Self::Disabled, _) => 0.0,
            (Self::Active, false) => 0.1,
            (Self::Active, true) => 0.15,
            (Self::ActiveAlt, _) => 0.05,
            (Self::Default, false) => 0.5,
            (Self::Default, true) => 0.8,
            (Self::Ghost, true) => 0.05,
            (Self::Primary | Self::Destructive, false) => 1.0,
            (Self::Primary | Self::Destructive, true) => 0.9,
        }
    }
    pub fn text_color(&self) -> Srgba {
        match self {
            Self::Default | Self::Ghost | Self::ActiveAlt => TEXT_BODY_COLOR,
            Self::Primary | Self::Destructive => TEXT_DISPLAY_COLOR,
            Self::Active => PRIMARY_COLOR.lighter(0.05),
            Self::Disabled => TEXT_MUTED_COLOR,
        }
    }
    pub fn border_color(&self) -> Srgba {
        match self {
            Self::Default | Self::Ghost | Self::Disabled => tailwind::ZINC_700,
            Self::Primary | Self::Active => PRIMARY_COLOR,
            Self::Destructive => tailwind::RED_500,
            Self::ActiveAlt => TEXT_BODY_COLOR,
        }
    }
    pub fn border(&self) -> Val {
        match self {
            Self::Default | Self::ActiveAlt => Val::Px(1.0),
            _ => Val::Px(0.0),
        }
    }
    pub fn border_opacity(&self, hovered: bool) -> f32 {
        match (self, hovered) {
            (Self::Ghost, false) | (Self::Disabled, _) => 0.0,
            (Self::ActiveAlt, _) => 0.2,
            _ => 1.0,
        }
    }
}

impl ButtonSize {
    fn width(&self) -> Val {
        match self {
            Self::Icon => Val::Px(28.0),
            Self::IconSM => Val::Px(24.0),
            Self::MD => Val::Auto,
        }
    }
    fn height(&self) -> Val {
        match self {
            Self::IconSM => Val::Px(24.0),
            _ => Val::Px(28.0),
        }
    }
    fn padding(&self) -> Val {
        match self {
            Self::MD => px(12.0),
            Self::Icon | Self::IconSM => px(0.0),
        }
    }
    fn icon_size(&self) -> Val {
        match self {
            Self::IconSM => Val::Px(14.0),
            _ => Val::Px(16.0),
        }
    }
}

#[derive(Default)]
pub struct ButtonProps {
    pub content: String,
    pub variant: ButtonVariant,
    pub size: ButtonSize,
    pub align_left: bool,
    pub left_icon: Option<String>,
    pub right_icon: Option<String>,
    pub direction: FlexDirection,
    pub subtitle: Option<String>,
}

impl ButtonProps {
    pub fn new(content: impl Into<String>) -> Self {
        Self {
            content: content.into(),
            ..default()
        }
    }
    pub fn with_variant(mut self, variant: ButtonVariant) -> Self {
        self.variant = variant;
        self
    }
    pub fn with_size(mut self, size: ButtonSize) -> Self {
        self.size = size;
        self
    }
    pub fn align_left(mut self) -> Self {
        self.align_left = true;
        self
    }
    pub fn with_left_icon(mut self, icon: impl Into<String>) -> Self {
        self.left_icon = Some(icon.into());
        self
    }
    pub fn with_right_icon(mut self, icon: impl Into<String>) -> Self {
        self.right_icon = Some(icon.into());
        self
    }
    pub fn with_direction(mut self, direction: FlexDirection) -> Self {
        self.direction = direction;
        self
    }
    pub fn with_subtitle(mut self, subtitle: impl Into<String>) -> Self {
        self.subtitle = Some(subtitle.into());
        self
    }
}

#[derive(Default)]
pub struct IconButtonProps {
    pub icon: String,
    pub color: Option<Srgba>,
    pub variant: ButtonVariant,
    pub size: ButtonSize,
    pub alpha: Option<f32>,
}

impl IconButtonProps {
    pub fn new(icon: impl Into<String>) -> Self {
        Self {
            icon: icon.into(),
            size: ButtonSize::Icon,
            ..default()
        }
    }
    pub fn color(mut self, color: Srgba) -> Self {
        self.color = Some(color);
        self
    }
    pub fn variant(mut self, variant: ButtonVariant) -> Self {
        self.variant = variant;
        self
    }
    pub fn with_alpha(mut self, alpha: f32) -> Self {
        self.alpha = Some(alpha);
        self
    }
    pub fn with_size(mut self, size: ButtonSize) -> Self {
        self.size = size;
        self
    }
}

pub(crate) fn button_base(
    variant: ButtonVariant,
    size: ButtonSize,
    align_left: bool,
    direction: FlexDirection,
) -> impl Bundle {
    let (node, bg, border_color) = button_base_parts(variant, size, align_left, direction);
    (
        Button,
        EditorButton,
        variant,
        size,
        Hovered::default(),
        node,
        BackgroundColor(bg.into()),
        BorderColor::all(border_color),
    )
}

fn button_base_parts(
    variant: ButtonVariant,
    size: ButtonSize,
    align_left: bool,
    direction: FlexDirection,
) -> (Node, Srgba, Srgba) {
    let is_column = direction == FlexDirection::Column;

    let node = Node {
        width: if align_left { percent(100) } else { size.width() },
        height: if is_column { Val::Auto } else { size.height() },
        padding: UiRect::axes(size.padding(), if is_column { px(6.0) } else { px(0.0) }),
        border: UiRect::all(variant.border()),
        border_radius: BorderRadius::all(CORNER_RADIUS_LG),
        flex_direction: direction,
        column_gap: px(6.0),
        row_gap: px(6.0),
        justify_content: if align_left {
            JustifyContent::Start
        } else {
            JustifyContent::Center
        },
        align_items: if is_column {
            AlignItems::Start
        } else {
            AlignItems::Center
        },
        ..default()
    };

    let bg = variant.bg_color(false).with_alpha(variant.bg_opacity(false));
    let border_color = variant
        .border_color()
        .with_alpha(variant.border_opacity(false));

    (node, bg, border_color)
}

pub fn button(props: ButtonProps) -> impl Scene {
    let ButtonProps {
        content,
        variant,
        size,
        align_left,
        left_icon,
        right_icon,
        direction,
        subtitle,
    } = props;

    let (mut node, bg, border_color) = button_base_parts(variant, size, align_left, direction);
    let is_column = direction == FlexDirection::Column;

    let left_padding = if left_icon.is_some() || is_column {
        px(6.0)
    } else {
        size.padding()
    };
    let right_padding = if right_icon.is_some() || is_column {
        px(6.0)
    } else {
        size.padding()
    };
    node.padding = UiRect::axes(left_padding, node.padding.top);
    node.padding.right = right_padding;

    let text_color = variant.text_color();
    let icon_size = size.icon_size();

    let mut children: Vec<Box<dyn SceneList>> = Vec::new();

    if let Some(icon) = left_icon {
        children.push(Box::new(bsn_list![(
            ImageNode {
                image: { icon },
                color: { Color::Srgba(text_color) },
            }
            Node {
                width: { icon_size },
                height: { icon_size },
            }
        )]) as Box<dyn SceneList>);
    }

    if !content.is_empty() {
        children.push(Box::new(bsn_list![(
            Text({ content })
            TextFont {
                font: { FontSourceTemplate::Handle(FONT_PATH.into()) },
                font_size: TEXT_SIZE,
                weight: { FontWeight::MEDIUM },
            }
            TextColor(text_color)
            Node {
                flex_grow: 1.0,
            }
        )]) as Box<dyn SceneList>);
    }

    if let Some(subtitle) = subtitle {
        children.push(Box::new(bsn_list![(
            Text({ subtitle })
            TextFont {
                font: { FontSourceTemplate::Handle(FONT_PATH.into()) },
                font_size: TEXT_SIZE_SM,
            }
            TextColor(TEXT_MUTED_COLOR)
            Node {
                margin: { UiRect::top(px(-6.0)) },
            }
        )]) as Box<dyn SceneList>);
    }

    if let Some(icon) = right_icon {
        children.push(Box::new(bsn_list![(
            ImageNode {
                image: { icon },
                color: { Color::Srgba(text_color) },
            }
            Node {
                width: { icon_size },
                height: { icon_size },
            }
        )]) as Box<dyn SceneList>);
    }

    bsn! {
        Button
        EditorButton
        template_value(variant)
        template_value(size)
        Hovered
        template_value(node)
        BackgroundColor({ bg })
        template_value(BorderColor::all(border_color))
        Children [ { children } ]
    }
}

fn handle_hover(
    mut buttons: Query<
        (
            &ButtonVariant,
            &Hovered,
            &mut BackgroundColor,
            &mut BorderColor,
        ),
        (Changed<Hovered>, With<EditorButton>),
    >,
) {
    for (variant, hovered, mut bg, mut border) in &mut buttons {
        let is_hovered = hovered.get();
        bg.0 = variant
            .bg_color(is_hovered)
            .with_alpha(variant.bg_opacity(is_hovered))
            .into();
        *border = BorderColor::all(
            variant
                .border_color()
                .with_alpha(variant.border_opacity(is_hovered)),
        );
    }
}

fn handle_button_click(
    interactions: Query<
        (Entity, &Interaction, &ButtonVariant),
        (Changed<Interaction>, With<EditorButton>),
    >,
    mut commands: Commands,
) {
    for (entity, interaction, variant) in &interactions {
        if *interaction == Interaction::Pressed && *variant != ButtonVariant::Disabled {
            commands.trigger(ButtonClickEvent { entity });
        }
    }
}

pub fn icon_button(props: IconButtonProps) -> impl Scene {
    let IconButtonProps {
        icon,
        color,
        variant,
        size,
        alpha,
    } = props;
    let alpha = alpha.unwrap_or(1.0);
    let icon_color = color.unwrap_or(variant.text_color()).with_alpha(alpha);
    let icon_size = size.icon_size();

    let (node, bg, border_color) = button_base_parts(variant, size, false, FlexDirection::Row);

    bsn! {
        Button
        EditorButton
        template_value(variant)
        template_value(size)
        Hovered
        template_value(node)
        BackgroundColor({ bg })
        template_value(BorderColor::all(border_color))
        Children [
            (
                ImageNode {
                    image: { icon },
                    color: { Color::Srgba(icon_color) },
                }
                Node {
                    width: { icon_size },
                    height: { icon_size },
                }
            )
        ]
    }
}

pub fn set_button_variant(
    variant: ButtonVariant,
    bg: &mut BackgroundColor,
    border: &mut BorderColor,
) {
    bg.0 = variant
        .bg_color(false)
        .with_alpha(variant.bg_opacity(false))
        .into();
    *border = BorderColor::all(
        variant
            .border_color()
            .with_alpha(variant.border_opacity(false)),
    );
}
