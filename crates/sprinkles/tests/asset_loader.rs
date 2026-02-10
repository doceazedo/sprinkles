use bevy::asset::{AssetLoader, AssetPlugin, AssetServer, Assets, LoadState};
use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use std::path::Path;
use thiserror::Error;

use sprinkles::asset::{ParticleSystemAsset, ParticleSystemAssetLoader};

#[derive(Asset, TypePath, Debug, Serialize, Deserialize, PartialEq)]
struct DummyData {
    id: u32,
    label: String,
    values: Vec<f32>,
}

#[derive(Default, TypePath)]
struct DummyDataAssetLoader;

#[non_exhaustive]
#[derive(Debug, Error)]
enum DummyDataAssetLoaderError {
    #[error("Could not load asset: {0}")]
    Io(#[from] std::io::Error),
    #[error("Could not parse RON: {0}")]
    Ron(#[from] ron::error::SpannedError),
}

impl AssetLoader for DummyDataAssetLoader {
    type Asset = DummyData;
    type Settings = ();
    type Error = DummyDataAssetLoaderError;

    async fn load(
        &self,
        reader: &mut dyn bevy::asset::io::Reader,
        _settings: &(),
        _load_context: &mut bevy::asset::LoadContext<'_>,
    ) -> Result<Self::Asset, Self::Error> {
        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes).await?;
        let asset = ron::de::from_bytes::<DummyData>(&bytes)?;
        Ok(asset)
    }

    fn extensions(&self) -> &[&str] {
        &["ron"]
    }
}

fn fixtures_path() -> String {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .to_string_lossy()
        .to_string()
}

fn create_test_app() -> App {
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

    app.init_asset::<DummyData>()
        .init_asset_loader::<DummyDataAssetLoader>();

    app
}

fn run_until_loaded<T: Asset>(app: &mut App, handle: &Handle<T>, max_updates: u32) -> bool {
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

fn run_until_failed<T: Asset>(app: &mut App, handle: &Handle<T>, max_updates: u32) -> bool {
    for _ in 0..max_updates {
        app.update();

        let asset_server = app.world().resource::<AssetServer>();
        match asset_server.load_state(handle) {
            LoadState::Failed(_) => return true,
            LoadState::Loaded => return false,
            _ => continue,
        }
    }
    false
}

#[test]
fn test_bevy_loads_valid_ron_particle_system() {
    let mut app = create_test_app();

    let handle: Handle<ParticleSystemAsset> = {
        let asset_server = app.world().resource::<AssetServer>();
        asset_server.load("valid_particle_system.ron")
    };

    assert!(
        run_until_loaded(&mut app, &handle, 100),
        "Should load valid particle system RON"
    );

    let assets = app.world().resource::<Assets<ParticleSystemAsset>>();
    let asset = assets.get(&handle).expect("Asset should be available");

    assert_eq!(asset.name, "Test Particle System");
    assert_eq!(asset.emitters.len(), 1);
    assert_eq!(asset.emitters[0].name, "Test Emitter");
    assert_eq!(asset.emitters[0].amount, 16);
}

#[test]
fn test_bevy_loads_valid_whatever_extension_particle_system() {
    let mut app = create_test_app();

    let handle: Handle<ParticleSystemAsset> = {
        let asset_server = app.world().resource::<AssetServer>();
        asset_server.load("valid_particle_system.whatever")
    };

    assert!(
        run_until_loaded(&mut app, &handle, 100),
        "Should load particle system with .whatever extension"
    );

    let assets = app.world().resource::<Assets<ParticleSystemAsset>>();
    let asset = assets.get(&handle).expect("Asset should be available");

    assert_eq!(asset.name, "Test Particle System (Whatever Extension)");
    assert_eq!(asset.emitters[0].amount, 8);
}

#[test]
fn test_bevy_fails_to_load_invalid_ron_as_particle_system() {
    let mut app = create_test_app();

    let handle: Handle<ParticleSystemAsset> = {
        let asset_server = app.world().resource::<AssetServer>();
        asset_server.load("invalid_particle_system.ron")
    };

    assert!(
        run_until_failed(&mut app, &handle, 100),
        "Should fail to load invalid RON as particle system"
    );
}

#[test]
fn test_bevy_loads_dummy_data_ron() {
    let mut app = create_test_app();

    let handle: Handle<DummyData> = {
        let asset_server = app.world().resource::<AssetServer>();
        asset_server.load("dummy_data.ron")
    };

    assert!(
        run_until_loaded(&mut app, &handle, 100),
        "Should load dummy data RON"
    );

    let assets = app.world().resource::<Assets<DummyData>>();
    let asset = assets.get(&handle).expect("Asset should be available");

    assert_eq!(asset.id, 123);
    assert_eq!(asset.label, "Test Dummy Data");
    assert_eq!(asset.values, vec![1.0, 2.0, 3.0]);
}

#[test]
fn test_bevy_coexisting_ron_loaders_load_correct_types() {
    let mut app = create_test_app();

    let particle_handle: Handle<ParticleSystemAsset> = {
        let asset_server = app.world().resource::<AssetServer>();
        asset_server.load("valid_particle_system.ron")
    };

    let dummy_handle: Handle<DummyData> = {
        let asset_server = app.world().resource::<AssetServer>();
        asset_server.load("dummy_data.ron")
    };

    assert!(
        run_until_loaded(&mut app, &particle_handle, 100),
        "ParticleSystem should load from particle_system.ron"
    );

    assert!(
        run_until_loaded(&mut app, &dummy_handle, 100),
        "DummyData should load from dummy_data.ron"
    );

    let particle_assets = app.world().resource::<Assets<ParticleSystemAsset>>();
    let particle = particle_assets
        .get(&particle_handle)
        .expect("Particle system should be available");
    assert_eq!(particle.name, "Test Particle System");

    let dummy_assets = app.world().resource::<Assets<DummyData>>();
    let dummy = dummy_assets
        .get(&dummy_handle)
        .expect("Dummy data should be available");
    assert_eq!(dummy.id, 123);
}

#[test]
fn test_bevy_wrong_loader_for_wrong_data_fails() {
    let mut app = create_test_app();

    let handle: Handle<DummyData> = {
        let asset_server = app.world().resource::<AssetServer>();
        asset_server.load("valid_particle_system.ron")
    };

    assert!(
        run_until_failed(&mut app, &handle, 100),
        "Loading particle system as DummyData should fail"
    );
}

#[test]
fn test_particle_system_loader_extension() {
    let loader = ParticleSystemAssetLoader;
    let extensions = loader.extensions();
    assert_eq!(extensions, &["ron"]);
}

#[test]
fn test_dummy_data_loader_extension() {
    let loader = DummyDataAssetLoader;
    let extensions = loader.extensions();
    assert_eq!(extensions, &["ron"]);
}
