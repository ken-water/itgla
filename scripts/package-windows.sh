#!/usr/bin/env bash
set -euo pipefail

repository_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
target="x86_64-pc-windows-gnu"

for required_command in cargo python3 zip file sha256sum x86_64-w64-mingw32-strip x86_64-w64-mingw32-objdump; do
    if ! command -v "${required_command}" >/dev/null 2>&1; then
        echo "missing required command: ${required_command}" >&2
        exit 1
    fi
done

cd "${repository_root}"
version="$(cargo metadata --locked --no-deps --format-version 1 | python3 -c 'import json, sys; data = json.load(sys.stdin); print(data["packages"][0]["version"])')"
package_name="itgla-v${version}-windows-x86_64"
executable="target/${target}/release/itgla.exe"
output_directory="dist"
archive="${output_directory}/${package_name}.zip"
checksum="${archive}.sha256"
temporary_root="$(mktemp -d "${TMPDIR:-/tmp}/itgla-windows-package.XXXXXX")"

cleanup() {
    rm -rf -- "${temporary_root}"
}
trap cleanup EXIT

cargo build --release --locked --target "${target}"

if [[ ! -f "${executable}" ]]; then
    echo "Windows executable was not produced: ${executable}" >&2
    exit 1
fi

mkdir -p "${temporary_root}/${package_name}" "${output_directory}"
cp "${executable}" "${temporary_root}/${package_name}/itgla.exe"
cp README.md CHANGELOG.md "${temporary_root}/${package_name}/"
cp docs/windows-package.md "${temporary_root}/${package_name}/WINDOWS-README.md"
x86_64-w64-mingw32-strip "${temporary_root}/${package_name}/itgla.exe"

file "${temporary_root}/${package_name}/itgla.exe" | grep 'PE32+ executable.*x86-64' >/dev/null
x86_64-w64-mingw32-objdump -p "${temporary_root}/${package_name}/itgla.exe" | grep 'Subsystem.*Windows GUI' >/dev/null

(
    cd "${temporary_root}"
    zip -q -r "${package_name}.zip" "${package_name}"
)
mv "${temporary_root}/${package_name}.zip" "${archive}"
(
    cd "${output_directory}"
    sha256sum "${package_name}.zip" > "${package_name}.zip.sha256"
)

echo "Created ${archive}"
echo "Created ${checksum}"
