mod v0_1;

use serde::Deserialize;
use thiserror::Error;

use super::ParticlesAsset;

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

/// The result of a [`migrate`] call.
pub struct MigrationResult {
    /// The particle system asset in the current format version.
    pub asset: ParticlesAsset,
    /// Whether the asset was migrated from an older version.
    pub was_migrated: bool,
}

/// Migrates a RON-encoded particle system asset to the current format version.
pub fn migrate(bytes: &[u8]) -> Result<MigrationResult, MigrationError> {
    let probe: VersionProbe = ron::de::from_bytes(bytes)?;
    let current = current_format_version();

    match probe.sprinkles_version.as_str() {
        v if v == current => {
            let asset: ParticlesAsset = ron::de::from_bytes(bytes)?;
            Ok(MigrationResult {
                asset,
                was_migrated: false,
            })
        }
        "0.1" => {
            let old: v0_1::ParticlesAsset = ron::de::from_bytes(bytes)?;
            let asset: ParticlesAsset = old.into();
            Ok(MigrationResult {
                asset,
                was_migrated: true,
            })
        }
        unknown => Err(MigrationError::UnknownVersion(unknown.to_string())),
    }
}

/// Migrates a RON-encoded particle system asset from a string.
pub fn migrate_str(ron: &str) -> Result<MigrationResult, MigrationError> {
    migrate(ron.as_bytes())
}
