//! Province policies use the same cards, choices and effect badges as Governance.

use super::campaign_widgets::{paint_icon, Icon};
use super::policy_widgets::{self, compact_decimal, signed_decimal, INK};
use super::province_panel::PAPER;
use crate::game::economy::*;
use bevy_egui::egui;

fn scale(ui: &egui::Ui) -> f32 {
    let base = ui.style().text_styles[&egui::TextStyle::Body].size / 14.0;
    // Four full Governance-style choices plus the right-hand effect stack.
    base.min(ui.available_width() / 530.0)
}

fn speed_multiplier(value: f64) -> String {
    format!("{value:.2}").trim_end_matches('0').trim_end_matches('.').to_owned()
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
        let value_color = if *symbol == Icon::Happiness && value.starts_with('-') {
            super::resource_hud::hud_delta_color(-1.0)
        } else if *symbol == Icon::Happiness && value.starts_with('+') {
            super::resource_hud::hud_delta_color(1.0)
        } else {
            INK
        };
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
            value_color,
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
        let effects: Vec<_> =
            descriptions[index].split("; ").flat_map(|effect| effect.split(". ")).collect();
        policy_widgets::hover_effects(response, scale, &effects);
    }
    if !ui.is_enabled() && !super::spectator::read_only(ui.ctx()) {
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
        super::campaign_widgets::portrait(
            ui,
            super::campaign_widgets::ProvinceLandscape::Policies,
            76.0 * scale,
        );
        ui.add_space(8.0 * scale);
        ui.add_enabled_ui(p.can_administer(player), |ui| {
        policy_widgets::section(ui, scale, "ECONOMY & WORKS");
        let weights = config.focus_weights[p.policies.focus as usize];
        let descriptions: Vec<_> = config.focus_weights.iter().map(|w| format!("Food labor weight: {}; Metal labor weight: {}; Stone labor weight: {}", compact_decimal(w[0]), compact_decimal(w[1]), compact_decimal(w[2]))).collect();
        card(ui, "Resource Focus", &mut p.policies.focus, &[(ResourceFocus::Balanced, "Balanced"), (ResourceFocus::Food, "Food"), (ResourceFocus::Metal, "Metal"), (ResourceFocus::Stone, "Stone")], &[(Icon::Food, format!("×{}", compact_decimal(weights[0]))), (Icon::Metal, format!("×{}", compact_decimal(weights[1]))), (Icon::Stone, format!("×{}", compact_decimal(weights[2])))], &descriptions, scale);
        let pace = p.policies.construction as usize;
        let descriptions: Vec<_> = (0..3).map(|i| format!(
            "Construction speed ×{}; Diverts {}% of slave labor while a building or wonder is under construction",
            speed_multiplier(config.construction_speed[i]), compact_decimal(config.construction_labor[i] * 100.0),
        )).collect();
        card(ui, "Construction Pace", &mut p.policies.construction,
            &[(ConstructionPace::Slow, "Slow"), (ConstructionPace::Normal, "Normal"), (ConstructionPace::Urgent, "Urgent")],
            &[(Icon::Construction, format!("×{}", speed_multiplier(config.construction_speed[pace]))), (Icon::Slaves, format!("−{}%", compact_decimal(config.construction_labor[pace] * 100.0)))],
            &descriptions, scale);

        policy_widgets::section(ui, scale, "POPULATION & WELFARE");
        let migration = p.policies.migration as usize;
        let descriptions: Vec<_> = (0..4).map(|i| format!("Arrival attraction ×{}; Departures ×{}", compact_decimal(config.migration_in[i]), compact_decimal(config.migration_out[i]))).collect();
        card(ui, "Migration Focus", &mut p.policies.migration, &[(MigrationPolicy::Encourage, "Open"), (MigrationPolicy::Normal, "Normal"), (MigrationPolicy::Discourage, "Limited"), (MigrationPolicy::Closed, "Closed")], &[(Icon::Population, format!("In ×{}", compact_decimal(config.migration_in[migration]))), (Icon::Change, format!("Out ×{}", compact_decimal(config.migration_out[migration]))), (Icon::Happiness, signed_decimal(config.migration_happiness[migration]))], &descriptions, scale);
        let civic = p.policies.civic_spending as usize;
        let civic_budget = p.civic_spending_cost(config);
        let descriptions: Vec<_> = (0..3).map(|i| format!(
            "Budget {} sestertii per free resident; Happiness {} for free classes",
            policy_widgets::coin_decimal(config.civic_coin_per_free_resident[i]), signed_decimal(config.civic_happiness[i]),
        )).collect();
        card(ui, "Civic Spending", &mut p.policies.civic_spending,
            &[(CivicSpending::Frugal, "Frugal"), (CivicSpending::Normal, "Normal"), (CivicSpending::Generous, "Generous")],
            &[(Icon::Coin, compact_decimal(civic_budget)), (Icon::Happiness, signed_decimal(config.civic_happiness[civic]))],
            &descriptions, scale);
        let manumission = p.policies.manumission as usize;
        let descriptions = vec![
            format!("Plebeians becoming slaves: {}%; Plebeian happiness: {}", compact_decimal(config.manumission_rates[0].abs() * 100.0), signed_decimal(config.manumission_happiness[0][2])),
            "Class conversion: 0%; Happiness: 0".to_owned(),
            format!("Slaves becoming plebeians: {}%; Noble happiness: {}", compact_decimal(config.manumission_rates[2] * 100.0), signed_decimal(config.manumission_happiness[2][0])),
        ];
        let (conversion_icon, happiness) = match p.policies.manumission {
            ManumissionPolicy::Enslave => (Icon::Slaves, config.manumission_happiness[manumission][2]),
            ManumissionPolicy::Normal => (Icon::Change, 0.0),
            ManumissionPolicy::Free => (Icon::Plebeians, config.manumission_happiness[manumission][0]),
        };
        card(ui, "Manumission", &mut p.policies.manumission,
            &[(ManumissionPolicy::Enslave, "Enslave"), (ManumissionPolicy::Normal, "Normal"), (ManumissionPolicy::Free, "Free")],
            &[(conversion_icon, format!("{}%", compact_decimal(config.manumission_rates[manumission].abs() * 100.0))), (Icon::Happiness, signed_decimal(happiness))],
            &descriptions, scale);

        policy_widgets::section(ui, scale, "MILITARY");
        let effort = p.policies.recruitment as usize;
        let monthly_cost = |level: usize| {
            policy_widgets::coin_decimal(config.recruitment_coin_per_project[level])
        };
        let descriptions: Vec<_> = (0..3).map(|i| format!(
            "Recruitment speed ×{}; Recruitment cost: {} sestertii per month per active cohort; Citizen and plebeian happiness: {}",
            speed_multiplier(config.recruitment_speed[i]), monthly_cost(i), signed_decimal(config.recruitment_happiness[i]),
        )).collect();
        card(ui, "Recruitment Effort", &mut p.policies.recruitment,
            &[(RecruitmentEffort::Low, "Low"), (RecruitmentEffort::Normal, "Normal"), (RecruitmentEffort::High, "High")],
            &[(Icon::Attack, format!("×{}", speed_multiplier(config.recruitment_speed[effort]))), (Icon::Coin, monthly_cost(effort)), (Icon::Happiness, signed_decimal(config.recruitment_happiness[effort]))],
            &descriptions, scale);
        });
    if p.occupied {
        ui.small("Enemy occupation blocks province policies. Defeat the occupying army to regain access.");
    } else if p.owner != Some(player) {
        ui.small("Only the direct owner can change province policies.");
    }
    });
}
