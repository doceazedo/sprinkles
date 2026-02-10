use sprinkles::asset::*;

#[test]
fn range_default() {
    let range = Range::default();
    assert_eq!(range.min, 0.0);
    assert_eq!(range.max, 1.0);
}

#[test]
fn range_with_values() {
    let range = Range::new(2.0, 5.0);
    assert_eq!(range.min, 2.0);
    assert_eq!(range.max, 5.0);
}

#[test]
fn range_span() {
    let range = Range::new(1.0, 4.0);
    assert_eq!(range.span(), 3.0);
}

#[test]
fn range_span_zero_returns_one() {
    let range = Range::new(5.0, 5.0);
    assert_eq!(range.span(), 1.0, "span of zero should return 1.0");
}
