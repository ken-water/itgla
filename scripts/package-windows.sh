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
output_directory="dist"
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

cat > "${temporary_root}/itgla.nsi" <<EOF
Unicode true
!include MUI2.nsh

Name "ITGLA"
OutFile "${repository_root}/${installer}"
InstallDir "\$LOCALAPPDATA\\ITGLA"
RequestExecutionLevel user
ShowInstDetails show
ShowUnInstDetails show

!define MUI_ABORTWARNING
!define MUI_FINISHPAGE_RUN "\$INSTDIR\\itgla.exe"
!define MUI_FINISHPAGE_RUN_TEXT "Launch ITGLA"

!insertmacro MUI_PAGE_WELCOME
!insertmacro MUI_PAGE_DIRECTORY
!insertmacro MUI_PAGE_INSTFILES
!insertmacro MUI_PAGE_FINISH
!insertmacro MUI_UNPAGE_CONFIRM
!insertmacro MUI_UNPAGE_INSTFILES
!insertmacro MUI_LANGUAGE "English"

Section "ITGLA" SecMain
  SetOutPath "\$INSTDIR"
  File "${temporary_root}/${package_name}/itgla.exe"
  File "${temporary_root}/${package_name}/itgla.ico"
  File "${repository_root}/README.md"
  File "${repository_root}/CHANGELOG.md"
  WriteUninstaller "\$INSTDIR\\Uninstall.exe"

  CreateDirectory "\$SMPROGRAMS\\ITGLA"
  CreateShortcut "\$SMPROGRAMS\\ITGLA\\ITGLA.lnk" "\$INSTDIR\\itgla.exe" "" "\$INSTDIR\\itgla.ico" 0
  CreateShortcut "\$SMPROGRAMS\\ITGLA\\Uninstall ITGLA.lnk" "\$INSTDIR\\Uninstall.exe"
  CreateShortcut "\$DESKTOP\\ITGLA.lnk" "\$INSTDIR\\itgla.exe" "" "\$INSTDIR\\itgla.ico" 0

  WriteRegStr HKCU "Software\\Microsoft\\Windows\\CurrentVersion\\Uninstall\\ITGLA" "DisplayName" "ITGLA"
  WriteRegStr HKCU "Software\\Microsoft\\Windows\\CurrentVersion\\Uninstall\\ITGLA" "DisplayVersion" "${version}"
  WriteRegStr HKCU "Software\\Microsoft\\Windows\\CurrentVersion\\Uninstall\\ITGLA" "Publisher" "ITGLA"
  WriteRegStr HKCU "Software\\Microsoft\\Windows\\CurrentVersion\\Uninstall\\ITGLA" "InstallLocation" "\$INSTDIR"
  WriteRegStr HKCU "Software\\Microsoft\\Windows\\CurrentVersion\\Uninstall\\ITGLA" "DisplayIcon" "\$INSTDIR\\itgla.exe"
  WriteRegStr HKCU "Software\\Microsoft\\Windows\\CurrentVersion\\Uninstall\\ITGLA" "UninstallString" "\$INSTDIR\\Uninstall.exe"
  WriteRegDWORD HKCU "Software\\Microsoft\\Windows\\CurrentVersion\\Uninstall\\ITGLA" "NoModify" 1
  WriteRegDWORD HKCU "Software\\Microsoft\\Windows\\CurrentVersion\\Uninstall\\ITGLA" "NoRepair" 1
SectionEnd

Section "Uninstall"
  Delete "\$DESKTOP\\ITGLA.lnk"
  Delete "\$SMPROGRAMS\\ITGLA\\ITGLA.lnk"
  Delete "\$SMPROGRAMS\\ITGLA\\Uninstall ITGLA.lnk"
  RMDir "\$SMPROGRAMS\\ITGLA"
  Delete "\$INSTDIR\\itgla.exe"
  Delete "\$INSTDIR\\itgla.ico"
  Delete "\$INSTDIR\\README.md"
  Delete "\$INSTDIR\\CHANGELOG.md"
  Delete "\$INSTDIR\\Uninstall.exe"
  RMDir "\$INSTDIR"
  DeleteRegKey HKCU "Software\\Microsoft\\Windows\\CurrentVersion\\Uninstall\\ITGLA"
SectionEnd
EOF
makensis "${temporary_root}/itgla.nsi" >/dev/null
(
    cd "${output_directory}"
    sha256sum "${package_name}-setup.exe" > "${package_name}-setup.exe.sha256"
)

echo "Created ${archive}"
echo "Created ${checksum}"
echo "Created ${installer}"
echo "Created ${installer_checksum}"
