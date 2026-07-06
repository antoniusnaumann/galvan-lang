use std::path::PathBuf;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum RustdocError {
    #[error("failed to run cargo metadata: {0}")]
    CargoMetadata(std::io::Error),
    #[error("cargo metadata returned invalid JSON: {0}")]
    InvalidCargoMetadata(serde_json::Error),
    #[error("failed to read rustdoc JSON cache {0}: {1}")]
    ReadCache(PathBuf, std::io::Error),
    #[error("failed to parse rustdoc JSON cache {0}: {1}")]
    ParseCache(PathBuf, serde_json::Error),
    #[error("rustdoc JSON cache {0} is missing a format_version field; delete it and let galvan regenerate it with the pinned nightly toolchain")]
    MissingFormatVersion(PathBuf),
    #[error("rustdoc JSON cache {0} has format_version {1}, but this build of galvan only understands format_version {2}; delete the cache so galvan regenerates it with the pinned nightly toolchain (override via GALVAN_RUSTDOC_TOOLCHAIN), or upgrade galvan")]
    UnsupportedFormatVersion(PathBuf, u64, u64),
}
