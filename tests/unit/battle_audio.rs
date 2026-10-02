use super::*;

fn visible(id: u64, gain: f32) -> AudibleBattle {
    AudibleBattle {
        battle: id,
        gain,
        strikes: vec![],
    }
}

#[test]
fn camera_audio_selects_nearby_combat_and_releases_silent_battles() {
    assert!(focus(&[], Some(1)).is_none());
    assert!(focus(&[visible(1, 0.)], Some(1)).is_none());
    let battles = [visible(1, 0.30), visible(2, 0.9)];
    assert_eq!(focus(&battles, Some(1)).unwrap().battle, 2);
    assert_eq!(focus(&battles, None).unwrap().battle, 2);
    let adjacent = [visible(1, 0.82), visible(2, 0.9)];
    assert_eq!(focus(&adjacent, Some(1)).unwrap().battle, 1);
}
