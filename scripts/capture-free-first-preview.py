"""Capture real Slint software renders from isolated fictional demo workspaces.

This does not establish OS DPI, compositor frame timing or screen-reader support.
Requires the built debug executable and Pillow for PPM-to-PNG conversion.
"""
import hashlib
import json
import os
from pathlib import Path
import subprocess
import tempfile
import time

from PIL import Image

ROOT = Path(__file__).resolve().parents[1]


def main():
    binary = ROOT / "target/debug/itgla.exe"
    if not binary.is_file():
        raise SystemExit("Build the current native debug candidate first.")
    output = Path(tempfile.mkdtemp(prefix="itgla-free-first-native-"))
    captures = []
    for view, large in [("overview", False), ("guide", False), ("backup", True),
                        ("recovery", True), ("assets", True), ("connections", True)]:
        ppm = output / f"{view}.ppm"
        env = {k: v for k, v in os.environ.items() if not k.startswith("ITGLA_AUDIT_")}
        env.update(ITGLA_DATA_DIR=str(output / f"workspace-{view}"),
                   ITGLA_AUDIT_DEMO="1", ITGLA_AUDIT_VIEW=view,
                   ITGLA_AUDIT_WINDOW_SIZE="1100x720", ITGLA_AUDIT_SNAPSHOT=str(ppm),
                   SLINT_BACKEND="winit-software")
        if large:
            env["ITGLA_AUDIT_LARGE_DARK"] = "1"
        with (output / f"{view}.log").open("wb") as log:
            process = subprocess.Popen([str(binary)], cwd=ROOT, env=env, stdout=log, stderr=subprocess.STDOUT)
            try:
                deadline = time.monotonic() + 45
                while time.monotonic() < deadline:
                    if ppm.is_file():
                        try:
                            with Image.open(ppm) as image:
                                image.load()
                                image.save(ppm.with_suffix(".png"))
                            break
                        except (OSError, ValueError):
                            pass  # The renderer may still be writing the file.
                    if process.poll() is not None:
                        raise RuntimeError(f"{view} exited before capture; inspect {output}")
                    time.sleep(.2)
                else:
                    raise RuntimeError(f"Capture timed out for {view}; inspect {output}")
            finally:
                if process.poll() is None:
                    process.terminate()  # Only the process started by this capture script.
                    try:
                        process.wait(timeout=5)
                    except subprocess.TimeoutExpired:
                        process.kill()
                        process.wait(timeout=5)
        captures.append(dict(view=view, large_dark=large, png=str(ppm.with_suffix(".png"))))
    manifest = dict(binary=str(binary), sha256=hashlib.sha256(binary.read_bytes()).hexdigest(),
                    scope="Native software rendering, isolated fictional demo, 1100x720; not physical DPI or frame timing",
                    captures=captures)
    (output / "manifest.json").write_text(json.dumps(manifest, indent=2) + "\n", encoding="utf-8")
    print(output / "manifest.json")


if __name__ == "__main__":
    main()
