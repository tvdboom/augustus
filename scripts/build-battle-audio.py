"""Rebuild CC0 battle and elephant recruitment sounds. Requires numpy and soundfile."""
import argparse
import io
from pathlib import Path
import urllib.request
import zipfile
import time

import numpy as np
import soundfile as sf

ROOT = Path(__file__).resolve().parents[1]
OUTPUT = ROOT / "assets" / "audio"
RATE = 44100


def fetch(url, source_directory=None):
    if source_directory is not None:
        filenames = {"weapons-apparel.zip": "apparel.zip", "battle_sound_effects_0.zip": "weapons.zip", "527845_11431915-lq.ogg": "elephant-trumpet.ogg"}
        return (source_directory / filenames[url.rsplit("/", 1)[-1]]).read_bytes()
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


def build_battle_sounds(source_directory):
    weapons = zipfile.ZipFile(io.BytesIO(fetch("https://opengameart.org/sites/default/files/weapons-apparel.zip", source_directory)))
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
    bows = zipfile.ZipFile(io.BytesIO(fetch("https://opengameart.org/sites/default/files/battle_sound_effects_0.zip", source_directory)))
    save("battle-archers", decode(bows.read("battle_sound_effects/Bow.wav")))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("source_directory", nargs="?", type=Path,
                        help="read apparel.zip, weapons.zip and elephant-trumpet.ogg locally")
    parser.add_argument("--elephants-only", action="store_true",
                        help="rebuild only the elephant battle and recruitment cues")
    args = parser.parse_args()
    names = []
    if not args.elephants_only:
        build_battle_sounds(args.source_directory)
        names.extend(("battle-ambience", "battle-infantry", "battle-archers"))
    # A short, distinct trumpet and growl, without a long zoo ambience tail.
    elephant = decode(fetch("https://cdn.freesound.org/previews/527/527845_11431915-lq.ogg", args.source_directory))
    save("battle-elephants", elephant.copy(), peak=0.55)
    save("recruit-elephants", elephant, peak=0.32)
    names.extend(("battle-elephants", "recruit-elephants"))
    for name in names:
        data, rate = sf.read(OUTPUT / f"{name}.ogg")
        print(f"{name}: {len(data) / rate:.2f}s, peak {np.max(np.abs(data)):.3f}")


if __name__ == "__main__":
    main()
