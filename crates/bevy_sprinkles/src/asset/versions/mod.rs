mod v0_1;

use serde::Deserialize;
use thiserror::Error;

use super::ParticleSystemAsset;

/// The current asset format version.
const CURRENT_FORMAT_VERSION: &str = "0.2";

/// Returns the current asset format version string.
pub fn current_format_version() -> &'static str {
    CURRENT_FORMAT_VERSION
}

#[derive(Deserialize)]
struct VersionProbe {
    sprinkles_version: String,
}

/// Errors that can occur during asset version migration.
#[derive(Debug, Error)]
pub enum MigrationError {
    /// The asset file contained invalid RON syntax.
    #[error("Could not parse RON: {0}")]
    Ron(#[from] ron::error::SpannedError),
    /// The asset file has an unrecognized format version.
    #[error("Unknown sprinkles_version \"{0}\". You may need a newer version of Sprinkles.")]
    UnknownVersion(String),
}

/// The result of migrating a particle system asset from an older format version.
pub struct MigrationResult {
    /// The migrated asset, always in the current format version.
    pub asset: ParticleSystemAsset,
    /// Whether the asset was migrated from an older version.
    pub was_migrated: bool,
}

/// Migrates a RON-encoded particle system asset from any known format version
/// to the current version.
pub fn migrate(bytes: &[u8]) -> Result<MigrationResult, MigrationError> {
    let probe: VersionProbe = ron::de::from_bytes(bytes)?;
    let current = current_format_version();

    match probe.sprinkles_version.as_str() {
        v if v == current => {
            let asset: ParticleSystemAsset = ron::de::from_bytes(bytes)?;
            Ok(MigrationResult {
                asset,
                was_migrated: false,
            })
        }
        "0.1" => {
            let old: v0_1::ParticleSystemAsset = ron::de::from_bytes(bytes)?;
            let asset: ParticleSystemAsset = old.into();
            Ok(MigrationResult {
                asset,
                was_migrated: true,
            })
        }
        unknown => Err(MigrationError::UnknownVersion(unknown.to_string())),
    }
}

/// Migrates a RON string. Convenience wrapper around [`migrate`] for callers
/// that have the asset as a `&str` rather than `&[u8]`.
pub fn migrate_str(ron: &str) -> Result<MigrationResult, MigrationError> {
    migrate(ron.as_bytes())
}
