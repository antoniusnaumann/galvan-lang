use std::io;
use std::path::PathBuf;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum RustdocError {
    #[error("failed to run cargo metadata: {0}")]
    CargoMetadata(io::Error),
    #[error("cargo metadata failed: {0}")]
    CargoMetadataFailed(String),
    #[error("cargo metadata returned invalid JSON: {0}")]
    InvalidCargoMetadata(serde_json::Error),
    #[error("crate `{0}` was not found in cargo metadata")]
    DependencyNotFound(Box<str>),
    #[error("rustup was not found on PATH; install rustup, or set GALVAN_RUSTDOC_COMMAND to a nightly cargo executable that can emit rustdoc JSON")]
    ToolchainUnavailable,
    #[error("the rustdoc toolchain `{0}` is not installed; run `rustup toolchain install {0}`, set GALVAN_RUSTDOC_TOOLCHAIN to an installed compatible nightly, or set GALVAN_RUSTDOC_COMMAND to a nightly cargo executable")]
    ToolchainNotInstalled(String),
    #[error("failed to launch rustdoc JSON generation: {0}")]
    RustdocSpawn(io::Error),
    #[error("rustdoc JSON generation failed for crate `{crate_name}`: {stderr}")]
    RustdocGeneration {
        crate_name: Box<str>,
        stderr: String,
    },
    #[error("rustdoc succeeded but generated JSON {0} was not found")]
    GeneratedJsonMissing(PathBuf),
    #[error("failed to read rustdoc JSON cache {0}: {1}")]
    ReadCache(PathBuf, io::Error),
    #[error("failed to write rustdoc JSON cache {0}: {1}")]
    WriteCache(PathBuf, io::Error),
    #[error("failed to parse rustdoc JSON cache {0}: {1}")]
    ParseCache(PathBuf, serde_json::Error),
    #[error("rustdoc JSON cache {0} is missing a format_version field; delete it and let galvan regenerate it with the pinned nightly toolchain (override the rustup toolchain with GALVAN_RUSTDOC_TOOLCHAIN, or bypass rustup with GALVAN_RUSTDOC_COMMAND)")]
    MissingFormatVersion(PathBuf),
    #[error("rustdoc JSON cache {0} has format_version {1}, but this build of galvan only understands format_version {2}; delete the cache so galvan regenerates it with the pinned nightly toolchain (override the rustup toolchain with GALVAN_RUSTDOC_TOOLCHAIN, or bypass rustup with GALVAN_RUSTDOC_COMMAND), or upgrade galvan")]
    UnsupportedFormatVersion(PathBuf, u64, u64),
    #[error("rustdoc JSON for crate `{0}` did not lift any usable items while GALVAN_RUSTDOC_REQUIRE_LIFT is set")]
    RequiredLiftEmpty(Box<str>),
}
