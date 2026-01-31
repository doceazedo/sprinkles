use bevy::color::palettes::tailwind;
use bevy::picking::hover::Hovered;
use bevy::prelude::*;

use crate::ui::tokens::{
    CORNER_RADIUS_LG, FONT_PATH, PRIMARY_COLOR, TEXT_BODY_COLOR, TEXT_DISPLAY_COLOR, TEXT_SIZE,
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
            Self::Ghost | Self::ActiveAlt => TEXT_BODY_COLOR,
            Self::Primary | Self::Active => PRIMARY_COLOR,
        }
    }
    pub fn bg_opacity(&self, hovered: bool) -> f32 {
        match (self, hovered) {
            (Self::Ghost, false) => 0.0,
            (Self::Active, false) => 0.1,
            (Self::Active, true) => 0.15,
            (Self::ActiveAlt, _) => 0.1,
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
        }
    }
    pub fn border_color(&self) -> Srgba {
        match self {
            Self::Default | Self::Ghost => tailwind::ZINC_700,
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
            (Self::Ghost, false) => 0.0,
            (Self::ActiveAlt, _) => 0.4,
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

fn button_base(variant: ButtonVariant, size: ButtonSize, align_left: bool) -> impl Bundle {
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
            height: size.height(),
            padding: UiRect::horizontal(size.padding()),
            border: UiRect::all(variant.border()),
            border_radius: BorderRadius::all(CORNER_RADIUS_LG),
            column_gap: px(6.0),
            justify_content: if align_left {
                JustifyContent::Start
            } else {
                JustifyContent::Center
            },
            align_items: AlignItems::Center,
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
    } = props;

    (
        button_base(variant, size, align_left),
        ButtonConfig {
            content,
            left_icon,
            right_icon,
            initialized: false,
        },
    )
}

fn setup_button(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut buttons: Query<
        (Entity, &mut ButtonConfig, &ButtonVariant, &ButtonSize),
        Added<ButtonConfig>,
    >,
) {
    let font: Handle<Font> = asset_server.load(FONT_PATH);

    for (entity, mut config, variant, size) in &mut buttons {
        if config.initialized {
            continue;
        }
        config.initialized = true;

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
    interactions: Query<(Entity, &Interaction), (Changed<Interaction>, With<EditorButton>)>,
    mut commands: Commands,
) {
    for (entity, interaction) in &interactions {
        if *interaction == Interaction::Pressed {
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
        button_base(variant, size, false),
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
