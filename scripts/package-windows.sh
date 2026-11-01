#!/usr/bin/env bash
set -euo pipefail

repository_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
target="x86_64-pc-windows-gnu"

for required_command in cargo python3 zip file sha256sum convert makensis x86_64-w64-mingw32-windres x86_64-w64-mingw32-strip x86_64-w64-mingw32-objdump; do
    if ! command -v "${required_command}" >/dev/null 2>&1; then
        echo "missing required command: ${required_command}" >&2
        exit 1
    fi
done

cd "${repository_root}"
version="$(cargo metadata --locked --no-deps --format-version 1 | python3 -c 'import json, sys; data = json.load(sys.stdin); print(data["packages"][0]["version"])')"
package_name="itgla-v${version}-windows-x86_64"
executable="target/${target}/release/itgla.exe"
output_directory="${ITGLA_PACKAGE_OUTPUT:-${repository_root}/dist}"
mkdir -p "${output_directory}"
output_directory="$(cd "${output_directory}" && pwd)"
archive="${output_directory}/${package_name}.zip"
checksum="${archive}.sha256"
installer="${output_directory}/${package_name}-setup.exe"
installer_checksum="${installer}.sha256"
temporary_root="$(mktemp -d "${TMPDIR:-/tmp}/itgla-windows-package.XXXXXX")"

cleanup() {
    rm -rf -- "${temporary_root}"
}
trap cleanup EXIT

convert assets/itgla.png -define icon:auto-resize=256,128,64,48,32,16 "${temporary_root}/itgla.ico"
cat > "${temporary_root}/itgla.rc" <<EOF
1 ICON "${temporary_root}/itgla.ico"
1 VERSIONINFO
FILEVERSION ${version//./,},0
PRODUCTVERSION ${version//./,},0
FILEFLAGSMASK 0x3fL
FILEFLAGS 0x0L
FILEOS 0x40004L
FILETYPE 0x1L
BEGIN
 BLOCK "StringFileInfo"
 BEGIN
  BLOCK "040904B0"
  BEGIN
   VALUE "CompanyName", "ITGLA\\0"
   VALUE "FileDescription", "ITGLA - IT in a glance\\0"
   VALUE "FileVersion", "${version}\\0"
   VALUE "ProductName", "ITGLA\\0"
   VALUE "ProductVersion", "${version}\\0"
   VALUE "OriginalFilename", "itgla.exe\\0"
   VALUE "LegalCopyright", "Copyright ITGLA\\0"
  END
 END
 BLOCK "VarFileInfo"
 BEGIN
  VALUE "Translation", 0x0409, 1200
 END
END
EOF
x86_64-w64-mingw32-windres "${temporary_root}/itgla.rc" -O coff -o "${temporary_root}/itgla.res"
RUSTFLAGS="-C link-arg=${temporary_root}/itgla.res" cargo build --release --locked --target "${target}"

if [[ ! -f "${executable}" ]]; then
    echo "Windows executable was not produced: ${executable}" >&2
    exit 1
fi

mkdir -p "${temporary_root}/${package_name}" "${output_directory}"
cp "${executable}" "${temporary_root}/${package_name}/itgla.exe"
cp "${temporary_root}/itgla.ico" "${temporary_root}/${package_name}/itgla.ico"
cp README.md CHANGELOG.md "${temporary_root}/${package_name}/"
cp docs/windows-package.md "${temporary_root}/${package_name}/WINDOWS-README.md"
x86_64-w64-mingw32-strip "${temporary_root}/${package_name}/itgla.exe"

# A configured signer must sign AND verify the supplied file, returning nonzero on failure.
# Pass an executable path, never shell code. Signing happens before ZIP/NSIS compression.
if [[ -n "${ITGLA_WINDOWS_SIGNER:-}" ]]; then
    "${ITGLA_WINDOWS_SIGNER}" "${temporary_root}/${package_name}/itgla.exe"
fi
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

python3 scripts/create-installer-art.py --output "${temporary_root}/art"
version_quad="${version}.0"
makensis -DVERSION="${version}" -DVERSION_QUAD="${version_quad}" \
    -DPAYLOAD="${temporary_root}/${package_name}" -DART="${temporary_root}/art" \
    -DOUTPUT="${installer}" packaging/windows/installer.nsi

if [[ -n "${ITGLA_WINDOWS_SIGNER:-}" ]]; then
    "${ITGLA_WINDOWS_SIGNER}" "${installer}"
fi
(
    cd "${output_directory}"
    sha256sum "${package_name}-setup.exe" > "${package_name}-setup.exe.sha256"
)

echo "Created ${archive}"
echo "Created ${checksum}"
echo "Created ${installer}"
echo "Created ${installer_checksum}"
