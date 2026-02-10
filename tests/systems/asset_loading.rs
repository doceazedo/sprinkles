use super::helpers::*;

use bevy::prelude::*;
use sprinkles::asset::*;

#[test]
fn load_minimal_particle_system() {
    let mut app = create_minimal_app();
    let asset = load_asset(&mut app, "minimal_particle_system.ron");

    assert_eq!(asset.name, "Minimal System");
    assert_eq!(asset.dimension, ParticleSystemDimension::D3);
    assert_eq!(asset.emitters.len(), 1);
    assert!(asset.colliders.is_empty());

    let emitter = &asset.emitters[0];
    assert_eq!(emitter.name, "Emitter 1");
    assert_eq!(emitter.time.lifetime, 1.0);
    assert_eq!(emitter.emission.particles_amount, 8);
}

#[test]
fn load_system_with_all_emission_shapes() {
    let mut app = create_minimal_app();
    let asset = load_asset(&mut app, "all_emission_shapes.ron");

    assert_eq!(asset.emitters.len(), 5);

    assert_eq!(asset.emitters[0].emission.shape, EmissionShape::Point);

    match asset.emitters[1].emission.shape {
        EmissionShape::Sphere { radius } => assert_eq!(radius, 3.0),
        _ => panic!("expected Sphere"),
    }

    match asset.emitters[2].emission.shape {
        EmissionShape::SphereSurface { radius } => assert_eq!(radius, 5.0),
        _ => panic!("expected SphereSurface"),
    }

    match asset.emitters[3].emission.shape {
        EmissionShape::Box { extents } => {
            assert_eq!(extents, Vec3::new(2.0, 3.0, 4.0));
        }
        _ => panic!("expected Box"),
    }

    match asset.emitters[4].emission.shape {
        EmissionShape::Ring {
            axis,
            height,
            radius,
            inner_radius,
        } => {
            assert_eq!(axis, Vec3::new(0.0, 1.0, 0.0));
            assert_eq!(height, 0.5);
            assert_eq!(radius, 3.0);
            assert_eq!(inner_radius, 1.0);
        }
        _ => panic!("expected Ring"),
    }
}

#[test]
fn load_system_with_gradients() {
    let mut app = create_minimal_app();
    let asset = load_asset(&mut app, "gradients_test.ron");

    assert_eq!(asset.emitters.len(), 2);

    let linear_emitter = &asset.emitters[0];
    assert!(linear_emitter.colors.initial_color.is_gradient());
    if let SolidOrGradientColor::Gradient { gradient } = &linear_emitter.colors.initial_color {
        assert_eq!(gradient.stops.len(), 3);
        assert_eq!(gradient.interpolation, GradientInterpolation::Linear);
        assert_eq!(gradient.stops[0].color, [1.0, 0.0, 0.0, 1.0]);
        assert_eq!(gradient.stops[0].position, 0.0);
        assert_eq!(gradient.stops[1].color, [0.0, 1.0, 0.0, 1.0]);
        assert_eq!(gradient.stops[1].position, 0.5);
        assert_eq!(gradient.stops[2].color, [0.0, 0.0, 1.0, 1.0]);
        assert_eq!(gradient.stops[2].position, 1.0);
    }

    assert_eq!(
        linear_emitter.colors.color_over_lifetime.interpolation,
        GradientInterpolation::Steps
    );
    assert_eq!(linear_emitter.colors.color_over_lifetime.stops.len(), 2);

    let smoothstep_emitter = &asset.emitters[1];
    if let SolidOrGradientColor::Gradient { gradient } = &smoothstep_emitter.colors.initial_color {
        assert_eq!(gradient.interpolation, GradientInterpolation::Smoothstep);
        assert_eq!(gradient.stops.len(), 2);
    } else {
        panic!("expected Gradient");
    }
}

#[test]
fn load_system_with_curves() {
    let mut app = create_minimal_app();
    let asset = load_asset(&mut app, "curves_test.ron");

    let emitter = &asset.emitters[0];

    let scale_curve = emitter
        .scale
        .scale_over_lifetime
        .as_ref()
        .expect("should have scale curve");
    assert_eq!(scale_curve.points.len(), 3);
    assert_eq!(scale_curve.points[0].mode, CurveMode::SingleCurve);
    assert_eq!(scale_curve.points[0].tension, 0.5);
    assert_eq!(scale_curve.points[0].easing, CurveEasing::Power);
    assert_eq!(scale_curve.points[1].mode, CurveMode::Hold);
    assert_eq!(scale_curve.points[1].value, 0.5);
    assert_eq!(scale_curve.points[2].mode, CurveMode::DoubleCurve);
    assert_eq!(scale_curve.points[2].easing, CurveEasing::Sine);

    let alpha_curve = emitter
        .colors
        .alpha_over_lifetime
        .as_ref()
        .expect("should have alpha curve");
    assert_eq!(alpha_curve.points.len(), 2);
    assert_eq!(alpha_curve.points[0].easing, CurveEasing::Expo);
    assert_eq!(alpha_curve.points[1].easing, CurveEasing::Circ);
}

#[test]
fn load_system_with_colliders() {
    let mut app = create_minimal_app();
    let asset = load_asset(&mut app, "collision_test.ron");

    assert_eq!(asset.colliders.len(), 2);

    let floor = &asset.colliders[0];
    assert_eq!(floor.name, "Floor");
    assert!(floor.enabled);
    assert_eq!(floor.position, Vec3::new(0.0, -2.0, 0.0));
    match &floor.shape {
        ParticlesColliderShape3D::Box { size } => {
            assert_eq!(*size, Vec3::new(10.0, 1.0, 10.0));
        }
        _ => panic!("expected Box collider"),
    }

    let wall = &asset.colliders[1];
    assert_eq!(wall.name, "Wall");
    match &wall.shape {
        ParticlesColliderShape3D::Sphere { radius } => {
            assert_eq!(*radius, 3.0);
        }
        _ => panic!("expected Sphere collider"),
    }
    assert_eq!(wall.position, Vec3::new(5.0, 0.0, 0.0));
}

#[test]
fn load_system_with_sub_emitters() {
    let mut app = create_minimal_app();
    let asset = load_asset(&mut app, "sub_emitter_test.ron");

    assert_eq!(asset.emitters.len(), 2);

    let main_emitter = &asset.emitters[0];
    let sub_config = main_emitter
        .sub_emitter
        .as_ref()
        .expect("should have sub_emitter config");
    assert_eq!(sub_config.mode, SubEmitterMode::Constant);
    assert_eq!(sub_config.target_emitter, 1);
    assert_eq!(sub_config.frequency, 4.0);
    assert_eq!(sub_config.amount, 2);
    assert!(sub_config.keep_velocity);

    assert!(asset.emitters[1].sub_emitter.is_none());
}

#[test]
fn load_system_preserves_all_emitter_fields() {
    let mut app = create_minimal_app();
    let asset = load_asset(&mut app, "maximal_emitter.ron");

    let e = &asset.emitters[0];
    assert_eq!(e.name, "Full Config");
    assert!(e.enabled);
    assert_eq!(e.position, Vec3::new(1.0, 2.0, 3.0));

    // time
    assert_eq!(e.time.lifetime, 3.0);
    assert_eq!(e.time.lifetime_randomness, 0.2);
    assert_eq!(e.time.delay, 0.5);
    assert!(!e.time.one_shot);
    assert_eq!(e.time.explosiveness, 0.3);
    assert_eq!(e.time.spawn_time_randomness, 0.1);
    assert_eq!(e.time.fixed_fps, 60);
    assert_eq!(e.time.fixed_seed, Some(123));

    // draw pass
    assert_eq!(e.draw_pass.draw_order, DrawOrder::Lifetime);
    assert!(!e.draw_pass.shadow_caster);
    assert_eq!(e.draw_pass.transform_align, Some(TransformAlign::YToVelocity));
    match &e.draw_pass.mesh {
        ParticleMesh::Quad {
            orientation,
            size,
            subdivide,
        } => {
            assert_eq!(*orientation, QuadOrientation::FaceY);
            assert_eq!(*size, Vec2::new(0.5, 0.5));
            assert_eq!(*subdivide, Vec2::new(2.0, 2.0));
        }
        _ => panic!("expected Quad mesh"),
    }

    // emission
    assert_eq!(e.emission.offset, Vec3::new(0.5, 1.0, -0.5));
    assert_eq!(e.emission.scale, Vec3::new(2.0, 2.0, 2.0));
    assert_eq!(e.emission.particles_amount, 64);

    // scale
    assert_eq!(e.scale.range.min, 0.5);
    assert_eq!(e.scale.range.max, 2.0);
    assert!(e.scale.scale_over_lifetime.is_some());

    // angle
    assert_eq!(e.angle.range.min, -45.0);
    assert_eq!(e.angle.range.max, 45.0);
    assert!(e.angle.angle_over_lifetime.is_some());

    // colors
    assert!(e.colors.initial_color.is_gradient());
    assert!(e.colors.alpha_over_lifetime.is_some());
    assert!(e.colors.emission_over_lifetime.is_some());

    // velocities
    assert_eq!(e.velocities.initial_direction, Vec3::new(0.0, 1.0, 0.0));
    assert_eq!(e.velocities.spread, 30.0);
    assert_eq!(e.velocities.flatness, 0.2);
    assert_eq!(e.velocities.initial_velocity.min, 5.0);
    assert_eq!(e.velocities.initial_velocity.max, 10.0);
    assert_eq!(e.velocities.radial_velocity.velocity.min, 1.0);
    assert_eq!(e.velocities.radial_velocity.velocity.max, 3.0);
    assert!(e.velocities.radial_velocity.velocity_over_lifetime.is_some());
    assert_eq!(e.velocities.angular_velocity.velocity.min, -90.0);
    assert_eq!(e.velocities.angular_velocity.velocity.max, 90.0);
    assert_eq!(e.velocities.pivot, Vec3::new(0.0, 0.5, 0.0));
    assert_eq!(e.velocities.inherit_ratio, 0.5);

    // accelerations
    assert_eq!(e.accelerations.gravity, Vec3::new(0.0, -15.0, 0.0));

    // turbulence
    assert!(e.turbulence.enabled);
    assert_eq!(e.turbulence.noise_strength, 2.0);
    assert_eq!(e.turbulence.noise_scale, 3.5);
    assert_eq!(e.turbulence.noise_speed, Vec3::new(1.0, 0.5, 0.0));
    assert_eq!(e.turbulence.noise_speed_random, 0.3);
    assert_eq!(e.turbulence.influence.min, 0.1);
    assert_eq!(e.turbulence.influence.max, 0.5);
    assert!(e.turbulence.influence_over_lifetime.is_some());

    // collision
    match &e.collision.mode {
        Some(EmitterCollisionMode::Rigid { friction, bounce }) => {
            assert_eq!(*friction, 0.3);
            assert_eq!(*bounce, 0.6);
        }
        _ => panic!("expected Rigid collision mode"),
    }
    assert!(e.collision.use_scale);
    assert_eq!(e.collision.base_size, 0.05);

    // sub_emitter is None in this fixture (only one emitter)
    assert!(e.sub_emitter.is_none());

    // particle_flags: ROTATE_Y (2) | DISABLE_Z (4) = 6
    assert!(e.particle_flags.contains(ParticleFlags::ROTATE_Y));
    assert!(e.particle_flags.contains(ParticleFlags::DISABLE_Z));
}

#[test]
fn load_system_preserves_material_fields() {
    let mut app = create_minimal_app();
    let asset = load_asset(&mut app, "maximal_emitter.ron");

    let material = match &asset.emitters[0].draw_pass.material {
        DrawPassMaterial::Standard(mat) => mat,
        _ => panic!("expected Standard material"),
    };

    assert_eq!(material.base_color, [1.0, 0.5, 0.2, 1.0]);
    assert_eq!(material.emissive, [0.5, 0.1, 0.0, 1.0]);
    assert_eq!(material.alpha_mode, SerializableAlphaMode::Blend);
    assert_eq!(material.perceptual_roughness, 0.8);
    assert_eq!(material.metallic, 0.3);
    assert_eq!(material.reflectance, 0.7);
    assert!(material.double_sided);
    assert!(material.unlit);
    assert!(!material.fog_enabled);
    assert!(material.base_color_texture.is_none());
    assert!(material.emissive_texture.is_none());
}

#[test]
fn load_system_preserves_particle_flags() {
    let mut app = create_minimal_app();
    let asset = load_asset(&mut app, "maximal_emitter.ron");

    let flags = asset.emitters[0].particle_flags;
    assert!(flags.contains(ParticleFlags::ROTATE_Y));
    assert!(flags.contains(ParticleFlags::DISABLE_Z));
    assert_eq!(flags, ParticleFlags::ROTATE_Y | ParticleFlags::DISABLE_Z);
}

#[test]
fn default_values_when_fields_omitted() {
    let mut app = create_minimal_app();
    let asset = load_asset(&mut app, "minimal_particle_system.ron");

    let e = &asset.emitters[0];

    // time defaults
    assert_eq!(e.time.lifetime, 1.0);
    assert_eq!(e.time.lifetime_randomness, 0.0);
    assert_eq!(e.time.delay, 0.0);
    assert!(!e.time.one_shot);
    assert_eq!(e.time.explosiveness, 0.0);
    assert_eq!(e.time.spawn_time_randomness, 0.0);
    assert_eq!(e.time.fixed_fps, 0);
    assert!(e.time.fixed_seed.is_none());

    // emission defaults
    assert_eq!(e.emission.offset, Vec3::ZERO);
    assert_eq!(e.emission.scale, Vec3::ONE);
    assert_eq!(e.emission.shape, EmissionShape::Point);

    // velocity defaults
    assert_eq!(e.velocities.initial_direction, Vec3::X);
    assert_eq!(e.velocities.spread, 45.0);
    assert_eq!(e.velocities.flatness, 0.0);
    assert_eq!(e.velocities.initial_velocity.min, 0.0);
    assert_eq!(e.velocities.initial_velocity.max, 0.0);
    assert_eq!(e.velocities.pivot, Vec3::ZERO);
    assert_eq!(e.velocities.inherit_ratio, 0.0);

    // acceleration defaults
    assert_eq!(e.accelerations.gravity, Vec3::new(0.0, -9.8, 0.0));

    // scale defaults
    assert_eq!(e.scale.range.min, 1.0);
    assert_eq!(e.scale.range.max, 1.0);
    assert!(e.scale.scale_over_lifetime.is_none());

    // angle defaults
    assert_eq!(e.angle.range.min, 0.0);
    assert_eq!(e.angle.range.max, 0.0);
    assert!(e.angle.angle_over_lifetime.is_none());

    // colors defaults
    assert!(e.colors.initial_color.is_solid());
    assert!(e.colors.alpha_over_lifetime.is_none());
    assert!(e.colors.emission_over_lifetime.is_none());

    // turbulence defaults
    assert!(!e.turbulence.enabled);

    // collision defaults
    assert!(e.collision.mode.is_none());
    assert!(!e.collision.use_scale);

    // sub_emitter defaults
    assert!(e.sub_emitter.is_none());

    // flags default
    assert!(e.particle_flags.is_empty());

    // draw pass defaults
    assert_eq!(e.draw_pass.draw_order, DrawOrder::Index);
    assert!(e.draw_pass.shadow_caster);
    assert!(e.draw_pass.transform_align.is_none());

    // emitter defaults
    assert!(e.enabled);
    assert_eq!(e.position, Vec3::ZERO);
}

#[test]
fn roundtrip_serialize_deserialize() {
    let mut app = create_minimal_app();
    let original = load_asset(&mut app, "maximal_emitter.ron");

    let serialized =
        ron::ser::to_string_pretty(&original, ron::ser::PrettyConfig::default()).unwrap();
    let deserialized: ParticleSystemAsset = ron::from_str(&serialized).unwrap();

    assert_eq!(deserialized.name, original.name);
    assert_eq!(deserialized.dimension, original.dimension);
    assert_eq!(deserialized.emitters.len(), original.emitters.len());
    assert_eq!(deserialized.colliders.len(), original.colliders.len());

    let orig_e = &original.emitters[0];
    let deser_e = &deserialized.emitters[0];

    assert_eq!(deser_e.name, orig_e.name);
    assert_eq!(deser_e.enabled, orig_e.enabled);
    assert_eq!(deser_e.position, orig_e.position);
    assert_eq!(deser_e.time.lifetime, orig_e.time.lifetime);
    assert_eq!(deser_e.time.delay, orig_e.time.delay);
    assert_eq!(deser_e.time.fixed_seed, orig_e.time.fixed_seed);
    assert_eq!(deser_e.emission.particles_amount, orig_e.emission.particles_amount);
    assert_eq!(deser_e.velocities.spread, orig_e.velocities.spread);
    assert_eq!(deser_e.accelerations.gravity, orig_e.accelerations.gravity);
    assert_eq!(deser_e.particle_flags, orig_e.particle_flags);
    assert_eq!(deser_e.draw_pass.draw_order, orig_e.draw_pass.draw_order);
    assert_eq!(deser_e.draw_pass.mesh, orig_e.draw_pass.mesh);
    assert_eq!(
        deser_e.draw_pass.transform_align,
        orig_e.draw_pass.transform_align
    );
}
