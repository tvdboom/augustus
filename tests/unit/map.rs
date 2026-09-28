use super::*;

#[test]
fn wave_crests_disappear_before_reforming_with_a_new_shape() {
    let duration = 5.0;
    let quiet = 7.0;
    assert_eq!(crest_life(0.0, 0.0, duration, quiet).2, 0.0);
    assert!(crest_life(2.5, 0.0, duration, quiet).2 > 0.99);
    for time in [5.0, 8.0, 11.999, 12.0] {
        assert_eq!(crest_life(time, 0.0, duration, quiet).2, 0.0);
    }
    assert!(crest_life(12.001, 0.0, duration, quiet).2 < 0.00001);
    let first = crest_variation(1289, 0);
    let second = crest_variation(1289, 1);
    assert!(first.iter().zip(second).any(|(a, b)| (a - b).abs() > 0.2));
    assert!(first.iter().chain(second.iter()).all(|value| (0.0..1.0).contains(value)));
}

#[test]
fn terrain_uvs_face_north_and_cover_every_playable_vertex() {
    for bounds in TERRAIN_TILE_BOUNDS {
        let [west, south, east, north] = bounds;
        assert_eq!(terrain_uv([west, north], bounds), egui::pos2(1.0 / 1922.0, 1.0 / 1112.0));
        assert_eq!(terrain_uv([east, south], bounds), egui::pos2(1921.0 / 1922.0, 1111.0 / 1112.0));
        assert_eq!(
            terrain_uv([(west + east) * 0.5, (south + north) * 0.5], bounds),
            egui::pos2(0.5, 0.5)
        );
    }
    for province in &atlas().provinces {
        for point in province.parts.iter().flat_map(|part| &part.v) {
            assert!(
                TERRAIN_TILE_BOUNDS.iter().any(|bounds| {
                    point[0] >= bounds[0]
                        && point[0] <= bounds[2]
                        && point[1] >= bounds[1]
                        && point[1] <= bounds[3]
                }),
                "terrain raster misses {} at {point:?}",
                province.name
            );
        }
        for tile in &province.terrain {
            let bounds = TERRAIN_TILE_BOUNDS[tile.tile];
            for point in &tile.geometry.v {
                assert!(point[0] >= bounds[0] && point[0] <= bounds[2]);
                assert!(point[1] >= bounds[1] && point[1] <= bounds[3]);
                let uv = terrain_uv(*point, bounds);
                assert!((0.0..=1.0).contains(&uv.x) && (0.0..=1.0).contains(&uv.y));
            }
        }
    }
}

#[test]
fn terrain_tile_seams_preserve_every_provinces_area() {
    let projection = Projection {
        origin: egui::Pos2::ZERO,
        scale: 30.0,
        center: [20.0, 40.0],
    };
    let area = |part: &MapMesh| -> f64 {
        part.t
            .chunks_exact(3)
            .map(|face| {
                terrain_triangle_area(
                    part.v[face[0] as usize],
                    part.v[face[1] as usize],
                    part.v[face[2] as usize],
                )
            })
            .sum()
    };
    for province in &atlas().provinces {
        let original: f64 = province.parts.iter().map(&area).sum();
        let tiled: f64 = province.terrain.iter().map(|tile| area(&tile.geometry)).sum();
        assert!(
            (original - tiled).abs() <= original * 0.00001,
            "terrain loses or overlaps {} at a seam: {original} versus {tiled}",
            province.name
        );
        for tile in &province.terrain {
            let mesh = province_terrain_mesh(
                tile,
                &projection,
                egui::Rect::EVERYTHING,
                egui::TextureId::Managed(12),
            );
            assert!(mesh.is_valid(), "invalid terrain mesh for {}", province.name);
        }
    }
}

#[test]
fn terrain_preserves_a_province_hole_and_excludes_the_backdrop() {
    let mut province = Province {
        name: "Playable ring".into(),
        short: "Ring".into(),
        label: [0.5, 0.5],
        bounds: [0.0, 0.0, 4.0, 4.0],
        parts: vec![MapMesh {
            v: vec![
                [0.0, 0.0],
                [4.0, 0.0],
                [4.0, 4.0],
                [0.0, 4.0],
                [1.0, 1.0],
                [3.0, 1.0],
                [3.0, 3.0],
                [1.0, 3.0],
            ],
            r: vec![4, 8],
            t: vec![0, 1, 5, 0, 5, 4, 1, 2, 6, 1, 6, 5, 2, 3, 7, 2, 7, 6, 3, 0, 4, 3, 4, 7],
            bounds: [0.0, 0.0, 4.0, 4.0],
        }],
        terrain: Vec::new(),
        visual_center: [0.5, 0.5],
        production: [0; 3],
    };
    // Position the playable ring across both tile seams.
    for part in &mut province.parts {
        for point in &mut part.v {
            point[0] += 18.0;
            point[1] += 38.5;
        }
        part.cache_bounds();
    }
    province.bounds = [18.0, 38.5, 22.0, 42.5];
    province.terrain = build_terrain_tiles(&province.parts);
    assert_eq!(province.terrain.len(), 4);
    let projection = Projection {
        origin: egui::pos2(80.0, 50.0),
        scale: 20.0,
        center: [20.0, 40.5],
    };
    let texture = egui::TextureId::Managed(12);
    let mut area = 0.0;
    for tile in &province.terrain {
        let mesh = province_terrain_mesh(tile, &projection, egui::Rect::EVERYTHING, texture);
        assert_eq!(mesh.texture_id, texture);
        for face in mesh.indices.chunks_exact(3) {
            let [a, b, c] = [face[0], face[1], face[2]]
                .map(|index| projection.inverse(mesh.vertices[index as usize].pos));
            let center = [(a[0] + b[0] + c[0]) / 3.0, (a[1] + b[1] + c[1]) / 3.0];
            assert!(province.contains(center), "terrain face escaped the playable ring");
            area += ((b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0])).abs() * 0.5;
        }
    }
    assert!((area - 12.0).abs() < 0.0001, "terrain filled the non-playable hole");
    let outside = projection.bounds_rect([25.0, 45.0, 26.0, 46.0]);
    assert!(province.terrain.iter().all(|tile| province_terrain_mesh(
        tile,
        &projection,
        outside,
        texture
    )
    .indices
    .is_empty()));
}

#[test]
fn terrain_tiles_load_with_eguis_default_texture_limit() {
    let context = egui::Context::default();
    context.begin_pass(Default::default());
    let limit = context.input(|input| input.max_texture_side);
    for (name, png) in &ENVIRONMENT_IMAGES
        [TERRAIN_TEXTURE_START..TERRAIN_TEXTURE_START + TERRAIN_TILE_BOUNDS.len()]
    {
        let texture = load_map_texture(&context, name, png);
        assert!(texture.size().into_iter().all(|side| side <= limit));
        assert_eq!(texture.size(), [1922, 1112]);
    }
    let mut output = context.end_pass();
    output.textures_delta.clear();
}

#[test]
fn resource_badges_have_only_a_below_name_position() {
    let viewport = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(300.0, 200.0));
    let name = egui::Rect::from_min_size(egui::pos2(90.0, 60.0), egui::vec2(80.0, 24.0));
    let badge = resource_badge_below(name, egui::vec2(64.0, 22.0), viewport).unwrap();
    assert_eq!(badge.top(), name.bottom() + 3.0);
    assert_eq!(badge.center().x, name.center().x);
    let bottom_name = name.translate(egui::vec2(0.0, 100.0));
    assert!(resource_badge_below(bottom_name, egui::vec2(64.0, 22.0), viewport).is_none());
}

#[test]
fn zoomed_pan_can_center_edge_provinces() {
    let rect = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1440.0, 900.0));
    let (center, width, height, fit) = map_geometry(rect, atlas());
    let zoom = PAN_MARGIN_FULL_ZOOM;
    let x_limit = pan_limit(width * fit * zoom, rect.width(), zoom);
    let y_limit = pan_limit(height * fit * zoom, rect.height(), zoom);
    for province in &atlas().provinces {
        let x_offset = (province.label[0] - center[0]).abs() * LONGITUDE_SCALE * fit * zoom;
        let y_offset = (province.label[1] - center[1]).abs() * fit * zoom;
        assert!(x_offset <= x_limit && y_offset <= y_limit, "{}", province.name);
    }

    let spain = atlas().provinces.iter().find(|province| province.name == "Lusitania").unwrap();
    let overview_offset = (spain.label[0] - center[0]).abs() * LONGITUDE_SCALE * fit * MIN_ZOOM;
    let overview_limit = pan_limit(width * fit * MIN_ZOOM, rect.width(), MIN_ZOOM);
    assert!(overview_offset > overview_limit);
}

#[test]
fn every_mapped_province_has_plausible_nonnegative_production() {
    let atlas = atlas();
    assert_eq!(atlas.provinces.len(), super::super::production::OUTPUT.len());
    for province in &atlas.provinces {
        assert!(province.production.iter().all(|amount| *amount >= 0), "{}", province.name);
        assert!(province.production.iter().any(|amount| *amount > 0), "{}", province.name);
    }
    assert!(for_province("Aegyptus")[0] > for_province("Arabia")[0]);
    assert!(for_province("Noricum")[1] > for_province("Latium")[1]);
    assert!(for_province("Achaia")[2] > for_province("Picenum")[2]);
}

#[test]
fn starting_population_uses_area_city_bias_and_four_near_standard_classes() {
    let provinces = &atlas().provinces;
    for province in provinces {
        let population = ProvincePopulation::starting(province);
        assert!(population.counts().into_iter().all(|count| count > 0.0), "{}", province.name);
        assert!((20.0..=80.0).contains(&population.total()), "{}", province.name);
        let target_percentages = if URBAN_PROVINCES.contains(&province.name.as_str()) {
            [12.0, 23.0, 37.0, 28.0]
        } else {
            [10.0, 20.0, 40.0, 30.0]
        };
        for (class, (count, percentage)) in
            population.counts().into_iter().zip(target_percentages).enumerate()
        {
            assert!(
                (count * 100.0 - population.total() * percentage).abs()
                    <= population.total()
                        * if class == 3 {
                            7.0
                        } else {
                            3.0
                        },
                "{} class share differs from the province type",
                province.name
            );
        }
    }
    assert!(starting_total_for(20.0, false) > starting_total_for(1.0, false));
    assert!(starting_total_for(1.0, true) > starting_total_for(20.0, false));
}

#[test]
fn resource_poor_city_starts_gain_population_without_changing_yields() {
    let mut ownership = ProvinceOwnership::default();
    ownership.start_game(&[
        egui::Color32::RED,
        egui::Color32::BLUE,
        egui::Color32::GREEN,
        egui::Color32::YELLOW,
    ]);
    let candidates = starting_candidates();
    let scores: Vec<_> = candidates
        .iter()
        .map(|&index| {
            starting_economy_score(
                atlas().provinces[index].production,
                ownership.populations[index],
            )
        })
        .collect();
    let target = scores.iter().copied().fold(0.0_f64, f64::max);
    for (index, &owner) in ownership.owners.iter().enumerate() {
        if let Some(player) = owner {
            let province = &atlas().provinces[index];
            let population = ownership.populations[index];
            assert!(
                (starting_economy_score(province.production, population) - target).abs() < 1e-8,
                "{} has an unbalanced opening economy",
                province.name
            );
            assert!(ownership.net_production_for(player)[0] > 0.0);
        }
    }

    let ordinary = ProvincePopulation {
        nobles: 6.0,
        citizens: 12.0,
        plebeians: 20.0,
        slaves: 16.0,
    };
    let rich_yields = [6, 1, 4];
    let poor_yields = [3, 1, 1];
    let target = starting_economy_score(rich_yields, ordinary);
    let balanced = balanced_starting_population(poor_yields, ordinary, target);
    assert!(balanced.total() > ordinary.total());
    assert!(balanced.slaves - ordinary.slaves > balanced.nobles - ordinary.nobles);
    assert!((starting_economy_score(poor_yields, balanced) - target).abs() < 1e-8);
    assert_eq!(
        balanced_starting_population(rich_yields, ordinary, target).total(),
        ordinary.total()
    );
}

#[test]
fn owned_population_reconciles_with_class_sources_and_famine_trend() {
    let mut ownership = ProvinceOwnership::default();
    ownership.start_game(&[egui::Color32::RED, egui::Color32::BLUE]);
    for player in 0..2 {
        let totals = ownership.population_for(player);
        for (class, total) in totals.into_iter().enumerate() {
            let sources = ownership.population_sources(player, class);
            assert_eq!(sources.iter().map(|(_, count)| count).sum::<f64>(), total);
            assert_eq!(sources.len(), 1);
        }
        let expected_growth = ownership.total_population_for(player) * 0.01;
        assert!(expected_growth > 0.0);
        assert_eq!(ownership.population_change_for(player, f64::MAX / 2.0, 0), expected_growth);
    }
    let owned = ownership.owners.iter().position(|owner| *owner == Some(0)).unwrap();
    ownership.populations[owned] = ProvincePopulation {
        nobles: 40.0,
        citizens: 0.0,
        plebeians: 0.0,
        slaves: 0.0,
    };
    assert_eq!(ownership.population_change_for(0, 0.0, 0), 0.0);
    assert_eq!(ownership.population_change_for(0, 0.0, 3), -2.0);
    assert_eq!(ownership.population_change_for(0, 0.0, 8), -5.0);
    ownership.advance_population(0, -5.0);
    assert_eq!(ownership.population_for(0).into_iter().sum::<f64>(), 35.0);
}

#[test]
fn starting_provinces_are_non_rome_cities_varied_and_well_separated() {
    let atlas = atlas();
    let candidates = starting_candidates();
    assert_eq!(candidates.len(), URBAN_PROVINCES.len() - 1);
    assert!(candidates.iter().all(|&index| {
        let name = atlas.provinces[index].name.as_str();
        URBAN_PROVINCES.contains(&name) && name != ROME_PROVINCE
    }));
    for count in 1..=4 {
        let mut seen = std::collections::HashSet::new();
        for _ in 0..12 {
            let selected = spread_out_starts(count);
            assert_eq!(selected.len(), count);
            for (offset, &left) in selected.iter().enumerate() {
                let province = &atlas.provinces[left];
                assert!(candidates.contains(&left));
                for &right in &selected[offset + 1..] {
                    assert_ne!(left, right);
                    assert!(
                        province_distance_km(province.label, atlas.provinces[right].label)
                            >= if count == 2 {
                                TWO_PLAYER_MIN_START_DISTANCE_KM
                            } else {
                                MIN_START_DISTANCE_KM
                            }
                    );
                }
            }
            let mut set = selected;
            set.sort_unstable();
            seen.insert(set);
        }
        assert!(seen.len() > 1, "{count} players always receive the same province set");
    }
}

#[test]
fn every_urban_province_has_its_city_name() {
    for name in URBAN_PROVINCES {
        assert!(city_name_for_province(name).is_some(), "{name}");
    }
    assert_eq!(city_name_for_province("Britannia"), None);
}

#[test]
fn city_markers_target_their_provinces() {
    let atlas = atlas();
    assert_eq!(CITIES.len(), URBAN_PROVINCES.len());
    for city in CITIES {
        assert!(URBAN_PROVINCES.contains(&city.province));
        assert!(city_name_for_province(city.province).is_some());
        assert!(atlas.provinces.iter().any(|province| province.name == city.province));
    }
}

#[test]
fn ownership_uses_each_players_color_and_only_owned_production() {
    let colors =
        [egui::Color32::RED, egui::Color32::BLUE, egui::Color32::GREEN, egui::Color32::YELLOW];
    let mut ownership = ProvinceOwnership::default();
    ownership.start_game(&colors);
    for (player, color) in colors.into_iter().enumerate() {
        let owned: Vec<_> = ownership
            .owners
            .iter()
            .enumerate()
            .filter(|(_, owner)| **owner == Some(player))
            .collect();
        assert_eq!(owned.len(), 1);
        assert_eq!(ownership.color(owned[0].0), Some(color));
        assert_eq!(ownership.production_for(player), ownership.output_for(owned[0].0));
    }
    ownership.start_game(&colors[..1]);
    assert_eq!(ownership.owners.iter().filter(|owner| owner.is_some()).count(), 1);
}

#[test]
fn map_tint_can_differ_from_exact_banner_color() {
    let banner = [egui::Color32::from_rgb(109, 36, 55)];
    let tint = [egui::Color32::from_rgb(225, 76, 158)];
    let mut ownership = ProvinceOwnership::default();
    ownership.start_game(&banner);
    ownership.set_map_colors(&tint);
    let province = ownership.owners.iter().position(|owner| *owner == Some(0)).unwrap();
    assert_eq!(ownership.map_color(province), Some(tint[0]));
    assert_eq!(ownership.color(province), Some(banner[0]));
    assert_eq!(ownership.province_overview(province).unwrap().owner_color, Some(banner[0]));
}

#[test]
fn trade_requires_a_shared_land_border_with_an_owned_province() {
    let provinces = &atlas().provinces;
    let find = |name| provinces.iter().position(|province| province.name == name).unwrap();
    let aegyptus = find("Aegyptus");
    let cyrenaica = find("Cyrenaica");
    let syria = find("Syria");
    let mut ownership = ProvinceOwnership::default();
    ownership.owners = vec![None; provinces.len()];
    ownership.owners[aegyptus] = Some(0);
    assert!(ownership.can_trade_with(cyrenaica, 0));
    assert!(!ownership.can_trade_with(syria, 0));
    assert!(!ownership.can_trade_with(aegyptus, 0));
    assert!(!ownership.can_trade_with(cyrenaica, 1));
}

#[test]
fn slaves_add_more_output_than_plebeians_and_every_class_eats() {
    let population = ProvincePopulation {
        nobles: 2.0,
        citizens: 3.0,
        plebeians: 4.0,
        slaves: 2.0,
    };
    assert_eq!(monthly_output([3, 2, 4], population), [21.0, 14.0, 28.0]);
    assert_eq!(
        monthly_output(
            [1, 1, 1],
            ProvincePopulation {
                nobles: 0.0,
                citizens: 0.0,
                plebeians: 0.0,
                slaves: 1.0,
            }
        ),
        [1.5, 1.5, 1.5]
    );
    assert_eq!(population.food_upkeep(), 11.0);

    let mut ownership = ProvinceOwnership::default();
    ownership.start_game(&[egui::Color32::RED]);
    let province =
        atlas().provinces.iter().position(|province| province.name == "Aegyptus").unwrap();
    ownership.populations[province] = ProvincePopulation {
        nobles: 10.0,
        citizens: 18.0,
        plebeians: 12.0,
        slaves: 0.0,
    };
    let plebeian_output = ownership.output_for(province);
    ownership.populations[province] = ProvincePopulation {
        nobles: 10.0,
        citizens: 18.0,
        plebeians: 0.0,
        slaves: 12.0,
    };
    let slave_output = ownership.output_for(province);
    assert!(slave_output[0] > plebeian_output[0]);
    assert_eq!(ownership.food_upkeep_for(province), 40.0);
    assert_eq!(
        ProvincePopulation {
            nobles: 1.0,
            citizens: 1.0,
            plebeians: 1.0,
            slaves: 1.0
        }
        .food_upkeep(),
        4.0
    );
    assert_eq!(
        ProvincePopulation {
            nobles: 0.0,
            citizens: 0.0,
            plebeians: 0.0,
            slaves: 0.0
        }
        .food_upkeep(),
        0.0
    );
}

#[test]
fn each_pop_class_grows_one_percent_per_fed_month() {
    let mut population = ProvincePopulation {
        nobles: 200.0,
        citizens: 400.0,
        plebeians: 800.0,
        slaves: 600.0,
    };
    assert_eq!(population.monthly_growth(0.01, 0.01), 20.0);
    population.grow(0.01, 0.01);
    assert_eq!(population.counts(), [202.0, 404.0, 808.0, 606.0]);
    assert_eq!(population.food_upkeep(), 2_020.0);

    let mut small_population = ProvincePopulation {
        nobles: 0.0,
        citizens: 0.0,
        plebeians: 1.0,
        slaves: 0.0,
    };
    small_population.grow(0.01, 0.01);
    assert_eq!(small_population.plebeians, 1.01);
    assert_eq!(monthly_output([1, 0, 0], small_population)[0], 1.01);
}

#[test]
fn province_sources_reconcile_with_all_resource_deltas() {
    let mut ownership = ProvinceOwnership::default();
    ownership.start_game(&[egui::Color32::RED]);
    for resource in 0..3 {
        let sources = ownership.production_sources(0, resource);
        assert_eq!(sources.len(), 1);
        let (_, produced, consumed) = sources[0];
        assert_eq!(ownership.production_for(0)[resource], produced);
        if resource == 0 {
            assert_eq!(consumed, ownership.total_population_for(0));
            assert_eq!(ownership.net_production_for(0)[resource], produced - consumed);
        } else {
            assert_eq!(consumed, 0.0);
            assert_eq!(ownership.net_production_for(0)[resource], produced);
        }
    }
}

#[test]
fn coin_taxes_use_owned_population_and_reconcile_with_monthly_delta() {
    let mut ownership = ProvinceOwnership::default();
    ownership.start_game(&[egui::Color32::RED, egui::Color32::BLUE]);
    let aegyptus =
        atlas().provinces.iter().position(|province| province.name == "Aegyptus").unwrap();
    ownership.owners.fill(None);
    ownership.owners[aegyptus] = Some(0);
    ownership.populations[aegyptus] = ProvincePopulation {
        nobles: 3.0,
        citizens: 20.0,
        plebeians: 5.0,
        slaves: 4.0,
    };

    let opening_taxes = ownership.coin_taxes_for(0);
    assert_eq!(opening_taxes, 38.0);
    assert_eq!(ownership.coin_delta_for(0), opening_taxes);
    assert_eq!(ownership.influence_delta_for(0), 3.0);
    assert_eq!(ownership.coin_taxes_for(1), 0.0);
    assert_eq!(ownership.coin_delta_for(1), 0.0);
    assert_eq!(ownership.influence_delta_for(1), 0.0);

    ownership.advance_population(0, 1.0);
    assert!((ownership.coin_delta_for(0) - opening_taxes * 1.01).abs() < 1e-9);
}

#[test]
fn governance_edicts_change_only_the_owning_players_monthly_rates() {
    use super::super::EdictLevel;

    let mut ownership = ProvinceOwnership::default();
    ownership.start_game(&[egui::Color32::RED, egui::Color32::BLUE]);
    let province =
        atlas().provinces.iter().position(|province| province.name == "Aegyptus").unwrap();
    ownership.owners.fill(None);
    ownership.owners[province] = Some(0);
    ownership.populations[province] = ProvincePopulation {
        nobles: 10.0,
        citizens: 20.0,
        plebeians: 10.0,
        slaves: 10.0,
    };
    let baseline_output = ownership.output_for(province);
    assert_eq!(baseline_output[0], 150.0);
    assert_eq!(ownership.net_production_for(0)[0], 100.0);
    assert_eq!(ownership.coin_taxes_for(0), 50.0);
    assert_eq!(ownership.influence_delta_for(0), 10.0);

    let mut edicts = Governance {
        food_rations: EdictLevel::Low,
        ..Default::default()
    };
    ownership.set_governance_for(0, edicts);
    assert_eq!(ownership.net_production_for(0)[0], 110.0);
    assert_eq!(ownership.food_upkeep_for(province), 40.0);
    assert_eq!(ownership.governance_for(1), Governance::default());

    edicts.food_rations = EdictLevel::High;
    ownership.set_governance_for(0, edicts);
    assert_eq!(ownership.output_for(province), baseline_output);
    assert_eq!(ownership.food_upkeep_for(province), 60.0);
    assert_eq!(ownership.net_production_for(0)[0], 90.0);
    assert_eq!(ownership.influence_delta_for(0), 10.0);

    edicts.food_rations = EdictLevel::Medium;
    edicts.slave_labor = EdictLevel::Low;
    ownership.set_governance_for(0, edicts);
    assert_eq!(ownership.output_for(province)[0], 105.0);
    edicts.slave_labor = EdictLevel::High;
    ownership.set_governance_for(0, edicts);
    assert_eq!(ownership.output_for(province)[0], 195.0);

    edicts.noble_taxes = EdictLevel::Low;
    ownership.set_governance_for(0, edicts);
    assert_eq!(ownership.coin_taxes_for(0), 45.0);
    edicts.noble_taxes = EdictLevel::High;
    ownership.set_governance_for(0, edicts);
    assert_eq!(ownership.coin_taxes_for(0), 55.0);

    edicts.army_wages = EdictLevel::High;
    ownership.set_governance_for(0, edicts);
    assert_eq!(ownership.military_wages_for(0), 0.0);
    assert_eq!(ownership.coin_delta_for(0), 55.0);
}

#[test]
fn food_rations_and_hard_labor_set_class_growth_rates() {
    use super::super::EdictLevel;

    let mut ownership = ProvinceOwnership::default();
    ownership.start_game(&[egui::Color32::RED]);
    let province =
        atlas().provinces.iter().position(|province| province.name == "Aegyptus").unwrap();
    ownership.owners.fill(None);
    ownership.owners[province] = Some(0);
    let baseline = ProvincePopulation {
        nobles: 100.0,
        citizens: 200.0,
        plebeians: 400.0,
        slaves: 300.0,
    };
    for (level, expected_growth) in
        [(EdictLevel::Low, 5.0), (EdictLevel::Medium, 10.0), (EdictLevel::High, 15.0)]
    {
        ownership.populations[province] = baseline;
        ownership.set_governance_for(
            0,
            Governance {
                food_rations: level,
                ..Default::default()
            },
        );
        let change = ownership.population_change_for(0, 100.0, 0);
        assert!((change - expected_growth).abs() < 1e-9);
        ownership.advance_population(0, change);
        assert!((ownership.total_population_for(0) - (1_000.0 + expected_growth)).abs() < 1e-9);

        ownership.populations[province] = baseline;
        ownership.set_governance_for(
            0,
            Governance {
                food_rations: level,
                slave_labor: EdictLevel::High,
                ..Default::default()
            },
        );
        let hard_labor_change = ownership.population_change_for(0, 100.0, 0);
        assert!((hard_labor_change - (expected_growth - 1.5)).abs() < 1e-9);
        ownership.advance_population(0, hard_labor_change);
        let expected_slave_growth =
            300.0 * (ownership.governance_for(0).slave_population_growth_rate());
        assert!(
            (ownership.populations[province].slaves - 300.0 - expected_slave_growth).abs() < 1e-9
        );
    }
}

#[test]
fn rotated_label_fits_a_narrow_province() {
    let province = Province {
        name: "Narrow".into(),
        short: "Narrow".into(),
        label: [0.5, 2.0],
        bounds: [0.0, 0.0, 1.0, 4.0],
        terrain: Vec::new(),
        parts: vec![MapMesh {
            v: vec![[0.0, 0.0], [1.0, 0.0], [1.0, 4.0], [0.0, 4.0]],
            r: vec![4],
            t: Vec::new(),
            bounds: [0.0, 0.0, 1.0, 4.0],
        }],
        visual_center: [0.5, 2.0],
        production: [0; 3],
    };
    let projection = Projection {
        origin: egui::Pos2::ZERO,
        scale: 40.0,
        center: [0.5, 2.0],
    };
    let text_size = egui::vec2(50.0, 12.0);
    assert!(!label_fits_province(&province, &projection, province.label, text_size, 0.0));
    assert!(label_fits_province(
        &province,
        &projection,
        province.label,
        text_size,
        std::f32::consts::FRAC_PI_2,
    ));
}

#[test]
fn label_candidates_leave_room_and_offer_marker_fallbacks() {
    let province = Province {
        name: "Tarraconensis".into(),
        short: "Tarraconensis".into(),
        label: [10.0, 5.0],
        bounds: [0.0, 0.0, 20.0, 10.0],
        terrain: Vec::new(),
        parts: vec![MapMesh {
            v: vec![[0.0, 0.0], [20.0, 0.0], [20.0, 10.0], [0.0, 10.0]],
            r: vec![4],
            t: vec![0, 1, 2, 0, 2, 3],
            bounds: [0.0, 0.0, 20.0, 10.0],
        }],
        visual_center: [10.0, 5.0],
        production: [0; 3],
    };
    let projection = Projection {
        origin: egui::Pos2::ZERO,
        scale: 10.0,
        center: [10.0, 5.0],
    };
    let context = egui::Context::default();
    context.begin_pass(Default::default());
    let painter = context.layer_painter(egui::LayerId::background());
    let candidates = label_candidates(&painter, &province, &projection, 2.0);
    assert!(candidates.len() > 1);
    let first = &candidates[0];
    assert_eq!(first.center, province.visual_center);
    let alternate = candidates.iter().find(|candidate| {
        candidate.center != first.center && candidate.font_size == first.font_size
    });
    assert!(alternate.is_some(), "a blocked center must have a readable fallback");
    let text_size = painter
        .layout_no_wrap(province.name.clone(), egui::FontId::proportional(first.font_size), INK)
        .size();
    assert!(label_fit_size(text_size).x >= text_size.x * 1.18);
    for zoom in [1.0, 4.0] {
        let fitted = anchored_label_candidates(&painter, &province, &projection, zoom, first);
        assert!(!fitted.is_empty());
        assert!(fitted.iter().all(|candidate| {
            candidate.center == first.center && candidate.angle == first.angle
        }));
    }
    let mut output = context.end_pass();
    output.textures_delta.clear();
}

#[test]
fn achaia_keeps_its_anchor_across_zoom_levels() {
    let province = atlas().provinces.iter().find(|province| province.name == "Achaia").unwrap();
    let context = egui::Context::default();
    context.begin_pass(Default::default());
    let painter = context.layer_painter(egui::LayerId::background());
    let initial_projection = Projection {
        origin: egui::Pos2::ZERO,
        scale: 40.0,
        center: province.label,
    };
    let overview_projection = Projection {
        origin: egui::Pos2::ZERO,
        scale: 13.0 * 1.5,
        center: province.label,
    };
    let prepared = vec![
        vec![label_candidates(&painter, province, &overview_projection, 1.5)],
        vec![label_candidates(&painter, province, &initial_projection, MAX_ZOOM)],
    ];
    assert_ne!(prepared[0][0][0].angle, 0.0);
    let anchor =
        stable_label_anchors(&prepared, 1).remove(0).expect("Achaia should have a label placement");
    assert_eq!(anchor.angle, 0.0);
    for zoom in [1.5, 2.5, 3.7, 5.0, 8.0] {
        let projection = Projection {
            origin: egui::Pos2::ZERO,
            scale: 13.0 * zoom,
            center: province.label,
        };
        let anchored = anchored_label_candidates(&painter, province, &projection, zoom, &anchor);
        let regular = label_candidates(&painter, province, &projection, zoom);
        assert!(label_choice_order(&anchored, &regular, true, false).iter().all(
            |(candidate, relocated)| {
                !relocated && candidate.center == anchor.center && candidate.angle == anchor.angle
            }
        ));
    }
    let mut output = context.end_pass();
    output.textures_delta.clear();
}

#[test]
fn tarraconensis_has_readable_placements_away_from_its_marker() {
    let province =
        atlas().provinces.iter().find(|province| province.name == "Tarraconensis").unwrap();
    let projection = Projection {
        origin: egui::Pos2::ZERO,
        scale: 48.0,
        center: province.label,
    };
    let context = egui::Context::default();
    context.begin_pass(Default::default());
    let painter = context.layer_painter(egui::LayerId::background());
    let candidates = label_candidates(&painter, province, &projection, 3.7);
    let preferred_size = candidates[0].font_size;
    let marker =
        egui::Rect::from_center_size(projection.point([-4.117, 40.948]), egui::vec2(42.0, 42.0));
    assert!(candidates.iter().any(|candidate| {
        let galley = painter.layout_no_wrap(
            province.name.clone(),
            egui::FontId::proportional(candidate.font_size),
            INK,
        );
        let bounds =
            rotated_bounds(projection.point(candidate.center), galley.size(), candidate.angle);
        candidate.full_name && candidate.font_size == preferred_size && !marker.intersects(bounds)
    }));
    let anchor = LabelPlacement {
        center: province.label,
        angle: 0.0,
        font_size: preferred_size,
        full_name: true,
    };
    let anchored = anchored_label_candidates(&painter, province, &projection, 3.7, &anchor);
    let blocked_center =
        egui::Rect::from_center_size(projection.point(anchor.center), egui::vec2(42.0, 42.0));
    let readable_floor = readable_label_floor(&candidates);
    assert!(marker_blocks_readable_anchor(
        &painter,
        province,
        &projection,
        &anchored,
        &candidates,
        &[blocked_center],
    ));
    let (selected, relocated) = label_choice_order(&anchored, &candidates, true, true)
        .into_iter()
        .find(|(candidate, _)| {
            let name = if candidate.full_name {
                &province.name
            } else {
                &province.short
            };
            let galley = painter.layout_no_wrap(
                name.clone(),
                egui::FontId::proportional(candidate.font_size),
                INK,
            );
            let bounds =
                rotated_bounds(projection.point(candidate.center), galley.size(), candidate.angle);
            !blocked_center.intersects(bounds)
        })
        .expect("a blocked Tarraconensis label needs a fallback");
    assert!(relocated);
    assert!(selected.font_size >= readable_floor);
    assert!(label_choice_order(&anchored, &candidates, true, false).iter().all(
        |(candidate, relocated)| {
            !relocated && candidate.center == anchor.center && candidate.angle == anchor.angle
        }
    ));
    let mut output = context.end_pass();
    output.textures_delta.clear();
}

#[test]
fn rhodes_wonder_anchor_is_on_the_island() {
    let rhodes = WONDERS.iter().find(|wonder| wonder.name == "Colossus of Rhodes").unwrap();
    let province = atlas()
        .provinces
        .iter()
        .find(|province| province.contains(rhodes.position))
        .expect("the Colossus should be on Rhodes, within Asia's island geometry");
    assert_eq!(province.name, "Asia");
    assert!(atlas().land.iter().any(|part| part.contains(rhodes.position)));
}

#[test]
fn mismatched_coastal_backdrop_does_not_tint_sea() {
    for point in [[-6.235, 37.013], [-1.273, 44.175], [4.521, 51.682]] {
        assert!(
            !atlas().land.iter().any(|part| part.contains(point)),
            "coastal water at {point:?} should have the sea color"
        );
    }
    assert!(atlas().land.iter().any(|part| part.contains([10.0, 60.0])));
}

#[test]
fn colossus_grows_with_zoom_without_leaving_rhodes() {
    let colossus = WONDERS.iter().position(|wonder| wonder.name == "Colossus of Rhodes").unwrap();
    let rect = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1000.0, 545.0));
    let marker_at_zoom = |zoom| {
        let projection = Projection {
            origin: rect.center(),
            scale: 14.7 * zoom,
            center: WONDERS[colossus].position,
        };
        let marker = layout_wonders(&projection, rect, zoom)
            .into_iter()
            .find(|marker| marker.index == colossus)
            .expect("the Colossus should appear on Rhodes at close zoom");
        let image = marker.image.expect("the close view should show the Colossus art");
        assert_eq!(image.center(), projection.point(WONDERS[colossus].position));
        image.width()
    };
    assert!(marker_at_zoom(8.0) > marker_at_zoom(3.6));
}

#[test]
fn overview_wonders_stay_at_their_sites() {
    let rect = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1000.0, 545.0));
    let projection = Projection {
        origin: rect.center(),
        scale: 12.0,
        center: [20.0, 40.0],
    };
    let wonders = layout_wonders(&projection, rect, MIN_ZOOM);
    assert_eq!(wonders.len(), WONDERS.len());
    assert_eq!(city_blend(MIN_ZOOM), 0.0);
    for wonder in &wonders {
        let site = projection.point(WONDERS[wonder.index].position);
        assert!(wonder.icon.center().distance(site) < 0.001);
    }
}

#[test]
fn every_wonder_has_illustration_at_close_zoom() {
    let rect = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1000.0, 545.0));
    for zoom in [CITY_BLEND_END, 4.0, MAX_ZOOM] {
        for (index, wonder) in WONDERS.iter().enumerate() {
            let projection = Projection {
                origin: rect.center(),
                scale: 14.7 * zoom,
                center: wonder.position,
            };
            let marker = layout_wonders(&projection, rect, zoom)
                .into_iter()
                .find(|marker| marker.index == index)
                .expect("the wonder site should remain on screen");
            let image = marker
                .image
                .unwrap_or_else(|| panic!("{} needs its illustration at zoom {zoom}", wonder.name));
            let site = projection.point(wonder.position);
            assert!(image.center().distance(site) < 0.001, "{} art moved", wonder.name);
            assert_eq!(marker.icon.center().x, site.x);
            assert!(marker.icon.bottom() < image.top(), "{} icon overlaps its art", wonder.name);
            assert!(marker.bounds(zoom).contains_rect(marker.icon));
            assert!(marker.bounds(zoom).contains_rect(image));
            assert!(
                (image.height() / zoom - 18.0).abs() < 0.001,
                "{} art changes map scale at zoom {zoom}",
                wonder.name,
            );
        }
    }
}

#[test]
fn wonders_do_not_share_city_sites() {
    for wonder in &WONDERS {
        for city in &CITIES {
            assert!(
                (wonder.position[0] - city.position[0]).abs() >= 0.05
                    || (wonder.position[1] - city.position[1]).abs() >= 0.05,
                "{} overlaps a city site",
                wonder.name,
            );
        }
    }
}

#[test]
fn wonders_are_hidden_until_started_and_keep_their_icon_with_construction_or_completed_art() {
    use crate::game::economy::{
        ConstructionProject, EconomicProvince, EconomyWorld, Terrain, WonderProject,
    };

    let ctx = egui::Context::default();
    let textures: Vec<_> =
        WONDERS.iter().map(|wonder| load_map_texture(&ctx, wonder.name, wonder.png)).collect();
    let province = EconomicProvince::new(
        "Wonder site",
        10.0,
        Terrain::Farmland,
        false,
        [1.0; 3],
        [10.0; 4],
        1,
    );
    let mut world = EconomyWorld::new(1, vec![province], vec![vec![]]);
    let rect = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1000.0, 545.0));
    for (index, wonder) in WONDERS.iter().enumerate() {
        world.provinces[0].wonder_sites = vec![index];
        for state in 0..3 {
            world.provinces[0].completed_wonder = (state == 2).then_some(index);
            world.provinces[0].construction =
                (state == 1).then_some(ConstructionProject::Wonder(WonderProject {
                    wonder_id: index,
                    progress: 12.0,
                    required_progress: 36.0,
                    assigned_slaves: 0.0,
                }));
            for zoom in [MIN_ZOOM, CITY_BLEND_END, MAX_ZOOM] {
                let projection = Projection {
                    origin: rect.center(),
                    scale: 14.7 * zoom,
                    center: wonder.position,
                };
                let markers = layout_wonders(&projection, rect, zoom);
                let mut output = ctx.run_ui(
                    egui::RawInput {
                        screen_rect: Some(rect),
                        ..Default::default()
                    },
                    |ui| paint_wonders(ui.painter(), &markers, zoom, &textures, Some(&world), 1.5),
                );
                let expected_texture = if state == 1 && zoom > MIN_ZOOM {
                    wonder_construction_texture(&ctx, index).id()
                } else {
                    textures[index].id()
                };
                let artwork: Vec<_> = output
                    .shapes
                    .iter()
                    .filter_map(|shape| match &shape.shape {
                        egui::Shape::Mesh(mesh) if mesh.texture_id == expected_texture => {
                            Some(mesh)
                        },
                        _ => None,
                    })
                    .collect();
                assert_eq!(
                    artwork.len(),
                    usize::from(state != 0 && zoom > MIN_ZOOM),
                    "{} art missing in construction state {state} at zoom {zoom}",
                    wonder.name
                );
                if let Some(mesh) = artwork.first() {
                    assert!(mesh.vertices.iter().all(|vertex| vertex.color.a() == 255));
                }
                let icons = output.shapes.iter().filter(|shape| {
                    matches!(&shape.shape, egui::Shape::Path(path)
                        if path.stroke.color == egui::epaint::ColorMode::Solid(egui::Color32::WHITE))
                }).count();
                assert_eq!(
                    icons,
                    if state == 0 {
                        0
                    } else {
                        3
                    },
                    "{} icon visibility in construction state {state} at zoom {zoom}",
                    wonder.name
                );
                if state == 0 {
                    assert!(output.shapes.is_empty(), "Unbuilt wonders must draw nothing");
                }
                output.textures_delta.clear();
            }
        }
    }
}

#[test]
fn every_canonical_wonder_has_one_province_and_no_province_has_multiple_wonders() {
    let mut ownership = ProvinceOwnership::default();
    ownership.start_game(&[egui::Color32::RED]);
    let seeds = ownership.campaign_seeds();
    let config = crate::game::economy::EconomyConfig::default();
    assert_eq!(WONDERS.len(), 7);
    assert_eq!(config.wonders.len(), WONDERS.len());
    for (id, wonder) in WONDERS.iter().enumerate() {
        assert_ne!(wonder.name, "Ay Khanum");
        assert_eq!(config.wonders[id].wonder_id, id);
        let hosts: Vec<_> = seeds.iter().filter(|p| p.wonder_sites.contains(&id)).collect();
        assert_eq!(
            hosts.len(),
            1,
            "{} must be buildable in exactly one playable province",
            wonder.name
        );
    }
    assert!(seeds.iter().flat_map(|p| &p.wonder_sites).all(|&id| id < WONDERS.len()));
    for province in &seeds {
        assert!(province.wonder_sites.len() <= 1, "{} has multiple wonder sites", province.name);
    }
    for (province_name, wonder_name) in
        [("Asia", "Colossus of Rhodes"), ("Achaia", "Temple of Zeus at Olympia")]
    {
        let province = seeds.iter().find(|p| p.name == province_name).unwrap();
        assert_eq!(province.wonder_sites.len(), 1);
        assert_eq!(WONDERS[province.wonder_sites[0]].name, wonder_name);
    }
}

#[test]
fn every_canonical_wonder_has_four_visible_transparent_construction_frames() {
    for wonder in &WONDERS {
        let completed = image::load_from_memory(wonder.png).expect("completed wonder art");
        assert!(completed.width() > 0 && completed.height() > 0);
        let sheet =
            image::load_from_memory(wonder.construction).expect("construction art").to_rgba8();
        assert_eq!(sheet.dimensions(), (512, 512), "{} runtime normalization", wonder.name);
        assert!(sheet.pixels().any(|p| p[3] == 0), "{} needs transparent padding", wonder.name);
        let frames: Vec<_> = (0..4)
            .map(|frame| {
                image::imageops::crop_imm(&sheet, (frame % 2) * 256, (frame / 2) * 256, 256, 256)
                    .to_image()
            })
            .collect();
        for frame in &frames {
            assert!(
                frame.pixels().filter(|p| p[3] > 64).count() > 1000,
                "{} has an empty frame",
                wonder.name
            );
        }
        assert!(
            frames.windows(2).any(|pair| pair[0] != pair[1]),
            "{} requires actual animation",
            wonder.name
        );
    }
}

#[test]
fn new_wonders_are_on_land_and_cities_stay_at_sites() {
    for name in ["Aqueduct of Segovia", "Pont du Gard"] {
        let wonder = WONDERS.iter().find(|wonder| wonder.name == name).unwrap();
        assert!(
            atlas().land.iter().any(|part| part.contains(wonder.position)),
            "{} must be on the map's land geometry",
            wonder.name,
        );
        let rect = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1000.0, 545.0));
        let projection = Projection {
            origin: rect.center(),
            scale: 14.7 * 8.0,
            center: wonder.position,
        };
        assert!(
            layout_wonders(&projection, rect, 8.0)
                .iter()
                .any(|marker| WONDERS[marker.index].name == wonder.name && marker.image.is_some()),
            "{} should be visible at close zoom",
            wonder.name,
        );
    }
    let rect = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1000.0, 545.0));
    let projection = Projection {
        origin: rect.center(),
        scale: 12.0,
        center: [20.0, 40.0],
    };
    let overview = layout_cities(&projection, rect, MIN_ZOOM);
    let close = layout_cities(&projection, rect, MAX_ZOOM);
    assert_eq!(overview.len(), CITIES.len());
    assert_eq!(close.len(), CITIES.len());
    for ((city, icon), illustration) in CITIES.iter().zip(&overview).zip(&close) {
        let anchor = projection.point(city.position);
        for image in [icon.icon, illustration.image] {
            let hotspot = image.min
                + egui::vec2(image.width() * city.hotspot[0], image.height() * city.hotspot[1]);
            assert!(hotspot.distance(anchor) < 0.001);
        }
        assert!(illustration.image.width() > icon.image.width());
    }
    assert!(overview[0].is_rome && close[0].is_rome);
    assert!(overview[0].icon.width() > overview[1].icon.width());
    assert!(close[0].image.width() > close[1].image.width());
    assert_eq!(city_blend(MIN_ZOOM), 0.0);
    assert_eq!(city_blend(MAX_ZOOM), 1.0);
}
