use bevy::color::palettes::tailwind;
use bevy::picking::hover::Hovered;
use bevy::prelude::*;

use crate::ui::tokens::{
    CORNER_RADIUS_LG, FONT_PATH, PRIMARY_COLOR, TEXT_BODY_COLOR, TEXT_DISPLAY_COLOR, TEXT_MUTED_COLOR, TEXT_SIZE,
    TEXT_SIZE_SM,
};

#[derive(EntityEvent)]
pub struct ButtonClickEvent {
    pub entity: Entity,
}

pub fn plugin(app: &mut App) {
    app.add_systems(Update, (setup_button, handle_hover, handle_button_click));
}

#[derive(Component)]
pub struct EditorButton;

#[derive(Component, Default, Clone, Copy, PartialEq)]
pub enum ButtonVariant {
    #[default]
    Default,
    Primary,
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
}

impl ButtonVariant {
    pub fn bg_color(&self) -> Srgba {
        match self {
            Self::Default => tailwind::ZINC_700,
            Self::Ghost | Self::ActiveAlt | Self::Disabled => TEXT_BODY_COLOR,
            Self::Primary | Self::Active => PRIMARY_COLOR,
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
            (Self::Primary, false) => 1.0,
            (Self::Primary, true) => 0.9,
        }
    }
    pub fn text_color(&self) -> Srgba {
        match self {
            Self::Default | Self::Ghost | Self::ActiveAlt => TEXT_BODY_COLOR,
            Self::Primary => TEXT_DISPLAY_COLOR,
            Self::Active => PRIMARY_COLOR.lighter(0.05),
            Self::Disabled => TEXT_MUTED_COLOR,
        }
    }
    pub fn border_color(&self) -> Srgba {
        match self {
            Self::Default | Self::Ghost | Self::Disabled => tailwind::ZINC_700,
            Self::Primary | Self::Active => PRIMARY_COLOR,
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
            Self::MD => Val::Auto,
        }
    }
    fn height(&self) -> Val {
        Val::Px(28.0)
    }
    fn padding(&self) -> Val {
        match self {
            Self::MD => px(12.0),
            Self::Icon => px(0.0),
        }
    }
    fn icon_size(&self) -> Val {
        Val::Px(16.0)
    }
}

#[derive(Component)]
struct ButtonConfig {
    content: String,
    left_icon: Option<String>,
    right_icon: Option<String>,
    subtitle: Option<String>,
    initialized: bool,
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

fn button_base(
    variant: ButtonVariant,
    size: ButtonSize,
    align_left: bool,
    direction: FlexDirection,
) -> impl Bundle {
    let is_column = direction == FlexDirection::Column;

    (
        Button,
        EditorButton,
        variant,
        size,
        Hovered::default(),
        Node {
            width: if align_left {
                percent(100)
            } else {
                size.width()
            },
            height: if is_column { Val::Auto } else { size.height() },
            padding: UiRect::axes(
                size.padding(),
                if is_column { px(6.0) } else { px(0.0) },
            ),
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
        },
        BackgroundColor(
            variant
                .bg_color()
                .with_alpha(variant.bg_opacity(false))
                .into(),
        ),
        BorderColor::all(
            variant
                .border_color()
                .with_alpha(variant.border_opacity(false)),
        ),
    )
}

pub fn button(props: ButtonProps) -> impl Bundle {
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

    (
        button_base(variant, size, align_left, direction),
        ButtonConfig {
            content,
            left_icon,
            right_icon,
            subtitle,
            initialized: false,
        },
    )
}

fn setup_button(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut buttons: Query<
        (Entity, &mut ButtonConfig, &ButtonVariant, &ButtonSize, &mut Node),
        Added<ButtonConfig>,
    >,
) {
    let font: Handle<Font> = asset_server.load(FONT_PATH);

    for (entity, mut config, variant, size, mut node) in &mut buttons {
        if config.initialized {
            continue;
        }
        config.initialized = true;

        let is_column = node.flex_direction == FlexDirection::Column;
        let left_padding = if config.left_icon.is_some() || is_column {
            px(6.0)
        } else {
            size.padding()
        };
        let right_padding = if config.right_icon.is_some() || is_column {
            px(6.0)
        } else {
            size.padding()
        };
        node.padding = UiRect::axes(left_padding, node.padding.top);
        node.padding.right = right_padding;

        commands.entity(entity).with_children(|parent| {
            if let Some(ref icon_path) = config.left_icon {
                parent.spawn((
                    ImageNode::new(asset_server.load(icon_path))
                        .with_color(variant.text_color().into()),
                    Node {
                        width: size.icon_size(),
                        height: size.icon_size(),
                        ..default()
                    },
                ));
            }

            if !config.content.is_empty() {
                parent.spawn((
                    Text::new(&config.content),
                    TextFont {
                        font: font.clone(),
                        font_size: TEXT_SIZE,
                        weight: FontWeight::MEDIUM,
                        ..default()
                    },
                    TextColor(variant.text_color().into()),
                    Node {
                        flex_grow: 1.0,
                        ..default()
                    },
                ));
            }

            if let Some(ref subtitle) = config.subtitle {
                parent.spawn((
                    Text::new(subtitle),
                    TextFont {
                        font: font.clone(),
                        font_size: TEXT_SIZE_SM,
                        ..default()
                    },
                    TextColor(TEXT_MUTED_COLOR.into()),
                    Node {
                        margin: UiRect::top(px(-6.0)),
                        ..default()
                    },
                ));
            }

            if let Some(ref icon_path) = config.right_icon {
                parent.spawn((
                    ImageNode::new(asset_server.load(icon_path))
                        .with_color(variant.text_color().into()),
                    Node {
                        width: size.icon_size(),
                        height: size.icon_size(),
                        ..default()
                    },
                ));
            }
        });
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
            .bg_color()
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

pub fn icon_button(props: IconButtonProps, asset_server: &AssetServer) -> impl Bundle {
    let IconButtonProps {
        icon,
        color,
        variant,
        size,
        alpha,
    } = props;
    let alpha = alpha.unwrap_or(1.0);
    let icon_color = color.unwrap_or(variant.text_color()).with_alpha(alpha);

    (
        button_base(variant, size, false, FlexDirection::Row),
        children![(
            ImageNode::new(asset_server.load(&icon)).with_color(Color::Srgba(icon_color)),
            Node {
                width: size.icon_size(),
                height: size.icon_size(),
                ..default()
            },
        )],
    )
}

pub fn set_button_variant(
    variant: ButtonVariant,
    bg: &mut BackgroundColor,
    border: &mut BorderColor,
) {
    bg.0 = variant
        .bg_color()
        .with_alpha(variant.bg_opacity(false))
        .into();
    *border = BorderColor::all(
        variant
            .border_color()
            .with_alpha(variant.border_opacity(false)),
    );
}
