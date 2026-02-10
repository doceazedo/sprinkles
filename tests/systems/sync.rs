use super::helpers::*;

use bevy::asset::Assets;
use bevy::prelude::*;
use sprinkles::asset::*;
use sprinkles::runtime::{
    CurrentMaterialConfig, CurrentMeshConfig, EmitterRuntime, ParticleMaterial,
    ParticleMaterialHandle, ParticleMeshHandle, ParticleSystem3D,
};

#[test]
fn mesh_config_matches_initial_asset() {
    let (mut app, handle, _) = setup_loaded_system("minimal_particle_system.ron");

    let config = app
        .world_mut()
        .query::<&CurrentMeshConfig>()
        .iter(app.world())
        .next()
        .expect("should have mesh config");

    let assets = app.world().resource::<Assets<ParticleSystemAsset>>();
    let asset = assets.get(&handle).unwrap();
    assert_eq!(
        config.0, asset.emitters[0].draw_pass.mesh,
        "mesh config should match asset"
    );
}

#[test]
fn material_config_matches_initial_asset() {
    let (mut app, _, _) = setup_loaded_system("minimal_particle_system.ron");

    let has_material_config = app
        .world_mut()
        .query::<&CurrentMaterialConfig>()
        .iter(app.world())
        .next()
        .is_some();
    assert!(has_material_config, "emitter should have material config");
}

#[test]
fn mesh_change_updates_config() {
    let (mut app, handle, _) = setup_loaded_system("minimal_particle_system.ron");

    // modify the asset's mesh to a different variant
    let mut assets = app.world_mut().resource_mut::<Assets<ParticleSystemAsset>>();
    let asset = assets.get_mut(&handle).unwrap();
    asset.emitters[0].draw_pass.mesh = ParticleMesh::Cuboid {
        half_size: Vec3::new(0.5, 0.5, 0.5),
    };

    advance_frames(&mut app, 3);

    let config = app
        .world_mut()
        .query::<&CurrentMeshConfig>()
        .iter(app.world())
        .next()
        .expect("should have mesh config");
    match &config.0 {
        ParticleMesh::Cuboid { half_size } => {
            assert_eq!(*half_size, Vec3::new(0.5, 0.5, 0.5));
        }
        _ => panic!("mesh config should have been updated to Cuboid"),
    }
}

#[test]
fn each_mesh_variant_can_be_assigned() {
    let (mut app, _, _) = setup_loaded_system("all_emission_shapes.ron");

    let mesh_count = app
        .world_mut()
        .query::<&ParticleMeshHandle>()
        .iter(app.world())
        .len();
    assert_eq!(mesh_count, 5, "all 5 emitters should have mesh handles");
}

#[test]
fn material_change_updates_handle() {
    let (mut app, handle, _) = setup_loaded_system("minimal_particle_system.ron");

    // get original material handle id
    let original_handle = app
        .world_mut()
        .query::<&ParticleMaterialHandle>()
        .iter(app.world())
        .next()
        .expect("should have material handle")
        .0
        .id();

    // modify the material in the asset
    let mut assets = app.world_mut().resource_mut::<Assets<ParticleSystemAsset>>();
    let asset = assets.get_mut(&handle).unwrap();
    if let DrawPassMaterial::Standard(ref mut mat) = asset.emitters[0].draw_pass.material {
        mat.base_color = [1.0, 0.0, 0.0, 1.0];
    }

    advance_frames(&mut app, 3);

    let new_handle = app
        .world_mut()
        .query::<&ParticleMaterialHandle>()
        .iter(app.world())
        .next()
        .expect("should have material handle")
        .0
        .id();

    assert_ne!(
        original_handle, new_handle,
        "material handle should change after material modification"
    );
}

#[test]
fn collider_enabled_toggle() {
    use sprinkles::runtime::{ColliderEntity, ParticlesCollider3D};

    let (mut app, handle, _) = setup_loaded_system("collision_test.ron");

    // all colliders should be enabled by default
    let colliders: Vec<_> = app
        .world_mut()
        .query::<(&ColliderEntity, &ParticlesCollider3D)>()
        .iter(app.world())
        .collect();
    assert!(colliders.iter().all(|(_, c)| c.enabled));

    // disable a collider in the asset
    let mut assets = app.world_mut().resource_mut::<Assets<ParticleSystemAsset>>();
    let asset = assets.get_mut(&handle).unwrap();
    asset.colliders[0].enabled = false;

    advance_frames(&mut app, 3);

    let colliders: Vec<_> = app
        .world_mut()
        .query::<(&ColliderEntity, &ParticlesCollider3D)>()
        .iter(app.world())
        .collect();
    let floor = colliders
        .iter()
        .find(|(ce, _)| ce.collider_index == 0)
        .expect("should have floor collider");
    assert!(
        !floor.1.enabled,
        "collider should be disabled after asset modification"
    );
}

#[test]
fn removed_system_cleans_up_colliders() {
    use sprinkles::runtime::ColliderEntity;

    let (mut app, _, system_entity) = setup_loaded_system("collision_test.ron");

    let collider_count = app
        .world_mut()
        .query::<&ColliderEntity>()
        .iter(app.world())
        .len();
    assert_eq!(collider_count, 2, "should have 2 colliders before cleanup");

    app.world_mut()
        .entity_mut(system_entity)
        .remove::<ParticleSystem3D>();
    advance_frames(&mut app, 5);

    let collider_count = app
        .world_mut()
        .query::<&ColliderEntity>()
        .iter(app.world())
        .len();
    assert_eq!(collider_count, 0, "colliders should be cleaned up");
}
