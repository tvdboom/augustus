"""Extract the principal rivers for the local Augustus map.

Source: Natural Earth 1:50m rivers and lake centerlines (public domain).
Run with: uv run --no-project --with shapely python scripts/build-terrain.py
"""

import json
import math
from pathlib import Path

from shapely.geometry import LineString, Point, Polygon, box, shape
from shapely.ops import nearest_points, unary_union


ROOT = Path(__file__).resolve().parents[1]
SOURCE = ROOT / "assets/map/world-rivers.geojson"
OUTPUT = ROOT / "assets/map/terrain-rivers.json"
WORLD_LAND = ROOT / "assets/map/world-land.geojson"
ATLAS = ROOT / "assets/map/atlas.json"
EXTENT = box(-18, 17, 52, 61)

# The 50m source uses regional spellings and splits several large rivers into
# separately named reaches. Keep those reaches together as one visual system.
MAJOR = {
    "Nile", "Damietta Branch", "Rosetta Branch", "Danube", "Donau",
    "Rhine", "Rhein", "Rhin", "Euphrates", "Firat", "Al Furat",
    "Tigris", "Dicle", "Shatt al Arab", "Volga", "Dnipro", "Dnepre",
}
SECONDARY = {
    "Ebro", "Po", "Rhône", "Seine", "Loire", "Garonne", "Elbe",
    "Vistula", "Oder", "Dniester", "Don", "Tajo", "Tejo", "Duero",
    "Thames", "Tisza", "Tisa", "Drava", "Daugava", "Niger",
    "Atbara", "Ural", "Severnaya Dvina", "Sukhona",
}

# Smaller historical delta channels make the Nile fan out between its two
# source branches. Their seaward ends are trimmed to the painted coast below.
NILE_DELTA_CHANNELS = [
    [[30.8482, 30.4707], [30.89, 30.63], [30.94, 30.78], [30.90, 30.92], [30.93, 31.05], [30.87, 31.19], [30.85, 31.33], [30.78, 31.46], [30.72, 31.68]],
    [[30.7702, 30.7800], [30.89, 30.89], [30.98, 31.00], [31.01, 31.13], [30.99, 31.27], [31.04, 31.41], [31.02, 31.56], [31.04, 31.69]],
    [[31.2240, 30.8686], [31.29, 31.00], [31.31, 31.13], [31.36, 31.26], [31.33, 31.39], [31.37, 31.52], [31.36, 31.70]],
]

# Natural Earth's Elbe centerline ends at Hamburg, upstream from the painted
# tidal shore. Continue it as a subdued estuary rather than leaving a gap.
ELBE_ESTUARY = [
    [10.0820, 53.4399], [10.005, 53.472], [9.928, 53.499],
    [9.851, 53.536], [9.75, 53.585],
]


def lines(geometry):
    if isinstance(geometry, LineString):
        yield geometry
    elif hasattr(geometry, "geoms"):
        for part in geometry.geoms:
            yield from lines(part)


def painted_land():
    atlas = json.loads(ATLAS.read_text(encoding="utf-8"))
    meshes = atlas["land"] + [part for province in atlas["provinces"] for part in province["parts"]]
    polygons = []
    for mesh in meshes:
        starts = [0] + mesh["r"][:-1]
        rings = [mesh["v"][start:end] for start, end in zip(starts, mesh["r"])]
        if rings:
            polygon = Polygon(rings[0], rings[1:])
            if not polygon.is_valid:
                polygon = polygon.buffer(0)
            if polygon.area > 0:
                polygons.append(polygon)
    return unary_union(polygons)


def world_coast():
    source = json.loads(WORLD_LAND.read_text(encoding="utf-8"))
    outlines = []
    for feature in source["features"]:
        geometry = shape(feature["geometry"])
        polygons = geometry.geoms if hasattr(geometry, "geoms") else [geometry]
        outlines.extend(polygon.exterior for polygon in polygons)
    return unary_union(outlines)


def shore_point(inside, outside, land):
    """Return a point just inside the painted shoreline along a short reach."""
    low, high = inside, outside
    for _ in range(20):
        middle = [(low[0] + high[0]) / 2, (low[1] + high[1]) / 2]
        if land.covers(Point(middle)):
            low = middle
        else:
            high = middle
    dx, dy = outside[0] - inside[0], outside[1] - inside[1]
    length = math.hypot(dx, dy)
    return [low[0] - dx / length * 0.001, low[1] - dy / length * 0.001]


def align_mouth(points, side, land, coast, force=False):
    """End a coastal river at painted land, without a blue tip in the sea."""
    ordered = points if side == 0 else list(reversed(points))
    endpoint = ordered[0]
    origin = Point(endpoint)
    if not force and origin.distance(coast) > 0.04 and origin.distance(land.boundary) > 0.04:
        return False

    if not land.covers(origin):
        for index in range(1, len(ordered)):
            if land.covers(Point(ordered[index])):
                ordered = [shore_point(ordered[index], ordered[index - 1], land)] + ordered[index:]
                break
        else:
            return False
    else:
        neighbor = ordered[1]
        dx, dy = endpoint[0] - neighbor[0], endpoint[1] - neighbor[1]
        length = math.hypot(dx, dy)
        if length == 0:
            return False
        dx, dy = dx / length, dy / length
        previous = endpoint
        target = None
        for step in range(1, 81):
            distance = step * 0.02
            candidate = [endpoint[0] + dx * distance, endpoint[1] + dy * distance]
            beyond = [endpoint[0] + dx * (distance + 0.12), endpoint[1] + dy * (distance + 0.12)]
            if not land.covers(Point(candidate)) and not land.covers(Point(beyond)):
                target = shore_point(previous, candidate, land)
                break
            if land.covers(Point(candidate)):
                previous = candidate
        if target is None:
            # Some estuaries turn sharply. Connect to the nearby painted shore.
            shore = nearest_points(origin, land.boundary)[1]
            if origin.distance(shore) > 0.20:
                return False
            dx, dy = shore.x - endpoint[0], shore.y - endpoint[1]
            length = math.hypot(dx, dy)
            if length == 0:
                return False
            target = [shore.x - dx / length * 0.001, shore.y - dy / length * 0.001]
        ordered.insert(0, target)

    points[:] = ordered if side == 0 else list(reversed(ordered))
    return True


def trim_at_first_shore(points, side, land):
    """Remove any final reach that slips across water and back onto land."""
    downstream = points if side == -1 else list(reversed(points))
    scan_start = len(downstream) - 2
    distance = 0.0
    while scan_start > 0 and distance < 1.8:
        a, b = downstream[scan_start], downstream[scan_start + 1]
        distance += math.hypot(b[0] - a[0], b[1] - a[1])
        scan_start -= 1

    for index in range(scan_start, len(downstream) - 1):
        a, b = downstream[index], downstream[index + 1]
        length = math.hypot(b[0] - a[0], b[1] - a[1])
        steps = max(1, math.ceil(length / 0.008))
        previous = a
        was_land = land.covers(Point(a))
        for step in range(1, steps + 1):
            t = step / steps
            current = [a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t]
            is_land = land.covers(Point(current))
            if was_land and not is_land:
                shore = shore_point(previous, current, land)
                downstream[:] = downstream[:index + 1] + [shore]
                points[:] = downstream if side == -1 else list(reversed(downstream))
                return
            previous, was_land = current, is_land


def smooth(points, passes=2):
    """Chaikin smoothing keeps both endpoints fixed and removes sharp bends."""
    for _ in range(passes):
        rounded = [points[0]]
        for a, b in zip(points, points[1:]):
            rounded.append([a[0] * 0.75 + b[0] * 0.25, a[1] * 0.75 + b[1] * 0.25])
            rounded.append([a[0] * 0.25 + b[0] * 0.75, a[1] * 0.25 + b[1] * 0.75])
        rounded.append(points[-1])
        points = rounded
    return points


def main():
    source = json.loads(SOURCE.read_text(encoding="utf-8"))
    land = painted_land()
    coast = world_coast()
    rivers = []
    aligned = 0
    for feature in source["features"]:
        name = feature["properties"].get("name")
        if name not in MAJOR | SECONDARY:
            continue
        clipped = shape(feature["geometry"]).intersection(EXTENT)
        for line in lines(clipped):
            if line.length < 0.08:
                continue
            points = [list(point) for point in line.simplify(0.015, preserve_topology=False).coords]
            if len(points) >= 2:
                mouth_start = align_mouth(points, 0, land, coast)
                mouth_end = align_mouth(points, -1, land, coast)
                if mouth_start:
                    trim_at_first_shore(points, 0, land)
                if mouth_end:
                    trim_at_first_shore(points, -1, land)
                aligned += int(mouth_start) + int(mouth_end)
                rivers.append({
                    "name": name,
                    "major": name in MAJOR,
                    "mouth_start": mouth_start,
                    "mouth_end": mouth_end,
                    "points": [[round(lon, 4), round(lat, 4)] for lon, lat in smooth(points)],
                })
    for index, channel in enumerate(NILE_DELTA_CHANNELS):
        points = [list(point) for point in channel]
        if not align_mouth(points, -1, land, coast, force=True):
            raise ValueError(f"Nile delta channel {index} misses the painted coast")
        trim_at_first_shore(points, -1, land)
        rivers.append({
            "name": f"Nile delta distributary {index + 1}",
            "major": False,
            "minor": True,
            "mouth_start": False,
            "mouth_end": True,
            "points": [[round(lon, 4), round(lat, 4)] for lon, lat in smooth(points)],
        })
    estuary = [list(point) for point in ELBE_ESTUARY]
    if not align_mouth(estuary, -1, land, coast, force=True):
        raise ValueError("Elbe estuary misses the painted coast")
    trim_at_first_shore(estuary, -1, land)
    rivers.append({
        "name": "Elbe estuary",
        "major": False,
        "mouth_start": False,
        "mouth_end": True,
        "points": [[round(lon, 4), round(lat, 4)] for lon, lat in smooth(estuary)],
    })
    for river in rivers:
        for key, point in (("mouth_start", river["points"][0]), ("mouth_end", river["points"][-1])):
            if river[key] and (not land.covers(Point(point)) or Point(point).distance(land.boundary) > 0.02):
                raise ValueError(f"{river['name']} {key} is not at the painted shoreline")
    OUTPUT.write_text(json.dumps(rivers, separators=(",", ":")) + "\n", encoding="utf-8")
    print(f"Wrote {len(rivers)} river reaches ({aligned} aligned mouths) to {OUTPUT.relative_to(ROOT)}")


if __name__ == "__main__":
    main()
