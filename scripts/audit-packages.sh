#!/usr/bin/env bash
set -euo pipefail
repository_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
artifact_directory="${1:-${repository_root}/dist}"
cd "${repository_root}"
version="$(cargo metadata --locked --no-deps --format-version 1 | python3 -c 'import json, sys; print(json.load(sys.stdin)["packages"][0]["version"])')"
# No success-by-skipping: full release audit requires both DMGs and all five
# Windows/Linux packages. A local candidate can explicitly select one platform.
python3 scripts/verify-release-packages.py "${artifact_directory}" --version "${version}" --platform "${ITGLA_AUDIT_PLATFORM:-all}"

# Retain native archive/layout checks in addition to cross-platform hash verification.
if [[ "${ITGLA_AUDIT_PLATFORM:-all}" != windows && "${ITGLA_AUDIT_PLATFORM:-all}" != macos ]]; then
    stem="${artifact_directory}/itgla-v${version}-linux-x86_64"
    dpkg-deb --info "${stem}.deb" >/dev/null
    deb_contents="$(dpkg-deb --contents "${stem}.deb")"
    grep -Eq 'usr/(bin|lib)/itgla$' <<< "${deb_contents}"
    [[ -n "$(dpkg-deb -f "${stem}.deb" Depends)" ]] || { echo 'DEB Depends is empty' >&2; exit 1; }
    rpm -qip "${stem}.rpm" >/dev/null
    rpm_contents="$(rpm -qlp "${stem}.rpm")"
    grep -Fx '/usr/bin/itgla' <<< "${rpm_contents}" >/dev/null
    grep -Fx '/usr/share/applications/itgla.desktop' <<< "${rpm_contents}" >/dev/null
    grep -Fx '/usr/share/icons/hicolor/256x256/apps/itgla.png' <<< "${rpm_contents}" >/dev/null
    python3 scripts/check-linux-abi.py "${stem}.AppImage" --appimage
fi
