//! Viewer-local playback of recorded rounds, independent of the campaign clock.
use super::*;

const STATE_ID: &str = "campaign-battle-replay";
pub(super) const CONTROLS_WIDTH: f32 = 198.;
const ROUND_SECONDS: f64 = 0.75;
const SPEEDS: [f64; 4] = [0.5, 1., 2., 3.];

#[derive(Clone, Copy)]
enum ReplayIcon {
    Play,
    Pause,
    Speed(usize),
    End,
}

#[derive(Clone)]
pub(super) struct Replay {
    pub cursor: usize,
    pub playing: bool,
    speed: usize,
    elapsed: f64,
    last_time: f64,
}

impl Replay {
    pub fn load(ctx: &egui::Context, outcome: &BattleOutcome) -> Self {
        let now = ctx.input(|input| input.time);
        let mut replay = ctx.data(|data| data.get_temp(egui::Id::new(STATE_ID))).unwrap_or(Self {
            cursor: outcome.round_history.len(),
            playing: false,
            speed: 1,
            elapsed: 0.,
            last_time: now,
        });
        let delta = (now - replay.last_time).max(0.);
        replay.last_time = now;
        if replay.playing {
            replay.elapsed += delta * SPEEDS[replay.speed];
            let steps = (replay.elapsed / ROUND_SECONDS).floor() as usize;
            replay.elapsed %= ROUND_SECONDS;
            replay.cursor = replay.cursor.saturating_add(steps).min(outcome.round_history.len());
            if replay.cursor == outcome.round_history.len() {
                replay.playing = false;
            }
        }
        replay
    }

    pub fn save(&self, ctx: &egui::Context) {
        ctx.data_mut(|data| data.insert_temp(egui::Id::new(STATE_ID), self.clone()));
        if self.playing {
            ctx.request_repaint_after(std::time::Duration::from_millis(16));
        }
    }

    pub fn frame<'a>(&self, outcome: &'a BattleOutcome) -> Option<&'a BattleReplayFrame> {
        if self.cursor == outcome.round_history.len() {
            None
        } else if self.cursor == 0 {
            outcome.replay_start.as_ref()
        } else {
            outcome.round_history.get(self.cursor - 1)?.replay.as_ref()
        }
    }
}

pub(super) fn clear(ctx: &egui::Context) {
    ctx.data_mut(|data| data.remove::<Replay>(egui::Id::new(STATE_ID)));
}

fn available(outcome: &BattleOutcome) -> bool {
    outcome.replay_start.is_some()
        && !outcome.round_history.is_empty()
        && outcome.round_history.iter().all(|round| round.replay.is_some())
}

fn button(
    ui: &egui::Ui,
    rect: egui::Rect,
    id: &str,
    icon: ReplayIcon,
    enabled: bool,
    scale: f32,
) -> egui::Response {
    let response = ui.interact(
        rect,
        ui.id().with(id),
        if enabled {
            egui::Sense::click()
        } else {
            egui::Sense::hover()
        },
    );
    let visuals = if enabled {
        ui.style().interact(&response)
    } else {
        &ui.visuals().widgets.noninteractive
    };
    ui.painter().rect(
        rect,
        3. * scale,
        visuals.bg_fill,
        egui::Stroke::new(scale, visuals.bg_stroke.color),
        egui::StrokeKind::Inside,
    );
    let ink = if enabled {
        INK
    } else {
        RULE
    };
    let point = |x, y| rect.center() + egui::vec2(x, y) * scale;
    let triangle = |left, right, center_y, height| {
        ui.painter().add(egui::Shape::convex_polygon(
            vec![
                point(left, center_y - height),
                point(right, center_y),
                point(left, center_y + height),
            ],
            ink,
            egui::Stroke::NONE,
        ));
    };
    let label = match icon {
        ReplayIcon::Play => {
            triangle(-4., 6., 0., 6.);
            "Play recorded battle".to_owned()
        },
        ReplayIcon::Pause => {
            for left in [-5., 2.] {
                ui.painter().rect_filled(
                    egui::Rect::from_min_max(point(left, -6.), point(left + 3., 6.)),
                    scale * 0.5,
                    ink,
                );
            }
            "Pause recorded battle".to_owned()
        },
        ReplayIcon::Speed(speed) => {
            triangle(-8., -1., -2., 5.);
            triangle(1., 8., -2., 5.);
            for index in 0..SPEEDS.len() {
                let left = -8. + index as f32 * 4.5;
                ui.painter().rect_filled(
                    egui::Rect::from_min_max(point(left, 6.), point(left + 3., 8.)),
                    scale * 0.5,
                    if enabled && index <= speed {
                        INK
                    } else {
                        RULE
                    },
                );
            }
            format!("Replay speed: {}×", SPEEDS[speed])
        },
        ReplayIcon::End => {
            triangle(-6., 3., 0., 6.);
            ui.painter().rect_filled(
                egui::Rect::from_min_max(point(4., -6.), point(7., 6.)),
                scale * 0.5,
                ink,
            );
            "Return to finished battle".to_owned()
        },
    };
    response.widget_info(|| {
        egui::WidgetInfo::labeled(egui::WidgetType::Button, enabled, label.as_str())
    });
    response.on_hover_cursor(if enabled {
        egui::CursorIcon::PointingHand
    } else {
        egui::CursorIcon::Default
    })
}

pub(super) fn controls(
    ui: &egui::Ui,
    header: egui::Rect,
    outcome: &BattleOutcome,
    replay: &mut Replay,
    scale: f32,
) {
    let enabled = available(outcome);
    let origin = egui::pos2(
        header.left() + header.width() * 0.75 - CONTROLS_WIDTH * scale / 2.,
        header.center().y - 14. * scale,
    );
    let rect = |offset, width| {
        egui::Rect::from_min_size(
            origin + egui::vec2(offset * scale, 0.),
            egui::vec2(width, 28.) * scale,
        )
    };
    let play = button(
        ui,
        rect(0., 28.),
        "replay-play",
        if replay.playing {
            ReplayIcon::Pause
        } else {
            ReplayIcon::Play
        },
        enabled,
        scale,
    );
    if play.clicked() && enabled {
        if replay.cursor == outcome.round_history.len() {
            replay.cursor = 0;
            replay.elapsed = 0.;
        }
        replay.playing = !replay.playing;
    }
    if button(ui, rect(34., 28.), "replay-speed", ReplayIcon::Speed(replay.speed), enabled, scale)
        .clicked()
        && enabled
    {
        replay.speed = (replay.speed + 1) % SPEEDS.len();
    }
    let progress = rect(68., 96.);
    ui.painter().text(
        progress.center(),
        egui::Align2::CENTER_CENTER,
        format!("Round {}/{}", replay.cursor, outcome.round_history.len()),
        egui::FontId::proportional(12. * scale),
        PAPER,
    );
    if button(ui, rect(170., 28.), "replay-end", ReplayIcon::End, enabled, scale).clicked()
        && enabled
    {
        replay.cursor = outcome.round_history.len();
        replay.playing = false;
        replay.elapsed = 0.;
    }
}

#[cfg(test)]
#[path = "../../tests/unit/military_replay.rs"]
mod tests;
