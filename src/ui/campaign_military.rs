//! Campaign military details, matching the enclosing parchment panel palette.
//!
//! Rendering returns a typed action so the campaign bridge owns transactional
//! diplomacy, civilian population, and global-resource mutations.

use super::spectator::InspectionHover;
use std::collections::BTreeSet;

use super::campaign_widgets::{
    directory_owner_marker, directory_province_name, icon, paint_directory_row, paint_icon,
    paint_purchase_background, stat, work_queue, Icon, WorkQueueAction,
};
use super::resource_hud::format_food_demand;
use bevy::prelude::*;
use bevy_egui::egui;

use crate::game::economy::{BuildingType, EconomyWorld};
use crate::game::military::*;

#[path = "military_battle.rs"]
mod battle_panel;
#[path = "military_orders.rs"]
mod orders;
pub(in crate::app) use battle_panel::{draw_battle_panel, open_battle_panel};
pub(in crate::app) use orders::{draw_orders_menu, prepare_orders};

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
    /// Consolidate the stationary army's damaged cohorts by type.
    MergeArmy,
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
    /// Declare hostility and march along the previewed invasion route.
    Attack {
        destination: usize,
        units: Vec<UnitId>,
        route: Vec<ProvinceId>,
        plan: BattlePlan,
    },
    /// Enter an independent province and exert limited non-combat control.
    Pressure {
        destination: usize,
        units: Vec<UnitId>,
        route: Vec<ProvinceId>,
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
const ARMY_COLLAPSED_ID: &str = "campaign-army-collapsed";
const ARMY_TITLE_SEARCH_ID: &str = "campaign-army-title-search";
const ARMY_PROVINCE_CLICK_ID: &str = "campaign-army-province-click";

pub(in crate::app) fn take_army_province_click(ctx: &egui::Context) -> Option<usize> {
    ctx.data_mut(|data| data.remove_temp::<usize>(egui::Id::new(ARMY_PROVINCE_CLICK_ID)))
}

#[derive(Clone, Default)]
struct ArmyTitleSearch {
    open: bool,
    query: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct ArmyPanelSelection {
    province: usize,
    player: usize,
    owner: ForceOwner,
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
    orders::clear(ctx);
    ctx.data_mut(|data| {
        data.remove::<bool>(egui::Id::new(ARMY_COLLAPSED_ID));
        data.remove::<ArmyTitleSearch>(egui::Id::new(ARMY_TITLE_SEARCH_ID));
        if let Some(selected) = selected {
            data.insert_temp(egui::Id::new(ARMY_PANEL_ID), selected);
        } else {
            data.remove::<ArmyPanelSelection>(egui::Id::new(ARMY_PANEL_ID));
        }
    });
    sync_map_selection(ctx);
}

/// Retain the order origin without rendering a card while another inspector is in use.
pub(super) fn collapse_army_panel(ctx: &egui::Context) {
    ctx.data_mut(|data| data.insert_temp(egui::Id::new(ARMY_COLLAPSED_ID), true));
}

pub(super) fn army_details_open(ctx: &egui::Context) -> bool {
    selected_army(ctx).is_some()
        && !ctx
            .data(|data| data.get_temp::<bool>(egui::Id::new(ARMY_COLLAPSED_ID)).unwrap_or(false))
        && !orders::is_open(ctx)
}

pub(super) fn cancel_orders(ctx: &egui::Context) {
    orders::clear(ctx);
}

fn sync_map_selection(ctx: &egui::Context) {
    let selection = selected_army(ctx)
        .filter(|army| army.panel == ArmyPanel::Stationary)
        .map(|army| (army.province, orders::destination(ctx)));
    crate::map::set_army_order_selection(ctx, selection);
}

/// Escape closes the independent army window before the game menu is opened.
pub(in crate::app) fn dismiss_army_panel(ctx: &egui::Context) -> bool {
    if orders::dismiss(ctx) || battle_panel::dismiss(ctx) {
        return true;
    }
    if selected_army(ctx).is_none() {
        return false;
    }
    set_selected_army(ctx, None);
    true
}

/// Open an owned army, including a specific marching order, from non-map navigation.
pub(in crate::app) fn open_army_panel(
    ctx: &egui::Context,
    province: usize,
    player: usize,
    movement: Option<u64>,
) {
    battle_panel::dismiss(ctx);
    set_selected_army(
        ctx,
        Some(ArmyPanelSelection {
            province,
            player,
            owner: ForceOwner::Player(player),
            panel: movement.map_or(ArmyPanel::Stationary, ArmyPanel::Movement),
        }),
    );
}

/// Reopen hidden details; a second click on the expanded army closes its window.
pub(in crate::app) fn toggle_map_army_panel(
    ctx: &egui::Context,
    province: usize,
    player: usize,
    movement: Option<u64>,
) {
    battle_panel::dismiss(ctx);
    toggle_army_panel(
        ctx,
        province,
        player,
        movement.map_or(ArmyPanel::Stationary, ArmyPanel::Movement),
    );
}

fn toggle_army_panel(ctx: &egui::Context, province: usize, player: usize, panel: ArmyPanel) {
    toggle_force_panel(ctx, province, player, ForceOwner::Player(player), panel);
}

/// Spectators inspect the actual force, including another player's army or local defenders.
pub(in crate::app) fn toggle_inspection_army_panel(
    ctx: &egui::Context,
    province: usize,
    player: usize,
    owner: ForceOwner,
    movement: Option<u64>,
) {
    battle_panel::dismiss(ctx);
    toggle_force_panel(
        ctx,
        province,
        player,
        owner,
        movement.map_or(ArmyPanel::Stationary, ArmyPanel::Movement),
    );
}

fn toggle_force_panel(
    ctx: &egui::Context,
    province: usize,
    player: usize,
    owner: ForceOwner,
    panel: ArmyPanel,
) {
    let selected = ArmyPanelSelection {
        province,
        player,
        owner,
        panel,
    };
    let close = selected_army(ctx) == Some(selected) && army_details_open(ctx);
    set_selected_army(ctx, (!close).then_some(selected));
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
        let training =
            self.units.iter().map(|unit| unit.training * unit.current_manpower).sum::<f64>()
                / manpower.max(0.001);
        (army_morale(&self.units), training)
    }
}

/// The caller supplies the current, visibility-filtered military view without dated reports.
fn army_overview_rows(world: &MilitaryWorld, player: usize) -> Vec<ArmyOverviewRow> {
    army_overview_rows_for_owner(world, ForceOwner::Player(player))
}

fn army_overview_rows_for_owner(world: &MilitaryWorld, own: ForceOwner) -> Vec<ArmyOverviewRow> {
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
fn rank_ladder(
    ui: &mut egui::Ui,
    world: &MilitaryWorld,
    player: usize,
    influence: f64,
    scale: f32,
    promotion: &mut Option<MilitaryRank>,
) {
    let owner = ForceOwner::Player(player);
    let current = world.rank(owner);
    let peak =
        world.peak_manpower.get(&owner).copied().unwrap_or(0.0).max(world.total_manpower(owner));
    let wins = world.victories.get(&owner).copied().unwrap_or(0);
    ui.columns(4, |columns| {
        for (ui, rank) in columns.iter_mut().zip([
            MilitaryRank::Centurion,
            MilitaryRank::MilitaryTribune,
            MilitaryRank::Legate,
            MilitaryRank::Imperator,
        ]) {
            let index = rank as usize;
            let requirements = rank.promotion_requirements();
            let available = world.promotion_eligibility(owner, rank, influence).is_ok();
            let emphasized = available || rank == current;
            let (rect, response) = ui.allocate_exact_size(
                egui::vec2(ui.available_width(), 78. * scale),
                if available {
                    egui::Sense::click()
                } else {
                    egui::Sense::hover()
                },
            );
            let response = response.on_hover_cursor(if available {
                egui::CursorIcon::PointingHand
            } else {
                egui::CursorIcon::Default
            });
            if rank == current {
                ui.painter().rect_filled(rect, 3. * scale, egui::Color32::from_rgb(218, 198, 162));
            } else {
                paint_purchase_background(
                    ui,
                    rect,
                    available,
                    response.hovered(),
                    response.is_pointer_button_down_on(),
                    3. * scale,
                    scale,
                );
            }
            let center = rect.center().x;
            super::campaign_widgets::paint_raster_icon(
                ui,
                Icon::MilitaryRank(rank),
                egui::Rect::from_center_size(
                    egui::pos2(center, rect.top() + 27. * scale),
                    egui::vec2(38. * scale, 38. * scale),
                ),
                egui::Color32::from_white_alpha(if emphasized {
                    255
                } else {
                    217
                }),
            );
            ui.painter().text(
                egui::pos2(center, rect.top() + 57. * scale),
                egui::Align2::CENTER_CENTER,
                rank.name(),
                egui::FontId::proportional(14. * scale),
                if emphasized {
                    super::province_panel::INK
                } else {
                    super::campaign_widgets::UNAVAILABLE_PURCHASE_INK
                },
            );
            if available && response.clicked() {
                *promotion = Some(rank);
            }
            response.inspection_hover_ui(|ui| {
                let mut rows = Vec::new();
                if let Some(req) = requirements {
                    rows.extend([
                        (
                            current == req.previous || index < current as usize,
                            format!("Previous rank: {}", req.previous.name()),
                        ),
                        (
                            peak >= req.peak_manpower,
                            format!(
                                "Army manpower: {}/{:.0}",
                                peak.round() as u64,
                                req.peak_manpower
                            ),
                        ),
                        (wins >= req.victories, format!("Battles won: {wins}/{}", req.victories)),
                        (
                            influence >= req.influence,
                            format!(
                                "Influence cost: {}/{:.0}",
                                influence.round() as u64,
                                req.influence
                            ),
                        ),
                    ]);
                }
                super::campaign_widgets::rank_tooltip(
                    ui,
                    scale,
                    &rows,
                    &[format!("Influence: +{:.0} per month", world.config.rank_influence[index])],
                );
            });
        }
    });
}

/// National military career and one province ledger for owned and visiting armies.
pub(in crate::app) fn overview(
    ui: &mut egui::Ui,
    world: &MilitaryWorld,
    economy: &EconomyWorld,
    player: usize,
    influence: f64,
    colors: &[egui::Color32],
    scale: f32,
    promotion: &mut Option<MilitaryRank>,
) -> Option<ProvinceId> {
    let own = ForceOwner::Player(player);
    let mut rows = army_overview_rows(world, player);
    rows.retain(|row| {
        row.province < economy.provinces.len()
            && (row.owner == own
                || (matches!(row.owner, ForceOwner::Player(_))
                    && economy.provinces[row.province].owner == Some(player)))
    });
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
    let rows: Vec<_> = rows.iter().collect();
    let height = (ui.clip_rect().bottom() - ui.next_widget_position().y).max(0.);
    let mut selected_province = None;
    egui::ScrollArea::vertical()
        .id_salt(("military-overview", player))
        .max_height(height)
        .auto_shrink([false, false])
        .show(ui, |ui| {
            ui.set_width((ui.available_width() - ui.spacing().scroll.allocated_width()).max(1.));
            super::campaign_widgets::portrait(
                ui,
                super::campaign_widgets::ProvinceLandscape::Military,
                76.0 * scale,
            );
            super::policy_widgets::section(ui, scale, "MILITARY RANK");
            rank_ladder(ui, world, player, influence, scale, promotion);
            ui.add_space(10. * scale);
            overview_table(
                ui,
                &rows,
                world,
                economy,
                player,
                colors,
                scale,
                &mut selected_province,
            );
            if rows.is_empty() {
                ui.label("You have no armies.");
            }
        });
    selected_province
}

const OVERVIEW_FRACTIONS: [f32; 6] = [0., 0.24, 0.62, 0.75, 0.875, 1.];

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
    align: egui::Align,
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
    let x = if align == egui::Align::Center {
        cell.center().x - galley.size().x * 0.5
    } else {
        cell.left()
    };
    painter.galley(egui::pos2(x, cell.center().y - galley.size().y * 0.5), galley, color);
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
    overview_text(ui, value_cell, value, 12., super::province_panel::INK, egui::Align::Min, scale);
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
    use super::province_panel::{INK, PAPER, RULE};
    let width = ui.available_width().max(1.);
    let (header, _) = ui.allocate_exact_size(egui::vec2(width, 34. * scale), egui::Sense::hover());
    ui.painter().rect_filled(header, 2. * scale, egui::Color32::from_rgb(73, 69, 61));
    for (cell, label) in overview_cells(header, scale)
        .into_iter()
        .zip(["Province", "Units", "Tactic", "Morale", "Training"])
    {
        let align = if label == "Tactic" {
            egui::Align::Center
        } else {
            egui::Align::Min
        };
        overview_text(ui, cell, label, 12., PAPER, align, scale);
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
        paint_directory_row(ui, rect, &response, index, scale);
        let cell = overview_cells(rect, scale);
        let color = match row.owner {
            ForceOwner::Player(id) => colors.get(id).copied().unwrap_or(RULE),
            ForceOwner::Local(_) => super::province_panel::NEUTRAL,
        };
        let mut marker_ui = ui.new_child(
            egui::UiBuilder::new()
                .max_rect(rect.shrink(7. * scale))
                .layout(egui::Layout::left_to_right(egui::Align::Center)),
        );
        let marker = directory_owner_marker(&mut marker_ui, color, rect.height(), scale);
        let province_cell = egui::Rect::from_min_max(
            egui::pos2(marker.right() + 8. * scale, rect.center().y - 17. * scale),
            egui::pos2(cell[0].right(), rect.center().y + 17. * scale),
        );
        let mut province_ui = ui.new_child(
            egui::UiBuilder::new()
                .max_rect(province_cell)
                .layout(egui::Layout::left_to_right(egui::Align::Center)),
        );
        province_ui.set_clip_rect(province_ui.clip_rect().intersect(province_cell));
        let name_width = province_ui.available_width().max(1.);
        directory_province_name(
            &mut province_ui,
            &economy.provinces[row.province].name,
            name_width,
            scale,
        );
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
            let image =
                egui::Rect::from_center_size(cell[2].center(), egui::vec2(26., 26.) * scale);
            paint_tactic(ui, tactic, image);
            ui.interact(image, response.id.with("tactic"), egui::Sense::hover())
                .inspection_hover_text(tactic.name());
        } else {
            overview_text(ui, cell[2], "?", 12., INK, egui::Align::Center, scale);
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
        if response.clicked() {
            if row.owner == ForceOwner::Player(player) {
                open_army_panel(ui.ctx(), row.province, player, row.movement);
            } else {
                *selected_province = Some(row.province);
            }
        }
        ui.add_space(3. * scale);
    }
}

/// Show the province rank and offer recruitment only in owned provinces.
fn military_banner(
    ui: &mut egui::Ui,
    rank: MilitaryRank,
    can_recruit: bool,
    tab: &mut MilitaryTab,
    scale: f32,
) -> egui::Rect {
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
        .inspection_hover_text("Current military rank.");
    paint_army_banner_badge(
        ui,
        rank_rect,
        rank.name(),
        Icon::MilitaryRank(rank),
        egui::Color32::from_black_alpha(180),
        badge_scale,
    );
    if !can_recruit {
        return banner;
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
    banner
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
    let inspection = super::spectator::read_only(ui.ctx());
    let can_recruit = super::spectator::domestic_view(ui.ctx(), economic.owner == Some(player));
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
    let banner = military_banner(ui, rank, can_recruit, &mut tab, scale);
    super::campaign_widgets::space_after_portrait(ui, banner, scale, 0.0);
    ui.ctx().data_mut(|data| data.insert_temp(tab_id, tab));
    let mut action = None;
    if tab == MilitaryTab::Recruitment {
        action = ui
            .scope(|ui| {
                if inspection {
                    ui.visuals_mut().disabled_alpha = 1.0;
                    ui.disable();
                }
                recruitment_view(ui, world, economy, province, player, false)
            })
            .inner;
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
                        own || inspection,
                        !inspection
                            && own
                            && economic.owner == Some(player)
                            && !world.province_in_battle(province),
                        scale,
                    );
                    if clicked {
                        toggle_force_panel(
                            ui.ctx(),
                            province,
                            player,
                            *force_owner,
                            ArmyPanel::Stationary,
                        );
                    }
                    if disband {
                        action = Some(MilitaryUiAction::DisbandArmy);
                    }
                    ui.add_space(6. * scale);
                }
                for movement in world.movements.iter().filter(|movement| {
                    (movement.owner == owner || inspection)
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
                        toggle_force_panel(
                            ui.ctx(),
                            movement.origin,
                            player,
                            movement.owner,
                            ArmyPanel::Movement(movement.id),
                        );
                    }
                    ui.add_space(6. * scale);
                }
            });
    }
    action.filter(|_| !inspection)
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
    if selected.player != player && !super::spectator::read_only(ctx) {
        set_selected_army(ctx, None);
        return None;
    }
    let player = if super::spectator::read_only(ctx) {
        selected.player
    } else {
        player
    };
    let province = selected.province;
    let panel = selected.panel;
    let owner = selected.owner;
    let Some(economic) = economy.provinces.get(province) else {
        set_selected_army(ctx, None);
        return None;
    };
    if orders::is_open(ctx) {
        return None;
    }
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
    if !army_details_open(ctx) {
        return None;
    }
    let scale = super::viewport_ui_scale(ctx.content_rect().size());
    let screen = ctx.content_rect();
    let bottom = ctx
        .data(|data| data.get_temp::<egui::Rect>(egui::Id::new("practice-players-panel-rect")))
        .map_or(screen.bottom() - 8. * scale, |rect| rect.bottom());
    let width = (640. * scale).min(screen.width() - 16. * scale);
    let color = if super::spectator::read_only(ctx) {
        army_owner_color(ctx, owner)
    } else {
        ctx.data(|data| {
            data.get_temp::<egui::Color32>(egui::Id::new("campaign-active-player-color"))
        })
        .unwrap_or(super::PLAYER_COLORS[player % super::PLAYER_COLORS.len()])
    };
    let panel_id = egui::Id::new(ARMY_PANEL_ID);
    let choices: Vec<_> = army_overview_rows_for_owner(world, owner)
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
                    owner,
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
                    let (selected_army, province_clicked) =
                        army_panel_header(ui, &economic.name, color, scale, &mut open, &options);
                    if province_clicked {
                        ctx.data_mut(|data| {
                            data.insert_temp(egui::Id::new(ARMY_PROVINCE_CLICK_ID), province);
                        });
                    }
                    if let Some(index) = selected_army {
                        set_selected_army(ctx, Some(choices[index].0));
                        ctx.request_repaint();
                    }
                    egui::Frame::new().inner_margin(10. * scale).show(ui, |ui| {
                        if super::spectator::read_only(ctx) {
                            ui.visuals_mut().disabled_alpha = 1.0;
                            ui.disable();
                        }
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
    action.filter(|_| !super::spectator::read_only(ctx)).map(|action| (province, action))
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ArmyPanel {
    Stationary,
    Movement(u64),
}

/// Persistent navigation for each stationary or marching army.
fn army_panel_header(
    ui: &mut egui::Ui,
    province_name: &str,
    color: egui::Color32,
    scale: f32,
    open: &mut bool,
    options: &[super::campaign_widgets::TitleSearchOption<'_>],
) -> (Option<usize>, bool) {
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
    let province_icon = egui::Rect::from_min_size(
        rect.min + egui::vec2(9., 7.) * scale,
        egui::vec2(28., 28.) * scale,
    );
    let province_response =
        ui.interact(province_icon, ui.id().with("open-army-province"), egui::Sense::click());
    paint_icon(ui, Icon::Attack, province_icon);
    province_response.widget_info(|| {
        egui::WidgetInfo::labeled(egui::WidgetType::Button, true, "Open province military")
    });
    let province_clicked =
        province_response.on_hover_cursor(egui::CursorIcon::PointingHand).clicked();
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
    (selected, province_clicked)
}

fn army_detail_banner(
    ui: &mut egui::Ui,
    rank: MilitaryRank,
    disband: Result<(), &'static str>,
    merge: Result<(), &'static str>,
) -> Option<MilitaryUiAction> {
    let scale = super::viewport_ui_scale(ui.ctx().content_rect().size());
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
    let badge_scale = scale.min((banner.width() - 32. * scale) / (rank_width + 2. * 112.));
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
    let mut action = None;
    for (index, label, art, availability, tip, value) in [
        (
            0,
            "Merge",
            Icon::Cohorts,
            merge,
            "Combine damaged cohorts of the same type into as many full 1,000-person cohorts as possible. Morale and Training are weighted by transferred manpower.",
            MilitaryUiAction::MergeArmy,
        ),
        (
            1,
            "Disband",
            Icon::Cancel,
            disband,
            "Disband this army. Soldiers will return to their respective population classes.",
            MilitaryUiAction::DisbandArmy,
        ),
    ] {
        let enabled = availability.is_ok() && ui.is_enabled();
        let rect = egui::Rect::from_min_size(
            egui::pos2(
                banner.right() - 8. * scale - (2. - index as f32) * 112. * badge_scale
                    - (1. - index as f32) * 6. * badge_scale,
                bottom - 38. * badge_scale,
            ),
            egui::vec2(112., 38.) * badge_scale,
        );
        let response = ui.interact(
            rect,
            ui.id().with(("army-banner-action", label)),
            if enabled { egui::Sense::click() } else { egui::Sense::hover() },
        );
        let response = if enabled {
            response.on_hover_cursor(egui::CursorIcon::PointingHand)
        } else {
            response
        };
        let response = response.inspection_hover_text(availability.err().unwrap_or(tip));
        response.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, enabled, label));
        if response.clicked() {
            action = Some(value);
        }
        let fill = if !enabled {
            egui::Color32::from_rgb(73, 56, 49)
        } else if response.is_pointer_button_down_on() {
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
                if enabled && response.hovered() {
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
    action
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
    let owner = selected_army(ui.ctx())
        .filter(|army| army.province == province)
        .map_or(ForceOwner::Player(player), |army| army.owner);
    let army = province_force_units(world, province, owner);
    let in_battle = world.province_in_battle(province);
    let mergeable = UnitType::ALL.into_iter().any(|kind| {
        let matching: Vec<_> = army.iter().filter(|unit| unit.unit_type == kind).collect();
        let people: u64 = matching.iter().map(|unit| unit.people()).sum();
        matching.len() > people.div_ceil(1_000) as usize
    });
    let disband = if in_battle {
        Err("Disband is unavailable during battle.")
    } else if economic.owner != Some(player) {
        Err("Disband requires direct control of this province.")
    } else {
        Ok(())
    };
    let merge = if in_battle {
        Err("Merge is unavailable during battle.")
    } else if !mergeable {
        Err(if army.iter().all(|unit| unit.people() >= 1_000) {
            "All cohorts are at full strength."
        } else {
            "No damaged cohorts of the same type can be combined."
        })
    } else {
        Ok(())
    };
    let mut action = army_detail_banner(ui, world.rank(owner), disband, merge);
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
    if plan != previous && !in_battle && action.is_none() {
        action = Some(MilitaryUiAction::SavePlan(plan));
    }
    if in_battle {
        ui.small("Formation and tactics are locked during battle.");
    }
    if let Some(battle) = world.battles.iter().find(|battle| battle.province == province) {
        let attacker = battle.attackers.units.iter().any(|unit| unit.owner == owner);
        let side = if attacker {
            &battle.attackers
        } else {
            &battle.defenders
        };
        let can_retreat = battle.months >= world.config.minimum_retreat_months
            && !side.units.iter().any(|unit| battle.trapped.contains(&unit.owner));
        if ui.add_enabled(can_retreat, egui::Button::new("Retreat"))
            .inspection_hover_text("Available after one combat month when an adjacent province permits stationing. Surrounded troops keep fighting.")
            .clicked() {
            action = Some(MilitaryUiAction::Retreat { battle: battle.id, attacker });
        }
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
    army_detail_banner(
        ui,
        world.rank(owner),
        Err("Disband is unavailable while this army is marching."),
        Err("Merge is unavailable while this army is marching."),
    );
    let mut plan = movement.plan;
    normalize_plan_preferences(&mut plan, &movement.units);
    let previous = plan;
    army_summary(ui, &movement.units, terrain, world);
    edit_plan(ui, &mut plan, id as usize + 10_000, &movement.units, &world.config);
    if let Some(location) = movement.destination().and_then(|id| graph.get(id)) {
        deployment_editor(ui, &plan, &movement.units, location.terrain, &world.config);
    }
    ui.small(format!("Marching to {}", destination.name));
    ui.add(
        egui::ProgressBar::new(crate::map::movement_visual_progress(ui.ctx(), movement))
            .desired_height(12.),
    );
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
    let directly_owned = super::spectator::domestic_view(ui.ctx(), economic.owner == Some(player));
    let occupied =
        economic.occupied || world.province_occupied_by_enemy(province, ForceOwner::Player(player));
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
        let progress = recruitment_completion(
            ui,
            project,
            if occupied {
                0.0
            } else {
                speed
            },
        );
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
            if occupied {
                "Recruitment paused: enemy occupation"
            } else {
                "Recruiting"
            },
            active,
            &queued,
            directly_owned && !occupied,
            true,
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
                    let reason = if occupied {
                        Some("Enemy occupation blocks recruitment")
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
                    response.inspection_hover_ui(|ui| {
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
    let width = (480.0 * scale).min(ui.ctx().content_rect().width() - 24.0);
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
                    .inspection_hover_text(explanation);
                }
            });
            ui.add_space(10.0 * scale);
            ui.add(
                egui::Label::new(egui::RichText::new(unit_description(kind)).size(14.0 * scale))
                    .wrap(),
            );
            if let Some(reason) = reason {
                ui.add_space(10.0 * scale);
                super::campaign_widgets::unavailable_reason(ui, reason, scale);
            }
            ui.add_space(12.0 * scale);
            cohort_combat_stats(ui, kind, definition, "recruitment-combat-stats", None, scale);
            ui.add_space(12.0 * scale);
            cohort_capabilities(ui, kind, config, scale);
            ui.add_space(12.0 * scale);
            tactic_capabilities(ui, kind, config, scale);
        });
    });
}

fn cohort_combat_stats(
    ui: &mut egui::Ui,
    kind: UnitType,
    definition: &UnitDefinition,
    grid_id: &'static str,
    condition: Option<(f64, f64)>,
    scale: f32,
) {
    ui.label(egui::RichText::new("Statistics").strong().size(18.0 * scale));
    ui.add_space(4.0 * scale);
    egui::Grid::new((grid_id, kind))
        .num_columns(3)
        .spacing(egui::vec2(16.0, 4.0) * scale)
        .striped(true)
        .show(ui, |ui| {
            let mut rows = vec![
                (Icon::Offense, "Offense", format!("{:.2}", definition.offense)),
                (Icon::Defense, "Defense", format!("{:.2}", definition.defense)),
                (Icon::Speed, "Speed", format!("{:.1}", definition.movement_speed)),
                (Icon::Maneuver, "Maneuver", definition.maneuver.to_string()),
            ];
            if let Some((training, morale)) = condition {
                rows.push((Icon::MilitaryPower, "Training", format!("{training:.0}%")));
                rows.push((Icon::Morale, "Morale", format!("{morale:.0}%")));
            }
            for (art, label, value) in rows {
                icon(ui, art, 24.0 * scale);
                ui.allocate_ui_with_layout(
                    egui::vec2(140.0, 24.0) * scale,
                    egui::Layout::left_to_right(egui::Align::Center),
                    |ui| {
                        ui.set_min_size(egui::vec2(140.0, 24.0) * scale);
                        ui.label(egui::RichText::new(label).size(14.0 * scale));
                    },
                );
                ui.label(egui::RichText::new(value).strong().size(14.0 * scale));
                ui.end_row();
            }
        });
}

fn cohort_capabilities(ui: &mut egui::Ui, kind: UnitType, config: &MilitaryConfig, scale: f32) {
    ui.label(egui::RichText::new("Cohort capabilities").strong().size(18.0 * scale));
    ui.add_space(4.0 * scale);
    egui::Grid::new(("cohort-capabilities", kind))
        .num_columns(3)
        .spacing(egui::vec2(16.0, 4.0) * scale)
        .striped(true)
        .show(ui, |ui| {
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
                icon(ui, Icon::Unit(opponent), 24.0 * scale);
                ui.allocate_ui_with_layout(
                    egui::vec2(140.0, 24.0) * scale,
                    egui::Layout::left_to_right(egui::Align::Center),
                    |ui| {
                        ui.set_min_size(egui::vec2(140.0, 24.0) * scale);
                        ui.label(egui::RichText::new(opponent.name()).size(14.0 * scale));
                    },
                );
                ui.label(
                    egui::RichText::new(format!("{adjustment:+.0}%"))
                        .strong()
                        .size(14.0 * scale)
                        .color(color),
                );
                ui.end_row();
            }
        });
}

/// Keep individual cohort fit and the army's composition fit on the same scale.
fn tactic_fit_color(fit: f64) -> egui::Color32 {
    if fit >= 0.75 {
        egui::Color32::from_rgb(48, 112, 60)
    } else if fit >= 0.40 {
        egui::Color32::from_rgb(170, 104, 35)
    } else {
        egui::Color32::from_rgb(170, 45, 35)
    }
}

/// Show this unit type's suitability for each tactic as one percentage per row.
fn tactic_capabilities(ui: &mut egui::Ui, kind: UnitType, config: &MilitaryConfig, scale: f32) {
    ui.label(egui::RichText::new("Tactic capabilities").strong().size(18.0 * scale));
    ui.add_space(4.0 * scale);
    egui::Grid::new(("tactic-capabilities", kind))
        .num_columns(3)
        .spacing(egui::vec2(16.0, 4.0) * scale)
        .striped(true)
        .show(ui, |ui| {
            for tactic in CombatTactic::ALL {
                let suitability = config.tactic_fit[kind as usize][tactic as usize];
                let (rect, _) =
                    ui.allocate_exact_size(egui::vec2(24.0, 24.0) * scale, egui::Sense::hover());
                paint_tactic(ui, tactic, rect);
                ui.allocate_ui_with_layout(
                    egui::vec2(140.0, 24.0) * scale,
                    egui::Layout::left_to_right(egui::Align::Center),
                    |ui| {
                        ui.set_min_size(egui::vec2(140.0, 24.0) * scale);
                        ui.label(egui::RichText::new(tactic.name()).size(14.0 * scale));
                    },
                );
                ui.label(
                    egui::RichText::new(format!("{:.0}%", suitability * 100.0))
                        .strong()
                        .size(14.0 * scale)
                        .color(tactic_fit_color(suitability)),
                )
                .inspection_hover_text("Unit suitability for this tactic. Higher fit increases its bonus when it counters an enemy tactic. Even 0% fit does not remove normal attack damage.");
                ui.end_row();
            }
        });
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
    let strength: f64 = units.iter().map(|unit| unit.effective_strength(&world.config)).sum();
    let strength = if strength > 0.0 {
        strength
    } else {
        0.0
    };
    let food: f64 = units.iter().map(|unit| unit.food_demand(&world.config)).sum();
    let columns = 5;
    let gap = 6. * scale;
    let width = (ui.available_width() - (columns - 1) as f32 * gap) / columns as f32;
    let terrain_tip = format!(
        "{terrain:?} terrain: battlefield width is {} cohort slots.",
        world.config.combat_widths[terrain as usize]
    );
    let badges = [
        (
            Icon::MilitaryPower,
            "Army strength",
            format!("{strength:.0}"),
            "Proxy for total strength of this army.",
        ),
        (Icon::Cohorts, "Cohorts", units.len().to_string(), "Total cohorts in this army."),
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
            "Morale average by manpower across this army's cohorts.",
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
    if manpower <= 0.0 {
        return 0.0;
    }
    units
        .iter()
        .map(|unit| unit.morale.clamp(0.0, 100.0) * unit.current_manpower.max(0.0))
        .sum::<f64>()
        / manpower
}

/// A full-width army row with public composition, strength, morale and monthly upkeep.
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
    // Keep the cohort composition clear of the effect badges on the right.
    let composition_width = (ui.available_width() / scale - 121.).max(0.);
    let count_digits =
        composition.iter().map(|(_, count)| count.to_string().len()).max().unwrap_or(1) as f32;
    let wide_cell = (66. + 13. * count_digits).max(82.);
    let compact_cell = (52. + 11. * count_digits).max(68.);
    let single_row = composition.len() as f32 * wide_cell <= composition_width;
    let (cell_width, icon_size, count_size, row_height) = if single_row {
        (wide_cell, 54., 20., 62.)
    } else {
        (compact_cell, 44., 17., 50.)
    };
    let column_stride = cell_width + 8.;
    let columns = (composition_width / column_stride).floor().max(1.) as usize;
    let composition_rows = composition.len().div_ceil(columns);
    let row_columns = composition.len().div_ceil(composition_rows.max(1)).max(1);
    let height = (64. + row_height * composition_rows as f32)
        .max(super::policy_widgets::ROW_HEIGHT + 22.)
        * scale;
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
    paint_icon(ui, Icon::Attack, army_icon);
    painter.galley(
        title_rect.min + egui::vec2(26., 0.) * scale,
        painter.layout_job(job),
        owner_color,
    );
    let strength: f64 = units.iter().map(|unit| unit.effective_strength(config)).sum();
    let strength = if strength > 0.0 {
        strength
    } else {
        0.0
    };
    let morale = army_morale(units);
    let food: f64 = units.iter().map(|unit| unit.food_demand(config)).sum();
    for (index, (symbol, value, tip)) in [
        (Icon::MilitaryPower, format!("{strength:.0}"), "Army strength"),
        (Icon::Morale, format!("{morale:.0}%"), "Army morale"),
        (Icon::Food, format!("{}/mo", format_food_demand(food)), "Food demand"),
    ]
    .into_iter()
    .enumerate()
    {
        let badge = super::policy_widgets::effect_rect(&painter, rect, scale, 3, index);
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
            .inspection_hover_text(tip);
    }
    for (index, (kind, count)) in composition.iter().enumerate() {
        let min = rect.left_bottom()
            + egui::vec2(
                8. + (index % row_columns) as f32 * column_stride,
                -icon_size - 8. - (composition_rows - 1 - index / row_columns) as f32 * row_height,
            ) * scale;
        let image = egui::Rect::from_min_size(min, egui::Vec2::splat(icon_size * scale));
        paint_icon(ui, Icon::Unit(*kind), image);
        painter.text(
            image.right_center() + egui::vec2(4. * scale, 0.),
            egui::Align2::LEFT_CENTER,
            count.to_string(),
            egui::FontId::proportional(count_size * scale),
            INK,
        );
        let unit_rect =
            egui::Rect::from_min_size(image.min, egui::vec2(cell_width - 2., icon_size) * scale);
        ui.interact(unit_rect, response.id.with(("unit", *kind)), egui::Sense::hover())
            .inspection_hover_text(kind.name());
    }
    let disband = if can_disband || super::spectator::read_only(ui.ctx()) {
        let button_rect = egui::Rect::from_min_size(
            egui::pos2(rect.right() - 92. * scale, rect.top() + 8. * scale),
            egui::vec2(84., 23.) * scale,
        );
        let button =
            ui.add_enabled_ui(can_disband, |ui| ui.place(button_rect, egui::Button::new(""))).inner;
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
            egui::WidgetInfo::labeled(egui::WidgetType::Button, can_disband, "Disband army")
        });
        button
            .inspection_hover_text(
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

/// Load the Augustus tactic portraits prepared at build time.
fn tactic_texture(ui: &egui::Ui, tactic: CombatTactic) -> egui::TextureId {
    let key = egui::Id::new(("augustus-tactic-icon", tactic as usize));
    if let Some(texture) = ui.ctx().data(|data| data.get_temp::<egui::TextureHandle>(key)) {
        return texture.id();
    }
    macro_rules! portrait {
        ($name:literal) => {
            include_bytes!(concat!(env!("OUT_DIR"), "/panel-icons/", $name, ".png")) as &[u8]
        };
    }
    let bytes = match tactic {
        CombatTactic::ShockAction => portrait!("tactic-shock-action"),
        CombatTactic::Envelopment => portrait!("tactic-envelopment"),
        CombatTactic::Skirmishing => portrait!("tactic-skirmishing"),
        CombatTactic::Deception => portrait!("tactic-deception"),
        CombatTactic::Bottleneck => portrait!("tactic-bottleneck"),
        CombatTactic::Phalanx => portrait!("tactic-phalanx"),
    };
    let image = image::load_from_memory(bytes).expect("Augustus tactic portrait").to_rgba8();
    let texture = ui.ctx().load_texture(
        format!("augustus-{tactic:?}"),
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

/// All tactic portraits use transparent artwork without a screenshot tile.
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

/// Show both favorable and unfavorable matchups beside the current army suitability.
fn tactic_choice(
    ui: &mut egui::Ui,
    tactic: CombatTactic,
    selected: bool,
    units: &[Unit],
    config: &MilitaryConfig,
) -> egui::Response {
    let scale = super::viewport_ui_scale(ui.ctx().content_rect().size());
    let fit = tactic_effectiveness(units.iter(), tactic, config);
    let (rect, response) =
        ui.allocate_exact_size(egui::vec2(ui.available_width(), 78. * scale), egui::Sense::click());
    let paper = if selected || response.hovered() {
        egui::Color32::from_rgb(226, 210, 180)
    } else {
        super::province_panel::PAPER
    };
    ui.painter().rect_filled(rect, 2. * scale, paper);
    ui.painter().rect_stroke(
        rect,
        2. * scale,
        egui::Stroke::new(scale, super::province_panel::RULE),
        egui::StrokeKind::Inside,
    );
    let at = |x, y| rect.min + egui::vec2(x, y) * scale;
    let paint_symbol = |kind, x, y, size| {
        paint_tactic(
            ui,
            kind,
            egui::Rect::from_min_size(at(x, y), egui::Vec2::splat(size * scale)),
        );
    };
    paint_symbol(tactic, 7., 5., 40.);
    ui.painter().text(
        at(27., 65.),
        egui::Align2::CENTER_CENTER,
        format!("{:.0}%", fit * 100.),
        egui::FontId::proportional(12. * scale),
        tactic_fit_color(fit),
    );
    ui.painter().text(
        at(59., 17.),
        egui::Align2::LEFT_CENTER,
        tactic.name(),
        egui::FontId::proportional(16. * scale),
        super::province_panel::INK,
    );
    for (row, &counter) in config.counters[tactic as usize].iter().enumerate() {
        let y = 31. + row as f32 * 22.;
        paint_symbol(counter, 60., y, 20.);
        ui.painter().text(
            at(85., y + 11.),
            egui::Align2::LEFT_CENTER,
            format!("+{:.1}%", config.tactic_bonus[tactic as usize] * fit * 100.),
            egui::FontId::proportional(13. * scale),
            egui::Color32::from_rgb(74, 117, 58),
        );
    }
    for (row, countered_by) in CombatTactic::ALL
        .into_iter()
        .filter(|candidate| candidate.counters(tactic, config))
        .enumerate()
    {
        let y = 31. + row as f32 * 22.;
        paint_symbol(countered_by, 180., y, 20.);
        ui.painter().text(
            at(205., y + 11.),
            egui::Align2::LEFT_CENTER,
            format!("{:.0}%", (config.countered_multiplier - 1.) * 100.),
            egui::FontId::proportional(13. * scale),
            egui::Color32::from_rgb(154, 57, 47),
        );
    }
    response.widget_info(|| {
        egui::WidgetInfo::labeled(egui::WidgetType::Button, ui.is_enabled(), tactic.name())
    });
    response.on_hover_cursor(egui::CursorIcon::PointingHand)
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
            let popup = egui::Popup::menu(&tactic_button)
                .frame(egui::Frame::popup(ui.style()).inner_margin(4. * scale))
                .show(|ui| {
                    ui.set_width(252. * scale);
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
                response.inspection_hover_text(format!(
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
                    let width = (1..=5)
                        .map(|size| {
                            ui.painter()
                                .layout_no_wrap(
                                    format!("{size} per side"),
                                    egui::TextStyle::Button.resolve(ui.style()),
                                    super::province_panel::INK,
                                )
                                .size()
                                .x
                        })
                        .fold(0., f32::max);
                    ui.set_width(width + 12. * scale);
                    for size in [1, 2, 3, 4, 5] {
                        if ui
                            .add(
                                egui::Button::selectable(plan.flank_size == size, ())
                                    .left_text(format!("{size} per side"))
                                    .min_size(egui::vec2(ui.available_width(), 28. * scale)),
                            )
                            .on_hover_cursor(egui::CursorIcon::PointingHand)
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
            response.inspection_hover_text(
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
            let choices: Vec<_> = UnitType::ALL
                .into_iter()
                .filter(|kind| {
                    !kind.is_support()
                        && units
                            .iter()
                            .any(|unit| unit.unit_type == *kind && unit.current_manpower > 0.)
                })
                .collect();
            let text_width = choices
                .iter()
                .map(|kind| {
                    ui.painter()
                        .layout_no_wrap(
                            kind.name().to_owned(),
                            egui::FontId::proportional(14. * scale),
                            super::province_panel::INK,
                        )
                        .size()
                        .x
                })
                .fold(0., f32::max);
            let scroll_width = if choices.len() as f32 * (32. * scale + ui.spacing().item_spacing.y)
                - ui.spacing().item_spacing.y
                > 260. * scale
            {
                ui.spacing().scroll.allocated_width()
            } else {
                0.
            };
            ui.set_width(text_width + 46. * scale + scroll_width);
            egui::ScrollArea::vertical().max_height(260. * scale).show(ui, |ui| {
                for kind in choices {
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
    (rect, response.on_hover_cursor(egui::CursorIcon::PointingHand))
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
            ui.allocate_exact_size(egui::vec2(row_width, 26. * scale), egui::Sense::hover());
        let group_left = |start: usize| {
            labels.left()
                + start as f32 * (cell + spacing)
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
        for (label, start, count) in [
            (left_label, 0, wing),
            (format!("Frontline · {center}"), wing, center),
            (right_label, wing + center, wing),
        ] {
            let left = group_left(start);
            let right = left + count as f32 * cell + count.saturating_sub(1) as f32 * spacing;
            ui.painter().text(
                egui::pos2((left + right) * 0.5, labels.top() + 7. * scale),
                egui::Align2::CENTER_CENTER,
                label,
                egui::FontId::proportional(10. * scale),
                super::province_panel::INK,
            );
            paint_deployment_brace(ui, left, right, labels.bottom() - 5. * scale, scale);
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

fn paint_deployment_brace(ui: &egui::Ui, left: f32, right: f32, y: f32, scale: f32) {
    let middle = (left + right) * 0.5;
    let curl = 4. * scale;
    let points = vec![
        egui::pos2(left, y - 2. * scale),
        egui::pos2(left, y),
        egui::pos2(left + curl, y + 2. * scale),
        egui::pos2(middle - curl, y + 2. * scale),
        egui::pos2(middle, y + 4. * scale),
        egui::pos2(middle + curl, y + 2. * scale),
        egui::pos2(right - curl, y + 2. * scale),
        egui::pos2(right, y),
        egui::pos2(right, y - 2. * scale),
    ];
    ui.painter().add(egui::Shape::line(
        points,
        egui::Stroke::new(0.8 * scale, super::province_panel::RULE),
    ));
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
        .inspection_hover_text(unit.map_or("Empty position", |unit| unit.unit_type.name()));
}

/// Use the singular label for one surviving cohort.
fn cohort_count_label(count: usize) -> String {
    format!(
        "{count} cohort{}",
        if count == 1 {
            ""
        } else {
            "s"
        }
    )
}

/// Read-only manpower and cohort counts by type below the deployment preview.
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
            let manpower = format_person_count(cohorts.iter().map(|unit| unit.people()).sum());
            let (rect, response) =
                ui.allocate_exact_size(egui::vec2(width, 86. * scale), egui::Sense::hover());
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
                    egui::pos2(rect.center().x, rect.top() + 20. * scale),
                    egui::vec2((width - 6. * scale).min(36. * scale), 36. * scale),
                ),
            );
            ui.painter().text(
                egui::pos2(rect.center().x, rect.top() + 53. * scale),
                egui::Align2::CENTER_CENTER,
                manpower,
                egui::FontId::proportional(12. * scale),
                super::province_panel::INK,
            );
            ui.painter().text(
                egui::pos2(rect.center().x, rect.top() + 71. * scale),
                egui::Align2::CENTER_CENTER,
                cohort_count_label(cohorts.len()),
                egui::FontId::proportional(10. * scale),
                super::province_panel::INK,
            );
            response
                .on_hover_cursor(egui::CursorIcon::Default)
                .inspection_hover_ui(|ui| army_unit_type_hover(ui, kind, &cohorts, config, scale));
        }
    });
}

/// Keep the army's current condition and matchup details beside the unit art.
fn army_unit_type_hover(
    ui: &mut egui::Ui,
    kind: UnitType,
    cohorts: &[&Unit],
    config: &MilitaryConfig,
    scale: f32,
) {
    let definition = config.unit(kind);
    let manpower: f64 = cohorts.iter().map(|unit| unit.current_manpower.max(0.0)).sum();
    let weighted_average = |value: fn(&Unit) -> f64| {
        cohorts
            .iter()
            .map(|unit| value(unit).clamp(0.0, 100.0) * unit.current_manpower.max(0.0))
            .sum::<f64>()
            / manpower.max(0.001)
    };
    let training = weighted_average(|unit| unit.training);
    let morale = weighted_average(|unit| unit.morale);
    let width = (480. * scale).min(ui.ctx().content_rect().width() - 24.);
    ui.set_width(width.max(120.));
    ui.horizontal_top(|ui| {
        icon(ui, Icon::Unit(kind), 112. * scale);
        ui.add_space(8. * scale);
        let width = ui.available_width();
        ui.vertical(|ui| {
            ui.set_width(width);
            ui.add_space(8.0 * scale);
            ui.label(egui::RichText::new(kind.name()).strong().size(20. * scale));
            ui.add_space(8.0 * scale);
            ui.add(
                egui::Label::new(egui::RichText::new(unit_description(kind)).size(14.0 * scale))
                    .wrap(),
            );
            ui.add_space(12.0 * scale);
            cohort_combat_stats(
                ui,
                kind,
                definition,
                "army-unit-combat-stats",
                Some((training, morale)),
                scale,
            );
            ui.add_space(12.0 * scale);
            cohort_capabilities(ui, kind, config, scale);
            ui.add_space(12.0 * scale);
            tactic_capabilities(ui, kind, config, scale);
        });
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
                    response.inspection_hover_text(format!(
                        "{} · {}\n{} manpower ({:.1}% strength)\nMorale {:.1}% · Training {:.0}",
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
                        unit.morale,
                        unit.training,
                    ));
                } else {
                    ui.painter().circle_filled(
                        rect.center(),
                        1.5,
                        egui::Color32::from_rgb(191, 171, 143),
                    );
                    response.inspection_hover_text(if is_support {
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
            ui.small(format!("Reserve {}",formation.reserves.len())).inspection_hover_text("Reserves fill vacancies after casualties or routing; surviving deployed units retain their positions.");
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
        ui.small(format!("{:?} · {} cohorts wide",battle.terrain,config.combat_widths[battle.terrain as usize]));
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
                let manpower: f64 = own.iter().map(|unit| unit.current_manpower.max(0.0)).sum();
                let morale = if manpower > 0.0 {
                    own.iter()
                        .map(|unit| unit.morale.clamp(0.0, 100.0) * unit.current_manpower.max(0.0))
                        .sum::<f64>()
                        / manpower
                } else {
                    0.0
                };
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
            .inspection_hover_text(detail);
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
        egui::CollapsingHeader::new(cohort_count_label(side.units.len()))
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
            let allowed = battle.months >= config.minimum_retreat_months
                && !side.units.iter().any(|unit| battle.trapped.contains(&unit.owner));
            if ui.add_enabled(allowed,egui::Button::new("Retreat")).inspection_hover_text("Available after one full combat month if an adjacent province permits stationing. Surrounded troops keep fighting.").clicked(){*action=Some(MilitaryUiAction::Retreat{battle:battle.id,attacker});}
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
    if let ForceOwner::Player(_) = owner {
        return owner_name(owner);
    }
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
        .inspection_hover_text(unit_definition_tip(unit.unit_type, config));
    let meter_width = 84.;
    let text_width =
        (ui.available_width() - meter_width - reserved_width - ui.spacing().item_spacing.x * 2.)
            .max(40.);
    ui.allocate_ui_with_layout(egui::vec2(text_width,40.),egui::Layout::top_down(egui::Align::Min),|ui| {
        ui.add(egui::Label::new(egui::RichText::new(unit.unit_type.name()).strong()).wrap());
        ui.horizontal_wrapped(|ui| {
            ui.small(cohort_manpower_text(unit)).inspection_hover_text(format!("{:.1}% strength. Cohorts regain manpower and morale each supplied month according to military rank.",unit.manpower_ratio()*100.));
            if let Some(role)=role{ui.small(role);}else{
                stat(ui,Icon::Food,&format!("{:.1}",unit.food_demand(config)),"Monthly Food upkeep. All units share the owner's supply ratio, including movement and combat.");
            }
        });
    });
    ui.allocate_ui_with_layout(egui::vec2(meter_width, 40.), egui::Layout::top_down(egui::Align::Min), |ui| {
        condition_meter(ui, "M", unit.morale, egui::Color32::from_rgb(85, 118, 92),
            &format!("Morale {:.1}/100. Below 20, this cohort leaves combat if an adjacent friendly stationing province exists.", unit.morale));
        if private {
            condition_meter(ui,"T",unit.training,egui::Color32::from_rgb(157,117,61),&format!("Training {:.0}/100\nAttack +{:.1}%; defense +{:.1}%. Supplied troops gain {:.0} Training per month.",unit.training,config.training_attack*unit.training,config.training_defense*unit.training,config.passive_training));
        }
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
    response.inspection_hover_text(tip);
}

/// Group cohort images by type so force size is readable without eleven text labels.
fn unit_composition(ui: &mut egui::Ui, units: &[Unit]) {
    for kind in UnitType::ALL {
        let count = units.iter().filter(|u| u.unit_type == kind).count();
        if count > 0 {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 1.;
                icon(ui, Icon::Unit(kind), 26.).inspection_hover_text(format!(
                    "{} · {}",
                    kind.name(),
                    cohort_count_label(count)
                ));
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
