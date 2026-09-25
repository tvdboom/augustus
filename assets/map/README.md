# Local map data

`provinces.geojson` and `provinces_label.geojson` come from the [Digital Atlas of the Roman Empire map](https://github.com/barionleg/roman-empire/tree/dbbce0d90f2c53a45c2eefe7877aa701fbf3dce6/data). Its 53 shapes include the eleven numbered regions of Roman Italy. `scripts/build-map.py` gives those regions their [Augustan names](https://etc.usf.edu/maps/pages/2000/2074.htm) in the generated atlas.

The builder displays only the first part of names joined by `et` (for example, `Lucania`), while leaving other multiword names intact. It also divides the source's combined `Creta et Cyrene` shape into separate `Creta` and `Cyrenaica` provinces. The source GeoJSON retains its original names and geometry.

`world-land.geojson` is [Natural Earth 1:50m land](https://github.com/nvkelso/natural-earth-vector/blob/ca96624a56bd078437bca8184e78163e5039ad19/geojson/ne_50m_land.geojson), a public domain dataset. It supplies coastline and land beyond the mapped Roman regions. That land is backdrop only: it has no province interaction.

`world-rivers.geojson` is [Natural Earth 1:50m rivers and lake centerlines](https://github.com/nvkelso/natural-earth-vector/blob/ca96624a56bd078437bca8184e78163e5039ad19/geojson/ne_50m_rivers_lake_centerlines.geojson), also public domain. `scripts/build-terrain.py` clips, smooths, and aligns selected principal rivers to the painted coastline in `terrain-rivers.json`. It adds three decorative distributaries in the Nile delta, completes the Elbe estuary, and omits the Jordan River. Province borders and rivers appear above the continuous land-cover texture and its mountain relief. The Dead Sea uses a widened, connected silhouette guided by [Natural Earth 1:50m lake geometry](https://github.com/nvkelso/natural-earth-vector/blob/ca96624a56bd078437bca8184e78163e5039ad19/geojson/ne_50m_lakes.geojson) and shares the ocean's continuously warped water currents. Terrain is decorative and does not change provinces or movement rules.

The build trims backdrop land near province coastlines where the two sources disagree, including around small islands. This prevents land-colored slivers from appearing in the sea while retaining neutral inland terrain.

`atlas.json` is generated from those three source files with:

```sh
uv run --no-project --with shapely --with mapbox-earcut --with pyproj python scripts/build-map.py
uv run --no-project --with pillow python scripts/build-coast-gradient.py
uv run --no-project --with shapely python scripts/build-terrain.py
```

The generated file stores pretriangulated geometry so the game does no map conversion while drawing.
Playable province triangles also clip the continuous Natural Earth land-cover texture. The terrain source, geographic extent, and regeneration command are documented in [`../images/map/README.md`](../images/map/README.md). Ownership uses translucent color over terrain; non-playable land retains its neutral backdrop.
The builder removes separate land pieces smaller than Malta (about 316 km²) from both province and backdrop geometry, while retaining Malta itself despite differences between the source coastlines.
Small-island source outlines are retained separately as `marker_land` for fitting wonder art; only the trimmed `land` meshes are painted.
