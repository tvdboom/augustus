//! Campaign inspectors using the existing parchment, banner colors, and button treatment.

use super::campaign::Campaign;
use super::campaign_confirmation::ConfirmationAction;
use super::campaign_notifications::{CampaignNotice, NoticeAction};
use super::campaign_widgets::{icon, stat, Icon};
use super::*;
use crate::game::economy::ConstructionProject;
use crate::game::military::*;
use crate::game::politics::diplomacy::*;
use crate::game::politics::espionage::SpyAssignment;
use crate::game::politics::PoliticalError;

/// Global banner destinations and the contextual province inspector.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum CampaignTab {
    Governance,
    Military,
    Trade,
    Senate,
    Province,
}

const PROVINCE_SECTION_COUNT: usize = 6;

/// View state is separate from game rules and never changes simulation outcomes.
#[derive(Resource, Default)]
pub(crate) struct CampaignUi {
    pub open: Option<CampaignTab>,
    pub province: Option<usize>,
    section: usize,
    last_detail: Option<MapDetail>,
    pub(super) notice: String,
    highlight_scandal: Option<u64>,
    province_selector_open: bool,
    province_search: String,
    politics_section: usize,
    politics_province_search: String,
    politics_spy_search: String,
    pub(super) confirmation: Option<campaign_confirmation::PendingConfirmation>,
}

impl CampaignUi {
    /// The mouse's forward/back side buttons cycle the open province's tabs.
    fn cycle_province_tabs(&mut self, ctx: &egui::Context) {
        if self.open != Some(CampaignTab::Province) || self.confirmation_open() {
            return;
        }
        let (back, forward) = ctx.input(|input| {
            (
                input.pointer.button_pressed(egui::PointerButton::Extra1),
                input.pointer.button_pressed(egui::PointerButton::Extra2),
            )
        });
        if back != forward {
            self.section = (self.section
                + if forward {
                    1
                } else {
                    PROVINCE_SECTION_COUNT - 1
                })
                % PROVINCE_SECTION_COUNT;
            self.close_province_selector();
        }
    }

    pub(crate) fn confirmation_open(&self) -> bool {
        self.confirmation.is_some()
    }

    /// Escape dismisses the confirmation before touching the underlying panel.
    pub(crate) fn dismiss_confirmation(&mut self) -> bool {
        self.confirmation.take().is_some()
    }

    /// Rome's city and province selections lead directly to the Senate.
    fn select_map_detail(&mut self, selected: MapDetail, campaign: &Campaign) {
        self.close_province_selector();
        let province = match selected {
            MapDetail::Province(id) | MapDetail::City(id) => id,
        };
        if self.province.is_none() && campaign.economy.provinces[province].name != "Latium" {
            self.section = if matches!(selected, MapDetail::City(_)) {
                2
            } else {
                0
            };
        }
        self.province = Some(province);
        self.last_detail = Some(selected);
        if campaign.economy.provinces[province].name == "Latium" {
            self.politics_section = 0;
        }
        self.open = Some(if campaign.economy.provinces[province].name == "Latium" {
            CampaignTab::Senate
        } else {
            CampaignTab::Province
        });
    }
    /// Keep an explicit destination when map selection changes in the next frame.
    pub(crate) fn open_province_section(&mut self, province: usize, section: usize) {
        self.close_province_selector();
        self.province = Some(province);
        self.section = section;
        self.open = Some(CampaignTab::Province);
        self.last_detail = Some(MapDetail::Province(province));
    }
    /// Escape dismisses the search before closing the inspector itself.
    pub(crate) fn close_province_selector(&mut self) -> bool {
        std::mem::take(&mut self.province_selector_open)
    }

    /// Open the context containing acquired evidence instead of a generic overview.
    pub(crate) fn open_evidence(&mut self, province: Option<usize>) {
        self.close_province_selector();
        if let Some(province) = province {
            self.province = Some(province);
            self.section = 4;
            self.open = Some(CampaignTab::Province);
            self.last_detail = Some(MapDetail::Province(province));
        } else {
            self.politics_section = 0;
            self.open = Some(CampaignTab::Senate);
        }
    }
}

/// Match the hand-painted map's parchment surfaces locally without changing menu styling.
fn panel_style(ui: &mut egui::Ui, scale: f32) {
    *ui.style_mut() = campaign_widgets::map_style(scale);
}

/// The same frame is shared by native rendering and whole-panel layout checks.
fn panel_frame(scale: f32) -> egui::Frame {
    egui::Frame::new().fill(province_panel::PAPER).corner_radius(6.0 * scale).inner_margin(0.0)
}

/// The title gains an outline on hover or focus and opens an aligned province list.
fn province_title_selector(
    ui: &mut egui::Ui,
    size: egui::Vec2,
    ink: egui::Color32,
    scale: f32,
    provinces: &[crate::game::economy::EconomicProvince],
    player: usize,
    view: &mut CampaignUi,
) {
    let province = view.province.unwrap_or(0).min(provinces.len() - 1);
    let options: Vec<_> = provinces
        .iter()
        .enumerate()
        .map(|(id, candidate)| campaign_widgets::TitleSearchOption {
            name: &candidate.name,
            label: format!(
                "{}{}",
                candidate.name,
                if candidate.owner == Some(player) {
                    " · yours"
                } else {
                    ""
                }
            ),
            selected: province == id,
        })
        .collect();
    if let Some(id) = campaign_widgets::title_search_selector(
        ui,
        size,
        ink,
        scale,
        &provinces[province].name,
        ui.id().with("province-title-search"),
        &mut view.province_selector_open,
        &mut view.province_search,
        &options,
        "No provinces match",
    ) {
        view.province = Some(id);
    }
}

/// Header content participates in normal layout so wrapped titles and tabs are
/// accounted for before the body receives its remaining vertical space.
fn panel_header(
    ui: &mut egui::Ui,
    title: &str,
    scale: f32,
    provinces: &[crate::game::economy::EconomicProvince],
    player: usize,
    view: &mut CampaignUi,
) -> bool {
    let tab = view.open.unwrap_or(CampaignTab::Province);
    let province = view.province.unwrap_or(0).min(provinces.len() - 1);
    let mut closed = false;
    let color = ui
        .ctx()
        .data(|d| d.get_temp::<egui::Color32>(egui::Id::new("campaign-owner-color")))
        .unwrap_or(PLAYER_COLORS[0]);
    let brightness =
        f32::from(color.r()) * 0.299 + f32::from(color.g()) * 0.587 + f32::from(color.b()) * 0.114;
    let header_ink = if brightness > 150.0 {
        province_panel::INK
    } else {
        egui::Color32::from_rgb(255, 238, 210)
    };
    let radius = (6.0 * scale).round() as u8;
    egui::Frame::new()
        .fill(color)
        .inner_margin(6.0 * scale)
        .corner_radius(egui::CornerRadius {
            nw: radius,
            ne: radius,
            sw: 0,
            se: 0,
        })
        .show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            ui.horizontal(|ui| {
                icon(ui, Icon::Eagle, 32.0 * scale);
                let available = (ui.available_width() - 40.0 * scale).max(1.0);
                if tab == CampaignTab::Province {
                    ui.allocate_ui_with_layout(
                        egui::vec2(available, 32.0 * scale),
                        egui::Layout::top_down(egui::Align::Center),
                        |ui| {
                            ui.set_min_size(egui::vec2(available, 32.0 * scale));
                            province_title_selector(
                                ui,
                                egui::vec2(available.min(300.0 * scale), 32.0 * scale),
                                header_ink,
                                scale,
                                provinces,
                                player,
                                view,
                            );
                        },
                    );
                } else {
                    ui.add_sized(
                        [available, 32.0 * scale],
                        egui::Label::new(
                            egui::RichText::new(title).size(20.0 * scale).color(header_ink),
                        )
                        .truncate(),
                    )
                    .on_hover_text(title);
                }
                let (rect, response) =
                    ui.allocate_exact_size(egui::vec2(28.0, 28.0) * scale, egui::Sense::click());
                let painter = ui.painter();
                let (fill, close_ink) = province_panel::header_close_colors(
                    color,
                    response.hovered(),
                    response.is_pointer_button_down_on() || response.clicked(),
                );
                painter.circle(
                    rect.center(),
                    12.0 * scale,
                    fill,
                    egui::Stroke::new(1.3 * scale, close_ink),
                );
                for sign in [-1.0, 1.0] {
                    painter.line_segment(
                        [
                            rect.center() + egui::vec2(-4.5, sign * -4.5) * scale,
                            rect.center() + egui::vec2(4.5, sign * 4.5) * scale,
                        ],
                        egui::Stroke::new(1.7 * scale, close_ink),
                    );
                }
                response.widget_info(|| {
                    egui::WidgetInfo::labeled(egui::WidgetType::Button, true, "Close panel")
                });
                closed = response.on_hover_cursor(egui::CursorIcon::PointingHand).clicked();
            });
        });
    if tab == CampaignTab::Senate {
        egui::Frame::new()
            .inner_margin(egui::Margin::symmetric((12.0 * scale).round() as i8, 0))
            .show(ui, |ui| {
                let tabs =
                    [(Icon::Eagle, "Senate"), (Icon::Province, "Provinces"), (Icon::Spy, "Spies")];
                let gap = ui.spacing().item_spacing.x;
                let width = ((ui.available_width() - gap * 2.0) / 3.0 - 3.0 * scale).max(1.0);
                ui.horizontal(|ui| {
                    for (index, (symbol, label)) in tabs.into_iter().enumerate() {
                        let selected = view.politics_section == index;
                        let fill = if selected {
                            egui::Color32::from_rgb(219, 200, 168)
                        } else {
                            province_panel::TABLE_STRIPE
                        };
                        let response = egui::Frame::new()
                            .fill(fill)
                            .stroke(egui::Stroke::new(
                                scale,
                                if selected {
                                    color
                                } else {
                                    province_panel::RULE
                                },
                            ))
                            .corner_radius(3.0 * scale)
                            .show(ui, |ui| {
                                ui.set_width(width);
                                ui.horizontal_centered(|ui| {
                                    icon(ui, symbol, 22.0 * scale);
                                    ui.strong(label);
                                });
                            })
                            .response
                            .interact(egui::Sense::click());
                        if response.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                            view.politics_section = index;
                        }
                    }
                });
                ui.separator();
            });
    }
    if !matches!(
        tab,
        CampaignTab::Senate | CampaignTab::Governance | CampaignTab::Military | CampaignTab::Trade
    ) {
        egui::Frame::new()
            .inner_margin(egui::Margin {
                left: (12.0 * scale).round() as i8,
                right: (12.0 * scale).round() as i8,
                top: 0,
                bottom: 0,
            })
            .show(ui, |ui| {
                if tab != CampaignTab::Province {
                    egui::ComboBox::from_id_salt("campaign_province_selector")
                        .selected_text(&provinces[province].name)
                        .width((ui.available_width() - 28.0 * scale).max(1.0))
                        .show_ui(ui, |ui| {
                            for (id, province) in provinces.iter().enumerate() {
                                ui.selectable_value(
                                    &mut view.province,
                                    Some(id),
                                    format!(
                                        "{}{}",
                                        province.name,
                                        if province.owner == Some(player) {
                                            " · yours"
                                        } else {
                                            ""
                                        }
                                    ),
                                );
                            }
                        });
                }
                if tab == CampaignTab::Province {
                    let owned = provinces[view.province.unwrap_or(province)].owner == Some(player);
                    let tabs: [_; PROVINCE_SECTION_COUNT] = [
                        (Icon::Province, "Overview"),
                        if owned {
                            (Icon::Policies, "Policies")
                        } else {
                            (Icon::Trade, "Trade")
                        },
                        (Icon::Construction, "Buildings"),
                        (Icon::Attack, "Military"),
                        (Icon::Diplomacy, "Diplomacy"),
                        (Icon::Notifications, "Notifications"),
                    ];
                    let width = (ui.available_width() - 5.0 * ui.spacing().item_spacing.x) / 6.0;
                    ui.horizontal(|ui| {
                        for (index, (symbol, label)) in tabs.into_iter().enumerate() {
                            let (rect, response) = ui.allocate_exact_size(
                                egui::vec2(width, 47.0 * scale),
                                egui::Sense::click(),
                            );
                            let selected = view.section == index;
                            let hovered = response.hovered();
                            let pressed = response.is_pointer_button_down_on();
                            if selected || hovered || pressed {
                                let fill = if pressed {
                                    egui::Color32::from_rgb(199, 163, 111)
                                } else if selected && hovered {
                                    egui::Color32::from_rgb(211, 185, 145)
                                } else if selected {
                                    egui::Color32::from_rgb(219, 200, 168)
                                } else {
                                    egui::Color32::from_rgb(231, 213, 181)
                                };
                                ui.painter().rect_filled(rect, 3.0 * scale, fill);
                                ui.painter().rect_stroke(
                                    rect,
                                    3.0 * scale,
                                    egui::Stroke::new(
                                        if pressed {
                                            1.8 * scale
                                        } else {
                                            scale
                                        },
                                        if pressed {
                                            province_panel::INK
                                        } else if selected {
                                            color
                                        } else {
                                            province_panel::RULE
                                        },
                                    ),
                                    egui::StrokeKind::Inside,
                                );
                            }
                            ui.scope_builder(
                                egui::UiBuilder::new()
                                    .max_rect(rect)
                                    .layout(egui::Layout::top_down(egui::Align::Center)),
                                |ui| {
                                    icon(ui, symbol, 26.0 * scale);
                                    ui.add_sized(
                                        [width, 16.0 * scale],
                                        egui::Label::new(
                                            egui::RichText::new(label).size(11.0 * scale),
                                        )
                                        .truncate()
                                        .show_tooltip_when_elided(false),
                                    );
                                },
                            );
                            if selected {
                                ui.painter().hline(
                                    rect.x_range(),
                                    rect.bottom(),
                                    egui::Stroke::new(3.0 * scale, color),
                                );
                            }
                            if response.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                                view.section = index;
                            }
                        }
                    });
                }
                ui.separator();
                if tab == CampaignTab::Province && view.section == 0 {
                    let selected = &provinces[view.province.unwrap_or(province)];
                    let landscape = campaign_widgets::ProvinceLandscape::Terrain(selected.terrain);
                    let landscape = campaign_widgets::portrait(ui, landscape, 76.0 * scale);
                    let city = selected
                        .has_city
                        .then(|| crate::map::city_name_for_province(&selected.name))
                        .flatten();
                    if campaign_widgets::landscape_badges(
                        ui,
                        landscape,
                        selected.terrain,
                        city,
                        scale,
                    ) {
                        view.open = Some(CampaignTab::Province);
                        view.section = 2;
                    }
                }
            });
    }
    closed
}

/// Read political values each frame so selection and monthly changes stay current.
fn province_overview(
    ui: &mut egui::Ui,
    campaign: &Campaign,
    province: usize,
    player: usize,
    scale: f32,
) -> Option<ConfirmationAction> {
    let politics = campaign.politics.get(province).filter(
        |politics| !matches!(politics.state, PoliticalState::Owned { owner } if owner != player),
    );
    let political_height = if let Some(politics) = politics {
        if acquisition_actions(politics, player).is_empty() {
            80.0
        } else {
            110.0
        }
    } else {
        0.0
    };
    let owned = campaign.economy.provinces[province].owner == Some(player);
    let height = political_height
        + if owned {
            campaign_economy::overview_height(&campaign.economy, province)
        } else {
            province_intelligence::overview_height(campaign, province, player)
        };
    let scale = scale.min(ui.available_width() / 360.0).min(ui.available_height() / height);
    ui.scope(|ui| {
        panel_style(ui, scale);
        ui.spacing_mut().item_spacing.y = 0.0;
        let mut action = None;
        if let Some(politics) = politics {
            action = overview_politics(ui, politics, player, scale);
            ui.add_space(7.0 * scale);
        }
        if owned {
            campaign_economy::overview(ui, &campaign.economy, &campaign.military, province, scale);
        } else {
            province_intelligence::overview(ui, campaign, province, player, scale);
        }
        action
    })
    .inner
}

/// Header and frame margins are accounted for before fitting the read-only overview.
fn province_overview_body(
    ui: &mut egui::Ui,
    inner_bottom: f32,
    campaign: &Campaign,
    province: usize,
    player: usize,
    scale: f32,
) -> Option<ConfirmationAction> {
    ui.set_max_height((inner_bottom - ui.next_widget_position().y).max(0.0));
    province_overview(ui, campaign, province, player, scale)
}

/// Keep building sections inside the panel, scrolling when needed.
fn province_buildings_body(
    ui: &mut egui::Ui,
    inner_bottom: f32,
    campaign: &mut Campaign,
    province: usize,
    player: usize,
    scale: f32,
    view: &mut CampaignUi,
) -> Option<String> {
    let remaining = (inner_bottom - ui.next_widget_position().y).max(0.0);
    ui.set_max_height(remaining);
    let before = construction_underway(campaign, province);
    let province_state = &campaign.economy.provinces[province];
    let landscape = if province_state.has_city {
        campaign_widgets::ProvinceLandscape::City
    } else {
        campaign_widgets::ProvinceLandscape::Village
    };
    campaign_widgets::portrait(ui, landscape, 76.0 * scale);
    let remaining = (inner_bottom - ui.next_widget_position().y).max(0.0);
    let message = egui::ScrollArea::vertical()
        .id_salt(("province-buildings", province, player))
        .max_height(remaining)
        .min_scrolled_height(0.0)
        .auto_shrink([false, false])
        .show(ui, |ui| {
            let owned = campaign.economy.provinces[province].owner == Some(player);
            let queue_action = campaign_economy::construction_queue_view(
                ui,
                &campaign.economy,
                province,
                owned,
                scale,
            );
            if let Some(action) = queue_action {
                view.confirmation = campaign_confirmation::PendingConfirmation::construction(
                    campaign, province, player, action,
                );
            }
            if owned {
                campaign_economy::buildings(ui, &mut campaign.economy, province, player, scale)
            } else {
                province_intelligence::buildings(ui, campaign, province, player, scale);
                None
            }
        })
        .inner;
    let after = construction_underway(campaign, province);
    if before != after {
        if let Some(Icon::Wonder(wonder)) = after {
            campaign.notify_wonder_started(player, province, wonder);
        }
    }
    message
}

/// Compact meters show political values; full ownership fills the domestic indicators.
fn overview_politics(
    ui: &mut egui::Ui,
    politics: &ProvincePolitics,
    player: usize,
    scale: f32,
) -> Option<ConfirmationAction> {
    if politics.state == PoliticalState::Rome {
        ui.label("Rome · protected capital. Conquest grants immediate victory.");
        return None;
    }
    let gold = egui::Color32::from_rgb(190, 150, 76);
    let (control, control_text) = match &politics.state {
        PoliticalState::Independent {
            shares,
            ..
        } => {
            let value = shares.get(player).copied().unwrap_or(0.0);
            (value / 100.0, format!("{value:.0}/100"))
        },
        PoliticalState::Vassal {
            control,
            ..
        } => (control / 100.0, format!("{control:.0}/100")),
        PoliticalState::Owned {
            ..
        } => (1.0, "100/100".to_owned()),
        PoliticalState::Rome => unreachable!("Rome has no political meters"),
    };
    let domestic = matches!(politics.state, PoliticalState::Owned { owner } if owner == player);
    let control_tooltip = match &politics.state {
        PoliticalState::Independent {
            ..
        } => {
            "Your political power over this province. Above 50 Control, you can vassalize it; at 100, you can take direct ownership."
        },
        PoliticalState::Vassal {
            ..
        } => {
            "The overlord's political hold over this vassal. At 100 Control, the overlord can take direct ownership; at 0, the province becomes independent."
        },
        PoliticalState::Owned {
            owner,
        } if *owner == player => "You directly own this province and have full political control.",
        PoliticalState::Owned {
            ..
        } => {
            "This province is directly owned by another player, who has full political control. Independent Control shares and vassalization no longer apply."
        },
        PoliticalState::Rome => unreachable!("Rome has no political meters"),
    };
    let relation_tooltip = if domestic {
        "This is your own province. Relation is always full."
    } else {
        "How friendly this province is toward you."
    };
    let relation = politics.relation(player).clamp(0.0, 100.0);
    let relation_color = if relation < 20.0 {
        egui::Color32::from_rgb(155, 69, 58)
    } else if relation < 40.0 {
        egui::Color32::from_rgb(182, 108, 75)
    } else if relation < 60.0 {
        gold
    } else if relation < 80.0 {
        egui::Color32::from_rgb(139, 161, 112)
    } else {
        egui::Color32::from_rgb(103, 149, 118)
    };
    let actions = acquisition_actions(politics, player);
    let mut requested = None;
    ui.columns(2, |columns| {
        political_card(
            &mut columns[0],
            Icon::Control,
            "Control",
            control as f32,
            &control_text,
            gold,
            scale,
            &actions,
            &mut requested,
        )
        .on_hover_text(control_tooltip);
        political_card(
            &mut columns[1],
            Icon::Relation,
            "Relation",
            if domestic {
                1.0
            } else {
                relation as f32 / 100.0
            },
            &if domestic {
                "100/100".to_owned()
            } else {
                format!("{relation:.0}/100")
            },
            if domestic {
                egui::Color32::from_rgb(103, 149, 118)
            } else {
                relation_color
            },
            scale,
            &[],
            &mut requested,
        )
        .on_hover_ui(|ui| {
            ui.label(relation_tooltip);
            ui.add_space(4.0 * scale);
            for level in [
                "• 0–19 · Hostile: no trade or peaceful troop access.",
                "• 20–39 · Unfriendly: unfavorable trade terms; no peaceful troop access.",
                "• 40–59 · Neutral: standard trade terms; no peaceful troop access.",
                "• 60–79 · Friendly: better trade terms; you can move troops through the province.",
                "• 80–100 · Very friendly: the best trade terms; you can move through and station troops in the province.",
            ] {
                ui.label(level);
            }
        });
    });
    requested
}

/// The domain preview also enforces sovereignty and unique-leader eligibility.
fn acquisition_actions(politics: &ProvincePolitics, player: usize) -> Vec<ConfirmationAction> {
    let mut actions = Vec::new();
    if politics.clone().vassalize(player).is_ok() {
        actions.push(ConfirmationAction::Vassalize);
    }
    if politics.clone().take_ownership(player).is_ok() {
        actions.push(ConfirmationAction::Integrate);
    }
    actions
}

/// Cards retain a readable icon, label and exact value at compact panel widths.
fn political_card(
    ui: &mut egui::Ui,
    symbol: Icon,
    title: &str,
    progress: f32,
    value: &str,
    color: egui::Color32,
    scale: f32,
    actions: &[ConfirmationAction],
    requested: &mut Option<ConfirmationAction>,
) -> egui::Response {
    let width = ui.available_width();
    egui::Frame::new()
        .fill(province_panel::TABLE_STRIPE)
        .stroke(egui::Stroke::new(scale, province_panel::RULE))
        .corner_radius(4.0 * scale)
        .inner_margin(8.0 * scale)
        .show(ui, |ui| {
            ui.set_width((width - 2.0 * (8.0 * scale).round() - 2.0 * scale).max(1.0));
            if !actions.is_empty() {
                ui.allocate_ui_with_layout(egui::vec2(ui.available_width(), 22.0 * scale), egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.spacing_mut().item_spacing.x = 3.0 * scale;
                    for &action in actions.iter().rev() {
                        let (label, symbol) = match action {
                            ConfirmationAction::Vassalize => ("Vassalize", Icon::Vassalize),
                            ConfirmationAction::Integrate => ("Integrate", Icon::Integrate),
                            _ => unreachable!(),
                        };
                        let text = ui.painter().layout_no_wrap(label.into(), egui::FontId::proportional(10.0 * scale), province_panel::INK);
                        let (rect, response) = ui.allocate_exact_size(egui::vec2(text.size().x + 22.0 * scale, 22.0 * scale), egui::Sense::click());
                        let fill = if response.hovered() { province_panel::PAPER } else { province_panel::TABLE_STRIPE };
                        ui.painter().rect_filled(rect, 3.0 * scale, fill);
                        ui.painter().rect_stroke(rect, 3.0 * scale, egui::Stroke::new(scale, province_panel::RULE), egui::StrokeKind::Inside);
                        campaign_widgets::paint_icon(ui, symbol, egui::Rect::from_min_size(rect.min + egui::vec2(3.0, 3.0) * scale, egui::vec2(16.0, 16.0) * scale));
                        ui.painter().galley(rect.min + egui::vec2(20.0 * scale, (rect.height() - text.size().y) * 0.5), text, province_panel::INK);
                        if response.on_hover_text(format!("{label} this province. Opens a confirmation before changing its status.")).clicked() { *requested = Some(action); }
                    }
                });
            }
            ui.horizontal(|ui| {
                icon(ui, symbol, 28.0 * scale);
                ui.label(egui::RichText::new(title).size(15.0 * scale));
            });
            ui.add_space(4.0 * scale);
            ui.visuals_mut().extreme_bg_color = egui::Color32::from_rgb(224, 215, 195);
            ui.add(
                egui::ProgressBar::new(progress.clamp(0.0, 1.0))
                    .desired_width(ui.available_width())
                    .desired_height(20.0 * scale)
                    .corner_radius(4.0 * scale)
                    .fill(color)
                    .text(egui::RichText::new(value).size(13.0 * scale).color(province_panel::INK)),
            );
        })
        .response
}

/// Use all space below the header for content without carrying action text across tabs.
fn scroll_body<R>(
    ui: &mut egui::Ui,
    inner_bottom: f32,
    id: impl std::hash::Hash + std::fmt::Debug,
    render: impl FnOnce(&mut egui::Ui) -> R,
) -> R {
    let remaining = (inner_bottom - ui.next_widget_position().y).max(0.0);
    let id = egui::Id::new(id);
    let response = egui::ScrollArea::vertical()
        .id_salt(id)
        .max_height(remaining)
        .min_scrolled_height(0.0)
        .auto_shrink([false, false])
        .show(ui, render);
    response.inner
}

/// Every province section uses the overview's fixed panel height.
pub(in crate::app) fn draw(
    mut contexts: EguiContexts,
    state: Res<State<AppState>>,
    clock: Res<GameClock>,
    mut campaign: ResMut<Campaign>,
    mut view: ResMut<CampaignUi>,
    mut detail: ResMut<ProvincePanelOpen>,
    mut governance_open: ResMut<GovernancePanelOpen>,
    mut close_click: ResMut<MapPanelCloseClick>,
    practice: Res<LocalPractice>,
    mut map_view: ResMut<MapView>,
    sound: Res<MenuAudio>,
    audio: Res<Audio>,
    assets: Res<AssetServer>,
) {
    if !campaign.active || *state.get() != AppState::Map {
        return;
    }
    if detail.0 != view.last_detail {
        view.last_detail = detail.0;
        if let Some(selected) = detail.0 {
            view.select_map_detail(selected, &campaign);
        }
    }
    if view.open == Some(CampaignTab::Province)
        && view.section != 3
        && view.province.is_some_and(|id| campaign.economy.provinces[id].name == "Latium")
    {
        view.open = Some(CampaignTab::Senate);
        view.politics_section = 0;
        view.close_province_selector();
    }
    let Ok(ctx) = contexts.ctx_mut() else {
        return;
    };
    let player = practice.active_player;
    if let Some(hit) = crate::map::take_army_click(ctx) {
        open_map_army(ctx, hit.province, hit.owner, hit.movement, player);
    }
    let Some(tab) = view.open else {
        return;
    };
    // Notices still name the Governance destination; render its original map card.
    if tab == CampaignTab::Governance {
        governance_open.0 = true;
        view.open = None;
        return;
    }
    if player >= campaign.actors.len() {
        return;
    }
    let scale = viewport_ui_scale(ctx.content_rect().size());
    let screen = ctx.content_rect();
    let width = match tab {
        CampaignTab::Military => 820.0,
        CampaignTab::Senate => 730.0,
        _ => 570.0,
    } * scale;
    let width = width.min((screen.width() - 80.0 * scale).max(120.0));
    let desired_height = 720.0;
    let height = (desired_height * scale).min((screen.height() - 90.0 * scale).max(140.0));
    let rect = map_corner_panel_rect(screen, scale, egui::vec2(width, height));
    if view.province.is_none() {
        view.province =
            campaign.economy.provinces.iter().position(|p| p.owner == Some(player)).or(Some(0));
    }
    let mut province = view.province.unwrap_or(0).min(campaign.politics.len() - 1);
    let previous_province = province;
    campaign.pull_wallets();
    campaign.refresh_profiles();
    let header = match tab {
        CampaignTab::Senate => "Rome · Politics".to_owned(),
        CampaignTab::Governance => "Governance".to_owned(),
        CampaignTab::Military => "Military".to_owned(),
        CampaignTab::Trade => "Trade".to_owned(),
        CampaignTab::Province => campaign.economy.provinces[province].name.clone(),
    };
    let mut closed = false;
    let mut action = None;
    let mut navigation = None;
    let mut army_province = None;
    let mut politics_target = None;
    let old_notice = view.notice.clone();
    let confirmation_was_open = view.confirmation_open();
    let previous_section = view.section;
    let previous_politics_section = view.politics_section;
    view.cycle_province_tabs(ctx);
    let previous_rank = campaign.actors[player].rank;
    let owner_color = campaign.economy.provinces[province]
        .owner
        .and_then(|owner| practice.players.get(owner))
        .map(|owner| PLAYER_COLORS[owner.color_index])
        .unwrap_or(province_panel::NEUTRAL);
    let player_colors: Vec<_> =
        practice.players.iter().map(|p| PLAYER_COLORS[p.color_index]).collect();
    ctx.data_mut(|d| {
        d.insert_temp(egui::Id::new("campaign-player-colors"), player_colors.clone());
        d.insert_temp(egui::Id::new("campaign-active-player-color"), player_colors[player]);
        d.insert_temp(
            egui::Id::new("campaign-construction-month-fraction"),
            (clock.month_progress / SECONDS_PER_MONTH).clamp(0.0, 1.0),
        );
        d.insert_temp(
            egui::Id::new("campaign-owner-color"),
            if matches!(
                tab,
                CampaignTab::Senate
                    | CampaignTab::Governance
                    | CampaignTab::Military
                    | CampaignTab::Trade
            ) {
                PLAYER_COLORS[practice.players[player].color_index]
            } else {
                owner_color
            },
        )
    });
    egui::Area::new(egui::Id::new("augustus_campaign_panel"))
        .fixed_pos(rect.min)
        .order(egui::Order::Foreground)
        .movable(false)
        .sense(egui::Sense::hover())
        .show(ctx, |ui| {
            panel_style(ui, scale);
            let panel = panel_frame(scale).show(ui, |ui| {
                ui.set_width(width);
                ui.set_height(height);
                closed = panel_header(
                    ui,
                    &header,
                    scale,
                    &campaign.economy.provinces,
                    player,
                    &mut view,
                );
                province = view.province.unwrap_or(province);
                if tab == CampaignTab::Province && campaign.economy.provinces[province].name == "Latium" {
                    view.open = Some(CampaignTab::Senate);
                    view.politics_section = 0;
                    return;
                }
                egui::Frame::new()
                    .inner_margin(egui::Margin {
                        left: (12.0 * scale).round() as i8,
                        right: (12.0 * scale).round() as i8,
                        top: 0,
                        bottom: (12.0 * scale).round() as i8,
                    })
                    .show(ui, |ui| {
                        if tab == CampaignTab::Military {
                            ui.set_clip_rect(ui.clip_rect().intersect(egui::Rect::from_min_max(
                                rect.min,
                                egui::pos2(rect.right(), rect.bottom() - 12.0 * scale),
                            )));
                            let military = campaign.military_view(player, false);
                            let colors: Vec<_> = practice
                                .players
                                .iter()
                                .map(|p| PLAYER_COLORS[p.color_index])
                                .collect();
                            army_province = campaign_military::overview(
                                ui,
                                &military,
                                &campaign.economy,
                                player,
                                &colors,
                                scale,
                            );
                            return;
                        }
                        if tab == CampaignTab::Province
                            && view.section == 1
                            && campaign.economy.provinces[province].owner == Some(player)
                        {
                            ui.set_max_height(
                                (rect.bottom() - 12.0 * scale - ui.next_widget_position().y)
                                    .max(0.0),
                            );
                            let previous = campaign.economy.provinces[province].policies;
                            campaign_economy::policies(ui, &mut campaign.economy, province, player);
                            if campaign.economy.provinces[province].policies != previous {
                                play_click(&sound, &audio, &assets);
                            }
                            return;
                        }
                        if tab == CampaignTab::Province && view.section == 0 {
                            let requested = province_overview_body(
                                ui,
                                rect.bottom() - 12.0 * scale,
                                &campaign,
                                province,
                                player,
                                scale,
                            );
                            if let Some(action) = requested {
                                view.confirmation = campaign_confirmation::PendingConfirmation::new(&campaign, province, player, action);
                            }
                            return;
                        }
                        if tab == CampaignTab::Province && view.section == 2 {
                            if let Some(message) = province_buildings_body(
                                ui,
                                rect.bottom() - 12.0 * scale,
                                &mut campaign,
                                province,
                                player,
                                scale,
                                &mut view,
                            ) {
                                view.notice = message;
                            }
                            return;
                        }
                        if tab == CampaignTab::Province && view.section == 5 {
                            navigation = scroll_body(
                                ui,
                                rect.bottom() - 12.0 * scale,
                                ("campaign_body", tab as usize, province, view.section),
                                |ui| province_notifications(ui, &campaign, province, player, scale),
                            );
                            return;
                        }
                        if tab == CampaignTab::Province && view.section == 3 {
                            ui.set_max_height((rect.bottom() - 12. * scale - ui.next_widget_position().y).max(0.));
                            ui.set_clip_rect(ui.clip_rect().intersect(egui::Rect::from_min_max(rect.min, egui::pos2(rect.right(), rect.bottom() - 12. * scale))));
                                            let access = campaign.access_snapshot();
                                            let military = campaign.military_view(player, true);
                                            let observed: Vec<_> = (0..campaign.politics.len())
                                                .map(|id| campaign.observes_military(player, id))
                                                .collect();
                                            action = campaign_military::show(
                                                ui,
                                                &military,
                                                &campaign.economy,
                                                &campaign.graph,
                                                province,
                                                player,
                                                |id| observed.get(id).copied().unwrap_or(false),
                                                |_| None,
                                                |owner, id| match owner {
                                                    ForceOwner::Player(p) => access[p][id],
                                                    ForceOwner::Local(home) => {
                                                        if home == id {
                                                            MilitaryAccess::Peaceful
                                                        } else {
                                                            MilitaryAccess::Blocked
                                                        }
                                                    },
                                                },
                                                |a, b| campaign.forces_hostile(a, b),
                                            );
                            return;
                        }
                        scroll_body(
                            ui,
                            rect.bottom() - 12.0 * scale,
                            (
                                "campaign_body",
                                tab as usize,
                                province,
                                if tab == CampaignTab::Senate {
                                    view.politics_section
                                } else {
                                    view.section
                                },
                            ),
                            |ui| {
                                if tab == CampaignTab::Trade {
                                    if let Some(message) =
                                        campaign_trade::show(ui, &mut campaign, player, scale)
                                    {
                                        view.notice = message;
                                    }
                                } else if tab == CampaignTab::Province && view.section == 1 {
                                    if let Some(message) = campaign_trade::show_province(
                                        ui,
                                        &mut campaign,
                                        province,
                                        player,
                                        scale,
                                    ) {
                                        view.notice = message;
                                    }
                                } else if tab == CampaignTab::Senate {
                                    if view.politics_section == 1 {
                                        politics_target = politics_provinces(
                                            ui, &campaign, player, &player_colors,
                                            &mut view.politics_province_search, scale,
                                        );
                                        return;
                                    }
                                    if view.politics_section == 2 {
                                        let (target, message) = politics_spies(
                                            ui, &mut campaign, player,
                                            &mut view.politics_spy_search, scale,
                                        );
                                        politics_target = target;
                                        if let Some(message) = message {
                                            view.notice = message;
                                        }
                                        return;
                                    }
                                    if let Some(capital) = campaign.economy.provinces.iter().position(|p| p.name == "Latium") {
                                        ui.horizontal_wrapped(|ui| {
                                            ui.strong("Rome · protected capital");
                                            ui.label(format!("{} defending cohorts", campaign.military.all_units().filter(|u| u.owner == ForceOwner::Local(capital)).count()));
                                        });
                                        ui.small("Conquer Rome to win immediately. The capital permits no provincial actions.");
                                        if campaign.senate.winner.is_none() && !campaign.npc_wars[player][capital]
                                            && ui.button("March on Rome").on_hover_text("Declare war on the capital so your armies can enter Rome. Defeat its large army to win immediately.").clicked() {
                                            campaign.declare_hostility(player, capital);
                                            view.notice = "War declared on Rome. Move your armies to the capital to attack.".into();
                                        }
                                        ui.separator();
                                    }
                                    if let Some(id) = view.highlight_scandal {
                                        ui.group(|ui| {
                                    if let Some(scandal) = campaign
                                        .espionage
                                        .scandals
                                        .iter()
                                        .find(|s| s.id == id && s.holder == player)
                                    {
                                        ui.strong(format!(
                                            "Selected evidence · {}",
                                            scandal.kind.label()
                                        ));
                                        ui.label(format!(
                                            "{:?} · {:?} · {} months remaining",
                                            scandal.target,
                                            scandal.severity,
                                            scandal.expires.saturating_sub(campaign.senate.month)
                                        ));
                                    } else {
                                        ui.label("This evidence has been consumed or has expired.");
                                    }
                                    if ui.small_button("Dismiss selection").clicked() {
                                        view.highlight_scandal = None;
                                    }
                                });
                                    }
                                    let colors: Vec<_> = practice
                                        .players
                                        .iter()
                                        .map(|p| PLAYER_COLORS[p.color_index])
                                        .collect();
                                    let Campaign {
                                        senate,
                                        actors,
                                        profiles,
                                        senate_config,
                                        espionage,
                                        espionage_config,
                                        ..
                                    } = &mut *campaign;
                                    if let Some(message) = campaign_politics::show(
                                        ui,
                                        senate,
                                        actors,
                                        profiles,
                                        senate_config,
                                        player,
                                        espionage,
                                        espionage_config,
                                        &colors,
                                    ) {
                                        view.notice = message;
                                    }
                                } else {
                                    let section = match tab {
                                        CampaignTab::Governance => 1,
                                        _ => view.section,
                                    };
                                    match section {
                                        0 => {
                                            if let Some(action) = province_overview(ui, &campaign, province, player, scale) {
                                                view.confirmation = campaign_confirmation::PendingConfirmation::new(&campaign, province, player, action);
                                            }
                                        },
                                        1 => campaign_economy::policies(
                                            ui,
                                            &mut campaign.economy,
                                            province,
                                            player,
                                        ),
                                        3 => {
                                            let access = campaign.access_snapshot();
                                            let military = campaign.military_view(player, true);
                                            let observed: Vec<_> = (0..campaign.politics.len())
                                                .map(|id| campaign.observes_military(player, id))
                                                .collect();
                                            action = campaign_military::show(
                                                ui,
                                                &military,
                                                &campaign.economy,
                                                &campaign.graph,
                                                province,
                                                player,
                                                |id| observed.get(id).copied().unwrap_or(false),
                                                |_| None,
                                                |owner, id| match owner {
                                                    ForceOwner::Player(p) => access[p][id],
                                                    ForceOwner::Local(home) => {
                                                        if home == id {
                                                            MilitaryAccess::Peaceful
                                                        } else {
                                                            MilitaryAccess::Blocked
                                                        }
                                                    },
                                                },
                                                |a, b| campaign.forces_hostile(a, b),
                                            );
                                        },
                                        4 => {
                                            if let Some(message) =
                                                diplomacy(ui, &mut campaign, province, player, &player_colors, &mut view)
                                            {
                                                view.notice = message;
                                            }
                                        },
                                        _ => {
                                            navigation = province_notifications(
                                                ui, &campaign, province, player, scale,
                                            );
                                        },
                                    }
                                }
                            },
                        );
                    });
            });
            ui.painter().rect_stroke(
                panel.response.rect,
                6.0 * scale,
                egui::Stroke::new(1.2 * scale, province_panel::RULE),
                egui::StrokeKind::Inside,
            );
        });
    audio_controls::play_construction_sound(ctx, &sound, &audio, &assets);
    if province != previous_province {
        detail.0 = Some(MapDetail::Province(province));
        view.last_detail = detail.0;
        close_click.0 = true;
        play_click(&sound, &audio, &assets);
    }
    // Political and economic controls have separate wallet boundaries.
    if tab == CampaignTab::Senate || (tab == CampaignTab::Province && view.section == 4) {
        campaign.push_wallets();
    }
    if campaign.actors[player].rank != previous_rank {
        use crate::game::politics::senate::SenateEvent;
        let rank = campaign.actors[player].rank;
        campaign.record_senate_event(&if rank == crate::game::politics::PoliticalRank::Augustus {
            SenateEvent::Victory(player)
        } else {
            SenateEvent::RankAdvanced(player, rank)
        });
    }
    if let Some(action) = action {
        dispatch_military_action(&mut view, &mut campaign, province, player, action);
    }
    if let Some(province) = army_province {
        open_military_province(ctx, &mut view, &mut detail, &mut map_view, province, player);
        close_click.0 = true;
        play_click(&sound, &audio, &assets);
    }
    if let Some(target) = politics_target {
        detail.0 = Some(MapDetail::Province(target));
        view.last_detail = detail.0;
        map_view.focus_province(target);
        if campaign.economy.provinces[target].name == "Latium" {
            view.politics_section = 0;
        } else {
            view.open_province_section(target, 0);
        }
        close_click.0 = true;
        play_click(&sound, &audio, &assets);
    }
    if let Some(notice) = navigation {
        if notice.kind == super::campaign_notifications::NoticeKind::TradeInterrupted {
            campaign_trade::open_routes(ctx, player);
        }
        open_notification(&mut view, &notice, &campaign, &mut detail, &mut map_view);
        close_click.0 = true;
        play_click(&sound, &audio, &assets);
    }
    if closed {
        view.close_province_selector();
        view.open = None;
        detail.0 = None;
        view.last_detail = None;
        close_click.0 = true;
        play_click(&sound, &audio, &assets);
    } else if old_notice != view.notice
        || view.section != previous_section
        || view.politics_section != previous_politics_section
        || (!confirmation_was_open && view.confirmation_open())
    {
        play_click(&sound, &audio, &assets);
    }
}

/// Destructive commands wait for an explicit Yes; all other orders remain immediate.
pub(super) fn dispatch_military_action(
    view: &mut CampaignUi,
    campaign: &mut Campaign,
    province: usize,
    player: usize,
    action: campaign_military::MilitaryUiAction,
) {
    use campaign_military::MilitaryUiAction::*;
    let pending = match action {
        CancelRecruitment => campaign_confirmation::ConfirmationAction::CancelRecruitment,
        CancelQueuedRecruitment(index) => {
            campaign_confirmation::ConfirmationAction::CancelQueuedRecruitment(index)
        },
        Disband(id) => campaign_confirmation::ConfirmationAction::Disband(id),
        DisbandArmy => campaign_confirmation::ConfirmationAction::DisbandArmy,
        action => {
            view.notice = apply_military_action(campaign, province, player, action);
            return;
        },
    };
    view.confirmation =
        campaign_confirmation::PendingConfirmation::new(campaign, province, player, pending);
}

/// Map figures select only their own army window.
fn open_map_army(
    ctx: &egui::Context,
    province: usize,
    owner: ForceOwner,
    movement: Option<u64>,
    player: usize,
) {
    if owner == ForceOwner::Player(player) {
        campaign_military::open_army_panel(ctx, province, player, movement);
    }
}

/// The army window has its own render pass so province navigation cannot hide it.
pub(in crate::app) fn draw_army_panel(
    mut contexts: EguiContexts,
    state: Res<State<AppState>>,
    practice: Res<LocalPractice>,
    mut campaign: ResMut<Campaign>,
    mut view: ResMut<CampaignUi>,
) {
    if !campaign.active || *state.get() != AppState::Map {
        return;
    }
    let player = practice.active_player;
    if player >= campaign.actors.len() {
        return;
    }
    let Ok(ctx) = contexts.ctx_mut() else {
        return;
    };
    let military = campaign.military_view(player, true);
    let access = campaign.access_snapshot();
    let observed: Vec<_> =
        (0..campaign.politics.len()).map(|id| campaign.observes_military(player, id)).collect();
    if let Some((province, action)) = campaign_military::draw_army_panel(
        ctx,
        &military,
        &campaign.economy,
        &campaign.graph,
        player,
        |id| observed.get(id).copied().unwrap_or(false),
        |_| None,
        |owner, id| match owner {
            ForceOwner::Player(p) => access[p][id],
            ForceOwner::Local(home) => {
                if home == id {
                    MilitaryAccess::Peaceful
                } else {
                    MilitaryAccess::Blocked
                }
            },
        },
        |a, b| campaign.forces_hostile(a, b),
    ) {
        dispatch_military_action(&mut view, &mut campaign, province, player, action);
    }
}

/// Synchronize map selection so the next frame retains the requested military section.
fn open_military_province(
    ctx: &egui::Context,
    view: &mut CampaignUi,
    detail: &mut ProvincePanelOpen,
    map: &mut MapView,
    province: usize,
    player: usize,
) {
    view.open_province_section(province, 3);
    view.notice.clear();
    campaign_military::open_army_tab(ctx, province, player);
    detail.0 = Some(MapDetail::Province(province));
    map.focus_province(province);
}

/// Dispatch validated military commands; never grant free recruits or mutate locked battle plans.
pub(super) fn apply_military_action(
    campaign: &mut Campaign,
    province: usize,
    player: usize,
    action: campaign_military::MilitaryUiAction,
) -> String {
    use campaign_military::MilitaryUiAction::*;
    let result: Result<(), String> = match action {
        Recruit(kind) => {
            let p = &mut campaign.economy.provinces[province];
            campaign
                .military
                .recruit(
                    province,
                    player,
                    kind,
                    p.owner == Some(player),
                    &recruitment_tags(&p.name),
                    &mut p.population,
                    &mut campaign.economy.players[player].resources[1],
                )
                .map_err(|e| e.to_string())
        },
        CancelRecruitment => campaign
            .military
            .cancel_recruitment(province, ForceOwner::Player(player))
            .map_err(|e| e.to_string()),
        CancelQueuedRecruitment(index) => {
            let p = &mut campaign.economy.provinces[province];
            campaign
                .military
                .cancel_queued_recruitment(
                    province,
                    player,
                    index,
                    p.owner == Some(player),
                    &mut p.population,
                    &mut campaign.economy.players[player].resources[1],
                )
                .map_err(|e| e.to_string())
        },
        Disband(id) => {
            let p = &mut campaign.economy.provinces[province];
            campaign
                .military
                .disband(province, player, id, p.owner == Some(player), &mut p.population)
                .map_err(|e| e.to_string())
        },
        DisbandArmy => {
            let p = &mut campaign.economy.provinces[province];
            campaign
                .military
                .disband_army(province, player, p.owner == Some(player), &mut p.population)
                .map_err(|e| e.to_string())
        },
        SavePlan(plan) => campaign
            .military
            .set_plan(province, ForceOwner::Player(player), plan)
            .map_err(|e| e.to_string()),
        SaveMovementPlan {
            order,
            plan,
        } => campaign
            .military
            .set_movement_plan(order, ForceOwner::Player(player), plan)
            .map_err(|e| e.to_string()),
        Move {
            destination,
            units,
            route,
            plan,
        } => {
            if route.last().copied() != Some(destination) {
                return "The selected route no longer reaches the destination.".into();
            }
            let access = campaign.access_snapshot();
            campaign
                .military
                .order_movement_route(
                    province,
                    ForceOwner::Player(player),
                    &units,
                    Some(plan),
                    &route,
                    &campaign.graph,
                    |owner, id| match owner {
                        ForceOwner::Player(p) => access[p][id],
                        _ => MilitaryAccess::Blocked,
                    },
                )
                .map(|_| ())
                .map_err(|e| e.to_string())
        },
        Retreat {
            battle,
            attacker,
        } => {
            let config = campaign.military.config.clone();
            campaign
                .military
                .battles
                .iter_mut()
                .find(|b| b.id == battle)
                .ok_or_else(|| "Battle has ended.".to_owned())
                .and_then(|b| b.request_retreat(attacker, &config).map_err(|e| e.to_string()))
        },
    };
    result.map_or_else(|message| message, |_| String::new())
}

/// Foreign diplomacy shares the overview cards and offers persistent spy missions.
fn diplomacy(
    ui: &mut egui::Ui,
    campaign: &mut Campaign,
    province: usize,
    player: usize,
    colors: &[egui::Color32],
    view: &mut CampaignUi,
) -> Option<String> {
    let scale = super::viewport_ui_scale(ui.ctx().content_rect().size());
    let portrait = campaign_widgets::portrait(
        ui,
        campaign_widgets::ProvinceLandscape::Diplomacy,
        76.0 * scale,
    );
    let state = campaign.politics[province].state.clone();
    if state == PoliticalState::Rome {
        ui.label("Rome permits no provincial actions. Open the Senate to manage politics or declare a march on the capital.");
        return None;
    }
    if matches!(state, PoliticalState::Owned { owner } if owner == player) {
        return military_diplomacy(ui, campaign, province, player, player, colors);
    }
    let distance = campaign.distance(player, province);
    political_distance_badge(ui, portrait, distance, scale);
    ui.add_space(6.0 * scale);
    if let Some(action) = overview_politics(ui, &campaign.politics[province], player, scale) {
        view.confirmation =
            campaign_confirmation::PendingConfirmation::new(campaign, province, player, action);
    }
    ui.add_space(6.0 * scale);
    if matches!(state, PoliticalState::Vassal { overlord, .. } if overlord == player) {
        policy_widgets::section(ui, scale, "VASSAL GOVERNMENT");
        ui.label("This province is your vassal. Relations and a stationed garrison build its Control over time.");
        return None;
    }
    let message = spy_network(ui, campaign, province, player, scale);
    let evidence: Vec<_> = campaign
        .espionage
        .scandals
        .iter()
        .filter(|evidence| {
            evidence.holder == player
                && evidence.province == Some(province)
                && (evidence.expires > campaign.economy.month || evidence.reserved_for_motion)
        })
        .collect();
    if !evidence.is_empty() {
        policy_widgets::section(ui, scale, "DISCOVERED EVIDENCE");
        for scandal in evidence {
            ui.label(scandal.kind.label()).on_hover_text(format!(
                "{:?} · {} months remaining",
                scandal.severity,
                scandal.expires.saturating_sub(campaign.economy.month)
            ));
        }
    }
    message
}

const SPY_ACTIONS: [(SpyAssignment, &str, Icon); 4] = [
    (SpyAssignment::GainControl, "Build control", Icon::SpyBuildControl),
    (SpyAssignment::ImproveRelations, "Improve relations", Icon::SpyImproveRelations),
    (SpyAssignment::DiscoverScandals, "Uncover scandals", Icon::SpyUncoverScandals),
    (SpyAssignment::UndermineOpponents, "Undermine opponents", Icon::SpyUndermineOpponents),
];

/// A single political reading is shared by the directory and its summary.
fn politics_reading(
    politics: &ProvincePolitics,
    player: usize,
) -> (&'static str, f64, Option<f64>) {
    match &politics.state {
        PoliticalState::Rome => ("Capital", 0.0, None),
        PoliticalState::Independent {
            shares,
            ..
        } => (
            "Independent",
            shares.get(player).copied().unwrap_or(0.0),
            Some(politics.relation(player)),
        ),
        PoliticalState::Vassal {
            overlord,
            control,
            ..
        } => (
            if *overlord == player {
                "Your vassal"
            } else {
                "Foreign vassal"
            },
            if *overlord == player {
                *control
            } else {
                0.0
            },
            Some(politics.relation(player)),
        ),
        PoliticalState::Owned {
            owner,
        } => (
            if *owner == player {
                "Your province"
            } else {
                "Rival province"
            },
            if *owner == player {
                100.0
            } else {
                0.0
            },
            Some(if *owner == player {
                100.0
            } else {
                politics.relation(player)
            }),
        ),
    }
}

fn politics_provinces(
    ui: &mut egui::Ui,
    campaign: &Campaign,
    player: usize,
    colors: &[egui::Color32],
    search: &mut String,
    scale: f32,
) -> Option<usize> {
    let provinces = &campaign.economy.provinces;
    let controlled = campaign.politics.iter().filter(|p| matches!(p.state,
        PoliticalState::Owned { owner } | PoliticalState::Vassal { overlord: owner, .. } if owner == player
    )).count();
    policy_widgets::section(ui, scale, "PROVINCIAL POLITICS");
    ui.horizontal_wrapped(|ui| {
        ui.strong(format!("{} provinces", provinces.len()));
        ui.label(format!("· {controlled} under your rule"));
        ui.label("· Select a province to open its overview");
    });
    ui.add_space(5.0 * scale);
    ui.add_sized(
        [ui.available_width(), 28.0 * scale],
        egui::TextEdit::singleline(search).hint_text("Find a province…"),
    );
    ui.add_space(8.0 * scale);
    let query = search.trim().to_lowercase();
    let mut ids: Vec<_> = (0..provinces.len())
        .filter(|&id| provinces[id].name.to_lowercase().contains(&query))
        .collect();
    ids.sort_by_key(|&id| provinces[id].name.to_lowercase());
    if ids.is_empty() {
        ui.label("No provinces match your search.");
    }
    let mut target = None;
    for (row, id) in ids.into_iter().enumerate() {
        let province = &provinces[id];
        let politics = &campaign.politics[id];
        let (status, control, relation) = politics_reading(politics, player);
        let owner_color = province
            .owner
            .and_then(|owner| colors.get(owner).copied())
            .unwrap_or(province_panel::NEUTRAL);
        let fill = if row % 2 == 0 {
            province_panel::TABLE_STRIPE
        } else {
            province_panel::PAPER
        };
        egui::Frame::new()
            .fill(fill)
            .stroke(egui::Stroke::new(scale, province_panel::RULE))
            .corner_radius(3.0 * scale)
            .inner_margin(7.0 * scale)
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    let (swatch, _) =
                        ui.allocate_exact_size(egui::vec2(7.0, 26.0) * scale, egui::Sense::hover());
                    ui.painter().rect_filled(swatch, 2.0 * scale, owner_color);
                    if ui
                        .add_sized(
                            [173.0 * scale, 26.0 * scale],
                            egui::Button::new(egui::RichText::new(&province.name).strong()),
                        )
                        .on_hover_text("Open province overview")
                        .clicked()
                    {
                        target = Some(id);
                    }
                    ui.add_sized([108.0 * scale, 26.0 * scale], egui::Label::new(status));
                    ui.vertical(|ui| {
                        ui.small("YOUR CONTROL");
                        ui.add(
                            egui::ProgressBar::new((control / 100.0).clamp(0.0, 1.0) as f32)
                                .desired_width(125.0 * scale)
                                .fill(egui::Color32::from_rgb(166, 121, 67))
                                .text(format!("{control:.0}%")),
                        );
                    });
                    ui.vertical(|ui| {
                        ui.small("RELATION");
                        if let Some(relation) = relation {
                            let color = if relation < 40.0 {
                                egui::Color32::from_rgb(166, 83, 66)
                            } else if relation < 60.0 {
                                egui::Color32::from_rgb(179, 137, 74)
                            } else {
                                egui::Color32::from_rgb(103, 149, 118)
                            };
                            ui.add(
                                egui::ProgressBar::new((relation / 100.0).clamp(0.0, 1.0) as f32)
                                    .desired_width(125.0 * scale)
                                    .fill(color)
                                    .text(format!("{relation:.0}%")),
                            );
                        } else {
                            ui.label("—").on_hover_text("Rome has no provincial relation.");
                        }
                    });
                });
            });
        ui.add_space(4.0 * scale);
    }
    target
}

/// The directory uses the same deployment quotes and eligibility as province diplomacy.
fn politics_spies(
    ui: &mut egui::Ui,
    campaign: &mut Campaign,
    player: usize,
    search: &mut String,
    scale: f32,
) -> (Option<usize>, Option<String>) {
    let active_count = campaign.espionage.missions.iter().filter(|m| m.owner == player).count();
    policy_widgets::section(ui, scale, "SPY NETWORKS");
    ui.horizontal_wrapped(|ui| {
        ui.strong(format!("{active_count} active networks"));
        ui.label("· Recall agents or launch a mission directly from this list");
    });
    ui.add_space(5.0 * scale);
    ui.add_sized(
        [ui.available_width(), 28.0 * scale],
        egui::TextEdit::singleline(search).hint_text("Find a province…"),
    );
    ui.add_space(8.0 * scale);
    let query = search.trim().to_lowercase();
    let mut ids: Vec<_> = (0..campaign.politics.len())
        .filter(|&id| campaign.economy.provinces[id].name.to_lowercase().contains(&query))
        .collect();
    ids.sort_by_key(|&id| {
        (
            !campaign.espionage.missions.iter().any(|m| m.owner == player && m.province == id),
            campaign.economy.provinces[id].name.to_lowercase(),
        )
    });
    if ids.is_empty() {
        ui.label("No provinces match your search.");
    }
    let mut target = None;
    let mut message = None;
    for (row, id) in ids.into_iter().enumerate() {
        let name = campaign.economy.provinces[id].name.clone();
        let state = campaign.politics[id].state.clone();
        let active = campaign
            .espionage
            .missions
            .iter()
            .find(|m| m.owner == player && m.province == id)
            .cloned();
        let distance = campaign.distance(player, id);
        let upkeep = campaign.espionage_config.monthly_cost_at(distance).ok();
        let fill = if active.is_some() {
            egui::Color32::from_rgb(231, 219, 195)
        } else if row % 2 == 0 {
            province_panel::TABLE_STRIPE
        } else {
            province_panel::PAPER
        };
        egui::Frame::new()
            .fill(fill)
            .stroke(egui::Stroke::new(scale, province_panel::RULE))
            .corner_radius(3.0 * scale)
            .inner_margin(8.0 * scale)
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    icon(ui, Icon::Spy, 22.0 * scale);
                    if ui.add_sized([170.0 * scale, 26.0 * scale],
                        egui::Button::new(egui::RichText::new(&name).strong()))
                        .on_hover_text("Open province overview").clicked() {
                        target = Some(id);
                    }
                    if let Some(mission) = &active {
                        let label = SPY_ACTIONS.iter()
                            .find(|(assignment, _, _)| *assignment == mission.assignment)
                            .map_or("Active network", |(_, label, _)| *label);
                        ui.strong(label);
                        ui.label(format!("· {} months active", mission.months_active));
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.button("Recall spy").on_hover_text("End this network and its monthly upkeep.").clicked() {
                                campaign.espionage.withdraw(player, id);
                                message = Some(format!("Spy recalled from {name}. Monthly upkeep has ended."));
                            }
                        });
                    } else if matches!(state, PoliticalState::Rome) {
                        ui.label("Protected capital · no spy missions");
                    } else if matches!(state, PoliticalState::Owned { owner } if owner == player)
                        || matches!(state, PoliticalState::Vassal { overlord, .. } if overlord == player) {
                        ui.label("Under your rule · no spy missions");
                    } else {
                        ui.label("No spy deployed");
                    }
                });
                if active.is_some() {
                    ui.small(format!("Upkeep: {} sestertii/month · Network results and evidence are shown in provincial diplomacy.",
                        upkeep.map_or("—".to_owned(), |cost| format!("{cost:.0}"))));
                } else if !matches!(state, PoliticalState::Rome)
                    && !matches!(state, PoliticalState::Owned { owner } if owner == player)
                    && !matches!(state, PoliticalState::Vassal { overlord, .. } if overlord == player) {
                    ui.horizontal_wrapped(|ui| {
                        ui.small("DEPLOY");
                        for (assignment, label, symbol) in SPY_ACTIONS {
                            let cost = campaign.espionage_config.deployment_cost_at(assignment, distance).ok();
                            let available = assignment.eligible(&state)
                                && cost.is_some_and(|cost| campaign.actors[player].influence >= cost)
                                && message.is_none();
                            let reason = if !assignment.eligible(&state) {
                                "Unavailable under this province's government.".to_owned()
                            } else if distance.is_none() {
                                "No usable political route from your controlled territory.".to_owned()
                            } else if !available {
                                "Not enough Influence to deploy a spy.".to_owned()
                            } else {
                                format!("Spend {:.0} Influence to deploy. Upkeep: {} sestertii/month.",
                                    cost.unwrap_or(0.0), upkeep.map_or("—".to_owned(), |cost| format!("{cost:.0}")))
                            };
                            let image = egui::Image::new((
                                campaign_widgets::texture(ui.ctx(), symbol),
                                egui::Vec2::splat(18.0 * scale),
                            ));
                            if ui.add_enabled(available, egui::Button::image_and_text(image, label))
                                .on_hover_text(reason).clicked() {
                                let result = campaign.espionage.deploy_assignment(
                                    player, id, &mut campaign.actors, &campaign.politics,
                                    &campaign.espionage_config, assignment, distance,
                                );
                                message = Some(result.map_or_else(
                                    |error| error.to_string(),
                                    |_| format!("Spy deployed to {name}: {label}."),
                                ));
                            }
                        }
                    });
                }
            });
        ui.add_space(5.0 * scale);
    }
    (target, message)
}

/// Deployment history and recall above the illustrated mission choices.
fn spy_network(
    ui: &mut egui::Ui,
    campaign: &mut Campaign,
    province: usize,
    player: usize,
    scale: f32,
) -> Option<String> {
    let active = campaign
        .espionage
        .missions
        .iter()
        .find(|mission| mission.owner == player && mission.province == province)
        .cloned();
    let distance = campaign.distance(player, province);
    let state = campaign.politics[province].state.clone();
    let upkeep = campaign.espionage_config.monthly_cost_at(distance).ok();
    let mut message = None;
    policy_widgets::section(ui, scale, "SPY NETWORK");
    if let Some(mission) = &active {
        let (name, result_icon) = SPY_ACTIONS
            .iter()
            .find(|(assignment, _, _)| *assignment == mission.assignment)
            .map_or(("Unknown", Icon::Spy), |(_, name, icon)| (*name, *icon));
        ui.horizontal(|ui| {
            ui.add_space(8.0 * scale);
            let (icon_rect, _) =
                ui.allocate_exact_size(egui::Vec2::splat(20.0 * scale), egui::Sense::hover());
            campaign_widgets::paint_icon(ui, result_icon, icon_rect);
            ui.label(egui::RichText::new(name).size(14.0 * scale));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.visuals_mut().widgets.inactive.bg_fill = province_panel::TABLE_STRIPE;
                ui.visuals_mut().widgets.inactive.weak_bg_fill = province_panel::TABLE_STRIPE;
                ui.visuals_mut().widgets.hovered.bg_fill = egui::Color32::from_rgb(224, 210, 181);
                ui.visuals_mut().widgets.hovered.weak_bg_fill =
                    egui::Color32::from_rgb(224, 210, 181);
                ui.visuals_mut().widgets.active.bg_fill = egui::Color32::from_rgb(207, 180, 137);
                ui.visuals_mut().widgets.active.weak_bg_fill =
                    egui::Color32::from_rgb(207, 180, 137);
                let image = egui::Image::new((
                    campaign_widgets::texture(ui.ctx(), Icon::Cancel),
                    egui::Vec2::splat(20.0 * scale),
                ));
                if ui.add(egui::Button::image_and_text(image, "Recall spy")).clicked() {
                    campaign.espionage.withdraw(player, province);
                    message = Some("Spy recalled. Monthly upkeep has ended.".into());
                }
            });
        });
        let totals = &mission.totals;
        let (result_caption, result_value, result_tip) = match mission.assignment {
            SpyAssignment::GainControl => (
                "Control added",
                policy_widgets::compact_decimal(totals.control_contributed),
                "Control contributed by this spy since deployment.".to_owned(),
            ),
            SpyAssignment::ImproveRelations => (
                "Relation gained",
                policy_widgets::compact_decimal(totals.relation_gained),
                "Relation improved by this spy since deployment.".to_owned(),
            ),
            SpyAssignment::DiscoverScandals => (
                "Scandals found",
                totals.scandals_revealed.to_string(),
                "Scandals revealed by this spy, including spent or expired evidence.".to_owned(),
            ),
            SpyAssignment::UndermineOpponents => {
                if campaign.economy.provinces[province].owner.is_some() {
                    (
                        "Happiness lost",
                        policy_widgets::compact_decimal(totals.happiness_reduced),
                        "Population happiness reduced by this spy since deployment.".to_owned(),
                    )
                } else {
                    (
                        "Rival losses",
                        format!(
                            "{} C · {} R",
                            policy_widgets::compact_decimal(totals.rival_control_reduced),
                            policy_widgets::compact_decimal(totals.rival_relation_reduced)
                        ),
                        "C = rival Control reduced; R = rival Relation reduced.".to_owned(),
                    )
                }
            },
        };
        let risk = crate::game::politics::espionage::detection_chance(
            campaign.economy.provinces[province].happiness[0],
            &campaign.espionage_config,
        ) * 100.0;
        let badges = [
            (
                Icon::Coin,
                "Sestertii spent",
                policy_widgets::compact_decimal(totals.coin_spent),
                "Total sestertii paid in monthly upkeep.".to_owned(),
            ),
            (result_icon, result_caption, result_value, result_tip),
            (
                Icon::Spy,
                "Detection",
                format!("{}%", policy_widgets::compact_decimal(risk)),
                "Current chance of detection at each check.".to_owned(),
            ),
        ];
        ui.add_space(7.0 * scale);
        let columns = if ui.available_width() >= 390.0 * scale {
            3
        } else {
            2
        };
        let gap = 6.0 * scale;
        let width = (ui.available_width() - (columns - 1) as f32 * gap) / columns as f32;
        for row in badges.chunks(columns) {
            let (row_rect, _) = ui.allocate_exact_size(
                egui::vec2(ui.available_width(), 48.0 * scale),
                egui::Sense::hover(),
            );
            for (index, (symbol, caption, value, tip)) in row.iter().enumerate() {
                let rect = egui::Rect::from_min_size(
                    row_rect.min + egui::vec2(index as f32 * (width + gap), 0.0),
                    egui::vec2(width, row_rect.height()),
                );
                campaign_economy::overview_badge(
                    ui,
                    rect,
                    *symbol,
                    caption,
                    value,
                    tip,
                    province_panel::INK,
                    scale,
                );
            }
            ui.add_space(gap);
        }
    } else {
        ui.label("No spy deployed.");
    }
    policy_widgets::section(ui, scale, "SPY MISSIONS");
    let actions: Vec<_> = SPY_ACTIONS
        .into_iter()
        .filter(|(assignment, _, _)| {
            !(matches!(assignment, SpyAssignment::GainControl | SpyAssignment::ImproveRelations)
                && (campaign.economy.provinces[province].owner.is_some()
                    || matches!(state, PoliticalState::Owned { .. })))
        })
        .collect();
    let gap = 10.0 * scale;
    let button_size = egui::vec2((ui.available_width() - gap) / 2.0, 84.0 * scale);
    let rows = actions.len().div_ceil(2);
    let (grid, _) = ui.allocate_exact_size(
        egui::vec2(ui.available_width(), rows as f32 * (button_size.y + gap) - gap),
        egui::Sense::hover(),
    );
    for (index, (assignment, name, symbol)) in actions.into_iter().enumerate() {
        let deployment = campaign.espionage_config.deployment_cost_at(assignment, distance).ok();
        let description = match assignment {
            SpyAssignment::GainControl => "Gain one control per month.",
            SpyAssignment::ImproveRelations => "Gain one relation per month.",
            SpyAssignment::DiscoverScandals => {
                "Search for scandals to use against the enemy player. Discovery is not guaranteed."
            },
            SpyAssignment::UndermineOpponents => {
                if campaign.economy.provinces[province].owner.is_some() {
                    "Reduce population happiness in this province. Success is not guaranteed."
                } else {
                    "Reduce an opponent's control or relation in this province. Success is not guaranteed."
                }
            },
        };
        let has_spy = campaign
            .espionage
            .missions
            .iter()
            .any(|mission| mission.owner == player && mission.province == province);
        let blocked = if has_spy {
            Some(
                "You already have a spy deployed here. Recall the spy before choosing another mission.",
            )
        } else if !assignment.eligible(&state) {
            Some("This mission is unavailable under the province's current government.")
        } else if distance.is_none() {
            Some("No usable political route from your controlled territory.")
        } else if campaign.actors[player].influence < deployment.unwrap_or(f64::INFINITY) {
            Some("Not enough Influence to deploy a spy.")
        } else {
            None
        };
        let available = blocked.is_none() && message.is_none();
        let rect = egui::Rect::from_min_size(
            grid.min
                + egui::vec2(
                    (index % 2) as f32 * (button_size.x + gap),
                    (index / 2) as f32 * (button_size.y + gap),
                ),
            button_size,
        );
        let response = ui.interact(
            rect,
            ui.id().with(("spy-action", province, assignment as u8)),
            if available {
                egui::Sense::click()
            } else {
                egui::Sense::hover()
            },
        );
        response
            .widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, available, name));
        campaign_widgets::paint_purchase_background(
            ui,
            rect,
            available,
            response.hovered(),
            response.is_pointer_button_down_on(),
            4.0 * scale,
            scale,
        );
        ui.painter().rect_stroke(
            rect,
            4.0 * scale,
            egui::Stroke::new(scale, province_panel::RULE),
            egui::StrokeKind::Inside,
        );
        let image_size = (rect.height() - 10.0 * scale).min(64.0 * scale).min(rect.width() * 0.30);
        let image_rect = egui::Rect::from_center_size(
            egui::pos2(rect.left() + 6.0 * scale + image_size * 0.5, rect.center().y),
            egui::Vec2::splat(image_size),
        );
        ui.scope(|ui| {
            if !available {
                ui.set_opacity(0.85);
            }
            campaign_widgets::paint_icon(ui, symbol, image_rect);
        });
        let ink = if available {
            province_panel::INK
        } else {
            campaign_widgets::UNAVAILABLE_PURCHASE_INK
        };
        let content_left = image_rect.right() + 6.0 * scale;
        let content_width = (rect.right() - content_left - 6.0 * scale).max(1.0);
        let title = ui.painter().layout(
            name.to_owned(),
            egui::FontId::proportional(16.0 * scale),
            ink,
            content_width,
        );
        let title_top = rect.top() + 9.0 * scale;
        let costs_top = rect.bottom() - 36.0 * scale;
        ui.painter().galley(egui::pos2(content_left, title_top), title, ink);
        let mut cost_left = content_left;
        for (symbol, value) in [
            (Icon::Influence, deployment.map_or("—".to_owned(), |cost| format!("{cost:.0}"))),
            (Icon::Coin, upkeep.map_or("—/mo".to_owned(), |cost| format!("{cost:.0}/mo"))),
        ] {
            let center = egui::pos2(cost_left, costs_top + 11.0 * scale);
            campaign_widgets::paint_icon(
                ui,
                symbol,
                egui::Rect::from_center_size(
                    center + egui::vec2(10.0 * scale, 0.0),
                    egui::Vec2::splat(20.0 * scale),
                ),
            );
            let cost =
                ui.painter().layout_no_wrap(value, egui::FontId::proportional(14.0 * scale), ink);
            let position = center + egui::vec2(23.0 * scale, -cost.size().y * 0.5);
            cost_left = position.x + cost.size().x + 12.0 * scale;
            ui.painter().galley(position, cost, ink);
        }
        if response.on_hover_text(description).clicked() && available {
            let result = campaign.espionage.deploy_assignment(
                player,
                province,
                &mut campaign.actors,
                &campaign.politics,
                &campaign.espionage_config,
                assignment,
                distance,
            );
            message = Some(
                result.map_or_else(|error| error.to_string(), |_| format!("Spy deployed: {name}.")),
            );
        }
    }
    message
}

/// The dark portrait badge uses the same treatment as the terrain and city labels.
fn political_distance_badge(
    ui: &mut egui::Ui,
    portrait: egui::Rect,
    distance: Option<usize>,
    scale: f32,
) {
    let multiplier = distance_multiplier(distance).ok();
    let value = multiplier.map_or_else(
        || "—".into(),
        |value| format!("×{}", format!("{value:.2}").trim_end_matches('0').trim_end_matches('.')),
    );
    let text = ui.painter().layout_no_wrap(
        format!("Political distance {value}"),
        egui::FontId::proportional(12.0 * scale),
        egui::Color32::WHITE,
    );
    let size = egui::vec2(text.size().x + 34.0 * scale, 27.0 * scale);
    let rect = egui::Rect::from_min_size(
        portrait.right_bottom() - size - egui::vec2(7.0, 7.0) * scale,
        size,
    );
    ui.painter().rect_filled(rect, 3.0 * scale, egui::Color32::from_black_alpha(180));
    campaign_widgets::paint_icon(
        ui,
        Icon::PoliticalDistance,
        egui::Rect::from_min_size(
            rect.min + egui::vec2(5.0, 4.0) * scale,
            egui::vec2(19.0, 19.0) * scale,
        ),
    );
    ui.painter().galley(
        rect.min + egui::vec2(28.0 * scale, (rect.height() - text.size().y) * 0.5),
        text,
        egui::Color32::WHITE,
    );
    ui.interact(rect, ui.id().with("political-distance-badge"), egui::Sense::hover()).on_hover_text(if multiplier.is_some() {
        "Political distance multiplies spy deployment Influence and monthly sestertii upkeep. The mission cards show the actual whole-number costs. A usable route is required."
    } else {
        "Political distance: this province has no usable route from your controlled territory. Spy deployment is unavailable."
    });
}
#[cfg(test)]
#[path = "../../tests/unit/ui_layout.rs"]
mod layout_tests;

#[cfg(test)]
#[path = "../../tests/unit/buildings_ui.rs"]
mod buildings_ui;

/// Apply integration's already-computed relation shift once while preserving other causes.
pub(super) fn integrate_province(
    campaign: &mut Campaign,
    province: usize,
    player: usize,
) -> Result<(), PoliticalError> {
    let shift = campaign.politics[province].take_ownership(player)?;
    for happiness in &mut campaign.economy.provinces[province].temporary_happiness {
        *happiness += shift;
    }
    Ok(())
}

/// Province access and bilateral peace controls, surfaced from owned-province context.
fn military_diplomacy(
    ui: &mut egui::Ui,
    campaign: &mut Campaign,
    province: usize,
    player: usize,
    owner: usize,
    colors: &[egui::Color32],
) -> Option<String> {
    let mut message = None;
    if owner == player {
        let scale = egui::TextStyle::Body.resolve(ui.style()).size / 14.0;
        ui.add_space(8.0 * scale);
        policy_widgets::section(ui, scale, "Military Access");
        egui::Frame::new()
            .inner_margin(egui::Margin::symmetric(
                (6.0 * scale).round() as i8,
                (10.0 * scale).round() as i8,
            ))
            .show(ui, |ui| {
                ui.add(egui::Label::new("Permit peaceful military entry to this province. If revoked, foreign units automatically march back to their home province.").wrap());
                ui.add_space(12.0 * scale);
                ui.horizontal_wrapped(|ui| {
                    ui.spacing_mut().item_spacing = egui::vec2(8.0, 8.0) * scale;
                    for guest in 0..campaign.actors.len() {
                        if guest == player {
                            continue;
                        }
                        let war = campaign.wars[player][guest];
                        let mut invited = campaign.province_access_granted(province, player, guest);
                        let color = colors.get(guest).copied().unwrap_or(province_panel::INK);
                        let label = egui::RichText::new(format!("Player {}", guest + 1)).color(color);
                        let changed = egui::Frame::new()
                            .fill(egui::Color32::from_rgb(247, 243, 232))
                            .stroke(egui::Stroke::new(0.8 * scale, province_panel::RULE))
                            .corner_radius(2.0 * scale)
                            .inner_margin(egui::Margin::symmetric(
                                (8.0 * scale).round() as i8,
                                (5.0 * scale).round() as i8,
                            ))
                            .show(ui, |ui| {
                                let response = ui.add_enabled_ui(!war, |ui| {
                                    ui.add_sized(
                                        [136.0 * scale, 26.0 * scale],
                                        egui::Checkbox::new(&mut invited, label),
                                    )
                                }).inner.on_hover_cursor(egui::CursorIcon::PointingHand)
                                  .on_disabled_hover_text("Access unavailable during war.");
                                response.changed()
                            })
                            .inner;
                        if changed {
                            match campaign.set_province_access(province, player, guest, invited) {
                                Ok(()) => {
                                    message = Some(format!(
                                        "Military access {} for Player {}.",
                                        if invited { "granted" } else { "revoked" },
                                        guest + 1
                                    ))
                                },
                                Err(error) => {
                                    let text = error.to_string();
                                    campaign.notifications.province_notice(
                                        player,
                                        province,
                                        campaign.economy.month,
                                        super::campaign_notifications::NoticeSeverity::Warning,
                                        super::campaign_notifications::NoticeKind::MilitaryMovementStopped,
                                        "Military access could not be changed",
                                        &text,
                                    );
                                    message = Some(text);
                                },
                            }
                        }
                    }
                });
            });
    } else if campaign.wars[player][owner] {
        stat(
            ui,
            Icon::Attack,
            &format!("War · P{}", owner + 1),
            "Military access invitations are suspended during war.",
        );
        let fighting = campaign.military.battles.iter().any(|battle| {
            let player = ForceOwner::Player(player);
            let owner = ForceOwner::Player(owner);
            (battle.attackers.plans.contains_key(&player)
                && battle.defenders.plans.contains_key(&owner))
                || (battle.attackers.plans.contains_key(&owner)
                    && battle.defenders.plans.contains_key(&player))
        });
        if ui.add_enabled(!fighting, egui::Button::new("Make peace"))
            .on_hover_text("End bilateral hostility. Existing ownership remains unchanged. Peaceful entry still requires the owner's invitation.")
            .on_disabled_hover_text("Resolve or retreat from your active battles against this player before making peace.").clicked() {
            campaign.wars[player][owner] = false; campaign.wars[owner][player] = false;
            message = Some(format!("Peace established with Player {}.", owner + 1));
        }
    } else {
        stat(
            ui,
            Icon::MilitaryAccess,
            if campaign.province_access_granted(province, owner, player) {
                "Access granted"
            } else {
                "Access closed"
            },
            "Peaceful military entry requires the owner's invitation. Peaceful stationing grants no occupation Control.",
        );
        if ui.button("Declare hostility").on_hover_text("Ends military invitations between both players. Entering hostile territory begins combat; victory captures an owned province immediately.").clicked() {
            campaign.invitations[player][owner] = false; campaign.invitations[owner][player] = false;
            campaign.declare_hostility(player, province);
            message = Some(format!("Hostility declared against Player {}.", owner + 1));
        }
    }
    message
}

/// Read construction identity to announce new wonders to other players.
pub(super) fn construction_underway(campaign: &Campaign, province: usize) -> Option<Icon> {
    match &campaign.economy.provinces[province].construction {
        Some(ConstructionProject::Building(project)) => Some(Icon::Building(project.building)),
        Some(ConstructionProject::Wonder(project)) => Some(Icon::Wonder(project.wonder_id)),
        _ => None,
    }
}

#[cfg(test)]
#[path = "../../tests/unit/campaign_ui.rs"]
mod tests;

/// List every event from this province belonging to the active player.
fn province_notifications(
    ui: &mut egui::Ui,
    campaign: &Campaign,
    province: usize,
    player: usize,
    scale: f32,
) -> Option<CampaignNotice> {
    let mut navigation = None;
    let mut empty = true;
    ui.spacing_mut().item_spacing.y = 8.0 * scale;
    for notice in campaign
        .notifications
        .history_for(player)
        .filter(|notice| notice.province == Some(province))
    {
        empty = false;
        ui.push_id(notice.id, |ui| {
            if campaign_notices::card(ui, notice, scale).clicked() {
                navigation = Some(notice.clone());
            }
        });
    }
    if empty {
        ui.add_space(8. * scale);
        ui.horizontal(|ui| {
            ui.add_space(12. * scale);
            ui.label("No notifications from this province yet.");
        });
    }
    navigation
}

/// History cards and transient notices use the same destinations.
pub(in crate::app) fn open_notification(
    view: &mut CampaignUi,
    notice: &CampaignNotice,
    campaign: &Campaign,
    detail: &mut ProvincePanelOpen,
    map: &mut MapView,
) {
    match notice.action {
        NoticeAction::OpenSenate => {
            view.open = Some(CampaignTab::Senate);
            view.politics_section = 0;
            view.close_province_selector();
            detail.0 = None;
            view.last_detail = None;
        },
        NoticeAction::OpenProvince(id) => {
            if matches!(
                notice.kind,
                super::campaign_notifications::NoticeKind::FoodShortage
                    | super::campaign_notifications::NoticeKind::TradeInterrupted
            ) {
                view.open = Some(
                    if notice.kind == super::campaign_notifications::NoticeKind::TradeInterrupted {
                        CampaignTab::Trade
                    } else {
                        CampaignTab::Governance
                    },
                );
                view.close_province_selector();
                detail.0 = None;
                view.last_detail = None;
                return;
            }
            view.open_province_section(id, campaign_notices::province_section(notice.kind));
            detail.0 = Some(MapDetail::Province(id));
            map.focus_province(id);
        },
        NoticeAction::FocusWonder(id) => {
            map.focus_wonder(id);
            if let Some(province) = notice.province {
                view.open_province_section(province, 2);
                detail.0 = Some(MapDetail::Province(province));
            }
        },
        NoticeAction::OpenScandal {
            scandal,
            province,
        } => {
            let npc = campaign
                .espionage
                .scandals
                .iter()
                .find(|evidence| evidence.id == scandal)
                .is_some_and(|evidence| {
                    matches!(
                        evidence.target,
                        crate::game::politics::espionage::ScandalTarget::Province(_)
                    )
                });
            view.open_evidence(if npc {
                province
            } else {
                None
            });
            if npc {
                detail.0 = province.map(MapDetail::Province);
            } else {
                detail.0 = None;
                view.last_detail = None;
                view.highlight_scandal = Some(scandal);
            }
            if let Some(province) = province {
                map.focus_province(province);
            }
        },
    }
}
