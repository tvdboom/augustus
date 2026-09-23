"""Rasterize a soft shallow-water tint from the rendered map geometry.

Run after build-map.py with:
    uv run --no-project --with pillow python scripts/build-coast-gradient.py
"""

import json
from pathlib import Path

from PIL import Image, ImageDraw, ImageFilter


ROOT = Path(__file__).resolve().parents[1]
ATLAS = ROOT / "assets" / "map" / "atlas.json"
OUTPUT = ROOT / "assets" / "images" / "map" / "coastal-gradient.png"
WIDTH, HEIGHT = 2048, 1664  # Within egui's 2048-pixel texture limit.
WEST, SOUTH, EAST, NORTH = -18.0, 17.0, 52.0, 61.0


def screen_point(point):
    longitude, latitude = point
    return (
        (longitude - WEST) * (WIDTH - 1) / (EAST - WEST),
        (NORTH - latitude) * (HEIGHT - 1) / (NORTH - SOUTH),
    )


def draw_mesh(draw, mesh):
    start = 0
    for ring_index, end in enumerate(mesh["r"]):
        ring = mesh["v"][start:end]
        if len(ring) >= 3:
            draw.polygon([screen_point(point) for point in ring], fill=255 if ring_index == 0 else 0)
        start = end


def main():
    atlas = json.loads(ATLAS.read_text(encoding="utf-8"))
    land_mask = Image.new("L", (WIDTH, HEIGHT))
    draw = ImageDraw.Draw(land_mask)
    for mesh in atlas["land"]:
        draw_mesh(draw, mesh)
    for province in atlas["provinces"]:
        for mesh in province["parts"]:
            draw_mesh(draw, mesh)

    # Blurring the land silhouette produces a continuous fade on the water
    # side. Land is rendered over this texture, hiding its inland half.
    blurred = land_mask.filter(ImageFilter.GaussianBlur(radius=22))
    alpha = blurred.point(lambda value: min(140, round(value * 1.05)))
    gradient = Image.new("RGBA", (WIDTH, HEIGHT), (102, 178, 205, 0))
    gradient.putalpha(alpha)
    OUTPUT.parent.mkdir(parents=True, exist_ok=True)
    gradient.save(OUTPUT, optimize=True)
    print(f"wrote {OUTPUT.relative_to(ROOT)} ({WIDTH}x{HEIGHT})")


if __name__ == "__main__":
    main()
