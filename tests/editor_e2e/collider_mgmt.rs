use super::helpers::*;

use bevy::math::Vec3;
use sprinkles::asset::{ColliderData, ParticlesColliderShape3D};
use sprinkles_editor::state::{EditorState, Inspectable, Inspecting};

#[test]
fn test_add_collider() {
    let mut asset = create_test_asset_with_colliders(&[]);
    assert_eq!(asset.colliders.len(), 0);

    asset.colliders.push(ColliderData::default());

    assert_eq!(asset.colliders.len(), 1);
    assert_eq!(asset.colliders[0].name, "Collider");
}

#[test]
fn test_remove_collider() {
    let mut asset = create_test_asset_with_colliders(&["Floor", "Wall", "Ceiling"]);
    assert_eq!(asset.colliders.len(), 3);

    asset.colliders.remove(1);

    assert_eq!(asset.colliders.len(), 2);
    assert_eq!(asset.colliders[0].name, "Floor");
    assert_eq!(asset.colliders[1].name, "Ceiling");
}

#[test]
fn test_duplicate_collider() {
    let mut asset = create_test_asset_with_colliders(&["Floor"]);
    asset.colliders[0].shape = ParticlesColliderShape3D::Box {
        size: Vec3::new(10.0, 1.0, 10.0),
    };
    asset.colliders[0].position = Vec3::new(0.0, -2.0, 0.0);

    let mut cloned = asset.colliders[0].clone();
    cloned.name = "Floor 2".to_string();
    asset.colliders.insert(1, cloned);

    assert_eq!(asset.colliders.len(), 2);
    assert_eq!(asset.colliders[1].name, "Floor 2");
    assert_eq!(asset.colliders[1].position, Vec3::new(0.0, -2.0, 0.0));
    assert!(matches!(
        asset.colliders[1].shape,
        ParticlesColliderShape3D::Box { size } if size == Vec3::new(10.0, 1.0, 10.0)
    ));
}

#[test]
fn test_rename_collider() {
    let mut asset = create_test_asset_with_colliders(&["Collider"]);
    asset.colliders[0].name = "Floor".to_string();
    assert_eq!(asset.colliders[0].name, "Floor");
}

#[test]
fn test_select_collider_updates_inspecting() {
    let asset = create_test_asset_with_colliders(&["Floor", "Wall"]);
    let mut state = EditorState::default();

    state.inspecting = Some(Inspecting {
        kind: Inspectable::Collider,
        index: 0,
    });
    assert_eq!(
        asset.colliders[state.inspecting.unwrap().index as usize].name,
        "Floor"
    );

    state.inspecting = Some(Inspecting {
        kind: Inspectable::Collider,
        index: 1,
    });
    assert_eq!(
        asset.colliders[state.inspecting.unwrap().index as usize].name,
        "Wall"
    );
}

#[test]
fn test_collider_enabled_toggle() {
    let mut asset = create_test_asset_with_colliders(&["Floor"]);
    assert!(asset.colliders[0].enabled);

    asset.colliders[0].enabled = false;
    assert!(!asset.colliders[0].enabled);

    asset.colliders[0].enabled = true;
    assert!(asset.colliders[0].enabled);
}
