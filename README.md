<div align="center">

# Augustus
### Roman provincial strategy for desktop and browser

<br><br>
[![Play](https://gist.githubusercontent.com/cxmeel/0dbc95191f239b631c3874f4ccf114e2/raw/play.svg)](https://tvdboom.itch.io/augustus)
<br><br>
</div>

The repository contains a playable **local campaign for one to four players**, alongside the retained
menu and offline lobby preview. Local players share a computer and switch viewpoints using the player
list. The published itch.io build may lag behind the working tree.

Augustus combines provincial government, trade, political influence, espionage, and military forces
on the existing 54-province historical map. The current campaign runs locally. The lobby/Supabase
boundary does **not** provide a networked campaign or server-authoritative turns.

## Play locally

Run `just run`, choose the local game, choose one to four players and a color, and start. Players begin
in provinces with the existing assigned colors; independent provinces have configured local defenders.
Switch between players using the lower-right player list to make decisions and accept trade proposals.

| Control | Action |
| --- | --- |
| Click a province or city | Open its contextual overview and campaign tabs |
| Left-side Governance / Military / Trade / Politics banners | Open the corresponding campaign panel |
| Province selector inside a panel | Inspect another province |
| Mouse wheel / map drag | Zoom and pan |
| Space or the date/pause control | Pause or resume the monthly simulation |
| Ctrl + Left / Right, or speed − / + | Change simulation speed |
| Escape | Close the active panel; otherwise open/close the game menu |
| Enter in the game menu | Resume |
| Hover statistics, policy choices, Senate blocs, and actions | Read formulas, costs, prerequisites, and explanations |

Pause while reviewing several players' decisions. Everyone uses the same monthly clock and Senate
schedule. Music, mute, volume, and click feedback remain in the shared circular audio controls.
Settings retains the existing audio options.

## Population and resources

Provinces contain **Nobles, Citizens, Plebeians, and Slaves**, each with a separate count and happiness.
Comfortable capacity comes from area, terrain, cities, and buildings. Overcrowding lowers happiness
and encourages migration; it never directly deletes population. Happiness affects births, while
natural mortality continues independently. Automatic class changes preserve population.

Plebeians and Slaves provide productive labor. Each worker is allocated once between Food, Metal,
and Stone according to geographic potential and the selected resource focus. Slaves have higher
baseline productivity. Light/Normal/Harsh labor changes output, happiness, and mortality. Each owned
province has its own food, labor, focus, and migration policies.

Food, Metal, and Stone are **global player stockpiles**. Coin and Influence have no physical storage
limit. Local storage buildings expand global capacity. Excess physical goods are discarded after
monthly production, trade, and consumption. Taxes come from Nobles, Citizens, and Plebeians.

One-time exchanges and ownership changes discard storage overflow immediately. Above 150% comfortable
population capacity, births receive additional gradual suppression even if buildings keep happiness
high; population is never forcibly truncated to its capacity.

Civilian and military food requests share a proportional supply ratio. Partial shortage lowers births
and happiness and causes proportional famine deaths. NPC provinces use abstract internal provisioning:
their deficits affect trade demand and scarcity without hidden stockpiles or automatic NPC famine.

Influence comes from domestic Nobles, relevant city buildings, completed directly owned wonders,
political rank, and controlled vassals. The province overview explains production allocation, capacity,
food requests, last month's supply, demographic changes, and happiness modifiers. Geographic
specialization makes policies, buildings, ownership, and trade meaningful.

## Construction

Each province has **one construction slot**, separate from its recruitment slot. Standard buildings
include Granary, Warehouse, Aqueduct, Farm/Irrigation, Mine, Quarry, Road, Fort, Armory, and Stone Yard.
The eight existing city provinces additionally allow Forum, Baths, Temple, Arena, Urban Market, and
City Walls. Building levels have no fixed gameplay maximum: benefits are linear while costs and
construction time grow exponentially.

Buildings cost Stone and Metal, paid in full when work starts. Cancellation has no refund.
Projects and progress remain attached to their province after capture.

The ten canonical wonder sites retain their existing geography and completed artwork. Ay Khanum was
removed at the user's request because its site lay outside the playable atlas. A province
can complete at most one wonder. Wonders grant a one-time completion Influence reward and passive
Influence to the current direct owner. Assigned local Slaves accelerate work with diminishing returns,
remain part of population and food demand, and stop ordinary resource production while assigned.
A change of ownership resets their assignment.

Construction is available in the province panel's **Buildings** tab. Hover an illustration or Build
button for effects and prerequisites; hover the Slave slider for the exact acceleration thresholds.
The current tiers retain the document's 25/50/100/200 assigned Slaves; optional rescaling awaits a user decision.

## Diplomacy, trade, and spies

**Relation** measures friendship; **Control** measures political power. Independent provinces have
Local Control plus every player's share totaling 100. Gifts, Influence, recurring programs, trade,
interference, and hostile occupation affect these separate values.

At **51 Independent Control and a unique lead**, a player may voluntarily vassalize a province.
Starting Vassal Control equals the previous Independent Control minus 50; rival shares disappear
and relations remain. Low relation erodes Vassal Control. Diplomacy and real military garrisons can
support it. Tribute trades income against goodwill. Vassals provide Influence according to Control
but do not contribute ordinary production or wonder income to the overlord's stores. Integration at
100 Control is optional; at zero a vassal becomes independent. Independent provinces can also be
directly integrated at 100 Control.

Player trades require both players' acceptance. NPC trades evaluate need, surplus, scarcity, relation,
cash, and recurring budget. Proposals show the legal route and delivered bundles after transport loss.
Switch to the invited player's view and open **Trade** to accept or cancel a proposal; the acceptance
hover shows its current route and net delivery. One-time acceptance requires both parties' resources.
Roads improve route efficiency. Hostile transit, including declared NPC war, can interrupt routes.
Visible sea crossings are shared by trade, military movement, political distance, and migration.

Monthly deals can suspend and resume; three consecutive failures cancel them. Established NPC deals
with at least 80% supply scale both sides equally; new agreements require the full promised amount.
Recurring NPC trade improves relation and may slowly raise independent Control up to the trade
ceiling of 40. One-time exchange never generates Control.

Spy networks pay deployment and upkeep, risk detection, and discover evidence of real player
misconduct or hidden NPC opportunities. Evidence supports NPC blackmail, Senate scandal exposure,
and evidence-backed removal motions. Detection precedes discovery. Evidence has an owner, target,
severity, and lifetime; exposing or consuming it changes actual state.

## Senate and victory

The retained political ladder is:

**Quaestor → Aedile → Praetor → Censor → Consul → Augustus**

Aedile is purchased with Influence. Praetor, **Censor**, Consul, and Augustus use the global Senate
nomination and voting system. A 12-month Nomination Year uses sealed monthly Influence commitments;
its winner receives a 12-month Campaign Year. Tied leaders enter additional sealed rounds.
Committed nomination Influence is spent even when the nomination is lost.

The 100 Senate seats belong to Aristocrats, Merchants, Provincials, Populares, and Military blocs.
Hover a bloc to inspect its current factors. Campaigning, endorsements, opposition, bribery, and
actual scandals affect support. The chamber distinguishes committed YES, committed NO, and undecided
senators. Only undecided votes are rolled, and the saved result animates without rerolling.
**51 YES votes** pass a motion.

At most two Consuls serve, with 48-month terms. Expired or removed Consuls become Proconsuls and must
regain an active seat before seeking Augustus. Winning the Augustus ballot wins the campaign.
The older README's Imperium Maius, eight-city, mutiny, and treasury/happiness victory checklist has
been superseded by this Senate system.

## Military and battle plans

Forces belong to an owner and province; there are no persistent Army objects. Recruitment drafts
actual Plebeians or Citizens and pays Metal immediately. The eleven types are Light/Heavy Infantry,
Archers, Light/Heavy Cavalry, Horse Archers, War Chariots, War Camels, War Elephants, Ballista, and
Catapult. Special troops require explicit provincial traditions.

Cohorts retain permanent manpower losses. Training and Morale affect combat, and military Renown
forms a career separate from political rank. Food upkeep scales with surviving strength. There is
no monthly Metal upkeep, manpower healing, or automatic regeneration of destroyed NPC defenders.
Disbanding in owned territory returns only survivors to their original class.

Save a **Battle Plan** with a primary type, secondary type, flank type, flank slots per side, and tactic.
Deployment uses real available units, reserves, terrain width, and deterministic fallbacks. Plans lock
on engagement. Tactic fit uses surviving composition. Explicit unit matchups, tactic counters,
support protection, maneuver, terrain, forts, Training, and Morale affect simultaneous round damage.
Pre-battle assessments are qualitative and preserve unknown enemy tactics.

Select cohorts and their destination in the military panel. Optional ordered waypoints let you choose
the route; its crossings and arrival time are previewed before departure. Scouting shows known hostile
cohorts, including defenders already fighting, while enemy tactics remain unknown. Movement uses the slowest unit, province
size, terrain, and roads, and revalidates access at each crossing. Hostile arrivals stop for battle.
Independent victories establish occupation before Control accumulation; victory over an enemy-owned
province transfers ownership. Close zoom shows owner-colored representative animated units; distant
zoom hides individual troops.

Combat values are configurable Augustus defaults inspired by the supplied Imperator-style design.
They are **not a verified complete reproduction of one particular Imperator: Rome patch**.

## Implementation status

The canonical [design](docs/AUGUSTUS-game-design.md) and
[numbered implementation audit](docs/implementation-audit.md) record scope, later overrides,
verification, and honest limits. The [README feature audit](docs/README-feature-audit.md) traces
each gameplay claim to its player controls, application code and tests. Detailed audits are available for
[economy/construction](docs/implementation-economy.md),
[politics/espionage/notifications](docs/implementation-politics.md), and
[military/combat](docs/implementation-military.md).

The campaign has one authoritative in-process state. Online campaign synchronization, server validation
of hidden bids, save/load, and autonomous strategic AI for rival player empires are not implemented.
NPCs choose economic focus, evaluate trade, provide espionage opportunities, and defend their own
territory with configured troops, as requested. They do not launch offensive wars. Numerical invariant
tests and combat smoke simulations do not establish long-term multiplayer balance.

## Development

Rust, Cargo, and `just` are required. KTX-Software 4.x is needed for the external texture pipeline.
Keep public Supabase configuration in `src/platform/config.rs`; never ship a service-role key.

```text
just run               # cargo run --package augustus --bin augustus
just assets            # regenerate runtime assets after source asset changes
just check
just test              # cargo test --package augustus --all-targets
just lint
just fmt-check
just assets-check
just check-wasm
just packaging-check
just package-native
just package-web
```

`AUGUSTUS_JOBS` and `AUGUSTUS_ASSET_JOBS` override the bounded worker defaults. Keep package scripts
and path-safety checks aligned when output paths change. Source art/audio/fonts live in `assets/`;
generated disk-loaded assets live in `assets-runtime/`. Embedded map artwork follows the established
source-asset embedding and lazy normalization path.

| Folder | Responsibility |
| --- | --- |
| `src/app.rs` | States, shared resources, system registration |
| `src/menu/` | Reference menu, forms, wallpaper, audio |
| `src/ui/` | HUD, parchment panels, tooltips, notifications, circular audio |
| `tests/unit/` | Rust tests grouped into one file per topic, including private-state unit regressions |
| `tests/scripts/` | Packaging and output-path checks |
| `src/game/economy/` | Population, resources, construction, markets, trade |
| `src/game/politics/` | Relations, control, espionage, ranks, Senate |
| `src/game/military/` | Recruitment, units, movement, formations, combat |
| `src/game/campaign*.rs` | Monthly cross-system integration and campaign events |
| `src/map/` | Canonical geography, crossings, cities, wonders, rendering |
| `src/multiplayer/` | Offline lobby and future Supabase boundary |
| `supabase/schema.sql` | Initial lobby/player-card schema |

[AGENTS.md](AGENTS.md) retains contributor instructions. Preserve the reference menu's dimensions,
placement, font, form-card structure, footer, and circular audio controls when adding features.

All Rust test bodies are outside `src`; see [test organization](tests/README.md).
`just ci` runs formatting, Clippy, tests, asset checks, wasm compilation and packaging checks.
