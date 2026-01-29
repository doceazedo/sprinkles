use bevy::color::palettes::tailwind;
use bevy::picking::hover::Hovered;
use bevy::prelude::*;

use crate::ui::tokens::{
    CORNER_RADIUS_LG, FONT_PATH, PRIMARY_COLOR, TEXT_BODY_COLOR, TEXT_DISPLAY_COLOR, TEXT_SIZE,
};

pub fn plugin(app: &mut App) {
    app.add_systems(Update, handle_hover);
}

#[derive(Component)]
pub struct EditorButton;

#[derive(Component, Default, Clone, Copy)]
pub enum ButtonVariant {
    #[default]
    Default,
    Primary,
}

#[derive(Component, Default, Clone, Copy)]
pub enum ButtonSize {
    #[default]
    Default,
    Sm,
    Icon,
}

impl ButtonVariant {
    fn bg_color(&self) -> Srgba {
        match self {
            Self::Default => tailwind::ZINC_700,
            Self::Primary => PRIMARY_COLOR,
        }
    }
    fn bg_opacity(&self, hovered: bool) -> f32 {
        match (self, hovered) {
            (Self::Default, false) => 0.5,
            (Self::Default, true) => 0.8,
            (Self::Primary, _) => 1.0,
        }
    }
    fn text_color(&self) -> Srgba {
        match self {
            Self::Default => TEXT_BODY_COLOR,
            Self::Primary => TEXT_DISPLAY_COLOR,
        }
    }
    fn border(&self) -> Val {
        match self {
            Self::Default => Val::Px(1.0),
            Self::Primary => Val::Px(0.0),
        }
    }
}

impl ButtonSize {
    fn width(&self) -> Val {
        match self {
            Self::Icon => Val::Px(28.0),
            _ => Val::Auto,
        }
    }
    fn height(&self) -> Val {
        Val::Px(28.0)
    }
    fn padding(&self) -> Val {
        match self {
            Self::Default => Val::Px(12.0),
            Self::Sm => Val::Px(6.0),
            Self::Icon => Val::Px(0.0),
        }
    }
}

#[derive(Default)]
pub struct ButtonProps {
    pub content: String,
    pub variant: ButtonVariant,
    pub size: ButtonSize,
}

impl ButtonProps {
    pub fn new(content: impl Into<String>) -> Self {
        Self {
            content: content.into(),
            ..default()
        }
    }
    pub fn variant(mut self, variant: ButtonVariant) -> Self {
        self.variant = variant;
        self
    }
    pub fn size(mut self, size: ButtonSize) -> Self {
        self.size = size;
        self
    }
}

pub fn button(props: ButtonProps, asset_server: &AssetServer) -> impl Bundle {
    let ButtonProps {
        content,
        variant,
        size,
    } = props;
    let font: Handle<Font> = asset_server.load(FONT_PATH);

    (
        Button,
        EditorButton,
        variant,
        size,
        Hovered::default(),
        Node {
            width: size.width(),
            height: size.height(),
            padding: UiRect::horizontal(size.padding()),
            border: UiRect::all(variant.border()),
            border_radius: BorderRadius::all(CORNER_RADIUS_LG),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            ..default()
        },
        BackgroundColor(
            variant
                .bg_color()
                .with_alpha(variant.bg_opacity(false))
                .into(),
        ),
        BorderColor::all(tailwind::ZINC_700),
        children![(
            Text::new(content),
            TextFont {
                font: font.into(),
                font_size: TEXT_SIZE,
                weight: FontWeight::MEDIUM,
                ..default()
            },
            TextColor(variant.text_color().into()),
        )],
    )
}

fn handle_hover(
    mut buttons: Query<
        (&ButtonVariant, &Hovered, &mut BackgroundColor),
        (Changed<Hovered>, With<EditorButton>),
    >,
) {
    for (variant, hovered, mut bg) in &mut buttons {
        bg.0 = variant
            .bg_color()
            .with_alpha(variant.bg_opacity(hovered.get()))
            .into();
    }
}
