#!/usr/bin/env bash
set -euo pipefail

repository_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
minimum_line_coverage="${ITGLA_MIN_LINE_COVERAGE:-60.0}"
user_home_directory="$(getent passwd "$(id -u)" | cut -d: -f6)"
cargo_bin_directory="${user_home_directory}/.cargo/bin"

[[ -x "${cargo_bin_directory}/cargo-llvm-cov" ]] || {
    echo "SKIP: cargo-llvm-cov is not installed" >&2
    exit 2
}

cd "${repository_root}"
summary="$(PATH="${cargo_bin_directory}:${PATH}" cargo llvm-cov --workspace --all-features --summary-only)"
echo "${summary}"
line_coverage="$(awk '/^TOTAL/{gsub(/%/, "", $10); print $10}' <<< "${summary}")"
[[ -n "${line_coverage}" ]] || { echo "cannot parse line coverage" >&2; exit 1; }
awk -v actual="${line_coverage}" -v minimum="${minimum_line_coverage}" 'BEGIN { exit !(actual + 0 >= minimum + 0) }' || {
    echo "line coverage ${line_coverage}% is below ${minimum_line_coverage}%" >&2
    exit 1
}
echo "PASS: line coverage ${line_coverage}% (minimum ${minimum_line_coverage}%)"
