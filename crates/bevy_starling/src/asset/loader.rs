use bevy::{
    asset::{io::Reader, AssetLoader, LoadContext},
    prelude::*,
};
use thiserror::Error;

use super::format::ParticleSystemAsset;

#[derive(Default, TypePath)]
pub struct ParticleSystemAssetLoader;

#[non_exhaustive]
#[derive(Debug, Error)]
pub enum ParticleSystemAssetLoaderError {
    #[error("Could not load asset: {0}")]
    Io(#[from] std::io::Error),
    #[error("Could not parse RON: {0}")]
    Ron(#[from] ron::error::SpannedError),
}

impl AssetLoader for ParticleSystemAssetLoader {
    type Asset = ParticleSystemAsset;
    type Settings = ();
    type Error = ParticleSystemAssetLoaderError;

    async fn load(
        &self,
        reader: &mut dyn Reader,
        _settings: &(),
        _load_context: &mut LoadContext<'_>,
    ) -> Result<Self::Asset, Self::Error> {
        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes).await?;
        let asset = ron::de::from_bytes::<ParticleSystemAsset>(&bytes)?;
        Ok(asset)
    }

    fn extensions(&self) -> &[&str] {
        &["starling"]
    }
}
