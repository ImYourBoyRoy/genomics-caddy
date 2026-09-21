#!/usr/bin/env python3
# ./scripts/lib/make_github_social_preview.py
"""Build the GitHub social-preview PNG (1280×640, under 1 MB).

How to run: python3 ./scripts/lib/make_github_social_preview.py
Inputs: src-tauri/icons/icon.png and Noto Sans on the host.
Outputs: docs/images/github-social-preview.png
Notes: Solid dark canvas so GitHub and dark-mode embeds stay readable.
Never reads app databases or genotype files.
"""

from __future__ import annotations

from pathlib import Path

from PIL import Image, ImageDraw, ImageFont

ROOT = Path(__file__).resolve().parents[2]
ICON = ROOT / "src-tauri" / "icons" / "icon.png"
OUT = ROOT / "docs" / "images" / "github-social-preview.png"
FONT_REGULAR = Path("/usr/share/fonts/truetype/noto/NotoSans-Regular.ttf")
FONT_BOLD = Path("/usr/share/fonts/truetype/noto/NotoSans-Bold.ttf")

WIDTH, HEIGHT = 1280, 640
BG = (8, 11, 20, 255)
TITLE = (248, 250, 252, 255)
SUB = (148, 163, 184, 255)
ACCENT = (45, 212, 191, 255)


def load_font(path: Path, size: int) -> ImageFont.FreeTypeFont:
    if not path.is_file():
        raise SystemExit(f"missing font {path}")
    return ImageFont.truetype(str(path), size=size)


def main() -> None:
    canvas = Image.new("RGBA", (WIDTH, HEIGHT), BG)
    draw = ImageDraw.Draw(canvas)

    icon = Image.open(ICON).convert("RGBA")
    icon_size = 220
    icon = icon.resize((icon_size, icon_size), Image.Resampling.LANCZOS)
    icon_x = 88
    icon_y = (HEIGHT - icon_size) // 2
    canvas.alpha_composite(icon, (icon_x, icon_y))

    text_x = icon_x + icon_size + 56
    title_font = load_font(FONT_BOLD, 72)
    sub_font = load_font(FONT_REGULAR, 28)
    meta_font = load_font(FONT_REGULAR, 22)

    title = "Genomics Caddy"
    subtitle = "Local-first desktop DNA explorer"
    meta = "AncestryDNA  ·  23andMe  ·  MCP  ·  on-device"

    draw.text((text_x, 188), title, font=title_font, fill=TITLE)
    draw.text((text_x, 292), subtitle, font=sub_font, fill=SUB)
    draw.rectangle((text_x, 352, text_x + 72, 356), fill=ACCENT)
    draw.text((text_x, 380), meta, font=meta_font, fill=ACCENT)

    OUT.parent.mkdir(parents=True, exist_ok=True)
    rgb = canvas.convert("RGB")
    rgb.save(OUT, format="PNG", optimize=True)
    size = OUT.stat().st_size
    if size >= 1_000_000:
        raise SystemExit(f"{OUT} is {size} bytes; GitHub social preview must stay under 1 MB")
    print(f"wrote {OUT.relative_to(ROOT)} ({size} bytes)")


if __name__ == "__main__":
    main()
