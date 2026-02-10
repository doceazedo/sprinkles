use super::*;

use std::path::Path;

use bevy::math::Vec3;
use sprinkles_editor::project::load_project_from_path;

#[test]
fn test_inspect_maximal_fixture_reflection_paths() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("maximal_emitter.ron");
    let asset = load_project_from_path(&path).expect("should load maximal_emitter.ron");
    let emitter = &asset.emitters[0];

    assert_eq!(read_f32(emitter, "time.lifetime"), 3.0);
    assert_eq!(read_f32(emitter, "time.explosiveness"), 0.3);
    assert_eq!(read_f32(emitter, "time.delay"), 0.5);
    assert!(!read_bool(emitter, "time.one_shot"));
    assert_eq!(read_u32(emitter, "time.fixed_fps"), 60);
    assert_eq!(read_u32(emitter, "emission.particles_amount"), 64);
    assert_eq!(read_f32(emitter, "velocities.spread"), 30.0);
    assert!(read_bool(emitter, "turbulence.enabled"));
    assert_eq!(read_f32(emitter, "turbulence.noise_strength"), 2.0);
    assert_eq!(read_f32(emitter, "collision.base_size"), 0.05);
    assert!(read_bool(emitter, "collision.use_scale"));
    assert_eq!(
        read_vec3(emitter, "accelerations.gravity"),
        Vec3::new(0.0, -15.0, 0.0)
    );
    assert_eq!(
        read_vec3(emitter, "emission.offset"),
        Vec3::new(0.5, 1.0, -0.5)
    );
    assert_eq!(read_f32(emitter, "scale.range.min"), 0.5);
    assert_eq!(read_f32(emitter, "scale.range.max"), 2.0);
}
