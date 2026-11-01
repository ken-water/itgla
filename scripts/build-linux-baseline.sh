#!/usr/bin/env bash
set -euo pipefail
repository_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
# Docker runs only the dedicated build container. No production containers are changed.
image="${ITGLA_LINUX_BUILD_IMAGE:-ubuntu:22.04}"
target_directory="${ITGLA_BASELINE_TARGET:-${repository_root}/target/linux-baseline}"
toolchain="$(rustc --print sysroot)"
cargo_directory="${CARGO_HOME:-$(getent passwd "$(id -u)" | cut -d: -f6)/.cargo}"
mkdir -p "${target_directory}"
docker run --rm \
    -v "${repository_root}:/work:ro" \
    -v "${target_directory}:/target" \
    -v "${toolchain}:/opt/rust:ro" \
    -v "${cargo_directory}/registry:/usr/local/cargo/registry:ro" \
    -e CARGO_TARGET_DIR=/target -e CARGO_HOME=/usr/local/cargo \
    -e http_proxy -e https_proxy -e HTTP_PROXY -e HTTPS_PROXY \
    -w /work "${image}" bash -euc '
        export DEBIAN_FRONTEND=noninteractive
        apt-get update
        apt-get install -y --no-install-recommends build-essential pkg-config libfontconfig1-dev libxkbcommon-dev libx11-dev libxcb1-dev libxrandr-dev libxi-dev libxcursor-dev libxinerama-dev ca-certificates python3 binutils
        export PATH=/opt/rust/bin:$PATH
        cargo build --offline --release --locked
        python3 scripts/check-linux-abi.py /target/release/itgla
    '
# Restore host ownership after the root-owned container build.
if [[ -n "${SUDO_UID:-}" ]]; then
    chown -R "${SUDO_UID}:${SUDO_GID}" "${target_directory}"
fi
printf 'Baseline binary: %s/release/itgla\n' "${target_directory}"
