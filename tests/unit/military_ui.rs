use super::*;

#[test]
fn scouting_excludes_neutral_guests_and_includes_engaged_hostile_cohorts() {
    let mut world = MilitaryWorld::new(2);
    let enemy = ForceOwner::Player(1);
    let local = ForceOwner::Local(1);
    let hostile_id = world.seed_unit(1, enemy, UnitType::HeavyInfantry).unwrap();
    world.seed_unit(1, ForceOwner::Player(2), UnitType::LightCavalry).unwrap();
    world.seed_unit(1, local, UnitType::LightInfantry).unwrap();
    world
        .start_battle(1, &[enemy], &[local], None, Some(0), MilitaryTerrain::Plains, 0, 9)
        .unwrap();
    let known = known_hostile_units(&world, 1, ForceOwner::Player(0), |_, target| target == enemy);
    assert_eq!(known.len(), 1);
    assert_eq!(known[0].id, hostile_id);
    assert_eq!(known[0].unit_type, UnitType::HeavyInfantry);
}
