#!/usr/bin/env python3
"""Build multi-size Windows .ico from the Dark-Blood mark raster.

Writes:
  assets/icons/perci-darkblood.ico

Optional:
  --desktop  also copy to Desktop\\Perci-DarkBlood-Icon.ico for legacy shortcuts
"""
from __future__ import annotations

import argparse
import shutil
from pathlib import Path

from PIL import Image

ROOT = Path(__file__).resolve().parents[1]
SRC = ROOT / "assets" / "icons" / "perci-darkblood-mark.jpg"
OUT = ROOT / "assets" / "icons" / "perci-darkblood.ico"
SIZES = [(16, 16), (24, 24), (32, 32), (48, 48), (64, 64), (128, 128), (256, 256)]


def build(src: Path = SRC, out: Path = OUT) -> Path:
    if not src.is_file():
        raise SystemExit(f"missing mark raster: {src}")
    img = Image.open(src).convert("RGBA")
    w, h = img.size
    side = max(w, h)
    # Near-black void matching dark-blood theme
    canvas = Image.new("RGBA", (side, side), (5, 2, 3, 255))
    canvas.paste(img, ((side - w) // 2, (side - h) // 2), img)
    canvas_256 = canvas.resize((256, 256), Image.Resampling.LANCZOS)
    out.parent.mkdir(parents=True, exist_ok=True)
    canvas_256.save(out, format="ICO", sizes=SIZES)
    return out


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--desktop",
        action="store_true",
        help="Also write Desktop/Perci-DarkBlood-Icon.ico for legacy shortcuts",
    )
    args = parser.parse_args()
    path = build()
    print(f"wrote {path} ({path.stat().st_size} bytes)")
    if args.desktop:
        desktop = Path.home() / "OneDrive" / "Desktop" / "Perci-DarkBlood-Icon.ico"
        if not desktop.parent.is_dir():
            desktop = Path.home() / "Desktop" / "Perci-DarkBlood-Icon.ico"
        shutil.copy2(path, desktop)
        print(f"wrote {desktop}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
