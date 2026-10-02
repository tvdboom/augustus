use super::*;
use crate::game::economy::{
    BuildingProject, BuildingType, ConstructionProject, EconomicProvince, EconomyWorld, Terrain,
};
use crate::game::military::{
    BattlePlan, CombatTactic, MilitaryProvince, MilitaryTerrain, MovementOrder, RecruitmentProject,
    UnitType,
};
use crate::game::politics::diplomacy::{PoliticalState, ProvincePolitics, Tribute};
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
    c.economy =
        EconomyWorld::new(2, provinces, vec![vec![1, 2], vec![0], vec![0, 3, 4], vec![2], vec![2]]);
    c.wars = vec![vec![false; 2]; 2];
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
fn local_policies_and_construction_pace_are_current_without_a_network() {
    use crate::game::economy::{
        CivicSpending, ConstructionPace, ManumissionPolicy, RecruitmentEffort,
    };
    let mut c = campaign();
    let p = &mut c.economy.provinces[1];
    p.policies.construction = ConstructionPace::Urgent;
    p.policies.civic_spending = CivicSpending::Generous;
    p.policies.recruitment = RecruitmentEffort::High;
    p.policies.manumission = ManumissionPolicy::Free;
    p.construction = Some(ConstructionProject::Building(BuildingProject {
        building: BuildingType::Granary,
        target_level: 1,
        progress: 0.0,
        required_progress: 3.0,
        paid_metal: 0.0,
        paid_stone: 0.0,
    }));
    assert!(buildings_text(&c, 1, 0).contains("3 months left"), "{}", buildings_text(&c, 1, 0));
    c.economy.provinces[1].policies.construction = ConstructionPace::Slow;
    assert!(buildings_text(&c, 1, 0).contains("4 months left"));
}

#[test]
fn spy_upkeep_detection_and_withdrawal_do_not_gate_public_facts() {
    for unpaid in [false, true] {
        let mut c = campaign();
        deploy(&mut c, 0, 1);
        resolve(&mut c);
        assert_eq!(c.actors[0].coin, 995.0);
        c.advance_espionage();
        assert_eq!(c.actors[0].coin, 995.0, "duplicate monthly ticks must not charge twice");
        c.economy.provinces[1].happiness[0] = 89.0;
        if unpaid {
            c.actors[0].coin = 0.0;
        } else {
            c.espionage_config.detection_range = [1.0; 2];
        }
        resolve(&mut c);
        assert!(c.espionage.missions.is_empty());
        assert_eq!(noble_happiness(&c, 1, 0).as_deref(), Some("89"));
    }
}

#[test]
fn all_provinces_show_live_composition_without_foreign_plans_or_condition() {
    let mut c = campaign();
    let enemy = ForceOwner::Player(1);
    let own = ForceOwner::Player(0);
    for id in [1, 2, 3, 4] {
        c.military.seed_unit(id, enemy, UnitType::Archers).unwrap();
        let unit = &mut c.military.provinces[id].forces.get_mut(&enemy).unwrap()[0];
        unit.training = 97.0;
        unit.morale = 81.0;
        c.military.provinces[id].plans.insert(
            enemy,
            BattlePlan {
                tactic: CombatTactic::ShockAction,
                ..Default::default()
            },
        );
    }
    c.military.seed_unit(0, own, UnitType::HeavyInfantry).unwrap();
    c.military.provinces[0].forces.get_mut(&own).unwrap()[0].training = 64.0;
    let marching = c.military.provinces[1].forces[&enemy][0].clone();
    c.military.movements.push(MovementOrder {
        id: 99,
        owner: enemy,
        units: vec![marching],
        origin: 1,
        route: vec![4],
        progress: 0.0,
        required_progress: 2.0,
        plan: BattlePlan::default(),
        withdrawing: false,
    });
    for include_reports in [false, true] {
        let view = c.military_view(0, include_reports);
        for id in [1, 2, 3, 4] {
            assert!(c.observes_military(0, id));
            assert_eq!(view.provinces[id].forces[&enemy].len(), 1);
            let unit = &view.provinces[id].forces[&enemy][0];
            assert_eq!(unit.unit_type, UnitType::Archers);
            assert_eq!(unit.training, c.military.config.starting_training);
            assert_eq!(unit.morale, 81.0);
            assert!(view.provinces[id].plans.is_empty());
        }
        assert_eq!(view.provinces[0].forces[&own][0].training, 64.0);
        assert!(view.movements.is_empty());
    }
    deploy(&mut c, 0, 1);
    for _ in 0..3 {
        resolve(&mut c);
    }
    c.military.provinces[1].forces.clear();
    for include_reports in [false, true] {
        assert!(
            c.military_view(0, include_reports).provinces[1].forces.is_empty(),
            "historical spy troops must never replace current public composition"
        );
    }
    assert_eq!(
        c.military.provinces[2].forces[&enemy][0].training, 97.0,
        "redacting the view must not mutate simulation state"
    );
}

#[test]
fn battle_views_preserve_own_details_and_hide_foreign_deployment_and_condition() {
    let mut c = campaign();
    let own = ForceOwner::Player(0);
    let enemy = ForceOwner::Player(1);
    c.military.seed_unit(1, own, UnitType::HeavyInfantry).unwrap();
    c.military.seed_unit(1, enemy, UnitType::Archers).unwrap();
    c.military
        .set_plan(
            1,
            enemy,
            BattlePlan {
                tactic: CombatTactic::ShockAction,
                flank_size: 2,
                ..Default::default()
            },
        )
        .unwrap();
    c.military
        .start_battle(1, &[own], &[enemy], None, None, MilitaryTerrain::Plains, 0, 10)
        .unwrap();
    let foreign_unit = &mut c.military.battles[0].defenders.units[0];
    foreign_unit.training = 97.0;
    foreign_unit.morale = 81.0;
    let own_formation = c.military.battles[0].attackers.formation.clone();
    let view = c.military_view(0, false);
    let battle = &view.battles[0];
    assert_eq!(battle.defenders.units.len(), 1);
    assert!(battle.defenders.plans.is_empty());
    assert!(battle.defenders.formation.front.iter().all(Option::is_none));
    assert!(battle.defenders.formation.support.iter().all(Option::is_none));
    assert!(battle.defenders.formation.reserves.is_empty());
    assert_eq!(battle.defenders.formation.flank_size, 0);
    assert_eq!(battle.defenders.units[0].training, c.military.config.starting_training);
    assert_eq!(battle.defenders.units[0].morale, 81.0);
    assert_eq!(battle.attackers.formation.front, own_formation.front);
    assert!(battle.attackers.plans.contains_key(&own));
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

fn noble_happiness(c: &Campaign, province: usize, player: usize) -> Option<String> {
    overview_text(c, province, player)
        .lines()
        .skip_while(|line| *line != "Nobles")
        .nth(3)
        .map(str::to_owned)
}

fn buildings_text(c: &Campaign, province: usize, player: usize) -> String {
    use bevy_egui::egui;
    let ctx = egui::Context::default();
    let mut output = ctx.run_ui(
        egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(600.0, 1400.0),
            )),
            ..Default::default()
        },
        |root| {
            egui::CentralPanel::default().show(root, |ui| {
                crate::app::ui::province_intelligence::buildings(ui, c, province, player, 1.0);
            });
        },
    );
    let text = output
        .shapes
        .iter()
        .filter_map(|shape| match &shape.shape {
            egui::Shape::Text(text) => Some(text.galley.job.text.as_str()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("\n");
    output.textures_delta.clear();
    text
}

#[test]
fn foreign_overviews_show_current_local_facts_without_spies_or_national_balances() {
    let mut c = campaign();
    c.economy.players[1].coin = 987654.0;
    c.economy.players[1].influence = 765432.0;
    c.economy.players[1].resources = [876543.0; 3];
    for province in [1, 2, 3, 4] {
        c.economy.provinces[province].happiness[0] = 12.0;
        c.economy.provinces[province].construction =
            Some(ConstructionProject::Building(BuildingProject {
                building: BuildingType::Granary,
                target_level: 1,
                progress: 1.0,
                required_progress: 4.0,
                paid_metal: 0.0,
                paid_stone: 0.0,
            }));
        let text = overview_text(&c, province, 0);
        assert_eq!(noble_happiness(&c, province, 0).as_deref(), Some("12"));
        assert!(text.contains("Population"), "Missing Population: {text}");
        for hidden in ["Food demand", "Food supplied"] {
            assert!(!text.contains(hidden), "Unexpected {hidden}: {text}");
        }
        assert!(!text.contains("Observed in month"));
        assert!(!text.contains("987654") && !text.contains("876543") && !text.contains("765432"));
        if c.economy.provinces[province].owner.is_some() {
            for expected in ["Civic Power", "Influence", "Sestertius", "Resources"] {
                assert!(text.contains(expected), "Missing {expected}: {text}");
            }
            let production = c.economy.provinces[province].production(&c.economy.config).1;
            for value in production {
                assert!(text.lines().any(|line| line == format!("{:+.0}", value.floor())));
            }
        } else {
            for hidden in ["Civic Power", "Influence", "Sestertius", "Resources", "Metal", "Stone"]
            {
                assert!(!text.contains(hidden), "Unexpected {hidden}: {text}");
            }
        }
        c.economy.provinces[province].happiness[0] = 99.0;
        assert_eq!(noble_happiness(&c, province, 0).as_deref(), Some("99"));
    }
    deploy(&mut c, 0, 1);
    for _ in 0..3 {
        resolve(&mut c);
    }
    c.espionage.withdraw(0, 1);
    c.economy.provinces[1].happiness[0] = 88.0;
    let current = overview_text(&c, 1, 0);
    assert_eq!(noble_happiness(&c, 1, 0).as_deref(), Some("88"));
    assert!(!current.contains("last report"));
    assert!(!current.contains("? / ?"), "Province resources must not show national stockpiles");
    let own = overview_text(&c, 1, 1);
    assert!(own.contains("Food demand") && own.contains("Food supplied"));
    assert!(own.contains("987654") && own.contains("765432"));
    assert!(!own.contains("876543"), "National stockpiles belong in the shared HUD");
}

#[test]
fn foreign_recruitment_is_current_and_read_only_without_a_network() {
    let mut c = campaign();
    c.military.provinces[1].recruitment = Some(RecruitmentProject {
        owner: ForceOwner::Player(1),
        unit_type: UnitType::Archers,
        progress: 1.0,
        required_progress: 3.0,
        population_cost: 1.0,
        cohort_manpower: 8.0,
        manpower_class: 2,
        paid_metal: 0.0,
    });
    let view = c.military_view(0, false);
    assert_eq!(view.provinces[1].recruitment.as_ref().unwrap().progress, 1.0);
    c.military.provinces[1].recruitment.as_mut().unwrap().progress = 2.0;
    assert_eq!(c.military_view(0, false).provinces[1].recruitment.as_ref().unwrap().progress, 2.0);
}

#[test]
fn foreign_buildings_show_current_levels_and_construction_without_controls() {
    use bevy_egui::egui;
    let mut c = campaign();
    c.economy.provinces[1].buildings[BuildingType::Granary as usize] = 2;
    c.economy.provinces[1].construction = Some(ConstructionProject::Building(BuildingProject {
        building: BuildingType::Granary,
        target_level: 3,
        progress: 1.0,
        required_progress: 4.0,
        paid_metal: 0.0,
        paid_stone: 0.0,
    }));
    let ctx = egui::Context::default();
    let mut output = ctx.run_ui(
        egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(600.0, 1400.0),
            )),
            ..Default::default()
        },
        |root| {
            egui::CentralPanel::default().show(root, |ui| {
                crate::app::ui::province_intelligence::buildings(ui, &c, 1, 0, 1.0);
            });
        },
    );
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
    assert!(text.contains("Level 2"));
    assert!(text.contains("Granary level 3 · 25% · 3 months left"), "{text}");
    assert!(!text.contains("spy report") && !text.contains("Cancel"));
}
