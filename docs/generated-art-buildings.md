# Generated building artwork

Generated with the built-in imagegen tool for the reduced four-countryside/eight-city catalog. The four building outputs are square isometric illustrations with real transparent alpha; the village banner below is an opaque landscape. The existing `assets/images/buildings/granary.png` remains the Warehouse illustration; existing Aqueduct, Forum, Market, Temple, Arena, Academy and Foundry art is retained. Building icons are embedded and normalized by `build.rs`; the village banner is embedded by the shared portrait widget.

Visual review: complete isolated subjects, matching warm Roman architectural style, and transparent margins inspected in the generated previews. Panel layout is checked separately.

## Village landscape banner

Output: `assets/images/cities/village-panel-banner.png`

Mode: built-in imagegen, opaque panoramic landscape. Generated at 2172×724. The existing city banner informed the visual style; the source city image remains unchanged.

Used by the Buildings header for provinces whose `has_city` is false. City provinces use the existing city banner; the Overview retains its terrain portrait. Village cottages, fields and olive trees were visually inspected in the generated preview; their central placement supports the panel's shallow crop.

Prompt:

> Use case: historical-scene. Asset type: a wide landscape banner for the Augustus historical strategy game's Buildings panel in a province WITHOUT a city. Primary request: create a new peaceful Roman countryside village landscape, distinctly a small rural settlement. Subject: a modest cluster of about eight simple single-storey cream plaster and rough limestone farm cottages with terracotta tile roofs around a dirt lane, surrounded by cultivated fields, an olive grove, a few cypress trees and low green-gold hills, with hazy distant mountains beneath a soft blue sky. Style: detailed classical hand-painted landscape illustration, warm Mediterranean daylight, natural ochre and sage-green palette, atmospheric perspective and richly painted textures consistent with the game's Roman city banner. Composition: broad horizontal panorama about 3:1, landscape fills the whole opaque canvas edge to edge; the village rooftops and fields are concentrated across the central horizontal band so the scene remains recognizable when center-cropped into a shallow UI banner. Calm, pastoral, modest settlement scale. Avoid monumental buildings, temples, palaces, aqueducts, fortifications, large cities, prominent people, modern objects, text, labels, decorative frames and watermarks.

## Granary

Output: `assets/images/buildings/granary-rural.png`

Prompt:

> Use case: stylized-concept. Asset type: one transparent isometric building illustration for the Augustus historical strategy game. Primary request: create a NEW Roman countryside granary, visibly devoted to food storage, distinctly different from a grand urban warehouse. Subject: a compact rustic Roman horreum with warm limestone walls, a terracotta tiled gable roof, raised ventilated stone floor, wooden double doors open to neat sacks of wheat, several grain sacks and two storage jars on the small foundation. Style: richly painted polished isometric game building icon, clean crisp materials and strong readable silhouette at 80 pixels, warm cream stone, terracotta red, subtle gold details. Camera: three-quarter elevated isometric, building front faces lower right, top-left light. Composition: one complete isolated building centered on a small square stone foundation with transparent margins around all sides; square canvas. Real alpha transparency, no scenery beyond foundation, no people, no text, no labels, no watermark, no border, no sprite sheet.

## Road

Output: `assets/images/buildings/road.png`

Prompt:

> Use case: stylized-concept. Asset type: one transparent isometric building illustration for the Augustus historical strategy game. Primary request: create a NEW a short Roman military road crossing diagonally across a small rectangular foundation, broad fitted pale stone paving slabs, visible layered curb and drainage edges, a single Roman stone milestone beside the road, sparse low grass along the verge. The paved road is the dominant subject, no buildings, no arches, no people or armies. Style: richly painted polished isometric game building icon, clean crisp materials and strong readable silhouette at 80 pixels, warm cream limestone and sandstone, terracotta red where appropriate, restrained gold details. Camera: three-quarter elevated isometric, subject front faces lower right, top-left light. Composition: one complete isolated subject centered on a small stone foundation with generous transparent margins around all sides; square canvas. Actual alpha transparency, no scenery beyond small foundation, no text, no labels, no watermark, no border, no sprite sheet. Match the established warm Roman architectural game illustration style.

## Baths

Output: `assets/images/buildings/baths.png`

Prompt:

> Use case: stylized-concept. Asset type: one transparent isometric building illustration for the Augustus historical strategy game. Primary request: create a NEW a Roman public bathhouse, a compact warm limestone building with terracotta roofs and a clearly visible open bathing courtyard, turquoise rectangular bathing pool, arched colonnade, small domed bathing chamber and a subtle wisp of steam. Keep the building silhouette simple and recognizable at small icon size, no people. Style: richly painted polished isometric game building icon, clean crisp materials and strong readable silhouette at 80 pixels, warm cream limestone and sandstone, terracotta red where appropriate, restrained gold details. Camera: three-quarter elevated isometric, subject front faces lower right, top-left light. Composition: one complete isolated subject centered on a small stone foundation with generous transparent margins around all sides; square canvas. Actual alpha transparency, no scenery beyond small foundation, no text, no labels, no watermark, no border, no sprite sheet. Match the established warm Roman architectural game illustration style.

## Walls

Output: `assets/images/buildings/walls.png`

Prompt:

> Use case: stylized-concept. Asset type: one transparent isometric building illustration for the Augustus historical strategy game. Primary request: create a NEW a short Roman city fortification with two sturdy crenellated warm limestone towers flanking a central stone wall and large arched gate with closed wooden double doors, a small terracotta roof over the gatehouse, a pair of dark red Roman banners. Compact standalone wall section, no city or other buildings behind it, no people. Style: richly painted polished isometric game building icon, clean crisp materials and strong readable silhouette at 80 pixels, warm cream limestone and sandstone, terracotta red where appropriate, restrained gold details. Camera: three-quarter elevated isometric, subject front faces lower right, top-left light. Composition: one complete isolated subject centered on a small stone foundation with generous transparent margins around all sides; square canvas. Actual alpha transparency, no scenery beyond small foundation, no text, no labels, no watermark, no border, no sprite sheet. Match the established warm Roman architectural game illustration style.
