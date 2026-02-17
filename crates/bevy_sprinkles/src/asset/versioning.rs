struct FormatVersion {
    version: &'static str,
    breaking: bool,
}

// all known asset format versions. the last must always be the current version.
const FORMAT_VERSIONS: &[FormatVersion] = &[
    FormatVersion {
        version: "0.0",
        breaking: false,
    }, // initial
    FormatVersion {
        version: "0.1",
        breaking: false,
    },
];

/// The result of validating an asset's `sprinkles_version` against the current format version.
pub enum VersionStatus {
    /// The asset version matches the current format version.
    Current,
    /// The asset version is older but can be auto-upgraded.
    Outdated {
        /// The version found in the asset.
        found: String,
        /// The current format version.
        current: &'static str,
    },
    /// The asset version is older and has breaking changes that prevent auto-upgrade.
    Incompatible {
        /// The version found in the asset.
        found: String,
        /// The current format version.
        current: &'static str,
    },
    /// The asset version is not recognized (might be from a newer Sprinkles version).
    Unknown,
}

/// Returns the current asset format version string.
pub fn current_format_version() -> &'static str {
    FORMAT_VERSIONS
        .last()
        .expect("FORMAT_VERSIONS must not be empty")
        .version
}

fn find_version_index(version: &str) -> Option<usize> {
    FORMAT_VERSIONS.iter().position(|v| v.version == version)
}

/// Returns `true` if an asset can be automatically upgraded from one version to another
/// without any breaking changes in between.
pub fn can_auto_upgrade(from: &str, to: &str) -> bool {
    let Some(from_idx) = find_version_index(from) else {
        return false;
    };
    let Some(to_idx) = find_version_index(to) else {
        return false;
    };
    if from_idx >= to_idx {
        return false;
    }
    !FORMAT_VERSIONS[from_idx + 1..=to_idx]
        .iter()
        .any(|v| v.breaking)
}

/// Validates a version string against the current format version and returns
/// the appropriate [`VersionStatus`].
pub fn validate_version(version: &str) -> VersionStatus {
    let current = current_format_version();
    if version == current {
        VersionStatus::Current
    } else if find_version_index(version).is_none() {
        VersionStatus::Unknown
    } else if can_auto_upgrade(version, current) {
        VersionStatus::Outdated {
            found: version.to_string(),
            current,
        }
    } else {
        VersionStatus::Incompatible {
            found: version.to_string(),
            current,
        }
    }
}
