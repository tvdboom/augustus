//! Centered confirmations share one input-blocking backdrop and deferred command.

use super::campaign_panel::{apply_military_action, CampaignUi};
use super::campaign_widgets::WorkQueueAction;
use super::*;
use crate::game::economy::ConstructionProject;
use crate::game::military::{ForceOwner, RecruitmentProject, UnitId};

#[derive(Clone, Copy)]
pub(super) enum ConfirmationAction {
    CancelConstruction(WorkQueueAction),
    CancelRecruitment,
    CancelQueuedRecruitment(usize),
    Disband(UnitId),
    DisbandArmy,
    Vassalize,
    Integrate,
}

/// Retain the original player and province even if the inspector later changes.
#[derive(Clone)]
pub(super) struct PendingConfirmation {
    province: usize,
    player: usize,
    action: ConfirmationAction,
    order: Option<OrderSnapshot>,
    title: &'static str,
    question: String,
}

/// Keep the selected order distinct from one that takes its slot as months advance.
#[derive(Clone)]
enum OrderSnapshot {
    Construction(ConstructionProject, usize),
    Recruitment(RecruitmentProject, usize),
}

fn same_construction(a: &ConstructionProject, b: &ConstructionProject) -> bool {
    match (a, b) {
        (ConstructionProject::Building(a), ConstructionProject::Building(b)) => {
            a.building == b.building && a.target_level == b.target_level
        },
        (ConstructionProject::Wonder(a), ConstructionProject::Wonder(b)) => {
            a.wonder_id == b.wonder_id
        },
        _ => false,
    }
}

fn same_recruitment(a: &RecruitmentProject, b: &RecruitmentProject) -> bool {
    a.owner == b.owner
        && a.unit_type == b.unit_type
        && a.required_progress == b.required_progress
        && a.population_cost == b.population_cost
        && a.cohort_manpower == b.cohort_manpower
        && a.manpower_class == b.manpower_class
        && a.paid_metal == b.paid_metal
}

impl PendingConfirmation {
    pub(super) fn construction(
        campaign: &campaign::Campaign,
        province: usize,
        player: usize,
        action: WorkQueueAction,
    ) -> Option<Self> {
        Self::new(campaign, province, player, ConfirmationAction::CancelConstruction(action))
    }

    pub(super) fn new(
        campaign: &campaign::Campaign,
        province: usize,
        player: usize,
        action: ConfirmationAction,
    ) -> Option<Self> {
        use ConfirmationAction::*;
        let p = campaign.economy.provinces.get(province)?;
        let (title, description) = match action {
            Vassalize => {
                campaign.politics.get(province)?.clone().vassalize(player).ok()?;
                ("Vassalize province", format!("vassalize {}", p.name))
            },
            Integrate => {
                campaign.politics.get(province)?.clone().take_ownership(player).ok()?;
                ("Integrate province", format!("integrate {}", p.name))
            },
            CancelConstruction(order) => {
                let (project, queued) = match order {
                    WorkQueueAction::CancelActive => (p.construction.as_ref()?, false),
                    WorkQueueAction::CancelQueued(index) => {
                        (p.construction_queue.get(index)?, true)
                    },
                };
                let name = match project {
                    ConstructionProject::Building(project) => project.building.name().to_owned(),
                    ConstructionProject::Wonder(project) => {
                        crate::map::wonder_name(project.wonder_id)?.to_owned()
                    },
                };
                (
                    "Cancel construction",
                    format!(
                        "cancel {}construction of {name} in {}",
                        if queued {
                            "queued "
                        } else {
                            ""
                        },
                        p.name
                    ),
                )
            },
            CancelRecruitment | CancelQueuedRecruitment(_) => {
                let state = campaign.military.provinces.get(province)?;
                let queued = matches!(action, CancelQueuedRecruitment(_));
                let project = match action {
                    CancelQueuedRecruitment(index) => state.recruitment_queue.get(index)?,
                    _ => state.recruitment.as_ref()?,
                };
                (
                    "Cancel recruitment",
                    format!(
                        "cancel {}recruitment of {} in {}",
                        if queued {
                            "queued "
                        } else {
                            ""
                        },
                        project.unit_type.name(),
                        p.name
                    ),
                )
            },
            Disband(id) => {
                let unit = campaign.military.all_units().find(|unit| unit.id == id)?;
                (
                    "Disband unit",
                    format!("disband the {} cohort in {}", unit.unit_type.name(), p.name),
                )
            },
            DisbandArmy => {
                let units = campaign
                    .military
                    .provinces
                    .get(province)?
                    .forces
                    .get(&crate::game::military::ForceOwner::Player(player))?;
                if units.is_empty() {
                    return None;
                }
                ("Disband army", format!("disband your entire army in {}", p.name))
            },
        };
        let order = match action {
            CancelConstruction(position) => {
                let project = match position {
                    WorkQueueAction::CancelActive => p.construction.as_ref()?,
                    WorkQueueAction::CancelQueued(index) => p.construction_queue.get(index)?,
                };
                Some(OrderSnapshot::Construction(project.clone(), p.construction_queue.len()))
            },
            CancelRecruitment | CancelQueuedRecruitment(_) => {
                let state = campaign.military.provinces.get(province)?;
                let project = match action {
                    CancelRecruitment => state.recruitment.as_ref()?,
                    CancelQueuedRecruitment(index) => state.recruitment_queue.get(index)?,
                    _ => unreachable!(),
                };
                Some(OrderSnapshot::Recruitment(project.clone(), state.recruitment_queue.len()))
            },
            _ => None,
        };
        Some(Self {
            province,
            player,
            action,
            order,
            title,
            question: format!("Are you sure you want to {description}?"),
        })
    }

    fn still_applicable(&self, campaign: &campaign::Campaign) -> bool {
        use ConfirmationAction::*;
        let Some(province) = campaign.economy.provinces.get(self.province) else {
            return false;
        };
        if self.player >= campaign.economy.players.len() {
            return false;
        }
        if matches!(self.action, Vassalize | Integrate) {
            return campaign.politics.get(self.province).is_some_and(|politics| {
                if matches!(self.action, Vassalize) {
                    politics.clone().vassalize(self.player).is_ok()
                } else {
                    politics.clone().take_ownership(self.player).is_ok()
                }
            });
        }
        if province.owner != Some(self.player) {
            return false;
        }
        match (&self.action, &self.order) {
            (CancelConstruction(position), Some(OrderSnapshot::Construction(original, count))) => {
                let current = match position {
                    WorkQueueAction::CancelActive => province.construction.as_ref(),
                    WorkQueueAction::CancelQueued(index) => province.construction_queue.get(*index),
                };
                province.construction_queue.len() == *count
                    && current.is_some_and(|current| same_construction(original, current))
            },
            (
                CancelRecruitment | CancelQueuedRecruitment(_),
                Some(OrderSnapshot::Recruitment(original, count)),
            ) => {
                let Some(state) = campaign.military.provinces.get(self.province) else {
                    return false;
                };
                let current = match self.action {
                    CancelRecruitment => state.recruitment.as_ref(),
                    CancelQueuedRecruitment(index) => state.recruitment_queue.get(index),
                    _ => unreachable!(),
                };
                state.recruitment_queue.len() == *count
                    && current.is_some_and(|current| same_recruitment(original, current))
            },
            (Disband(id), _) => {
                !campaign.military.province_in_battle(self.province)
                    && campaign.military.provinces.get(self.province).is_some_and(|state| {
                        state
                            .forces
                            .get(&ForceOwner::Player(self.player))
                            .is_some_and(|units| units.iter().any(|unit| unit.id == *id))
                    })
            },
            (DisbandArmy, _) => {
                !campaign.military.province_in_battle(self.province)
                    && campaign.military.provinces.get(self.province).is_some_and(|state| {
                        state
                            .forces
                            .get(&ForceOwner::Player(self.player))
                            .is_some_and(|units| !units.is_empty())
                    })
            },
            _ => false,
        }
    }

    fn apply(self, ctx: &egui::Context, campaign: &mut campaign::Campaign) -> String {
        if !self.still_applicable(campaign) {
            return "This order has changed. Please select it again.".into();
        }
        use campaign_military::MilitaryUiAction;
        let action = match self.action {
            ConfirmationAction::Vassalize | ConfirmationAction::Integrate => {
                let result = if matches!(self.action, ConfirmationAction::Vassalize) {
                    campaign
                        .politics
                        .get_mut(self.province)
                        .ok_or(crate::game::politics::PoliticalError::MissingTarget)
                        .and_then(|p| p.vassalize(self.player))
                } else if self.province < campaign.politics.len() {
                    super::campaign_panel::integrate_province(campaign, self.province, self.player)
                } else {
                    Err(crate::game::politics::PoliticalError::MissingTarget)
                };
                return result.map_or_else(
                    |error| error.to_string(),
                    |_| {
                        campaign.reconcile_provinces();
                        campaign.espionage.withdraw(self.player, self.province);
                        if matches!(self.action, ConfirmationAction::Vassalize) {
                            "Province vassalized.".into()
                        } else {
                            "Province integrated.".into()
                        }
                    },
                );
            },
            ConfirmationAction::CancelConstruction(action) => {
                let before = campaign_panel::construction_underway(campaign, self.province);
                let message = campaign_economy::cancel_construction_order(
                    ctx,
                    &mut campaign.economy,
                    self.province,
                    self.player,
                    action,
                );
                let after = campaign_panel::construction_underway(campaign, self.province);
                if before != after {
                    if let Some(campaign_widgets::Icon::Wonder(wonder)) = after {
                        campaign.notify_wonder_started(self.player, self.province, wonder);
                    }
                }
                return message;
            },
            ConfirmationAction::CancelRecruitment => MilitaryUiAction::CancelRecruitment,
            ConfirmationAction::CancelQueuedRecruitment(index) => {
                MilitaryUiAction::CancelQueuedRecruitment(index)
            },
            ConfirmationAction::Disband(id) => MilitaryUiAction::Disband(id),
            ConfirmationAction::DisbandArmy => MilitaryUiAction::DisbandArmy,
        };
        apply_military_action(campaign, self.province, self.player, action)
    }
}

/// Draw last so the backdrop covers map panels, notifications and audio controls.
pub(in crate::app) fn draw(
    mut contexts: EguiContexts,
    state: Res<State<AppState>>,
    mut campaign: ResMut<campaign::Campaign>,
    mut view: ResMut<CampaignUi>,
    practice: Res<LocalPractice>,
    sound: Res<MenuAudio>,
    audio: Res<Audio>,
    assets: Res<AssetServer>,
    terminal: Res<TerminalPresentation>,
) {
    if terminal.spectating || *state.get() != AppState::Map || !campaign.active {
        view.dismiss_confirmation();
        return;
    }
    if view.confirmation.as_ref().is_some_and(|pending| pending.player != practice.active_player) {
        view.dismiss_confirmation();
        return;
    }
    let Ok(ctx) = contexts.ctx_mut() else {
        return;
    };
    if view.confirmation.as_ref().is_some_and(|pending| pending.still_applicable(&campaign))
        && view.take_confirmation_sound()
        && sound.mode != AudioMode::Mute
        && sound.volume > 0.001
    {
        let decibels = 20.0 * sound.volume.clamp(0.001, 1.0).log10();
        audio.play(assets.load("audio/message.ogg")).with_volume(decibels);
    }
    if show(ctx, &mut view, &mut campaign) {
        play_click(&sound, &audio, &assets);
    }
}

/// Yes is the only path that dispatches; No, Escape and backdrop clicks discard it.
pub(super) fn show(
    ctx: &egui::Context,
    view: &mut CampaignUi,
    campaign: &mut campaign::Campaign,
) -> bool {
    let Some(pending) = &view.confirmation else {
        return false;
    };
    if !pending.still_applicable(campaign) {
        view.dismiss_confirmation();
        return false;
    }
    let scale = viewport_ui_scale(ctx.content_rect().size());
    let burgundy = egui::Color32::from_rgb(100, 48, 38);
    let gold = egui::Color32::from_rgb(190, 150, 76);
    let mut yes = false;
    let mut no = false;
    let modal = egui::Modal::new(egui::Id::new("augustus_action_confirmation"))
        .frame(
            egui::Frame::new()
                .fill(province_panel::PAPER)
                .stroke(egui::Stroke::new(2.0 * scale, burgundy))
                .corner_radius(6.0 * scale)
                .shadow(egui::epaint::Shadow {
                    offset: [0, (6.0 * scale) as i8],
                    blur: (20.0 * scale) as u8,
                    spread: (2.0 * scale) as u8,
                    color: egui::Color32::from_black_alpha(90),
                })
                .inner_margin(6.0 * scale),
        )
        .show(ctx, |ui| {
            *ui.style_mut() = campaign_widgets::map_style(scale);
            ui.set_width((400.0 * scale).min((ctx.content_rect().width() - 64.0 * scale).max(1.0)));
            ui.spacing_mut().item_spacing.y = 0.0;
            egui::Frame::new()
                .stroke(egui::Stroke::new(scale, gold))
                .corner_radius(3.0 * scale)
                .show(ui, |ui| {
                    egui::Frame::new().fill(burgundy).inner_margin(12.0 * scale).show(ui, |ui| {
                        ui.set_min_width(ui.available_width());
                        ui.vertical_centered(|ui| {
                            ui.label(
                                egui::RichText::new(pending.title)
                                    .size(21.0 * scale)
                                    .strong()
                                    .color(province_panel::PAPER),
                            );
                        });
                    });
                    egui::Frame::new().inner_margin(20.0 * scale).show(ui, |ui| {
                        ui.vertical_centered(|ui| {
                            ui.add(
                                egui::Label::new(&pending.question)
                                    .halign(egui::Align::Center)
                                    .wrap(),
                            );
                            ui.add_space(16.0 * scale);
                            let (rule, _) = ui.allocate_exact_size(
                                egui::vec2(ui.available_width(), 8.0 * scale),
                                egui::Sense::hover(),
                            );
                            let center = rule.center();
                            let gap = 9.0 * scale;
                            for (start, end) in [
                                (rule.left_center(), center - egui::vec2(gap, 0.0)),
                                (center + egui::vec2(gap, 0.0), rule.right_center()),
                            ] {
                                ui.painter().line_segment(
                                    [start, end],
                                    egui::Stroke::new(scale, province_panel::RULE),
                                );
                            }
                            let tip = 4.0 * scale;
                            ui.painter().add(egui::Shape::convex_polygon(
                                vec![
                                    center - egui::vec2(tip, 0.0),
                                    center - egui::vec2(0.0, tip),
                                    center + egui::vec2(tip, 0.0),
                                    center + egui::vec2(0.0, tip),
                                ],
                                gold,
                                egui::Stroke::NONE,
                            ));
                            ui.add_space(16.0 * scale);
                            let button_size = egui::vec2(90.0 * scale, 34.0 * scale);
                            let button_gap = 12.0 * scale;
                            let inset = (ui.available_width() - button_size.x * 2.0 - button_gap)
                                .max(0.0)
                                / 2.0;
                            ui.horizontal(|ui| {
                                ui.spacing_mut().item_spacing.x = button_gap;
                                ui.add_space(inset);
                                yes = ui
                                    .scope(|ui| {
                                        let widgets = &mut ui.visuals_mut().widgets;
                                        for (visuals, fill) in [
                                            (&mut widgets.inactive, burgundy),
                                            (
                                                &mut widgets.hovered,
                                                egui::Color32::from_rgb(130, 65, 48),
                                            ),
                                            (
                                                &mut widgets.active,
                                                egui::Color32::from_rgb(78, 36, 30),
                                            ),
                                        ] {
                                            visuals.bg_fill = fill;
                                            visuals.weak_bg_fill = fill;
                                            visuals.bg_stroke = egui::Stroke::new(scale, gold);
                                        }
                                        ui.add_sized(
                                            button_size,
                                            egui::Button::new(
                                                egui::RichText::new("Yes")
                                                    .color(province_panel::PAPER),
                                            ),
                                        )
                                        .clicked()
                                    })
                                    .inner;
                                no = ui.add_sized(button_size, egui::Button::new("No")).clicked();
                            });
                        });
                    });
                });
        });
    ctx.move_to_top(modal.response.layer_id);
    let dismiss = no
        || modal.should_close()
        || modal.backdrop_response.secondary_clicked()
        || modal.backdrop_response.middle_clicked();
    if yes || dismiss {
        let pending = view.confirmation.take().unwrap();
        if yes && !dismiss {
            view.notice = pending.apply(ctx, campaign);
        }
        return true;
    }
    false
}

#[cfg(test)]
#[path = "../../tests/unit/confirmation_ui.rs"]
mod tests;
