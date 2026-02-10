use super::{
    capture_frame,
    comparison::capture_and_compare,
    helpers::compare_images,
    BASELINE_TOLERANCE_AVG_DIFF, BASELINE_TOLERANCE_RATIO, PER_CHANNEL_TOLERANCE,
};

fn test_fixed_seed_determinism() {
    let data_a = capture_frame("fixed_seed.ron", 30).expect("failed to capture frame (run a)");
    let data_b = capture_frame("fixed_seed.ron", 30).expect("failed to capture frame (run b)");
    let diff = compare_images(&data_a, &data_b, PER_CHANNEL_TOLERANCE);
    let ratio = diff.different_pixels as f64 / diff.total_pixels as f64;
    assert!(
        diff.within_tolerance(BASELINE_TOLERANCE_RATIO, BASELINE_TOLERANCE_AVG_DIFF),
        "fixed_seed runs differ too much: {:.2}% pixels differ, avg diff {:.2}, max channel diff {}",
        ratio * 100.0,
        diff.avg_diff,
        diff.max_channel_diff,
    );
}

pub(crate) fn all_tests() -> Vec<(&'static str, Box<dyn Fn()>)> {
    let snapshot_tests: Vec<(&str, &str, u32)> = vec![
        ("fountain_frame_30", "visual_reference_fountain.ron", 30),
        ("fountain_frame_60", "visual_reference_fountain.ron", 60),
        ("explosion_frame_10", "visual_reference_explosion.ron", 10),
        ("explosion_frame_30", "visual_reference_explosion.ron", 30),
        ("emission_shape_point", "minimal_particle_system.ron", 30),
        ("emission_shape_sphere", "all_emission_shapes.ron", 30),
        ("multiple_emitters", "two_emitters.ron", 30),
        ("color_over_lifetime", "gradients_test.ron", 30),
        ("scale_curves", "curves_test.ron", 30),
        ("collision", "collision_test.ron", 30),
        ("sub_emitter", "sub_emitter_test.ron", 30),
        ("maximal_emitter", "maximal_emitter.ron", 30),
        ("disabled_emitter", "disabled_emitter.ron", 30),
        ("fixed_fps", "fixed_fps.ron", 30),
        ("one_shot", "one_shot.ron", 20),
    ];

    snapshot_tests
        .into_iter()
        .map(|(name, fixture, frame)| {
            let fixture = fixture.to_string();
            let name_owned = name.to_string();
            let test_fn: Box<dyn Fn()> =
                Box::new(move || capture_and_compare(&name_owned, &fixture, frame));
            (name, test_fn)
        })
        .chain(std::iter::once((
            "fixed_seed_determinism",
            Box::new(test_fixed_seed_determinism) as Box<dyn Fn()>,
        )))
        .collect()
}
