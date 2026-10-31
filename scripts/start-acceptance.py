#!/usr/bin/env python3
"""Prepare an isolated native acceptance workspace; launch only with --launch."""
import argparse
from datetime import datetime, timezone
import hashlib
import json
import os
from pathlib import Path
import subprocess
from uuid import uuid4

ROOT = Path(__file__).resolve().parents[1]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binary", required=True, type=Path, help="Exact candidate executable to inspect")
    parser.add_argument("--launch", action="store_true", help="Open the candidate with the isolated workspace")
    args = parser.parse_args()
    binary = args.binary.resolve()
    if not binary.is_file():
        parser.error(f"Candidate binary does not exist: {binary}")
    stamp = datetime.now(timezone.utc).strftime("%Y%m%dT%H%M%SZ")
    output = ROOT / "test-results" / f"acceptance-{stamp}-{uuid4().hex[:8]}"
    workspace = output / "workspace"
    workspace.mkdir(parents=True, exist_ok=False)
    env = dict(os.environ, ITGLA_DATA_DIR=str(workspace), SLINT_BACKEND="winit-software")
    for key in list(env):
        if key.startswith("ITGLA_AUDIT_"):
            env.pop(key)
    record = dict(created_utc=stamp, binary=str(binary),
                  sha256=hashlib.sha256(binary.read_bytes()).hexdigest(),
                  workspace=str(workspace), renderer=env["SLINT_BACKEND"],
                  acceptance="NOT_RUN", launched=False)
    metadata = output / "launch.json"
    metadata.write_text(json.dumps(record, indent=2) + "\n", encoding="utf-8")
    print(f"Workspace: {workspace}")
    print(f"Binary SHA256: {record['sha256']}")
    if args.launch:
        with (output / "native.log").open("wb") as log:
            try:
                process = subprocess.Popen([str(binary)], cwd=output, env=env,
                                           stdout=log, stderr=subprocess.STDOUT)
            except OSError as exc:
                record["launch_error"] = str(exc)
                metadata.write_text(json.dumps(record, indent=2) + "\n", encoding="utf-8")
                parser.exit(1, f"Cannot launch candidate: {exc}\n")
        record.update(launched=True, pid=process.pid)
        metadata.write_text(json.dumps(record, indent=2) + "\n", encoding="utf-8")
        print(f"Started PID {process.pid}. Quit the app normally after acceptance.")
    print("No acceptance pass is inferred from launch. Use fictional sample data only.")


if __name__ == "__main__":
    main()
