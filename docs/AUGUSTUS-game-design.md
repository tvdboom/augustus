# Augustus game rules

This document specifies vassal policies, provincial population movements, hostile-neighbour aggression,
starting NPC armies, battle-line replacements, and recurring NPC trade. Other campaign mechanics are described in the
[gameplay README](../README.md#gameplay).

## Vassal policies

The nationwide Policies tab includes a Politics section. Each player's selections
apply to all their current and future vassals, including after another player takes
over as overlord. Both policies default to Normal and can be selected before acquiring
a vassal. Changing a policy applies its income rate immediately and does not grant an
instant Relation reward; sentiment changes once per completed month.

| Policy | Low | Normal | High |
| --- | --- | --- | --- |
| Vassal Tribute | 10% tribute; +0.5 Relation/month | 20% tribute; no Relation change | 30% tribute; −1 Relation/month |
| Vassal Autonomy | +25% vassal Influence; −0.5 Relation/month | Baseline Influence; no Relation change | −25% vassal Influence; +0.5 Relation/month |

Tribute uses the vassal's monthly coin income, capped by its available treasury, and
is debited from that treasury. Baseline vassal Influence is `Control × 0.05` per month
at the default balance values. Autonomy multiplies that income only; it does not change
tribute, directly grant Control, or affect Influence from other sources.

The two Relation changes add together and Relation stays within 0–100. The existing
stability rules use the resulting Relation: below 50, Control erodes, which also lowers
Influence income. At zero Control the province becomes independent and stops paying
both incomes; at 100 Control it can be integrated for direct ownership.

Vassals also contribute to Provincial Senate support and serve as sources of diplomatic
reach. They retain local government and their own armies; becoming a vassal does not give
the overlord command of those troops or ordinary production and wonder income. Existing
trade, military access and hostile-neighbour rules still apply. Older saves receive
Normal defaults for the newly exposed tribute selection and autonomy policy.

## Provincial population movements

Population capacity describes comfortable settlement, rather than a hard maximum.
Independent and vassal NPC provinces feel increasing emigration pressure above 80%
of that capacity, giving free residents time to leave before overcrowding sets in.
Departures require a reachable adjacent province with better conditions. Space,
class happiness, cities, and local opportunities determine that improvement; policy,
Markets, and diplomatic relations determine how migrants divide among destinations.
Nearly equal conditions produce only a trickle. Hostile borders block migration,
residents stay when no better destination exists, and every migrant is conserved.
Slaves never migrate freely.

NPC birth rates and migration opportunities follow gradual local demographic cycles.
Each province has its own phase and a period of roughly 12–18 years. During a weak
period births can fall below ordinary deaths, including among slaves; better periods
allow recovery. These cycles combine with existing happiness, overcrowding, food,
and migration rules. They do not change the displayed capacity, create famine in
abstractly provisioned NPC provinces, or truncate population. Directly owned player
provinces retain their existing birth rates and crowding threshold.

Cycles derive from the province's stable name and campaign month. Forecasts, saved
games, and online replay therefore use the same conditions without extra random draws.
Older saves receive the default demographic tuning.

## Starting NPC armies

New campaigns give every NPC province three times its previous starting army:
18–36 full-strength, 1,000-soldier cohorts according to its historical roster.
Rome starts with 450 cohorts, retaining its four guard types. These troops do not
consume civilian population when seeded, and defeated defenders do not regenerate.
Existing saves retain their recorded armies.

## Battle-line replacements

Destroyed or routed cohorts leave their deployment slots immediately after simultaneous
round damage resolves. Reserves fill empty front slots using the locked battle plan's
replacement preferences. When no reserve replacement is available, a surviving support
cohort directly behind an empty front slot advances into that slot. Other surviving
deployed cohorts keep their positions. A support cohort behind a living front cohort
remains protected; support troops advancing into the front take the usual exposure damage.

Replacement reserves begin fighting in the following round. Advancing them does not add
another attack, incoming damage, or participation credit to the round just resolved.

## Hostile neighbours

Relation below 40 is hostile; Relation below 20 is very hostile. Exactly 20 does not
qualify for a border attack.

Independent and vassal provincial governments may send their existing local armies
against a player with whom they have very hostile relations. Player-controlled
provinces keep player-issued orders. Rome and slave rebels do not launch border attacks.

At each completed month, each eligible province–player pair has a 10% chance of launching
an attack. Eligibility requires living troops with positive morale at home and at least
one adjacent province directly owned by that player. Defeated players are excluded.
Provinces under foreign occupation or fighting a battle cannot launch an attack. Targets
already occupied or in battle are excluded. Among eligible targets, the army chooses
the weakest player garrison, breaking ties by province ID.

Local defenders prioritize hostile visitors in their own province. The existing rule
remains: local defenders attack visiting player troops immediately when Relation falls
below 50, including on arrival or after a diplomatic action. Defending against that
attack does not count as an unjustified player invasion.

A province sends at most one army per month and cannot send another while its army is
marching, fighting elsewhere, or stationed outside its home province. It uses existing
troops; an attack never creates extra units. Improving Relation to 20 or above prevents
new border attacks, but does not recall an army already marching or end declared hostility.

The monthly chance is independent, without a forced deadline. With an eligible army
and target throughout, one hostile neighbour has probability `1 - 0.9^months` of
starting an attack: approximately 92% after 24 months and 98% after 36 months.

## Resolving an attack

The player receives a warning naming the attacking province and destination when the
army departs. Troops travel along the shared border using the normal terrain, road,
area, and unit-speed rules. They remain visible in movement records before arrival.
Hostility is recorded against the attacking provincial government and blocks affected
trade as usual. NPC initiation does not accuse the defending player of treaty-breaking
or an unjustified attack.

On arrival, the NPC army attacks hostile troops. The territorial defender takes priority;
mutually hostile players are never forced into an alliance. Existing battles accept
compatible reinforcements. City Walls benefit defenders, and ordinary combat, supply,
morale, retreat, and result notifications apply. A defeated attacking army may retreat
to its adjacent home province. Invading NPC troops draw food from their home government's
reserves, including while fighting or occupying enemy land.

Victory or entry into unguarded enemy land establishes occupation and blocks the displaced
owner's province actions. The occupying army remains stationed there. Beginning at the
following monthly boundary, unopposed NPC occupation reduces the owner's Control using
the same effective-strength curve as player occupation: `5 × power / (power + 20)` at
the default balance values, including the occupying army's military-rank multiplier.
Pressure uses surviving troops and their morale and training, has diminishing returns,
and stops while the province is in battle or the owner's troops are present. Troops
still marching toward the province do not stop the loss.

Lost Control returns to local government. At zero owner Control, any directly owned
province becomes independent NPC territory, preserving rival player shares and existing
buildings. It is not annexed by the attacking province. Independence ends the NPC
occupation marker; the surviving invading troops remain stationed there and retain
their original army identity. Independence does not create extra units. A player is
defeated only upon losing their last directly owned province, even if it was their starting
province. Defeating the occupier before independence restores province actions and
stops further occupation loss; it does not immediately restore lost Control.

The invasion chance and authoritative random stream survive saves and online snapshot
replay; older saves use the default threshold and monthly chance.

## Recurring NPC trade

NPC governments assess supply, demand, scarcity prices, budget, and Relation when accepting
a new agreement. New agreements require full supply at acceptance.

The partner market separates actual shortages from reserve purchases. Wants is internal need
minus local production and delivered imports, floored at zero; Reserves is the remaining
full-value import allowance beyond that shortage. Single-time exchanges use the same
threefold period allowance as supply. Exporting a surplus does not itself create a shortage.
Coins per unit shows local resource prices. Each side of the exchange displays its total
listed coin value, including any coin or enabled Influence payment, before transport losses
and excess-import discounts. Player-to-player totals use base prices. These totals explain
the amounts; acceptance still applies Relation, distance, transport, and import discounts.

Once a monthly route is accepted, the NPC commits to the agreed gross amounts indefinitely.
Changes in its production, internal needs, demand, prices, trade modifiers, export capacity,
cash budget, or treasury do not reduce delivery, suspend the route, or cancel it. Normal
transport losses still apply, and NPC treasury balances stay nonnegative.

Only the participating player may voluntarily close or abort the route, using the existing
notice and cancellation rules. Relation below the trade threshold (20 by default), war,
occupation, or loss of a permitted route still blocks delivery. Three consecutive blocked
months end the agreement. A decline in Relation that stays at or above the trade threshold
does not renegotiate its terms, including for the player's own vassals.

The player must supply the full agreed payment before either side transfers anything. Missing
payment pauses delivery until the player can afford it; it never cancels the NPC's commitment.
Player-to-player agreements and one-time NPC exchanges retain their existing rules. Accepted
and suspended monthly NPC routes loaded from saves use the same commitment rule.
