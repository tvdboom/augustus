# Generated campaign icons

Generated on 2026-09-28 with the built-in imagegen tool, one transparent PNG per semantic icon. Source outputs retain their generated alpha. These are newly painted adaptations using the existing happiness and plebeian assets for palette, material shading and lighting. The original [Imperator diplomacy screenshot](https://forumcontent.paradoxplaza.com/public/437142/client_state.png) was inspected online and supplied as a small-object reference. The official Diplomacy wiki returned HTTP 401. These assets are not extracted game files.

All icons use the existing build.rs premultiplied-alpha normalization to 64×64, then the shared cached campaign texture renderer. No circle, disk or laurel frame is painted around them. Happiness and population-class source art is preserved.

## Shared prompt

Each complete prompt is the shared specification below followed by that icon's subject specification.

Use case: stylized-concept. Asset type: one transparent painted historical-strategy UI icon for Augustus, matching Imperator: Rome's small object icons. Input image 1 is the existing happiness mask, input image 2 the existing plebeian portrait: STYLE REFERENCES ONLY. Input image 3 is an original Imperator diplomacy screenshot: reference the small free-standing painted objects and warm classical palette ONLY. Generate a NEW single object icon, not a screenshot. Closely match the first two references' richly shaded painterly realism, tactile antique materials, warm gold/bronze/ivory, dark brown contact shadows and bright upper-left highlights. Strong simple silhouette, legible at 20-32 pixels. Center the complete object in a square canvas, fill about 80-85 percent of its longest dimension. Actual transparent alpha background. No surrounding circle, no disk, no badge, no laurel wreath, no frame, no square backing, no ambient scenery, no text, no labels, no watermark. Only the isolated subject.

## Subject specifications and saved files

### amount

Saved: `assets/images/icons/amount.png`

Subject: a compact Roman counting tablet / amount symbol formed by THREE solid antique gold ingots standing side by side at ascending heights, broad bevels, realistic rubbed gold texture, dark recessed edges, a very small bronze base connecting the three bars. Straight-on with slight three-quarter depth; clean readable three-bar silhouette. Do not add chart lines or arrows.

### diplomacy

Saved: `assets/images/icons/diplomacy.png`

Subject: an unfurled ivory parchment treaty with curled ends, angled slightly, with a white feather quill and a small deep red wax seal at the lower corner. Painterly tactile aged parchment, bronze nib, crimson wax, natural feather detail. The seal is part of the treaty, never a surrounding badge. Only a few short indistinct ink strokes, no readable writing.

### notifications

Saved: `assets/images/icons/notifications.png`

Subject: one compact antique Roman brass handbell, elegantly flared gold-bronze body, small cast handle at top, visible dark mouth and tiny clapper at bottom, three-quarter perspective. Richly shaded metal with broad warm highlights and crisp dark rim. No notification dot, no exclamation mark, no surrounding frame.

### control

Saved: `assets/images/icons/control.png`

Subject: one upright Roman scutum authority shield, broad rectangular shape with slightly curved sides, dark deep crimson painted wood, a thick worn antique gold rim and large raised bronze central boss, subtle gold wing/thunderbolt relief radiating from boss. Slight three-quarter depth. Strong simple shield silhouette. No weapons behind it, no decorative outer frame.

### relation

Saved: `assets/images/icons/relation.png`

Subject: two realistic hands clasped firmly in a diplomatic handshake, shown isolated from wrists to fingers, one wrist with a simple muted green ancient linen cuff and the other with an ivory linen cuff with a restrained antique gold trim. Warm natural skin, anatomically convincing fingers, detailed painterly shading, three-quarter view. The two forearms enter from left and right, hands centered. No arm bodies, no background, no floating stars or symbols.

### policies

Saved: `assets/images/icons/policies.png`

Subject: one Roman wax writing diptych used for administrative policy: two hinged warm dark walnut tablets, one opened amber-brown wax panel with a few indistinct engraved strokes, the other partly open wooden backing, with a single short bronze stylus lying diagonally across the front. Subtle ivory/gold hinge fittings, richly shaded tactile wood and wax, simple compact silhouette, slight three-quarter view. No people, no manpower or portrait, no readable writing, no surrounding circle or shield.

### construction

Saved: `assets/images/icons/construction.png`

Subject: two crossed ancient Roman artisan tools, one heavy dark iron mason's hammer with a worn wooden handle and one bronze carpenter's adze with a worn wooden handle. Entire tools fit in the canvas. Centered X silhouette, broad tool heads, realistic bronze and iron highlights and wood grain, simple clearly recognizable miniature. No scaffold or building.

### delta

Saved: `assets/images/icons/delta.png`

Subject: a single broad upward-right growth arrow made of solid antique gold, with thick stepped shaft and a clearly recognizable triangular arrowhead, rising diagonally from lower left to upper right. Physical sculpted beveled gold object with dark side faces, scratched antique metal, bright upper-left highlights. No bars behind it, no chart lines, no plus sign, no surrounding circle.

### change

Saved: `assets/images/icons/change.png`

Subject: two small broad gold arrows forming a horizontal exchange pair, the upper arrow pointing right and the lower arrow pointing left, staggered vertically with a clear transparent gap, connected only by their composition. Physical sculpted beveled antique bronze-gold, dark side faces, scratched metal, matching mask's highlights. Clearly readable opposing arrowheads at small sizes. No circle, no rotation ring, no background.

### cancel

Saved: `assets/images/icons/cancel.png`

Subject: one thick diagonal X made from two intersecting solid antique bronze-gold bars. A physical beveled cancellation symbol, scratched gold surfaces, dark side faces, warm upper-left highlight matching the happiness mask. Clean bold silhouette, equally long arms, isolated object. No circle, no disk, no framing ornament.

### confirm

Saved: `assets/images/icons/confirm.png`

Subject: one large check mark cast from thick antique gold with short left stroke and longer ascending right stroke. A physical sculpted acceptance symbol with broad beveled edges, scratched gold surfaces and dark side faces, bright warm upper-left highlights. Clean simple recognizable silhouette, isolated object. No circle, no disk, no framing ornament.

### notice

Saved: `assets/images/icons/notice.png`

Subject: one small antique hourglass as a waiting / notice-period icon, warm dark walnut frame, bronze/gold top and bottom plates, two clear glass chambers, visible golden sand in both chambers and a thin stream joining them. Slight three-quarter view, painterly realism with broad readable gold highlights. Compact simple silhouette, no clock dial, no text, no surrounding frame outside the hourglass itself.

## Verification

- 25 headless layout and interaction checks passed, including original Governance styling, compact policy cards, direct-owner policy changes, individual icon textures, transparent corners and the absence of painted circle frames.
- Desktop (570 px) and compact (380 px) policy previews and an icon comparison sheet were rendered from the actual egui textures and meshes and visually reviewed. Temporary export test wiring was removed; preview artifacts remain in `target/icon-policies-desktop.png`, `target/icon-policies-compact.png` and `target/icon-art-review.png`.
- `just assets` completed with 12 build and asset workers; `cargo check --package augustus --bin augustus -j12` passed. Changed Rust files passed rustfmt checks and `git diff --check` passed.
