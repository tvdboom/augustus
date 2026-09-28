use super::*;
use crate::game::economy::{
    BuildingProject, BuildingType, EconomicProvince, EconomyWorld, ResourceFocus, Terrain,
};
use crate::game::military::{
    BattlePlan, CombatTactic, MilitaryProvince, MilitaryTerrain, MovementOrder, RecruitmentProject,
    UnitType,
};
use crate::game::politics::diplomacy::{ProvincePolitics, Tribute};
use crate::game::politics::PoliticalPlayer;

fn campaign() -> Campaign {
    let mut c = Campaign::default();
    let mut provinces: Vec<_> = (0..5)
        .map(|id| {
            EconomicProvince::new(
                format!("Province {id}"),
                50.0,
                Terrain::Plains,
                true,
                [1.0, 2.0, 3.0],
                [8.0, 22.0, 33.0, 37.0],
                2,
            )
        })
        .collect();
    provinces[0].owner = Some(0);
    provinces[1].owner = Some(1);
    provinces[3].overlord = Some(0);
    provinces[4].overlord = Some(1);
    c.economy = EconomyWorld::new(2, provinces, vec![vec![2], vec![], vec![0], vec![], vec![]]);
    c.politics = vec![
        ProvincePolitics::owned(2, 0),
        ProvincePolitics::owned(2, 1),
        ProvincePolitics::independent(2),
        ProvincePolitics::independent(2),
        ProvincePolitics::independent(2),
    ];
    c.politics[3].state = PoliticalState::Vassal {
        overlord: 0,
        control: 70.0,
        tribute: Tribute::Normal,
    };
    c.politics[4].state = PoliticalState::Vassal {
        overlord: 1,
        control: 70.0,
        tribute: Tribute::Normal,
    };
    c.actors = vec![
        PoliticalPlayer {
            coin: 1000.0,
            influence: 1000.0,
            ..Default::default()
        };
        2
    ];
    c.military = MilitaryWorld::new(5);
    c.graph = c
        .economy
        .adjacency
        .iter()
        .map(|neighbors| MilitaryProvince {
            terrain: MilitaryTerrain::Plains,
            area: 100.0,
            road_level: 0,
            neighbors: neighbors.clone(),
        })
        .collect();
    c.espionage_config.detection_range = [0.0; 2];
    c.espionage_config.npc_generation_chance = 0.0;
    c
}

fn deploy(c: &mut Campaign, player: usize, province: usize) {
    c.espionage.deploy(player, province, &mut c.actors, &c.politics, &c.espionage_config).unwrap();
}

fn resolve(c: &mut Campaign) {
    c.economy.month += 1;
    c.advance_espionage();
}

#[test]
fn local_policies_and_construction_pace_use_dated_intelligence_without_live_policy_leaks() {
    use crate::game::economy::{CivicSpending, ManumissionPolicy, RecruitmentEffort};
    let mut c = campaign();
    let p = &mut c.economy.provinces[1];
    p.policies.construction = ConstructionPace::Urgent;
    p.policies.civic_spending = CivicSpending::Generous;
    p.policies.recruitment = RecruitmentEffort::High;
    p.policies.manumission = ManumissionPolicy::Encouraged;
    p.construction = Some(ConstructionProject::Building(BuildingProject {
        building: BuildingType::Granary,
        target_level: 1,
        progress: 0.0,
        required_progress: 3.0,
    }));
    let policies = p.policies;
    deploy(&mut c, 0, 1);
    resolve(&mut c);
    assert!(c.intelligence[&(0, 1)].economy.is_none());
    resolve(&mut c);
    assert_eq!(c.intelligence[&(0, 1)].economy.as_ref().unwrap().policies, policies);
    resolve(&mut c);
    c.espionage.withdraw(0, 1);
    c.economy.provinces[1].policies.construction = ConstructionPace::Slow;
    c.economy.provinces[1].policies.civic_spending = CivicSpending::Frugal;
    resolve(&mut c);
    let report = &c.intelligence[&(0, 1)];
    assert_eq!(report.economy.as_ref().unwrap().policies, policies);
    let ops = report.operations.as_ref().unwrap();
    assert_eq!(ops.month, 3);
    assert_eq!(ops.construction_pace, ConstructionPace::Urgent);
    assert_eq!(
        ops.construction
            .as_ref()
            .unwrap()
            .months_remaining(&c.economy.config, ops.construction_pace),
        2
    );
    assert_eq!(
        c.economy.provinces[1]
            .construction
            .as_ref()
            .unwrap()
            .months_remaining(&c.economy.config, ConstructionPace::Slow),
        4
    );
}

#[test]
fn human_npc_and_foreign_vassal_reports_unlock_after_surviving_months() {
    for province in [1, 2, 4] {
        let mut c = campaign();
        c.economy.players[1].coin = 987654.0;
        c.economy.players[1].resources = [123456.0; 3];
        c.economy.provinces[province].policies.focus = ResourceFocus::Stone;
        c.economy.provinces[province].happiness = [13.25, 24.5, 36.75, 48.0];
        c.economy.provinces[province].construction =
            Some(ConstructionProject::Building(BuildingProject {
                building: BuildingType::Granary,
                target_level: 1,
                progress: 1.25,
                required_progress: 4.0,
            }));
        c.military.seed_unit(province, ForceOwner::Player(1), UnitType::HeavyInfantry).unwrap();
        c.military.provinces[province].recruitment = Some(RecruitmentProject {
            owner: ForceOwner::Player(1),
            unit_type: UnitType::Archers,
            progress: 1.0,
            required_progress: 3.0,
            manpower: 10.0,
        });
        deploy(&mut c, 0, province);
        assert!(c.intelligence.is_empty(), "deployment must reveal nothing");
        resolve(&mut c);
        let report = &c.intelligence[&(0, province)];
        assert_eq!(report.demographics.population, c.economy.provinces[province].population);
        assert_eq!(report.demographics.happiness, [13.25, 24.5, 36.75, 48.0]);
        assert!(report.economy.is_none() && report.operations.is_none());
        assert!(!c.intelligence.contains_key(&(1, province)));
        assert!(!c.intelligence.contains_key(&(0, 0)));
        resolve(&mut c);
        let report = &c.intelligence[&(0, province)];
        assert_eq!(report.economy.as_ref().unwrap().policies.focus, ResourceFocus::Stone);
        assert_eq!(
            report.economy.as_ref().unwrap().production,
            c.economy.provinces[province].production(&c.economy.config).1
        );
        assert!(report.operations.is_none());
        resolve(&mut c);
        let ops = c.intelligence[&(0, province)].operations.as_ref().unwrap();
        assert!(ops.construction.is_some());
        assert_eq!(ops.forces[&ForceOwner::Player(1)].len(), 1);
        assert_eq!(ops.recruitment.as_ref().unwrap().unit_type, UnitType::Archers);
        assert_eq!(c.actors[0].coin, 985.0);
        assert_eq!(c.espionage.missions[0].months_active, 3);
    }
}

#[test]
fn active_reports_change_only_on_monthly_resolution() {
    let mut c = campaign();
    deploy(&mut c, 0, 1);
    c.economy.last_report.month = 1;
    c.economy.last_report.province_reports =
        vec![crate::game::economy::ProvinceMonth::default(); c.economy.provinces.len()];
    c.economy.last_report.province_reports[1].births = [10.5; 4];
    c.economy.last_report.province_reports[1].normal_deaths = [2.25; 4];
    c.economy.last_report.province_reports[1].famine_deaths = [1.25; 4];
    c.economy.last_report.province_reports[1].migration = [-3.5; 4];
    resolve(&mut c);
    let old = c.intelligence[&(0, 1)].demographics.population;
    assert_eq!(c.intelligence[&(0, 1)].demographics.population_change, Some([3.5; 4]));
    c.economy.provinces[1].population = [999.0; 4];
    c.economy.last_report.province_reports[1].migration = [999.0; 4];
    c.advance_espionage();
    assert_eq!(c.espionage.missions[0].months_active, 1);
    assert_eq!(c.actors[0].coin, 995.0);
    assert_eq!(c.intelligence[&(0, 1)].demographics.population, old);
    assert_eq!(c.intelligence[&(0, 1)].demographics.population_change, Some([3.5; 4]));
    resolve(&mut c);
    assert_eq!(c.intelligence[&(0, 1)].demographics.population, [999.0; 4]);
    assert_eq!(c.intelligence[&(0, 1)].demographics.population_change, None);
}

#[test]
fn detected_or_unpaid_network_delivers_no_new_information() {
    for unpaid in [false, true] {
        let mut c = campaign();
        deploy(&mut c, 0, 1);
        resolve(&mut c);
        let old = c.intelligence[&(0, 1)].demographics.population;
        c.economy.provinces[1].population = [999.0; 4];
        if unpaid {
            c.actors[0].coin = 0.0;
        } else {
            c.espionage_config.detection_range = [1.0; 2];
        }
        resolve(&mut c);
        assert!(c.espionage.missions.is_empty());
        let report = &c.intelligence[&(0, 1)];
        assert_eq!(report.demographics.month, 1);
        assert_eq!(report.demographics.population, old);
        assert!(report.economy.is_none());
    }
    let mut c = campaign();
    deploy(&mut c, 0, 2);
    c.espionage_config.detection_range = [1.0; 2];
    resolve(&mut c);
    assert!(c.intelligence.is_empty());
}

#[test]
fn withdrawal_redeployment_and_switching_targets_reset_access_and_keep_dated_reports() {
    let mut c = campaign();
    deploy(&mut c, 0, 1);
    for _ in 0..3 {
        resolve(&mut c);
    }
    c.espionage.withdraw(0, 1);
    c.economy.provinces[1].population = [999.0; 4];
    resolve(&mut c);
    assert_eq!(c.intelligence[&(0, 1)].demographics.month, 3);
    deploy(&mut c, 0, 1);
    resolve(&mut c);
    assert_eq!(c.espionage.missions[0].months_active, 1);
    let report = &c.intelligence[&(0, 1)];
    assert_eq!(report.demographics.month, 5);
    assert_eq!(report.demographics.population, [999.0; 4]);
    assert_eq!(report.economy.as_ref().unwrap().month, 3);
    assert_eq!(report.operations.as_ref().unwrap().month, 3);
    c.espionage.withdraw(0, 1);
    deploy(&mut c, 0, 2);
    assert!(!c.intelligence.contains_key(&(0, 2)));
    resolve(&mut c);
    assert!(c.intelligence[&(0, 2)].operations.is_none());
}

#[test]
fn administration_and_nearby_troops_reveal_only_the_allowed_military_information() {
    let mut c = campaign();
    assert!(c.administers_province(0, 0));
    assert!(c.administers_province(0, 3));
    assert!(!c.administers_province(0, 1));
    assert!(!c.administers_province(0, 2));
    assert!(!c.administers_province(0, 4));
    for id in [1, 2, 3, 4] {
        c.military.seed_unit(id, ForceOwner::Player(1), UnitType::Archers).unwrap();
        c.military.provinces[id].plans.insert(
            ForceOwner::Player(1),
            BattlePlan {
                tactic: CombatTactic::ShockAction,
                ..Default::default()
            },
        );
    }
    let unit = c.military.provinces[1].forces[&ForceOwner::Player(1)][0].clone();
    c.military.movements.push(MovementOrder {
        id: 99,
        owner: ForceOwner::Player(1),
        units: vec![unit],
        origin: 1,
        route: vec![4],
        progress: 0.0,
        required_progress: 2.0,
        plan: BattlePlan::default(),
    });
    let hidden = c.military_view(0, true);
    assert!(hidden.provinces[1].forces.is_empty());
    assert!(hidden.provinces[2].forces.is_empty());
    assert_eq!(hidden.provinces[3].forces[&ForceOwner::Player(1)].len(), 1);
    assert!(hidden.movements.is_empty());
    assert!(hidden.provinces[3].plans.is_empty());
    c.military.seed_unit(0, ForceOwner::Player(0), UnitType::LightInfantry).unwrap();
    assert!(c.observes_military(0, 2));
    assert!(!c.observes_military(0, 1));
    let observed = c.military_view(0, false);
    assert_eq!(observed.provinces[2].forces[&ForceOwner::Player(1)].len(), 1);
    assert!(observed.provinces[2].plans.is_empty());
    assert!(observed.provinces[1].forces.is_empty());
    deploy(&mut c, 0, 1);
    for _ in 0..3 {
        resolve(&mut c);
    }
    c.military.provinces[1].forces.clear();
    assert_eq!(c.military_view(0, true).provinces[1].forces[&ForceOwner::Player(1)].len(), 1);
    assert!(
        c.military_view(0, false).provinces[1].forces.is_empty(),
        "map must not show historical reports as live troops"
    );
}

fn overview_text(c: &Campaign, province: usize, player: usize) -> String {
    use bevy_egui::egui;
    let ctx = egui::Context::default();
    let mut output = ctx.run_ui(egui::RawInput::default(), |root| {
        egui::CentralPanel::default().show(root, |ui| {
            *ui.style_mut() = crate::app::ui::campaign_widgets::map_style(1.0);
            ui.set_width(546.0);
            crate::app::ui::province_intelligence::overview(ui, c, province, player, 1.0);
        });
    });
    let text = output
        .shapes
        .iter()
        .filter_map(|s| match &s.shape {
            egui::Shape::Text(t) => Some(t.galley.job.text.as_str()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("\n");
    output.textures_delta.clear();
    text
}

#[test]
fn foreign_overview_displays_only_public_facts_or_dated_reports_and_vassals_are_current() {
    let mut c = campaign();
    c.economy.players[1].coin = 987654.0;
    c.economy.players[1].resources = [876543.0; 3];
    c.economy.provinces[1].happiness = [12.345, 23.456, 34.567, 45.678];
    let hidden = overview_text(&c, 1, 0);
    assert_eq!(hidden.lines().filter(|line| *line == "?").count(), 15);
    assert_eq!(hidden.lines().filter(|line| *line == "? / ?").count(), 3);
    assert!(!hidden.contains("Hidden") && !hidden.contains("requires"));
    assert!(!hidden.contains("Construction"));
    assert!(!hidden.contains("12.345"));
    assert!(!hidden.contains("987654") && !hidden.contains("876543"));
    deploy(&mut c, 0, 1);
    resolve(&mut c);
    let visible = overview_text(&c, 1, 0);
    assert!(visible.contains("12.345%"));
    assert!(visible.contains("Observed in month 1"));
    assert_eq!(visible.lines().filter(|line| *line == "?").count(), 7);
    assert!(!visible.contains("hidden") && !visible.contains("requires"));
    assert!(!visible.contains("Construction"));
    c.economy.provinces[1].happiness[0] = 99.123;
    c.espionage.withdraw(0, 1);
    let historical = overview_text(&c, 1, 0);
    assert!(historical.contains("12.345%"));
    assert!(!historical.contains("99.123"));
    assert!(historical.contains("last report; network inactive"));
    let other_player = overview_text(&c, 1, 1);
    assert!(other_player.contains("99.123%"));
    c.economy.provinces[3].happiness[0] = 87.654;
    let vassal = overview_text(&c, 3, 0);
    assert!(vassal.contains("87.654%") && vassal.contains("Current administrative report"));
    let foreign_vassal = overview_text(&c, 3, 1);
    assert!(!foreign_vassal.contains("87.654"));
}
