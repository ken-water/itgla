#!/usr/bin/env bash
set -euo pipefail

repository_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "${repository_root}"

for required_command in cargo cargo-packager rpmbuild python3 sha256sum; do
    if ! command -v "${required_command}" >/dev/null 2>&1; then
        echo "missing required command: ${required_command}" >&2
        exit 1
    fi
done

version="$(cargo metadata --locked --no-deps --format-version 1 | python3 -c 'import json, sys; print(json.load(sys.stdin)["packages"][0]["version"])')"
package_stem="itgla-v${version}-linux-x86_64"
output_directory="${repository_root}/dist"
build_root="$(mktemp -d "${TMPDIR:-/tmp}/itgla-linux-installers.XXXXXX")"
rpm_top="${build_root}/rpmbuild"

cleanup() {
    rm -rf -- "${build_root}"
}
trap cleanup EXIT

mkdir -p "${output_directory}"
cargo build --release --locked

# cargo-packager supplies the native Debian and AppImage layouts.
cargo packager --release --formats deb,appimage --out-dir "${output_directory}"

deb="$(find "${output_directory}" -maxdepth 1 -type f -name '*.deb' -print -quit)"
appimage="$(find "${output_directory}" -maxdepth 1 -type f -iname '*.appimage' -print -quit)"
[[ -n "${deb}" && -n "${appimage}" ]] || { echo "cargo-packager did not produce deb/AppImage" >&2; exit 1; }
mv "${deb}" "${output_directory}/${package_stem}.deb"
mv "${appimage}" "${output_directory}/${package_stem}.AppImage"

mkdir -p "${rpm_top}"/{BUILD,BUILDROOT,RPMS,SOURCES,SPECS,SRPMS}
cp target/release/itgla "${rpm_top}/SOURCES/itgla"
cat > "${rpm_top}/SPECS/itgla.spec" <<EOF
Name:           itgla
Version:        ${version}
Release:        1%{?dist}
Summary:        Local-first IT resource relationship manager
License:        Proprietary
URL:            https://itgla.com
Source0:        itgla

%description
ITGLA is a local-first desktop inventory for IT resources.

%prep

%build

%install
install -D -m 0755 %{SOURCE0} %{buildroot}/usr/bin/itgla

%files
/usr/bin/itgla

%changelog
* Thu Sep 25 2026 ITGLA <hello@itgla.com> - ${version}-1
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
