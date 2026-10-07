//! Campaign inspectors using the existing parchment, banner colors, and button treatment.

use super::campaign::Campaign;
use super::campaign_confirmation::ConfirmationAction;
use super::campaign_notifications::{CampaignNotice, NoticeAction};
use super::campaign_widgets::{directory_owner_marker, directory_province_name, icon, stat, Icon};
use super::spectator::InspectionHover;
use super::*;
use crate::game::economy::{ConstructionProject, EconomyWorld, TradeParty};
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
pub(super) const PANEL_WIDTH: f32 = 570.0;
pub(super) const PANEL_HEIGHT: f32 = 720.0;

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
    pub(super) overview_section: usize,
    pub(super) notice_filters: campaign_notices::NoticeFilters,
    pub(super) scandal_filters: campaign_scandals::ScandalFilters,
    province_notice_filters: campaign_notices::NoticeFilters,
    pub(super) politics_province_search: String,
    pub(super) politics_spy_search: String,
    pub(super) confirmation: Option<campaign_confirmation::PendingConfirmation>,
    confirmation_sound_pending: bool,
    pub(super) promotion: Option<super::rank_promotion::Promotion>,
}

impl CampaignUi {
    /// Preserve remembered province tabs while making room for the selected army.
    fn close_inspector(&mut self, detail: &mut ProvincePanelOpen) {
        self.close_province_selector();
        self.open = None;
        detail.0 = None;
        self.last_detail = None;
    }

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
        self.confirmation_sound_pending = false;
        self.confirmation.take().is_some()
    }

    fn open_confirmation(&mut self, pending: Option<campaign_confirmation::PendingConfirmation>) {
        self.confirmation_sound_pending = pending.is_some();
        self.confirmation = pending;
    }

    pub(super) fn take_confirmation_sound(&mut self) -> bool {
        std::mem::take(&mut self.confirmation_sound_pending)
    }

    /// Only the separate capital opens Rome; Latium uses the normal province inspector.
    fn select_map_detail(&mut self, selected: MapDetail, campaign: &Campaign) {
        self.close_province_selector();
        let province = match selected {
            MapDetail::Province(id) | MapDetail::City(id) => id,
        };
        if self.province.is_none() && campaign.economy.provinces[province].name != "Rome" {
            self.section = if matches!(selected, MapDetail::City(_)) {
                2
            } else {
                0
            };
        }
        self.province = Some(province);
        self.last_detail = Some(selected);
        self.open = Some(if campaign.economy.provinces[province].name == "Rome" {
            CampaignTab::Senate
        } else {
            CampaignTab::Province
        });
    }
    /// Reopen a province without replacing the remembered inspector tab.
    pub(crate) fn open_province(&mut self, province: usize) {
        self.open_province_section(province, self.section);
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
    let ids: Vec<_> = provinces
        .iter()
        .enumerate()
        .filter_map(|(id, p)| (p.name != "Rome").then_some(id))
        .collect();
    let options: Vec<_> = ids
        .iter()
        .map(|&id| {
            let candidate = &provinces[id];
            campaign_widgets::TitleSearchOption {
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
            }
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
        view.province = Some(ids[id]);
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
    menu_icon: Option<&egui::TextureHandle>,
) -> (bool, Option<egui::Rect>) {
    let tab = view.open.unwrap_or(CampaignTab::Province);
    let province = view.province.unwrap_or(0).min(provinces.len() - 1);
    let mut closed = false;
    let mut overview_portrait = None;
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
    let global_header = tab != CampaignTab::Province;
    let heading_height = if global_header {
        34.0
    } else {
        32.0
    } * scale;
    egui::Frame::new()
        .fill(color)
        .inner_margin(
            if global_header {
                8.0
            } else {
                6.0
            } * scale,
        )
        .corner_radius(egui::CornerRadius {
            nw: radius,
            ne: radius,
            sw: 0,
            se: 0,
        })
        .show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            ui.horizontal(|ui| {
                if let Some(menu_icon) = menu_icon {
                    ui.add(
                        egui::Image::new(menu_icon)
                            .fit_to_exact_size(egui::Vec2::splat(heading_height)),
                    );
                } else {
                    icon(ui, Icon::Eagle, 32.0 * scale);
                }
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
                        [available, heading_height],
                        egui::Label::new(
                            egui::RichText::new(title)
                                .size(
                                    if global_header {
                                        25.0
                                    } else {
                                        20.0
                                    } * scale,
                                )
                                .color(header_ink),
                        )
                        .halign(egui::Align::Center)
                        .truncate()
                        .show_tooltip_when_elided(false),
                    );
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
                    let owned = spectator::domestic_view(
                        ui.ctx(),
                        provinces[view.province.unwrap_or(province)].owner == Some(player),
                    );
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
                    overview_portrait = Some(landscape);
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
    (closed, overview_portrait)
}

/// Read political values each frame so selection and monthly changes stay current.
fn province_overview(
    ui: &mut egui::Ui,
    campaign: &Campaign,
    province: usize,
    player: usize,
    scale: f32,
) -> Option<ConfirmationAction> {
    let politics = campaign.politics.get(province);
    // Reserve the largest political card layout so navigating between
    // provinces never changes the population table's scale.
    let political_height = if politics.is_some() {
        110.0
    } else {
        0.0
    };
    let owned = spectator::domestic_view(
        ui.ctx(),
        campaign.economy.provinces[province].owner == Some(player),
    );
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
            action = overview_politics(
                ui,
                politics,
                player,
                scale,
                campaign.economy.provinces[province].mean_happiness(),
            );
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
    portrait: Option<egui::Rect>,
) -> Option<ConfirmationAction> {
    ui.set_max_height((inner_bottom - ui.next_widget_position().y).max(0.0));
    if let Some(portrait) = portrait {
        campaign_widgets::space_after_portrait(ui, portrait, scale, 0.0);
    }
    if campaign.economy.provinces[province].occupied {
        ui.small("Enemy occupation: recruitment, construction, policies and provincial trade are blocked. Defeat the occupying army to regain access.");
    }
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
    let portrait = campaign_widgets::portrait(ui, landscape, 76.0 * scale);
    campaign_widgets::space_after_portrait(ui, portrait, scale, 3.0);
    let remaining = (inner_bottom - ui.next_widget_position().y).max(0.0);
    let message = egui::ScrollArea::vertical()
        .id_salt(("province-buildings", province, player))
        .max_height(remaining)
        .min_scrolled_height(0.0)
        .auto_shrink([false, false])
        .show(ui, |ui| {
            let owned = spectator::domestic_view(
                ui.ctx(),
                campaign.economy.provinces[province].owner == Some(player),
            );
            let queue_action = campaign_economy::construction_queue_view(
                ui,
                &campaign.economy,
                province,
                owned,
                scale,
            );
            if let Some(action) = queue_action {
                view.open_confirmation(campaign_confirmation::PendingConfirmation::construction(
                    campaign, province, player, action,
                ));
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
    domestic_happiness: f64,
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
        } => {
            let value = politics.control(player);
            (value / 100.0, format!("{value:.0}/100"))
        },
        PoliticalState::Rome => unreachable!("Rome has no political meters"),
    };
    let domestic = matches!(politics.state, PoliticalState::Owned { owner } if owner == player);
    let control_tooltip = match &politics.state {
        PoliticalState::Vassal {
            ..
        } => {
            "The overlord's political hold over this vassal. At 100 Control, the overlord can take direct ownership; at 0, the province becomes independent."
        },
        PoliticalState::Owned {
            owner,
        } if *owner == player => "Your political hold over this province. Other players can gain Control and reduce your share. A rebellion without your troops present also reduces Control each month, in proportion to its surviving strength. Troops fighting here count as present. At zero Control from rebellion, the province becomes independent.",
        PoliticalState::Independent {
            ..
        }
        | PoliticalState::Owned {
            ..
        } => {
            "Your political power over this province. Above 50 Control, you can vassalize it; at 100, you can take direct ownership."
        },
        PoliticalState::Rome => unreachable!("Rome has no political meters"),
    };
    let relation_tooltip = if domestic {
        "Your relationship with this province's population, measured by its population-weighted average Happiness."
    } else {
        "How friendly this province is toward you."
    };
    let relation = if domestic {
        domestic_happiness
    } else {
        politics.relation(player)
    }
    .clamp(0.0, 100.0);
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
            control_tooltip,
            control as f32,
            &control_text,
            gold,
            scale,
            &actions,
            &mut requested,
        );
        political_card(
            &mut columns[1],
            Icon::Relation,
            "Relation",
            relation_tooltip,
            relation as f32 / 100.0,
            &format!("{relation:.0}/100"),
            relation_color,
            scale,
            &[],
            &mut requested,
        );
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
    tooltip: &str,
    progress: f32,
    value: &str,
    color: egui::Color32,
    scale: f32,
    actions: &[ConfirmationAction],
    requested: &mut Option<ConfirmationAction>,
) -> egui::Response {
    let width = ui.available_width();
    let mut action_hovered = false;
    let response = egui::Frame::new()
        .fill(province_panel::TABLE_STRIPE)
        .stroke(egui::Stroke::new(scale, province_panel::RULE))
        .corner_radius(4.0 * scale)
        .inner_margin(8.0 * scale)
        .show(ui, |ui| {
            ui.set_width((width - 2.0 * (8.0 * scale).round() - 2.0 * scale).max(1.0));
            if !actions.is_empty() {
                let side = 22.0 * scale;
                let gap = 3.0 * scale;
                let buttons: Vec<_> = actions
                    .iter()
                    .map(|&action| {
                        let (label, symbol) = match action {
                            ConfirmationAction::Vassalize => ("Vassalize", Icon::Vassalize),
                            ConfirmationAction::Integrate => ("Integrate", Icon::Integrate),
                            _ => unreachable!(),
                        };
                        let text = ui.painter().layout_no_wrap(
                            label.into(),
                            egui::FontId::proportional(10.0 * scale),
                            province_panel::INK,
                        );
                        (action, label, symbol, text)
                    })
                    .collect();
                let labeled_width = buttons.iter().map(|(_, _, _, text)| text.size().x + side).sum::<f32>()
                    + gap * (buttons.len() - 1) as f32;
                // Choose the presentation from the card width before handling hover.
                let show_labels = labeled_width <= ui.available_width();
                let (row, _) = ui.allocate_exact_size(
                    egui::vec2(ui.available_width(), side),
                    egui::Sense::hover(),
                );
                let mut right = row.right();
                for (action, label, symbol, text) in buttons.into_iter().rev() {
                    let button_width = if show_labels { text.size().x + side } else { side };
                    let rect = egui::Rect::from_min_size(
                        egui::pos2(right - button_width, row.top()),
                        egui::vec2(button_width, side),
                    );
                    right = rect.left() - gap;
                    let response = ui.interact(rect, ui.id().with(("political-action", label)), egui::Sense::click());
                    action_hovered |= response.contains_pointer();
                    let fill = if response.hovered() { province_panel::PAPER } else { province_panel::TABLE_STRIPE };
                    ui.painter().rect_filled(rect, 3.0 * scale, fill);
                    ui.painter().rect_stroke(rect, 3.0 * scale, egui::Stroke::new(scale, province_panel::RULE), egui::StrokeKind::Inside);
                    let icon_rect = if show_labels {
                        egui::Rect::from_min_size(rect.min + egui::vec2(3.0, 3.0) * scale, egui::vec2(16.0, 16.0) * scale)
                    } else {
                        egui::Rect::from_center_size(rect.center(), egui::vec2(16.0, 16.0) * scale)
                    };
                    campaign_widgets::paint_icon(ui, symbol, icon_rect);
                    if show_labels {
                        ui.painter().galley(rect.min + egui::vec2(20.0 * scale, (rect.height() - text.size().y) * 0.5), text, province_panel::INK);
                    }
                    response.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, label));
                    if response
                        .on_hover_cursor(egui::CursorIcon::PointingHand)
                        .inspection_hover_text(format!("{label} this province. Opens a confirmation before changing its status."))
                        .clicked()
                    {
                        *requested = Some(action);
                    }
                }
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
        .response;
    if action_hovered {
        response
    } else {
        response.inspection_hover_text(tooltip)
    }
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
    mut campaign: ResMut<Campaign>,
    mut view: ResMut<CampaignUi>,
    mut detail: ResMut<ProvincePanelOpen>,
    mut governance_open: ResMut<GovernancePanelOpen>,
    mut close_click: ResMut<MapPanelCloseClick>,
    practice: Res<LocalPractice>,
    mut map_view: ResMut<MapView>,
    mut menu_icons: Local<Option<[Option<egui::TextureHandle>; 4]>>,
    sound: Res<MenuAudio>,
    audio: Res<Audio>,
    assets: Res<AssetServer>,
    terminal: Res<TerminalPresentation>,
) {
    if !campaign.active || *state.get() != AppState::Map {
        return;
    }
    let Ok(ctx) = contexts.ctx_mut() else {
        return;
    };
    let mut player = practice.active_player;
    let player_colors: Vec<_> =
        practice.players.iter().map(|p| PLAYER_COLORS[p.color_index]).collect();
    // Battle and army cards need the palette even when the province inspector is closed.
    ctx.data_mut(|data| {
        data.insert_temp(egui::Id::new("campaign-player-colors"), player_colors.clone());
    });
    if detail.0 != view.last_detail {
        view.last_detail = detail.0;
        if let Some(selected) = detail.0 {
            campaign_military::cancel_orders(ctx);
            view.select_map_detail(selected, &campaign);
        }
    }
    let orders_opened = !terminal.spectating
        && prepare_map_orders(
            ctx,
            &campaign.military,
            &campaign.economy,
            player,
            &mut view,
            &mut detail,
            &mut governance_open,
        );
    if !orders_opened && (view.open.is_some() || governance_open.0) {
        campaign_military::collapse_army_panel(ctx);
    }
    if view.open == Some(CampaignTab::Province)
        && view.section != 3
        && view.province.is_some_and(|id| campaign.economy.provinces[id].name == "Rome")
    {
        view.open = Some(CampaignTab::Senate);
        view.close_province_selector();
    }
    if let Some(hit) = crate::map::take_army_click(ctx) {
        open_map_army(ctx, &mut view, &mut detail, hit.province, hit.owner, hit.movement, player);
        governance_open.0 = false;
    }
    if let Some(battle) = crate::map::take_battle_click(ctx) {
        open_map_battle(ctx, &mut view, &mut detail, battle, player);
        governance_open.0 = false;
        play_click(&sound, &audio, &assets);
    }
    if terminal.spectating {
        player = spectator::inspection_player(&campaign, view.province, player);
    }
    let Some(tab) = view.open else {
        return;
    };
    // Existing economic and political widgets accept mutable state. Spectators render
    // a disposable snapshot as well as disabling controls, so inspection cannot mutate authority.
    let mut inspection = terminal.spectating.then(|| campaign.clone());
    let campaign = inspection.as_mut().unwrap_or(&mut campaign);
    // Governance notices open the Overview card on its Governance tab.
    if tab == CampaignTab::Governance {
        governance_open.0 = true;
        view.overview_section = 0;
        view.open = None;
        return;
    }
    if player >= campaign.actors.len() {
        return;
    }
    let scale = viewport_ui_scale(ctx.content_rect().size());
    let screen = ctx.content_rect();
    let width = PANEL_WIDTH * scale;
    let width = width.min((screen.width() - 80.0 * scale).max(120.0));
    let height = (PANEL_HEIGHT * scale).min((screen.height() - 90.0 * scale).max(140.0));
    let size = egui::vec2(width, height);
    let rect = if terminal.spectating {
        // The hidden left menu frees this corner; match the existing bottom inset.
        let inset = screen.height() * MAP_CORNER_PANEL_BOTTOM_FRACTION;
        egui::Rect::from_min_size(screen.left_bottom() + egui::vec2(inset, -inset - height), size)
    } else {
        map_corner_panel_rect(screen, scale, size)
    };
    if view.province.is_none() {
        view.province =
            campaign.economy.provinces.iter().position(|p| p.owner == Some(player)).or(Some(0));
    }
    let mut province = view.province.unwrap_or(0).min(campaign.politics.len() - 1);
    let previous_province = province;
    campaign.pull_wallets();
    campaign.refresh_profiles();
    let header = match tab {
        CampaignTab::Senate => "Senate".to_owned(),
        CampaignTab::Governance => "Governance".to_owned(),
        CampaignTab::Military => "Military".to_owned(),
        CampaignTab::Trade => "Trade".to_owned(),
        CampaignTab::Province => campaign.economy.provinces[province].name.clone(),
    };
    let mut closed = false;
    let mut action = None;
    let mut navigation = None;
    let mut army_province = None;
    let mut requested_promotion = None;
    let old_notice = view.notice.clone();
    let confirmation_was_open = view.confirmation_open();
    let previous_section = view.section;
    let menu_icon = match tab {
        CampaignTab::Governance => Some(0),
        CampaignTab::Military => Some(1),
        CampaignTab::Trade => Some(2),
        CampaignTab::Senate => Some(3),
        CampaignTab::Province => None,
    }
    .map(|index| {
        let textures = menu_icons.get_or_insert_with(|| std::array::from_fn(|_| None));
        map_menu_icon(ctx, textures, index)
    });
    view.cycle_province_tabs(ctx);
    let previous_rank = campaign.actors[player].rank;
    let owner_color = campaign.economy.provinces[province]
        .owner
        .and_then(|owner| practice.players.get(owner))
        .map(|owner| PLAYER_COLORS[owner.color_index])
        .unwrap_or(province_panel::NEUTRAL);
    ctx.data_mut(|d| {
        d.insert_temp(egui::Id::new("campaign-active-player-color"), player_colors[player]);
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
                let (header_closed, overview_portrait) = panel_header(
                    ui,
                    &header,
                    scale,
                    &campaign.economy.provinces,
                    player,
                    &mut view,
                    menu_icon.as_ref(),
                );
                closed = header_closed;
                province = view.province.unwrap_or(province);
                if terminal.spectating {
                    player = spectator::inspection_player(campaign, Some(province), player);
                }
                if tab == CampaignTab::Province
                    && view.section != 3
                    && campaign.economy.provinces[province].name == "Rome"
                {
                    view.open = Some(CampaignTab::Senate);
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
                        if terminal.spectating
                            && !(tab == CampaignTab::Province && matches!(view.section, 3 | 5))
                        {
                            ui.visuals_mut().disabled_alpha = 1.0;
                            ui.disable();
                        }
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
                                campaign.economy.players[player].influence,
                                &colors,
                                scale,
                                &mut requested_promotion,
                            );
                            return;
                        }
                        if tab == CampaignTab::Province
                            && view.section == 1
                            && spectator::domestic_view(
                                ui.ctx(),
                                campaign.economy.provinces[province].owner == Some(player),
                            )
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
                                campaign,
                                province,
                                player,
                                scale,
                                overview_portrait,
                            );
                            if let Some(action) = requested {
                                view.open_confirmation(
                                    campaign_confirmation::PendingConfirmation::new(
                                        campaign, province, player, action,
                                    ),
                                );
                            }
                            return;
                        }
                        if tab == CampaignTab::Province && view.section == 2 {
                            if let Some(message) = province_buildings_body(
                                ui,
                                rect.bottom() - 12.0 * scale,
                                campaign,
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
                            ui.set_max_height(
                                (rect.bottom() - 12.0 * scale - ui.next_widget_position().y)
                                    .max(0.0),
                            );
                            navigation = province_notifications(
                                ui,
                                campaign,
                                province,
                                player,
                                &player_colors,
                                &mut view.province_notice_filters,
                                scale,
                            );
                            return;
                        }
                        if tab == CampaignTab::Province && view.section == 3 {
                            ui.set_max_height(
                                (rect.bottom() - 12. * scale - ui.next_widget_position().y).max(0.),
                            );
                            ui.set_clip_rect(ui.clip_rect().intersect(egui::Rect::from_min_max(
                                rect.min,
                                egui::pos2(rect.right(), rect.bottom() - 12. * scale),
                            )));
                            let access = campaign.access_snapshot();
                            let military = if terminal.spectating {
                                campaign.military.clone()
                            } else {
                                campaign.military_view(player, true)
                            };
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
                            ("campaign_body", tab as usize, province, view.section),
                            |ui| {
                                if tab == CampaignTab::Trade {
                                    if let Some(message) =
                                        campaign_trade::show(ui, campaign, player, scale)
                                    {
                                        view.notice = message;
                                    }
                                } else if tab == CampaignTab::Province && view.section == 1 {
                                    if let Some(message) = campaign_trade::show_province(
                                        ui, campaign, province, player, scale,
                                    ) {
                                        view.notice = message;
                                    }
                                } else if tab == CampaignTab::Senate {
                                    if let Some(scandal) = view.highlight_scandal.and_then(|id| {
                                        campaign.espionage.scandals.iter().find(|s| {
                                            s.id == id
                                                && s.holder == player
                                                && s.is_current(
                                                    campaign
                                                        .economy
                                                        .month
                                                        .max(campaign.senate.month),
                                                )
                                        })
                                    }) {
                                        policy_widgets::section(ui, scale, "SELECTED SCANDAL");
                                        campaign_scandals::headers(ui, scale);
                                        let response = campaign_scandals::row(
                                            ui,
                                            campaign,
                                            scandal,
                                            &player_colors,
                                            0,
                                            scale,
                                        );
                                        if let Some(command) = response.command {
                                            view.notice = campaign_scandals::apply_command(
                                                campaign, player, command,
                                            );
                                            view.highlight_scandal = None;
                                        }
                                        if ui.small_button("Dismiss selection").clicked() {
                                            view.highlight_scandal = None;
                                        }
                                    } else {
                                        view.highlight_scandal = None;
                                    }
                                    let colors: Vec<_> = practice
                                        .players
                                        .iter()
                                        .map(|p| PLAYER_COLORS[p.color_index])
                                        .collect();
                                    if campaign.defeated.get(player).copied().unwrap_or(false) {
                                        ui.heading("Campaign lost · no provinces remain");
                                    }
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
                                            if let Some(action) = province_overview(
                                                ui, campaign, province, player, scale,
                                            ) {
                                                view.open_confirmation(
                                                    campaign_confirmation::PendingConfirmation::new(
                                                        campaign, province, player, action,
                                                    ),
                                                );
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
                                            if let Some(message) = diplomacy(
                                                ui,
                                                campaign,
                                                province,
                                                player,
                                                &player_colors,
                                                &mut view,
                                            ) {
                                                view.notice = message;
                                            }
                                        },
                                        _ => {
                                            navigation = province_notifications(
                                                ui,
                                                campaign,
                                                province,
                                                player,
                                                &player_colors,
                                                &mut view.province_notice_filters,
                                                scale,
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
        view.promotion = Some(super::rank_promotion::Promotion::new(
            super::rank_promotion::Rank::Political(rank),
            player,
            ctx.input(|input| input.time),
        ));
        if sound.mode != AudioMode::Mute && sound.volume > 0.001 {
            let decibels = -8.0 + 20.0 * sound.volume.clamp(0.001, 1.0).log10();
            audio.play(assets.load("audio/victory.ogg")).with_volume(decibels);
        }
    }
    if let Some(rank) = requested_promotion {
        match campaign.promote_military(player, rank) {
            Ok(()) => {
                view.promotion = Some(super::rank_promotion::Promotion::new(
                    super::rank_promotion::Rank::Military(rank),
                    player,
                    ctx.input(|input| input.time),
                ));
                if sound.mode != AudioMode::Mute && sound.volume > 0.001 {
                    let decibels = -8.0 + 20.0 * sound.volume.clamp(0.001, 1.0).log10();
                    audio.play(assets.load("audio/victory.ogg")).with_volume(decibels);
                }
            },
            Err(message) => view.notice = message,
        }
    }
    if let Some(action) = action {
        if let Some(kind) = dispatch_military_action(&mut view, campaign, province, player, action)
        {
            play_recruitment(kind, &sound, &audio, &assets);
        }
    }
    if let Some(province) = army_province {
        open_military_province(ctx, &mut view, &mut detail, &mut map_view, province, player);
        close_click.0 = true;
        play_click(&sound, &audio, &assets);
    }
    if campaign_military::army_details_open(ctx) {
        view.close_inspector(&mut detail);
        governance_open.0 = false;
    }
    if let Some(notice) = navigation {
        if notice.kind == super::campaign_notifications::NoticeKind::TradeInterrupted {
            campaign_trade::open_routes(ctx, player);
        }
        open_notification(&mut view, &notice, campaign, &mut detail, &mut map_view);
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
        || (!confirmation_was_open && view.confirmation_open())
    {
        play_click(&sound, &audio, &assets);
    }
}

/// Right-click orders replace inspectors and use the visible province as their origin.
fn prepare_map_orders(
    ctx: &egui::Context,
    world: &MilitaryWorld,
    economy: &EconomyWorld,
    player: usize,
    view: &mut CampaignUi,
    detail: &mut ProvincePanelOpen,
    governance_open: &mut GovernancePanelOpen,
) -> bool {
    let province = view
        .province
        .filter(|_| view.open == Some(CampaignTab::Province) || view.last_detail.is_some());
    if !campaign_military::prepare_orders(ctx, world, economy, player, province) {
        return false;
    }
    view.close_inspector(detail);
    view.dismiss_confirmation();
    governance_open.0 = false;
    view.notice.clear();
    true
}

/// Destructive commands wait for an explicit Yes; all other orders remain immediate.
pub(super) fn dispatch_military_action(
    view: &mut CampaignUi,
    campaign: &mut Campaign,
    province: usize,
    player: usize,
    action: campaign_military::MilitaryUiAction,
) -> Option<UnitType> {
    use campaign_military::MilitaryUiAction::*;
    let recruited = match &action {
        Recruit(kind) => Some(*kind),
        _ => None,
    };
    let pending = match action {
        CancelRecruitment => campaign_confirmation::ConfirmationAction::CancelRecruitment,
        CancelQueuedRecruitment(index) => {
            campaign_confirmation::ConfirmationAction::CancelQueuedRecruitment(index)
        },
        Disband(id) => campaign_confirmation::ConfirmationAction::Disband(id),
        DisbandArmy => campaign_confirmation::ConfirmationAction::DisbandArmy,
        action => {
            view.notice = apply_military_action(campaign, province, player, action);
            return if view.notice.is_empty() {
                recruited
            } else {
                None
            };
        },
    };
    view.open_confirmation(campaign_confirmation::PendingConfirmation::new(
        campaign, province, player, pending,
    ));
    None
}

/// Audibly acknowledge accepted recruitment orders, grouped by recognizable unit family.
fn play_recruitment(kind: UnitType, sound: &MenuAudio, audio: &Audio, assets: &AssetServer) {
    if sound.mode == AudioMode::Mute || sound.volume <= 0.001 {
        return;
    }
    let name = match kind {
        UnitType::LightInfantry | UnitType::HeavyInfantry => "recruit-infantry",
        UnitType::Archers => "recruit-archers",
        UnitType::LightCavalry
        | UnitType::HeavyCavalry
        | UnitType::HorseArchers
        | UnitType::WarChariots => "recruit-horses",
        UnitType::WarCamels => "recruit-camels",
        UnitType::WarElephants => "recruit-elephants",
        UnitType::Ballista | UnitType::Catapult => "recruit-siege",
    };
    let decibels = -8.0 + 20.0 * sound.volume.clamp(0.001, 1.0).log10();
    audio.play(assets.load(format!("audio/{name}.ogg"))).with_volume(decibels);
}

/// A battle replaces both inspectors before either has a chance to draw this frame.
fn open_map_battle(
    ctx: &egui::Context,
    view: &mut CampaignUi,
    detail: &mut ProvincePanelOpen,
    battle: u64,
    player: usize,
) {
    view.close_inspector(detail);
    view.notice.clear();
    campaign_military::open_battle_panel(ctx, battle, player);
}

/// Map figures open an owned army window or the foreign province's military section.
fn open_map_army(
    ctx: &egui::Context,
    view: &mut CampaignUi,
    detail: &mut ProvincePanelOpen,
    province: usize,
    owner: ForceOwner,
    movement: Option<u64>,
    player: usize,
) {
    if spectator::read_only(ctx) {
        campaign_military::toggle_inspection_army_panel(ctx, province, player, owner, movement);
        view.close_inspector(detail);
    } else if owner == ForceOwner::Player(player) {
        campaign_military::toggle_map_army_panel(ctx, province, player, movement);
        view.close_inspector(detail);
    } else {
        campaign_military::dismiss_army_panel(ctx);
        view.open_province_section(province, 3);
        view.notice.clear();
        campaign_military::open_army_tab(ctx, province, player);
        detail.0 = Some(MapDetail::Province(province));
    }
}

/// The army window has its own render pass so province navigation cannot hide it.
pub(in crate::app) fn draw_army_panel(
    mut contexts: EguiContexts,
    state: Res<State<AppState>>,
    practice: Res<LocalPractice>,
    mut campaign: ResMut<Campaign>,
    mut view: ResMut<CampaignUi>,
    mut detail: ResMut<ProvincePanelOpen>,
    mut map_view: ResMut<MapView>,
    mut close_click: ResMut<MapPanelCloseClick>,
    terminal: Res<TerminalPresentation>,
    sound: Res<MenuAudio>,
    audio: Res<Audio>,
    assets: Res<AssetServer>,
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
    let military = if terminal.spectating {
        campaign.military.clone()
    } else {
        campaign.military_view(player, true)
    };
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
        if !terminal.spectating {
            if let Some(kind) =
                dispatch_military_action(&mut view, &mut campaign, province, player, action)
            {
                play_recruitment(kind, &sound, &audio, &assets);
            }
        }
    }
    if let Some(province) = campaign_military::take_army_province_click(ctx) {
        open_military_province(ctx, &mut view, &mut detail, &mut map_view, province, player);
        close_click.0 = true;
        play_click(&sound, &audio, &assets);
    }
    if terminal.spectating {
        campaign_military::draw_battle_panel(ctx, &military, &campaign.economy, player);
        return;
    }
    if let Some((province, action)) = campaign_military::draw_orders_menu(
        ctx,
        &military,
        &campaign.economy,
        &campaign.graph,
        player,
        |owner, id| match owner {
            ForceOwner::Player(p) => access[p][id],
            _ => MilitaryAccess::Blocked,
        },
        |id| {
            matches!(campaign.politics[id].state, PoliticalState::Independent { .. })
                && !campaign.npc_wars[player][id]
                && !campaign.military.provinces[id].slave_rebellion
        },
        &view.notice,
    ) {
        view.notice = apply_military_action(&mut campaign, province, player, action);
        if view.notice.is_empty() {
            if let Some(order) = campaign.military.movements.last() {
                campaign_military::open_army_panel(ctx, order.origin, player, Some(order.id));
                campaign_military::collapse_army_panel(ctx);
            }
            play_click(&sound, &audio, &assets);
        }
    }
    if let Some((province, action)) =
        campaign_military::draw_battle_panel(ctx, &campaign.military, &campaign.economy, player)
    {
        view.notice = apply_military_action(&mut campaign, province, player, action);
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
pub(in crate::app) fn apply_military_action(
    campaign: &mut Campaign,
    province: usize,
    player: usize,
    action: campaign_military::MilitaryUiAction,
) -> String {
    use campaign_military::MilitaryUiAction::*;
    let disbanding_army = matches!(&action, DisbandArmy);
    let order_kind = match &action {
        Attack {
            ..
        } => ArmyOrderKind::Attack,
        Pressure {
            ..
        } => ArmyOrderKind::Pressure,
        _ => ArmyOrderKind::Move,
    };
    let result: Result<(), String> = match action {
        Recruit(kind) => {
            let p = &mut campaign.economy.provinces[province];
            campaign
                .military
                .recruit(
                    province,
                    player,
                    kind,
                    p.can_administer(player),
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
        MergeArmy => campaign
            .military
            .merge_army(province, ForceOwner::Player(player))
            .map(|_| ())
            .map_err(|e| e.to_string()),
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
        }
        | Attack {
            destination,
            units,
            route,
            plan,
        }
        | Pressure {
            destination,
            units,
            route,
            plan,
        } => campaign
            .order_army(province, player, destination, &units, &route, plan, order_kind)
            .map(|_| ()),
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
    if result.is_ok() && disbanding_army {
        let name = &campaign.economy.provinces[province].name;
        campaign.notifications.province_notice(
            player,
            province,
            campaign.economy.month,
            super::campaign_notifications::NoticeSeverity::Info,
            super::campaign_notifications::NoticeKind::ArmyDisbanded,
            "Army disbanded",
            format!("Your army in {name} disbanded. Survivors rejoined the population."),
        );
    }
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
        campaign_widgets::space_after_portrait(ui, portrait, scale, 0.0);
        ui.label("Rome permits no provincial actions. Open the Senate to manage politics or declare a march on the capital.");
        return None;
    }
    if matches!(state, PoliticalState::Owned { owner } if owner == player) {
        campaign_widgets::space_after_portrait(ui, portrait, scale, 3.0);
        return military_diplomacy(ui, campaign, province, player, player, colors);
    }
    let distance = campaign.distance(player, province);
    campaign_widgets::territorial_distance_badge(
        ui,
        portrait,
        distance,
        scale,
        "Territorial distance multiplies the spy costs and upkeep.",
    );
    campaign_widgets::space_after_portrait(ui, portrait, scale, 0.0);
    if let Some(action) = overview_politics(
        ui,
        &campaign.politics[province],
        player,
        scale,
        campaign.economy.provinces[province].mean_happiness(),
    ) {
        view.open_confirmation(campaign_confirmation::PendingConfirmation::new(
            campaign, province, player, action,
        ));
    }
    ui.add_space(6.0 * scale);
    if matches!(state, PoliticalState::Vassal { overlord, .. } if overlord == player) {
        policy_widgets::section(ui, scale, "VASSAL GOVERNMENT");
        ui.label("This province is your vassal. Relations and a stationed garrison build its Control over time.");
        ui.add_space(6.0 * scale);
        return political_maneuvers(ui, campaign, province, player, scale, false);
    }
    let mut message = spy_network(ui, campaign, province, player, scale);
    if let Some(action_message) =
        political_maneuvers(ui, campaign, province, player, scale, message.is_some())
    {
        message = Some(action_message);
    }
    let evidence: Vec<_> = campaign
        .espionage
        .scandals
        .iter()
        .filter(|evidence| {
            evidence.holder == player
                && evidence.province == Some(province)
                && evidence.is_current(campaign.economy.month.max(campaign.senate.month))
        })
        .collect();
    if !evidence.is_empty() {
        campaign_scandals::headers(ui, scale);
        let mut command = None;
        let mut selected = None;
        for (index, scandal) in evidence.into_iter().enumerate() {
            let response = campaign_scandals::row(ui, campaign, scandal, colors, index, scale);
            command = response.command.or(command);
            selected = response.selected.or(selected);
            ui.add_space(3.0 * scale);
        }
        if let Some(command) = command {
            message = Some(campaign_scandals::apply_command(campaign, player, command));
        }
        if let Some(id) = selected {
            if campaign.espionage.scandals.iter().any(|scandal| {
                scandal.id == id
                    && matches!(
                        scandal.target,
                        crate::game::politics::espionage::ScandalTarget::Player(_)
                    )
            }) {
                view.open_evidence(None);
                view.highlight_scandal = Some(id);
            }
        }
    }
    message
}

const SPY_ACTIONS: [(SpyAssignment, &str, Icon); 6] = [
    (SpyAssignment::GainControl, "Build control", Icon::SpyBuildControl),
    (SpyAssignment::ImproveRelations, "Improve relations", Icon::SpyImproveRelations),
    (SpyAssignment::DiscoverScandals, "Uncover scandals", Icon::SpyUncoverScandals),
    (SpyAssignment::UndermineOpponents, "Undermine foes", Icon::SpyUndermineOpponents),
    (SpyAssignment::SupportRevolt, "Support revolt", Icon::SpySupportRevolt),
    (SpyAssignment::DiscreditRivals, "Discredit rivals", Icon::SpyDiscreditRivals),
];

/// A single political reading is shared by the directory and its summary.
fn politics_reading(
    politics: &ProvincePolitics,
    player: usize,
    domestic_happiness: f64,
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
            politics.control(player),
            Some(if *owner == player {
                domestic_happiness
            } else {
                politics.relation(player)
            }),
        ),
    }
}

fn directory_search(ui: &mut egui::Ui, search: &mut String, scale: f32) {
    egui::Frame::new()
        .fill(province_panel::TABLE_STRIPE)
        .stroke(egui::Stroke::new(scale, province_panel::RULE))
        .corner_radius(4.0 * scale)
        .inner_margin(egui::Margin::symmetric((8.0 * scale) as i8, 0))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.visuals_mut().text_cursor.stroke.color = egui::Color32::BLACK;
            ui.add_sized(
                [ui.available_width(), 33.0 * scale],
                egui::TextEdit::singleline(search)
                    .frame(egui::Frame::NONE)
                    .hint_text("Search provinces…")
                    .vertical_align(egui::Align::Center),
            );
        });
}

fn directory_section(ui: &mut egui::Ui, scale: f32, title: &str, first: bool) {
    if first {
        // The section fill starts 3 px inside its rect. Account for that inset
        // so its visible gap matches the gap above the search field.
        let spacing = ui.spacing().item_spacing.y;
        ui.spacing_mut().item_spacing.y = (spacing - 3.0 * scale).max(0.0);
        policy_widgets::section(ui, scale, title);
        ui.spacing_mut().item_spacing.y = spacing;
    } else {
        policy_widgets::section(ui, scale, title);
    }
}

fn directory_meter(
    ui: &mut egui::Ui,
    symbol: Icon,
    value: f64,
    color: egui::Color32,
    width: f32,
    scale: f32,
) {
    let (rect, _) = ui.allocate_exact_size(egui::vec2(width, 24.0 * scale), egui::Sense::hover());
    ui.painter().rect_filled(rect, 4.0 * scale, egui::Color32::from_rgb(224, 215, 195));
    let fill = egui::Rect::from_min_size(
        rect.min,
        egui::vec2(rect.width() * (value / 100.0).clamp(0.0, 1.0) as f32, rect.height()),
    );
    if fill.width() > 0.0 {
        ui.painter().rect_filled(fill, 4.0 * scale, color);
    }
    campaign_widgets::paint_icon(
        ui,
        symbol,
        egui::Rect::from_center_size(
            egui::pos2(rect.left() + 13.0 * scale, rect.center().y),
            egui::Vec2::splat(18.0 * scale),
        ),
    );
    ui.painter().text(
        egui::pos2(rect.left() + 27.0 * scale, rect.center().y),
        egui::Align2::LEFT_CENTER,
        format!("{value:.0}%"),
        egui::FontId::proportional(11.0 * scale),
        province_panel::INK,
    );
}

fn directory_row(
    ui: &mut egui::Ui,
    row: usize,
    scale: f32,
    contents: impl FnOnce(&mut egui::Ui),
) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(
        egui::vec2(ui.available_width(), campaign_widgets::DIRECTORY_ROW_HEIGHT * scale),
        egui::Sense::click(),
    );
    campaign_widgets::paint_directory_row(ui, rect, &response, row, scale);
    ui.scope_builder(
        egui::UiBuilder::new()
            .max_rect(rect.shrink(7.0 * scale))
            .layout(egui::Layout::left_to_right(egui::Align::Center)),
        |ui| {
            ui.spacing_mut().item_spacing.x = 8.0 * scale;
            contents(ui);
        },
    );
    response.on_hover_cursor(egui::CursorIcon::PointingHand)
}

fn directory_empty_group(ui: &mut egui::Ui, scale: f32) {
    ui.horizontal(|ui| {
        ui.add_space(12.0 * scale);
        ui.small("No matching provinces.");
    });
}

pub(super) fn politics_provinces(
    ui: &mut egui::Ui,
    campaign: &Campaign,
    player: usize,
    colors: &[egui::Color32],
    search: &mut String,
    scale: f32,
) -> Option<usize> {
    let provinces = &campaign.economy.provinces;
    directory_search(ui, search, scale);
    let query = search.trim().to_lowercase();
    let mut ids: Vec<_> = (0..provinces.len())
        .filter(|&id| {
            provinces[id].name != "Rome" && provinces[id].name.to_lowercase().contains(&query)
        })
        .collect();
    ids.sort_by_key(|&id| provinces[id].name.to_lowercase());
    if ids.is_empty() {
        ui.label("No provinces match your search.");
    }
    let mut target = None;
    for (group, heading) in ["OWN PROVINCES", "VASSALS", "OTHER PROVINCES"].into_iter().enumerate()
    {
        directory_section(ui, scale, heading, group == 0);
        let mut row = 0;
        for &id in &ids {
            let province = &provinces[id];
            let politics = &campaign.politics[id];
            let belongs = match &politics.state {
                PoliticalState::Owned {
                    owner,
                } if *owner == player => 0,
                PoliticalState::Vassal {
                    overlord,
                    ..
                } if *overlord == player => 1,
                _ => 2,
            };
            if belongs != group {
                continue;
            }
            let (_, control, relation) =
                politics_reading(politics, player, province.mean_happiness());
            let owner_color = province
                .owner
                .and_then(|owner| colors.get(owner).copied())
                .unwrap_or(province_panel::NEUTRAL);
            let response = directory_row(ui, row, scale, |ui| {
                directory_owner_marker(
                    ui,
                    owner_color,
                    campaign_widgets::DIRECTORY_ROW_HEIGHT * scale,
                    scale,
                );
                let meter_width = (ui.available_width() * 0.27).max(70.0 * scale);
                let name_width =
                    (ui.available_width() - meter_width * 2.0 - 22.0 * scale).max(80.0 * scale);
                directory_province_name(ui, &province.name, name_width, scale);
                directory_meter(
                    ui,
                    Icon::Control,
                    control,
                    egui::Color32::from_rgb(166, 121, 67),
                    meter_width,
                    scale,
                );
                if let Some(relation) = relation {
                    let color = if relation < 40.0 {
                        egui::Color32::from_rgb(166, 83, 66)
                    } else if relation < 60.0 {
                        egui::Color32::from_rgb(179, 137, 74)
                    } else {
                        egui::Color32::from_rgb(103, 149, 118)
                    };
                    directory_meter(ui, Icon::Relation, relation, color, meter_width, scale);
                } else {
                    ui.add_sized([meter_width, 34.0 * scale], egui::Label::new("—"));
                }
            });
            if response.clicked() {
                target = Some(id);
            }
            ui.add_space(3.0 * scale);
            row += 1;
        }
        if row == 0 {
            directory_empty_group(ui, scale);
        }
        ui.add_space(8.0 * scale);
    }
    target
}

/// Search eligible foreign spy targets and open provincial diplomacy from any row.
pub(super) fn politics_spies(
    ui: &mut egui::Ui,
    campaign: &mut Campaign,
    player: usize,
    colors: &[egui::Color32],
    search: &mut String,
    scale: f32,
) -> (Option<usize>, Option<String>) {
    directory_search(ui, search, scale);
    let query = search.trim().to_lowercase();
    let mut ids: Vec<_> = (0..campaign.politics.len())
        .filter(|&id| {
            let state = &campaign.politics[id].state;
            !matches!(state, PoliticalState::Rome)
                && !matches!(state, PoliticalState::Owned { owner } if *owner == player)
                && !matches!(state, PoliticalState::Vassal { overlord, .. } if *overlord == player)
                && campaign.economy.provinces[id].name.to_lowercase().contains(&query)
        })
        .collect();
    ids.sort_by_key(|&id| campaign.economy.provinces[id].name.to_lowercase());
    if ids.is_empty() {
        ui.label("No provinces match your search.");
    }
    let mut target = None;
    let mut message = None;
    for (active_group, heading) in
        [(true, "ACTIVE NETWORKS"), (false, "PROVINCES WITHOUT NETWORKS")]
    {
        directory_section(ui, scale, heading, active_group);
        let mut row = 0;
        for &id in &ids {
            let mission = campaign
                .espionage
                .missions
                .iter()
                .find(|mission| mission.owner == player && mission.province == id);
            if mission.is_some() != active_group {
                continue;
            }
            let province = &campaign.economy.provinces[id];
            let owner_color = province
                .owner
                .and_then(|owner| colors.get(owner).copied())
                .unwrap_or(province_panel::NEUTRAL);
            let mut selected_action = None;
            let mut over_action = false;
            let mut over_disabled_action = false;
            let response = directory_row(ui, row, scale, |ui| {
                directory_owner_marker(
                    ui,
                    owner_color,
                    campaign_widgets::DIRECTORY_ROW_HEIGHT * scale,
                    scale,
                );
                let controls_width = mission.map_or(205.0, |mission| {
                    if mission.recall_month.is_some() {
                        233.0
                    } else {
                        163.0
                    }
                }) * scale;
                let name_width = (ui.available_width() * 0.35)
                    .min((ui.available_width() - controls_width).max(80.0 * scale));
                directory_province_name(ui, &province.name, name_width, scale);
                if let Some(mission) = mission {
                    let (assignment, symbol) = SPY_ACTIONS
                        .iter()
                        .find(|(assignment, _, _)| *assignment == mission.assignment)
                        .map_or(("Active network", Icon::Spy), |(_, label, icon)| (*label, *icon));
                    let (icon_rect, _) = ui
                        .allocate_exact_size(egui::Vec2::splat(20.0 * scale), egui::Sense::hover());
                    campaign_widgets::paint_icon(ui, symbol, icon_rect);
                    let recall_width = if mission.recall_month.is_some() {
                        98.0
                    } else {
                        28.0
                    } * scale;
                    let label_width =
                        (ui.available_width() - recall_width - 28.0 * scale - 11.0 * scale)
                            .max(1.0);
                    ui.allocate_ui_with_layout(
                        egui::vec2(label_width, 34.0 * scale),
                        egui::Layout::left_to_right(egui::Align::Center),
                        |ui| ui.add(egui::Label::new(assignment).truncate()),
                    )
                    .inner
                    .inspection_hover_text(assignment);
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.spacing_mut().item_spacing.x = 3.0 * scale;
                        for action in [SpyDirectoryAction::Flee, SpyDirectoryAction::Recall] {
                            let response = match action {
                                SpyDirectoryAction::Flee => spy_directory_icon_button(
                                    ui, Icon::SpyFlee, "Flee", true, scale,
                                ),
                                _ => {
                                    if let Some(due) = mission.recall_month {
                                        let months = due.saturating_sub(campaign.economy.month);
                                        spy_recall_button(ui, scale, Some(months))
                                            .on_disabled_hover_text(format!("Spy recalled · returns in {months} {}. Normal upkeep and detection continue until then.", if months == 1 { "month" } else { "months" }))
                                    } else {
                                        spy_directory_icon_button(ui, Icon::Cancel, "Recall", true, scale)
                                    }
                                },
                            };
                            over_action |= ui.input(|input| {
                                input
                                    .pointer
                                    .hover_pos()
                                    .is_some_and(|pos| response.rect.contains(pos))
                            });
                            over_disabled_action |= !response.enabled() && response.contains_pointer();
                            if response.clicked() {
                                selected_action = Some(action);
                            }
                        }
                    });
                } else {
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.spacing_mut().item_spacing.x = 3.0 * scale;
                        for &(assignment, label, symbol) in SPY_ACTIONS.iter().rev() {
                            let cost = campaign
                                .espionage_config
                                .deployment_cost_at(assignment, campaign.distance(player, id))
                                .ok();
                            let blocked = if !assignment.eligible(&campaign.politics[id].state) {
                                Some("Unavailable under this province's government")
                            } else if campaign.actors[player].influence
                                < cost.unwrap_or(f64::INFINITY)
                            {
                                Some("Not enough Influence to deploy a spy")
                            } else {
                                None
                            };
                            let tooltip = blocked.map_or_else(
                                || label.to_owned(),
                                |reason| format!("{label}: {reason}"),
                            );
                            let response = spy_directory_icon_button(
                                ui,
                                symbol,
                                &tooltip,
                                blocked.is_none(),
                                scale,
                            );
                            over_action |= ui.input(|input| {
                                input
                                    .pointer
                                    .hover_pos()
                                    .is_some_and(|pos| response.rect.contains(pos))
                            });
                            over_disabled_action |=
                                !response.enabled() && response.contains_pointer();
                            if response.clicked() {
                                selected_action = Some(SpyDirectoryAction::Deploy(assignment));
                            }
                        }
                    });
                }
            });
            if over_disabled_action {
                // Disabled actions take precedence over the clickable province row.
                ui.ctx().set_cursor_icon(egui::CursorIcon::Default);
            }
            if let Some(action) = selected_action {
                message = Some(match action {
                    SpyDirectoryAction::Recall => campaign
                        .espionage
                        .recall(player, id, campaign.economy.month, &campaign.espionage_config)
                        .map_or_else(
                            |error| error.to_string(),
                            |_| "Spy recall started. Six months remain.".to_owned(),
                        ),
                    SpyDirectoryAction::Flee => campaign.flee_spy(player, id).map_or_else(
                        |error| error,
                        |detected| {
                            if detected {
                                "Spy fled, but was detected."
                            } else {
                                "Spy fled safely."
                            }
                            .to_owned()
                        },
                    ),
                    SpyDirectoryAction::Deploy(assignment) => {
                        let name = SPY_ACTIONS
                            .iter()
                            .find(|(kind, _, _)| *kind == assignment)
                            .map_or("Spy", |(_, name, _)| *name);
                        let distance = campaign.distance(player, id);
                        campaign
                            .espionage
                            .deploy_assignment(
                                player,
                                id,
                                &mut campaign.actors,
                                &campaign.politics,
                                &campaign.espionage_config,
                                assignment,
                                distance,
                            )
                            .map_or_else(
                                |error| error.to_string(),
                                |_| format!("Spy deployed: {name}."),
                            )
                    },
                });
            } else if response.clicked() && !over_action {
                target = Some(id);
            }
            ui.add_space(3.0 * scale);
            row += 1;
        }
        if row == 0 {
            directory_empty_group(ui, scale);
        }
        ui.add_space(8.0 * scale);
    }
    (target, message)
}

#[derive(Clone, Copy)]
enum SpyDirectoryAction {
    Recall,
    Flee,
    Deploy(SpyAssignment),
}

fn spy_directory_icon_button(
    ui: &mut egui::Ui,
    icon: Icon,
    tooltip: &str,
    enabled: bool,
    scale: f32,
) -> egui::Response {
    let response = ui
        .add_enabled_ui(enabled, |ui| {
            ui.add_sized([28.0 * scale, 28.0 * scale], egui::Button::new(""))
        })
        .inner;
    let rect =
        egui::Rect::from_center_size(response.rect.center(), egui::Vec2::splat(19.0 * scale));
    if icon == Icon::Cancel {
        campaign_widgets::paint_raster_icon(
            ui,
            icon,
            rect,
            if enabled {
                egui::Color32::WHITE
            } else {
                egui::Color32::from_white_alpha(100)
            },
        );
    } else {
        campaign_widgets::paint_icon(ui, icon, rect);
    }
    response.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, enabled, tooltip));
    if response.enabled() {
        response.on_hover_cursor(egui::CursorIcon::PointingHand).on_hover_text(tooltip)
    } else {
        response.on_disabled_hover_text(tooltip)
    }
}
/// A shared compact card for spy missions and one-time political actions.
pub(super) fn diplomacy_action_card(
    ui: &mut egui::Ui,
    rect: egui::Rect,
    id: egui::Id,
    name: &str,
    symbol: Icon,
    costs: &[(Icon, String)],
    available: bool,
    scale: f32,
) -> egui::Response {
    diplomacy_action_card_with_title_inset(ui, rect, id, name, symbol, costs, available, scale, 0.0)
}

/// Reserve title space for a compact control in the card's upper-right corner.
pub(super) fn diplomacy_action_card_with_title_inset(
    ui: &mut egui::Ui,
    rect: egui::Rect,
    id: egui::Id,
    name: &str,
    symbol: Icon,
    costs: &[(Icon, String)],
    available: bool,
    scale: f32,
    title_inset: f32,
) -> egui::Response {
    let response = ui.interact(
        rect,
        id,
        if available {
            egui::Sense::click()
        } else {
            egui::Sense::hover()
        },
    );
    let response = if available {
        response.on_hover_cursor(egui::CursorIcon::PointingHand)
    } else {
        response
    };
    response.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, available, name));
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
    let image_size = (38.0 * scale).min(rect.width() * 0.27);
    let image_rect = egui::Rect::from_center_size(
        egui::pos2(rect.left() + 6.0 * scale + image_size * 0.5, rect.center().y - 4.0 * scale),
        egui::Vec2::splat(image_size),
    );
    campaign_widgets::paint_raster_icon(
        ui,
        symbol,
        image_rect,
        if available {
            egui::Color32::WHITE
        } else {
            egui::Color32::from_white_alpha(217)
        },
    );
    let ink = if available {
        province_panel::INK
    } else {
        campaign_widgets::UNAVAILABLE_PURCHASE_INK
    };
    let content_left = image_rect.right() + 5.0 * scale;
    let title_width = (rect.right() - content_left - (5.0 + title_inset) * scale).max(0.0);
    let font_sizes: &[f32] = if title_inset > 0.0 {
        &[16.0, 15.5, 15.0, 14.0, 13.0]
    } else {
        &[16.0, 15.5, 15.0]
    };
    let title = font_sizes
        .iter()
        .map(|size| {
            ui.painter().layout_no_wrap(
                name.to_owned(),
                egui::FontId::proportional(size * scale),
                ink,
            )
        })
        .find(|title| title.size().x <= title_width)
        .unwrap_or_else(|| {
            ui.painter().layout_no_wrap(
                name.to_owned(),
                egui::FontId::proportional(15.0 * scale),
                ink,
            )
        });
    ui.painter().galley(egui::pos2(content_left, rect.top() + 10.0 * scale), title, ink);
    let mut cost_left = content_left;
    for (index, (cost_icon, value)) in costs.iter().enumerate() {
        if index > 0 {
            cost_left += 17.0 * scale;
        }
        let center_y = rect.top() + 47.0 * scale;
        campaign_widgets::paint_icon(
            ui,
            *cost_icon,
            egui::Rect::from_center_size(
                egui::pos2(cost_left + 7.0 * scale, center_y),
                egui::Vec2::splat(14.0 * scale),
            ),
        );
        let cost = ui.painter().layout_no_wrap(
            value.clone(),
            egui::FontId::proportional(14.5 * scale),
            ink,
        );
        let position = egui::pos2(cost_left + 16.0 * scale, center_y - cost.size().y * 0.5);
        cost_left = position.x + cost.size().x;
        ui.painter().galley(position, cost, ink);
    }
    response
}

/// One-time diplomacy actions appear below the spy network.
fn political_maneuvers(
    ui: &mut egui::Ui,
    campaign: &mut Campaign,
    province: usize,
    player: usize,
    scale: f32,
    already_acted: bool,
) -> Option<String> {
    policy_widgets::section(ui, scale, "POLITICAL MANEUVERS");
    let mut message = None;
    let quote = campaign.noble_bribe_quote(player, province).ok();
    let bribe_cost = campaign.noble_bribe_cost(player, province).ok();
    let year = campaign.economy.month / 12;
    let months_until_next_year = 12 - campaign.economy.month % 12;
    let bribe_cooldown = (campaign.diplomacy_used.get(&(player, false)) == Some(&year))
        .then_some(months_until_next_year);
    let insult_cooldown = (campaign.diplomacy_used.get(&(player, true)) == Some(&year))
        .then_some(months_until_next_year);
    let can_bribe = bribe_cost.is_some_and(|price| campaign.economy.players[player].coin >= price)
        && !already_acted;
    let target = campaign.insult_target(player, province).ok();
    let can_insult = target.is_some() && insult_cooldown.is_none() && !already_acted;
    let gap = 7.0 * scale;
    let columns = if ui.available_width() >= 390.0 * scale {
        3
    } else {
        2
    };
    let button_size = egui::vec2(
        (ui.available_width() - (columns - 1) as f32 * gap) / columns as f32,
        72.0 * scale,
    );
    let (grid, _) = ui.allocate_exact_size(button_size, egui::Sense::hover());
    for (index, (name, symbol, cost, available, cooldown)) in [
        (
            "Bribe nobles",
            Icon::BribeNobles,
            quote.map_or("—".to_owned(), |value| format!("{value:.0}")),
            can_bribe,
            bribe_cooldown,
        ),
        ("Send insult", Icon::InsultPlayer, "Free".to_owned(), can_insult, insult_cooldown),
    ]
    .into_iter()
    .enumerate()
    {
        let rect = egui::Rect::from_min_size(
            grid.min + egui::vec2(index as f32 * (button_size.x + gap), 0.0),
            button_size,
        );
        let response = diplomacy_action_card(
            ui,
            rect,
            ui.id().with(("political-maneuver", province, index)),
            name,
            symbol,
            &[(Icon::Coin, cost)],
            available && message.is_none(),
            scale,
        );
        let description = if index == 0 {
            format!(
                "Pay {} sestertii once for random 0–10 Control. Once per year.",
                quote.map_or("unavailable".to_owned(), |value| format!("{value:.0}"))
            )
        } else if matches!(target, Some(TradeParty::Player(_))) {
            "Their provinces lose 10 Relation toward you. Once per year.".to_owned()
        } else {
            "This province loses 10 Relation toward you. Once per year.".to_owned()
        };
        let blocked = if let Some(months) = cooldown {
            let unit = if months == 1 {
                "month"
            } else {
                "months"
            };
            Some(format!("Try again in {months} {unit}."))
        } else if index == 0 && quote.is_none() {
            Some("Bribe nobles requires an independent or rival-owned province.".to_owned())
        } else if index == 0 && !can_bribe && !already_acted {
            Some("Not enough sestertii to bribe the nobles.".to_owned())
        } else if index == 1 && target.is_none() {
            Some("Send insult requires an NPC or another player's province.".to_owned())
        } else {
            None
        };
        let response = response.inspection_hover_ui(|ui| {
            ui.label(description);
            if let Some(reason) = blocked {
                ui.add_space(ui.text_style_height(&egui::TextStyle::Body));
                ui.label(egui::RichText::new(reason).color(egui::Color32::from_rgb(170, 45, 35)));
            }
        });
        if index == 0 {
            if response.clicked() {
                message = Some(match campaign.bribe_nobles(player, province) {
                    Ok((gain, evidence)) => {
                        if evidence.is_some() {
                            format!(
                                "Nobles bribed: +{gain} Control. A rival spy exposed the payment."
                            )
                        } else {
                            format!("Nobles bribed: +{gain} Control.")
                        }
                    },
                    Err(error) => error.to_string(),
                });
            }
        } else if response.clicked() {
            message = Some(match campaign.send_insult(player, province) {
                Ok(TradeParty::Player(owner)) => {
                    format!("Player {} insulted: −10 Relation in their provinces.", owner + 1)
                },
                Ok(TradeParty::Npc(province)) => {
                    format!("{} insulted: −10 Relation.", campaign.economy.provinces[province].name)
                },
                Err(error) => error.to_string(),
            });
        }
    }
    message
}

fn spy_flee_button(ui: &mut egui::Ui, scale: f32) -> egui::Response {
    campaign_widgets::network_action_button(ui, Icon::SpyFlee, "Flee", true, scale)
}

fn spy_recall_button(
    ui: &mut egui::Ui,
    scale: f32,
    months_remaining: Option<u32>,
) -> egui::Response {
    let label = months_remaining.map_or_else(
        || "Recall".to_owned(),
        |months| {
            format!(
                "{months} {}",
                if months == 1 {
                    "month"
                } else {
                    "months"
                }
            )
        },
    );
    campaign_widgets::network_action_button(
        ui,
        Icon::Cancel,
        &label,
        months_remaining.is_none(),
        scale,
    )
}

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
                if spy_flee_button(ui, scale)
                    .inspection_hover_text(format!(
                        "Return immediately at {:.0}% detection chance.",
                        crate::game::politics::espionage::flee_detection_chance(
                            campaign.economy.provinces[province].happiness[0],
                            &campaign.espionage_config,
                        ) * 100.0
                    ))
                    .clicked()
                {
                    message = Some(match campaign.flee_spy(player, province) {
                        Ok(true) => "Spy fled, but was detected.".into(),
                        Ok(false) => "Spy fled safely.".into(),
                        Err(error) => error,
                    });
                }
                if spy_recall_button(
                    ui,
                    scale,
                    mission.recall_month.map(|due| due.saturating_sub(campaign.economy.month)),
                )
                .inspection_hover_text(
                    "Return after six months; normal upkeep and detection continue until then.",
                )
                .clicked()
                {
                    message = Some(
                        match campaign.espionage.recall(
                            player,
                            province,
                            campaign.economy.month,
                            &campaign.espionage_config,
                        ) {
                            Ok(_) => "Spy recall started. Six months remain.".into(),
                            Err(error) => format!("{error:?}"),
                        },
                    );
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
            SpyAssignment::UndermineOpponents | SpyAssignment::SupportRevolt => (
                "Happiness lost",
                policy_widgets::compact_decimal(totals.happiness_reduced),
                "Population happiness reduced by this spy since deployment.".to_owned(),
            ),
            SpyAssignment::DiscreditRivals => (
                "Rival losses",
                format!(
                    "{} C · {} R",
                    policy_widgets::compact_decimal(totals.rival_control_reduced),
                    policy_widgets::compact_decimal(totals.rival_relation_reduced)
                ),
                "C = rival Control reduced; R = rival Relation reduced.".to_owned(),
            ),
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
                format!("{}%/mo", policy_widgets::compact_decimal(risk)),
                "One check per spy each month. Time deployed does not increase this rate; Noble happiness can change it. Fleeing, including when the treasury is exhausted, uses a higher chance.".to_owned(),
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
    }
    if active.is_none() {
        let actions: Vec<_> = SPY_ACTIONS
            .into_iter()
            .filter(|(assignment, _, _)| {
                !(matches!(
                    assignment,
                    SpyAssignment::GainControl | SpyAssignment::ImproveRelations
                ) && matches!(state, PoliticalState::Owned { owner } if owner == player))
            })
            .collect();
        let gap = 7.0 * scale;
        let columns = if ui.available_width() >= 390.0 * scale {
            3
        } else {
            2
        };
        let button_size = egui::vec2(
            (ui.available_width() - (columns - 1) as f32 * gap) / columns as f32,
            72.0 * scale,
        );
        let rows = actions.len().div_ceil(columns);
        let (grid, _) = ui.allocate_exact_size(
            egui::vec2(ui.available_width(), rows as f32 * (button_size.y + gap) - gap),
            egui::Sense::hover(),
        );
        for (index, (assignment, name, symbol)) in actions.into_iter().enumerate() {
            let deployment =
                campaign.espionage_config.deployment_cost_at(assignment, distance).ok();
            let description = match assignment {
            SpyAssignment::GainControl => "Gain 0–3 Control each month.",
            SpyAssignment::ImproveRelations => "Gain 0–3 Relation each month.",
            SpyAssignment::DiscoverScandals => {
                "Search for scandals to use against the enemy player. Discovery is not guaranteed."
            },
            SpyAssignment::UndermineOpponents => "Reduce 0–3 happiness for one random Noble, Citizen or Plebeian class each month.",
            SpyAssignment::SupportRevolt => "Reduce the happiness of slaves by 0–3 each month.",
            SpyAssignment::DiscreditRivals => "Reduce a random rivals' Control or Relation by 0–3 each month.",
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
                Some(match assignment {
                    SpyAssignment::GainControl => {
                        "Build control is available only in independent or rival-owned provinces."
                    },
                    SpyAssignment::DiscreditRivals => {
                        "Discredit rivals is available only in independent provinces."
                    },
                    _ => "This mission is unavailable under the province's current government.",
                })
            } else if campaign.actors[player].influence < deployment.unwrap_or(f64::INFINITY) {
                Some("Not enough Influence to deploy a spy.")
            } else {
                None
            };
            let available = blocked.is_none() && message.is_none();
            let rect = egui::Rect::from_min_size(
                grid.min
                    + egui::vec2(
                        (index % columns) as f32 * (button_size.x + gap),
                        (index / columns) as f32 * (button_size.y + gap),
                    ),
                button_size,
            );
            let response = diplomacy_action_card(
                ui,
                rect,
                ui.id().with(("spy-action", province, assignment as u8)),
                name,
                symbol,
                &[
                    (
                        Icon::Influence,
                        deployment.map_or("—".to_owned(), |cost| format!("{cost:.0}")),
                    ),
                    (Icon::Coin, upkeep.map_or("—/mo".to_owned(), |cost| format!("{cost:.0}/mo"))),
                ],
                available,
                scale,
            );
            if response
                .inspection_hover_ui(|ui| {
                    ui.label(description);
                    if let Some(reason) = blocked {
                        ui.add_space(ui.text_style_height(&egui::TextStyle::Body));
                        ui.label(
                            egui::RichText::new(reason).color(egui::Color32::from_rgb(170, 45, 35)),
                        );
                    }
                })
                .clicked()
                && available
            {
                let result = campaign.espionage.deploy_assignment(
                    player,
                    province,
                    &mut campaign.actors,
                    &campaign.politics,
                    &campaign.espionage_config,
                    assignment,
                    distance,
                );
                message =
                    Some(result.map_or_else(
                        |error| error.to_string(),
                        |_| format!("Spy deployed: {name}."),
                    ));
            }
        }
    }
    message
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
    campaign.reconcile_provinces();
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
        policy_widgets::section(ui, scale, "Military Access");
        egui::Frame::new()
            .inner_margin(egui::Margin::symmetric(
                (6.0 * scale).round() as i8,
                (10.0 * scale).round() as i8,
            ))
            .show(ui, |ui| {
                ui.add(egui::Label::new("Permit peaceful military entry to this province. If revoked, foreign units march to the closest own province.").wrap());
                ui.add_space(8.0 * scale);
                for guest in 0..campaign.actors.len() {
                    if guest == player {
                        continue;
                    }
                    let war = campaign.wars[player][guest];
                    let mut invited = campaign.province_access_granted(province, player, guest);
                    let color = colors.get(guest).copied().unwrap_or(province_panel::INK);
                    let label = format!("Player {}", guest + 1);
                    let (rect, response) = ui.allocate_exact_size(
                        egui::vec2(ui.available_width(), 42.0 * scale),
                        if war { egui::Sense::hover() } else { egui::Sense::click() },
                    );
                    response.widget_info(|| {
                        egui::WidgetInfo::labeled(
                            egui::WidgetType::Checkbox,
                            invited,
                            format!("Military access for {label}"),
                        )
                    });
                    let response = if war {
                        response.inspection_hover_text("Access unavailable during war.")
                    } else {
                        response.on_hover_cursor(egui::CursorIcon::PointingHand)
                    };
                    let changed = !war && response.clicked();
                    if changed {
                        invited = !invited;
                    }
                    let now = ui.input(|input| input.time);
                    let flash_id = response.id.with("access-flash-until");
                    let flash_until = if changed {
                        let until = now + 0.16;
                        ui.ctx().data_mut(|data| data.insert_temp(flash_id, until));
                        until
                    } else {
                        ui.ctx().data(|data| data.get_temp::<f64>(flash_id).unwrap_or(0.0))
                    };
                    let pressed = !war && (response.is_pointer_button_down_on() || now < flash_until);
                    if pressed {
                        ui.ctx().request_repaint_after(std::time::Duration::from_millis(16));
                    }
                    let painter = ui.painter_at(rect);
                    if pressed || (response.hovered() && !war) {
                        painter.rect_filled(
                            rect,
                            2.0 * scale,
                            if pressed {
                                egui::Color32::from_rgb(222, 202, 174)
                            } else {
                                egui::Color32::from_rgb(239, 230, 213)
                            },
                        );
                    }
                    painter.line_segment(
                        [rect.left_bottom(), rect.right_bottom()],
                        egui::Stroke::new(0.8 * scale, province_panel::RULE),
                    );
                    let center = rect.left_center() + egui::vec2(10.0 * scale, 0.0);
                    painter.circle_filled(center, 5.0 * scale, color);
                    painter.circle_stroke(
                        center,
                        5.0 * scale,
                        egui::Stroke::new(0.8 * scale, province_panel::RULE),
                    );
                    painter.text(
                        rect.left_center() + egui::vec2(24.0 * scale, 0.0),
                        egui::Align2::LEFT_CENTER,
                        label,
                        egui::FontId::proportional(14.0 * scale),
                        color,
                    );
                    let state_color = if war {
                        province_panel::RULE
                    } else if invited {
                        egui::Color32::from_rgb(105, 83, 53)
                    } else {
                        province_panel::INK
                    };
                    painter.text(
                        rect.right_center() - egui::vec2(34.0 * scale, 0.0),
                        egui::Align2::RIGHT_CENTER,
                        if war { "At war" } else if invited { "Granted" } else { "Closed" },
                        egui::FontId::proportional(12.5 * scale),
                        state_color,
                    );
                    let seal = rect.right_center() - egui::vec2(16.0 * scale, 0.0);
                    painter.circle_stroke(
                        seal,
                        7.0 * scale,
                        egui::Stroke::new(1.2 * scale, state_color),
                    );
                    if invited && !war {
                        painter.circle_filled(seal, 3.8 * scale, state_color);
                    }
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
                                    "Access change failed",
                                    &text,
                                );
                                message = Some(text);
                            },
                        }
                    }
                }
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
            .inspection_hover_text("End bilateral hostility. Existing ownership remains unchanged. Peaceful entry still requires the owner's invitation.")
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
            "Peaceful military entry requires the owner's invitation. Invited troops build up to 1 Control per month, based on stationed strength.",
        );
        let relation = campaign.politics[province].relation(player);
        let attack_tip = if relation > 10.0 {
            "No casus belli: the attack creates a Senate scandal. Military invitations end; entering hostile territory begins combat."
        } else {
            "Casus belli: relation is 10 or lower. Military invitations end; entering hostile territory begins combat."
        };
        if ui.button("Declare hostility").inspection_hover_text(attack_tip).clicked() {
            campaign.invitations[player][owner] = false;
            campaign.invitations[owner][player] = false;
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
    colors: &[egui::Color32],
    filters: &mut campaign_notices::NoticeFilters,
    scale: f32,
) -> Option<CampaignNotice> {
    campaign_notices::filtered_list(
        ui,
        filters,
        scale,
        ("province_notices", province, player),
        |ui, filters| {
            let mut navigation = None;
            let mut empty = true;
            for (row, notice) in campaign
                .notifications
                .history_for(player)
                .filter(|notice| notice.province == Some(province))
                .filter(|notice| campaign_notices::allows(filters, notice))
                .enumerate()
            {
                empty = false;
                ui.push_id(notice.id, |ui| {
                    if campaign_notices::card(ui, notice, campaign, colors, row, scale).clicked() {
                        navigation = Some(notice.clone());
                    }
                });
            }
            if empty {
                ui.add_space(8. * scale);
                ui.horizontal(|ui| {
                    ui.add_space(12. * scale);
                    if campaign
                        .notifications
                        .history_for(player)
                        .any(|notice| notice.province == Some(province))
                    {
                        ui.label("No notifications match these filters.");
                    } else {
                        ui.label("No notifications from this province yet.");
                    }
                });
            }
            navigation
        },
    )
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
            view.close_province_selector();
            detail.0 = None;
            view.last_detail = None;
        },
        NoticeAction::OpenMilitary => {
            view.open = Some(CampaignTab::Military);
            view.close_province_selector();
            detail.0 = None;
            view.last_detail = None;
        },
        NoticeAction::OpenProvince(id) => {
            if notice.kind == super::campaign_notifications::NoticeKind::TradeInterrupted {
                view.open = Some(CampaignTab::Trade);
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
            open_scandal(view, scandal, province, campaign, detail, map);
        },
    }
}

/// Overview evidence cards and discovery notifications share their destinations.
pub(super) fn open_scandal(
    view: &mut CampaignUi,
    scandal: u64,
    source_province: Option<usize>,
    campaign: &Campaign,
    detail: &mut ProvincePanelOpen,
    map: &mut MapView,
) {
    let evidence = campaign.espionage.scandals.iter().find(|evidence| {
        evidence.id == scandal
            && evidence.is_current(campaign.economy.month.max(campaign.senate.month))
    });
    // Historical notices remain useful without reopening an empty selection.
    if evidence.is_none() {
        view.highlight_scandal = None;
        if let Some(province) = source_province {
            view.open_province_section(province, 5);
            detail.0 = Some(MapDetail::Province(province));
            map.focus_province(province);
        } else {
            view.open_evidence(None);
            detail.0 = None;
            view.last_detail = None;
        }
        return;
    }
    let npc_province = evidence.and_then(|evidence| match evidence.target {
        crate::game::politics::espionage::ScandalTarget::Province(province) => Some(province),
        _ => None,
    });
    view.open_evidence(npc_province);
    view.highlight_scandal = npc_province.is_none().then_some(scandal);
    detail.0 = npc_province.map(MapDetail::Province);
    if npc_province.is_none() {
        view.last_detail = None;
    }
    if let Some(province) = npc_province.or(source_province) {
        map.focus_province(province);
    }
}
