"""Rebuild CC0 battle sounds. Requires numpy and soundfile; run from repository root."""
import io
from pathlib import Path
import urllib.request
import zipfile
import time
import sys

import numpy as np
import soundfile as sf

ROOT = Path(__file__).resolve().parents[1]
OUTPUT = ROOT / "assets" / "audio"
RATE = 44100


def fetch(url):
    if len(sys.argv) > 1:
        filenames = {"weapons-apparel.zip": "apparel.zip", "battle_sound_effects_0.zip": "weapons.zip", "819668_754558-lq.ogg": "elephant.ogg"}
        return (Path(sys.argv[1]) / filenames[url.rsplit("/", 1)[-1]]).read_bytes()
    request = urllib.request.Request(url, headers={"User-Agent": "Augustus asset build"})
    for attempt in range(3):
        try:
            with urllib.request.urlopen(request, timeout=30) as response:
                return response.read()
        except OSError:
            if attempt == 2:
                raise
            time.sleep(1)


def decode(data):
    samples, rate = sf.read(io.BytesIO(data), always_2d=True)
    samples = samples.mean(axis=1)
    if rate != RATE:
        samples = np.interp(np.arange(round(len(samples) * RATE / rate)) * rate / RATE,
                            np.arange(len(samples)), samples)
    return samples


def save(name, samples, peak=0.65):
    samples = np.asarray(samples, dtype=float)
    samples *= peak / max(float(np.max(np.abs(samples))), 0.001)
    fade = min(440, len(samples) // 4)
    samples[:fade] *= np.linspace(0, 1, fade)
    samples[-fade:] *= np.linspace(1, 0, fade)
    # Small writes also avoid a libvorbis stack overflow on Windows for long buffers.
    with sf.SoundFile(OUTPUT / f"{name}.ogg", "w", samplerate=RATE, channels=1,
                      format="OGG", subtype="VORBIS") as output:
        for offset in range(0, len(samples), 4096):
            output.write(samples[offset:offset + 4096])


def main():
    weapons = zipfile.ZipFile(io.BytesIO(fetch("https://opengameart.org/sites/default/files/weapons-apparel.zip")))
    clashes = [decode(weapons.read(f"sfx/sword-knife-clash-{i:02}.wav")) for i in range(1, 9)]
    save("battle-infantry", clashes[0][:RATE * 2])
    # A deterministic, seamless bed of overlapping shield/blade contacts.
    rng = np.random.default_rng(31)
    bed = np.zeros(RATE * 16)
    for i in range(52):
        sample = clashes[i % len(clashes)]
        offset = int(rng.integers(0, len(bed)))
        indices = (offset + np.arange(len(sample))) % len(bed)
        np.add.at(bed, indices, sample * rng.uniform(0.12, 0.32))
    save("battle-ambience", bed, peak=0.45)
    bows = zipfile.ZipFile(io.BytesIO(fetch("https://opengameart.org/sites/default/files/battle_sound_effects_0.zip")))
    save("battle-archers", decode(bows.read("battle_sound_effects/Bow.wav")))
    elephant = decode(fetch("https://cdn.freesound.org/previews/819/819668_754558-lq.ogg"))
    save("battle-elephants", elephant, peak=0.55)
    for name in ("battle-ambience", "battle-infantry", "battle-archers", "battle-elephants"):
        data, rate = sf.read(OUTPUT / f"{name}.ogg")
        print(f"{name}: {len(data) / rate:.2f}s, peak {np.max(np.abs(data)):.3f}")


if __name__ == "__main__":
    main()
