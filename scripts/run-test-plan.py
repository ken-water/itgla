#!/usr/bin/env python3
"""Run local ITGLA checks; retain logs and an unfilled manual acceptance checklist.

Python 3.10+, standard library only. No installs, remote writes or user DB access.
"""
from __future__ import annotations

import argparse
import csv
import hashlib
import json
import os
from pathlib import Path
import platform
import shutil
import signal
import subprocess
import sys
import time
from datetime import datetime, timezone
from uuid import uuid4

ROOT = Path(__file__).resolve().parents[1]


def commands(profile: str, toolchain: str, website: bool = False) -> list[dict]:
    cargo = ["cargo", f"+{toolchain}"]
    def gate(name, command, cwd=ROOT, timeout=900):
        return dict(id=name, command=command, cwd=str(cwd), timeout=timeout)
    result = [
        gate("runner", [sys.executable, "scripts/test_test_plan_runner.py"]),
        gate("format", cargo + ["fmt", "--all", "--", "--check"]),
        gate("english", ["node", "scripts/check-english-ui.mjs"]),
        gate("ui-static", [sys.executable, "scripts/audit-ui.py"]),
        gate("whitespace", ["git", "diff", "--check"]),
    ]
    test = cargo + ["test", "--all-features", "--locked", "-j", "2"]
    if profile == "quick":
        test += ["asset_context::tests::"]
    result.append(gate("rust-tests", test, timeout=1800))
    if profile != "quick":
        result.append(gate("clippy", cargo + ["clippy", "--all-targets", "--all-features",
                                                "--locked", "-j", "2", "--", "-D", "warnings"],
                           timeout=1800))
    if profile == "release":
        result += [
            gate("advisories", cargo + ["deny", "check", "advisories"]),
            gate("coverage", cargo + ["llvm-cov", "--workspace", "--all-features", "--locked",
                                      "-j", "2", "--summary-only", "--fail-under-lines", "60"], timeout=3600),
            gate("release-build", cargo + ["build", "--release", "--locked", "-j", "2"],
                 timeout=3600),
        ]
    if website:
        # Resolve npm-cli.js to avoid shell=True / Windows .cmd argument interpolation.
        node = shutil.which("node")
        cli = Path(node).parent / "node_modules/npm/bin/npm-cli.js" if node else Path("missing-npm")
        result.append(gate("website", ["node", str(cli), "test"], ROOT / "website", 1800))
    return result


def stop_tree(process: subprocess.Popen) -> None:
    if os.name == "nt":
        subprocess.run(["taskkill", "/PID", str(process.pid), "/T", "/F"],
                       stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL, timeout=20)
    else:
        try:
            os.killpg(process.pid, signal.SIGKILL)
        except ProcessLookupError:
            pass
    process.wait(timeout=20)


def run_gate(spec: dict, output: Path, env: dict, timeout_override=None) -> dict:
    log = output / f"{spec['id']}.log"
    command = spec["command"]
    result = dict(spec, status="BLOCKED", exit_code=None, log=log.name, seconds=0)
    if not shutil.which(command[0]):
        log.write_text(f"Missing executable: {command[0]}\n", encoding="utf-8")
        return result
    started = time.monotonic()
    process = None
    with log.open("w", encoding="utf-8") as stream:
        stream.write(json.dumps(command) + "\n")
        stream.flush()
        try:
            process = subprocess.Popen(command, cwd=spec["cwd"], env=env, stdout=stream,
                                       stderr=subprocess.STDOUT,
                                       start_new_session=os.name != "nt",
                                       creationflags=subprocess.CREATE_NEW_PROCESS_GROUP if os.name == "nt" else 0)
            code = process.wait(timeout=timeout_override or spec["timeout"])
            result.update(status="PASS" if code == 0 else "FAIL", exit_code=code)
        except subprocess.TimeoutExpired:
            stop_tree(process)
            result["status"] = "TIMEOUT"
        except KeyboardInterrupt:
            if process is not None:
                stop_tree(process)
            result["status"] = "INTERRUPTED"
        except OSError as exc:
            stream.write(f"Cannot launch gate: {type(exc).__name__}: {exc}\n")
    result["seconds"] = round(time.monotonic() - started, 3)
    return result


def exit_code(results: list[dict]) -> int:
    if any(r["status"] in ("FAIL", "TIMEOUT", "INTERRUPTED") for r in results):
        return 1
    return 2 if any(r["status"] != "PASS" for r in results) else 0


def capture(command: list[str]) -> str:
    try:
        return subprocess.check_output(command, cwd=ROOT, stderr=subprocess.STDOUT,
                                       timeout=20).decode("utf-8", errors="replace").strip()
    except (OSError, subprocess.SubprocessError):
        return "UNAVAILABLE"


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--profile", choices=["quick", "full", "release"], default="quick")
    parser.add_argument("--toolchain", default="1.92.0")
    parser.add_argument("--website", action="store_true", help="Also run the existing local website suite")
    parser.add_argument("--list", action="store_true", help="Print commands without executing or writing")
    args = parser.parse_args()
    specs = commands(args.profile, args.toolchain, args.website)
    if args.list:
        print(json.dumps(specs, indent=2))
        return 0
    stamp = datetime.now(timezone.utc).strftime("%Y%m%dT%H%M%SZ")
    output = ROOT / "test-results" / f"plan-{stamp}-{uuid4().hex[:8]}"
    output.mkdir(parents=True, exist_ok=False)
    workspace = output / "workspace"
    workspace.mkdir()
    scratch = output / "temp"
    scratch.mkdir()
    env = dict(os.environ, ITGLA_DATA_DIR=str(workspace), TMP=str(scratch), TEMP=str(scratch),
               TMPDIR=str(scratch))
    # A prior native audit session must not change deterministic test fixtures.
    for key in list(env):
        if key.startswith("ITGLA_AUDIT_"):
            env.pop(key)
    case_path = ROOT / "docs/testing/phase-one-cases.json"
    cases = json.loads(case_path.read_text(encoding="utf-8"))
    fields = ["id", "priority", "title", "coverage", "status", "tester", "environment", "evidence", "issue"]
    with (output / "acceptance.csv").open("w", newline="", encoding="utf-8-sig") as stream:
        writer = csv.DictWriter(stream, fields)
        writer.writeheader()
        for case in cases:
            writer.writerow({**{k: case[k] for k in fields if k in case}, "status": "NOT_RUN"})
    metadata = dict(started_utc=stamp, profile=args.profile, platform=platform.platform(),
                    python=sys.version, toolchain=args.toolchain,
                    git_head=capture(["git", "rev-parse", "HEAD"]),
                    git_status_before=capture(["git", "status", "--short"]),
                    rust=capture(["rustc", f"+{args.toolchain}", "--version"]),
                    node=capture(["node", "--version"]),
                    case_sha256=hashlib.sha256(case_path.read_bytes()).hexdigest())
    results = []
    print(f"Evidence: {output}", flush=True)
    for spec in specs:
        print(f"RUN {spec['id']}", flush=True)
        result = run_gate(spec, output, env)
        results.append(result)
        print(f"{result['status']} {spec['id']} ({result['seconds']}s)", flush=True)
        if result["status"] == "INTERRUPTED":
            for pending in specs[len(results):]:
                results.append(dict(pending, status="NOT_RUN", exit_code=None,
                                    seconds=0, log=""))
            break
    metadata.update(finished_utc=datetime.now(timezone.utc).isoformat(),
                    git_status_after=capture(["git", "status", "--short"]))
    binaries = [ROOT / "target/release/itgla.exe", ROOT / "target/release/itgla"]
    metadata["existing_release_artifacts"] = [
        dict(path=str(p), sha256=hashlib.sha256(p.read_bytes()).hexdigest(),
             mtime_utc=datetime.fromtimestamp(p.stat().st_mtime, timezone.utc).isoformat())
        for p in binaries if p.is_file()
    ]
    code = exit_code(results)
    report = dict(metadata=metadata, automated_exit_code=code, gates=results,
                  release_readiness="NOT_EVALUATED", acceptance="NOT_RUN",
                  note="Existing binary hashes do not establish a build from this source state.")
    (output / "summary.json").write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
    lines = ["# ITGLA local test report", "", f"Profile: {args.profile}; automated exit: {code}.", "",
             "Release readiness: NOT EVALUATED. Manual acceptance: NOT RUN.", "",
             "| Gate | Status | Seconds | Log |", "|---|---|---:|---|"]
    lines += [f"| {r['id']} | {r['status']} | {r['seconds']} | [{r['log']}]({r['log']}) |" for r in results]
    lines += ["", "See summary.json for commands, source revision, working-tree changes and environment.",
              "Fill acceptance.csv after checking the steps in docs/testing/phase-one-cases.json.",
              "A passing automation profile is not installer, accessibility or user acceptance approval.", ""]
    (output / "report.md").write_text("\n".join(lines), encoding="utf-8")
    print(f"Report: {output / 'report.md'}", flush=True)
    return code


if __name__ == "__main__":
    sys.exit(main())
