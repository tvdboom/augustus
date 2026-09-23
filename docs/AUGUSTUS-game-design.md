# AUGUSTUS: Game Design Document

---

## 1. Executive Summary & Core Pillars

**Title:** *AUGUSTUS*

**Genre:** Grand Strategy / Historical Simulation

**Core Premise:** Transform a fragile Roman Republic into an enduring empire. Rather than micro-managing dozens of identical provinces, players manage **~10 Great Urban Hubs** and issue **Empire-Wide Edicts** to govern a dynamic pop economy, organic migrations, and tactical legionary warfare.

### Core Pillars

1. **Urban-Rural Dualism**: ~10 major Cities handle administration, processing, high-tier class housing, and civic buildings. ~40 Rural Countryside Provinces extract raw resources without building slot micro-management.
2. **Macro Edicts Over Micro**: Manage class mobility, taxation, and recruitment via empire-wide decrees and regional blueprints.
3. **Dynamic Per-Tick Engine**: Population growth, health, and happiness operate as continuous rates of change ($\Delta / \text{tick}$) rather than flat static modifiers.
4. **Economic Substitution**: Drafting soldiers converts productive civilian taxpayers into costly military assets, directly impacting regional tax bases and food outputs.
5. **Reconnaissance & Rock-Paper-Scissors Warfare**: Dynamic intelligence levels dictate tactical visibility, where stance counters and morale determine battle outcomes.

---

## 2. Spatial Map & Demographic Architecture

The map is divided into **~50 Total Provinces**, sharing a uniform underlying data framework. The distinction between urban and rural domains is determined by a **City Visual Overlay** that unlocks high-tier urban building slots.

```
+-----------------------------------------------------------------------+
|                       UNIFIED PROVINCE FRAMEWORK                      |
|  - Data: Terrain, Resource Node, Population Tiers, Road Connections   |
|  - Logic: Dynamic Food Ratios, Per-Tick Growth/Happiness Drift        |
+-----------------------------------++----------------------------------+
                                    ||
           +------------------------++------------------------+
           |                                                  |
           v                                                  v
+-----------------------------------+              +-----------------------------------+
|     CITY PROVINCE (~10 TOTAL)     |              |    RURAL PROVINCE (~40 TOTAL)     |
| - High-tier Urban Graphic         |              | - Extraction Field Graphic        |
| - 6 Base Building Slots (Max 8)   |              | - NO Urban Building Slots         |
| - Class Bias:                     |              | - Field Projects: Roads, Dikes    |
|   25% Nobles, 45% Citizens,       |              | - Class Bias:                     |
|   20% Plebeians, 10% Slaves       |              |   5% Nobles, 15% Citizens,        |
| - Role: Processing, Trade, Tax,   |              |   45% Plebeians, 35% Slaves       |
|   Law, High-tier Military Draft   |              | - Role: Grain, Metal, Timber,     |
|                                   |              |   Livestock Extraction            |
+-----------------------------------+              +-----------------------------------+

```

---

## 3. Demographics, Overcrowding & Organic Migration

### Pop Carrying Capacity

* **Base Province Capacity**: 100 Pops.
* **Overcrowding Unhappiness Penalty**: Applied to all local Pop tiers when capacity is exceeded:

$$\text{Overcrowding Penalty} = -5 \text{ Happiness per } 10\% \text{ over Capacity}$$



### Organic Inter-Provincial Migration

On every monthly tick (3 seconds), Pops automatically migrate between adjacent owned provinces based on a **Push-Pull Gradient**.

```
[Overcrowded / Low Happiness Origin] ── (Push Force) ──> [Adjacent Free Capacity Destination]

```

* **Push Index ($P_{\text{push}}$)**:

$$P_{\text{push}} = \max\left(0, \frac{\text{Current Pops} - \text{Capacity}}{\text{Capacity}}\right) \times 1.5 + \left( \frac{100 - \text{Avg Local Happiness}}{100} \right)$$


* **Pull Index ($P_{\text{pull}}$)**:

$$P_{\text{pull}} = \max\left(0, \frac{\text{Capacity} - \text{Current Pops}}{\text{Capacity}}\right) \times \left( \frac{\text{Avg Local Happiness}}{100} \right)$$


* **Monthly Flow Rate**:

$$\text{Migrants}_{\text{tick}} = \text{Current Pops}(A) \times 0.5\% \times \Big( P_{\text{push}}(A) \times P_{\text{pull}}(B) \Big) \quad (\text{Capped at } 3.0\% \text{ max / tick})$$



### Class Migration Rules

* **Plebeians & Citizens**: Follow standard push-pull migration paths toward open capacity and high-happiness zones.
* **Slaves**: Static workforce ($0\%$ natural migration). Relocated via supply channels or moved during Slave Revolts.
* **Nobles**: Migrate only if local Noble Happiness drops below 30 or during an active siege, fleeing directly toward the **Capital Province** or nearest **Forum Seat**.
* **Refugee Influx**: If an adjacent foreign province drops below 15 Happiness or suffers famine, $1.0\%$ of free foreign Pops per tick flee across borders into adjacent owned provinces with open capacity.

---

## 4. Class Mobility & Imperial Edicts

Class mobility operates as upfront **Batch Actions** combined with **Lock-out Cooldown Periods** to eliminate micro-management.

```
+------------------------+-----------------------+-------------------+-----------------+-----------------------------------+
| Edict Name             | Target Conversion     | Batch % / Scope   | Total Cooldown  | Immediate Happiness Shift         |
+------------------------+-----------------------+-------------------+-----------------+-----------------------------------+
| Imperial Manumission   | Slaves ➔ Plebeians    | 10% Domain Slaves | 24 Ticks (2 Yrs)| +15 Slave, -10 Noble              |
| Civic Enfranchisement  | Plebeians ➔ Citizens  | 5% Domain Plebs   | 36 Ticks (3 Yrs)| +10 Citizen, -15 Noble            |
| Patrician Elevation    | Citizens ➔ Nobles     | 1 Pop / 5 Cities  | 48 Ticks (4 Yrs)| +20 Noble, -15 Plebeian           |
| Debt Bondage Decree    | Free Pops ➔ Slaves    | 5% Domain Plebs   | 24 Ticks (2 Yrs)| -35 Target, +15 Noble (*Scandal*) |
+------------------------+-----------------------+-------------------+-----------------+-----------------------------------+

```

### Provincial Stance Overrides

To protect specific high-value production nodes, players assign provincial stances via simple toggles:

* **Balanced (Default)**: Fully participates in all global edicts.
* **Protected Workforces**: Excludes local Slaves from Imperial Manumission.
* **Aristocratic Seat**: Locks Citizens from Patrician Elevation or Enfranchisement in the province.

---

## 5. Dynamic Growth & Binary Food Engine

### Dynamic Per-Tick Drift ($\Delta / \text{tick}$)

Modifiers from buildings, tax rates, and health adjust provincial trajectory continuously over time:

* **Aqueduct & Sewers**: $+0.2$ Happiness / tick drift until cap is reached.
* **Heavy Taxation**: $-0.5$ Happiness / tick.
* **Military Draft Pressure**: $-0.8$ Happiness / tick.
* **Event Exceptions (e.g., Ludi Games)**: Immediate flat $+15$ Happiness spike, decaying by $-3$ / tick post-event.

### Binary Food & Famine Escalation Model

Food is tracked at the global level against total civilian and military consumption.

```
[ GLOBAL FOOD STOCKPILE ]
   │
   ├─► Surplus (Food > 0) ───► Baseline Organic Growth & Stability
   │
   └─► Depleted (Food = 0) ──► FAMINE TIMER ACTIVATED
                                 │
                                 ├─► Ticks 1–3 (Shortage)  : Pop Growth Freezes (ΔPops = 0)
                                 │                           -0.5 Happiness/tick (Plebs/Slaves)
                                 │
                                 ├─► Ticks 4–8 (Starvation) : -2.0 Pops/tick mortality
                                 │                           -2.0 Happiness/tick (All tiers)
                                 │                           10% Casualty & -5 Morale/tick (Legions)
                                 │
                                 └─► Ticks 9+ (Collapse)   : -5.0 Pops/tick mortality
                                                             Automatic Rebel/Slave Uprisings
                                                             Legion Mutinies into Rogue Bandits

```

---

## 6. Infrastructure, Building Trees & Macro Blueprints

### City Building Tree (6–8 Slots per City)

#### Infrastructure & Capacity

* **Aqueduct & Sewers**: Cost: 150 Gold, 30 Stone | Build Time: 4 Ticks | Bonus: **+50 Pop Capacity**, $+0.2$ Happiness/tick, $+20\%$ Attraction Pull Index ($P_{\text{pull}}$).
* **Insulae Districts**: Cost: 100 Gold, 20 Wood | Build Time: 2 Ticks | Bonus: **+30 Pop Capacity**, $+0.1\%$ Pop Growth rate.
* **Forum Expansion**: Cost: 200 Gold, 40 Stone | Build Time: 6 Ticks | Bonus: **+20 Pop Capacity**, $+15\%$ Local Tax Generation, $+10$ Noble Happiness.

#### Industry & Trade

* **Central Granaries & Latifundia**: Cost: 120 Gold, 20 Wood | Build Time: 3 Ticks | Bonus: **+50 Food Yield / tick**, $+15\%$ Slave Work Efficiency.
* **Grand Mint & Emporium**: Cost: 180 Gold, 25 Stone | Build Time: 5 Ticks | Bonus: **+25% Trade Revenue**, $+10$ Gold/tick baseline, $+5\%$ Citizen Income.
* **Ergastulum**: Cost: 100 Gold, 30 Metal | Build Time: 3 Ticks | Bonus: **+30 Trade Goods / Metal Yield / tick**, $+25\%$ Slave Output ($+0.2$ Slave Revolt Risk / tick).

#### Military & Logistics

* **Castra (Garrison)**: Cost: 150 Gold, 40 Wood, 20 Metal | Build Time: 4 Ticks | Bonus: **Draft Speed set to 1 Tick**, $+15\%$ Garrison Defense.
* **Campus Martius**: Cost: 220 Gold, 30 Stone, 30 Metal | Build Time: 6 Ticks | Bonus: Drafted Legions spawn with **+20 Starting Morale** and +1 Veteran Rank.
* **Stone Ramparts**: Cost: 250 Gold, 60 Stone | Build Time: 8 Ticks | Bonus: **Siege Holdout Time +12 Ticks**, $2\times$ Siege Attrition to attackers.

#### Public Spectacles & Culture

* **Amphitheatre**: Cost: 200 Gold, 40 Stone | Build Time: 5 Ticks | Bonus: Unlocks **Ludi Games** event, $+15$ Plebeian Happiness on activation.
* **Basilica**: Cost: 180 Gold, 30 Stone | Build Time: 4 Ticks | Bonus: **+25% Local Espionage Shield**, $+5$ Influence generation.

### Rural Field Projects (No Building Slots Required)

* **Stone Roads**: Cost: 50 Gold, 10 Stone | Build Time: 2 Ticks | Bonus: **+50% Army Movement Speed**, $+0.5\%$ Migration Flow Rate to adjacent lands.
* **Irrigation Dikes**: Cost: 40 Gold, 10 Wood | Build Time: 2 Ticks | Bonus: **+1.5 Food Output / tick**, $+0.2$ Pop Capacity Growth / tick.

### Macro Automation & Blueprint Engine

1. **Construction Presets**: Players assign macro focus presets (**Breadbasket**, **Industrial Extraction**, **Militarized Marches**) to entire regional clusters.
2. **Auto-Spend Slider**: Set a treasury buffer (e.g., 1,000 Gold). Excess income automatically funds blueprint structures in empty slots based on priority queues (Overcrowded $\rightarrow$ Frontier Borders $\rightarrow$ Resource Nodes).
3. **Regional Grouping**: Issue 1-click upgrade commands across all provinces within geographical regions (*Italia*, *Hispania*, *Aegyptus*).

---

## 7. Legion Recruitment, Maintenance & Economic Substitution

Drafting converts productive taxpayers into army assets, resulting in tax loss and workforce decay.

```
CIVILIAN POP:   Generates Tax Gold (-2 Gold/tick lost)  +  Consumes Standard Food
                          │  (Drafting Action)
                          v
LEGIONARY UNIT: Consumes Military Wages (3 Gold/tick)   +  Consumes Double Food Rations (2.0 Food/tick)

```

### Recruitment Parameters

* **Demographic Consumption**:
* Standard Legions consume **1 Plebeian Pop unit**.
* Cavalry / Auxiliaries consume **1 Citizen Pop unit**.


* **Draft Limits**: Maximum **5% of a province's free population drafted per 12 ticks**.
* **Draft Speed**: Base 1 Legion per 3 ticks (Reduced to **1 tick** with a Castra).
* **Demobilization**: Disbanding a Legion inside an owned province returns **1 Pop unit** to the local workforce.

---

## 8. Tactical Combat & Reconnaissance Matrix

### Unit Roster & Operational Profile

```
+--------------------+-------------------+---------------+-------------------+--------+---------+-------+---------------------------------------+
| Unit Class         | Required Pop Tier | Upfront Cost  | Upkeep / Tick     | Attack | Defense | Speed | Tactical Role                         |
+--------------------+-------------------+---------------+-------------------+--------+---------+-------+---------------------------------------+
| Hastati / Velites  | Plebeian          | 20 G, 10 M    | 2 Gold, 1.5 Food  | 10     | 8       | High  | Light Infantry / Harassment           |
| Roman Legionary    | Plebeian          | 50 G, 25 M    | 3 Gold, 2.0 Food  | 25     | 30      | Med   | Heavy Line Infantry                   |
| Equites Cavalry    | Citizen           | 90 G, 15 M    | 6 Gold, 2.5 Food  | 35     | 18      | V.High| Flanking / Rear Line Disruption       |
| Auxiliary Archers  | Plebeian          | 40 G, 10 W    | 3 Gold, 1.5 Food  | 20     | 5       | Med   | Backline Ranged Fire                  |
| Siege Engines      | Slaves (Crew)     | 150 G, 50 W   | 8 Gold, 3.0 Food  | 40     | 10      | Low   | Fortification Breakers & Splash Damage|
+--------------------+-------------------+---------------+-------------------+--------+---------+-------+---------------------------------------+

```

### Stance Counter Matrix (Rock-Paper-Scissors)

Tactical stances modify base damage by **$\pm 30\%$**:

```
              +--------------------------------------+
              |                                      |
              v                                      |
       [1. SHOCK ACTION] ─── (Counters) ───► [2. SKIRMISH]
              │                                      │
          (Counters)                             (Counters)
              │                                      │
              v                                      v
       [3. ENVELOPMENT]  ◄─── (Counters) ─── [4. BOTTLE-NECK]

```

1. **Shock Action (Frontal Charge)**: $+30\%$ vs. Skirmish; $-30\%$ vs. Envelopment.
2. **Skirmish (Harass & Retreat)**: $+30\%$ vs. Bottleneck; $-30\%$ vs. Shock Action.
3. **Envelopment (Cavalry Flank)**: $+30\%$ vs. Shock Action; $-30\%$ vs. Bottleneck.
4. **Bottleneck / Testudo (Defensive Wall)**: $+30\%$ vs. Envelopment; $-30\%$ vs. Skirmish.

* **Morale & Rout**: Armies rout when average Morale drops below **20%**, suffering $+50\%$ damage from pursuing cavalry while fleeing to adjacent provinces.

### Reconnaissance & Fog of War Matrix

Knowing the enemy army composition is required to counter their tactical stances:

```
+------------------+---------------------------------------------------------------------------------+
| Intelligence Level| Information Revealed                                                            |
+------------------+---------------------------------------------------------------------------------+
| Level 0          | Unknown Enemy Stack (Banner model only).                                        |
| Level 1          | Estimated Headcount (~4,000–5,000 Troops) and Commander Rank.                   |
| Level 2          | Exact Unit Category Breakdown (e.g., 3k Infantry, 1k Cav, 500 Archers).         |
| Level 3          | Complete Unit Stats, Morale, Health, and currently selected Tactical Stance.   |
+------------------+---------------------------------------------------------------------------------+

```

#### Sources of Intelligence

* **Covert Espionage**: Embedded Spies provide passive Level 2 Info across target regions. Executing an *Infiltrate Camp* mission grants **Level 3 Info** for 12 ticks.
* **Scout Cavalry**: Armies with at least 1 Light Cavalry/Equites unit grant **Level 2 Info** on adjacent enemy stacks.
* **Territorial Vision**: Moving through owned territory or adjacent to a border **Castra** grants passive **Level 2 Info**.
* **Deception**: Cavalry screening lowers enemy intelligence levels by **-1 Level**. Deception Stances mask true tactical choices until combat engagement.

---

## 9. Victory Condition: The Ascension to *AUGUSTUS*

To win the campaign, a player must fulfill the constitutional, military, and civic milestones required to have the Senate bestow the ultimate title: **AUGUSTUS**.

```
                         AUGUSTUS VICTORY MILESTONES
  ┌───────────────────────────────────┼───────────────────────────────────┐
  ▼                                   ▼                                   ▼
[MILITARY HEGEMONY]                 [CIVIC CONSENSUS]                   [ECONOMIC STABILITY]
• Imperium Maius Active             • Avg Happiness > 60 across          • Positive Global Food Balance
• Control Rome + 8 Great Cities       Plebeians, Citizens & Nobles       • Treasury Buffer > 5,000 Gold
• Zero Active Legion Mutinies        • Senate Loyalty > 75%              • No Active Famine Timers

```

Upon fulfilling all milestones, the Senate convenes to grant the player the lifelong title of **Augustus**, transitioning the Roman Republic into an Imperial Principate and securing campaign victory.