//! Campaign military details, matching the enclosing parchment panel palette.
//!
//! Rendering returns a typed action so the campaign bridge owns transactional
//! diplomacy, civilian population, and global-resource mutations.

use std::collections::BTreeSet;

use super::campaign_widgets::{icon, paint_icon, stat, work_queue, Icon, WorkQueueAction};
use super::resource_hud::format_food_demand;
use bevy_egui::egui;

use crate::game::economy::{BuildingType, EconomyWorld};
use crate::game::military::*;

/// One player intent dispatched by the campaign bridge after rendering.
pub(in crate::app) enum MilitaryUiAction {
    /// Draft the selected type in the current province.
    Recruit(UnitType),
    /// Cancel the current project without a refund.
    CancelRecruitment,
    /// Remove one waiting cohort and refund its original population and Metal.
    CancelQueuedRecruitment(usize),
    /// Return surviving soldiers to the local civilian class.
    #[allow(dead_code)] // Per-cohort disbanding will return with the orders page.
    Disband(UnitId),
    /// Return every surviving cohort in the stationary army to civilians.
    DisbandArmy,
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
    #[allow(dead_code)] // Movement orders are intentionally deferred in the panel.
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

const ARMY_PANEL_ID: &str = "campaign-army-panel";
const ARMY_TITLE_SEARCH_ID: &str = "campaign-army-title-search";

#[derive(Clone, Default)]
struct ArmyTitleSearch {
    open: bool,
    query: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct ArmyPanelSelection {
    province: usize,
    player: usize,
    panel: ArmyPanel,
}

fn selected_army(ctx: &egui::Context) -> Option<ArmyPanelSelection> {
    ctx.data(|data| data.get_temp(egui::Id::new(ARMY_PANEL_ID)))
}

#[cfg(test)]
pub(super) fn selected_army_province(ctx: &egui::Context) -> Option<usize> {
    selected_army(ctx).map(|selected| selected.province)
}

fn set_selected_army(ctx: &egui::Context, selected: Option<ArmyPanelSelection>) {
    ctx.data_mut(|data| {
        data.remove::<ArmyTitleSearch>(egui::Id::new(ARMY_TITLE_SEARCH_ID));
        if let Some(selected) = selected {
            data.insert_temp(egui::Id::new(ARMY_PANEL_ID), selected);
        } else {
            data.remove::<ArmyPanelSelection>(egui::Id::new(ARMY_PANEL_ID));
        }
    });
}

/// Escape closes the independent army window before the game menu is opened.
pub(in crate::app) fn dismiss_army_panel(ctx: &egui::Context) -> bool {
    if selected_army(ctx).is_none() {
        return false;
    }
    set_selected_army(ctx, None);
    true
}

/// Map troop clicks open the matching owned army, including a specific marching order.
pub(in crate::app) fn open_army_panel(
    ctx: &egui::Context,
    province: usize,
    player: usize,
    movement: Option<u64>,
) {
    set_selected_army(
        ctx,
        Some(ArmyPanelSelection {
            province,
            player,
            panel: movement.map_or(ArmyPanel::Stationary, ArmyPanel::Movement),
        }),
    );
}

fn toggle_army_panel(ctx: &egui::Context, province: usize, player: usize, panel: ArmyPanel) {
    let selected = ArmyPanelSelection {
        province,
        player,
        panel,
    };
    set_selected_army(ctx, (selected_army(ctx) != Some(selected)).then_some(selected));
}

/// One owner's province force, or one distinct force currently marching from it.
struct ArmyOverviewRow {
    province: ProvinceId,
    owner: ForceOwner,
    movement: Option<u64>,
    destination: Option<ProvinceId>,
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
            units,
            tactic: Some(movement.plan.tactic),
        });
    }
    rows.sort_by_key(|row| (row.province, row.owner != own, row.owner, row.movement));
    rows
}

/// Player-wide military career, displayed above the national army ledger.
fn rank_ladder(ui: &mut egui::Ui, world: &MilitaryWorld, player: usize, scale: f32) {
    let owner = ForceOwner::Player(player);
    let current = world.rank(owner);
    let renown = world.renown.get(&owner).copied().unwrap_or(0.);
    ui.small(format!("{} · {renown:.0} Military Renown", current.name()));
    ui.add_space(4. * scale);
    ui.columns(4, |columns| {
        for (ui, rank) in columns.iter_mut().zip([
            MilitaryRank::Centurion,
            MilitaryRank::MilitaryTribune,
            MilitaryRank::Legate,
            MilitaryRank::Imperator,
        ]) {
            let index = rank as usize;
            egui::Frame::new()
                .fill(if rank == current {
                    egui::Color32::from_rgb(218, 198, 162)
                } else {
                    super::province_panel::TABLE_STRIPE
                })
                .corner_radius(3. * scale)
                .inner_margin(6. * scale)
                .show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.vertical_centered(|ui| {
                        ui.scope(|ui| {
                            ui.set_opacity(if index <= current as usize { 1. } else { 0.4 });
                            icon(ui, Icon::MilitaryRank(rank), 36. * scale);
                        });
                        ui.label(rank.name());
                        ui.small(format!("{:.0} Renown", world.config.rank_thresholds[index]));
                    });
                })
                .response
                .on_hover_text(format!(
                    "{}{}\n{renown:.0} Military Renown · requires {:.0}\n+{:.0} morale · ×{:.2} garrison effectiveness · +{:.0} Military Senate support",
                    rank.name(),
                    if rank == current { " · current rank" } else { "" },
                    world.config.rank_thresholds[index],
                    world.config.rank_morale[index],
                    world.config.rank_control[index],
                    world.config.rank_senate[index],
                ));
        }
    });
}

/// National military career and separate owned and visiting army ledgers.
pub(in crate::app) fn overview(
    ui: &mut egui::Ui,
    world: &MilitaryWorld,
    economy: &EconomyWorld,
    player: usize,
    colors: &[egui::Color32],
    scale: f32,
) -> Option<ProvinceId> {
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
    let friendly_rows: Vec<_> = rows
        .iter()
        .filter(|row| {
            row.owner != own
                && matches!(row.owner, ForceOwner::Player(_))
                && economy.provinces[row.province].owner == Some(player)
        })
        .collect();
    let height = (ui.clip_rect().bottom() - ui.next_widget_position().y).max(0.);
    let mut selected_province = None;
    egui::ScrollArea::vertical()
        .id_salt(("military-overview", player))
        .max_height(height)
        .auto_shrink([false, false])
        .show(ui, |ui| {
            ui.set_width((ui.available_width() - ui.spacing().scroll.allocated_width()).max(1.));
            super::policy_widgets::section(ui, scale, "MILITARY RANK");
            rank_ladder(ui, world, player, scale);
            ui.add_space(10. * scale);
            super::policy_widgets::section(ui, scale, "ARMIES");
            if own_rows.is_empty() {
                ui.label("You have no armies.");
            } else {
                overview_table(
                    ui,
                    &own_rows,
                    world,
                    economy,
                    player,
                    colors,
                    scale,
                    &mut selected_province,
                );
            }
            ui.add_space(10. * scale);
            super::policy_widgets::section(ui, scale, "FRIENDLY ARMIES");
            if friendly_rows.is_empty() {
                ui.label("No friendly armies in your provinces.");
            } else {
                overview_table(
                    ui,
                    &friendly_rows,
                    world,
                    economy,
                    player,
                    colors,
                    scale,
                    &mut selected_province,
                );
            }
        });
    selected_province
}

const OVERVIEW_FRACTIONS: [f32; 6] = [0., 0.24, 0.49, 0.75, 0.875, 1.];

fn overview_cells(rect: egui::Rect, scale: f32) -> [egui::Rect; 5] {
    std::array::from_fn(|index| {
        egui::Rect::from_min_max(
            egui::pos2(
                rect.left() + rect.width() * OVERVIEW_FRACTIONS[index] + 5. * scale,
                rect.top(),
            ),
            egui::pos2(
                rect.left() + rect.width() * OVERVIEW_FRACTIONS[index + 1] - 5. * scale,
                rect.bottom(),
            ),
        )
    })
}

fn overview_text(
    ui: &egui::Ui,
    cell: egui::Rect,
    label: &str,
    size: f32,
    color: egui::Color32,
    scale: f32,
) {
    let painter = ui.painter().with_clip_rect(ui.clip_rect().intersect(cell));
    let font = egui::FontId::proportional(size * scale);
    let row_height = ui.fonts_mut(|fonts| fonts.row_height(&font));
    let mut job =
        egui::text::LayoutJob::simple(label.to_owned(), font, color, cell.width().max(1.));
    job.wrap.max_rows = (cell.height() / row_height).floor().max(1.) as usize;
    job.wrap.break_anywhere = job.wrap.max_rows == 1;
    let galley = painter.layout_job(job);
    painter.galley(cell.left_center() - egui::vec2(0., galley.size().y * 0.5), galley, color);
}

fn overview_metric(ui: &egui::Ui, cell: egui::Rect, symbol: Icon, value: &str, scale: f32) {
    let size = (cell.width() * 0.32).min(22. * scale).max(12. * scale);
    let image = egui::Rect::from_center_size(
        egui::pos2(cell.left() + size * 0.5, cell.center().y),
        egui::vec2(size, size),
    );
    paint_icon(ui, symbol, image);
    let value_cell =
        egui::Rect::from_min_max(egui::pos2(image.right() + 3. * scale, cell.top()), cell.max);
    overview_text(ui, value_cell, value, 12., super::province_panel::INK, scale);
}

fn overview_table(
    ui: &mut egui::Ui,
    rows: &[&ArmyOverviewRow],
    world: &MilitaryWorld,
    economy: &EconomyWorld,
    player: usize,
    colors: &[egui::Color32],
    scale: f32,
    selected_province: &mut Option<ProvinceId>,
) {
    use super::province_panel::{INK, PAPER, RULE, TABLE_STRIPE};
    let width = ui.available_width().max(1.);
    let (header, _) = ui.allocate_exact_size(egui::vec2(width, 34. * scale), egui::Sense::hover());
    ui.painter().rect_filled(header, 2. * scale, egui::Color32::from_rgb(73, 69, 61));
    for (cell, label) in overview_cells(header, scale)
        .into_iter()
        .zip(["Province", "Units", "Tactic", "Morale", "Training"])
    {
        overview_text(ui, cell, label, 12., PAPER, scale);
    }
    for (index, row) in rows.iter().enumerate() {
        let composition: Vec<_> = UnitType::ALL
            .into_iter()
            .filter_map(|kind| {
                let count = row.units.iter().filter(|unit| unit.unit_type == kind).count();
                (count > 0).then_some((kind, count))
            })
            .collect();
        let unit_width = width * (OVERVIEW_FRACTIONS[2] - OVERVIEW_FRACTIONS[1]) - 10. * scale;
        let columns = (unit_width / (48. * scale)).floor().max(1.) as usize;
        let height = (composition.len().div_ceil(columns) as f32 * 28. + 12.).max(54.) * scale;
        let (rect, response) =
            ui.allocate_exact_size(egui::vec2(width, height), egui::Sense::click());
        let response = response.on_hover_cursor(egui::CursorIcon::PointingHand);
        response.widget_info(|| {
            egui::WidgetInfo::labeled(
                egui::WidgetType::Button,
                true,
                format!(
                    "{} · {}",
                    economy.provinces[row.province].name,
                    military_owner_name(row.owner, world)
                ),
            )
        });
        ui.painter().rect_filled(
            rect,
            0.,
            if response.hovered() || response.has_focus() {
                egui::Color32::from_rgb(229, 217, 190)
            } else if index.is_multiple_of(2) {
                PAPER
            } else {
                TABLE_STRIPE
            },
        );
        let cell = overview_cells(rect, scale);
        let color = match row.owner {
            ForceOwner::Player(id) => colors.get(id).copied().unwrap_or(RULE),
            ForceOwner::Local(_) => super::province_panel::NEUTRAL,
        };
        ui.painter().rect_filled(
            egui::Rect::from_min_size(rect.min, egui::vec2(3. * scale, height)),
            0.,
            color,
        );
        if row.owner == ForceOwner::Player(player) {
            overview_text(ui, cell[0], &economy.provinces[row.province].name, 13., INK, scale);
        } else {
            let province = egui::Rect::from_min_max(
                cell[0].min,
                egui::pos2(cell[0].right(), cell[0].center().y),
            );
            let owner = egui::Rect::from_min_max(
                egui::pos2(cell[0].left(), cell[0].center().y),
                cell[0].max,
            );
            overview_text(ui, province, &economy.provinces[row.province].name, 13., INK, scale);
            overview_text(ui, owner, &military_owner_name(row.owner, world), 11., INK, scale);
        }
        let painter = ui.painter().with_clip_rect(ui.clip_rect().intersect(cell[1]));
        for (unit_index, &(kind, count)) in composition.iter().enumerate() {
            let min = cell[1].min
                + egui::vec2(
                    (unit_index % columns) as f32 * 48. + 1.,
                    (unit_index / columns) as f32 * 28. + 6.,
                ) * scale;
            paint_icon(
                ui,
                Icon::Unit(kind),
                egui::Rect::from_min_size(min, egui::vec2(24., 24.) * scale),
            );
            painter.text(
                min + egui::vec2(27., 12.) * scale,
                egui::Align2::LEFT_CENTER,
                count.to_string(),
                egui::FontId::proportional(12. * scale),
                INK,
            );
        }
        if let Some(tactic) = row.tactic {
            let image = egui::Rect::from_center_size(
                egui::pos2(cell[2].left() + 11. * scale, cell[2].center().y),
                egui::vec2(22., 22.) * scale,
            );
            paint_tactic(ui, tactic, image);
            overview_text(
                ui,
                egui::Rect::from_min_max(
                    egui::pos2(image.right() + 4. * scale, cell[2].top()),
                    cell[2].max,
                ),
                tactic.name(),
                12.,
                INK,
                scale,
            );
        } else {
            overview_text(ui, cell[2], "?", 12., INK, scale);
        }
        let (morale, training) = row.averages();
        overview_metric(ui, cell[3], Icon::Morale, &format!("{morale:.0}%"), scale);
        overview_metric(
            ui,
            cell[4],
            Icon::MilitaryPower,
            &if row.owner == ForceOwner::Player(player) {
                format!("{training:.0}%")
            } else {
                "?".into()
            },
            scale,
        );
        ui.painter().hline(
            rect.x_range(),
            rect.bottom(),
            egui::Stroke::new(scale, RULE.gamma_multiply(0.3)),
        );
        if response.clicked() {
            if row.owner == ForceOwner::Player(player) {
                open_army_panel(ui.ctx(), row.province, player, row.movement);
            } else {
                *selected_province = Some(row.province);
            }
        }
    }
}

/// Show the province rank and offer recruitment only in owned provinces.
fn military_banner(
    ui: &mut egui::Ui,
    rank: MilitaryRank,
    can_recruit: bool,
    tab: &mut MilitaryTab,
    scale: f32,
) {
    let banner = super::campaign_widgets::portrait(
        ui,
        super::campaign_widgets::ProvinceLandscape::Military,
        76. * scale,
    );
    let painter = ui.painter().with_clip_rect(banner);
    let text_width = |label: &str| {
        painter
            .layout_no_wrap(label.to_owned(), egui::FontId::proportional(14.), egui::Color32::WHITE)
            .size()
            .x
    };
    let rank_width = text_width(rank.name()) + 48.;
    let switch_width = text_width("Stationed units").max(text_width("Recruit units")) + 56.;
    let margin = 8. * scale;
    let badge_scale = if can_recruit {
        scale.min((banner.width() - 3. * margin) / (rank_width + switch_width))
    } else {
        scale.min((banner.width() - 2. * margin) / rank_width)
    };
    let height = 32. * badge_scale;
    let button_height = 38. * badge_scale;
    let bottom = banner.bottom() - margin;
    let rank_rect = egui::Rect::from_min_max(
        egui::pos2(banner.left() + margin, bottom - height),
        egui::pos2(banner.left() + margin + rank_width * badge_scale, bottom),
    );
    let switch_rect = egui::Rect::from_min_max(
        egui::pos2(banner.right() - margin - switch_width * badge_scale, bottom - button_height),
        egui::pos2(banner.right() - margin, bottom),
    );
    ui.interact(rank_rect, ui.id().with("military-rank-badge"), egui::Sense::hover())
        .on_hover_text("Current military rank.");
    paint_army_banner_badge(
        ui,
        rank_rect,
        rank.name(),
        Icon::MilitaryRank(rank),
        egui::Color32::from_black_alpha(180),
        badge_scale,
    );
    if !can_recruit {
        return;
    }
    let response = ui
        .interact(switch_rect, ui.id().with("military-recruitment-badge"), egui::Sense::click())
        .on_hover_cursor(egui::CursorIcon::PointingHand);
    if response.clicked() {
        *tab = match *tab {
            MilitaryTab::Army => MilitaryTab::Recruitment,
            MilitaryTab::Recruitment => MilitaryTab::Army,
        };
    }
    let (label, art) = match *tab {
        MilitaryTab::Army => ("Recruit units", Icon::Recruitment),
        MilitaryTab::Recruitment => ("Stationed units", Icon::MilitaryPower),
    };
    response.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, label));
    let switch_fill = if response.is_pointer_button_down_on() {
        egui::Color32::from_rgb(80, 31, 24)
    } else if response.hovered() {
        egui::Color32::from_rgb(145, 61, 39)
    } else {
        egui::Color32::from_rgb(110, 44, 32)
    };
    paint_army_banner_badge(ui, switch_rect, label, art, switch_fill, badge_scale);
    painter.rect_stroke(
        switch_rect,
        3. * badge_scale,
        egui::Stroke::new(
            if response.hovered() {
                2.
            } else {
                1.5
            } * badge_scale,
            egui::Color32::from_rgb(240, 206, 149),
        ),
        egui::StrokeKind::Inside,
    );
}

/// Province Military is a compact ledger; only the player's rows open an army panel.
pub(in crate::app) fn show(
    ui: &mut egui::Ui,
    world: &MilitaryWorld,
    economy: &EconomyWorld,
    graph: &[MilitaryProvince],
    province: usize,
    player: usize,
    _observed: impl Fn(usize) -> bool,
    _report_month: impl Fn(usize) -> Option<u32>,
    _access: impl Fn(ForceOwner, usize) -> MilitaryAccess,
    _hostile: impl Fn(ForceOwner, ForceOwner) -> bool,
) -> Option<MilitaryUiAction> {
    let state = world.provinces.get(province)?;
    let economic = economy.provinces.get(province)?;
    let terrain = graph.get(province)?.terrain;
    let owner = ForceOwner::Player(player);
    let scale = super::viewport_ui_scale(ui.ctx().content_rect().size());
    let tab_id = egui::Id::new(("province-military-tab", province, player));
    let mut tab =
        ui.ctx().data_mut(|data| data.get_temp::<MilitaryTab>(tab_id)).unwrap_or_default();
    let can_recruit = economic.owner == Some(player);
    if !can_recruit {
        tab = MilitaryTab::Army;
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
    let mut armies: Vec<_> = owners
        .into_iter()
        .filter_map(|force_owner| {
            let units = province_force_units(world, province, force_owner);
            (!units.is_empty()).then_some((force_owner, units))
        })
        .collect();
    armies.sort_by_key(|(force_owner, _)| (*force_owner != owner, *force_owner));
    let rank = match economic.owner {
        Some(province_owner) if province_owner != player => {
            world.rank(ForceOwner::Player(province_owner))
        },
        _ => MilitaryRank::Centurion,
    };
    military_banner(ui, rank, can_recruit, &mut tab, scale);
    ui.ctx().data_mut(|data| data.insert_temp(tab_id, tab));
    let mut action = None;
    if tab == MilitaryTab::Recruitment {
        action = recruitment_view(ui, world, economy, province, player, false);
    } else {
        let all_units: Vec<_> =
            armies.iter().flat_map(|(_, units)| units.iter().cloned()).collect();
        army_summary(ui, &all_units, terrain, world);
        ui.add_space(8. * scale);
        let height = (ui.clip_rect().bottom() - ui.next_widget_position().y).max(0.);
        egui::ScrollArea::vertical()
            .id_salt(("province-army-ledger", province, player))
            .max_height(height)
            .min_scrolled_height(0.)
            .auto_shrink([false, true])
            .show(ui, |ui| {
                if armies.is_empty() {
                    ui.horizontal(|ui| {
                        ui.add_space(12. * scale);
                        ui.label("No army stationed here.");
                    });
                }
                for (force_owner, units) in &armies {
                    let own = *force_owner == owner;
                    let color = army_owner_color(ui.ctx(), *force_owner);
                    let (clicked, disband) = stationed_army_row(
                        ui,
                        units,
                        &world.config,
                        army_name(*force_owner, economy, world),
                        color,
                        own,
                        own && economic.owner == Some(player)
                            && !world.province_in_battle(province),
                        scale,
                    );
                    if clicked {
                        toggle_army_panel(ui.ctx(), province, player, ArmyPanel::Stationary);
                    }
                    if disband {
                        action = Some(MilitaryUiAction::DisbandArmy);
                    }
                    ui.add_space(6. * scale);
                }
                for movement in world.movements.iter().filter(|movement| {
                    movement.owner == owner
                        && (movement.origin == province || movement.destination() == Some(province))
                }) {
                    let destination = movement
                        .destination()
                        .and_then(|id| economy.provinces.get(id))
                        .map_or("destination", |province| province.name.as_str());
                    let color = army_owner_color(ui.ctx(), movement.owner);
                    let (clicked, _) = stationed_army_row(
                        ui,
                        &movement.units,
                        &world.config,
                        format!(
                            "{} · marching to {destination}",
                            army_name(movement.owner, economy, world)
                        ),
                        color,
                        true,
                        false,
                        scale,
                    );
                    if clicked {
                        toggle_army_panel(
                            ui.ctx(),
                            province,
                            player,
                            ArmyPanel::Movement(movement.id),
                        );
                    }
                    ui.add_space(6. * scale);
                }
            });
    }
    action
}

/// Render the selected army without depending on the province inspector's visibility.
pub(in crate::app) fn draw_army_panel(
    ctx: &egui::Context,
    world: &MilitaryWorld,
    economy: &EconomyWorld,
    graph: &[MilitaryProvince],
    player: usize,
    observed: impl Fn(usize) -> bool,
    report_month: impl Fn(usize) -> Option<u32>,
    access: impl Fn(ForceOwner, usize) -> MilitaryAccess,
    hostile: impl Fn(ForceOwner, ForceOwner) -> bool,
) -> Option<(usize, MilitaryUiAction)> {
    let selected = selected_army(ctx)?;
    if selected.player != player {
        set_selected_army(ctx, None);
        return None;
    }
    let province = selected.province;
    let panel = selected.panel;
    let owner = ForceOwner::Player(player);
    let Some(economic) = economy.provinces.get(province) else {
        set_selected_army(ctx, None);
        return None;
    };
    let exists = match panel {
        ArmyPanel::Stationary => !province_force_units(world, province, owner).is_empty(),
        ArmyPanel::Movement(id) => {
            world.movements.iter().any(|movement| movement.id == id && movement.owner == owner)
        },
    };
    if !exists {
        set_selected_army(ctx, None);
        return None;
    }
    let scale = super::viewport_ui_scale(ctx.content_rect().size());
    let screen = ctx.content_rect();
    let bottom = ctx
        .data(|data| data.get_temp::<egui::Rect>(egui::Id::new("practice-players-panel-rect")))
        .map_or(screen.bottom() - 8. * scale, |rect| rect.bottom());
    let width = (640. * scale).min(screen.width() - 16. * scale);
    let color = ctx
        .data(|data| data.get_temp::<egui::Color32>(egui::Id::new("campaign-active-player-color")))
        .unwrap_or(super::PLAYER_COLORS[player % super::PLAYER_COLORS.len()]);
    let panel_id = egui::Id::new(ARMY_PANEL_ID);
    let choices: Vec<_> = army_overview_rows(world, player)
        .into_iter()
        .filter(|row| row.owner == owner)
        .filter_map(|row| {
            let economic = economy.provinces.get(row.province)?;
            let label = if let Some(destination) = row.destination {
                let destination = economy
                    .provinces
                    .get(destination)
                    .map_or("destination", |province| province.name.as_str());
                format!("{} · marching to {destination}", economic.name)
            } else {
                economic.name.clone()
            };
            Some((
                ArmyPanelSelection {
                    province: row.province,
                    player,
                    panel: row.movement.map_or(ArmyPanel::Stationary, ArmyPanel::Movement),
                },
                label,
            ))
        })
        .collect();
    let options: Vec<_> = choices
        .iter()
        .map(|(choice, label)| super::campaign_widgets::TitleSearchOption {
            name: &economy.provinces[choice.province].name,
            label: label.clone(),
            selected: *choice == selected,
        })
        .collect();
    let mut open = true;
    let mut action = None;
    let panel_response = egui::Area::new(panel_id.with("window"))
        .pivot(egui::Align2::RIGHT_BOTTOM)
        .fixed_pos(egui::pos2(screen.right() - 7. * scale, bottom + scale))
        .order(egui::Order::Foreground)
        .movable(false)
        .show(ctx, |ui| {
            *ui.style_mut() = super::campaign_widgets::map_style(scale);
            ui.spacing_mut().item_spacing.y = 4. * scale;
            egui::Frame::new()
                .fill(super::province_panel::PAPER)
                .stroke(egui::Stroke::new(scale, super::province_panel::RULE))
                .corner_radius(5. * scale)
                .show(ui, |ui| {
                    ui.set_width(width);
                    if let Some(index) =
                        army_panel_header(ui, &economic.name, color, scale, &mut open, &options)
                    {
                        set_selected_army(ctx, Some(choices[index].0));
                        ctx.request_repaint();
                    }
                    egui::Frame::new().inner_margin(10. * scale).show(ui, |ui| {
                        ui.set_width(width - 20. * scale);
                        ui.set_min_height(450. * scale);
                        action = match panel {
                            ArmyPanel::Stationary => stationary_army_panel(
                                ui,
                                world,
                                economy,
                                graph,
                                province,
                                player,
                                &observed,
                                &report_month,
                                &access,
                                &hostile,
                            ),
                            ArmyPanel::Movement(id) => {
                                movement_army_panel(ui, world, economy, graph, owner, id)
                            },
                        };
                    });
                });
        });
    ctx.move_to_top(panel_response.response.layer_id);
    if !open {
        set_selected_army(ctx, None);
    }
    action.map(|action| (province, action))
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ArmyPanel {
    Stationary,
    Movement(u64),
}

/// Persistent navigation for each stationary or marching army.
#[derive(Clone, Copy, Default, PartialEq, Eq)]
enum ArmyDetailTab {
    #[default]
    Overview,
    Orders,
}

fn army_panel_header(
    ui: &mut egui::Ui,
    province_name: &str,
    color: egui::Color32,
    scale: f32,
    open: &mut bool,
    options: &[super::campaign_widgets::TitleSearchOption<'_>],
) -> Option<usize> {
    let (rect, _) =
        ui.allocate_exact_size(egui::vec2(ui.available_width(), 42. * scale), egui::Sense::hover());
    ui.painter().rect_filled(
        rect,
        egui::CornerRadius {
            nw: (5. * scale) as u8,
            ne: (5. * scale) as u8,
            sw: 0,
            se: 0,
        },
        color,
    );
    let brightness = color.r() as f32 * 0.299 + color.g() as f32 * 0.587 + color.b() as f32 * 0.114;
    let ink = if brightness > 150. {
        super::province_panel::INK
    } else {
        egui::Color32::from_rgb(255, 238, 210)
    };
    paint_icon(
        ui,
        Icon::MilitaryPower,
        egui::Rect::from_min_size(
            rect.min + egui::vec2(9., 7.) * scale,
            egui::vec2(28., 28.) * scale,
        ),
    );
    let title_region = egui::Rect::from_min_max(
        rect.min + egui::vec2(44., 0.) * scale,
        rect.max - egui::vec2(40., 0.) * scale,
    );
    let title_size = egui::vec2(title_region.width().min(300. * scale), 32. * scale);
    let title_rect = egui::Rect::from_center_size(title_region.center(), title_size);
    let search_id = egui::Id::new(ARMY_TITLE_SEARCH_ID);
    let mut search =
        ui.ctx().data(|data| data.get_temp::<ArmyTitleSearch>(search_id)).unwrap_or_default();
    let mut title_ui =
        ui.new_child(egui::UiBuilder::new().id_salt("army-title").max_rect(title_rect));
    let selected = super::campaign_widgets::title_search_selector(
        &mut title_ui,
        title_size,
        ink,
        scale,
        province_name,
        search_id,
        &mut search.open,
        &mut search.query,
        options,
        "No armies match",
    );
    ui.ctx().data_mut(|data| data.insert_temp(search_id, search));
    let close = egui::Rect::from_center_size(
        egui::pos2(rect.right() - 21. * scale, rect.center().y),
        egui::vec2(28., 28.) * scale,
    );
    let response = ui.interact(close, ui.id().with("close-army"), egui::Sense::click());
    let (fill, close_ink) = super::province_panel::header_close_colors(
        color,
        response.hovered(),
        response.is_pointer_button_down_on() || response.clicked(),
    );
    ui.painter().circle(
        close.center(),
        12. * scale,
        fill,
        egui::Stroke::new(1.3 * scale, close_ink),
    );
    for sign in [-1., 1.] {
        ui.painter().line_segment(
            [
                close.center() + egui::vec2(-4.5, sign * -4.5) * scale,
                close.center() + egui::vec2(4.5, sign * 4.5) * scale,
            ],
            egui::Stroke::new(1.7 * scale, close_ink),
        );
    }
    response
        .widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, "Close panel"));
    if response.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
        *open = false;
    }
    selected
}

fn army_detail_tabs(ui: &mut egui::Ui, key: egui::Id, rank: MilitaryRank) -> ArmyDetailTab {
    let scale = super::viewport_ui_scale(ui.ctx().content_rect().size());
    let mut tab = ui.ctx().data(|data| data.get_temp::<ArmyDetailTab>(key)).unwrap_or_default();
    let banner = super::campaign_widgets::portrait(
        ui,
        super::campaign_widgets::ProvinceLandscape::Army,
        112. * scale,
    );
    let painter = ui.painter_at(banner);
    let rank_width = painter
        .layout_no_wrap(
            rank.name().to_owned(),
            egui::FontId::proportional(14.),
            egui::Color32::WHITE,
        )
        .size()
        .x
        + 48.;
    let badge_scale = scale.min((banner.width() - 24. * scale) / (rank_width + 128.));
    let bottom = banner.bottom() - 8. * scale;
    let rank_rect = egui::Rect::from_min_size(
        egui::pos2(banner.left() + 8. * scale, bottom - 32. * badge_scale),
        egui::vec2(rank_width, 32.) * badge_scale,
    );
    paint_army_banner_badge(
        ui,
        rank_rect,
        rank.name(),
        Icon::MilitaryRank(rank),
        egui::Color32::from_black_alpha(180),
        badge_scale,
    );
    {
        let (value, label, art) = (ArmyDetailTab::Orders, "Orders", Icon::Orders);
        let rect = egui::Rect::from_min_size(
            egui::pos2(
                banner.right() - 8. * scale - 128. * badge_scale,
                bottom - 38. * badge_scale,
            ),
            egui::vec2(128., 38.) * badge_scale,
        );
        let response = ui
            .interact(rect, key.with(label), egui::Sense::click())
            .on_hover_cursor(egui::CursorIcon::PointingHand);
        if response.clicked() {
            tab = if tab == value {
                ArmyDetailTab::Overview
            } else {
                value
            };
        }
        let fill = if response.is_pointer_button_down_on() || tab == value {
            egui::Color32::from_rgb(80, 31, 24)
        } else if response.hovered() {
            egui::Color32::from_rgb(145, 61, 39)
        } else {
            egui::Color32::from_rgb(110, 44, 32)
        };
        paint_army_banner_badge(ui, rect, label, art, fill, badge_scale);
        painter.rect_stroke(
            rect,
            3. * badge_scale,
            egui::Stroke::new(
                if response.hovered() {
                    2.
                } else {
                    1.5
                } * badge_scale,
                egui::Color32::from_rgb(240, 206, 149),
            ),
            egui::StrokeKind::Inside,
        );
    }
    ui.add_space(6. * scale);
    ui.ctx().data_mut(|data| data.insert_temp(key, tab));
    tab
}

fn paint_army_banner_badge(
    ui: &egui::Ui,
    rect: egui::Rect,
    label: &str,
    art: Icon,
    fill: egui::Color32,
    scale: f32,
) {
    ui.painter().rect_filled(rect, 3. * scale, fill);
    let font = egui::FontId::proportional(14. * scale);
    let text_width =
        ui.painter().layout_no_wrap(label.to_owned(), font.clone(), egui::Color32::WHITE).size().x;
    let group_width = (26. + 8.) * scale + text_width;
    let group_left = rect.center().x - group_width * 0.5;
    paint_icon(
        ui,
        art,
        egui::Rect::from_center_size(
            egui::pos2(group_left + 13. * scale, rect.center().y),
            egui::vec2(26., 26.) * scale,
        ),
    );
    ui.painter().text(
        egui::pos2(group_left + 34. * scale, rect.center().y),
        egui::Align2::LEFT_CENTER,
        label,
        font,
        egui::Color32::WHITE,
    );
}

fn army_orders_placeholder(ui: &mut egui::Ui) {
    let scale = super::viewport_ui_scale(ui.ctx().content_rect().size());
    ui.allocate_ui_with_layout(
        egui::vec2(ui.available_width(), 260. * scale),
        egui::Layout::top_down(egui::Align::Center),
        |ui| {
            ui.add_space(70. * scale);
            icon(ui, Icon::MilitaryPower, 44. * scale);
            ui.strong("Army orders");
            ui.small("Army orders will be added here later.");
        },
    );
}

/// Only one page is rendered at a time, keeping the overview free of scrolling.
fn stationary_army_panel(
    ui: &mut egui::Ui,
    world: &MilitaryWorld,
    economy: &EconomyWorld,
    graph: &[MilitaryProvince],
    province: usize,
    player: usize,
    _observed: impl Fn(usize) -> bool,
    _report_month: impl Fn(usize) -> Option<u32>,
    _access: impl Fn(ForceOwner, usize) -> MilitaryAccess,
    _hostile: impl Fn(ForceOwner, ForceOwner) -> bool,
) -> Option<MilitaryUiAction> {
    let economic = economy.provinces.get(province)?;
    let terrain = graph.get(province)?.terrain;
    let owner = ForceOwner::Player(player);
    let army = province_force_units(world, province, owner);
    let in_battle = world.province_in_battle(province);
    let mut action = None;
    let tab = army_detail_tabs(
        ui,
        egui::Id::new(("army-detail-tab", province, owner, None::<u64>)),
        world.rank(owner),
    );
    match tab {
        ArmyDetailTab::Overview => {
            army_summary(ui, &army, terrain, world);
            let mut plan = province_force_plan(world, province, owner);
            if !in_battle {
                normalize_plan_preferences(&mut plan, &army);
            }
            let previous = plan;
            ui.add_enabled_ui(!in_battle, |ui| {
                edit_plan(ui, &mut plan, province, &army, &world.config);
                if let Some(location) = graph.get(province) {
                    deployment_editor(ui, &plan, &army, location.terrain, &world.config);
                }
            });
            if plan != previous && !in_battle {
                action = Some(MilitaryUiAction::SavePlan(plan));
            }
            if in_battle {
                ui.small("Formation and tactics are locked during battle.");
            }
        },
        ArmyDetailTab::Orders => {
            army_orders_placeholder(ui);
            if economic.owner == Some(player) && !in_battle && ui.button("Disband army").clicked() {
                action = Some(MilitaryUiAction::DisbandArmy);
            }
            if let Some(battle) = world.battles.iter().find(|battle| battle.province == province) {
                // Keep the existing retreat action available during active combat.
                let attacker = battle.attackers.units.iter().any(|unit| unit.owner == owner);
                if ui.button("Retreat").clicked() {
                    action = Some(MilitaryUiAction::Retreat {
                        battle: battle.id,
                        attacker,
                    });
                }
            }
        },
    }
    action
}

fn movement_army_panel(
    ui: &mut egui::Ui,
    world: &MilitaryWorld,
    economy: &EconomyWorld,
    graph: &[MilitaryProvince],
    owner: ForceOwner,
    id: u64,
) -> Option<MilitaryUiAction> {
    let movement =
        world.movements.iter().find(|movement| movement.id == id && movement.owner == owner)?;
    let destination = movement.destination().and_then(|id| economy.provinces.get(id))?;
    let terrain = movement.destination().and_then(|id| graph.get(id))?.terrain;
    let tab = army_detail_tabs(
        ui,
        egui::Id::new(("army-detail-tab", movement.origin, owner, Some(id))),
        world.rank(owner),
    );
    let mut plan = movement.plan;
    normalize_plan_preferences(&mut plan, &movement.units);
    let previous = plan;
    match tab {
        ArmyDetailTab::Overview => {
            army_summary(ui, &movement.units, terrain, world);
            edit_plan(ui, &mut plan, id as usize + 10_000, &movement.units, &world.config);
            if let Some(location) = movement.destination().and_then(|id| graph.get(id)) {
                deployment_editor(ui, &plan, &movement.units, location.terrain, &world.config);
            }
            ui.small(format!("Marching to {}", destination.name));
            ui.add(egui::ProgressBar::new(movement.interpolation() as f32).desired_height(12.));
        },
        ArmyDetailTab::Orders => army_orders_placeholder(ui),
    }
    (plan != previous).then_some(MilitaryUiAction::SaveMovementPlan {
        order: id,
        plan,
    })
}

/// A dedicated recruitment page keeps drafting separate from army orders.
fn recruitment_view(
    ui: &mut egui::Ui,
    world: &MilitaryWorld,
    economy: &EconomyWorld,
    province: usize,
    player: usize,
    _reported: bool,
) -> Option<MilitaryUiAction> {
    let state = world.provinces.get(province)?;
    let economic = economy.provinces.get(province)?;
    let directly_owned = economic.owner == Some(player);
    let in_battle = world.province_in_battle(province);
    let mut action = None;
    if !directly_owned {
        if let Some(project) = &state.recruitment {
            ui.label(format!(
                "{} · {:.1}/{:.1} months",
                project.unit_type.name(),
                project.progress,
                project.required_progress
            ));
        } else {
            ui.small("No recruitment in progress.");
        }
        return None;
    }
    let scale = super::viewport_ui_scale(ui.ctx().content_rect().size());
    ui.separator();
    ui.horizontal_wrapped(|ui| {
        ui.strong("RECRUITMENT");
        stat(
            ui,
            Icon::Plebeians,
            &super::resource_hud::format_population(economic.population[2]),
            "Plebeians available to draft.",
        );
        stat(
            ui,
            Icon::Citizens,
            &super::resource_hud::format_population(economic.population[1]),
            "Citizens available to draft.",
        );
    });
    ui.separator();
    // Size the roster from the empty panel, before the work strip consumes space.
    // Round down so the six rows still fit after egui rounds their positions.
    let empty_height = (ui.clip_rect().bottom() - ui.next_widget_position().y - 8. * scale).max(1.);
    let card_height =
        ((empty_height - 5. * ui.spacing().item_spacing.y) / 6.).max(62. * scale).floor();
    let speed = economy.config.recruitment_speed[economic.policies.recruitment as usize].max(0.001);
    let active = state.recruitment.as_ref().map(|project| {
        let progress = recruitment_completion(ui, project, speed);
        (
            Icon::Unit(project.unit_type),
            progress,
            format!(
                "{} · {:.0} months left",
                project.unit_type.name(),
                ((project.required_progress - project.progress).max(0.0) / speed).ceil()
            ),
        )
    });
    let queued: Vec<_> = state
        .recruitment_queue
        .iter()
        .enumerate()
        .map(|(index, project)| {
            (
                index,
                Icon::Unit(project.unit_type),
                format!(
                    "{}. {} · {:.0} months",
                    index + 1,
                    project.unit_type.name(),
                    project.required_progress / speed
                ),
            )
        })
        .collect();
    let has_work = active.is_some() || !queued.is_empty();
    if has_work {
        if let Some(queue_action) = work_queue(
            ui,
            egui::Id::new(("recruitment-queue", province)),
            "Recruiting",
            active,
            &queued,
            directly_owned,
            scale,
        ) {
            action = Some(match queue_action {
                WorkQueueAction::CancelActive => MilitaryUiAction::CancelRecruitment,
                WorkQueueAction::CancelQueued(index) => {
                    MilitaryUiAction::CancelQueuedRecruitment(index)
                },
            });
        }
        ui.separator();
    }
    let height = (ui.clip_rect().bottom() - ui.next_widget_position().y - 8. * scale).max(1.);
    let columns = if ui.available_width() >= 500. * scale + ui.spacing().item_spacing.x {
        2
    } else {
        1
    };
    let rows = UnitType::ALL.len().div_ceil(columns) as f32;
    let roster_height = rows * card_height + (rows - 1.) * ui.spacing().item_spacing.y;
    let mut scroll = egui::ScrollArea::vertical()
        .id_salt(("recruitment-grid", province, player))
        .max_height(height)
        .auto_shrink([false, false]);
    if !has_work && roster_height <= height {
        scroll = scroll.vertical_scroll_offset(0.);
    }
    scroll.show(ui, |ui| {
        let width = (ui.available_width() - (columns - 1) as f32 * ui.spacing().item_spacing.x)
            / columns as f32;
        for row in UnitType::ALL.chunks(columns) {
            ui.horizontal(|ui| {
                for &kind in row {
                    let def = world.config.unit(kind);
                    let reason = if in_battle {
                        Some("Recruitment is unavailable during battle")
                    } else if state.recruitment_queue_full() {
                        Some("Recruitment queue is full")
                    } else if def
                        .special_tag
                        .is_some_and(|tag| !recruitment_tags(&economic.name).contains(&tag))
                    {
                        Some("Requires the province's recruitment tradition")
                    } else if economic.population[def.manpower_class] < def.population_cost {
                        Some("Not enough population")
                    } else if economy
                        .players
                        .get(player)
                        .is_none_or(|p| p.resources[1] < def.metal_cost)
                    {
                        Some("Not enough Metal")
                    } else {
                        None
                    };
                    let (rect, response) = ui.allocate_exact_size(
                        egui::vec2(width, card_height),
                        if reason.is_none() {
                            egui::Sense::click()
                        } else {
                            egui::Sense::hover()
                        },
                    );
                    response.widget_info(|| {
                        egui::WidgetInfo::labeled(
                            egui::WidgetType::Button,
                            reason.is_none(),
                            kind.name(),
                        )
                    });
                    let painter = ui.painter().with_clip_rect(rect.intersect(ui.clip_rect()));
                    super::campaign_widgets::paint_purchase_background(
                        ui,
                        rect,
                        reason.is_none(),
                        response.hovered(),
                        response.is_pointer_button_down_on(),
                        3.,
                        scale,
                    );
                    let ink = if reason.is_none() {
                        super::province_panel::INK
                    } else {
                        super::campaign_widgets::UNAVAILABLE_PURCHASE_INK
                    };
                    painter.rect_stroke(
                        rect,
                        3.,
                        egui::Stroke::new(0.7, super::province_panel::RULE),
                        egui::StrokeKind::Inside,
                    );
                    let image_size = (card_height - 12. * scale).min(56. * scale).min(width * 0.25);
                    let image_rect = egui::Rect::from_center_size(
                        egui::pos2(rect.left() + 6. * scale + image_size * 0.5, rect.center().y),
                        egui::vec2(image_size, image_size),
                    );
                    let art_tint = egui::Color32::from_white_alpha(if reason.is_some() {
                        217
                    } else {
                        255
                    });
                    super::campaign_widgets::paint_raster_icon(
                        ui,
                        Icon::Unit(kind),
                        image_rect,
                        art_tint,
                    );
                    let content_left = image_rect.right() + 8. * scale;
                    let content_width = (rect.right() - content_left - 6. * scale).max(1.);
                    let title = painter.layout(
                        kind.name().to_owned(),
                        egui::FontId::proportional(14. * scale),
                        ink,
                        content_width,
                    );
                    let title_top =
                        rect.center().y - (title.size().y + 5. * scale + 20. * scale) * 0.5;
                    let costs_top = title_top + title.size().y + 5. * scale;
                    painter.galley(egui::pos2(content_left, title_top), title, ink);
                    let stats = [
                        (
                            if def.manpower_class == 1 {
                                Icon::Citizens
                            } else {
                                Icon::Plebeians
                            },
                            def.population_cost,
                        ),
                        (Icon::Metal, def.metal_cost),
                        (Icon::Coin, def.coin_per_month),
                        (Icon::Food, def.food_per_month),
                        (Icon::Duration, (def.recruitment_months / speed).ceil()),
                    ];
                    let costs = stats.map(|(art, value)| {
                        (
                            art,
                            painter.layout_no_wrap(
                                super::policy_widgets::compact_decimal(value),
                                egui::FontId::proportional(13. * scale),
                                ink,
                            ),
                        )
                    });
                    let costs_width: f32 =
                        costs.iter().map(|(_, value)| 21. * scale + value.size().x).sum();
                    let gap = ((content_width - costs_width) / (costs.len() - 1) as f32)
                        .clamp(0., 10. * scale);
                    let mut x = content_left;
                    for (art, value) in costs {
                        super::campaign_widgets::paint_raster_icon(
                            ui,
                            art,
                            egui::Rect::from_min_size(
                                egui::pos2(x, costs_top),
                                egui::vec2(18., 20.) * scale,
                            ),
                            art_tint,
                        );
                        let text_position = egui::pos2(
                            x + 21. * scale,
                            costs_top + (20. * scale - value.size().y) * 0.5,
                        );
                        x = text_position.x + value.size().x + gap;
                        painter.galley(text_position, value, ink);
                    }
                    if response.clicked() && reason.is_none() {
                        action = Some(MilitaryUiAction::Recruit(kind));
                    }
                    let response = if reason.is_none() {
                        response.on_hover_cursor(egui::CursorIcon::PointingHand)
                    } else {
                        response
                    };
                    response.on_hover_ui(|ui| {
                        recruitment_hover(ui, kind, &world.config, speed, reason, scale)
                    });
                }
            });
        }
    });
    action
}

/// Use the buildings' game-clock fraction; the monthly tick still completes cohorts.
fn recruitment_completion(ui: &egui::Ui, project: &RecruitmentProject, speed: f64) -> f32 {
    let fraction = ui.ctx().data(|data| {
        data.get_temp::<f32>(egui::Id::new("campaign-construction-month-fraction")).unwrap_or(0.0)
    });
    ((project.progress + speed * f64::from(fraction)) / project.required_progress.max(0.001))
        .clamp(0.0, 0.9999) as f32
}

/// Match the building hover card, keeping every detail beside the illustration.
fn recruitment_hover(
    ui: &mut egui::Ui,
    kind: UnitType,
    config: &MilitaryConfig,
    speed: f64,
    reason: Option<&str>,
    scale: f32,
) {
    let definition = config.unit(kind);
    let width = (440.0 * scale).min(ui.ctx().content_rect().width() - 24.0);
    ui.set_width(width.max(120.0));
    ui.horizontal_top(|ui| {
        icon(ui, Icon::Unit(kind), 112.0 * scale);
        ui.add_space(8.0 * scale);
        let width = ui.available_width();
        ui.vertical(|ui| {
            ui.set_width(width);
            ui.add(
                egui::Label::new(egui::RichText::new(kind.name()).strong().size(20.0 * scale))
                    .wrap(),
            );
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing.x = 14.0 * scale;
                for (art, value, explanation) in [
                    (
                        if definition.manpower_class == 1 {
                            Icon::Citizens
                        } else {
                            Icon::Plebeians
                        },
                        super::resource_hud::format_population(definition.population_cost),
                        if definition.manpower_class == 1 {
                            "Citizen population units drafted"
                        } else {
                            "Plebeian population units drafted"
                        },
                    ),
                    (Icon::Metal, format!("{:.0}", definition.metal_cost), "Metal"),
                    (
                        Icon::Coin,
                        super::policy_widgets::compact_decimal(definition.coin_per_month),
                        "Monthly Coin wages at full strength",
                    ),
                    (
                        Icon::Food,
                        super::policy_widgets::compact_decimal(definition.food_per_month),
                        "Food",
                    ),
                    (
                        Icon::Duration,
                        format!("{:.0}", (definition.recruitment_months / speed).ceil()),
                        "Recruitment time",
                    ),
                ] {
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = 4.0 * scale;
                        icon(ui, art, 28.0 * scale);
                        ui.label(egui::RichText::new(value).strong().size(17.0 * scale));
                    })
                    .response
                    .on_hover_text(explanation);
                }
            });
            if let Some(reason) = reason {
                ui.add(
                    egui::Label::new(
                        egui::RichText::new(reason)
                            .size(14.0 * scale)
                            .color(egui::Color32::from_rgb(170, 45, 35)),
                    )
                    .wrap(),
                );
            }
            ui.add_space(6.0 * scale);
            ui.add(
                egui::Label::new(egui::RichText::new(unit_description(kind)).size(14.0 * scale))
                    .wrap(),
            );
            ui.add_space(12.0 * scale);
            egui::Grid::new(("recruitment-combat-stats", kind))
                .num_columns(3)
                .spacing(egui::vec2(10.0, 4.0) * scale)
                .striped(true)
                .show(ui, |ui| {
                    for (art, label, value) in [
                        (Icon::MilitaryPower, "Manpower", definition.cohort_people().to_string()),
                        (Icon::Offense, "Offense", format!("{:.2}", definition.offense)),
                        (Icon::Defense, "Defense", format!("{:.2}", definition.defense)),
                        (Icon::Speed, "Speed", format!("{:.1}", definition.movement_speed)),
                        (Icon::Maneuver, "Maneuver", definition.maneuver.to_string()),
                    ] {
                        icon(ui, art, 24.0 * scale);
                        ui.label(egui::RichText::new(label).size(14.0 * scale));
                        ui.label(egui::RichText::new(value).strong().size(14.0 * scale));
                        ui.end_row();
                    }
                });
            ui.add_space(12.0 * scale);
            cohort_capabilities(ui, kind, config, scale);
        });
    });
}

fn cohort_capabilities(ui: &mut egui::Ui, kind: UnitType, config: &MilitaryConfig, scale: f32) {
    ui.strong(egui::RichText::new("Cohort capabilities").size(14.0 * scale));
    for opponent in UnitType::ALL {
        let adjustment = (config.matchups[kind as usize][opponent as usize] - 1.) * 100.;
        if adjustment.abs() <= 0.01 {
            continue;
        }
        let color = if adjustment > 0. {
            egui::Color32::from_rgb(48, 112, 60)
        } else {
            egui::Color32::from_rgb(170, 45, 35)
        };
        ui.horizontal_top(|ui| {
            ui.label(egui::RichText::new("•").size(14.0 * scale).color(egui::Color32::BLACK));
            let mut label = egui::text::LayoutJob::default();
            let format = egui::TextFormat {
                font_id: egui::FontId::proportional(14.0 * scale),
                color: egui::Color32::BLACK,
                ..Default::default()
            };
            label.append(&format!("Against {}: ", opponent.name()), 0.0, format.clone());
            label.append(
                &format!("{adjustment:+.0}%"),
                0.0,
                egui::TextFormat {
                    color,
                    ..format
                },
            );
            ui.add(egui::Label::new(label).wrap());
        });
    }
}

fn unit_description(kind: UnitType) -> &'static str {
    use UnitType::*;
    match kind {
        LightInfantry => "Lightly equipped foot soldiers who fill the battle line at a low Metal cost.",
        HeavyInfantry => "Armored foot soldiers with strong offense and defense for the center of the battle line.",
        Archers => "Foot archers who fire from the support row behind the frontline.",
        LightCavalry => "Fast mounted soldiers with the maneuver to reach targets along the battle line.",
        HeavyCavalry => "Armored mounted soldiers with powerful attacks and strong defenses.",
        HorseArchers => "Mounted bowmen combining fast movement with wide attack reach.",
        WarChariots => "Chariot crews that fight in the battle line and strike hard against light infantry.",
        WarCamels => "Camel-mounted soldiers with wide attack reach and an advantage in desert terrain.",
        WarElephants => "Elephants and their crews bring powerful offense and defense at a high Food cost.",
        Ballista => "Bolt-throwing artillery that fires from the support row and helps suppress fortifications.",
        Catapult => "Stone-throwing artillery that supports the frontline and helps suppress fortifications.",
    }
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

/// Use the same icon, caption and value badges as the province Overview.
fn army_summary(
    ui: &mut egui::Ui,
    units: &[Unit],
    terrain: MilitaryTerrain,
    world: &MilitaryWorld,
) {
    let scale = super::viewport_ui_scale(ui.ctx().content_rect().size());
    let morale = army_morale(units);
    let food: f64 = units.iter().map(|unit| unit.food_demand(&world.config)).sum();
    let columns = 4;
    let gap = 6. * scale;
    let width = (ui.available_width() - (columns - 1) as f32 * gap) / columns as f32;
    let terrain_tip = format!(
        "{terrain:?} terrain: battlefield width is {} cohort slots.",
        world.config.combat_widths[terrain as usize]
    );
    let badges = [
        (
            Icon::MilitaryPower,
            "Cohorts",
            units.len().to_string(),
            "Total number of individual units in this province.",
        ),
        (
            Icon::Terrain,
            "Terrain",
            world.config.combat_widths[terrain as usize].to_string(),
            terrain_tip.as_str(),
        ),
        (
            Icon::Food,
            "Food demand",
            format!("{}/mo", format_food_demand(food)),
            "Monthly food demand.",
        ),
        (
            Icon::Morale,
            "Army morale",
            format!("{morale:.0}%"),
            "Average morale across all units in this province.",
        ),
    ];
    for row in badges.chunks(columns) {
        let (rect, _) = ui.allocate_exact_size(
            egui::vec2(ui.available_width(), 48. * scale),
            egui::Sense::hover(),
        );
        for (index, (symbol, caption, value, tip)) in row.iter().enumerate() {
            let cell = egui::Rect::from_min_size(
                rect.min + egui::vec2(index as f32 * (width + gap), 0.),
                egui::vec2(width, rect.height()),
            );
            super::campaign_economy::overview_badge(
                ui,
                cell,
                *symbol,
                caption,
                value,
                tip,
                if *symbol == Icon::Food && food > 0. {
                    super::resource_hud::hud_delta_color(-food)
                } else {
                    super::province_panel::INK
                },
                scale,
            );
        }
        ui.add_space(2. * scale);
    }
}

fn army_morale(units: &[Unit]) -> f64 {
    let manpower: f64 = units.iter().map(|unit| unit.current_manpower.max(0.)).sum();
    units.iter().map(|unit| unit.morale * unit.current_manpower.max(0.)).sum::<f64>()
        / manpower.max(0.001)
}

/// A full-width army row with public composition, morale and monthly upkeep.
fn stationed_army_row(
    ui: &mut egui::Ui,
    units: &[Unit],
    config: &MilitaryConfig,
    name: String,
    owner_color: egui::Color32,
    own: bool,
    can_disband: bool,
    scale: f32,
) -> (bool, bool) {
    use super::province_panel::{INK, RULE, TABLE_STRIPE};
    let composition: Vec<_> = UnitType::ALL
        .into_iter()
        .filter_map(|kind| {
            let count = units
                .iter()
                .filter(|unit| unit.unit_type == kind && unit.current_manpower > 0.)
                .count();
            (count > 0).then_some((kind, count))
        })
        .collect();
    // Reserve the right side for the same effect stack used on policy cards.
    let columns = ((ui.available_width() - 121. * scale) / (54. * scale)).floor().max(1.) as usize;
    let composition_rows = composition.len().div_ceil(columns);
    let row_columns = composition.len().div_ceil(composition_rows.max(1)).max(1);
    let height =
        (45. + 34. * composition_rows as f32).max(super::policy_widgets::ROW_HEIGHT) * scale;
    let (rect, response) = ui.allocate_exact_size(
        egui::vec2(ui.available_width(), height),
        if own {
            egui::Sense::click()
        } else {
            egui::Sense::hover()
        },
    );
    let painter = ui.painter().with_clip_rect(ui.clip_rect().intersect(rect));
    painter.rect_filled(
        rect,
        3. * scale,
        if own && response.hovered() {
            egui::Color32::from_rgb(226, 210, 180)
        } else {
            TABLE_STRIPE
        },
    );
    painter.rect_stroke(rect, 3. * scale, egui::Stroke::new(scale, RULE), egui::StrokeKind::Inside);
    let title_rect = egui::Rect::from_min_max(
        rect.min + egui::vec2(8., 8.) * scale,
        egui::pos2(
            rect.right()
                - if can_disband {
                    100.
                } else {
                    8.
                } * scale,
            rect.top() + 29. * scale,
        ),
    );
    let mut job = egui::text::LayoutJob::simple(
        name,
        egui::FontId::proportional(14. * scale),
        owner_color,
        (title_rect.width() - 26. * scale).max(1.),
    );
    job.wrap.max_rows = 1;
    job.wrap.break_anywhere = true;
    let army_icon = egui::Rect::from_min_size(title_rect.min, egui::vec2(20., 20.) * scale);
    paint_icon(ui, Icon::MilitaryPower, army_icon);
    painter.galley(
        title_rect.min + egui::vec2(26., 0.) * scale,
        painter.layout_job(job),
        owner_color,
    );
    let morale = army_morale(units);
    let food: f64 = units.iter().map(|unit| unit.food_demand(config)).sum();
    for (index, (symbol, value, tip)) in [
        (Icon::Morale, format!("{morale:.0}%"), "Army morale"),
        (Icon::Food, format!("{}/mo", format_food_demand(food)), "Food demand"),
    ]
    .into_iter()
    .enumerate()
    {
        let badge = super::policy_widgets::effect_rect(&painter, rect, scale, 2, index);
        let image = egui::Rect::from_min_size(
            badge.min + egui::vec2(1., 1.) * scale,
            egui::vec2(17., 17.) * scale,
        );
        paint_icon(ui, symbol, image);
        painter.text(
            badge.right_center() - egui::vec2(4. * scale, 0.),
            egui::Align2::RIGHT_CENTER,
            value,
            egui::FontId::proportional(11.5 * scale),
            if symbol == Icon::Food && food > 0. {
                super::resource_hud::hud_delta_color(-food)
            } else {
                INK
            },
        );
        ui.interact(badge, response.id.with(("stat", symbol)), egui::Sense::hover())
            .on_hover_text(tip);
    }
    for (index, (kind, count)) in composition.iter().enumerate() {
        let min = rect.left_bottom()
            + egui::vec2(
                8. + (index % row_columns) as f32 * 54.,
                -35. - (composition_rows - 1 - index / row_columns) as f32 * 34.,
            ) * scale;
        let image = egui::Rect::from_min_size(min, egui::vec2(30., 30.) * scale);
        paint_icon(ui, Icon::Unit(*kind), image);
        painter.text(
            image.right_center() + egui::vec2(3. * scale, 0.),
            egui::Align2::LEFT_CENTER,
            count.to_string(),
            egui::FontId::proportional(14.5 * scale),
            INK,
        );
        let unit_rect = egui::Rect::from_min_size(image.min, egui::vec2(52., 30.) * scale);
        ui.interact(unit_rect, response.id.with(("unit", *kind)), egui::Sense::hover())
            .on_hover_text(kind.name());
    }
    let disband = if can_disband {
        let button_rect = egui::Rect::from_min_size(
            egui::pos2(rect.right() - 92. * scale, rect.top() + 8. * scale),
            egui::vec2(84., 23.) * scale,
        );
        let button = ui.place(button_rect, egui::Button::new(""));
        paint_icon(
            ui,
            Icon::Cancel,
            egui::Rect::from_min_size(
                button_rect.min + egui::vec2(5., 3.) * scale,
                egui::vec2(17., 17.) * scale,
            ),
        );
        ui.painter().text(
            button_rect.min + egui::vec2(28., 11.5) * scale,
            egui::Align2::LEFT_CENTER,
            "Disband",
            egui::FontId::proportional(13. * scale),
            INK,
        );
        button.widget_info(|| {
            egui::WidgetInfo::labeled(egui::WidgetType::Button, true, "Disband army")
        });
        button
            .on_hover_text(
                "Disband this army. Soldiers will return to their respective population classes.",
            )
            .clicked()
    } else {
        false
    };
    let clicked = own && response.clicked() && !disband;
    if own {
        response.on_hover_cursor(egui::CursorIcon::PointingHand);
    };
    (clicked, disband)
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
        CombatTactic::Balanced => {
            include_bytes!(concat!(env!("OUT_DIR"), "/panel-icons/balanced-owl.png")) as &[u8]
        },
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

/// All tactic symbols use transparent artwork without a screenshot tile.
fn paint_tactic(ui: &egui::Ui, tactic: CombatTactic, rect: egui::Rect) {
    ui.painter().image(
        tactic_texture(ui, tactic),
        rect,
        egui::Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(1., 1.)),
        egui::Color32::WHITE,
    );
}

fn normalize_plan_preferences(plan: &mut BattlePlan, units: &[Unit]) {
    plan.flank_size = plan.normalized_flank_size();
    let available: Vec<_> = UnitType::ALL
        .into_iter()
        .filter(|kind| {
            !kind.is_support()
                && units.iter().any(|u| u.unit_type == *kind && u.current_manpower > 0.)
        })
        .collect();
    if let Some(&fallback) = available.first() {
        for preference in
            [&mut plan.primary_unit_type, &mut plan.secondary_unit_type, &mut plan.flank_unit_type]
        {
            if !available.contains(preference) {
                *preference = fallback;
            }
        }
    }
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
        paint_tactic(ui, kind, egui::Rect::from_min_size(at(x, y), egui::vec2(size, size)));
    };
    paint_symbol(tactic, 7., 5., 40.);
    ui.painter().text(
        at(27., 55.),
        egui::Align2::CENTER_CENTER,
        if tactic == CombatTactic::Balanced {
            "Balanced".to_owned()
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
        "A steady formation with no tactic bonus or counter penalty.".to_owned()
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
    let scale = super::viewport_ui_scale(ui.ctx().content_rect().size());
    ui.push_id(("military-plan", province), |ui| {
        super::policy_widgets::section(ui, scale, "Deployment");
        let width = (ui.available_width() - 4. * ui.spacing().item_spacing.x) / 5.;
        ui.horizontal(|ui| {
            let (rect, tactic_button) = formation_role_card(ui, "Tactics", width, scale);
            paint_tactic(
                ui,
                plan.tactic,
                egui::Rect::from_center_size(
                    rect.center() - egui::vec2(0., 1.) * scale,
                    egui::vec2(28., 28.) * scale,
                ),
            );
            super::campaign_economy::ledger_text(
                ui,
                egui::Rect::from_min_max(
                    egui::pos2(rect.left() + 3. * scale, rect.bottom() - 20. * scale),
                    rect.max - egui::vec2(3., 3.) * scale,
                ),
                plan.tactic.name(),
                10. * scale,
                super::province_panel::INK,
                egui::Align::Center,
            );
            paint_dropdown_arrow(ui, rect, scale);
            let popup = egui::Popup::menu(&tactic_button).show(|ui| {
                ui.set_width(340. * scale);
                for tactic in CombatTactic::ALL {
                    if tactic_choice(ui, tactic, plan.tactic == tactic, units, config).clicked() {
                        plan.tactic = tactic;
                        ui.close();
                    }
                }
            });
            if let Some(popup) = popup {
                if popup.response.layer_id.order == tactic_button.layer_id.order {
                    ui.ctx().set_sublayer(tactic_button.layer_id, popup.response.layer_id);
                }
            }
            for (label, value) in [
                ("Frontline", &mut plan.primary_unit_type),
                ("Rear line", &mut plan.secondary_unit_type),
                ("Flanks", &mut plan.flank_unit_type),
            ] {
                let (rect, response) = formation_role_card(ui, label, width, scale);
                paint_icon(
                    ui,
                    Icon::Unit(*value),
                    egui::Rect::from_center_size(
                        rect.center() - egui::vec2(0., 1.) * scale,
                        egui::vec2(28., 28.) * scale,
                    ),
                );
                super::campaign_economy::ledger_text(
                    ui,
                    egui::Rect::from_min_max(
                        egui::pos2(rect.left() + 3. * scale, rect.bottom() - 20. * scale),
                        rect.max - egui::vec2(3., 3.) * scale,
                    ),
                    value.name(),
                    10. * scale,
                    super::province_panel::INK,
                    egui::Align::Center,
                );
                paint_dropdown_arrow(ui, rect, scale);
                if let Some(kind) = unit_type_dropdown(ui, &response, *value, units, scale) {
                    *value = kind;
                }
                response.on_hover_text(format!(
                    "{label}: {}. Choose a preferred cohort type for this formation role.",
                    value.name()
                ));
            }
            let (rect, response) = formation_role_card(ui, "Flank size", width, scale);
            ui.painter().text(
                rect.center(),
                egui::Align2::CENTER_CENTER,
                plan.flank_size.to_string(),
                egui::FontId::proportional(25. * scale),
                super::province_panel::INK,
            );
            ui.painter().text(
                egui::pos2(rect.center().x, rect.bottom() - 12. * scale),
                egui::Align2::CENTER_CENTER,
                "per side",
                egui::FontId::proportional(10. * scale),
                super::province_panel::INK,
            );
            paint_dropdown_arrow(ui, rect, scale);
            let popup = egui::Popup::menu(&response)
                .align(egui::RectAlign::BOTTOM_START)
                .gap(0.)
                .frame(egui::Frame::popup(ui.style()).inner_margin(4. * scale))
                .show(|ui| {
                    ui.set_width(140. * scale);
                    for size in [1, 2, 3, 4, 5] {
                        if ui
                            .add(
                                egui::Button::selectable(plan.flank_size == size, ())
                                    .left_text(format!("{size} per side"))
                                    .min_size(egui::vec2(ui.available_width(), 28. * scale)),
                            )
                            .clicked()
                        {
                            plan.flank_size = size;
                            ui.close();
                        }
                    }
                });
            if let Some(popup) = popup {
                if popup.response.layer_id.order == response.layer_id.order {
                    ui.ctx().set_sublayer(response.layer_id, popup.response.layer_id);
                }
            }
            response.on_hover_text(
                "Cohort slots per wing (up to 5 on each side). Terrain may reduce the effective wing width.",
            );
        });
    });
}

fn choice_fill(response: &egui::Response) -> egui::Color32 {
    if response.is_pointer_button_down_on()
        || egui::Popup::is_id_open(&response.ctx, egui::Popup::default_response_id(response))
    {
        egui::Color32::from_rgb(210, 187, 145)
    } else if response.hovered() {
        egui::Color32::from_rgb(226, 210, 180)
    } else {
        super::province_panel::TABLE_STRIPE
    }
}

fn paint_dropdown_arrow(ui: &egui::Ui, rect: egui::Rect, scale: f32) {
    let center = egui::pos2(rect.right() - 10. * scale, rect.top() + 12. * scale);
    ui.painter().line_segment(
        [center + egui::vec2(-3., -1.) * scale, center + egui::vec2(0., 2.) * scale],
        egui::Stroke::new(scale, super::province_panel::INK),
    );
    ui.painter().line_segment(
        [center + egui::vec2(0., 2.) * scale, center + egui::vec2(3., -1.) * scale],
        egui::Stroke::new(scale, super::province_panel::INK),
    );
}

/// Shared full-row selection treatment, matching the province name dropdown.
fn unit_type_dropdown(
    ui: &mut egui::Ui,
    response: &egui::Response,
    preferred: UnitType,
    units: &[Unit],
    scale: f32,
) -> Option<UnitType> {
    let mut selected = None;
    let popup = egui::Popup::menu(response)
        .align(egui::RectAlign::BOTTOM_START)
        .gap(0.)
        .frame(egui::Frame::popup(ui.style()).inner_margin(4. * scale))
        .show(|ui| {
            ui.set_width(220. * scale);
            egui::ScrollArea::vertical().max_height(260. * scale).show(ui, |ui| {
                for kind in UnitType::ALL.into_iter().filter(|kind| {
                    !kind.is_support()
                        && units
                            .iter()
                            .any(|unit| unit.unit_type == *kind && unit.current_manpower > 0.)
                }) {
                    let (rect, response) = ui.allocate_exact_size(
                        egui::vec2(ui.available_width(), 32. * scale),
                        egui::Sense::click(),
                    );
                    let response = response.on_hover_cursor(egui::CursorIcon::PointingHand);
                    let pressed = response.is_pointer_button_down_on();
                    let is_selected = preferred == kind;
                    let fill = if pressed {
                        egui::Color32::from_rgb(199, 163, 111)
                    } else if is_selected && response.hovered() {
                        egui::Color32::from_rgb(211, 185, 145)
                    } else if is_selected {
                        egui::Color32::from_rgb(219, 200, 168)
                    } else if response.hovered() {
                        egui::Color32::from_rgb(231, 213, 181)
                    } else {
                        super::province_panel::PAPER
                    };
                    ui.painter().rect_filled(rect, 3. * scale, fill);
                    paint_icon(
                        ui,
                        Icon::Unit(kind),
                        egui::Rect::from_center_size(
                            rect.left_center() + egui::vec2(17. * scale, 0.),
                            egui::vec2(26., 26.) * scale,
                        ),
                    );
                    ui.painter().text(
                        rect.left_center() + egui::vec2(38. * scale, 0.),
                        egui::Align2::LEFT_CENTER,
                        kind.name(),
                        egui::FontId::proportional(14. * scale),
                        super::province_panel::INK,
                    );
                    if response.clicked() {
                        selected = Some(kind);
                        ui.close();
                    }
                }
            });
        });
    if let Some(popup) = popup {
        if popup.response.layer_id.order == response.layer_id.order {
            ui.ctx().set_sublayer(response.layer_id, popup.response.layer_id);
        }
    }
    selected
}

fn formation_role_card(
    ui: &mut egui::Ui,
    label: &str,
    width: f32,
    scale: f32,
) -> (egui::Rect, egui::Response) {
    let (rect, response) =
        ui.allocate_exact_size(egui::vec2(width, 66. * scale), egui::Sense::click());
    ui.painter().rect_filled(rect, 3. * scale, choice_fill(&response));
    ui.painter().rect_stroke(
        rect,
        3. * scale,
        egui::Stroke::new(0.7 * scale, super::province_panel::RULE),
        egui::StrokeKind::Inside,
    );
    ui.painter().text(
        egui::pos2(rect.center().x, rect.top() + 12. * scale),
        egui::Align2::CENTER_CENTER,
        label,
        egui::FontId::proportional(11. * scale),
        super::province_panel::INK,
    );
    (rect, response)
}

/// Read-only preview of the automatically deployed cohorts and their selected roles.
fn deployment_editor(
    ui: &mut egui::Ui,
    plan: &BattlePlan,
    units: &[Unit],
    terrain: MilitaryTerrain,
    config: &MilitaryConfig,
) {
    let scale = super::viewport_ui_scale(ui.ctx().content_rect().size());
    let width = config.combat_widths[terrain as usize];
    let formation = deploy_formation(
        units,
        &[(units.first().map_or(ForceOwner::Player(0), |unit| unit.owner), *plan)].into(),
        width,
        &BTreeSet::new(),
        config,
    );
    let gap = 6. * scale;
    let spacing = 2. * scale;
    let cell = ((ui.available_width() - spacing * width.saturating_sub(1) as f32 - 2. * gap)
        / width.max(1) as f32)
        .min(34. * scale);
    let row_width = width as f32 * cell + width.saturating_sub(1) as f32 * spacing + 2. * gap;
    let wing = formation.flank_size;
    let center = width.saturating_sub(2 * wing);
    ui.add_space(8. * scale);
    ui.vertical_centered(|ui| {
        let (labels, _) =
            ui.allocate_exact_size(egui::vec2(row_width, 17. * scale), egui::Sense::hover());
        let group_center = |start: usize, count: usize| {
            labels.left()
                + start as f32 * (cell + spacing)
                + count as f32 * cell * 0.5
                + count.saturating_sub(1) as f32 * spacing * 0.5
                + if start > 0 {
                    gap
                } else {
                    0.
                }
                + if start > wing {
                    gap
                } else {
                    0.
                }
        };
        let left_label = if wing < 3 {
            format!("L · {wing}")
        } else {
            format!("Left flank · {wing}")
        };
        let right_label = if wing < 3 {
            format!("R · {wing}")
        } else {
            format!("Right flank · {wing}")
        };
        for (label, x) in [
            (left_label, group_center(0, wing)),
            (format!("Frontline · {center}"), group_center(wing, center)),
            (right_label, group_center(wing + center, wing)),
        ] {
            ui.painter().text(
                egui::pos2(x, labels.center().y),
                egui::Align2::CENTER_CENTER,
                label,
                egui::FontId::proportional(10. * scale),
                super::province_panel::INK,
            );
        }
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = spacing;
            ui.add_space(((ui.available_width() - row_width) * 0.5).max(0.));
            for (index, id) in formation.front.iter().enumerate() {
                if index == wing || index == width - wing {
                    ui.add_space(gap - spacing);
                }
                let unit = id.and_then(|id| units.iter().find(|unit| unit.id == id));
                deployment_slot(ui, unit, cell);
            }
        });
        // The second row shows protected support and uncommitted center replacements.
        let mut rear: Vec<_> = formation
            .reserves
            .iter()
            .filter_map(|id| units.iter().find(|unit| unit.id == *id))
            .filter(|unit| !unit.unit_type.is_support())
            .collect();
        rear.sort_by_key(|unit| (unit.unit_type != plan.secondary_unit_type, unit.id));
        rear.extend(
            formation
                .support
                .iter()
                .flatten()
                .filter_map(|id| units.iter().find(|unit| unit.id == *id)),
        );
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = spacing;
            ui.add_space(((ui.available_width() - row_width) * 0.5).max(0.));
            for index in 0..width {
                if index == wing || index == width - wing {
                    ui.add_space(gap - spacing);
                }
                if index < wing || index >= width - wing {
                    ui.allocate_exact_size(
                        egui::vec2(cell, cell + 6. * scale),
                        egui::Sense::hover(),
                    );
                } else {
                    deployment_slot(ui, rear.get(index - wing).copied(), cell);
                }
            }
        });
    });
    ui.add_space(12. * scale);
    army_unit_type_row(ui, units, config, scale);
}

fn deployment_slot(ui: &mut egui::Ui, unit: Option<&Unit>, cell: f32) {
    let scale = super::viewport_ui_scale(ui.ctx().content_rect().size());
    let (rect, response) =
        ui.allocate_exact_size(egui::vec2(cell, cell + 6. * scale), egui::Sense::hover());
    ui.painter().rect_filled(rect, 3., super::province_panel::TABLE_STRIPE);
    ui.painter().rect_stroke(
        rect,
        3.,
        egui::Stroke::new(0.8, super::province_panel::RULE),
        egui::StrokeKind::Inside,
    );
    if let Some(unit) = unit {
        paint_icon(ui, Icon::Unit(unit.unit_type), rect.shrink(3.));
    } else {
        ui.painter().text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            "·",
            egui::FontId::proportional(20.),
            super::province_panel::RULE,
        );
    }
    response
        .on_hover_cursor(egui::CursorIcon::Default)
        .on_hover_text(unit.map_or("Empty position", |unit| unit.unit_type.name()));
}

/// Read-only cohort counts by type below the deployment preview.
fn army_unit_type_row(ui: &mut egui::Ui, units: &[Unit], config: &MilitaryConfig, scale: f32) {
    let kinds: Vec<_> = UnitType::ALL
        .into_iter()
        .filter(|kind| {
            units.iter().any(|unit| unit.unit_type == *kind && unit.current_manpower > 0.)
        })
        .collect();
    let width = ((ui.available_width() - kinds.len().saturating_sub(1) as f32 * 4. * scale)
        / kinds.len().max(1) as f32)
        .min(70. * scale);
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 4. * scale;
        for kind in kinds {
            let cohorts: Vec<_> = units
                .iter()
                .filter(|unit| unit.unit_type == kind && unit.current_manpower > 0.)
                .collect();
            let (rect, response) =
                ui.allocate_exact_size(egui::vec2(width, 66. * scale), egui::Sense::hover());
            ui.painter().rect_filled(rect, 2. * scale, super::province_panel::TABLE_STRIPE);
            ui.painter().rect_stroke(
                rect,
                2. * scale,
                egui::Stroke::new(0.7 * scale, super::province_panel::RULE),
                egui::StrokeKind::Inside,
            );
            paint_icon(
                ui,
                Icon::Unit(kind),
                egui::Rect::from_center_size(
                    egui::pos2(rect.center().x, rect.top() + 21. * scale),
                    egui::vec2((width - 6. * scale).min(38. * scale), 38. * scale),
                ),
            );
            ui.painter().text(
                egui::pos2(rect.center().x, rect.top() + 51. * scale),
                egui::Align2::CENTER_CENTER,
                cohorts.len().to_string(),
                egui::FontId::proportional(13. * scale),
                super::province_panel::INK,
            );
            response
                .on_hover_cursor(egui::CursorIcon::Default)
                .on_hover_ui(|ui| army_unit_type_hover(ui, kind, &cohorts, config, scale));
        }
    });
}

/// Show actual survivors and upkeep beside the same unit art and combat facts as recruitment.
fn army_unit_type_hover(
    ui: &mut egui::Ui,
    kind: UnitType,
    cohorts: &[&Unit],
    config: &MilitaryConfig,
    scale: f32,
) {
    let definition = config.unit(kind);
    let soldiers: u64 = cohorts.iter().map(|unit| unit.people()).sum();
    let full_soldiers: u64 = cohorts.iter().map(|unit| unit.max_people()).sum();
    let manpower: f64 = cohorts.iter().map(|unit| unit.current_manpower).sum();
    let food: f64 = cohorts.iter().map(|unit| unit.food_demand(config)).sum();
    let wages: f64 = cohorts.iter().map(|unit| unit.coin_demand(config)).sum();
    let training = cohorts.iter().map(|unit| unit.training * unit.current_manpower).sum::<f64>()
        / manpower.max(0.001);
    let width = (440. * scale).min(ui.ctx().content_rect().width() - 24.);
    ui.set_width(width.max(120.));
    ui.horizontal_top(|ui| {
        icon(ui, Icon::Unit(kind), 112. * scale);
        ui.add_space(8. * scale);
        let width = ui.available_width();
        ui.vertical(|ui| {
            ui.set_width(width);
            ui.label(egui::RichText::new(kind.name()).strong().size(20. * scale));
            ui.add_space(4. * scale);
            egui::Grid::new(("army-unit-summary", kind))
                .num_columns(3)
                .spacing(egui::vec2(8., 4.) * scale)
                .show(ui, |ui| {
                    for (art, label, value) in [
                        (
                            Icon::Population,
                            "Soldiers",
                            format!(
                                "{} / {}",
                                format_person_count(soldiers),
                                format_person_count(full_soldiers)
                            ),
                        ),
                        (Icon::Unit(kind), "Cohorts", cohorts.len().to_string()),
                        (
                            Icon::Recruitment,
                            "New cohort",
                            format_person_count(definition.cohort_people()),
                        ),
                        (Icon::Food, "Food", format!("{food:.1}/mo")),
                        (Icon::Coin, "Wages", format!("{wages:.1}/mo")),
                        (Icon::MilitaryPower, "Training", format!("{training:.0}%")),
                    ] {
                        icon(ui, art, 22. * scale);
                        ui.label(egui::RichText::new(label).size(14. * scale));
                        ui.label(egui::RichText::new(value).strong().size(14. * scale));
                        ui.end_row();
                    }
                });
        });
    });
    ui.add_space(6. * scale);
    ui.separator();
    egui::Grid::new(("army-unit-combat-stats", kind))
        .num_columns(3)
        .spacing(egui::vec2(10., 4.) * scale)
        .striped(true)
        .show(ui, |ui| {
            for (art, label, value) in [
                (Icon::Offense, "Offense", format!("{:.2}", definition.offense)),
                (Icon::Defense, "Defense", format!("{:.2}", definition.defense)),
                (Icon::Speed, "Speed", format!("{:.1}", definition.movement_speed)),
                (Icon::Maneuver, "Maneuver", definition.maneuver.to_string()),
            ] {
                icon(ui, art, 24. * scale);
                ui.label(egui::RichText::new(label).size(14. * scale));
                ui.label(egui::RichText::new(value).strong().size(14. * scale));
                ui.end_row();
            }
        });
    ui.add_space(10. * scale);
    cohort_capabilities(ui, kind, config, scale);
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
            let row_width = row.len() as f32 * cell + row.len().saturating_sub(1) as f32 * 2.;
            ui.add_space(((ui.available_width() - row_width) * 0.5).max(0.));
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
                    paint_icon(ui, Icon::Unit(unit.unit_type), image);
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
                        "{} · {}\n{} manpower ({:.1}% strength)\nTraining {:.0}",
                        unit.unit_type.name(),
                        if is_support {
                            "Support"
                        } else if flank {
                            "Flank"
                        } else {
                            "Center"
                        },
                        cohort_manpower_text(unit),
                        unit.manpower_ratio() * 100.,
                        unit.training,
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
#[allow(dead_code)] // Retained combat detail renderer for the future orders page.
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
        .units
        .iter()
        .map(|u| config.unit(u.unit_type).siege_power * u.manpower_ratio())
        .sum();
    let fort = (f64::from(battle.fortification_level) * config.fort_defense_per_level)
        .min(config.fort_defense_cap);
    let suppression =
        (siege * config.siege_suppression_per_point).min(config.siege_suppression_cap);
    ui.horizontal_wrapped(|ui| {
        ui.small(format!("{:?} · {} slots",battle.terrain,config.combat_widths[battle.terrain as usize])).on_hover_text(format!("Defender terrain bonus: +{:.0}%",(config.terrain_defense[battle.terrain as usize]-1.)*100.));
        stat(ui,Icon::Building(BuildingType::CityWalls),&format!("+{:.0}%",fort*100.),&format!("Level {} fortification: base defense +{:.0}%. Total siege power {:.1} could suppress {:.0}% of that bonus. Actual suppression depends on private deployment. Protected support fights at {:.0}% power; exposed support takes {:.0}% extra casualties.",battle.fortification_level,fort*100.,siege,suppression*100.,config.support_effectiveness*100.,(config.exposed_support_casualties-1.)*100.));
    });
    for (attacker, label, side) in
        [(true, "ATTACKERS", &battle.attackers), (false, "DEFENDERS", &battle.defenders)]
    {
        ui.horizontal_wrapped(|ui| {
            ui.strong(label);
            stat(
                ui,
                Icon::Population,
                &format_people(side.manpower()),
                "Surviving people, including reserves and routed cohorts.",
            );
            let own: Vec<_> = side.units.iter().filter(|unit| unit.owner == owner).collect();
            if !own.is_empty() {
                let morale =
                    own.iter().map(|unit| unit.morale * unit.current_manpower).sum::<f64>()
                        / own.iter().map(|unit| unit.current_manpower).sum::<f64>().max(0.001);
                stat(
                    ui,
                    Icon::Morale,
                    &format!("{morale:.0}"),
                    "Your troops' manpower-weighted Morale.",
                );
            }
            unit_composition(ui, &side.units);
        });
        for (&force_owner, plan) in
            side.plans.iter().filter(|(force_owner, _)| **force_owner == owner)
        {
            let rank = side.ranks.get(&force_owner).copied().unwrap_or_default();
            let detail=format!("{}: {} gives +{:.0} Morale.\nTactic and formation are locked until battle ends. Fit follows actual surviving deployed units.",owner_name(force_owner),rank.name(),config.rank_morale[rank as usize]);
            ui.small(format!(
                "{} · {} · {:.0}% fit",
                owner_name(force_owner),
                plan.tactic.name(),
                side.tactic_fit(force_owner, config) * 100.
            ))
            .on_hover_text(detail);
        }
        if side.units.iter().any(|unit| unit.owner == owner) {
            let own_units: Vec<_> =
                side.units.iter().filter(|unit| unit.owner == owner).cloned().collect();
            let own_ids: BTreeSet<_> = own_units.iter().map(|unit| unit.id).collect();
            let mut formation = side.formation.clone();
            for slot in formation.front.iter_mut().chain(&mut formation.support) {
                if slot.is_some_and(|id| !own_ids.contains(&id)) {
                    *slot = None;
                }
            }
            formation.reserves.retain(|id| own_ids.contains(id));
            formation_row(ui, &formation, &own_units);
        }
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
                            cohort_facts(
                                ui,
                                unit,
                                config,
                                (unit.owner == owner).then_some(role),
                                0.,
                                unit.owner == owner,
                            );
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

fn military_owner_name(owner: ForceOwner, world: &MilitaryWorld) -> String {
    match owner {
        ForceOwner::Local(province) if world.provinces[province].slave_rebellion => {
            "Slave rebels".to_owned()
        },
        _ => owner_name(owner),
    }
}

fn army_name(owner: ForceOwner, economy: &EconomyWorld, world: &MilitaryWorld) -> String {
    if matches!(owner, ForceOwner::Local(province) if world.provinces[province].slave_rebellion) {
        return "Slave rebel army".to_owned();
    }
    let name = match owner {
        ForceOwner::Player(_) => owner_name(owner),
        ForceOwner::Local(province) => economy
            .provinces
            .get(province)
            .map_or("Province", |province| province.name.as_str())
            .to_owned(),
    };
    format!("{name}'s army")
}

fn army_owner_color(ctx: &egui::Context, owner: ForceOwner) -> egui::Color32 {
    match owner {
        ForceOwner::Player(player) => ctx
            .data(|data| {
                data.get_temp::<Vec<egui::Color32>>(egui::Id::new("campaign-player-colors"))
            })
            .and_then(|colors| colors.get(player).copied())
            .unwrap_or(super::PLAYER_COLORS[player % super::PLAYER_COLORS.len()]),
        ForceOwner::Local(_) => egui::Color32::from_rgb(118, 118, 111),
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

/// Convert economy population units into a whole-person cohort count.
fn format_people(population_units: f64) -> String {
    ((population_units.max(0.) * PEOPLE_PER_POPULATION).round() as u64).to_string()
}

fn format_person_count(count: u64) -> String {
    let digits = count.to_string();
    let mut grouped = String::with_capacity(digits.len() + digits.len() / 3);
    for (index, digit) in digits.chars().enumerate() {
        if index > 0 && (digits.len() - index).is_multiple_of(3) {
            grouped.push(',');
        }
        grouped.push(digit);
    }
    grouped
}

/// Show surviving and original cohort sizes as whole people.
fn cohort_manpower_text(unit: &Unit) -> String {
    format!("{}/{}", unit.people(), unit.max_people())
}

/// Show the identifying image and manpower beside two compact condition meters.
fn cohort_facts(
    ui: &mut egui::Ui,
    unit: &Unit,
    config: &MilitaryConfig,
    role: Option<&str>,
    reserved_width: f32,
    private: bool,
) {
    icon(ui, Icon::Unit(unit.unit_type), 36.)
        .on_hover_text(unit_definition_tip(unit.unit_type, config));
    let meter_width = if private {
        84.
    } else {
        0.
    };
    let text_width =
        (ui.available_width() - meter_width - reserved_width - ui.spacing().item_spacing.x * 2.)
            .max(40.);
    ui.allocate_ui_with_layout(egui::vec2(text_width,40.),egui::Layout::top_down(egui::Align::Min),|ui| {
        ui.add(egui::Label::new(egui::RichText::new(unit.unit_type.name()).strong()).wrap());
        ui.horizontal_wrapped(|ui| {
            ui.small(cohort_manpower_text(unit)).on_hover_text(format!("{:.1}% strength. Each casualty removes one person; losses are permanent and cohorts never reinforce automatically.",unit.manpower_ratio()*100.));
            if let Some(role)=role{ui.small(role);}else{
                stat(ui,Icon::Food,&format!("{:.1}",unit.food_demand(config)),"Monthly Food upkeep. All units share the owner's supply ratio, including movement and combat.");
            }
        });
    });
    if private {
        ui.allocate_ui_with_layout(egui::vec2(meter_width,40.),egui::Layout::top_down(egui::Align::Min),|ui| {
        condition_meter(ui,"T",unit.training,egui::Color32::from_rgb(157,117,61),&format!("Training {:.0}/100\nAttack +{:.1}%; defense +{:.1}%. Supplied troops gain {:.0} Training per month.",unit.training,config.training_attack*unit.training,config.training_defense*unit.training,config.passive_training));

    });
    }
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
    let mut tip = format!(
        "{} · {} people per cohort\nOffense {:.2} · Defense {:.2}\nSpeed {:.1} · Maneuver {}",
        kind.name(),
        definition.cohort_people(),
        definition.offense,
        definition.defense,
        definition.movement_speed,
        definition.maneuver
    );
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
#[cfg(test)]
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
        .map(|unit| {
            let mut unit = unit.clone();
            unit.training = world.config.starting_training;
            unit
        })
        .collect()
}

#[cfg(test)]
#[path = "../../tests/unit/military_ui.rs"]
mod tests;
