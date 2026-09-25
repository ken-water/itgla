#!/usr/bin/env bash
set -euo pipefail

repository_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "${repository_root}"

for required_command in cargo cargo-packager python3 sha256sum; do
    if ! command -v "${required_command}" >/dev/null 2>&1; then
        echo "missing required command: ${required_command}" >&2
        exit 1
    fi
done

version="$(cargo metadata --locked --no-deps --format-version 1 | python3 -c 'import json, sys; print(json.load(sys.stdin)["packages"][0]["version"])')"
package_stem="itgla-v${version}-macos-arm64"
output_directory="${repository_root}/dist"
mkdir -p "${output_directory}"

cargo build --release --locked
cargo packager --release --formats dmg --out-dir "${output_directory}"
dmg="$(find "${output_directory}" -maxdepth 1 -type f -name '*.dmg' -print -quit)"
[[ -n "${dmg}" ]] || { echo "cargo-packager did not produce a DMG" >&2; exit 1; }
mv "${dmg}" "${output_directory}/${package_stem}.dmg"
sha256sum "${output_directory}/${package_stem}.dmg" > "${output_directory}/${package_stem}.dmg.sha256"

echo "Created ${output_directory}/${package_stem}.dmg"
