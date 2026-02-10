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
