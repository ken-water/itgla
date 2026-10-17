#!/usr/bin/env bash
set -euo pipefail

repository_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "${repository_root}"

user_home_directory="$(getent passwd "$(id -u)" | cut -d: -f6)"
cargo_packager="$(command -v cargo-packager 2>/dev/null || true)"
if [[ -z "${cargo_packager}" && -x "${user_home_directory}/.cargo/bin/cargo-packager" ]]; then
    cargo_packager="${user_home_directory}/.cargo/bin/cargo-packager"
fi

for required_command in cargo rpmbuild python3 sha256sum; do
    if ! command -v "${required_command}" >/dev/null 2>&1; then
        echo "missing required command: ${required_command}" >&2
        exit 1
    fi
done
[[ -n "${cargo_packager}" ]] || { echo "missing required command: cargo-packager" >&2; exit 1; }

version="$(cargo metadata --locked --no-deps --format-version 1 | python3 -c 'import json, sys; print(json.load(sys.stdin)["packages"][0]["version"])')"
release_date="$(date -u '+%a %b %d %Y')"
package_stem="itgla-v${version}-linux-x86_64"
output_directory="${repository_root}/dist"
build_root="$(mktemp -d "${TMPDIR:-/tmp}/itgla-linux-installers.XXXXXX")"
rpm_top="${build_root}/rpmbuild"
package_output="${build_root}/packages"

cleanup() {
    rm -rf -- "${build_root}"
}
trap cleanup EXIT

mkdir -p "${output_directory}" "${package_output}"
cargo build --release --locked

# cargo-packager supplies the native Debian and AppImage layouts.
packager_status=0
"${cargo_packager}" --release --formats deb,appimage --out-dir "${package_output}" || packager_status=$?

# appimagetool normally downloads its runtime at the end of each build. If that
# network request fails, reuse the runtime prefix from the last verified
# AppImage. Only the runtime launcher is reused; the application filesystem is
# rebuilt from the current release binary.
if [[ ${packager_status} -ne 0 ]]; then
    app_dir="${package_output}/.cargo-packager/appimage/itgla.AppDir"
    previous_appimage="${output_directory}/${package_stem}.AppImage"
    plugin="${user_home_directory}/.cache/.cargo-packager/AppImage/linuxdeploy-plugin-appimage.AppImage"
    if [[ ! -d "${app_dir}" || ! -x "${previous_appimage}" || ! -x "${plugin}" ]]; then
        echo "cargo-packager failed and no verified AppImage runtime is available" >&2
        exit "${packager_status}"
    fi
    runtime_offset="$("${previous_appimage}" --appimage-offset)"
    [[ "${runtime_offset}" =~ ^[0-9]+$ ]] || { echo "cannot read AppImage runtime offset" >&2; exit 1; }
    dd if="${previous_appimage}" of="${build_root}/runtime-x86_64" bs=1 count="${runtime_offset}" status=none
    mkdir -p "${build_root}/appimagetool"
    (cd "${build_root}/appimagetool" && "${plugin}" --appimage-extract >/dev/null)
    "${build_root}/appimagetool/squashfs-root/usr/bin/appimagetool" \
        --runtime-file "${build_root}/runtime-x86_64" \
        "${app_dir}" "${package_output}/itgla_${version}_x86_64.AppImage"
fi

deb="$(find "${package_output}" -maxdepth 1 -type f -name '*.deb' -print -quit)"
appimage="$(find "${package_output}" -maxdepth 1 -type f -iname '*.appimage' -print -quit)"
[[ -n "${deb}" && -n "${appimage}" ]] || { echo "cargo-packager did not produce deb/AppImage" >&2; exit 1; }
mv "${deb}" "${output_directory}/${package_stem}.deb"
mv "${appimage}" "${output_directory}/${package_stem}.AppImage"

mkdir -p "${rpm_top}"/{BUILD,BUILDROOT,RPMS,SOURCES,SPECS,SRPMS}
cp target/release/itgla "${rpm_top}/SOURCES/itgla"
cp assets/itgla.png "${rpm_top}/SOURCES/itgla.png"
cat > "${rpm_top}/SPECS/itgla.spec" <<EOF
Name:           itgla
Version:        ${version}
Release:        1%{?dist}
Summary:        Local-first IT resource relationship manager
License:        Proprietary
URL:            https://itgla.com
Source0:        itgla
Source1:        itgla.png

%description
ITGLA is a local-first desktop inventory for IT resources.

%prep

%build

%install
install -D -m 0755 %{SOURCE0} %{buildroot}/usr/bin/itgla
install -D -m 0644 %{SOURCE1} %{buildroot}/usr/share/icons/hicolor/256x256/apps/itgla.png
install -D -m 0644 /dev/stdin %{buildroot}/usr/share/applications/itgla.desktop <<'DESKTOP'
[Desktop Entry]
Type=Application
Name=ITGLA
Comment=Local-first IT resource relationship manager
Exec=itgla
Icon=itgla
Terminal=false
Categories=Utility;
DESKTOP

%files
/usr/bin/itgla
/usr/share/applications/itgla.desktop
/usr/share/icons/hicolor/256x256/apps/itgla.png

%changelog
* ${release_date} ITGLA <service@itgla.com> - ${version}-1
- Package ITGLA ${version}
EOF
rpmbuild --define "_topdir ${rpm_top}" -bb "${rpm_top}/SPECS/itgla.spec"
rpm="$(find "${rpm_top}/RPMS" -type f -name '*.rpm' -print -quit)"
[[ -n "${rpm}" ]] || { echo "rpmbuild did not produce an RPM" >&2; exit 1; }
cp "${rpm}" "${output_directory}/${package_stem}.rpm"

for artifact in "${output_directory}/${package_stem}.deb" "${output_directory}/${package_stem}.rpm" "${output_directory}/${package_stem}.AppImage"; do
    sha256sum "${artifact}" > "${artifact}.sha256"
done

echo "Created Linux installers in ${output_directory}"
