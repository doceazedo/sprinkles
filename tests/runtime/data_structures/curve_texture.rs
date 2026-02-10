use sprinkles::asset::*;

#[test]
fn curve_sample_edges() {
    let curve = CurveTexture {
        name: None,
        points: vec![
            CurvePoint::new(0.0, 0.0),
            CurvePoint::new(1.0, 1.0),
        ],
        range: Range::new(0.0, 1.0),
    };
    let at_zero = curve.sample(0.0);
    let at_one = curve.sample(1.0);
    assert!((at_zero - 0.0).abs() < 0.01, "sample at t=0 should be ~0, got {at_zero}");
    assert!((at_one - 1.0).abs() < 0.01, "sample at t=1 should be ~1, got {at_one}");
}

#[test]
fn curve_sample_midpoint() {
    let curve = CurveTexture {
        name: None,
        points: vec![
            CurvePoint::new(0.0, 0.0),
            CurvePoint::new(1.0, 1.0),
        ],
        range: Range::new(0.0, 1.0),
    };
    let mid = curve.sample(0.5);
    assert!(
        (mid - 0.5).abs() < 0.1,
        "sample at t=0.5 should be ~0.5, got {mid}"
    );
}

#[test]
fn curve_sample_empty_returns_one() {
    let curve = CurveTexture {
        name: None,
        points: vec![],
        range: Range::new(0.0, 1.0),
    };
    assert_eq!(curve.sample(0.5), 1.0);
}

#[test]
fn curve_sample_single_point() {
    let curve = CurveTexture {
        name: None,
        points: vec![CurvePoint::new(0.5, 0.75)],
        range: Range::new(0.0, 1.0),
    };
    assert_eq!(curve.sample(0.0), 0.75);
    assert_eq!(curve.sample(1.0), 0.75);
}

#[test]
fn curve_hold_mode() {
    let curve = CurveTexture {
        name: None,
        points: vec![
            CurvePoint::new(0.0, 1.0),
            CurvePoint::new(1.0, 0.0).with_mode(CurveMode::Hold),
        ],
        range: Range::new(0.0, 1.0),
    };
    let mid = curve.sample(0.5);
    assert!(
        (mid - 1.0).abs() < 0.01,
        "hold mode should stay at left value, got {mid}"
    );
}

#[test]
fn curve_is_constant() {
    let constant = CurveTexture {
        name: None,
        points: vec![
            CurvePoint::new(0.0, 1.0),
            CurvePoint::new(1.0, 1.0),
        ],
        range: Range::new(0.0, 1.0),
    };
    assert!(constant.is_constant());

    let varying = CurveTexture {
        name: None,
        points: vec![
            CurvePoint::new(0.0, 1.0),
            CurvePoint::new(1.0, 0.0),
        ],
        range: Range::new(0.0, 1.0),
    };
    assert!(!varying.is_constant());
}

#[test]
fn curve_default() {
    let curve = CurveTexture::default();
    assert_eq!(curve.points.len(), 2);
    assert!(curve.is_constant(), "default curve should be constant");
    assert_eq!(curve.sample(0.0), 1.0);
    assert_eq!(curve.sample(1.0), 1.0);
}

#[test]
fn curve_cache_key_differs_for_different_curves() {
    let curve_a = CurveTexture {
        name: None,
        points: vec![
            CurvePoint::new(0.0, 1.0),
            CurvePoint::new(1.0, 0.0),
        ],
        range: Range::new(0.0, 1.0),
    };
    let curve_b = CurveTexture {
        name: None,
        points: vec![
            CurvePoint::new(0.0, 0.0),
            CurvePoint::new(1.0, 1.0),
        ],
        range: Range::new(0.0, 1.0),
    };
    assert_ne!(curve_a.cache_key(), curve_b.cache_key());
}
