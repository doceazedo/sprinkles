use sprinkles::asset::EmitterTime;

#[test]
fn compute_phase_no_delay() {
    let time = EmitterTime {
        lifetime: 2.0,
        delay: 0.0,
        ..Default::default()
    };
    let phase = sprinkles::runtime::compute_phase(1.0, &time);
    assert!((phase - 0.5).abs() < 0.001);
}

#[test]
fn compute_phase_with_delay() {
    let time = EmitterTime {
        lifetime: 1.0,
        delay: 0.5,
        ..Default::default()
    };
    let phase = sprinkles::runtime::compute_phase(1.0, &time);
    assert!(
        (phase - 0.5).abs() < 0.001,
        "phase should be (1.0 - 0.5) / 1.0 = 0.5, got {phase}"
    );
}

#[test]
fn compute_phase_before_delay() {
    let time = EmitterTime {
        lifetime: 1.0,
        delay: 2.0,
        ..Default::default()
    };
    let phase = sprinkles::runtime::compute_phase(1.0, &time);
    assert_eq!(phase, 0.0);
}

#[test]
fn free_fn_is_past_delay_before() {
    let time = EmitterTime {
        lifetime: 1.0,
        delay: 0.5,
        ..Default::default()
    };
    assert!(!sprinkles::runtime::is_past_delay(0.2, &time));
}

#[test]
fn free_fn_is_past_delay_after() {
    let time = EmitterTime {
        lifetime: 1.0,
        delay: 0.5,
        ..Default::default()
    };
    assert!(sprinkles::runtime::is_past_delay(0.6, &time));
}
