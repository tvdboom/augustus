//! Short-lived map notices, following Stellarion's stacked and actionable toast pattern.

use std::collections::VecDeque;

use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts};

use super::{
    play_click, viewport_ui_scale, ActiveGame, AppState, AudioMode, GovernancePanelOpen,
    HudResource, HudResources, LocalPractice, MenuAudio, ProvinceOwnership, POP_CLASS_NAMES,
};
use bevy_kira_audio::prelude::{Audio, AudioControl};

const TOAST_SECONDS: f32 = 5.0;
const MAX_TOASTS: usize = 7;
const HAPPINESS_LIMITS: [f64; 4] = [40.0, 30.0, 20.0, 10.0];
const FORECAST_MONTHS: f64 = 3.0;
const LOW_FOOD: f64 = 10.0;
const LOW_COIN: f64 = 25.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum ToastLevel {
    Info,
    Warning,
    Error,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum ToastAction {
    OpenGovernance,
}

#[derive(Clone, Debug)]
pub(super) struct Toast {
    text: String,
    level: ToastLevel,
    action: Option<ToastAction>,
    seconds_left: f32,
}

impl Toast {
    pub(super) fn info(text: impl Into<String>) -> Self {
        Self::new(text, ToastLevel::Info)
    }

    pub(super) fn warning(text: impl Into<String>) -> Self {
        Self::new(text, ToastLevel::Warning)
    }

    #[allow(dead_code)]
    pub(super) fn error(text: impl Into<String>) -> Self {
        Self::new(text, ToastLevel::Error)
    }

    fn new(text: impl Into<String>, level: ToastLevel) -> Self {
        Self {
            text: text.into(),
            level,
            action: None,
            seconds_left: TOAST_SECONDS,
        }
    }

    pub(super) fn with_action(mut self, action: ToastAction) -> Self {
        self.action = Some(action);
        self
    }
}

#[derive(Resource, Default)]
pub(super) struct ToastQueue(VecDeque<Toast>, [bool; 3]);

impl ToastQueue {
    pub(super) fn push(&mut self, toast: Toast) {
        self.1[toast.level as usize] = true;
        self.0.push_back(toast);
        while self.0.len() > MAX_TOASTS {
            self.0.pop_front();
        }
    }

    pub(super) fn clear(&mut self) {
        self.0.clear();
        self.1 = [false; 3];
    }
}

/// Plays at most one Stellarion cue of each severity for each update.
pub(super) fn play_pending_sounds(
    mut toasts: ResMut<ToastQueue>,
    sound: Res<MenuAudio>,
    audio: Res<Audio>,
    assets: Res<AssetServer>,
) {
    let pending = std::mem::take(&mut toasts.1);
    if sound.mode == AudioMode::Mute || sound.volume <= 0.001 {
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
pub(super) struct WarningWatch {
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
        toasts: &mut ToastQueue,
    ) {
        if self.player != Some(player) {
            toasts.clear();
            self.player = Some(player);
            self.active = [false; 6];
        }
        if !self.announced_game {
            toasts.push(Toast::info("Local practice started."));
            self.announced_game = true;
        }

        let conditions = warning_conditions(resources, happiness);
        for (index, is_active) in conditions.into_iter().enumerate() {
            if is_active && !self.active[index] {
                let text = match index {
                    0..=3 => format!("{} are unhappy.", POP_CLASS_NAMES[index]),
                    4 => "Food stores are almost empty.".to_owned(),
                    _ => "The treasury is almost empty.".to_owned(),
                };
                toasts.push(Toast::warning(text).with_action(ToastAction::OpenGovernance));
            }
            self.active[index] = is_active;
        }
    }
}

fn almost_empty(stock: HudResource, low_stock: f64) -> bool {
    stock.monthly_delta <= 0.0
        && stock.amount <= low_stock.max(-stock.monthly_delta * FORECAST_MONTHS)
}

fn warning_conditions(resources: [HudResource; 7], happiness: [HudResource; 4]) -> [bool; 6] {
    let mut warnings = [false; 6];
    for class in 0..4 {
        warnings[class] = happiness[class].amount < HAPPINESS_LIMITS[class];
    }
    warnings[4] = almost_empty(resources[0], LOW_FOOD);
    warnings[5] = almost_empty(resources[3], LOW_COIN);
    warnings
}

pub(super) fn watch_warnings(
    state: Res<State<AppState>>,
    game: Res<ActiveGame>,
    practice: Res<LocalPractice>,
    ownership: Res<ProvinceOwnership>,
    resources: Res<HudResources>,
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
    watch.observe(player, current, resources.happiness_for(player), &mut toasts);
}

pub(super) fn advance(
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

pub(super) fn draw(
    mut contexts: EguiContexts,
    state: Res<State<AppState>>,
    game: Res<ActiveGame>,
    mut toasts: ResMut<ToastQueue>,
    mut governance_open: ResMut<GovernancePanelOpen>,
    sound: Res<MenuAudio>,
    audio: Res<Audio>,
    assets: Res<AssetServer>,
) {
    if *state.get() != AppState::Map || *game != ActiveGame::LocalPractice || toasts.0.is_empty() {
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
            for (index, toast) in toasts.0.iter().enumerate() {
                let (fill, accent) = match toast.level {
                    ToastLevel::Info => (
                        egui::Color32::from_rgba_unmultiplied(49, 34, 25, 242),
                        egui::Color32::from_rgb(213, 167, 112),
                    ),
                    ToastLevel::Warning => (
                        egui::Color32::from_rgba_unmultiplied(65, 47, 21, 244),
                        egui::Color32::from_rgb(245, 198, 83),
                    ),
                    ToastLevel::Error => (
                        egui::Color32::from_rgba_unmultiplied(70, 30, 29, 244),
                        egui::Color32::from_rgb(244, 120, 105),
                    ),
                };
                let response = egui::Frame::new()
                    .fill(fill)
                    .stroke(egui::Stroke::new(scale, accent))
                    .corner_radius(5.0 * scale)
                    .inner_margin(egui::Margin::symmetric(12, 8))
                    .show(ui, |ui| {
                        ui.add(
                            egui::Label::new(
                                egui::RichText::new(&toast.text).size(14.0 * scale).color(accent),
                            )
                            .wrap(),
                        );
                    })
                    .response;
                if let Some(action) = toast.action {
                    if response
                        .interact(egui::Sense::click())
                        .on_hover_cursor(egui::CursorIcon::PointingHand)
                        .on_hover_text("Open Governance")
                        .clicked()
                    {
                        clicked = Some((index, action));
                    }
                }
            }
        });
    if let Some((index, action)) = clicked {
        toasts.0.remove(index);
        match action {
            ToastAction::OpenGovernance => governance_open.0 = true,
        }
        play_click(&sound, &audio, &assets);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn resource(amount: f64, monthly_delta: f64) -> HudResource {
        HudResource {
            amount,
            monthly_delta,
        }
    }

    #[test]
    fn happiness_warnings_use_strict_class_thresholds() {
        let mut happiness = [resource(50.0, 0.0); 4];
        let resources = [resource(100.0, 1.0); 7];
        happiness[0].amount = 40.0;
        happiness[1].amount = 30.0;
        happiness[2].amount = 20.0;
        happiness[3].amount = 10.0;
        assert_eq!(warning_conditions(resources, happiness), [false; 6]);
        for class in 0..4 {
            happiness[class].amount -= 0.1;
        }
        assert_eq!(
            warning_conditions(resources, happiness),
            [true, true, true, true, false, false]
        );
    }

    #[test]
    fn shortages_use_three_month_forecast_and_require_nonpositive_net() {
        let happiness = [resource(50.0, 0.0); 4];
        let mut resources = [resource(100.0, 1.0); 7];
        resources[0] = resource(30.0, -10.0);
        resources[3] = resource(24.0, 0.0);
        assert_eq!(warning_conditions(resources, happiness)[4..], [true, true]);
        resources[0].amount = 30.1;
        resources[3].monthly_delta = 1.0;
        assert_eq!(warning_conditions(resources, happiness)[4..], [false, false]);
    }

    #[test]
    fn warning_only_repeats_after_recovery() {
        let mut watch = WarningWatch::default();
        let mut toasts = ToastQueue::default();
        let mut resources = [resource(100.0, 1.0); 7];
        let happiness = [resource(50.0, 0.0); 4];
        resources[0] = resource(9.0, 0.0);
        watch.observe(0, resources, happiness, &mut toasts);
        assert_eq!(toasts.0.len(), 2); // Welcome info and food warning.
        watch.observe(0, resources, happiness, &mut toasts);
        assert_eq!(toasts.0.len(), 2);
        resources[0].amount = 20.0;
        watch.observe(0, resources, happiness, &mut toasts);
        resources[0].amount = 9.0;
        watch.observe(0, resources, happiness, &mut toasts);
        assert_eq!(toasts.0.len(), 3);
    }

    #[test]
    fn queued_toasts_request_one_sound_per_severity() {
        let mut toasts = ToastQueue::default();
        toasts.push(Toast::warning("First warning"));
        toasts.push(Toast::warning("Second warning"));
        toasts.push(Toast::info("Information"));
        toasts.push(Toast::error("Failure"));
        assert_eq!(toasts.1, [true, true, true]);
        toasts.clear();
        assert_eq!(toasts.1, [false; 3]);
    }
}
