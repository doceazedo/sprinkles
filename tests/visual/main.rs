mod comparison;
mod frame_capture;
mod scene;
mod test_cases;

#[path = "../helpers/mod.rs"]
mod helpers;

use std::sync::{Arc, Mutex};
use std::time::Duration;

use bevy::{
    app::ScheduleRunnerPlugin,
    prelude::*,
    window::ExitCondition,
    winit::WinitPlugin,
};

use frame_capture::*;
use helpers::*;
use scene::*;

pub(crate) const CAPTURE_WIDTH: u32 = 400;
pub(crate) const CAPTURE_HEIGHT: u32 = 300;
pub(crate) const PRE_ROLL_FRAMES: u32 = 20;
pub(crate) const BASELINE_TOLERANCE_RATIO: f64 = 0.15;
pub(crate) const BASELINE_TOLERANCE_AVG_DIFF: f64 = 12.0;
pub(crate) const PER_CHANNEL_TOLERANCE: u8 = 20;

pub(crate) fn capture_frame(fixture: &str, target_frame: u32) -> Option<Vec<u8>> {
    let output = CapturedFrameOutput(Arc::new(Mutex::new(None)));
    let output_clone = output.clone();

    let mut app = App::new();

    app.add_plugins(
        DefaultPlugins
            .set(bevy::asset::AssetPlugin {
                file_path: fixtures_path(),
                ..default()
            })
            .set(ImagePlugin::default_nearest())
            .set(WindowPlugin {
                primary_window: None,
                exit_condition: ExitCondition::DontExit,
                ..default()
            })
            .disable::<WinitPlugin>(),
    );

    app.add_plugins(ScheduleRunnerPlugin::run_loop(
        Duration::from_millis(1),
    ));

    app.add_plugins(sprinkles::SprinklesPlugin);
    app.add_plugins(ImageCopyPlugin);

    app.insert_resource(CaptureConfig {
        target_frame,
        current_frame: 0,
        width: CAPTURE_WIDTH,
        height: CAPTURE_HEIGHT,
        fixture: fixture.to_string(),
        system_spawned: false,
    });
    app.insert_resource(output);

    app.add_systems(Startup, setup_scene);
    app.add_systems(Update, (spawn_particle_system, capture_orchestrator));

    app.run();

    output_clone.0.lock().unwrap().take()
}

fn main() {
    if std::env::var("RUN_VISUAL_TESTS").is_err() {
        println!("visual regression tests are ignored. set RUN_VISUAL_TESTS=1 to run them.");
        return;
    }

    let tests = test_cases::all_tests();

    let args: Vec<String> = std::env::args().collect();
    let filter = args.get(1).map(|s| s.as_str());

    let filtered: Vec<_> = tests
        .into_iter()
        .filter(|(name, _)| filter.is_none_or(|f| name.contains(f)))
        .collect();

    if filtered.is_empty() {
        println!("no visual tests matched filter");
        return;
    }

    println!("\nrunning {} visual regression test(s)\n", filtered.len());

    let mut passed = 0u32;
    let mut failed = 0u32;
    let mut failed_names = Vec::new();

    for (name, test_fn) in &filtered {
        print!("test {name} ... ");
        match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| test_fn())) {
            Ok(_) => {
                println!("ok");
                passed += 1;
            }
            Err(e) => {
                let msg = if let Some(s) = e.downcast_ref::<String>() {
                    s.clone()
                } else if let Some(s) = e.downcast_ref::<&str>() {
                    s.to_string()
                } else {
                    "unknown panic".to_string()
                };
                println!("FAILED\n  {msg}");
                failed += 1;
                failed_names.push(*name);
            }
        }
    }

    println!(
        "\ntest result: {}. {passed} passed; {failed} failed\n",
        if failed == 0 { "ok" } else { "FAILED" }
    );

    if !failed_names.is_empty() {
        println!("failures:");
        for name in &failed_names {
            println!("    {name}");
        }
        println!();
        std::process::exit(1);
    }
}
