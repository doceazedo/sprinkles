pub mod asset;
pub mod core;

use bevy::prelude::*;

use asset::{ParticleSystemAsset, ParticleSystemAssetLoader};

pub struct StarlingPlugin;

impl Plugin for StarlingPlugin {
    fn build(&self, app: &mut App) {
        app.init_asset::<ParticleSystemAsset>()
            .init_asset_loader::<ParticleSystemAssetLoader>();
    }
}
