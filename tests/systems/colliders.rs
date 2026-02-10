use super::helpers::*;

use bevy::asset::Assets;
use bevy::prelude::*;
use sprinkles::asset::ParticleSystemAsset;
use sprinkles::runtime::{ColliderEntity, ParticlesCollider3D};

#[test]
fn collider_entities_spawned_with_system() {
    let mut app = create_minimal_app();
    let handle = load_fixture(&mut app, "collision_test.ron");
    let system_entity = spawn_3d_particle_system(&mut app, handle.clone());

    assert!(
        run_until_loaded(&mut app, &handle, 100),
        "fixture should load"
    );

    advance_frames(&mut app, 5);

    let colliders: Vec<_> = app
        .world_mut()
        .query::<&ColliderEntity>()
        .iter(app.world())
        .collect();
    assert_eq!(colliders.len(), 2, "should spawn 2 colliders");

    for collider in &colliders {
        assert_eq!(
            collider.parent_system, system_entity,
            "collider should reference parent system"
        );
    }

    let mut collider_indices: Vec<usize> = colliders.iter().map(|c| c.collider_index).collect();
    collider_indices.sort();
    assert_eq!(
        collider_indices,
        vec![0, 1],
        "colliders should have indices 0 and 1"
    );
}

#[test]
fn collider_3d_components_match_config() {
    let mut app = create_minimal_app();
    let handle = load_fixture(&mut app, "collision_test.ron");
    spawn_3d_particle_system(&mut app, handle.clone());

    assert!(
        run_until_loaded(&mut app, &handle, 100),
        "fixture should load"
    );

    advance_frames(&mut app, 5);

    let colliders: Vec<_> = app
        .world_mut()
        .query::<(&ColliderEntity, &ParticlesCollider3D, &Transform)>()
        .iter(app.world())
        .collect();
    assert_eq!(colliders.len(), 2);

    for (collider_entity, collider_3d, transform) in &colliders {
        assert!(collider_3d.enabled, "colliders should be enabled by default");

        let assets = app.world().resource::<Assets<ParticleSystemAsset>>();
        let asset = assets.get(&handle).unwrap();
        let collider_data = &asset.colliders[collider_entity.collider_index];
        assert_eq!(
            transform.translation, collider_data.position,
            "collider position should match fixture"
        );
    }
}
