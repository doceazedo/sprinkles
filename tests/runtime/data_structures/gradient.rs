use sprinkles::asset::*;

#[test]
fn gradient_default() {
    let gradient = Gradient::default();
    assert_eq!(gradient.stops.len(), 2);
    assert_eq!(gradient.interpolation, GradientInterpolation::Linear);
    assert_eq!(gradient.stops[0].position, 0.0);
    assert_eq!(gradient.stops[1].position, 1.0);
}

#[test]
fn gradient_white() {
    let gradient = Gradient::white();
    assert_eq!(gradient.stops.len(), 2);
    assert_eq!(gradient.stops[0].color, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!(gradient.stops[1].color, [1.0, 1.0, 1.0, 1.0]);
}

#[test]
fn gradient_cache_key_differs() {
    let grad_a = Gradient {
        stops: vec![
            GradientStop { color: [1.0, 0.0, 0.0, 1.0], position: 0.0 },
            GradientStop { color: [0.0, 0.0, 1.0, 1.0], position: 1.0 },
        ],
        interpolation: GradientInterpolation::Linear,
    };
    let grad_b = Gradient {
        stops: vec![
            GradientStop { color: [0.0, 1.0, 0.0, 1.0], position: 0.0 },
            GradientStop { color: [1.0, 1.0, 0.0, 1.0], position: 1.0 },
        ],
        interpolation: GradientInterpolation::Linear,
    };
    assert_ne!(grad_a.cache_key(), grad_b.cache_key());
}

#[test]
fn gradient_interpolation_variants() {
    let linear = GradientInterpolation::Linear;
    let steps = GradientInterpolation::Steps;
    let smoothstep = GradientInterpolation::Smoothstep;

    assert_ne!(linear, steps);
    assert_ne!(linear, smoothstep);
    assert_ne!(steps, smoothstep);
    assert_eq!(GradientInterpolation::default(), GradientInterpolation::Linear);
}
