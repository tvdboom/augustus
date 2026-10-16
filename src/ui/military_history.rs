//! Completed battles in the national military ledger, backed by saved final states.
use super::super::province_panel::{INK, NEUTRAL, PAPER};
use super::*;

const CLICK_ID: &str = "campaign-battle-history-click";

pub(in crate::app) fn take_battle_click(ctx: &egui::Context) -> Option<u64> {
    ctx.data_mut(|data| data.remove_temp(egui::Id::new(CLICK_ID)))
}

#[cfg(test)]
#[path = "../../tests/unit/military_history.rs"]
mod tests;

fn sides(battle: &BattleOutcome, player: usize) -> Option<(&BattleSide, &BattleSide, bool)> {
    let owner = ForceOwner::Player(player);
    if battle.attackers.initial_manpower.contains_key(&owner) {
        Some((&battle.attackers, &battle.defenders, true))
    } else if battle.defenders.initial_manpower.contains_key(&owner) {
        Some((&battle.defenders, &battle.attackers, false))
    } else {
        None
    }
}

fn result_label(result: BattleResult, attacking: bool) -> &'static str {
    match (result, attacking) {
        (BattleResult::MutualRout, _) => "Mutual rout",
        (BattleResult::AttackerVictory, true) | (BattleResult::DefenderVictory, false) => "Victory",
        _ => "Defeat",
    }
}

fn cells(rect: egui::Rect, scale: f32) -> [egui::Rect; 4] {
    let mut left = rect.left();
    [0.25, 0.27, 0.16, 0.32].map(|fraction| {
        let right = left + rect.width() * fraction;
        let cell = egui::Rect::from_min_max(
            egui::pos2(left + 5. * scale, rect.top()),
            egui::pos2(right - 5. * scale, rect.bottom()),
        );
        left = right;
        cell
    })
}

fn opponent_name(
    ui: &egui::Ui,
    cell: egui::Rect,
    side: &BattleSide,
    economy: &EconomyWorld,
    world: &MilitaryWorld,
    scale: f32,
) {
    let font = egui::FontId::proportional(14. * scale);
    let row_height = ui.fonts_mut(|fonts| fonts.row_height(&font));
    let mut job = egui::text::LayoutJob::default();
    job.wrap.max_width = cell.width().max(1.);
    job.wrap.max_rows = (cell.height() / row_height).floor().max(1.) as usize;
    job.wrap.break_anywhere = job.wrap.max_rows == 1;
    for (index, &owner) in side.initial_manpower.keys().enumerate() {
        if index > 0 {
            job.append(" + ", 0., egui::TextFormat::simple(font.clone(), INK));
        }
        let color = match owner {
            ForceOwner::Player(_) => army_owner_color(ui.ctx(), owner),
            ForceOwner::Local(_) => INK,
        };
        job.append(
            &army_name(owner, economy, world),
            0.,
            egui::TextFormat::simple(font.clone(), color),
        );
    }
    let painter = ui.painter().with_clip_rect(ui.clip_rect().intersect(cell));
    let galley = painter.layout_job(job);
    painter.galley(egui::pos2(cell.left(), cell.center().y - galley.size().y * 0.5), galley, INK);
}

fn loss(ui: &egui::Ui, cell: egui::Rect, label: &str, casualties: f64, scale: f32) {
    let image = egui::Rect::from_center_size(
        egui::pos2(cell.left() + 9. * scale, cell.center().y),
        egui::Vec2::splat(18. * scale),
    );
    paint_icon(ui, Icon::Cancel, image);
    let value = format_person_count((casualties.max(0.) * PEOPLE_PER_POPULATION).round() as u64);
    let text =
        egui::Rect::from_min_max(egui::pos2(image.right() + 4. * scale, cell.top()), cell.max);
    overview_text(ui, text, &format!("{label}: {value}"), 12., INK, egui::Align::Min, scale);
}

pub(super) fn show(
    ui: &mut egui::Ui,
    world: &MilitaryWorld,
    economy: &EconomyWorld,
    player: usize,
    scale: f32,
) {
    let records: Vec<_> = world
        .history
        .iter()
        .rev()
        .filter_map(|battle| {
            Some((battle, economy.provinces.get(battle.province)?, sides(battle, player)?))
        })
        .collect();
    if records.is_empty() {
        ui.label("You have no completed battles.");
        return;
    }
    let width = ui.available_width().max(1.);
    let (header, _) = ui.allocate_exact_size(egui::vec2(width, 34. * scale), egui::Sense::hover());
    ui.painter().rect_filled(header, 2. * scale, egui::Color32::from_rgb(73, 69, 61));
    for (cell, label) in
        cells(header, scale).into_iter().zip(["Province", "Opponent", "Result", "Losses"])
    {
        overview_text(ui, cell, label, 12., PAPER, egui::Align::Min, scale);
    }
    for (index, (battle, province, (own, opponent, attacking))) in records.into_iter().enumerate() {
        let result = result_label(battle.result, attacking);
        let owner = ForceOwner::Player(player);
        let own_survivors: f64 = own
            .units
            .iter()
            .filter(|unit| unit.owner == owner)
            .map(|unit| unit.current_manpower)
            .sum();
        let (rect, _) =
            ui.allocate_exact_size(egui::vec2(width, 64. * scale), egui::Sense::hover());
        let response = ui
            .interact(rect, ui.id().with(("battle-history", battle.id)), egui::Sense::click())
            .on_hover_cursor(egui::CursorIcon::PointingHand);
        response.widget_info(|| {
            egui::WidgetInfo::labeled(
                egui::WidgetType::Button,
                true,
                format!("Battle of {} · {result}", province.name),
            )
        });
        paint_directory_row(ui, rect, &response, index, scale);
        let cell = cells(rect, scale);
        let owner_color = battle
            .territorial_owner
            .map_or(NEUTRAL, |owner| army_owner_color(ui.ctx(), ForceOwner::Player(owner)));
        let mut marker_ui = ui.new_child(
            egui::UiBuilder::new()
                .max_rect(rect.shrink(7. * scale))
                .layout(egui::Layout::left_to_right(egui::Align::Center)),
        );
        let marker = directory_owner_marker(&mut marker_ui, owner_color, rect.height(), scale);
        let province_cell = egui::Rect::from_min_max(
            egui::pos2(marker.right() + 8. * scale, cell[0].top()),
            cell[0].max,
        );
        let color = match result {
            "Victory" => egui::Color32::from_rgb(32, 116, 58),
            "Defeat" => egui::Color32::from_rgb(166, 44, 34),
            _ => INK,
        };
        overview_text(ui, province_cell, &province.name, 14., INK, egui::Align::Min, scale);
        opponent_name(ui, cell[1], opponent, economy, world, scale);
        overview_text(ui, cell[2], result, 12., color, egui::Align::Min, scale);
        for (index, label, casualties) in [
            (0, "You", own.initial_manpower[&owner] - own_survivors),
            (1, "Opponent", opponent.initial_manpower.values().sum::<f64>() - opponent.manpower()),
        ] {
            let line = egui::Rect::from_min_size(
                egui::pos2(
                    cell[3].left(),
                    cell[3].center().y - 22. * scale + index as f32 * 24. * scale,
                ),
                egui::vec2(cell[3].width(), 20. * scale),
            );
            loss(ui, line, label, casualties, scale);
        }
        if response.clicked() {
            ui.ctx().data_mut(|data| data.insert_temp(egui::Id::new(CLICK_ID), battle.id));
        }
        ui.add_space(3. * scale);
    }
}
