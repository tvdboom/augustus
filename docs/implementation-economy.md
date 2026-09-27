# Economy implementation audit

This audit covers the population, resource, construction, and trade portions of the v10 gameplay specification, including later construction and military overrides. The canonical design is `docs/AUGUSTUS-game-design.md`. The authoritative engine is `src/game/economy/`; the application adapter is maintained separately in `src/game/campaign.rs`.

## Implemented rules

| Specification | Implementation | Checks |
| --- | --- | --- |
| 4–8, 15: population, capacity, births, deaths, class changes | `population.rs`: four aggregate classes; terrain/area/city/building capacity; capped overcrowding penalty; happiness-dependent births; separate normal deaths; gradual conservative class changes | Low happiness stops births without directly adding mortality; class promotions use one pre-conversion snapshot; no hard population cap |
| 9–10: food and famine | `simulation.rs`: player-global production/trade precedes proportional civilian and army consumption; fractional shortages affect happiness, births, and famine deaths | Two provinces plus military receive exactly the same supply ratio; partial shortage gives proportional deaths |
| 11–14: migration | `population.rs`: adjacent destinations, own/vassal/friendly/neutral weights, hostile access rejection, destination attraction, simultaneous class deltas | World population conserved; slaves cannot migrate; Closed reduces but does not zero flows; migrants stay home if no destination exists |
| 16–24, 72: economy and labor | Global Food/Metal/Stone, uncapped Coin/Influence, global storage buildings, partitioned labor, local production focus and slave policies, taxes, scarce domestic Influence | No worker counted fully in multiple sectors; zero potential receives zero labor; storage recomputed after ownership loss; stocks remain finite/nonnegative |
| 48–51: player trade and routes | `trade.rs`: shortest permitted graph route including supplied sea edges, symmetric transport loss, player proposals and explicit counterparty acceptance, one-time/monthly agreements | Bilateral preflight prevents partial payments, one-time cannot execute twice, enemy transit rejects route, blocked routes suspend then cancel |
| 52–65: NPC economy and prices | Monthly production/need/scarcity, focus AI, finite Coin treasury and recurring spending budget, cumulative export/import/budget reservations, relation margins, one-time premium, reduced established-route fulfillment | Oversized new proposals rejected; existing routes at 80%+ scale both sides equally; actual outgoing supply reserved; infinite/negative amounts rejected |
| 66–70: trade politics | Per-province/per-player monthly delivered value aggregation, relation cap, foreign-trade share, passive independent-control pressure, no vassal loyalty variable | One-time control always zero; monthly gain capped before politics enforces the total 40-control ceiling |
| 114–123, 138–144: buildings | `buildings.rs`: one shared construction slot, entire rounded Stone/Metal cost paid up front, no cancellation refund, unlimited levels with exponential costs/slower times and linear effects, city prerequisites | Failed start does not debit either resource, queued projects persist on capture, ordinary completion updates storage, city-only buildings reject rural provinces |
| 124–137: wonders | Existing map supplies site IDs and names; site-only construction; one completed wonder per province; no levels; diminishing slave acceleration; assigned slaves excluded from ordinary production but retained in population/food; completion and passive Influence only | Rewards paid exactly once to owner at completion; passive reward follows later owner; capture preserves progress and resets assignment; no refund; labor excluded for entire completion month |
| 139, 166: ordering/Influence | Explicit `begin_month` / `finish_month` boundaries let campaign diplomacy use the same wallet transaction; construction completes after production and before passive Influence/storage | Same-month production/trade/consumption resolves before physical stockpile clamping; rank and vassal Influence are owned by politics, not duplicated here |
| 98: blackmail trade terms | `trade_ratio_by_player` consumes political espionage's temporary required-value-ratio modifier | Ordinary multiplier 1; politics provides 0.75 favorable terms and expiration |

The province UI in `src/ui/campaign_economy.rs` provides the current population/capacity and class happiness; local labor/output; food requests and last supply; stockpile capacity; policy buttons with formulas on hover; construction prices, effects, progress, estimated completion, cancellation, and wonder slave assignment; NPC demand bands, valuations and treasury; live trade route and delivered-value preview; agreement status, acceptance, suspension explanations, and cancellation. It inherits the existing campaign parchment frame and button styling.

## Clarified and intentionally bounded behavior

- One-time exchanges discard physical overflow immediately for both participating players. Ownership changes immediately recalculate storage and discard any excess. Only the monthly production/trade/consumption transaction temporarily permits over-cap reserves; unchanged ownership synchronization does not interrupt that transaction.
- In addition to the happiness birth multiplier, a configurable space multiplier is `min(1, 1.5 / population_ratio)`. It is exactly 1 through 150% comfortable capacity and then smoothly reduces births. This prevents unlimited happiness-building levels from defeating the specification's natural-equilibrium requirement. It never increases mortality or clamps population; §7 expressly allows other birth modifiers.
- The proposed aggregate-scale adjustment to wonder labor thresholds is still awaiting the user's answer. Current thresholds remain 25/50/100/200 assigned Slaves, so many small provinces cannot yet reach an accelerated tier. This is a known balance limitation, not a completed rebalance.
- The user clarified that **NPC civilians are assumed provisioned**. Unowned independent/vassal provinces therefore do not suffer famine merely because their geography produces insufficient food. Their full civilian and local military food demand still enters import demand and scarcity prices. The shared NPC province supply report is 100%, also keeping local defender provisioning abstract. Player-owned provinces and player forces continue to consume actual global food and can suffer famine.
- Wonders use the existing map's geographic site lookup. Multiple canonical sites can fall in one province, but completing any one blocks the others there, as required. No secondary wonder production, happiness, trade, or military bonuses were added.
- The user removed Ay Khanum after the reachability audit found its longitude 69.42 outside the atlas (easternmost province bound 43.79084). Its completed/construction source art and live catalog/config entries were removed. All ten remaining sites map to exactly one playable province without moving their coordinates; the map regression and campaign construction regression enforce this.
- No player civilian monthly Metal/Stone sink, monthly military Metal upkeep, reinforcement, or manpower healing was introduced. The later military exclusions override the earlier resource-role examples mentioning reinforcement/ships.
- NPC physical resources remain monthly flow capacities, not hidden stockpiles. Coin is persistent. Executed deals reserve capacity, demand, and budgets cumulatively, preventing a province from selling the same exports repeatedly to different players.
- New NPC agreements must support **100%** of their promised amount. The 80% tolerance is only for changes affecting an already established agreement. Three consecutive unsuccessful monthly executions cancel it. Splitting one-time transactions cannot farm relation: their combined province/player reward is capped at 0.1 per turn, versus the recurring monthly cap of 1.
- NPC trade acceptance compares incoming value **after transport** with the NPC's gross outgoing cost. The other party also receives its shipment after transport. Using post-loss amounts on both sides of the cost comparison would cancel distance losses algebraically and contradict the requirement that distance worsen trade terms.
- Economics does not own territorial transitions, diplomatic programs, trade-derived total-Control ceilings, rank income, vassal tribute/control income, military recruitment, or military consumption calculation. Those are integrated through campaign/politics/military adapters. Economics returns food supply and actual trade effects instead of maintaining duplicate balances.

## Balance scale and evidence

Nobles generate **0.25 Influence per month each**, separate from rank income. A six-Noble opening city with 40 starting Influence reaches the 100-Influence Aedile price in approximately 40 months if its population and other sources stay unchanged. The earlier 0.05 rate would require about 200 months in that illustration and was increased after the opening-progression audit. Rank income is unchanged. These are calibrated game currencies, not historical economic estimates.

The existing map uses roughly 25–70 aggregate residents per ordinary province/city before legacy randomized starting compensation; these are game units, not individual persons. The default monthly birth rates are 0.35–0.45%, natural mortality 0.20%, and full-shortage famine mortality 8%. Overcrowding lowers births through happiness rather than deleting residents. Capacity normalizes atlas geometry with `22 + 3.5 * sqrt(area)`, applies a configurable scale and terrain multiplier, and adds city/building capacity.

The deterministic atlas audit seeds every one of the **54** existing provinces from its actual geometry, city flag, and resource potential, with fixed class shares to remove random starting noise. At Balanced focus, global initial Food output exceeds civilian demand by approximately **344.5/month**, while **30 of 54** provinces have local deficits. Food focus reduces that to **11** local deficits, including three zero-Food territories. This supports meaningful agricultural specialization and global storage/trade instead of making every province self-sufficient.

Illustrative opening provinces, before buildings or recruitment:

| Province | Residents | Balanced net Food | Food-focus net Food | Metal/month | Stone/month |
| --- | ---: | ---: | ---: | ---: | ---: |
| Achaia | 53.8 | −27.1 | −2.8 | 3.9 | 77.8 |
| Tarraconensis | 66.7 | −8.7 | +37.6 | 60.9 | 18.7 |
| Lugdunensis | 59.4 | +33.5 | +67.3 | 6.1 | 7.5 |
| Africa Proconsularis | 62.5 | +107.2 | +163.8 | 0.0 | 44.4 |
| Aegyptus | 68.3 | +125.9 | +210.3 | 3.2 | 62.8 |
| Asia | 57.0 | +31.0 | +82.8 | 3.3 | 64.0 |

Achaia's grain deficit is soluble with Food focus plus one Farm: +15% output, costing 80 Stone and 15 Metal, completed in three monthly ticks. Its 450 opening Food provides about 16 months at the Balanced opening deficit before military demand, and far longer under Food focus. Mineral-poor provinces deliberately need trade/expansion for equipment; a 40-Metal heavy-infantry cohort costs around ten months of Achaia's unmodified Metal output but less than one month of Tarraconensis's. Recruitment time, finite class manpower, lost civilian production, and recurring military Food remain the other costs, so mineral abundance alone does not allow instant unlimited forces.

An isolated, unmanaged, player-owned province with zero Food and no imports will eventually starve; this is expected and is not evidence of a stable playable strategy. The 300-month all-province stress check tests finite/nonnegative state and storage limits. It does **not** claim that ignoring every deficit, policy, building, and trade opportunity is viable. A separate 480-month connected-world test checks demographic/migration numerical stability. Long-campaign multiplayer balance remains a playtesting task, not something proven by invariant tests.

## Verification and remaining integration audit

The README was traced through the reachable application paths, rather than only checking domain functions:

| README claim | Player entry point | Authoritative mutation or monthly effect |
| --- | --- | --- |
| Four population classes, capacity, happiness, births/deaths/class changes | Province → Overview; global HUD hover | `Campaign::advance_month` → `EconomyWorld::finish_month` → `advance_demographics`; `sync_campaign` projects actual balances and population-weighted class happiness changes |
| Food, focus, slave labor and migration policies | Province → Policies, direct-owner segmented choices | `campaign_economy::policies` edits the selected province's policies; `production`, `food_request`, `advance_demographics`, `migrate` consume them |
| Production/storage/taxes/domestic Influence | Overview resource rows and HUD ledgers | `begin_month` credits output and runs trade; `finish_month` consumes food, credits income and clamps shared storage; ownership reconciliation and one-time trade clamp immediately |
| All sixteen standard/city buildings | Buildings illustrated rows; city-only rows require a canonical city | Build calls `start_building` with the displayed rounded quote; monthly completion updates the real level; effects feed capacity/output/storage/happiness/tax/Influence, trade and military adapters |
| Ten reachable canonical wonders | Buildings at the actual site province | `campaign_seeds` polygon containment → `wonder_sites` → illustrated Build action → `start_wonder`; monthly rewards and map construction state use the same project |
| Wonder slave assignment/cancellation/capture | Active project slider and Cancel | `assign_wonder_slaves` validates local Slaves; configured tier tooltip exposes actual speed; diverted labor still consumes food; cancel preserves the spent cost; ownership change resets assignment |
| Explicit player trade consent | Select a foreign province → Trade → Propose; switch player → Trade → Accept | The same agreement appears in both participants' ledgers; `accept_trade` verifies the invited player; immediate quote and execution share bilateral supply validation |
| NPC prices, demand and finite budgets | Independent/vassal province → Trade market and proposal editor | `refresh_npc_markets` uses production, need and real treasury; live quote validates route, scarcity terms and unreserved capacity; proposal/acceptance changes actual agreements |
| Route loss, recurring suspension/resumption and trade pressure | Trade preview and agreement status/hover | `quote_trade` exposes route and net shipments; `advance_trade` revalidates, scales established NPC deliveries, suspends or cancels; campaign forwards actual effects into politics |

The audit fixed an orphan wonder, a permanently zero HUD happiness-change value, missing crowding-birth and wonder-tier explanations, and missing live net-delivery/affordability feedback on human acceptance. Policy tooltip numbers now derive from the same mutable configuration as the rules. Headless interaction/layout and native visual verification must still be reported separately; code tracing does not establish visual approval.

The coordinated `just test` run passes all 166 tests, including 25 economy-domain tests,
full-atlas construction reachability, ownership/storage reconciliation and shared military supply.
The economy checks include labor conservation, birth/death separation, migration, class changes,
bilateral trade consent and affordability, finite NPC markets, suspension/cancellation,
proportional deliveries, wonder opportunity cost, happiness deltas and long simulations.
The command `cargo test --lib game::economy -- --nocapture` includes atlas balance output.

Tests live under `tests/unit`, outside production source. See the shared
[verification record](verification.md) for the final build/tooling results. Native visual review
remains pending; numerical invariants and headless widget bounds do not prove strategic balance
or final on-screen appearance.
