#!/usr/bin/env bash
set -euo pipefail

repository_root="${ITGLA_PACKAGE_ROOT:-$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)}"
cd "${repository_root}"

for required_command in cargo cargo-packager python3 shasum; do
    if ! command -v "${required_command}" >/dev/null 2>&1; then
        echo "missing required command: ${required_command}" >&2
        exit 1
    fi
done

version="$(cargo metadata --locked --no-deps --format-version 1 | python3 -c 'import json, sys; print(json.load(sys.stdin)["packages"][0]["version"])')"
case "$(uname -m)" in
    arm64) package_arch=arm64 ;;
    x86_64) package_arch=x86_64 ;;
    *) echo "Unsupported macOS architecture" >&2; exit 1 ;;
esac
package_stem="itgla-v${version}-macos-${package_arch}"
output_directory="${repository_root}/dist"
mkdir -p "${output_directory}"

cargo build --release --locked
lipo -verify_arch "${package_arch}" target/release/itgla
cargo packager --release --formats dmg --out-dir "${output_directory}"
dmg="$(find "${output_directory}" -maxdepth 1 -type f -name '*.dmg' -print -quit)"
[[ -n "${dmg}" ]] || { echo "cargo-packager did not produce a DMG" >&2; exit 1; }
mv "${dmg}" "${output_directory}/${package_stem}.dmg"
(
    cd "${output_directory}"
    shasum -a 256 "${package_stem}.dmg" > "${package_stem}.dmg.sha256"
)

hdiutil verify "${output_directory}/${package_stem}.dmg"
echo "Created ${output_directory}/${package_stem}.dmg"
