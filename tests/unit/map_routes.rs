use super::*;

fn province(name: &str) -> usize {
    atlas().provinces.iter().position(|p| p.name == name).unwrap()
}

#[test]
fn coastal_marches_follow_land_instead_of_cutting_across_bays() {
    let atlas = atlas();
    let nav = Navigation::new(&atlas.provinces);
    for (from, to) in [("Aegyptus", "Cyrenaica"), ("Achaia", "Macedonia"), ("Samnium", "Apulia")] {
        let (from, to) = (province(from), province(to));
        let (a, b) = if from == province("Aegyptus") {
            (
                geographic(
                    nav.nearest(point([29.9, 31.15]), |id| nav.triangles[id].province == from).1,
                ),
                geographic(
                    nav.nearest(point([20.07, 32.1]), |id| nav.triangles[id].province == to).1,
                ),
            )
        } else {
            (atlas.provinces[from].visual_center, atlas.provinces[to].visual_center)
        };
        let route = march_leg(from, to, a, b).unwrap();
        assert_eq!(route.first(), Some(&a));
        assert_eq!(route.last(), Some(&b));
        let land: Vec<_> = nav
            .triangles
            .iter()
            .enumerate()
            .filter_map(|(id, t)| (t.province == from || t.province == to).then_some(id))
            .collect();
        if from == province("Aegyptus") {
            assert!(
                !nav.visible(point(a), point(b), &land),
                "The coastal regression must cut across a bay"
            );
            assert!(route.len() > 2, "The coast must introduce a bend");
        }
        for edge in route.windows(2) {
            assert!(
                nav.visible(point(edge[0]), point(edge[1]), &land),
                "March from {} to {} left its land corridor: {edge:?}",
                atlas.provinces[from].name,
                atlas.provinces[to].name
            );
        }
    }
}

#[test]
fn every_campaign_border_has_a_geographic_march_path() {
    let atlas = atlas();
    let nav = Navigation::new(&atlas.provinces);
    for (from, neighbors) in atlas.adjacency.iter().enumerate() {
        for &to in neighbors.iter().filter(|&&to| to > from) {
            for (a, b) in [(from, to), (to, from)] {
                let route = march_leg(
                    a,
                    b,
                    atlas.provinces[a].visual_center,
                    atlas.provinces[b].visual_center,
                )
                .unwrap_or_else(|| {
                    panic!(
                        "No geographic route for {} → {}",
                        atlas.provinces[a].name, atlas.provinces[b].name
                    )
                });
                if !SEA_CROSSINGS.iter().any(|crossing| {
                    crossing.provinces.contains(&atlas.provinces[a].name.as_str())
                        && crossing.provinces.contains(&atlas.provinces[b].name.as_str())
                }) {
                    let corridor: Vec<_> = nav
                        .triangles
                        .iter()
                        .enumerate()
                        .filter_map(|(id, t)| (t.province == a || t.province == b).then_some(id))
                        .collect();
                    for edge in route.windows(2) {
                        assert!(
                            nav.visible(point(edge[0]), point(edge[1]), &corridor),
                            "Land route from {} to {} left its provinces: {edge:?}",
                            atlas.provinces[a].name,
                            atlas.provinces[b].name
                        );
                    }
                }
            }
        }
    }
}

#[test]
fn unobstructed_marches_use_gentle_arcs() {
    let atlas = atlas();
    let (from, to) = (province("Aegyptus"), province("Cyrenaica"));
    let route =
        march_leg(from, to, atlas.provinces[from].visual_center, atlas.provinces[to].visual_center)
            .unwrap();
    assert!(route.len() > 2, "Even unobstructed marches should use a gentle curve");
    assert_smooth(&route);
}

fn assert_smooth(path: &[[f32; 2]]) {
    let directions: Vec<_> = path
        .windows(2)
        .map(|edge| point(edge[1]) - point(edge[0]))
        .filter(|delta| delta.length() > 0.0001)
        .map(|delta| delta.normalized())
        .collect();
    for corner in directions.windows(2) {
        assert!(
            corner[0].dot(corner[1]) > 8_f32.to_radians().cos(),
            "Route should turn gradually instead of making a sharp corner: {corner:?}"
        );
    }
}

#[test]
fn africa_to_cyrenaica_takes_a_broad_inland_curve_without_a_hook() {
    let atlas = atlas();
    let (from, to) = (province("Africa Proconsularis"), province("Cyrenaica"));
    let (a, b) = ([13.2, 32.4], [20.4, 31.4]);
    assert!(atlas.provinces[from].contains(a));
    assert!(atlas.provinces[to].contains(b));
    let path = march_leg(from, to, a, b).unwrap();
    assert_eq!(path.first(), Some(&a));
    assert_eq!(path.last(), Some(&b));
    assert_smooth(&path);
    assert!(
        path.windows(2).all(|edge| edge[1][0] >= edge[0][0] - 0.00001),
        "An eastward march must not hook back west around an arbitrary waypoint"
    );
    assert!(
        path.iter().any(|point| point[1] < 31.),
        "The path should bow south, below the Gulf of Sirte"
    );
    let nav = Navigation::new(&atlas.provinces);
    let corridor: Vec<_> = nav
        .triangles
        .iter()
        .enumerate()
        .filter_map(|(id, t)| (t.province == from || t.province == to).then_some(id))
        .collect();
    assert!(path.windows(2).all(|edge| nav.visible(point(edge[0]), point(edge[1]), &corridor)));
}

#[test]
fn asia_has_continuous_land_routes_to_all_three_mainland_neighbors() {
    let atlas = atlas();
    let nav = Navigation::new(&atlas.provinces);
    let asia = province("Asia");
    for name in ["Bithynia", "Galatia", "Lycia"] {
        let neighbor = province(name);
        assert!(atlas.adjacency[asia].contains(&neighbor), "Asia must border {name}");
        assert!(
            atlas.adjacency[neighbor].contains(&asia),
            "The border with {name} must be bidirectional"
        );
        let corridor: Vec<_> = nav
            .triangles
            .iter()
            .enumerate()
            .filter_map(|(id, t)| (t.province == asia || t.province == neighbor).then_some(id))
            .collect();
        for (from, to) in [(asia, neighbor), (neighbor, asia)] {
            let path = march_leg(
                from,
                to,
                atlas.provinces[from].visual_center,
                atlas.provinces[to].visual_center,
            )
            .expect("A shared border must support a geographic march");
            assert!(
                path.windows(2).all(|edge| nav.visible(point(edge[0]), point(edge[1]), &corridor)),
                "The route to {name} must remain on continuous land"
            );
        }
    }
    assert!(
        !atlas.adjacency[asia].contains(&province("Thracia")),
        "The Bosporus must remain water"
    );
}

#[test]
fn sea_crossings_use_coasts_and_do_not_create_unmarked_shortcuts() {
    let atlas = atlas();
    for crossing in &SEA_CROSSINGS {
        let [from, to] = crossing.provinces.map(province);
        let route = march_leg(
            from,
            to,
            atlas.provinces[from].visual_center,
            atlas.provinces[to].visual_center,
        )
        .unwrap();
        assert!(route.len() >= 4, "Marked crossing needs coastal approach and departure");
    }
    assert!(march_leg(province("Aegyptus"), province("Sicilia"), [30., 30.], [14., 37.]).is_none());
}

#[test]
fn rome_connects_to_its_neighboring_provinces_on_land() {
    let atlas = atlas();
    for name in ["Latium", "Etruria", "Samnium"] {
        let id = province(name);
        assert!(march_leg(
            atlas.provinces.len(),
            id,
            CITIES[0].position,
            atlas.provinces[id].visual_center
        )
        .is_some());
        assert!(march_leg(
            id,
            atlas.provinces.len(),
            atlas.provinces[id].visual_center,
            CITIES[0].position
        )
        .is_some());
    }
}
