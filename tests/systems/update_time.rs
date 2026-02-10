use super::helpers::*;

use sprinkles::runtime::EmitterRuntime;

#[test]
fn simulation_steps_generated() {
    let mut app = create_minimal_app();
    let handle = load_fixture(&mut app, "minimal_particle_system.ron");
    spawn_3d_particle_system(&mut app, handle.clone());

    assert!(run_until_loaded(&mut app, &handle, 100));
    advance_frames(&mut app, 3);

    // after advancing, the emitter should have simulation steps from the last frame
    app.update();

    let runtime = app
        .world_mut()
        .query::<&EmitterRuntime>()
        .iter(app.world())
        .next()
        .expect("should have emitter");
    assert!(
        !runtime.simulation_steps.is_empty(),
        "should have simulation steps after update"
    );

    let step = &runtime.simulation_steps[0];
    assert!(step.delta_time > 0.0, "step should have positive delta");
    assert!(
        step.system_time > step.prev_system_time,
        "system_time should advance"
    );
}

#[test]
fn one_shot_emitter_stops_after_lifetime() {
    let mut app = create_minimal_app();
    let handle = load_fixture(&mut app, "one_shot.ron");
    spawn_3d_particle_system(&mut app, handle.clone());

    assert!(run_until_loaded(&mut app, &handle, 100));
    advance_frames(&mut app, 3);

    // one_shot fixture has lifetime = 0.5s + delay 0.0s = total_duration 0.5s
    // advance real time past the total duration so the emitter cycles
    advance_time(&mut app, 0.8);

    let runtime = app
        .world_mut()
        .query::<&EmitterRuntime>()
        .iter(app.world())
        .next()
        .expect("should have emitter");
    assert!(
        runtime.one_shot_completed,
        "one_shot emitter should be completed"
    );
    assert!(
        !runtime.emitting,
        "one_shot emitter should stop emitting after lifetime"
    );
}

#[test]
fn looping_emitter_cycles() {
    let mut app = create_minimal_app();
    let handle = load_fixture(&mut app, "minimal_particle_system.ron");
    spawn_3d_particle_system(&mut app, handle.clone());

    assert!(run_until_loaded(&mut app, &handle, 100));
    advance_frames(&mut app, 3);

    // minimal fixture has lifetime = 1.0s, total_duration = 1.0s
    // advance real time past the total_duration so the emitter wraps
    advance_time(&mut app, 1.5);

    let runtime = app
        .world_mut()
        .query::<&EmitterRuntime>()
        .iter(app.world())
        .next()
        .expect("should have emitter");
    assert!(
        runtime.cycle > 0,
        "emitter should have cycled at least once after exceeding lifetime"
    );
}

#[test]
fn fixed_fps_quantizes_simulation_steps() {
    let (mut app, ..) = setup_loaded_system("fixed_fps.ron");

    // run one more frame and check steps
    app.update();

    let runtime = app
        .world_mut()
        .query::<&EmitterRuntime>()
        .iter(app.world())
        .next()
        .expect("should have emitter");

    // with fixed_fps=30, each step should have delta_time ≈ 1/30 ≈ 0.0333s
    // the schedule runner ticks at 10ms, so we may or may not get a step each frame
    // but when steps exist, their delta_time should be consistent
    if !runtime.simulation_steps.is_empty() {
        let expected_delta = 1.0 / 30.0;
        for step in &runtime.simulation_steps {
            assert!(
                (step.delta_time - expected_delta).abs() < 0.001,
                "fixed_fps step delta should be ~{expected_delta}, got {}",
                step.delta_time
            );
        }
    }
}

#[test]
fn disabled_emitter_still_spawns_entity() {
    let (mut app, ..) = setup_loaded_system("disabled_emitter.ron");

    let emitter_count = app
        .world_mut()
        .query::<&EmitterRuntime>()
        .iter(app.world())
        .len();
    assert_eq!(
        emitter_count, 2,
        "both enabled and disabled emitters should spawn entities"
    );
}
