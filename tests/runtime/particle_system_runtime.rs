use sprinkles::runtime::ParticleSystemRuntime;

#[test]
fn default_runtime_not_paused() {
    let runtime = ParticleSystemRuntime::default();
    assert!(!runtime.paused);
}

#[test]
fn default_runtime_force_loop() {
    let runtime = ParticleSystemRuntime::default();
    assert!(runtime.force_loop);
}

#[test]
fn pause_sets_paused() {
    let mut runtime = ParticleSystemRuntime::default();
    runtime.pause();
    assert!(runtime.paused);
}

#[test]
fn resume_clears_paused() {
    let mut runtime = ParticleSystemRuntime::default();
    runtime.pause();
    runtime.resume();
    assert!(!runtime.paused);
}

#[test]
fn toggle_flips_paused() {
    let mut runtime = ParticleSystemRuntime::default();
    assert!(!runtime.paused);
    runtime.toggle();
    assert!(runtime.paused);
    runtime.toggle();
    assert!(!runtime.paused);
}
