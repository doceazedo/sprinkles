use sprinkles::asset::EmitterTime;
use sprinkles::runtime::EmitterRuntime;

#[test]
fn system_phase_no_delay() {
    let mut runtime = EmitterRuntime::new(0, None);
    runtime.system_time = 0.5;
    let time = EmitterTime {
        lifetime: 1.0,
        delay: 0.0,
        ..Default::default()
    };
    let phase = runtime.system_phase(&time);
    assert!((phase - 0.5).abs() < 0.001);
}

#[test]
fn system_phase_with_delay() {
    let mut runtime = EmitterRuntime::new(0, None);
    runtime.system_time = 0.7;
    let time = EmitterTime {
        lifetime: 1.0,
        delay: 0.5,
        ..Default::default()
    };
    let phase = runtime.system_phase(&time);
    assert!(
        (phase - 0.2).abs() < 0.001,
        "phase should be (0.7 - 0.5) / 1.0 = 0.2, got {phase}"
    );
}

#[test]
fn system_phase_before_delay() {
    let mut runtime = EmitterRuntime::new(0, None);
    runtime.system_time = 0.3;
    let time = EmitterTime {
        lifetime: 1.0,
        delay: 0.5,
        ..Default::default()
    };
    let phase = runtime.system_phase(&time);
    assert_eq!(phase, 0.0, "phase should be 0 before delay elapses");
}

#[test]
fn system_phase_zero_lifetime() {
    let mut runtime = EmitterRuntime::new(0, None);
    runtime.system_time = 1.0;
    let time = EmitterTime {
        lifetime: 0.0,
        ..Default::default()
    };
    assert_eq!(runtime.system_phase(&time), 0.0);
}

#[test]
fn is_past_delay_before() {
    let mut runtime = EmitterRuntime::new(0, None);
    runtime.system_time = 0.2;
    let time = EmitterTime {
        lifetime: 1.0,
        delay: 0.5,
        ..Default::default()
    };
    assert!(!runtime.is_past_delay(&time));
}

#[test]
fn is_past_delay_after() {
    let mut runtime = EmitterRuntime::new(0, None);
    runtime.system_time = 0.6;
    let time = EmitterTime {
        lifetime: 1.0,
        delay: 0.5,
        ..Default::default()
    };
    assert!(runtime.is_past_delay(&time));
}

#[test]
fn is_past_delay_zero_total_duration() {
    let mut runtime = EmitterRuntime::new(0, None);
    runtime.system_time = 0.0;
    let time = EmitterTime {
        lifetime: 0.0,
        delay: 0.0,
        ..Default::default()
    };
    assert!(
        runtime.is_past_delay(&time),
        "should return true when total_duration is 0"
    );
}
