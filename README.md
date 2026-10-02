<div align="center">

# Augustus
### Roman provincial strategy for desktop and browser

<br><br>
[![Play](https://gist.githubusercontent.com/cxmeel/0dbc95191f239b631c3874f4ccf114e2/raw/play.svg)](https://tvdboom.itch.io/augustus)
<br><br>
</div>


## Gameplay

In local practice, **Ctrl+Shift+Up** boosts the active player's resources and population, grants
the Influence and Senate support needed for the next political rank, and adds **three cohorts of
every unit type** to each directly owned province. Use the Senate promotion button to claim the
office. The shortcut clears that player's monthly promotion limit and Consul return cooldown;
Consul appointments still require an open seat.

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
Resource Focus. Slaves have higher baseline productivity. Food output scales with productive workers,
just like Metal and Stone. Slave revolts remove the entire local slave population and raise a hostile
army proportional to the population removed.
Revolts raise a red province alert and visible light infantry marked **Revolt**, including at wide zoom.
Rebels immediately engage a present garrison; an unguarded province keeps the hostile army on the map.
Goods enter **global player stockpiles**; provincial storage buildings expand capacity. Overflow is
discarded after monthly settlement, immediate transfers, and ownership changes. Sestertius and Influence
have no storage cap.
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
In two-player games, the five promotions require **200/360/560/800/1600 Influence** and
**10/20/30/40/60 supporters**. Support requirements scale down with player count; Augustus always
requires at least 51. There is no election calendar or nomination auction.

The Senate has 100 persistent senators, initially neutral gray, split into five labeled factions:
Aristocrats, Merchants, Provincials, Populares, and Military. Supporters use their player's color.
All players start with zero faction confidence. Ordinary gameplay accumulates confidence monthly;
each seat has ten shared internal points. Gains fill neutral seats first, then compete with rivals.
A fully influenced seat approves of its player. For example, a fulfilled recurring trade route adds
half a Merchant point per month, while wars hurt trade confidence. The UI shows approval counts,
six political career cards, and five faction badges with distinct emblems. Hovering a badge shows
positive/negative effect lists above the map banner; the chamber itself has no faction tooltips.
The color legend below the chamber lists Neutral and every player with approval counts in parentheses.
Exact faction confidence stays hidden.

Basic starting conditions are neutral: content Nobles do not earn Aristocrat confidence just by
existing, net monthly Coin income up to 30 does not earn Merchant confidence, normally supplied
rations do not earn Populares confidence, and the first province does not earn Imperialist confidence.
Aristocrats reward happy Nobles (with a larger happy population increasing the benefit), higher office,
Forums and wonders. Merchants reward net income above 30 after monthly costs, Markets and fulfilled
recurring routes with real deliveries. Provincials reward vassals, favorable relations and delivered
rural trade. Populares reward happier Citizens/Plebeians and generous rations in proportion to the
food actually supplied. Imperialists reward armies, earned military ranks, victories and territorial
Control beyond one province-equivalent. Positive conditions accumulate support; negative conditions
remove it, while paid senator actions can earn support separately.

Click a senator for petitions, gifts, patronage, bribery, threats, murder, public banquets,
discrediting a rival patron, or lobbying. **Lobby senator** costs 20 Influence upfront
and 4 Influence per month, listed as **Senator lobbying** in the Influence hover. It grants gradual favor
only in paid months and continues to maintain support until cancelled. **Bribe** costs 80 sestertii
upfront and 8 per month, builds confidence faster than lobbying, and risks exposure with every payment.
Recurring payments and upfront costs appear separately in the currency outflow panels.
Petitions build goodwill over three months; gifts and public banquets over four months, affecting
only the selected senator. Patronage and threats build confidence for six months. Earned personal
confidence gradually fades, including during unpaid months, without erasing support earned through
ordinary faction preferences. Risky actions have seeded detection chances and create discoverable evidence.
Exposure causes a larger immediate loss; murder replaces the senator with a neutral newcomer and
clears every player's confidence and active effects in that seat. Each player can maintain one ongoing
action per senator alongside any temporary actions. Different players can compete for the same senator
without seeing each other's actions. Your own ongoing action has a cross in its card's top-right corner
to cancel it, and only your own ongoing actions mark seats in the chamber. Personal actions use
illustrated cards with separate titles, icons, and currency costs; exact confidence stays hidden.

Scandals have three severity levels and affect only one or two relevant factions. Exposure consumes
real evidence and removes confidence immediately, rather than applying a recurring penalty.
Harsh slave labor is severity I; famine and senator murder are severity III. Public scrutiny lasts
one year, but lost confidence must be earned again.

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

Click your army on the map, then right-click a province to open orders beside the cursor. Use
**All / Half / None**, per-type counts, or individual cohort checkboxes to choose the detachment;
unselected troops stay behind. The menu previews travel time for **Move**, **Station / defend**
(where permitted), and **Attack**. Attack declares hostility and engages defenders on arrival.
Speed depends on the slowest selected unit, province size, terrain, and roads; access is checked
at each crossing. Defeating independent defenders establishes occupation, allowing later Control
gains. Enemy-owned provinces also require political Control before legal ownership changes.

**Pressure** enters an independent province at peace without fighting or removing its defenders.
It builds at most 2 Control per month, reduced by local resistance, up to a 40% ceiling, and costs
3 Relation each month while stationed. It grants no occupation or automatic conquest. Removing
the troops ends coercive access and gains. Rome and player-owned territory require ordinary
stationing permission or an invasion.

### Battle plans and combat

Save a **Battle Plan** with primary, secondary, and flank unit types, flank slots, and a tactic.
Plans lock when battle begins; deployment uses available cohorts, terrain width, and reserves.
Unit matchups, tactic counters and composition, flanking, support protection, terrain, forts,
Training, and Morale determine simultaneous combat rounds. Reserves replace routed troops.
Pre-battle assessments show known hostile forces but keep enemy tactics hidden.

Click a **BATTLE** marker or fighting troops to inspect the centered terrain-backed battle panel.
Participants see both formations, locked tactics, morale, casualties, and a six-sided die per side
for every simultaneous round. Four rounds resolve each month, spaced 0.75 seconds apart at normal
speed; pausing and speed controls also apply to combat. New arrivals stay visible before the first
round. Ordinary battles have a three-month
deadline (about nine seconds at normal speed). Surrounded forces and Rome's defenders continue
fighting until destroyed. Depleted and routed cohorts leave their slots, and reserves fill gaps.
The panel retains the outcome and round history after resolution. Battle ambience and unit-family
effects play while an active battle panel is open, respecting shared volume and mute controls.

Victories add career victories, Renown and experience, improve Military faction confidence
immediately, and contribute to recent-victory support. Defeats weaken that confidence. Direct
ownership, vassal Control and independent Control shares contribute to the faction's provincial
control factor. Military rank promotions still require army-size and victory milestones and an
Influence payment; winning a battle advances those requirements.
