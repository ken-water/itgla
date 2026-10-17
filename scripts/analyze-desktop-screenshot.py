#!/usr/bin/env python3
"""Reject blank, corrupt, or unexpectedly dark desktop screenshots."""

from __future__ import annotations

import json
import sys
from pathlib import Path

from PIL import Image, ImageChops, ImageStat


def main() -> int:
    if len(sys.argv) != 4:
        print("usage: analyze-desktop-screenshot.py IMAGE WIDTH HEIGHT", file=sys.stderr)
        return 2
    path = Path(sys.argv[1])
    expected = (int(sys.argv[2]), int(sys.argv[3]))
    image = Image.open(path).convert("RGB")
    colors = image.getcolors(maxcolors=image.width * image.height)
    extrema = ImageStat.Stat(image).extrema
    dark_pixels = sum(
        count for count, color in (colors or []) if max(color) < 24
    )
    dark_ratio = dark_pixels / (image.width * image.height) if colors else 0.0
    background = Image.new("RGB", image.size, image.getpixel((0, 0)))
    content_box = ImageChops.difference(image, background).getbbox()
    result = {
        "path": str(path),
        "size": list(image.size),
        "expected_size": list(expected),
        "dark_ratio": round(dark_ratio, 5),
        "extrema": extrema,
        "content_box": list(content_box) if content_box else None,
    }
    print(json.dumps(result, indent=2))
    if image.size != expected:
        print(f"FAIL: screenshot size {image.size} != {expected}", file=sys.stderr)
        return 1
    if content_box is None:
        print("FAIL: screenshot is a single color", file=sys.stderr)
        return 1
    if any(high - low < 24 for low, high in extrema):
        print("FAIL: screenshot has insufficient visual range", file=sys.stderr)
        return 1
    if dark_ratio > 0.35:
        print(f"FAIL: unexpected near-black area ({dark_ratio:.1%})", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
