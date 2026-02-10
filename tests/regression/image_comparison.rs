use super::helpers::*;

#[test]
fn image_comparison_identical_images() {
    let image_a: Vec<u8> = vec![
        255, 0, 0, 255, // red pixel
        0, 255, 0, 255, // green pixel
        0, 0, 255, 255, // blue pixel
        255, 255, 255, 255, // white pixel
    ];
    let image_b = image_a.clone();

    let diff = compare_images(&image_a, &image_b, 0);
    assert_eq!(diff.total_pixels, 4);
    assert_eq!(diff.different_pixels, 0);
    assert_eq!(diff.max_channel_diff, 0);
    assert!(diff.avg_diff < f64::EPSILON);
    assert!(diff.within_tolerance(0.0, 0.0));
}

#[test]
fn image_comparison_completely_different() {
    let image_a: Vec<u8> = vec![
        255, 0, 0, 255, // red
        255, 0, 0, 255, // red
    ];
    let image_b: Vec<u8> = vec![
        0, 255, 0, 255, // green
        0, 255, 0, 255, // green
    ];

    let diff = compare_images(&image_a, &image_b, 0);
    assert_eq!(diff.total_pixels, 2);
    assert_eq!(diff.different_pixels, 2);
    assert_eq!(diff.max_channel_diff, 255);
    assert!(!diff.within_tolerance(0.0, 0.0));
}

#[test]
fn image_comparison_within_tolerance() {
    let image_a: Vec<u8> = vec![100, 100, 100, 255];
    let image_b: Vec<u8> = vec![103, 98, 101, 255];

    let diff = compare_images(&image_a, &image_b, 5);
    assert_eq!(diff.different_pixels, 0, "within per-channel tolerance");

    let diff_strict = compare_images(&image_a, &image_b, 0);
    assert_eq!(
        diff_strict.different_pixels, 1,
        "should differ with zero tolerance"
    );
}

#[test]
fn image_comparison_ratio_check() {
    let image_a = vec![128u8; 400]; // 100 pixels, all gray
    let mut image_b = image_a.clone();

    // change 5 out of 100 pixels
    for i in 0..5 {
        image_b[i * 4] = 0;
    }

    let diff = compare_images(&image_a, &image_b, 0);
    assert_eq!(diff.different_pixels, 5);
    assert!(
        diff.within_tolerance(0.06, 10.0),
        "5% should be within 6% tolerance"
    );
    assert!(
        !diff.within_tolerance(0.04, 10.0),
        "5% should NOT be within 4% tolerance"
    );
}

#[test]
fn compare_screenshot_rgba_passes_for_identical() {
    let image: Vec<u8> = vec![128; 400];
    let result = compare_screenshot_rgba(&image, &image, 0, 0.0, 0.0);
    assert!(result.is_ok());
}

#[test]
fn compare_screenshot_rgba_fails_for_different() {
    let image_a: Vec<u8> = vec![0; 400];
    let image_b: Vec<u8> = vec![255; 400];
    let result = compare_screenshot_rgba(&image_a, &image_b, 0, 0.01, 1.0);
    assert!(result.is_err());
}
