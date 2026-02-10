use super::helpers::*;

use bevy::prelude::*;
use sprinkles::runtime::{EmitterRuntime, ParticleSystem3D};

#[test]
fn system_time_advances_each_frame() {
    let mut app = create_minimal_app();
    let handle = load_fixture(&mut app, "minimal_particle_system.ron");
    spawn_3d_particle_system(&mut app, handle.clone());

    assert!(
        run_until_loaded(&mut app, &handle, 100),
        "fixture should load"
    );

    advance_frames(&mut app, 5);

    let runtime = app
        .world_mut()
        .query::<&EmitterRuntime>()
        .iter(app.world())
        .next()
        .expect("should have emitter runtime");
    assert!(
        runtime.system_time > 0.0,
        "system time should have advanced"
    );
}

#[test]
fn cleanup_removes_emitters_when_system_despawned() {
    let mut app = create_minimal_app();
    let handle = load_fixture(&mut app, "minimal_particle_system.ron");
    let system_entity = spawn_3d_particle_system(&mut app, handle.clone());

    assert!(
        run_until_loaded(&mut app, &handle, 100),
        "fixture should load"
    );

    advance_frames(&mut app, 5);

    let emitter_count = app
        .world_mut()
        .query::<&EmitterRuntime>()
        .iter(app.world())
        .len();
    assert_eq!(emitter_count, 1, "should have 1 emitter before cleanup");

    app.world_mut()
        .entity_mut(system_entity)
        .remove::<ParticleSystem3D>();
    advance_frames(&mut app, 5);

    let emitter_count = app
        .world_mut()
        .query::<&EmitterRuntime>()
        .iter(app.world())
        .len();
    assert_eq!(emitter_count, 0, "emitters should be cleaned up");
}
