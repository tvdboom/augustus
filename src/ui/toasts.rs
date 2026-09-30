//! Short-lived, stacked and actionable Augustus map notices.

use std::collections::VecDeque;

use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts};

use super::{
    play_click, viewport_ui_scale, ActiveGame, AppState, AudioMode, GovernancePanelOpen,
    HudResource, HudResources, LocalPractice, MenuAudio, ProvinceOwnership, TerminalPresentation,
    POP_CLASS_NAMES,
};
use bevy_kira_audio::prelude::{Audio, AudioControl};

const TOAST_SECONDS: f32 = 5.0;
const MAX_TOASTS: usize = 7;
use super::campaign_notifications::{NoticeKind, NoticeSeverity};
use crate::game::economy::UNHAPPINESS_THRESHOLDS;
const FORECAST_MONTHS: f64 = 3.0;
const LOW_FOOD: f64 = 10.0;
const LOW_COIN: f64 = 25.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::app) enum ToastLevel {
    Info,
    Warning,
    Error,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::app) enum ToastAction {
    OpenGovernance,
    OpenProvince(usize),
    FocusWonder(usize),
    /// Navigate to a province's evidence controls or the global Senate inventory.
    OpenEvidence(Option<usize>),
}

#[derive(Clone, Debug)]
pub(in crate::app) struct Toast {
    text: String,
    title: Option<&'static str>,
    level: ToastLevel,
    play_sound: bool,
    action: Option<ToastAction>,
    seconds_left: f32,
    notice: Option<super::campaign_notifications::CampaignNotice>,
    month: Option<u32>,
}

impl Toast {
    pub(in crate::app) fn info(text: impl Into<String>) -> Self {
        Self::new(text, ToastLevel::Info)
    }

    pub(in crate::app) fn warning(text: impl Into<String>) -> Self {
        Self::new(text, ToastLevel::Warning)
    }

    #[allow(dead_code)]
    pub(in crate::app) fn error(text: impl Into<String>) -> Self {
        Self::new(text, ToastLevel::Error)
    }

    fn new(text: impl Into<String>, level: ToastLevel) -> Self {
        Self {
            text: text.into(),
            title: None,
            level,
            play_sound: true,
            action: None,
            seconds_left: TOAST_SECONDS,
            notice: None,
            month: None,
        }
    }

    pub(in crate::app) fn with_action(mut self, action: ToastAction) -> Self {
        self.action = Some(action);
        self
    }

    pub(in crate::app) fn with_title(mut self, title: &'static str) -> Self {
        self.title = Some(title);
        self
    }

    pub(in crate::app) fn without_sound(mut self) -> Self {
        self.play_sound = false;
        self
    }

    pub(in crate::app) fn with_notice(
        mut self,
        notice: super::campaign_notifications::CampaignNotice,
    ) -> Self {
        self.notice = Some(notice);
        self
    }

    fn message(&self, month: u32) -> super::campaign_notices::Message<'_> {
        use super::campaign_widgets::Icon;
        let (icon, title) = if let Some(title) = self.title {
            (Icon::Events, title)
        } else if self.text.contains("Food stores") {
            (Icon::Food, "Low food stocks")
        } else if self.text.contains("treasury") {
            (Icon::Coin, "Low treasury")
        } else if self.text.contains("unhappy") {
            (Icon::Happiness, "Population unhappy")
        } else if self.text == "Local practice started." {
            (Icon::Eagle, "Local practice")
        } else {
            (Icon::Notifications, "Campaign update")
        };
        super::campaign_notices::Message {
            icon,
            title,
            body: &self.text,
            month,
            warning: self.level != ToastLevel::Info,
            actionable: self.action.is_some(),
        }
    }
}

#[derive(Resource, Default)]
pub(in crate::app) struct ToastQueue(VecDeque<Toast>, [bool; 3]);

impl ToastQueue {
    pub(in crate::app) fn push(&mut self, toast: Toast) {
        // Domain notification IDs and WarningWatch own deduplication. Text alone
        // cannot distinguish a fresh relapse after recovery from the old warning.
        if toast.play_sound {
            self.1[toast.level as usize] = true;
        }
        self.0.push_back(toast);
        while self.0.len() > MAX_TOASTS {
            self.0.pop_front();
        }
    }

    pub(in crate::app) fn clear(&mut self) {
        self.0.clear();
        self.1 = [false; 3];
    }
}

/// Plays at most one shared audio cue of each severity for each update.
pub(in crate::app) fn play_pending_sounds(
    mut toasts: ResMut<ToastQueue>,
    sound: Res<MenuAudio>,
    audio: Res<Audio>,
    assets: Res<AssetServer>,
    terminal: Res<TerminalPresentation>,
) {
    let pending = std::mem::take(&mut toasts.1);
    if terminal.outcome.is_some() || sound.mode == AudioMode::Mute || sound.volume <= 0.001 {
        return;
    }
    let decibels = 20.0 * sound.volume.clamp(0.001, 1.0).log10();
    for (play, path) in
        pending.into_iter().zip(["audio/message.ogg", "audio/warning.ogg", "audio/error.ogg"])
    {
        if play {
            audio.play(assets.load(path)).with_volume(decibels);
        }
    }
}

/// Tracks active warnings so a prolonged shortage produces one toast until it recovers.
#[derive(Resource, Default)]
pub(in crate::app) struct WarningWatch {
    player: Option<usize>,
    active: [bool; 6],
    announced_game: bool,
}

impl WarningWatch {
    fn observe(
        &mut self,
        player: usize,
        resources: [HudResource; 7],
        happiness: [HudResource; 4],
        campaign: &mut super::campaign::Campaign,
        toasts: &mut ToastQueue,
    ) {
        if self.player != Some(player) {
            toasts.clear();
            self.player = Some(player);
            self.active = [false; 6];
        }
        if !self.announced_game {
            toasts.push(
                Toast::info("Local practice started.").with_action(ToastAction::OpenGovernance),
            );
            self.announced_game = true;
        }

        let conditions = warning_conditions(resources, happiness);
        for (index, is_active) in conditions.into_iter().enumerate() {
            if is_active && !self.active[index] {
                if index < POP_CLASS_NAMES.len() {
                    if let Some(province) = most_unhappy_province(campaign, player, index) {
                        let name = campaign.economy.provinces[province].name.clone();
                        campaign.notifications.province_notice(
                            player,
                            province,
                            campaign.economy.month,
                            NoticeSeverity::Warning,
                            NoticeKind::PopulationUnhappy(index),
                            "Population unhappy",
                            format!("{} are unhappy in {name}.", POP_CLASS_NAMES[index]),
                        );
                    } else {
                        toasts.push(
                            Toast::warning(format!("{} are unhappy.", POP_CLASS_NAMES[index]))
                                .with_action(ToastAction::OpenGovernance),
                        );
                    }
                } else {
                    let text = if index == 4 {
                        "Food stores are almost empty."
                    } else {
                        "The treasury is almost empty."
                    };
                    toasts.push(Toast::warning(text).with_action(ToastAction::OpenGovernance));
                }
            }
            self.active[index] = is_active;
        }
    }
}

/// The HUD average is weighted across owned residents; show the province with the
/// lowest happiness for the affected class when that average becomes unhappy.
fn most_unhappy_province(
    campaign: &super::campaign::Campaign,
    player: usize,
    class: usize,
) -> Option<usize> {
    campaign
        .economy
        .provinces
        .iter()
        .enumerate()
        .filter(|(_, province)| province.owner == Some(player) && province.population[class] > 0.0)
        .min_by(|(_, a), (_, b)| a.happiness[class].total_cmp(&b.happiness[class]))
        .map(|(id, _)| id)
}

fn almost_empty(stock: HudResource, low_stock: f64) -> bool {
    stock.monthly_delta <= 0.0
        && stock.amount <= low_stock.max(-stock.monthly_delta * FORECAST_MONTHS)
}

fn warning_conditions(resources: [HudResource; 7], happiness: [HudResource; 4]) -> [bool; 6] {
    let mut warnings = [false; 6];
    for class in 0..4 {
        warnings[class] = happiness[class].amount < UNHAPPINESS_THRESHOLDS[class];
    }
    warnings[4] = almost_empty(resources[0], LOW_FOOD);
    warnings[5] = almost_empty(resources[3], LOW_COIN);
    warnings
}

pub(in crate::app) fn watch_warnings(
    state: Res<State<AppState>>,
    game: Res<ActiveGame>,
    practice: Res<LocalPractice>,
    ownership: Res<ProvinceOwnership>,
    resources: Res<HudResources>,
    mut campaign: ResMut<super::campaign::Campaign>,
    mut watch: ResMut<WarningWatch>,
    mut toasts: ResMut<ToastQueue>,
) {
    if *state.get() != AppState::Map
        || *game != ActiveGame::LocalPractice
        || practice.players.is_empty()
    {
        return;
    }
    let player = practice.active_player.min(practice.players.len() - 1);
    let mut current = resources.for_player(player);
    current[0].monthly_delta = ownership.net_production_for(player)[0];
    current[3].monthly_delta = ownership.coin_delta_for(player);
    watch.observe(player, current, resources.happiness_for(player), &mut campaign, &mut toasts);
}

pub(in crate::app) fn advance(
    time: Res<Time>,
    state: Res<State<AppState>>,
    game: Res<ActiveGame>,
    mut toasts: ResMut<ToastQueue>,
) {
    if *state.get() != AppState::Map || *game != ActiveGame::LocalPractice {
        return;
    }
    for toast in &mut toasts.0 {
        toast.seconds_left -= time.delta_secs();
    }
    toasts.0.retain(|toast| toast.seconds_left > 0.0);
}

pub(in crate::app) fn draw(
    mut contexts: EguiContexts,
    state: Res<State<AppState>>,
    game: Res<ActiveGame>,
    mut toasts: ResMut<ToastQueue>,
    mut governance_open: ResMut<GovernancePanelOpen>,
    mut province_open: ResMut<super::ProvincePanelOpen>,
    mut campaign_ui: ResMut<super::campaign_panel::CampaignUi>,
    mut map_view: ResMut<crate::map::MapView>,
    sound: Res<MenuAudio>,
    audio: Res<Audio>,
    assets: Res<AssetServer>,
    campaign: Res<super::campaign::Campaign>,
    terminal: Res<TerminalPresentation>,
) {
    if terminal.spectating
        || *state.get() != AppState::Map
        || *game != ActiveGame::LocalPractice
        || toasts.0.is_empty()
    {
        return;
    }
    let Ok(context) = contexts.ctx_mut() else {
        return;
    };
    let viewport = context.content_rect();
    let scale = viewport_ui_scale(viewport.size());
    // Align with the right edge, below the settings and volume controls.
    let right_inset = 12.0 * scale;
    let max_width = (viewport.width() - right_inset - 28.0 * scale).min(420.0 * scale);
    let mut clicked = None;
    egui::Area::new(egui::Id::new("augustus_toasts"))
        .anchor(egui::Align2::RIGHT_TOP, egui::vec2(-right_inset, 76.0 * scale))
        .order(egui::Order::Foreground)
        .interactable(true)
        .show(context, |ui| {
            ui.set_max_width(max_width);
            ui.spacing_mut().item_spacing.y = 6.0 * scale;
            for (index, toast) in toasts.0.iter_mut().enumerate() {
                if let Some(notice) = &toast.notice {
                    ui.push_id(notice.id, |ui| {
                        if super::campaign_notices::card(ui, notice, scale).clicked() {
                            clicked =
                                Some((index, toast.action.unwrap_or(ToastAction::OpenGovernance)));
                        }
                    });
                    continue;
                }
                let month = *toast.month.get_or_insert(campaign.economy.month);
                let response =
                    super::campaign_notices::message_card(ui, toast.message(month), scale);
                if let Some(action) = toast.action {
                    if response.clicked() {
                        clicked = Some((index, action));
                    }
                }
            }
        });
    if let Some((index, action)) = clicked {
        let toast = toasts.0.remove(index).expect("clicked notice exists");
        if let Some(notice) = toast.notice {
            governance_open.0 = false;
            if notice.kind == super::campaign_notifications::NoticeKind::TradeInterrupted {
                super::campaign_trade::open_routes(context, notice.recipient);
            }
            super::campaign_panel::open_notification(
                &mut campaign_ui,
                &notice,
                &campaign,
                &mut province_open,
                &mut map_view,
            );
        } else {
            match action {
                ToastAction::OpenGovernance => {
                    governance_open.0 = true;
                    campaign_ui.open = None;
                    province_open.0 = None;
                },
                ToastAction::OpenProvince(id) => {
                    province_open.0 = Some(super::MapDetail::Province(id));
                    map_view.focus_province(id);
                },
                ToastAction::FocusWonder(id) => map_view.focus_wonder(id),
                ToastAction::OpenEvidence(province) => {
                    campaign_ui.open_evidence(province);
                    if let Some(province) = province {
                        province_open.0 = Some(super::MapDetail::Province(province));
                        map_view.focus_province(province);
                    } else {
                        province_open.0 = None;
                    }
                },
            }
        }
        play_click(&sound, &audio, &assets);
    }
}

#[cfg(test)]
#[path = "../../tests/unit/toasts.rs"]
mod tests;
