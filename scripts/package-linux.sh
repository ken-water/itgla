#!/usr/bin/env bash
set -euo pipefail

repository_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

for required_command in cargo python3 tar sha256sum; do
    if ! command -v "${required_command}" >/dev/null 2>&1; then
        echo "missing required command: ${required_command}" >&2
        exit 1
    fi
done

cd "${repository_root}"
version="$(cargo metadata --locked --no-deps --format-version 1 | python3 -c 'import json, sys; data = json.load(sys.stdin); print(data["packages"][0]["version"])')"
package_name="itgla-v${version}-linux-x86_64"
executable="target/release/itgla"
output_directory="dist"
archive="${output_directory}/${package_name}.tar.gz"
temporary_root="$(mktemp -d "${TMPDIR:-/tmp}/itgla-linux-package.XXXXXX")"

cleanup() {
    rm -rf -- "${temporary_root}"
}
trap cleanup EXIT

cargo build --release --locked

mkdir -p "${temporary_root}/${package_name}" "${output_directory}"
cp "${executable}" "${temporary_root}/${package_name}/itgla"
cp README.md CHANGELOG.md "${temporary_root}/${package_name}/"
tar -C "${temporary_root}" -czf "${archive}" "${package_name}"
(
    cd "${output_directory}"
    sha256sum "${package_name}.tar.gz" > "${package_name}.tar.gz.sha256"
)

echo "Created ${archive}"
echo "Created ${archive}.sha256"
