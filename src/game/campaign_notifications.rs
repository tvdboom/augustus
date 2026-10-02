//! Player-scoped, clickable political notifications with monthly aggregation and history.

use super::campaign::Campaign;
use crate::game::economy::{BuildingType, ConstructionProject};
use crate::game::military::{
    BattleResult, ForceOwner, MilitaryAccess, MilitaryEvent, MilitaryRank,
};
use crate::game::politics::diplomacy::PoliticalState;
use crate::game::politics::senate::SenateEvent;
use crate::game::politics::PoliticalRank;

/// How prominently a campaign event should be displayed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum NoticeSeverity {
    /// An opportunity or newly acquired information.
    Info,
    /// A hostile event or weakening political position.
    Warning,
}

/// Domain event identity used for aggregation, auditing and future networking.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum NoticeKind {
    /// A player paid Influence with sufficient loyal senators to gain an office.
    SenateOfficeAppointed,
    /// All political promotion requirements are currently satisfied.
    PoliticalPromotionAvailable,
    /// An active Consul's term expired.
    ConsulTermExpired,
    /// Loss of Senate confidence forced an incumbent to resign.
    ConsulRemoved,
    /// Senate support or conquest of Rome produced a campaign victor.
    AugustusVictory,
    /// A player lost their last directly owned province.
    PlayerDefeated,
    /// A paid cohort completed recruitment.
    RecruitmentCompleted,
    /// A player disbanded their whole stationary army.
    ArmyDisbanded,
    /// Whole cohorts were permanently destroyed in combat or a failed retreat.
    UnitsDestroyed,
    /// Foreign units arrived in owned or vassal territory with peaceful access.
    ForeignArrival,
    /// An invasion or hostile encounter began.
    InvasionBegins,
    /// A battle reached a permanent outcome.
    BattleResolved,
    /// The local NPC defending coalition was defeated.
    NpcDefeated,
    /// A player established hostile military occupation.
    OccupationEstablished,
    /// Peaceful military access became available.
    MilitaryAccessGranted,
    /// Previously available peaceful military access ended.
    MilitaryAccessRevoked,
    /// A changed route permission stopped a movement order.
    MilitaryMovementStopped,
    /// A vassal lost a material part of its military Control support.
    GarrisonWeakened,
    /// A player earned and paid for a separate military rank.
    MilitaryRankIncreased,
    /// All military promotion requirements are currently satisfied.
    MilitaryPromotionAvailable,
    /// A directly owned ordinary building completed.
    BuildingCompleted,
    /// Civilian and military demand exceeded the owner's global Food supply.
    FoodShortage,
    /// An empty treasury cannot cover recurring outflow.
    TreasuryExhausted,
    /// A player's class happiness fell below its warning threshold.
    PopulationUnhappy(usize),
    /// Enslaved residents left the province and formed hostile infantry.
    SlaveRevolt,
    /// One or more recurring agreements failed or were cancelled.
    TradeInterrupted,
    /// A foreign wonder construction project began.
    WonderStarted,
    /// A foreign wonder finished construction.
    WonderCompleted,
    /// Independent Control crossed 50 upward.
    ControlFifty,
    /// Independent Control crossed 100 upward.
    ControlFull,
    /// Directly owned Control first fell below 90.
    OwnedControlThreatened,
    /// A local spy was discovered and removed.
    SpyDetected,
    /// An unmaintained local spy was withdrawn.
    SpyWithdrawn,
    /// The local player acquired a particular scandal.
    ScandalDiscovered,
    /// A recurring senator bribe payment was publicly exposed.
    SenatorBriberyExposed,
    /// One monthly summary of lost Vassal Control.
    VassalWeakened,
    /// Foreign interference reduced owned population happiness.
    ForeignUnrest,
    /// Relation crossed below 40.
    HostileRelation,
    /// Relation crossed below 20.
    VeryHostileRelation,
    /// A vassal's relation to its overlord crossed below 50.
    VassalRelationDecay,
}

/// A reusable navigation intention; the renderer resolves canonical map coordinates.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum NoticeAction {
    /// Open the persistent Senate chamber and current office requirements.
    OpenSenate,
    /// Open the national military career and its promotion controls.
    OpenMilitary,
    /// Select the province and open its existing contextual panel.
    OpenProvince(usize),
    /// Focus and zoom to the canonical WONDERS site index.
    FocusWonder(usize),
    /// Open evidence in the Senate/espionage panel, with its originating province.
    OpenScandal {
        /// Unique evidence id.
        scandal: u64,
        /// Source province, when known.
        province: Option<usize>,
    },
}

/// A complete player-scoped event, retained after its transient toast disappears.
#[derive(Debug, Clone)]
pub(crate) struct CampaignNotice {
    /// Stable history entry id.
    pub id: u64,
    /// The only player who should receive this notification.
    pub recipient: usize,
    /// Presentation priority.
    pub severity: NoticeSeverity,
    /// Short title.
    pub title: String,
    /// Short explanation of the cause and consequence.
    pub body: String,
    /// Domain event category.
    pub kind: NoticeKind,
    /// Affected province, where applicable.
    pub province: Option<usize>,
    /// The building involved in this event, retained after construction finishes.
    pub building: Option<BuildingType>,
    /// Canonical wonder site, where applicable.
    pub wonder: Option<usize>,
    /// Unique evidence item, where applicable.
    pub scandal: Option<u64>,
    /// Authoritative creation month.
    pub month: u32,
    /// UI-independent click behavior.
    pub action: NoticeAction,
}

/// Notification storage does not mutate gameplay and does not pause monthly resolution.
#[derive(Debug, Clone)]
pub(crate) struct CampaignNotifications {
    pending: Vec<CampaignNotice>,
    history: Vec<CampaignNotice>,
    next_id: u64,
    foreign_happiness: Vec<(usize, usize, usize, f64)>,
    last_final: Option<NotificationSnapshot>,
    food_shortage_active: Vec<bool>,
    political_opportunities: Vec<Option<PoliticalRank>>,
    military_opportunities: Vec<Option<MilitaryRank>>,
    /// Material total loss across affected classes required for an unrest warning.
    pub happiness_warning_threshold: f64,
    /// Minimum loss of monthly military Control before reporting weakened garrisons.
    pub garrison_warning_minimum: f64,
    /// Relative loss required as well, avoiding repeated warnings for tiny changes.
    pub garrison_warning_fraction: f64,
}

impl Default for CampaignNotifications {
    /// Begin with an empty history and the specification's five-point warning threshold.
    fn default() -> Self {
        Self {
            pending: Vec::new(),
            history: Vec::new(),
            next_id: 0,
            foreign_happiness: Vec::new(),
            last_final: None,
            food_shortage_active: Vec::new(),
            political_opportunities: Vec::new(),
            military_opportunities: Vec::new(),
            happiness_warning_threshold: 5.0,
            garrison_warning_minimum: 0.5,
            garrison_warning_fraction: 0.25,
        }
    }
}

impl CampaignNotifications {
    /// Announce a shortage episode once per player, rearming only when supply recovers.
    pub fn food_shortage_started(&mut self, player: usize, supplied: f64) -> bool {
        self.food_shortage_active.resize(self.food_shortage_active.len().max(player + 1), false);
        let active = supplied < 0.9;
        let started = active && !self.food_shortage_active[player];
        self.food_shortage_active[player] = active;
        started
    }

    /// Deliver only the active player's pending notices; other players retain privacy.
    pub fn drain_for(&mut self, player: usize) -> Vec<CampaignNotice> {
        let mut result = Vec::new();
        self.pending.retain(|notice| {
            if notice.recipient == player {
                result.push(notice.clone());
                false
            } else {
                true
            }
        });
        result
    }

    /// Recent history for the viewing player, including previously displayed toasts.
    pub fn history_for(&self, player: usize) -> impl Iterator<Item = &CampaignNotice> {
        self.history.iter().rev().filter(move |notice| notice.recipient == player)
    }

    /// Aggregate actual enemy-caused losses, independently of famine and domestic policies.
    pub fn record_foreign_happiness(
        &mut self,
        actor: usize,
        owner: usize,
        province: usize,
        changes: [f64; 4],
    ) {
        let loss = changes.iter().map(|change| (-change).max(0.0)).sum::<f64>();
        if loss > 0.0 {
            self.foreign_happiness.push((actor, owner, province, loss));
        }
    }

    /// Aggregate monthly events while retaining each military access change.
    pub fn push(&mut self, mut notice: CampaignNotice) {
        let same = |old: &CampaignNotice| {
            !matches!(
                notice.kind,
                NoticeKind::MilitaryAccessGranted
                    | NoticeKind::MilitaryAccessRevoked
                    | NoticeKind::ArmyDisbanded
                    | NoticeKind::MilitaryRankIncreased
                    | NoticeKind::SenateOfficeAppointed
                    | NoticeKind::PoliticalPromotionAvailable
                    | NoticeKind::MilitaryPromotionAvailable
            ) && old.recipient == notice.recipient
                && old.kind == notice.kind
                && old.province == notice.province
                && old.building == notice.building
                && old.wonder == notice.wonder
                && old.scandal == notice.scandal
                && old.month == notice.month
        };
        if let Some(old) = self.history.iter_mut().find(|old| same(old)) {
            old.body.clone_from(&notice.body);
            if let Some(pending) = self.pending.iter_mut().find(|pending| pending.id == old.id) {
                pending.body.clone_from(&notice.body);
            }
            return;
        }
        self.next_id += 1;
        notice.id = self.next_id;
        self.pending.push(notice.clone());
        self.history.push(notice);
        if self.pending.len() > 200 {
            self.pending.remove(0);
        }
    }

    /// Create an ordinary province notice without repeating navigation metadata.
    pub fn province_notice(
        &mut self,
        player: usize,
        province: usize,
        month: u32,
        severity: NoticeSeverity,
        kind: NoticeKind,
        title: impl Into<String>,
        body: impl Into<String>,
    ) {
        self.push(CampaignNotice {
            id: 0,
            recipient: player,
            severity,
            title: title.into(),
            body: body.into(),
            kind,
            province: Some(province),
            building: None,
            wonder: None,
            scandal: None,
            month,
            action: NoticeAction::OpenProvince(province),
        });
    }
}

/// Previous final values needed for genuine threshold crossings.
#[derive(Debug, Clone)]
pub(crate) struct NotificationSnapshot {
    politics: Vec<(PoliticalState, Vec<f64>)>,
    controls: Vec<Vec<f64>>,
    completed_wonders: Vec<Option<usize>>,
    building_wonders: Vec<Option<usize>>,
    military_access: Vec<Vec<MilitaryAccess>>,
    occupations: Vec<Option<ForceOwner>>,
    garrison_support: Vec<Vec<f64>>,
}

impl Campaign {
    /// Announce each newly available promotion privately, rearming after eligibility is lost.
    pub fn notify_rank_opportunities(&mut self) {
        if !self.active {
            return;
        }
        self.pull_wallets();
        let count = self.actors.len();
        self.notifications.political_opportunities.resize(count, None);
        self.notifications.military_opportunities.resize(count, None);
        for player in 0..count {
            let playing = self.senate.winner.is_none()
                && !self.defeated.get(player).copied().unwrap_or(false);
            let political = playing
                .then(|| {
                    self.senate
                        .promotion_eligibility(player, &self.actors, &self.senate_config)
                        .ok()
                })
                .flatten();
            let military = if playing {
                [MilitaryRank::MilitaryTribune, MilitaryRank::Legate, MilitaryRank::Imperator]
                    .into_iter()
                    .find_map(|rank| {
                        self.military
                            .promotion_eligibility(
                                ForceOwner::Player(player),
                                rank,
                                self.economy.players[player].influence,
                            )
                            .ok()
                            .map(|requirements| (rank, requirements))
                    })
            } else {
                None
            };
            let next_political = political.map(|requirements| requirements.rank);
            let next_military = military.map(|(rank, _)| rank);
            if next_political != self.notifications.political_opportunities[player] {
                self.notifications.pending.retain(|notice| {
                    notice.recipient != player
                        || notice.kind != NoticeKind::PoliticalPromotionAvailable
                });
                if let Some(requirements) = political {
                    self.notifications.push(CampaignNotice {
                        id: 0,
                        recipient: player,
                        severity: NoticeSeverity::Info,
                        title: format!("You can become {}", requirements.rank.label()),
                        body: String::new(),
                        kind: NoticeKind::PoliticalPromotionAvailable,
                        province: None,
                        building: None,
                        wonder: None,
                        scandal: None,
                        month: self.economy.month,
                        action: NoticeAction::OpenSenate,
                    });
                }
                self.notifications.political_opportunities[player] = next_political;
            }
            if next_military != self.notifications.military_opportunities[player] {
                self.notifications.pending.retain(|notice| {
                    notice.recipient != player
                        || notice.kind != NoticeKind::MilitaryPromotionAvailable
                });
                if let Some((rank, _)) = military {
                    self.notifications.push(CampaignNotice {
                        id: 0,
                        recipient: player,
                        severity: NoticeSeverity::Info,
                        title: format!("You can become {}", rank.name()),
                        body: String::new(),
                        kind: NoticeKind::MilitaryPromotionAvailable,
                        province: None,
                        building: None,
                        wonder: None,
                        scandal: None,
                        month: self.economy.month,
                        action: NoticeAction::OpenMilitary,
                    });
                }
                self.notifications.military_opportunities[player] = next_military;
            }
        }
    }

    /// Senate events target the chamber explicitly rather than an unrelated province.
    pub fn record_senate_event(&mut self, event: &SenateEvent) {
        let (kind, title, body) = match event {
            SenateEvent::Victory(player) => (NoticeKind::AugustusVictory, "Augustus proclaimed".to_owned(), format!("Player {} has won the campaign.", player + 1)),
            SenateEvent::RankAdvanced(player, rank) => (NoticeKind::SenateOfficeAppointed, format!("Player {} became {}.", player + 1, rank.label()), String::new()),
            SenateEvent::ConsulExpired(player) => (NoticeKind::ConsulTermExpired, "Consular term expired".to_owned(), format!("Player {} is now a Proconsul. They may seek a Consul seat again after 12 months.", player + 1)),
            SenateEvent::ConsulRemoved(player) => (NoticeKind::ConsulRemoved, "Consul forced to resign".to_owned(), format!("Player {} lost Senate confidence and became Proconsul. They must wait 12 months to seek office again.", player + 1)),
        };
        for recipient in 0..self.actors.len() {
            self.notifications.push(CampaignNotice {
                id: 0,
                recipient,
                severity: NoticeSeverity::Info,
                title: title.clone(),
                body: body.clone(),
                kind,
                province: None,
                building: None,
                wonder: None,
                scandal: None,
                month: self.economy.month,
                action: NoticeAction::OpenSenate,
            });
        }
    }

    /// Turn authoritative military events into private, clickable player history.
    pub fn record_military_event(&mut self, event: &MilitaryEvent) {
        let month = self.economy.month;
        match *event {
            MilitaryEvent::Recruited {
                province,
                owner: ForceOwner::Player(player),
                unit_type,
                ..
            } => {
                self.notifications.province_notice(
                    player,
                    province,
                    month,
                    NoticeSeverity::Info,
                    NoticeKind::RecruitmentCompleted,
                    format!("{} recruited", unit_type.name()),
                    format!("The cohort is ready in {}.", self.economy.provinces[province].name),
                );
            },
            MilitaryEvent::UnitsDestroyed {
                province,
                owner: ForceOwner::Player(player),
                count,
            } => {
                self.notifications.province_notice(player,province,month,NoticeSeverity::Warning,NoticeKind::UnitsDestroyed,
                    format!("{count} cohorts lost in {}",self.economy.provinces[province].name),"These cohorts were destroyed in combat or had no legal retreat. Their manpower losses are permanent.");
            },
            MilitaryEvent::Arrived {
                province,
                owner: ForceOwner::Player(player),
                invasion,
                ..
            } => {
                let province_state = &self.economy.provinces[province];
                let guardian = province_state.owner.or(province_state.overlord);
                let name = province_state.name.clone();
                if invasion {
                    self.notifications.province_notice(player,province,month,NoticeSeverity::Warning,NoticeKind::InvasionBegins,
                        format!("Invasion of {name}"),"Your troops entered hostile territory. Defenders must be defeated before occupation can produce Control.");
                }
                if let Some(guardian) = guardian.filter(|&id| id != player) {
                    let hostile = invasion
                        || self.forces_hostile(
                            ForceOwner::Player(player),
                            ForceOwner::Player(guardian),
                        );
                    self.notifications.province_notice(
                        guardian,
                        province,
                        month,
                        if hostile {
                            NoticeSeverity::Warning
                        } else {
                            NoticeSeverity::Info
                        },
                        if hostile {
                            NoticeKind::InvasionBegins
                        } else {
                            NoticeKind::ForeignArrival
                        },
                        format!(
                            "{} units entered {name}",
                            if hostile {
                                "Enemy"
                            } else {
                                "Foreign"
                            }
                        ),
                        format!(
                            "Player {} has arrived. {}",
                            player + 1,
                            if hostile {
                                "A hostile encounter may now begin."
                            } else {
                                "Peaceful access does not generate occupation Control."
                            }
                        ),
                    );
                }
            },
            MilitaryEvent::MovementStopped {
                province,
                owner: ForceOwner::Player(player),
            } => {
                self.notifications.province_notice(player,province,month,NoticeSeverity::Warning,NoticeKind::MilitaryMovementStopped,
                    "Movement stopped",format!("Your troops remain in {} because the next crossing is no longer legal. Choose a new destination or secure access.",self.economy.provinces[province].name));
            },
            MilitaryEvent::OccupationEstablished {
                province,
                owner: ForceOwner::Player(player),
            } => self.notify_occupation(province, player),
            MilitaryEvent::RankIncreased {
                owner: ForceOwner::Player(player),
                rank,
            } => {
                for recipient in 0..self.actors.len() {
                    self.notifications.push(CampaignNotice {
                        id: 0,
                        recipient,
                        severity: NoticeSeverity::Info,
                        title: format!("Player {} became {}.", player + 1, rank.name()),
                        body: String::new(),
                        kind: NoticeKind::MilitaryRankIncreased,
                        province: None,
                        building: None,
                        wonder: None,
                        scandal: None,
                        month,
                        action: NoticeAction::OpenMilitary,
                    });
                }
            },
            MilitaryEvent::BattleEnded {
                province,
                battle,
                result,
                ..
            } => {
                if let Some(outcome) =
                    self.military.history.iter().find(|outcome| outcome.id == battle)
                {
                    let mut participants = Vec::new();
                    for (attacking, side) in
                        [(true, &outcome.attackers), (false, &outcome.defenders)]
                    {
                        let won = if attacking {
                            result == BattleResult::AttackerVictory
                        } else {
                            result == BattleResult::DefenderVictory
                        };
                        for owner in side.plans.keys() {
                            if let ForceOwner::Player(player) = owner {
                                participants.push((*player, won));
                            }
                        }
                    }
                    let defeated_side = match result {
                        BattleResult::AttackerVictory => Some(&outcome.defenders),
                        BattleResult::DefenderVictory => Some(&outcome.attackers),
                        BattleResult::MutualRout => None,
                    };
                    let npc_defeated = defeated_side.is_some_and(|side| {
                        side.plans.keys().any(|owner| matches!(owner, ForceOwner::Local(_)))
                    });
                    let name = &self.economy.provinces[province].name;
                    for (player, won) in participants {
                        self.notifications.province_notice(player,province,month,if won{NoticeSeverity::Info}else{NoticeSeverity::Warning},NoticeKind::BattleResolved,
                            format!("{} in {name}",if won{"Victory"}else{"Defeat"}),
                            if won{"Your surviving cohorts remain on the battlefield. Casualties remain permanent."}else{"Surviving cohorts withdrew to a legal adjacent province. Cohorts with no legal retreat were destroyed."});
                        if won && npc_defeated {
                            self.notifications.province_notice(player,province,month,NoticeSeverity::Info,NoticeKind::NpcDefeated,
                                format!("Local defenders defeated in {name}"),"Destroyed local cohorts do not automatically regenerate. Inspect occupation and Control before attempting political integration.");
                        }
                    }
                }
            },
            _ => {},
        }
    }

    /// Immediate battle-start notices include neutral territorial hosts as well as participants.
    pub fn notify_battle_started(&mut self, province: usize) {
        let Some(battle) = self.military.battles.iter().find(|battle| battle.province == province)
        else {
            return;
        };
        let mut recipients = std::collections::BTreeSet::new();
        for owner in battle.attackers.plans.keys().chain(battle.defenders.plans.keys()) {
            if let ForceOwner::Player(player) = owner {
                recipients.insert(*player);
            }
        }
        let p = &self.economy.provinces[province];
        let local_response =
            p.owner.is_none() && battle.attackers.plans.contains_key(&ForceOwner::Local(province));
        let body = if local_response {
            "Local defenders attacked your visiting army because relations fell below 50. Battle plans are locked; retreat becomes available after the first full combat month."
        } else {
            "Hostile forces have engaged. Battle plans are locked; retreat becomes available after the first full combat month."
        };
        if let Some(owner) = p.owner.or(p.overlord) {
            recipients.insert(owner);
        }
        for player in recipients {
            self.notifications.province_notice(
                player,
                province,
                self.economy.month,
                NoticeSeverity::Warning,
                NoticeKind::InvasionBegins,
                format!("Battle begins in {}", p.name),
                body,
            );
        }
    }

    /// Occupation may follow a battle or an undefended hostile arrival.
    fn notify_occupation(&mut self, province: usize, player: usize) {
        self.notifications.province_notice(player,province,self.economy.month,NoticeSeverity::Info,NoticeKind::OccupationEstablished,
            format!("Occupation established in {}",self.economy.provinces[province].name),"Surviving stationed strength can generate Control from the next monthly political tick, while occupation damages Relation. Ownership has not transferred.");
        if let Some(owner) = self.economy.provinces[province].owner.filter(|owner| *owner != player)
        {
            self.notifications.province_notice(
                owner, province, self.economy.month, NoticeSeverity::Warning,
                NoticeKind::OccupationEstablished,
                format!("{} is occupied", self.economy.provinces[province].name),
                "The province remains yours, but enemy occupation blocks recruitment, construction, policies and provincial trade. Your Control will fall. Send an army to defeat the occupier and restore access.",
            );
        }
    }

    /// Compare access, occupation and real military support at completed-month boundaries.
    fn record_military_thresholds(&mut self, before: &NotificationSnapshot) {
        let current_access = self.access_snapshot();
        for province in 0..self.economy.provinces.len() {
            let name = self.economy.provinces[province].name.clone();
            let old_state = before.politics.get(province).map(|p| &p.0);
            let unchanged_owner =
                old_state.is_some_and(|old| match (old, &self.politics[province].state) {
                    (
                        PoliticalState::Owned {
                            owner: a,
                        },
                        PoliticalState::Owned {
                            owner: b,
                        },
                    ) => a == b,
                    (
                        PoliticalState::Owned {
                            ..
                        },
                        _,
                    )
                    | (
                        _,
                        PoliticalState::Owned {
                            ..
                        },
                    ) => false,
                    _ => true,
                });
            if unchanged_owner {
                for (player, row) in current_access.iter().enumerate() {
                    let Some(previous) = before
                        .military_access
                        .get(player)
                        .and_then(|row| row.get(province))
                        .copied()
                    else {
                        continue;
                    };
                    let rank = |access| match access {
                        MilitaryAccess::Peaceful => 2,
                        MilitaryAccess::Transit => 1,
                        _ => 0,
                    };
                    let granted = rank(row[province]) > rank(previous);
                    let revoked = rank(row[province]) < rank(previous);
                    if granted || revoked {
                        self.notifications.province_notice(player,province,self.economy.month,if granted{NoticeSeverity::Info}else{NoticeSeverity::Warning},
                            if granted{NoticeKind::MilitaryAccessGranted}else{NoticeKind::MilitaryAccessRevoked},
                            format!("Military access {}: {name}",if granted{"granted"}else{"revoked"}),
                            match row[province] {
                                MilitaryAccess::Peaceful => "Peaceful troop passage and stationing are now permitted. Peaceful stationing never produces occupation Control.",
                                MilitaryAccess::Transit => "Troops may pass through this province, but may not station here. Existing movement orders recheck permission at their next crossing.",
                                _ => "New peaceful entry is no longer permitted. Existing movement orders recheck permission at their next crossing.",
                            });
                    }
                }
            }
            let current_occupation = self.military.provinces[province].occupation;
            if current_occupation != before.occupations.get(province).copied().flatten() {
                if let Some(ForceOwner::Player(player)) = current_occupation {
                    self.notify_occupation(province, player);
                }
            }
            if let PoliticalState::Vassal {
                overlord,
                ..
            } = self.politics[province].state
            {
                if !matches!(old_state,Some(PoliticalState::Vassal{overlord:old,..})if *old==overlord)
                {
                    continue;
                }
                let old = before
                    .garrison_support
                    .get(province)
                    .and_then(|row| row.get(overlord))
                    .copied()
                    .unwrap_or(0.);
                let owner = ForceOwner::Player(overlord);
                let current = self.military.config.garrison_control(
                    self.military.stationed_strength(province, owner),
                    self.military.rank(owner),
                );
                let minimum = self
                    .notifications
                    .garrison_warning_minimum
                    .max(old * self.notifications.garrison_warning_fraction);
                if old - current >= minimum {
                    self.notifications.province_notice(overlord,province,self.economy.month,NoticeSeverity::Warning,NoticeKind::GarrisonWeakened,
                        format!("Military support weakened in {name}"),format!("Garrison Control contribution fell from +{old:.2} to +{current:.2}/month as stationed strength changed. Check departures, battles and casualties."));
                }
            }
        }
    }

    /// Capture the starting political values so first-turn enemy actions are attributed.
    pub fn initialize_notification_snapshot(&mut self) {
        self.notifications.last_final = Some(self.notification_snapshot());
    }

    /// Capture pre-resolution values once; all causes are summarized after resolution.
    pub fn notification_snapshot(&self) -> NotificationSnapshot {
        NotificationSnapshot {
            politics: self
                .politics
                .iter()
                .map(|p| (p.state.clone(), p.relations.clone()))
                .collect(),
            controls: self
                .politics
                .iter()
                .map(|p| (0..self.actors.len()).map(|player| p.control(player)).collect())
                .collect(),
            completed_wonders: self.economy.provinces.iter().map(|p| p.completed_wonder).collect(),
            building_wonders: self
                .economy
                .provinces
                .iter()
                .map(|p| match &p.construction {
                    Some(ConstructionProject::Wonder(w)) => Some(w.wonder_id),
                    _ => None,
                })
                .collect(),
            military_access: self.access_snapshot(),
            occupations: self.military.provinces.iter().map(|p| p.occupation).collect(),
            garrison_support: (0..self.military.provinces.len())
                .map(|province| {
                    (0..self.actors.len())
                        .map(|player| {
                            let owner = ForceOwner::Player(player);
                            self.military.config.garrison_control(
                                self.military.stationed_strength(province, owner),
                                self.military.rank(owner),
                            )
                        })
                        .collect()
                })
                .collect(),
        }
    }

    /// Generate warnings and opportunities only on specified crossings or actual losses.
    pub fn record_monthly_notifications(&mut self, before: NotificationSnapshot) {
        let before = self.notifications.last_final.take().unwrap_or(before);
        let month = self.economy.month;
        self.record_military_thresholds(&before);
        for (province, politics) in self.politics.iter().enumerate() {
            let Some((old_state, old_relations)) = before.politics.get(province) else {
                continue;
            };
            let name = &self.economy.provinces[province].name;
            if let (
                PoliticalState::Owned {
                    owner: old_owner,
                },
                PoliticalState::Owned {
                    owner,
                },
            ) = (old_state, &politics.state)
            {
                if old_owner == owner {
                    let old_control = before
                        .controls
                        .get(province)
                        .and_then(|shares| shares.get(*owner))
                        .copied()
                        .unwrap_or(100.0);
                    let new_control = politics.control(*owner);
                    if old_control >= 90.0 && new_control < 90.0 {
                        self.notifications.province_notice(*owner, province, month, NoticeSeverity::Warning, NoticeKind::OwnedControlThreatened,
                            format!("Control in {name} is slipping"),
                            "Your Control has fallen below 90. Rival political pressure and unopposed rebellions can reduce your share. Open the province overview to inspect your remaining Control.");
                    }
                }
            }
            for player in 0..self.actors.len() {
                if let (
                    PoliticalState::Independent {
                        shares: old,
                        ..
                    },
                    PoliticalState::Independent {
                        shares: new,
                        ..
                    },
                ) = (old_state, &politics.state)
                {
                    let old = old.get(player).copied().unwrap_or(0.0);
                    let new = new.get(player).copied().unwrap_or(0.0);
                    for (threshold, kind) in
                        [(50.0, NoticeKind::ControlFifty), (100.0, NoticeKind::ControlFull)]
                    {
                        if old < threshold - 1e-7 && new >= threshold - 1e-7 {
                            self.notifications.province_notice(player, province, month, NoticeSeverity::Info, kind,
                                if threshold == 100.0 { format!("Full Control of {name}") } else { format!("Control in {name} has reached 50") },
                                if threshold == 100.0 { "Take Ownership is now available." } else { "Vassalize becomes available above 50 Control when you are the unique leader." });
                        }
                    }
                }
                if matches!(politics.state, PoliticalState::Owned { owner } if owner == player) {
                    continue;
                }
                let old = old_relations.get(player).copied().unwrap_or(50.0);
                let new = politics.relation(player);
                for (threshold, kind, label) in [
                    (40.0, NoticeKind::HostileRelation, "hostile"),
                    (20.0, NoticeKind::VeryHostileRelation, "very hostile"),
                ] {
                    if old >= threshold && new < threshold {
                        self.notifications.province_notice(
                            player,
                            province,
                            month,
                            NoticeSeverity::Warning,
                            kind,
                            format!("Relations with {name} are now {label}"),
                            format!("Relation fell from {old:.1} to {new:.1}."),
                        );
                    }
                }
                if matches!(politics.state, PoliticalState::Vassal { overlord, .. } if overlord == player)
                    && old >= 50.0
                    && new < 50.0
                {
                    self.notifications.province_notice(player, province, month, NoticeSeverity::Warning, NoticeKind::VassalRelationDecay,
                        format!("{name}'s support is weakening"), "Relation fell below 50. Vassal Control now decays unless garrisons or political support offset it.");
                }
            }
            if let PoliticalState::Vassal {
                overlord,
                control: old,
                ..
            } = old_state
            {
                let new = match politics.state {
                    PoliticalState::Vassal {
                        overlord: owner,
                        control,
                        ..
                    } if owner == *overlord => control,
                    PoliticalState::Independent {
                        ..
                    } => 0.0,
                    _ => *old,
                };
                if new < *old - 1e-7 {
                    let detail = &politics.last_control_change;
                    self.notifications.province_notice(*overlord, province, month, NoticeSeverity::Warning, NoticeKind::VassalWeakened,
                        format!("Control over {name} is weakening"), format!("{old:.1} → {new:.1}. Relation {:+.1}, garrison {:+.1}, support {:+.1}.", detail.relation, detail.military, detail.support));
                }
            }
        }
        for province in 0..self.economy.provinces.len() {
            let state = &self.economy.provinces[province];
            let owner = state.owner.or(state.overlord);
            let building = match &state.construction {
                Some(ConstructionProject::Wonder(w)) => Some(w.wonder_id),
                _ => None,
            };
            if building.is_some()
                && before.building_wonders.get(province).copied().flatten() != building
            {
                if let (Some(owner), Some(wonder)) = (owner, building) {
                    self.notify_wonder_started(owner, province, wonder);
                }
            }
            let state = &self.economy.provinces[province];
            if let Some(wonder) = state.completed_wonder {
                if before.completed_wonders.get(province).copied().flatten() != Some(wonder) {
                    if let Some(owner) = owner {
                        self.notify_wonder(owner, province, wonder, false);
                    }
                }
            }
        }
        let losses = std::mem::take(&mut self.notifications.foreign_happiness);
        let mut grouped = std::collections::BTreeMap::<
            (usize, usize),
            (f64, std::collections::BTreeSet<usize>),
        >::new();
        for (actor, owner, province, loss) in losses {
            let entry = grouped.entry((owner, province)).or_default();
            entry.0 += loss;
            entry.1.insert(actor);
        }
        for ((owner, province), (loss, actors)) in grouped {
            if loss < self.notifications.happiness_warning_threshold {
                continue;
            }
            let actors = actors
                .into_iter()
                .map(|actor| format!("Player {}", actor + 1))
                .collect::<Vec<_>>()
                .join(", ");
            self.notifications.province_notice(owner, province, month, NoticeSeverity::Warning, NoticeKind::ForeignUnrest,
                format!("Enemy agitation in {}", self.economy.provinces[province].name), format!("{actors} caused {loss:.1} total class-happiness points of hostile interference this month."));
        }
        self.notifications.last_final = Some(self.notification_snapshot());
    }

    /// Construction UI calls this immediately after a successful wonder start.
    pub fn notify_wonder_started(&mut self, owner: usize, province: usize, wonder: usize) {
        if self.notifications.history.iter().any(|notice| {
            notice.kind == NoticeKind::WonderStarted
                && notice.province == Some(province)
                && notice.wonder == Some(wonder)
        }) {
            return;
        }
        self.notify_wonder(owner, province, wonder, true);
    }

    /// Notify every opposing player once, targeting the canonical wonder position.
    fn notify_wonder(&mut self, owner: usize, province: usize, wonder: usize, started: bool) {
        let name = crate::map::wonder_name(wonder)
            .map_or_else(|| format!("wonder {}", wonder + 1), str::to_owned);
        for recipient in 0..self.actors.len() {
            if recipient == owner {
                continue;
            }
            self.notifications.push(CampaignNotice {
                id: 0,
                recipient,
                severity: NoticeSeverity::Warning,
                title: if started {
                    "Rival wonder construction".into()
                } else {
                    "Rival wonder completed".into()
                },
                body: format!(
                    "Player {} has {} {name}.",
                    owner + 1,
                    if started {
                        "begun constructing"
                    } else {
                        "completed"
                    }
                ),
                kind: if started {
                    NoticeKind::WonderStarted
                } else {
                    NoticeKind::WonderCompleted
                },
                province: Some(province),
                building: None,
                wonder: Some(wonder),
                scandal: None,
                month: self.economy.month,
                action: NoticeAction::FocusWonder(wonder),
            });
        }
    }
}

#[cfg(test)]
#[path = "../../tests/unit/notifications.rs"]
mod tests;
