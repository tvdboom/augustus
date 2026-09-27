# Numbered implementation audit

This checklist maps **all 368 numbered sections** of the supplied v10 document, copied to [AUGUSTUS-game-design.md](AUGUSTUS-game-design.md), to the actual local implementation. Later sections override earlier examples; explicit user clarifications override document prose. “Implemented” means a concrete engine or application code path is present and mapped here. It does not mean that every combination has been exhaustively tested, visually approved, or proven balanced.

## Scope and resolved decisions

- The existing political ladder retains **Censor between Praetor and Consul**, with a Senate promotion, minimum 150 Influence nomination and +3/month rank income. Other rank incomes follow the specification.
- The user chose local gameplay first. Up to four local players share authoritative in-process state. Online synchronization, hidden-state server validation and save/load are outside this delivery; the existing lobby/Supabase boundary remains.
- NPC civilians are abstractly provisioned. Their Food shortages still affect demand and price. Local NPC defenders use the same abstract provisioning report; player civilians and troops consume real global Food.
- NPC forces defend their own territories only, as clarified by the user. They do not conduct offensive expansion or automatically rebuild destroyed troops. Economic NPC behavior chooses a resource focus and evaluates bounded trade; espionage supplies bounded hidden opportunities.
- Enemy-owned provinces transfer ownership immediately after a victorious invasion, as requested. Independent NPC provinces use occupation and subsequent Control accumulation.
- The first complete Catapult roster/tactic row resolves the duplicate rows. The isolated Onager mention does not add another unit type.
- Recruitment uses one tenth of the illustrative manpower to match existing aggregate population. Noble Influence is 0.25/month per Noble, avoiding an approximately 200-month opening wait for Aedile at a typical six-Noble start. With 40 opening Influence and no other changes, that illustration becomes 40 months. This is initial calibration, not a balance proof.
- Aegyptus uses fertile Nile farmland for economic carrying capacity while retaining its existing map/combat terrain identity.
- Births receive an additional configurable `min(1, 1.5 / population_ratio)` space modifier. This preserves normal growth through 150% capacity and restores a soft demographic equilibrium even when high-level happiness buildings cancel the capped happiness penalty. It does not raise deaths or impose a population ceiling.
- Physical storage clamps immediately after a one-time exchange or a change of territorial ownership; temporary monthly overflow is retained only through that month's production, trade and consumption.
- Wonder labor thresholds remain unchanged while the user decides whether to scale them down with aggregate population. Small provinces therefore often cannot reach the first 25-Slave acceleration tier yet.

## Verification status and known limits

The domain audits record focused tests, numerical assumptions and reproducible commands: [economy](implementation-economy.md), [politics](implementation-politics.md), [military](implementation-military.md). The economy includes deterministic 300-month checks across all 54 actual atlas provinces and a 480-month demographic test. Military smoke simulations cover several equal-equipment-budget matchups; they do not prove long-term balance.

Exact current Imperator: Rome unit tables could not be independently verified because the official wiki/diary pages returned an access challenge. Configurable Augustus combat defaults follow the supplied document and verified official patch context; no exact patch parity is claimed. New unit artwork is generated and does not claim to be copied game assets.

The final `just ci` run passed formatting, Clippy with warnings denied, all **166 Rust tests**,
20 runtime-asset hash checks, wasm compilation and packaging path checks. Tests are grouped
into 20 topic files under `tests/unit`, with no test bodies in `src`. The audit contains exactly
368 unique numbered rows and its source links resolve. See [verification.md](verification.md)
and the [README feature audit](README-feature-audit.md) for command results and player entry points.

Native visual inspection previously reached the Overview at 1600x900. Desktop control then
reported an Escape stop, including after the user's earlier attempt to resume. Remaining native
panels, tooltips, animation and compact-window review are **pending**. Headless layout and artwork
checks pass, but are not presented as a replacement for that requested visual review.

Section 292 now has player-scoped, clickable notices for recruitment, destroyed cohorts, foreign arrivals, invasions, NPC defeat, occupation, military access changes, material garrison-support loss and military promotion. Battle results and interrupted movement are also reported. Senate campaign, tie, result, term and victory notices explicitly open the Senate chamber. Long-term multiplayer balance, online transport and persistent saves are not delivered features.

## Numbered coverage

Source references link to the implementing modules; detailed domain audits explain shared behavior. “Revised” and “User clarified” rows explicitly document precedence rather than claiming two contradictory rules coexist. “UI present; review partial” records the distinction between code and runtime visual verification.

| § | Requirement | Status | Source | Implementation / precedence |
| ---: | --- | --- | --- | --- |
| 1 | Scope and design goals | Implemented | [APP], [EMODEL], [DIP], [MIL] | Aggregate state split across three engines. |
| 2 | Core terminology | Implemented | [APP], [EMODEL], [DIP], [MIL] | Aggregate state split across three engines. |
| 3 | Recommended data model | Implemented | [APP], [EMODEL], [DIP], [MIL] | Aggregate state split across three engines. |
| 4 | Population capacity | Implemented | [EPOP], [ECON], [UIE] | Capacity, happiness, demographics and policy rules. |
| 5 | Overpopulation | Tuned formula | [EPOP], [ECON], [UIE] | Capped happiness pressure plus birth-only space suppression above 150% capacity. |
| 6 | Happiness | Implemented | [EPOP], [ECON], [UIE] | Capacity, happiness, demographics and policy rules. |
| 7 | Births | Tuned formula | [EPOP], [ECON], [UIE] | Happiness, food and soft space multipliers; mortality remains separate. |
| 8 | Deaths | Implemented | [EPOP], [ECON], [UIE] | Capacity, happiness, demographics and policy rules. |
| 9 | Food shortages and famine | User clarified | [ECON], [ETEST] | Player famine proportional; NPC civilians provisioned by user clarification. |
| 10 | Food-supply policy | Implemented | [EPOP], [ECON], [UIE] | Capacity, happiness, demographics and policy rules. |
| 11 | Automatic migration | Implemented | [EPOP], [ECON], [UIE] | Capacity, happiness, demographics and policy rules. |
| 12 | Emigration rate | Implemented | [EPOP], [ECON], [UIE] | Capacity, happiness, demographics and policy rules. |
| 13 | Migration destination | Implemented | [EPOP], [ECON], [UIE] | Capacity, happiness, demographics and policy rules. |
| 14 | Migration policy | Implemented | [EPOP], [ECON], [UIE] | Capacity, happiness, demographics and policy rules. |
| 15 | Class changes | Implemented | [EPOP], [ECON], [UIE] | Capacity, happiness, demographics and policy rules. |
| 16 | Player global resources | Implemented | [EMODEL], [ECON], [BUILD], [UIE] | Global resources, labor, storage and income. |
| 17 | Global storage | Implemented | [EMODEL], [ECON], [ETRADE], [APP] | Immediate one-time/ownership overflow loss; monthly overflow waits for consumption. |
| 18 | Resource production | Implemented | [EMODEL], [ECON], [BUILD], [UIE] | Global resources, labor, storage and income. |
| 19 | Resource focus | Implemented | [EMODEL], [ECON], [BUILD], [UIE] | Global resources, labor, storage and income. |
| 20 | Slave-labor policy | Implemented | [EMODEL], [ECON], [BUILD], [UIE] | Global resources, labor, storage and income. |
| 21 | Resource roles | Revised by later sections | [ECON], [MIL] | Military Metal is recruitment-only; no ships or reinforcement. |
| 22 | Coin | Implemented | [EMODEL], [ECON], [BUILD], [UIE] | Global resources, labor, storage and income. |
| 23 | Player taxes | Implemented | [EMODEL], [ECON], [BUILD], [UIE] | Global resources, labor, storage and income. |
| 24 | Influence | Tuned formula | [ECON], [APP] | Nobles 0.25/month, plus buildings/wonders/rank/vassals. |
| 25 | Relation | Implemented | [DIP], [UIP] | Separate relation/control; costs and graph distance. |
| 26 | Passive relation improvement | Implemented | [DIP], [UIP] | Separate relation/control; costs and graph distance. |
| 27 | One-time relation improvement | Implemented | [DIP], [UIP] | Separate relation/control; costs and graph distance. |
| 28 | Trade and relation | Implemented | [DIP], [UIP] | Separate relation/control; costs and graph distance. |
| 29 | Negative relation events | Implemented | [DIP], [UIP] | Separate relation/control; costs and graph distance. |
| 30 | Independent control | Implemented | [DIP], [UIP] | Separate relation/control; costs and graph distance. |
| 31 | Gaining control with coin | Implemented | [DIP], [UIP] | Separate relation/control; costs and graph distance. |
| 32 | Gaining control with influence | Implemented | [DIP], [UIP] | Separate relation/control; costs and graph distance. |
| 33 | Undermine rival | Implemented | [DIP], [UIP] | Separate relation/control; costs and graph distance. |
| 34 | Control entrenchment | Implemented | [DIP], [UIP] | Separate relation/control; costs and graph distance. |
| 35 | Military force and control | Revised by §§274–281 | [DIP], [MIL], [APP] | Actual surviving occupied force, not army count. |
| 36 | Political distance | Implemented | [DIP], [UIP] | Separate relation/control; costs and graph distance. |
| 37 | Vassalization | Revised by §§146–168 | [DIP], [UIP] | Vassalize at 51 unique lead; resulting control = share−50. |
| 38 | Vassal control | Revised by §§146–159 | [DIP] | Vassal control follows revised conversion and range. |
| 39 | Passive vassal-control decay from poor relation | Implemented | [DIP], [UIP] | Revised political lifecycle applies. |
| 40 | Armies stationed in a vassal | Revised by §§245,276–280 | [DIP], [MIL], [APP] | Diminishing effective unit strength replaces army counts. |
| 41 | Direct vassal-control support | Implemented | [DIP], [UIP] | Revised political lifecycle applies. |
| 42 | Vassal-control monthly calculation | Implemented | [DIP], [UIP] | Revised political lifecycle applies. |
| 43 | Tribute | Implemented | [DIP], [UIP] | Revised political lifecycle applies. |
| 44 | Enemy interference with vassals | Implemented | [DIP], [UIP] | Revised political lifecycle applies. |
| 45 | Three ways to maintain a vassal | Implemented | [DIP], [UIP] | Revised political lifecycle applies. |
| 46 | Annexation | Revised by §§151,168 | [DIP], [UIP] | Explicit ownership at 100; voluntary independent/vassal integration. |
| 47 | Owned province political state | Implemented | [DIP], [UIP] | Revised political lifecycle applies. |
| 48 | Trade overview | Implemented | [ETRADE], [UIE] | Validated routes, bilateral trades and bounded NPC markets. |
| 49 | Trade routes | Implemented | [ETRADE], [UIE] | Validated routes, bilateral trades and bounded NPC markets. |
| 50 | Route efficiency | Implemented | [ETRADE], [UIE] | Validated routes, bilateral trades and bounded NPC markets. |
| 51 | Player-to-player trade | Implemented | [ETRADE], [UIE] | Validated routes, bilateral trades and bounded NPC markets. |
| 52 | NPC trade model | Implemented | [ETRADE], [UIE] | Validated routes, bilateral trades and bounded NPC markets. |
| 53 | NPC production potential | Implemented | [ETRADE], [ECON] | NPC monthly flow production and adaptive resource focus. |
| 54 | NPC food need | User clarified | [ETRADE], [ECON] | Food deficits price trade; NPC civilians abstractly provisioned. |
| 55 | NPC metal need | Implemented | [ETRADE], [UIE] | Validated routes, bilateral trades and bounded NPC markets. |
| 56 | NPC stone need | Implemented | [ETRADE], [UIE] | Validated routes, bilateral trades and bounded NPC markets. |
| 57 | NPC surplus and shortage | Implemented | [ETRADE], [UIE] | Validated routes, bilateral trades and bounded NPC markets. |
| 58 | NPC visible demand bands | Implemented | [ETRADE], [UIE] | Validated routes, bilateral trades and bounded NPC markets. |
| 59 | NPC resource valuation | Implemented | [ETRADE], [UIE] | Validated routes, bilateral trades and bounded NPC markets. |
| 60 | NPC relation and trade terms | Implemented | [ETRADE], [UIE] | Validated routes, bilateral trades and bounded NPC markets. |
| 61 | NPC coin treasury | Implemented | [ETRADE], [UIE] | Validated routes, bilateral trades and bounded NPC markets. |
| 62 | NPC trade budget | Implemented | [ETRADE], [UIE] | Validated routes, bilateral trades and bounded NPC markets. |
| 63 | Coin in NPC trade | Implemented | [ETRADE], [UIE] | Validated routes, bilateral trades and bounded NPC markets. |
| 64 | NPC recurring trade routes | Implemented | [ETRADE], [UIE] | Validated routes, bilateral trades and bounded NPC markets. |
| 65 | NPC one-time trade | Implemented | [ETRADE], [UIE] | Validated routes, bilateral trades and bounded NPC markets. |
| 66 | Recurring trade and relation | Implemented | [ETRADE], [UIE] | Validated routes, bilateral trades and bounded NPC markets. |
| 67 | Recurring trade and independent control | Implemented | [ETRADE], [UIE] | Validated routes, bilateral trades and bounded NPC markets. |
| 68 | Competing foreign trade | Implemented | [ETRADE], [UIE] | Validated routes, bilateral trades and bounded NPC markets. |
| 69 | Vassal trade | Implemented | [ETRADE], [UIE] | Validated routes, bilateral trades and bounded NPC markets. |
| 70 | Trade interruption | Implemented | [ETRADE], [UIE] | Validated routes, bilateral trades and bounded NPC markets. |
| 71 | NPC hostility and attacks | User clarified | [APP], [DIP], [MOVE] | NPCs defend their own territory; offensive AI excluded by user. |
| 72 | Buildings relevant to this system | Implemented | [BUILD] | Building effects feed population/economy. |
| 73 | Monthly simulation order | Implemented | [APP], [ECON], [DIP] | Deterministic monthly integration and invariants. |
| 74 | Immediate actions during a turn | Implemented | [APP], [ECON], [DIP] | Deterministic monthly integration and invariants. |
| 75 | Political state transitions | Implemented | [APP], [ECON], [DIP] | Deterministic monthly integration and invariants. |
| 76 | Invariants | Implemented | [APP], [ECON], [DIP] | Deterministic monthly integration and invariants. |
| 77 | UI requirements | UI present; review partial | [UIE], [UIP], [UIM], [UIH] | Parchment panels, contextual actions and tooltips. |
| 78 | Configuration | Implemented | [ECON], [DIP], [BUILD] | Central configurable starting constants; tuned aggregate scale. |
| 79 | Recommended initial balance constants | Implemented | [ECON], [DIP], [BUILD] | Central configurable starting constants; tuned aggregate scale. |
| 80 | Worked population example | Reference | [EAUD], [ETEST] | Worked population formulas at calibrated aggregate map scale. |
| 81 | Worked vassal example | Reference | [PAUD] | Later51 threshold/strength-garrison example supersedes old totals. |
| 82 | Worked NPC recurring-trade example | Reference | [EAUD], [PAUD] | Worked examples are explanatory; revised rules take precedence. |
| 83 | Worked independent-control competition example | Reference | [EAUD], [PAUD] | Worked examples are explanatory; revised rules take precedence. |
| 84 | Core gameplay loops | Implemented | [APP], [UIE], [UIP], [UIM] | Economic, diplomatic, political and military loops. |
| 85 | Explicit non-goals | Intentional exclusion | [EAUD], [PAUD], [MAUD] | Excluded mechanics remain excluded unless later revisions add them. |
| 86 | Final implementation principle | Implemented | [EAUD], [PAUD], [MAUD] | Explicit, inspectable formulas and cross-system state. |
| 87 | Espionage | Implemented | [SPY], [SPYAPP], [UIP] | Persistent networks, real opportunities, evidence and blackmail. |
| 88 | Monthly spy resolution | Implemented | [SPY], [SPYAPP], [UIP] | Persistent networks, real opportunities, evidence and blackmail. |
| 89 | Spy detection | Implemented | [SPY], [SPYAPP], [UIP] | Persistent networks, real opportunities, evidence and blackmail. |
| 90 | Discovered spy against NPC | Implemented | [SPY], [SPYAPP], [UIP] | Persistent networks, real opportunities, evidence and blackmail. |
| 91 | Discovered spy against player | Implemented | [SPY], [SPYAPP], [UIP] | Persistent networks, real opportunities, evidence and blackmail. |
| 92 | Player scandal sources | Implemented with applicable sources | [SPY], [SPYAPP] | Real supported actions/conditions; no synthetic high-tax action. |
| 93 | Scandal opportunity model | Implemented | [SPY], [SPYAPP], [UIP] | Persistent networks, real opportunities, evidence and blackmail. |
| 94 | Active versus previously discovered scandal | Implemented | [SPY], [SPYAPP], [UIP] | Persistent networks, real opportunities, evidence and blackmail. |
| 95 | Scandal discovery chance | Implemented | [SPY], [SPYAPP], [UIP] | Persistent networks, real opportunities, evidence and blackmail. |
| 96 | NPC scandals | Implemented | [SPY], [SPYAPP], [UIP] | Persistent networks, real opportunities, evidence and blackmail. |
| 97 | Scandal inventory | Implemented | [SPY], [SPYAPP], [UIP] | Persistent networks, real opportunities, evidence and blackmail. |
| 98 | Using scandals against NPC provinces | Implemented | [SPY], [SPYAPP], [UIP] | Persistent networks, real opportunities, evidence and blackmail. |
| 99 | Using scandals against players | Implemented | [SPY], [SPYAPP], [UIP] | Persistent networks, real opportunities, evidence and blackmail. |
| 100 | Foreign interference | Implemented | [DIP], [SPY], [UIP] | Targeted attributed interference and resistance. |
| 101 | Smear Campaign — lower rival Relation | Implemented | [DIP], [SPY], [UIP] | Targeted attributed interference and resistance. |
| 102 | Fund Opposition — lower rival Independent Control | Implemented | [DIP], [SPY], [UIP] | Targeted attributed interference and resistance. |
| 103 | Political Agitation — lower Vassal Control | Implemented | [DIP], [SPY], [UIP] | Targeted attributed interference and resistance. |
| 104 | Fund Dissidents — lower Vassal Relation toward Overlord | Implemented | [DIP], [SPY], [UIP] | Targeted attributed interference and resistance. |
| 105 | Direct versus indirect vassal destabilization | Implemented | [DIP], [SPY], [UIP] | Targeted attributed interference and resistance. |
| 106 | Relation resistance to interference | Implemented | [DIP], [SPY], [UIP] | Targeted attributed interference and resistance. |
| 107 | Distance penalties for interference | Implemented | [DIP], [SPY], [UIP] | Targeted attributed interference and resistance. |
| 108 | Interaction with spies | Implemented | [DIP], [SPY], [UIP] | Targeted attributed interference and resistance. |
| 109 | Discovery / attribution of foreign interference | Implemented | [DIP], [SPY], [UIP] | Targeted attributed interference and resistance. |
| 110 | Updated vassal monthly example | Reference | [PAUD] | Later vassal/garrison revisions supersede illustrative totals. |
| 111 | Updated monthly espionage/interference order | Implemented | [SPY], [SPYAPP], [APP] | Monthly detection before discovery, bounded evidence/state. |
| 112 | Additional espionage invariants | Implemented | [SPY], [SPYAPP], [APP] | Monthly detection before discovery, bounded evidence/state. |
| 113 | Additional interference invariants | Implemented | [SPY], [SPYAPP], [APP] | Monthly detection before discovery, bounded evidence/state. |
| 114 | Construction system | Implemented | [BUILD] | One building/wonder slot, separate from recruitment. |
| 115 | Standard buildings | Implemented | [BUILD], [UIE] | Atomic costs, shared slot, unlimited levels and city gates. |
| 116 | Building resources | Implemented | [BUILD], [UIE] | Atomic costs, shared slot, unlimited levels and city gates. |
| 117 | Paying construction costs | Implemented | [BUILD], [UIE] | Atomic costs, shared slot, unlimited levels and city gates. |
| 118 | Unlimited building levels | Implemented | [BUILD], [UIE] | Atomic costs, shared slot, unlimited levels and city gates. |
| 119 | Building construction time | Implemented | [BUILD], [UIE] | Atomic costs, shared slot, unlimited levels and city gates. |
| 120 | Building progress UI | Implemented | [BUILD], [UIE] | Atomic costs, shared slot, unlimited levels and city gates. |
| 121 | Building benefits | Implemented | [BUILD], [UIE] | Atomic costs, shared slot, unlimited levels and city gates. |
| 122 | City provinces | Implemented | [BUILD], [UIE] | Atomic costs, shared slot, unlimited levels and city gates. |
| 123 | City-only buildings | Implemented | [BUILD], [UIE] | Atomic costs, shared slot, unlimited levels and city gates. |
| 124 | Wonder sites use the existing repository data | Implemented with user override | [BUILD], [ECON], [MAP], [UIE] | Ay Khanum removed at the user's request; ten remaining sites preserve coordinates/art. |
| 125 | Wonder eligibility by province | Implemented | [BUILD], [ECON], [MAP], [UIE] | Every remaining site maps to exactly one playable province; regression starts each construction in its atlas province. |
| 126 | Maximum one wonder per province | Implemented | [BUILD], [ECON], [MAP], [UIE] | Canonical site restrictions, slave labor and ownership rewards. |
| 127 | Wonder construction prerequisites | Implemented | [BUILD], [ECON], [MAP], [UIE] | Canonical site restrictions, slave labor and ownership rewards. |
| 128 | Wonder costs | Implemented | [BUILD], [ECON], [MAP], [UIE] | Canonical site restrictions, slave labor and ownership rewards. |
| 129 | Wonder construction time | Implemented | [BUILD], [ECON], [MAP], [UIE] | Canonical site restrictions, slave labor and ownership rewards. |
| 130 | Slaves assigned to wonder construction | Implemented | [BUILD], [ECON], [MAP], [UIE] | Canonical site restrictions, slave labor and ownership rewards. |
| 131 | Wonder construction speed from slaves | Implemented; balance question pending | [BUILD], [ECON], [MAP], [UIE] | Original 25/50/100/200 thresholds retained pending aggregate-scale decision. |
| 132 | Slave availability changes during wonder construction | Implemented | [BUILD], [ECON], [MAP], [UIE] | Canonical site restrictions, slave labor and ownership rewards. |
| 133 | Wonder construction and slave mortality | Implemented | [BUILD], [EPOP] | Assigned slaves remain local pops subject to normal mortality. |
| 134 | Wonder rewards | Implemented | [BUILD], [ECON], [MAP], [UIE] | Canonical site restrictions, slave labor and ownership rewards. |
| 135 | Wonder Influence ownership | Implemented | [BUILD], [ECON], [MAP], [UIE] | Canonical site restrictions, slave labor and ownership rewards. |
| 136 | Wonders in vassal provinces | Implemented | [BUILD], [ECON], [MAP], [UIE] | Canonical site restrictions, slave labor and ownership rewards. |
| 137 | Wonder construction on ownership change | Implemented | [BUILD], [ECON], [MAP], [UIE] | Canonical site restrictions, slave labor and ownership rewards. |
| 138 | Standard construction on ownership change | Implemented | [BUILD], [ECON], [MAP], [UIE] | Canonical site restrictions, slave labor and ownership rewards. |
| 139 | Building and wonder monthly update order | Implemented | [BUILD], [ECON], [MAP], [UIE] | Canonical site restrictions, slave labor and ownership rewards. |
| 140 | Construction UI requirements | UI present; review partial | [UIE] | Costs, effects, progress and assignment controls. |
| 141 | Wonder map integration | Implemented | [MAP], [BUILD] | Existing canonical wonder names, locations and completed art. |
| 142 | Canonical wonder list | Implemented | [MAP], [BUILD] | Existing canonical wonder names, locations and completed art. |
| 143 | Construction invariants | Implemented | [BUILD], [ECON], [ETEST] | Construction constraints and configurable costs/rewards. |
| 144 | Construction configuration | Implemented | [BUILD], [ECON], [ETEST] | Construction constraints and configurable costs/rewards. |
| 145 | Updated explicit non-goals for construction | Intentional exclusion | [BUILD], [EAUD] | No levels for wonders or undeclared secondary effects. |
| 146 | Revised vassalization threshold and control conversion | Implemented | [DIP], [PTEST] | 51 and unique lead; control becomes prior share−50. |
| 147 | Vassalization resets competing independent control | Implemented | [DIP], [UIP], [APP] | 51 threshold, optional integration and revised vassal income. |
| 148 | Vassal control range and purpose | Implemented | [DIP], [UIP], [APP] | 51 threshold, optional integration and revised vassal income. |
| 149 | Passive Influence from vassals | Implemented | [DIP], [APP] | Vassal Influence = 0.05×control per month. |
| 150 | Increasing Vassal Control | Implemented | [DIP], [UIP], [APP] | 51 threshold, optional integration and revised vassal income. |
| 151 | Vassal Control at 100 — integration option | Implemented | [DIP], [UIP] | Explicit integrate action at 100 Vassal Control. |
| 152 | Relation-to-happiness conversion on integration | Implemented | [DIP], [UIP], [APP] | 51 threshold, optional integration and revised vassal income. |
| 153 | Owned provinces cannot be externally controlled | Implemented | [DIP], [UIP], [APP] | 51 threshold, optional integration and revised vassal income. |
| 154 | Enemy political actions against owned provinces | Implemented | [DIP], [UIP], [APP] | 51 threshold, optional integration and revised vassal income. |
| 155 | Fund Unrest in owned provinces | Implemented | [DIP], [UIP], [APP] | 51 threshold, optional integration and revised vassal income. |
| 156 | Espionage against owned provinces | Implemented | [DIP], [UIP], [APP] | 51 threshold, optional integration and revised vassal income. |
| 157 | Relation after ownership | Implemented | [DIP], [UIP], [APP] | 51 threshold, optional integration and revised vassal income. |
| 158 | Updated political lifecycle | Implemented | [DIP], [UIP], [APP] | 51 threshold, optional integration and revised vassal income. |
| 159 | Vassal Influence lifecycle | Implemented | [DIP], [UIP], [APP] | 51 threshold, optional integration and revised vassal income. |
| 160 | Wonder construction animation requirement | Asset validation pending | [MAP], [ART], [BUILD] | Dedicated wonder-specific construction sheets; not one generic overlay. |
| 161 | Wonder construction animation generation | Asset validation pending | [MAP], [ART], [BUILD] | Eleven specific sheets and stage renderer undergoing final integration. |
| 162 | Wonder construction rendering | Asset validation pending | [MAP], [ART], [BUILD] | Eleven specific sheets and stage renderer undergoing final integration. |
| 163 | Wonder animation and progress | Asset validation pending | [MAP], [ART], [BUILD] | Eleven specific sheets and stage renderer undergoing final integration. |
| 164 | Wonder construction assets and repository implementation | Asset validation pending | [MAP], [ART], [BUILD] | Eleven specific sheets and stage renderer undergoing final integration. |
| 165 | Revised political invariants | Implemented | [DIP], [APP], [ECON] | Updated ownership, control and Influence invariants. |
| 166 | Revised monthly Influence generation | Implemented | [DIP], [APP], [ECON] | Updated ownership, control and Influence invariants. |
| 167 | Revised implementation non-goals | Intentional exclusion | [EAUD], [PAUD] | Later added combat/Senate rules override earlier exclusions. |
| 168 | Optional vassalization versus direct ownership | Implemented | [DIP], [UIP], [PTEST] | Voluntary state transitions; simultaneous conserved control. |
| 169 | Simultaneous Independent Control resolution | Implemented | [DIP], [UIP], [PTEST] | Voluntary state transitions; simultaneous conserved control. |
| 170 | Allocate remaining Local Control proportionally | Implemented | [DIP], [UIP], [PTEST] | Voluntary state transitions; simultaneous conserved control. |
| 171 | Resolve competing Control pressure | Implemented | [DIP], [UIP], [PTEST] | Voluntary state transitions; simultaneous conserved control. |
| 172 | Unequal simultaneous Control example | Implemented | [DIP], [UIP], [PTEST] | Voluntary state transitions; simultaneous conserved control. |
| 173 | Multi-player simultaneous Control resolution | Implemented | [DIP], [UIP], [PTEST] | Voluntary state transitions; simultaneous conserved control. |
| 174 | Notification / toast system | Implemented | [NOTIFY], [APP], [UIT] | Recipient-specific history, crossing events and focus actions. |
| 175 | Wonder-start warning toast | Implemented | [NOTIFY], [APP], [UIT] | Recipient-specific history, crossing events and focus actions. |
| 176 | Wonder-completed warning toast | Implemented | [NOTIFY], [APP], [UIT] | Recipient-specific history, crossing events and focus actions. |
| 177 | Control-threshold info toast | Implemented | [NOTIFY], [APP], [UIT] | Recipient-specific history, crossing events and focus actions. |
| 178 | Spy-uncovered warning toast | Implemented | [NOTIFY], [APP], [UIT] | Recipient-specific history, crossing events and focus actions. |
| 179 | Scandal-discovered info toast | Implemented | [NOTIFY], [APP], [UIT] | Recipient-specific history, crossing events and focus actions. |
| 180 | Vassal-Control-drop warning toast | Implemented | [NOTIFY], [APP], [UIT] | Recipient-specific history, crossing events and focus actions. |
| 181 | Owned-province happiness-drop warning toast | Implemented | [NOTIFY], [APP], [UIT] | Recipient-specific history, crossing events and focus actions. |
| 182 | Relation-drop-to-hostile warning toast | Implemented | [NOTIFY], [APP], [UIT] | Recipient-specific history, crossing events and focus actions. |
| 183 | Relation-drop warning for vassals | Implemented | [NOTIFY], [APP], [UIT] | Recipient-specific history, crossing events and focus actions. |
| 184 | Toast deduplication and aggregation | Implemented | [NOTIFY], [APP], [UIT] | Recipient-specific history, crossing events and focus actions. |
| 185 | Toast camera/navigation API | Implemented | [NOTIFY], [APP], [UIT] | Recipient-specific history, crossing events and focus actions. |
| 186 | Notification event sources | Implemented | [NOTIFY], [APP], [UIT] | Recipient-specific history, crossing events and focus actions. |
| 187 | Notification implementation invariants | Implemented | [NOTIFY], [APP], [UIT] | Recipient-specific history, crossing events and focus actions. |
| 188 | Rome political system | Implemented | [SEN], [UIP] | Ranks, global sealed nominations and shared 24-month cycle. |
| 189 | Cursus Honorum | User clarified | [SEN], [UIP] | Quaestor→Aedile→Praetor→Censor→Consul→Augustus. |
| 190 | Rank Influence income | User clarified | [SEN], [ECON] | Spec rank incomes retained; Censor+3; noble income separate. |
| 191 | Aedile | Implemented | [SEN], [UIP] | Ranks, global sealed nominations and shared 24-month cycle. |
| 192 | Praetor | Implemented | [SEN], [UIP] | Ranks, global sealed nominations and shared 24-month cycle. |
| 193 | Consul | User clarified | [SEN], [UIP] | Consul election requires Censor or eligible Proconsul. |
| 194 | Consul term and Proconsul | Implemented | [SEN], [PTEST] | 48-month term; same-month Augustus ballot resolves first. |
| 195 | Augustus | Implemented | [SEN], [UIP] | Ranks, global sealed nominations and shared 24-month cycle. |
| 196 | One global Senate schedule | Implemented | [SEN], [UIP] | Ranks, global sealed nominations and shared 24-month cycle. |
| 197 | Global 24-month cycle | Implemented | [SEN], [UIP] | Ranks, global sealed nominations and shared 24-month cycle. |
| 198 | Nomination Year | Implemented | [SEN], [UIP] | Ranks, global sealed nominations and shared 24-month cycle. |
| 199 | Monthly sealed-bid nomination rounds | Implemented | [SEN], [UIP] | Ranks, global sealed nominations and shared 24-month cycle. |
| 200 | Nomination example | Implemented | [SEN], [UIP] | Ranks, global sealed nominations and shared 24-month cycle. |
| 201 | Final nomination month | Implemented | [SEN], [UIP] | Ranks, global sealed nominations and shared 24-month cycle. |
| 202 | Nomination Influence is spent | Implemented | [SEN], [UIP] | Ranks, global sealed nominations and shared 24-month cycle. |
| 203 | Nomination ties | Implemented | [SEN], [UIP] | Ranks, global sealed nominations and shared 24-month cycle. |
| 204 | Campaign Year | Implemented | [SEN], [UIP] | Ranks, global sealed nominations and shared 24-month cycle. |
| 205 | Senate composition | Implemented | [SEN], [SPY], [APP], [UIP] | Live bloc profiles, spending, bribes and real evidence. |
| 206 | Senate vote format | Implemented | [SEN], [SPY], [APP], [UIP] | Live bloc profiles, spending, bribes and real evidence. |
| 207 | Senate support model | Implemented | [SEN], [SPY], [APP], [UIP] | Live bloc profiles, spending, bribes and real evidence. |
| 208 | Aristocrats | Implemented | [SEN], [SPY], [APP], [UIP] | Live bloc profiles, spending, bribes and real evidence. |
| 209 | Merchants | Implemented | [SEN], [SPY], [APP], [UIP] | Live bloc profiles, spending, bribes and real evidence. |
| 210 | Provincials | Implemented | [SEN], [SPY], [APP], [UIP] | Live bloc profiles, spending, bribes and real evidence. |
| 211 | Populares | Implemented | [SEN], [SPY], [APP], [UIP] | Live bloc profiles, spending, bribes and real evidence. |
| 212 | Military | Implemented | [SEN], [SPY], [APP], [UIP] | Live bloc profiles, spending, bribes and real evidence. |
| 213 | Avoiding permanent Senate dominance | Implemented | [SEN], [SPY], [APP], [UIP] | Live bloc profiles, spending, bribes and real evidence. |
| 214 | Candidate campaigning | Implemented | [SEN], [SPY], [APP], [UIP] | Live bloc profiles, spending, bribes and real evidence. |
| 215 | Other players supporting the candidate | Implemented | [SEN], [SPY], [APP], [UIP] | Live bloc profiles, spending, bribes and real evidence. |
| 216 | Other players opposing the candidate | Implemented | [SEN], [SPY], [APP], [UIP] | Live bloc profiles, spending, bribes and real evidence. |
| 217 | Coin bribery | Implemented | [SEN], [SPY], [APP], [UIP] | Live bloc profiles, spending, bribes and real evidence. |
| 218 | Exposing scandals | Implemented | [SEN], [SPY], [APP], [UIP] | Live bloc profiles, spending, bribes and real evidence. |
| 219 | Campaign state is live | Implemented | [SEN], [SPY], [APP], [UIP] | Live bloc profiles, spending, bribes and real evidence. |
| 220 | Failed vote | Implemented | [SEN], [UIP], [PTEST] | Ballots, two seats, terms, removal and Augustus victory. |
| 221 | Consul seat availability | Implemented | [SEN], [UIP], [PTEST] | Ballots, two seats, terms, removal and Augustus victory. |
| 222 | Removing a Consul early | Implemented | [SEN], [UIP], [PTEST] | Ballots, two seats, terms, removal and Augustus victory. |
| 223 | Proconsul | Implemented | [SEN], [UIP], [PTEST] | Ballots, two seats, terms, removal and Augustus victory. |
| 224 | Augustus competition | Implemented | [SEN], [UIP], [PTEST] | Ballots, two seats, terms, removal and Augustus victory. |
| 225 | Augustus Campaign | Implemented | [SEN], [UIP], [PTEST] | Ballots, two seats, terms, removal and Augustus victory. |
| 226 | Rome Senate tab UI | UI present; review partial | [UIP] | Rome panels, bloc tooltips, chamber and rank ladder. |
| 227 | Rome Campaign UI | UI present; review partial | [UIP] | Rome panels, bloc tooltips, chamber and rank ladder. |
| 228 | Bloc explanation UI | UI present; review partial | [UIP] | Rome panels, bloc tooltips, chamber and rank ladder. |
| 229 | Cursus Honorum UI | UI present; review partial | [UIP] | Rome panels, bloc tooltips, chamber and rank ladder. |
| 230 | Political ladder summary | User clarified | [SEN], [UIP] | Censor retained between Praetor and Consul. |
| 231 | Senate multiplayer requirement | Local scope | [SEN], [UIP] | All local players can bid/support/oppose; online boundary retained. |
| 232 | Senate invariants | Implemented | [SEN], [PTEST] | Configuration, authoritative result and political invariants. |
| 233 | Senate configuration | Implemented | [SEN], [PTEST] | Configuration, authoritative result and political invariants. |
| 234 | Senate explicit non-goals | Intentional exclusion | [PAUD] | No unrequested election systems or extra ranks. |
| 235 | Senate chamber visualization | UI present; review partial | [SEN], [UIP], [PTEST] | 100 semicircle seats; committed votes and saved random undecided rolls. |
| 236 | Senator vote colors | UI present; review partial | [SEN], [UIP], [PTEST] | 100 semicircle seats; committed votes and saved random undecided rolls. |
| 237 | Determinate and undecided Senators | UI present; review partial | [SEN], [UIP], [PTEST] | 100 semicircle seats; committed votes and saved random undecided rolls. |
| 238 | Undecided vote resolution | UI present; review partial | [SEN], [UIP], [PTEST] | 100 semicircle seats; committed votes and saved random undecided rolls. |
| 239 | Committing Senator votes | UI present; review partial | [SEN], [UIP], [PTEST] | 100 semicircle seats; committed votes and saved random undecided rolls. |
| 240 | Vote animation | UI present; review partial | [UIP], [SEN] | Saved votes animate; UI never rerolls outcomes. |
| 241 | Senate projection display | UI present; review partial | [SEN], [UIP], [PTEST] | 100 semicircle seats; committed votes and saved random undecided rolls. |
| 242 | Senate bloc visualization | UI present; review partial | [SEN], [UIP], [PTEST] | 100 semicircle seats; committed votes and saved random undecided rolls. |
| 243 | Senate random-vote invariant | UI present; review partial | [SEN], [UIP], [PTEST] | 100 semicircle seats; committed votes and saved random undecided rolls. |
| 244 | Military system scope | Implemented | [MIL] | Units owned by province/player; eleven types; no Army entity. |
| 245 | No Army entity | Implemented | [MIL] | Units owned by province/player; eleven types; no Army entity. |
| 246 | Unit state | Implemented | [MIL] | Units owned by province/player; eleven types; no Army entity. |
| 247 | Initial unit roster | Implemented | [MIL] | Units owned by province/player; eleven types; no Army entity. |
| 248 | Recruitment is province-based | Implemented | [MIL], [ECON], [UIM] | Drafting, Metal, Food, permanent losses and survivor disbanding. |
| 249 | Recruitment slot | Implemented | [MIL], [ECON], [UIM] | Drafting, Metal, Food, permanent losses and survivor disbanding. |
| 250 | Recruitment flow | Implemented | [MIL], [ECON], [UIM] | Drafting, Metal, Food, permanent losses and survivor disbanding. |
| 251 | Population classes used for recruitment | Implemented | [MIL], [ECON], [UIM] | Drafting, Metal, Food, permanent losses and survivor disbanding. |
| 252 | Example initial manpower requirements | Tuned formula | [MIL], [MCONF] | Manpower examples divided by 10 to fit existing aggregate pops. |
| 253 | Recruitment happiness effect | Implemented | [MIL], [ECON], [UIM] | Drafting, Metal, Food, permanent losses and survivor disbanding. |
| 254 | Recruitment Metal cost | Tuned formula | [MCONF], [EAUD], [MAUD] | Configured Metal cost, not literal Imperator gold currency. |
| 255 | No recurring Metal upkeep | Implemented | [MIL], [ECON], [UIM] | Drafting, Metal, Food, permanent losses and survivor disbanding. |
| 256 | Food upkeep | Implemented | [MIL], [ECON], [UIM] | Drafting, Metal, Food, permanent losses and survivor disbanding. |
| 257 | Military Food shortage | Implemented | [MIL], [ECON], [UIM] | Drafting, Metal, Food, permanent losses and survivor disbanding. |
| 258 | No reinforcement or manpower recovery | Implemented | [MIL], [ECON], [UIM] | Drafting, Metal, Food, permanent losses and survivor disbanding. |
| 259 | Unit destruction | Implemented | [MIL], [ECON], [UIM] | Drafting, Metal, Food, permanent losses and survivor disbanding. |
| 260 | Disbanding | Implemented | [MIL], [ECON], [UIM] | Drafting, Metal, Food, permanent losses and survivor disbanding. |
| 261 | Movement | Implemented | [MOVE], [APP], [UIP], [UIM] | Access, invitations, coexistence and hostile encounters. |
| 262 | Movement permissions | Implemented | [MOVE], [APP], [UIP], [UIM] | Access, invitations, coexistence and hostile encounters. |
| 263 | Multiple forces can coexist | Implemented | [MOVE], [APP], [UIP], [UIM] | Access, invitations, coexistence and hostile encounters. |
| 264 | Peaceful presence does not create Control | Implemented | [MOVE], [APP], [UIP], [UIM] | Access, invitations, coexistence and hostile encounters. |
| 265 | Hostility while coexisting | Implemented | [MOVE], [APP], [UIP], [UIM] | Access, invitations, coexistence and hostile encounters. |
| 266 | NPC starting forces | User clarified | [MIL], [APP] | Defenders stay in home province; no autonomous offensive orders. |
| 267 | NPC rebuilding after defeat | Implemented | [MIL], [APP] | Configured home defenders; no automatic regeneration. |
| 268 | Special-unit recruitment tags | Implemented | [MIL], [MCONF], [UIM] | Regional tags and two artillery types. |
| 269 | Siege/artillery units | Implemented | [MIL], [MCONF], [UIM] | Regional tags and two artillery types. |
| 270 | Training | Implemented | [MIL], [COMBAT] | Training, surviving strength and Morale. |
| 271 | Training from battle | Implemented | [MIL], [COMBAT] | Training, surviving strength and Morale. |
| 272 | Veteran depleted units | Implemented | [MIL], [COMBAT] | Training, surviving strength and Morale. |
| 273 | Morale | Implemented | [MIL], [COMBAT] | Training, surviving strength and Morale. |
| 274 | Military occupation of an independent NPC province | Implemented | [MIL], [DIP], [APP] | Hostile occupation and actual garrison strength. |
| 275 | Occupation state | Implemented | [MIL], [DIP], [APP] | Hostile occupation and actual garrison strength. |
| 276 | Effective stationed strength | Implemented | [MIL], [DIP], [APP] | Hostile occupation and actual garrison strength. |
| 277 | Military Rank modifies stationed strength | Implemented | [MIL], [DIP], [APP] | Hostile occupation and actual garrison strength. |
| 278 | Military occupation Control gain | Implemented | [MIL], [DIP], [APP] | Hostile occupation and actual garrison strength. |
| 279 | Occupation damages Relation | Implemented | [MIL], [DIP], [APP] | Hostile occupation and actual garrison strength. |
| 280 | Vassal military garrison | Tuned formula | [DIP], [MIL], [APP] | 4×power/(power+20), rank-scaled; casualties reduce bonus. |
| 281 | Vassal military example | Implemented | [MIL], [DIP], [APP] | Hostile occupation and actual garrison strength. |
| 282 | Military Rank | Implemented | [MIL], [APP], [SEN] | Independent military career feeds political support. |
| 283 | Military Renown | Implemented | [MIL], [APP], [SEN] | Independent military career feeds political support. |
| 284 | Military Rank thresholds | Implemented | [MIL], [APP], [SEN] | Independent military career feeds political support. |
| 285 | Military Rank effects | Implemented | [MIL], [APP], [SEN] | Independent military career feeds political support. |
| 286 | Military Rank and Senate | Implemented | [MIL], [APP], [SEN] | Independent military career feeds political support. |
| 287 | Separate political and military careers | Implemented | [MIL], [APP], [SEN] | Independent military career feeds political support. |
| 288 | Province military UI | UI present; review partial | [UIM], [UIP] | Grouped forces, costs, draft progress, destination and route UI. |
| 289 | Recruitment UI | UI present; review partial | [UIM], [UIP] | Grouped forces, costs, draft progress, destination and route UI. |
| 290 | Movement UI | UI present; review partial | [UIM], [UIP] | Grouped forces, costs, draft progress, destination and route UI. |
| 291 | Military monthly order | Implemented | [APP], [MIL] | Recruitment/supply/battle precede political occupation pressure. |
| 292 | Military notifications recommended | Implemented | [APP], [NOTIFY] | All listed event categories have private clickable notices; monthly access/garrison thresholds and permanent cohort-loss events are integrated. |
| 293 | Military configuration | Implemented | [MCONF], [MTEST] | Central military constants and ownership/upkeep invariants. |
| 294 | Military invariants | Implemented | [MCONF], [MTEST] | Central military constants and ownership/upkeep invariants. |
| 295 | Military explicit non-goals | Intentional exclusion | [MAUD] | Excluded systems remain excluded. |
| 296 | Combat design basis | Source limitation | [MCONF], [MAUD] | Supplied defaults; exact current Imperator tables not verified. |
| 297 | Province force and tactic | Implemented | [MCONF], [COMBAT], [FORM] | Tactics, fit, casualty character and asymmetric matchups. |
| 298 | Combat tactics | Implemented | [MCONF], [COMBAT], [FORM] | Tactics, fit, casualty character and asymmetric matchups. |
| 299 | Tactic composition effectiveness | User clarified | [MCONF], [COMBAT] | Complete six-tactic fit table; first full Catapult row retained. |
| 300 | Calculating tactic effectiveness | Implemented | [MCONF], [COMBAT], [FORM] | Tactics, fit, casualty character and asymmetric matchups. |
| 301 | Tactic combat modifier | Implemented | [MCONF], [COMBAT], [FORM] | Tactics, fit, casualty character and asymmetric matchups. |
| 302 | Tactic casualty character | Implemented | [MCONF], [COMBAT], [FORM] | Tactics, fit, casualty character and asymmetric matchups. |
| 303 | Core unit combat stats | User clarified | [MCONF], [MAUD] | User-resolved Catapult row; exact current Imperator parity unverified. |
| 304 | Unit matchup matrix | Implemented | [MCONF], [COMBAT], [FORM] | Tactics, fit, casualty character and asymmetric matchups. |
| 305 | Siege-unit field matchups | Implemented | [MCONF], [COMBAT], [FORM] | Tactics, fit, casualty character and asymmetric matchups. |
| 306 | Combat width | Implemented | [FORM], [COMBAT] | Width, deployment, reserves, protected support and maneuver. |
| 307 | Automatic deployment | Tuned fallback | [FORM], [MTEST] | Centered reduced frontage prevents small-force deadlock. |
| 308 | Front-line deployment priority | Revised by §§352–368 | [FORM] | Primary center fallback extended by final plan rules. |
| 309 | Support row | Implemented | [FORM], [COMBAT] | Width, deployment, reserves, protected support and maneuver. |
| 310 | Maneuver and flanking | Implemented | [FORM], [COMBAT] | Width, deployment, reserves, protected support and maneuver. |
| 311 | Reserves | Implemented | [FORM], [COMBAT] | Width, deployment, reserves, protected support and maneuver. |
| 312 | Terrain combat modifiers | Implemented | [COMBAT], [MCONF], [MTEST] | Terrain/forts, seeded simultaneous rounds, retreat and permanent results. |
| 313 | Fortification combat bonus | Implemented | [COMBAT], [MCONF], [MTEST] | Terrain/forts, seeded simultaneous rounds, retreat and permanent results. |
| 314 | Siege suppression of forts | Implemented | [COMBAT], [MCONF], [MTEST] | Terrain/forts, seeded simultaneous rounds, retreat and permanent results. |
| 315 | Training combat effect | Implemented | [COMBAT], [MCONF], [MTEST] | Terrain/forts, seeded simultaneous rounds, retreat and permanent results. |
| 316 | Morale combat effect | Implemented | [COMBAT], [MCONF], [MTEST] | Terrain/forts, seeded simultaneous rounds, retreat and permanent results. |
| 317 | Combat randomness | Implemented | [COMBAT], [MCONF], [MTEST] | Terrain/forts, seeded simultaneous rounds, retreat and permanent results. |
| 318 | Combat damage formula | Implemented | [COMBAT], [MCONF], [MTEST] | Terrain/forts, seeded simultaneous rounds, retreat and permanent results. |
| 319 | Simultaneous combat rounds | Implemented | [COMBAT], [MCONF], [MTEST] | Terrain/forts, seeded simultaneous rounds, retreat and permanent results. |
| 320 | Combat rounds per month | Implemented | [COMBAT], [MCONF] | Four rounds/month; six-month maximum; survivors retreat. |
| 321 | Routing | Implemented | [COMBAT], [MCONF], [MTEST] | Terrain/forts, seeded simultaneous rounds, retreat and permanent results. |
| 322 | Voluntary retreat | Implemented | [COMBAT], [MCONF], [MTEST] | Terrain/forts, seeded simultaneous rounds, retreat and permanent results. |
| 323 | Retreat destination | Implemented | [COMBAT], [MCONF], [MTEST] | Terrain/forts, seeded simultaneous rounds, retreat and permanent results. |
| 324 | Battle result and Training | Implemented | [COMBAT], [MCONF], [MTEST] | Terrain/forts, seeded simultaneous rounds, retreat and permanent results. |
| 325 | Battle result and Morale | Implemented | [COMBAT], [MCONF], [MTEST] | Terrain/forts, seeded simultaneous rounds, retreat and permanent results. |
| 326 | Military Renown from battles | Implemented | [COMBAT], [MCONF], [MTEST] | Terrain/forts, seeded simultaneous rounds, retreat and permanent results. |
| 327 | Battle victory and occupation | Implemented | [APP], [COMBAT] | Occupation, user-resolved conquest and multi-owner cohorts. |
| 328 | Combat in owned enemy provinces | User clarified | [APP], [COMBAT] | Immediate ownership transfer after enemy-owned battle victory. |
| 329 | Battle with multiple friendly owners | Implemented | [COMBAT], [APP] | Friendly coalition retains per-owner plans/ranks; third hostility waits. |
| 330 | Movement orders are transient groups, not Armies | Implemented | [MOVE], [APP], [UIM] | Transient selected cohorts, roads/terrain, route progress/access. |
| 331 | Movement speed uses slowest unit | Implemented | [MOVE], [APP], [UIM] | Transient selected cohorts, roads/terrain, route progress/access. |
| 332 | Province crossing distance | Implemented | [MOVE], [APP], [UIM] | Transient selected cohorts, roads/terrain, route progress/access. |
| 333 | Terrain movement modifier | Implemented | [MOVE], [APP], [UIM] | Transient selected cohorts, roads/terrain, route progress/access. |
| 334 | Roads and movement | Implemented | [MOVE], [APP], [UIM] | Transient selected cohorts, roads/terrain, route progress/access. |
| 335 | Edge travel cost | Implemented | [MOVE], [APP], [UIM] | Transient selected cohorts, roads/terrain, route progress/access. |
| 336 | Monthly movement progress | Implemented | [MOVE], [APP], [UIM] | Transient selected cohorts, roads/terrain, route progress/access. |
| 337 | Multi-province movement | Implemented | [MOVE], [APP], [UIM] | Transient selected cohorts, roads/terrain, route progress/access. |
| 338 | Entering a hostile province | Implemented | [MOVE], [APP], [UIM] | Transient selected cohorts, roads/terrain, route progress/access. |
| 339 | Map unit sprite requirement | UI present; review partial | [VIS], [MAP], [ART], [UIM] | Eleven generated unit sheets, LOD, owner colors and animation. |
| 340 | Required unit sprite animations | UI present; review partial | [VIS], [MAP], [ART], [UIM] | Eleven generated unit sheets, LOD, owner colors and animation. |
| 341 | Unit sprite generation and asset pipeline | UI present; review partial | [VIS], [MAP], [ART], [UIM] | Eleven generated unit sheets, LOD, owner colors and animation. |
| 342 | Map level-of-detail for forces | UI present; review partial | [VIS], [MAP], [ART], [UIM] | Eleven generated unit sheets, LOD, owner colors and animation. |
| 343 | Representative unit sprites | UI present; review partial | [VIS], [MAP], [ART], [UIM] | Eleven generated unit sheets, LOD, owner colors and animation. |
| 344 | Multiple owners' sprites in one province | UI present; review partial | [VIS], [MAP], [ART], [UIM] | Eleven generated unit sheets, LOD, owner colors and animation. |
| 345 | Movement map animation | UI present; review partial | [VIS], [MAP], [ART], [UIM] | Eleven generated unit sheets, LOD, owner colors and animation. |
| 346 | Battle map animation | UI present; review partial | [VIS], [MAP], [ART], [UIM] | Eleven generated unit sheets, LOD, owner colors and animation. |
| 347 | Battle UI | UI present; review partial | [UIM], [COMBAT] | Visible battle facts and qualitative estimate with unknown enemy tactic. |
| 348 | Pre-battle estimate | UI present; review partial | [UIM], [COMBAT] | Visible battle facts and qualitative estimate with unknown enemy tactic. |
| 349 | Combat configuration | Implemented | [MCONF], [MTEST] | Central combat constants and tested invariants. |
| 350 | Combat invariants | Implemented | [MCONF], [MTEST] | Central combat constants and tested invariants. |
| 351 | Combat explicit non-goals | Intentional exclusion | [MAUD] | No naval battles, prisoners, healing or extra siege subsystem. |
| 352 | Pre-combat formation plan | Implemented | [FORM], [COMBAT], [MOVE], [MIL] | Saved primary/secondary/flank plans; deterministic deployment and locks. |
| 353 | Primary line unit type | Implemented | [FORM], [COMBAT], [MOVE], [MIL] | Saved primary/secondary/flank plans; deterministic deployment and locks. |
| 354 | Secondary line unit type | Implemented | [FORM], [COMBAT], [MOVE], [MIL] | Saved primary/secondary/flank plans; deterministic deployment and locks. |
| 355 | Flanking unit type | Implemented | [FORM], [COMBAT], [MOVE], [MIL] | Saved primary/secondary/flank plans; deterministic deployment and locks. |
| 356 | Flank size | Implemented | [FORM], [COMBAT], [MOVE], [MIL] | Saved primary/secondary/flank plans; deterministic deployment and locks. |
| 357 | Flank size and narrow terrain | Implemented | [FORM], [COMBAT], [MOVE], [MIL] | Saved primary/secondary/flank plans; deterministic deployment and locks. |
| 358 | Deterministic formation deployment | Implemented | [FORM], [MTEST] | Health/training/morale/id ordering; terrain-clamped slots. |
| 359 | Reserve replacement priority | Implemented | [FORM], [COMBAT], [MOVE], [MIL] | Saved primary/secondary/flank plans; deterministic deployment and locks. |
| 360 | Flank position versus Maneuver | Implemented | [FORM], [COMBAT], [MOVE], [MIL] | Saved primary/secondary/flank plans; deterministic deployment and locks. |
| 361 | Formation and tactic interaction | Implemented | [FORM], [COMBAT], [MOVE], [MIL] | Saved primary/secondary/flank plans; deterministic deployment and locks. |
| 362 | Editing and locking Battle Plans | Implemented | [FORM], [COMBAT], [MOVE], [MIL] | Saved primary/secondary/flank plans; deterministic deployment and locks. |
| 363 | Saved Province Force defaults | Implemented | [FORM], [COMBAT], [MOVE], [MIL] | Saved primary/secondary/flank plans; deterministic deployment and locks. |
| 364 | Pre-combat Battle Plan UI | UI present; review partial | [UIM], [FORM], [MOVE] | Composition, terrain, fallback preview, tactics, editable waypoints. |
| 365 | Battle Plan preview information | UI present; review partial | [UIM], [FORM], [MOVE] | Composition, terrain, fallback preview, tactics, editable waypoints. |
| 366 | Unknown enemy tactic | UI present; review partial | [UIM], [FORM], [MOVE] | Composition, terrain, fallback preview, tactics, editable waypoints. |
| 367 | Pre-combat player choices | UI present; review partial | [UIM], [MOVE] | Unit selection, waypoint routes, timing, tactic and formation controls. |
| 368 | Formation invariants | Implemented | [FORM], [MTEST] | Deterministic plan, reserve and composition invariants. |

[APP]: ../src/game/campaign.rs
[EMODEL]: ../src/game/economy/model.rs
[EPOP]: ../src/game/economy/population.rs
[ECON]: ../src/game/economy/simulation.rs
[ETRADE]: ../src/game/economy/trade.rs
[BUILD]: ../src/game/economy/buildings.rs
[DIP]: ../src/game/politics/diplomacy.rs
[SPY]: ../src/game/politics/espionage.rs
[SPYAPP]: ../src/game/campaign_espionage.rs
[SEN]: ../src/game/politics/senate.rs
[MIL]: ../src/game/military/world.rs
[MCONF]: ../src/game/military/config.rs
[FORM]: ../src/game/military/formation.rs
[COMBAT]: ../src/game/military/combat.rs
[MOVE]: ../src/game/military/movement.rs
[NOTIFY]: ../src/game/campaign_notifications.rs
[UIE]: ../src/ui/campaign_economy.rs
[UIP]: ../src/ui/campaign_politics.rs
[UIM]: ../src/ui/campaign_military.rs
[UIH]: ../src/ui/campaign_panel.rs
[UIT]: ../src/ui/toasts.rs
[MAP]: ../src/map/map.rs
[VIS]: ../src/map/military_visuals.rs
[ART]: ../docs/generated-art.md
[ETEST]: ../tests/unit/economy.rs
[PTEST]: ../src/game/politics/mod.rs
[MTEST]: ../tests/unit/military.rs
[EAUD]: ../docs/implementation-economy.md
[PAUD]: ../docs/implementation-politics.md
[MAUD]: ../docs/implementation-military.md
