use super::*;

use bevy::math::Vec3;
use sprinkles::asset::EmitterData;

#[test]
fn test_inspect_vec3_offset() {
    let mut emitter = EmitterData::default();
    assert_eq!(read_vec3(&emitter, "emission.offset"), Vec3::ZERO);

    write_vec3(&mut emitter, "emission.offset", Vec3::new(1.0, 2.0, 3.0));
    assert_eq!(emitter.emission.offset, Vec3::new(1.0, 2.0, 3.0));
}

#[test]
fn test_inspect_vec3_gravity() {
    let mut emitter = EmitterData::default();
    let gravity = read_vec3(&emitter, "accelerations.gravity");
    assert_eq!(gravity, Vec3::new(0.0, -9.8, 0.0));

    write_vec3(
        &mut emitter,
        "accelerations.gravity",
        Vec3::new(0.0, -15.0, 0.0),
    );
    assert_eq!(emitter.accelerations.gravity, Vec3::new(0.0, -15.0, 0.0));
}

#[test]
fn test_inspect_range_scale() {
    let mut emitter = EmitterData::default();
    assert_eq!(read_f32(&emitter, "scale.range.min"), 1.0);
    assert_eq!(read_f32(&emitter, "scale.range.max"), 1.0);

    write_f32(&mut emitter, "scale.range.min", 0.5);
    write_f32(&mut emitter, "scale.range.max", 2.0);
    assert_eq!(emitter.scale.range.min, 0.5);
    assert_eq!(emitter.scale.range.max, 2.0);
}

#[test]
fn test_inspect_range_initial_velocity() {
    let mut emitter = EmitterData::default();

    write_f32(&mut emitter, "velocities.initial_velocity.min", 5.0);
    write_f32(&mut emitter, "velocities.initial_velocity.max", 10.0);
    assert_eq!(emitter.velocities.initial_velocity.min, 5.0);
    assert_eq!(emitter.velocities.initial_velocity.max, 10.0);
}
