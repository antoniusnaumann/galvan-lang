#!/usr/bin/env bash
set -euo pipefail

# Keep this in lockstep with RUSTDOC_FORMAT_VERSION in src/interop/state.rs and
# DEFAULT_RUSTDOC_TOOLCHAIN in src/cache/rustdoc.rs.
TOOLCHAIN="nightly-2026-07-02"

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
FIXTURE_DIR="${SCRIPT_DIR}/interop_fixture"
GOLDEN="${SCRIPT_DIR}/interop_fixture.golden.json"

(
  cd "${FIXTURE_DIR}"
  cargo "+${TOOLCHAIN}" rustdoc --lib -- -Z unstable-options --output-format json
)

cp "${FIXTURE_DIR}/target/doc/interop_fixture.json" "${GOLDEN}"
