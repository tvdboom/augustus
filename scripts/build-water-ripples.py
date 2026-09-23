"""Recolor a sourced, seamless wave heightmap for the close-zoom sea layer."""

from hashlib import sha256
from io import BytesIO
from pathlib import Path
from urllib.request import urlopen
from zipfile import ZipFile

from PIL import Image


ROOT = Path(__file__).resolve().parents[1]
SOURCE_URL = "https://opengameart.org/sites/default/files/waves5.zip"
SOURCE_SHA256 = "49f58ebd009ee5820252642d86bcd3f449006528bebd0aa3ed376837233fa2fb"
OUTPUT = ROOT / "assets" / "images" / "map" / "ripples.png"


def main() -> None:
    archive = urlopen(SOURCE_URL, timeout=30).read()
    if sha256(archive).hexdigest() != SOURCE_SHA256:
        raise ValueError("Downloaded wave archive does not match the reviewed source")
    with ZipFile(BytesIO(archive)) as files:
        heightmap = Image.open(BytesIO(files.read("waves5/000.png"))).convert("L")

    # The heightmap supplies the wave shapes. Only its colors and opacity are
    # adapted so the map's own sea blue still controls the water color.
    pixels = []
    for height in heightmap.tobytes():
        if height >= 124:
            pixels.append((195, 226, 232, min(150, (height - 124) * 2)))
        else:
            pixels.append((20, 58, 86, min(65, (124 - height) * 2)))
    ripples = Image.new("RGBA", heightmap.size)
    ripples.putdata(pixels)
    ripples.save(OUTPUT, optimize=True)
    print(OUTPUT)


if __name__ == "__main__":
    main()
