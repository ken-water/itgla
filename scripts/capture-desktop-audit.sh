#!/usr/bin/env bash
set -euo pipefail

repository_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
output_directory="${1:-${repository_root}/test-results/visual}"
binary="${repository_root}/target/debug/itgla"
data_directory="$(mktemp -d "${TMPDIR:-/tmp}/itgla-desktop-audit.XXXXXX")"

cleanup() {
    rm -rf -- "${data_directory}"
}
trap cleanup EXIT

for command in cargo xvfb-run xwininfo import sqlite3 python3; do
    command -v "${command}" >/dev/null 2>&1 || { echo "missing required command: ${command}" >&2; exit 1; }
done

mkdir -p "${output_directory}"
cd "${repository_root}"
cargo build --locked

capture() {
    local view="$1"
    local width="$2"
    local height="$3"
    local output="${output_directory}/${view}-${width}x${height}.png"
    local runtime_log="${output_directory}/${view}-${width}x${height}.log"
    xvfb-run -a -s "-screen 0 ${width}x${height}x24" bash -c '
        set -euo pipefail
        ITGLA_DATA_DIR="$2" ITGLA_AUDIT_VIEW="$3" ITGLA_AUDIT_WINDOW_SIZE="$4x$5" "$1" >"$7" 2>&1 &
        app_pid=$!
        # Dense tables can take longer to map their first frame than an empty
        # workspace. Wait for the actual named window, not a fixed startup delay.
        for attempt in $(seq 1 40); do
            kill -0 "$app_pid"
            if xwininfo -root -tree | grep -q ITGLA; then
                break
            fi
            sleep 0.25
        done
        xwininfo -root -tree | grep -q ITGLA || { echo "ITGLA window did not appear" >&2; exit 1; }
        sleep 1
        import -window root "$6"
        kill "$app_pid" 2>/dev/null || true
        wait "$app_pid" 2>/dev/null || true
    ' _ "${binary}" "${data_directory}" "${view}" "${width}" "${height}" "${output}" "${runtime_log}"
    python3 scripts/analyze-desktop-screenshot.py "${output}" "${width}" "${height}" > "${output%.png}.json"
}

# First launch also initializes and migrates the isolated database.
capture guide 1000 640

{
    echo 'BEGIN;'
    for index in $(seq 1 32); do
        echo "INSERT INTO server_custom_columns(name,position) VALUES('Field ${index}',${index});"
    done
    for index in $(seq 1 60); do
        echo "INSERT INTO server_records(name,purpose,tags,ip_address,ports,environment,status,position) VALUES('server-${index}','Service host ${index}','production, service-${index}','203.0.113.${index}','22, 443, 8000','production','active',${index});"
    done
    for server in $(seq 1 60); do
        for column in $(seq 1 32); do
            echo "INSERT INTO server_custom_values(server_id,column_id,value) VALUES(${server},${column},'value-${server}-${column}');"
        done
    done
    echo 'COMMIT;'
} | sqlite3 "${data_directory}/itgla.db"

capture servers 1000 640
capture servers 1240 800
capture servers 1440 900
capture sites 1000 640
capture sites 1240 800
capture sites 1440 900
capture import 1000 640

echo "Desktop audit captures: ${output_directory}"
