use std::f32::consts::FRAC_PI_2;
use std::ops::Range;

use bevy::input::mouse::AccumulatedMouseMotion;
use bevy::prelude::*;

#[derive(Component)]
pub struct EditorCamera;

#[derive(Resource)]
pub struct OrbitCameraSettings {
    pub orbit_distance: f32,
    pub pitch_speed: f32,
    pub pitch_range: Range<f32>,
    pub yaw_speed: f32,
}

impl Default for OrbitCameraSettings {
    fn default() -> Self {
        let pitch_limit = FRAC_PI_2 - 0.01;
        Self {
            orbit_distance: 10.0,
            pitch_speed: 0.003,
            pitch_range: -pitch_limit..pitch_limit,
            yaw_speed: 0.004,
        }
    }
}

pub fn setup_camera(mut commands: Commands) {
    let initial_distance = 10.0;
    let initial_pitch = -std::f32::consts::FRAC_PI_4;
    let initial_yaw = std::f32::consts::FRAC_PI_4;

    let rotation = Quat::from_euler(EulerRot::YXZ, initial_yaw, initial_pitch, 0.0);
    let forward = rotation * Vec3::NEG_Z;
    let translation = Vec3::ZERO - forward * initial_distance;

    commands.spawn((
        EditorCamera,
        Camera3d::default(),
        Transform::from_translation(translation).with_rotation(rotation),
    ));
}

pub fn orbit_camera(
    mut camera: Single<&mut Transform, With<EditorCamera>>,
    settings: Res<OrbitCameraSettings>,
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    mouse_motion: Res<AccumulatedMouseMotion>,
) {
    if !mouse_buttons.pressed(MouseButton::Middle) {
        return;
    }

    let delta = mouse_motion.delta;
    let delta_pitch = delta.y * settings.pitch_speed;
    let delta_yaw = delta.x * settings.yaw_speed;

    let (yaw, pitch, roll) = camera.rotation.to_euler(EulerRot::YXZ);
    let pitch = (pitch + delta_pitch).clamp(settings.pitch_range.start, settings.pitch_range.end);
    let yaw = yaw + delta_yaw;

    camera.rotation = Quat::from_euler(EulerRot::YXZ, yaw, pitch, roll);

    let target = Vec3::ZERO;
    camera.translation = target - camera.forward() * settings.orbit_distance;
}
