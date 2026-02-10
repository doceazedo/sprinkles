use bevy::prelude::*;
use sprinkles::asset::*;

fn roundtrip_ron<T: serde::Serialize + serde::de::DeserializeOwned>(value: &T) -> T {
    let serialized = ron::ser::to_string_pretty(value, ron::ser::PrettyConfig::default()).unwrap();
    ron::from_str(&serialized).unwrap()
}

#[test]
fn particle_mesh_quad_roundtrip() {
    let mesh = ParticleMesh::Quad {
        orientation: QuadOrientation::FaceZ,
        size: Vec2::new(1.0, 2.0),
        subdivide: Vec2::new(3.0, 4.0),
    };
    assert_eq!(roundtrip_ron(&mesh), mesh);
}

#[test]
fn particle_mesh_sphere_roundtrip() {
    let mesh = ParticleMesh::Sphere { radius: 2.5 };
    assert_eq!(roundtrip_ron(&mesh), mesh);
}

#[test]
fn particle_mesh_cuboid_roundtrip() {
    let mesh = ParticleMesh::Cuboid {
        half_size: Vec3::new(1.0, 2.0, 3.0),
    };
    assert_eq!(roundtrip_ron(&mesh), mesh);
}

#[test]
fn particle_mesh_cylinder_roundtrip() {
    let mesh = ParticleMesh::Cylinder {
        top_radius: 1.0,
        bottom_radius: 2.0,
        height: 3.0,
        radial_segments: 16,
        rings: 4,
        cap_top: true,
        cap_bottom: false,
    };
    assert_eq!(roundtrip_ron(&mesh), mesh);
}

#[test]
fn particle_mesh_prism_roundtrip() {
    let mesh = ParticleMesh::Prism {
        left_to_right: 0.5,
        size: Vec3::new(1.0, 2.0, 3.0),
        subdivide: Vec3::ZERO,
    };
    assert_eq!(roundtrip_ron(&mesh), mesh);
}

#[test]
fn emission_shape_point_roundtrip() {
    let shape = EmissionShape::Point;
    assert_eq!(roundtrip_ron(&shape), shape);
}

#[test]
fn emission_shape_sphere_roundtrip() {
    let shape = EmissionShape::Sphere { radius: 3.0 };
    assert_eq!(roundtrip_ron(&shape), shape);
}

#[test]
fn emission_shape_sphere_surface_roundtrip() {
    let shape = EmissionShape::SphereSurface { radius: 5.0 };
    assert_eq!(roundtrip_ron(&shape), shape);
}

#[test]
fn emission_shape_box_roundtrip() {
    let shape = EmissionShape::Box {
        extents: Vec3::new(2.0, 3.0, 4.0),
    };
    assert_eq!(roundtrip_ron(&shape), shape);
}

#[test]
fn emission_shape_ring_roundtrip() {
    let shape = EmissionShape::Ring {
        axis: Vec3::Y,
        height: 0.5,
        radius: 3.0,
        inner_radius: 1.0,
    };
    assert_eq!(roundtrip_ron(&shape), shape);
}

#[test]
fn alpha_mode_all_variants_roundtrip() {
    let variants = [
        SerializableAlphaMode::Opaque,
        SerializableAlphaMode::Mask { cutoff: 0.5 },
        SerializableAlphaMode::Blend,
        SerializableAlphaMode::Premultiplied,
        SerializableAlphaMode::Add,
        SerializableAlphaMode::Multiply,
        SerializableAlphaMode::AlphaToCoverage,
    ];
    for mode in &variants {
        assert_eq!(&roundtrip_ron(mode), mode, "failed for {mode:?}");
    }
}

#[test]
fn transform_align_all_variants_roundtrip() {
    let variants = [
        TransformAlign::Billboard,
        TransformAlign::YToVelocity,
        TransformAlign::BillboardYToVelocity,
        TransformAlign::BillboardFixedY,
    ];
    for align in &variants {
        assert_eq!(&roundtrip_ron(align), align, "failed for {align:?}");
    }
}

#[test]
fn draw_order_all_variants_roundtrip() {
    let variants = [
        DrawOrder::Index,
        DrawOrder::Lifetime,
        DrawOrder::ReverseLifetime,
        DrawOrder::ViewDepth,
    ];
    for order in &variants {
        assert_eq!(&roundtrip_ron(order), order, "failed for {order:?}");
    }
}

#[test]
fn collision_mode_all_variants_roundtrip() {
    let variants = [
        EmitterCollisionMode::Rigid {
            friction: 0.5,
            bounce: 0.8,
        },
        EmitterCollisionMode::HideOnContact,
    ];
    for mode in &variants {
        let serialized =
            ron::ser::to_string_pretty(mode, ron::ser::PrettyConfig::default()).unwrap();
        let deserialized: EmitterCollisionMode = ron::from_str(&serialized).unwrap();
        match (mode, &deserialized) {
            (
                EmitterCollisionMode::Rigid { friction: f1, bounce: b1 },
                EmitterCollisionMode::Rigid { friction: f2, bounce: b2 },
            ) => {
                assert_eq!(f1, f2);
                assert_eq!(b1, b2);
            }
            (EmitterCollisionMode::HideOnContact, EmitterCollisionMode::HideOnContact) => {}
            _ => panic!("collision mode mismatch"),
        }
    }
}

#[test]
fn sub_emitter_mode_all_variants_roundtrip() {
    let variants = [
        SubEmitterMode::Constant,
        SubEmitterMode::AtEnd,
        SubEmitterMode::AtCollision,
        SubEmitterMode::AtStart,
    ];
    for mode in &variants {
        assert_eq!(&roundtrip_ron(mode), mode, "failed for {mode:?}");
    }
}

#[test]
fn curve_mode_all_variants_roundtrip() {
    let variants = [
        CurveMode::SingleCurve,
        CurveMode::DoubleCurve,
        CurveMode::Hold,
        CurveMode::Stairs,
        CurveMode::SmoothStairs,
    ];
    for mode in &variants {
        assert_eq!(&roundtrip_ron(mode), mode, "failed for {mode:?}");
    }
}

#[test]
fn curve_easing_all_variants_roundtrip() {
    let variants = [
        CurveEasing::Power,
        CurveEasing::Sine,
        CurveEasing::Expo,
        CurveEasing::Circ,
    ];
    for easing in &variants {
        assert_eq!(&roundtrip_ron(easing), easing, "failed for {easing:?}");
    }
}

#[test]
fn particle_flags_roundtrip() {
    let flags = ParticleFlags::ROTATE_Y | ParticleFlags::DISABLE_Z;
    let serialized =
        ron::ser::to_string_pretty(&flags, ron::ser::PrettyConfig::default()).unwrap();
    let deserialized: ParticleFlags = ron::from_str(&serialized).unwrap();
    assert_eq!(deserialized, flags);
}

#[test]
fn particle_flags_empty_roundtrip() {
    let flags = ParticleFlags::empty();
    let serialized =
        ron::ser::to_string_pretty(&flags, ron::ser::PrettyConfig::default()).unwrap();
    let deserialized: ParticleFlags = ron::from_str(&serialized).unwrap();
    assert_eq!(deserialized, flags);
}

#[test]
fn gradient_interpolation_all_variants_roundtrip() {
    let variants = [
        GradientInterpolation::Steps,
        GradientInterpolation::Linear,
        GradientInterpolation::Smoothstep,
    ];
    for interp in &variants {
        assert_eq!(&roundtrip_ron(interp), interp, "failed for {interp:?}");
    }
}

#[test]
fn quad_orientation_all_variants_roundtrip() {
    let variants = [
        QuadOrientation::FaceX,
        QuadOrientation::FaceY,
        QuadOrientation::FaceZ,
    ];
    for orient in &variants {
        assert_eq!(&roundtrip_ron(orient), orient, "failed for {orient:?}");
    }
}

#[test]
fn dimension_all_variants_roundtrip() {
    let variants = [
        ParticleSystemDimension::D3,
        ParticleSystemDimension::D2,
    ];
    for dim in &variants {
        assert_eq!(&roundtrip_ron(dim), dim, "failed for {dim:?}");
    }
}
