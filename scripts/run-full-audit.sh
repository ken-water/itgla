#!/usr/bin/env bash
set -uo pipefail

repository_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
timestamp="$(date -u +%Y%m%dT%H%M%SZ)"
result_directory="${repository_root}/test-results/${timestamp}"
summary="${result_directory}/report.md"
mkdir -p "${result_directory}/logs"
cd "${repository_root}"

passed=0
failed=0
skipped=0

{
    echo "# ITGLA full automated audit"
    echo
    echo "- Run: ${timestamp}"
    echo "- Commit: $(git rev-parse --short HEAD 2>/dev/null || echo unavailable)"
    echo "- Working tree: $(git status --short | wc -l) changed paths"
    echo
    echo "| Gate | Result | Evidence |"
    echo "|---|---|---|"
} > "${summary}"

run_gate() {
    local name="$1"
    shift
    local slug="$(echo "${name}" | tr '[:upper:] ' '[:lower:]-' | tr -cd 'a-z0-9_-')"
    local log="${result_directory}/logs/${slug}.log"
    "$@" >"${log}" 2>&1
    local status=$?
    if [[ ${status} -eq 0 ]]; then
        echo "| ${name} | PASS | logs/${slug}.log |" >> "${summary}"
        passed=$((passed + 1))
    elif [[ ${status} -eq 2 ]]; then
        echo "| ${name} | SKIP | logs/${slug}.log |" >> "${summary}"
        skipped=$((skipped + 1))
    else
        echo "| ${name} | FAIL (${status}) | logs/${slug}.log |" >> "${summary}"
        failed=$((failed + 1))
    fi
}

run_gate "Rust format" cargo fmt --all -- --check
run_gate "Rust compile" cargo check --workspace --all-targets --all-features --locked
run_gate "Rust lint" cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
run_gate "Rust tests" cargo test --workspace --all-features --locked
run_gate "Rust advisories" bash scripts/audit-rust-advisories.sh
run_gate "Rust coverage" bash scripts/audit-rust-coverage.sh
run_gate "English UI" node scripts/check-english-ui.mjs
run_gate "Slint UI contracts" python3 scripts/audit-ui.py
run_gate "Release build" cargo build --release --locked
run_gate "Desktop visual matrix" bash scripts/capture-desktop-audit.sh "${result_directory}/visual"
run_gate "Desktop interactions" bash scripts/audit-desktop-interactions.sh "${result_directory}/interactions"
run_gate "Current packages" bash scripts/audit-packages.sh
run_gate "Whitespace" git diff --check

{
    echo
    echo "## Summary"
    echo
    echo "- Passed: ${passed}"
    echo "- Failed: ${failed}"
    echo "- Skipped: ${skipped}"
    echo
    echo "## Manual-only residual checks"
    echo
    echo "- Physical Windows Narrator/NVDA announcement quality (automated Slint accessibility-tree role/name/action checks run in Rust tests)."
    echo "- Physical Windows full keyboard traversal and focus visibility (automated forward/reverse navigation and custom-control activation run in Rust tests)."
    echo "- Physical Windows 125%, 150%, and 200% DPI/font-rendering inspection (scale-factor events are covered automatically)."
    echo "- Native installer interaction, upgrade, uninstall, and OS security prompts."
    echo "- macOS signing/notarization and physical high-DPI/multi-monitor behavior."
    echo "- Clipboard integration and native file-picker behavior on each target OS."
} >> "${summary}"

python3 - "${result_directory}" "${passed}" "${failed}" "${skipped}" <<'PY'
import json
import sys
from pathlib import Path

directory = Path(sys.argv[1])
payload = {
    "passed": int(sys.argv[2]),
    "failed": int(sys.argv[3]),
    "skipped": int(sys.argv[4]),
    "report": str(directory / "report.md"),
}
(directory / "summary.json").write_text(json.dumps(payload, indent=2) + "\n", encoding="utf-8")
PY

echo "Audit report: ${summary}"
[[ ${failed} -eq 0 ]]
