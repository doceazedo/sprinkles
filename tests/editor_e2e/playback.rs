use super::helpers::*;

use sprinkles::runtime::EmitterRuntime;

#[test]
fn test_elapsed_time_advances() {
    let mut app = create_minimal_app();

    let handle = load_fixture(&mut app, "minimal_particle_system.ron");
    assert!(run_until_loaded(&mut app, &handle, 100));
    spawn_3d_particle_system(&mut app, handle);

    advance_time(&mut app, 0.2);

    let mut found = false;
    for runtime in app.world_mut().query::<&EmitterRuntime>().iter(app.world()) {
        assert!(
            runtime.system_time > 0.0,
            "system_time should advance, got {}",
            runtime.system_time
        );
        found = true;
    }
    assert!(found, "should find at least one EmitterRuntime");
}
