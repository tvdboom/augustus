# Military implementation audit

This audit maps specification v10 sections 244–368 to the implemented military rules. `src/game/military/` is independent of Bevy, uses stable province/player indices, and exposes transactional population/Metal boundaries. The app adapter in `src/game/campaign.rs` connects those rules to the civilian economy, diplomatic access, notifications, and immediate conquest of enemy-owned provinces.

## Sources and explicit decisions

- The supplied v10 document is the numerical combat baseline. Its values are **Augustus defaults**, not a claim of exact current Imperator: Rome data extraction.
- The official [Paradox development diary on land combat](https://forum.paradoxplaza.com/forum/threads/imperator-development-diary-21-22nd-of-october-2018.1124887/) and [official land-unit wiki](https://imperator.paradoxwikis.com/Land_units) were consulted on 2026-09-27, but returned a JavaScript challenge / HTTP 401. Those unavailable tables could not be independently verified.
- Verified official developer [Cicero patch notes](https://store.steampowered.com/news/posts/?appgroupname=Imperator%3A+Rome&appids=859580&enddate=1572368458) establish that recruitment prices and matchups changed between versions, including mounted-unit prices and the Archer/Heavy Infantry matchup. The [official game announcements](https://steamcommunity.com/app/859580/announcements/) also document later Archer maneuver/morale changes. This implementation therefore keeps every stat, matchup, fit, and price configurable rather than presenting one version's numbers as universal.
- The user resolved the duplicated Catapult rows: offense 0.80, defense 0.25, speed 1.5, maneuver 0, morale damage taken 1.30, manpower damage taken 1.50; siege power 2; first complete tactic-fit row. The accidental Onager reference does not add a twelfth unit.
- The user explicitly requested immediate ownership transfer after victory in an enemy-owned province. This supersedes v10 §328's deferred conquest rule. Independent provinces still use occupation and Control rather than immediate annexation.
- Existing provinces contain roughly tens of aggregate pops. Recruitment manpower is one tenth of the illustrative v10 counts. Combat manpower damage is a fraction of target nominal cohort size, preserving unit effectiveness across this change in population scale. This makes drafting several cohorts a substantial economic choice instead of making infantry recruitment impossible in small provinces.
- Small forces use centered occupied frontage. Without this automatic fallback, literal outer-flank-first deployment strands a lone infantry cohort beyond maneuver range of another small force with a different plan. Full-width formations preserve the specified flank-first preferences, and existing surviving combatants never reposition between rounds.

## Implemented requirements

The left-menu Military panel is a national army overview. Rows group forces by province and owner, with your own force first and other currently visible forces in that location on separate player-colored rows. Unit-type icons always show cohort counts, alongside the selected tactic, manpower-weighted morale, and average training. Engaged cohorts remain in their province force; each marching order has a separate row using its current origin, next destination, and captured tactic. Foreign tactics remain hidden, and dated spy reports are excluded from this current-army ledger. Clicking any row focuses that province and opens Military → Army; recruitment remains exclusively in the province's Recruit units tab.

| Specification | Implementation and verification |
| --- | --- |
| 244–247, 294 | Province/owner forces, 11 individually identified unit types, no persistent Army model. `MilitaryWorld::all_units` visits province, movement, and battle ownership exactly once. |
| 248–254, 268–269, 289 | One military recruitment slot separate from construction; direct ownership, local class and explicit regional tag checks; atomic drafting/equipment payment; progress, cancellation, temporary draft penalties. Failed recruitment leaves population and stock unchanged. |
| 255–260 | Food upkeep includes all unit locations and scales with surviving strength. Proportional shortages lower morale. No recurring Metal expense, reinforcement, healing, or slave/noble recruiting. Owned-province disbanding returns only survivors to the original class. |
| 261–265 | Selected-unit movement and diplomatic access, with peaceful coexistence. Hostility while coexisting starts an encounter. Fresh arrivals join an existing coalition while preserving locked plans. A third mutually hostile coalition waits for the current engagement to resolve, then enters the next encounter. |
| 266–267 | Explicit initial NPC setups by province; important provinces have more defenders. Starting defenders are independent of current civilian population. Destroyed NPC defenders never automatically regenerate. As clarified by the user, NPC forces defend their own territory and issue no offensive movement orders. |
| 270–273 | Unit-specific Training and Morale, supplied monthly Training, bounded post-battle experience only for surviving participants, victory morale, gradual peaceful morale recovery without manpower recovery. |
| 274–281 | Explicit hostile occupation and diminishing strength-based Control; peaceful access gives zero occupation pressure. Military casualties reduce garrison power. New occupation begins generating Control on the next political tick. |
| 282–287 | Renown promotions at 0 / 150 / 400 / 900, separate from political rank. Morale and garrison bonuses are configured; the campaign feeds military rank/strength/victories into the Senate profile. |
| 288–293 | Province military UI groups owners and labels ownership/access/hostility/occupation; illustrated striped rows display manpower, Training/Morale meters and upkeep, draft costs/time/tags, ongoing recruitment, movement, and explanatory hover text. Recruitment, battle and rank changes are exposed as events. |
| 296–305 | Six tactics, the complete counter cycle, actual surviving-cohort composition fit, casualty character, 11×11 asymmetric matchups, siege field matchups, and centralized unit stats. |
| 306–316 | Terrain widths, deterministic center/flank/support deployment, reserves, protected versus exposed support, maneuver reach, terrain modifiers, capped forts and siege suppression, Training and Morale combat scaling. |
| 317–326 | Seeded authoritative random rolls; damage for both sides computed from immutable pre-round state; four rounds per month, six-month maximum; routing, voluntary retreat after one month, legal deterministic retreat, permanent losses, bounded Training and strength-scaled Renown. |
| 327–329 | Battle outcomes identify the old territorial owner and principal surviving winner. Independent occupation and direct conquest are separate. Conquest requires hostility against the territorial owner; combat between neutral guests cannot capture their host. Friendly owners share frontage but retain their own plans, tactics, and military ranks. Mutually hostile candidates never form a defending coalition; the territorial force has first priority. |
| 330–338 | Transient movement orders snapshot a BattlePlan; slowest troop controls speed; square-root normalized province area, terrain and roads determine edge time; deterministic legal Dijkstra routes; one observable tick minimum per edge; revalidation at every crossing; hostile arrivals stop for battle. The player can add, remove and reorder waypoints; the UI previews every crossing and its total time, and the exact approved route is validated before troops leave. |
| 339–346 | Eleven generated transparent 4×4 sheets; distinct icon, Idle, Move, Combat rows with four frames each. The build pipeline normalizes source sheets; lazy texture decoding follows the existing embedded map-image architecture. Troops hide below close zoom, represent dominant/significant composition, retain owner-colored rings/banners, separate owner clusters, interpolate movement and show active deployed combatants using opposing combat animations. |
| 347–348 | Battle panel shows terrain, width, fortifications, both sides' locked tactics and fit, deployed image rows, reserves and unit facts. Pre-battle estimates use visible composition, matchups, rank, Training, Morale, terrain and forts; the enemy tactic remains explicitly unknown, and the assessment is qualitative. Expandable Scouting displays hostile stationed and engaged cohorts with public rank and condition; neutral guests are excluded. |
| 349–351 | All numerical combat/movement/roster configuration lives in `MilitaryConfig`; no naval combat, prisoners, permanent Army objects, manual soldier placement, manpower healing, monthly Metal upkeep, or long-term siege subsystem is introduced. |
| 352–368 | Saved primary/secondary/flank/flank-size/tactic defaults; deterministic health/experience/morale/id tie-breaks; terrain-clamped flank size, actual fallback preview, immutable engaged plans, movement overrides editable in transit, reserve replacement, and surviving-composition tactic fit. |

## Initial recruitment and logistics calibration

Costs are Augustus Metal costs, **not Imperator gold values**. The user's economy uses global Metal for equipment and Food for upkeep; reproducing gold prices literally would be a category error. Infantry equipment takes several months of a small province's productive surplus, while expensive animal troops trade powerful cohort combat for much greater food demand.

| Unit | Drafted class/count | Metal | Months | Full Food/month | Offense | Defense | Speed | Maneuver |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| Light Infantry | Plebeians 10 | 12 | 2 | 5 | 0.80 | 0.85 | 2.5 | 1 |
| Heavy Infantry | Plebeians 10 | 40 | 3 | 8 | 1.15 | 1.25 | 2.5 | 1 |
| Archers | Plebeians 8 | 16 | 2 | 6 | 1.00 | 0.75 | 2.5 | 2 |
| Light Cavalry | Citizens 5 | 28 | 3 | 12 | 1.00 | 0.90 | 4.0 | 3 |
| Heavy Cavalry | Citizens 4 | 48 | 4 | 14 | 1.30 | 1.20 | 3.5 | 2 |
| Horse Archers | Citizens 5 | 40 | 3 | 12 | 1.15 | 0.90 | 4.0 | 5 |
| War Chariots | Citizens 3 | 24 | 3 | 12 | 1.00 | 0.90 | 2.5 | 1 |
| War Camels | Citizens 4 | 36 | 3 | 10 | 1.05 | 0.95 | 3.5 | 4 |
| War Elephants | Citizens 2 | 100 | 5 | 30 | 1.60 | 1.50 | 2.5 | 0 |
| Ballista | Plebeians 3 | 60 | 4 | 6 | 0.70 | 0.30 | 2.0 | 0 |
| Catapult | Plebeians 3 | 80 | 5 | 6 | 0.80 | 0.25 | 1.5 | 0 |

Drafting Plebeians immediately removes productive workers, while drafting Citizens reduces the taxable upper-class base. Two Heavy Infantry cost 80 Metal, 20 productive civilians, and 16 Food/month; a single War Elephant costs 100 Metal, two handlers, and 30 Food/month. Siege artillery has poor field defense, occupies support/frontage, and slows an entire mixed movement order. Roads cannot lower a province's crossing cost below half of its unroaded cost.

## Verification and remaining release checks

The coordinated `just test` run passes all 166 tests, including 18 military-domain tests,
13 cross-system military tests, hostile-only scouting, map-sprite placement/assets, and the
actual military-panel command bridge. The bridge regression covers paid recruitment,
cancellation, monthly completion, survivor disbanding, saved plans, movement, engagement locks
and retreat. Real monthly tests verify private recruitment/combat/access/garrison notifications.

All eleven unit sheets have valid transparent animation cells. Close-zoom troop clusters reserve
label space and avoid city, wonder and earlier troop markers. The real egui roster/active-battle
layout tests pass at 380/500-point widths and scales 0.85/1.0. Native visual review remains pending;
these checks cannot establish the quality of animations on screen.

All section 292 notification categories now reach affected players' history and clickable
navigation. Capture requires war against the territorial owner; neutral hosts are protected.
Mutually hostile defenders do not form an accidental alliance. A third mutually hostile coalition
waits for the existing engagement to resolve before the next encounter starts.

A 64-seed flat-terrain smoke simulation for each of five equal-Metal budgets produced these illustrative results: four Light Cavalry defeated seven Archers in all runs (mean 3.17 months); five Horse Archers defeated two War Elephants in all runs (one month); five Heavy Infantry defeated two War Elephants in all runs (two months); four Ballista lost to six Heavy Infantry in all runs (one month). Three Heavy Infantry attacking ten Light Infantry reached the six-month withdrawal limit in every run: the surviving formations could not exploit distant gaps with their low Maneuver. These checks demonstrate counters, cost tradeoffs, artillery vulnerability and the need for mixed mobile forces; they do not prove global balance across terrain, rank, supplies, or tactics.

The shared [verification record](verification.md) reports final command results. Long-term
competitive balance remains unproven; configuration values are an initial playable calibration.
