//! Headless layout checks run real campaign panel bodies at desktop/compact widths.

use super::*;
use crate::game::economy::{EconomicProvince, EconomyWorld, Terrain};
use crate::game::politics::espionage::{Scandal, ScandalKind, ScandalTarget, Severity};
use crate::game::politics::senate::{Ballot, Campaign as SenateCampaign, PoliticalProfile};
use crate::game::politics::PoliticalRank;

/// Deterministic representative states cover ownership, independence, vassals and troops.
fn fixture() -> Campaign {
    let names = ["Italia", "Africa Proconsularis", "Achaia", "Cappadocia", "Sardinia et Corsica"];
    let mut provinces: Vec<_> = names
        .iter()
        .map(|name| {
            EconomicProvince::new(
                *name,
                60.0,
                Terrain::Farmland,
                true,
                [1.4, 0.8, 0.6],
                [8.0, 22.0, 33.0, 37.0],
                2,
            )
        })
        .collect();
    provinces[0].owner = Some(0);
    provinces[0].wonder_sites = vec![0];
    provinces[1].owner = Some(1);
    provinces[3].overlord = Some(0);
    provinces[4].overlord = Some(1);
    let graph = vec![vec![1, 2], vec![0, 3], vec![0, 3, 4], vec![1, 2, 4], vec![2, 3]];
    let mut campaign = Campaign::default();
    campaign.active = true;
    campaign.economy = EconomyWorld::new(2, provinces, graph.clone());
    for wallet in &mut campaign.economy.players {
        wallet.coin = 5000.0;
        wallet.influence = 1000.0;
        wallet.resources = [2000.0; 3];
    }
    campaign.actors = vec![
        PoliticalPlayer {
            coin: 5000.0,
            influence: 1000.0,
            rank: PoliticalRank::Aedile,
            consul_until: None
        };
        2
    ];
    campaign.politics = vec![
        ProvincePolitics::owned(2, 0),
        ProvincePolitics::owned(2, 1),
        ProvincePolitics::independent(2),
        ProvincePolitics::independent(2),
        ProvincePolitics::independent(2),
    ];
    campaign.politics[2].state = PoliticalState::Independent {
        local: 15.0,
        shares: vec![55.0, 30.0],
    };
    campaign.politics[3].state = PoliticalState::Vassal {
        overlord: 0,
        control: 72.0,
        tribute: Tribute::High,
    };
    campaign.politics[4].state = PoliticalState::Vassal {
        overlord: 1,
        control: 40.0,
        tribute: Tribute::Normal,
    };
    campaign.military = MilitaryWorld::new(5);
    campaign.graph = graph
        .into_iter()
        .map(|neighbors| MilitaryProvince {
            terrain: MilitaryTerrain::Plains,
            area: 100.0,
            road_level: 1,
            neighbors,
        })
        .collect();
    for unit_type in UnitType::ALL {
        campaign.military.seed_unit(0, ForceOwner::Player(0), unit_type).unwrap();
    }
    campaign.military.seed_unit(1, ForceOwner::Player(1), UnitType::HeavyInfantry).unwrap();
    campaign.wars = vec![vec![false; 2]; 2];
    campaign.npc_wars = vec![vec![false; 5]; 2];
    campaign.invitations = vec![vec![true; 2]; 2];
    campaign.profiles = vec![PoliticalProfile::default(); 2];
    campaign.recent_victories = vec![0.0; 2];
    campaign.actors[1].rank = PoliticalRank::Consul;
    campaign.actors[1].consul_until = Some(48);
    campaign.espionage.scandals = vec![
        Scandal {
            id: 1,
            holder: 0,
            target: ScandalTarget::Province(2),
            kind: ScandalKind::MilitaryIncompetence,
            severity: Severity::Major,
            province: Some(2),
            source_id: 10,
            acquired: 0,
            expires: 24,
            reserved_for_motion: false,
        },
        Scandal {
            id: 2,
            holder: 0,
            target: ScandalTarget::Player(1),
            kind: ScandalKind::FriendlyAttack,
            severity: Severity::Medium,
            province: Some(1),
            source_id: 11,
            acquired: 0,
            expires: 24,
            reserved_for_motion: false,
        },
    ];
    campaign.notifications.province_notice(0, 1, 0, NoticeSeverity::Warning, super::super::campaign_notifications::NoticeKind::HostileRelation, "Relations with Africa Proconsularis have become hostile", "A foreign political campaign reduced our relationship. Inspect the provincial diplomacy panel for its full breakdown.");
    campaign.economy.refresh_npc_markets(&campaign.inputs());
    campaign
}

/// Exercise egui's actual layout, including expanded sections, without a desktop window.
fn layout_context() -> egui::Context {
    let ctx = egui::Context::default();
    ctx.set_global_style(augustus_ui_style());
    ctx.add_font(FontInsert::new(
        "firasans",
        egui::FontData::from_static(include_bytes!("../../assets/fonts/FiraSans-Bold.ttf")),
        vec![InsertFontFamily {
            family: egui::FontFamily::Proportional,
            priority: FontPriority::Highest,
        }],
    ));
    ctx.memory_mut(|memory| memory.set_everything_is_visible(true));
    ctx
}

#[test]
fn all_hud_ledgers_fit_with_a_full_atlas_of_owned_provinces() {
    let mut campaign = fixture();
    let mut province = campaign.economy.provinces[0].clone();
    province.name = "Africa Proconsularis extended source label".into();
    campaign.economy.provinces = vec![province; 54];
    for scale in [1.0, 0.85] {
        for index in 0..7 {
            assert_body_fits("HUD source ledger", 380.0 * scale, scale, |ui| {
                super::super::resource_hud::campaign_resource_breakdown(
                    ui,
                    index,
                    0,
                    &campaign,
                    &campaign.economy.players[0],
                    scale,
                );
            });
        }
    }
}

/// Measure actual panel bodies without substituting implementation-shaped mock widgets.
fn assert_body_fits(label: &str, width: f32, scale: f32, mut render: impl FnMut(&mut egui::Ui)) {
    let ctx = layout_context();
    let mut used_width: f32 = 0.0;
    let mut used_height: f32 = 0.0;
    // A second frame resolves font/layout cache and CollapsingHeader animation state.
    for frame in 0..2 {
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1600.0, 12_000.0),
            )),
            time: Some(frame as f64),
            ..Default::default()
        };
        let mut output = ctx.run_ui(input, |root| {
            root.scope_builder(
                egui::UiBuilder::new()
                    .max_rect(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(width, 12_000.0),
                    ))
                    .layout(egui::Layout::top_down(egui::Align::Min)),
                |ui| {
                    ui.set_max_width(width);
                    panel_style(ui, scale);
                    let start = ui.next_widget_position().x;
                    render(ui);
                    used_width = used_width.max(ui.min_rect().right() - start);
                    used_height = used_height.max(ui.min_rect().height());
                },
            );
        });
        // Headless layout intentionally has no GPU; acknowledge atlas/icon deltas.
        output.textures_delta.clear();
        assert!(!output.shapes.is_empty(), "{label}: empty output would not verify a real panel");
    }
    assert!(used_height > 25.0, "{label}: the actual content was not laid out");
    assert!(
        used_width <= width + 1.5,
        "{label} overflow: used {used_width:.1}px inside {width:.1}px at scale {scale:.2}"
    );
}

#[test]
fn complete_panel_shell_bounds_long_headers_and_status_with_scrolled_content() {
    for (width, height, scale) in [(570.0, 720.0, 1.0), (404.0, 540.0, 0.85), (404.0, 400.0, 1.0)] {
        for tab in [CampaignTab::Province, CampaignTab::Senate] {
            for status in [String::new(), "Political action completed. The affected province's relationship and control have changed; review the detailed modifiers and the next monthly resolution before committing more resources. ".repeat(20)] {
                let ctx = layout_context();
                let mut campaign = fixture();
                let mut view = CampaignUi { open: Some(tab), province: Some(4), ..Default::default() };
                let rect = egui::Rect::from_min_size(egui::pos2(30.0, 30.0), egui::vec2(width, height));
                for frame in 0..2 {
                    let input = egui::RawInput { screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1200.0, 900.0))), time: Some(frame as f64), ..Default::default() };
                    let mut outer = rect;
                    let mut output = ctx.run_ui(input, |root| {
                        root.scope_builder(egui::UiBuilder::new().max_rect(rect), |ui| {
                            panel_style(ui, scale);
                            outer = panel_frame(scale).show(ui, |ui| {
                                ui.set_width(width - 24.0 * scale - 2.4);
                                ui.set_min_height(height - 24.0 * scale - 2.4);
                                panel_header(ui, "Sardinia et Corsica · provincial administration", scale, &campaign.economy.provinces, 0, &mut view);
                                scroll_body_with_status(ui, rect.bottom() - 12.0 * scale - 1.2, scale, &status, "shell_test_body", |ui| {
                                    if tab == CampaignTab::Province {
                                        diplomacy(ui, &mut campaign, 4, 0);
                                    } else {
                                        campaign_politics::show(ui, &mut campaign.senate, &mut campaign.actors, &campaign.profiles, &campaign.senate_config, 0, &mut campaign.espionage, &campaign.espionage_config, &[PLAYER_COLORS[0], PLAYER_COLORS[1]]);
                                    }
                                    recent_notifications(ui, &campaign, 0);
                                });
                            }).response.rect;
                        });
                    });
                    output.textures_delta.clear();
                    assert!(outer.right() <= rect.right() + 1.0, "{tab:?}: header/frame horizontal overflow at {width}px: {outer:?}");
                    assert!(outer.bottom() <= rect.bottom() + 1.0, "{tab:?}: frame/footer vertical overflow at {width}×{height}px: {outer:?}");
                }
            }
        }
    }
}

#[test]
fn diplomacy_legal_states_fit_desktop_and_compact_panels() {
    for (width, scale) in [(500.0, 1.0), (380.0, 0.85), (380.0, 1.0)] {
        for province in 0..5 {
            let mut campaign = fixture();
            assert_body_fits(&format!("Diplomacy province {province}"), width, scale, |ui| {
                diplomacy(ui, &mut campaign, province, 0);
                recent_notifications(ui, &campaign, 0);
            });
        }
    }
}

#[test]
fn senate_nomination_campaign_and_result_fit_desktop_and_compact_panels() {
    for (width, scale) in [(500.0, 1.0), (380.0, 0.85), (380.0, 1.0)] {
        for phase in 0..3 {
            let mut campaign = fixture();
            if phase > 0 {
                campaign.senate.campaign = Some(SenateCampaign {
                    candidate: 0,
                    ballot: Ballot::Praetor,
                    elapsed: 10,
                    support_spending: [50.0; 5],
                    opposition_spending: [25.0; 5],
                    bribery: [100.0; 5],
                    scandal_penalties: [0.05; 5],
                });
            }
            if phase == 2 {
                for _ in 0..2 {
                    campaign.senate.advance_month(
                        &mut campaign.actors,
                        &campaign.profiles,
                        &campaign.senate_config,
                    );
                }
            }
            assert_body_fits(&format!("Senate phase {phase}"), width, scale, |ui| {
                campaign_politics::show(
                    ui,
                    &mut campaign.senate,
                    &mut campaign.actors,
                    &campaign.profiles,
                    &campaign.senate_config,
                    0,
                    &mut campaign.espionage,
                    &campaign.espionage_config,
                    &[PLAYER_COLORS[0], PLAYER_COLORS[1]],
                );
                recent_notifications(ui, &campaign, 0);
            });
        }
    }
}

#[test]
fn economy_overview_policies_buildings_and_trade_fit_compact_panels() {
    for (width, scale) in [(500.0, 1.0), (380.0, 0.85), (380.0, 1.0)] {
        for section in 0..4 {
            let mut campaign = fixture();
            assert_body_fits(
                &format!("Economy section {section}"),
                width,
                scale,
                |ui| match section {
                    0 => campaign_economy::overview(ui, &campaign.economy, 0),
                    1 => campaign_economy::policies(ui, &mut campaign.economy, 0, 0),
                    2 => {
                        campaign_economy::buildings(ui, &mut campaign.economy, 0, 0);
                    },
                    _ => {
                        let inputs = campaign.inputs();
                        campaign_economy::trade(ui, &mut campaign.economy, &inputs, 2, 0);
                    },
                },
            );
        }
    }
}

#[test]
fn military_roster_and_active_battle_fit_desktop_and_compact_panels() {
    for (width, scale) in [(500.0, 1.0), (380.0, 0.85), (380.0, 1.0)] {
        for in_battle in [false, true] {
            let mut campaign = fixture();
            let province = if in_battle {
                1
            } else {
                0
            };
            if in_battle {
                campaign
                    .military
                    .seed_unit(1, ForceOwner::Player(0), UnitType::LightCavalry)
                    .unwrap();
                campaign.military.seed_unit(1, ForceOwner::Player(0), UnitType::Archers).unwrap();
                campaign
                    .military
                    .start_battle(
                        1,
                        &[ForceOwner::Player(0)],
                        &[ForceOwner::Player(1)],
                        Some(1),
                        Some(0),
                        MilitaryTerrain::Hills,
                        2,
                        9,
                    )
                    .unwrap();
            }
            assert_body_fits(&format!("Military battle={in_battle}"), width, scale, |ui| {
                campaign_military::show(
                    ui,
                    &campaign.military,
                    &campaign.economy,
                    &campaign.graph,
                    province,
                    0,
                    |_, _| MilitaryAccess::Peaceful,
                    |a, b| campaign.forces_hostile(a, b),
                );
            });
        }
    }
}

#[test]
fn military_panel_actions_drive_paid_drafting_movement_locked_battle_and_retreat() {
    use campaign_military::MilitaryUiAction as Action;
    let mut c = fixture();
    let owner = ForceOwner::Player(0);
    c.military.config.units[UnitType::LightInfantry as usize].recruitment_months = 1.;
    let population = c.economy.provinces[0].population[2];
    let metal = c.economy.players[0].resources[1];
    assert_eq!(
        apply_military_action(&mut c, 0, 0, Action::Recruit(UnitType::LightInfantry)),
        "Order accepted."
    );
    assert_eq!(c.economy.provinces[0].population[2], population - 10.);
    assert_eq!(c.economy.players[0].resources[1], metal - 12.);
    assert_eq!(apply_military_action(&mut c, 0, 0, Action::CancelRecruitment), "Order accepted.");
    assert_eq!(
        c.economy.provinces[0].population[2],
        population - 10.,
        "cancel must not refund drafted population"
    );
    assert_eq!(
        apply_military_action(&mut c, 0, 0, Action::Recruit(UnitType::LightInfantry)),
        "Order accepted."
    );
    c.advance_month();
    let recruit = c.military.provinces[0].forces[&owner].last().unwrap().clone();
    let civilians = c.economy.provinces[0].population[2];
    assert_eq!(apply_military_action(&mut c, 0, 0, Action::Disband(recruit.id)), "Order accepted.");
    assert_eq!(c.economy.provinces[0].population[2], civilians + recruit.current_manpower);
    let plan = BattlePlan {
        tactic: CombatTactic::Envelopment,
        ..Default::default()
    };
    assert_eq!(apply_military_action(&mut c, 0, 0, Action::SavePlan(plan)), "Order accepted.");
    let unit = c.military.provinces[0].forces[&owner]
        .iter()
        .find(|unit| unit.unit_type == UnitType::HeavyInfantry)
        .unwrap()
        .id;
    c.declare_hostility(0, 1);
    assert_eq!(
        apply_military_action(
            &mut c,
            0,
            0,
            Action::Move {
                destination: 1,
                units: vec![unit],
                route: vec![1],
                plan
            }
        ),
        "Order accepted."
    );
    assert!(c.military.provinces[0].forces[&owner].iter().all(|cohort| cohort.id != unit));
    let moving = c.military.movements[0].id;
    let override_plan = BattlePlan {
        tactic: CombatTactic::ShockAction,
        ..plan
    };
    assert_eq!(
        apply_military_action(
            &mut c,
            0,
            0,
            Action::SaveMovementPlan {
                order: moving,
                plan: override_plan
            }
        ),
        "Order accepted."
    );
    assert_eq!(
        c.military.provinces[0].plans[&owner], plan,
        "moving override must preserve stationary default"
    );
    c.military.config.base_manpower_damage = 0.;
    c.military.config.base_morale_damage = 0.;
    c.advance_month();
    let battle = &c.military.battles[0];
    let battle_id = battle.id;
    assert_eq!(battle.attackers.plans[&owner], override_plan);
    assert_ne!(apply_military_action(&mut c, 1, 0, Action::SavePlan(plan)), "Order accepted.");
    assert_eq!(
        apply_military_action(
            &mut c,
            1,
            0,
            Action::Retreat {
                battle: battle_id,
                attacker: true
            }
        ),
        "Order accepted."
    );
    c.advance_month();
    assert!(c.military.battles.is_empty());
    assert!(c.military.provinces[0].forces[&owner].iter().any(|cohort| cohort.id == unit));
    assert_eq!(c.economy.provinces[1].owner, Some(1));
}

#[test]
fn saved_senate_vote_notice_opens_the_chamber_for_each_local_player() {
    use super::super::campaign_notifications::{NoticeAction, NoticeKind};
    let mut c = fixture();
    c.senate.campaign = Some(SenateCampaign {
        candidate: 0,
        ballot: Ballot::Praetor,
        elapsed: 11,
        support_spending: [0.; 5],
        opposition_spending: [0.; 5],
        bribery: [0.; 5],
        scandal_penalties: [0.; 5],
    });
    c.advance_month();
    assert!(
        c.senate.last_result.is_some(),
        "notification must refer to a stored authoritative vote"
    );
    for player in 0..2 {
        let result = c
            .notifications
            .history_for(player)
            .find(|notice| notice.kind == NoticeKind::SenateVoteResolved)
            .unwrap();
        assert_eq!(result.action, NoticeAction::OpenSenate);
        assert_eq!(result.province, None);
    }
}
