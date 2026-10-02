# Battle and elephant recruitment sound sources

The following sounds are used under CC0 1.0 (public domain dedication):

- `battle-infantry.ogg` and `battle-ambience.ogg`: sword/knife contacts by Vehicle (Jan Schupke), [Fantasy Weapons and Apparel SFX Library](https://opengameart.org/content/fantasy-weapons-and-apparel-sfx-library). Downmixed, normalized, and layered into a loop.
- `battle-archers.ogg`: Bow.wav by artisticdude, submitted by Ogrebane, [Battle Sound Effects](https://opengameart.org/content/battle-sound-effects). Downmixed and normalized.
- `battle-elephants.ogg` and `recruit-elephants.ogg`: trumpet and growl by D.jones, [Elephant Trumpets Growls.flac](https://freesound.org/people/D.jones/sounds/527845/). Two-second public Ogg preview downmixed, faded and normalized; the quieter recruitment cue matches the other recruitment effects.

License: https://creativecommons.org/publicdomain/zero/1.0/

Horse, camel and siege cues reuse the game's existing recruitment effects.
`scripts/build-battle-audio.py` reproduces the four imported battle assets and the elephant recruitment cue. Use `--elephants-only` to rebuild just the two elephant cues. An optional source directory supplies `apparel.zip`, `weapons.zip` and the new `elephant-trumpet.ogg` preview for an offline rebuild.
