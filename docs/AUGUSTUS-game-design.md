# Augustus — Population, Resources, Trade, Relations, Control, and Vassals

> Implementation specification.  
> This document is intended to be sufficient for another coding agent to implement the mechanics described here without needing the design conversation.

> User override, 2026-09-27: remove Ay Khanum because its coordinate lies outside the playable atlas.
> The live wonder catalog contains the remaining ten sites at their existing coordinates. This overrides
> the original attachment's eleven-site list in sections 124 and 142.

---

# 1. Scope and design goals

This specification defines:

- province population state;
- population capacity and overpopulation;
- class happiness;
- births and deaths;
- famine;
- automatic migration;
- class changes;
- province policies;
- global player resources and storage;
- resource production;
- buildings relevant to these systems;
- coin and influence;
- province relation toward players;
- independent-province control;
- peaceful and hostile ways to change control;
- vassalization;
- vassal control;
- passive vassal control gain/loss;
- tribute;
- armies stationed in vassals;
- recurring diplomatic spending;
- player-to-player trade;
- player-to-NPC trade;
- NPC resource needs, supply, pricing, and coin budgets;
- route validation and distance penalties;
- recurring and one-time trade;
- trade effects on relation and control;
- monthly simulation order;
- invariants and edge cases.

The core design constraints are:

1. A player will normally control only about **6–8 provinces**.
2. Province-level policy is therefore acceptable, but individual-pop or individual-worker micromanagement is not.
3. Population is aggregate, not simulated as individuals.
4. Population must naturally stop growing without a hard population ceiling.
5. Pops move automatically rather than by player drag-and-drop.
6. Player physical resources are **global**, not stored per province.
7. Buildings are province-local, but some buildings modify global storage.
8. Independent provinces can be contested by several players simultaneously.
9. `Relation` and `Control` are separate concepts.
10. A province can have **high control and low relation**, e.g. after invasion.
11. Vassals use **Relation + Vassal Control**. There is no separate loyalty score.
12. Trade must matter economically and politically without becoming a detailed market simulator.

---

# 2. Core terminology

## 2.1 Province

A map territory with:

- area;
- terrain;
- adjacency;
- city state;
- buildings;
- population;
- local policies;
- resource-production potential;
- political state.

A province is exactly one of:

```text
Independent
Vassal
Owned
```

## 2.2 Pop classes

The four aggregate population classes are:

```text
Nobles
Citizens
Plebeians
Slaves
```

Each class has:

```text
count: float
happiness: float in [0, 100]
```

## 2.3 Relation

`Relation[player]` is how positively the province views that player.

```text
0   = extreme hostility
50  = neutral
100 = extreme friendship
```

Relation does **not** measure political control.

## 2.4 Independent control

An independent province has a control distribution:

```text
Local + control[player_1] + control[player_2] + ... = 100
```

This measures political power over the province.

## 2.5 Vassal control

A vassal has one `vassal_control` value belonging to its overlord:

```text
0..100
```

It measures whether the overlord can keep the province as a vassal.

At `0`, the vassal becomes independent.

## 2.6 Player physical resources

The physical resources are:

```text
Food
Metal
Stone
```

These are global per player.

## 2.7 Strategic currencies

The non-physical currencies are:

```text
Coin
Influence
```

They do not require storage capacity.

---

# 3. Recommended data model

The exact Rust types may differ from the pseudocode, but the state separation should remain.

```rust
struct Province {
    id: ProvinceId,
    area: f32,
    terrain: TerrainType,
    adjacent: Vec<ProvinceId>,

    has_city: bool,
    buildings: Vec<BuildingInstance>,

    population: PopulationState,
    policies: ProvincePolicies,

    resource_potential: ResourcePotential,

    political_state: ProvincePoliticalState,

    // Relation is tracked separately from political state.
    relation_by_player: HashMap<PlayerId, f32>,
}
```

```rust
struct PopulationState {
    nobles: PopClassState,
    citizens: PopClassState,
    plebeians: PopClassState,
    slaves: PopClassState,
}
```

```rust
struct PopClassState {
    count: f32,
    happiness: f32, // clamp 0..100
}
```

```rust
struct ProvincePolicies {
    migration: MigrationPolicy,
    resource_focus: ResourceFocus,
    food_supply: FoodSupplyPolicy,
    slave_labor: SlaveLaborPolicy,
}
```

```rust
enum ProvincePoliticalState {
    Independent {
        local_control: f32,
        player_control: HashMap<PlayerId, f32>,
    },

    Vassal {
        overlord: PlayerId,
        vassal_control: f32,
        tribute: TributePolicy,

        // Optional recurring diplomatic support configured by overlord.
        monthly_coin_support: bool,
        monthly_influence_support: bool,
    },

    Owned {
        owner: PlayerId,
    },
}
```

Player economy:

```rust
struct PlayerEconomy {
    food: StoredResource,
    metal: StoredResource,
    stone: StoredResource,

    coin: f32,
    influence: f32,
}
```

```rust
struct StoredResource {
    current: f32,
    max_storage: f32,
}
```

Trade agreements:

```rust
enum TradeFrequency {
    OneTime,
    Monthly,
}
```

```rust
struct TradeBundle {
    food: f32,
    metal: f32,
    stone: f32,
    coin: f32,
    influence: f32, // allow initially; can be disabled by config later
}
```

```rust
struct TradeAgreement {
    id: TradeAgreementId,
    party_a: TradeParty,
    party_b: TradeParty,

    frequency: TradeFrequency,

    a_gives: TradeBundle,
    b_gives: TradeBundle,

    status: TradeStatus,
}
```

`TradeParty` can identify a player or an NPC province.

---

# 4. Population capacity

Every province has a **comfortable population capacity**.

It is not a hard cap.

Population may exceed it.

Capacity:

```text
population_capacity =
    area × terrain_capacity_modifier
    + city_capacity_bonus
    + building_capacity_bonuses
```

Recommended configurable terrain modifiers:

```text
Farmland     1.50
Plains       1.00
Forest       0.75
Hills        0.70
Mountains    0.35
Desert       0.20
```

These are initial balancing defaults, not architectural constants.

`area` must be normalized into a scale compatible with population counts. If existing map area values are too large, define:

```text
capacity_area = province_area × AREA_TO_CAPACITY_SCALE
```

and tune `AREA_TO_CAPACITY_SCALE`.

Example:

```text
area-derived capacity: 300
city bonus:            +100
aqueducts/buildings:    +80

total capacity:          480
```

---

# 5. Overpopulation

Define:

```text
population_ratio =
    total_population / population_capacity
```

When:

```text
population_ratio <= 1.0
```

there is no overcrowding happiness penalty.

When above `1.0`, overcrowding lowers happiness.

Recommended initial formula:

```text
overcrowding =
    max(0, population_ratio - 1)

overcrowding_happiness_penalty =
    min(50, overcrowding × 100)
```

Examples:

```text
100% capacity →  0 penalty
110%          → 10 penalty
125%          → 25 penalty
150%          → 50 penalty
200%          → 50 penalty due to cap
```

The exact curve must be kept configurable.

Important:

> Overpopulation does not directly kill pops.

It indirectly causes population decline through:

- lower happiness;
- fewer births;
- more emigration;
- deaths continuing normally.

---

# 6. Happiness

Each population class has happiness in:

```text
0..100
```

Interpretation:

```text
50 = normal / neutral
>50 = happy
<50 = unhappy
```

Happiness is calculated from a base of `50` plus modifiers.

Recommended structure:

```text
happiness =
    50
    + food_policy_modifier
    + building_modifier
    + event_modifier
    + occupation_modifier
    + class_specific_modifier
    - overcrowding_penalty
    - shortage_penalty
    - taxation_or_other_penalties
```

Always clamp to:

```text
0..100
```

Slaves may receive different class-specific modifiers from slave-labor policy.

---

# 7. Births

Births are separate from deaths.

For each class:

```text
births =
    class_population
    × base_birth_rate[class]
    × happiness_birth_modifier
    × food_policy_birth_modifier
    × other_birth_modifiers
```

Recommended initial happiness modifier:

```text
if happiness >= 50:
    happiness_birth_modifier =
        1 + (happiness - 50) / 100
else:
    happiness_birth_modifier =
        happiness / 50
```

Examples:

```text
Happiness 100 → 1.50x births
Happiness  75 → 1.25x
Happiness  50 → 1.00x
Happiness  25 → 0.50x
Happiness   0 → 0.00x
```

This is intentional:

- being happy gives moderate extra growth;
- being unhappy strongly reduces growth;
- at `0` happiness, births stop.

Do not let happiness above 50 double or triple births.

Base birth rates should be configurable per class.

---

# 8. Deaths

Deaths occur every month in every class.

```text
normal_deaths =
    class_population
    × base_death_rate[class]
    × food_policy_death_modifier
    × other_death_modifiers
```

Deaths are **not** directly increased merely because happiness is low.

Low happiness affects population through:

- fewer births;
- more emigration.

Additional death sources include:

- famine;
- events;
- war;
- harsh slave labor.

Monthly class update:

```text
new_count =
    old_count
    + births
    - normal_deaths
    - famine_deaths
    + immigration
    - emigration
    + class_changes_in
    - class_changes_out
```

Clamp count to `>= 0`.

---

# 9. Food shortages and famine

Food is global per player.

Each owned province requests food according to:

- population;
- food-supply policy.

Military units also consume food.

Let:

```text
total_food_requested =
    sum(province_food_requests)
    + army_food_requests
```

If:

```text
global_food_available >= total_food_requested
```

all requests are fulfilled.

Otherwise define:

```text
supply_ratio =
    global_food_available / total_food_requested
```

Use proportional allocation unless a future priority system is added.

For each province:

```text
food_received =
    requested_food × supply_ratio
```

Define shortage:

```text
shortage =
    1 - supply_ratio
```

Recommended famine mortality:

```text
famine_death_rate =
    shortage × MAX_FAMINE_DEATH_RATE
```

Then:

```text
famine_deaths =
    class_population × famine_death_rate
```

`MAX_FAMINE_DEATH_RATE` is configurable.

Shortage should also apply a happiness penalty and birth penalty.

Conceptually:

```text
100% supplied → normal
90%           → mild effect
75%           → serious
50%           → severe
0%            → catastrophic
```

Do not model famine as a binary `food == 0` switch.

---

# 10. Food-supply policy

Per province:

```text
Low
Normal
High
```

Recommended initial modifiers:

```text
Low:
    food_consumption ×0.80
    happiness -10
    births ×0.75
    normal_deaths ×1.15

Normal:
    food_consumption ×1.00
    happiness +0
    births ×1.00
    normal_deaths ×1.00

High:
    food_consumption ×1.20
    happiness +10
    births ×1.10
    normal_deaths ×0.95
```

Keep these in configuration.

Important:

- requested policy does not guarantee supply;
- if global food is insufficient, famine mechanics override the intended ration level.

---

# 11. Automatic migration

Migration is automatic.

The player never manually transfers ordinary pops between provinces.

Main mobile classes:

```text
Plebeians
Citizens
```

Nobles:

```text
much lower base migration
```

Slaves:

```text
no free migration
```

---

# 12. Emigration rate

For a mobile class:

```text
emigration_rate =
    base_migration_rate[class]
    × unhappiness_modifier
    × overpopulation_modifier
    × migration_policy_out_modifier
```

Recommended component behavior:

```text
unhappiness_modifier =
    max(0, (50 - happiness) / 50)
```

This produces:

```text
Happiness 50+ → 0 unhappiness pressure
Happiness 25  → 0.5
Happiness 0   → 1.0
```

Overpopulation:

```text
overpopulation_modifier =
    1 + max(0, population_ratio - 1) × OVERPOP_MIGRATION_SCALE
```

Recommended base rates should be small enough that migration is gradual.

Example target behavior, not hardcoded rates:

```text
happy + under capacity       → almost no migration
slightly unhappy             → low migration
very unhappy                 → meaningful migration
strongly over capacity       → strong migration
```

---

# 13. Migration destination

Migration is primarily to **adjacent provinces**.

Candidate destinations:

- adjacent owned provinces;
- adjacent vassals;
- adjacent friendly foreign provinces;
- adjacent neutral foreign provinces;
- hostile provinces only if explicitly allowed by later mechanics.

For each candidate calculate attractiveness:

```text
free_capacity_ratio =
    max(0, capacity - population) / capacity
```

Example score:

```text
attractiveness =
    free_capacity_weight × free_capacity_ratio
    + happiness_weight × normalized_happiness
    + city_bonus
    + migration_policy_in_bonus
    + ownership_relationship_bonus
```

Recommended political preference:

```text
Own province       1.00x
Own vassal         0.90x
Friendly foreign   0.60x
Neutral foreign    0.25x
Hostile            0.00x
```

Normalize candidate weights and distribute emigrants proportionally.

Migration conserves world population.

---

# 14. Migration policy

Per province:

```text
Encourage
Normal
Discourage
Closed
```

Recommended semantics:

```text
Encourage:
    higher incoming attractiveness

Normal:
    no modifier

Discourage:
    lower incoming attractiveness
    somewhat reduced outgoing migration

Closed:
    strongly suppress incoming and outgoing migration
```

Do not make `Closed` magically suppress all migration if the province is collapsing. Use strong modifiers rather than absolute prevention unless explicitly intended.

---

# 15. Class changes

Class changes are automatic and gradual.

Allowed examples:

```text
Slave     → Plebeian
Plebeian  → Citizen
Citizen   → Noble
```

Drivers may include:

- citizenship policy;
- manumission;
- city status;
- wealth;
- buildings;
- events;
- laws.

Class changes preserve total population.

The initial implementation may keep class-change rates simple/configured because the detailed political law system is not yet specified.

---

# 16. Player global resources

Physical resources:

```text
Food
Metal
Stone
```

are global per player.

There are **no province stockpiles** for owned provinces.

Example:

```text
Player economy:

Food     1,250 / 1,800
Metal      340 /   500
Stone      620 /   900
Coin     3,250
Influence   87
```

All production from owned provinces goes directly to global stockpiles.

---

# 17. Global storage

Each physical resource has:

```text
current
max_storage
```

Storage:

```text
max_storage =
    base_storage
    + sum(storage_building_bonuses)
    + other_modifiers
```

Buildings are local to provinces but their storage contribution is global.

Examples:

```text
Granary:
    +global food storage

Warehouse:
    +global food/metal/stone storage

Armory:
    +global metal storage

Stone Yard:
    +global stone storage
```

When stock exceeds maximum after production or after loss of a storage building:

```text
current = min(current, max_storage)
```

Excess is lost.

This is intentional and prevents effectively infinite accumulation.

Coin and influence do not use physical storage limits.

---

# 18. Resource production

Owned provinces produce:

```text
Food
Metal
Stone
```

using:

- Plebeians;
- Slaves;
- province resource potential;
- resource-focus policy;
- buildings;
- slave-labor modifiers.

The same worker must **not** simultaneously count as a full worker in every resource sector.

Compute available productive labor:

```text
productive_labor =
    plebeians × PLEBEIAN_PRODUCTIVITY
    + slaves × SLAVE_PRODUCTIVITY × slave_labor_modifier
```

Then distribute that labor among resources.

---

# 19. Resource focus

Per owned province:

```text
Balanced
Food
Metal
Stone
```

Use weights.

Initial defaults:

```text
Balanced:
    Food  1
    Metal 1
    Stone 1

Food Focus:
    Food  3
    Metal 1
    Stone 1

Metal Focus:
    Food  1
    Metal 3
    Stone 1

Stone Focus:
    Food  1
    Metal 1
    Stone 3
```

For each resource:

```text
allocation_weight[resource] =
    resource_potential[resource]
    × focus_weight[resource]
```

Normalize weights.

```text
allocated_labor[resource] =
    productive_labor
    × allocation_weight[resource]
    / sum(allocation_weights)
```

If resource potential is `0`, it receives `0` labor.

Production:

```text
production[resource] =
    allocated_labor[resource]
    × resource_potential[resource]
    × building_modifier
    × other_modifier
```

Exact scaling constants are balancing parameters.

---

# 20. Slave-labor policy

Per province:

```text
Light
Normal
Harsh
```

Recommended initial modifiers:

```text
Light:
    slave productivity ×0.80
    slave happiness +10
    slave normal deaths ×0.90
    revolt pressure reduced

Normal:
    baseline

Harsh:
    slave productivity ×1.25
    slave happiness -15
    slave normal deaths ×1.20
    revolt pressure increased
```

This policy creates a real production/stability tradeoff.

---

# 21. Resource roles

## Food

Recurring uses:

- population;
- armies;
- trade.

Food is both stockpile and monthly flow.

UI should show:

```text
Food: 1,200 / 2,000 (-20/month)
```

## Metal

Main uses:

- army recruitment;
- equipment;
- reinforcement after military losses;
- ships;
- forts;
- selected buildings;
- trade.

No artificial civilian monthly consumption is needed.

## Stone

Main uses:

- buildings;
- city development;
- fortifications;
- infrastructure;
- storage buildings;
- capacity-increasing buildings;
- trade.

No artificial civilian monthly consumption is needed.

---

# 22. Coin

Coin is a global player currency.

Player coin enters through:

- taxes from owned provinces;
- vassal tribute;
- selling goods;
- events/rewards.

Player coin leaves through:

- buildings;
- army costs;
- diplomacy;
- control purchases;
- gifts;
- espionage;
- trade purchases;
- vassal subsidies;
- recurring relation support.

World coin is **not conserved**.

NPC economies generate abstract monthly coin income.

This is intentional.

---

# 23. Player taxes

A simple initial owned-province coin-income model:

```text
province_tax_income =
    nobles    × noble_tax_rate
    + citizens   × citizen_tax_rate
    + plebeians  × plebeian_tax_rate
    + modifiers
```

Slaves need not pay direct tax because their contribution is already represented through production.

Exact rates are configurable.

Later systems may add:

- taxation policy;
- happiness effects;
- occupation penalties;
- tax buildings.

---

# 24. Influence

Influence is a global strategic currency.

Primary uses:

- improve relation;
- gain independent control;
- undermine rival control;
- support vassal control;
- enemy agitation;
- espionage;
- political actions.

Influence is deliberately scarcer than coin.

---

# 25. Relation

Every independent or vassal province tracks:

```text
relation[player]: 0..100
```

Recommended labels:

```text
0–19     Very Hostile
20–39    Hostile
40–59    Neutral
60–79    Friendly
80–100   Very Friendly
```

`50` is neutral.

Relation affects:

- NPC willingness to trade;
- NPC trade terms;
- route cost through that province;
- cost of peaceful control actions;
- likelihood of NPC aggression;
- vassal control decay;
- diplomatic-event outcomes.

Relation must remain separate from control.

---

# 26. Passive relation improvement

There must be passive ways to improve relation besides trade.

For any independent province or vassal, a player may enable:

```text
Monthly Coin Support
Monthly Influence Mission
```

Recommended initial behavior:

```text
Monthly Coin Support:
    pay configured coin cost each month
    +1 relation/month

Monthly Influence Mission:
    pay configured influence cost each month
    +1 relation/month
```

Both may be active simultaneously.

Recommended cap:

```text
maximum passive relation gain from these two support programs:
+2 relation/month
```

Costs are modified by political distance.

If the player cannot pay that month:

- no resource is deducted;
- no relation bonus is applied.

Do not auto-debt.

---

# 27. One-time relation improvement

The player can also use:

```text
Gift Coin
Spend Influence
```

for immediate relation gain.

One-time gifts have diminishing returns.

Moving relation from `20 → 30` should cost less than `80 → 90`.

A configurable formula:

```text
effective_cost_multiplier =
    1 + max(0, relation - 50) / 50
```

Then multiply by distance modifier.

---

# 28. Trade and relation

Successful recurring trade gradually improves relation.

Recommended rule:

```text
trade_relation_gain =
    min(
        TRADE_RELATION_MAX_PER_MONTH,
        recurring_trade_value × TRADE_RELATION_SCALE
    )
```

Recommended cap:

```text
+1 relation/month
```

or at most `+2` for very large trade.

One-time trade gives only a small immediate relation effect and never more than recurring sustained trade.

A gift is evaluated separately from trade.

---

# 29. Negative relation events

Relation can fall from:

- attack;
- invasion;
- occupation;
- broken agreements;
- embargo;
- detected spy;
- sabotage;
- support for enemies;
- excessive vassal tribute;
- hostile political actions;
- events.

Recommended example penalties:

```text
Detected hostile spy    -10
Direct attack           -20 or more
Invasion/occupation     very large negative modifier
```

Exact values are configuration.

---

# 30. Independent control

For an independent province:

```text
local_control + Σ player_control = 100
```

Example:

```text
Local      40
Player A   35
Player B   20
Player C    5
```

The entire distribution is visible to all players.

A new independent province usually starts:

```text
Local 100
Players 0
```

---

# 31. Gaining control with coin

Action:

```text
Buy Support
```

The player spends coin to gain control.

While `Local > 0`, control should normally come from Local first.

Base structure:

```text
coin_cost =
    BASE_COIN_PER_CONTROL
    × amount
    × existing_control_cost_modifier
    × relation_cost_modifier
    × distance_cost_modifier
    × entrenchment_modifier_if_taking_from_rival
```

Recommended behavior:

- low existing control: cheap;
- 25–49: more expensive;
- 50–74: expensive;
- 75–99: very expensive.

Relation:

- friendly province: cheaper;
- hostile province: more expensive;
- never impossible solely because relation is low.

---

# 32. Gaining control with influence

Action:

```text
Political Campaign
```

Influence is the preferred resource for directly fighting another player's political position.

While Local remains, it may take from Local.

When Local is exhausted or insufficient, take from rivals.

Deterministic rival-selection rule:

```text
Take from the rival with the highest control first.
Ties: stable deterministic ordering by PlayerId.
```

Structure:

```text
influence_cost =
    BASE_INFLUENCE_PER_CONTROL
    × amount
    × relation_modifier
    × distance_modifier
    × target_entrenchment_modifier
```

---

# 33. Undermine rival

Action:

```text
Undermine Rival
```

Effect:

```text
target_player_control -= amount
local_control += amount
```

The acting player gains no control.

This is useful when the player only wants to stop a rival.

Cost uses influence and is modified by:

- distance;
- target entrenchment;
- optionally relation.

---

# 34. Control entrenchment

High control is harder to remove.

Recommended initial modifiers:

```text
Target control 0–49   ×1.00
50–74                 ×1.25
75–89                 ×1.50
90–100                ×2.00
```

These multiply the cost of removing that player's control.

---

# 35. Military force and control

Military invasion can produce:

```text
higher control
lower relation
```

Example valid state:

```text
Control 85
Relation 12
```

This is intentional.

The detailed war/occupation formula is outside this document because the combat/occupation system has not yet been fully specified.

Implementation requirement:

> Do not couple relation and control so tightly that invasion cannot create high-control/low-relation states.

---

# 36. Political distance

Political and diplomatic actions become more expensive with distance.

Distance uses the province graph.

Find the shortest valid political path from any owned province or vassal connection to the target.

Recommended initial distance tiers:

```text
Adjacent                    ×1.00 cost
2–3 province steps          ×1.25
4–5 province steps          ×1.50
6+ province steps           ×2.00
No usable connection        unavailable or specially restricted
```

Keep the exact tier table configurable.

---

# 37. Vassalization

An independent province may be vassalized when:

```text
player_control >= 50
```

and the player is the **unique leading controller**.

Examples:

```text
A 50
B 50
→ neither can vassalize
```

```text
A 51
B 49
→ A may vassalize
```

On vassalization:

1. province state changes to `Vassal`;
2. old Local and all independent control shares are deleted;
3. overlord is set;
4. `vassal_control` is initialized to the overlord's control immediately before vassalization;
5. all relation values are retained;
6. tribute starts at `Normal` unless explicitly selected during vassalization.

Example:

```text
Before:
Local      10
A          55
B          30
C           5

Relation[A] = 32
```

After A vassalizes:

```text
Vassal of A
Vassal Control = 55
Relation[A] = 32
```

There is **no separate loyalty variable**.

---

# 38. Vassal control

`vassal_control`:

```text
0..100
```

means:

```text
how strongly the overlord can keep the province politically subordinated
```

At:

```text
vassal_control <= 0
```

the province becomes independent.

At independence:

```text
political_state = Independent
local_control = 100
all player_control = 0
```

All relation values are retained.

---

# 39. Passive vassal-control decay from poor relation

A vassal with poor relation toward its overlord slowly loses control.

No decay at or above relation `50`.

Recommended initial formula:

```text
relation_control_change =
    min(0, (relation_to_overlord - 50) / 10)
```

Equivalent examples:

```text
Relation 50+   0/month
Relation 40   -1/month
Relation 30   -2/month
Relation 20   -3/month
Relation 10   -4/month
Relation  0   -5/month
```

This is the baseline political instability of an unfriendly vassal.

---

# 40. Armies stationed in a vassal

Overlord military units physically stationed in the vassal provide passive control.

Recommended initial rule:

```text
stationed_army_control_bonus =
    min(number_of_qualifying_armies, 3)
```

Thus:

```text
1 army   +1 control/month
2 armies +2/month
3+       +3/month
```

"Qualifying army" means an overlord-controlled army whose current location is inside the vassal province.

Do not count armies merely adjacent to it.

Armies do **not** improve relation.

Optionally, a later occupation penalty may make a heavy military presence lower relation, but that is not required for the initial implementation.

This creates:

```text
Diplomacy → sustainable voluntary control
Military  → coercive control
```

---

# 41. Direct vassal-control support

The overlord may also spend influence/coin to maintain control.

Recommended supported actions:

```text
Support Government with Coin
Support Government with Influence
```

These may be:

- one-time actions;
- recurring monthly support.

If recurring support is implemented initially, use:

```text
monthly control support:
    pay configured cost
    +1 vassal control/month
```

Apply distance modifier to cost.

This is distinct from monthly relation support.

Therefore an overlord can separately choose to:

- make the vassal like them more;
- simply hold political control.

---

# 42. Vassal-control monthly calculation

Recommended order:

```text
vassal_control_change =
    relation_control_change
    + stationed_army_bonus
    + overlord_control_support
    - enemy_control_interference
    + event_modifiers
```

Then:

```text
vassal_control =
    clamp(vassal_control + vassal_control_change, 0, 100)
```

Example:

```text
Relation: 27

relation decay         -2.3
1 stationed army       +1.0
overlord support       +1.0
enemy agitation        -0.5
---------------------------
net                    -0.8

Control: 61 → 60.2
```

If the result reaches `0`, independence is resolved after the monthly calculation.

---

# 43. Tribute

Each vassal has:

```text
Low
Normal
High
```

Recommended effects:

```text
Low:
    low tribute paid
    +0.5 relation/month

Normal:
    baseline tribute
    0 relation/month

High:
    high tribute paid
    -1 relation/month
```

Exact tribute values depend on NPC economic output and should be configured.

Tribute may contain:

- coin;
- resources;
- or a mix.

For initial implementation, coin-only tribute is simplest.

High tribute indirectly threatens vassal stability:

```text
High tribute
→ relation falls
→ poor-relation control decay increases
→ control eventually falls unless offset
```

---

# 44. Enemy interference with vassals

Rival players cannot gain independent control while the province remains a vassal.

Instead they may:

```text
Fund Opposition with Coin
Political Agitation with Influence
Espionage
```

Effects may include:

- reduce `vassal_control`;
- reduce relation toward overlord;
- both for some espionage actions.

Distance strongly increases costs.

Detected spies cause relation penalties toward the spying player.

---

# 45. Three ways to maintain a vassal

The system deliberately supports:

## Friendly rule

```text
high relation
→ little/no passive control decay
→ few armies required
```

## Political rule

```text
spend coin/influence monthly
→ improve relation and/or control
```

## Military rule

```text
low relation
+ stationed armies
→ control remains high through coercion
```

All three use the same mechanics.

---

# 46. Annexation

An independent province can be directly annexed when one player reaches:

```text
100 independent control
```

Then:

```text
Local = 0
all rivals = 0
player = 100
```

The province becomes `Owned`.

Recommended optional safeguard:

```text
must remain at 100 until the next monthly resolution
```

before annexation becomes available.

This avoids instant last-action annexation without counterplay.

---

# 47. Owned province political state

Owned provinces do not have:

- independent-control distribution;
- vassal control.

They retain domestic:

- population;
- happiness;
- buildings;
- policies;
- production.

A later rebellion system may introduce domestic unrest or occupation control, but it is outside this specification.

---

# 48. Trade overview

Trade exists between:

```text
Player ↔ Player
Player ↔ Independent NPC Province
Player ↔ Vassal NPC Province
```

Owned provinces do not need internal trade because player resources are global.

Trade can exchange:

```text
Food
Metal
Stone
Coin
Influence
```

Keep Influence trade behind a configuration flag because it may be too exploitable.

Trade supports:

```text
One-time deal
Monthly recurring agreement
```

---

# 49. Trade routes

Every trade requires a valid route through the province graph.

A valid route may use:

- trader-owned provinces;
- trader vassals;
- the counterparty's territory;
- neutral/friendly provinces through which trade is permitted.

A route may **not** pass through an enemy province.

If every possible route crosses enemy territory:

```text
trade is impossible
```

Use shortest valid route.

Sea trade can later be represented by additional graph edges between eligible ports.

---

# 50. Route efficiency

Distance penalizes both sides.

Define route cost from:

- number of province steps;
- terrain;
- roads;
- ports later;
- relation with transit NPC provinces.

Initial simple implementation may use only path length:

```text
distance_steps = edges in shortest valid route
```

Recommended default efficiency:

```text
1 step    1.00
2 steps   0.95
3 steps   0.90
4 steps   0.85
5 steps   0.80
6+ steps  continue decreasing to a configurable minimum
```

Or formula:

```text
route_efficiency =
    max(MIN_TRADE_EFFICIENCY,
        1 - TRADE_LOSS_PER_STEP × max(0, steps - 1))
```

Recommended:

```text
TRADE_LOSS_PER_STEP = 0.05
MIN_TRADE_EFFICIENCY = 0.50
```

Relation may later improve/reduce transit cost.

Both directions use the same efficiency unless route direction has special mechanics.

Example at 80% efficiency:

```text
A sends 100 Food
B receives 80 Food

B sends 50 Metal
A receives 40 Metal
```

The lost amount represents:

- transport;
- spoilage;
- tolls;
- guards;
- merchants;
- friction.

For simplicity, apply the same efficiency to coin and influence initially as well.

---

# 51. Player-to-player trade

Players may propose arbitrary barter.

Example:

```text
Player A gives:
    100 Food
    50 Coin

Player B gives:
    20 Metal
    15 Stone
```

The game does **not** evaluate fairness.

Both human players decide.

## One-time

Execute once after both accept, if:

- both can supply the offered amounts;
- route still exists.

## Monthly

Repeat every month.

Example:

```text
Every month:

A gives:
    20 Food
    30 Coin

B gives:
    5 Metal
```

Agreement remains active until:

- canceled;
- route becomes invalid;
- war blocks route;
- a party cannot fulfill obligations;
- diplomacy invalidates trade.

Recommended failure behavior:

```text
temporary inability → suspend this month's execution
persistent inability for N months → auto-cancel
```

Use configurable `N`, recommended `3`.

---

# 52. NPC trade model

NPC provinces do **not** need full physical stockpiles.

Instead, each month calculate an abstract trade-economic state:

```rust
struct NpcTradeEconomy {
    food: NpcResourceMarket,
    metal: NpcResourceMarket,
    stone: NpcResourceMarket,

    coin_treasury: f32,
    monthly_coin_income: f32,
    monthly_coin_expenses: f32,
    trade_budget: f32,
}
```

```rust
struct NpcResourceMarket {
    production_potential: f32,
    internal_need: f32,

    export_capacity: f32,
    import_demand: f32,

    local_unit_value: f32,
}
```

---

# 53. NPC production potential

NPC production uses the same province fundamentals:

```text
production_potential(resource) =
    productive_population
    × resource_potential
    × building_modifiers
    × npc_focus_modifier
    × other_modifiers
```

NPC focus is chosen automatically.

A simple first AI:

```text
if severe food shortage:
    Food focus
else:
    focus highest comparative advantage / current demand
```

No player-facing worker allocation is required.

---

# 54. NPC food need

NPC food need is real recurring need:

```text
food_need =
    civilian_food_requirement
    + npc_military_food_requirement
```

Civilian food uses the same population consumption scale as players.

The NPC does not need a full hidden food stockpile for trading purposes.

Its recurring ability to import/export is derived from current monthly balance.

---

# 55. NPC metal need

NPC metal need is abstract.

It represents:

- military equipment;
- recruitment;
- reinforcement;
- fortifications;
- war pressure;
- selected development.

Recommended structure:

```text
metal_need =
    BASE_METAL_NEED
    + military_size × METAL_PER_MILITARY
    + war_modifier
    + recruitment_modifier
    + fortification_modifier
```

A peaceful small province should have low demand.

A threatened/militarizing province should have high demand.

---

# 56. NPC stone need

NPC stone need represents:

- construction;
- city development;
- walls;
- infrastructure;
- repairs.

Recommended structure:

```text
stone_need =
    BASE_STONE_NEED
    + city_modifier
    + active_development_modifier
    + fortification_modifier
    + repair_modifier
```

---

# 57. NPC surplus and shortage

For each resource:

```text
net =
    production_potential - internal_need
```

Then:

```text
if net > 0:
    export_capacity = net × NPC_EXPORT_SHARE
    import_demand = 0

if net < 0:
    import_demand = -net × NPC_IMPORT_SHARE
    export_capacity = 0
```

Recommended initial:

```text
NPC_EXPORT_SHARE = 1.0
NPC_IMPORT_SHARE = 1.0
```

These can be reduced if NPC trade volume is too large.

This recalculates monthly.

---

# 58. NPC visible demand bands

For UI, convert import/surplus state to:

```text
Severe Shortage
Shortage
Balanced
Surplus
Large Surplus
```

Example:

```text
Achaia

Food    Severe Shortage
Metal   Balanced
Stone   Large Surplus
```

Exact numeric thresholds are configurable.

---

# 59. NPC resource valuation

Each resource has a base value:

```text
BASE_VALUE_FOOD
BASE_VALUE_METAL
BASE_VALUE_STONE
```

NPC local value:

```text
local_unit_value =
    base_value × scarcity_multiplier
```

Recommended initial scarcity multipliers:

```text
Large Surplus      ×0.70
Surplus            ×0.85
Balanced           ×1.00
Shortage           ×1.35
Severe Shortage    ×1.75
```

These are balancing defaults.

---

# 60. NPC relation and trade terms

NPC relation modifies how favorable a trade must be for the NPC.

Define:

```text
required_value_ratio
```

meaning:

```text
value_received_by_npc
>=
value_given_by_npc × required_value_ratio
```

Recommended defaults:

```text
Relation 80–100    1.00
60–79              1.05
40–59              1.10
20–39              1.25
0–19               1.50 or refuse trade below a threshold
```

Then apply route efficiency to what each side actually receives **before** evaluating the trade.

Thus distant trade is inherently worse.

---

# 61. NPC coin treasury

Unlike physical resources, NPC coin is tracked as a treasury because NPCs must be able to pay for imports.

Each NPC province has:

```text
coin_treasury
monthly_coin_income
monthly_coin_expenses
```

Monthly coin income may be abstractly estimated:

```text
monthly_coin_income =
    nobles × noble_tax_value
    + citizens × citizen_tax_value
    + plebeians × plebeian_tax_value
    + city_bonus
    + prosperity_bonus
    + trade_income
```

No complete hidden fiscal simulation is needed.

NPC coin is intentionally created from this abstract internal economy.

---

# 62. NPC trade budget

NPCs must not spend their entire treasury on recurring trade.

Define:

```text
trade_budget =
    monthly_coin_income × NPC_TRADE_INCOME_SHARE
    + min(
        coin_treasury × NPC_TREASURY_SPEND_SHARE,
        NPC_MAX_TREASURY_DRAW
      )
```

Recommended initial values:

```text
NPC_TRADE_INCOME_SHARE = 0.40
NPC_TREASURY_SPEND_SHARE = 0.05
```

Exact values configurable.

Recurring trade is accepted only if monthly coin payments are sustainable within trade budget.

One-time trades may draw more heavily from treasury.

---

# 63. Coin in NPC trade

Coin has stable nominal value.

Physical resources fluctuate with scarcity.

Examples:

```text
Player gives:
    30 Food

NPC gives:
    20 Coin
```

or:

```text
Player gives:
    40 Coin

NPC gives:
    12 Stone
```

Coin is the common denominator when resource barter does not align.

---

# 64. NPC recurring trade routes

Recurring trade is the **main NPC trade form**.

Example:

```text
Every month:

Player sends:
    20 Food

Achaia sends:
    8 Stone
```

The NPC accepts only if:

- route exists;
- relation allows trade;
- value test passes;
- promised exports are within sustainable export capacity;
- promised coin payment is within trade budget;
- imported quantities do not exceed a reasonable multiple of import demand.

Recalculate NPC economy monthly.

Minor changes should not immediately kill a route.

Recommended tolerance:

```text
if NPC can provide at least 80% of promised amount:
    execute reduced amount proportionally

if below 80%:
    suspend route for that month
```

After `3` consecutive suspended months:

```text
auto-cancel
```

Keep thresholds configurable.

---

# 65. NPC one-time trade

NPC one-time trade is allowed.

It represents:

- emergency purchase;
- emergency sale;
- opportunistic exchange;
- immediate barter.

It should offer worse terms than recurring trade.

Recommended additional NPC margin:

```text
one_time_required_value_ratio =
    recurring_required_value_ratio × 1.10
```

One-time trades:

- may draw more from treasury;
- may use a larger temporary export allowance;
- may slightly improve relation;
- do **not** generate control.

---

# 66. Recurring trade and relation

Recurring trade improves relation.

Use actual delivered trade value after route loss.

Recommended:

```text
monthly_trade_relation_gain =
    min(1.0, delivered_trade_value / TRADE_VALUE_PER_RELATION)
```

Tune `TRADE_VALUE_PER_RELATION`.

A one-time trade can grant a small immediate bonus but should not outperform a stable recurring relationship.

---

# 67. Recurring trade and independent control

Recurring trade with an independent province generates slow passive control.

One-time trade generates **no control**.

Recommended:

```text
trade_control_gain =
    min(
        TRADE_CONTROL_MAX_PER_MONTH,
        delivered_trade_value × TRADE_CONTROL_SCALE
    )
```

Recommended monthly cap:

```text
1 control/month
```

Recommended total trade-derived control ceiling:

```text
40
```

Meaning trade cannot increase that player's independent control above `40` by itself.

If player control is already above 40 because of other actions, trade does not add more.

---

# 68. Competing foreign trade

If multiple players trade with the same NPC, track each player's recurring delivered trade value.

Example:

```text
A: 60%
B: 30%
C: 10%
```

Trade-derived control may be multiplied by foreign-trade share:

```text
effective_trade_control_gain =
    raw_gain × foreign_trade_share
```

This makes economic competition matter.

---

# 69. Vassal trade

Vassals may trade with their overlord.

Recurring trade with the overlord:

- improves relation normally;
- can therefore indirectly stabilize vassal control.

Do **not** add a separate automatic loyalty bonus because there is no loyalty stat.

The chain is:

```text
trade
→ relation rises
→ low-relation control decay decreases
→ vassal becomes more stable
```

This is simpler and avoids duplicate variables.

---

# 70. Trade interruption

A recurring route is suspended if:

- no valid route remains;
- enemy territory blocks all paths;
- war invalidates the deal;
- one party cannot fulfill;
- NPC relation falls below the trading threshold;
- NPC supply/demand changes too far;
- a party cancels.

Temporary route failure should suspend rather than instantly delete the agreement.

Persistent failure may auto-cancel.

---

# 71. NPC hostility and attacks

Relation affects NPC aggression.

Exact military-AI rules are outside this document, but the relationship should be usable as an input.

Recommended broad behavior:

```text
80–100   strongly friendly
60–79    friendly
40–59    neutral
20–39    hostile
0–19     very hostile / possible attack
```

Low relation should increase attack probability or permit hostile actions.

Do not guarantee attack solely from relation; military AI may include strength and opportunity.

---

# 72. Buildings relevant to this system

Examples:

## Aqueduct

```text
+province population capacity
```

## Irrigation / Farms

```text
+food production
and/or
+population capacity
```

## Granary

```text
+global food storage
```

## Warehouse

```text
+global storage
```

## Armory

```text
+global metal storage
```

## Stone Yard

```text
+global stone storage
```

## Mine

```text
+province metal production
```

## Quarry

```text
+province stone production
```

## Roads

```text
+trade-route efficiency
+possibly migration attractiveness
```

## City improvements

```text
+population capacity
+Citizen/Noble happiness
```

Buildings should have clear, narrow effects.

---

# 73. Monthly simulation order

Use deterministic ordering.

Recommended exact order:

```text
1. Resolve completed construction and state-changing actions from the turn.

2. Recalculate owned-province:
   - capacity
   - happiness base modifiers
   - productive labor
   - production allocation

3. Calculate owned-province Food/Metal/Stone production.

4. Add production to each player's global physical stockpiles,
   temporarily allowing values above max until all same-month consumption/trade
   is resolved if desired; otherwise clamp after each stage consistently.

5. Recalculate NPC trade economies:
   - production potential
   - food/metal/stone need
   - import demand
   - export capacity
   - scarcity valuation
   - monthly coin income
   - trade budget

6. Validate recurring trade routes.

7. Execute recurring trade agreements:
   - check route
   - compute route efficiency
   - apply supply/budget limits
   - transfer delivered amounts
   - track delivered trade value
   - update consecutive-failure counters

8. Apply one-month recurring diplomatic programs:
   - monthly coin relation support
   - monthly influence relation support
   - vassal control-support spending

9. Calculate owned-province food requests from:
   - population
   - food policy

10. Add army food requests.

11. Fulfill food requests from global player food stock.

12. Calculate provincial food shortage ratios.

13. Calculate final class happiness:
   - baseline 50
   - overcrowding
   - food policy
   - shortage
   - slave-labor effects
   - buildings/events/other modifiers

14. Calculate births.

15. Calculate normal deaths.

16. Calculate famine deaths.

17. Apply class changes.

18. Calculate emigrants.

19. Distribute migrants to adjacent valid destinations.

20. Apply relation changes:
   - recurring trade
   - tribute
   - recurring diplomatic spending
   - occupation/events
   - other monthly modifiers

21. Apply independent-province passive trade control gain.

22. Apply vassal tribute transfers.

23. Calculate each vassal's control change:
   - poor-relation decay
   - stationed-army bonus
   - overlord control support
   - enemy interference
   - events

24. Apply player tax income.

25. Apply NPC monthly coin income/expenses.

26. Recalculate player global storage maxima from surviving buildings.

27. Clamp physical stockpiles to storage maxima.

28. Clamp:
   - happiness 0..100
   - relation 0..100
   - vassal control 0..100
   - resource stocks >= 0

29. Resolve political state transitions:
   - vassal control <= 0 → independent
   - annexation eligibility
   - other explicitly triggered transitions

30. Save/emit monthly summary deltas for UI.
```

If implementation order differs, preserve the same logical semantics.

---

# 74. Immediate actions during a turn

The following can execute immediately rather than waiting for monthly tick:

```text
One-time player trade
One-time NPC trade
Gift
One-time influence relation action
Buy independent control
Political Campaign
Undermine Rival
Espionage attempt
Attack / invasion
Vassalize
Annex when eligible
Change province policy
Change tribute
Enable/disable recurring support
Create/cancel recurring trade agreement
```

Monthly consequences are applied in the monthly simulation.

---

# 75. Political state transitions

## Independent → Vassal

Requirements:

```text
player control >= 50
player is unique control leader
```

Result:

```text
state = Vassal
overlord = player
vassal_control = player's former independent control
all independent control removed
relations retained
```

## Independent → Owned

Requirement:

```text
player independent control = 100
```

Result:

```text
state = Owned
owner = player
independent control removed
relations may be retained internally if useful for future unrest
```

## Vassal → Independent

Requirement:

```text
vassal_control <= 0
```

Result:

```text
state = Independent
local_control = 100
all player_control = 0
relations retained
```

There is currently no direct `Vassal → Owned` rule specified. If desired later, define an annex-vassal mechanic separately.

---

# 76. Invariants

The implementation must enforce:

```text
population class count >= 0

0 <= happiness <= 100

population_capacity > 0

player physical resource stock >= 0

player physical resource stock <= max storage after final monthly clamp

coin >= 0 unless debt is explicitly added later

influence >= 0

0 <= relation <= 100

for Independent:
    local_control >= 0
    each player_control >= 0
    local_control + Σ player_control = 100 within float tolerance

for Vassal:
    exactly one overlord
    0 <= vassal_control <= 100
    no independent-control distribution

for Owned:
    exactly one owner
    no independent-control distribution
    no vassal control

migration does not change global population

class change does not change total population

births/deaths are demographic source/sink

one-time NPC trade never grants control

recurring NPC trade control cannot exceed configured trade-control ceiling

trade cannot execute without a valid route

trade route cannot pass through enemy territory

vassal control at 0 resolves to independence
```

For floating-point independent control, normalize after modifications so the total is exactly 100 within a small epsilon.

---

# 77. UI requirements

## Owned province

Show:

```text
Population
Capacity
Population ratio

Nobles       count / happiness
Citizens     count / happiness
Plebeians    count / happiness
Slaves       count / happiness

Monthly births/deaths/migration

Policies:
    Migration
    Resource focus
    Food supply
    Slave labor

Buildings

Production:
    Food
    Metal
    Stone
```

## Global player economy

Show:

```text
Food      current / max     monthly expected change
Metal     current / max     monthly expected change
Stone     current / max     monthly expected change
Coin                      monthly expected change
Influence
```

## Independent NPC province

Show:

```text
Relation with you

Control:
    Local
    Player A
    Player B
    ...

Resource market:
    Food    shortage/surplus band
    Metal   shortage/surplus band
    Stone   shortage/surplus band

Trade route availability
Distance / route efficiency
```

## Vassal

Show:

```text
Vassal of <player>

Relation with overlord
Vassal Control
Expected monthly Vassal Control change

Breakdown:
    relation decay
    stationed armies
    support spending
    enemy interference

Tribute policy

Recurring diplomatic support:
    Coin support on/off
    Influence mission on/off

Trade
```

## Trade proposal

Show:

```text
Route valid: yes/no
Route length
Route efficiency

You give
You receive

For NPC:
    Their valuation / acceptable or not
    Recurring or one-time
    Expected relation effect
    Expected trade-control effect if independent
```

---

# 78. Configuration

Do not scatter balance values through systems.

Create centralized configuration structures/files for:

```text
terrain capacity modifiers
city capacity bonus

birth rates
death rates

food consumption per class
army food consumption

food-policy modifiers

overcrowding curve
famine mortality

migration base rates
migration policy modifiers
destination weights

resource production scaling
resource-focus weights
slave-labor modifiers

base physical storage
building storage bonuses

tax rates

relation gift costs
relation influence costs
recurring relation-support costs

independent control coin costs
independent control influence costs
entrenchment multipliers

political distance multipliers

vassal relation-decay formula
stationed-army control cap
vassal support costs

tribute values/modifiers

trade route loss per step
minimum trade efficiency

NPC base resource values
NPC scarcity multipliers
NPC trade acceptance relation modifiers
NPC monthly resource-needs constants
NPC trade budget percentages

recurring trade relation scale/cap
recurring trade control scale/cap
trade-derived total control cap

trade suspension/cancellation thresholds
```

---

# 79. Recommended initial balance constants

These are starting defaults only.

```text
Neutral happiness: 50

Overcrowding penalty:
    min(50, max(0, population/capacity - 1) × 100)

Birth modifier:
    happiness < 50: happiness / 50
    happiness >= 50: 1 + (happiness - 50) / 100

Food policy:
    Low     consumption 0.80, happiness -10, births 0.75, deaths 1.15
    Normal  consumption 1.00, happiness  0, births 1.00, deaths 1.00
    High    consumption 1.20, happiness +10, births 1.10, deaths 0.95

Resource focus:
    focused resource weight 3
    other resource weights 1

Slave labor:
    Light   productivity 0.80, happiness +10, death 0.90
    Normal  productivity 1.00, happiness   0, death 1.00
    Harsh   productivity 1.25, happiness -15, death 1.20

Independent control:
    vassal threshold 50
    annex threshold 100

Trade-derived control:
    max +1/month
    total ceiling 40

Vassal relation-control decay:
    relation >=50: 0
    relation 40: -1/month
    relation 30: -2/month
    relation 20: -3/month
    relation 10: -4/month
    relation  0: -5/month

Stationed army control:
    +1/month per qualifying army
    max +3/month

Recurring relation support:
    Coin program      +1 relation/month
    Influence program +1 relation/month
    combined max      +2/month

Trade:
    5% loss per route step after first
    minimum efficiency 50%

NPC one-time trade:
    10% worse required value ratio than recurring

Recurring trade failure:
    <80% deliverable → suspend month
    3 consecutive suspensions → cancel
```

Any constant not explicitly specified remains a balancing/configuration value, not a hidden mechanic.

---

# 80. Worked population example

Province:

```text
Capacity: 400
Population: 500
Population ratio: 1.25
```

Overcrowding penalty:

```text
(1.25 - 1.0) × 100 = 25
```

Suppose Plebeian base happiness before overcrowding is 55:

```text
Plebeian happiness = 55 - 25 = 30
```

Birth modifier:

```text
30 / 50 = 0.60
```

Deaths continue at normal rate.

Plebeian emigration pressure is also positive because happiness is below 50 and capacity is exceeded.

Therefore:

```text
births fall
deaths continue
emigration increases
```

Population trends downward without hard-clamping to 400.

---

# 81. Worked vassal example

A player vassalizes at:

```text
independent control: 58
relation: 25
```

Vassal starts:

```text
Vassal Control = 58
Relation = 25
```

Monthly poor-relation decay:

```text
(25 - 50) / 10 = -2.5
```

One army is stationed:

```text
+1
```

Overlord also funds control support:

```text
+1
```

Enemy agitation:

```text
-0.5
```

Net:

```text
-2.5 +1 +1 -0.5 = -1.0
```

After month:

```text
Control 58 → 57
```

If the overlord also runs monthly Coin Support for relation:

```text
Relation 25 → 26
```

Over time this slowly reduces the natural control decay.

---

# 82. Worked NPC recurring-trade example

Achaia calculates:

```text
Food production potential: 40
Food need:                 65
Food deficit:              25

Stone production potential: 70
Stone need:                 30
Stone surplus:              40
```

Player proposes monthly:

```text
Player gives: 20 Food
Achaia gives: 8 Stone
```

Route:

```text
3 steps
90% efficiency
```

Delivered:

```text
Achaia receives 18 Food
Player receives 7.2 Stone
```

NPC evaluates the **delivered** values using:

- food shortage value;
- stone surplus value;
- relation margin.

If accepted, every month:

- exchange occurs;
- relation improves slowly;
- if Achaia is independent, trade may grant passive control;
- if supply/demand changes slightly, delivered amount may scale;
- if it becomes unsustainable for 3 months, the route ends.

---

# 83. Worked independent-control competition example

Initial:

```text
Local   100
A         0
B         0
```

After trade and political spending:

```text
Local    35
A        40
B        25
```

A buys +5 control:

```text
Local    30
A        45
B        25
```

Later:

```text
Local     0
A        60
B        40
```

If B gains +5 through influence, Local cannot supply it.

Take from largest rival:

```text
A        55
B        45
```

If B instead undermines A by 5:

```text
Local     5
A        55
B        40
```

---

# 84. Core gameplay loops

## Population

```text
Area + Terrain + City + Buildings
                ↓
        Population Capacity
                ↓
           Overcrowding
                ↓
            Happiness
          ↙           ↘
       Births       Migration
          ↓
    Deaths continue
          ↓
 Population self-stabilizes
```

## Economy

```text
Plebeians + Slaves
        ↓
Province Resource Focus
        ↓
Food / Metal / Stone Production
        ↓
Player Global Stockpiles
        ↓
Global Storage Limits
        ↓
Consumption / Buildings / Armies / Trade
```

## Independent politics

```text
Trade / Gifts / Influence
          ↓
       Relation

Coin / Influence / Trade / War
          ↓
        Control
      ↙        ↘
  50+ leader   100
      ↓         ↓
   Vassal      Owned
```

## Vassal politics

```text
Trade / Subsidy / Diplomacy
             ↓
          Relation
             ↓
 low relation causes passive Control decay

Stationed armies / political support
             ↓
       passive Control gain

Control reaches 0
             ↓
        Independence
```

---

# 85. Explicit non-goals

Do not implement these unless separately requested:

- individual pop entities;
- individual jobs;
- wages;
- household economies;
- province-local player stockpiles;
- internal player trade routes between owned provinces;
- full NPC physical-resource stockpiles;
- global conserved coin supply;
- Victoria-style market clearing;
- individual migrant pathfinding across many provinces;
- hidden control values;
- hidden relation values by default;
- a separate vassal loyalty stat;
- automatic conversion of relation into control;
- automatic conversion of control into relation.

---

# 86. Final implementation principle

The player should make strategic choices:

```text
Which buildings?
Which resource focus?
How much food?
How hard are slaves worked?
Do we encourage migration?
Who do we trade with?
Do we improve relation?
Do we buy political control?
Do we vassalize now or build more control first?
How much tribute?
Do we maintain a vassal through friendship, political spending, or armies?
```

The simulation handles automatically:

```text
births
deaths
famine
migration
worker allocation
resource production
NPC supply/demand
NPC pricing
recurring trade execution
relation drift
trade-derived control
vassal-control decay/gain
political state transitions
```

The system should remain readable enough that the player can understand **why** a number changed each month. UI breakdowns should therefore expose the main contributors to happiness, relation, control, resource balance, and trade efficiency.

---

# 87. Espionage

Espionage is a persistent province assignment.

A player may deploy a spy to a foreign independent province, foreign vassal, or province owned by another player.

Recommended state:

```rust
struct SpyMission {
    owner: PlayerId,
    target_province: ProvinceId,
    active: bool,
    months_active: u32,
}
```

Deployment has:

```text
one-time influence cost
monthly coin maintenance
```

Recommended initial defaults:

```text
Deploy spy:        10 influence
Maintenance:        5 coin/month
```

Keep these values configurable.

Allow at most one active spy network per player per target province.

If monthly maintenance cannot be paid:

```text
spy becomes inactive and is withdrawn
```

Do not create debt.

---

# 88. Monthly spy resolution

Each active spy performs two logically separate monthly checks:

```text
1. Detection check
2. If not detected: scandal-discovery check
```

Detection determines whether the spy is exposed.

Discovery determines whether the spy learns compromising information.

The two systems must remain separate.

---

# 89. Spy detection

Every active spy has a monthly chance to be discovered.

Detection increases with Noble happiness in the target province.

Reason:

```text
happy / loyal elites cooperate more effectively with the local government
```

Recommended initial mapping:

```text
Noble happiness    Monthly detection chance

0                   2%
25                  4%
50                  6%
75                  8%
100                10%
```

Equivalent interpolated formula is acceptable.

Future modifiers may include:

- counter-espionage buildings;
- laws;
- events;
- spy experience.

On successful detection:

```text
remove spy
```

Then resolve consequences according to target type.

---

# 90. Discovered spy against NPC

If an NPC province discovers a player's spy:

```text
relation[target_province][spying_player] -= SPY_DISCOVERY_RELATION_PENALTY
```

Recommended initial penalty:

```text
-10 relation
```

Clamp relation to `0..100`.

No player scandal is generated because the target is an NPC.

---

# 91. Discovered spy against player

If a spy is discovered in a province controlled by another human player:

1. remove the spy;
2. the target player receives a scandal against the spying player.

Example:

```text
Scandal:
"Player A conducted espionage against Player B"
```

This scandal is based on an actual action and may later be used by the victim for political leverage.

Recommended severity:

```text
Medium
```

Repeated independently discovered spy missions may generate separate scandals.

---

# 92. Player scandal sources

Human-player scandals must come from **real game state or actual decisions**.

Do not generate random scandals for players.

Examples of valid scandal sources:

```text
Low Food Supply
High Taxes
Harsh Slave Labor
Very unhappy Citizens
Very unhappy Plebeians
Active famine
Mass starvation
High vassal tribute
Military occupation of a hostile vassal
Breaking an agreement
Attacking a friendly province
Detected espionage
Future explicitly-defined abusive or unpopular policies
```

A scandal source is discoverable only while its underlying condition is active, unless it represents a completed one-time action such as treaty-breaking or detected espionage.

---

# 93. Scandal opportunity model

Represent discoverable information as:

```rust
struct ScandalOpportunity {
    source_id: ScandalSourceId,
    target_player: PlayerId,
    province: Option<ProvinceId>,
    kind: ScandalKind,
    severity: ScandalSeverity,
    discovery_chance: f32,
}
```

The important field is `source_id`.

It uniquely identifies one activation of one scandal source.

Example:

```text
Player A
Italia
Low Food Supply
January–May activation
```

A spy owner may discover the same `source_id` only once.

If the policy ends and is later restarted, create a new `source_id`.

---

# 94. Active versus previously discovered scandal

Example:

```text
Low Food Supply active in Italia
```

Enemy spies may discover it each month until:

- the scandal is discovered by that player; or
- the policy ends.

When the policy ends:

```text
new discovery chance = 0
```

However, any scandal already discovered remains in the spying player's scandal inventory until used or expired.

Thus:

```text
condition ending stops discovery
condition ending does not erase discovered evidence
```

---

# 95. Scandal discovery chance

Every scandal type has a configurable base discovery chance.

Recommended initial examples:

```text
Low Food Supply             10% / month
Harsh Slave Labor            8% / month
High Taxes                   6% / month
Citizens happiness <25      10% / month
Active Famine               20% / month
Treaty Violation            25% / month
```

Then:

```text
final_discovery_chance =
    base_discovery_chance
    × severity_modifier
    × spy_modifiers
```

Noble happiness does **not** reduce discovery chance.

It affects spy detection instead.

This separation keeps the system understandable:

```text
Noble happiness → chance spy gets caught
Scandal severity → chance spy finds scandal
```

---

# 96. NPC scandals

NPC provinces do not require a complete history of bad political decisions.

Instead they maintain a small hidden scandal pool.

Possible NPC scandal types:

```text
Corrupt Governor
Secret Payments
Abuse of Citizens
Illegal Tax Collection
Military Incompetence
Elite Feud
Smuggling
```

Recommended generation:

```text
small monthly chance to create a hidden scandal
maximum 2–3 hidden scandals per NPC province
```

Example initial generation chance:

```text
5% per month
```

Keep configurable.

An active spy may discover one hidden scandal.

Once used, remove it from the pool.

---

# 97. Scandal inventory

Each player owns discovered scandals.

Recommended model:

```rust
struct Scandal {
    id: ScandalId,
    target: ScandalTarget,
    kind: ScandalKind,
    severity: ScandalSeverity,
    source_province: Option<ProvinceId>,
    acquired_at: GameDate,
    expires_at: Option<GameDate>,
}
```

Possible target:

```text
Player
NPC Province
```

Recommended default expiration:

```text
24 months
```

Major scandals may have longer expiry.

Expiration must be configurable.

---

# 98. Using scandals against NPC provinces

NPC scandals are direct political leverage.

A scandal may be consumed for one of two initial uses.

## Gain Control

```text
Consume scandal
→ gain independent control
→ reduce relation
```

Recommended initial effect:

```text
Minor scandal     +5 control
Major scandal    +10 control
```

Control comes from `Local` first.

If insufficient Local control remains, use the normal deterministic rival-removal rule.

Blackmail also damages relation.

Recommended:

```text
-5 relation
```

## Force Favorable Trade

Consume scandal to temporarily improve NPC trade terms.

Recommended initial effect:

```text
6 months
NPC required value ratio ×0.75
```

The scandal is consumed immediately.

The favorable-trade modifier applies only between the scandal owner and that NPC.

---

# 99. Using scandals against players

A scandal against another player does **not** directly:

- steal resources;
- remove control;
- damage armies;
- transfer provinces.

Instead it is political leverage.

Future and related systems may consume scandals for:

```text
vote pressure
Senate motions
political demands
treaty negotiation leverage
public accusation
blocking or weakening political actions
other explicitly-defined political systems
```

The scandal inventory therefore acts as stored leverage.

---

# 100. Foreign interference

Players need direct ways to attack another player's political position in a province without taking that position for themselves.

Foreign interference is separate from espionage.

Espionage discovers information and scandals.

Foreign interference spends resources to alter:

```text
another player's Relation
another player's Independent Control
an overlord's Vassal Control
```

All interference actions target:

```text
one province
one specific rival player
```

This is intentional because control and relation values are visible.

---

# 101. Smear Campaign — lower rival Relation

Available against:

- a rival player in an independent NPC province;
- the overlord of a vassal province.

Action:

```text
Smear Campaign
```

Cost:

```text
Influence
```

Effect:

```text
relation[target_player] -= amount
```

The acting player's own relation is unchanged.

Recommended initial action:

```text
Base cost:       10 influence
Base effect:     -5 relation
```

Final cost:

```text
cost =
    base_cost
    × political_distance_modifier
    × resistance_modifier
```

Relation is clamped to `0..100`.

Limit:

```text
at most one Smear Campaign
per acting player
per target player
per province
per month
```

This prevents unlimited same-tick dumping.

---

# 102. Fund Opposition — lower rival Independent Control

Available only in an **independent** province.

Target:

```text
specific rival player with control > 0
```

Action:

```text
Fund Opposition
```

Recommended resource:

```text
Coin
```

Effect:

```text
target_player_control -= amount
local_control += amount
```

The acting player gains no control.

Recommended initial action:

```text
Base cost:       100 coin
Base effect:       -5 target control
```

Final cost:

```text
cost =
    base_cost
    × political_distance_modifier
    × target_entrenchment_modifier
    × target_relation_resistance_modifier
```

Never remove more control than the target has.

Example:

```text
Before:
Local      10
A          55
B          35

B targets A with Fund Opposition for 5:

After:
Local      15
A          50
B          35
```

This is the coin-based equivalent of the influence-based `Undermine Rival` action already defined earlier.

Implementation may merge both into one generic control-reduction action with different payment methods, but the player-facing distinction should remain clear.

---

# 103. Political Agitation — lower Vassal Control

Available only against an enemy vassal.

Target is implicitly:

```text
the vassal's overlord
```

Action:

```text
Political Agitation
```

Recommended cost resource:

```text
Influence
```

Effect:

```text
vassal_control -= amount
```

Recommended initial values:

```text
Base cost:       10 influence
Base effect:     -3 vassal control
```

Final cost:

```text
cost =
    base_cost
    × political_distance_modifier
    × vassal_control_entrenchment_modifier
```

A vassal with high control is harder to destabilize.

Recommended resistance:

```text
Vassal Control 0–49    ×1.00
50–74                  ×1.25
75–89                  ×1.50
90–100                 ×2.00
```

At `0` control, the normal vassal-independence transition occurs.

---

# 104. Fund Dissidents — lower Vassal Relation toward Overlord

Available only against an enemy vassal.

Action:

```text
Fund Dissidents
```

Recommended cost:

```text
Coin
```

Effect:

```text
relation[overlord] -= amount
```

Recommended initial values:

```text
Base cost:      100 coin
Base effect:     -5 relation
```

This does not immediately change vassal control.

Instead it worsens future passive stability:

```text
lower relation
→ stronger monthly poor-relation control decay
```

This creates a slower but potentially more efficient destabilization strategy.

---

# 105. Direct versus indirect vassal destabilization

Two distinct approaches therefore exist.

## Direct

```text
Political Agitation
→ immediately lowers Vassal Control
```

Fast but expensive.

## Indirect

```text
Fund Dissidents
→ lowers Relation toward overlord
→ monthly Vassal Control decay increases
```

Slower but persistent until the overlord repairs relations.

This distinction should remain visible in the UI.

---

# 106. Relation resistance to interference

A province that strongly likes the targeted player should be harder to turn against them.

Recommended modifier for actions that lower a rival's relation:

```text
Target relation 0–39     ×1.00 cost
40–59                    ×1.25
60–79                    ×1.50
80–100                   ×2.00
```

Thus:

```text
destroying an already-poor relationship is cheap
turning a loyal province against someone is expensive
```

For direct control attacks, use control entrenchment primarily rather than relation.

---

# 107. Distance penalties for interference

All foreign-interference actions use the existing political-distance multiplier.

Recommended:

```text
Adjacent              ×1.00
2–3 steps             ×1.25
4–5 steps             ×1.50
6+ steps              ×2.00
```

If there is no valid political connection, the action is unavailable.

This applies to:

- Smear Campaign;
- Fund Opposition;
- Political Agitation;
- Fund Dissidents.

---

# 108. Interaction with spies

Spies are **not required** for the basic interference actions above.

This keeps political competition available even when espionage is absent.

However, spies can enhance interference later.

Recommended optional modifier:

```text
active undetected spy in target province
→ -20% interference cost
```

This is optional for the first implementation.

Scandals may also provide special leverage beyond the normal interference actions.

---

# 109. Discovery / attribution of foreign interference

Base interference actions are treated as political operations, not secret spy missions.

Therefore their mechanical effect is deterministic and does not require a discovery roll.

For the first implementation:

```text
the target can see:
- that Relation or Control changed
- the acting player responsible
```

This keeps multiplayer transparent and avoids adding another hidden-information layer.

If covert political operations are desired later, they should be a separate espionage action with detection risk and scandal consequences.

---

# 110. Updated vassal monthly example

Vassal state:

```text
Overlord: Player A
Relation[A]: 35
Vassal Control: 62
```

Baseline relation decay contribution:

```text
(35 - 50) / 10 = -1.5 control/month
```

Player A has:

```text
2 armies stationed
→ +2 control/month
```

Player B performs:

```text
Fund Dissidents
→ Relation[A] 35 → 30
```

Next month relation decay becomes:

```text
(30 - 50) / 10 = -2 control/month
```

If B also uses Political Agitation:

```text
-3 immediate Vassal Control
```

Then the political contest becomes:

```text
Player A:
    armies / support / diplomacy

Player B:
    lower relation / lower control
```

without Player B directly owning any political share while the province remains a vassal.

---

# 111. Updated monthly espionage/interference order

Extend the monthly simulation with:

```text
A. Pay spy maintenance.
B. Remove unpaid spy missions.
C. Resolve spy detection.
D. Generate/update player scandal opportunities from current game state.
E. Generate NPC hidden scandals if below pool cap.
F. Resolve scandal discovery for surviving spies.
G. Expire old scandal inventory entries.
H. Apply recurring relation/control support and foreign-interference effects.
I. Continue with normal relation and vassal-control monthly resolution.
```

Immediate interference actions may execute during the turn, but their consequences must be visible in the monthly breakdown.

---

# 112. Additional espionage invariants

```text
one active spy per player per target province

spy deployment costs influence once

active spy costs coin every month

unpaid spy is withdrawn

detected spy is removed

NPC detection lowers relation

player detection generates a real scandal against spy owner

human-player scandals must originate from actual decisions/states

inactive scandal source cannot generate new discoveries

previously discovered scandal remains after source ends

same scandal source cannot be discovered twice by same player

one-time NPC trade does not generate control

NPC scandal may be consumed for control or favorable trade

player scandal is leverage, not direct resource/control damage
```

---

# 113. Additional interference invariants

```text
all interference targets a specific rival player

independent control reduction returns removed control to Local

vassal interference never creates independent player control while vassal state exists

Smear Campaign changes target Relation only

Fund Opposition changes target Independent Control only

Political Agitation changes Vassal Control only

Fund Dissidents changes Relation toward overlord only

distance modifies all interference costs

relation resistance applies to relation attacks

entrenchment applies to control attacks

Vassal Control reaching 0 triggers independence normally
```

---

# 114. Construction system

Construction is province-local.

Every province has exactly one construction slot:

```text
construction: None | BuildingProject | WonderProject
```

Therefore:

```text
maximum active construction projects per province = 1
```

A province cannot:

- build two standard buildings simultaneously;
- upgrade two buildings simultaneously;
- build a standard building while constructing a wonder.

Construction advances once per monthly tick.

---

# 115. Standard buildings

Standard buildings are repeatable province improvements.

Examples include:

```text
Granary
Warehouse
Aqueduct
Farm / Irrigation
Mine
Quarry
Road
Fort
```

City provinces additionally unlock city-only buildings defined in section 122.

Each building has:

```rust
struct BuildingDefinition {
    id: BuildingType,
    base_stone_cost: f32,
    base_metal_cost: f32,
    base_build_months: f32,
    cost_growth: f32,
    time_growth: f32,
    effects_per_level: BuildingEffects,
    requires_city: bool,
}
```

Province state stores only levels:

```rust
struct ProvinceBuildings {
    levels: HashMap<BuildingType, u32>,
}
```

Missing entry means level `0`.

---

# 116. Building resources

Buildings cost:

```text
Stone
Metal
```

Stone is normally the primary construction resource.

Typical rules:

```text
most buildings:
    high Stone cost
    lower Metal cost

some buildings:
    Stone only
```

A building may have `0` Metal cost.

Do not require Coin unless a future design explicitly adds it.

Construction costs are paid from the player's global resource stockpile.

---

# 117. Paying construction costs

For the initial implementation, pay the **entire resource cost when construction starts**.

Starting construction:

```text
player.stone -= stone_cost
player.metal -= metal_cost
province.construction = project
```

Construction may start only if:

```text
province has no active project
player has sufficient Stone
player has sufficient Metal
building prerequisites are met
```

Cancellation:

```text
project is deleted
already-paid resources are not refunded
```

This no-refund rule keeps construction accounting deterministic and prevents cancellation exploits.

A refund mechanic may be added later, but it is not part of this specification.

---

# 118. Unlimited building levels

Standard buildings have no fixed maximum level.

A player may build:

```text
Level 1
Level 2
Level 3
...
```

without a hard cap.

However, cost increases exponentially, making extreme levels progressively less efficient.

For upgrading from current level `L` to `L + 1`, where an unbuilt structure has `L = 0`:

```text
stone_cost(L + 1) =
    base_stone_cost × cost_growth^L

metal_cost(L + 1) =
    base_metal_cost × cost_growth^L
```

Recommended initial:

```text
cost_growth = 1.60
```

Round displayed/payable resource costs consistently to whole units.

Example for base Stone cost `100`:

```text
Level 1      100
Level 2      160
Level 3      256
Level 4      410
Level 5      655
Level 6    1,049
```

The implementation must not special-case a maximum level.

---

# 119. Building construction time

Construction duration also increases with level, but more slowly than cost.

```text
required_progress(L + 1) =
    base_build_months × time_growth^L
```

Recommended initial:

```text
time_growth = 1.20
```

The project tracks progress as a float:

```rust
struct BuildingProject {
    building: BuildingType,
    target_level: u32,
    progress: f32,
    required_progress: f32,
}
```

Normal building construction progresses:

```text
+1.0 progress per month
```

until:

```text
progress >= required_progress
```

Then:

1. increase building level;
2. clear construction slot;
3. recalculate province/global modifiers affected by the building.

---

# 120. Building progress UI

Every active construction project must expose:

```text
building/wonder name
current target level if applicable
progress
required progress
estimated remaining months
```

Example:

```text
Granary III → IV

Progress:
████████░░░░

4.0 / 7.0 months
Estimated remaining: 3 months
```

The progress bar represents construction progress, not resources paid.

Resources were already paid when the project started.

---

# 121. Building benefits

For normal buildings, prefer **linear benefits per level** combined with exponential costs.

This keeps effects easy to understand.

Examples:

## Granary

```text
each level:
    +global Food storage
```

## Warehouse

```text
each level:
    +global Food storage
    +global Metal storage
    +global Stone storage
```

## Aqueduct

```text
each level:
    +province population capacity
```

## Farm / Irrigation

```text
each level:
    +province Food production
and/or
    +province population capacity
```

## Mine

```text
each level:
    +province Metal production
```

## Quarry

```text
each level:
    +province Stone production
```

## Road

```text
each level:
    +trade-route efficiency for routes passing through province
    +migration attractiveness if configured
```

## Fort

```text
each level:
    +province military defense
```

Exact numerical effects belong in centralized balance configuration.

---

# 122. City provinces

The existing code currently marks these eight provinces as city provinces:

```text
Latium                  → Rome
Lugdunensis             → Lutetia
Tarraconensis           → Tarraco
Africa Proconsularis    → Carthage
Achaia                  → Athens
Aegyptus                → Alexandria
Asia                    → Ephesus
Syria                   → Antioch
```

These existing city mappings are authoritative unless changed in the map data.

City provinces may build all normal buildings plus city-only buildings.

---

# 123. City-only buildings

City-only buildings require:

```text
province.has_city == true
```

Initial categories may include:

```text
Forum
Baths
Temple
Arena
Urban Market
City Walls
```

Suggested roles:

## Forum

```text
+passive Influence generation
```

## Baths

```text
+population capacity
+population happiness
```

## Temple

```text
+happiness
+passive Influence generation
```

## Arena

```text
+happiness
```

## Urban Market

```text
+province tax / Coin generation
+trade-related modifier
```

## City Walls

```text
+province military defense
```

These are examples of the standard city building set. Exact numeric values belong in balance configuration.

City-only buildings use the **same single construction slot** as normal buildings.

---

# 124. Wonder sites use the existing repository data

Do not invent additional wonders.

The canonical wonder set is the existing `WONDERS` array in:

```text
src/map/map.rs
```

The current repository defines exactly these 10 wonder sites:

```text
1. Great Pyramid of Giza
2. Oracle of Dodona
3. Stonehenge
4. Acropolis of Pergamon
5. Temple of Zeus at Olympia
6. Palace of the Argeads
7. Mausoleum at Halicarnassus
8. Colossus of Rhodes
9. Aqueduct of Segovia
10. Pont du Gard
```

Their existing:

```text
name
map position
image asset
```

must be reused.

Do not duplicate wonder coordinates in a second hardcoded gameplay table if avoidable.

Instead extend the existing wonder definition, or map gameplay data to the existing wonder identifier/name.

The current map code already treats each wonder as a concrete geographic site and renders it at its defined position.

---

# 125. Wonder eligibility by province

A wonder may only be built in the province that contains its existing map position.

At game initialization or map-load time:

```text
wonder_site_province =
    province containing WONDERS[wonder].position
```

Cache this association.

Therefore:

- many provinces have no wonder site;
- some provinces have a wonder site;
- a wonder cannot be moved to another province;
- the current repository coordinates remain authoritative.

The existing repository already verifies, for example, that the Colossus of Rhodes lies in the `Asia` province geometry.

Do not manually assign a different province.

---

# 126. Maximum one wonder per province

A province may contain at most one completed wonder.

```text
max_completed_wonders_per_province = 1
```

A province with no canonical wonder site cannot construct one.

A province with a canonical site may construct only that site's existing wonder.

If future repository data ever puts multiple candidate wonders in the same province, the first completed wonder permanently blocks the others unless this rule is intentionally redesigned.

For the current 11-site set, use the existing geographic sites as the source of truth.

---

# 127. Wonder construction prerequisites

To begin construction:

```text
province is owned by player
province contains canonical wonder site
wonder is not already completed
province does not already contain another completed wonder
province construction slot is empty
player has required Stone
player has required Metal
```

A wonder does **not** require a city unless explicitly configured for that specific wonder later.

Do not infer city requirements merely from historical location.

---

# 128. Wonder costs

Wonders are much more expensive than standard buildings.

They require:

```text
large Stone cost
large Metal cost
```

Stone should normally be the larger component.

Example scale only:

```text
Stone: 2,000–5,000+
Metal:   300–1,000+
```

Exact cost is stored per wonder in configuration.

Unlike standard buildings:

```text
wonder has no level
wonder cannot be upgraded
```

The resources are paid in full when construction begins.

Cancellation:

```text
wonder project removed
no resource refund
```

---

# 129. Wonder construction time

Wonders take much longer than standard buildings.

Recommended base range:

```text
24–60 months
```

Each wonder has:

```rust
struct WonderGameplayDefinition {
    wonder_id: WonderId,
    stone_cost: f32,
    metal_cost: f32,
    required_progress: f32,

    completion_influence: f32,
    monthly_influence: f32,
}
```

Wonder project:

```rust
struct WonderProject {
    wonder_id: WonderId,
    progress: f32,
    required_progress: f32,
    assigned_slaves: f32,
}
```

---

# 130. Slaves assigned to wonder construction

Wonder construction can be accelerated with slaves from that province.

The player selects:

```text
assigned_slaves
```

Subject to:

```text
0 <= assigned_slaves <= province.slaves
```

Assigned slaves remain part of the province population.

They still:

- consume Food;
- have happiness;
- experience births/deaths according to the applicable systems.

However, they are removed from normal resource-production labor.

When calculating productive labor:

```text
productive_slaves =
    total_slaves - wonder_assigned_slaves
```

Then:

```text
productive_labor =
    plebeians × PLEBEIAN_PRODUCTIVITY
    + productive_slaves
      × SLAVE_PRODUCTIVITY
      × slave_labor_modifier
```

This opportunity cost is mandatory.

---

# 131. Wonder construction speed from slaves

Assigned slaves increase monthly wonder progress with diminishing returns.

Do **not** use unlimited linear scaling.

Recommended initial discrete speed table:

```text
Assigned slaves      Progress/month

0–24                 1.00
25–49                1.25
50–99                1.50
100–199              1.75
200+                  2.00
```

This table is configurable.

Each month:

```text
wonder.progress += slave_speed_multiplier
```

Completion occurs when:

```text
progress >= required_progress
```

Example:

```text
required progress = 48

0 slaves:
    1.00/month
    about 48 months

50 slaves:
    1.50/month
    about 32 months

100 slaves:
    1.75/month
    about 28 months

200 slaves:
    2.00/month
    about 24 months
```

Changing slave assignment changes speed starting with the next monthly construction tick.

Do not precompute an immutable completion date.

---

# 132. Slave availability changes during wonder construction

Assigned slave count must remain valid if slave population changes.

Before every monthly wonder tick:

```text
assigned_slaves =
    min(assigned_slaves, current_slave_population)
```

Examples that can reduce available slaves:

- deaths;
- famine;
- war losses;
- manumission;
- other future mechanics.

If slave population falls below the configured assignment, automatically lower the assignment rather than creating negative productive slaves.

The UI must show the updated assigned amount and estimated completion.

---

# 133. Wonder construction and slave mortality

For the first implementation, wonder assignment itself does **not** add a separate death modifier beyond the existing slave-labor and food systems.

Reason:

- the required opportunity cost is already lost resource production;
- separate wonder mortality is not yet a decided mechanic.

Do not implement the previously suggested `wonder slave death ×1.25` unless separately approved later.

---

# 134. Wonder rewards

Wonder rewards are intentionally simple.

A completed wonder gives exactly:

```text
1. a large one-time Influence reward on completion;
2. passive Influence every month thereafter.
```

There are currently **no other wonder secondary effects**.

Do not add:

- trade bonuses;
- happiness bonuses;
- storage bonuses;
- production bonuses;
- relation bonuses;
- military bonuses;
- migration bonuses;
- unique per-wonder abilities.

Those may be designed later.

Example scale:

```text
completion Influence:
    large, e.g. 200–500

monthly passive Influence:
    significant, e.g. +2 to +5/month
```

Exact values are configured per wonder or shared globally.

---

# 135. Wonder Influence ownership

The one-time completion reward goes to:

```text
the player who owns the province when construction completes
```

It is granted once.

The passive monthly Influence belongs to the current direct owner of the province containing the completed wonder.

If another player later conquers the province:

```text
new owner receives the passive monthly Influence
new owner does NOT receive the old one-time completion reward
```

The wonder is therefore a permanent strategic asset attached to the province.

---

# 136. Wonders in vassal provinces

A completed wonder belongs to the province.

If the province is a vassal rather than directly owned:

```text
the overlord does not automatically receive the wonder's full monthly Influence
```

For the initial implementation, passive wonder Influence is generated for the political entity that directly owns/administers the province.

If NPC-vassal Influence is not otherwise represented, the passive Influence may simply not transfer to the overlord.

Do not silently grant it to the overlord unless a later tribute rule explicitly does so.

This avoids bypassing the distinction between direct ownership and vassalage.

---

# 137. Wonder construction on ownership change

If ownership changes while a wonder is under construction:

Recommended initial rule:

```text
construction remains attached to the province
progress is preserved
assigned_slaves are reset to 0
```

The new owner may:

```text
continue construction
change slave assignment
cancel it
```

The original builder receives no refund.

If the province becomes a vassal during construction, preserve the project and reset assigned slaves to `0` until its controlling entity resumes/sets assignment.

This makes conquest of an unfinished monumental project strategically meaningful.

---

# 138. Standard construction on ownership change

Use the same simple rule for ordinary buildings:

```text
construction project remains
progress remains
new owner may continue or cancel
```

All previously paid resources are sunk.

This avoids difficult refund/accounting logic.

---

# 139. Building and wonder monthly update order

Insert construction into the main monthly order as follows:

```text
1. Validate active construction.

2. For wonder projects:
   - clamp assigned slaves to current slave population;
   - remove assigned slaves from productive-labor calculation.

3. Calculate province resource production.

4. Execute the rest of the monthly economy/population systems.

5. Advance construction:
   - normal building: +1 progress;
   - wonder: +slave-speed progress.

6. Resolve completed projects.

7. On normal-building completion:
   - increment level;
   - clear construction;
   - recalculate affected modifiers.

8. On wonder completion:
   - set completed wonder on province;
   - grant one-time Influence;
   - clear construction;
   - assigned slaves return to normal labor automatically next month.

9. During monthly Influence generation:
   - add passive Influence from all completed wonders directly owned by player.
```

Important:

Wonder-assigned slaves must be excluded from production for the entire month in which they work on the wonder.

---

# 140. Construction UI requirements

Province construction panel must show:

```text
Available standard buildings
Current level of each building
Next-level Stone cost
Next-level Metal cost
Next-level construction time
Effect of next level

City-only buildings if province has city

Canonical wonder if province has a wonder site
Wonder cost
Wonder progress
Wonder Influence reward
Wonder monthly passive Influence
Assigned slaves
Current speed multiplier
Estimated months remaining
```

Active construction should always be visible from the province overview.

---

# 141. Wonder map integration

Reuse the existing wonder map data and artwork.

The repository currently stores:

```text
WonderAsset {
    name,
    position,
    png,
}
```

The construction system should extend or associate gameplay data with these same entries.

A completed wonder should continue using the existing wonder marker/art system.

Before construction completes, the UI may:

```text
show the site marker differently
or
show the existing marker with an "unbuilt" state
```

but do not create duplicate geographic positions.

---

# 142. Canonical wonder list

The exact currently-defined wonder set is:

```text
Great Pyramid of Giza
Oracle of Dodona
Stonehenge
Acropolis of Pergamon
Temple of Zeus at Olympia
Palace of the Argeads
Mausoleum at Halicarnassus
Colossus of Rhodes
Aqueduct of Segovia
Pont du Gard
```

This list comes from the current repository's `WONDERS` constant and is authoritative for implementation.

If that constant changes later, gameplay should use the updated repository list rather than keeping this document's list as a divergent source of truth.

---

# 143. Construction invariants

The implementation must enforce:

```text
one active construction project per province

normal building level >= 0

normal buildings have no hard maximum level

building upgrade target = current level + 1

construction progress >= 0

construction costs are paid in full at project start

canceling a project gives no refund

Stone and Metal cannot become negative when starting construction

city-only buildings require a city

province without a canonical wonder site cannot build a wonder

maximum one completed wonder per province

wonder has no levels

wonder assigned slaves >= 0

wonder assigned slaves <= current province slave population

wonder assigned slaves do not contribute to resource production

wonder slave acceleration has diminishing returns

wonder completion gives one-time Influence exactly once

completed wonder gives passive monthly Influence

no other wonder secondary bonuses exist yet
```

---

# 144. Construction configuration

Centralized balance configuration must contain:

```text
per standard building:
    base Stone cost
    base Metal cost
    base build time
    cost-growth factor
    time-growth factor
    effect per level
    city-only flag

global/default:
    standard building cost growth
    standard building time growth

per wonder:
    reference to canonical existing wonder
    Stone cost
    Metal cost
    required progress
    one-time completion Influence
    passive Influence/month

wonder slave speed thresholds
```

Do not hardcode balance constants directly in UI or monthly systems.

---

# 145. Updated explicit non-goals for construction

Do not implement unless separately requested:

```text
multiple parallel construction slots in one province
construction queues longer than the single active project
worker assignment to normal buildings
Coin cost for normal buildings
maintenance cost for buildings
building decay
hard maximum building levels
unique wonder gameplay bonuses besides Influence
new wonders not currently present in the repository
wonder relocation
more than one wonder per province
gradual monthly resource payment for construction
automatic resource refunds
```


---

# 146. Revised vassalization threshold and control conversion

The vassalization rules are revised.

A player may vassalize an independent province when:

```text
player independent control >= 51
```

The player must still be the unique leading controller.

Example:

```text
A 51
B 49
```

A may vassalize.

At vassalization:

```text
new_vassal_control =
    old_independent_control - 50
```

Examples:

```text
Vassalize at 51 independent control
→ start with 1 Vassal Control

Vassalize at 65
→ start with 15 Vassal Control

Vassalize at 90
→ start with 40 Vassal Control

Vassalize at 100
→ start with 50 Vassal Control
```

This replaces the previous rule where vassal control started equal to the old independent-control value.

The first 50 control represents the political effort required to create the vassal relationship.

Any control above 50 becomes the initial strength of the new vassal relationship.

---

# 147. Vassalization resets competing independent control

When a player vassalizes an independent province:

```text
all other players' independent control = 0
local independent control = 0
independent-control state is removed
```

The province changes to:

```text
Vassal {
    overlord,
    vassal_control,
    tribute,
    ...
}
```

All `Relation[player]` values are retained.

Example:

```text
Before:

Local       10
Player A    60
Player B    25
Player C     5
```

Player A vassalizes.

Result:

```text
Vassal of A

Vassal Control:
60 - 50 = 10

Player B independent control:
removed

Player C independent control:
removed

Relations:
unchanged
```

Other players may later destabilize the vassal through relation/control interference, but they do not retain old independent-control shares.

---

# 148. Vassal control range and purpose

Vassal Control remains:

```text
0..100
```

It now serves three purposes:

```text
1. determines whether the province remains a vassal;
2. determines passive Influence generated for the overlord;
3. unlocks direct ownership at 100.
```

At:

```text
Vassal Control <= 0
```

the province becomes independent.

At:

```text
Vassal Control >= 100
```

the overlord may choose to integrate the province into direct ownership.

Do not automatically integrate at 100.

The player must explicitly choose:

```text
Take Ownership
```

---

# 149. Passive Influence from vassals

Every vassal generates passive monthly Influence for its overlord.

The amount increases with Vassal Control.

Recommended initial formula:

```text
monthly_vassal_influence =
    BASE_VASSAL_INFLUENCE
    + vassal_control × VASSAL_INFLUENCE_PER_CONTROL
```

Recommended initial defaults:

```text
BASE_VASSAL_INFLUENCE = 0
VASSAL_INFLUENCE_PER_CONTROL = 0.05
```

Examples:

```text
Control 20  → +1.0 Influence/month
Control 40  → +2.0
Control 60  → +3.0
Control 80  → +4.0
Control 100 → +5.0
```

Keep values configurable.

This means increasing vassal control has an ongoing political reward even before full integration.

Passive vassal Influence is added during the normal monthly Influence-generation step.

---

# 150. Increasing Vassal Control

Once a province is a vassal, the overlord may increase Vassal Control through the already-defined mechanisms:

```text
stationed armies
direct control support with Coin
direct control support with Influence
positive relation reducing/ending passive decay
events
future political effects
```

Enemies can reduce Vassal Control through:

```text
Political Agitation
other approved direct-control interference
```

Enemies may also indirectly weaken control by lowering relation toward the overlord.

The normal monthly formula remains:

```text
vassal_control_change =
    relation_control_change
    + stationed_army_bonus
    + overlord_control_support
    - enemy_control_interference
    + event_modifiers
```

Then:

```text
vassal_control =
    clamp(vassal_control + change, 0, 100)
```

---

# 151. Vassal Control at 100 — integration option

When:

```text
vassal_control == 100
```

the overlord gains an explicit action:

```text
Take Ownership
```

This changes political state:

```text
Vassal → Owned
```

On integration:

```text
owner = former overlord
vassal state is removed
vassal control is removed
tribute is removed
vassal-specific passive Influence ends
```

The province now behaves as a normal directly owned province.

The vassal-control value does not continue to exist after ownership.

---

# 152. Relation-to-happiness conversion on integration

When a vassal becomes directly owned, the province's relation toward the former overlord is converted into local population happiness.

This provides continuity between foreign political sentiment and domestic population sentiment.

Let:

```text
integration_relation =
    relation[former_overlord]
```

Recommended initial conversion:

```text
new_class_happiness =
    clamp(
        current_class_happiness
        + (integration_relation - 50),
        0,
        100
    )
```

This means:

```text
Relation 80
→ +30 happiness shift

Relation 50
→ no happiness shift

Relation 20
→ -30 happiness shift
```

Apply the shift to:

```text
Nobles
Citizens
Plebeians
Slaves
```

unless future class-specific conversion modifiers are introduced.

After conversion:

```text
relation toward former overlord is no longer used for owned-province politics
```

The province is domestic territory and uses class happiness instead.

This ensures:

```text
friendly integration
→ relatively happy new province

coercive integration
→ unhappy newly owned province
```

---

# 153. Owned provinces cannot be externally controlled

Once a province is directly owned:

```text
enemy players cannot gain Independent Control
enemy players cannot gain Vassal Control
```

The province no longer participates in foreign Control competition.

Therefore the following actions are unavailable against an owned province:

```text
Buy Support for Control
Political Campaign for Control
Fund Opposition against Control
Political Agitation against Vassal Control
Vassalize
```

Ownership is not reversed through the Control system.

Changing ownership later requires:

```text
war
rebellion
explicit future political systems
```

not ordinary control accumulation.

---

# 154. Enemy political actions against owned provinces

Enemy political interference against an owned province targets **population happiness**, not Control.

The main peaceful/hostile political objective becomes:

```text
make the owner's population unhappy
```

Recommended initial action:

```text
Agitate Population
```

Target:

```text
specific owned province
specific enemy owner
```

Cost:

```text
Influence
```

Effect:

```text
reduce one or more pop-class happiness values
```

Recommended initial implementation:

```text
Base cost: 10 Influence
Effect:    -5 happiness to Citizens and Plebeians
```

Apply normal political-distance modifiers.

Future variants may target:

```text
Nobles
Citizens
Plebeians
Slaves
```

separately.

For the initial system, Citizens + Plebeians is sufficient.

---

# 155. Fund Unrest in owned provinces

A Coin-based alternative is:

```text
Fund Unrest
```

Recommended initial effect:

```text
Base cost: 100 Coin
Effect:    -5 happiness to Plebeians
```

This represents:

```text
funding opposition
bribing local agitators
supporting strikes/riots
distributing hostile propaganda
```

It does not create Control.

---

# 156. Espionage against owned provinces

Spies remain valid in owned enemy provinces.

Their primary purposes are:

```text
discover active player scandals
discover bad domestic conditions
support future political leverage
```

A spy does not create Control.

If a player runs:

```text
Low Food
High Taxes
Harsh Slave Labor
creates severe unhappiness
causes famine
```

these remain discoverable scandal opportunities under the existing espionage rules.

This works naturally with the owned-province system because enemies can no longer politically "buy" an owned province away; instead they gather scandals and worsen domestic stability.

---

# 157. Relation after ownership

`Relation[player]` is primarily a foreign-political value.

Once a province becomes directly owned:

```text
the owner's former relation value is consumed into population happiness during integration
```

Relations toward other foreign players may remain stored if useful for:

```text
migration
future diplomacy
occupation/rebellion
events
```

but they do not grant those players Control.

Do not convert every foreign relation into happiness.

Only the relation toward the integrating player is transformed when ownership changes.

---

# 158. Updated political lifecycle

The complete progression is now:

```text
INDEPENDENT
    ↓
players compete for Independent Control
    ↓
51+ and unique leader
    ↓
VASSALIZE
    ↓
Vassal Control = Independent Control - 50
all rival control removed
    ↓
VASSAL
    ↓
Vassal Control rises/falls over time
    ├─ reaches 0
    │     ↓
    │  INDEPENDENT
    │
    └─ reaches 100
          ↓
       player may choose Take Ownership
          ↓
       Relation → Pop Happiness
          ↓
         OWNED
```

Once `Owned`:

```text
no foreign Control accumulation
enemy politics target happiness/scandals instead
```

---

# 159. Vassal Influence lifecycle

While Vassal:

```text
monthly passive Influence =
    function(Vassal Control)
```

At:

```text
Control 100
```

the vassal gives maximum passive vassal Influence.

If the player chooses `Take Ownership`:

```text
vassal passive Influence stops
```

The directly owned province then provides only normal Influence sources:

```text
Nobles
city buildings
wonders
other domestic mechanics
```

This makes the choice between:

```text
keeping a highly controlled vassal
versus
directly integrating it
```

strategically meaningful.

---

# 160. Wonder construction animation requirement

Every existing canonical wonder must have a dedicated **construction-state animated sprite** generated during implementation.

This is separate from the existing completed-wonder artwork.

For each wonder in the repository's canonical `WONDERS` array, implementation must create or generate:

```text
completed wonder sprite/art        # existing wonder artwork
construction animation sprite      # new requirement
```

The construction animation must visually communicate:

```text
the wonder is only partially built
workers are actively constructing it
scaffolding / building activity is visible where appropriate
```

Examples of acceptable visual elements:

```text
workers carrying stone
workers moving around the site
scaffolding
partially completed columns/walls/statue
stone blocks
construction equipment appropriate to the setting
```

The animation must depict the specific wonder under construction rather than use one generic construction icon for every wonder.

---

# 161. Wonder construction animation generation

At implementation time, generate a construction sprite/animation for **every canonical wonder**.

Canonical wonders remain exactly those defined in the repository's existing `WONDERS` array.

Do not add additional wonders merely to support the animation system.

Recommended asset organization:

```text
assets/wonders/<wonder_id>/completed.*
assets/wonders/<wonder_id>/construction.*
```

or the closest structure consistent with the current repository.

The generated construction art should:

```text
match the visual style of the existing game
be readable at the map's wonder-rendering scale
loop cleanly
avoid excessive frame count
show active workers
show incomplete construction
```

Recommended initial animation size:

```text
4–8 frames
```

unless the existing asset pipeline strongly favors another format.

---

# 162. Wonder construction rendering

Wonder rendering state:

```text
Not started
Under construction
Completed
```

## Not started

The site may:

```text
show no wonder
or
show a subtle site marker
```

depending on existing map UI.

## Under construction

Render:

```text
the wonder-specific animated construction sprite
```

The animation must remain visible while the wonder project is active.

## Completed

Render:

```text
the existing completed-wonder artwork
```

Do not show the construction sprite after completion.

---

# 163. Wonder animation and progress

The construction animation does not need to exactly represent numeric completion percentage.

For the initial implementation:

```text
one looping construction animation per wonder
```

is sufficient throughout the construction period.

Optional later enhancement:

```text
early / middle / late construction variants
```

may use progress thresholds, but this is not required.

The numeric progress bar remains the authoritative construction-progress UI.

---

# 164. Wonder construction assets and repository implementation

The implementation agent must:

```text
1. enumerate the canonical WONDERS array;
2. ensure every wonder has a construction-animation asset;
3. generate missing construction assets;
4. integrate them into the game's asset-loading pipeline;
5. render construction animation while WonderProject is active;
6. render existing completed artwork after completion;
7. add tests/checks ensuring every canonical wonder has both required asset states.
```

If the project uses compressed/runtime-optimized image formats, generated construction assets must follow the same asset-processing pipeline as other game images.

Do not leave individual wonders using placeholders once implementation is complete.

---

# 165. Revised political invariants

Replace earlier vassalization/integration invariants with:

```text
Independent → Vassal requires control >= 51 and unique lead

On vassalization:
    vassal_control = previous_independent_control - 50
    all independent control is removed
    all rival independent control is lost
    relations are retained

0 <= vassal_control <= 100

Vassal Control <= 0:
    province becomes Independent
    Local control = 100
    all player independent control = 0

Vassal Control generates passive Influence for overlord

Vassal Control == 100:
    Take Ownership action becomes available

Taking Ownership:
    removes vassal control
    removes tribute
    removes vassal passive Influence
    converts relation toward former overlord into pop happiness
    changes province to Owned

Owned province:
    cannot accumulate foreign Independent Control
    cannot accumulate foreign Vassal Control
    enemy peaceful political interference targets pop happiness instead

Wonder under construction:
    render wonder-specific construction animation

Completed wonder:
    render completed wonder art
    grant only Influence rewards currently specified
```

---

# 166. Revised monthly Influence generation

Player monthly Influence now includes:

```text
Influence from normal domestic sources
+ Influence from relevant city buildings
+ passive Influence from completed directly-owned wonders
+ passive Influence from vassals based on Vassal Control
+ other future modifiers
```

For each vassal:

```text
monthly_vassal_influence =
    control × VASSAL_INFLUENCE_PER_CONTROL
```

or the configured equivalent.

Calculate this using the final Vassal Control value for the monthly tick consistently.

Recommended:

```text
use post-control-update value
```

so destabilizing or strengthening a vassal immediately affects that month's final Influence summary.

---

# 167. Revised implementation non-goals

Do not implement unless separately requested:

```text
foreign Control accumulation in directly owned provinces

automatic ownership at 100 Vassal Control

retaining Vassal Control after integration

retaining vassal passive Influence after direct ownership

turning all foreign Relation values into pop happiness

generic one-size-fits-all wonder construction sprite

new wonders beyond the repository's canonical WONDERS array

multiple construction-stage animations per wonder
unless easy and intentionally added later
```

---

# 168. Optional vassalization versus direct ownership

Reaching the vassalization threshold does **not** force vassalization.

For an independent province:

```text
Independent Control >= 51
```

gives the player the option:

```text
Vassalize
```

The player may instead leave the province independent and continue competing for Control.

If the player reaches:

```text
Independent Control == 100
```

they may choose:

```text
Take Ownership
```

directly, without first creating a vassal.

Therefore there are two valid expansion paths:

```text
51+ Control
→ Vassalize early
→ grow Vassal Control toward 100
→ optionally integrate later
```

or:

```text
continue Independent Control competition
→ reach 100
→ Take Ownership directly
```

The UI should clearly expose both choices when applicable.

---

# 169. Simultaneous Independent Control resolution

Independent Control gains in the same monthly resolution must be **order-independent**.

Do not resolve Player A completely before Player B, because execution order would create unfair first-mover advantage.

All Control-changing actions scheduled for the same monthly resolution are aggregated first.

For each independent province:

```text
attempted_gain[player] =
    sum of that player's valid positive Control-gain actions this month
```

Examples include:

```text
Buy Support
Political Campaign
eligible recurring trade Control
other future positive Control actions
```

Negative targeted actions such as `Fund Opposition` and `Undermine Rival` are resolved in the same political-resolution phase using their explicit targets.

---

# 170. Allocate remaining Local Control proportionally

Positive Control gains first compete for remaining `Local` Control.

Let:

```text
local_available = Local Control

total_attempted_gain =
    sum(attempted_gain[player])
```

If:

```text
total_attempted_gain <= local_available
```

every player receives their full attempted gain from Local.

If:

```text
total_attempted_gain > local_available
```

allocate Local proportionally:

```text
local_gain[player] =
    local_available
    × attempted_gain[player]
    / total_attempted_gain
```

Then:

```text
remaining_pressure[player] =
    attempted_gain[player] - local_gain[player]
```

After this step:

```text
Local = max(0, Local - sum(local_gain))
```

---

# 171. Resolve competing Control pressure

Any positive Control pressure remaining after Local reaches zero represents attempts to take Control from rival players.

For two players, opposing pressure cancels directly.

Example:

```text
Start:
Local 10
A     45
B     45

Attempts:
A +10
B +10
```

Local allocation:

```text
A +5
B +5
Local → 0
```

Temporary:

```text
A 50
B 50
```

Remaining pressure:

```text
A 5
B 5
```

The two pressures cancel.

Final:

```text
A 50
B 50
```

Neither player receives an advantage from processing order.

---

# 172. Unequal simultaneous Control example

Start:

```text
Local 10
A     45
B     45
```

Attempts:

```text
A +10
B +5
```

Local is distributed in the same `2:1` ratio:

```text
A receives 6.6667
B receives 3.3333
```

Temporary:

```text
A     51.6667
B     48.3333
Local  0
```

Remaining pressure:

```text
A 3.3333
B 1.6667
```

Opposing pressure cancels:

```text
net A pressure = 1.6666
```

Apply that against B:

```text
A ≈ 53.3333
B ≈ 46.6667
```

A may now choose to vassalize, or may continue toward `100`.

If A vassalizes at approximately `53.3333`:

```text
initial Vassal Control ≈ 3.3333
```

because the first `50` Control is consumed by vassalization.

---

# 173. Multi-player simultaneous Control resolution

For more than two players, use a deterministic simultaneous algorithm.

Recommended implementation:

1. Aggregate every player's positive attempted gain.
2. Allocate remaining Local proportionally.
3. Compute remaining positive pressure for each player.
4. Apply explicitly targeted negative actions first to their specified targets.
5. For untargeted remaining positive pressure:
   - repeatedly target the current largest rival Control holder;
   - calculate all such transfers from the same pre-transfer snapshot for that iteration;
   - apply transfers simultaneously;
   - repeat until no positive pressure remains or no rival Control is available.
6. Normalize final Control values to total exactly `100` within float tolerance.
7. Only after final values are resolved, evaluate:
   - vassalization availability (`>=51` and unique leader);
   - direct ownership availability (`==100`).

Stable `PlayerId` ordering is used only to resolve exact ties in target selection, never as a first-mover advantage.

---

# 174. Notification / toast system

Important political, espionage, and wonder events generate clickable toasts.

Toast severity used in this specification:

```text
Info
Warning
```

Every toast contains:

```text
severity
title
short body
event type
related province if applicable
related wonder if applicable
related spy/scandal if applicable
click action
created-at timestamp / game month
```

Toasts must not pause the simulation.

They remain visible long enough to be noticed and should also be available in a recent-notifications/history panel if the UI already supports one or if one is added.

---

# 175. Wonder-start warning toast

When an **enemy player** starts construction of a wonder:

```text
severity: Warning
```

Example:

```text
Player B has begun constructing the Colossus of Rhodes.
```

Trigger exactly once when the wonder project starts.

Do not show the warning for the local player's own wonder.

Click behavior:

```text
center map on wonder site
zoom in to the wonder
```

The camera target should be the canonical wonder position from the repository's `WONDERS` data.

If the wonder is under construction, the player should see its construction animation after zooming.

---

# 176. Wonder-completed warning toast

When an **enemy player** completes a wonder:

```text
severity: Warning
```

Example:

```text
Player B has completed the Colossus of Rhodes.
```

Trigger exactly once on completion.

Click behavior:

```text
center map on wonder site
zoom in to the completed wonder
```

The completed wonder artwork must be visible at the destination.

---

# 177. Control-threshold info toast

When the local player reaches either important Independent Control threshold in a province:

```text
50 Control
100 Control
```

show:

```text
severity: Info
```

The `50` toast represents approaching/entering vassalization territory even though actual vassalization requires `>=51`.

Recommended wording:

```text
Control in Achaia has reached 50.
```

At `51`, the province panel itself should expose the `Vassalize` action if the player is the unique leader.

At `100`:

```text
You now have full Control of Achaia.
```

Click behavior for both:

```text
open the province panel
focus/select that province
```

The `100` province panel exposes:

```text
Take Ownership
```

if the province is still independent.

Do not spam repeated threshold toasts if Control oscillates around the threshold every month.

Recommended trigger rule:

```text
show when crossing upward from below threshold to >= threshold
```

Optionally allow a new toast only after Control has first fallen below the threshold again.

---

# 178. Spy-uncovered warning toast

When one of the local player's spies is discovered:

```text
severity: Warning
```

Example:

```text
Our spy network in Italia has been uncovered.
```

Include consequences in the body when known:

```text
Relation with Italia decreased by 10.
```

or, against another player:

```text
Player B gained an espionage scandal against us.
```

Click behavior:

```text
open/select the target province panel
```

If a dedicated espionage panel exists, opening it with the failed mission selected is also acceptable.

---

# 179. Scandal-discovered info toast

When one of the local player's active spies discovers a scandal:

```text
severity: Info
```

Example:

```text
Our spies discovered a scandal in Sicilia: Low Food Supply.
```

For an NPC:

```text
Our spies uncovered corruption in Achaia.
```

Click behavior:

```text
open the scandal / espionage view
select the newly discovered scandal
```

If no dedicated scandal view exists, open the target province panel and highlight the newly acquired scandal.

Trigger once per newly discovered scandal.

---

# 180. Vassal-Control-drop warning toast

When one of the local player's vassals loses Vassal Control during monthly resolution:

```text
severity: Warning
```

Recommended trigger:

```text
final Vassal Control this month
<
previous month's final Vassal Control
```

Example:

```text
Control over Cappadocia is weakening: 61 → 58.
```

Click behavior:

```text
open the vassal province panel
```

The panel should show the Control-change breakdown:

```text
relation decay
stationed armies
support spending
enemy interference
events
```

To avoid noise, aggregate all same-month causes into one toast per vassal.

---

# 181. Owned-province happiness-drop warning toast

Because directly owned provinces no longer have foreign Control, enemy interference manifests through pop happiness.

When enemy action causes a material drop in one or more local pop-class happiness values:

```text
severity: Warning
```

Recommended trigger:

```text
enemy-caused aggregate happiness loss >= configured threshold
```

Recommended initial threshold:

```text
5 happiness points in one monthly resolution
```

Example:

```text
Enemy agitation has reduced happiness in Sicilia.
```

Click behavior:

```text
open the owned province panel
```

Show the happiness breakdown and hostile-interference source if attribution is known.

---

# 182. Relation-drop-to-hostile warning toast

When a province's relation toward the local player crosses into `Hostile`:

```text
previous relation >= 40
new relation < 40
```

show:

```text
severity: Warning
```

Example:

```text
Relations with Achaia have become hostile.
```

Click behavior:

```text
open the province panel
```

Use the existing relation bands:

```text
0–19     Very Hostile
20–39    Hostile
40–59    Neutral
60–79    Friendly
80–100   Very Friendly
```

Only toast on downward band crossing into Hostile/Very Hostile, not every month relation remains there.

A further crossing:

```text
20 → 19
```

may generate another warning:

```text
Relations with Achaia are now very hostile.
```

---

# 183. Relation-drop warning for vassals

For a local player's vassal, relation degradation is strategically important because it increases passive Vassal Control decay.

When relation toward the overlord crosses:

```text
50 downward
```

show a warning even though `40` is the normal Hostile-band boundary.

Example:

```text
Cappadocia's relation has fallen below 50; Vassal Control will now decay.
```

Click behavior:

```text
open vassal province panel
```

This is in addition to the general Hostile toast if it later crosses below `40`.

---

# 184. Toast deduplication and aggregation

The notification system must avoid spam.

Rules:

```text
Wonder start:
    once per construction start

Wonder complete:
    once per completion

Control 50:
    upward threshold crossing only

Control 100:
    upward threshold crossing only

Spy uncovered:
    once per spy discovery/removal

Scandal found:
    once per scandal acquisition

Vassal Control drop:
    at most once per vassal per monthly tick

Relation becomes Hostile:
    once per downward band crossing

Relation below 50 for local vassal:
    once per downward crossing

Owned-pop happiness loss from enemy actions:
    at most once per province per monthly tick
```

If multiple enemy actions produce the same monthly Vassal Control or happiness decline, summarize them in one toast.

---

# 185. Toast camera/navigation API

Implement toast click actions through reusable navigation commands rather than toast-specific map code.

Recommended abstractions:

```rust
enum NotificationAction {
    FocusWonder(WonderId),
    OpenProvince(ProvinceId),
    OpenSpyMission(SpyMissionId),
    OpenScandal(ScandalId),
}
```

`FocusWonder` must:

```text
select relevant province/site if appropriate
center camera on canonical wonder position
zoom to configured wonder-focus zoom level
```

`OpenProvince` must:

```text
select province
open province panel
```

This keeps notification creation independent from UI navigation implementation.

---

# 186. Notification event sources

Game systems should emit domain events.

Examples:

```text
WonderConstructionStarted
WonderCompleted
IndependentControlChanged
SpyDiscovered
ScandalDiscovered
VassalControlChanged
RelationChanged
OwnedPopHappinessChangedByForeignAction
```

The notification system subscribes to these events and decides whether they concern the local player and satisfy toast-trigger conditions.

Do not scatter direct UI-toast calls throughout simulation code.

---

# 187. Notification implementation invariants

```text
enemy wonder start/completion creates Warning toast

wonder toast click centers and zooms to wonder

local-player Control upward crossing of 50 creates Info toast

local-player Control upward crossing of 100 creates Info toast

Control toast click opens target province panel

local spy uncovered creates Warning toast

local spy scandal discovery creates Info toast

local vassal losing Control creates Warning toast

relation crossing into Hostile creates Warning toast

local vassal relation crossing below 50 creates Warning toast

owned province enemy-caused material happiness loss creates Warning toast

same event must not generate duplicate toasts

notification generation must not affect simulation outcome
```

---

# 188. Rome political system

Clicking Rome opens a special political panel instead of only the normal province view.

```text
ROME
├─ Senate
└─ Cursus Honorum
```

The Rome panel is the central UI for:

- political rank;
- nomination for the next Senate ballot;
- Senate blocs;
- campaigning;
- scandals;
- current Consuls;
- Proconsuls;
- Augustus candidacy;
- recent Senate results;
- victory.

---

# 189. Cursus Honorum

The political ladder is:

```text
No Office
    ↓
Aedile
    ↓
Praetor
    ↓
Consul
    ↓
Augustus
```

A former Consul becomes:

```text
Proconsul
```

A Proconsul may later become Consul again, but cannot become Augustus unless first reelected Consul.

---

# 190. Rank Influence income

Recommended initial monthly Influence:

```text
Aedile       +1
Praetor      +2
Consul       +4
Proconsul    +4
Augustus     victory
```

Only the current office/status grants its bonus.

Do not stack old offices.

Exact values are configuration.

---

# 191. Aedile

Aedile is bought directly with Influence.

Recommended initial cost:

```text
100 Influence
```

No Senate vote is required.

Buying Aedile does not consume the global Senate schedule.

---

# 192. Praetor

To become Praetor:

```text
current rank = Aedile
```

The player must:

1. win nomination for the next Senate ballot;
2. campaign for one year;
3. win the Senate YES/NO vote.

There may be multiple Praetors.

---

# 193. Consul

There may be at most:

```text
2 active Consuls
```

Eligibility:

```text
Praetor
or
Proconsul
```

A Consul seat must be available when the vote resolves.

A seat can become available because:

- it is already vacant;
- a Consul's 48-month term expires by the vote date;
- a Consul has been removed through a successful political motion.

---

# 194. Consul term and Proconsul

A Consul term lasts:

```text
48 months
```

At expiry:

```text
Consul → Proconsul
```

A Proconsul:

- no longer occupies a Consul seat;
- receives the same passive Influence as Consul;
- may seek Consul again;
- may not seek Augustus.

The four-year term is deliberate because the global Senate vote occurs every two years, normally giving an active Consul two opportunities to obtain an Augustus ballot.

If an Augustus vote for a Consul resolves in the same month their term expires:

```text
resolve Augustus vote first
```

If successful:

```text
Augustus → victory
```

If unsuccessful:

```text
Consul → Proconsul
```

after the vote.

---

# 195. Augustus

Only an active Consul may seek Augustus.

The player must:

1. win nomination for the next global Senate ballot;
2. complete the Campaign Year;
3. obtain at least 51 YES votes.

Result:

```text
Augustus
→ game victory
```

A Proconsul cannot seek Augustus directly.

---

# 196. One global Senate schedule

There is exactly:

```text
one Senate ballot at a time
```

All promotion levels share the same schedule.

There are no separate Praetor, Consul, or Augustus election calendars.

The next ballot can therefore be claimed by an eligible player seeking:

```text
Praetor
Consul
Augustus
```

or by an eligible political motion such as a No Confidence vote.

This makes access to the Senate itself a shared multiplayer resource.

---

# 197. Global 24-month cycle

The Senate uses a repeating 24-month cycle:

```text
Months 1–12:
Nomination Year

Months 13–24:
Campaign Year

End of Month 24:
Senate Vote
```

After the vote:

```text
new 24-month cycle starts
```

The first year acts as the cooldown before the next campaign.

---

# 198. Nomination Year

During the 12-month Nomination Year, eligible players compete for the single next ballot using Influence.

Minimum cumulative Influence to enter:

```text
Praetor      100
Consul       200
Augustus     400
```

Recommended initial values only; keep configurable.

The office sought does not give priority.

Example:

```text
Player A seeks Praetor   420 Influence
Player B seeks Consul    380 Influence
Player C seeks Augustus  400 Influence
```

A currently leads the nomination despite seeking the lower office.

---

# 199. Monthly sealed-bid nomination rounds

Do not use:

```text
first player to click wins
```

and do not use a continuous auction.

Each Nomination Year consists of 12 monthly sealed bidding rounds.

For each month:

1. players privately submit additional Influence;
2. bids are hidden until month resolution;
3. all bids resolve simultaneously;
4. Influence is committed/spent;
5. new cumulative totals become public;
6. next sealed month begins.

This removes last-second and network-latency advantages.

---

# 200. Nomination example

Public cumulative totals:

```text
A 180
B 200
```

Private bids this month:

```text
A +40
B +10
```

At monthly resolution:

```text
A 220
B 210
```

Nobody can react to the hidden bid until the next month.

---

# 201. Final nomination month

Month 12 remains sealed.

Example before final round:

```text
A 280
B 270
C 230
```

Private final bids:

```text
A +20
B +50
C +80
```

Resolve simultaneously:

```text
A 300
B 320
C 310
```

B wins the next ballot.

No player can observe B's final bid and immediately outbid it.

---

# 202. Nomination Influence is spent

All committed nomination Influence is permanently spent.

This applies to winners and losers.

```text
cannot withdraw
cannot reduce
cannot refund losing bids
```

This creates the strategic choice:

```text
spend heavily to control the next ballot
or
save Influence to fight during the Campaign Year
```

---

# 203. Nomination ties

Do not break ties by:

- click order;
- connection order;
- PlayerId;
- host status.

If highest cumulative totals tie after month 12:

```text
run one additional sealed sudden-death month
```

Only the tied leaders may add Influence.

Reveal simultaneously.

Repeat if still tied.

The Campaign Year begins only after a unique winner exists.

---

# 204. Campaign Year

The nomination winner becomes the sole candidate for the next ballot.

Campaign lasts:

```text
12 months
```

During that year:

- candidate campaigns for support;
- every other player may support the candidate;
- every other player may campaign against the candidate;
- players may expose scandals;
- spies may discover new scandals;
- game-state changes alter structural support;
- projected Senate support updates monthly.

No additional nomination bidding occurs during the Campaign Year.

---

# 205. Senate composition

Use 100 Senate votes split into five blocs:

```text
Aristocrats   30
Merchants     20
Provincials   20
Populares     20
Military      10
----------------
Total        100
```

These are recommended initial sizes and should be configurable.

---

# 206. Senate vote format

Because there is only one candidate:

```text
YES — grant requested office
NO  — reject
```

Promotion succeeds at:

```text
YES >= 51
```

Otherwise it fails.

Example:

```text
Should Player B become Praetor?

YES 54
NO  46
```

---

# 207. Senate support model

Each bloc determines its own YES support.

General structure:

```text
bloc_score =
    structural_support
    + candidate_campaign
    + endorsements
    - opposition_campaign
    - scandal_penalties
    + event_modifiers
```

Convert that score into the number of YES votes contributed by the bloc.

The model must be deterministic and explainable in UI.

---

# 208. Aristocrats

Primary drivers:

```text
Noble population
Noble happiness
political rank
prestige / Influence
completed wonders
political city buildings
relevant scandals
campaigning
```

Noble population must use diminishing returns, e.g.:

```text
sqrt(total_nobles)
```

rather than raw linear population.

---

# 209. Merchants

Primary drivers:

```text
Coin income
trade volume
trade reliability
city Markets
economic stability
resource availability
commerce-related scandals
campaigning
```

---

# 210. Provincials

Primary drivers:

```text
vassal stability
vassal relation
provincial free-pop happiness
trade with provinces/vassals
reasonable tribute
treatment of provinces
campaigning
```

High Vassal Control with terrible Relation must not automatically create strong Provincial support.

---

# 211. Populares

Primary drivers:

```text
Citizen happiness
Plebeian happiness
food policy
food security
famine
tax pressure
harsh domestic policies
relevant scandals
campaigning
```

Citizens matter strongly.

Plebeians affect popular legitimacy mainly through happiness/unrest.

Slaves do not vote.

---

# 212. Military

Primary drivers:

```text
army strength
military victories
successful defense
recent defeats
military prestige
campaigning
```

Exact war-derived values depend on the final military implementation.

---

# 213. Avoiding permanent Senate dominance

A larger population must help without making one player unbeatable.

Use:

```text
diminishing returns
```

for Noble and Citizen contribution.

Examples:

```text
sqrt(population)
log(population + 1)
soft caps
```

Recommended overall balance target:

```text
60–70% structural support
30–40% active politics
```

Structural:

- population/happiness;
- economy;
- military;
- provinces;
- rank.

Active politics:

- campaigning;
- opposition;
- endorsements;
- scandals;
- current events.

This ensures other players can materially affect another player's vote.

---

# 214. Candidate campaigning

During Campaign Year the candidate may spend Influence on a selected bloc:

```text
Campaign with Aristocrats
Campaign with Merchants
Campaign with Provincials
Campaign with Populares
Campaign with Military
```

Campaign effects must have diminishing returns within the same campaign.

Do not allow unlimited linear purchase of Senate votes.

---

# 215. Other players supporting the candidate

Any other player may spend Influence to:

```text
Support Candidate
```

targeting a selected bloc.

This increases the candidate's support in that bloc.

This enables multiplayer bargaining:

```text
support my Consul vote now
and I will support your Praetor vote later
```

The game does not need to enforce informal promises between human players.

---

# 216. Other players opposing the candidate

Any non-candidate may spend Influence to:

```text
Campaign Against Candidate
```

targeting a selected bloc.

This lowers the candidate's support in that bloc.

Players who are not eligible for promotion therefore still have a meaningful role in every Senate campaign.

---

# 217. Coin bribery

The candidate may use Coin to bribe Senators.

Bribery:

- improves campaign support;
- creates a real player scandal opportunity;
- may be discovered by enemy spies.

This distinguishes:

```text
Influence = legitimate political campaigning
Coin      = potentially stronger/cheaper but scandalous
```

Exact strength and discoverability are balancing values.

---

# 218. Exposing scandals

A player holding a scandal against the candidate may consume it during the Campaign Year.

Different scandal types affect different blocs.

Examples:

```text
Low Food / Famine
→ strong Populares penalty
→ Provincial penalty

Harsh Slave Labor
→ Populares / Provincial penalty

Political Bribery
→ Aristocrat / Merchant penalty

Espionage Against Player
→ broad Senate penalty

High Vassal Tribute / Abuse
→ Provincial penalty
```

Severity modifies the effect.

Do not convert every scandal into a generic fixed vote loss.

---

# 219. Campaign state is live

Structural support is recalculated during the Campaign Year.

Examples:

```text
Low Food enabled
→ Citizen/Plebeian happiness may fall
→ Populares support falls
→ scandal opportunity appears

Famine fixed
→ support may recover

Military victory
→ Military support rises

Wonder completed
→ Aristocrat prestige may rise
```

Projected vote updates after monthly resolution.

---

# 220. Failed vote

If:

```text
YES <= 50
```

promotion fails.

Then:

```text
new 12-month Nomination Year begins
```

The failed player may compete again if eligible.

There is no immediate repeat vote.

The global Senate cycle itself provides the one-year cooldown.

---

# 221. Consul seat availability

There are exactly two active Consul seats.

A Consul candidacy may enter nomination only when a seat is expected to be available at the vote date.

Valid reasons:

```text
seat already vacant
current Consul term expires by vote
successful removal has created vacancy
```

Never create a third active Consul.

---

# 222. Removing a Consul early

A Consul can be forced to step down through:

```text
Motion of No Confidence
```

Requirements:

```text
target is an active Consul
initiator owns a scandal against target
initiator participates through the global Senate schedule
```

The motion itself must win the nomination auction for the next ballot, because only one Senate vote exists at a time.

During Campaign Year:

```text
YES = remove Consul
NO  = retain Consul
```

All players may campaign for either side.

At:

```text
YES >= 51
```

the target immediately becomes Proconsul and the Consul seat becomes vacant.

If the motion fails:

- target remains Consul;
- scandal is consumed;
- nomination/campaign Influence stays spent.

---

# 223. Proconsul

A player becomes Proconsul after:

```text
normal 48-month Consul term
or
successful No Confidence removal
```

Proconsul:

```text
same passive Influence as Consul
does not occupy Consul seat
can seek Consul again
cannot seek Augustus
```

---

# 224. Augustus competition

Active Consuls compete for access to the same global nomination schedule as everyone else.

Example nomination participants:

```text
A seeks Praetor
B seeks Consul
C seeks Augustus
```

The highest final Influence commitment wins the one next ballot regardless of office sought.

Therefore a player may deliberately outbid an Augustus attempt with a Praetor or Consul request.

This is intentional multiplayer counterplay.

---

# 225. Augustus Campaign

If an active Consul wins the Augustus nomination:

```text
12-month Augustus Campaign begins
```

Other players may:

- support;
- oppose;
- expose scandals;
- continue espionage;
- target blocs;
- negotiate support.

Final vote:

```text
YES >= 51
→ Augustus
→ victory
```

---

# 226. Rome Senate tab UI

During Nomination Year:

```text
NEXT SENATE BALLOT — NOMINATION

Months remaining

Player A
Requested office: Praetor
Committed Influence: 180

Player B
Requested office: Consul
Committed Influence: 220

Player C
Requested office: Augustus
Committed Influence: 400
```

Current cumulative totals are public.

Current-month additional bids are private until resolution.

Local controls show:

```text
eligible requested office
minimum bid
already committed
current public leader
private additional bid
```

---

# 227. Rome Campaign UI

During Campaign Year:

```text
NEXT SENATE VOTE

Candidate: <player>
Office: <Praetor / Consul / Augustus / No Confidence>
Vote in: <months>

Projected:
YES 54
NO  46
```

Bloc breakdown:

```text
Aristocrats    18 / 30 YES
Merchants      12 / 20
Provincials    10 / 20
Populares       9 / 20
Military        5 / 10
```

Each bloc is expandable to show contribution breakdown.

---

# 228. Bloc explanation UI

Example:

```text
POPULARES

Citizen happiness      +6
Plebeian happiness     +4
Normal food policy     +3
Recent famine          -4
Candidate campaign     +2
Enemy campaign         -1
Exposed scandal        -3
```

The player must understand why projected support changed.

---

# 229. Cursus Honorum UI

The rank tab shows:

```text
Aedile
Praetor
Consul
Proconsul
Augustus
```

For local player display:

```text
current rank
passive Influence
next eligible office
minimum nomination bid
Consul term remaining
lock reasons
```

Example:

```text
CONSUL

Locked:
✓ rank requirement met
✗ no Consul seat available at next vote
```

---

# 230. Political ladder summary

```text
No Office
    ↓ pay fixed Influence
AEDILE
    ↓ win global nomination
    ↓ 12-month campaign
    ↓ Senate vote
PRAETOR
    ↓ win global nomination
    ↓ 12-month campaign
    ↓ Senate vote
CONSUL
    ├─ 48-month term
    ├─ may seek Augustus
    │     ↓
    │   win global nomination
    │     ↓
    │   12-month campaign
    │     ↓
    │   Senate vote
    │     ↓
    │   AUGUSTUS → VICTORY
    │
    └─ term expires / removed
          ↓
      PROCONSUL
          ↓ later Consul vote
        CONSUL
```

---

# 231. Senate multiplayer requirement

The candidate must never effectively play a private single-player promotion minigame.

Every other player must be able to influence the result by:

```text
supporting candidate
opposing candidate
targeting blocs
exposing scandals
discovering scandals
bidding for the nomination instead
making political agreements
```

A strong empire should have structural advantages, but political opponents must be able to delay, block, or support advancement through deliberate action.

---

# 232. Senate invariants

```text
one global Senate ballot at a time

Nomination Year = 12 months
Campaign Year = 12 months

all nomination bids are sealed within each month
all monthly bids resolve simultaneously

public information:
    resolved cumulative Influence commitments

private information:
    current month's additional bids until resolution

all committed Influence is spent
losers receive no refund

no first-click advantage
no last-second reaction advantage
no connection-order tiebreak
no host advantage

ties use sealed sudden-death rounds

Aedile requires no Senate ballot

Praetor requires Aedile + successful vote

Consul requires Praetor/Proconsul + successful vote
maximum 2 active Consuls
Consul term = 48 months

Proconsul has same Influence income as Consul
Proconsul cannot seek Augustus

Augustus requires active Consul + successful Senate vote
Augustus triggers victory
```

---

# 233. Senate configuration

Centralize:

```text
Aedile Influence cost

minimum Praetor nomination bid
minimum Consul nomination bid
minimum Augustus nomination bid

rank Influence/month

Nomination Year length
Campaign Year length
Consul term length

Senate bloc sizes
bloc structural weights

campaign Influence costs/effects
campaign diminishing-return curve

endorsement effects
opposition effects

bribery cost/effect
bribery scandal discoverability

scandal-to-bloc effect map
scandal severity multipliers

YES threshold

No Confidence requirements/costs
```

Do not hardcode balance values in UI logic.

---

# 234. Senate explicit non-goals

Do not implement unless separately requested:

```text
separate election calendars per rank
multiple simultaneous Senate ballots
first-click nomination
continuous real-time auction
individual Senator simulation
permanent ownership of Senators
raw population = raw Senate votes
automatic promotion from stats alone
automatic Augustus victory
Proconsul seeking Augustus
more than two active Consuls
```

---

# 235. Senate chamber visualization

The Rome Senate UI must visually represent all `100` Senators as individual circles arranged in a layered semicircle resembling a senate chamber.

Requirements:

```text
100 circles exactly
multiple curved / semicircular rows
centered around the speaker / floor area
stable seat positions
```

Each Senator circle represents one Senate vote.

The layout is presentation-only. Senators are not simulated as persistent individual political characters.

---

# 236. Senator vote colors

During a Campaign Year, each Senator circle is colored according to the currently projected vote.

For a normal promotion ballot:

```text
Candidate YES      candidate/player color
NO                 opposition color
Undecided          gray
```

For a No Confidence ballot:

```text
Remove Consul      motion/support color
Keep Consul        target/incumbent color
Undecided          gray
```

The exact palette must remain readable and must match player colors where appropriate.

A legend must always identify the meaning of each color.

---

# 237. Determinate and undecided Senators

Not all projected Senate votes need to be committed before the final vote.

Each bloc calculation produces:

```text
committed YES Senators
committed NO Senators
undecided Senators
```

Therefore the Rome UI may show, for example:

```text
YES        44
NO         38
Undecided  18
```

as:

```text
44 candidate-colored circles
38 opposition-colored circles
18 gray circles
```

The player must always be able to see exactly how many votes are already committed and how many remain uncertain.

---

# 238. Undecided vote resolution

At the actual Senate vote, every Senator still marked `Undecided` votes randomly.

The probability is not necessarily 50/50.

Each undecided Senator uses the final underlying support probability for its Senate bloc.

Example:

```text
Populares final candidate support probability = 0.65
```

Every undecided Populares Senator independently resolves:

```text
65% chance YES
35% chance NO
```

This creates a controlled random component while making the amount of uncertainty visible beforehand.

Do not randomize already committed Senators.

---

# 239. Committing Senator votes

Campaign support should progressively convert uncertain Senators into committed votes.

A bloc's final state can be modeled as:

```text
support_probability
certainty
```

where:

```text
certainty determines how many bloc Senators are committed
support_probability determines which side those committed Senators support
and the random tendency of remaining undecided Senators
```

The precise mathematical function belongs in centralized Senate balancing configuration.

Important behavior:

```text
strong structural support + strong campaigning
→ many committed YES votes

strong opposition
→ many committed NO votes

close / weakly contested bloc
→ more gray undecided Senators
```

---

# 240. Vote animation

When the vote resolves:

1. keep committed circles unchanged;
2. resolve undecided Senators one at a time or in a short animated sequence;
3. gray circles change to final YES/NO colors;
4. update vote totals as circles resolve;
5. display final result after all 100 have resolved.

This should make the visible uncertainty meaningful and make major votes dramatic without hiding the underlying probabilities.

The animation must not affect simulation outcome; all random results may be generated first, then visually revealed.

For deterministic multiplayer synchronization, the authoritative game state/server/host must generate the undecided-vote results once and distribute the result to all clients.

Do not allow clients to independently roll Senate results.

---

# 241. Senate projection display

During Campaign Year show both:

```text
Projected committed votes
Undecided votes
```

Example:

```text
PRAETOR VOTE

YES        46
NO         39
Undecided  15

Estimated YES probability among undecided: 58%
```

The UI may additionally show an estimated expected final total, but it must not present that estimate as guaranteed.

Example:

```text
Expected final YES: ~55
```

The 15 undecided circles remain visibly gray.

---

# 242. Senate bloc visualization

The 100-circle semicircle should preserve bloc identity where possible.

Recommended layout:

```text
Aristocrats   30 seats
Merchants     20 seats
Provincials   20 seats
Populares     20 seats
Military      10 seats
```

Possible implementation:

- seat contiguous sections of the semicircle by bloc;
- show subtle separators or labels;
- hovering/tapping a Senator reveals its bloc and current state.

The primary visual color remains the vote state, not the bloc.

---

# 243. Senate random-vote invariant

The Senate system must enforce:

```text
committed Senators never reroll at vote time

only gray / Undecided Senators are random

random probability comes from final bloc support

number of undecided Senators is visible before the vote

all clients see the same authoritative result

total final votes always equals exactly 100
```

This supersedes any earlier implication that the Senate vote is fully deterministic.

The strategic game remains mostly predictable, but unresolved support produces explicitly visible risk.

---

# 244. Military system scope

This section defines:

- recruitment;
- unit types;
- population sources for recruits;
- recruitment costs and time;
- province military state;
- movement;
- coexistence;
- NPC starting forces;
- upkeep;
- casualties;
- disbanding;
- training;
- morale;
- special-unit availability;
- military occupation and political Control;
- vassal garrisons;
- Military Rank;
- Military Renown;
- Military Senate-bloc effects.

This section intentionally does **not** define battle resolution itself.

Do not implement from this section:

```text
combat phases
damage formulas
unit matchup tables
flanking
terrain combat modifiers
battle casualty formulas
siege resolution
```

Those will be designed separately.

---

# 245. No Army entity

There is no separate persistent `Army` object.

Military units belong directly to a province and an owner.

Conceptually:

```text
Province
├─ Local / NPC force
├─ Player A units
├─ Player B units
├─ Player C units
└─ ...
```

Recommended state:

```rust
struct ProvinceMilitaryState {
    forces: HashMap<ForceOwnerId, Vec<Unit>>,
}
```

All units owned by one player in one province together form that player's effective force in the province.

Do not implement:

```text
Create Army
Merge Army
Split Army
Army stack objects
```

Moving units automatically changes which province force they belong to.

---

# 246. Unit state

Recommended model:

```rust
struct Unit {
    id: UnitId,
    owner: ForceOwnerId,
    unit_type: UnitType,

    current_manpower: f32,
    max_manpower: f32,

    training: f32, // 0..100
    morale: f32,   // 0..100
}
```

Units remain individually identifiable because different units may have different:

- manpower;
- training;
- morale;
- history.

---

# 247. Initial unit roster

Use:

```text
Light Infantry
Heavy Infantry
Archers

Light Cavalry
Heavy Cavalry
Horse Archers

War Chariots
War Camels
War Elephants

Ballista
Catapult
```

Do not include Trebuchet.

---

# 248. Recruitment is province-based

Every directly owned province may recruit units.

Recruitment is **not limited to city provinces**.

Reason:

- recruits come from province population;
- war should directly reduce civilian population;
- recruitment should affect province production, tax income, and happiness;
- populous provinces should be militarily valuable.

Cities may later provide recruitment bonuses, but are not required for ordinary recruitment.

---

# 249. Recruitment slot

Each province has:

```text
1 active military recruitment project
```

at a time.

This recruitment slot is separate from the construction slot.

Therefore a province may simultaneously have:

```text
1 building/wonder construction
1 military recruitment
```

but cannot recruit two units in parallel unless changed later.

Recommended:

```rust
struct RecruitmentProject {
    unit_type: UnitType,
    progress: f32,
    required_progress: f32,
}
```

---

# 250. Recruitment flow

When recruitment starts:

```text
1. Province must be directly owned.
2. Unit must be recruitable in that province.
3. Required local pop class must contain enough people.
4. Player must have enough global Metal.
5. Remove required pops immediately.
6. Deduct Metal immediately.
7. Create recruitment project.
8. Recruits no longer produce/tax as civilians.
9. Advance recruitment monthly.
10. On completion, create unit in that province.
```

Recommended cancellation rule:

```text
cancelled recruitment gives no pop refund
cancelled recruitment gives no Metal refund
```

This is intentionally strict and simple.

---

# 251. Population classes used for recruitment

Recommended initial mapping:

```text
Light Infantry      ← Plebeians
Heavy Infantry      ← Plebeians
Archers             ← Plebeians

Light Cavalry       ← Citizens
Heavy Cavalry       ← Citizens
Horse Archers       ← Citizens

War Chariots        ← Citizens
War Camels          ← Citizens
War Elephants       ← Citizens for handlers/crew

Ballista            ← Plebeians
Catapult            ← Plebeians
```

Do not use Slaves for ordinary unit recruitment.

Do not consume Nobles for ordinary units.

This creates economic tradeoffs:

```text
recruit Plebeians
→ lower productive labor

recruit Citizens
→ lower tax base
→ possible political/Senate consequences
```

---

# 252. Example initial manpower requirements

Recommended balancing defaults:

```text
Light Infantry       100 Plebeians
Heavy Infantry       100 Plebeians
Archers               80 Plebeians

Light Cavalry         50 Citizens
Heavy Cavalry         40 Citizens
Horse Archers         50 Citizens

War Chariots          30 Citizens
War Camels            40 Citizens
War Elephants         20 Citizens

Ballista              30 Plebeians
Catapult              30 Plebeians                30 Plebeians
```

These are configuration values, not architectural constants.

There is no separate abstract manpower resource.

---

# 253. Recruitment happiness effect

Drafting a significant share of a class should temporarily lower that class's happiness in the recruiting province.

Recommended structure:

```text
draft_fraction =
    recruited_manpower / class_population_before_draft

draft_penalty =
    min(MAX_DRAFT_PENALTY,
        draft_fraction × DRAFT_HAPPINESS_SCALE)
```

Apply to the recruited class.

The penalty should decay over time.

This keeps mass mobilization politically costly.

---

# 254. Recruitment Metal cost

Every unit has an initial global Metal cost.

Metal represents:

- weapons;
- armor;
- fittings;
- specialized military equipment.

Recommended definition:

```rust
struct UnitDefinition {
    manpower_class: PopClass,
    manpower: f32,

    metal_cost: f32,
    recruitment_months: f32,
    food_per_month: f32,

    base_strength: f32,
    special_recruitment_tag: Option<RecruitmentTag>,
}
```

Metal is deducted in full when recruitment starts.

---

# 255. No recurring Metal upkeep

Units do not consume Metal every month.

Metal cost occurs at recruitment.

There is no reinforcement mechanic, so damaged units do not later consume Metal to restore manpower.

Do not implement:

```text
monthly weapon decay
monthly Metal upkeep
automatic equipment replacement
```

unless designed separately later.

---

# 256. Food upkeep

Every unit consumes global Food every month.

Each unit type has:

```text
food_per_month
```

Recommended relative scale:

```text
Light Infantry      low
Heavy Infantry      normal
Archers             low-normal

Light Cavalry       high
Heavy Cavalry       high
Horse Archers       high

War Chariots        high
War Camels          high
War Elephants       very high

Ballista/Catapult/
```

Animal-heavy units therefore impose meaningful logistical costs.

---

# 257. Military Food shortage

Military Food demand joins the player's normal global Food requirement.

If insufficient Food exists:

```text
military_supply_ratio < 1
```

then units suffer Morale penalties.

Initial implementation:

```text
Food shortage
→ lower Morale
```

Do not instantly remove manpower because of one low-supply month.

Long-term attrition may be designed later.

---

# 258. No reinforcement or manpower recovery

Casualties are permanent.

Example:

```text
Heavy Infantry:
100 → battle → 63
```

The unit remains:

```text
63 / 100
```

There is no action to add replacement Plebeians.

There is no passive recovery.

The player may:

- keep using the depleted unit;
- move it;
- disband survivors;
- eventually lose it entirely.

---

# 259. Unit destruction

When:

```text
current_manpower <= 0
```

remove the unit.

There is no zero-manpower unit.

---

# 260. Disbanding

A player may disband a unit only in a directly owned province.

On disband:

```text
remaining manpower
→ returned to original recruitment class
in current province
```

Examples:

```text
63 Heavy Infantry
→ +63 Plebeians

28 Heavy Cavalry
→ +28 Citizens
```

Metal is never refunded.

Training and Morale disappear with the unit.

Do not allow normal disbanding in foreign, hostile, or vassal territory unless explicitly added later.

---

# 261. Movement

Players select one or more of their units in Province A and order them into an adjacent Province B.

On movement resolution:

```text
remove selected units from A
add selected units to same owner's force in B
```

There is no merge/split logic.

---

# 262. Movement permissions

Destination behavior:

```text
Directly owned province
→ allowed

Own vassal
→ allowed

Friendly NPC province
→ allowed only with military access / sufficient Relation

Other player's province
→ allowed only when invited / granted military access

Hostile NPC province
→ invasion

Enemy player province
→ invasion / war movement
```

Exact travel-time mechanics can be designed separately.

---

# 263. Multiple forces can coexist

Two or more military owners may have units in the same province.

Examples:

```text
NPC force + friendly Player A force

Player A force + invited Player B force
```

Peaceful coexistence requires diplomatic permission.

For an NPC, the first implementation may automatically grant military access above a Relation threshold.

Recommended:

```text
Relation >= 70
```

Keep configurable.

For a player-owned province:

```text
province owner explicitly grants/invites military access
```

---

# 264. Peaceful presence does not create Control

Foreign units peacefully stationed in a province do not create military Control.

This includes:

```text
friendly NPC access
allied/invited player access
units passing through
```

Military Control requires hostile occupation.

Mandatory distinction:

```text
military access ≠ occupation
```

---

# 265. Hostility while coexisting

If two owners with units in the same province become hostile:

```text
their forces become opposing forces
```

and must be handled by the future battle system.

Do not automatically eject, destroy, or teleport either force.

---

# 266. NPC starting forces

Every independent NPC province starts with its own configured military units.

Recommended map/province data:

```rust
struct ProvinceMilitarySetup {
    starting_units: Vec<UnitTemplate>,
}
```

Example:

```text
Achaia:
2 Light Infantry
1 Heavy Infantry
1 Archer
```

Important provinces may start stronger.

Do not generate initial defense solely from current population.

---

# 267. NPC rebuilding after defeat

This specification does not define NPC recruitment/replacement after the starting force is destroyed.

Minimum required behavior:

```text
independent NPC provinces start with configured forces
```

Do not automatically regenerate a destroyed army to full strength.

Future NPC military recruitment can be designed separately.

---

# 268. Special-unit recruitment tags

Basic units may be generally available:

```text
Light Infantry
Heavy Infantry
Archers
Light Cavalry
Heavy Cavalry
```

Special units require province tags:

```text
HORSE_ARCHERS
CHARIOTS
CAMELS
ELEPHANTS
```

Mapping:

```text
Horse Archers  → HORSE_ARCHERS
War Chariots   → CHARIOTS
War Camels     → CAMELS
War Elephants  → ELEPHANTS
```

Tags are explicit province data.

Do not derive availability solely from terrain.

This makes conquest of particular provinces strategically valuable.

---

# 269. Siege/artillery units

Initial siege/artillery roster:

```text
Ballista
Catapult
```

They:

- consume Plebeian crew;
- cost significant Metal;
- generally take longer to recruit;
- consume Food;
- will have special battle/siege behavior later.

Battle behavior is not defined here.

---

# 270. Training

Training is unit-specific:

```text
0..100
```

Recommended starting Training:

```text
10
```

Passive Training:

```text
+1/month
```

while the unit exists and is adequately supplied.

Clamp at `100`.

Training changes slowly and represents long-term discipline and experience.

---

# 271. Training from battle

Surviving units gain additional Training after battle.

The future battle result must provide at least:

```text
unit participated
friendly casualty ratio
battle result
```

Recommended concept:

```text
base battle participation       +2

friendly losses:
0–10%                            +0
10–25%                           +1
25–50%                           +2
50%+                             +3
```

Thus harder battles can create more experienced survivors.

Cap gains strongly enough that deliberately suffering losses is not optimal.

---

# 272. Veteran depleted units

Because units cannot reinforce, experienced depleted formations are expected.

Example:

```text
Heavy Infantry
Manpower: 34 / 100
Training: 82
```

This is an intentional strategic state.

The player decides whether to preserve, use, move, or disband it.

---

# 273. Morale

Morale is unit-specific:

```text
0..100
```

Recommended normal baseline:

```text
50
```

Morale factors include:

```text
Military Rank
Training
Food supply
recent victory
recent defeat
battle casualties
events
```

Morale changes faster than Training.

Morale may recover over time toward its normal baseline when supplied.

Morale recovery is **not** manpower recovery.

---

# 274. Military occupation of an independent NPC province

Simply entering a hostile independent province is insufficient to gain military Control.

Required sequence:

```text
enter/invade
↓
local NPC force exists
↓
battle
↓
local defending force defeated
↓
occupying force remains
↓
military occupation established
↓
stationed strength generates Independent Control over time
```

No occupation Control exists while the local defender remains undefeated.

---

# 275. Occupation state

Recommended state:

```rust
occupation: Option<ForceOwnerId>
```

Presence does not imply occupation.

Set occupation only after the battle/war system establishes that owner as the occupying victor.

Peaceful access never sets occupation.

---

# 276. Effective stationed strength

Occupation and vassal garrison effects use total **effective strength**, not number of units or armies.

Recommended political-strength estimate:

```text
unit_effective_strength =
    current_manpower
    × unit_type_base_strength
    × training_modifier
    × morale_modifier
```

Then:

```text
total_stationed_strength =
    Σ unit_effective_strength
```

This is a political/garrison-strength calculation only.

It is not the final combat formula.

---

# 277. Military Rank modifies stationed strength

For occupation/control purposes:

```text
effective_garrison_power =
    total_stationed_strength
    × military_rank_control_modifier
```

Recommended initial multipliers:

```text
Centurion           ×1.00
Military Tribune    ×1.10
Legate              ×1.20
Imperator           ×1.30
```

Exact values are configurable.

---

# 278. Military occupation Control gain

Convert effective garrison power into monthly Independent Control gain using diminishing returns.

Recommended behavior:

```text
weak garrison          about +1 Control/month
medium                 about +2
strong                 about +3
overwhelming           about +4 maximum
```

Use a configurable curve rather than hard thresholds if convenient.

Occupation Control:

1. consumes Local Control first;
2. then competes with rivals through the existing simultaneous Control-resolution rules.

---

# 279. Occupation damages Relation

Hostile military occupation lowers the province's Relation toward the occupier.

Recommended initial effect:

```text
-2 Relation/month
```

while occupation continues.

This supports intended states such as:

```text
Control 80
Relation 12
```

Military conquest therefore produces political domination without friendship.

---

# 280. Vassal military garrison

Overlord units stationed in a vassal provide passive Vassal Control.

The bonus depends on:

```text
effective stationed unit strength
× Military Rank modifier
```

not:

```text
number of armies
number of units
```

Recommended:

```text
vassal_garrison_control_bonus =
    diminishing_function(effective_garrison_power)
```

The exact curve is configurable.

This supersedes the earlier temporary `+1 per army` concept.

---

# 281. Vassal military example

Monthly:

```text
Relation decay             -3.0
Stationed military power   +2.4
Political support          +1.0
Enemy interference         -0.5
--------------------------------
Net Vassal Control         -0.1
```

If stationed units suffer battle losses, their contribution automatically falls next month.

Military Rank also changes the contribution because it multiplies effective garrison power.

---

# 282. Military Rank

Military Rank is separate from political rank.

Initial ladder:

```text
Centurion
    ↓
Military Tribune
    ↓
Legate
    ↓
Imperator
```

Do not include Primus Pilus.

Military Rank is global per player.

---

# 283. Military Renown

Military Rank is earned through:

```text
Military Renown
```

Main source:

```text
winning battles
```

Battle Renown should eventually consider:

```text
enemy strength
relative strength
battle importance
casualties inflicted
friendly casualty intensity
victory/defeat
```

Recommended target scale:

```text
minor easy victory             +10
normal victory                 +25
major victory                  +50
defeat much stronger force     +75
```

Losses may grant small Renown for major battles, but victory should dominate progression.

Exact Renown calculation waits for the battle system.

---

# 284. Military Rank thresholds

Recommended initial thresholds:

```text
Centurion             0
Military Tribune    150
Legate               400
Imperator            900
```

Promotion occurs automatically when threshold is reached.

No Senate vote is needed.

---

# 285. Military Rank effects

Military Rank grants exactly these broad categories of bonuses initially:

```text
1. Morale bonus to player's units.
2. Higher occupation/Vassal-garrison Control effectiveness.
3. Higher structural support from the Military Senate bloc.
```

Do not grant direct political rank.

Do not add free units.

---

# 286. Military Rank and Senate

Military Rank contributes to the existing Military Senate bloc.

It modifies:

```text
Military-bloc structural score
```

rather than adding fixed Senate votes.

Thus:

```text
Imperator + weak politics
```

does not automatically win Senate elections, but military reputation helps.

---

# 287. Separate political and military careers

Valid combinations include:

```text
Imperator + Aedile
Consul + Centurion
Legate + Praetor
```

Political ladder:

```text
Aedile → Praetor → Consul → Augustus
```

Military ladder:

```text
Centurion → Military Tribune → Legate → Imperator
```

They interact through Morale, Control effectiveness, and the Military Senate bloc, but remain separate systems.

---

# 288. Province military UI

Every province's military panel groups units by owner.

Example:

```text
ACHAIA

Local Forces
2 Light Infantry
1 Archer

Player A
1 Heavy Infantry
1 Light Cavalry

Player B
1 Archer
```

For each unit display:

```text
unit type
current / maximum manpower
Training
Morale
Food/month
```

Also display:

```text
peaceful access / hostile / occupation status
```

---

# 289. Recruitment UI

For directly owned provinces show:

```text
available local Plebeians
available local Citizens
current recruitment
recruitable unit list
```

For each unit:

```text
pop class
manpower required
Metal cost
Food/month
recruitment time
special province tag if required
```

Example:

```text
Heavy Infantry

Requires:
100 Plebeians
40 Metal
3 months

Upkeep:
8 Food/month
```

Values are illustrative/configurable.

---

# 290. Movement UI

Player selects one or more local units and an adjacent destination.

Destination state should read clearly:

```text
Owned          → Move
Own Vassal     → Move
Friendly NPC   → Military Access
Other Player   → Invited / Access
Hostile NPC    → Invade
Enemy Player   → Invade
Blocked        → Cannot Enter
```

No Army creation or reorganization screen is needed.

---

# 291. Military monthly order

Recommended military-related monthly sequence:

```text
1. Advance/resolve recruitment.
2. Resolve scheduled unit movement.
3. Determine military Food demand.
4. Apply global Food supply ratio.
5. Apply Food-related Morale effects.
6. Apply normal Morale recovery/decay.
7. Apply passive Training gain.
8. Resolve battles when battle system exists.
9. Update occupation state from battle outcomes.
10. Calculate effective stationed strength.
11. Calculate military Independent-Control contribution.
12. Calculate military Vassal-Control contribution.
13. Apply Military Rank modifiers.
14. Feed resulting values into political Control resolution.
```

The eventual battle phase must occur before a newly invading force gains occupation Control.

---

# 292. Military notifications recommended

Recommended future events/toasts:

```text
Recruitment completed
Unit destroyed
Foreign units enter province
Enemy invasion begins
NPC defender defeated
Occupation established
Military access granted/revoked
Vassal military support drops significantly
Military Rank increased
```

Exact toast behavior can be specified separately.

---

# 293. Military configuration

Centralized configuration must include:

```text
per unit:
    source pop class
    manpower requirement
    maximum manpower
    Metal cost
    recruitment months
    Food/month
    base political strength
    special recruitment tag

draft happiness penalty
draft penalty decay

new-unit Training
passive Training gain
battle Training gain rules

base Morale
Food shortage Morale penalties
Morale recovery

NPC starting units

military access Relation threshold

occupation-strength scaling
occupation Control curve
occupation Relation penalty

Vassal garrison Control curve

Military Renown thresholds
Military Rank Morale bonus
Military Rank Control multiplier
Military Rank Military-bloc modifier
```

---

# 294. Military invariants

The implementation must enforce:

```text
no Army entity

units belong directly to province + owner

recruitment consumes real province pops

recruitment consumes global Metal once

units consume global Food monthly

no monthly Metal upkeep

no reinforcement
no passive manpower recovery

casualties are permanent

unit at 0 manpower is removed

disband only in directly owned province

disband returns surviving soldiers to original pop class

disband never refunds Metal

moving units changes province force membership automatically

multiple owners may coexist when access permits

peaceful foreign presence never grants Control

occupation Control requires occupation state after defeating defender

independent NPC provinces start with configured forces

Vassal military Control support uses total effective stationed strength

Military Rank modifies Vassal/occupation Control effectiveness

Training is per unit

Morale is per unit

Military Rank is:
Centurion → Military Tribune → Legate → Imperator

political and military careers remain separate
```

---

# 295. Military explicit non-goals

Do not implement from this section:

```text
battle resolution formulas
combat phases
matchup tables
flanking
terrain combat calculations
casualty calculation
siege battle calculation
Army entities
Army merging/splitting
reinforcement
automatic manpower recovery
monthly Metal maintenance
Trebuchet
ordinary Slave recruitment
```

---

# 296. Combat design basis

The land-combat model is intentionally inspired by *Imperator: Rome* rather than copied exactly.

Keep these principles:

```text
unit types counter specific other unit types
unit composition matters
chosen tactic matters
tactics counter other tactics
tactic effectiveness depends on the units actually present
Training reduces losses and improves performance
Morale determines staying power
terrain affects combat
mobile units can flank
force movement speed is limited by the slowest selected unit
roads improve movement
```

All numerical values below are Augustus balancing defaults and belong in configuration.

---

# 297. Province force and tactic

There is still no persistent `Army` entity.

For combat purposes:

```text
all units belonging to one owner in one province
= that owner's Province Force
```

Each Province Force stores one selected combat tactic:

```rust
struct ProvinceForceSettings {
    tactic: CombatTactic,
}
```

When a subset of units moves together, the movement order also stores the tactic they will use if they enter combat before joining another force.

Once a battle begins:

```text
tactic is locked for that battle
```

Do not allow a player to see the opponent tactic and immediately switch tactics after engagement.

After battle ends, the player may change the province force tactic again.

---

# 298. Combat tactics

Initial universally available tactics:

```text
Balanced
Shock Action
Bottleneck
Envelopment
Skirmishing
Deception
```

`Balanced` is neutral:

```text
no counter bonus
no counter weakness
```

The other five form a counter cycle:

```text
Bottleneck   counters Shock Action
Shock Action counters Deception
Deception    counters Skirmishing
Skirmishing  counters Envelopment
Envelopment  counters Bottleneck
```

Therefore:

```text
Shock Action
    good vs Deception
    bad vs Bottleneck
```

and so on.

This creates predictable counterplay without requiring dozens of tactics.

---

# 299. Tactic composition effectiveness

A tactic only gets its full benefit when the units in the Province Force are suitable for it.

Each unit type has a `0..1` fit score for every tactic.

Recommended initial table:

```text
                     Shock  Bottle  Envelop  Skirmish  Deception
Light Infantry       0.50    0.80     0.30      0.80      0.60
Heavy Infantry       1.00    1.00     0.20      0.20      0.60
Archers              0.20    0.80     0.30      1.00      0.70
Light Cavalry        0.50    0.20     1.00      0.70      0.90
Heavy Cavalry        1.00    0.30     0.90      0.30      0.70
Horse Archers        0.25    0.20     1.00      1.00      1.00
War Chariots         0.75    0.30     0.80      0.50      1.00
War Camels           0.50    0.20     1.00      0.60      0.90
War Elephants        1.00    0.75     0.10      0.10      0.30
Ballista             0.00    0.50     0.00      0.50      0.20
Catapult             0.00    0.50     0.00      0.40      0.20               0.00    0.50     0.00      0.30      0.20
```

`Balanced` ignores tactic fit.

---

# 300. Calculating tactic effectiveness

Use surviving unit strength, not nominal unit count.

For each unit:

```text
unit_weight =
    current_manpower / max_manpower
```

Then:

```text
tactic_effectiveness =
    Σ(unit_weight × tactic_fit[unit_type])
    / Σ(unit_weight)
```

Clamp:

```text
0..1
```

A nearly destroyed unit therefore contributes little to tactic effectiveness.

Siege units normally contribute very little.

---

# 301. Tactic combat modifier

If the selected tactic counters the enemy tactic:

```text
damage multiplier =
    1 + 0.20 × tactic_effectiveness
```

Maximum:

```text
+20%
```

If the enemy tactic counters yours:

```text
damage multiplier ×0.90
```

Therefore a perfectly fitted counter can produce approximately:

```text
1.20 vs 0.90
```

before other combat factors.

Neutral matchup:

```text
×1.00
```

`Balanced` never counters and is never countered.

---

# 302. Tactic casualty character

Tactics may also modify overall casualty intensity.

Recommended initial values:

```text
Balanced:
    no modifier

Shock Action:
    both sides' manpower losses ×1.10

Bottleneck:
    no casualty modifier

Envelopment:
    no casualty modifier

Skirmishing:
    both sides' manpower losses ×0.90

Deception:
    no casualty modifier
```

These apply only after ordinary damage calculation.

Keep configurable.

---

# 303. Core unit combat stats

Each unit type has:

```rust
struct UnitCombatStats {
    offense: f32,
    defense: f32,
    movement_speed: f32,
    maneuver: u8,
    morale_damage_taken: f32,
    manpower_damage_taken: f32,
    role: UnitRole,
}
```

Recommended initial values:

```text
Unit              Off   Def   Speed  Maneuver  MoraleTaken  ManpowerTaken

Light Infantry    0.80  0.85   2.5      1        0.80          1.10
Heavy Infantry    1.15  1.25   2.5      1        0.90          0.90
Archers           1.00  0.75   2.5      2        1.25          1.00

Light Cavalry     1.00  0.90   4.0      3        1.00          1.00
Heavy Cavalry     1.30  1.20   3.5      2        1.00          0.90
Horse Archers     1.15  0.90   4.0      5        1.15          1.00

War Chariots      1.00  0.90   2.5      1        1.00          1.00
War Camels        1.05  0.95   3.5      4        1.00          1.00
War Elephants     1.60  1.50   2.5      0        1.15          0.60

Ballista          0.70  0.30   2.0      0        1.30          1.50
Catapult          0.80  0.25   1.5      0        1.30          1.50            0.90  0.20   1.5      0        1.30          1.50
```

These are balancing defaults.

---

# 304. Unit matchup matrix

Combat effectiveness is asymmetric.

Values below multiply the attacker's combat power against the defender.

Abbreviations:

```text
LI   Light Infantry
HI   Heavy Infantry
AR   Archers
LC   Light Cavalry
HC   Heavy Cavalry
HA   Horse Archers
CH   War Chariots
CA   War Camels
EL   War Elephants
```

Initial matchup matrix:

```text
ATTACKER ↓ / DEFENDER →   LI    HI    AR    LC    HC    HA    CH    CA    EL

LI                         1.00  0.70  0.95  0.80  0.65  0.75  0.75  0.85  0.55
HI                         1.35  1.00  1.05  1.25  1.20  0.85  1.40  1.10  0.80
AR                         1.25  1.10  1.00  0.65  0.55  0.70  1.00  0.70  0.80

LC                         1.20  0.60  1.50  1.00  0.75  1.35  1.15  0.85  0.50
HC                         1.35  0.75  1.50  1.30  1.00  1.20  1.35  1.10  0.65
HA                         1.45  1.25  1.30  0.75  0.70  1.00  1.30  0.90  1.10

CH                         1.50  0.60  1.20  0.75  0.60  0.65  1.00  0.85  0.50
CA                         1.30  0.80  1.20  1.20  0.90  1.15  1.25  1.00  0.60
EL                         1.60  1.40  1.25  1.60  1.50  0.90  1.60  1.40  1.00
```

Design intentions:

```text
Heavy Infantry:
    strongly counters Light Infantry, cavalry, and Chariots
    weaker against Horse Archers and Elephants

Archers:
    strong against infantry
    vulnerable to cavalry

Light Cavalry:
    strong against Archers, Light Infantry, Horse Archers
    weak against Heavy Infantry and Elephants

Heavy Cavalry:
    strong against light troops and other cavalry
    checked by Heavy Infantry and Elephants

Horse Archers:
    strong against infantry and Archers
    relatively effective against Elephants
    countered by Light/Heavy Cavalry

Chariots:
    punish Light Infantry
    perform poorly into Heavy Infantry, cavalry, Elephants

Camels:
    strong against lighter troops and mobile forces
    weaker into Heavy Infantry/Elephants

Elephants:
    extremely strong front-line unit
    expensive, slow, food-intensive
    Horse Archers are their best ordinary counter
```

All values must live in configuration/data rather than combat code.

---

# 305. Siege-unit field matchups

Siege units are support weapons, not normal front-line troops.

Recommended field targeting bonuses:

```text
Ballista:
    ×1.35 against Heavy Infantry
    ×1.50 against War Elephants
    ×1.10 against Heavy Cavalry

Catapult:
    ×1.20 against Heavy Infantry
    ×1.30 against War Elephants
:
    ×1.10 against dense infantry
    primarily valuable against fortifications
```

Against fast cavalry:

```text
field effectiveness ×0.60
```

These modifiers apply on top of their relatively low base field offense.

---

# 306. Combat width

Each battle has a front-line width determined mainly by terrain.

Recommended initial widths:

```text
Farmland       8
Plains         8
Desert         8
Forest         6
Hills          6
Mountains      4
```

Each side can deploy up to:

```text
combat_width
```

front-line units at once.

A second support row also has:

```text
combat_width
```

slots.

Remaining units are reserves.

---

# 307. Automatic deployment

The player does **not** manually place individual units in combat.

Deployment is automatic from unit role and priorities.

Recommended roles:

```text
CENTER:
    War Elephants
    Heavy Infantry
    Light Infantry
    War Chariots

FLANK:
    Horse Archers
    War Camels
    Light Cavalry
    Heavy Cavalry
    War Chariots

SUPPORT:
    Archers
    Ballista
    Catapult
    Onager
```

Units may fill other positions if their preferred position cannot be filled.

---

# 308. Front-line deployment priority

Recommended center priority:

```text
War Elephants
Heavy Infantry
Light Infantry
War Chariots
Heavy Cavalry
War Camels
Light Cavalry
Horse Archers
Archers
Siege units
```

Recommended flank priority:

```text
Horse Archers
War Camels
Light Cavalry
Heavy Cavalry
War Chariots
Light Infantry
Heavy Infantry
```

Recommended support priority:

```text
Archers
Ballista
Catapult
```

---

# 309. Support row

Support units attack from behind the front line.

While protected:

```text
support attack effectiveness ×0.50
```

This intentionally mirrors the general idea of reduced back-line effectiveness.

Support units do not receive ordinary melee attacks while a friendly front-line unit protects their aligned area.

If the front line collapses:

```text
support units become exposed
```

Exposed support units:

```text
fight at normal offensive effectiveness
take manpower damage ×1.50
```

This makes ranged/siege-heavy forces dangerous but fragile.

---

# 310. Maneuver and flanking

`maneuver` controls how far a unit can reach sideways when its directly opposed enemy slot is empty.

Examples:

```text
Heavy Infantry      maneuver 1
Light Cavalry       maneuver 3
War Camels          maneuver 4
Horse Archers       maneuver 5
War Elephants       maneuver 0
```

If a flank is defeated, high-maneuver units can attack progressively farther into the enemy formation.

This makes cavalry/horse archers valuable in battles where one side has a numerical or mobility advantage.

---

# 311. Reserves

Units that do not fit into active rows remain in reserve.

Reserve units:

```text
do not deal damage
do not take normal direct damage
```

When an active unit:

- routs;
- is destroyed;

the next suitable reserve unit fills that slot at the start of the next combat round.

A reserve entering battle retains its current Training and Morale.

---

# 312. Terrain combat modifiers

Recommended initial terrain effects:

## Farmland / Plains

```text
Light Cavalry      offense +10%
Heavy Cavalry      offense +10%
War Chariots       offense +10%
```

## Forest

```text
defender defense +10%
Archers offense +10%
Light Infantry offense +10%
all cavalry offense -10%
War Chariots offense -20%
```

## Hills

```text
defender defense +10%
Archers offense +10%
cavalry offense -10%
War Chariots offense -15%
```

## Mountains

```text
defender defense +20%
cavalry offense -25%
War Chariots offense -40%
War Elephants offense -10%
```

## Desert

```text
War Camels offense +20%
Light Cavalry offense +5%
Heavy Infantry offense -5%
```

Keep these in configuration.

---

# 313. Fortification combat bonus

A defending directly owned/vassal province may have Fort/City Wall levels.

Recommended defensive bonus:

```text
+5% defense per fortification level
```

Recommended cap:

```text
+30%
```

Siege units reduce the effective fortification bonus.

---

# 314. Siege suppression of forts

Assign siege power:

```text
Ballista    1
Catapult    2      3
```

Attacking total siege power:

```text
Σ active siege power
```

Recommended fort suppression:

```text
5% of fort bonus removed per siege-power point
```

Maximum:

```text
80% of fortification bonus can be suppressed
```

This is a battlefield fortification effect only.

A separate long-term siege/conquest system may later be added.

---

# 315. Training combat effect

Training is `0..100`.

Recommended attack modifier:

```text
training_attack =
    1 + 0.15 × training/100
```

Recommended defense modifier:

```text
training_defense =
    1 + 0.25 × training/100
```

Therefore at `100` Training:

```text
+15% attack effectiveness
+25% effective defense against losses
```

This captures the intended idea that experienced troops survive better.

---

# 316. Morale combat effect

Use the unit's stored Morale plus Military Rank bonus, clamped to `0..100`.

Recommended strength-damage output modifier:

```text
strength_morale_factor =
    0.75 + 0.50 × morale/100
```

Range:

```text
0 Morale   → ×0.75
50         → ×1.00
100        → ×1.25
```

Recommended morale-damage output modifier:

```text
morale_attack_factor =
    0.50 + 1.00 × morale/100
```

Range:

```text
0 Morale   → ×0.50
50         → ×1.00
100        → ×1.50
```

High-Morale formations therefore break enemies faster.

---

# 317. Combat randomness

Each attack receives a small authoritative random multiplier:

```text
0.90 .. 1.10
```

The authoritative simulation generates the random result.

Clients must not independently roll combat.

Randomness should matter at the margins but not overwhelm:

- unit counters;
- tactics;
- Training;
- Morale;
- terrain;
- force composition.

---

# 318. Combat damage formula

For an attacking unit against a target:

```text
manpower_ratio =
    current_manpower / max_manpower

raw_attack =
    offense
    × manpower_ratio
    × matchup_modifier
    × tactic_modifier
    × training_attack
    × strength_morale_factor
    × terrain_attack_modifier
    × random_factor
```

Defensive strength:

```text
effective_defense =
    target.defense
    × target.training_defense
    × terrain_defense_modifier
    × fortification_modifier
```

Then:

```text
damage_ratio =
    raw_attack / effective_defense
```

Manpower loss:

```text
manpower_loss =
    BASE_MANPOWER_DAMAGE
    × damage_ratio
    × target.manpower_damage_taken
    × tactic_casualty_modifier
```

Morale loss:

```text
morale_loss =
    BASE_MORALE_DAMAGE
    × damage_ratio
    × morale_attack_factor
    × target.morale_damage_taken
```

Both base constants are configuration.

---

# 319. Simultaneous combat rounds

Damage within a combat round is simultaneous.

Process:

```text
1. determine all active targets
2. calculate all attacks from pre-round state
3. calculate all support attacks
4. sum incoming damage per unit
5. apply all manpower losses simultaneously
6. apply all Morale losses simultaneously
7. destroy 0-manpower units
8. mark 0-Morale units as routed
9. refill open slots from reserves for next round
```

A unit destroyed during the round still performs the attack already calculated for that same round.

This prevents processing-order advantage.

---

# 320. Combat rounds per month

Recommended:

```text
4 combat rounds per monthly tick
```

A battle can therefore last across multiple months.

After each monthly tick:

- casualties persist;
- Morale persists;
- tactic remains locked;
- battle remains active if neither side has broken.

Recommended safety cap:

```text
maximum battle duration = 6 months
```

If still unresolved after the cap:

```text
attacker must withdraw
```

Keep configurable.

---

# 321. Routing

A unit routes when:

```text
Morale <= 0
```

A routed unit:

```text
stops fighting
keeps surviving manpower
cannot return to the same battle
```

Routing does not replenish manpower.

A side loses the battle when it has no non-routed combat-capable units remaining.

Support/siege units count as combat-capable only after becoming exposed.

---

# 322. Voluntary retreat

A player may order retreat after:

```text
at least one full combat month
```

The retreat order executes at the next combat-round boundary.

A retreating side loses the battle.

Retreat is preferable to complete destruction when Morale is collapsing.

---

# 323. Retreat destination

Preferred retreat destination:

```text
attacker:
    origin province if still legal

otherwise:
    adjacent directly owned province
    own vassal
    province with valid military access
```

Choose shortest/legal option deterministically.

If no legal retreat province exists:

```text
routed/retreating units are destroyed
```

No prisoner-of-war system exists initially.

---

# 324. Battle result and Training

At battle end, surviving participating units gain Training.

Recommended:

```text
base participation gain = +2
```

Additional gain from friendly casualty ratio:

```text
0–10%      +0
10–25%     +1
25–50%     +2
50%+       +3
```

Winner may receive:

```text
+1 additional Training
```

Cap total battle gain.

This implements the intended rule that harder battles teach survivors more, without making deliberate slaughter efficient.

---

# 325. Battle result and Morale

Recommended post-battle effects:

Winner:

```text
+10 Morale
```

Loser survivors:

```text
Morale remains low after retreat
```

Do not reset Morale to 50 immediately.

Normal monthly Morale recovery later restores it.

---

# 326. Military Renown from battles

The battle result feeds Military Renown.

Renown should consider:

```text
victory
enemy effective strength
own effective strength
casualty intensity
battle importance
```

Suggested conceptual formula:

```text
base victory Renown
× opponent_strength_factor
× battle_scale_factor
```

A victory against a stronger force should be worth substantially more than crushing a tiny force.

Exact formula remains part of combat balancing.

---

# 327. Battle victory and occupation

If an attacker defeats the defending military force of an independent NPC province:

```text
occupation = attacker owner
```

starting after battle completion.

Military Control gain begins on the next monthly political-resolution tick.

If friendly/neutral local units were not hostile, no occupation is created.

---

# 328. Combat in owned enemy provinces

Defeating the owner's force does not allow ordinary Independent Control accumulation because owned provinces cannot be politically controlled through the Control system.

A hostile military occupation may:

```text
reduce local pop happiness
disrupt production
support future war/conquest mechanics
```

but ownership transfer itself is outside this combat specification.

Do not convert an owned province into an Independent Control pie merely because its army lost.

---

# 329. Battle with multiple friendly owners

Multiple friendly players may fight on the same side.

Each owner's Province Force keeps:

```text
its own selected tactic
its own Military Rank bonuses
its own units
```

Combat deployment combines allied units into the same side for frontage.

Tactic modifiers are calculated per attacking unit from that unit owner's tactic.

This avoids creating a temporary shared Army entity.

---

# 330. Movement orders are transient groups, not Armies

A player may select multiple units and move them together.

Represent this as:

```rust
struct MovementOrder {
    owner: PlayerId,
    unit_ids: Vec<UnitId>,
    origin: ProvinceId,
    destination: ProvinceId,
    progress: f32,
    required_progress: f32,
    tactic: CombatTactic,
}
```

A `MovementOrder` exists only while traveling.

It is not an Army.

When it arrives:

```text
units join the owner's Province Force
movement order is deleted
```

---

# 331. Movement speed uses slowest unit

For a MovementOrder:

```text
force_speed =
    minimum movement_speed of all selected units
```

Examples:

```text
Light Cavalry + Horse Archers
→ speed 4.0

Light Cavalry + Heavy Infantry
→ speed 2.5

Light Cavalry + Catapult
→ speed 1.5
```

This is intentional.

The player may move fast units separately if speed matters.

---

# 332. Province crossing distance

Movement time depends on province size.

For every province calculate a normalized crossing length:

```text
crossing_length =
    sqrt(province_area / median_province_area)
```

Thus:

```text
median-sized province → 1.0
larger province        → >1.0
smaller province       → <1.0
```

Using square root approximates linear travel distance from area.

Cache this value.

---

# 333. Terrain movement modifier

Recommended movement-cost multipliers:

```text
Farmland      0.90
Plains        1.00
Forest        1.20
Hills         1.25
Mountains     1.60
Desert        1.30
```

This affects travel through that province.

---

# 334. Roads and movement

Road levels reduce the crossing cost of the province containing the road.

Recommended:

```text
road_speed_bonus_per_level = +15%
```

Equivalent crossing-cost formula:

```text
road_cost_multiplier =
    1 / (1 + 0.15 × road_level)
```

Optional minimum:

```text
road_cost_multiplier >= 0.50
```

so roads cannot reduce travel time without limit.

Roads do not directly improve combat stats.

---

# 335. Edge travel cost

Movement from adjacent Province A to Province B consumes half a crossing of each province.

Calculate:

```text
cost_A =
    0.5
    × crossing_length[A]
    × terrain_move_modifier[A]
    × road_cost_multiplier[A]

cost_B =
    0.5
    × crossing_length[B]
    × terrain_move_modifier[B]
    × road_cost_multiplier[B]

edge_distance_cost =
    cost_A + cost_B
```

Then:

```text
required_months =
    BASE_MOVEMENT_SCALE
    × edge_distance_cost
    × REFERENCE_SPEED
    / force_speed
```

Recommended:

```text
REFERENCE_SPEED = 2.5
BASE_MOVEMENT_SCALE tuned so median plains infantry crossing ≈ 1 month
```

Movement time may be fractional internally.

---

# 336. Monthly movement progress

Each monthly movement tick:

```text
movement_progress += 1 month
```

When:

```text
progress >= required_months
```

the force enters the destination province.

Minimum observable adjacent movement time:

```text
1 monthly tick
```

Very large, mountainous, roadless provinces may require several months.

Roads and faster unit composition reduce the duration.

---

# 337. Multi-province movement

The player may queue a destination several provinces away.

Pathfinding uses:

```text
fastest legal route
```

with edge cost derived from:

- province size;
- terrain;
- roads;
- unit force speed;
- access/hostility.

Use Dijkstra/A* on the province adjacency graph.

Movement is still resolved edge-by-edge.

After each province arrival:

```text
revalidate next edge
```

because:

- access may change;
- war may begin;
- hostile troops may appear.

---

# 338. Entering a hostile province

When a MovementOrder completes into a province containing hostile units:

```text
movement stops
battle begins
```

The movement order's chosen tactic becomes the arriving force's locked battle tactic.

The defending force uses its currently selected province tactic.

If no hostile defending force exists:

```text
movement completes normally
```

For independent NPC occupation, military Control only starts if the relevant local defender has been defeated.

---

# 339. Map unit sprite requirement

At implementation time, create/generate animated map sprites for every initial unit type:

```text
Light Infantry
Heavy Infantry
Archers
Light Cavalry
Heavy Cavalry
Horse Archers
War Chariots
War Camels
War Elephants
Ballista
Catapult
```

Sprites must match the existing game's visual style.

Do not use one generic soldier sprite for all unit types.

---

# 340. Required unit sprite animations

Each unit type must have at least:

```text
Idle
Move
Combat
```

Recommended where practical:

```text
Rout
```

For siege units:

```text
Idle
Move
Fire
```

Animation frame count should remain compact and appropriate for map scale.

Recommended:

```text
4–8 frames per loop
```

unless the existing asset pipeline favors another format.

---

# 341. Unit sprite generation and asset pipeline

Implementation must:

```text
1. generate/create sprites for every unit type;
2. process them through the same optimized image pipeline as other game assets;
3. load them lazily/appropriately according to the existing asset architecture;
4. test that every configured UnitType maps to valid visual assets;
5. never ship a unit type with only a placeholder once implementation is complete.
```

If the project uses KTX2 or another optimized texture format, follow the repository's existing asset conventions.

---

# 342. Map level-of-detail for forces

Military sprites are shown only when sufficiently zoomed in.

Recommended:

```text
Far zoom:
    show no individual military sprites

Medium zoom:
    optional compact owner/unit marker if needed

Close zoom:
    show animated representative unit sprites in province
```

The user's explicit preference is:

```text
zoomed out → no visible individual soldiers
zoomed in  → animated units visibly stand in province
```

Do not clutter the strategic map at far zoom.

---

# 343. Representative unit sprites

A Province Force may contain many units.

Do not render one map sprite per actual unit.

Instead determine composition by surviving effective manpower.

For each owner in province:

```text
type_share =
    effective manpower of type
    / total effective manpower
```

At close zoom:

```text
show dominant unit-type sprite
```

Example:

```text
8 Heavy Infantry
2 Archers
1 Light Cavalry
```

should visually show:

```text
animated Heavy Infantry
```

because Heavy Infantry dominates the force.

Optional:

```text
show up to 3 representative sprites
```

when several unit types each exceed a configured share threshold.

Recommended:

```text
secondary type shown if >=25% of force
maximum 3 representative sprites per owner
```

---

# 344. Multiple owners' sprites in one province

If multiple peaceful forces coexist:

```text
Player A
Player B
NPC local force
```

render separate small clusters around the province's display anchor.

Each cluster should use:

- owner's color;
- banner/ring/outline;
- representative animated unit sprite(s).

Clusters must not overlap exactly.

---

# 345. Movement map animation

While units are traveling and map zoom is close enough:

```text
show an animated movement sprite
```

Use the MovementOrder's dominant unit type.

The sprite interpolates from origin province anchor toward destination province anchor based on:

```text
progress / required_progress
```

Example:

```text
mostly Heavy Infantry movement order
→ walking Heavy Infantry animation

mostly Light Cavalry
→ cavalry movement animation
```

At far zoom, movement sprites remain hidden or use a compact marker only if later desired.

---

# 346. Battle map animation

When a battle is active and zoomed in:

```text
show opposing representative Combat animations
```

Do not attempt to render every soldier.

Use the dominant active unit types for each side.

Optional battle indicator at medium zoom may show:

```text
crossed-weapons icon
owners
approximate current strength
```

The detailed unit state remains in the battle panel.

---

# 347. Battle UI

Battle panel should show:

```text
Province / terrain
Attacker(s)
Defender(s)

Selected tactics
Tactic counter relationship
Tactic effectiveness

Combat width

For each side:
    active center units
    flank units
    support units
    reserves

For each unit:
    manpower
    Morale
    Training

Terrain modifiers
Fortification modifier
Military Rank modifier

Current battle month / round
```

The UI should explain key matchup effects.

Example tooltip:

```text
Heavy Infantry vs Light Cavalry:
+25% matchup effectiveness
```

---

# 348. Pre-battle estimate

Before entering a known hostile province, show a rough battle estimate.

Use visible/known information:

```text
unit composition
Training
Morale
Military Rank
terrain
selected tactic
known enemy composition
```

If enemy tactic is unknown:

```text
do not assume it in the prediction
```

Present:

```text
Favorable
Even
Unfavorable
```

plus key reasons.

Do not show a fake precise guaranteed win percentage.

---

# 349. Combat configuration

Centralized configuration must include:

```text
unit offense
unit defense
unit movement speed
unit maneuver
unit morale-damage-taken
unit manpower-damage-taken

full unit matchup matrix

tactic counter graph
tactic composition-fit matrix
tactic damage bonus
tactic countered penalty
tactic casualty modifiers

combat widths by terrain

deployment priorities
support-row effectiveness
exposed-support casualty multiplier

terrain combat modifiers

fort defense per level
fort defense cap
siege suppression values

Training attack/defense scaling
Morale combat scaling

combat random range

base manpower damage
base Morale damage

combat rounds/month
maximum battle duration

retreat rules

post-battle Training
post-battle Morale
Military Renown scaling

movement speeds
terrain movement costs
road speed bonus
province area normalization
base movement scale

sprite LOD thresholds
representative sprite thresholds
```

---

# 350. Combat invariants

The implementation must enforce:

```text
combat is province-force based; no Army entity is introduced

each owner has one selected tactic per Province Force

tactic locks when battle begins

tactics have counters and weaknesses

tactic bonus depends on surviving force composition

unit matchups are explicit and asymmetric

Training affects combat

Morale affects combat and routing

terrain affects combat

frontage limits active units

ranged/siege support is protected only while front line exists

maneuver enables flanking

combat-round damage is simultaneous

casualties remain permanent

no reinforcement occurs

only surviving units gain post-battle Training

military movement uses slowest selected unit

province size affects movement time

terrain affects movement time

Road levels improve movement through that province

movement resolves edge-by-edge

entering hostile force stops movement and starts battle

unit sprites are hidden at far zoom

animated representative sprites appear at close zoom

every UnitType has appropriate generated animated assets
```

---

# 351. Combat explicit non-goals

Do not add unless separately designed:

```text
persistent Army objects
manual per-slot formation placement
individual soldier simulation
real-time twitch control
reinforcement
healing manpower
monthly Metal upkeep
naval combat
prisoners of war
edge-interception battles between provinces
full long-term siege system
unique cultural tactic trees beyond the initial generic tactics
```

---

# 352. Pre-combat formation plan

Before a force enters combat, the player may configure a formation plan inspired by the cohort deployment controls in *Imperator: Rome*.

The player chooses:

```text
Primary line unit type
Secondary line unit type
Flanking unit type
Flank size
Combat tactic
```

This does **not** mean manually placing individual units into exact combat slots. The player specifies preferred roles/types, and combat deploys actual surviving units automatically according to those preferences.

Recommended state:

```rust
struct BattlePlan {
    primary_unit_type: UnitType,
    secondary_unit_type: UnitType,
    flank_unit_type: UnitType,
    flank_size: u8,
    tactic: CombatTactic,
}
```

Each Province Force stores a saved default `BattlePlan`. A `MovementOrder` stores a snapshot of the selected plan. Once combat begins, the plan is locked for that battle.

---

# 353. Primary line unit type

`Primary line` is the preferred type used to fill the center/front line first.

Recommended selectable primary types:

```text
Light Infantry
Heavy Infantry
Heavy Cavalry
War Chariots
War Camels
War Elephants
```

Example:

```text
Primary: Heavy Infantry
```

Deployment attempts to use available Heavy Infantry in center slots before other center-capable units.

---

# 354. Secondary line unit type

`Secondary line` is the preferred fallback/replacement type for the center.

Example:

```text
Primary: Heavy Infantry
Secondary: Light Infantry
```

Center deployment order:

```text
1. Primary type
2. Secondary type
3. best remaining eligible front-line unit
```

Secondary units also preferentially replace routed/destroyed center units from reserve.

---

# 355. Flanking unit type

`Flank type` is the preferred unit placed on both wings.

Recommended selectable flank types:

```text
Light Cavalry
Heavy Cavalry
Horse Archers
War Camels
War Chariots
Light Infantry
```

If there are not enough preferred flank units:

```text
1. use all available preferred flank units;
2. fill remaining flank slots with the best remaining mobile units;
3. if still insufficient, use other combat-capable units;
4. never leave an otherwise fillable slot empty merely because the preferred type is unavailable.
```

---

# 356. Flank size

The player chooses how many front-line slots on **each side** are reserved for the flank.

Initial options:

```text
1
2
3
```

Thus:

```text
total flank slots = flank_size × 2
```

Example at combat width `8`.

Flank size `1`:

```text
[LC][HI][HI][HI][HI][HI][HI][LC]
```

Flank size `2`:

```text
[LC][LC][HI][HI][HI][HI][LC][LC]
```

Flank size `3`:

```text
[LC][LC][LC][HI][HI][LC][LC][LC]
```

A larger flank is not automatically better: it activates more mobile troops immediately but weakens the center by consuming frontage.

---

# 357. Flank size and narrow terrain

Actual deployed flank size is constrained by combat width.

Recommended:

```text
MIN_CENTER_SLOTS = 2

effective_flank_size =
    min(
        requested_flank_size,
        floor((combat_width - MIN_CENTER_SLOTS) / 2)
    )
```

Example:

```text
Mountain combat width = 4
Requested flank size = 3
Effective flank size = 1

[Flank][Center][Center][Flank]
```

The pre-combat UI must show the expected effective flank size when terrain is known.

---

# 358. Deterministic formation deployment

At battle start:

```text
1. determine combat width;
2. determine effective flank size;
3. reserve left/right flank slots;
4. fill flank slots with preferred flank type;
5. fill remaining flank slots with fallback mobile units;
6. fill center slots with Primary type;
7. fill remaining center slots with Secondary type;
8. fill remaining center slots with best eligible units;
9. fill support row with Archers, Ballista, Catapult;
10. place all remaining units in reserve.
```

When several units of the same type compete for a slot, prefer:

```text
1. higher current manpower ratio
2. higher Training
3. higher Morale
4. stable UnitId tie-break
```

This makes deployment deterministic and preferentially commits healthier veteran formations.

---

# 359. Reserve replacement priority

For an empty center slot:

```text
Primary-type reserve
→ Secondary-type reserve
→ best remaining center-capable reserve
```

For an empty flank slot:

```text
preferred Flank-type reserve
→ best remaining mobile reserve
→ any remaining combat-capable reserve
```

Support replacements prioritize:

```text
Archers
Ballista
Catapult
```

---

# 360. Flank position versus Maneuver

Flank size and Maneuver are different mechanics.

```text
Flank size:
    determines how many wing slots are reserved and where flankers begin.

Maneuver:
    determines how far sideways an active unit may attack once gaps appear.
```

Example:

```text
Horse Archers
Flank position: outer wing
Maneuver: 5
```

Once the opposing wing collapses, Horse Archers may reach several positions inward. Heavy Infantry with Maneuver `1` has far less ability to exploit the opening.

---

# 361. Formation and tactic interaction

The player chooses both formation preferences and tactic.

Example:

```text
Primary: Heavy Infantry
Secondary: Light Infantry
Flank: Light Cavalry
Flank size: 3
Tactic: Envelopment
```

This can produce strong Envelopment fit if enough cavalry is actually present.

Another example:

```text
Primary: Heavy Infantry
Secondary: Light Infantry
Flank: Heavy Cavalry
Flank size: 1
Tactic: Bottleneck
```

is better suited to narrow terrain and a strong center.

Tactic effectiveness continues to use actual surviving composition. Merely selecting a unit type does not create tactic fit if those units are absent.

---

# 362. Editing and locking Battle Plans

A BattlePlan may be edited:

```text
while units are stationary and not in combat
before issuing a MovementOrder
while preparing an attack
```

A `MovementOrder` takes a snapshot when issued.

The player may update the movement plan before the units cross into the destination province, provided combat has not started.

Once battle begins, the following are locked until battle ends:

```text
Primary type
Secondary type
Flank type
Flank size
Tactic
```

---

# 363. Saved Province Force defaults

Each player's Province Force stores a default BattlePlan so configuration does not need to be repeated for every battle.

Example:

```text
Achaia Force Battle Plan

Primary: Heavy Infantry
Secondary: Light Infantry
Flank: Light Cavalry
Flank size: 2
Tactic: Envelopment
```

New MovementOrders from that province inherit the default. The player may override the plan for a specific movement without changing the saved default.

---

# 364. Pre-combat Battle Plan UI

Before entering a known hostile province, show:

```text
BATTLE PLAN

Tactic:
[ Envelopment ▼ ]

Primary line:
[ Heavy Infantry ▼ ]

Secondary line:
[ Light Infantry ▼ ]

Flanking unit:
[ Light Cavalry ▼ ]

Flank size:
[ 1 ] [ 2 ] [ 3 ]

Expected combat width:
8

Expected effective flank size:
2
```

Render a formation preview:

```text
LC  LC | HI  HI  HI  HI | LC  LC
```

If preferred units are unavailable, preview the expected fallback deployment instead.

---

# 365. Battle Plan preview information

Also show:

```text
Terrain
Fortification
Your Tactic fit
Known enemy composition
Known enemy Training/Morale if visible
Enemy Military Rank if public
Expected arrival time
```

Provide a qualitative estimate:

```text
Favorable
Even
Unfavorable
```

with explanatory reasons.

Example:

```text
Assessment: Favorable

+ Light Cavalry strongly counters enemy Archers
+ Envelopment has 82% composition fit
+ Your Training is higher
- Enemy Heavy Infantry counters cavalry frontally
- Defender has Fort II
```

Do not show a fake precise win probability.

---

# 366. Unknown enemy tactic

The defender's selected tactic is not automatically revealed before battle.

UI:

```text
Enemy tactic: Unknown
```

The player must decide whether to:

```text
choose a specialized tactic
use Balanced to avoid a counter weakness
infer likely tactic from enemy composition
```

Only future intelligence/espionage mechanics may reveal the enemy tactic beforehand.

---

# 367. Pre-combat player choices

Before intentionally entering combat, the player controls:

```text
which units move
route
timing
whether to wait for more Training/Morale
whether to bring slow Ballista/Catapult support
Primary line type
Secondary line type
Flank type
Flank size
Combat Tactic
whether to attack at all
```

Once combat begins, intervention remains intentionally limited to:

```text
continue
or
retreat when permitted
```

Do not add manual per-round unit repositioning.

---

# 368. Formation invariants

The implementation must enforce:

```text
BattlePlan has Primary, Secondary, Flank type, Flank size, and Tactic

Flank size means slots per side

requested flank size is clamped by actual combat width

formation deployment is deterministic

player does not manually place individual units

center fills Primary before Secondary

flanks fill preferred flank type before fallback mobile units

support row uses Archers, Ballista, Catapult

remaining units enter reserves

Maneuver remains separate from Flank size

BattlePlan locks when combat begins

tactic fit uses actual units, not selected labels

saved Province Force BattlePlans are supported

MovementOrders may snapshot or override Province Force default BattlePlan
```
