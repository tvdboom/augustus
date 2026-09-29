# Opening economy balance audit (29 September 2026)

This audit uses the live atlas, default policies, the monthly simulation, and all seven eligible urban starting provinces. The figures below are rounded from one sampled start per city; starting populations vary slightly between games. The regression samples one through four players and checks every opening year.

| Resource | Opening stock | Sampled monthly position | Assessment |
| --- | ---: | ---: | --- |
| Food | 450 | 120–268 produced; 68–130 consumed by civilians | Each city opens with a local surplus. The smallest sampled margin is about 22 in Achaia, the largest about 178 in Africa Proconsularis. No sampled start has a shortage over twelve idle months. Food output now uses diminishing returns, reducing the prior maximum surplus of more than 700. |
| Metal | 120 | 0–112 produced | Early equipment is scarce in several starts. Africa Proconsularis has no local Metal output, so it relies on stock, trade, or expansion. Metal has no passive upkeep. |
| Stone | 200 | 30–248 produced | Every start can order a Road or Granary immediately. Larger works still require saving or trade. |
| Coin | 201 | 13–19 in provincial taxes before spending and trades | A normal Heavy Infantry recruitment project requests 0.2 Coin/month for its one drafted population unit, then the completed cohort costs 2 Coin/month in normal wages. An idle opening year stays solvent. |
| Influence | 40 | About 2.0–2.5 from local Nobles/month before rank | Early spy and diplomatic actions are available. The first 100 Influence Senate promotion still requires saving. |
| Population | About 68–130 units in a sampled starting province | About 25–44 Plebeian and 15–21 Citizen units initially | Each cohort drafts one population unit from its required class. The cohort's whole-person soldier count is separate. |

## Choices and constraints

- Heavy Infantry costs one Plebeian unit and 40 Metal; Light Infantry costs one Plebeian unit and 12 Metal. The initial 120 Metal buys at most three Heavy Infantry cohorts before other equipment spending, and each province trains one cohort at a time.
- A one-population-unit levy now incurs at least 2 happiness points of draft fatigue plus its share-of-class penalty. The penalty loses 1 point per month; repeated drafts stack. Recruitment Effort charges against drafted population units, so the displayed 1,000 soldiers in a Heavy Infantry cohort do not create a 200 Coin/month training bill.
- Full-strength monthly wages vary by unit type: Light Infantry and Archers cost 1 Coin, Heavy Infantry and most specialist cohorts cost 2–2.5, Heavy Cavalry costs 3, and War Elephants cost 5. Casualties lower wages proportionally. Army Wages policy applies its existing multiplier and morale effect.
- Food output is `raw / (1 + raw / 400)` after normal production modifiers. Achaia's sampled margin remains about 22 Food/month; the start check also advances twelve months to catch early shortages. Large farming provinces still feed more cohorts and can export Food.
- City capacity rises from 25 to 90. Some urban starts, especially Lugdunensis, received a population bonus far above their former capacity. That drove happiness to zero and could trigger an idle slave revolt and Food collapse. The start regression now checks for severe overcrowding as well as supply.
- The open market remembers each player's cumulative Buy and Sell volume for each resource until the next month. The marginal price worsens as that volume grows. One large order and several smaller orders of the same total quantity have the same total Coin value.
- A regression makes a modest opening in random one-player starts: recruit Heavy Infantry, order a Road, choose normal civic spending, sell 100 Food, buy 20 Metal, and advance twelve months. It checks supply, solvency, completed construction, and the trained cohort.

These are opening-balance checks. Long campaigns, expanded empires, warfare, and player trading patterns still need playtesting.
