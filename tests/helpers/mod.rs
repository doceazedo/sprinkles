#![allow(dead_code)]

use bevy::asset::{AssetPlugin, AssetServer, LoadState};
use bevy::prelude::*;
use bevy::render::storage::ShaderStorageBuffer;
use sprinkles::asset::{
    ColliderData, EmitterData, ParticleSystemAsset, ParticleSystemAssetLoader,
    ParticleSystemDimension,
};
use sprinkles::runtime::{ParticleMaterial, ParticleSystem3D};
use std::path::Path;

pub fn fixtures_path() -> String {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .to_string_lossy()
        .to_string()
}

pub fn screenshots_tmp_path() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("screenshots")
        .join("tmp")
}

pub fn screenshots_baseline_path() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("screenshots")
}

pub fn create_minimal_app() -> App {
    let mut app = App::new();

    app.add_plugins(
        MinimalPlugins.set(bevy::app::ScheduleRunnerPlugin::run_loop(
            std::time::Duration::from_millis(10),
        )),
    );

    app.add_plugins(AssetPlugin {
        file_path: fixtures_path(),
        ..default()
    });

    app.init_asset::<ParticleSystemAsset>()
        .init_asset_loader::<ParticleSystemAssetLoader>();

    app.init_asset::<Mesh>();
    app.init_asset::<StandardMaterial>();
    app.init_asset::<ShaderStorageBuffer>();
    app.init_asset::<ParticleMaterial>();

    app.add_systems(
        Update,
        (
            sprinkles::spawning::setup_particle_systems,
            sprinkles::spawning::update_particle_time,
            sprinkles::spawning::sync_particle_mesh,
            sprinkles::spawning::sync_particle_material,
            sprinkles::spawning::sync_emitter_mesh_transforms,
            sprinkles::spawning::sync_collider_data,
            sprinkles::spawning::cleanup_particle_entities,
        ),
    );

    app
}

pub fn load_fixture(app: &mut App, filename: &str) -> Handle<ParticleSystemAsset> {
    let asset_server = app.world().resource::<AssetServer>();
    asset_server.load(filename.to_string())
}

pub fn run_until_loaded<T: Asset>(app: &mut App, handle: &Handle<T>, max_updates: u32) -> bool {
    for _ in 0..max_updates {
        app.update();

        let asset_server = app.world().resource::<AssetServer>();
        match asset_server.load_state(handle) {
            LoadState::Loaded => return true,
            LoadState::Failed(_) => return false,
            _ => continue,
        }
    }
    false
}

pub fn spawn_3d_particle_system(app: &mut App, handle: Handle<ParticleSystemAsset>) -> Entity {
    app.world_mut().spawn(ParticleSystem3D { handle }).id()
}

pub fn setup_loaded_system(fixture: &str) -> (App, Handle<ParticleSystemAsset>, Entity) {
    let mut app = create_minimal_app();
    let handle = load_fixture(&mut app, fixture);
    let entity = spawn_3d_particle_system(&mut app, handle.clone());
    assert!(
        run_until_loaded(&mut app, &handle, 100),
        "fixture should load"
    );
    advance_frames(&mut app, 5);
    (app, handle, entity)
}

pub fn load_asset(app: &mut App, fixture: &str) -> ParticleSystemAsset {
    let handle = load_fixture(app, fixture);
    for _ in 0..100 {
        app.update();
        let asset_server = app.world().resource::<AssetServer>();
        match asset_server.load_state(&handle) {
            LoadState::Loaded => {
                let assets = app
                    .world()
                    .resource::<bevy::asset::Assets<ParticleSystemAsset>>();
                return assets.get(&handle).expect("asset should exist").clone();
            }
            LoadState::Failed(err) => {
                panic!("fixture failed to load '{fixture}': {err:?}");
            }
            _ => continue,
        }
    }
    panic!("fixture timed out loading: {fixture}");
}

pub fn advance_frames(app: &mut App, n: u32) {
    for _ in 0..n {
        app.update();
    }
}

/// advances the app for approximately the given number of seconds of real time.
/// useful for tests that depend on system_time exceeding a threshold.
pub fn advance_time(app: &mut App, seconds: f32) {
    let frame_count = (seconds / 0.016).ceil() as u32 + 2;
    let sleep_per_frame = std::time::Duration::from_secs_f64(seconds as f64 / frame_count as f64);
    for _ in 0..frame_count {
        std::thread::sleep(sleep_per_frame);
        app.update();
    }
}

pub struct ImageDiff {
    pub total_pixels: usize,
    pub different_pixels: usize,
    pub max_channel_diff: u8,
    pub avg_diff: f64,
}

impl ImageDiff {
    pub fn within_tolerance(&self, max_different_ratio: f64, max_avg_diff: f64) -> bool {
        let ratio = self.different_pixels as f64 / self.total_pixels as f64;
        ratio <= max_different_ratio && self.avg_diff <= max_avg_diff
    }
}

pub fn compare_images(actual: &[u8], expected: &[u8], per_channel_tolerance: u8) -> ImageDiff {
    assert_eq!(
        actual.len(),
        expected.len(),
        "images must have the same size"
    );

    let total_pixels = actual.len() / 4;
    let mut different_pixels = 0usize;
    let mut max_channel_diff: u8 = 0;
    let mut total_diff: u64 = 0;

    for (a, e) in actual.chunks_exact(4).zip(expected.chunks_exact(4)) {
        let mut pixel_differs = false;
        for i in 0..4 {
            let diff = (a[i] as i16 - e[i] as i16).unsigned_abs() as u8;
            if diff > per_channel_tolerance {
                pixel_differs = true;
            }
            max_channel_diff = max_channel_diff.max(diff);
            total_diff += diff as u64;
        }
        if pixel_differs {
            different_pixels += 1;
        }
    }

    let avg_diff = if actual.is_empty() {
        0.0
    } else {
        total_diff as f64 / actual.len() as f64
    };

    ImageDiff {
        total_pixels,
        different_pixels,
        max_channel_diff,
        avg_diff,
    }
}

pub fn create_test_asset(emitter_names: &[&str]) -> ParticleSystemAsset {
    ParticleSystemAsset {
        name: "Test".to_string(),
        dimension: ParticleSystemDimension::D3,
        emitters: emitter_names
            .iter()
            .map(|name| EmitterData {
                name: name.to_string(),
                ..Default::default()
            })
            .collect(),
        colliders: vec![],
    }
}

pub fn create_test_asset_with_colliders(collider_names: &[&str]) -> ParticleSystemAsset {
    ParticleSystemAsset {
        name: "Test".to_string(),
        dimension: ParticleSystemDimension::D3,
        emitters: vec![EmitterData {
            name: "Emitter".to_string(),
            ..Default::default()
        }],
        colliders: collider_names
            .iter()
            .map(|name| ColliderData {
                name: name.to_string(),
                ..Default::default()
            })
            .collect(),
    }
}

pub fn next_unique_name(base_name: &str, existing: &[&str]) -> String {
    if !existing.contains(&base_name) {
        return base_name.to_string();
    }
    let mut n = 2;
    loop {
        let candidate = format!("{} {}", base_name, n);
        if !existing.iter().any(|name| *name == candidate) {
            return candidate;
        }
        n += 1;
    }
}

pub fn compare_screenshot_rgba(
    actual_rgba: &[u8],
    expected_rgba: &[u8],
    per_channel_tolerance: u8,
    max_different_ratio: f64,
    max_avg_diff: f64,
) -> Result<(), String> {
    let diff = compare_images(actual_rgba, expected_rgba, per_channel_tolerance);
    if diff.within_tolerance(max_different_ratio, max_avg_diff) {
        Ok(())
    } else {
        let ratio = diff.different_pixels as f64 / diff.total_pixels as f64;
        Err(format!(
            "screenshot mismatch: {:.2}% pixels differ (max {:.2}%), avg diff {:.2} (max {:.2}), max channel diff {}",
            ratio * 100.0,
            max_different_ratio * 100.0,
            diff.avg_diff,
            max_avg_diff,
            diff.max_channel_diff,
        ))
    }
}
