use bevy::math::Vec3;
use sprinkles::asset::{
    ColliderData, EmitterData, ParticleFlags, ParticlesColliderShape3D,
};

#[test]
fn test_inspect_color_over_lifetime_gradient() {
    let mut emitter = EmitterData::default();

    let stops = &emitter.colors.color_over_lifetime.stops;
    assert!(
        !stops.is_empty(),
        "default color_over_lifetime should have stops"
    );

    emitter.colors.color_over_lifetime.stops = vec![
        sprinkles::asset::GradientStop {
            color: [1.0, 0.0, 0.0, 1.0],
            position: 0.0,
        },
        sprinkles::asset::GradientStop {
            color: [0.0, 0.0, 0.0, 0.0],
            position: 1.0,
        },
    ];

    assert_eq!(emitter.colors.color_over_lifetime.stops.len(), 2);
    assert_eq!(
        emitter.colors.color_over_lifetime.stops[0].color,
        [1.0, 0.0, 0.0, 1.0]
    );
}

#[test]
fn test_inspect_initial_color_variant_switch() {
    let mut emitter = EmitterData::default();
    assert!(matches!(
        emitter.colors.initial_color,
        sprinkles::asset::SolidOrGradientColor::Solid { .. }
    ));

    emitter.colors.initial_color =
        sprinkles::asset::SolidOrGradientColor::Gradient {
            gradient: sprinkles::asset::Gradient {
                stops: vec![
                    sprinkles::asset::GradientStop {
                        color: [1.0, 0.0, 0.0, 1.0],
                        position: 0.0,
                    },
                    sprinkles::asset::GradientStop {
                        color: [0.0, 0.0, 1.0, 1.0],
                        position: 1.0,
                    },
                ],
                interpolation: sprinkles::asset::GradientInterpolation::Linear,
            },
        };

    assert!(matches!(
        emitter.colors.initial_color,
        sprinkles::asset::SolidOrGradientColor::Gradient { .. }
    ));
}

#[test]
fn test_inspect_scale_over_lifetime_curve() {
    let mut emitter = EmitterData::default();
    assert!(emitter.scale.scale_over_lifetime.is_none());

    emitter.scale.scale_over_lifetime = Some(sprinkles::asset::CurveTexture {
        name: None,
        points: vec![
            sprinkles::asset::CurvePoint::new(0.0, 1.0),
            sprinkles::asset::CurvePoint::new(1.0, 0.0),
        ],
        range: sprinkles::asset::Range::new(0.0, 1.0),
    });

    assert!(emitter.scale.scale_over_lifetime.is_some());
    let curve = emitter.scale.scale_over_lifetime.as_ref().unwrap();
    assert_eq!(curve.points.len(), 2);
}

#[test]
fn test_inspect_angle_over_lifetime_curve() {
    let mut emitter = EmitterData::default();
    assert!(emitter.angle.angle_over_lifetime.is_none());

    emitter.angle.angle_over_lifetime = Some(sprinkles::asset::CurveTexture {
        name: None,
        points: vec![
            sprinkles::asset::CurvePoint::new(0.0, 0.0),
            sprinkles::asset::CurvePoint::new(1.0, 1.0),
        ],
        range: sprinkles::asset::Range::new(0.0, 360.0),
    });

    assert!(emitter.angle.angle_over_lifetime.is_some());
    let curve = emitter.angle.angle_over_lifetime.as_ref().unwrap();
    assert_eq!(curve.range.max, 360.0);
}

#[test]
fn test_inspect_sub_emitter_constant() {
    let mut emitter = EmitterData::default();
    assert!(emitter.sub_emitter.is_none());

    emitter.sub_emitter = Some(sprinkles::asset::SubEmitterConfig {
        mode: sprinkles::asset::SubEmitterMode::Constant,
        target_emitter: 1,
        frequency: 4.0,
        amount: 2,
        keep_velocity: true,
    });

    let sub = emitter.sub_emitter.as_ref().unwrap();
    assert!(matches!(sub.mode, sprinkles::asset::SubEmitterMode::Constant));
    assert_eq!(sub.target_emitter, 1);
}

#[test]
fn test_inspect_sub_emitter_frequency() {
    let mut emitter = EmitterData::default();
    emitter.sub_emitter = Some(sprinkles::asset::SubEmitterConfig {
        mode: sprinkles::asset::SubEmitterMode::Constant,
        target_emitter: 1,
        frequency: 4.0,
        amount: 2,
        keep_velocity: false,
    });

    emitter.sub_emitter.as_mut().unwrap().frequency = 8.0;
    assert_eq!(emitter.sub_emitter.as_ref().unwrap().frequency, 8.0);
}

#[test]
fn test_inspect_sub_emitter_keep_velocity() {
    let mut emitter = EmitterData::default();
    emitter.sub_emitter = Some(sprinkles::asset::SubEmitterConfig {
        mode: sprinkles::asset::SubEmitterMode::Constant,
        target_emitter: 1,
        frequency: 4.0,
        amount: 2,
        keep_velocity: false,
    });

    assert!(!emitter.sub_emitter.as_ref().unwrap().keep_velocity);

    emitter.sub_emitter.as_mut().unwrap().keep_velocity = true;
    assert!(emitter.sub_emitter.as_ref().unwrap().keep_velocity);
}

#[test]
fn test_inspect_particle_flags_rotate_y() {
    let mut emitter = EmitterData::default();
    assert!(emitter.particle_flags.is_empty());

    emitter.particle_flags = ParticleFlags::ROTATE_Y;
    assert!(emitter.particle_flags.contains(ParticleFlags::ROTATE_Y));
    assert!(!emitter.particle_flags.contains(ParticleFlags::DISABLE_Z));
}

#[test]
fn test_inspect_particle_flags_combined() {
    let mut emitter = EmitterData::default();

    emitter.particle_flags = ParticleFlags::ROTATE_Y | ParticleFlags::DISABLE_Z;
    assert!(emitter.particle_flags.contains(ParticleFlags::ROTATE_Y));
    assert!(emitter.particle_flags.contains(ParticleFlags::DISABLE_Z));
}

#[test]
fn test_inspect_collider_box_shape_size() {
    let mut collider = ColliderData::default();
    collider.shape = ParticlesColliderShape3D::Box {
        size: Vec3::new(10.0, 1.0, 10.0),
    };

    if let ParticlesColliderShape3D::Box { size } = &collider.shape {
        assert_eq!(*size, Vec3::new(10.0, 1.0, 10.0));
    } else {
        panic!("expected Box shape");
    }
}

#[test]
fn test_inspect_collider_position() {
    let mut collider = ColliderData::default();
    assert_eq!(collider.position, Vec3::ZERO);

    collider.position = Vec3::new(0.0, -5.0, 0.0);
    assert_eq!(collider.position, Vec3::new(0.0, -5.0, 0.0));
}
