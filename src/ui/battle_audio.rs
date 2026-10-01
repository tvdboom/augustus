//! Battle inspection ambience and sparse composition-specific effects.
use super::*;
use crate::game::military::{Unit, UnitType};

#[derive(Default)]
pub(in crate::app) struct BattleAudio {
    battle: Option<u64>,
    ambience: Option<Handle<AudioInstance>>,
    cues: Vec<Handle<AudioInstance>>,
    next_cue: f64,
    rotation: usize,
    stopping: Vec<Handle<AudioInstance>>,
}

fn composition_cues<'a>(units: impl Iterator<Item = &'a Unit>) -> Vec<&'static str> {
    let types: std::collections::BTreeSet<_> =
        units.filter(|u| u.current_manpower > 0.).map(|u| u.unit_type).collect();
    let mut cues = Vec::new();
    if types.contains(&UnitType::WarElephants) {
        cues.push("battle-elephants");
    }
    if types.iter().any(|kind| {
        matches!(
            kind,
            UnitType::LightCavalry
                | UnitType::HeavyCavalry
                | UnitType::HorseArchers
                | UnitType::WarChariots
        )
    }) {
        cues.push("recruit-horses");
    }
    if types.contains(&UnitType::WarCamels) {
        cues.push("recruit-camels");
    }
    if types.contains(&UnitType::Archers) {
        cues.push("battle-archers");
    }
    if types.iter().any(|kind| matches!(kind, UnitType::Ballista | UnitType::Catapult)) {
        cues.push("recruit-siege");
    }
    if types.iter().any(|kind| matches!(kind, UnitType::LightInfantry | UnitType::HeavyInfantry)) {
        cues.push("battle-infantry");
    }
    cues
}

impl BattleAudio {
    fn stop(&mut self, instances: &mut Assets<AudioInstance>) {
        self.stopping.extend(self.ambience.take());
        self.stopping.append(&mut self.cues);
        self.stop_pending(instances);
        self.battle = None;
        self.rotation = 0;
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
    practice: Res<LocalPractice>,
    terminal: Res<TerminalPresentation>,
    sound: Res<MenuAudio>,
    audio: Res<Audio>,
    assets: Res<AssetServer>,
    mut instances: ResMut<Assets<AudioInstance>>,
    time: Res<Time>,
    mut playing: Local<BattleAudio>,
) {
    playing.stop_pending(&mut instances);
    let selected = contexts.ctx_mut().ok().and_then(|ctx| campaign_military::selected_battle(ctx));
    let battle = selected
        .filter(|&(_, player)| player == practice.active_player)
        .and_then(|(id, _)| campaign.military.battles.iter().find(|b| b.id == id));
    let enabled = *state.get() == AppState::Map
        && campaign.active
        && !terminal.spectating
        && sound.mode != AudioMode::Mute
        && sound.volume > 0.001;
    let Some(battle) = battle.filter(|_| enabled) else {
        playing.stop(&mut instances);
        return;
    };
    let now = time.elapsed_secs_f64();
    let volume = 20. * sound.volume.clamp(0.001, 1.).log10();
    if playing.battle != Some(battle.id) {
        playing.stop(&mut instances);
        playing.battle = Some(battle.id);
        playing.next_cue = now;
        playing.ambience = Some(
            audio
                .play(assets.load("audio/battle-ambience.ogg"))
                .looped()
                .with_volume(-18. + volume)
                .handle(),
        );
    }
    if sound.is_changed() {
        if let Some(mut instance) =
            playing.ambience.as_ref().and_then(|handle| instances.get_mut(handle))
        {
            instance.set_decibels(-18. + volume, AudioTween::linear(Duration::from_millis(90)));
        }
        for handle in &playing.cues {
            if let Some(mut instance) = instances.get_mut(handle) {
                instance.set_decibels(-13. + volume, AudioTween::linear(Duration::from_millis(90)));
            }
        }
    }
    playing.cues.retain(|handle| {
        instances.get(handle).is_none_or(|instance| instance.state() != PlaybackState::Stopped)
    });
    if now >= playing.next_cue {
        let cues = composition_cues(battle.attackers.units.iter().chain(&battle.defenders.units));
        if !cues.is_empty() {
            let cue = cues[playing.rotation % cues.len()];
            playing.rotation += 1;
            let handle = audio
                .play(assets.load(format!("audio/{cue}.ogg")))
                .with_volume(-13. + volume)
                .handle();
            playing.cues.push(handle);
        }
        playing.next_cue = now + 4.2;
    }
}
