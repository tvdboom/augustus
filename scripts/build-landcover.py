"""Import continuous, georeferenced terrain for the playable Augustus map.

Run: uv run --no-project --with pillow --with numpy --with scipy python scripts/build-landcover.py
Natural Earth I shaded relief is public domain. Only this regional crop ships;
the reviewed global source is cached under target/terrain-source.
"""

from hashlib import sha256
from io import BytesIO
import json
from pathlib import Path
from urllib.request import urlopen
from zipfile import ZipFile

import numpy as np
from PIL import Image
from scipy.ndimage import distance_transform_edt


ROOT = Path(__file__).resolve().parents[1]
SOURCE_URL = "https://naturalearth.s3.amazonaws.com/10m_raster/NE1_HR_LC_SR.zip"
SOURCE_SHA256 = "d107ebd0cca189f15a42ead57c24a2f936f9b5da97b742a510c5d716e1f65180"
CACHE = ROOT / "target/terrain-source/NE1_HR_LC_SR.zip"
OUTPUT = ROOT / "assets/images/map"
# Pixel-edge bounds, west/south/east/north. Keep TERRAIN_TILE_BOUNDS in map.rs aligned.
BOUNDS = (-12, 22, 52, 59)
PIXELS_PER_DEGREE = 60


def source_archive():
    if not CACHE.exists():
        CACHE.parent.mkdir(parents=True, exist_ok=True)
        print("Downloading Natural Earth I shaded relief...", flush=True)
        with urlopen(SOURCE_URL, timeout=60) as response, CACHE.open("wb") as output:
            while chunk := response.read(1024 * 1024):
                output.write(chunk)
    digest = sha256(CACHE.read_bytes()).hexdigest()
    if digest != SOURCE_SHA256:
        raise ValueError(f"Natural Earth archive checksum mismatch: {digest}")
    return ZipFile(CACHE)


def main():
    west, south, east, north = BOUNDS
    atlas = json.loads((ROOT / "assets/map/atlas.json").read_text(encoding="utf-8"))
    for province in atlas["provinces"]:
        for part in province["parts"]:
            if not all(west < lon < east and south < lat < north for lon, lat in part["v"]):
                raise ValueError(f"Terrain extent does not cover {province['name']}")

    # This trusted, hash-pinned 21600x10800 raster exceeds Pillow's default
    # pixel limit. Its world file places pixel centers half a pixel inward.
    Image.MAX_IMAGE_PIXELS = 250_000_000
    with source_archive() as archive:
        with Image.open(BytesIO(archive.read("NE1_HR_LC_SR.tif"))) as source:
            if source.size != (21600, 10800):
                raise ValueError(f"Unexpected source raster dimensions: {source.size}")
            terrain = source.crop((
                (west + 180) * PIXELS_PER_DEGREE,
                (90 - north) * PIXELS_PER_DEGREE,
                (east + 180) * PIXELS_PER_DEGREE,
                (90 - south) * PIXELS_PER_DEGREE,
            )).convert("RGB")

    rgb = np.asarray(terrain).copy()
    # The no-water source uses an off-white ocean. Extend adjacent land colors
    # into this unused area so small differences between the raster's coast and
    # Augustus province geometry cannot expose white seams or blank islands.
    # Actual coastlines/holes remain clipped by the original province triangles.
    empty = (rgb.min(axis=2) >= 249) & (np.ptp(rgb, axis=2) <= 2)
    nearest = distance_transform_edt(empty, return_distances=False, return_indices=True)
    rgb[empty] = rgb[nearest[0][empty], nearest[1][empty]]

    # Match the warm, earthy mountain relief without changing land-cover detail.
    rgb = rgb.astype(np.float32) / 255
    luminance = (rgb * np.array([0.2126, 0.7152, 0.0722], dtype=np.float32)).sum(axis=2)[..., None]
    rgb = (luminance + (rgb - luminance) * 1.20) * 1.12 - 0.20
    rgb *= np.array([1.025, 1.0, 0.955], dtype=np.float32)
    terrain = Image.fromarray(np.clip(rgb * 255, 0, 255).round().astype(np.uint8))
    # Four native-resolution tiles fit egui/WebGL's default 2048 texture limit.
    # Pixel-edge UVs meet at lon 20 / lat 40.5, without losing source detail.
    # A one-pixel gutter supplies neighboring samples to bilinear filtering.
    OUTPUT.mkdir(parents=True, exist_ok=True)
    tile_width, tile_height = terrain.width // 2, terrain.height // 2
    padded = Image.fromarray(np.pad(np.asarray(terrain), ((1, 1), (1, 1), (0, 0)), mode="edge"))
    for name, column, row in [("nw", 0, 0), ("ne", 1, 0), ("sw", 0, 1), ("se", 1, 1)]:
        tile = padded.crop((
            column * tile_width, row * tile_height,
            (column + 1) * tile_width + 2, (row + 1) * tile_height + 2,
        ))
        path = OUTPUT / f"terrain-landcover-{name}.png"
        tile.save(path, optimize=True)
        print(f"Saved {path.relative_to(ROOT)}: {tile.width}x{tile.height}")


if __name__ == "__main__":
    main()
