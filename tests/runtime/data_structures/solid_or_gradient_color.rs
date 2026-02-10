use sprinkles::asset::*;

#[test]
fn solid_color_default() {
    let color = SolidOrGradientColor::default();
    assert!(color.is_solid());
    assert!(!color.is_gradient());
    assert_eq!(color.as_solid_color(), Some([1.0, 1.0, 1.0, 1.0]));
}

#[test]
fn gradient_color() {
    let color = SolidOrGradientColor::Gradient {
        gradient: Gradient::default(),
    };
    assert!(!color.is_solid());
    assert!(color.is_gradient());
    assert_eq!(color.as_solid_color(), None);
}
