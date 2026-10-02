//! Camera-local battle ambience with effects timed to the visible attacks.
use super::*;
use crate::map::AudibleBattle;

#[derive(Default)]
pub(in crate::app) struct BattleAudio {
    battle: Option<u64>,
    ambience: Option<Handle<AudioInstance>>,
    cues: Vec<Handle<AudioInstance>>,
    strikes: std::collections::BTreeMap<u64, i64>,
    next_cue: f64,
    gain: f32,
    stopping: Vec<Handle<AudioInstance>>,
}

/// Keep the current audible battle through small camera movements near two armies.
fn focus(battles: &[AudibleBattle], current: Option<u64>) -> Option<&AudibleBattle> {
    let loudest = battles
        .iter()
        .filter(|battle| battle.gain > 0.001)
        .max_by(|a, b| a.gain.total_cmp(&b.gain))?;
    battles
        .iter()
        .find(|battle| {
            Some(battle.battle) == current
                && battle.gain > 0.001
                && battle.gain >= loudest.gain * 0.8
        })
        .or(Some(loudest))
}

impl BattleAudio {
    fn stop(&mut self, instances: &mut Assets<AudioInstance>) {
        self.stopping.extend(self.ambience.take());
        self.stopping.append(&mut self.cues);
        self.stop_pending(instances);
        self.battle = None;
        self.strikes.clear();
        self.gain = 0.;
    }

    fn stop_pending(&mut self, instances: &mut Assets<AudioInstance>) {
        self.stopping.retain(|handle| {
            if let Some(mut instance) = instances.get_mut(handle) {
                instance.stop(AudioTween::linear(Duration::from_millis(180)));
                false
            } else {
                true
            }
        });
    }
}

pub(in crate::app) fn update(
    mut contexts: EguiContexts,
    state: Res<State<AppState>>,
    campaign: Res<campaign::Campaign>,
    sound: Res<MenuAudio>,
    audio: Res<Audio>,
    assets: Res<AssetServer>,
    mut instances: ResMut<Assets<AudioInstance>>,
    time: Res<Time>,
    mut playing: Local<BattleAudio>,
) {
    playing.stop_pending(&mut instances);
    let visible =
        contexts.ctx_mut().ok().map(|ctx| crate::map::audible_battles(ctx)).unwrap_or_default();
    let battle = focus(&visible, playing.battle).filter(|visible| {
        campaign.military.battles.iter().any(|battle| battle.id == visible.battle)
    });
    let enabled = *state.get() == AppState::Map
        && campaign.active
        && sound.mode != AudioMode::Mute
        && sound.volume > 0.001;
    let Some(battle) = battle.filter(|_| enabled) else {
        playing.stop(&mut instances);
        return;
    };
    let now = time.elapsed_secs_f64();
    let gain = sound.volume.clamp(0., 1.) * battle.gain;
    let volume = 20. * gain.max(0.000_001).log10();
    if playing.battle != Some(battle.battle) {
        playing.stop(&mut instances);
        playing.battle = Some(battle.battle);
        playing.next_cue = now;
        playing.strikes =
            battle.strikes.iter().map(|strike| (strike.actor, strike.cycle)).collect();
        playing.ambience = Some(
            audio
                .play(assets.load("audio/battle-ambience.ogg"))
                .looped()
                .with_volume(-16. + volume)
                .handle(),
        );
    }
    if sound.is_changed() || (playing.gain - battle.gain).abs() > 0.001 {
        if let Some(mut instance) =
            playing.ambience.as_ref().and_then(|handle| instances.get_mut(handle))
        {
            instance.set_decibels(-16. + volume, AudioTween::linear(Duration::from_millis(90)));
        }
        for handle in &playing.cues {
            if let Some(mut instance) = instances.get_mut(handle) {
                instance.set_decibels(-13. + volume, AudioTween::linear(Duration::from_millis(90)));
            }
        }
    }
    playing.gain = battle.gain;
    playing.cues.retain(|handle| {
        instances.get(handle).is_none_or(|instance| instance.state() != PlaybackState::Stopped)
    });
    let mut cue = None;
    for strike in &battle.strikes {
        let previous = playing.strikes.insert(strike.actor, strike.cycle);
        if previous.is_some_and(|previous| strike.cycle > previous) {
            cue = cue.or(Some(strike.cue));
        }
    }
    playing.strikes.retain(|actor, _| battle.strikes.iter().any(|strike| strike.actor == *actor));
    if now >= playing.next_cue && playing.cues.len() < 8 {
        if let Some(cue) = cue {
            let handle = audio
                .play(assets.load(format!("audio/{cue}.ogg")))
                .with_volume(-13. + volume)
                .handle();
            playing.cues.push(handle);
            playing.next_cue = now + 0.12;
        }
    }
}

#[cfg(test)]
#[path = "../../tests/unit/battle_audio.rs"]
mod tests;
