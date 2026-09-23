# Local map data

`provinces.geojson` and `provinces_label.geojson` come from the [Digital Atlas of the Roman Empire map](https://github.com/barionleg/roman-empire/tree/dbbce0d90f2c53a45c2eefe7877aa701fbf3dce6/data). Its 53 shapes include the eleven numbered regions of Roman Italy. `scripts/build-map.py` gives those regions their [Augustan names](https://etc.usf.edu/maps/pages/2000/2074.htm) in the generated atlas.

`world-land.geojson` is [Natural Earth 1:50m land](https://github.com/nvkelso/natural-earth-vector/blob/ca96624a56bd078437bca8184e78163e5039ad19/geojson/ne_50m_land.geojson), a public domain dataset. It supplies coastline and land beyond the mapped Roman regions. That land is backdrop only: it has no province interaction.

The build trims backdrop land near province coastlines where the two sources disagree, including around small islands. This prevents land-colored slivers from appearing in the sea while retaining neutral inland terrain.

`atlas.json` is generated from those three source files with:

```sh
uv run --no-project --with shapely --with mapbox-earcut --with pyproj python scripts/build-map.py
uv run --no-project --with pillow python scripts/build-coast-gradient.py
```

The generated file stores pretriangulated geometry so the game does no map conversion while drawing.
The builder removes separate land pieces smaller than Malta (about 316 km²) from both province and backdrop geometry, while retaining Malta itself despite differences between the source coastlines.
Small-island source outlines are retained separately as `marker_land` for fitting wonder art; only the trimmed `land` meshes are painted.
