//! Cursor-anchored province orders with an explicit cohort detachment.
use super::*;

const MENU_ID: &str = "province-army-orders";

#[derive(Clone)]
struct ProvinceOrders {
    army: ArmyPanelSelection,
    destination: usize,
    position: egui::Pos2,
    units: BTreeSet<UnitId>,
}

pub(super) fn clear(ctx: &egui::Context) {
    ctx.data_mut(|data| data.remove::<ProvinceOrders>(egui::Id::new(MENU_ID)));
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
) -> Result<Vec<usize>, MilitaryError> {
    fastest_route(
        graph,
        menu.army.province,
        menu.destination,
        ForceOwner::Player(menu.army.player),
        units,
        |owner, province| {
            if kind == ArmyOrderKind::Attack
                && economy.provinces[menu.destination].owner.is_some()
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
    let click = crate::map::take_province_order_click(ctx);
    let key = egui::Id::new(MENU_ID);
    if let Some(click) = click {
        clear(ctx);
        if let Some(army) = selected_army(ctx).filter(|army| army.player == player) {
            let units = world
                .provinces
                .get(army.province)
                .and_then(|state| state.forces.get(&ForceOwner::Player(player)))
                .map(|units| units.iter().map(|u| u.id).collect())
                .unwrap_or_default();
            ctx.data_mut(|data| {
                data.insert_temp(
                    key,
                    ProvinceOrders {
                        army,
                        destination: click.province,
                        position: click.position,
                        units,
                    },
                )
            });
        }
    }
    let mut menu = ctx.data(|data| data.get_temp::<ProvinceOrders>(key))?;
    if selected_army(ctx) != Some(menu.army) || menu.army.player != player {
        clear(ctx);
        return None;
    }
    let economic = economy.provinces.get(menu.destination)?;
    let own = ForceOwner::Player(player);
    let army =
        world.provinces.get(menu.army.province)?.forces.get(&own).cloned().unwrap_or_default();
    let marching = matches!(menu.army.panel, ArmyPanel::Movement(_));
    menu.units.retain(|id| army.iter().any(|u| u.id == *id));
    let scale = super::super::viewport_ui_scale(ctx.content_rect().size());
    let screen = ctx.content_rect().shrink(8. * scale);
    let width = (320. * scale).min(screen.width());
    // Keep the menu beside the cursor; clamp its full scrollable body at screen edges.
    let position = egui::pos2(
        (menu.position.x + 12. * scale).min(screen.right() - width).max(screen.left()),
        menu.position.y.min(screen.bottom() - 420. * scale).max(screen.top()),
    );
    let mut action = None;
    let mut close = false;
    let response = egui::Area::new(key).order(egui::Order::Foreground)
        .fixed_pos(position).movable(false).show(ctx, |ui| {
            *ui.style_mut() = super::super::campaign_widgets::map_style(scale);
            egui::Frame::new().fill(super::super::province_panel::PAPER)
                .stroke(egui::Stroke::new(1., super::super::province_panel::RULE))
                .corner_radius(5.).inner_margin(10. * scale).show(ui, |ui| {
                ui.set_width((width - 20. * scale).max(1.));
                ui.horizontal(|ui| {
                    ui.strong(&economic.name);
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        close = ui.button("×").clicked();
                    });
                });
                if marching { ui.label("This army is marching. Select a stationed army to detach cohorts."); return; }
                if world.province_in_battle(menu.army.province) { ui.label("This army is committed to battle."); return; }
                if menu.army.province == menu.destination { ui.label("This army is already here."); return; }
                ui.separator();
                ui.horizontal(|ui| {
                    ui.strong("Send cohorts");
                    if ui.small_button("All").clicked() { menu.units = army.iter().map(|u| u.id).collect(); }
                    if ui.small_button("Half").clicked() {
                        menu.units.clear();
                        for kind in UnitType::ALL {
                            let group: Vec<_> = army.iter().filter(|u| u.unit_type == kind).collect();
                            menu.units.extend(group.iter().take(group.len() / 2).map(|u| u.id));
                        }
                        // Split the whole army evenly, including singleton unit types.
                        for kind in UnitType::ALL {
                            if menu.units.len() >= army.len().div_ceil(2) { break; }
                            let group: Vec<_> = army.iter().filter(|u| u.unit_type == kind).collect();
                            if !group.len().is_multiple_of(2) {
                                if let Some(unit) = group.iter().find(|u| !menu.units.contains(&u.id)) {
                                    menu.units.insert(unit.id);
                                }
                            }
                        }
                    }
                    if ui.small_button("None").clicked() { menu.units.clear(); }
                });
                egui::ScrollArea::vertical().id_salt("cohort-detachment").max_height(165. * scale).show(ui, |ui| {
                    for kind in UnitType::ALL {
                        let group: Vec<_> = army.iter().filter(|u| u.unit_type == kind).collect();
                        if group.is_empty() { continue; }
                        let count = group.iter().filter(|u| menu.units.contains(&u.id)).count();
                        ui.horizontal(|ui| {
                            icon(ui, Icon::Unit(kind), 24. * scale);
                            ui.label(kind.name());
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                if ui.add_enabled(count < group.len(), egui::Button::new("+")).clicked() {
                                    if let Some(u) = group.iter().find(|u| !menu.units.contains(&u.id)) { menu.units.insert(u.id); }
                                }
                                ui.label(format!("{count}/{}", group.len()));
                                if ui.add_enabled(count > 0, egui::Button::new("−")).clicked() {
                                    if let Some(u) = group.iter().rev().find(|u| menu.units.contains(&u.id)) { menu.units.remove(&u.id); }
                                }
                            });
                        });
                    }
                    egui::CollapsingHeader::new("Choose individual cohorts").show(ui, |ui| {
                        for unit in &army {
                            let mut send = menu.units.contains(&unit.id);
                            if ui.checkbox(&mut send, format!("{} #{} · {} soldiers · {:.0}% morale", unit.unit_type.abbreviation(), unit.id, unit.people(), unit.morale)).changed() {
                                if send { menu.units.insert(unit.id); } else { menu.units.remove(&unit.id); }
                            }
                        }
                    });
                });
                let units: Vec<_> = army.iter().filter(|u| menu.units.contains(&u.id)).cloned().collect();
                ui.small(format!("{} cohorts / {} soldiers sent · {} cohorts stay", units.len(), units.iter().map(Unit::people).sum::<u64>(), army.len() - units.len()));
                ui.separator();
                let friendly = access(own, menu.destination) == MilitaryAccess::Peaceful;
                let mut choices = Vec::new();
                if economic.owner == Some(player) || friendly {
                    choices.push((ArmyOrderKind::Move, if economic.owner == Some(player) { "Move" } else { "Station / defend" }, "March peacefully and defend this province. Requires stationing permission."));
                }
                if economic.owner != Some(player) {
                    choices.push((ArmyOrderKind::Attack, "Attack", "Declare hostility, march to this province and fight its defenders on arrival."));
                    if pressure_allowed(menu.destination) {
                        choices.push((ArmyOrderKind::Pressure, "Pressure", "Enter without combat. Local defenders remain alive. Build up to 40% Control, at most 2 points each month, reduced by local resistance. Relations fall by 3 each month. Pressure alone cannot conquer the province."));
                    }
                }
                for (kind, label, tip) in choices {
                    let route = order_route(world, economy, graph, &menu, &units, kind, &access);
                    let subtitle = route.as_ref().map(|r| format!("{} months · {} crossings", travel_months(graph, menu.army.province, r, &units, &world.config), r.len()));
                    let text = subtitle.as_ref().map_or_else(|_| label.to_owned(), |eta| format!("{label}   ·   {eta}"));
                    let button = ui.add_enabled(route.is_ok(), egui::Button::new(text).min_size(egui::vec2(ui.available_width(), 30. * scale)))
                        .on_hover_text(tip).on_disabled_hover_text(route.as_ref().err().map(ToString::to_string).unwrap_or_default());
                    if button.clicked() {
                        let route = route.unwrap();
                        let ids = units.iter().map(|u| u.id).collect();
                        let plan = world.provinces[menu.army.province].plans.get(&own).copied().unwrap_or_default();
                        action = Some((menu.army.province, match kind {
                            ArmyOrderKind::Move => MilitaryUiAction::Move { destination: menu.destination, units: ids, route, plan },
                            ArmyOrderKind::Attack => MilitaryUiAction::Attack { destination: menu.destination, units: ids, route, plan },
                            ArmyOrderKind::Pressure => MilitaryUiAction::Pressure { destination: menu.destination, units: ids, route, plan },
                        }));
                    }
                }
                if !notice.is_empty() { ui.colored_label(egui::Color32::DARK_RED, notice); }
            });
        });
    ctx.move_to_top(response.response.layer_id);
    if click.is_none()
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

#[cfg(test)]
#[path = "../../tests/unit/military_orders.rs"]
mod tests;
