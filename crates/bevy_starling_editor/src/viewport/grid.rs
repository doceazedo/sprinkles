use std::f32::consts::PI;

use bevy::color::LinearRgba;
use bevy::prelude::*;

pub fn draw_grid(mut gizmos: Gizmos) {
    gizmos.grid(
        Quat::from_rotation_x(PI / 2.0),
        UVec2::splat(20),
        Vec2::new(1.0, 1.0),
        LinearRgba::gray(0.35),
    );
}
