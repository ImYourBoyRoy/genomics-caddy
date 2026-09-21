#!/usr/bin/env python3
# ./scripts/lib/make_theme_webp.py
"""Build a looping light/dark animated WebP from two still PNG frames.

How to run: python3 ./scripts/lib/make_theme_webp.py --dark a-dark.png --light a-light.png --out a.webp
Inputs: same-size PNG masters (dark then light).
Outputs: animated WebP, dark frame first. Never reads app databases.
"""

from __future__ import annotations

import argparse
from pathlib import Path

from PIL import Image


def as_rgb(image: Image.Image) -> Image.Image:
    if image.mode == "RGB":
        return image
    return image.convert("RGB")


def main() -> None:
    parser = argparse.ArgumentParser(description="Encode a dark/light animated WebP.")
    parser.add_argument("--dark", required=True)
    parser.add_argument("--light", required=True)
    parser.add_argument("--out", required=True)
    parser.add_argument("--duration-ms", type=int, default=2200)
    args = parser.parse_args()
    dark = as_rgb(Image.open(args.dark))
    light = as_rgb(Image.open(args.light))
    if dark.size != light.size:
        raise SystemExit(f"frame size mismatch: {dark.size} vs {light.size}")
    out = Path(args.out)
    out.parent.mkdir(parents=True, exist_ok=True)
    dark.save(
        out,
        "WEBP",
        save_all=True,
        append_images=[light],
        duration=max(args.duration_ms, 800),
        loop=0,
        quality=82,
        method=6,
    )
    print(f"wrote {out.name} {dark.size[0]}x{dark.size[1]}")


if __name__ == "__main__":
    main()
