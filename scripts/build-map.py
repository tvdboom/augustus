"""Build the static Augustus map meshes from the source GeoJSON.

Run with:
    uv run --no-project --with shapely --with mapbox-earcut --with pyproj python scripts/build-map.py

Province source: Digital Atlas of the Roman Empire, via
https://github.com/barionleg/roman-empire/tree/dbbce0d90f2c53a45c2eefe7877aa701fbf3dce6/data
Land source: Natural Earth 1:50m land (public domain), via
https://github.com/nvkelso/natural-earth-vector/tree/ca96624a56bd078437bca8184e78163e5039ad19/geojson
"""

import json
from pathlib import Path

import mapbox_earcut
import numpy as np
from pyproj import Geod
from shapely.geometry import Point, box, shape
from shapely.ops import unary_union


ROOT = Path(__file__).resolve().parents[1]
MAP = ROOT / "assets" / "map"
GEOD = Geod(ellps="WGS84")
MIN_ISLAND_AREA_KM2 = 316  # Approximate area of Malta, including its smaller islands.
MALTA = Point(14.45, 35.91)
REGIONS = {
    "I": "Latium",
    "II": "Apulia",
    "III": "Lucania",
    "IV": "Samnium",
    "V": "Picenum",
    "VI": "Umbria",
    "VII": "Etruria",
    "VIII": "Aemilia",
    "IX": "Liguria",
    "X": "Venetia",
    "XI": "Transpadana",
}
CRETA_CYRENE = "Creta et Cyrene"


def mesh(polygon):
    rings = [polygon.exterior, *polygon.interiors]
    vertices = []
    ends = []
    for ring in rings:
        # GeoJSON closes each ring by repeating the first vertex; earcut does not need it.
        vertices.extend([round(x, 5), round(y, 5)] for x, y in list(ring.coords)[:-1])
        ends.append(len(vertices))
    coordinates = np.asarray(vertices, dtype=np.float64)
    triangles = mapbox_earcut.triangulate_float64(coordinates, np.asarray(ends, dtype=np.uint32))
    return {"v": vertices, "r": ends, "t": triangles.tolist()}


def polygons(geometry):
    if geometry.geom_type == "Polygon":
        yield geometry
    elif geometry.geom_type in ("MultiPolygon", "GeometryCollection"):
        for part in geometry.geoms:
            yield from polygons(part)


def keep_land_part(part):
    # The two source atlases draw Malta at different sizes, so keep its main
    # island explicitly while applying the same real-world cutoff elsewhere.
    area_km2 = abs(GEOD.geometry_area_perimeter(part)[0]) / 1_000_000
    return area_km2 >= MIN_ISLAND_AREA_KM2 or part.covers(MALTA)


def main():
    source = json.loads((MAP / "provinces.geojson").read_text(encoding="utf-8"))
    label_source = json.loads((MAP / "provinces_label.geojson").read_text(encoding="utf-8"))
    labels = {feature["properties"]["name"]: feature["geometry"]["coordinates"] for feature in label_source["features"]}

    provinces = []
    province_shapes = []
    for feature in source["features"]:
        original = feature["properties"]["name"]
        geometry = shape(feature["geometry"])
        parts = [part for part in polygons(geometry) if keep_land_part(part)]
        if original == CRETA_CYRENE:
            # The source combines mainland Cyrenaica with Crete. Its islands
            # lie north of the gap at 34° N, so assign each retained polygon
            # to one independently selectable province.
            cyrenaica_parts = [part for part in parts if part.bounds[3] < 34]
            creta_parts = [part for part in parts if part.bounds[1] > 34]
            assert cyrenaica_parts and creta_parts
            assert len(cyrenaica_parts) + len(creta_parts) == len(parts)
            groups = [
                ("Cyrenaica", cyrenaica_parts, labels[original]),
                ("Creta", creta_parts, None),
            ]
        else:
            full_name = REGIONS.get(original, original)
            name = full_name.split(" et ", 1)[0]
            groups = [(name, parts, labels[original])]

        for name, province_parts, label in groups:
            visible_geometry = unary_union(province_parts)
            province_shapes.append(visible_geometry)
            if label is None or not visible_geometry.covers(Point(label)):
                label = list(visible_geometry.representative_point().coords[0])
            provinces.append({
                "name": name,
                "short": name,
                "label": [round(value, 5) for value in label],
                "bounds": [round(value, 5) for value in visible_geometry.bounds],
                "parts": [mesh(part) for part in province_parts],
            })

    land_source = json.loads((MAP / "world-land.geojson").read_text(encoding="utf-8"))
    frame = box(-35, 5, 70, 70)
    land_shapes = []
    for feature in land_source["features"]:
        geometry = shape(feature["geometry"])
        if not geometry.intersects(frame):
            continue
        for part in polygons(geometry.intersection(frame).simplify(0.006, preserve_topology=True)):
            if keep_land_part(part):
                land_shapes.append(part)

    # The province and backdrop sources use different coastlines. Where they
    # disagree, the backdrop otherwise peeks through as pale land-colored
    # slivers in the sea (notably off Baetica, Aquitania, and Germania).
    # Trim only the backdrop close to both a province and the shoreline;
    # neutral land farther inland remains visible.
    province_footprint = unary_union(province_shapes)
    backdrop = unary_union(land_shapes)
    marker_land = [
        mesh(part)
        for part in land_shapes
        if part.area < 0.2 and part.intersects(province_footprint)
    ]
    backdrop_only = backdrop.difference(province_footprint)
    coastal_slivers = (
        backdrop_only
        .intersection(province_footprint.buffer(0.35))
        .intersection(backdrop.boundary.buffer(0.35))
    )
    # Natural Earth fills the Guadalquivir estuary farther out than the DARE
    # Baetica shore. Its separate backdrop polygon must be removed completely;
    # the distance-based trim otherwise leaves a pale strip in the water.
    guadalquivir = next(
        part for part in polygons(backdrop_only) if part.covers(Point(-5.93, 37.16))
    )
    coastal_slivers = coastal_slivers.union(guadalquivir)

    # The same trim mistakes the southern edge of Mauretania Tingitana for a
    # shoreline near Rabat and cuts a water notch into the Moroccan mainland.
    # Preserve that one connected piece of the Natural Earth coast.
    rabat_mainland = next(
        part for part in polygons(coastal_slivers) if part.covers(Point(-6.8, 33.95))
    )
    coastal_slivers = coastal_slivers.difference(rabat_mainland)
    land = []
    for part in land_shapes:
        # Apply the same coastline cleanup to islands. Their two source
        # outlines can differ enough to expose pale wedges beside a province.
        cleaned = part.difference(coastal_slivers)
        for trimmed in polygons(cleaned):
            if keep_land_part(trimmed):
                land.append(mesh(trimmed))

    atlas = {"provinces": provinces, "land": land, "marker_land": marker_land}
    output = MAP / "atlas.json"
    output.write_text(json.dumps(atlas, ensure_ascii=False, separators=(",", ":")), encoding="utf-8")
    print(f"{len(provinces)} named regions, {len(land)} land shapes, {output.stat().st_size:,} bytes")


if __name__ == "__main__":
    main()
