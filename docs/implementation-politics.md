# Politics, Senate, espionage and notifications implementation audit

This audit maps the political portions of the supplied `augustus_gameplay_systems_implementation_spec_v10_formations.md` to concrete code. Later numbered revisions supersede earlier examples. Domain logic lives in `src/game/politics/`; renderer adapters and panels remain separate. The user expressly retained **Quaestor → Aedile → Praetor → Censor → Consul → Augustus**. Proconsul is an expired/removed Consul status and shares the existing Consul artwork slot.

## Rules and precedence

| Specification | Implemented rule | Location |
| --- | --- | --- |
| 25–29 | Separate, clamped relation; finite paid gifts; diminishing friendship returns; separately paid monthly coin/Influence relation support; no debt | `diplomacy.rs` |
| 30–36 | Independent Local + player shares total 100; coin/Influence costs account for relation, distance and entrenchment; shortest usable graph route includes explicit sea edges | `diplomacy.rs` |
| 37–47, superseded by 146–159, 168 | Vassalization requires 51 and unique lead; initial Vassal Control = old Independent Control − 50; rival independent shares discarded; relation retained; explicit ownership at independent 100 or vassal 100 | `ProvincePolitics::vassalize`, `take_ownership` |
| 38–45, 149–150, 280 | Vassal relation below 50 causes monthly control decay; real stationed military strength supplies diminishing control; separately paid recurring government support; coin tribute trades income against relation; control 0 returns Local to 100 | `ProvincePolitics::advance_month` |
| 149, 159, 166 | Vassal income is 0.05 Influence per control; reported to the economy once and ceases on ownership | `vassal_income` and campaign adapter |
| 152–157 | Integration returns the single sentiment-to-happiness shift; owned provinces reject foreign control and instead permit Citizen/Plebeian agitation or Plebeian unrest | `take_ownership`, `interfere` |
| 169–173 | All positive monthly control requests are pooled; Local is distributed proportionally; targeted losses return to Local; rival transfers use common iteration snapshots and conserve 100 | `resolve_control` |
| 66–69 | Successful trade receives relation and simultaneous political rewards; independent trade control has per-month cap 1 and total share ceiling 40; vassals receive relation only | `apply_trade`, economy trade adapter |
| 87–91 | One persistent network per owner/foreign province; deployment costs 10 Influence; monthly upkeep 5 Coin; detection resolves before discovery; detected networks removed; NPC detection reduces relation by 10; player victims receive real evidence | `espionage.rs` |
| 92–97 | Human scandals derive from active real conditions or completed actions; unique activation identities prevent duplicate discovery; ending a condition stops discoveries but preserves acquired evidence; 24-month expiry; bounded hidden NPC pool | `EspionageState`, campaign espionage adapter |
| 98–99 | NPC evidence can add 5/7.5/10 simultaneous control with −5 relation or grant six months at 0.75 required trade value; player evidence affects Senate politics rather than resources or ownership | `blackmail_control`, `blackmail_trade`, `bloc_penalties` |
| 100–113 | Attributed rival-specific smear, opposition, undermining, vassal agitation and dissident funding; appropriate relation/control resistance; only Smear Campaign has the specification's once-per-actor/rival/province/month limit | `interfere` |
| 188–204 | Shared 12-month nomination + 12-month campaign; sealed additional monthly escrow, simultaneous reveal, all bids spent; tied leaders alone enter repeated sealed sudden death; no click/host/player-ID tie-break | `senate.rs` |
| 189–195 + user's Censor amendment | Aedile costs 100 Influence without a ballot; Praetor, Censor, Consul and Augustus require Senate voting; Censor nomination default 150 and +3 monthly Influence; Consul requires Censor or Proconsul | `PoliticalRank`, `SenateConfig`, `eligibility` |
| 193–194, 221–225 | Maximum two Consuls; 48-month terms; Proconsul requires reelection before Augustus; same-month Augustus vote resolves before term expiration; evidence-backed No Confidence shares the one ballot | `apply_result`, `advance_month` |
| 205–219 | Blocs have 30/20/20/20/10 seats; live, soft-capped structural support; diminishing campaigning/endorsement/opposition; coin bribery creates actual discoverable misconduct; scandals have distinct bloc effects | `projection`, `structural_reasons`, Senate UI |
| 220–234 | Failure begins a new Nomination Year; every local player may contest the auction and campaign; 51 YES required; Augustus victory is only awarded by a successful ballot | `SenateState` |
| 235–243 | Exactly 100 stable semicircle circles; bloc hover explanations; committed YES/NO and gray undecided seats; only undecided seats are rolled; authoritative saved outcomes animate without rerolling | `src/ui/campaign_politics.rs` |
| 274–281 | Peaceful presence never grants occupation; only established hostile occupation supplies independent control and −2 relation/month; garrison strength depends on surviving troops and rank | Military adapter + `diplomacy.rs` |
| User clarification: owned conquest | Enemy-owned territory transfers immediately following authoritative military victory; independent NPC victories still establish occupation before control accumulation | `capture_owned`, military campaign adapter |

## Notifications and UI

`src/game/campaign_notifications.rs` defines player-scoped Info/Warning events with title, body, kind, province/wonder/scandal identity, creation month, and reusable navigation action. A bounded recent history survives transient toast expiration. Monthly snapshots generate independent-control upward crossings at **50** and **100**, hostile relation downward crossings at **40** and **20**, vassal relation below **50**, and one aggregated warning per weakening vassal per month. Foreign happiness warnings are sourced from attributed enemy actions rather than conflating domestic policy or famine. Their default aggregate threshold is five class-happiness points.

The construction panel emits wonder-start notices immediately after successful construction actions, and wonder completion is compared across monthly snapshots. Both exclude the constructing owner and carry the canonical wonder ID for camera focus. Spy detection, withdrawal and discovered evidence are scoped to the affected player, preserving local hotseat privacy. Deduplication keys include recipient, domain event, target, and month.

The Senate panel uses existing parchment/ink/gold styling and actual assigned player colors. It exposes rank, Consul terms/seats, public nomination commitments, the viewing player's private bid, eligibility explanations, bloc support/oppose/bribe controls, distinct scandal exposure, and the one-hundred-seat chamber. `npc_evidence` supplies both NPC blackmail actions in the contextual diplomacy panel. Diplomacy previews call the same domain actions on isolated clones to display actual relation-, distance-, and entrenchment-adjusted prices without charging funds or queuing control pressure. Disabled controls explain route, rank, ownership, and affordability restrictions. Owned-province diplomacy exposes military invitations, revocation, war and peace; the active-battle restriction prevents peace from silently terminating combat.

Every campaign panel exposes player-scoped recent history. History links focus provinces or canonical wonder coordinates and open the relevant overview, building or evidence section. Evidence toasts open NPC diplomacy or the Senate as appropriate; the NPC context offers control and trade blackmail, while player evidence remains in Senate politics.

## Balance choices supplied where the document left formulas open

- Censor: minimum nomination 150 Influence; income +3/month, between Praetor and Consul.
- Political gifts/control: base 20 Coin or 2 Influence per point; coin expansion costs increase with existing control. Recurring support costs 10 Coin or 2 Influence per month per program.
- Garrison control: `4 × power / (power + 20)`. The denominator uses the map's aggregate population and calibrated military manpower scale, not individual historical headcounts.
- Campaign probability starts at 45%, has configurable structural-factor multipliers, bounded 2–98% support, 25 percentage-point campaign caps with half-return at 60 Influence, and visible certainty based on support strength and campaigning.
- Evidence reserved for the active removal motion survives its normal expiry until the vote and cannot also be exposed. An expired supporting scandal before nomination resolves invalidates that removal candidacy; already committed Influence remains spent.
- NPC hidden scandal creation chance is 5%/month, maximum three, with a 15% base discovery chance modified by severity.

All economic prices and Senate increments are read from configuration by the UI. These are starting balance values, not evidence of a statistically validated multiplayer balance.

## Verification and remaining review

The coordinated `just test` run passes all 166 tests. These include 24 politics-domain tests
in `tests/unit/diplomacy.rs`, `espionage.rs`, and `senate.rs`, plus the campaign espionage
adapter, private notifications, Senate navigation and UI layout regressions. Coverage includes
sealed nominations and ties, the preserved Censor rank, two Consul seats, same-month term expiry,
real foreign-action evidence, detection before discovery, and evidence-backed removal motions.

The real egui layout tests use Fira Sans and expanded controls at 500/380-point body widths,
scales 1.0/0.85, and bounded full panels. They cover nomination, campaign and saved-result states,
independent/owned/vassal diplomacy, evidence, histories and populated tables. They verify bounds;
they do not certify visual quality or native tooltip placement.

Foreign-happiness actions record actual misconduct. Player-target evidence opens the Senate;
NPC evidence opens the relevant province. Hostility records friendly attacks and treaty violations
at the action point, and networks in a perpetrator's holdings can discover those foreign acts.
No nonexistent tax-policy decision is invented. Bid controls validate the same eligibility,
sealed-motion lock, funds and sudden-death rules as the authoritative engine.

Native review of the redesigned panels remains pending after desktop control was stopped.
The shared [verification record](verification.md) reports the final command results. Online
campaign authority remains outside the local-first scope. NPC military behavior remains
strictly defensive under the user's clarification.
