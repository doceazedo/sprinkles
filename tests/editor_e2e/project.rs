use std::path::Path;

use sprinkles_editor::project::load_project_from_path;

#[test]
fn load_project_from_valid_fixture() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("minimal_particle_system.ron");
    let asset = load_project_from_path(&path);
    assert!(asset.is_some(), "should load valid RON fixture");

    let asset = asset.unwrap();
    assert_eq!(asset.emitters.len(), 1);
    assert_eq!(asset.name, "Minimal System");
}

#[test]
fn load_project_from_two_emitters_fixture() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("two_emitters.ron");
    let asset = load_project_from_path(&path).expect("should load");
    assert_eq!(asset.emitters.len(), 2);
    assert_eq!(asset.emitters[0].name, "Emitter A");
    assert_eq!(asset.emitters[1].name, "Emitter B");
}

#[test]
fn load_project_from_collision_fixture() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("collision_test.ron");
    let asset = load_project_from_path(&path).expect("should load");
    assert_eq!(asset.colliders.len(), 2, "should have 2 colliders");
}

#[test]
fn load_project_from_nonexistent_file() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("does_not_exist.ron");
    assert!(
        load_project_from_path(&path).is_none(),
        "should return None for missing file"
    );
}

#[test]
fn load_project_from_invalid_ron() {
    let dir = std::env::temp_dir().join("sprinkles_test_invalid.ron");
    std::fs::write(&dir, "this is { not valid ron }}}}").unwrap();
    assert!(
        load_project_from_path(&dir).is_none(),
        "should return None for invalid RON"
    );
    std::fs::remove_file(&dir).ok();
}
