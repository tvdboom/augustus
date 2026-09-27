//! Application adapter: one authoritative monthly transaction across all rule systems.

use super::*;
use crate::game::economy::*;
use crate::game::military::*;
use crate::game::politics::diplomacy::*;
use crate::game::politics::espionage::{EspionageConfig, EspionageState};
use crate::game::politics::senate::{PoliticalProfile, SenateConfig, SenateState};
use crate::game::politics::PoliticalPlayer;

#[cfg(test)]
#[path = "../../tests/unit/campaign_military.rs"]
mod military_integration_tests;

#[cfg(test)]
#[path = "../../tests/unit/campaign.rs"]
mod integration_tests;

/// Campaign authority; UI/HUD and map ownership are projections of these rules.
#[derive(Resource)]
pub(crate) struct Campaign {
    pub active: bool,
    pub economy: EconomyWorld,
    pub politics: Vec<ProvincePolitics>,
    pub actors: Vec<PoliticalPlayer>,
    pub senate: SenateState,
    pub senate_config: SenateConfig,
    pub diplomacy_config: DiplomacyConfig,
    pub espionage: EspionageState,
    pub espionage_config: EspionageConfig,
    pub military: MilitaryWorld,
    pub graph: Vec<MilitaryProvince>,
    pub wars: Vec<Vec<bool>>,
    pub npc_wars: Vec<Vec<bool>>,
    pub invitations: Vec<Vec<bool>>,
    pub profiles: Vec<PoliticalProfile>,
    pub messages: Vec<(String, Option<usize>)>,
    pub notifications: super::campaign_notifications::CampaignNotifications,
    pub recent_victories: Vec<f64>,
}

/// Project campaign state into the existing HUD/rank artwork and geographic ownership overlay.
pub(super) fn sync_campaign(
    mut campaign: ResMut<Campaign>,
    mut resources: ResMut<HudResources>,
    mut ownership: ResMut<ProvinceOwnership>,
    mut practice: ResMut<LocalPractice>,
    mut toasts: ResMut<toasts::ToastQueue>,
    mut paused: ResMut<GamePaused>,
) {
    if !campaign.active {
        return;
    }
    campaign.reconcile_provinces();
    for (id, p) in campaign.economy.provinces.iter().enumerate() {
        ownership.sync_campaign_province(
            id,
            p.owner,
            p.population,
            p.production(&campaign.economy.config).1,
            p.food_request(&campaign.economy.config),
        );
    }
    for player in 0..campaign.actors.len() {
        let wallet = &campaign.economy.players[player];
        let balances = wallet.balances();
        let delta =
            campaign.economy.last_report.player_delta.get(player).copied().unwrap_or([0.0; 5]);
        for r in 0..5 {
            resources.players[player][r] = HudResource {
                amount: balances[r],
                monthly_delta: delta[r],
            };
        }
        let owned: Vec<_> = campaign
            .economy
            .provinces
            .iter()
            .enumerate()
            .filter(|(_, p)| p.owner == Some(player))
            .collect();
        let total: f64 = owned.iter().map(|(_, p)| p.total_population()).sum();
        let change: f64 = owned
            .iter()
            .filter_map(|(id, _)| campaign.economy.last_report.province_reports.get(*id))
            .map(|r| r.population_delta)
            .sum();
        resources.players[player][5] = HudResource {
            amount: total,
            monthly_delta: change,
        };
        for class in 0..4 {
            let (happiness, monthly_delta) = campaign.economy.player_happiness(player, class);
            resources.happiness[player][class] = HudResource {
                amount: happiness,
                monthly_delta,
            };
        }
        if let Some(p) = practice.players.get_mut(player) {
            p.rank = campaign.actors[player].rank.ladder_index();
        }
    }
    for (text, province) in std::mem::take(&mut campaign.messages) {
        let mut toast = toasts::Toast::info(text);
        if let Some(id) = province {
            toast = toast.with_action(toasts::ToastAction::OpenProvince(id));
        }
        toasts.push(toast);
    }
    for notice in campaign.notifications.drain_for(practice.active_player) {
        use super::campaign_notifications::{NoticeAction, NoticeSeverity};
        let text = format!("{} {}", notice.title, notice.body);
        let toast = match notice.severity {
            NoticeSeverity::Info => toasts::Toast::info(text),
            NoticeSeverity::Warning => toasts::Toast::warning(text),
        };
        let action = match notice.action {
            NoticeAction::OpenSenate => toasts::ToastAction::OpenEvidence(None),
            NoticeAction::OpenProvince(id) => toasts::ToastAction::OpenProvince(id),
            NoticeAction::FocusWonder(id) => toasts::ToastAction::FocusWonder(id),
            NoticeAction::OpenScandal {
                province,
                ..
            } => toasts::ToastAction::OpenEvidence(province),
        };
        toasts.push(toast.with_action(action));
    }
    if campaign.senate.winner.is_some() {
        paused.0 = true;
    }
}

impl Default for Campaign {
    /// Allocate a dormant resource before a local game starts.
    fn default() -> Self {
        Self {
            active: false,
            economy: EconomyWorld::new(0, vec![], vec![]),
            politics: vec![],
            actors: vec![],
            senate: SenateState::new(1),
            senate_config: SenateConfig::default(),
            diplomacy_config: DiplomacyConfig::default(),
            espionage: EspionageState::new(2),
            espionage_config: EspionageConfig::default(),
            military: MilitaryWorld::new(0),
            graph: vec![],
            wars: vec![],
            npc_wars: vec![],
            invitations: vec![],
            profiles: vec![],
            messages: vec![],
            notifications: Default::default(),
            recent_victories: vec![],
        }
    }
}

impl Campaign {
    /// Seed all systems from the existing atlas and randomized local starting positions.
    pub fn start(&mut self, ownership: &ProvinceOwnership, count: usize) {
        *self = Self::default();
        let seeds = ownership.campaign_seeds();
        let provinces = seeds
            .iter()
            .map(|seed| {
                // The atlas's desert portrait covers the Nile; its inhabited economic
                // capacity follows irrigated farmland without changing combat terrain.
                let terrain = match seed.terrain {
                    _ if seed.name == "Aegyptus" => Terrain::Farmland,
                    0 => Terrain::Desert,
                    1 => Terrain::Farmland,
                    2 => Terrain::Forest,
                    3 => Terrain::Hills,
                    6 => Terrain::Marsh,
                    7 => Terrain::Mountains,
                    _ => Terrain::Plains,
                };
                let mut province = EconomicProvince::new(
                    &seed.name,
                    normalized_capacity_area(seed.area),
                    terrain,
                    seed.city,
                    seed.potential,
                    seed.population,
                    count,
                );
                province.owner = seed.owner;
                province.wonder_sites = seed.wonder_sites.clone();
                province
            })
            .collect();
        self.economy = EconomyWorld::new(
            count,
            provinces,
            seeds.iter().map(|p| p.neighbors.clone()).collect(),
        );
        self.politics = seeds
            .iter()
            .map(|p| {
                p.owner.map_or_else(
                    || ProvincePolitics::independent(count),
                    |o| ProvincePolitics::owned(count, o),
                )
            })
            .collect();
        self.actors = vec![PoliticalPlayer::default(); count];
        self.profiles = vec![PoliticalProfile::default(); count];
        self.wars = vec![vec![false; count]; count];
        self.npc_wars = vec![vec![false; seeds.len()]; count];
        self.invitations = vec![vec![false; count]; count];
        self.recent_victories = vec![0.0; count];
        self.graph = seeds
            .iter()
            .map(|p| MilitaryProvince {
                terrain: match p.terrain {
                    0 => MilitaryTerrain::Desert,
                    1 => MilitaryTerrain::Farmland,
                    2 => MilitaryTerrain::Forest,
                    3 => MilitaryTerrain::Hills,
                    6 => MilitaryTerrain::Marsh,
                    7 => MilitaryTerrain::Mountains,
                    _ => MilitaryTerrain::Plains,
                },
                area: p.area.max(0.1),
                road_level: 0,
                neighbors: p.neighbors.clone(),
            })
            .collect();
        self.military = MilitaryWorld::new(seeds.len());
        for (id, p) in seeds.iter().enumerate() {
            if p.owner.is_none() {
                let _ = self.military.seed_local_defenders(id, &p.name);
            }
        }
        let seed = rand::random::<u64>();
        self.senate = SenateState::new(seed);
        self.espionage = EspionageState::new(seed ^ 0x51a7);
        self.active = true;
        self.economy.refresh_npc_markets(&self.inputs());
        self.pull_wallets();
        self.refresh_profiles();
        self.initialize_notification_snapshot();
    }

    /// Copy economic wallets before political actions; only one module owns each balance.
    pub fn pull_wallets(&mut self) {
        for (actor, wallet) in self.actors.iter_mut().zip(&self.economy.players) {
            actor.coin = wallet.coin;
            actor.influence = wallet.influence;
        }
    }

    /// Commit spending back after political actions, retaining rank and term separately.
    pub fn push_wallets(&mut self) {
        for (actor, wallet) in self.actors.iter().zip(&mut self.economy.players) {
            wallet.coin = actor.coin;
            wallet.influence = actor.influence;
        }
    }

    /// Snapshot external demand, including units in movement and battle.
    pub fn inputs(&self) -> MonthlyInputs {
        MonthlyInputs {
            army_food: (0..self.actors.len())
                .map(|p| self.military.food_demand(ForceOwner::Player(p)))
                .collect(),
            npc_army_food: (0..self.politics.len())
                .map(|p| self.military.food_demand(ForceOwner::Local(p)))
                .collect(),
            player_hostility: self.wars.clone(),
            npc_hostility: self.npc_wars.clone(),
        }
    }

    /// Price political reach from owned provinces, using the same sea/land graph as trade.
    pub fn distance(&self, player: usize, target: usize) -> Option<usize> {
        let sources: Vec<_> = self
            .economy
            .provinces
            .iter()
            .enumerate()
            .filter_map(|(i, p)| (p.owner == Some(player)).then_some(i))
            .collect();
        political_distance(&self.economy.adjacency, &sources, target, |id| {
            let province = &self.economy.provinces[id];
            province.owner.is_none_or(|other| !self.wars[player][other])
        })
    }

    /// Movement permission separates invitations/friendship from declared invasions.
    pub fn access_snapshot(&self) -> Vec<Vec<MilitaryAccess>> {
        (0..self.actors.len())
            .map(|player| {
                self.politics
                    .iter()
                    .enumerate()
                    .map(|(id, p)| match p.state {
                        PoliticalState::Owned {
                            owner,
                        } => {
                            if self.wars[player][owner] {
                                MilitaryAccess::Invasion
                            } else if owner == player || self.invitations[owner][player] {
                                MilitaryAccess::Peaceful
                            } else {
                                MilitaryAccess::Blocked
                            }
                        },
                        PoliticalState::Vassal {
                            overlord,
                            ..
                        } if overlord == player => MilitaryAccess::Peaceful,
                        _ => {
                            if self.npc_wars[player][id] {
                                MilitaryAccess::Invasion
                            } else if p.relation(player) >= self.military.config.npc_access_relation
                            {
                                MilitaryAccess::Peaceful
                            } else {
                                MilitaryAccess::Blocked
                            }
                        },
                    })
                    .collect()
            })
            .collect()
    }

    /// Declare hostility explicitly; peaceful stationing never grants political power.
    pub fn declare_hostility(&mut self, player: usize, province: usize) {
        use crate::game::politics::espionage::{ScandalKind, Severity};
        let target = self.economy.provinces[province]
            .owner
            .map_or(TradeParty::Npc(province), TradeParty::Player);
        let already_hostile = match target {
            TradeParty::Player(other) => self.wars[player][other],
            TradeParty::Npc(id) => self.npc_wars[player][id],
        };
        if already_hostile || target == TradeParty::Player(player) {
            return;
        }
        if self.politics[province].relation(player) >= 60.0 {
            self.espionage.record_action(
                player,
                Some(province),
                ScandalKind::FriendlyAttack,
                Severity::Major,
                self.economy.month,
                &self.espionage_config,
            );
        }
        if self.economy.trades.iter().any(|t| {
            t.frequency == TradeFrequency::Monthly
                && t.status == TradeStatus::Active
                && ((t.party_a == TradeParty::Player(player) && t.party_b == target)
                    || (t.party_b == TradeParty::Player(player) && t.party_a == target))
        }) {
            self.espionage.record_action(
                player,
                Some(province),
                ScandalKind::TreatyViolation,
                Severity::Medium,
                self.economy.month,
                &self.espionage_config,
            );
        }
        match self.politics[province].state {
            PoliticalState::Owned {
                owner,
            } if owner != player => {
                self.wars[player][owner] = true;
                self.wars[owner][player] = true;
                self.invitations[player][owner] = false;
                self.invitations[owner][player] = false;
            },
            PoliticalState::Owned {
                ..
            } => return,
            _ => {
                self.npc_wars[player][province] = true;
                self.politics[province].change_relation(player, -30.0);
            },
        }
        self.begin_encounter(province, ForceOwner::Player(player), None);
        self.reconcile_provinces();
    }

    /// Military hostility belongs to force owners, independently of territorial access.
    pub fn forces_hostile(&self, a: ForceOwner, b: ForceOwner) -> bool {
        match (a, b) {
            (ForceOwner::Player(a), ForceOwner::Player(b)) => self.wars[a][b],
            (ForceOwner::Player(player), ForceOwner::Local(province))
            | (ForceOwner::Local(province), ForceOwner::Player(player)) => {
                self.npc_wars[player][province]
            },
            (ForceOwner::Local(_), ForceOwner::Local(_)) => false,
        }
    }

    /// Start a battle only against actually hostile stationed owners.
    fn begin_encounter(&mut self, province: usize, attacker: ForceOwner, origin: Option<usize>) {
        let ForceOwner::Player(player) = attacker else {
            return;
        };
        if self.military.provinces[province].forces.get(&attacker).is_none_or(|u| u.is_empty()) {
            return;
        }
        if let Some(battle) = self.military.battles.iter().find(|b| b.province == province) {
            let hostile = |owner: &ForceOwner| self.forces_hostile(attacker, *owner);
            let against_attackers = battle.attackers.plans.keys().any(hostile);
            let against_defenders = battle.defenders.plans.keys().any(hostile);
            let coalition = if battle.attackers.plans.contains_key(&attacker) {
                Some(true)
            } else if battle.defenders.plans.contains_key(&attacker) {
                Some(false)
            } else if against_defenders && !against_attackers {
                Some(true)
            } else if against_attackers && !against_defenders {
                Some(false)
            } else {
                None
            };
            if let Some(attacking) = coalition {
                let _ = self.military.join_battle(province, attacker, attacking);
            }
            // A third mutually hostile coalition waits for this engagement to resolve.
            // The monthly encounter scan starts its battle next, preserving all troops.
            return;
        }
        let p = &self.economy.provinces[province];
        let mut candidates: Vec<_> = self.military.provinces[province]
            .forces
            .iter()
            .filter_map(|(&owner, units)| {
                let hostile = self.forces_hostile(attacker, owner);
                (hostile && !units.is_empty()).then_some(owner)
            })
            .collect();
        // The territorial defender takes precedence. Other hostile coalitions
        // wait rather than being forced into an alliance with their enemies.
        candidates.sort_by_key(|&owner| {
            let territorial = match owner {
                ForceOwner::Player(player) => p.owner == Some(player),
                ForceOwner::Local(home) => p.owner.is_none() && home == province,
            };
            (!territorial, owner)
        });
        let mut defenders = Vec::new();
        for candidate in candidates {
            if defenders.iter().all(|&other| !self.forces_hostile(candidate, other)) {
                defenders.push(candidate);
            }
        }
        if defenders.is_empty() {
            if p.owner.is_some_and(|owner| owner != player && self.wars[player][owner]) {
                let _ = self.politics[province].capture_owned(player);
                self.messages.push((
                    format!("{} captured by Player {}.", p.name, player + 1),
                    Some(province),
                ));
            } else if p.owner.is_none() && self.npc_wars[player][province] {
                self.military.provinces[province].occupation = Some(attacker);
            }
            return;
        }
        let started = self.military.start_battle(
            province,
            &[attacker],
            &defenders,
            p.owner,
            origin,
            self.graph[province].terrain,
            p.level(BuildingType::Fort) + p.level(BuildingType::CityWalls),
            u64::from(self.economy.month) * 179 + province as u64 + 1,
        );
        if started.is_ok() {
            self.notify_battle_started(province);
        }
    }

    /// Synchronize legal ownership before economic calculations and after political transitions.
    pub fn reconcile_provinces(&mut self) {
        let mut ownership_changed = false;
        for (id, politics) in self.politics.iter().enumerate() {
            let (owner, overlord) = match politics.state {
                PoliticalState::Owned {
                    owner,
                } => (Some(owner), None),
                PoliticalState::Vassal {
                    overlord,
                    ..
                } => (None, Some(overlord)),
                _ => (None, None),
            };
            ownership_changed |= self.economy.provinces[id].owner != owner;
            self.economy.provinces[id].change_owner(owner, overlord);
            self.economy.provinces[id].relation_by_player.clone_from(&politics.relations);
            self.economy.provinces[id].trade_ratio_by_player = (0..self.actors.len())
                .map(|p| self.espionage.trade_ratio(p, id, self.economy.month))
                .collect();
            self.graph[id].road_level = self.economy.provinces[id].level(BuildingType::Road);
        }
        if ownership_changed {
            self.economy.recalculate_storage();
            for wallet in &mut self.economy.players {
                wallet.clamp_storage();
            }
        }
    }

    /// Execute one month exactly once, with shared supply and simultaneous political pressure.
    pub fn advance_month(&mut self) {
        if !self.active || self.senate.winner.is_some() {
            return;
        }
        let notification_before = self.notification_snapshot();
        self.reconcile_provinces();
        let previous_occupations: Vec<_> =
            self.military.provinces.iter().map(|p| p.occupation).collect();
        let before: Vec<_> = self.economy.players.iter().map(PlayerEconomy::balances).collect();
        // Complete paid cohorts before calculating any military food requests.
        let owners: Vec<_> = self.economy.provinces.iter().map(|p| p.owner).collect();
        let mut military_events = self.military.advance_recruitment(|p| owners[p]);
        for (id, province) in self.economy.provinces.iter_mut().enumerate() {
            province.happiness_modifiers =
                self.military.provinces[id].draft_penalties.map(|penalty| -penalty);
        }
        let access = self.access_snapshot();
        let permission = |owner: ForceOwner, id: usize| match owner {
            ForceOwner::Player(p) => access[p][id],
            ForceOwner::Local(home) => {
                if home == id {
                    MilitaryAccess::Peaceful
                } else {
                    MilitaryAccess::Blocked
                }
            },
        };
        let arrivals = self.military.advance_movement(&self.graph, permission);
        for event in &arrivals {
            if let MilitaryEvent::Arrived {
                province,
                origin,
                owner,
                ..
            } = event
            {
                self.begin_encounter(*province, *owner, Some(*origin));
            }
        }
        military_events.extend(arrivals);
        // Also engage when relations change between owners already sharing a province.
        let stationed: Vec<_> = self
            .military
            .provinces
            .iter()
            .enumerate()
            .flat_map(|(id, p)| {
                p.forces
                    .iter()
                    .filter(|(_, units)| !units.is_empty())
                    .map(move |(&owner, _)| (id, owner))
            })
            .collect();
        for (province, owner) in stationed {
            self.begin_encounter(province, owner, None);
        }
        self.reconcile_provinces();
        let inputs = self.inputs();
        let report = self.economy.begin_month(&inputs);
        // Programs follow trade, but precede food, tax, noble and rank income.
        self.pull_wallets();
        let mut paid_support = Vec::with_capacity(self.politics.len());
        for id in 0..self.politics.len() {
            let distances: Vec<_> = (0..self.actors.len()).map(|p| self.distance(p, id)).collect();
            paid_support.push(self.politics[id].spend_recurring_support(
                &mut self.actors,
                &distances,
                &self.diplomacy_config,
            ));
        }
        self.push_wallets();
        let report = self.economy.finish_month(&inputs, report);
        self.record_economic_events(&report);
        for (p, &supply) in report.food_supply_ratio.iter().enumerate() {
            self.military.apply_supply(ForceOwner::Player(p), supply);
        }
        for (id, p) in report.province_reports.iter().enumerate() {
            self.military.apply_supply(ForceOwner::Local(id), p.food_supply_ratio);
        }
        military_events.extend(self.military.advance_battles(&self.graph, permission));
        for event in military_events {
            if let MilitaryEvent::BattleEnded {
                battle,
                result,
                ..
            } = &event
            {
                if let Some(outcome) =
                    self.military.history.iter().find(|record| record.id == *battle)
                {
                    for (attacking, side) in
                        [(true, &outcome.attackers), (false, &outcome.defenders)]
                    {
                        let won = (attacking && *result == BattleResult::AttackerVictory)
                            || (!attacking && *result == BattleResult::DefenderVictory);
                        for owner in side.plans.keys() {
                            if let ForceOwner::Player(p) = owner {
                                self.recent_victories[*p] += if won {
                                    1.0
                                } else {
                                    -1.0
                                };
                            }
                        }
                    }
                }
            }
            self.record_military_event(&event);
            if let MilitaryEvent::BattleEnded {
                province,
                previous_owner,
                winner: Some(ForceOwner::Player(player)),
                result,
                ..
            } = event
            {
                if result == BattleResult::AttackerVictory
                    && previous_owner.is_some_and(|old| old != player && self.wars[player][old])
                {
                    let _ = self.politics[province].capture_owned(player);
                    self.economy.provinces[province].temporary_happiness = [-15.0; 4];
                }
            }
        }
        self.reconcile_provinces();
        self.pull_wallets();
        for effect in &report.trade_effects {
            self.politics[effect.province].apply_trade(
                effect.player,
                effect.relation_gain,
                effect.control_gain,
                &self.diplomacy_config,
            );
        }
        for (id, previous_occupation) in previous_occupations.iter().enumerate() {
            let power: Vec<_> = (0..self.actors.len())
                .map(|p| {
                    if self.military.province_in_battle(id) {
                        return 0.0;
                    }
                    let owner = ForceOwner::Player(p);
                    self.military.stationed_strength(id, owner)
                        * self.military.config.rank_control[self.military.rank(owner) as usize]
                })
                .collect();
            // New occupation first generates Control on the following month's political tick.
            let occupation = match self.military.provinces[id].occupation {
                Some(ForceOwner::Player(p))
                    if *previous_occupation == Some(ForceOwner::Player(p)) =>
                {
                    Some(p)
                },
                _ => None,
            };
            self.politics[id].resolve_month(
                &power,
                occupation,
                paid_support[id],
                &self.diplomacy_config,
            );
            if let Some((p, amount)) = self.politics[id].vassal_income(&self.diplomacy_config) {
                self.actors[p].influence += amount;
            }
            let market = &mut self.economy.provinces[id].market;
            if let Some((p, tribute)) =
                self.politics[id].tribute_due(market.monthly_coin_income.min(market.coin_treasury))
            {
                market.coin_treasury -= tribute;
                self.actors[p].coin += tribute;
            }
        }
        self.push_wallets();
        self.reconcile_provinces();
        self.pull_wallets();
        for actor in &mut self.actors {
            actor.influence += self.senate_config.rank_income(actor.rank);
        }
        self.advance_espionage();
        self.refresh_profiles();
        for event in
            self.senate.advance_month(&mut self.actors, &self.profiles, &self.senate_config)
        {
            self.handle_senate_evidence(&event);
            self.record_senate_event(&event);
        }
        self.push_wallets();
        for (p, wallet) in self.economy.players.iter().enumerate() {
            if let Some(delta) = self.economy.last_report.player_delta.get_mut(p) {
                *delta = std::array::from_fn(|r| wallet.balances()[r] - before[p][r]);
            }
        }
        for victory in &mut self.recent_victories {
            *victory *= 0.95;
        }
        self.record_monthly_notifications(notification_before);
    }

    /// Route economic outcomes only to affected players, including failures hidden by capped stocks.
    fn record_economic_events(&mut self, report: &MonthlyReport) {
        use super::campaign_notifications::{NoticeKind, NoticeSeverity};
        let mut failed_trades = vec![Vec::<String>::new(); self.actors.len()];
        for event in &report.events {
            match event {
                EconomyEvent::BuildingCompleted {
                    province,
                    building,
                    level,
                } => {
                    if let Some(owner) = self.economy.provinces[*province].owner {
                        self.notifications.province_notice(
                            owner,
                            *province,
                            report.month,
                            NoticeSeverity::Info,
                            NoticeKind::BuildingCompleted,
                            format!("{} completed", building.name()),
                            format!(
                                "{} reached level {level}.",
                                self.economy.provinces[*province].name
                            ),
                        );
                    }
                },
                EconomyEvent::WonderCompleted {
                    province,
                    wonder,
                    owner: Some(owner),
                } => {
                    self.notifications.province_notice(
                        *owner,
                        *province,
                        report.month,
                        NoticeSeverity::Info,
                        NoticeKind::WonderCompleted,
                        "Wonder completed",
                        format!(
                            "{} now produces Influence.",
                            crate::map::wonder_name(*wonder).unwrap_or("Monument")
                        ),
                    );
                },
                EconomyEvent::FoodShortage {
                    player,
                    supplied,
                } => {
                    if let Some(province) =
                        self.economy.provinces.iter().position(|p| p.owner == Some(*player))
                    {
                        self.notifications.province_notice(*player,province,report.month,NoticeSeverity::Warning,NoticeKind::FoodShortage,
                            "Food shortage",format!("Civilian and military food requests were supplied at {:.0}%. Review production, rations, and imports.",supplied*100.0));
                    }
                },
                EconomyEvent::TradeSuspended {
                    agreement,
                    ..
                }
                | EconomyEvent::TradeCancelled {
                    agreement,
                } => {
                    if let Some(trade) = self.economy.trades.iter().find(|t| t.id == *agreement) {
                        for party in [trade.party_a, trade.party_b] {
                            if let TradeParty::Player(player) = party {
                                failed_trades[player].push(format!(
                                    "Agreement #{agreement}: {}",
                                    trade.last_failure.as_deref().unwrap_or("cancelled")
                                ));
                            }
                        }
                    }
                },
                _ => {},
            }
        }
        for (player, mut failures) in failed_trades.into_iter().enumerate() {
            failures.sort();
            failures.dedup();
            if !failures.is_empty() {
                if let Some(province) =
                    self.economy.provinces.iter().position(|p| p.owner == Some(player))
                {
                    self.notifications.province_notice(
                        player,
                        province,
                        report.month,
                        NoticeSeverity::Warning,
                        NoticeKind::TradeInterrupted,
                        "Trade interrupted",
                        failures.join(" · "),
                    );
                }
            }
        }
    }

    /// Derive live Senate attitudes from the same public facts displayed in province panels.
    pub fn refresh_profiles(&mut self) {
        for player in 0..self.actors.len() {
            let owned: Vec<_> =
                self.economy.provinces.iter().filter(|p| p.owner == Some(player)).collect();
            let mut counts = [0.0; 4];
            let mut happy = [0.0; 4];
            for p in &owned {
                for c in 0..4 {
                    counts[c] += p.population[c];
                    happy[c] += p.population[c] * p.happiness[c];
                }
            }
            for c in 0..4 {
                happy[c] = if counts[c] > 0.0 {
                    happy[c] / counts[c]
                } else {
                    50.0
                };
            }
            let food =
                self.economy.last_report.food_supply_ratio.get(player).copied().unwrap_or(1.0);
            let n = owned.len().max(1) as f64;
            let recurring: Vec<_> = self
                .economy
                .trades
                .iter()
                .filter(|t| {
                    t.frequency == TradeFrequency::Monthly
                        && matches!(t.status, TradeStatus::Active | TradeStatus::Suspended)
                        && (t.party_a == TradeParty::Player(player)
                            || t.party_b == TradeParty::Player(player))
                })
                .collect();
            let delivered = |trade: &&TradeAgreement| {
                if trade.last_executed_month == Some(self.economy.month) {
                    trade.last_delivered_value
                } else {
                    0.0
                }
            };
            let trade_volume = recurring.iter().map(delivered).sum();
            let trade_reliability = if recurring.is_empty() {
                1.0
            } else {
                recurring
                    .iter()
                    .map(|t| {
                        if t.last_executed_month == Some(self.economy.month) {
                            t.last_fulfillment
                        } else {
                            0.0
                        }
                    })
                    .sum::<f64>()
                    / recurring.len() as f64
            };
            let provincial_trade = recurring
                .iter()
                .filter(|t| {
                    matches!(t.party_a, TradeParty::Npc(_))
                        || matches!(t.party_b, TradeParty::Npc(_))
                })
                .map(delivered)
                .sum();
            let wallet = &self.economy.players[player];
            let needs = [
                owned.iter().map(|p| p.food_request(&self.economy.config)).sum::<f64>()
                    + self.military.food_demand(ForceOwner::Player(player)),
                self.economy.config.trade.npc_base_need[1] * n,
                self.economy.config.trade.npc_base_need[2] * n,
            ];
            let resource_security = (0..3)
                .map(|r| (wallet.resources[r] / needs[r].max(1.0)).clamp(0.0, 1.0))
                .sum::<f64>()
                / 3.0;
            let vassals: Vec<_> = self
                .politics
                .iter()
                .filter(
                    |p| matches!(p.state, PoliticalState::Vassal {overlord,..} if overlord==player),
                )
                .collect();
            let mut profile = PoliticalProfile {
                nobles: counts[0],
                noble_happiness: happy[0],
                citizen_happiness: happy[1],
                plebeian_happiness: happy[2],
                provincial_happiness: (happy[0] + happy[1] + happy[2]) / 3.0,
                food_security: food,
                famine: 1.0 - food,
                coin_income: self
                    .economy
                    .last_report
                    .player_delta
                    .get(player)
                    .map_or(0.0, |d| d[3]),
                trade_volume,
                trade_reliability,
                provincial_trade,
                resource_security,
                wonders: owned.iter().filter(|p| p.completed_wonder.is_some()).count() as f64,
                political_buildings: owned
                    .iter()
                    .map(|p| f64::from(p.level(BuildingType::Forum)))
                    .sum(),
                markets: owned.iter().map(|p| f64::from(p.level(BuildingType::UrbanMarket))).sum(),
                food_policy: owned
                    .iter()
                    .map(|p| p.policies.food as usize as f64 - 1.0)
                    .sum::<f64>()
                    / n,
                harsh_policies: owned
                    .iter()
                    .filter(|p| p.policies.slave_labor == SlaveLabor::Harsh)
                    .count() as f64
                    / n,
                military_strength: self
                    .military
                    .all_units()
                    .filter(|u| u.owner == ForceOwner::Player(player))
                    .map(|u| u.effective_strength(&self.military.config))
                    .sum(),
                military_rank: self.military.rank(ForceOwner::Player(player)) as usize as f64,
                recent_victories: self.recent_victories[player],
                ..PoliticalProfile::default()
            };
            if !vassals.is_empty() {
                profile.vassal_relation =
                    vassals.iter().map(|p| p.relation(player)).sum::<f64>() / vassals.len() as f64;
                profile.voluntary_vassal_stability = vassals
                    .iter()
                    .map(|p| match p.state {
                        PoliticalState::Vassal {
                            control,
                            ..
                        } => control / 100.0 * ((p.relation(player) - 50.0) / 50.0).max(0.0),
                        _ => 0.0,
                    })
                    .sum::<f64>()
                    / vassals.len() as f64;
                profile.high_tribute = vassals
                    .iter()
                    .filter(|p| {
                        matches!(
                            p.state,
                            PoliticalState::Vassal {
                                tribute: Tribute::High,
                                ..
                            }
                        )
                    })
                    .count() as f64
                    / vassals.len() as f64;
            }
            self.profiles[player] = profile;
        }
    }
}
