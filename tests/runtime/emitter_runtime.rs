use sprinkles::runtime::EmitterRuntime;

#[test]
fn new_emitter_runtime_starts_emitting() {
    let runtime = EmitterRuntime::new(0, None);
    assert!(runtime.emitting);
}

#[test]
fn new_emitter_runtime_starts_at_zero() {
    let runtime = EmitterRuntime::new(0, None);
    assert_eq!(runtime.system_time, 0.0);
    assert_eq!(runtime.prev_system_time, 0.0);
    assert_eq!(runtime.cycle, 0);
}

#[test]
fn new_emitter_runtime_stores_index() {
    let runtime = EmitterRuntime::new(3, None);
    assert_eq!(runtime.emitter_index, 3);
}

#[test]
fn new_emitter_runtime_uses_fixed_seed() {
    let runtime = EmitterRuntime::new(0, Some(42));
    assert_eq!(runtime.random_seed, 42);
}

#[test]
fn play_resumes_emitting() {
    let mut runtime = EmitterRuntime::new(0, None);
    runtime.emitting = false;
    runtime.one_shot_completed = true;
    runtime.play();
    assert!(runtime.emitting);
    assert!(!runtime.one_shot_completed);
}

#[test]
fn stop_resets_state() {
    let mut runtime = EmitterRuntime::new(0, Some(42));
    runtime.system_time = 5.0;
    runtime.prev_system_time = 4.5;
    runtime.cycle = 3;
    runtime.accumulated_delta = 0.5;
    runtime.one_shot_completed = true;
    runtime.simulation_steps.push(sprinkles::runtime::SimulationStep {
        prev_system_time: 0.0,
        system_time: 0.0,
        cycle: 0,
        delta_time: 0.0,
        clear_requested: false,
    });

    runtime.stop(Some(42));

    assert!(!runtime.emitting);
    assert_eq!(runtime.system_time, 0.0);
    assert_eq!(runtime.prev_system_time, 0.0);
    assert_eq!(runtime.cycle, 0);
    assert_eq!(runtime.accumulated_delta, 0.0);
    assert!(!runtime.one_shot_completed);
    assert!(runtime.clear_requested);
    assert!(runtime.simulation_steps.is_empty());
}

#[test]
fn restart_resets_and_starts_emitting() {
    let mut runtime = EmitterRuntime::new(0, Some(42));
    runtime.system_time = 5.0;
    runtime.emitting = false;

    runtime.restart(Some(42));

    assert!(runtime.emitting);
    assert_eq!(runtime.system_time, 0.0);
    assert!(runtime.clear_requested);
}

#[test]
fn seek_sets_time() {
    let mut runtime = EmitterRuntime::new(0, None);
    runtime.seek(2.5);
    assert_eq!(runtime.system_time, 2.5);
    assert_eq!(runtime.prev_system_time, 2.5);
}
