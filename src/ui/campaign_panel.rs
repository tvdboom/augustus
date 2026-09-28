//! Campaign inspectors using the existing parchment, banner colors, and button treatment.

use super::campaign::Campaign;
use super::campaign_notifications::{CampaignNotice, NoticeAction};
use super::campaign_widgets::{icon, stat, Icon};
use super::*;
use crate::game::economy::ConstructionProject;
use crate::game::military::*;
use crate::game::politics::diplomacy::*;
use crate::game::politics::{Currency, PoliticalError, PoliticalPlayer};

/// Global banner destinations and the contextual province inspector.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum CampaignTab {
    Governance,
    Military,
    Trade,
    Senate,
    Province,
}

/// View state is separate from game rules and never changes simulation outcomes.
#[derive(Resource, Default)]
pub(crate) struct CampaignUi {
    pub open: Option<CampaignTab>,
    pub province: Option<usize>,
    section: usize,
    last_detail: Option<MapDetail>,
    notice: String,
    highlight_scandal: Option<u64>,
    province_selector_open: bool,
    province_search: String,
}

impl CampaignUi {
    /// Rome's city and province selections lead directly to the Senate.
    fn select_map_detail(&mut self, selected: MapDetail, campaign: &Campaign) {
        self.close_province_selector();
        let province = match selected {
            MapDetail::Province(id) | MapDetail::City(id) => id,
        };
        self.province = Some(province);
        self.last_detail = Some(selected);
        self.section = if matches!(selected, MapDetail::City(_)) {
            2
        } else {
            0
        };
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
    let search_id = ui.id().with("province-title-search");
    let editing = view.province_selector_open;
    let response = ui
        .scope(|ui| {
            ui.visuals_mut().weak_text_color = Some(if editing {
                ink.gamma_multiply(0.5)
            } else {
                ink
            });
            ui.visuals_mut().text_cursor.stroke.color = ink;
            ui.add_sized(
                size,
                egui::TextEdit::singleline(&mut view.province_search)
                    .id(search_id)
                    .frame(
                        egui::Frame::new()
                            .fill(egui::Color32::from_black_alpha(if editing {
                                18
                            } else {
                                8
                            }))
                            .stroke(egui::Stroke::new(scale, egui::Color32::TRANSPARENT))
                            .corner_radius(3.0 * scale)
                            .inner_margin(egui::vec2(8.0 * scale, 0.0)),
                    )
                    .min_size(size)
                    .horizontal_align(egui::Align::Center)
                    .vertical_align(egui::Align::Center)
                    .font(egui::FontId::proportional(20.0 * scale))
                    .text_color(ink)
                    .hint_text(egui::RichText::new(&provinces[province].name).size(20.0 * scale)),
            )
        })
        .inner
        .on_hover_text("Click and type to search provinces");
    if (response.clicked() || response.gained_focus()) && !view.province_selector_open {
        view.province_selector_open = true;
        response.request_focus();
    }
    let query = view.province_search.trim().to_lowercase();
    let mut matches: Vec<_> = provinces
        .iter()
        .enumerate()
        .filter(|(_, province)| province.name.to_lowercase().contains(&query))
        .collect();
    matches.sort_unstable_by(|(_, a), (_, b)| a.name.cmp(&b.name));
    let mut selected = None;
    let popup_frame = egui::Frame::popup(ui.style()).inner_margin(4.0 * scale);
    egui::Popup::from_response(&response)
        .id(search_id.with("dropdown"))
        .open_bool(&mut view.province_selector_open)
        .close_behavior(if response.contains_pointer() {
            egui::PopupCloseBehavior::IgnoreClicks
        } else {
            egui::PopupCloseBehavior::CloseOnClickOutside
        })
        .align(egui::RectAlign::BOTTOM_START)
        .align_alternatives(&[])
        .gap(0.0)
        .width(response.rect.width())
        .frame(popup_frame)
        .show(|ui| {
            ui.set_width((response.rect.width() - popup_frame.total_margin().sum().x).max(1.0));
            if ui.input_mut(|input| input.consume_key(egui::Modifiers::NONE, egui::Key::Enter)) {
                selected = matches.first().map(|(id, _)| *id);
            }
            egui::ScrollArea::vertical()
                .id_salt("province-search-results")
                .max_height((ui.ctx().content_rect().height() * 0.4).min(280.0 * scale))
                .show(ui, |ui| {
                    if matches.is_empty() {
                        ui.label("No provinces match");
                    }
                    for (id, province) in matches {
                        if ui
                            .add(
                                egui::Button::selectable(view.province == Some(id), ())
                                    .left_text(format!(
                                        "{}{}",
                                        province.name,
                                        if province.owner == Some(player) {
                                            " · yours"
                                        } else {
                                            ""
                                        }
                                    ))
                                    .truncate()
                                    .min_size(egui::vec2(ui.available_width(), 28.0 * scale)),
                            )
                            .clicked()
                        {
                            selected = Some(id);
                        }
                    }
                });
        });
    if let Some(id) = selected {
        view.province = Some(id);
        view.province_selector_open = false;
    }
    if !view.province_selector_open {
        view.province_search.clear();
        ui.memory_mut(|memory| memory.surrender_focus(search_id));
    }
    if response.hovered() || response.has_focus() {
        ui.painter().rect_stroke(
            response.rect,
            3.0 * scale,
            egui::Stroke::new(scale, ink.gamma_multiply(0.7)),
            egui::StrokeKind::Inside,
        );
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
    if !matches!(tab, CampaignTab::Senate | CampaignTab::Governance | CampaignTab::Trade) {
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
                    let tabs = [
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
                            if selected || response.hovered() {
                                ui.painter().rect_filled(
                                    rect,
                                    3.0,
                                    if selected {
                                        egui::Color32::from_rgb(219, 200, 168)
                                    } else {
                                        province_panel::TABLE_STRIPE
                                    },
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
                                        .truncate(),
                                    );
                                },
                            );
                            if selected {
                                ui.painter().hline(
                                    rect.x_range(),
                                    rect.bottom(),
                                    egui::Stroke::new(2.0 * scale, color),
                                );
                            }
                            if response
                                .on_hover_text(label)
                                .on_hover_cursor(egui::CursorIcon::PointingHand)
                                .clicked()
                            {
                                view.section = index;
                            }
                        }
                    });
                }
                ui.separator();
                if tab == CampaignTab::Province && matches!(view.section, 0 | 2) {
                    let selected = &provinces[view.province.unwrap_or(province)];
                    let landscape = if view.section == 2 {
                        if selected.has_city {
                            campaign_widgets::ProvinceLandscape::City
                        } else {
                            campaign_widgets::ProvinceLandscape::Village
                        }
                    } else {
                        campaign_widgets::ProvinceLandscape::Terrain(selected.terrain)
                    };
                    let landscape = campaign_widgets::portrait(ui, landscape, 76.0 * scale);
                    if !(tab == CampaignTab::Province && view.section == 2) {
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
) {
    let political_height = if campaign.politics.get(province).is_some() {
        80.0
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
        if let Some(politics) = campaign.politics.get(province) {
            overview_politics(ui, politics, player, scale);
            ui.add_space(7.0 * scale);
        }
        if owned {
            campaign_economy::overview(ui, &campaign.economy, province, scale);
        } else {
            province_intelligence::overview(ui, campaign, province, player, scale);
        }
    });
}

/// Header and frame margins are accounted for before fitting the read-only overview.
fn province_overview_body(
    ui: &mut egui::Ui,
    inner_bottom: f32,
    campaign: &Campaign,
    province: usize,
    player: usize,
    scale: f32,
) {
    ui.set_max_height((inner_bottom - ui.next_widget_position().y).max(0.0));
    province_overview(ui, campaign, province, player, scale);
}

/// Keep building sections and status inside the panel, scrolling when needed.
fn province_buildings_body(
    ui: &mut egui::Ui,
    inner_bottom: f32,
    campaign: &mut Campaign,
    province: usize,
    player: usize,
    scale: f32,
    status: &str,
) -> Option<String> {
    let remaining = (inner_bottom - ui.next_widget_position().y).max(0.0);
    let footer = if status.is_empty() {
        0.0
    } else {
        (26.0 * scale).min(remaining * 0.2)
    };
    ui.set_max_height(remaining - footer);
    let before = construction_underway(campaign, province);
    let message = egui::ScrollArea::vertical()
        .id_salt(("province-buildings", province, player))
        .max_height(remaining - footer)
        .min_scrolled_height(0.0)
        .auto_shrink([false, false])
        .show(ui, |ui| {
            if campaign.economy.provinces[province].owner == Some(player) {
                campaign_economy::buildings(ui, &mut campaign.economy, province, player, scale)
            } else {
                province_intelligence::buildings(ui, campaign, province, player, scale);
                None
            }
        })
        .inner;
    let after = construction_underway(campaign, province);
    if before != after {
        if after.is_some() {
            campaign.notify_construction_started(player, province);
        }
        if let Some(Icon::Wonder(wonder)) = after {
            campaign.notify_wonder_started(player, province, wonder);
        }
    }
    if footer > 0.0 {
        let rect = egui::Rect::from_min_max(
            egui::pos2(ui.max_rect().left(), inner_bottom - footer),
            egui::pos2(ui.max_rect().right(), inner_bottom),
        );
        ui.put(rect, egui::Label::new(egui::RichText::new(status).size(12.0 * scale)).truncate())
            .on_hover_text(status);
    }
    message
}

/// Compact meters show political values; full ownership fills the domestic indicators.
fn overview_politics(ui: &mut egui::Ui, politics: &ProvincePolitics, player: usize, scale: f32) {
    if politics.state == PoliticalState::Rome {
        ui.label("Rome · protected capital. Conquest grants immediate victory.");
        return;
    }
    let gold = egui::Color32::from_rgb(190, 150, 76);
    let (control, control_caption, control_text, control_tip) = match &politics.state {
        PoliticalState::Independent { shares, .. } => {
            let value = shares.get(player).copied().unwrap_or(0.0);
            (value / 100.0, "Your political share".to_owned(), format!("{value:.0}/100"),
                "Your current share of Independent Control. Local government and all player shares total 100. Relation measures friendship separately.")
        },
        PoliticalState::Vassal { overlord, control, .. } => (
            control / 100.0,
            if *overlord == player { "Your vassal".to_owned() } else { format!("Player {}'s vassal", overlord + 1) },
            format!("{control:.0}/100"),
            "Vassal Control belongs to the province's overlord. It measures stability and integration progress. Relation below 50 causes decay; garrisons and government support can offset it.",
        ),
        PoliticalState::Owned { owner } => (
            1.0,
            if *owner == player { "Your province".to_owned() } else { format!("Player {}'s province", owner + 1) },
            "100/100".to_owned(),
            "Direct ownership replaces diplomatic Control. Owned provinces have no Independent or Vassal Control score.",
        ),
        PoliticalState::Rome => unreachable!("Rome has no political meters"),
    };
    let domestic = matches!(politics.state, PoliticalState::Owned { owner } if owner == player);
    let relation = politics.relation(player).clamp(0.0, 100.0);
    let (band, relation_color) = if relation < 20.0 {
        ("Very hostile", egui::Color32::from_rgb(155, 69, 58))
    } else if relation < 40.0 {
        ("Hostile", egui::Color32::from_rgb(182, 108, 75))
    } else if relation < 60.0 {
        ("Neutral", gold)
    } else if relation < 80.0 {
        ("Friendly", egui::Color32::from_rgb(139, 161, 112))
    } else {
        ("Strongly friendly", egui::Color32::from_rgb(103, 149, 118))
    };
    ui.columns(2, |columns| {
        political_card(&mut columns[0], Icon::Control, "Control", &control_caption, control as f32, &control_text, gold, control_tip, scale);
        political_card(
            &mut columns[1], Icon::Relation, "Relation",
            if domestic { "Domestic province" } else { band },
            if domestic { 1.0 } else { relation as f32 / 100.0 },
            &if domestic { "100/100".to_owned() } else { format!("{relation:.0}/100") },
            if domestic { egui::Color32::from_rgb(103, 149, 118) } else { relation_color },
            if domestic { "The full bar indicates your domestic province. Owned provinces use population happiness instead of diplomatic Relation. Integration converts the former relation into happiness." }
            else { "This province's current Relation toward you, from 0 to 100. Relation measures friendship; Control separately measures political power." },
            scale,
        );
    });
}

/// Cards retain a readable icon, label and exact value at compact panel widths.
fn political_card(
    ui: &mut egui::Ui,
    symbol: Icon,
    title: &str,
    caption: &str,
    progress: f32,
    value: &str,
    color: egui::Color32,
    explanation: &str,
    scale: f32,
) {
    let width = ui.available_width();
    egui::Frame::new()
        .fill(province_panel::TABLE_STRIPE)
        .stroke(egui::Stroke::new(scale, province_panel::RULE))
        .corner_radius(4.0 * scale)
        .inner_margin(8.0 * scale)
        .show(ui, |ui| {
            ui.set_width((width - 2.0 * (8.0 * scale).round() - 2.0 * scale).max(1.0));
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
        .on_hover_text(format!("{caption}\n{explanation}"));
}

/// Split the actual space left by the header between scrollable content and a
/// bounded, wrapped status message. Newly produced status appears next frame so
/// an action cannot grow the panel after its body budget has already been used.
fn scroll_body_with_status<R>(
    ui: &mut egui::Ui,
    inner_bottom: f32,
    scale: f32,
    status: &str,
    id: impl std::hash::Hash + std::fmt::Debug,
    render: impl FnOnce(&mut egui::Ui) -> R,
) -> R {
    let remaining = (inner_bottom - ui.next_widget_position().y).max(0.0);
    let gap = ui.spacing().item_spacing.y;
    let footer = if status.is_empty() {
        0.0
    } else {
        (72.0 * scale).min(remaining * 0.25)
    };
    let body = (remaining
        - footer
        - if footer > 0.0 {
            gap
        } else {
            0.0
        })
    .max(0.0);
    let id = egui::Id::new(id);
    let response = egui::ScrollArea::vertical()
        .id_salt(id)
        .max_height(body)
        .min_scrolled_height(0.0)
        .auto_shrink([false, false])
        .show(ui, render);
    if footer > 0.0 {
        egui::ScrollArea::vertical()
            .id_salt(id.with("status"))
            .max_height(footer)
            .min_scrolled_height(0.0)
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.add(egui::Label::new(status).wrap());
            });
    }
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
        && view.province.is_some_and(|id| campaign.economy.provinces[id].name == "Latium")
    {
        view.open = Some(CampaignTab::Senate);
        view.close_province_selector();
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
    let Ok(ctx) = contexts.ctx_mut() else {
        return;
    };
    let player = practice.active_player;
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
        CampaignTab::Senate => "Rome · Senate".to_owned(),
        CampaignTab::Governance => "Governance".to_owned(),
        CampaignTab::Military => "Military".to_owned(),
        CampaignTab::Trade => "Trade".to_owned(),
        CampaignTab::Province => campaign.economy.provinces[province].name.clone(),
    };
    let mut closed = false;
    let mut action = None;
    let mut navigation = None;
    let mut army_province = None;
    let old_notice = view.notice.clone();
    let previous_rank = campaign.actors[player].rank;
    let owner_color = campaign.economy.provinces[province]
        .owner
        .and_then(|owner| practice.players.get(owner))
        .map(|owner| PLAYER_COLORS[owner.color_index])
        .unwrap_or(province_panel::NEUTRAL);
    ctx.data_mut(|d| {
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
                            province_overview_body(
                                ui,
                                rect.bottom() - 12.0 * scale,
                                &campaign,
                                province,
                                player,
                                scale,
                            );
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
                                &old_notice,
                            ) {
                                view.notice = message;
                            }
                            return;
                        }
                        scroll_body_with_status(
                            ui,
                            rect.bottom() - 12.0 * scale,
                            scale,
                            &old_notice,
                            ("campaign_body", tab as usize, province, view.section),
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
                                        0 => province_overview(
                                            ui, &campaign, province, player, scale,
                                        ),
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
                                            let report_months: Vec<_> =
                                                (0..campaign.politics.len())
                                                    .map(|id| {
                                                        campaign
                                                            .intelligence
                                                            .get(&(player, id))
                                                            .and_then(|r| r.operations.as_ref())
                                                            .map(|r| r.month)
                                                    })
                                                    .collect();
                                            action = campaign_military::show(
                                                ui,
                                                &military,
                                                &campaign.economy,
                                                &campaign.graph,
                                                province,
                                                player,
                                                |id| observed.get(id).copied().unwrap_or(false),
                                                |id| report_months.get(id).copied().flatten(),
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
                                                diplomacy(ui, &mut campaign, province, player)
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
        view.notice = apply_military_action(&mut campaign, province, player, action);
    }
    if let Some(province) = army_province {
        open_military_province(ctx, &mut view, &mut detail, &mut map_view, province, player);
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
    } else if old_notice != view.notice {
        play_click(&sound, &audio, &assets);
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
fn apply_military_action(
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
        Disband(id) => {
            let p = &mut campaign.economy.provinces[province];
            campaign
                .military
                .disband(province, player, id, p.owner == Some(player), &mut p.population)
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
    result.map_or_else(|message| message, |_| "Order accepted.".to_owned())
}

/// Display sovereignty, exact domain-quoted prices, eligibility, spies and military access.
fn diplomacy(
    ui: &mut egui::Ui,
    campaign: &mut Campaign,
    province: usize,
    player: usize,
) -> Option<String> {
    let state = campaign.politics[province].state.clone();
    if state == PoliticalState::Rome {
        ui.label("Rome permits no provincial actions. Open the Senate to manage politics or declare a march on the capital.");
        return None;
    }
    let relation = campaign.politics[province].relation(player);
    let distance = campaign.distance(player, province);
    let config = campaign.diplomacy_config.clone();
    ui.horizontal_wrapped(|ui| {
        stat(ui, Icon::Coin, &format!("{:.0}", campaign.actors[player].coin), "Available Coin.");
        stat(ui, Icon::Influence, &format!("{:.0}", campaign.actors[player].influence), "Available Influence.");
        stat(ui, Icon::Trade, &distance.map_or_else(|| "—".into(), |d| d.to_string()), &distance.map_or_else(|| "No usable political route.".into(), |d| format!("Political distance: {d} edges. Spending multiplier ×{:.2}. Includes usable sea crossings.", distance_multiplier(Some(d)).unwrap_or(1.0))));
    });
    if !matches!(state, PoliticalState::Owned { owner } if owner == player) {
        diplomatic_meter(ui, Icon::Relation, "Relation", relation, "Friendship toward you. Political Control separately measures power; occupation can create high Control with low Relation.");
    }
    let mut result: Option<Result<(), PoliticalError>> = None;
    let mut message = None;
    match &state {
        PoliticalState::Rome => unreachable!("Rome has no diplomatic actions"),
        PoliticalState::Independent {
            local,
            shares,
        } => {
            ui.strong("Independent").on_hover_text("51 Control and a unique lead permit vassalization. 100 Control permits ownership. Vassalization is optional; initial Vassal Control equals Independent Control minus 50.");
            diplomatic_meter(
                ui,
                Icon::Province,
                "Local",
                *local,
                "Control retained by the independent local government.",
            );
            for (id, share) in shares.iter().enumerate() {
                if *share > 0.0 || id == player {
                    diplomatic_meter(ui, Icon::Control, &format!("P{}{}", id + 1, if id == player { " · you" } else { "" }), *share, "Independent Control share. All players and Local sum to 100. Monthly political pressure resolves simultaneously.");
                }
            }
            let pending = campaign.politics[province].queued_control_pressure(player);
            if pending > 0.0 {
                stat(ui, Icon::Control, &format!("+{pending:.1} pending"), "Your paid control and blackmail pressure awaits simultaneous month-end resolution. Rival pressure can change the final gain; this has not already been added to your Control.");
            }
            let mut preview = campaign.politics[province].clone();
            let vassal_eligible = preview.vassalize(player).is_ok();
            let mut preview = campaign.politics[province].clone();
            let ownership_eligible = preview.take_ownership(player).is_ok();
            ui.horizontal_wrapped(|ui| {
                if ui.add_enabled(vassal_eligible, egui::Button::new("Vassalize"))
                    .on_hover_text("Create a vassal; retain relations and remove every independent control share.")
                    .on_disabled_hover_text("Requires at least 51 Independent Control and a unique lead over every rival.").clicked() {
                    result = Some(campaign.politics[province].vassalize(player));
                }
                if ui.add_enabled(ownership_eligible, egui::Button::new("Integrate"))
                    .on_hover_text(format!("Directly own this province. Current relation produces a {:+.1} happiness shift for every class.", relation - 50.0))
                    .on_disabled_hover_text("Requires 100 Independent Control, resolved at the monthly boundary.").clicked() {
                    result = Some(integrate_province(campaign, province, player));
                }
            });
            ui.horizontal_wrapped(|ui| {
                for currency in [Currency::Coin, Currency::Influence] {
                    let quote = political_quote(&campaign.politics[province], &campaign.actors[player], currency,
                        |p, actor| p.buy_control(player, actor, currency, 5.0, distance, &config));
                    if political_button(ui, "+5 Control", currency, quote,
                        "Adds pressure to the next simultaneous monthly resolution. Distance, relation, your existing position and rival entrenchment determine the price.") {
                        result = Some(campaign.politics[province].buy_control(player, &mut campaign.actors[player], currency, 5.0, distance, &config));
                    }
                }
            });
        },
        PoliticalState::Vassal {
            overlord,
            control,
            ..
        } => {
            let delta = &campaign.politics[province].last_control_change;
            ui.horizontal_wrapped(|ui| {
                ui.strong(format!("Vassal · P{}", overlord + 1));
                stat(ui, Icon::Control, &format!("{:+.1}/mo", delta.total), &format!("Last monthly change. Relation {:+.2}; garrison {:+.2}; paid support {:+.2}. At 0 the province becomes independent; at 100 integration is optional.", delta.relation, delta.military, delta.support));
            });
            diplomatic_meter(ui, Icon::Control, "Control", *control, "Vassal stability and integration progress. Relation below 50 causes decay; military garrisons and paid government support can offset it.");
            if *overlord == player {
                if let PoliticalState::Vassal {
                    tribute,
                    ..
                } = &mut campaign.politics[province].state
                {
                    ui.horizontal_wrapped(|ui| {
                        ui.label("Tribute:");
                        for (level, label) in [(Tribute::Low, "Low"), (Tribute::Normal, "Normal"), (Tribute::High, "High")] {
                            ui.selectable_value(tribute, level, label).on_hover_text(format!("Pays {:.0}% of available monthly Coin output. Relation changes {:+.1} per month.", level.income_fraction() * 100.0, level.relation_delta()));
                        }
                    });
                }
                if ui.add_enabled(*control >= 100.0 - 1e-7, egui::Button::new("Integrate"))
                    .on_hover_text(format!("End vassal tribute and vassal Influence. Relation produces a {:+.1} happiness shift for every population class.", relation - 50.0))
                    .on_disabled_hover_text("Requires 100 Vassal Control. Integration is always your choice.").clicked() {
                    result = Some(integrate_province(campaign, province, player));
                }
                ui.horizontal_wrapped(|ui| {
                    for currency in [Currency::Coin, Currency::Influence] {
                        let quote = political_quote(
                            &campaign.politics[province],
                            &campaign.actors[player],
                            currency,
                            |p, actor| {
                                p.support_government(
                                    player, actor, currency, 5.0, distance, &config,
                                )
                            },
                        );
                        if political_button(
                            ui,
                            "+5 Control",
                            currency,
                            quote,
                            "Support the government directly. This does not improve Relation.",
                        ) {
                            result = Some(campaign.politics[province].support_government(
                                player,
                                &mut campaign.actors[player],
                                currency,
                                5.0,
                                distance,
                                &config,
                            ));
                        }
                    }
                });
            }
        },
        PoliticalState::Owned {
            owner,
        } => {
            ui.horizontal_wrapped(|ui| {
                icon(ui, Icon::Province, 22.0);
                ui.strong(format!("Owned · P{}", owner + 1)).on_hover_text(
                    "Domestic happiness replaces foreign Control in directly owned provinces.",
                );
            });
            if let Some(text) = military_diplomacy(ui, campaign, province, player, *owner) {
                message = Some(text);
            }
        },
    }
    if !matches!(campaign.politics[province].state, PoliticalState::Owned { .. }) {
        ui.separator();
        ui.horizontal_wrapped(|ui| {
            for currency in [Currency::Coin, Currency::Influence] {
                let quote = political_quote(&campaign.politics[province], &campaign.actors[player], currency,
                    |p, actor| p.improve_relation(player, actor, currency, 5.0, distance, &config));
                if political_button(ui, "+5 Relation", currency, quote, "Pay immediately. High existing friendship has diminishing returns; political distance also raises the price.") {
                    result = Some(campaign.politics[province].improve_relation(player, &mut campaign.actors[player], currency, 5.0, distance, &config));
                }
            }
        });
        ui.collapsing("Monthly support", |ui| {
        let route = distance_multiplier(distance);
        let support = &mut campaign.politics[province].support[player];
        for (enabled, currency, base, label) in [
            (&mut support.relation_coin, Currency::Coin, config.monthly_coin, "+1 Relation"),
            (
                &mut support.relation_influence,
                Currency::Influence,
                config.monthly_influence,
                "+1 Relation",
            ),
        ] {
            recurring_support(ui, enabled, currency, base, route, label);
        }
        if matches!(campaign.politics[province].state, PoliticalState::Vassal { overlord, .. } if overlord == player)
        {
            let support = &mut campaign.politics[province].support[player];
            recurring_support(
                ui,
                &mut support.control_coin,
                Currency::Coin,
                config.monthly_coin,
                route,
                "+1 Control",
            );
            recurring_support(
                ui,
                &mut support.control_influence,
                Currency::Influence,
                config.monthly_influence,
                route,
                "+1 Control",
            );
        }
        });
    }
    ui.separator();
    let foreign = !matches!(campaign.politics[province].state, PoliticalState::Owned { owner } if owner == player)
        && !matches!(campaign.politics[province].state, PoliticalState::Vassal { overlord, .. } if overlord == player);
    if foreign {
        let deployed = campaign
            .espionage
            .missions
            .iter()
            .any(|mission| mission.owner == player && mission.province == province);
        let spy_config = &campaign.espionage_config;
        let can_deploy = campaign.actors[player].influence >= spy_config.deployment_influence;
        let risk = campaign.intelligence.get(&(player, province)).map_or_else(
            || "Detection risk: ?".to_owned(),
            |report| {
                format!(
                    "Detection risk from month {} report: {:.1}% (Noble happiness {}).",
                    report.demographics.month,
                    crate::game::politics::espionage::detection_chance(
                        report.demographics.happiness[0],
                        spy_config
                    ) * 100.0,
                    report.demographics.happiness[0]
                )
            },
        );
        let age = campaign
            .espionage
            .missions
            .iter()
            .find(|mission| mission.owner == player && mission.province == province)
            .map_or(0, |mission| mission.months_active);
        ui.horizontal_wrapped(|ui| {
        icon(ui, Icon::Spy, 24.0).on_hover_text(format!("{} · {age} surviving months. {risk}\nMonth 1: population/happiness. Month 2: production/consumption/policies. Month 3: construction/troops/recruitment.\nReports refresh monthly after detection. Scandal discovery is separate. National stockpiles, treasury, tactics and movement destinations stay private.", if deployed { "Network active" } else { "No network" }));
        if deployed { ui.small(format!("Access {}/3", age.min(3))); }
        let label = if deployed { "Withdraw" } else { "Deploy spy" };
        if ui.add_enabled(deployed || can_deploy, egui::Button::new(label))
            .on_hover_text(format!("Deployment: {:.1} Influence. Maintenance: {:.1} Coin/month. Detection is checked before reports and separate scandal discovery. Information unlocks after 1 / 2 / 3 surviving months. Withdrawal or detection stops updates; redeployment starts at access 0.", spy_config.deployment_influence, spy_config.monthly_coin))
            .on_disabled_hover_text(format!("Requires {:.1} Influence to deploy; only one network per player and province is allowed.", spy_config.deployment_influence)).clicked() {
            if deployed { campaign.espionage.withdraw(player, province); message = Some("Spy network withdrawn.".into()); }
            else { result = Some(campaign.espionage.deploy(player, province, &mut campaign.actors, &campaign.politics, &campaign.espionage_config)); }
        }
        stat(ui, Icon::Coin, &format!("{:.0}/mo", spy_config.monthly_coin), "Monthly upkeep of a deployed spy network.");
        });
        if !matches!(campaign.politics[province].state, PoliticalState::Owned { .. }) {
            let hostile = campaign.npc_wars[player][province];
            let battling = campaign.military.province_in_battle(province);
            if hostile {
                if ui.add_enabled(!battling, egui::Button::new("End hostility"))
                    .on_hover_text("End invasion permission and military occupation. Peaceful access still requires friendly relations.")
                    .on_disabled_hover_text("Resolve or retreat from the battle before ending hostility.").clicked() {
                    campaign.npc_wars[player][province] = false;
                    if campaign.military.provinces[province].occupation == Some(ForceOwner::Player(player)) { campaign.military.provinces[province].occupation = None; }
                    message = Some("Hostility ended. Military occupation no longer creates Control.".into());
                }
            } else if ui.button("Declare hostility").on_hover_text("Permits invasion and starts a battle if hostile units coexist. NPC defenders must be defeated before occupation can generate Control.").clicked() {
                campaign.declare_hostility(player, province); message = Some("Hostility declared.".into());
            }
        }
    }
    let current_state = campaign.politics[province].state.clone();
    for rival in 0..campaign.actors.len() {
        if rival == player {
            continue;
        }
        let actions: &[Interference] = match current_state {
            PoliticalState::Independent {
                ..
            } => &[
                Interference::SmearCampaign,
                Interference::FundOpposition,
                Interference::UndermineRival,
            ],
            PoliticalState::Vassal {
                overlord,
                ..
            } if overlord == rival => &[
                Interference::SmearCampaign,
                Interference::PoliticalAgitation,
                Interference::FundDissidents,
            ],
            PoliticalState::Owned {
                owner,
            } if owner == rival => &[Interference::AgitatePopulation, Interference::FundUnrest],
            _ => &[],
        };
        if actions.is_empty() {
            continue;
        }
        egui::CollapsingHeader::new(format!("Rival · P{}", rival + 1))
            .id_salt((province, rival))
            .show(ui, |ui| {
                for &action in actions {
                    let (label, currency, explanation) = interference_details(action);
                    let quote = political_quote(
                        &campaign.politics[province],
                        &campaign.actors[player],
                        currency,
                        |p, actor| {
                            p.interfere(player, rival, actor, action, distance, &config).map(|_| ())
                        },
                    );
                    if political_button(ui, label, currency, quote, explanation) {
                        result = Some(
                            campaign.politics[province]
                                .interfere(
                                    player,
                                    rival,
                                    &mut campaign.actors[player],
                                    action,
                                    distance,
                                    &config,
                                )
                                .map(|changes| {
                                    campaign
                                        .notifications
                                        .record_foreign_happiness(player, rival, province, changes);
                                    for (class, change) in changes.into_iter().enumerate() {
                                        campaign.economy.provinces[province].temporary_happiness
                                            [class] += change;
                                    }
                                }),
                        );
                    }
                }
            });
    }
    if let Some(text) = campaign_politics::npc_evidence(
        ui,
        &mut campaign.espionage,
        &mut campaign.politics,
        player,
        province,
        campaign.economy.month,
        &campaign.espionage_config,
    ) {
        message = Some(text);
    }
    if matches!(campaign.politics[province].state, PoliticalState::Owned { .. }) {
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
            ui.separator();
            ui.collapsing(format!("Evidence · {}", evidence.len()), |ui| {
            for scandal in evidence {
                ui.horizontal_wrapped(|ui| {
                    icon(ui, Icon::Spy, 18.0);
                    ui.label(scandal.kind.label()).on_hover_text(format!("{:?} · expires in {} months. Expose player evidence in the Senate to undermine faction loyalties or force an unpopular Consul to resign.", scandal.severity, scandal.expires.saturating_sub(campaign.economy.month)));
                });
            }
            });
        }
    }
    result
        .map(|value| {
            value.map_or_else(|error| error.to_string(), |_| "Political action accepted.".into())
        })
        .or(message)
}

#[cfg(test)]
#[path = "../../tests/unit/ui_layout.rs"]
mod layout_tests;

#[cfg(test)]
#[path = "../../tests/unit/buildings_ui.rs"]
mod buildings_ui;

/// Apply integration's already-computed relation shift once while preserving other causes.
fn integrate_province(
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

/// A validated price and the original actor's ability to pay it.
#[derive(Debug, Clone, Copy)]
struct PoliticalQuote {
    cost: f64,
    affordable: bool,
}

/// A compact illustrated meter puts the rule explanation on the entire row's hover.
fn diplomatic_meter(ui: &mut egui::Ui, symbol: Icon, label: &str, value: f64, explanation: &str) {
    ui.horizontal(|ui| {
        icon(ui, symbol, 21.0);
        ui.add_sized([61.0, 20.0], egui::Label::new(label));
        ui.add(
            egui::ProgressBar::new((value / 100.0).clamp(0.0, 1.0) as f32)
                .desired_width(ui.available_width().max(1.0))
                .text(format!("{value:.1}")),
        );
    })
    .response
    .on_hover_text(explanation);
}

/// Run the authoritative operation on cloned state so UI prices and legality cannot
/// diverge from the actual action. No pending pressure or wallet change escapes the preview.
fn political_quote(
    province: &ProvincePolitics,
    actor: &PoliticalPlayer,
    currency: Currency,
    operation: impl FnOnce(&mut ProvincePolitics, &mut PoliticalPlayer) -> Result<(), PoliticalError>,
) -> Result<PoliticalQuote, PoliticalError> {
    let mut preview = province.clone();
    let mut wallet = actor.clone();
    // The panel quotes five-point actions. This finite budget safely exceeds every
    // configured price while retaining sub-cent precision in the subtraction.
    let preview_budget = 1_000_000_000.0;
    wallet.coin = preview_budget;
    wallet.influence = preview_budget;
    operation(&mut preview, &mut wallet)?;
    let (remaining, available) = match currency {
        Currency::Coin => (wallet.coin, actor.coin),
        Currency::Influence => (wallet.influence, actor.influence),
    };
    let cost = (preview_budget - remaining).max(0.0);
    Ok(PoliticalQuote {
        cost,
        affordable: available + 0.000_001 >= cost,
    })
}

/// Render a priced action with consistent explanations when disabled or affordable.
fn political_button(
    ui: &mut egui::Ui,
    label: &str,
    currency: Currency,
    quote: Result<PoliticalQuote, PoliticalError>,
    explanation: &str,
) -> bool {
    let name = currency_label(currency);
    let (enabled, tooltip) = match quote {
        Ok(quote) => (
            quote.affordable,
            if quote.affordable {
                format!("{label} · {:.1} {name}\n{explanation}", quote.cost)
            } else {
                format!(
                    "Requires {:.1} {name}; insufficient available funds.\n{explanation}",
                    quote.cost
                )
            },
        ),
        Err(error) => (false, format!("{error}\n{explanation}")),
    };
    let mut clicked = false;
    ui.horizontal(|ui| {
        icon(ui, currency_icon(currency), 20.0).on_hover_text(name);
        clicked = ui
            .add_enabled(enabled, egui::Button::new(label))
            .on_hover_text(&tooltip)
            .on_disabled_hover_text(tooltip)
            .clicked();
    });
    clicked
}

/// Use the same illustrated resource identity in every priced political control.
fn currency_icon(currency: Currency) -> Icon {
    match currency {
        Currency::Coin => Icon::Coin,
        Currency::Influence => Icon::Influence,
    }
}

/// Common currency labels keep configuration-driven prices readable.
fn currency_label(currency: Currency) -> &'static str {
    match currency {
        Currency::Coin => "Coin",
        Currency::Influence => "Influence",
    }
}

/// Display a recurring program's actual routed monthly cost; enabled programs may
/// always be switched off, including after their political route becomes invalid.
fn recurring_support(
    ui: &mut egui::Ui,
    enabled: &mut bool,
    currency: Currency,
    base_cost: f64,
    route: Result<f64, PoliticalError>,
    effect: &str,
) {
    let (cost, valid) = match route {
        Ok(distance) => {
            (format!("{:.1} {}/month", base_cost * distance, currency_label(currency)), true)
        },
        Err(_) => ("No usable route".into(), false),
    };
    ui.horizontal(|ui| {
    icon(ui, currency_icon(currency), 20.0);
    ui.add_enabled(valid || *enabled, egui::Checkbox::new(enabled, effect))
        .on_hover_text(format!("{cost}. Pays once each month if the route is usable and sufficient funds are available. Otherwise skips the month without debt."))
        .on_disabled_hover_text("A usable political connection is required before enabling this program.");
    });
}

/// Map each allowed hostile action to its actual payment resource and distinct effect.
fn interference_details(action: Interference) -> (&'static str, Currency, &'static str) {
    match action {
        Interference::SmearCampaign => ("Smear", Currency::Influence, "Lowers only this rival's Relation. Limited to one smear per actor, rival and province each month. Strong existing friendship resists the action."),
        Interference::FundOpposition => ("Opposition", Currency::Coin, "Reduces this rival's Independent Control at monthly resolution and returns it to Local. You gain no Control. Entrenchment and friendship increase the cost."),
        Interference::UndermineRival => ("Undermine", Currency::Influence, "Reduces this rival's Independent Control at monthly resolution and returns it to Local. You gain no Control."),
        Interference::PoliticalAgitation => ("Agitate", Currency::Influence, "Immediately reduces enemy Vassal Control. At zero the province becomes independent. It does not give you a control share."),
        Interference::FundDissidents => ("Dissidents", Currency::Coin, "Lowers Relation toward the overlord. Poorer relations increase future Vassal Control decay; this does not directly remove Control."),
        Interference::AgitatePopulation => ("Agitate", Currency::Influence, "Reduces Citizen and Plebeian happiness in the rival's owned province. Ownership and Control do not change. The action is attributed to you."),
        Interference::FundUnrest => ("Unrest", Currency::Coin, "Reduces Plebeian happiness in the rival's owned province. Ownership and Control do not change. The action is attributed to you."),
    }
}

/// Global player access and bilateral peace controls, surfaced from owned-province context.
fn military_diplomacy(
    ui: &mut egui::Ui,
    campaign: &mut Campaign,
    province: usize,
    player: usize,
    owner: usize,
) -> Option<String> {
    let mut message = None;
    if owner == player {
        ui.collapsing("Military access", |ui| {
            for guest in 0..campaign.actors.len() {
                if guest == player { continue; }
                let war = campaign.wars[player][guest];
                let mut invited = campaign.invitations[player][guest];
                if ui.add_enabled(!war, egui::Checkbox::new(&mut invited, format!("Invite Player {}", guest + 1)))
                    .on_hover_text("Permit peaceful military entry. Uncheck to revoke future entry; stationed units are not automatically expelled.")
                    .on_disabled_hover_text("Military invitations are unavailable between players at war.").changed() {
                    campaign.invitations[player][guest] = invited;
                    message = Some(format!("Military access for Player {} {}.", guest + 1, if invited { "granted" } else { "revoked" }));
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
            .on_hover_text("End bilateral hostility. Existing ownership remains unchanged. Peaceful entry still requires the owner's invitation.")
            .on_disabled_hover_text("Resolve or retreat from your active battles against this player before making peace.").clicked() {
            campaign.wars[player][owner] = false; campaign.wars[owner][player] = false;
            message = Some(format!("Peace established with Player {}.", owner + 1));
        }
    } else {
        stat(ui, Icon::Trade, if campaign.invitations[owner][player] { "Access granted" } else { "Access closed" }, "Peaceful military entry requires the owner's invitation. Peaceful stationing grants no occupation Control.");
        if ui.button("Declare hostility").on_hover_text("Ends military invitations between both players. Entering hostile territory begins combat; victory captures an owned province immediately.").clicked() {
            campaign.invitations[player][owner] = false; campaign.invitations[owner][player] = false;
            campaign.declare_hostility(player, province);
            message = Some(format!("Hostility declared against Player {}.", owner + 1));
        }
    }
    message
}

/// Read construction identity before and after the building UI to emit its one start event.
fn construction_underway(campaign: &Campaign, province: usize) -> Option<Icon> {
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
        ui.label("No notifications from this province yet.");
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
