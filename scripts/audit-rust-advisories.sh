#!/usr/bin/env bash
set -euo pipefail

repository_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
user_home_directory="$(getent passwd "$(id -u)" | cut -d: -f6)"
cargo_deny="${user_home_directory}/.cargo/bin/cargo-deny"

[[ -x "${cargo_deny}" ]] || {
    echo "SKIP: cargo-deny is not installed" >&2
    exit 2
}

cd "${repository_root}"
"${cargo_deny}" --offline check advisories
