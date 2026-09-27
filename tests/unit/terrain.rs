#[test]
fn every_playable_province_has_a_terrain_portrait() {
    for (name, _) in crate::map::production::OUTPUT {
        let terrain = super::for_province(name);
        assert!(terrain.image_index() < 12, "{name}");
    }
}
