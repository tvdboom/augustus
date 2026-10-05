//! Application adapter: one authoritative monthly transaction across all rule systems.

use super::*;
use crate::game::economy::*;
use crate::game::military::*;
use crate::game::politics::diplomacy::*;
use crate::game::politics::espionage::{EspionageConfig, EspionageState};
use crate::game::politics::senate::{PoliticalProfile, SenateConfig, SenateState};
use crate::game::politics::{Currency, PoliticalError, PoliticalPlayer, PoliticalRng};

#[cfg(test)]
#[path = "../../tests/unit/campaign_military.rs"]
mod military_integration_tests;

#[cfg(test)]
#[path = "../../tests/unit/campaign.rs"]
mod integration_tests;

/// Campaign authority; UI/HUD and map ownership are projections of these rules.
#[derive(Resource, Clone)]
pub(crate) struct Campaign {
    pub active: bool,
    pub governance: Vec<Governance>,
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
    /// Province-specific access overrides keyed by province, host and guest.
    pub province_access: std::collections::BTreeMap<(usize, usize, usize), bool>,
    /// Explicit coercive entry into independent provinces, without declaring war.
    pub military_pressure: std::collections::BTreeSet<(usize, usize)>,
    /// A player loses permanently when their last directly owned province is lost.
    pub defeated: Vec<bool>,
    pub profiles: Vec<PoliticalProfile>,
    pub messages: Vec<(String, Option<usize>)>,
    pub notifications: super::campaign_notifications::CampaignNotifications,
    pub recent_victories: Vec<f64>,
    /// Last use year for each actor and political action (bribe or insult).
    pub diplomacy_used: std::collections::BTreeMap<(usize, bool), u32>,
    /// Last use month for each actor and nationwide civic event.
    pub event_used: std::collections::BTreeMap<(usize, super::campaign_events::CivicEvent), u32>,
    pub diplomacy_rng: PoliticalRng,
}

/// Project campaign state into the existing HUD/rank artwork and geographic ownership overlay.
pub(super) fn sync_campaign(
    mut campaign: ResMut<Campaign>,
    mut resources: ResMut<HudResources>,
    mut ownership: ResMut<ProvinceOwnership>,
    mut practice: ResMut<LocalPractice>,
    mut toasts: ResMut<toasts::ToastQueue>,
    mut paused: ResMut<GamePaused>,
    terminal: Res<TerminalPresentation>,
) {
    if !campaign.active {
        return;
    }
    toasts.set_player(practice.active_player);
    campaign.reconcile_provinces();
    campaign.notify_rank_opportunities();
    for (id, p) in campaign.economy.provinces.iter().enumerate().filter(|(_, p)| p.name != "Rome") {
        ownership.sync_campaign_province(
            id,
            p.owner,
            p.population,
            p.production(&campaign.economy.config).1,
            p.food_request(&campaign.economy.config),
        );
    }
    let monthly_inputs = campaign.inputs();
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
        // Show the current monthly food flow, including army rations, before
        // the first turn and immediately after province or army changes.
        resources.players[player][0].monthly_delta =
            campaign.projected_food_delta(player, &monthly_inputs);
        let (total, _, change) = campaign.economy.player_population(player);
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
        } else {
            toast = toast.with_action(toasts::ToastAction::OpenEvidence(None));
        }
        toasts.push(toast);
    }
    for notice in campaign.notifications.drain_for(practice.active_player) {
        toasts.push(toasts::Toast::from_notice(notice));
    }
    if !terminal.spectating
        && (campaign.senate.winner.is_some()
            || campaign.defeated.get(practice.active_player).copied().unwrap_or(false))
    {
        paused.0 = true;
    }
}

impl Default for Campaign {
    /// Allocate a dormant resource before a local game starts.
    fn default() -> Self {
        Self {
            active: false,
            governance: vec![],
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
            province_access: Default::default(),
            military_pressure: Default::default(),
            defeated: vec![],
            profiles: vec![],
            messages: vec![],
            notifications: Default::default(),
            recent_victories: vec![],
            diplomacy_used: Default::default(),
            event_used: Default::default(),
            diplomacy_rng: PoliticalRng::new(3),
        }
    }
}

impl Campaign {
    /// Seed all systems from the existing atlas and randomized local starting positions.
    pub fn start(&mut self, ownership: &ProvinceOwnership, count: usize) {
        self.military_pressure.clear();
        *self = Self::default();
        let seeds = ownership.campaign_seeds();
        let mut provinces: Vec<_> = seeds
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
        // Append the capital after the geographic provinces, preserving every atlas ID.
        // It has no provincial population, production, or economic trade routes.
        let rome = provinces.len();
        provinces.push(EconomicProvince::new(
            "Rome",
            0.0,
            Terrain::Plains,
            true,
            [0.0; 3],
            [0.0; 4],
            count,
        ));
        let mut adjacency: Vec<_> = seeds.iter().map(|p| p.neighbors.clone()).collect();
        adjacency.push(vec![]);
        self.economy = EconomyWorld::new(count, provinces, adjacency);
        self.politics = seeds
            .iter()
            .map(|p| {
                p.owner.map_or_else(
                    || ProvincePolitics::independent(count),
                    |o| ProvincePolitics::owned(count, o),
                )
            })
            .collect();
        self.politics.push(ProvincePolitics::rome(count));
        self.actors = vec![PoliticalPlayer::default(); count];
        self.governance = (0..count).map(|player| ownership.governance_for(player)).collect();
        self.profiles = vec![PoliticalProfile::default(); count];
        self.wars = vec![vec![false; count]; count];
        self.npc_wars = vec![vec![false; self.politics.len()]; count];
        self.invitations = vec![vec![false; count]; count];
        self.defeated = vec![false; count];
        self.recent_victories = vec![0.0; count];
        self.diplomacy_used.clear();
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
        let approaches: Vec<_> = seeds
            .iter()
            .enumerate()
            .filter_map(|(id, p)| ROME_APPROACHES.contains(&p.name.as_str()).then_some(id))
            .collect();
        for &id in &approaches {
            self.graph[id].neighbors.push(rome);
        }
        self.graph.push(MilitaryProvince {
            terrain: MilitaryTerrain::Plains,
            area: 0.1,
            road_level: 0,
            neighbors: approaches,
        });
        self.military = MilitaryWorld::new(self.politics.len());
        for (id, p) in seeds.iter().enumerate() {
            if p.owner.is_none() {
                let _ = self.military.seed_local_defenders(id, &p.name);
            }
        }
        let _ = self.military.seed_local_defenders(rome, "Rome");
        let seed = rand::random::<u64>();
        self.diplomacy_rng = PoliticalRng::new(seed ^ 0xB12BE);
        self.senate = SenateState::with_config(seed, &self.senate_config);
        self.espionage = EspionageState::new(seed ^ 0x51a7);
        self.active = true;
        self.economy.refresh_npc_markets(&self.inputs());
        self.sync_owned_relations();
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

    /// Stable campaign location of the city; it has no geographic province polygon.
    pub(crate) fn rome_location(&self) -> Option<usize> {
        self.economy.provinces.iter().position(|p| p.name == "Rome")
    }

    /// Domestic Relation follows the people living in each owned province.
    fn sync_owned_relations(&mut self) {
        for (politics, province) in self.politics.iter_mut().zip(&self.economy.provinces) {
            if let PoliticalState::Owned {
                owner,
            } = politics.state
            {
                politics.relations[owner] = province.mean_happiness();
            }
        }
    }

    /// Commit spending back after political actions, retaining rank and term separately.
    pub fn push_wallets(&mut self) {
        for (actor, wallet) in self.actors.iter().zip(&mut self.economy.players) {
            wallet.coin = actor.coin;
            wallet.influence = actor.influence;
        }
    }

    /// Pay for the next military rank after its historical army and battle milestones.
    pub fn promote_military(&mut self, player: usize, target: MilitaryRank) -> Result<(), String> {
        if !self.active || player >= self.economy.players.len() {
            return Err("No active military career for this player.".into());
        }
        let owner = ForceOwner::Player(player);
        self.military.observe_peak_manpower(owner);
        let requirements = self.military.promotion_eligibility(
            owner,
            target,
            self.economy.players[player].influence,
        )?;
        let wallet = &mut self.economy.players[player];
        wallet.influence -= requirements.influence;
        self.military.ranks.insert(owner, target);
        self.pull_wallets();
        self.record_military_event(&MilitaryEvent::RankIncreased {
            owner,
            rank: target,
        });
        Ok(())
    }

    /// One payment purchases between zero and ten Control, once per actor per year.
    pub fn noble_bribe_quote(&self, player: usize, province: usize) -> Result<f64, PoliticalError> {
        let target = self.politics.get(province).ok_or(PoliticalError::MissingTarget)?;
        if player >= self.actors.len()
            || !(matches!(target.state, PoliticalState::Independent { .. })
                || matches!(target.state, PoliticalState::Owned { owner } if owner != player))
        {
            return Err(PoliticalError::Ineligible);
        }
        Ok(500.0 + 5.0 * target.control(player))
    }

    pub fn noble_bribe_cost(&self, player: usize, province: usize) -> Result<f64, PoliticalError> {
        let cost = self.noble_bribe_quote(player, province)?;
        if self.diplomacy_used.get(&(player, false)) == Some(&(self.economy.month / 12)) {
            return Err(PoliticalError::AlreadyUsed);
        }
        Ok(cost)
    }

    pub fn bribe_nobles(
        &mut self,
        player: usize,
        province: usize,
    ) -> Result<(u32, Option<(usize, u64)>), PoliticalError> {
        let cost = self.noble_bribe_cost(player, province)?;
        self.pull_wallets();
        self.actors[player].spend(Currency::Coin, cost)?;
        let gain = (self.diplomacy_rng.unit() * 11.0).floor() as u32;
        self.politics[province].gain_control_now(player, gain as f64)?;
        self.diplomacy_used.insert((player, false), self.economy.month / 12);
        self.push_wallets();
        let evidence = self.espionage.observe_noble_bribe(
            player,
            province,
            self.economy.month,
            &self.espionage_config,
        );
        if let Some((observer, _)) = evidence {
            self.notifications.province_notice(
                observer,
                province,
                self.economy.month,
                super::campaign_notifications::NoticeSeverity::Info,
                super::campaign_notifications::NoticeKind::ScandalDiscovered,
                "Noble bribery uncovered",
                format!(
                    "Your spy caught Player {} bribing nobles in {}.",
                    player + 1,
                    self.economy.provinces[province].name
                ),
            );
        }
        Ok((gain, evidence))
    }

    /// Resolve a diplomatic insult to a rival player or the selected NPC province.
    pub fn insult_target(
        &self,
        player: usize,
        province: usize,
    ) -> Result<TradeParty, PoliticalError> {
        if player >= self.actors.len() {
            return Err(PoliticalError::Ineligible);
        }
        match self.politics.get(province).map(|p| &p.state) {
            Some(PoliticalState::Owned {
                owner,
            }) if *owner != player => Ok(TradeParty::Player(*owner)),
            Some(
                PoliticalState::Independent {
                    ..
                }
                | PoliticalState::Vassal {
                    ..
                },
            ) => Ok(TradeParty::Npc(province)),
            _ => Err(PoliticalError::Ineligible),
        }
    }

    /// A public insult costs only ten Relation, once per actor per year.
    pub fn send_insult(
        &mut self,
        player: usize,
        province: usize,
    ) -> Result<TradeParty, PoliticalError> {
        let target = self.insult_target(player, province)?;
        if self.diplomacy_used.get(&(player, true)) == Some(&(self.economy.month / 12)) {
            return Err(PoliticalError::AlreadyUsed);
        }
        match target {
            TradeParty::Player(target) => {
                for politics in &mut self.politics {
                    if matches!(politics.state, PoliticalState::Owned { owner } if owner == target)
                    {
                        politics.change_relation(player, -10.0);
                    }
                }
            },
            TradeParty::Npc(province) => self.politics[province].change_relation(player, -10.0),
        }
        self.diplomacy_used.insert((player, true), self.economy.month / 12);
        self.reconcile_provinces();
        Ok(target)
    }

    /// Snapshot external demand, including units in movement and battle.
    pub fn inputs(&self) -> MonthlyInputs {
        let mut army_food = vec![0.; self.actors.len()];
        let mut npc_army_food = vec![0.; self.politics.len()];
        for (province, unit) in self.military.units_with_province() {
            let payer = province
                .and_then(|id| self.economy.provinces.get(id))
                .and_then(|province| province.owner)
                .map(ForceOwner::Player)
                .unwrap_or(unit.owner);
            let demand = unit.food_demand(&self.military.config);
            match payer {
                ForceOwner::Player(player) => {
                    if let Some(food) = army_food.get_mut(player) {
                        *food += demand;
                    }
                },
                ForceOwner::Local(id)
                    if self.politics.get(id).is_some_and(|p| p.state != PoliticalState::Rome) =>
                {
                    if let Some(food) = npc_army_food.get_mut(id) {
                        *food += demand;
                    }
                },
                ForceOwner::Local(_) => {},
            }
        }
        MonthlyInputs {
            army_food,
            npc_army_food,
            player_hostility: self.wars.clone(),
            npc_hostility: self.npc_wars.clone(),
        }
    }

    /// Current provincial food output and civilian/military demand. Hosted
    /// armies are charged to the province; other armies remain a separate row.
    pub(crate) fn food_breakdown(&self, player: usize) -> Vec<(&str, f64, f64, f64)> {
        let mut sources: Vec<_> = self
            .economy
            .provinces
            .iter()
            .enumerate()
            .filter(|(_, province)| province.owner == Some(player))
            .map(|(index, province)| {
                (
                    index,
                    province.name.as_str(),
                    province.production(&self.economy.config).1[0],
                    province.food_request(&self.economy.config),
                    0.0,
                )
            })
            .collect();
        let mut assigned = 0.0;
        for (province, unit) in self.military.units_with_province() {
            let Some(province) = province else {
                continue;
            };
            let Some(state) = self.economy.provinces.get(province) else {
                continue;
            };
            if state.owner == Some(player) {
                if let Some((_, _, _, _, military)) =
                    sources.iter_mut().find(|(index, _, _, _, _)| *index == province)
                {
                    let demand = unit.food_demand(&self.military.config);
                    *military += demand;
                    assigned += demand;
                }
            }
        }
        let elsewhere =
            (self.inputs().army_food.get(player).copied().unwrap_or(0.0) - assigned).max(0.0);
        if elsewhere > 0.0 {
            sources.push((usize::MAX, "Armies elsewhere", 0.0, 0.0, elsewhere));
        }
        sources
            .into_iter()
            .map(|(_, name, produced, civilian, military)| (name, produced, civilian, military))
            .collect()
    }

    pub(crate) fn projected_food_delta(&self, player: usize, inputs: &MonthlyInputs) -> f64 {
        let local_flow: f64 = self
            .economy
            .provinces
            .iter()
            .filter(|province| province.owner == Some(player))
            .map(|province| {
                province.production(&self.economy.config).1[0]
                    - province.food_request(&self.economy.config)
            })
            .sum();
        local_flow - inputs.army_food.get(player).copied().unwrap_or(0.0)
    }

    /// Price distance from owned provinces and vassals, using the sea/land province graph.
    pub fn distance(&self, player: usize, target: usize) -> Option<usize> {
        if target >= self.economy.provinces.len() {
            return None;
        }
        let sources: Vec<_> = self
            .economy
            .provinces
            .iter()
            .enumerate()
            .filter_map(|(i, p)| {
                (p.owner == Some(player) || p.overlord == Some(player)).then_some(i)
            })
            .collect();
        // Disconnected map components still permit political and spy activity.
        // Price them at the farthest tier instead of treating them as inaccessible.
        Some(political_distance(&self.economy.adjacency, &sources, target).unwrap_or(16))
    }

    /// Movement permission separates invitations/friendship from declared invasions.
    pub fn order_army(
        &mut self,
        province: usize,
        player: usize,
        destination: usize,
        units: &[UnitId],
        route: &[ProvinceId],
        plan: BattlePlan,
        kind: ArmyOrderKind,
    ) -> Result<u64, String> {
        if player >= self.actors.len()
            || destination >= self.politics.len()
            || route.last().copied() != Some(destination)
        {
            return Err("The selected route no longer reaches the destination.".into());
        }
        if self.rome_location() == Some(destination)
            && (kind != ArmyOrderKind::Attack
                || route != [destination]
                || !self
                    .economy
                    .provinces
                    .get(province)
                    .is_some_and(|p| ROME_APPROACHES.contains(&p.name.as_str())))
        {
            return Err("Attack Rome with an army stationed in Etruria, Latium, or Samnium.".into());
        }
        if kind == ArmyOrderKind::Attack
            && self.economy.provinces[destination].owner == Some(player)
            && !self.military.province_occupied_by_enemy(destination, ForceOwner::Player(player))
        {
            return Err("Use Move for your own province.".into());
        }
        if kind == ArmyOrderKind::Pressure
            && (!matches!(self.politics[destination].state, PoliticalState::Independent { .. })
                || self.npc_wars[player][destination]
                || self.military.provinces[destination].slave_rebellion)
        {
            return Err(
                "Pressure requires an independent province at peace, without an active uprising."
                    .into(),
            );
        }
        let access = self.access_snapshot();
        let attacked_owner = self.economy.provinces[destination].owner;
        let permission = |owner: ForceOwner, id: ProvinceId| {
            if id != destination && self.rome_location() == Some(id) {
                return MilitaryAccess::Blocked;
            }
            if kind == ArmyOrderKind::Attack
                && attacked_owner.is_some()
                && attacked_owner != Some(player)
                && self.economy.provinces[id].owner == attacked_owner
            {
                return MilitaryAccess::Invasion;
            }
            if id == destination && owner == ForceOwner::Player(player) {
                match kind {
                    ArmyOrderKind::Attack => return MilitaryAccess::Invasion,
                    ArmyOrderKind::Pressure => return MilitaryAccess::Peaceful,
                    ArmyOrderKind::Move => {},
                }
            }
            if id != destination && self.military.province_in_battle(id) {
                return MilitaryAccess::Blocked;
            }
            match owner {
                ForceOwner::Player(p) => access[p][id],
                _ => MilitaryAccess::Blocked,
            }
        };
        if kind == ArmyOrderKind::Move
            && permission(ForceOwner::Player(player), destination) != MilitaryAccess::Peaceful
        {
            return Err("Peaceful movement requires permission to station troops.".into());
        }
        // Validate the selection and complete route before committing diplomatic consequences.
        let mut ordered = self.military.clone();
        let order = ordered
            .order_movement_route(
                province,
                ForceOwner::Player(player),
                units,
                Some(plan),
                route,
                &self.graph,
                permission,
            )
            .map_err(|e| e.to_string())?;
        if kind == ArmyOrderKind::Attack {
            let target = attacked_owner
                .filter(|&owner| owner != player)
                .map(ForceOwner::Player)
                .or_else(|| {
                    self.military.provinces[destination]
                        .occupation
                        .filter(|&owner| owner != ForceOwner::Player(player))
                })
                .unwrap_or(ForceOwner::Local(destination));
            ordered
                .movements
                .iter_mut()
                .find(|movement| movement.id == order)
                .unwrap()
                .attack_target = Some(target);
        }
        self.military = ordered;
        match kind {
            ArmyOrderKind::Attack => self.declare_hostility(player, destination),
            ArmyOrderKind::Pressure => {
                self.military_pressure.insert((player, destination));
            },
            ArmyOrderKind::Move => {},
        }
        Ok(order)
    }

    /// Movement permission separates invitations/friendship from declared invasions.
    pub fn access_snapshot(&self) -> Vec<Vec<MilitaryAccess>> {
        (0..self.actors.len())
            .map(|player| {
                self.politics
                    .iter()
                    .enumerate()
                    .map(|(id, p)| match p.state {
                        PoliticalState::Independent {
                            ..
                        } if self.military_pressure.contains(&(player, id))
                            && !self.npc_wars[player][id] =>
                        {
                            MilitaryAccess::Peaceful
                        },
                        PoliticalState::Rome => {
                            if self.npc_wars[player][id] {
                                MilitaryAccess::Invasion
                            } else {
                                MilitaryAccess::Blocked
                            }
                        },
                        PoliticalState::Owned {
                            owner,
                        } => {
                            if self.wars[player][owner] {
                                MilitaryAccess::Invasion
                            } else if owner == player
                                || self.province_access_granted(id, owner, player)
                            {
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
                            } else if p.relation(player)
                                >= self.military.config.npc_stationing_relation
                            {
                                MilitaryAccess::Peaceful
                            } else if p.relation(player) >= self.military.config.npc_access_relation
                            {
                                MilitaryAccess::Transit
                            } else {
                                MilitaryAccess::Blocked
                            }
                        },
                    })
                    .collect()
            })
            .collect()
    }

    /// Read the selected province's invitation, falling back to legacy bilateral access.
    pub fn province_access_granted(&self, province: usize, host: usize, guest: usize) -> bool {
        self.province_access.get(&(province, host, guest)).copied().unwrap_or_else(|| {
            self.invitations.get(host).and_then(|row| row.get(guest)).copied().unwrap_or(false)
        })
    }

    /// Change access immediately and withdraw stationed guests to the closest owned province.
    pub fn set_province_access(
        &mut self,
        province: usize,
        host: usize,
        guest: usize,
        granted: bool,
    ) -> Result<(), MilitaryError> {
        use super::campaign_notifications::{NoticeKind, NoticeSeverity};
        if guest >= self.actors.len()
            || guest == host
            || !matches!(self.politics.get(province).map(|p| &p.state), Some(PoliticalState::Owned { owner }) if *owner == host)
        {
            return Err(MilitaryError::NotDirectlyOwned);
        }
        if self.military.province_occupied_by_enemy(province, ForceOwner::Player(host)) {
            return Err(MilitaryError::Occupied);
        }
        if self.wars[host][guest] {
            return Err(MilitaryError::InvalidBattle);
        }
        if self.province_access_granted(province, host, guest) == granted {
            return Ok(());
        }
        let owner = ForceOwner::Player(guest);
        let units = self.military.provinces[province].forces.get(&owner);
        let name = &self.economy.provinces[province].name;
        let mut body = if granted {
            format!("Peaceful entry to {name} is permitted.")
        } else {
            format!("Peaceful entry to {name} is blocked.")
        };
        if !granted && units.is_some_and(|units| !units.is_empty()) {
            let ids: Vec<_> = units.unwrap().iter().map(|unit| unit.id).collect();
            // Withdrawal grants transit only to this fixed order; it never invades.
            let speed = force_speed(units.unwrap(), &self.military.config);
            let median = median_province_area(&self.graph);
            let (destination, route, _) = self
                .economy
                .provinces
                .iter()
                .enumerate()
                .filter(|(_, target)| target.owner == Some(guest))
                .filter_map(|(destination, _)| {
                    let route = fastest_route(
                        &self.graph,
                        province,
                        destination,
                        owner,
                        units.unwrap(),
                        |_, id| {
                            if self.military.province_in_battle(id) {
                                MilitaryAccess::Blocked
                            } else {
                                MilitaryAccess::Peaceful
                            }
                        },
                        &self.military.config,
                    )
                    .ok()?;
                    let mut previous = province;
                    let travel = route
                        .iter()
                        .map(|&next| {
                            let duration = edge_travel_months(
                                &self.graph[previous],
                                &self.graph[next],
                                median,
                                speed,
                                &self.military.config,
                            )
                            .max(1.)
                            .ceil();
                            previous = next;
                            duration
                        })
                        .sum::<f64>();
                    Some((destination, route, travel))
                })
                .min_by(|a, b| a.2.total_cmp(&b.2).then(a.0.cmp(&b.0)))
                .ok_or(MilitaryError::NoLegalRoute)?;
            let order = self.military.order_movement_route(
                province,
                owner,
                &ids,
                None,
                &route,
                &self.graph,
                |_, _| MilitaryAccess::Peaceful,
            )?;
            self.military
                .movements
                .iter_mut()
                .find(|movement| movement.id == order)
                .unwrap()
                .withdrawing = true;
            body.push_str(&format!(
                " Units marching to {}.",
                self.economy.provinces[destination].name
            ));
        }
        self.province_access.insert((province, host, guest), granted);
        self.notifications.province_notice(
            guest,
            province,
            self.economy.month,
            if granted {
                NoticeSeverity::Info
            } else {
                NoticeSeverity::Warning
            },
            if granted {
                NoticeKind::MilitaryAccessGranted
            } else {
                NoticeKind::MilitaryAccessRevoked
            },
            format!(
                "Military access {}",
                if granted {
                    "granted"
                } else {
                    "revoked"
                }
            ),
            body,
        );
        Ok(())
    }

    /// Declare hostility explicitly, ending peaceful military pressure.
    pub fn declare_hostility(&mut self, player: usize, province: usize) {
        self.military_pressure.remove(&(player, province));
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
        use crate::game::politics::espionage::{ScandalKind, Severity};
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
                self.province_access.retain(|&(_, host, guest), _| {
                    !((host == player && guest == owner) || (host == owner && guest == player))
                });
            },
            PoliticalState::Owned {
                ..
            } => return,
            _ => {
                self.npc_wars[player][province] = true;
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
        let local_response = !self.npc_wars[player][province]
            && matches!(
                self.politics[province].state,
                PoliticalState::Independent { .. } | PoliticalState::Vassal { .. }
            )
            && self.politics[province].relation(player)
                < self.military.config.npc_hostility_relation
            && (self.military.provinces[province]
                .forces
                .get(&ForceOwner::Local(province))
                .is_some_and(|units| units.iter().any(|unit| unit.current_manpower > 0.))
                || self.military.battles.iter().any(|battle| {
                    battle.province == province
                        && battle.attackers.units.iter().chain(&battle.defenders.units).any(
                            |unit| {
                                unit.owner == ForceOwner::Local(province)
                                    && unit.current_manpower > 0.
                            },
                        )
                }));
        if local_response {
            self.npc_wars[player][province] = true;
            self.military_pressure.remove(&(player, province));
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
        let hostile_territory = match self.politics[province].state {
            PoliticalState::Owned {
                owner,
            } => owner != player && self.wars[player][owner],
            PoliticalState::Rome
            | PoliticalState::Independent {
                ..
            }
            | PoliticalState::Vassal {
                ..
            } => self.npc_wars[player][province],
        };
        if hostile_territory
            && !local_response
            && self.military.provinces[province].occupation != Some(attacker)
        {
            if self.politics[province].relation(player) > 10.0 {
                self.senate.record_unjustified_attack(player, &self.senate_config);
            }
            self.politics[province].change_relation(player, -30.0);
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
                self.military.provinces[province].occupation = Some(attacker);
                self.notifications.province_notice(
                    player,
                    province,
                    self.economy.month,
                    super::campaign_notifications::NoticeSeverity::Info,
                    super::campaign_notifications::NoticeKind::OccupationEstablished,
                    "Occupation established",
                    format!("Your troops now occupy {} and build Control.", p.name),
                );
            } else if p.owner.is_none() && self.npc_wars[player][province] {
                self.military.provinces[province].occupation = Some(attacker);
                if self.politics[province].state == PoliticalState::Rome {
                    self.conquer_rome(player, province);
                }
            }
            return;
        }
        // A local uprising attacks the province's own garrison. The owner
        // defends and retains the benefit of its walls.
        let uprising = p.owner == Some(player)
            && defenders.len() == 1
            && defenders[0] == ForceOwner::Local(province);
        let (attacking, defending) = if uprising || local_response {
            (defenders.as_slice(), &[attacker][..])
        } else {
            (&[attacker][..], defenders.as_slice())
        };
        let started = self.military.start_battle(
            province,
            attacking,
            defending,
            p.owner,
            origin,
            self.graph[province].terrain,
            if local_response {
                0
            } else {
                p.level(BuildingType::CityWalls)
            },
            u64::from(self.economy.month) * 179 + province as u64 + 1,
        );
        if let Ok(battle_id) = started {
            if self.politics[province].state == PoliticalState::Rome {
                if let Some(battle) =
                    self.military.battles.iter_mut().find(|battle| battle.id == battle_id)
                {
                    battle.protect_rome_defenders();
                }
            }
            self.notify_battle_started(province);
        }
    }

    /// Synchronize legal ownership before economic calculations and after political transitions.
    pub fn reconcile_provinces(&mut self) {
        let previously_owned: Vec<_> = (0..self.actors.len())
            .map(|player| self.economy.provinces.iter().position(|p| p.owner == Some(player)))
            .collect();
        self.province_access.retain(|&(province, host, _), _| {
            matches!(self.politics.get(province).map(|p| &p.state), Some(PoliticalState::Owned { owner }) if *owner == host)
        });
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
        for (id, state) in self.military.provinces.iter_mut().enumerate() {
            if let Some(occupier) = state.occupation {
                let stationed = state
                    .forces
                    .get(&occupier)
                    .is_some_and(|units| units.iter().any(|unit| unit.current_manpower > 0.));
                let fighting = self.military.battles.iter().any(|battle| {
                    battle.province == id
                        && (battle.attackers.plans.contains_key(&occupier)
                            || battle.defenders.plans.contains_key(&occupier))
                });
                if !stationed && !fighting {
                    state.occupation = None;
                }
            }
            if let Some(ForceOwner::Player(occupier)) = state.occupation {
                if self.politics.get(id).is_some_and(|politics| {
                    matches!(politics.state,
                        PoliticalState::Owned { owner } if owner == occupier)
                        || matches!(politics.state,
                            PoliticalState::Vassal { overlord, .. } if overlord == occupier)
                }) {
                    state.occupation = None;
                }
            }
            self.economy.provinces[id].occupied =
                self.economy.provinces[id].owner.is_some_and(|owner| {
                    state.occupation.is_some_and(|occupier| occupier != ForceOwner::Player(owner))
                });
        }
        // Relation changes can happen in UI actions or mid-month. Engage on this
        // frame's reconciliation, without waiting for the next monthly boundary.
        let visitors: Vec<_> = self
            .military
            .provinces
            .iter()
            .enumerate()
            .flat_map(|(id, state)| {
                state.forces.keys().filter_map(move |&owner| match owner {
                    ForceOwner::Player(player) => Some((id, player)),
                    ForceOwner::Local(_) => None,
                })
            })
            .filter(|&(id, player)| {
                matches!(
                    self.politics[id].state,
                    PoliticalState::Independent { .. } | PoliticalState::Vassal { .. }
                ) && !self.npc_wars[player][id]
                    && self.politics[id].relation(player)
                        < self.military.config.npc_hostility_relation
            })
            .collect();
        if self.active && self.senate.winner.is_none() {
            for (province, player) in visitors {
                self.begin_encounter(province, ForceOwner::Player(player), None);
            }
        }
        let inputs = self.inputs();
        self.economy.suspend_blocked_trade_routes(&inputs);
        if ownership_changed {
            self.economy.recalculate_storage();
            for wallet in &mut self.economy.players {
                wallet.clamp_storage();
            }
        }
        for (player, &former_province) in previously_owned.iter().enumerate() {
            let Some(former_province) = former_province else {
                continue;
            };
            if !self.defeated.get(player).copied().unwrap_or(false)
                && !self.economy.provinces.iter().any(|p| p.owner == Some(player))
            {
                self.defeated[player] = true;
                self.notifications.province_notice(
                    player,
                    former_province,
                    self.economy.month,
                    super::campaign_notifications::NoticeSeverity::Warning,
                    super::campaign_notifications::NoticeKind::PlayerDefeated,
                    "Campaign lost",
                    "You lost your last owned province.",
                );
            }
        }
        self.sync_governance();
    }

    fn recruitment_budget(&self, province: usize) -> Option<(usize, f64)> {
        let economic = self.economy.provinces.get(province)?;
        let owner = economic.owner?;
        if economic.occupied {
            return None;
        }
        let project = self.military.provinces.get(province)?.recruitment.as_ref()?;
        if project.owner != ForceOwner::Player(owner) {
            return None;
        }
        Some((
            owner,
            self.economy.config.recruitment_coin_per_project
                [economic.policies.recruitment as usize]
                .max(0.0),
        ))
    }

    pub(crate) fn recruitment_effort_cost(&self, player: usize) -> f64 {
        (0..self.economy.provinces.len())
            .filter_map(|id| self.recruitment_budget(id))
            .filter(|(owner, _)| *owner == player)
            .map(|(_, cost)| cost)
            .sum()
    }

    /// Pay active projects proportionally before cohorts complete and military food is calculated.
    fn pay_recruitment_effort(&mut self) -> Vec<f64> {
        let budgets: Vec<_> =
            (0..self.economy.provinces.len()).map(|id| self.recruitment_budget(id)).collect();
        let mut totals = vec![0.0; self.economy.players.len()];
        for &(owner, cost) in budgets.iter().flatten() {
            if let Some(total) = totals.get_mut(owner) {
                *total += cost;
            }
        }
        let coverage: Vec<_> = self
            .economy
            .players
            .iter_mut()
            .zip(totals)
            .map(|(wallet, requested)| {
                let paid = wallet.coin.max(0.0).min(requested);
                wallet.coin -= paid;
                if requested > 0.0 {
                    paid / requested
                } else {
                    1.0
                }
            })
            .collect();
        self.economy
            .provinces
            .iter_mut()
            .zip(budgets)
            .map(|(province, budget)| {
                province.recruitment_happiness = 0.0;
                if province.occupied {
                    return 0.0;
                }
                let Some((owner, cost)) = budget else {
                    return 1.0;
                };
                let funded = if cost > 0.0 {
                    coverage.get(owner).copied().unwrap_or(0.0)
                } else {
                    1.0
                };
                let effort = province.policies.recruitment as usize;
                province.recruitment_happiness =
                    self.economy.config.recruitment_happiness[effort] * funded;
                self.economy.config.recruitment_speed[effort].max(0.0) * funded
            })
            .collect()
    }

    /// A crisis below five happiness has a growing monthly chance to become an uprising.
    fn resolve_slave_revolts(&mut self) {
        let population_cost = self.military.config.unit(UnitType::LightInfantry).population_cost;
        if !population_cost.is_finite() || population_cost <= 0.0 {
            return;
        }
        for province in 0..self.economy.provinces.len() {
            let p = &self.economy.provinces[province];
            let Some(owner) = p.owner else {
                continue;
            };
            let happiness = p.happiness[3];
            if happiness > 5.0
                || p.population[3] < population_cost
                || self.military.provinces[province]
                    .forces
                    .get(&ForceOwner::Local(province))
                    .is_some_and(|units| !units.is_empty())
            {
                continue;
            }
            let [at_five, at_zero] = self.economy.config.slave_revolt_chance;
            let chance = (at_five + (at_zero - at_five) * (5.0 - happiness).clamp(0.0, 5.0) / 5.0)
                .clamp(0.0, 1.0);
            if rand::random::<f64>() >= chance {
                continue;
            }
            self.start_slave_revolt(province, owner);
        }
    }

    /// Every slave leaves the economy, while only the mobilized share becomes infantry.
    fn start_slave_revolt(&mut self, province: usize, owner: usize) {
        use super::campaign_notifications::{NoticeKind, NoticeSeverity};
        let lost = self.economy.provinces[province].population[3];
        let mobilized =
            lost * self.military.config.slave_revolt_mobilization_fraction.clamp(0.0, 1.0);
        let Ok(cohorts) = self.military.seed_population_force(
            province,
            ForceOwner::Local(province),
            UnitType::LightInfantry,
            mobilized,
        ) else {
            return;
        };
        self.military.provinces[province].slave_rebellion = true;
        self.economy.provinces[province].population[3] = 0.0;
        self.economy.provinces[province].validate_slave_assignment();
        if let Some(summary) = self.economy.last_report.province_reports.get_mut(province) {
            summary.population_delta -= lost;
            summary.slave_revolt_loss += lost;
        }
        self.npc_wars[owner][province] = true;
        self.begin_encounter(province, ForceOwner::Player(owner), None);
        // If the garrison is already fighting, join the opposing coalition when
        // its existing members are not also hostile to the rebels.
        let rebel = ForceOwner::Local(province);
        let join = self.military.battles.iter().find_map(|battle| {
            if battle.province != province
                || self.military.provinces[province].forces.get(&rebel).is_none_or(Vec::is_empty)
            {
                return None;
            }
            let player = ForceOwner::Player(owner);
            let attacking = if battle.defenders.plans.contains_key(&player) {
                true
            } else if battle.attackers.plans.contains_key(&player) {
                false
            } else {
                return None;
            };
            let allies = if attacking {
                &battle.attackers
            } else {
                &battle.defenders
            };
            allies.plans.keys().all(|&ally| !self.forces_hostile(rebel, ally)).then_some(attacking)
        });
        if let Some(attacking) = join {
            let _ = self.military.join_battle(province, rebel, attacking);
        }
        let name = self.economy.provinces[province].name.clone();
        let fighting = self.military.battles.iter().any(|battle| {
            battle.province == province
                && (battle.attackers.plans.contains_key(&rebel)
                    || battle.defenders.plans.contains_key(&rebel))
        });
        let situation = if fighting {
            "Rebels are fighting your army.".to_owned()
        } else {
            format!(
                "Unopposed rebels cost you {:.2} Control per month.",
                self.unopposed_rebellion_control_loss(province),
            )
        };
        self.notifications.province_notice(
            owner, province, self.economy.month, NoticeSeverity::Warning,
            NoticeKind::SlaveRevolt, format!("Slave revolt in {name}"),
            format!("All {lost:.0} slaves revolted, forming {cohorts} light infantry cohorts. {situation}"),
        );
    }

    /// Surviving local rebels erode Control only when the owner's troops are absent.
    fn unopposed_rebellion_control_loss(&self, province: usize) -> f64 {
        if !self.military.provinces[province].slave_rebellion {
            return 0.0;
        }
        let PoliticalState::Owned {
            owner,
        } = self.politics[province].state
        else {
            return 0.0;
        };
        let local_units =
            self.military.provinces[province].forces.values().flatten().chain(
                self.military.battles.iter().filter(|battle| battle.province == province).flat_map(
                    |battle| battle.attackers.units.iter().chain(&battle.defenders.units),
                ),
            );
        let mut rebel_manpower = 0.0;
        for unit in local_units {
            if unit.current_manpower <= 0.0 || unit.morale <= 0.0 {
                continue;
            }
            if unit.owner == ForceOwner::Player(owner) {
                return 0.0;
            }
            if unit.owner == ForceOwner::Local(province) {
                rebel_manpower += unit.current_manpower;
            }
        }
        let config = &self.military.config;
        (rebel_manpower / config.unit(UnitType::LightInfantry).manpower.max(0.001)
            * config.rebellion_control_loss_per_cohort.max(0.0))
        .min(config.maximum_rebellion_control_loss.max(0.0))
    }

    /// Apply rebellion pressure once at the monthly political boundary, after combat and arrivals.
    fn resolve_rebellion_control(&mut self) {
        use super::campaign_notifications::{NoticeKind, NoticeSeverity};
        for province in 0..self.politics.len() {
            let loss = self.unopposed_rebellion_control_loss(province);
            if loss <= 0.0 {
                continue;
            }
            let PoliticalState::Owned {
                owner,
            } = self.politics[province].state
            else {
                continue;
            };
            let _ = self.politics[province].apply_rebellion_control_loss(loss);
            if matches!(self.politics[province].state, PoliticalState::Independent { .. }) {
                self.notifications.province_notice(
                    owner,
                    province,
                    self.economy.month,
                    NoticeSeverity::Warning,
                    NoticeKind::OwnedControlThreatened,
                    format!("{} became independent", self.economy.provinces[province].name),
                    "Rebels reduced your Control to zero.",
                );
            }
        }
    }

    /// Execute one month exactly once, with shared supply and simultaneous political pressure.
    #[cfg(test)]
    pub fn advance_month(&mut self) {
        self.resolve_month(true);
    }
    /// Resolve the economic and movement boundary on the live clock. Battles
    /// advance separately, allowing new arrivals to be inspected before combat.
    pub fn advance_live_month(&mut self) {
        self.resolve_month(false);
    }
    /// Resolve one combat round, applying victory and Senate effects immediately.
    pub fn advance_live_combat(&mut self) {
        if !self.active || self.senate.winner.is_some() {
            return;
        }
        let access = self.access_snapshot();
        let permission = |owner: ForceOwner, id: usize| match owner {
            ForceOwner::Player(p) => access[p][id],
            ForceOwner::Local(home) if home == id => MilitaryAccess::Peaceful,
            ForceOwner::Local(_) => MilitaryAccess::Blocked,
        };
        let events = self.military.advance_battle_rounds(&self.graph, permission);
        let battle_ended =
            events.iter().any(|event| matches!(event, MilitaryEvent::BattleEnded { .. }));
        self.integrate_military_events(events);
        self.report_zero_morale_disbands();
        self.reconcile_provinces();
        if battle_ended {
            self.refresh_profiles();
            self.notify_rank_opportunities();
        }
    }
    fn integrate_military_events(&mut self, events: Vec<MilitaryEvent>) {
        for event in events {
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
                                self.senate.record_battle_result(*p, won);
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
                    && self.politics[province].state == PoliticalState::Rome
                    && self.military.provinces[province].occupation
                        == Some(ForceOwner::Player(player))
                {
                    self.conquer_rome(player, province);
                }
                let _ = previous_owner;
            }
        }
    }
    fn resolve_month(&mut self, resolve_combat: bool) {
        if !self.active || self.senate.winner.is_some() {
            return;
        }
        let notification_before = self.notification_snapshot();
        self.reconcile_provinces();
        self.report_zero_morale_disbands();
        self.reconcile_provinces();
        let previous_occupations: Vec<_> =
            self.military.provinces.iter().map(|p| p.occupation).collect();
        let before: Vec<_> = self.economy.players.iter().map(PlayerEconomy::balances).collect();
        // Complete paid cohorts before calculating any military food requests.
        let recruitment_speeds = self.pay_recruitment_effort();
        let owners: Vec<_> = self.economy.provinces.iter().map(|p| p.owner).collect();
        let mut military_events =
            self.military.advance_recruitment_with_speed(|p| owners[p], |p| recruitment_speeds[p]);
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
        for payment in self.senate.pay_outreach(&mut self.actors, &self.senate_config) {
            use crate::game::politics::espionage::{ScandalKind, Severity};
            self.espionage.record_action(
                payment.player,
                None,
                ScandalKind::PoliticalBribery,
                Severity::Medium,
                self.senate.month,
                &self.espionage_config,
            );
            if payment.caught {
                use super::campaign_notifications::{
                    CampaignNotice, NoticeAction, NoticeKind, NoticeSeverity,
                };
                self.notifications.push(CampaignNotice {
                    id: 0,
                    recipient: payment.player,
                    severity: NoticeSeverity::Warning,
                    title: "Senator bribery exposed".into(),
                    body: format!(
                        "Your bribe to Senator {} ended after exposure. Faction confidence fell.",
                        payment.senator + 1,
                    ),
                    kind: NoticeKind::SenatorBriberyExposed,
                    province: None,
                    building: None,
                    wonder: None,
                    scandal: None,
                    month: self.economy.month,
                    action: NoticeAction::OpenSenate,
                });
            }
        }
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
        self.sync_owned_relations();
        self.record_economic_events(&report);
        self.resolve_slave_revolts();
        let morale_before_wages: std::collections::BTreeMap<_, _> =
            self.military.all_units().map(|unit| (unit.id, unit.morale)).collect();
        for (p, &supply) in report.food_supply_ratio.iter().enumerate() {
            self.pay_noble_wages(p);
            self.pay_army_wages(p, supply);
        }
        for (id, p) in report.province_reports.iter().enumerate() {
            self.military.apply_supply_with_provisioning(
                ForceOwner::Local(id),
                |province| {
                    province
                        .and_then(|id| self.economy.provinces.get(id))
                        .and_then(|province| province.owner)
                        .map_or_else(
                            || {
                                if self.politics[id].state == PoliticalState::Rome {
                                    1.0
                                } else {
                                    p.food_supply_ratio
                                }
                            },
                            |host| report.food_supply_ratio[host],
                        )
                },
                0.0,
            );
        }
        self.apply_insolvency(&report, &morale_before_wages);
        self.report_zero_morale_disbands();
        if resolve_combat {
            military_events.extend(self.military.advance_battles(&self.graph, permission));
        }
        self.report_zero_morale_disbands();
        self.integrate_military_events(military_events);
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
        // A fulfilled, approved player route improves sentiment in each party's
        // owned provinces toward the trading partner. It grants no Control.
        for trade in &self.economy.trades {
            let (TradeParty::Player(a), TradeParty::Player(b)) = (trade.party_a, trade.party_b)
            else {
                continue;
            };
            if trade.frequency != TradeFrequency::Monthly
                || trade.last_executed_month != Some(self.economy.month)
            {
                continue;
            }
            let gain = (trade.last_delivered_value / self.economy.config.trade.value_per_relation)
                .min(self.economy.config.trade.relation_cap);
            for (politics, province) in self.politics.iter_mut().zip(&self.economy.provinces) {
                match province.owner {
                    Some(owner) if owner == a => politics.change_relation(b, gain),
                    Some(owner) if owner == b => politics.change_relation(a, gain),
                    _ => {},
                }
            }
        }
        // Surviving spy pressure joins this month's simultaneous political pool.
        self.advance_espionage();
        self.sync_owned_relations();
        self.reconcile_provinces();
        self.military_pressure.retain(|&(player, province)| {
            matches!(self.politics[province].state, PoliticalState::Independent { .. })
                && !self.npc_wars[player][province]
                && (self.military.provinces[province]
                    .forces
                    .get(&ForceOwner::Player(player))
                    .is_some_and(|u| !u.is_empty())
                    || self.military.movements.iter().any(|m| {
                        m.owner == ForceOwner::Player(player) && m.route.last() == Some(&province)
                    }))
        });
        for (id, previous_occupation) in previous_occupations.iter().enumerate() {
            let mut power: Vec<_> = (0..self.actors.len())
                .map(|p| {
                    if self.military.province_in_battle(id) {
                        return 0.0;
                    }
                    let owner = ForceOwner::Player(p);
                    self.military.stationed_strength(id, owner)
                        * self.military.config.rank_control[self.military.rank(owner) as usize]
                })
                .collect();
            for (player, strength) in power.iter_mut().enumerate() {
                if self.military_pressure.contains(&(player, id)) && *strength > 0. {
                    let local_strength =
                        self.military.stationed_strength(id, ForceOwner::Local(id));
                    let control = self.politics[id].control(player);
                    let gain = (self.military.config.pressure_monthly_control * *strength
                        / (*strength
                            + local_strength
                            + self.military.config.garrison_half_saturation))
                        .min((self.military.config.pressure_control_ceiling - control).max(0.));
                    if gain > 0. {
                        let _ = self.politics[id].queue_control_gain(player, gain);
                    }
                    // Pressure replaces the invited-garrison gain, so its ceiling cannot be bypassed.
                    *strength = 0.;
                }
            }
            // New occupation first generates Control on the following month's political tick.
            let occupation = match self.military.provinces[id].occupation {
                Some(ForceOwner::Player(p))
                    if *previous_occupation == Some(ForceOwner::Player(p)) =>
                {
                    Some(p)
                },
                _ => None,
            };
            if let Some(ForceOwner::Player(new_occupier)) = self.military.provinces[id].occupation {
                if *previous_occupation != Some(ForceOwner::Player(new_occupier)) {
                    power[new_occupier] = 0.0;
                }
            }
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
        self.resolve_rebellion_control();
        self.push_wallets();
        self.reconcile_provinces();
        self.pull_wallets();
        for (player, actor) in self.actors.iter_mut().enumerate() {
            let military_rank = self.military.rank(ForceOwner::Player(player));
            actor.influence += self.senate_config.rank_income(actor.rank)
                + self.military.config.rank_influence[military_rank as usize];
        }
        self.push_wallets();
        // Senate income must use the same settled change as the HUD, including
        // wages, spy/recruitment upkeep, recurring support and vassal tribute.
        for (p, wallet) in self.economy.players.iter().enumerate() {
            if let Some(delta) = self.economy.last_report.player_delta.get_mut(p) {
                *delta = std::array::from_fn(|r| wallet.balances()[r] - before[p][r]);
            }
        }
        self.refresh_profiles();
        self.senate.advance_month(&self.actors, &self.profiles, &self.senate_config);
        self.push_wallets();
        for victory in &mut self.recent_victories {
            *victory *= 0.95;
        }
        self.record_monthly_notifications(notification_before);
    }

    /// Insolvency requires both an empty wallet and recurring outflow above income.
    /// Resolve flight before ordinary monthly spy maintenance and detection.
    fn apply_insolvency(
        &mut self,
        report: &MonthlyReport,
        morale_before_wages: &std::collections::BTreeMap<u64, f64>,
    ) {
        const NOBLE_LOSS: f64 = 3.0;
        const MORALE_LOSS: f64 = 2.0;
        for player in 0..self.economy.players.len() {
            let taxes: f64 = report
                .province_reports
                .iter()
                .enumerate()
                .filter(|(id, _)| self.economy.provinces[*id].owner == Some(player))
                .map(|(_, province)| province.tax_income)
                .sum();
            let tributes: f64 = self
                .politics
                .iter()
                .enumerate()
                .map(|(id, politics)| {
                    let market = &self.economy.provinces[id].market;
                    politics
                        .tribute_due(market.monthly_coin_income.min(market.coin_treasury))
                        .filter(|(owner, _)| *owner == player)
                        .map_or(0.0, |(_, amount)| amount)
                })
                .sum();
            let civic: f64 = self
                .economy
                .provinces
                .iter()
                .filter(|province| province.owner == Some(player))
                .map(|province| province.civic_spending_cost(&self.economy.config))
                .sum();
            let outflow = self.noble_wages(player)
                + self.army_wages(player)
                + civic
                + self.recruitment_effort_cost(player)
                + self.spy_upkeep(player);
            let insolvent = self.economy.players[player].coin <= 0.0 && outflow > taxes + tributes;
            let newly_insolvent = insolvent
                && self.economy.provinces.iter().any(|province| {
                    province.owner == Some(player) && province.insolvency_unhappiness == 0.0
                });
            for (id, province) in self.economy.provinces.iter_mut().enumerate() {
                if province.owner == Some(player) {
                    let old = province.insolvency_unhappiness;
                    province.insolvency_unhappiness = if insolvent {
                        (old + NOBLE_LOSS).min(100.0)
                    } else {
                        0.0
                    };
                    let before = province.happiness[0];
                    province.happiness[0] = province.calculate_happiness(&self.economy.config)[0];
                    if let Some(summary) = self.economy.last_report.province_reports.get_mut(id) {
                        summary.happiness_delta[0] += province.happiness[0] - before;
                    }
                }
            }
            if !insolvent {
                continue;
            }
            if newly_insolvent {
                use super::campaign_notifications::{
                    CampaignNotice, NoticeAction, NoticeKind, NoticeSeverity,
                };
                self.notifications.push(CampaignNotice {
                    id: 0, recipient: player, severity: NoticeSeverity::Warning,
                    title: "Treasury exhausted".into(),
                    body: "Spending exceeds income. Spies flee, noble Happiness drops, and army Morale falls.".into(),
                    kind: NoticeKind::TreasuryExhausted, province: None,
                    building: None, wonder: None, scandal: None,
                    month: self.economy.month,
                    action: self.economy.provinces.iter().position(|province| province.owner == Some(player))
                        .map_or(NoticeAction::OpenSenate, NoticeAction::OpenProvince),
                });
            }
            self.military.reduce_morale(
                ForceOwner::Player(player),
                MORALE_LOSS,
                Some(morale_before_wages),
            );
            let missions: Vec<_> = self
                .espionage
                .missions
                .iter()
                .filter(|mission| mission.owner == player)
                .map(|mission| mission.province)
                .collect();
            for province in missions {
                let _ = self.flee_spy(player, province);
            }
        }
    }

    fn report_zero_morale_disbands(&mut self) {
        use super::campaign_notifications::{
            CampaignNotice, NoticeAction, NoticeKind, NoticeSeverity,
        };
        let mut counts = std::collections::BTreeMap::new();
        for (province, unit) in self.military.disband_zero_morale() {
            if let (Some(id), ForceOwner::Player(player)) = (province, unit.owner) {
                if let Some(home) =
                    self.economy.provinces.get_mut(id).filter(|home| home.owner == Some(player))
                {
                    let definition = self.military.config.unit(unit.unit_type);
                    let returned = definition.population_cost * unit.manpower_ratio();
                    home.population[definition.manpower_class] += returned;
                    if let Some(report) = self.economy.last_report.province_reports.get_mut(id) {
                        report.population_delta += returned;
                    }
                }
            }
            *counts.entry(unit.owner).or_insert(0usize) += 1;
        }
        for (owner, count) in counts {
            let ForceOwner::Player(player) = owner else {
                continue;
            };
            self.notifications.push(CampaignNotice {
                id: 0,
                recipient: player,
                severity: NoticeSeverity::Warning,
                title: "Army disbanded".into(),
                body: format!("{count} cohorts disbanded after their morale reached zero."),
                kind: NoticeKind::ArmyDisbanded,
                province: None,
                building: None,
                wonder: None,
                scandal: None,
                month: self.economy.month,
                action: self
                    .economy
                    .provinces
                    .iter()
                    .position(|province| province.owner == Some(player))
                    .map_or(NoticeAction::OpenSenate, NoticeAction::OpenProvince),
            });
        }
    }

    /// Military conquest is the capital's only ownership transition and wins at once.
    fn conquer_rome(&mut self, player: usize, province: usize) {
        if self.senate.winner.is_some() {
            return;
        }
        self.politics[province].state = PoliticalState::Owned {
            owner: player,
        };
        self.actors[player].rank = crate::game::politics::PoliticalRank::Augustus;
        self.senate.winner = Some(player);
        for recipient in 0..self.actors.len() {
            self.notifications.province_notice(
                recipient,
                province,
                self.economy.month,
                super::campaign_notifications::NoticeSeverity::Info,
                super::campaign_notifications::NoticeKind::AugustusVictory,
                "Rome conquered",
                format!("Player {} captured Rome and won the campaign.", player + 1),
            );
        }
    }

    /// Route economic outcomes only to affected players, including failures hidden by capped stocks.
    fn record_economic_events(&mut self, report: &MonthlyReport) {
        use super::campaign_notifications::{
            CampaignNotice, NoticeAction, NoticeKind, NoticeSeverity,
        };
        let mut failed_trades = vec![Vec::<u64>::new(); self.actors.len()];
        for event in &report.events {
            match event {
                EconomyEvent::BuildingCompleted {
                    province,
                    building,
                    level,
                } => {
                    if let Some(owner) = self.economy.provinces[*province].owner {
                        self.notifications.push(CampaignNotice {
                            id: 0,
                            recipient: owner,
                            province: Some(*province),
                            building: Some(*building),
                            wonder: None,
                            scandal: None,
                            month: report.month,
                            severity: NoticeSeverity::Info,
                            kind: NoticeKind::BuildingCompleted,
                            title: format!("{} completed", building.name()),
                            body: format!(
                                "Level {level} finished in {}.",
                                self.economy.provinces[*province].name
                            ),
                            action: NoticeAction::OpenProvince(*province),
                        });
                    }
                },
                EconomyEvent::WonderCompleted {
                    province,
                    wonder,
                    owner: Some(owner),
                } => {
                    self.notifications.push(CampaignNotice {
                        id: 0,
                        recipient: *owner,
                        province: Some(*province),
                        building: None,
                        wonder: Some(*wonder),
                        scandal: None,
                        month: report.month,
                        severity: NoticeSeverity::Info,
                        kind: NoticeKind::WonderCompleted,
                        title: "Wonder completed".into(),
                        body: format!(
                            "{} now produces Influence.",
                            crate::map::wonder_name(*wonder).unwrap_or("Monument")
                        ),
                        action: NoticeAction::OpenProvince(*province),
                    });
                },
                // Continuous shortages are announced once below, and rearm after recovery.
                EconomyEvent::FoodShortage {
                    ..
                } => {},
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
                                failed_trades[player].push(*agreement);
                            }
                        }
                    }
                },
                EconomyEvent::TradeNoticeCompleted {
                    agreement,
                } => {
                    if let Some(trade) = self.economy.trades.iter().find(|t| t.id == *agreement) {
                        if let (TradeParty::Player(player), TradeParty::Npc(province)) =
                            (trade.party_a, trade.party_b)
                        {
                            self.notifications.province_notice(
                                player,
                                province,
                                report.month,
                                NoticeSeverity::Info,
                                NoticeKind::TradeInterrupted,
                                "Trade route ended",
                                format!("Route #{agreement} ended without a relation penalty."),
                            );
                        }
                    }
                },
                _ => {},
            }
        }
        for (player, &supplied) in report.food_supply_ratio.iter().enumerate() {
            if self.notifications.food_shortage_started(player, supplied) {
                if let Some(province) =
                    self.economy.provinces.iter().position(|p| p.owner == Some(player))
                {
                    self.notifications.province_notice(
                        player,
                        province,
                        report.month,
                        NoticeSeverity::Warning,
                        NoticeKind::FoodShortage,
                        "Food shortage",
                        format!("Only {:.0}% of food demand was met.", supplied * 100.0),
                    );
                }
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
                        if failures.len() == 1 {
                            format!("Agreement #{} was interrupted.", failures[0])
                        } else {
                            format!("{} trade agreements were interrupted.", failures.len())
                        },
                    );
                }
            }
        }
    }

    /// Derive live Senate attitudes from the same public facts displayed in province panels.
    pub fn refresh_profiles(&mut self) {
        let inputs = self.inputs();
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
                    [t.party_a, t.party_b].into_iter().any(|party| {
                        if let TradeParty::Npc(province) = party {
                            self.economy.provinces.get(province).is_some_and(|p| !p.has_city)
                        } else {
                            false
                        }
                    })
                })
                .map(delivered)
                .sum();
            let wallet = &self.economy.players[player];
            let needs = [
                owned.iter().map(|p| p.food_request(&self.economy.config)).sum::<f64>()
                    + inputs.army_food[player],
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
            let other_provinces: Vec<_> = self
                .politics
                .iter()
                .filter(|province| {
                    province.state != PoliticalState::Rome
                        && !matches!(province.state, PoliticalState::Owned { owner } if owner == player)
                })
                .collect();
            let mut profile = PoliticalProfile {
                nobles: counts[0],
                noble_happiness: happy[0],
                citizen_happiness: happy[1],
                plebeian_happiness: happy[2],
                vassal_count: vassals.len() as f64,
                province_relation: if other_provinces.is_empty() {
                    50.0
                } else {
                    other_provinces.iter().map(|p| p.relation(player)).sum::<f64>()
                        / other_provinces.len() as f64
                },
                food_security: food,
                food_reserve_months: if !owned.is_empty() && needs[0] > 0.0 {
                    wallet.resources[0].max(0.0) / needs[0]
                } else {
                    0.0
                },
                famine: 1.0 - food,
                coin_income: self
                    .economy
                    .last_report
                    .player_delta
                    .get(player)
                    .map_or(0.0, |d| d[3]),
                trade_volume,
                active_trade_routes: recurring
                    .iter()
                    .filter(|t| {
                        t.status == TradeStatus::Active
                            && t.last_executed_month == Some(self.economy.month)
                            && t.last_fulfillment >= 0.99
                            && t.last_delivered_value > 0.0
                    })
                    .count() as f64,
                active_wars: self.wars[player].iter().filter(|war| **war).count() as f64
                    + self.npc_wars[player].iter().filter(|war| **war).count() as f64,
                controlled_provinces: self
                    .politics
                    .iter()
                    .map(|province| match province.state {
                        PoliticalState::Owned {
                            owner,
                        } if owner == player => 1.,
                        PoliticalState::Vassal {
                            overlord,
                            control,
                            ..
                        } if overlord == player => control / 100.,
                        PoliticalState::Independent {
                            ..
                        } => province.control(player) / 100.,
                        _ => 0.,
                    })
                    .sum(),
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
