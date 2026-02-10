use super::helpers::*;

use bevy::prelude::*;
use sprinkles::runtime::{
    CurrentMaterialConfig, CurrentMeshConfig, EmitterEntity, EmitterRuntime, ParticleBufferHandle,
    ParticleMaterialHandle, ParticleMeshHandle, ParticleSystemRuntime,
};

#[test]
fn particle_system_3d_spawns_emitter_entities() {
    let (mut app, _, system_entity) = setup_loaded_system("minimal_particle_system.ron");

    let emitter_count = app
        .world_mut()
        .query::<&EmitterRuntime>()
        .iter(app.world())
        .len();
    assert_eq!(emitter_count, 1, "should spawn 1 emitter");

    let emitter_entity_ref = app
        .world_mut()
        .query::<&EmitterEntity>()
        .iter(app.world())
        .next()
        .expect("emitter entity should exist");
    assert_eq!(
        emitter_entity_ref.parent_system, system_entity,
        "emitter should reference the parent system"
    );
}

#[test]
fn particle_system_3d_spawns_multiple_emitters() {
    let (mut app, ..) = setup_loaded_system("two_emitters.ron");

    let emitter_count = app
        .world_mut()
        .query::<&EmitterRuntime>()
        .iter(app.world())
        .len();
    assert_eq!(emitter_count, 2, "should spawn 2 emitters");

    let mut indices: Vec<usize> = app
        .world_mut()
        .query::<&EmitterRuntime>()
        .iter(app.world())
        .map(|r| r.emitter_index)
        .collect();
    indices.sort();
    assert_eq!(indices, vec![0, 1], "emitters should have indices 0 and 1");
}

#[test]
fn particle_system_gets_runtime_component() {
    let (mut app, _, system_entity) = setup_loaded_system("minimal_particle_system.ron");

    let runtime = app
        .world()
        .get::<ParticleSystemRuntime>(system_entity)
        .expect("system should have runtime component");
    assert!(!runtime.paused, "system should not be paused by default");
    assert!(runtime.force_loop, "system should force_loop by default");
}

#[test]
fn emitters_get_required_components() {
    let (mut app, ..) = setup_loaded_system("minimal_particle_system.ron");

    let emitter_entity = app
        .world_mut()
        .query_filtered::<Entity, With<EmitterRuntime>>()
        .iter(app.world())
        .next()
        .expect("should have an emitter");

    assert!(
        app.world().get::<EmitterEntity>(emitter_entity).is_some(),
        "emitter should have EmitterEntity"
    );
    assert!(
        app.world()
            .get::<ParticleBufferHandle>(emitter_entity)
            .is_some(),
        "emitter should have ParticleBufferHandle"
    );
    assert!(
        app.world()
            .get::<CurrentMeshConfig>(emitter_entity)
            .is_some(),
        "emitter should have CurrentMeshConfig"
    );
    assert!(
        app.world()
            .get::<CurrentMaterialConfig>(emitter_entity)
            .is_some(),
        "emitter should have CurrentMaterialConfig"
    );
    assert!(
        app.world()
            .get::<ParticleMeshHandle>(emitter_entity)
            .is_some(),
        "emitter should have ParticleMeshHandle"
    );
    assert!(
        app.world()
            .get::<ParticleMaterialHandle>(emitter_entity)
            .is_some(),
        "emitter should have ParticleMaterialHandle"
    );
}

#[test]
fn particle_buffer_matches_particle_count() {
    let (mut app, ..) = setup_loaded_system("minimal_particle_system.ron");

    let buffer = app
        .world_mut()
        .query::<&ParticleBufferHandle>()
        .iter(app.world())
        .next()
        .expect("should have a buffer handle");
    assert_eq!(
        buffer.max_particles, 8,
        "buffer max_particles should match fixture"
    );
}
