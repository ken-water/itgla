#!/usr/bin/env bash
set -euo pipefail

repository_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
output_directory="${1:-${repository_root}/test-results/interactions}"
data_directory="$(mktemp -d "${TMPDIR:-/tmp}/itgla-interaction-audit.XXXXXX")"
driver="$(mktemp "${TMPDIR:-/tmp}/itgla-x11-driver.XXXXXX")"

cleanup() {
    rm -rf -- "${data_directory}"
    rm -f -- "${driver}"
}
trap cleanup EXIT

for command in cargo gcc xvfb-run import sqlite3 python3; do
    command -v "${command}" >/dev/null 2>&1 || { echo "missing required command: ${command}" >&2; exit 1; }
done

mkdir -p "${output_directory}"
cd "${repository_root}"
cargo build --locked
gcc scripts/x11-interaction-driver.c -o "${driver}" -lX11 -lXtst

xvfb-run -a -s '-screen 0 1000x640x24' bash -c '
    set -euo pipefail
    ITGLA_DATA_DIR="$1" ITGLA_AUDIT_VIEW=servers ITGLA_AUDIT_WINDOW_SIZE=1000x640 ITGLA_AUDIT_SERVER_FIELDS=1 "$2" >"$3" 2>&1 &
    app_pid=$!
    sleep 2
    "$4"
    sleep 1
    kill -0 "$app_pid"
    import -window root "$5"
    kill "$app_pid" 2>/dev/null || true
    wait "$app_pid" 2>/dev/null || true
' _ "${data_directory}" "${repository_root}/target/debug/itgla" "${output_directory}/runtime.log" "${driver}" "${output_directory}/final.png"

if grep -Eq 'panicked|fatal runtime error' "${output_directory}/runtime.log"; then
    echo "desktop runtime panicked during interaction audit" >&2
    cat "${output_directory}/runtime.log" >&2
    exit 1
fi

record="$(sqlite3 -separator '|' "${data_directory}/itgla.db" "SELECT name,tags,ip_address,ports,environment,status,archived_at IS NULL FROM server_records ORDER BY id LIMIT 1;")"
IFS='|' read -r saved_name saved_tags saved_ip saved_ports saved_environment saved_status saved_active <<< "${record}"
[[ "${saved_name}" == 'audit-server' && "${saved_tags}" == 'audit' && "${saved_ip}" == '203.0.113.10' && "${saved_environment}" == 'Production' && "${saved_status}" == 'Active' && "${saved_active}" == '1' ]] || {
    echo "unexpected saved server after pointer-positioned edit/hide/restore: ${record}" >&2
    exit 1
}
[[ "${saved_ports}" != '22, 443' && "${saved_ports}" == *8* && "${saved_ports}" != 8* && "${saved_ports}" != *8 ]] || {
    echo "pointer-positioned edit was not inserted inside Ports: ${saved_ports}" >&2
    exit 1
}
font_size_step="$(sqlite3 "${data_directory}/itgla.db" "SELECT value FROM app_settings WHERE key='font_size_step';")"
[[ "${font_size_step}" == '2' ]] || {
    echo "font-size increase was not persisted: ${font_size_step}" >&2
    exit 1
}
name_width="$(sqlite3 "${data_directory}/itgla.db" "SELECT json_extract(value,'\$[0]') FROM app_settings WHERE key='column_widths';")"
[[ "${name_width}" == '200.0' ]] || {
    echo "held Name-column drag did not persist the expected width: ${name_width}" >&2
    exit 1
}

python3 scripts/analyze-desktop-screenshot.py "${output_directory}/final.png" 1000 640 > "${output_directory}/final.json"
echo "PASS: native server fields, pointer-positioned edit, save, fit, font, guide, import, tabs, horizontal scroll, hide, restore, and persistent column dragging"
