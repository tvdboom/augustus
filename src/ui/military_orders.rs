//! Cursor-anchored province orders with an explicit cohort detachment.
use super::super::campaign_widgets::{portrait, ProvinceLandscape};
use super::super::province_panel::{INK, PAPER, RULE, TABLE_STRIPE};
use super::*;

const MENU_ID: &str = "province-army-orders";
const BURGUNDY: egui::Color32 = egui::Color32::from_rgb(100, 48, 38);
const GOLD: egui::Color32 = egui::Color32::from_rgb(213, 179, 119);
const MUTED: egui::Color32 = egui::Color32::from_rgb(112, 91, 71);

#[derive(Clone)]
struct ProvinceOrders {
    army: ArmyPanelSelection,
    destination: usize,
    position: egui::Pos2,
    units: BTreeSet<UnitId>,
}

pub(super) fn clear(ctx: &egui::Context) {
    ctx.data_mut(|data| {
        let key = egui::Id::new(MENU_ID);
        data.remove::<ProvinceOrders>(key);
        data.remove::<egui::Vec2>(key.with("size"));
    });
    sync_map_selection(ctx);
}

pub(super) fn is_open(ctx: &egui::Context) -> bool {
    ctx.data(|data| data.get_temp::<ProvinceOrders>(egui::Id::new(MENU_ID)).is_some())
}

pub(super) fn destination(ctx: &egui::Context) -> Option<usize> {
    ctx.data(|data| data.get_temp::<ProvinceOrders>(egui::Id::new(MENU_ID)))
        .map(|menu| menu.destination)
}

/// Consume map intent before either inspector paints, freeing the map in the same pass.
pub(in crate::app) fn prepare_orders(
    ctx: &egui::Context,
    world: &MilitaryWorld,
    economy: &EconomyWorld,
    player: usize,
    province: Option<usize>,
) -> bool {
    let Some(click) = crate::map::take_province_order_click(ctx) else {
        return false;
    };
    if super::super::spectator::read_only(ctx) {
        clear(ctx);
        return false;
    }
    let army = province
        .map(|province| ArmyPanelSelection {
            province,
            player,
            owner: ForceOwner::Player(player),
            panel: ArmyPanel::Stationary,
        })
        .or_else(|| selected_army(ctx).filter(|army| army.player == player));
    let Some(army) = army else {
        return false;
    };
    if economy.provinces.get(army.province).is_none()
        || world.province_in_battle(army.province)
        || matches!(army.panel, ArmyPanel::Movement(_))
        || click.province >= world.provinces.len()
        || click.province >= economy.provinces.len()
    {
        return false;
    }
    let Some(units) = world
        .provinces
        .get(army.province)
        .and_then(|state| state.forces.get(&ForceOwner::Player(player)))
        .filter(|units| !units.is_empty())
    else {
        return false;
    };
    let units = units.iter().map(|u| u.id).collect();
    battle_panel::dismiss(ctx);
    set_selected_army(ctx, Some(army));
    ctx.data_mut(|data| {
        data.insert_temp(
            egui::Id::new(MENU_ID),
            ProvinceOrders {
                army,
                destination: click.province,
                position: click.position,
                units,
            },
        )
    });
    collapse_army_panel(ctx);
    sync_map_selection(ctx);
    true
}

pub(super) fn dismiss(ctx: &egui::Context) -> bool {
    let open = ctx.data(|data| data.get_temp::<ProvinceOrders>(egui::Id::new(MENU_ID)).is_some());
    clear(ctx);
    open
}

fn order_route(
    world: &MilitaryWorld,
    economy: &EconomyWorld,
    graph: &[MilitaryProvince],
    menu: &ProvinceOrders,
    units: &[Unit],
    kind: ArmyOrderKind,
    access: &impl Fn(ForceOwner, usize) -> MilitaryAccess,
) -> Result<Vec<usize>, String> {
    if economy.provinces[menu.destination].name == "Rome"
        && (kind != ArmyOrderKind::Attack
            || !ROME_APPROACHES.contains(&economy.provinces[menu.army.province].name.as_str()))
    {
        return Err("Attack Rome with an army stationed in Etruria, Latium, or Samnium.".into());
    }
    fastest_route(
        graph,
        menu.army.province,
        menu.destination,
        ForceOwner::Player(menu.army.player),
        units,
        |owner, province| {
            if province != menu.destination && economy.provinces[province].name == "Rome" {
                return MilitaryAccess::Blocked;
            }
            if kind == ArmyOrderKind::Attack
                && economy.provinces[menu.destination].owner.is_some()
                && economy.provinces[menu.destination].owner != Some(menu.army.player)
                && economy.provinces[province].owner == economy.provinces[menu.destination].owner
            {
                return MilitaryAccess::Invasion;
            }
            if province == menu.destination {
                match kind {
                    ArmyOrderKind::Attack => return MilitaryAccess::Invasion,
                    ArmyOrderKind::Pressure => return MilitaryAccess::Peaceful,
                    ArmyOrderKind::Move => {},
                }
            }
            if province != menu.destination && world.province_in_battle(province) {
                MilitaryAccess::Blocked
            } else {
                access(owner, province)
            }
        },
        &world.config,
    )
    .map_err(|error| error.to_string())
}

fn travel_months(
    graph: &[MilitaryProvince],
    origin: usize,
    route: &[usize],
    units: &[Unit],
    config: &MilitaryConfig,
) -> u32 {
    let median = median_province_area(graph);
    let speed = force_speed(units, config);
    let mut previous = origin;
    route
        .iter()
        .map(|&next| {
            let months = edge_travel_months(&graph[previous], &graph[next], median, speed, config)
                .max(1.)
                .ceil() as u32;
            previous = next;
            months
        })
        .sum()
}

pub(in crate::app) fn draw_orders_menu(
    ctx: &egui::Context,
    world: &MilitaryWorld,
    economy: &EconomyWorld,
    graph: &[MilitaryProvince],
    player: usize,
    access: impl Fn(ForceOwner, usize) -> MilitaryAccess,
    pressure_allowed: impl Fn(usize) -> bool,
    notice: &str,
) -> Option<(usize, MilitaryUiAction)> {
    let opened = prepare_orders(ctx, world, economy, player, None);
    let key = egui::Id::new(MENU_ID);
    let mut menu = ctx.data(|data| data.get_temp::<ProvinceOrders>(key))?;
    if selected_army(ctx) != Some(menu.army) || menu.army.player != player {
        clear(ctx);
        return None;
    }
    let economic = economy.provinces.get(menu.destination)?;
    let own = ForceOwner::Player(player);
    let army =
        world.provinces.get(menu.army.province)?.forces.get(&own).cloned().unwrap_or_default();
    if army.is_empty()
        || economy.provinces.get(menu.army.province).is_none()
        || world.province_in_battle(menu.army.province)
    {
        clear(ctx);
        return None;
    }
    let marching = matches!(menu.army.panel, ArmyPanel::Movement(_));
    menu.units.retain(|id| army.iter().any(|u| u.id == *id));
    let scale = super::super::viewport_ui_scale(ctx.content_rect().size());
    let screen = ctx.content_rect().shrink(8. * scale);
    let width = (410. * scale).min(screen.width());
    let friendly = access(own, menu.destination) == MilitaryAccess::Peaceful;
    let mut choices = Vec::new();
    if economic.owner == Some(player) || friendly {
        choices.push((
            ArmyOrderKind::Move,
            if economic.owner == Some(player) {
                "Move"
            } else {
                "Station / defend"
            },
            "March & defend",
            Icon::MilitaryAccess,
            "March peacefully and defend this province. Requires stationing permission. Stationed troops build Control without lowering relations. Local defenders attack if relations fall below 50.",
        ));
    }
    if economic.owner != Some(player) || world.province_occupied_by_enemy(menu.destination, own) {
        choices.push((
            ArmyOrderKind::Attack,
            "Attack",
            "Fight on arrival",
            Icon::Attack,
            if economic.owner == Some(player) {
                "March to your occupied province and defeat the occupying army to restore access."
            } else {
                "Declare hostility, march to this province and fight its defenders on arrival."
            },
        ));
        if pressure_allowed(menu.destination) {
            choices.push((ArmyOrderKind::Pressure, "Pressure", "Enter & build Control", Icon::Control, "Enter without declaring war. Build up to 40% Control, at most 2 points each month, reduced by local resistance. Stationing does not lower relations. Local defenders immediately attack if relations fall below 50. Pressure alone cannot conquer the province."));
        }
    }
    let reserved = (252.
        + choices.len() as f32 * 66.
        + if notice.is_empty() {
            0.
        } else {
            60.
        })
        * scale;
    let list_height =
        (screen.height().min(720. * scale) - reserved).clamp(60. * scale, 320. * scale);
    let groups =
        UnitType::ALL.into_iter().filter(|kind| army.iter().any(|u| u.unit_type == *kind)).count();
    let estimate = reserved + ((groups as f32 * 54. + 30.) * scale).min(list_height);
    let height = ctx
        .data(|data| data.get_temp::<egui::Vec2>(key.with("size")))
        .map_or(estimate, |size| size.y)
        .min(screen.height());
    // Prefer the opposite side at the right edge so the destination stays visible.
    let x = if menu.position.x + 12. * scale + width <= screen.right() {
        menu.position.x + 12. * scale
    } else {
        menu.position.x - width - 12. * scale
    };
    let position = egui::pos2(
        x.clamp(screen.left(), (screen.right() - width).max(screen.left())),
        menu.position.y.clamp(screen.top(), (screen.bottom() - height).max(screen.top())),
    );
    let mut action = None;
    let mut close = false;
    let response = egui::Area::new(key)
        .order(egui::Order::Foreground)
        .fixed_pos(position)
        .movable(false)
        .show(ctx, |ui| {
            *ui.style_mut() = super::super::campaign_widgets::map_style(scale);
            ui.spacing_mut().item_spacing.y = 0.;
            egui::Frame::new()
                .fill(PAPER)
                .stroke(egui::Stroke::new(scale, RULE))
                .corner_radius(5. * scale)
                .inner_margin(0.)
                .shadow(egui::epaint::Shadow {
                    offset: [0, 5],
                    blur: 18,
                    spread: 0,
                    color: egui::Color32::from_black_alpha(80),
                })
                .show(ui, |ui| {
                    ui.set_width(width);
                    close = orders_header(
                        ui,
                        &economic.name,
                        &economy.provinces[menu.army.province].name,
                        scale,
                    );
                    egui::Frame::new().inner_margin(14. * scale).show(ui, |ui| {
                        ui.set_width((width - 28. * scale).max(1.));
                        if marching {
                            ui.label(
                                "This army is marching. Select a stationed army to detach cohorts.",
                            );
                            return;
                        }
                        if world.province_in_battle(menu.army.province) {
                            ui.label("This army is committed to battle.");
                            return;
                        }
                        if menu.army.province == menu.destination {
                            ui.label("This army is already here.");
                            return;
                        }
                        ui.horizontal(|ui| {
                            ui.strong("Send cohorts");
                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    ui.spacing_mut().item_spacing.x = 0.;
                                    if preset_button(ui, "None", menu.units.is_empty(), scale)
                                        .clicked()
                                    {
                                        menu.units.clear();
                                    }
                                    if preset_button(
                                        ui,
                                        "Half",
                                        !menu.units.is_empty()
                                            && menu.units.len() == army.len().div_ceil(2)
                                            && menu.units.len() != army.len(),
                                        scale,
                                    )
                                    .clicked()
                                    {
                                        menu.units.clear();
                                        for kind in UnitType::ALL {
                                            let group: Vec<_> = army
                                                .iter()
                                                .filter(|u| u.unit_type == kind)
                                                .collect();
                                            menu.units.extend(
                                                group.iter().take(group.len() / 2).map(|u| u.id),
                                            );
                                        }
                                        // Split the whole army evenly, including singleton unit types.
                                        for kind in UnitType::ALL {
                                            if menu.units.len() >= army.len().div_ceil(2) {
                                                break;
                                            }
                                            let group: Vec<_> = army
                                                .iter()
                                                .filter(|u| u.unit_type == kind)
                                                .collect();
                                            if !group.len().is_multiple_of(2) {
                                                if let Some(unit) = group
                                                    .iter()
                                                    .find(|u| !menu.units.contains(&u.id))
                                                {
                                                    menu.units.insert(unit.id);
                                                }
                                            }
                                        }
                                    }
                                    if preset_button(
                                        ui,
                                        "All",
                                        menu.units.len() == army.len(),
                                        scale,
                                    )
                                    .clicked()
                                    {
                                        menu.units = army.iter().map(|u| u.id).collect();
                                    }
                                },
                            );
                        });
                        ui.add_space(10. * scale);
                        egui::ScrollArea::vertical()
                            .id_salt("cohort-detachment")
                            .max_height(list_height)
                            .auto_shrink([false, true])
                            .show(ui, |ui| {
                                for kind in UnitType::ALL {
                                    let group: Vec<_> =
                                        army.iter().filter(|u| u.unit_type == kind).collect();
                                    if group.is_empty() {
                                        continue;
                                    }
                                    cohort_row(ui, kind, &group, &mut menu.units, scale);
                                    ui.add_space(4. * scale);
                                }
                                ui.add_space(4. * scale);
                                egui::CollapsingHeader::new("Choose individual cohorts").show(
                                    ui,
                                    |ui| {
                                        for unit in &army {
                                            let mut send = menu.units.contains(&unit.id);
                                            if ui
                                                .checkbox(
                                                    &mut send,
                                                    format!(
                                                        "{} #{} · {} soldiers · {:.0}% morale",
                                                        unit.unit_type.abbreviation(),
                                                        unit.id,
                                                        format_person_count(unit.people()),
                                                        unit.morale
                                                    ),
                                                )
                                                .changed()
                                            {
                                                if send {
                                                    menu.units.insert(unit.id);
                                                } else {
                                                    menu.units.remove(&unit.id);
                                                }
                                            }
                                        }
                                    },
                                );
                            });
                        let units: Vec<_> =
                            army.iter().filter(|u| menu.units.contains(&u.id)).cloned().collect();
                        ui.add_space(10. * scale);
                        detachment_summary(ui, &army, &units, scale);
                        ui.add_space(8. * scale);
                        ui.separator();
                        ui.add_space(8. * scale);
                        for (kind, label, description, art, tip) in choices {
                            let route =
                                order_route(world, economy, graph, &menu, &units, kind, &access);
                            let eta = route.as_ref().map(|r| {
                                (
                                    travel_months(
                                        graph,
                                        menu.army.province,
                                        r,
                                        &units,
                                        &world.config,
                                    ),
                                    r.len(),
                                )
                            });
                            let reason = if units.is_empty() {
                                "Choose at least one cohort."
                            } else {
                                route.as_ref().err().map(String::as_str).unwrap_or(tip)
                            };
                            let button = order_button(
                                ui,
                                kind,
                                label,
                                description,
                                art,
                                eta.ok(),
                                reason,
                                scale,
                            );
                            if button.clicked() {
                                let route = route.unwrap();
                                let ids = units.iter().map(|u| u.id).collect();
                                let plan = world.provinces[menu.army.province]
                                    .plans
                                    .get(&own)
                                    .copied()
                                    .unwrap_or_default();
                                action = Some((
                                    menu.army.province,
                                    match kind {
                                        ArmyOrderKind::Move => MilitaryUiAction::Move {
                                            destination: menu.destination,
                                            units: ids,
                                            route,
                                            plan,
                                        },
                                        ArmyOrderKind::Attack => MilitaryUiAction::Attack {
                                            destination: menu.destination,
                                            units: ids,
                                            route,
                                            plan,
                                        },
                                        ArmyOrderKind::Pressure => MilitaryUiAction::Pressure {
                                            destination: menu.destination,
                                            units: ids,
                                            route,
                                            plan,
                                        },
                                    },
                                ));
                            }
                            ui.add_space(6. * scale);
                        }
                        if !notice.is_empty() {
                            ui.colored_label(BURGUNDY, notice);
                        }
                    });
                });
        });
    ctx.data_mut(|data| data.insert_temp(key.with("size"), response.response.rect.size()));
    ctx.move_to_top(response.response.layer_id);
    if !opened
        && ctx.input(|input| input.pointer.primary_clicked())
        && ctx
            .input(|input| input.pointer.interact_pos())
            .is_some_and(|p| !response.response.rect.contains(p))
    {
        close = true;
    }
    if close {
        clear(ctx);
    } else {
        ctx.data_mut(|data| data.insert_temp(key, menu));
    }
    action
}

fn orders_header(ui: &mut egui::Ui, destination: &str, origin: &str, scale: f32) -> bool {
    let banner = portrait(ui, ProvinceLandscape::Army, 92. * scale);
    let painter = ui.painter_at(banner);
    painter.rect_filled(banner, 0., egui::Color32::from_black_alpha(150));
    let left = banner.left() + 16. * scale;
    painter.text(
        egui::pos2(left, banner.top() + 16. * scale),
        egui::Align2::LEFT_CENTER,
        "SEND UNITS",
        egui::FontId::proportional(11. * scale),
        GOLD,
    );
    header_text(
        ui,
        destination,
        egui::pos2(left, banner.top() + 42. * scale),
        banner.width() - 66. * scale,
        22. * scale,
        PAPER,
    );
    header_text(
        ui,
        &format!("From {origin}"),
        egui::pos2(left, banner.top() + 71. * scale),
        banner.width() - 32. * scale,
        12. * scale,
        PAPER,
    );
    let rect = egui::Rect::from_min_size(
        banner.right_top() + egui::vec2(-38., 10.) * scale,
        egui::Vec2::splat(28. * scale),
    );
    let response = ui.interact(rect, ui.id().with("close-orders"), egui::Sense::click());
    painter.rect_filled(
        rect,
        3. * scale,
        egui::Color32::from_white_alpha(if response.hovered() {
            45
        } else {
            18
        }),
    );
    for sign in [-1., 1.] {
        painter.line_segment(
            [
                rect.center() + egui::vec2(-4., -4. * sign) * scale,
                rect.center() + egui::vec2(4., 4. * sign) * scale,
            ],
            egui::Stroke::new(1.5 * scale, PAPER),
        );
    }
    response
        .widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, "Cancel orders"));
    response
        .on_hover_cursor(egui::CursorIcon::PointingHand)
        .on_hover_text("Cancel orders · retain army selection")
        .clicked()
}

/// Elide unusually long province names instead of colliding with controls.
fn header_text(
    ui: &egui::Ui,
    text: &str,
    position: egui::Pos2,
    width: f32,
    size: f32,
    color: egui::Color32,
) {
    let mut job = egui::text::LayoutJob::simple(
        text.to_owned(),
        egui::FontId::proportional(size),
        color,
        width.max(1.),
    );
    job.wrap.max_rows = 1;
    job.wrap.break_anywhere = true;
    let galley = ui.painter().layout_job(job);
    ui.painter().galley(position - egui::vec2(0., galley.size().y * 0.5), galley, color);
}

fn preset_button(ui: &mut egui::Ui, label: &str, selected: bool, scale: f32) -> egui::Response {
    let button =
        egui::Button::new(egui::RichText::new(label).size(12. * scale).color(if selected {
            PAPER
        } else {
            INK
        }))
        .fill(if selected {
            BURGUNDY
        } else {
            TABLE_STRIPE
        })
        .stroke(egui::Stroke::new(scale, RULE))
        .corner_radius(0.);
    ui.add_sized(egui::vec2(48., 28.) * scale, button)
        .on_hover_cursor(egui::CursorIcon::PointingHand)
}

fn cohort_row(
    ui: &mut egui::Ui,
    kind: UnitType,
    group: &[&Unit],
    selected: &mut BTreeSet<UnitId>,
    scale: f32,
) {
    let (rect, _) =
        ui.allocate_exact_size(egui::vec2(ui.available_width(), 50. * scale), egui::Sense::hover());
    ui.painter().rect_filled(rect, 3. * scale, TABLE_STRIPE);
    let control = egui::Rect::from_center_size(
        egui::pos2(rect.right() - 60. * scale, rect.center().y),
        egui::vec2(108., 30.) * scale,
    );
    let count = group.iter().filter(|u| selected.contains(&u.id)).count();
    for (plus, enabled) in [(false, count > 0), (true, count < group.len())] {
        let center = egui::pos2(
            if plus {
                control.right() - 15. * scale
            } else {
                control.left() + 15. * scale
            },
            control.center().y,
        );
        let button = egui::Rect::from_center_size(center, egui::Vec2::splat(30. * scale));
        let response = ui.interact(
            button,
            ui.id().with(("cohort-step", kind, plus)),
            if enabled {
                egui::Sense::click()
            } else {
                egui::Sense::hover()
            },
        );
        let fill = if enabled && response.is_pointer_button_down_on() {
            GOLD
        } else if enabled && response.hovered() {
            egui::Color32::from_rgb(226, 210, 180)
        } else {
            PAPER
        };
        ui.painter().rect_filled(button, 3. * scale, fill);
        ui.painter().rect_stroke(
            button,
            3. * scale,
            egui::Stroke::new(scale, RULE),
            egui::StrokeKind::Inside,
        );
        ui.painter().text(
            center,
            egui::Align2::CENTER_CENTER,
            if plus {
                "+"
            } else {
                "−"
            },
            egui::FontId::proportional(18. * scale),
            if enabled {
                INK
            } else {
                RULE
            },
        );
        let label = format!(
            "{} one {} cohort",
            if plus {
                "Add"
            } else {
                "Leave behind"
            },
            kind.name()
        );
        response
            .widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, enabled, &label));
        let response = if enabled {
            response.on_hover_cursor(egui::CursorIcon::PointingHand)
        } else {
            response
        };
        if response.on_hover_text(label).clicked() {
            if plus {
                if let Some(unit) = group.iter().find(|u| !selected.contains(&u.id)) {
                    selected.insert(unit.id);
                }
            } else if let Some(unit) = group.iter().rev().find(|u| selected.contains(&u.id)) {
                selected.remove(&unit.id);
            }
        }
    }
    let count = group.iter().filter(|u| selected.contains(&u.id)).count();
    let people = group.iter().filter(|u| selected.contains(&u.id)).map(|u| u.people()).sum();
    paint_icon(
        ui,
        Icon::Unit(kind),
        egui::Rect::from_center_size(
            egui::pos2(rect.left() + 26. * scale, rect.center().y),
            egui::Vec2::splat(40. * scale),
        ),
    );
    let left = rect.left() + 54. * scale;
    let text_width = control.left() - left - 6. * scale;
    header_text(
        ui,
        kind.name(),
        egui::pos2(left, rect.top() + 16. * scale),
        text_width,
        14. * scale,
        INK,
    );
    header_text(
        ui,
        &format!("{} soldiers", format_person_count(people)),
        egui::pos2(left, rect.top() + 35. * scale),
        text_width,
        11. * scale,
        MUTED,
    );
    ui.painter().text(
        control.center(),
        egui::Align2::CENTER_CENTER,
        format!("{count} / {}", group.len()),
        egui::FontId::proportional(13. * scale),
        INK,
    );
}

fn detachment_summary(ui: &mut egui::Ui, army: &[Unit], units: &[Unit], scale: f32) {
    let people: u64 = units.iter().map(Unit::people).sum();
    let remaining = army.iter().map(Unit::people).sum::<u64>() - people;
    let (rect, _) =
        ui.allocate_exact_size(egui::vec2(ui.available_width(), 62. * scale), egui::Sense::hover());
    ui.painter().rect_filled(rect, 3. * scale, egui::Color32::from_rgb(227, 217, 195));
    for (index, title, count, soldiers, art) in [
        (0, "Marching", units.len(), people, Icon::Cohorts),
        (1, "Staying", army.len() - units.len(), remaining, Icon::Defense),
    ] {
        let left = rect.left() + rect.width() * index as f32 * 0.5 + 10. * scale;
        paint_icon(
            ui,
            art,
            egui::Rect::from_center_size(
                egui::pos2(left + 12. * scale, rect.center().y),
                egui::Vec2::splat(24. * scale),
            ),
        );
        ui.painter().text(
            egui::pos2(left + 32. * scale, rect.top() + 14. * scale),
            egui::Align2::LEFT_CENTER,
            title,
            egui::FontId::proportional(11. * scale),
            MUTED,
        );
        ui.painter().text(
            egui::pos2(left + 32. * scale, rect.top() + 32. * scale),
            egui::Align2::LEFT_CENTER,
            cohort_count_label(count),
            egui::FontId::proportional(14. * scale),
            INK,
        );
        ui.painter().text(
            egui::pos2(left + 32. * scale, rect.top() + 49. * scale),
            egui::Align2::LEFT_CENTER,
            format!("{} soldiers", format_person_count(soldiers)),
            egui::FontId::proportional(11. * scale),
            MUTED,
        );
    }
    ui.painter().line_segment(
        [
            rect.center_top() + egui::vec2(0., 10.) * scale,
            rect.center_bottom() - egui::vec2(0., 10.) * scale,
        ],
        egui::Stroke::new(scale, RULE),
    );
}

fn order_button(
    ui: &mut egui::Ui,
    kind: ArmyOrderKind,
    label: &str,
    description: &str,
    art: Icon,
    eta: Option<(u32, usize)>,
    tip: &str,
    scale: f32,
) -> egui::Response {
    let enabled = eta.is_some();
    let primary = kind == ArmyOrderKind::Attack || (kind == ArmyOrderKind::Move && label == "Move");
    let (rect, response) = ui.allocate_exact_size(
        egui::vec2(ui.available_width(), 60. * scale),
        if enabled {
            egui::Sense::click()
        } else {
            egui::Sense::hover()
        },
    );
    let fill = if !enabled {
        egui::Color32::from_rgb(226, 219, 203)
    } else if primary {
        if response.is_pointer_button_down_on() {
            egui::Color32::from_rgb(78, 36, 30)
        } else if response.hovered() {
            egui::Color32::from_rgb(130, 65, 48)
        } else {
            BURGUNDY
        }
    } else if response.hovered() {
        egui::Color32::from_rgb(226, 210, 180)
    } else {
        TABLE_STRIPE
    };
    let ink = if enabled && primary {
        PAPER
    } else if enabled {
        INK
    } else {
        MUTED
    };
    let secondary = if enabled && primary {
        GOLD
    } else {
        MUTED
    };
    ui.painter().rect_filled(rect, 4. * scale, fill);
    ui.painter().rect_stroke(
        rect,
        4. * scale,
        egui::Stroke::new(
            scale,
            if primary && enabled {
                GOLD
            } else {
                RULE
            },
        ),
        egui::StrokeKind::Inside,
    );
    paint_icon(
        ui,
        art,
        egui::Rect::from_center_size(
            egui::pos2(rect.left() + 28. * scale, rect.center().y),
            egui::Vec2::splat(34. * scale),
        ),
    );
    let left = rect.left() + 54. * scale;
    let width = rect.width() - 166. * scale;
    header_text(ui, label, egui::pos2(left, rect.top() + 21. * scale), width, 15. * scale, ink);
    header_text(
        ui,
        description,
        egui::pos2(left, rect.top() + 42. * scale),
        width,
        11. * scale,
        secondary,
    );
    if let Some((months, crossings)) = eta {
        for (row, art, text) in [
            (
                0,
                Icon::Duration,
                format!(
                    "{months} month{}",
                    if months == 1 {
                        ""
                    } else {
                        "s"
                    }
                ),
            ),
            (
                1,
                Icon::Province,
                format!(
                    "{crossings} crossing{}",
                    if crossings == 1 {
                        ""
                    } else {
                        "s"
                    }
                ),
            ),
        ] {
            let y = rect.top() + (20. + row as f32 * 22.) * scale;
            paint_icon(
                ui,
                art,
                egui::Rect::from_center_size(
                    egui::pos2(rect.right() - 96. * scale, y),
                    egui::Vec2::splat(18. * scale),
                ),
            );
            ui.painter().text(
                egui::pos2(rect.right() - 81. * scale, y),
                egui::Align2::LEFT_CENTER,
                text,
                egui::FontId::proportional(11. * scale),
                ink,
            );
        }
    } else {
        ui.painter().text(
            egui::pos2(rect.right() - 12. * scale, rect.center().y),
            egui::Align2::RIGHT_CENTER,
            "Unavailable",
            egui::FontId::proportional(11. * scale),
            MUTED,
        );
    }
    response.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, enabled, label));
    let response = if enabled {
        response.on_hover_cursor(egui::CursorIcon::PointingHand)
    } else {
        response
    };
    response.on_hover_text(tip)
}

#[cfg(test)]
#[path = "../../tests/unit/military_orders.rs"]
mod tests;
