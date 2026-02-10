use bevy::math::Vec3;
use sprinkles::asset::{EmissionShape, EmitterCollisionMode, EmitterData};

#[test]
fn test_inspect_emission_shape_point_to_sphere() {
    let mut emitter = EmitterData::default();
    assert!(matches!(emitter.emission.shape, EmissionShape::Point));

    emitter.emission.shape = EmissionShape::Sphere { radius: 3.0 };
    assert!(matches!(
        emitter.emission.shape,
        EmissionShape::Sphere { radius } if radius == 3.0
    ));
}

#[test]
fn test_inspect_emission_shape_ring_fields() {
    let mut emitter = EmitterData::default();
    emitter.emission.shape = EmissionShape::Ring {
        axis: Vec3::Y,
        height: 0.5,
        radius: 3.0,
        inner_radius: 1.0,
    };

    if let EmissionShape::Ring {
        radius,
        inner_radius,
        height,
        axis,
    } = &emitter.emission.shape
    {
        assert_eq!(*radius, 3.0);
        assert_eq!(*inner_radius, 1.0);
        assert_eq!(*height, 0.5);
        assert_eq!(*axis, Vec3::Y);
    } else {
        panic!("expected Ring variant");
    }
}

#[test]
fn test_inspect_collision_mode_rigid() {
    let mut emitter = EmitterData::default();
    emitter.collision.mode = Some(EmitterCollisionMode::Rigid {
        friction: 0.5,
        bounce: 0.8,
    });

    if let Some(EmitterCollisionMode::Rigid { friction, bounce }) = &emitter.collision.mode {
        assert_eq!(*friction, 0.5);
        assert_eq!(*bounce, 0.8);
    } else {
        panic!("expected Rigid collision mode");
    }
}

#[test]
fn test_inspect_collision_mode_hide_on_contact() {
    let mut emitter = EmitterData::default();
    emitter.collision.mode = Some(EmitterCollisionMode::HideOnContact);

    assert!(matches!(
        emitter.collision.mode,
        Some(EmitterCollisionMode::HideOnContact)
    ));
}

#[test]
fn test_inspect_collision_mode_none() {
    let mut emitter = EmitterData::default();
    emitter.collision.mode = Some(EmitterCollisionMode::Rigid {
        friction: 0.5,
        bounce: 0.8,
    });

    emitter.collision.mode = None;
    assert!(emitter.collision.mode.is_none());
}
