use std::path::Path;

use super::{
    capture_frame,
    helpers::{compare_images, screenshots_baseline_path, screenshots_tmp_path},
    BASELINE_TOLERANCE_AVG_DIFF, BASELINE_TOLERANCE_RATIO, CAPTURE_HEIGHT, CAPTURE_WIDTH,
    PER_CHANNEL_TOLERANCE,
};

fn save_png(path: &Path, data: &[u8], width: u32, height: u32) {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).unwrap();
    }
    let img = image::RgbaImage::from_raw(width, height, data.to_vec())
        .expect("failed to create image buffer");
    img.save(path).expect("failed to save png");
}

fn load_png(path: &Path) -> Vec<u8> {
    let img = image::open(path).expect("failed to load png");
    img.to_rgba8().into_raw()
}

pub(crate) fn compare_or_generate(test_name: &str, frame_data: &[u8]) {
    let baseline_dir = screenshots_baseline_path();
    let tmp_dir = screenshots_tmp_path();
    let baseline_path = baseline_dir.join(format!("{test_name}.png"));
    let tmp_path = tmp_dir.join(format!("{test_name}.png"));

    save_png(&tmp_path, frame_data, CAPTURE_WIDTH, CAPTURE_HEIGHT);

    if !baseline_path.exists() {
        save_png(&baseline_path, frame_data, CAPTURE_WIDTH, CAPTURE_HEIGHT);
        println!("  [generated baseline: {}]", baseline_path.display());
        return;
    }

    let baseline_data = load_png(&baseline_path);
    assert_eq!(
        frame_data.len(),
        baseline_data.len(),
        "frame size mismatch for {test_name}"
    );

    let diff = compare_images(frame_data, &baseline_data, PER_CHANNEL_TOLERANCE);
    let ratio = diff.different_pixels as f64 / diff.total_pixels as f64;

    assert!(
        diff.within_tolerance(BASELINE_TOLERANCE_RATIO, BASELINE_TOLERANCE_AVG_DIFF),
        "visual regression for '{test_name}': {:.2}% pixels differ (max {:.0}%), \
         avg diff {:.2} (max {:.0}), max channel diff {}",
        ratio * 100.0,
        BASELINE_TOLERANCE_RATIO * 100.0,
        diff.avg_diff,
        BASELINE_TOLERANCE_AVG_DIFF,
        diff.max_channel_diff,
    );
}

pub(crate) fn capture_and_compare(name: &str, fixture: &str, frame: u32) {
    let data = capture_frame(fixture, frame).expect("failed to capture frame");
    compare_or_generate(name, &data);
}
