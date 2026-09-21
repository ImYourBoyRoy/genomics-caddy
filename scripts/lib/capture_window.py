#!/usr/bin/env python3
# ./scripts/lib/capture_window.py
"""Capture one named X11 window to PNG and WebP.

How to run: python3 ./scripts/lib/capture_window.py --title "Genomics Caddy" --out docs/images/foo.png
Inputs: window title (exact xwininfo -name), output PNG path.
Outputs: PNG plus sibling WebP. Uses a full-monitor grab then crop so it does
not mix python-xlib and mss in one connection. Never reads app databases.
"""

from __future__ import annotations

import argparse
import re
import subprocess
from pathlib import Path

from PIL import Image
from mss import MSS


def window_geometry(title: str) -> tuple[int, int, int, int]:
    result = subprocess.run(
        ["xwininfo", "-name", title],
        check=False,
        capture_output=True,
        text=True,
    )
    if result.returncode != 0:
        raise SystemExit(result.stderr.strip() or f"no window named {title!r}")
    text = result.stdout
    x = int(re.search(r"Absolute upper-left X:\s+(-?\d+)", text).group(1))
    y = int(re.search(r"Absolute upper-left Y:\s+(-?\d+)", text).group(1))
    width = int(re.search(r"Width:\s+(\d+)", text).group(1))
    height = int(re.search(r"Height:\s+(\d+)", text).group(1))
    if width < 640 or height < 480:
        raise SystemExit(f"window too small: {width}x{height}")
    return x, y, width, height


def capture(title: str, out_png: Path) -> None:
    x, y, width, height = window_geometry(title)
    with MSS() as sct:
        monitor = sct.monitors[1] if len(sct.monitors) > 1 else sct.monitors[0]
        raw = sct.grab(monitor)
        image = Image.frombytes("RGB", raw.size, raw.rgb)
    left = max(x - int(monitor["left"]), 0)
    top = max(y - int(monitor["top"]), 0)
    image = image.crop((left, top, left + width, top + height))
    if image.size[0] < 640 or image.size[1] < 480:
        raise SystemExit(f"cropped capture too small: {image.size}")
    out_png.parent.mkdir(parents=True, exist_ok=True)
    image.save(out_png, "PNG", optimize=True)
    webp = out_png.with_suffix(".webp")
    image.save(webp, "WEBP", quality=82, method=6)
    print(f"captured {width}x{height} -> {out_png.name} {webp.name}")


def main() -> None:
    parser = argparse.ArgumentParser(description="Capture a named desktop window.")
    parser.add_argument("--title", default="Genomics Caddy")
    parser.add_argument("--out", required=True)
    args = parser.parse_args()
    capture(args.title, Path(args.out))


if __name__ == "__main__":
    main()
