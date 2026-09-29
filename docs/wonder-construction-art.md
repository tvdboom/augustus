# Wonder construction animation sources

Generated with the built-in `image_gen` tool on 2026-09-29 using transparent backgrounds. Each previous four-frame sheet was the reference for its replacement architecture plate. No external image-generation model or CLI was used.

## Sources

`assets/images/wonders/construction/` now contains single static architecture plates for `great_pyramid`, `stonehenge`, `zeus_temple`, `argeads_palace`, `rhodes_colossus`, `segovia_aqueduct`, `pont_du_gard`, `mausoleum_halicar`, `oracle_dodona`, `pergamon_acropolis`. `assets/images/wonders/construction-workers.png` contains twelve foreground activity poses (four columns, three rows).

## Assembly

`scripts/wonder-construction.rs`, called by `build.rs`, crops and resamples each plate once and copies the identical pixels into every 384 × 384 frame. It composites the workers at three fixed foreground work stations with staggered foot baselines. The resulting runtime PNGs have twelve frames in a 1536 × 1152 atlas. Building, scaffolding, crane, foundations and materials never move or rescale within the loop. Workers share one scale; only their poses and tools animate. The map samples at eight frames per second and shows either the overview icon or opaque artwork, without a map progress bar. Tests compare every pixel outside the three worker rectangles across every frame of all ten atlases, check transparent edges, and require twelve distinct activity poses.

## Architecture prompt

The following prompt was used for each asset, substituting its name:

Use case: precise-object-edit. Asset type: single transparent high-detail static construction plate for Augustus historical strategy game. Input image: EDIT TARGET, an old four-frame construction sprite sheet of {wonder name}. Replace this sheet with ONE single large centered rendering of exactly the same recognizable unfinished wonder and its timber scaffolding, ropes, crane, building materials and foundation, matching the top-left old frame's architecture, construction stage, camera, warm colors and painted miniature style. Make finely detailed stone masonry, timber joints, ropes and unfinished edges, crisp at zoom. Remove ALL people/workers entirely: this is the static architecture layer; workers are added separately in code. Render only one monument, not a sheet and not multiple views. Full square canvas with comfortable transparent margins around every edge, no cropping, no scenery, no UI, no progress bar, no symbols or text. Preserve the existing wonder's architectural identity and half-built details faithfully, no modern machinery, no new structures. Genuine transparent alpha background.

## Worker prompt

Use case: historical-scene. Asset type: transparent game animation strip for tiny ancient construction workers, to composite beside fixed wonder architecture. Create exactly TWELVE equal cells arranged FOUR columns by THREE rows, row-major looping animation. In EACH cell show the same three separate tiny full-body ancient laborers from elevated isometric three-quarter camera: left laborer chiseling a small pale limestone block with hammer, middle laborer carrying a small stone walking in place, right laborer pulling a rope beside a small timber winch. Consistent body sizes and planted foot baselines across all frames; subtle continuous work cycle across twelve steps, tools and limbs move smoothly, no camera movement. Warm ivory short tunics, brown leather sandals, bronze skin, dark hair, finely detailed hand-painted miniature style matching Augustus wonders. No buildings, no platforms, no background scenery, no floating numbers or text, no grid borders. Ample genuinely transparent margins in all twelve cells. All three workers must stay fully inside their equal cell, the three figures are separated by generous horizontal gaps; include small props but no connecting scenery. This is ONLY the animated foreground activity layer. Genuine transparent alpha PNG.
