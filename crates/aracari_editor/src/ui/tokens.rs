use bevy::color::palettes::tailwind;
use bevy::prelude::*;

// corner radius
pub const CORNER_RADIUS: Val = Val::Px(2.0);
pub const CORNER_RADIUS_LG: Val = Val::Px(4.0);

// colors
pub const PRIMARY_COLOR: Srgba = tailwind::BLUE_500;
pub const BACKGROUND_COLOR: Srgba = tailwind::ZINC_800;
pub const BORDER_COLOR: Srgba = tailwind::ZINC_700;
pub const TEXT_BODY_COLOR: Srgba = tailwind::ZINC_200;
pub const TEXT_DISPLAY_COLOR: Srgba = tailwind::ZINC_50;

// text sizes
pub const TEXT_SIZE_SM: f32 = 10.0;
pub const TEXT_SIZE: f32 = 12.0;
pub const TEXT_SIZE_LG: f32 = 14.0;

// font
pub const FONT_PATH: &str = "InterVariable.ttf";
