"""Render optional test egui captures for layout review; requires Pillow and NumPy."""
import json
from pathlib import Path
import sys

import numpy as np
from PIL import Image

source = Path(sys.argv[1])
data = json.loads(source.read_text())
width, height = map(int, data["size"])
canvas = np.zeros((height, width, 4), dtype=np.float64)
canvas[:, :, :3] = (39, 47, 49)
canvas[:, :, 3] = 255
textures = {}
for mesh in data["meshes"]:
    tid = mesh["texture"]
    if tid not in textures:
        textures[tid] = np.asarray(Image.open(source.with_name(f"{source.stem}-texture-{tid}.png")), dtype=float)
    texture = textures[tid]
    th, tw = texture.shape[:2]
    vertices = np.array(mesh["vertices"])
    clip = mesh["clip"]
    for triangle in np.array(mesh["indices"]).reshape(-1, 3):
        v = vertices[triangle]
        x0 = max(0, int(np.floor(v[:, 0].min())), int(np.ceil(clip[0])))
        x1 = min(width, int(np.ceil(v[:, 0].max())), int(np.floor(clip[2])))
        y0 = max(0, int(np.floor(v[:, 1].min())), int(np.ceil(clip[1])))
        y1 = min(height, int(np.ceil(v[:, 1].max())), int(np.floor(clip[3])))
        if x1 <= x0 or y1 <= y0:
            continue
        a, b, c = v[:, :2]
        denominator = (b[1] - c[1]) * (a[0] - c[0]) + (c[0] - b[0]) * (a[1] - c[1])
        if abs(denominator) < 1e-9:
            continue
        ys, xs = np.mgrid[y0:y1, x0:x1]
        xs = xs + 0.5
        ys = ys + 0.5
        wa = ((b[1] - c[1]) * (xs - c[0]) + (c[0] - b[0]) * (ys - c[1])) / denominator
        wb = ((c[1] - a[1]) * (xs - c[0]) + (a[0] - c[0]) * (ys - c[1])) / denominator
        wc = 1 - wa - wb
        mask = (wa >= -1e-7) & (wb >= -1e-7) & (wc >= -1e-7)
        attributes = wa[..., None] * v[0, 2:] + wb[..., None] * v[1, 2:] + wc[..., None] * v[2, 2:]
        tx = np.clip(attributes[..., 0] * tw - 0.5, 0, tw - 1)
        ty = np.clip(attributes[..., 1] * th - 0.5, 0, th - 1)
        ix, iy = tx.astype(int), ty.astype(int)
        fx, fy = (tx - ix)[..., None], (ty - iy)[..., None]
        ix1, iy1 = np.minimum(ix + 1, tw - 1), np.minimum(iy + 1, th - 1)
        sampled = ((1-fx)*(1-fy)*texture[iy,ix] + fx*(1-fy)*texture[iy,ix1] + (1-fx)*fy*texture[iy1,ix] + fx*fy*texture[iy1,ix1])
        color = sampled * attributes[..., 2:] / 255
        destination = canvas[y0:y1, x0:x1]
        destination[mask] = (color + destination * (1 - color[..., 3:4] / 255))[mask]
Image.fromarray(canvas.clip(0, 255).astype(np.uint8), "RGBA").save(source.with_suffix(".png"))
print(source.with_suffix(".png"))
