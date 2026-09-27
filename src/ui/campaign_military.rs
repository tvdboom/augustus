//! Campaign military details, matching the enclosing parchment panel palette.
//!
//! Rendering returns a typed action so the campaign bridge owns transactional
//! diplomacy, civilian population, and global-resource mutations.

use std::collections::BTreeSet;

use super::campaign_widgets::{icon, stat, Icon};
use bevy_egui::egui;

use crate::game::economy::{BuildingType, EconomyWorld};
use crate::game::military::*;

/// One player intent dispatched by the campaign bridge after rendering.
pub(in crate::app) enum MilitaryUiAction {
    /// Draft the selected type in the current province.
    Recruit(UnitType),
    /// Cancel the current project without a refund.
    CancelRecruitment,
    /// Return surviving soldiers to the local civilian class.
    Disband(UnitId),
    /// Save stationary force preferences.
    SavePlan(BattlePlan),
    /// Update a moving force's plan before engagement.
    SaveMovementPlan {
        /// Transient order identity.
        order: u64,
        /// Updated snapshot, retaining existing unit selection and route.
        plan: BattlePlan,
    },
    /// Move a unique selection using a snapshot of the displayed plan.
    Move {
        /// Final province destination.
        destination: usize,
        /// Stable selected troop identities.
        units: Vec<UnitId>,
        /// Every ordered adjacent crossing, already previewed for the player.
        route: Vec<ProvinceId>,
        /// Formation/tactic captured for the route.
        plan: BattlePlan,
    },
    /// Request retreat at the next round boundary.
    Retreat {
        /// Active battle identity.
        battle: u64,
        /// Coalition containing the player.
        attacker: bool,
    },
}

/// UI-only selection persisted separately for every player/province.
#[derive(Clone, Default)]
struct Selection {
    units: BTreeSet<UnitId>,
    destination: Option<usize>,
    waypoints: Vec<ProvinceId>,
    next_waypoint: Option<ProvinceId>,
}

/// Render live military facts and actionable recruitment/formation/movement controls.
pub(in crate::app) fn show(
    ui: &mut egui::Ui,
    world: &MilitaryWorld,
    economy: &EconomyWorld,
    graph: &[MilitaryProvince],
    province: usize,
    player: usize,
    access: impl Fn(ForceOwner, usize) -> MilitaryAccess,
    hostile: impl Fn(ForceOwner, ForceOwner) -> bool,
) -> Option<MilitaryUiAction> {
    let state = world.provinces.get(province)?;
    let economic = economy.provinces.get(province)?;
    let owner = ForceOwner::Player(player);
    let directly_owned = economic.owner == Some(player);
    let in_battle = world.province_in_battle(province);
    let mut action = None;
    let selection_id = egui::Id::new(("campaign-military-selection", province, player));
    let mut selection =
        ui.ctx().data_mut(|data| data.get_temp::<Selection>(selection_id)).unwrap_or_default();
    ui.horizontal_wrapped(|ui| {
        icon(ui, Icon::MilitaryPower, 28.);
        ui.strong(world.rank(owner).name());ui.label(format!("{:.0} Renown",world.renown.get(&owner).copied().unwrap_or(0.)))
            .on_hover_text("Military Renown advances Centurion → Military Tribune → Legate → Imperator. It improves morale, garrison effectiveness, and structural Military Senate support; political rank remains separate.");
    });
    if let Some(occupier) = state.occupation {
        ui.horizontal_wrapped(|ui| {
            stat(ui,Icon::Control,&format!("+{:.1}/mo",world.occupation_control(province,occupier)),&format!("{} occupies this province. Occupation costs {:.0} Relation/month. Peaceful access produces no Control.",owner_name(occupier),world.config.occupation_relation_loss));
            ui.small("Occupied");
        });
    }
    if let Some(battle) = world.battles.iter().find(|battle| battle.province == province) {
        battle_view(ui, battle, owner, &world.config, &mut action);
    }
    let mut disband = None;
    for (&force_owner, units) in &state.forces {
        if units.is_empty() {
            continue;
        }
        ui.separator();
        ui.horizontal_wrapped(|ui| {
            ui.strong(owner_name(force_owner));
            let (status,explanation)=if state.occupation==Some(force_owner) {
                ("Occupied","Hostile occupation generates Control. Peaceful presence never does.")
            } else if hostile(owner,force_owner) {
                ("Hostile","This force is at war with you. Coexisting hostile forces enter combat.")
            } else if matches!(force_owner,ForceOwner::Player(p) if economic.owner==Some(p)) {
                ("Owned","This force is stationed in its own province.")
            } else if access(force_owner,province)==MilitaryAccess::Peaceful {
                ("Peaceful","Military access permits stationing without occupation or Control gain.")
            } else if access(force_owner,province)==MilitaryAccess::Invasion {
                ("Invading","This force has declared hostility against the province.")
            } else {
                ("Access revoked","This force no longer has permission to enter. It receives no occupation Control merely by remaining here.")
            };
            ui.small(status).on_hover_text(explanation);
            stat(
                ui,
                Icon::Population,
                &format!("{:.1}", units.iter().map(|u| u.current_manpower).sum::<f64>()),
                "Total surviving manpower. Casualties never automatically recover.",
            );
            stat(
                ui,
                Icon::Food,
                &format!(
                    "{:.1}/mo",
                    units.iter().map(|u| u.food_demand(&world.config)).sum::<f64>()
                ),
                "Food upkeep for this stationary force.",
            );
        });
        for (index, unit) in units.iter().enumerate() {
            ui.push_id(unit.id,|ui| {
                stripe(ui,index,|ui| {
                    let controllable=force_owner==owner&&!in_battle;
                    ui.horizontal(|ui| {
                        if controllable {
                            let mut selected=selection.units.contains(&unit.id);
                            if ui.checkbox(&mut selected,"").on_hover_text("Select cohort for movement").changed() {
                                if selected{selection.units.insert(unit.id);}else{selection.units.remove(&unit.id);}
                            }
                        }
                        cohort_facts(ui,unit,&world.config,None,if controllable&&directly_owned{24.}else{0.});
                        if controllable&&directly_owned&&ui.small_button("×").on_hover_text("Disband: return surviving soldiers to their original civilian class. Metal is not refunded.").clicked(){disband=Some(unit.id);}
                    });
                });
            });
        }
    }
    if let Some(id) = disband {
        action = Some(MilitaryUiAction::Disband(id));
    }
    let own_units = state.forces.get(&owner).cloned().unwrap_or_default();
    selection.units.retain(|id| own_units.iter().any(|u| u.id == *id));
    for movement in world
        .movements
        .iter()
        .filter(|m| m.owner == owner && (m.origin == province || m.destination() == Some(province)))
    {
        let destination = movement
            .destination()
            .and_then(|p| economy.provinces.get(p))
            .map(|p| p.name.as_str())
            .unwrap_or("destination");
        ui.separator();
        ui.horizontal_wrapped(|ui| {
            icon(ui, Icon::Province, 22.);
            ui.strong(destination);
            ui.small(format!("{} cohorts", movement.units.len()));
            unit_composition(ui, &movement.units);
        });
        ui.add(
            egui::ProgressBar::new(movement.interpolation() as f32).text(format!(
                "{:.0} / {:.1} months",
                movement.progress, movement.required_progress
            )),
        );
        egui::CollapsingHeader::new("Movement battle plan")
            .id_salt(("moving-plan", movement.id))
            .show(ui, |ui| {
                let mut plan = movement.plan;
                edit_plan(ui, &mut plan, movement.id as usize + 10_000);
                if plan != movement.plan {
                    action = Some(MilitaryUiAction::SaveMovementPlan {
                        order: movement.id,
                        plan,
                    });
                }
            });
    }
    if directly_owned {
        ui.separator();
        ui.horizontal_wrapped(|ui| {
            ui.strong("RECRUITMENT");
            stat(
                ui,
                Icon::Plebeians,
                &format!("{:.1}", economic.population[2]),
                "Plebeians available for recruitment.",
            );
            stat(
                ui,
                Icon::Citizens,
                &format!("{:.1}", economic.population[1]),
                "Citizens available for recruitment.",
            );
        });
        egui::CollapsingHeader::new(if state.recruitment.is_some(){"Recruitment in progress"}else{"Available cohorts"})
            .id_salt(("military-recruitment-table",province))
            .default_open((own_units.is_empty()&&!in_battle)||state.recruitment.is_some())
            .show(ui,|ui| {
        if let Some(project) = &state.recruitment {
            stripe(ui, 0, |ui| {
                ui.horizontal(|ui| {
                    icon(ui,Icon::Unit(project.unit_type),42.);
                    let width=(ui.available_width()-32.).max(40.);
                    ui.allocate_ui_with_layout(egui::vec2(width,44.),egui::Layout::top_down(egui::Align::Min),|ui| {
                        ui.strong(project.unit_type.name());
                        ui.add(egui::ProgressBar::new((project.progress/project.required_progress.max(1.)) as f32).desired_width(width).text(format!("{:.0} / {:.0} months",project.progress,project.required_progress)));
                    });
                    if project.owner==owner&&ui.small_button("×").on_hover_text("Cancel recruitment. Drafted population and Metal are not refunded.").clicked(){action=Some(MilitaryUiAction::CancelRecruitment);}
                });
            });
        } else {
            for (index, kind) in UnitType::ALL.into_iter().enumerate() {
                let definition = world.config.unit(kind);
                let class = if definition.manpower_class == 1 {
                    "Citizens"
                } else {
                    "Plebeians"
                };
                let missing_tag = definition
                    .special_tag
                    .is_some_and(|tag| !recruitment_tags(&economic.name).contains(&tag));
                let metal = economy.players.get(player).map(|p| p.resources[1]).unwrap_or(0.);
                let reason = if in_battle {
                    Some("Recruitment is unavailable during battle")
                } else if missing_tag {
                    Some("This province lacks the required recruitment tradition")
                } else if economic.population[definition.manpower_class] < definition.manpower {
                    Some("Not enough people in the recruitment class")
                } else if metal < definition.metal_cost {
                    Some("Not enough global Metal")
                } else {
                    None
                };
                let draft = world.config.maximum_draft_penalty.min(
                    definition.manpower / economic.population[definition.manpower_class].max(0.001)
                        * world.config.draft_happiness_scale,
                );
                let tooltip=reason.map(str::to_owned).unwrap_or_else(||format!("Draft immediately removes civilians and equipment. {class} happiness temporarily falls by {draft:.1}; the penalty decays by {:.0}/month. Recruitment and construction have separate slots.",world.config.draft_penalty_decay));
                stripe(ui, index, |ui| {
                    ui.horizontal(|ui| {
                    icon(ui,Icon::Unit(kind),40.).on_hover_text(unit_definition_tip(kind,&world.config));
                    let width=(ui.available_width()-30.).max(40.);
                    ui.allocate_ui_with_layout(egui::vec2(width,42.),egui::Layout::top_down(egui::Align::Min),|ui| {
                        ui.horizontal_wrapped(|ui| {
                            ui.strong(kind.name());
                            ui.small(format!("{:.0} mo",definition.recruitment_months)).on_hover_text("Recruitment duration. Construction uses a separate slot.");
                        });
                        ui.horizontal_wrapped(|ui| {
                            stat(ui,if definition.manpower_class==1{Icon::Citizens}else{Icon::Plebeians},&format!("{:.0}",definition.manpower),&format!("{class} drafted immediately. Happiness penalty: {draft:.1}, decaying by {:.0} per month.",world.config.draft_penalty_decay));
                            stat(ui,Icon::Metal,&format!("{:.0}",definition.metal_cost),"Metal equipment cost, paid once.");
                            stat(ui,Icon::Food,&format!("{:.0}/mo",definition.food_per_month),"Food upkeep at full manpower. Falls with casualties.");
                        });
                    });
                    if ui.add_enabled(reason.is_none(),egui::Button::new("+")).on_hover_text(tooltip).clicked(){action=Some(MilitaryUiAction::Recruit(kind));}
                });
                });
            }
        }
        });
    }
    if !own_units.is_empty() && !in_battle {
        ui.separator();
        ui.strong("BATTLE PLAN");
        let mut plan = state.plans.get(&owner).copied().unwrap_or_default();
        let previous = plan;
        edit_plan(ui, &mut plan, province);
        if plan != previous {
            action = Some(MilitaryUiAction::SavePlan(plan));
        }
        let selected: Vec<_> =
            own_units.iter().filter(|u| selection.units.contains(&u.id)).cloned().collect();
        ui.horizontal_wrapped(|ui| {
            if ui.small_button("Select all").clicked() {
                selection.units = own_units.iter().map(|u| u.id).collect();
            }
            ui.label(format!("{} selected", selection.units.len()));
        });
        egui::ComboBox::from_id_salt(("military-destination", province))
            .width(ui.available_width().min(260.))
            .selected_text(
                selection
                    .destination
                    .and_then(|id| economy.provinces.get(id))
                    .map(|p| p.name.as_str())
                    .unwrap_or("Choose destination"),
            )
            .show_ui(ui, |ui| {
                for (id, province_data) in
                    economy.provinces.iter().enumerate().filter(|(id, _)| *id != province)
                {
                    let permission = access(owner, id);
                    let label = match permission {
                        MilitaryAccess::Peaceful => "Move",
                        MilitaryAccess::Invasion => "Invade",
                        MilitaryAccess::Blocked => "Blocked",
                    };
                    ui.add_enabled_ui(permission != MilitaryAccess::Blocked, |ui| {
                        ui.selectable_value(
                            &mut selection.destination,
                            Some(id),
                            format!("{} · {label}", province_data.name),
                        );
                    });
                }
            });
        if let Some(destination) =
            selection.destination.filter(|&p| p < graph.len() && p < economy.provinces.len())
        {
            egui::CollapsingHeader::new("Choose route / waypoints")
                .id_salt(("military-route",province)).show(ui,|ui| {
                    ui.label("Via").on_hover_text("Waypoints are visited in order using the fastest legal crossings between them. An empty list chooses the fastest complete route.");
                    let mut remove = None;
                    let mut move_up = None;
                    for (index,&waypoint) in selection.waypoints.iter().enumerate() {
                        ui.horizontal_wrapped(|ui| {
                            ui.label(format!("{}. {}",index+1,economy.provinces[waypoint].name));
                            if index>0 && ui.small_button("Earlier").clicked() {move_up=Some(index);}
                            if ui.small_button("Remove").clicked() {remove=Some(index);}
                        });
                    }
                    if let Some(index)=remove {selection.waypoints.remove(index);}
                    else if let Some(index)=move_up {selection.waypoints.swap(index,index-1);}
                    egui::ComboBox::from_id_salt(("military-waypoint",province))
                        .width(ui.available_width().min(260.))
                        .selected_text(selection.next_waypoint.and_then(|id|economy.provinces.get(id)).map(|p|p.name.as_str()).unwrap_or("Choose intermediate province"))
                        .show_ui(ui,|ui| {
                            for (id,p) in economy.provinces.iter().enumerate().filter(|(id,_)|*id!=province&&*id!=destination&&!selection.waypoints.contains(id)) {
                                if access(owner,id)==MilitaryAccess::Peaceful {ui.selectable_value(&mut selection.next_waypoint,Some(id),&p.name);}
                            }
                        });
                    ui.horizontal_wrapped(|ui| {
                        if ui.add_enabled(selection.next_waypoint.is_some(),egui::Button::new("Add waypoint")).clicked() {
                            if let Some(waypoint)=selection.next_waypoint.take() {
                                if waypoint!=province&&waypoint!=destination&&!selection.waypoints.contains(&waypoint){selection.waypoints.push(waypoint);}
                            }
                        }
                        if !selection.waypoints.is_empty()&&ui.small_button("Fastest route").clicked(){selection.waypoints.clear();}
                    });
                });
            let route = route_via(
                graph,
                province,
                destination,
                &selection.waypoints,
                owner,
                &selected,
                &access,
                &world.config,
            );
            let enemy = known_hostile_units(world, destination, owner, &hostile);
            let fort = economy.provinces[destination].level(BuildingType::Fort)
                + economy.provinces[destination].level(BuildingType::CityWalls);
            let enemy_rank = enemy.first().map(|u| world.rank(u.owner)).unwrap_or_default();
            let estimate = estimate_battle(
                &selected,
                &enemy,
                plan,
                graph[destination].terrain,
                fort,
                world.rank(owner),
                enemy_rank,
                &world.config,
            );
            ui.small(format!(
                "{:?} · {} slots · {} flanks/side",
                graph[destination].terrain,
                world.config.combat_widths[graph[destination].terrain as usize],
                estimate.formation.flank_size
            ));
            formation_row(ui, &estimate.formation, &selected);
            if access(owner, destination) == MilitaryAccess::Invasion || !enemy.is_empty() {
                ui.horizontal_wrapped(|ui| {
                    stat(ui,Icon::MilitaryPower,&format!("{:.0}%",estimate.tactic_fit*100.),"Your tactic's composition fit, based on surviving cohort strength.");
                    stat(ui,Icon::Building(BuildingType::Fort),&fort.to_string(),"Completed Fort and City Wall levels affecting the defender.");
                    ui.small("Enemy tactic: Unknown").on_hover_text("The estimate uses public troops and terrain; it assumes no hidden enemy tactic or counter bonus.");
                });
                ui.strong(estimate.assessment).on_hover_text(estimate.reasons.join("\n"));
                egui::CollapsingHeader::new(format!("Scouting · {} hostile cohorts", enemy.len()))
                    .id_salt(("military-scouting", province, destination))
                    .show(ui, |ui| {
                        if enemy.is_empty() {
                            ui.small("No known hostile defenders");
                        }
                        for enemy_owner in enemy.iter().map(|u| u.owner).collect::<BTreeSet<_>>() {
                            ui.horizontal_wrapped(|ui| {
                                ui.strong(owner_name(enemy_owner));
                                let rank = world.rank(enemy_owner);
                                ui.small(rank.name()).on_hover_text(format!(
                                    "Public Military Rank: +{:.0} Morale.",
                                    world.config.rank_morale[rank as usize]
                                ));
                            });
                            for (index, unit) in
                                enemy.iter().filter(|u| u.owner == enemy_owner).enumerate()
                            {
                                stripe(ui, index, |ui| {
                                    ui.horizontal(|ui| {
                                        cohort_facts(ui, unit, &world.config, None, 0.);
                                    });
                                });
                            }
                        }
                    });
            }
            match route {
                Ok(route) => {
                    let route_names = std::iter::once(province)
                        .chain(route.iter().copied())
                        .map(|id| economy.provinces[id].name.as_str())
                        .collect::<Vec<_>>()
                        .join(" → ");
                    ui.small(route_names).on_hover_text("This exact sequence of crossings will be issued. Each edge is revalidated as it is reached.");
                    let mut current = province;
                    let mut months = 0.;
                    let speed = force_speed(&selected, &world.config);
                    let median = median_province_area(graph);
                    for &next in &route {
                        months += edge_travel_months(
                            &graph[current],
                            &graph[next],
                            median,
                            speed,
                            &world.config,
                        )
                        .max(1.)
                        .ceil();
                        current = next;
                    }
                    ui.small(format!("{months:.0} months · {} crossings",route.len())).on_hover_text("Movement uses the slowest selected unit. Province area, terrain, and Road levels determine each crossing; access is rechecked at every arrival.");
                    let label = if access(owner, destination) == MilitaryAccess::Invasion {
                        "Invade"
                    } else {
                        "Move selected units"
                    };
                    if ui.button(label).clicked() {
                        action = Some(MilitaryUiAction::Move {
                            destination,
                            units: selection.units.iter().copied().collect(),
                            route,
                            plan,
                        });
                    }
                },
                Err(error) => {
                    ui.small(error.to_string());
                },
            }
        }
    }
    ui.ctx().data_mut(|data| data.insert_temp(selection_id, selection));
    action
}

/// Render constrained type preferences with no manual individual-slot placement.
fn edit_plan(ui: &mut egui::Ui, plan: &mut BattlePlan, province: usize) {
    egui::ComboBox::from_id_salt(("military-tactic",province)).width(ui.available_width().min(220.)).selected_text(plan.tactic.name()).show_ui(ui,|ui| {
        for tactic in CombatTactic::ALL{ui.selectable_value(&mut plan.tactic,tactic,tactic.name());}
    }).response.on_hover_text("Balanced is neutral. Bottleneck counters Shock; Shock counters Deception; Deception counters Skirmishing; Skirmishing counters Envelopment; Envelopment counters Bottleneck. Fit depends on actual surviving units. Tactics lock on engagement.");
    for (label, value, flank) in [
        ("Primary line", &mut plan.primary_unit_type, false),
        ("Secondary line", &mut plan.secondary_unit_type, false),
        ("Flanking unit", &mut plan.flank_unit_type, true),
    ] {
        ui.horizontal(|ui| {
            icon(ui, Icon::Unit(*value), 28.);
            ui.add_sized([96., 28.], egui::Label::new(label));
            egui::ComboBox::from_id_salt((label, province))
                .width(ui.available_width().min(180.))
                .selected_text(value.name())
                .show_ui(ui, |ui| {
                    for kind in UnitType::ALL {
                        let eligible = if flank {
                            matches!(
                                kind,
                                UnitType::LightCavalry
                                    | UnitType::HeavyCavalry
                                    | UnitType::HorseArchers
                                    | UnitType::WarCamels
                                    | UnitType::WarChariots
                                    | UnitType::LightInfantry
                            )
                        } else {
                            !kind.is_support()
                        };
                        if eligible {
                            ui.horizontal(|ui| {
                                icon(ui, Icon::Unit(kind), 24.);
                                ui.selectable_value(value, kind, kind.name());
                            });
                        }
                    }
                });
        });
    }
    ui.horizontal_wrapped(|ui| {
        ui.label("Flank width").on_hover_text("Slots per side. A larger flank consumes center frontage. Narrow terrain preserves at least two center slots. Maneuver determines sideways attack reach independently.");
        for size in 1..=3{ui.selectable_value(&mut plan.flank_size,size,size.to_string());}
    });
}

/// Image slots retain exact frontage alignment and show strength without a text diagram.
fn formation_row(ui: &mut egui::Ui, formation: &Formation, units: &[Unit]) {
    let count = formation.front.len().max(1);
    let cell = ((ui.available_width() - 2. * count.saturating_sub(1) as f32) / count as f32)
        .clamp(10., 38.);
    for (is_support, row) in [(false, &formation.front), (true, &formation.support)] {
        if is_support && !row.iter().any(Option::is_some) {
            continue;
        }
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 2.;
            for (index, id) in row.iter().enumerate() {
                let unit = id.and_then(|id| units.iter().find(|u| u.id == id));
                let flank = !is_support
                    && (index < formation.flank_size
                        || index >= count.saturating_sub(formation.flank_size));
                let (rect, response) =
                    ui.allocate_exact_size(egui::vec2(cell, cell + 7.), egui::Sense::hover());
                ui.painter().rect_filled(
                    rect,
                    2.,
                    if flank {
                        egui::Color32::from_rgb(226, 210, 182)
                    } else {
                        egui::Color32::from_rgb(244, 239, 225)
                    },
                );
                ui.painter().rect_stroke(
                    rect,
                    2.,
                    egui::Stroke::new(0.6, egui::Color32::from_rgb(191, 171, 143)),
                    egui::StrokeKind::Inside,
                );
                if let Some(unit) = unit {
                    let image = egui::Rect::from_min_size(
                        rect.min + egui::vec2(1., 1.),
                        egui::vec2(cell - 2., cell - 2.),
                    );
                    ui.painter().image(
                        crate::map::military_unit_icon(ui.ctx(), unit.unit_type),
                        image,
                        egui::Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(0.25, 0.25)),
                        egui::Color32::WHITE,
                    );
                    let bar = egui::Rect::from_min_size(
                        egui::pos2(rect.left() + 3., rect.bottom() - 5.),
                        egui::vec2((cell - 6.).max(1.), 2.),
                    );
                    ui.painter().rect_filled(bar, 1., egui::Color32::from_rgb(204, 188, 162));
                    ui.painter().rect_filled(
                        egui::Rect::from_min_size(
                            bar.min,
                            egui::vec2(bar.width() * unit.manpower_ratio() as f32, bar.height()),
                        ),
                        1.,
                        egui::Color32::from_rgb(104, 122, 83),
                    );
                    response.on_hover_text(format!(
                        "{} · {}\n{:.1}/{:.1} manpower\nTraining {:.0} · Morale {:.0}",
                        unit.unit_type.name(),
                        if is_support {
                            "Support"
                        } else if flank {
                            "Flank"
                        } else {
                            "Center"
                        },
                        unit.current_manpower,
                        unit.max_manpower,
                        unit.training,
                        unit.morale
                    ));
                } else {
                    ui.painter().circle_filled(
                        rect.center(),
                        1.5,
                        egui::Color32::from_rgb(191, 171, 143),
                    );
                    response.on_hover_text(if is_support {
                        "Empty support slot"
                    } else if flank {
                        "Empty flank slot"
                    } else {
                        "Empty center slot"
                    });
                }
            }
        });
    }
    if !formation.reserves.is_empty() {
        ui.horizontal_wrapped(|ui| {
            ui.small(format!("Reserve {}",formation.reserves.len())).on_hover_text("Reserves fill vacancies after casualties or routing; surviving deployed units retain their positions.");
            let reserves:Vec<_>=units.iter().filter(|u|formation.reserves.contains(&u.id)).cloned().collect();
            unit_composition(ui,&reserves);
        });
    }
}

/// Battle facts keep both sides' locked plans and casualties visible.
fn battle_view(
    ui: &mut egui::Ui,
    battle: &Battle,
    owner: ForceOwner,
    config: &MilitaryConfig,
    action: &mut Option<MilitaryUiAction>,
) {
    ui.separator();
    ui.horizontal_wrapped(|ui| {
        icon(ui, Icon::Attack, 28.);
        ui.strong("BATTLE");
        ui.small(format!("Month {} · round {}", battle.months + 1, battle.round));
    });
    let siege: f64 = battle
        .attackers
        .formation
        .front
        .iter()
        .chain(&battle.attackers.formation.support)
        .flatten()
        .filter_map(|id| {
            battle
                .attackers
                .units
                .iter()
                .find(|u| u.id == *id && !battle.attackers.routed.contains(id))
        })
        .map(|u| config.unit(u.unit_type).siege_power * u.manpower_ratio())
        .sum();
    let fort = (f64::from(battle.fortification_level) * config.fort_defense_per_level)
        .min(config.fort_defense_cap);
    let suppression =
        (siege * config.siege_suppression_per_point).min(config.siege_suppression_cap);
    ui.horizontal_wrapped(|ui| {
        ui.small(format!("{:?} · {} slots",battle.terrain,config.combat_widths[battle.terrain as usize])).on_hover_text(format!("Defender terrain bonus: +{:.0}%",(config.terrain_defense[battle.terrain as usize]-1.)*100.));
        stat(ui,Icon::Building(BuildingType::Fort),&format!("+{:.0}%",fort*(1.-suppression)*100.),&format!("Level {} fortification: base defense +{:.0}%. Active siege power {:.1} suppresses {:.0}% of that bonus. Protected support fights at {:.0}% power; exposed support takes {:.0}% extra casualties.",battle.fortification_level,fort*100.,siege,suppression*100.,config.support_effectiveness*100.,(config.exposed_support_casualties-1.)*100.));
    });
    for (attacker, label, side) in
        [(true, "ATTACKERS", &battle.attackers), (false, "DEFENDERS", &battle.defenders)]
    {
        ui.horizontal_wrapped(|ui| {
            ui.strong(label);
            stat(
                ui,
                Icon::Population,
                &format!("{:.1}", side.manpower()),
                "Surviving manpower, including reserves and routed cohorts.",
            );
            let morale=side.units.iter().map(|unit|unit.morale*unit.current_manpower).sum::<f64>()/side.manpower().max(0.001);
            stat(ui,Icon::Morale,&format!("{morale:.0}"),"Current manpower-weighted Morale. Individual cohort Morale is shown in the expanded roster.");
            unit_composition(ui, &side.units);
        });
        let opposing = if attacker {
            &battle.defenders
        } else {
            &battle.attackers
        };
        for (&force_owner, plan) in &side.plans {
            let rank = side.ranks.get(&force_owner).copied().unwrap_or_default();
            let mut detail=format!("{}: {} gives +{:.0} Morale.\nTactic and formation are locked until battle ends. Fit follows actual surviving deployed units.",owner_name(force_owner),rank.name(),config.rank_morale[rank as usize]);
            for (&enemy, enemy_plan) in &opposing.plans {
                if plan.tactic.counters(enemy_plan.tactic, config) {
                    detail.push_str(&format!(
                        "\nCounters {}: +{:.1}% damage",
                        owner_name(enemy),
                        config.tactic_bonus * side.tactic_fit(force_owner, config) * 100.
                    ));
                } else if enemy_plan.tactic.counters(plan.tactic, config) {
                    detail.push_str(&format!(
                        "\nCountered by {}: {:.0}% damage",
                        owner_name(enemy),
                        config.countered_multiplier * 100.
                    ));
                }
            }
            ui.small(format!(
                "{} · {} · {:.0}% fit",
                owner_name(force_owner),
                plan.tactic.name(),
                side.tactic_fit(force_owner, config) * 100.
            ))
            .on_hover_text(detail);
        }
        formation_row(ui, &side.formation, &side.units);
        egui::CollapsingHeader::new(format!("{} cohorts", side.units.len()))
            .id_salt(("battle-units", battle.id, attacker))
            .show(ui, |ui| {
                for (index, unit) in side.units.iter().enumerate() {
                    let role = if side.routed.contains(&unit.id) {
                        "Routed"
                    } else if side.formation.support.contains(&Some(unit.id)) {
                        "Support"
                    } else if side.formation.reserves.contains(&unit.id) {
                        "Reserve"
                    } else if unit.unit_type.is_support() {
                        "Exposed support"
                    } else if side
                        .formation
                        .front
                        .iter()
                        .position(|id| *id == Some(unit.id))
                        .is_some_and(|slot| {
                            slot < side.formation.flank_size
                                || slot
                                    >= side
                                        .formation
                                        .front
                                        .len()
                                        .saturating_sub(side.formation.flank_size)
                        })
                    {
                        "Flank"
                    } else {
                        "Center"
                    };
                    stripe(ui, index, |ui| {
                        ui.horizontal(|ui| {
                            cohort_facts(ui, unit, config, Some(role), 0.);
                        });
                    });
                }
            });
        if side.units.iter().any(|u| u.owner == owner) {
            let allowed = battle.months >= config.minimum_retreat_months;
            if ui.add_enabled(allowed,egui::Button::new("Retreat")).on_hover_text("Available after one full combat month. Retreat occurs at the next round boundary. Troops with no legal adjacent retreat are destroyed.").clicked(){*action=Some(MilitaryUiAction::Retreat{battle:battle.id,attacker});}
        }
    }
}

/// Display owner identity without assuming a player-color or name provider.
fn owner_name(owner: ForceOwner) -> String {
    match owner {
        ForceOwner::Player(player) => format!("Player {}", player + 1),
        ForceOwner::Local(_) => "Local defenders".to_owned(),
    }
}

/// Alternate the same pale table colors used by the established province/city cards.
fn stripe(ui: &mut egui::Ui, index: usize, contents: impl FnOnce(&mut egui::Ui)) {
    let width = ui.available_width();
    egui::Frame::new()
        .fill(if index.is_multiple_of(2) {
            egui::Color32::from_rgb(244, 239, 225)
        } else {
            egui::Color32::from_rgb(238, 233, 219)
        })
        .inner_margin(egui::Margin::symmetric(5, 4))
        .show(ui, |ui| {
            ui.set_width((width - 10.).max(1.));
            ui.spacing_mut().item_spacing = egui::vec2(5., 3.);
            contents(ui);
        });
}

/// Show the identifying image and manpower beside two compact condition meters.
fn cohort_facts(
    ui: &mut egui::Ui,
    unit: &Unit,
    config: &MilitaryConfig,
    role: Option<&str>,
    reserved_width: f32,
) {
    icon(ui, Icon::Unit(unit.unit_type), 36.)
        .on_hover_text(unit_definition_tip(unit.unit_type, config));
    let meter_width = 84.;
    let text_width =
        (ui.available_width() - meter_width - reserved_width - ui.spacing().item_spacing.x * 2.)
            .max(40.);
    ui.allocate_ui_with_layout(egui::vec2(text_width,40.),egui::Layout::top_down(egui::Align::Min),|ui| {
        ui.add(egui::Label::new(egui::RichText::new(unit.unit_type.name()).strong()).wrap());
        ui.horizontal_wrapped(|ui| {
            ui.small(format!("{:.1}/{:.1}",unit.current_manpower,unit.max_manpower)).on_hover_text("Surviving / original manpower. Casualties are permanent; cohorts never reinforce automatically.");
            if let Some(role)=role{ui.small(role);}else{
                stat(ui,Icon::Food,&format!("{:.1}",unit.food_demand(config)),"Monthly Food upkeep. All units share the owner's supply ratio, including movement and combat.");
            }
        });
    });
    ui.allocate_ui_with_layout(egui::vec2(meter_width,40.),egui::Layout::top_down(egui::Align::Min),|ui| {
        condition_meter(ui,"T",unit.training,egui::Color32::from_rgb(157,117,61),&format!("Training {:.0}/100\nAttack +{:.1}%; defense +{:.1}%. Supplied troops gain {:.0} Training per month.",unit.training,config.training_attack*unit.training,config.training_defense*unit.training,config.passive_training));
        condition_meter(ui,"M",unit.morale,egui::Color32::from_rgb(107,127,88),&format!("Morale {:.0}/100\nTroops rout at zero. Food shortages lower Morale; peaceful supplied troops gradually recover it.",unit.morale));
    });
}

/// Paint a labeled number over a quiet parchment meter, without widening the row.
fn condition_meter(ui: &mut egui::Ui, label: &str, value: f64, color: egui::Color32, tip: &str) {
    let (rect, response) = ui
        .allocate_exact_size(egui::vec2(ui.available_width().min(84.), 17.), egui::Sense::hover());
    ui.painter().rect_filled(rect, 2., egui::Color32::from_rgb(217, 205, 184));
    ui.painter().rect_filled(
        egui::Rect::from_min_size(
            rect.min,
            egui::vec2(rect.width() * (value / 100.).clamp(0., 1.) as f32, rect.height()),
        ),
        2.,
        color.gamma_multiply(0.55),
    );
    ui.painter().text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        format!("{label}  {value:.0}"),
        egui::FontId::proportional(12.),
        egui::Color32::from_rgb(57, 43, 37),
    );
    response.on_hover_text(tip);
}

/// Group cohort images by type so force size is readable without eleven text labels.
fn unit_composition(ui: &mut egui::Ui, units: &[Unit]) {
    for kind in UnitType::ALL {
        let count = units.iter().filter(|u| u.unit_type == kind).count();
        if count > 0 {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 1.;
                icon(ui, Icon::Unit(kind), 26.)
                    .on_hover_text(format!("{} · {count} cohorts", kind.name()));
                if count > 1 {
                    ui.small(count.to_string());
                }
            });
        }
    }
}

/// Detailed combat and matchup values stay available on the unit image hover.
fn unit_definition_tip(kind: UnitType, config: &MilitaryConfig) -> String {
    let definition = config.unit(kind);
    let mut tip=format!("{}\nOffense {:.2} · Defense {:.2}\nSpeed {:.1} · Maneuver {}\nMorale damage ×{:.2} · Manpower damage ×{:.2}",kind.name(),definition.offense,definition.defense,definition.movement_speed,definition.maneuver,definition.morale_damage_taken,definition.manpower_damage_taken);
    if definition.siege_power > 0. {
        tip.push_str(&format!("\nSiege power {:.1}", definition.siege_power));
    }
    if let Some(tag) = definition.special_tag {
        tip.push_str(&format!("\nRecruitment tradition: {tag:?}"));
    }
    for opponent in UnitType::ALL {
        let adjustment = (config.matchups[kind as usize][opponent as usize] - 1.) * 100.;
        if adjustment.abs() > 0.01 {
            tip.push_str(&format!("\nAgainst {}: {adjustment:+.0}%", opponent.name()));
        }
    }
    tip
}

/// Scout public hostile cohorts wherever the province currently stores them.
/// Neutral guests are excluded, while an ongoing battle cannot hide defenders.
fn known_hostile_units(
    world: &MilitaryWorld,
    province: ProvinceId,
    viewer: ForceOwner,
    hostile: impl Fn(ForceOwner, ForceOwner) -> bool,
) -> Vec<Unit> {
    let stationed =
        world.provinces.get(province).into_iter().flat_map(|p| p.forces.values()).flatten();
    let engaged = world
        .battles
        .iter()
        .filter(|b| b.province == province)
        .flat_map(|b| b.attackers.units.iter().chain(&b.defenders.units));
    stationed
        .chain(engaged)
        .filter(|unit| unit.current_manpower > 0. && hostile(viewer, unit.owner))
        .cloned()
        .collect()
}

#[cfg(test)]
#[path = "../../tests/unit/military_ui.rs"]
mod tests;
