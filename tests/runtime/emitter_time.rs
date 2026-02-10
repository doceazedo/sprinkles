use sprinkles::asset::EmitterTime;

#[test]
fn emitter_time_total_duration() {
    let time = EmitterTime {
        lifetime: 2.0,
        delay: 0.5,
        ..Default::default()
    };
    assert_eq!(time.total_duration(), 2.5);
}

#[test]
fn emitter_time_default_values() {
    let time = EmitterTime::default();
    assert_eq!(time.lifetime, 1.0);
    assert_eq!(time.delay, 0.0);
    assert!(!time.one_shot);
    assert_eq!(time.explosiveness, 0.0);
    assert_eq!(time.spawn_time_randomness, 0.0);
    assert_eq!(time.fixed_fps, 0);
    assert!(time.fixed_seed.is_none());
}
