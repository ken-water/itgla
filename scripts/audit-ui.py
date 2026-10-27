#!/usr/bin/env python3
"""Static, deterministic contracts for the Slint desktop surface."""

from __future__ import annotations

import re
import sys
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
UI = ROOT / "ui" / "app.slint"
MAIN = ROOT / "src" / "main.rs"


def luminance(color: str) -> float:
    channels = [int(color[index : index + 2], 16) / 255 for index in (1, 3, 5)]
    linear = [
        value / 12.92 if value <= 0.04045 else ((value + 0.055) / 1.055) ** 2.4
        for value in channels
    ]
    return 0.2126 * linear[0] + 0.7152 * linear[1] + 0.0722 * linear[2]


def contrast(foreground: str, background: str) -> float:
    lighter, darker = sorted(
        (luminance(foreground), luminance(background)), reverse=True
    )
    return (lighter + 0.05) / (darker + 0.05)


def main() -> int:
    source = UI.read_text(encoding="utf-8")
    controls = (ROOT / "ui" / "controls.slint").read_text(encoding="utf-8")
    rust_source = MAIN.read_text(encoding="utf-8")
    failures: list[str] = []

    required_contracts = {
        "minimum desktop viewport": "min-width: 1000px; min-height: 640px",
        "table geometry binding": "width: root.table-width",
        "table horizontal reset": "root.server-table-scroll-x = 0px",
        "Sites table geometry": "width: root.site-table-width",
        "Sites horizontal reset": "root.site-table-scroll-x = 0px",
        "Sites default fields": 'title: "ENVIRONMENT"',
        "resource-specific count": 'root.site-count + (root.site-count == 1 ? " SITE" : " SITES")',
        "first-run guide": "Start with one server",
        "import defaults": "Defaults for empty or missing columns",
        "readable default font": "font-size-step: 2",
        "inherited control font": "default-font-size: root.control-font-size",
        "semantic caption token": "property <length> caption-font-size:",
        "shared page header": "component PageHeader inherits Text",
        "shared field label": "component FieldLabel inherits Text",
        "keyboard navigation primitive": "component NavigationTab inherits Rectangle",
        "keyboard choice primitive": "component ChoiceRow inherits Rectangle",
        "visible custom focus": "border-width: tab-focus.has-focus ? 2px",
        "accessible custom action": "accessible-action-default =>",
        "explicit save result": 'text: root.status-message; color: root.status-is-error ? UiColors.error : UiColors.success',
        "guarded column drag": "if self.pressed { root.resize",
        "non-overlapping sort target": "width: parent.width - 14px; enabled: root.ascending-mode",
        "truncated-only tooltip": "measurement.preferred-width > root.width - 38px: Tooltip",
    }
    for label, fragment in required_contracts.items():
        if fragment not in source:
            failures.append(f"missing UI contract: {label}")

    if "window.set_sort_mode_index(state.sort_mode as i32)" not in rust_source:
        failures.append("sort state is not synchronized back to the table header")
    if "width: 292px; height: 54px" in source:
        failures.append("oversized instant cell preview has returned")

    forbidden_functional_sizes = re.findall(r"font-size:\s*(8|9)px", source)
    if forbidden_functional_sizes:
        failures.append(
            "functional 8px/9px text has returned; use semantic caption/body tokens"
        )

    # Drags, modal shields and tooltip hover areas intentionally have no focus target.
    pointer_only_controls = len(re.findall(r"TouchArea\s*\{", source))
    non_action_areas = len(re.findall(r"TouchArea\s*\{\s*\}", source)) + len(re.findall(r"TouchArea\s*\{[^{}]*?(?:moved\s*=>|mouse-cursor:\s*ew-resize|enabled:\s*root.show-tooltips)", source))
    focus_scopes = len(re.findall(r"FocusScope\s*\{", source))
    if focus_scopes < pointer_only_controls - non_action_areas - 2:  # resize grips have Fit columns equivalents
        failures.append(
            f"custom interaction coverage is incomplete: {pointer_only_controls} TouchAreas, "
            f"only {focus_scopes} FocusScopes"
        )

    for custom_component in ("NavigationTab", "DataCell", "HeaderCell", "TagChoice"):
        match = re.search(
            rf"component {custom_component}\b(?P<body>.*?)(?=\ncomponent |\nexport component )",
            source,
            re.S,
        )
        if not match:
            failures.append(f"missing custom interaction component: {custom_component}")
            continue
        body = match.group("body")
        for fragment in ("forward-focus:", "accessible-role:", "accessible-action-default =>"):
            if fragment not in body:
                failures.append(f"{custom_component} is missing {fragment}")

    modal_sizes = re.findall(
        r"\(parent\.width - (\d+)px\) / 2; y: \(parent\.height - (\d+)px\) / 2",
        source,
    )
    for width_text, height_text in modal_sizes:
        width, height = int(width_text), int(height_text)
        if width > 940 or height > 600:
            failures.append(
                f"modal {width}x{height} does not fit the 1000x640 viewport with safe margins"
            )

    # Explicit text/background pairs used by small body and metadata text. Decorative
    # borders are intentionally excluded. Small text must reach WCAG's 4.5:1 ratio.
    contrast_pairs = [
        ("#52645b", "#fbfcf9", "secondary text"),
        ("#52645b", "#eef1eb", "metadata on workspace"),
        ("#d7ee9d", "#183c32", "brand on header"),
        ("#bbc9d7", "#1e2833", "dark secondary text"),
        ("#edf2f7", "#1e2833", "dark body text"),
        ("#76501d", "#fff6e7", "impact category"),
    ]
    for foreground, background, label in contrast_pairs:
        if foreground not in source + controls or background not in source + controls:
            failures.append(f"contrast token missing for {label}")
            continue
        ratio = contrast(foreground, background)
        if ratio < 4.5:
            failures.append(f"contrast {label} is {ratio:.2f}:1; expected at least 4.5:1")

    button_heights = [int(value) for value in re.findall(r"Button\s*\{[^{}]*?height:\s*(\d+)px", source)]
    undersized = sorted({height for height in button_heights if height < 24})
    if undersized:
        failures.append(f"button heights below the 24px desktop minimum: {undersized}")

    if re.search(r"[\u4e00-\u9fff]", source):
        failures.append("English-only UI contains Han characters")

    if re.search(r"match\s+callback_repository\s*\.borrow_mut\(\)", rust_source):
        failures.append(
            "mutable Repository RefCell borrow is held by a match scrutinee; store the result first to prevent callback panics"
        )

    if failures:
        for failure in failures:
            print(f"FAIL: {failure}")
        return 1

    print(
        f"PASS: {len(required_contracts)} UI contracts, {len(modal_sizes)} modal bounds, "
        f"{len(contrast_pairs)} contrast pairs, and desktop target sizes"
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
