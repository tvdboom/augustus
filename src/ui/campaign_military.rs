//! Campaign military details, matching the enclosing parchment panel palette.
//!
//! Rendering returns a typed action so the campaign bridge owns transactional
//! diplomacy, civilian population, and global-resource mutations.

use std::collections::BTreeSet;

use super::campaign_widgets::{icon, paint_icon, stat, Icon};
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

#[derive(Clone, Copy, Default, PartialEq, Eq)]
enum MilitaryTab {
    #[default]
    Army,
    Recruitment,
}

/// Overview navigation always opens the army ledger, even after recruiting here.
pub(in crate::app) fn open_army_tab(ctx: &egui::Context, province: usize, player: usize) {
    ctx.data_mut(|data| {
        data.insert_temp(
            egui::Id::new(("province-military-tab", province, player)),
            MilitaryTab::Army,
        );
    });
}

/// One owner's province force, or one distinct force currently marching from it.
struct ArmyOverviewRow {
    province: ProvinceId,
    owner: ForceOwner,
    movement: Option<u64>,
    destination: Option<ProvinceId>,
    in_battle: bool,
    units: Vec<Unit>,
    tactic: Option<CombatTactic>,
}

impl ArmyOverviewRow {
    fn averages(&self) -> (f64, f64) {
        let manpower: f64 = self.units.iter().map(|unit| unit.current_manpower).sum();
        let average = |value: fn(&Unit) -> f64| {
            self.units.iter().map(|unit| value(unit) * unit.current_manpower).sum::<f64>()
                / manpower.max(0.001)
        };
        (average(|unit| unit.morale), average(|unit| unit.training))
    }
}

/// The caller supplies the current, visibility-filtered military view without dated reports.
fn army_overview_rows(world: &MilitaryWorld, player: usize) -> Vec<ArmyOverviewRow> {
    let own = ForceOwner::Player(player);
    let mut rows = Vec::new();
    for (province, state) in world.provinces.iter().enumerate() {
        let own_units = province_force_units(world, province, own);
        let marching = world.movements.iter().any(|movement| {
            movement.owner == own
                && movement.origin == province
                && movement.units.iter().any(|unit| unit.current_manpower > 0.)
        });
        if own_units.is_empty() && !marching {
            continue;
        }
        let owners: BTreeSet<_> = state
            .forces
            .keys()
            .copied()
            .chain(
                world
                    .battles
                    .iter()
                    .filter(|battle| battle.province == province)
                    .flat_map(|battle| battle.attackers.units.iter().chain(&battle.defenders.units))
                    .map(|unit| unit.owner),
            )
            .collect();
        for owner in owners {
            let units = if owner == own {
                own_units.clone()
            } else {
                province_force_units(world, province, owner)
            };
            if units.is_empty() {
                continue;
            }
            rows.push(ArmyOverviewRow {
                province,
                owner,
                movement: None,
                destination: None,
                in_battle: world.battles.iter().any(|battle| {
                    battle.province == province
                        && battle
                            .attackers
                            .units
                            .iter()
                            .chain(&battle.defenders.units)
                            .any(|unit| unit.owner == owner && unit.current_manpower > 0.)
                }),
                units,
                tactic: (owner == own).then(|| province_force_plan(world, province, owner).tactic),
            });
        }
    }
    for movement in world.movements.iter().filter(|movement| movement.owner == own) {
        let units: Vec<_> =
            movement.units.iter().filter(|unit| unit.current_manpower > 0.).cloned().collect();
        if units.is_empty() {
            continue;
        }
        rows.push(ArmyOverviewRow {
            province: movement.origin,
            owner: own,
            movement: Some(movement.id),
            destination: movement.destination(),
            in_battle: false,
            units,
            tactic: Some(movement.plan.tactic),
        });
    }
    rows.sort_by_key(|row| (row.province, row.owner != own, row.owner, row.movement));
    rows
}

/// National army ledger: read-only facts, with the whole row opening its province.
pub(in crate::app) fn overview(
    ui: &mut egui::Ui,
    world: &MilitaryWorld,
    economy: &EconomyWorld,
    player: usize,
    colors: &[egui::Color32],
    scale: f32,
) -> Option<ProvinceId> {
    use super::province_panel::{INK, PAPER, RULE, TABLE_STRIPE};
    let own = ForceOwner::Player(player);
    let mut rows = army_overview_rows(world, player);
    rows.retain(|row| row.province < economy.provinces.len());
    rows.sort_by(|a, b| {
        economy.provinces[a.province].name.cmp(&economy.provinces[b.province].name).then_with(
            || {
                (a.province, a.owner != own, a.owner, a.movement).cmp(&(
                    b.province,
                    b.owner != own,
                    b.owner,
                    b.movement,
                ))
            },
        )
    });
    let own_rows: Vec<_> = rows.iter().filter(|row| row.owner == own).collect();
    ui.strong(format!(
        "{} armies · {} cohorts",
        own_rows.len(),
        own_rows.iter().map(|row| row.units.len()).sum::<usize>()
    ));
    ui.small("Click an army to open its province. Other forces here are listed by owner.");
    ui.add_space(6. * scale);
    if own_rows.is_empty() {
        ui.label("You have no armies.");
        ui.small("Recruit units in a province's Military tab.");
        return None;
    }
    let width = (ui.available_width() - ui.spacing().scroll.allocated_width()).max(1.);
    let fractions = [0., 0.27, 0.56, 0.78, 0.89, 1.];
    let cells = |rect: egui::Rect| -> [egui::Rect; 5] {
        std::array::from_fn(|index| {
            egui::Rect::from_min_max(
                egui::pos2(rect.left() + width * fractions[index] + 5. * scale, rect.top()),
                egui::pos2(rect.left() + width * fractions[index + 1] - 5. * scale, rect.bottom()),
            )
        })
    };
    let text = |ui: &egui::Ui, cell: egui::Rect, label: &str, size: f32, color: egui::Color32| {
        // Wrap within the column, eliding long names before they reach the next label.
        let painter = ui.painter().with_clip_rect(ui.clip_rect().intersect(cell));
        let font = egui::FontId::proportional(size * scale);
        let row_height = ui.fonts_mut(|fonts| fonts.row_height(&font));
        let mut job = egui::text::LayoutJob::simple(label.to_owned(), font, color, cell.width());
        job.wrap.max_rows = (cell.height() / row_height).floor().max(1.) as usize;
        job.wrap.break_anywhere = job.wrap.max_rows == 1;
        let galley = painter.layout_job(job);
        painter.galley(cell.left_center() - egui::vec2(0., galley.size().y * 0.5), galley, color);
    };
    let (header, _) = ui.allocate_exact_size(egui::vec2(width, 46. * scale), egui::Sense::hover());
    ui.painter().rect_filled(header, 2. * scale, egui::Color32::from_rgb(73, 69, 61));
    for (cell, label) in cells(header).into_iter().zip([
        "Province / army",
        "Units",
        "Tactic",
        "Morale",
        "Avg. training",
    ]) {
        text(ui, cell, label, 12., PAPER);
    }
    let height = (ui.clip_rect().bottom() - ui.next_widget_position().y).max(0.);
    let mut selected = None;
    egui::ScrollArea::vertical().id_salt(("military-overview", player))
        .max_height(height).min_scrolled_height(0.).auto_shrink([false, false])
        .show(ui, |ui| {
            ui.set_width(width);
            for (index, row) in rows.iter().enumerate() {
                let composition: Vec<_> = UnitType::ALL.into_iter().filter_map(|kind| {
                    let count = row.units.iter().filter(|unit| unit.unit_type == kind).count();
                    (count > 0).then_some((kind, count))
                }).collect();
                let columns = ((width * (fractions[2] - fractions[1]) - 10. * scale)
                    / (48. * scale)).floor().max(1.) as usize;
                let height = (composition.len().div_ceil(columns) as f32 * 28. + 12.).max(76.) * scale;
                let (rect, response) = ui.allocate_exact_size(egui::vec2(width, height), egui::Sense::click());
                let response = response.on_hover_cursor(egui::CursorIcon::PointingHand);
                response.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true,
                    format!("{} · {} · open military", economy.provinces[row.province].name, owner_name(row.owner))));
                ui.painter().rect_filled(rect, 0., if response.hovered() || response.has_focus() {
                    egui::Color32::from_rgb(229, 217, 190)
                } else if index.is_multiple_of(2) { PAPER } else { TABLE_STRIPE });
                let cell = cells(rect);
                let color = match row.owner {
                    ForceOwner::Player(id) => colors.get(id).copied().unwrap_or(RULE),
                    ForceOwner::Local(_) => super::province_panel::NEUTRAL,
                };
                ui.painter().rect_filled(egui::Rect::from_min_size(rect.min, egui::vec2(3. * scale, height)), 0., color);
                let location = cell[0];
                let name_rect = egui::Rect::from_min_max(location.min + egui::vec2(0., 4. * scale),
                    egui::pos2(location.right(), location.top() + 34. * scale));
                text(ui, name_rect, &economy.provinces[row.province].name, 13., INK);
                let owner_rect = egui::Rect::from_min_max(
                    egui::pos2(location.left(), location.top() + 35. * scale),
                    egui::pos2(location.right(), location.top() + 52. * scale));
                text(ui, owner_rect, &if row.owner == own { "You".to_owned() } else { owner_name(row.owner) }, 12., INK);
                let status = if let Some(destination) = row.destination {
                    format!("Marching → {}", economy.provinces.get(destination).map(|p| p.name.as_str()).unwrap_or("destination"))
                } else if row.in_battle { "In battle".to_owned() } else { "Stationed".to_owned() };
                text(ui, egui::Rect::from_min_max(egui::pos2(location.left(), location.top() + 53. * scale), location.max),
                    &status, 11., INK.gamma_multiply(0.8));
                let unit_cell = cell[1];
                let painter = ui.painter().with_clip_rect(ui.clip_rect().intersect(unit_cell));
                for (unit_index, &(kind, count)) in composition.iter().enumerate() {
                    let min = unit_cell.min + egui::vec2((unit_index % columns) as f32 * 48. + 1.,
                        (unit_index / columns) as f32 * 28. + 6.) * scale;
                    let image = egui::Rect::from_min_size(min, egui::vec2(24., 24.) * scale);
                    paint_icon(ui, Icon::Unit(kind), image);
                    painter.text(min + egui::vec2(27., 12.) * scale, egui::Align2::LEFT_CENTER,
                        count.to_string(), egui::FontId::proportional(12. * scale), INK);
                }
                text(ui, cell[2], row.tactic.map_or("?", CombatTactic::name), 12., INK);
                let (morale, training) = row.averages();
                text(ui, cell[3], &format!("{morale:.0}%"), 13., INK);
                text(ui, cell[4], &format!("{training:.0}%"), 13., INK);
                ui.painter().hline(rect.x_range(), rect.bottom(), egui::Stroke::new(scale, RULE.gamma_multiply(0.3)));
                if response.clicked() { selected = Some(row.province); }
                response.on_hover_text(format!("{} · {}\n{} cohorts · {} surviving manpower\n{}\nMorale and training are weighted by surviving manpower.\n{}\nOpen this province's Military tab.",
                    economy.provinces[row.province].name, owner_name(row.owner), row.units.len(),
                    super::resource_hud::format_population(row.units.iter().map(|unit| unit.current_manpower).sum()),
                    composition.iter().map(|(kind, count)| format!("{}: {count}", kind.name())).collect::<Vec<_>>().join(" · "),
                    if row.tactic.is_none() { "Foreign tactics are hidden." } else { status.as_str() }));
            }
        });
    selected
}

/// Render live military facts and actionable recruitment/formation/movement controls.
pub(in crate::app) fn show(
    ui: &mut egui::Ui,
    world: &MilitaryWorld,
    economy: &EconomyWorld,
    graph: &[MilitaryProvince],
    province: usize,
    player: usize,
    observed: impl Fn(usize) -> bool,
    report_month: impl Fn(usize) -> Option<u32>,
    access: impl Fn(ForceOwner, usize) -> MilitaryAccess,
    hostile: impl Fn(ForceOwner, ForceOwner) -> bool,
) -> Option<MilitaryUiAction> {
    let state = world.provinces.get(province)?;
    let economic = economy.provinces.get(province)?;
    let owner = ForceOwner::Player(player);
    let directly_owned = economic.owner == Some(player);
    let in_battle = world.province_in_battle(province);
    let mut action = None;
    let tab_id = egui::Id::new(("province-military-tab", province, player));
    let mut tab =
        ui.ctx().data_mut(|data| data.get_temp::<MilitaryTab>(tab_id)).unwrap_or_default();
    ui.horizontal(|ui| {
        ui.selectable_value(&mut tab, MilitaryTab::Army, "Army");
        ui.selectable_value(&mut tab, MilitaryTab::Recruitment, "Recruit units");
        if state.recruitment.is_some() {
            ui.small("Recruitment in progress");
        }
    });
    ui.ctx().data_mut(|data| data.insert_temp(tab_id, tab));
    if tab == MilitaryTab::Recruitment {
        if directly_owned || economic.overlord == Some(player) {
            ui.small("Current administrative report");
        } else if let Some(month) = report_month(province) {
            ui.small(format!("Recruitment observed in month {month} · spy report"));
        }
        let height =
            (ui.clip_rect().bottom() - ui.next_widget_position().y - ui.spacing().item_spacing.y)
                .max(0.);
        return egui::ScrollArea::vertical()
            .id_salt(("province-recruitment-list", province, player))
            .max_height(height)
            .min_scrolled_height(0.)
            .auto_shrink([false, true])
            .show(ui, |ui| {
                recruitment_view(
                    ui,
                    world,
                    economy,
                    province,
                    player,
                    report_month(province).is_some(),
                )
            })
            .inner;
    }
    let selection_id = egui::Id::new(("campaign-military-selection", province, player));
    let mut selection =
        ui.ctx().data_mut(|data| data.get_temp::<Selection>(selection_id)).unwrap_or_default();
    let own_units = state.forces.get(&owner).cloned().unwrap_or_default();
    let army = province_force_units(world, province, owner);
    let mut plan = province_force_plan(world, province, owner);
    let displayed_owner = if directly_owned || !army.is_empty() {
        owner
    } else {
        state
            .forces
            .keys()
            .copied()
            .chain(world.battles.iter().filter(|battle| battle.province == province).flat_map(
                |battle| {
                    battle.attackers.plans.keys().chain(battle.defenders.plans.keys()).copied()
                },
            ))
            .find(|candidate| !province_force_units(world, province, *candidate).is_empty())
            .unwrap_or(owner)
    };
    let displayed_units = province_force_units(world, province, displayed_owner);
    if !directly_owned
        && displayed_units.is_empty()
        && !observed(province)
        && report_month(province).is_none()
    {
        ui.label("?");
    } else {
        army_summary(ui, &displayed_units, &economic.name, world, displayed_owner);
        if displayed_owner != owner {
            ui.small("Enemy tactic: ?");
        }
    }
    if !army.is_empty() || directly_owned {
        let previous = plan;
        ui.add_enabled_ui(!in_battle, |ui| {
            edit_plan(ui, &mut plan, province, &army, &world.config);
        });
        if plan != previous {
            action = Some(MilitaryUiAction::SavePlan(plan));
        }
        if in_battle {
            ui.small("Tactic and cohort preferences are locked until battle ends.");
        } else if let Some(location) = graph.get(province) {
            let width = world.config.combat_widths[location.terrain as usize];
            ui.small(format!(
                "{:?} · {width} combat width · {} flanks per side",
                location.terrain,
                plan.effective_flank_size(width)
            ));
            egui::CollapsingHeader::new("Deployment preview")
                .id_salt(("province-formation-preview", province, player))
                .show(ui, |ui| {
                    let formation = deploy_formation(
                        &army,
                        &[(owner, plan)].into(),
                        width,
                        &BTreeSet::new(),
                        &world.config,
                    );
                    formation_row(ui, &formation, &army);
                });
        }
    }
    // Keep the summary, tactic, preferences and sub-tabs above the scrolling ledger.
    let height =
        (ui.clip_rect().bottom() - ui.next_widget_position().y - ui.spacing().item_spacing.y)
            .max(0.);
    egui::ScrollArea::vertical().id_salt(("province-army-ledger", province, player))
        .max_height(height).min_scrolled_height(0.).auto_shrink([false, true])
        .show(ui, |ui| {
    if !directly_owned {
        if observed(province) {
            ui.small("Current troop observation");
        } else if let Some(month) = report_month(province) {
            ui.small(format!("Troops observed in month {month} · spy report"));
        }
    }
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
                &super::resource_hud::format_population(units.iter().map(|u| u.current_manpower).sum::<f64>()),
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
                cohort_row(ui,index,|ui| {
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
                edit_plan(
                    ui,
                    &mut plan,
                    movement.id as usize + 10_000,
                    &movement.units,
                    &world.config,
                );
                if plan != movement.plan {
                    action = Some(MilitaryUiAction::SaveMovementPlan {
                        order: movement.id,
                        plan,
                    });
                }
            });
    }
    if !own_units.is_empty() && !in_battle {
        ui.separator();
        ui.strong("ARMY ORDERS");
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
            let enemy_known = observed(destination) || report_month(destination).is_some();
            let fort = economy.provinces[destination].level(BuildingType::CityWalls);
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
                    stat(
                        ui,
                        Icon::MilitaryPower,
                        &format!("{:.0}%", estimate.tactic_fit * 100.),
                        "Your tactic's composition fit, based on surviving cohort strength.",
                    );
                    stat(
                        ui,
                        Icon::Building(BuildingType::CityWalls),
                        &fort.to_string(),
                        "Completed Walls levels affecting the defender.",
                    );
                    ui.small("Enemy tactic: ?");
                });
                if enemy_known {
                    ui.small(if observed(destination) {
                        "Current troop observation".into()
                    } else {
                        format!(
                            "Troops observed in month {} · spy report",
                            report_month(destination).unwrap()
                        )
                    });
                    ui.strong(estimate.assessment).on_hover_text(estimate.reasons.join("\n"));
                    egui::CollapsingHeader::new(format!(
                        "Scouting · {} hostile cohorts",
                        enemy.len()
                    ))
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
                } else {
                    ui.label("?");
                }
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
        });
    ui.ctx().data_mut(|data| data.insert_temp(selection_id, selection));
    action
}

/// A dedicated recruitment page keeps drafting separate from army orders.
fn recruitment_view(
    ui: &mut egui::Ui,
    world: &MilitaryWorld,
    economy: &EconomyWorld,
    province: usize,
    player: usize,
    reported: bool,
) -> Option<MilitaryUiAction> {
    let state = world.provinces.get(province)?;
    let economic = economy.provinces.get(province)?;
    let owner = ForceOwner::Player(player);
    let directly_owned = economic.owner == Some(player);
    let administrative = directly_owned || economic.overlord == Some(player);
    let in_battle = world.province_in_battle(province);
    let mut action = None;
    if directly_owned {
        ui.separator();
        ui.horizontal_wrapped(|ui| {
            ui.strong("RECRUITMENT");
            stat(
                ui,
                Icon::Plebeians,
                &super::resource_hud::format_population(economic.population[2]),
                "Plebeians available for recruitment.",
            );
            stat(
                ui,
                Icon::Citizens,
                &super::resource_hud::format_population(economic.population[1]),
                "Citizens available for recruitment.",
            );
        });
        ui.strong(if state.recruitment.is_some() {
            "Recruitment in progress"
        } else {
            "Available cohorts"
        });
        if let Some(project) = &state.recruitment {
            let effort = economic.policies.recruitment as usize;
            let speed = economy.config.recruitment_speed[effort].max(0.001);
            let remaining =
                ((project.required_progress - project.progress).max(0.0) / speed).ceil();
            let cost = project.manpower * economy.config.recruitment_coin_per_recruit[effort];
            ui.small(format!("Effort: {:?} · {remaining:.0} mo remaining · {cost:.2} Coin/mo", economic.policies.recruitment))
                .on_hover_text("Estimate assumes full funding. Insufficient Coin reduces High effort toward Normal speed. Effort can be changed under province Policies.");
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
                            let speed = economy.config.recruitment_speed[economic.policies.recruitment as usize].max(0.001);
                            ui.small(format!("{:.0} mo", (definition.recruitment_months / speed).ceil())).on_hover_text("Recruitment duration at the selected effort and full funding. Construction uses a separate slot.");
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
    }
    if !directly_owned {
        ui.separator();
        ui.strong("RECRUITMENT");
        if administrative || reported {
            if let Some(project) = &state.recruitment {
                ui.label(format!(
                    "{} · {:.1}/{:.1} months · {} manpower",
                    project.unit_type.name(),
                    project.progress,
                    project.required_progress,
                    super::resource_hud::format_population(project.manpower)
                ));
            } else {
                ui.small("No recruitment in this report.");
            }
        } else {
            ui.small("?");
        }
    }
    action
}

/// Include engaged troops in the current army, without counting moving armies.
fn province_force_units(world: &MilitaryWorld, province: usize, owner: ForceOwner) -> Vec<Unit> {
    world
        .provinces
        .get(province)
        .into_iter()
        .flat_map(|state| state.forces.get(&owner))
        .flatten()
        .chain(
            world
                .battles
                .iter()
                .filter(|battle| battle.province == province)
                .flat_map(|battle| battle.attackers.units.iter().chain(&battle.defenders.units)),
        )
        .filter(|unit| unit.owner == owner && unit.current_manpower > 0.)
        .cloned()
        .collect()
}

/// Engagement displays the actual locked plan rather than an editable default.
fn province_force_plan(world: &MilitaryWorld, province: usize, owner: ForceOwner) -> BattlePlan {
    world
        .battles
        .iter()
        .filter(|battle| battle.province == province)
        .find_map(|battle| {
            battle.attackers.plans.get(&owner).or_else(|| battle.defenders.plans.get(&owner))
        })
        .or_else(|| world.provinces.get(province).and_then(|state| state.plans.get(&owner)))
        .copied()
        .unwrap_or_default()
}

/// Imperator's strength summary and compact per-type composition cards.
fn army_summary(
    ui: &mut egui::Ui,
    units: &[Unit],
    name: &str,
    world: &MilitaryWorld,
    owner: ForceOwner,
) {
    ui.separator();
    ui.horizontal_wrapped(|ui| {
        icon(ui, Icon::Eagle, 28.);
        ui.strong(format!("Army · {name}"));
        ui.small(world.rank(owner).name()).on_hover_text(format!("{:.0} Military Renown. Rank improves morale, garrison effectiveness and Military Senate support.", world.renown.get(&owner).copied().unwrap_or(0.)));
    });
    if units.is_empty() {
        ui.label("No army stationed here.");
        ui.small("Recruit cohorts in the Recruit units tab or move an army into this province.");
        return;
    }
    let manpower: f64 = units.iter().map(|unit| unit.current_manpower).sum();
    let morale = units.iter().map(|unit| unit.morale * unit.current_manpower).sum::<f64>()
        / manpower.max(0.001);
    ui.horizontal_wrapped(|ui| {
        stat(ui, Icon::Population, &super::resource_hud::format_population(manpower), "Current surviving manpower in this province, including troops in battle. Moving armies are listed separately.");
        ui.label(format!("{} cohorts", units.len()));
        stat(ui, Icon::Morale, &format!("{morale:.0}%"), "Manpower-weighted morale of the current army.");
        stat(ui, Icon::Food, &format!("{:.1}/mo", units.iter().map(|unit| unit.food_demand(&world.config)).sum::<f64>()), "Monthly Food demand of the current army.");
    });
    let kinds: Vec<_> = UnitType::ALL
        .into_iter()
        .filter(|kind| units.iter().any(|unit| unit.unit_type == *kind))
        .collect();
    let columns = ((ui.available_width() + ui.spacing().item_spacing.x)
        / (64. + ui.spacing().item_spacing.x))
        .floor()
        .max(1.) as usize;
    for row in kinds.chunks(columns) {
        ui.horizontal(|ui| {
            for &kind in row {
                let count = units.iter().filter(|unit| unit.unit_type == kind).count();
                if count == 0 {
                    continue;
                }
                egui::Frame::new()
                    .fill(super::province_panel::TABLE_STRIPE)
                    .stroke(egui::Stroke::new(1., super::province_panel::RULE))
                    .inner_margin(4.)
                    .show(ui, |ui| {
                        ui.set_width(54.);
                        ui.vertical_centered(|ui| {
                            icon(ui, Icon::Unit(kind), 36.)
                                .on_hover_text(unit_definition_tip(kind, &world.config));
                            let strength: f64 = units
                                .iter()
                                .filter(|unit| unit.unit_type == kind)
                                .map(|unit| unit.current_manpower)
                                .sum();
                            ui.label(super::resource_hud::format_population(strength));
                            ui.small(format!("{count} cohorts")).on_hover_text(kind.name());
                        });
                    });
            }
        });
    }
}

/// Load original tactic symbols prepared from the official UI reference at build time.
fn tactic_texture(ui: &egui::Ui, tactic: CombatTactic) -> egui::TextureId {
    let key = egui::Id::new(("imperator-tactic-icon", tactic as usize));
    if let Some(texture) = ui.ctx().data(|data| data.get_temp::<egui::TextureHandle>(key)) {
        return texture.id();
    }
    macro_rules! original {
        ($name:literal) => {
            include_bytes!(concat!(env!("OUT_DIR"), "/tactic-icons/", $name, ".png")) as &[u8]
        };
    }
    let bytes = match tactic {
        CombatTactic::Balanced => original!("balanced"),
        CombatTactic::ShockAction => original!("shock-action"),
        CombatTactic::Bottleneck => original!("bottleneck"),
        CombatTactic::Envelopment => original!("envelopment"),
        CombatTactic::Skirmishing => original!("skirmishing"),
        CombatTactic::Deception => original!("deception"),
    };
    let image = image::load_from_memory(bytes).expect("original tactic symbol").to_rgba8();
    let texture = ui.ctx().load_texture(
        format!("imperator-{tactic:?}"),
        egui::ColorImage::from_rgba_unmultiplied(
            [image.width() as usize, image.height() as usize],
            image.as_raw(),
        ),
        egui::TextureOptions::LINEAR,
    );
    let id = texture.id();
    ui.ctx().data_mut(|data| data.insert_temp(key, texture));
    id
}

/// Match the adjacent Imperator tactic ledger: symbol/fit, name, and counter symbols.
fn tactic_choice(
    ui: &mut egui::Ui,
    tactic: CombatTactic,
    selected: bool,
    units: &[Unit],
    config: &MilitaryConfig,
) -> egui::Response {
    let fit = tactic_effectiveness(units.iter(), tactic, config);
    let (rect, response) = ui.allocate_exact_size(egui::vec2(340., 68.), egui::Sense::click());
    let paper = if selected || response.hovered() {
        egui::Color32::from_rgb(226, 210, 180)
    } else {
        super::province_panel::PAPER
    };
    ui.painter().rect_filled(rect, 2., paper);
    ui.painter().rect_stroke(
        rect,
        2.,
        egui::Stroke::new(1., super::province_panel::RULE),
        egui::StrokeKind::Inside,
    );
    let at = |x, y| rect.min + egui::vec2(x, y);
    let paint_symbol = |kind, x, y, size| {
        ui.painter().image(
            tactic_texture(ui, kind),
            egui::Rect::from_min_size(at(x, y), egui::vec2(size, size)),
            egui::Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(1., 1.)),
            egui::Color32::WHITE,
        );
    };
    paint_symbol(tactic, 7., 5., 40.);
    ui.painter().text(
        at(27., 55.),
        egui::Align2::CENTER_CENTER,
        if tactic == CombatTactic::Balanced {
            "Neutral".to_owned()
        } else {
            format!("{:.0}%", fit * 100.)
        },
        egui::FontId::proportional(12.),
        egui::Color32::from_rgb(42, 133, 148),
    );
    ui.painter().text(
        at(59., 17.),
        egui::Align2::LEFT_CENTER,
        tactic.name(),
        egui::FontId::proportional(16.),
        super::province_panel::INK,
    );
    let tip = if let Some(counter) = config.counters[tactic as usize] {
        paint_symbol(counter, 60., 34., 24.);
        ui.painter().text(
            at(90., 47.),
            egui::Align2::LEFT_CENTER,
            format!("+{:.1}%", config.tactic_bonus * fit * 100.),
            egui::FontId::proportional(13.),
            egui::Color32::from_rgb(74, 117, 58),
        );
        if let Some(countered_by) =
            CombatTactic::ALL.into_iter().find(|candidate| candidate.counters(tactic, config))
        {
            paint_symbol(countered_by, 165., 34., 24.);
            ui.painter().text(
                at(195., 47.),
                egui::Align2::LEFT_CENTER,
                format!("{:.0}%", (config.countered_multiplier - 1.) * 100.),
                egui::FontId::proportional(13.),
                egui::Color32::from_rgb(154, 57, 47),
            );
        }
        format!("{}\nCounters {}: +{:.1}% damage with this army.\nCasualty intensity ×{:.2}. Fit comes from real surviving cohorts. Tactics lock on engagement.", tactic.name(), counter.name(), config.tactic_bonus * fit * 100., config.casualty_intensity[tactic as usize])
    } else {
        ui.painter().text(
            at(59., 47.),
            egui::Align2::LEFT_CENTER,
            "No tactic bonus or counter penalty",
            egui::FontId::proportional(12.),
            super::province_panel::INK,
        );
        "Neutral tactic, with no tactic bonus or counter penalty.".to_owned()
    };
    response.widget_info(|| {
        egui::WidgetInfo::labeled(egui::WidgetType::Button, ui.is_enabled(), tactic.name())
    });
    response.on_hover_text(tip)
}

/// One icon-button opens the tactic list with real composition fit and counters.
fn edit_plan(
    ui: &mut egui::Ui,
    plan: &mut BattlePlan,
    province: usize,
    units: &[Unit],
    config: &MilitaryConfig,
) {
    ui.separator();
    ui.push_id(("military-plan", province), |ui| {
        let fit = tactic_effectiveness(units.iter(), plan.tactic, config);
        let image = egui::Image::new((tactic_texture(ui, plan.tactic), egui::vec2(36., 36.)));
        let label = if plan.tactic == CombatTactic::Balanced { "Balanced · neutral".to_owned() }
            else { format!("{} · {:.0}% fit", plan.tactic.name(), fit * 100.) };
        let tactic_button = ui.add(egui::Button::image_and_text(image, label));
        egui::Popup::menu(&tactic_button).align(egui::RectAlign::RIGHT_START)
            .show(|ui| {
                ui.set_width(340.);
                for tactic in CombatTactic::ALL {
                    if tactic_choice(ui, tactic, plan.tactic == tactic, units, config).clicked() {
                        plan.tactic = tactic;
                        ui.close();
                    }
                }
            });
        tactic_button.on_hover_text("Choose combat tactic. Effectiveness uses actual surviving cohorts, including reserves; preferred types do not create troops.");
        let width = (ui.available_width() - 3. * ui.spacing().item_spacing.x) / 4.;
        ui.horizontal(|ui| {
            for (label, value, flank) in [
                ("Frontline", &mut plan.primary_unit_type, false),
                ("Rear line", &mut plan.secondary_unit_type, false),
                ("Flanks", &mut plan.flank_unit_type, true),
            ] {
                ui.allocate_ui_with_layout(egui::vec2(width, 85.), egui::Layout::top_down(egui::Align::Center), |ui| {
                    ui.small(label);
                    let response = egui::containers::menu::MenuButton::from_button(egui::Button::new(" ").min_size(egui::vec2(44., 44.)))
                        .ui(ui, |ui| {
                            for kind in UnitType::ALL {
                                let eligible = if flank { matches!(kind, UnitType::LightCavalry | UnitType::HeavyCavalry | UnitType::HorseArchers | UnitType::WarCamels | UnitType::WarChariots | UnitType::LightInfantry) } else { !kind.is_support() };
                                if eligible {
                                    ui.horizontal(|ui| {
                                        icon(ui, Icon::Unit(kind), 28.);
                                        if ui.selectable_value(value, kind, kind.name()).clicked() { ui.close(); }
                                    });
                                }
                            }
                        }).0;
                    paint_icon(ui, Icon::Unit(*value), response.rect.shrink(4.));
                    response.on_hover_text(format!("{}: {}. {}", label, value.name(), if flank { "Preferred cohorts for both wings." } else if label == "Frontline" { "Primary cohorts fill the center first. Missing types fall back to available cohorts." } else { "Secondary cohorts replace the frontline. Ranged and siege support deploy automatically." }));
                    ui.add(egui::Label::new(egui::RichText::new(value.name()).small()).truncate());
                });
            }
            ui.allocate_ui_with_layout(egui::vec2(width, 85.), egui::Layout::top_down(egui::Align::Center), |ui| {
                ui.small("Flank size");
                egui::containers::menu::MenuButton::from_button(egui::Button::new(egui::RichText::new(plan.flank_size.to_string()).size(24.)).min_size(egui::vec2(44., 44.)))
                    .ui(ui, |ui| {
                        for size in 1..=3 {
                            if ui.selectable_value(&mut plan.flank_size, size, format!("{size} per side")).clicked() { ui.close(); }
                        }
                    }).0.on_hover_text("Cohorts per wing. Terrain can reduce this width to preserve two center slots. Maneuver controls attack reach separately.");
                ui.small("per side");
            });
        });
    });
}

/// The army ledger uses Imperator's red panels and gold text, with strength/morale meters.
fn cohort_row(ui: &mut egui::Ui, index: usize, contents: impl FnOnce(&mut egui::Ui)) {
    let width = ui.available_width();
    egui::Frame::new()
        .fill(if index.is_multiple_of(2) {
            egui::Color32::from_rgb(112, 32, 29)
        } else {
            egui::Color32::from_rgb(124, 39, 31)
        })
        .stroke(egui::Stroke::new(1., egui::Color32::from_rgb(177, 138, 66)))
        .inner_margin(4.)
        .show(ui, |ui| {
            ui.set_width((width - 10.).max(1.));
            ui.visuals_mut().override_text_color = Some(egui::Color32::from_rgb(244, 218, 155));
            contents(ui);
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
                        "{} · {}\n{}/{} manpower\nTraining {:.0} · Morale {:.0}",
                        unit.unit_type.name(),
                        if is_support {
                            "Support"
                        } else if flank {
                            "Flank"
                        } else {
                            "Center"
                        },
                        super::resource_hud::format_population(unit.current_manpower),
                        super::resource_hud::format_population(unit.max_manpower),
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
        stat(ui,Icon::Building(BuildingType::CityWalls),&format!("+{:.0}%",fort*(1.-suppression)*100.),&format!("Level {} fortification: base defense +{:.0}%. Active siege power {:.1} suppresses {:.0}% of that bonus. Protected support fights at {:.0}% power; exposed support takes {:.0}% extra casualties.",battle.fortification_level,fort*100.,siege,suppression*100.,config.support_effectiveness*100.,(config.exposed_support_casualties-1.)*100.));
    });
    for (attacker, label, side) in
        [(true, "ATTACKERS", &battle.attackers), (false, "DEFENDERS", &battle.defenders)]
    {
        ui.horizontal_wrapped(|ui| {
            ui.strong(label);
            stat(
                ui,
                Icon::Population,
                &super::resource_hud::format_population(side.manpower()),
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
            ui.small(format!("{}/{}",super::resource_hud::format_population(unit.current_manpower),super::resource_hud::format_population(unit.max_manpower))).on_hover_text("Surviving / original manpower. Casualties are permanent; cohorts never reinforce automatically.");
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
