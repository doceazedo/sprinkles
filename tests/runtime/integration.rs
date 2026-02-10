use super::helpers::*;

use sprinkles::runtime::ParticleSystemRuntime;

#[test]
fn paused_system_doesnt_advance_emitter_time() {
    let mut app = create_minimal_app();
    let handle = load_fixture(&mut app, "minimal_particle_system.ron");
    let system_entity = spawn_3d_particle_system(&mut app, handle.clone());

    assert!(
        run_until_loaded(&mut app, &handle, 100),
        "fixture should load"
    );

    advance_frames(&mut app, 3);

    let time_before = app
        .world_mut()
        .query::<&sprinkles::runtime::EmitterRuntime>()
        .iter(app.world())
        .next()
        .expect("should have emitter")
        .system_time;

    app.world_mut()
        .get_mut::<ParticleSystemRuntime>(system_entity)
        .expect("should have system runtime")
        .pause();

    advance_frames(&mut app, 10);

    let time_after = app
        .world_mut()
        .query::<&sprinkles::runtime::EmitterRuntime>()
        .iter(app.world())
        .next()
        .expect("should have emitter")
        .system_time;

    assert_eq!(
        time_before, time_after,
        "time should not advance when paused"
    );
}
