<div align="center">

# Augustus
### Roman provincial strategy for desktop and browser

<br><br>
[![Play](https://gist.githubusercontent.com/cxmeel/0dbc95191f239b631c3874f4ccf114e2/raw/play.svg)](https://tvdboom.itch.io/augustus)
<br><br>
</div>


## Gameplay

The campaign advances monthly. Hover values and actions for detailed effects, prerequisites, and costs.

### Provinces and geography

The map contains **54 provinces**, including eight cities with unique building options. Area and
terrain shape capacity, production, travel, and combat. Provinces can be independent, vassalized,
or directly owned; ownership grants production and policy controls. Visible sea crossings connect
routes for trade, movement, migration, and political distance.

### Population and happiness

Each province contains **Nobles, Citizens, Plebeians, and Slaves**, with separate populations and
happiness. Births, deaths, and class changes update them monthly. Area, terrain, cities, and buildings
provide comfortable capacity; overcrowding lowers happiness and births without a hard population cap.
Your civilians and troops share Food proportionally. Shortages lower happiness and births, cause
civilian famine deaths, and weaken military Morale.

### Migration

Unhappy free populations migrate automatically to neighboring provinces, favoring space, happiness,
and cities. Overcrowding increases departures; Migration Focus changes arrivals and departures.
Relations affect destinations, and hostility blocks routes. Slaves do not migrate freely.
Migration and class changes preserve total population.

### Resources, production, and storage

| Resource | Main role |
| --- | --- |
| **Food** | Feeds civilians and troops; shortages damage population and Morale |
| **Metal** | Equips recruits and pays part of construction costs |
| **Stone** | Builds and upgrades provincial improvements and wonders |
| **Sestertius** | Pays wages, trade, diplomacy, spies, and bribes; earned through taxes, tribute, and exchange |
| **Influence** | Funds politics; earned from Nobles, buildings, wonders, political rank, and vassals |

Plebeians and Slaves produce goods, with each worker allocated once by geographic potential and
Resource Focus. Slaves have higher baseline productivity. Goods enter **global player stockpiles**;
provincial storage buildings expand capacity. Overflow is discarded after monthly settlement,
immediate transfers, and ownership changes. Sestertius and Influence have no storage cap.
Only Citizens and Plebeians pay taxes; Nobles and Slaves are exempt.

### Governance and policies

Open **Governance** from the left menu to set nationwide policies:

- **Food Rations:** balance consumption against happiness, births, and mortality.
- **Slave Labor:** balance productivity against Slave happiness and mortality.
- **Army Wages:** trade treasury costs against military Morale; incomplete payment weakens support.

Domestic edicts cover all directly owned provinces, including new acquisitions; wages cover all your
troops. Vassals keep their policies. **Resource Focus** and **Migration Focus** remain province-local.

### Buildings and construction

Use **Buildings** for four countryside improvements: Granary (Food storage), Warehouse (Metal and
Stone storage), Road (army travel speed and trade routes), and Aqueduct (population capacity).
Cities add eight buildings: Forum, Baths, Market, Temple, Arena, Walls, Academy, and Foundry.
Academies add Influence and Citizen happiness; Foundries increase Metal output. The smaller cards
fit the usual panel without scrolling when there is no wonder. Each province has **one construction slot**, separate from recruitment.
Stone and Metal are paid upfront; cancellation gives no refund. Upgrades have no fixed cap: benefits
grow linearly, costs and time exponentially. Capture preserves progress.

### Wonders

Seven historical sites allow wonders, with at most one per province. Asia retains the Colossus of
Rhodes; Achaia retains the Temple of Zeus at Olympia. Unbuilt wonders are hidden. Active projects
show an icon with their construction animation when zoomed in; completed wonders show an icon
with the final artwork. Completion grants an Influence
reward and ongoing Influence to the direct owner. Assigned Slaves speed construction with diminishing
returns, still need Food, and stop ordinary production. Capture preserves progress but resets workers.

### Diplomacy and control

**Relation** measures friendship; **Control** measures political power. Independent provinces split
100 Control between local authorities and players. Gifts, Sestertius, Influence, recurring programs, trade,
and occupation improve your position; interference undermines rivals. Distance raises political
costs. Owned provinces leave Control competition, but enemy agitation and funded unrest lower happiness.

### Vassals and integration

At **51 Independent Control with a unique lead**, you may vassalize a province. Its starting Vassal
Control equals your previous share minus 50. Vassals grant Influence; tribute adds Sestertius at the cost
of goodwill. They do not supply ordinary production or wonder income. Poor relations erode Control;
diplomacy and garrisons support it. At zero they become independent. At **100 Control**, optionally
integrate a vassal or independent province into direct ownership; relations affect initial happiness.

### Trade agreements

Open the nationwide **Trade** panel from the left menu to propose monthly or one-time exchanges.
Player deals need both parties' consent; switch to the recipient's view to accept. NPC terms reflect
needs, surplus, scarcity, relations, and budget; one-time offers have worse terms. Previews show
routes and transport losses. Roads improve efficiency; hostile transit can interrupt deliveries.

Monthly deals suspend on failure, resume when viable, and cancel after three consecutive failures.
Established NPC deals can scale both sides equally when at least 80% can be supplied; new deals
require full supply. Recurring NPC trade improves Relation and can raise Independent Control up to
40; one-time exchanges never grant Control. Cancel NPC routes immediately for a Relation penalty,
or give **six months' notice** to avoid it. Player agreements end immediately without political effects.

### Open market

Buy or sell Food, Metal, and Stone immediately for Sestertius in **Trade**. Larger transactions receive
worse unit prices, so smaller exchanges give better returns. Purchases require Sestertius and storage.
Market exchanges grant no Relation or Control.

### Espionage and scandals

Spy networks uncover player misconduct or hidden NPC opportunities, costing deployment and upkeep
and risking detection. Evidence has a target, severity, and expiry. Use it to blackmail NPCs for
Control or favorable trade, or expose rivals to undermine their Senate support and consular office.
Exposure and blackmail consume evidence.

### Political ranks, Senate, and victory

Advance through **Quaestor → Aedile → Praetor → Censor → Consul → Augustus** by paying Influence
and attracting loyal senators. Promotions are immediate, with at most one per player per month.
In two-player games, the five promotions require **100/180/280/400/800 Influence** and
**5/12/22/40/60 supporters**. Support requirements scale down with player count; Augustus always
requires at least 51. There is no election calendar or nomination auction.

The Senate has 100 persistent senators, initially neutral gray, split into five labeled factions:
Aristocrats, Merchants, Provincials, Populares, and Military. Supporters use their player's color.
Every month, faction preferences and individual priorities compare all players' economy, welfare,
provinces, political standing and military career. Hover chamber sections to inspect the reasons.
Faction outreach fades over six months; Sestertius bribes lease one senator for six months, with an
escalating price and ten-senator cap. Exposed real scandals damage relevant factions for a year,
cancel the target's bribery leases, and can cause senators to switch sides or return to neutral.

Two Consuls serve at most, for **24 months**, then automatically become Proconsuls. All departing
Consuls must wait **12 months** and meet the full cost and support requirements to return.
Three monthly reviews below 60% of appointment support force resignation; active scandals reduce
this to one review. Augustus requires an active Consul seat and wins immediately.

Clicking **Rome** opens the Senate. The protected capital permits no ordinary province actions,
trade or espionage, and begins with **50 defending cohorts**. **Conquering Rome wins immediately**,
regardless of the conqueror's political rank or Senate support.

### Military recruitment and upkeep

Recruit cohorts in owned provinces using actual Plebeians or Citizens and upfront Metal. Eleven
troop types cover infantry, archers, cavalry, specialist mounts, and artillery; special units require
local traditions. Each province has one recruitment slot. Surviving soldiers consume Food and Sestertius
wages each month. Casualties are permanent; disbanding in owned territory returns only survivors to
their original class. Training and Morale affect combat, while battle-earned Renown advances a
military career separate from political office. Defeated NPC defenders do not automatically regenerate.

### Movement and conquest

Select cohorts, a destination, and optional waypoints in **Military** to preview the route and arrival
time. Speed depends on the slowest unit, province size, terrain, and roads; access is checked at each
crossing. Hostile arrivals stop for battle. Defeating independent defenders establishes occupation,
allowing later Control gains; victory in an enemy-owned province transfers ownership. Peaceful
military presence does not create occupation.

### Battle plans and combat

Save a **Battle Plan** with primary, secondary, and flank unit types, flank slots, and a tactic.
Plans lock when battle begins; deployment uses available cohorts, terrain width, and reserves.
Unit matchups, tactic counters and composition, flanking, support protection, terrain, forts,
Training, and Morale determine simultaneous combat rounds. Reserves replace routed troops.
Pre-battle assessments show known hostile forces but keep enemy tactics hidden.

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
