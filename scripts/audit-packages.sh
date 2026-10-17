#!/usr/bin/env bash
set -euo pipefail

repository_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
artifact_directory="${1:-${repository_root}/dist}"
temporary_directory="$(mktemp -d "${TMPDIR:-/tmp}/itgla-package-audit.XXXXXX")"
trap 'rm -rf -- "${temporary_directory}"' EXIT
cd "${repository_root}"
version="$(cargo metadata --locked --no-deps --format-version 1 | python3 -c 'import json, sys; print(json.load(sys.stdin)["packages"][0]["version"])')"
stem="itgla-v${version}"

mapfile -t artifacts < <(find "${artifact_directory}" -maxdepth 1 -type f \( -name "${stem}*.exe" -o -name "${stem}*.zip" -o -name "${stem}*.deb" -o -name "${stem}*.rpm" -o -name "${stem}*.AppImage" \) -print 2>/dev/null | sort)
if [[ ${#artifacts[@]} -eq 0 ]]; then
    echo "SKIP: no ${stem} packages in ${artifact_directory}"
    exit 2
fi

for artifact in "${artifacts[@]}"; do
    [[ -s "${artifact}" ]] || { echo "empty artifact: ${artifact}" >&2; exit 1; }
    if [[ -f "${artifact}.sha256" ]]; then
        (cd "$(dirname "${artifact}")" && sha256sum -c "$(basename "${artifact}.sha256")")
    fi
    case "${artifact}" in
        *-setup.exe)
            file "${artifact}" | grep -Eq 'Nullsoft Installer self-extracting archive'
            ;;
        *.exe)
            file "${artifact}" | grep -Eq 'PE32\+ executable.*x86-64'
            executable_metadata="$(x86_64-w64-mingw32-objdump -p "${artifact}")"
            grep -Eq 'Subsystem.*Windows GUI' <<< "${executable_metadata}"
            ;;
        *.deb)
            dpkg-deb --info "${artifact}" >/dev/null
            deb_contents="$(dpkg-deb --contents "${artifact}")"
            grep -Eq 'usr/(bin|lib)/itgla$' <<< "${deb_contents}"
            ;;
        *.rpm)
            rpm -qip "${artifact}" >/dev/null
            rpm_contents="$(rpm -qlp "${artifact}")"
            grep -Eq '^/usr/bin/itgla$' <<< "${rpm_contents}"
            grep -Eq '^/usr/share/applications/itgla.desktop$' <<< "${rpm_contents}"
            grep -Eq '^/usr/share/icons/hicolor/256x256/apps/itgla.png$' <<< "${rpm_contents}"
            ;;
        *.AppImage)
            file "${artifact}" | grep -Eq 'ELF 64-bit.*x86-64'
            [[ -x "${artifact}" ]] || { echo "AppImage is not executable: ${artifact}" >&2; exit 1; }
            ;;
        *.zip)
            unzip -t "${artifact}" >/dev/null
            unzip -l "${artifact}" | grep -Eq '/itgla.exe$'
            unzip -l "${artifact}" | grep -Eq '/itgla.ico$'
            unzip -p "${artifact}" '*/itgla.exe' > "${temporary_directory}/itgla.exe"
            file "${temporary_directory}/itgla.exe" | grep -Eq 'PE32\+ executable.*x86-64'
            executable_metadata="$(x86_64-w64-mingw32-objdump -p "${temporary_directory}/itgla.exe")"
            grep -Eq 'Subsystem.*Windows GUI' <<< "${executable_metadata}"
            ;;
    esac
done

missing=()
for suffix in windows-x86_64-setup.exe windows-x86_64.zip linux-x86_64.deb linux-x86_64.rpm linux-x86_64.AppImage; do
    [[ -f "${artifact_directory}/${stem}-${suffix}" ]] || missing+=("${suffix}")
done
if [[ ${#missing[@]} -gt 0 ]]; then
    echo "SKIP: validated ${#artifacts[@]} available artifacts, but current-version packages are missing: ${missing[*]}"
    exit 2
fi

echo "PASS: validated all ${#artifacts[@]} package artifacts for ${version}"
