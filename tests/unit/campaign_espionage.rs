use super::*;
use crate::game::economy::{EconomicProvince, EconomyWorld, Terrain};
use crate::game::politics::diplomacy::ProvincePolitics;
use crate::game::politics::espionage::SpyMission;
use crate::game::politics::PoliticalPlayer;

fn scandal_campaign(target: ScandalTarget, kind: ScandalKind, severity: Severity) -> Campaign {
    use crate::game::military::{MilitaryProvince, MilitaryTerrain};
    let provinces = ["Aquitania", "Rival province", "Our province", "Rome"]
        .into_iter()
        .enumerate()
        .map(|(id, name)| {
            let mut province =
                EconomicProvince::new(name, 60.0, Terrain::Farmland, true, [1.0; 3], [25.0; 4], 2);
            province.owner = match id {
                1 => Some(1),
                2 => Some(0),
                _ => None,
            };
            province
        })
        .collect();
    let mut campaign = Campaign {
        economy: EconomyWorld::new(2, provinces, vec![vec![]; 4]),
        actors: vec![PoliticalPlayer::default(); 2],
        politics: vec![
            ProvincePolitics::independent(2),
            ProvincePolitics::owned(2, 1),
            ProvincePolitics::owned(2, 0),
            ProvincePolitics::rome(2),
        ],
        graph: vec![
            MilitaryProvince {
                terrain: MilitaryTerrain::Plains,
                area: 60.0,
                road_level: 0,
                neighbors: vec![]
            };
            4
        ],
        ..Default::default()
    };
    campaign.espionage.scandals.push(crate::game::politics::espionage::Scandal {
        id: 1,
        holder: 0,
        target,
        kind,
        severity,
        province: Some(0),
        source_id: 1,
        acquired: 0,
        expires: u32::MAX,
        reserved_for_motion: false,
    });
    campaign
}

#[test]
fn scandals_grant_immediate_severity_scaled_control_and_relation_against_both_target_types() {
    for (target, province) in [(ScandalTarget::Province(0), 0), (ScandalTarget::Player(1), 1)] {
        for (severity, expected) in
            [(Severity::Minor, 5.0), (Severity::Medium, 7.5), (Severity::Major, 10.0)]
        {
            for usage in [ScandalUse::Control, ScandalUse::Relation] {
                let mut campaign = scandal_campaign(target, ScandalKind::SecretPayments, severity);
                assert_eq!(campaign.scandal_use_quote(0, 1, province, usage), Ok(expected));
                assert_eq!(campaign.use_scandal(0, 1, province, usage), Ok(expected));
                let politics = &campaign.politics[province];
                assert_eq!(
                    politics.control(0),
                    if usage == ScandalUse::Control {
                        expected
                    } else {
                        0.0
                    }
                );
                assert_eq!(
                    politics.relation(0),
                    if usage == ScandalUse::Relation {
                        50.0 + expected
                    } else {
                        50.0
                    }
                );
                assert_eq!(
                    campaign.economy.provinces[province].relation_by_player[0],
                    politics.relation(0)
                );
                assert!(campaign.espionage.scandals.is_empty());
                assert_eq!(
                    campaign.use_scandal(0, 1, province, usage),
                    Err(PoliticalError::ScandalRequired)
                );
            }
        }
    }
}

#[test]
fn invalid_scandal_uses_leave_evidence_and_politics_unchanged() {
    for (holder, province, usage, reserved, expires) in [
        (1, 0, ScandalUse::Relation, false, u32::MAX),
        (0, 1, ScandalUse::Control, false, u32::MAX),
        (0, 3, ScandalUse::Relation, false, u32::MAX),
        (0, 0, ScandalUse::Control, true, u32::MAX),
        (0, 0, ScandalUse::Relation, false, 0),
    ] {
        let mut campaign =
            scandal_campaign(ScandalTarget::Province(0), ScandalKind::EliteFeud, Severity::Major);
        campaign.espionage.scandals[0].reserved_for_motion = reserved;
        campaign.espionage.scandals[0].expires = expires;
        let state = campaign.politics[0].state.clone();
        let relations = campaign.politics[0].relations.clone();
        assert!(campaign.use_scandal(holder, 1, province, usage).is_err());
        assert_eq!(campaign.espionage.scandals.len(), 1);
        assert_eq!(campaign.politics[0].state, state);
        assert_eq!(campaign.politics[0].relations, relations);
    }
    let mut campaign =
        scandal_campaign(ScandalTarget::Player(1), ScandalKind::EliteFeud, Severity::Major);
    for province in [0, 2, 3] {
        assert_eq!(
            campaign.use_scandal(0, 1, province, ScandalUse::Relation),
            Err(PoliticalError::Ineligible)
        );
    }
    campaign.politics[1] = ProvincePolitics::owned(2, 0);
    assert_eq!(campaign.use_scandal(0, 1, 1, ScandalUse::Control), Err(PoliticalError::Ineligible));
    assert_eq!(campaign.espionage.scandals.len(), 1);
}

#[test]
fn provincial_claims_require_local_misconduct_and_never_waste_evidence_at_the_cap() {
    for kind in [
        ScandalKind::Espionage,
        ScandalKind::PoliticalBribery,
        ScandalKind::SenatorMurder,
        ScandalKind::SenatorCoercion,
        ScandalKind::PoliticalSmear,
    ] {
        let mut campaign = scandal_campaign(ScandalTarget::Player(1), kind, Severity::Major);
        assert_eq!(
            campaign.use_scandal(0, 1, 1, ScandalUse::Control),
            Err(PoliticalError::Ineligible)
        );
        assert_eq!(campaign.espionage.scandals.len(), 1);
        assert_eq!(campaign.use_scandal(0, 1, 1, ScandalUse::Relation), Ok(10.0));
    }
    for usage in [ScandalUse::Control, ScandalUse::Relation] {
        let mut campaign = scandal_campaign(
            ScandalTarget::Province(0),
            ScandalKind::IllegalTaxes,
            Severity::Major,
        );
        if usage == ScandalUse::Control {
            campaign.politics[0].gain_control_now(0, 98.0).unwrap();
        } else {
            campaign.politics[0].change_relation(0, 48.0);
        }
        assert_eq!(campaign.scandal_use_quote(0, 1, 0, usage), Ok(2.0));
        assert_eq!(campaign.use_scandal(0, 1, 0, usage), Ok(2.0));
        campaign.espionage.scandals.push(crate::game::politics::espionage::Scandal {
            id: 2,
            holder: 0,
            target: ScandalTarget::Province(0),
            kind: ScandalKind::IllegalTaxes,
            severity: Severity::Major,
            province: Some(0),
            source_id: 2,
            acquired: 0,
            expires: u32::MAX,
            reserved_for_motion: false,
        });
        assert_eq!(campaign.use_scandal(0, 2, 0, usage), Err(PoliticalError::Ineligible));
        assert_eq!(campaign.espionage.scandals.len(), 1);
    }
}

#[test]
fn npc_trade_leverage_remains_temporary_and_player_evidence_cannot_force_trade_terms() {
    let mut campaign =
        scandal_campaign(ScandalTarget::Province(0), ScandalKind::Smuggling, Severity::Medium);
    assert_eq!(campaign.use_scandal(0, 1, 0, ScandalUse::Trade), Ok(25.0));
    assert!(campaign.espionage.scandals.is_empty());
    assert_eq!(campaign.espionage.trade_ratio(0, 0, 0), 0.75);
    assert_eq!(campaign.economy.provinces[0].trade_ratio_by_player[0], 0.75);
    assert_eq!(campaign.espionage.trade_ratio(1, 0, 0), 1.0);
    assert_eq!(campaign.espionage.trade_ratio(0, 0, 6), 1.0);
    let mut campaign =
        scandal_campaign(ScandalTarget::Player(1), ScandalKind::Smuggling, Severity::Medium);
    assert_eq!(campaign.use_scandal(0, 1, 1, ScandalUse::Trade), Err(PoliticalError::Ineligible));
    assert_eq!(campaign.espionage.scandals.len(), 1);
}

#[test]
fn senate_row_use_exposes_exact_player_evidence_and_rejects_npcs_and_expired_evidence() {
    let mut campaign =
        scandal_campaign(ScandalTarget::Player(1), ScandalKind::SecretPayments, Severity::Major);
    let losses = ScandalKind::SecretPayments.senate_losses(Severity::Major);
    assert_eq!(campaign.expose_scandal(0, 1), Ok(1));
    assert!(campaign.espionage.scandals.is_empty());
    assert_eq!(campaign.senate.accusations.len(), 1);
    assert_eq!(campaign.senate.accusations[0].target, 1);
    assert_eq!(campaign.senate.accusations[0].penalties, losses);
    assert_eq!(campaign.expose_scandal(0, 1), Err(PoliticalError::ScandalRequired));
    let mut campaign =
        scandal_campaign(ScandalTarget::Province(0), ScandalKind::SecretPayments, Severity::Major);
    assert_eq!(campaign.expose_scandal(0, 1), Err(PoliticalError::Ineligible));
    assert_eq!(campaign.espionage.scandals.len(), 1);
    campaign.espionage.scandals[0].target = ScandalTarget::Player(1);
    campaign.espionage.scandals[0].expires = 5;
    campaign.economy.month = 10;
    assert_eq!(campaign.expose_scandal(0, 1), Err(PoliticalError::ScandalRequired));
    assert_eq!(campaign.espionage.scandals.len(), 1);
}

#[test]
fn multiple_campaign_spies_match_per_spy_risk_without_duplicate_checks() {
    use crate::game::politics::espionage::{detection_chance, EspionageConfig, EspionageState};

    const SAMPLES: u64 = 5_000;
    const SPIES: usize = 8;
    let provinces: Vec<_> = (0..=SPIES)
        .map(|id| {
            let mut province = EconomicProvince::new(
                format!("Province {id}"),
                40.0,
                Terrain::Farmland,
                true,
                [1.0; 3],
                [10.0, 20.0, 30.0, 40.0],
                2,
            );
            province.owner = if id == 0 {
                Some(0)
            } else if id % 2 == 0 {
                Some(1)
            } else {
                None
            };
            province.happiness = [50.0; 4];
            province
        })
        .collect();
    let template = Campaign {
        politics: provinces
            .iter()
            .map(|province| {
                province.owner.map_or_else(
                    || ProvincePolitics::independent(2),
                    |owner| ProvincePolitics::owned(2, owner),
                )
            })
            .collect(),
        economy: EconomyWorld::new(2, provinces, vec![vec![]; SPIES + 1]),
        actors: vec![
            PoliticalPlayer {
                coin: 10_000.0,
                influence: 1_000.0,
                ..Default::default()
            };
            2
        ],
        ..Default::default()
    };

    for config in [
        EspionageConfig {
            detection_range: [0.01, 0.07],
            ..Default::default()
        },
        EspionageConfig::default(),
    ] {
        let risk = detection_chance(50.0, &config);
        let mut caught = [0; 3];
        let mut fleets_with_losses = [0; 3];
        for seed in 0..SAMPLES {
            let mut campaign = template.clone();
            campaign.espionage_config = config.clone();
            campaign.espionage = EspionageState::new(seed);
            for province in 1..=SPIES {
                campaign
                    .espionage
                    .deploy(0, province, &mut campaign.actors, &campaign.politics, &config)
                    .unwrap();
            }
            for month in 1..=3 {
                campaign.economy.month = month;
                campaign.advance_espionage();
                let remaining = campaign.espionage.missions.len();
                let coin = campaign.actors[0].coin;
                campaign.advance_espionage();
                assert_eq!(campaign.actors[0].coin, coin);
                assert_eq!(campaign.espionage.missions.len(), remaining);
                let losses = SPIES - remaining;
                assert_eq!(
                    campaign
                        .notifications
                        .history_for(0)
                        .filter(|notice| notice.kind == NoticeKind::SpyDetected)
                        .count(),
                    losses
                );
                caught[(month - 1) as usize] += losses;
                fleets_with_losses[(month - 1) as usize] += usize::from(losses > 0);
            }
        }
        for index in 0..3 {
            let months = (index + 1) as i32;
            let expected = 1.0 - (1.0 - risk).powi(months);
            let observed = caught[index] as f64 / (SAMPLES as f64 * SPIES as f64);
            assert!(
                (observed - expected).abs() < 0.01,
                "{risk} per spy over {months} months: expected {expected}, observed {observed}"
            );
            let expected_fleets = 1.0 - (1.0 - risk).powi(months * SPIES as i32);
            let observed_fleets = fleets_with_losses[index] as f64 / SAMPLES as f64;
            assert!((observed_fleets - expected_fleets).abs() < 0.025,
                "{risk} with {SPIES} spies over {months} months: expected {expected_fleets}, observed {observed_fleets}");
            if months == 3 {
                eprintln!("{risk:.4} monthly risk: per-spy losses {observed:.4} (expected {expected:.4}), fleets with losses {observed_fleets:.4} (expected {expected_fleets:.4})");
            }
        }
    }
}

#[test]
fn projected_spy_upkeep_tracks_distance_even_when_the_graph_disconnects() {
    use crate::game::politics::espionage::SpyAssignment;
    let mut campaign = Campaign::default();
    let mut provinces: Vec<_> = ["Home", "Middle", "Target"]
        .into_iter()
        .map(|name| {
            EconomicProvince::new(
                name,
                40.0,
                Terrain::Farmland,
                true,
                [1.0; 3],
                [10.0, 20.0, 30.0, 40.0],
                1,
            )
        })
        .collect();
    provinces[0].owner = Some(0);
    campaign.economy = EconomyWorld::new(1, provinces, vec![vec![1], vec![0, 2], vec![1]]);
    campaign.actors = vec![PoliticalPlayer {
        coin: 100.0,
        influence: 100.0,
        ..Default::default()
    }];
    campaign.politics = vec![
        ProvincePolitics::owned(1, 0),
        ProvincePolitics::independent(1),
        ProvincePolitics::independent(1),
    ];
    campaign.wars = vec![vec![false]];
    campaign.espionage_config.detection_range = [0.0; 2];
    let distance = campaign.distance(0, 2);
    campaign
        .espionage
        .deploy_assignment(
            0,
            2,
            &mut campaign.actors,
            &campaign.politics,
            &campaign.espionage_config,
            SpyAssignment::GainControl,
            distance,
        )
        .unwrap();
    assert_eq!(campaign.actors[0].influence, 87.0);
    assert_eq!(campaign.spy_upkeep(0), 6.0);
    campaign.economy.adjacency[0].push(2);
    campaign.economy.adjacency[2].push(0);
    assert_eq!(campaign.spy_upkeep(0), 5.0);
    campaign.advance_espionage();
    assert_eq!(campaign.actors[0].coin, 95.0);
    assert_eq!(campaign.espionage.missions[0].totals.coin_spent, 5.0);
    assert_eq!(campaign.espionage.missions[0].distance, 1);
    campaign.economy.adjacency = vec![vec![], vec![], vec![]];
    assert_eq!(campaign.distance(0, 2), Some(16));
    let disconnected_upkeep = campaign.espionage_config.monthly_cost_at(None).unwrap();
    assert_eq!(campaign.spy_upkeep(0), disconnected_upkeep);
    campaign.economy.month += 1;
    campaign.advance_espionage();
    assert_eq!(campaign.espionage.missions.len(), 1);
    assert_eq!(campaign.espionage.missions[0].distance, 16);
    assert_eq!(campaign.espionage.missions[0].totals.coin_spent, 5.0 + disconnected_upkeep);
}

#[test]
fn owned_province_undermining_reduces_one_random_class_each_tick_and_keeps_a_temporary_penalty() {
    use crate::game::politics::espionage::SpyAssignment;
    let mut campaign = Campaign::default();
    let mut foreign = EconomicProvince::new(
        "Achaia",
        40.0,
        Terrain::Farmland,
        true,
        [1.0; 3],
        [10.0, 20.0, 30.0, 40.0],
        2,
    );
    foreign.owner = Some(1);
    foreign.happiness = [50.0; 4];
    foreign.temporary_happiness = [0.0; 4];
    let mut home = foreign.clone();
    home.owner = Some(0);
    campaign.economy = EconomyWorld::new(2, vec![foreign, home], vec![vec![1], vec![0]]);
    campaign.actors = vec![
        PoliticalPlayer {
            coin: 100.0,
            influence: 100.0,
            ..Default::default()
        };
        2
    ];
    campaign.politics = vec![ProvincePolitics::owned(2, 1), ProvincePolitics::owned(2, 0)];
    campaign.wars = vec![vec![false; 2]; 2];
    campaign.espionage_config.detection_range = [0.0; 2];
    // NPC success rolls must not prevent player-owned population losses.
    let distance = campaign.distance(0, 0);
    campaign
        .espionage
        .deploy_assignment(
            0,
            0,
            &mut campaign.actors,
            &campaign.politics,
            &campaign.espionage_config,
            SpyAssignment::UndermineOpponents,
            distance,
        )
        .unwrap();
    campaign.advance_espionage();
    let first_tick = campaign.economy.provinces[0].happiness;
    let first_loss = campaign.espionage.missions[0].totals.happiness_reduced;
    assert!((0.0..=3.0).contains(&first_loss));
    assert_eq!(first_tick[3], 50.0, "Undermine Opponents never targets slaves");
    assert!(first_tick[..3].iter().filter(|&&value| value < 50.0).count() <= 1);
    assert_eq!(
        campaign.economy.provinces[0].temporary_happiness,
        first_tick.map(|value| value - 50.0)
    );
    assert_eq!(campaign.economy.provinces[1].happiness, [50.0; 4], "Home population is unaffected");
    assert_eq!(campaign.politics[0].relation(0), 50.0);
    assert_eq!(
        campaign.politics[0].relation(1),
        50.0,
        "Owned provinces suffer happiness loss instead of Relation loss"
    );
    assert_eq!(campaign.actors[0].coin, 95.0);
    assert_eq!(campaign.actors[0].influence, 80.0);
    assert_eq!(campaign.spy_upkeep(0), 5.0);
    assert_eq!(campaign.spy_upkeep(1), 0.0, "Spy upkeep is private to its paying player");
    campaign.advance_espionage();
    assert_eq!(
        campaign.economy.provinces[0].happiness, first_tick,
        "Repeated ticks do not reduce Happiness twice"
    );
    for month in 1..=12 {
        let before = campaign.economy.provinces[0].happiness;
        campaign.economy.month = month;
        campaign.advance_espionage();
        let target = &campaign.economy.provinces[0];
        let changes =
            std::array::from_fn::<_, 4, _>(|class| before[class] - target.happiness[class]);
        assert_eq!(changes[3], 0.0);
        assert!(changes.iter().filter(|&&loss| loss > 0.0).count() <= 1);
        assert!((0.0..=3.0).contains(&changes.iter().sum::<f64>()));
        assert_eq!(target.temporary_happiness, target.happiness.map(|value| value - 50.0));
    }
    assert!(
        campaign.economy.provinces[0].happiness.iter().filter(|&&value| value < 50.0).count() > 1,
        "The selected class varies across ticks"
    );
    assert_eq!(
        campaign.espionage.missions[0].totals.happiness_reduced,
        campaign.economy.provinces[0].happiness[..3].iter().map(|value| 50.0 - value).sum::<f64>()
    );
    assert_eq!(campaign.espionage.missions[0].totals.coin_spent, 65.0);
    campaign.economy.provinces[0].happiness = [0.5; 4];
    campaign.economy.provinces[0].temporary_happiness = [0.0; 4];
    campaign.economy.month += 1;
    campaign.advance_espionage();
    assert_eq!(campaign.economy.provinces[0].happiness[3], 0.5);
    assert!(
        campaign.economy.provinces[0].happiness[..3].iter().filter(|&&value| value == 0.0).count()
            <= 1
    );
    campaign.economy.provinces[0].happiness = [0.0; 4];
    campaign.economy.month += 1;
    campaign.advance_espionage();
    assert_eq!(
        campaign.economy.provinces[0].happiness, [0.0; 4],
        "Happiness cannot fall below zero"
    );
    assert!(campaign.espionage.missions[0].totals.happiness_reduced >= first_loss);
    campaign.espionage.withdraw(0, 0);
    assert_eq!(campaign.spy_upkeep(0), 0.0, "Recall removes upkeep from projected outflow");
}

#[test]
fn detected_player_spy_evidence_opens_senate_and_retains_provincial_origin() {
    let mut campaign = Campaign::default();
    let mut province = EconomicProvince::new(
        "Achaia",
        40.0,
        Terrain::Farmland,
        true,
        [1.0; 3],
        [10.0, 20.0, 30.0, 40.0],
        2,
    );
    province.owner = Some(1);
    let mut home = province.clone();
    home.owner = Some(0);
    campaign.economy = EconomyWorld::new(2, vec![province, home], vec![vec![1], vec![0]]);
    campaign.actors = vec![
        PoliticalPlayer {
            coin: 100.0,
            influence: 100.0,
            ..Default::default()
        };
        2
    ];
    campaign.politics = vec![ProvincePolitics::owned(2, 1), ProvincePolitics::owned(2, 0)];
    campaign.wars = vec![vec![false; 2]; 2];
    campaign.espionage_config.detection_range = [1.0, 1.0];
    campaign.espionage.missions.push(SpyMission {
        owner: 0,
        province: 0,
        months_active: 0,
        assignment: crate::game::politics::espionage::SpyAssignment::DiscoverScandals,
        distance: 1,
        totals: Default::default(),
        recall_month: None,
    });
    campaign.advance_espionage();

    let notice = campaign
        .notifications
        .history_for(1)
        .find(|notice| notice.kind == NoticeKind::ScandalDiscovered)
        .expect("victim should receive discovered player evidence");
    assert_eq!(notice.province, Some(0));
    assert!(matches!(
        notice.action,
        NoticeAction::OpenScandal {
            province: None,
            ..
        }
    ));
    assert_eq!(campaign.espionage.scandals[0].target, ScandalTarget::Player(0));
    assert!(notice.body.contains("Permanent"));
    assert!(notice.body.contains("Serious"));
    assert!(!notice.body.contains("Severity II"));
}

#[test]
fn expired_scandals_notify_only_the_holder_and_keep_history_and_subject() {
    use crate::game::politics::espionage::{Scandal, ScandalKind, Severity};
    let mut campaign = Campaign::default();
    let scandal = Scandal {
        id: 3,
        holder: 1,
        target: ScandalTarget::Player(0),
        kind: ScandalKind::Espionage,
        severity: Severity::Medium,
        province: None,
        source_id: 1,
        acquired: 5,
        expires: 125,
        reserved_for_motion: false,
    };
    campaign.espionage.scandals.push(scandal);
    campaign.economy.month = 5;
    campaign.report_espionage_events(vec![EspionageEvent::EvidenceDiscovered(1, 3)]);
    campaign.economy.month = 125;
    campaign.advance_espionage();
    assert!(campaign.espionage.scandals.is_empty());
    assert!(campaign.notifications.drain_for(0).is_empty());
    let notices = campaign.notifications.drain_for(1);
    assert_eq!(notices.len(), 2);
    assert!(notices[0].body.contains("120 months remaining"));
    assert_eq!(notices[1].kind, NoticeKind::ScandalExpired);
    assert!(notices[1].title.contains("Detected Espionage"));
    assert!(notices[1].body.contains("120 months"));
    assert_eq!(campaign.notifications.scandal_target(3), Some(ScandalTarget::Player(0)));
    assert_eq!(campaign.notifications.history_for(1).count(), 2);
    campaign.economy.month += 1;
    campaign.advance_espionage();
    assert!(campaign.notifications.drain_for(1).is_empty());
}
