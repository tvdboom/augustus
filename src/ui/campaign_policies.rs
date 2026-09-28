//! Province policies use the same cards, choices and effect badges as Governance.

use super::campaign_widgets::{paint_icon, Icon};
use super::policy_widgets::{self, INK};
use super::province_panel::PAPER;
use crate::game::economy::*;
use bevy_egui::egui;

fn scale(ui: &egui::Ui) -> f32 {
    let base = ui.style().text_styles[&egui::TextStyle::Body].size / 14.0;
    // Four full Governance-style choices plus the right-hand effect stack.
    base.min(ui.available_width() / 530.0)
}

fn card<T: Copy + PartialEq>(
    ui: &mut egui::Ui,
    title: &str,
    selected: &mut T,
    options: &[(T, &str)],
    effects: &[(Icon, String)],
    descriptions: &[String],
    scale: f32,
) -> bool {
    let rect = policy_widgets::card(ui, scale, title);
    let painter = ui.painter_at(rect);
    let accent = ui
        .ctx()
        .data(|data| data.get_temp::<egui::Color32>(egui::Id::new("campaign-owner-color")))
        .unwrap_or(egui::Color32::from_rgb(155, 77, 52));
    for (index, (symbol, value)) in effects.iter().enumerate() {
        let badge = policy_widgets::effect_rect(&painter, rect, scale, effects.len(), index);
        paint_icon(
            ui,
            *symbol,
            egui::Rect::from_min_size(
                badge.min + egui::vec2(1.0, 1.0) * scale,
                egui::vec2(17.0, 17.0) * scale,
            ),
        );
        painter.text(
            badge.right_center() - egui::vec2(4.0 * scale, 0.0),
            egui::Align2::RIGHT_CENTER,
            value,
            egui::FontId::proportional(11.5 * scale),
            INK,
        );
    }
    let mut changed = false;
    for (index, &(candidate, label)) in options.iter().enumerate() {
        let chip = policy_widgets::choice_rect(rect, scale, index);
        let (choice_changed, response) = policy_widgets::choice(
            ui,
            chip,
            ui.id().with((title, index)),
            title,
            label,
            selected,
            candidate,
            accent,
            scale,
        );
        changed |= choice_changed;
        response.on_hover_text(&descriptions[index]);
    }
    if !ui.is_enabled() {
        painter.rect_filled(rect, 2.0 * scale, PAPER.gamma_multiply(0.55));
    }
    ui.add_space(8.0 * scale);
    changed
}

pub(in crate::app) fn province(
    ui: &mut egui::Ui,
    world: &mut EconomyWorld,
    province: usize,
    player: usize,
) {
    ui.spacing_mut().item_spacing.y = 0.0;
    let config = &world.config;
    let Some(p) = world.provinces.get_mut(province) else {
        return;
    };
    egui::ScrollArea::vertical()
        .id_salt(("province-policies", province))
        .auto_shrink([false, false])
        .max_height(ui.available_height())
        .show(ui, |ui| {
        let scale = scale(ui);
        ui.add_enabled_ui(p.owner == Some(player), |ui| {
        policy_widgets::section(ui, scale, "ECONOMY & WORKS");
        let weights = config.focus_weights[p.policies.focus as usize];
        let descriptions: Vec<_> = config.focus_weights.iter().map(|w| format!("Food / Metal / Stone labor weights: {:.1} / {:.1} / {:.1}. Geographic potential also applies; total labor is unchanged.", w[0], w[1], w[2])).collect();
        card(ui, "Resource Focus", &mut p.policies.focus, &[(ResourceFocus::Balanced, "Balanced"), (ResourceFocus::Food, "Food"), (ResourceFocus::Metal, "Metal"), (ResourceFocus::Stone, "Stone")], &[(Icon::Food, format!("×{:.1}", weights[0])), (Icon::Metal, format!("×{:.1}", weights[1])), (Icon::Stone, format!("×{:.1}", weights[2]))], &descriptions, scale);
        let pace = p.policies.construction as usize;
        let descriptions: Vec<_> = (0..3).map(|i| format!(
            "Construction speed ×{:.2}; diverts {:.0}% of remaining productive labor while a building or wonder is under construction. Wonder-assigned slaves are excluded first. No production penalty without an active project.",
            config.construction_speed[i], config.construction_labor[i] * 100.0,
        )).collect();
        card(ui, "Construction Pace", &mut p.policies.construction,
            &[(ConstructionPace::Slow, "Slow"), (ConstructionPace::Normal, "Normal"), (ConstructionPace::Urgent, "Urgent")],
            &[(Icon::Construction, format!("×{:.2}", config.construction_speed[pace])), (Icon::Population, format!("−{:.0}%", config.construction_labor[pace] * 100.0))],
            &descriptions, scale);

        policy_widgets::section(ui, scale, "POPULATION & WELFARE");
        let migration = p.policies.migration as usize;
        let descriptions: Vec<_> = (0..4).map(|i| format!("Arrival attraction ×{:.2}; departures ×{:.2}. Only unhappy free classes migrate automatically; slaves do not migrate. Hostility blocks routes.", config.migration_in[i], config.migration_out[i])).collect();
        card(ui, "Migration Focus", &mut p.policies.migration, &[(MigrationPolicy::Encourage, "Open"), (MigrationPolicy::Normal, "Normal"), (MigrationPolicy::Discourage, "Limited"), (MigrationPolicy::Closed, "Closed")], &[(Icon::Population, format!("In ×{:.2}", config.migration_in[migration])), (Icon::Change, format!("Out ×{:.2}", config.migration_out[migration]))], &descriptions, scale);
        let civic = p.policies.civic_spending as usize;
        let population = p.total_population();
        let descriptions: Vec<_> = (0..3).map(|i| format!(
            "Budget {:.2} Coin per resident each month ({:.2} Coin for current residents); {:+.1} happiness for every class at full funding. Paid before taxes. Insufficient Coin scales the happiness bonus proportionally across your provinces; bonuses do not accumulate.",
            config.civic_coin_per_resident[i], config.civic_coin_per_resident[i] * population, config.civic_happiness[i],
        )).collect();
        card(ui, "Civic Spending", &mut p.policies.civic_spending,
            &[(CivicSpending::Frugal, "Frugal"), (CivicSpending::Normal, "Normal"), (CivicSpending::Generous, "Generous")],
            &[(Icon::Coin, format!("{:.2}/mo", config.civic_coin_per_resident[civic] * population)), (Icon::Happiness, format!("{:+.1}", config.civic_happiness[civic]))],
            &descriptions, scale);
        let manumission = p.policies.manumission as usize;
        let descriptions: Vec<_> = (0..3).map(|i| format!(
            "Each month {:.3}% of surviving slaves become plebeians (×{:.2} ordinary rate). Total population is unchanged; freed people cannot be promoted again in the same month. Wonder assignments are clamped to the remaining slaves.",
            (config.class_change_rates[0] * config.manumission_multiplier[i]).clamp(0.0, 1.0) * 100.0, config.manumission_multiplier[i],
        )).collect();
        card(ui, "Manumission", &mut p.policies.manumission,
            &[(ManumissionPolicy::Restricted, "Restricted"), (ManumissionPolicy::Normal, "Normal"), (ManumissionPolicy::Encouraged, "Encouraged")],
            &[(Icon::Slaves, format!("×{:.2}", config.manumission_multiplier[manumission])), (Icon::Plebeians, format!("{:.3}%/mo", (config.class_change_rates[0] * config.manumission_multiplier[manumission]).clamp(0.0, 1.0) * 100.0))],
            &descriptions, scale);

        policy_widgets::section(ui, scale, "MILITARY");
        let effort = p.policies.recruitment as usize;
        let descriptions: Vec<_> = (0..3).map(|i| format!(
            "Recruitment progress ×{:.2}; extra {:.2} Coin per recruit per active month; {:+.1} local happiness while recruiting. High effort is paid before recruitment progresses; insufficient Coin scales its extra speed and happiness cost toward Normal. No cost or happiness effect without an active recruitment project.",
            config.recruitment_speed[i], config.recruitment_coin_per_recruit[i], config.recruitment_happiness[i],
        )).collect();
        card(ui, "Recruitment Effort", &mut p.policies.recruitment,
            &[(RecruitmentEffort::Low, "Low"), (RecruitmentEffort::Normal, "Normal"), (RecruitmentEffort::High, "High")],
            &[(Icon::Attack, format!("×{:.2}", config.recruitment_speed[effort])), (Icon::Coin, format!("{:.2}/pop", config.recruitment_coin_per_recruit[effort])), (Icon::Happiness, format!("{:+.1}", config.recruitment_happiness[effort]))],
            &descriptions, scale);
        });
    if p.owner != Some(player) {
        ui.small("Only the direct owner can change province policies.");
    }
    });
}
