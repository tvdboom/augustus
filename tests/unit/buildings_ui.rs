//! Exercise the building grid through panel layout, pointer input and hover output.

use super::*;
use crate::game::economy::{BuildingType, EconomicProvince, EconomyWorld, Terrain};

fn fixture(city: bool) -> Campaign {
    let mut province = EconomicProvince::new(
        "Italia",
        60.0,
        Terrain::Farmland,
        city,
        [1.4, 0.8, 0.6],
        [8.0, 22.0, 33.0, 37.0],
        2,
    );
    province.owner = Some(0);
    province.wonder_sites = vec![0];
    let mut campaign = Campaign::default();
    campaign.economy = EconomyWorld::new(2, vec![province], vec![vec![]]);
    campaign.economy.players[0].resources = [1_000_000.0; 3];
    campaign
}

fn render(
    ctx: &egui::Context,
    campaign: &mut Campaign,
    size: egui::Vec2,
    scale: f32,
    status: &str,
    time: f64,
    events: Vec<egui::Event>,
) -> (egui::FullOutput, egui::Rect) {
    let panel = egui::Rect::from_min_size(egui::pos2(30.0, 30.0), size);
    let mut outer = panel;
    let mut view = CampaignUi {
        open: Some(CampaignTab::Province),
        province: Some(0),
        section: 2,
        ..Default::default()
    };
    let mut output = ctx.run_ui(
        egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1200.0, 900.0),
            )),
            time: Some(time),
            events,
            ..Default::default()
        },
        |root| {
            root.scope_builder(egui::UiBuilder::new().max_rect(panel), |ui| {
                panel_style(ui, scale);
                outer = panel_frame(scale)
                    .show(ui, |ui| {
                        ui.set_width(size.x);
                        ui.set_min_height(size.y);
                        panel_header(
                            ui,
                            "Italia",
                            scale,
                            &campaign.economy.provinces,
                            0,
                            &mut view,
                        );
                        egui::Frame::new()
                            .inner_margin(egui::Margin {
                                left: (12.0 * scale).round() as i8,
                                right: (12.0 * scale).round() as i8,
                                top: 0,
                                bottom: (12.0 * scale).round() as i8,
                            })
                            .show(ui, |ui| {
                                province_buildings_body(
                                    ui,
                                    panel.bottom() - 12.0 * scale,
                                    campaign,
                                    0,
                                    0,
                                    scale,
                                    status,
                                );
                            });
                    })
                    .response
                    .rect;
            });
        },
    );
    output.textures_delta.clear();
    (output, outer)
}

fn text_rect(output: &egui::FullOutput, label: &str) -> egui::Rect {
    output
        .shapes
        .iter()
        .find_map(|shape| match &shape.shape {
            egui::Shape::Text(text) if text.galley.job.text == label => {
                Some(text.galley.rect.translate(text.pos.to_vec2()))
            },
            _ => None,
        })
        .unwrap_or_else(|| panic!("Missing text: {label}"))
}

fn cell_rect(output: &egui::FullOutput, label: &str) -> egui::Rect {
    let title = text_rect(output, label);
    output
        .shapes
        .iter()
        .find_map(|shape| match &shape.shape {
            egui::Shape::Rect(rect)
                if rect.fill == province_panel::TABLE_STRIPE && rect.rect.contains_rect(title) =>
            {
                Some(rect.rect)
            },
            _ => None,
        })
        .expect("Building title must be inside a card")
}

#[test]
fn building_sections_stay_inside_the_panel_with_equal_width_cards() {
    for (width, height, scale) in
        [(570.0, 720.0, 1.0), (404.0, 540.0, 0.85), (404.0, 400.0, 1.0), (320.0, 360.0, 0.85)]
    {
        for city in [false, true] {
            for state in 0..6 {
                let ctx = egui::Context::default();
                let mut campaign = fixture(city);
                match state {
                    1 => campaign.economy.start_building(0, 0, BuildingType::Granary).unwrap(),
                    2 => campaign.economy.start_wonder(0, 0, 0).unwrap(),
                    3 => campaign.economy.provinces[0].completed_wonder = Some(0),
                    4 => {
                        campaign.economy.provinces[0].buildings[BuildingType::Granary as usize] = 50
                    },
                    5 => campaign.economy.provinces[0].wonder_sites.clear(),
                    _ => {},
                }
                for status in
                    [String::new(), "Construction status with a very long explanation. ".repeat(50)]
                {
                    for time in [0.0, 0.1] {
                        let (output, outer) = render(
                            &ctx,
                            &mut campaign,
                            egui::vec2(width, height),
                            scale,
                            &status,
                            time,
                            vec![],
                        );
                        let panel = egui::Rect::from_min_size(
                            egui::pos2(30.0, 30.0),
                            egui::vec2(width, height),
                        );
                        assert!(outer.right() <= panel.right() + 1.0 && outer.bottom() <= panel.bottom() + 1.0,
                            "Sections exceed {width}×{height}, city={city}, state={state}: {outer:?}");
                        let reference = cell_rect(&output, "Granary");
                        let cards: Vec<_> = output
                            .shapes
                            .iter()
                            .filter_map(|shape| match &shape.shape {
                                egui::Shape::Rect(rect)
                                    if rect.fill == province_panel::TABLE_STRIPE
                                        && (rect.rect.height() - reference.height()).abs()
                                            < 1.0 =>
                                {
                                    Some((rect.rect, shape.clip_rect))
                                },
                                _ => None,
                            })
                            .collect();
                        assert!(!cards.is_empty(), "The building grid must render cards");
                        for (card, clip) in cards {
                            assert!(
                                (card.width() - reference.width()).abs() < 1.0
                                    && card.height() <= 120.0 * scale + 1.0,
                                "Cards must share their row width and stay compact: {card:?}"
                            );
                            assert!(
                                card.right() <= panel.right() + 1.0
                                    && clip.bottom() <= panel.bottom() + 1.0,
                                "Scroll content must be clipped to the panel"
                            );
                        }
                        let labels: Vec<_> = output
                            .shapes
                            .iter()
                            .filter_map(|shape| match &shape.shape {
                                egui::Shape::Text(text) => Some(text.galley.job.text.as_str()),
                                _ => None,
                            })
                            .collect();
                        for (category, present) in
                            [("City", city), ("Countryside", true), ("Wonders", state != 5)]
                        {
                            assert_eq!(labels.contains(&category), present,
                                "Unexpected category visibility for {category}, city={city}, state={state}");
                        }
                    }
                }
            }
        }
    }
}

#[test]
fn four_building_cards_fill_each_category_width_evenly() {
    for (width, height, scale) in
        [(730.0, 720.0, 1.0), (570.0, 720.0, 1.0), (404.0, 540.0, 0.85), (320.0, 360.0, 0.85)]
    {
        for city in [false, true] {
            for owned in [false, true] {
                let ctx = egui::Context::default();
                let mut campaign = fixture(city);
                campaign.economy.provinces[0].wonder_sites.clear();
                if !owned {
                    campaign.economy.provinces[0].owner = Some(1);
                }
                let (output, _) =
                    render(&ctx, &mut campaign, egui::vec2(width, height), scale, "", 0.0, vec![]);
                let mut groups =
                    vec![("Countryside", ["Granary", "Warehouse", "Road", "Aqueduct"])];
                if city {
                    groups.extend([
                        ("City", ["Forum", "Baths", "Market", "Temple"]),
                        ("City", ["Arena", "Walls", "Academy", "Foundry"]),
                    ]);
                }
                for (category, names) in groups {
                    let title = text_rect(&output, category);
                    let rule = output
                        .shapes
                        .iter()
                        .filter_map(|shape| match &shape.shape {
                            egui::Shape::LineSegment {
                                points,
                                ..
                            } if points[0].y == points[1].y && points[0].y >= title.bottom() => {
                                Some(points)
                            },
                            _ => None,
                        })
                        .min_by(|a, b| a[0].y.total_cmp(&b[0].y))
                        .expect("Category must have an underline");
                    let cards = names.map(|name| cell_rect(&output, name));
                    assert!((cards[0].left() - rule[0].x).abs() < 0.01 && (cards[3].right() - rule[1].x).abs() < 0.01,
                        "Four cards must span {category}'s available width, {width}px, city={city}, owned={owned}: {cards:?}, rule={rule:?}");
                    let gap = cards[1].left() - cards[0].right();
                    for pair in cards.windows(2) {
                        assert!((pair[0].width() - pair[1].width()).abs() < 0.01);
                        assert!(
                            (pair[0].top() - pair[1].top()).abs() < 0.01,
                            "Keep all four cards on one row"
                        );
                        assert!((pair[1].left() - pair[0].right() - gap).abs() < 0.01);
                    }
                }
            }
        }
    }
}

#[test]
fn buildings_header_selects_city_or_village_and_keeps_both_textures_distinct() {
    let ctx = egui::Context::default();
    let mut campaign = fixture(true);
    for (time, city) in [(0.0, true), (0.1, false), (0.2, true), (0.3, false)] {
        campaign.economy.provinces[0].has_city = city;
        let (output, _) =
            render(&ctx, &mut campaign, egui::vec2(570.0, 720.0), 1.0, "", time, vec![]);
        let textures = ctx.tex_manager();
        let textures = textures.read();
        let painted_portraits: Vec<_> = output
            .shapes
            .iter()
            .filter_map(|shape| match &shape.shape {
                egui::Shape::Mesh(mesh) => textures
                    .meta(mesh.texture_id)
                    .map(|texture| texture.name.as_str())
                    .filter(|name| name.starts_with("campaign-portrait-")),
                _ => None,
            })
            .collect();
        assert_eq!(
            painted_portraits,
            [if city {
                "campaign-portrait-city"
            } else {
                "campaign-portrait-village"
            }]
        );
    }
}

#[test]
fn all_normal_buildings_fit_without_scrolling_when_there_is_no_wonder() {
    for scale in [1.0, 0.85, 0.7] {
        for city in [false, true] {
            for active in [false, true] {
                let ctx = egui::Context::default();
                let mut campaign = fixture(city);
                campaign.economy.provinces[0].wonder_sites.clear();
                if active {
                    campaign.economy.start_building(0, 0, BuildingType::Granary).unwrap();
                }
                let size = egui::vec2(570.0, 720.0) * scale;
                let status = if active {
                    "Granary started."
                } else {
                    ""
                };
                let (output, outer) = render(&ctx, &mut campaign, size, scale, status, 0.0, vec![]);
                let expected: Vec<_> = campaign
                    .economy
                    .config
                    .buildings
                    .iter()
                    .filter(|d| !d.requires_city || city)
                    .map(|d| d.building.name())
                    .collect();
                assert_eq!(
                    expected.len(),
                    if city {
                        12
                    } else {
                        4
                    }
                );
                let positions: Vec<_> = expected
                    .iter()
                    .map(|name| {
                        let card = cell_rect(&output, name);
                        assert!(
                            outer.contains_rect(card),
                            "{name} exceeds the panel at scale {scale}, city={city}, active={active}: card={card:?}, panel={outer:?}"
                        );
                        assert!(
                            output.shapes.iter().any(|shape| match &shape.shape {
                                egui::Shape::Rect(rect) =>
                                    rect.rect == card && shape.clip_rect.contains_rect(card),
                                _ => false,
                            }),
                            "{name} must be completely visible without scrolling"
                        );
                        card.min
                    })
                    .collect();
                for time in [0.1, 1.0] {
                    let (scrolled, _) = render(
                        &ctx,
                        &mut campaign,
                        size,
                        scale,
                        status,
                        time,
                        vec![
                            egui::Event::PointerMoved(
                                outer.right_bottom() - egui::vec2(16.0, 30.0),
                            ),
                            egui::Event::MouseWheel {
                                phase: egui::TouchPhase::Move,
                                unit: egui::MouseWheelUnit::Point,
                                delta: egui::vec2(0.0, -3000.0),
                                modifiers: egui::Modifiers::NONE,
                            },
                        ],
                    );
                    for (name, before) in expected.iter().zip(&positions) {
                        assert_eq!(
                            cell_rect(&scrolled, name).min,
                            *before,
                            "The no-wonder panel must not scroll at scale {scale}, city={city}, active={active}"
                        );
                    }
                }
            }
        }
    }
}

#[test]
fn scrolling_reaches_wonders_without_growing_the_panel() {
    let ctx = egui::Context::default();
    let mut campaign = fixture(true);
    let size = egui::vec2(570.0, 720.0);
    render(&ctx, &mut campaign, size, 1.0, "", 0.0, vec![]);
    render(
        &ctx,
        &mut campaign,
        size,
        1.0,
        "",
        0.1,
        vec![
            egui::Event::PointerMoved(egui::pos2(300.0, 600.0)),
            egui::Event::MouseWheel {
                phase: egui::TouchPhase::Move,
                unit: egui::MouseWheelUnit::Point,
                delta: egui::vec2(0.0, -3000.0),
                modifiers: egui::Modifiers::NONE,
            },
        ],
    );
    let (output, outer) = render(&ctx, &mut campaign, size, 1.0, "", 1.0, vec![]);
    let wonder = text_rect(&output, crate::map::wonder_name(0).unwrap());
    assert!(outer.contains_rect(wonder), "Wonders must be reachable by scrolling");
    assert!(outer.height() <= size.y + 1.0);
}

#[test]
fn image_name_level_and_empty_cell_space_all_start_the_quoted_upgrade_once() {
    for part in 0..4 {
        let ctx = egui::Context::default();
        let mut campaign = fixture(true);
        campaign.economy.provinces[0].buildings[BuildingType::Granary as usize] = 2;
        let (output, _) =
            render(&ctx, &mut campaign, egui::vec2(570.0, 720.0), 1.0, "", 0.0, vec![]);
        let card = cell_rect(&output, "Granary");
        let position = match part {
            0 => card.center_top() + egui::vec2(0.0, card.height() * 0.3),
            1 => text_rect(&output, "Granary").center(),
            2 => text_rect(&output, "Level 2").center(),
            _ => card.left_top() + egui::vec2(4.0, card.height() * 0.5),
        };
        let definition = campaign
            .economy
            .config
            .buildings
            .iter()
            .find(|d| d.building == BuildingType::Granary)
            .unwrap();
        let quote = definition.quote(2);
        let before = campaign.economy.players[0].resources;
        for (time, pressed) in [(0.1, true), (0.2, false)] {
            render(
                &ctx,
                &mut campaign,
                egui::vec2(570.0, 720.0),
                1.0,
                "",
                time,
                vec![
                    egui::Event::PointerMoved(position),
                    egui::Event::PointerButton {
                        pos: position,
                        button: egui::PointerButton::Primary,
                        pressed,
                        modifiers: egui::Modifiers::NONE,
                    },
                ],
            );
        }
        assert!(
            matches!(&campaign.economy.provinces[0].construction, Some(ConstructionProject::Building(p)) if p.building == BuildingType::Granary && p.target_level == 3),
            "Cell part {part} did not start construction"
        );
        assert_eq!(campaign.economy.players[0].resources[1], before[1] - quote.metal);
        assert_eq!(campaign.economy.players[0].resources[2], before[2] - quote.stone);
        let notices = campaign.notifications.drain_for(0);
        assert_eq!(notices.len(), 1, "One successful click must create one start toast");
        let notice = &notices[0];
        assert_eq!(
            notice.kind,
            super::super::campaign_notifications::NoticeKind::ConstructionStarted
        );
        assert_eq!(notice.severity, super::super::campaign_notifications::NoticeSeverity::Info);
        assert_eq!(notice.title, "Granary started");
        assert_eq!(notice.action, NoticeAction::OpenProvince(0));
        assert!(campaign.notifications.drain_for(1).is_empty());
    }
}

#[test]
fn unavailable_cells_keep_effect_tooltips_and_do_not_build() {
    for reason in 0..3 {
        let ctx = egui::Context::default();
        let mut campaign = fixture(true);
        campaign.economy.config.buildings[0].effects.storage[0] = 735.0;
        match reason {
            0 => campaign.economy.provinces[0].owner = Some(1),
            1 => campaign.economy.players[0].resources = [0.0; 3],
            _ => campaign.economy.start_building(0, 0, BuildingType::Road).unwrap(),
        }
        let before = campaign.economy.players[0].resources;
        let (output, _) =
            render(&ctx, &mut campaign, egui::vec2(570.0, 720.0), 1.0, "", 0.0, vec![]);
        let position = cell_rect(&output, "Granary").center();
        render(
            &ctx,
            &mut campaign,
            egui::vec2(570.0, 720.0),
            1.0,
            "",
            0.1,
            vec![egui::Event::PointerMoved(position)],
        );
        render(&ctx, &mut campaign, egui::vec2(570.0, 720.0), 1.0, "", 1.0, vec![]);
        let (output, _) =
            render(&ctx, &mut campaign, egui::vec2(570.0, 720.0), 1.0, "", 1.1, vec![]);
        let tooltip = output
            .shapes
            .iter()
            .filter_map(|shape| match &shape.shape {
                egui::Shape::Text(text) => Some(text.galley.job.text.as_str()),
                _ => None,
            })
            .collect::<Vec<_>>()
            .join("\n");
        assert!(
            tooltip.contains("+735 Food storage"),
            "Hover must describe configured effects: {tooltip}"
        );
        let expected = match reason {
            0 => "Direct ownership required.",
            1 => "Insufficient global Stone or Metal.",
            _ => "Construction slot occupied.",
        };
        assert!(tooltip.contains(expected), "Hover must explain availability: {tooltip}");
        for (time, pressed) in [(1.2, true), (1.3, false)] {
            render(
                &ctx,
                &mut campaign,
                egui::vec2(570.0, 720.0),
                1.0,
                "",
                time,
                vec![
                    egui::Event::PointerMoved(position),
                    egui::Event::PointerButton {
                        pos: position,
                        button: egui::PointerButton::Primary,
                        pressed,
                        modifiers: egui::Modifiers::NONE,
                    },
                ],
            );
        }
        assert_eq!(campaign.economy.players[0].resources, before);
        assert!(campaign.economy.provinces[0].construction.as_ref().is_none_or(|p| matches!(p, ConstructionProject::Building(project) if project.building == BuildingType::Road)));
    }
}

#[test]
fn building_costs_and_effects_move_into_an_illustrated_hover_card() {
    let ctx = egui::Context::default();
    let mut campaign = fixture(false);
    campaign.economy.provinces[0].buildings[BuildingType::Granary as usize] = 2;
    let size = egui::vec2(570.0, 720.0);
    let (normal, _) = render(&ctx, &mut campaign, size, 1.0, "", 0.0, vec![]);
    let card = cell_rect(&normal, "Granary");
    let labels: Vec<_> = normal
        .shapes
        .iter()
        .filter_map(|shape| match &shape.shape {
            egui::Shape::Text(text) if card.contains(text.pos) => {
                Some(text.galley.job.text.as_str())
            },
            _ => None,
        })
        .collect();
    assert_eq!(labels, ["Granary", "Level 2"]);
    render(&ctx, &mut campaign, size, 1.0, "", 0.1, vec![egui::Event::PointerMoved(card.center())]);
    render(&ctx, &mut campaign, size, 1.0, "", 1.0, vec![]);
    let (hover, _) = render(&ctx, &mut campaign, size, 1.0, "", 1.1, vec![]);
    let quote = campaign
        .economy
        .config
        .buildings
        .iter()
        .find(|definition| definition.building == BuildingType::Granary)
        .unwrap()
        .quote(2);
    let stone = text_rect(&hover, &format!("{:.0} Stone", quote.stone));
    let metal = text_rect(&hover, &format!("{:.0} Metal", quote.metal));
    let title = hover
        .shapes
        .iter()
        .find_map(|shape| match &shape.shape {
            egui::Shape::Text(text)
                if text.galley.job.text == "Granary"
                    && text.galley.job.sections[0].format.font_id.size == 16.0 =>
            {
                Some(text.pos)
            },
            _ => None,
        })
        .expect("Hover card must repeat the building name");
    let description = hover
        .shapes
        .iter()
        .find_map(|shape| match &shape.shape {
            egui::Shape::Text(text)
                if text.galley.job.text.contains("Next level: 3")
                    && text.galley.job.text.contains("+600 Food storage") =>
            {
                Some(text.pos)
            },
            _ => None,
        })
        .expect("Hover card must describe the next upgrade and its configured effects");
    assert!(title.y < stone.top() && stone.bottom() <= description.y);
    assert!((stone.top() - metal.top()).abs() < 1.0);
    assert!(
        hover.shapes.iter().any(|shape| match &shape.shape {
            egui::Shape::Mesh(mesh) => {
                let bounds = mesh.calc_bounds();
                (bounds.width() - 64.0).abs() < 1.0
                    && (bounds.height() - 64.0).abs() < 1.0
                    && bounds.right() < title.x
                    && (bounds.top() - title.y).abs() < 3.0
            },
            _ => false,
        }),
        "The smaller illustration must sit to the left of the name"
    );
}

#[test]
fn the_active_card_shows_progress_and_the_busy_slot_blocks_another_click() {
    let ctx = egui::Context::default();
    let mut campaign = fixture(false);
    campaign.economy.start_building(0, 0, BuildingType::Granary).unwrap();
    if let Some(ConstructionProject::Building(project)) =
        &mut campaign.economy.provinces[0].construction
    {
        project.progress = project.required_progress * 0.5;
    }
    let size = egui::vec2(570.0, 720.0);
    let (output, _) = render(&ctx, &mut campaign, size, 1.0, "", 0.0, vec![]);
    let active = cell_rect(&output, "Granary");
    let progress = text_rect(&output, "50% · 2mo");
    assert!(
        active.contains_rect(progress) && progress.top() > active.top() + active.height() * 0.85,
        "The progress bar must be in the bottom of its building card"
    );
    let fills: Vec<_> = output
        .shapes
        .iter()
        .filter_map(|shape| match &shape.shape {
            egui::Shape::Rect(rect)
                if rect.fill == egui::Color32::from_rgb(190, 150, 76)
                    && active.contains_rect(rect.rect) =>
            {
                Some(rect.rect)
            },
            _ => None,
        })
        .collect();
    assert_eq!(fills.len(), 1);
    assert!((fills[0].width() - (active.width() - 10.0) * 0.5).abs() < 1.0);
    let another = cell_rect(&output, "Warehouse").center();
    let balances = campaign.economy.players[0].resources;
    for (time, pressed) in [(0.1, true), (0.2, false)] {
        render(
            &ctx,
            &mut campaign,
            size,
            1.0,
            "",
            time,
            vec![
                egui::Event::PointerMoved(another),
                egui::Event::PointerButton {
                    pos: another,
                    button: egui::PointerButton::Primary,
                    pressed,
                    modifiers: egui::Modifiers::NONE,
                },
            ],
        );
    }
    assert_eq!(campaign.economy.players[0].resources, balances);
    assert!(matches!(&campaign.economy.provinces[0].construction,
        Some(ConstructionProject::Building(project)) if project.building == BuildingType::Granary));
    assert!(
        campaign.notifications.drain_for(0).is_empty(),
        "Blocked clicks must not announce a start"
    );
    while campaign.economy.provinces[0].construction.is_some() {
        campaign.economy.advance_month(&Default::default());
    }
    render(&ctx, &mut campaign, size, 1.0, "", 1.0, vec![]);
    for (time, pressed) in [(1.1, true), (1.2, false)] {
        render(
            &ctx,
            &mut campaign,
            size,
            1.0,
            "",
            time,
            vec![
                egui::Event::PointerMoved(another),
                egui::Event::PointerButton {
                    pos: another,
                    button: egui::PointerButton::Primary,
                    pressed,
                    modifiers: egui::Modifiers::NONE,
                },
            ],
        );
    }
    assert!(
        matches!(&campaign.economy.provinces[0].construction,
        Some(ConstructionProject::Building(project)) if project.building == BuildingType::Warehouse),
        "Completing the project must re-enable building buttons"
    );
}
