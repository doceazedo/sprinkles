use super::helpers::*;

use bevy::asset::Assets;
use bevy::prelude::*;
use sprinkles::asset::*;
use sprinkles::runtime::{EmitterRuntime, ParticleBufferHandle};

#[test]
fn emitter_runtime_reflects_emitter_index() {
    let (mut app, ..) = setup_loaded_system("two_emitters.ron");

    let mut indices: Vec<usize> = app
        .world_mut()
        .query::<&EmitterRuntime>()
        .iter(app.world())
        .map(|r| r.emitter_index)
        .collect();
    indices.sort();
    assert_eq!(indices, vec![0, 1]);
}

#[test]
fn emitter_buffer_matches_particle_count() {
    let (mut app, handle, _) = setup_loaded_system("two_emitters.ron");

    let asset = {
        let assets = app.world().resource::<Assets<ParticleSystemAsset>>();
        assets.get(&handle).unwrap().clone()
    };

    let buffers: Vec<(usize, u32)> = app
        .world_mut()
        .query::<(&EmitterRuntime, &ParticleBufferHandle)>()
        .iter(app.world())
        .map(|(r, b)| (r.emitter_index, b.max_particles))
        .collect();

    for (emitter_index, max_particles) in &buffers {
        let expected = asset.emitters[*emitter_index].emission.particles_amount;
        assert_eq!(
            *max_particles, expected,
            "buffer max_particles should match emitter {} particle count",
            emitter_index
        );
    }
}

#[test]
fn multiple_systems_have_independent_runtimes() {
    let mut app = create_minimal_app();
    let handle_a = load_fixture(&mut app, "minimal_particle_system.ron");
    let handle_b = load_fixture(&mut app, "two_emitters.ron");
    let _entity_a = spawn_3d_particle_system(&mut app, handle_a.clone());
    let _entity_b = spawn_3d_particle_system(&mut app, handle_b.clone());

    assert!(run_until_loaded(&mut app, &handle_a, 100));
    assert!(run_until_loaded(&mut app, &handle_b, 100));
    advance_frames(&mut app, 5);

    let runtime_count = app
        .world_mut()
        .query::<&EmitterRuntime>()
        .iter(app.world())
        .len();
    assert_eq!(runtime_count, 3, "should have 1 + 2 = 3 emitter runtimes");
}
