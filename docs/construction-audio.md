# Construction sound

`assets/audio/construction.ogg` uses five hand-struck wooden mallet taps for
building and wonder construction feedback. The existing construction playback
path applies the shared volume and mute settings.

- Recording: [Wooden mallet](https://freesound.org/people/leo153/sounds/535629/)
  by leo153, published September 18, 2020. The creator describes tapping a board
  with a wooden mallet.
- License: [CC0 1.0](https://creativecommons.org/publicdomain/zero/1.0/).
- Download used: [high-quality MP3 preview](https://cdn.freesound.org/previews/535/535629_2535988-hq.mp3).
- Edit: excerpt from 5.28 to 7.08 seconds (1.80 seconds), mono, 48 kHz.
- Fades: 120 ms fade in and 200 ms fade out, baked into the audio so they also
  work in WebAssembly without a playback timer.
- Cleanup: 70 Hz high-pass and 7.5 kHz low-pass filters; gain of 0.744036734
  gives a peak near -8 dBFS before Vorbis encoding.
- Format: Ogg Vorbis, quality 5. Source and license are also identified in the
  file's metadata.

To reproduce with FFmpeg, download the linked preview as `wooden-mallet.mp3`
and run from the repository root:

```sh
ffmpeg -y -ss 5.28 -i wooden-mallet.mp3 -t 1.8 -ac 1 -ar 48000 -af "highpass=f=70,lowpass=f=7500,afade=t=in:st=0:d=0.12,afade=t=out:st=1.6:d=0.2,volume=0.744036734" -c:a libvorbis -q:a 5 -metadata "title=Augustus construction - wooden mallet" -metadata "artist=leo153" -metadata "comment=Wooden mallet (Freesound 535629), CC0 1.0; excerpt and fades for Augustus" assets/audio/construction.ogg
just assets
```
