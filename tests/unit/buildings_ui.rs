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
        notice: status.to_owned(),
        confirmation: ctx.data(|data| {
            data.get_temp::<Option<campaign_confirmation::PendingConfirmation>>(egui::Id::new(
                "test-confirmation",
            ))
            .flatten()
        }),
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
                            None,
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
                                    &mut view,
                                );
                            });
                    })
                    .response
                    .rect;
            });
            campaign_confirmation::show(root.ctx(), &mut view, campaign);
        },
    );
    ctx.data_mut(|data| data.insert_temp(egui::Id::new("test-confirmation"), view.confirmation));
    output.textures_delta.clear();
    if !status.is_empty() {
        assert!(!output.shapes.iter().any(|shape| matches!(&shape.shape, egui::Shape::Text(text) if text.galley.job.text == status)), "Action text must not appear in the building footer");
    }
    (output, outer)
}

fn confirm_yes(
    ctx: &egui::Context,
    campaign: &mut Campaign,
    size: egui::Vec2,
    scale: f32,
    time: f64,
) {
    let (output, _) = render(ctx, campaign, size, scale, "", time, vec![]);
    let yes = text_rect(&output, "Yes").center();
    for (offset, pressed) in [(0.01, true), (0.02, false)] {
        render(
            ctx,
            campaign,
            size,
            scale,
            "",
            time + offset,
            vec![
                egui::Event::PointerMoved(yes),
                egui::Event::PointerButton {
                    pos: yes,
                    button: egui::PointerButton::Primary,
                    pressed,
                    modifiers: egui::Modifiers::NONE,
                },
            ],
        );
    }
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
        .filter_map(|shape| match &shape.shape {
            egui::Shape::Rect(rect)
                if rect.rect.height() > 45.0
                    && rect.rect.height() <= 121.0
                    && rect.rect.contains_rect(title) =>
            {
                Some(rect.rect)
            },
            _ => None,
        })
        .min_by(|a, b| (a.width() * a.height()).total_cmp(&(b.width() * b.height())))
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
                        assert!(
                            outer.right() <= panel.right() + 1.0
                                && outer.bottom() <= panel.bottom() + 1.0,
                            "Sections exceed {width}×{height}, city={city}, state={state}: {outer:?}"
                        );
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
                        assert!(
                            !cards.is_empty(),
                            "The building grid must render cards: width={width}, height={height}, scale={scale}, city={city}, state={state}, reference={reference:?}"
                        );
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
                            assert_eq!(
                                labels.contains(&category),
                                present,
                                "Unexpected category visibility for {category}, city={city}, state={state}"
                            );
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
                    vec![("Countryside", ["Granary", "Road", "Warehouse", "Aqueduct"])];
                if city {
                    groups.extend([
                        ("City", ["Market", "Temple", "City Hall", "Forum"]),
                        ("City", ["Baths", "Academy", "Arena", "Walls"]),
                    ]);
                }
                for (category, names) in groups {
                    let title = text_rect(&output, category);
                    let header = output
                        .shapes
                        .iter()
                        .filter_map(|shape| match &shape.shape {
                            egui::Shape::Rect(rect)
                                if rect.fill == egui::Color32::from_rgb(73, 69, 61)
                                    && rect.rect.contains_rect(title) =>
                            {
                                Some(rect.rect)
                            },
                            _ => None,
                        })
                        .next()
                        .expect("Category must have the shared dark header");
                    let cards = names.map(|name| cell_rect(&output, name));
                    assert!(
                        (cards[0].left() - header.left()).abs() < 0.01
                            && (cards[3].right() - header.right()).abs() < 0.01,
                        "Four cards must span {category}'s available width, {width}px, city={city}, owned={owned}: {cards:?}, header={header:?}"
                    );
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
fn building_cards_follow_the_current_upgrade_cost() {
    let ctx = egui::Context::default();
    let mut campaign = fixture(true);
    campaign.economy.provinces[0].wonder_sites.clear();
    let (first, _) = render(&ctx, &mut campaign, egui::vec2(570.0, 720.0), 1.0, "", 0.0, vec![]);
    assert!(cell_rect(&first, "Market").left() < cell_rect(&first, "Temple").left());

    campaign.economy.provinces[0].buildings[BuildingType::UrbanMarket as usize] = 1;
    let (upgraded, _) = render(&ctx, &mut campaign, egui::vec2(570.0, 720.0), 1.0, "", 0.1, vec![]);
    let market = cell_rect(&upgraded, "Market");
    let arena = cell_rect(&upgraded, "Arena");
    let walls = cell_rect(&upgraded, "Walls");
    assert_eq!(market.top(), arena.top());
    assert!(arena.left() < market.left() && market.left() < walls.left());
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
        assert!(campaign.notifications.drain_for(0).is_empty());
        assert_eq!(campaign.notifications.history_for(0).count(), 0);
        assert!(audio_controls::take_construction_sound(&ctx));
        assert!(!audio_controls::take_construction_sound(&ctx), "Play once per accepted click");
        assert!(campaign.notifications.drain_for(1).is_empty());
    }
}

#[test]
fn unavailable_cells_keep_effect_tooltips_and_do_not_build() {
    for reason in [0, 1, 3, 4, 5, 6] {
        let ctx = egui::Context::default();
        super::super::campaign_widgets::configure_cursor(&ctx);
        let mut campaign = fixture(reason != 3);
        campaign.economy.config.buildings[0].effects.storage[0] = 735.0;
        match reason {
            0 => campaign.economy.provinces[0].owner = Some(1),
            1 => campaign.economy.players[0].resources = [0.0; 3],
            2 => campaign.economy.start_building(0, 0, BuildingType::Road).unwrap(),
            3 => {
                campaign.economy.provinces[0].wonder_sites = vec![2];
                campaign.economy.provinces[0].completed_wonder = Some(2);
            },
            4 => campaign.economy.players[0].resources[2] = 0.0,
            5 => campaign.economy.players[0].resources[1] = 0.0,
            _ => {
                campaign.economy.start_building(0, 0, BuildingType::Granary).unwrap();
                campaign.economy.players[0].resources = [0.0; 3];
            },
        }
        let before = campaign.economy.players[0].resources;
        let (output, _) =
            render(&ctx, &mut campaign, egui::vec2(570.0, 720.0), 1.0, "", 0.0, vec![]);
        let name = if reason == 3 {
            "Temple of Zeus"
        } else {
            "Granary"
        };
        let card = cell_rect(&output, name);
        let position = card.center();
        assert!(
            output.shapes.iter().any(|shape| matches!(&shape.shape,
            egui::Shape::Rect(rect) if rect.rect == card
                && rect.stroke == egui::Stroke::new(1.0, province_panel::RULE))),
            "Unavailable cards must share the neutral border, including active construction"
        );
        if reason == 3 {
            assert!(output.shapes.iter().any(|shape| matches!(&shape.shape,
                egui::Shape::Text(text) if text.galley.job.text == name
                    && text.galley.job.sections[0].format.font_id.size == 14.0)));
            assert!(!output.shapes.iter().any(|shape| matches!(&shape.shape,
                egui::Shape::Text(text) if matches!(text.galley.job.text.as_str(), "Not built" | "Completed"))));
        }
        let (hover, _) = render(
            &ctx,
            &mut campaign,
            egui::vec2(570.0, 720.0),
            1.0,
            "",
            0.1,
            vec![egui::Event::PointerMoved(position)],
        );
        assert_eq!(hover.platform_output.cursor_icon, egui::CursorIcon::Default);
        assert!(hover.shapes.iter().any(|shape| matches!(&shape.shape,
            egui::Shape::Rect(rect) if rect.rect.contains(position)
                && rect.fill == super::super::campaign_widgets::UNAVAILABLE_PURCHASE_FILL)));
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
            reason == 3 || tooltip.contains("+735 Food storage"),
            "Hover must describe configured effects: {tooltip}"
        );
        let expected = match reason {
            0 => "Province not owned.",
            1 => "Not enough Stone and Metal.",
            2 => "Another building is in progress.",
            3 => "Already built.",
            4 => "Not enough Stone.",
            5 => "Not enough Metal.",
            _ => "Not enough Stone and Metal.",
        };
        assert!(tooltip.contains(expected), "Hover must explain availability: {tooltip}");
        assert!(output.shapes.iter().any(|shape| matches!(&shape.shape,
            egui::Shape::Text(text) if text.galley.job.text == expected
                && text.galley.job.sections[0].format.color == egui::Color32::from_rgb(170, 45, 35))));
        assert!(
            !tooltip.contains("Direct ownership required")
                && !tooltip.contains("Only the direct owner")
                && !tooltip.contains("Public completed level")
        );
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
        assert!(campaign.economy.provinces[0].construction_queue.is_empty());
        assert!(!audio_controls::take_construction_sound(&ctx));
        assert!(campaign.economy.provinces[0].construction.as_ref().is_none_or(|p| matches!(p, ConstructionProject::Building(project) if project.building == if reason == 6 { BuildingType::Granary } else { BuildingType::Road })));
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
    let level = text_rect(&normal, "Level 2");
    assert!(
        card.contains_rect(level)
            && level.right() > card.center().x
            && level.bottom() < card.top() + 22.0
    );
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
    let stone = text_rect(&hover, &format!("{:.0}", quote.stone));
    let metal = text_rect(&hover, &format!("{:.0}", quote.metal));
    let title = hover
        .shapes
        .iter()
        .find_map(|shape| match &shape.shape {
            egui::Shape::Text(text)
                if text.galley.job.text == "Granary"
                    && text.galley.job.sections[0].format.font_id.size == 20.0 =>
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
            egui::Shape::Text(text) if text.galley.job.text.contains("keeps the harvest dry") => {
                Some(text.pos)
            },
            _ => None,
        })
        .expect("Hover card must describe the building");
    assert!(title.y < stone.top() && stone.bottom() <= description.y);
    assert!((description.x - title.x).abs() < 1.0);
    let heading = text_rect(&hover, "Each completed level:");
    let effect = text_rect(&hover, "+600 Food storage");
    assert!(description.y < heading.top() && heading.bottom() <= effect.top());
    assert!((heading.left() - title.x).abs() < 1.0 && effect.left() > title.x);
    assert!(hover.shapes.iter().any(|shape| matches!(&shape.shape,
        egui::Shape::Text(text) if text.galley.job.text == "•")));
    assert!((stone.top() - metal.top()).abs() < 1.0);
    assert!(
        hover.shapes.iter().any(|shape| match &shape.shape {
            egui::Shape::Mesh(mesh) => {
                let bounds = mesh.calc_bounds();
                (bounds.width() - 112.0).abs() < 1.0
                    && (bounds.height() - 112.0).abs() < 1.0
                    && bounds.right() + 12.0 <= title.x
                    && description.y < bounds.bottom()
                    && (bounds.top() - title.y).abs() < 3.0
            },
            _ => false,
        }),
        "The enlarged illustration must sit to the left of the name"
    );
}

#[test]
fn full_wonder_names_fit_the_cards_at_narrow_widths_and_during_construction() {
    for (width, scale) in [(570.0, 1.0), (420.0, 0.85), (320.0, 1.0)] {
        for wonder in 0..crate::map::WONDER_COUNT {
            for active in [false, true] {
                let ctx = egui::Context::default();
                let mut campaign = fixture(false);
                campaign.economy.provinces[0].wonder_sites = vec![wonder];
                if active {
                    campaign.economy.start_wonder(0, 0, wonder).unwrap();
                }
                let (output, _) =
                    render(&ctx, &mut campaign, egui::vec2(width, 720.0), scale, "", 0.0, vec![]);
                let name = crate::map::wonder_name(wonder).unwrap();
                let card = cell_rect(&output, name);
                let text = output
                    .shapes
                    .iter()
                    .find_map(|shape| match &shape.shape {
                        egui::Shape::Text(text) if text.galley.job.text == name => Some(text),
                        _ => None,
                    })
                    .unwrap();
                let bounds = text.galley.rect.translate(text.pos.to_vec2());
                assert!(
                    !text.galley.elided && text.galley.rows.len() <= 2,
                    "{name} must appear in full at width {width}"
                );
                assert!(card.contains_rect(bounds), "{name} escapes its card: {bounds:?}");
                if active {
                    assert!(
                        progress_fill(&output).bottom() < card.top(),
                        "The construction strip must precede the wonder card"
                    );
                }
            }
        }
    }
}

#[test]
fn foreign_hover_cards_show_real_costs_and_omit_zero_cost_resources() {
    for kind in 0..3 {
        let ctx = egui::Context::default();
        let mut campaign = fixture(false);
        campaign.economy.provinces[0].owner = Some(1);
        let (name, title, costs) = match kind {
            0 => {
                campaign.economy.config.buildings[0].stone_cost = 0.0;
                campaign.economy.config.buildings[0].metal_cost = 0.0;
                ("Granary", "Granary", vec![])
            },
            1 => {
                campaign.economy.provinces[0].buildings[BuildingType::Warehouse as usize] = 2;
                let quote = campaign
                    .economy
                    .config
                    .buildings
                    .iter()
                    .find(|definition| definition.building == BuildingType::Warehouse)
                    .unwrap()
                    .quote(2);
                ("Warehouse", "Warehouse", vec![quote.stone, quote.metal])
            },
            _ => {
                campaign.economy.provinces[0].wonder_sites = vec![3];
                let definition = &campaign.economy.config.wonders[3];
                (
                    "Palace of the Argeads",
                    "Palace of the Argeads",
                    vec![definition.stone_cost, definition.metal_cost],
                )
            },
        };
        let size = egui::vec2(570.0, 720.0);
        let (normal, _) = render(&ctx, &mut campaign, size, 1.0, "", 0.0, vec![]);
        let card = cell_rect(&normal, name);
        render(
            &ctx,
            &mut campaign,
            size,
            1.0,
            "",
            0.1,
            vec![egui::Event::PointerMoved(card.center())],
        );
        render(&ctx, &mut campaign, size, 1.0, "", 1.0, vec![]);
        let (hover, _) = render(&ctx, &mut campaign, size, 1.0, "", 1.1, vec![]);
        assert!(hover.shapes.iter().any(|shape| matches!(&shape.shape,
            egui::Shape::Text(text) if text.galley.job.text == title
                && text.galley.job.sections[0].format.font_id.size == 20.0)));
        let numbers: Vec<_> = hover
            .shapes
            .iter()
            .filter_map(|shape| match &shape.shape {
                egui::Shape::Text(text)
                    if text.galley.job.sections[0].format.font_id.size == 17.0 =>
                {
                    Some(text.galley.job.text.clone())
                },
                _ => None,
            })
            .collect();
        assert_eq!(numbers, costs.iter().map(|cost| format!("{cost:.0}")).collect::<Vec<_>>());
        let reason = text_rect(&hover, "Province not owned.");
        assert!(hover.shapes.iter().any(|shape| matches!(&shape.shape,
            egui::Shape::Rect(rect)
                if rect.rect.contains(reason.center())
                    && rect.fill.r() > rect.fill.g()
                    && rect.fill.g() > rect.fill.b()
                    && rect.fill.b() > 150
        )));
        let description = hover
            .shapes
            .iter()
            .find_map(|shape| match &shape.shape {
                egui::Shape::Text(text)
                    if text.galley.job.text.starts_with(match kind {
                        0 => "A sheltered storehouse",
                        1 => "Sturdy storerooms",
                        _ => "A royal palace",
                    }) =>
                {
                    Some(text.galley.rect.translate(text.pos.to_vec2()))
                },
                _ => None,
            })
            .expect("Each building and wonder needs a short description");
        assert!(reason.top() - description.bottom() >= 10.0);
        if let Some(cost) = costs.first() {
            let cost = text_rect(&hover, &format!("{cost:.0}"));
            assert!(description.top() - cost.bottom() >= 10.0);
        }
    }
}

#[test]
fn the_queue_strip_shows_progress_and_another_click_queues_a_paid_upgrade() {
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
    let progress = text_rect(&output, "50%");
    assert!(
        progress.bottom() < text_rect(&output, "Countryside").top()
            && progress.bottom() < active.top(),
        "The percentage must appear above the first category heading"
    );
    let level = text_rect(&output, "Level 0 → 1");
    assert!(
        active.contains_rect(level)
            && level.right() > active.center().x
            && level.bottom() < active.top() + 22.0
    );
    let fill = progress_fill(&output);
    assert!(!active.contains_rect(fill));
    assert!((fill.height() - 18.0).abs() < 1.0);
    assert!((progress.center().y - fill.center().y).abs() < 1.0);
    let track = output
        .shapes
        .iter()
        .find_map(|shape| match &shape.shape {
            egui::Shape::Rect(rect)
                if rect.fill == province_panel::RULE && rect.rect.contains_rect(progress) =>
            {
                Some(rect.rect)
            },
            _ => None,
        })
        .unwrap();
    assert!((fill.width() - track.width() * 0.5).abs() < 1.0);
    assert!((progress.center().x - track.center().x).abs() < 1.0);
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
    assert_eq!(campaign.economy.players[0].resources[1], balances[1] - 20.);
    assert_eq!(campaign.economy.players[0].resources[2], balances[2] - 140.);
    assert_eq!(campaign.economy.provinces[0].construction_queue.len(), 1);
    assert!(matches!(&campaign.economy.provinces[0].construction,
        Some(ConstructionProject::Building(project)) if project.building == BuildingType::Granary));
    assert!(
        campaign.notifications.drain_for(0).is_empty(),
        "Queued projects must not create start notifications"
    );
    assert_eq!(campaign.notifications.history_for(0).count(), 0);
    assert!(audio_controls::take_construction_sound(&ctx));
    assert!(!audio_controls::take_construction_sound(&ctx));
    while campaign.economy.provinces[0].construction.is_some() {
        campaign.economy.advance_month(&Default::default());
    }
    let (output, _) = render(&ctx, &mut campaign, size, 1.0, "", 1.0, vec![]);
    let another = cell_rect(&output, "Warehouse").center();
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
    assert!(audio_controls::take_construction_sound(&ctx));
    assert!(campaign.notifications.drain_for(0).is_empty());
    assert_eq!(campaign.notifications.history_for(0).count(), 0);
}

fn progress_fill(output: &egui::FullOutput) -> egui::Rect {
    output
        .shapes
        .iter()
        .find_map(|shape| match &shape.shape {
            egui::Shape::Rect(rect) if rect.fill == egui::Color32::from_rgb(190, 150, 76) => {
                Some(rect.rect)
            },
            _ => None,
        })
        .expect("The construction strip must contain a progress fill")
}

#[test]
fn construction_bar_moves_between_months_without_advancing_the_project() {
    let ctx = egui::Context::default();
    let mut campaign = fixture(false);
    campaign.economy.start_building(0, 0, BuildingType::Granary).unwrap();
    let size = egui::vec2(570.0, 720.0);
    let mut previous_width = 0.0;
    for (frame, fraction) in [0.25_f32, 0.5, 0.75, 0.999].into_iter().enumerate() {
        ctx.data_mut(|data| {
            data.insert_temp(egui::Id::new("campaign-construction-month-fraction"), fraction)
        });
        let (output, _) = render(&ctx, &mut campaign, size, 1.0, "", frame as f64, vec![]);
        let fill = progress_fill(&output);
        assert!(
            fill.width() > previous_width,
            "The bar must move within the same simulation month"
        );
        previous_width = fill.width();
        assert_eq!(campaign.economy.provinces[0].construction.as_ref().unwrap().progress().0, 0.0);
        assert_eq!(campaign.economy.provinces[0].level(BuildingType::Granary), 0);
    }
    let (paused, _) = render(&ctx, &mut campaign, size, 1.0, "", 10.0, vec![]);
    assert_eq!(
        progress_fill(&paused).width(),
        previous_width,
        "A stationary game clock must freeze the bar even while wall time passes"
    );
    campaign.economy.advance_month(&Default::default());
    ctx.data_mut(|data| {
        data.insert_temp(egui::Id::new("campaign-construction-month-fraction"), 0.0_f32)
    });
    let (next_month, _) = render(&ctx, &mut campaign, size, 1.0, "", 11.0, vec![]);
    assert!(
        (progress_fill(&next_month).width() * 0.999 - previous_width).abs() < 0.1,
        "The bar must meet the actual progress smoothly at the month boundary"
    );
}

#[test]
fn active_building_and_wonder_cancel_icons_release_the_slot_without_refunding() {
    for wonder in [false, true] {
        for scale in [1.0, 0.85] {
            let ctx = egui::Context::default();
            let mut campaign = fixture(false);
            let name = if wonder {
                campaign.economy.start_wonder(0, 0, 0).unwrap();
                crate::map::wonder_name(0).unwrap()
            } else {
                campaign.economy.start_building(0, 0, BuildingType::Granary).unwrap();
                "Granary"
            };
            let balances = campaign.economy.players[0].resources;
            let size = egui::vec2(570.0, 720.0) * scale;
            render(&ctx, &mut campaign, size, scale, "", 0.0, vec![]);
            let (output, _) = render(&ctx, &mut campaign, size, scale, "", 0.1, vec![]);
            let card = cell_rect(&output, name);
            assert!(!output.shapes.iter().any(|shape| matches!(&shape.shape,
                egui::Shape::Text(text) if text.galley.job.text == "Cancel"
                    || text.galley.job.text.contains(" · Level "))));
            let position = text_rect(&output, "\u{00d7}").center();
            assert!(position.y < card.top());
            for (time, pressed) in [(0.2, true), (0.3, false)] {
                render(
                    &ctx,
                    &mut campaign,
                    size,
                    scale,
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
                campaign.economy.provinces[0].construction.is_some(),
                "Opening confirmation must preserve construction"
            );
            confirm_yes(&ctx, &mut campaign, size, scale, 0.31);
            assert!(
                campaign.economy.provinces[0].construction.is_none(),
                "The cancel control in the construction strip must cancel"
            );
            assert_eq!(
                campaign.economy.players[0].resources, balances,
                "Cancellation must not refund the payment"
            );
            assert_eq!(campaign.economy.provinces[0].level(BuildingType::Granary), 0);
            assert!(campaign.economy.provinces[0].completed_wonder.is_none());
            campaign.economy.start_building(0, 0, BuildingType::Road).unwrap();
        }
    }
}

#[test]
fn right_clicking_the_shared_queue_refunds_the_correct_global_order() {
    let ctx = egui::Context::default();
    let mut campaign = fixture(true);
    for building in [
        BuildingType::Academy,
        BuildingType::Granary,
        BuildingType::CityHall,
        BuildingType::Academy,
        BuildingType::Road,
    ] {
        campaign.economy.start_building(0, 0, building).unwrap();
    }
    let receipt = match &campaign.economy.provinces[0].construction_queue[2] {
        ConstructionProject::Building(p) => p.clone(),
        _ => unreachable!(),
    };
    let paid = campaign.economy.players[0].resources;
    let size = egui::vec2(570.0, 720.0);
    render(&ctx, &mut campaign, size, 1.0, "", 0.0, vec![]);
    let (output, _) = render(&ctx, &mut campaign, size, 1.0, "", 0.1, vec![]);
    let city = text_rect(&output, "City");
    let construction = text_rect(&output, "Construction");
    let progress = text_rect(&output, "In progress");
    let queue = text_rect(&output, "Queue");
    let banner = output
        .shapes
        .iter()
        .find_map(|shape| match &shape.shape {
            egui::Shape::Mesh(mesh) if mesh.calc_bounds().height() == 76.0 => {
                Some(mesh.calc_bounds())
            },
            _ => None,
        })
        .expect("The province banner must be painted");
    assert!(banner.bottom() <= construction.top());
    assert!(construction.bottom() < progress.top());
    assert!(progress.bottom() < queue.top() && queue.bottom() < city.top());
    assert!(!output.shapes.iter().any(|shape| matches!(&shape.shape,
        egui::Shape::Text(text) if text.galley.job.text.contains("queued")
            || text.galley.job.text == "Q" || text.galley.job.text == "Constructing")));
    let active_level = text_rect(&output, "Level 0 → 1");
    assert!(cell_rect(&output, "Academy").contains_rect(active_level));
    assert_eq!(
        output
            .shapes
            .iter()
            .filter(|shape| matches!(&shape.shape,
        egui::Shape::Text(text) if text.galley.job.text == "Queue"))
            .count(),
        1
    );
    let waiting_icons: Vec<_> = output
        .shapes
        .iter()
        .filter_map(|shape| match &shape.shape {
            egui::Shape::Mesh(mesh) => {
                let bounds = mesh.calc_bounds();
                ((bounds.width() - 28.0).abs() < 0.1
                    && bounds.center().y > queue.top()
                    && bounds.bottom() < city.top())
                .then_some(bounds)
            },
            _ => None,
        })
        .collect();
    assert_eq!(waiting_icons.len(), 4);
    let position = waiting_icons[2].center();
    for (time, pressed, button) in [
        (0.2, true, egui::PointerButton::Primary),
        (0.3, false, egui::PointerButton::Primary),
        (0.4, true, egui::PointerButton::Secondary),
        (0.5, false, egui::PointerButton::Secondary),
    ] {
        render(
            &ctx,
            &mut campaign,
            size,
            1.0,
            "",
            time,
            vec![
                egui::Event::PointerMoved(position),
                egui::Event::PointerButton {
                    pos: position,
                    button,
                    pressed,
                    modifiers: egui::Modifiers::NONE,
                },
            ],
        );
        if time < 0.4 {
            assert_eq!(campaign.economy.provinces[0].construction_queue.len(), 4);
            assert_eq!(campaign.economy.players[0].resources, paid);
        }
    }
    assert_eq!(campaign.economy.provinces[0].construction_queue.len(), 4);
    assert_eq!(campaign.economy.players[0].resources, paid);
    confirm_yes(&ctx, &mut campaign, size, 1.0, 0.51);
    assert_eq!(campaign.economy.provinces[0].construction_queue.len(), 3);
    assert_eq!(campaign.economy.players[0].resources[1], paid[1] + receipt.paid_metal);
    assert_eq!(campaign.economy.players[0].resources[2], paid[2] + receipt.paid_stone);
    let waiting: Vec<_> = campaign.economy.provinces[0]
        .construction_queue
        .iter()
        .map(|project| match project {
            ConstructionProject::Building(p) => p.building,
            _ => unreachable!(),
        })
        .collect();
    assert_eq!(waiting, [BuildingType::Granary, BuildingType::CityHall, BuildingType::Road]);
    assert!(
        matches!(&campaign.economy.provinces[0].construction, Some(ConstructionProject::Building(p)) if p.building == BuildingType::Academy)
    );
}

#[test]
fn construction_queue_icons_align_and_only_the_cross_cancels_active_work() {
    for scale in [1.0, 0.85] {
        let ctx = egui::Context::default();
        let mut campaign = fixture(false);
        for building in [BuildingType::Granary, BuildingType::Granary, BuildingType::Road] {
            campaign.economy.start_building(0, 0, building).unwrap();
        }
        let size = egui::vec2(570.0, 720.0) * scale;
        render(&ctx, &mut campaign, size, scale, "", 0.0, vec![]);
        let (output, _) = render(&ctx, &mut campaign, size, scale, "", 0.1, vec![]);
        let heading = text_rect(&output, "Countryside");
        let icons: Vec<_> = output
            .shapes
            .iter()
            .filter_map(|shape| match &shape.shape {
                egui::Shape::Mesh(mesh) => {
                    let rect = mesh.calc_bounds();
                    ((rect.width() - 28.0 * scale).abs() < 0.1 && rect.bottom() < heading.top())
                        .then_some(rect)
                },
                _ => None,
            })
            .collect();
        assert_eq!(icons.len(), 3);
        assert!(
            (icons[0].left() - icons[1].left()).abs() < 0.1,
            "Queue alignment at scale {scale}: {icons:?}"
        );
        assert!(icons[1].top() - icons[0].bottom() >= 6.0 * scale - 0.1);
        assert!((icons[1].top() - icons[2].top()).abs() < 0.1);
        assert!(icons[2].left() > icons[1].right());
        text_rect(&output, "Queue");

        let track_position = text_rect(&output, "0%").center();
        let mut time = 0.2;
        for position in [icons[0].center(), track_position] {
            for button in [egui::PointerButton::Primary, egui::PointerButton::Secondary] {
                for pressed in [true, false] {
                    let (output, _) = render(
                        &ctx,
                        &mut campaign,
                        size,
                        scale,
                        "",
                        time,
                        vec![
                            egui::Event::PointerMoved(position),
                            egui::Event::PointerButton {
                                pos: position,
                                button,
                                pressed,
                                modifiers: egui::Modifiers::NONE,
                            },
                        ],
                    );
                    assert_eq!(output.platform_output.cursor_icon, egui::CursorIcon::Default);
                    assert!(campaign.economy.provinces[0].construction.is_some());
                    assert_eq!(campaign.economy.provinces[0].construction_queue.len(), 2);
                    time += 0.1;
                }
            }
        }
        render(
            &ctx,
            &mut campaign,
            size,
            scale,
            "",
            time,
            vec![egui::Event::PointerMoved(icons[0].center())],
        );
        render(&ctx, &mut campaign, size, scale, "", time + 1.0, vec![]);
        let (hover, _) = render(&ctx, &mut campaign, size, scale, "", time + 1.1, vec![]);
        text_rect(&hover, "Granary level 1, 3 months left");
        assert!(!hover.shapes.iter().any(|shape| matches!(&shape.shape,
            egui::Shape::Text(text) if text.galley.job.text.contains("Right-click to cancel")
                || text.galley.job.text.contains("Active work is not refunded"))));
    }
}

#[test]
fn cancelling_construction_starts_the_next_preview_at_zero_during_a_month() {
    let ctx = egui::Context::default();
    let mut campaign = fixture(false);
    for _ in 0..3 {
        campaign.economy.start_building(0, 0, BuildingType::Granary).unwrap();
    }
    if let Some(ConstructionProject::Building(project)) =
        &mut campaign.economy.provinces[0].construction
    {
        project.progress = 1.0;
    }
    let size = egui::vec2(570.0, 720.0);
    ctx.data_mut(|data| {
        data.insert_temp(egui::Id::new("campaign-construction-month-fraction"), 0.75_f32)
    });
    render(&ctx, &mut campaign, size, 1.0, "", 0.0, vec![]);
    let (output, _) = render(&ctx, &mut campaign, size, 1.0, "", 0.1, vec![]);
    assert!(progress_fill(&output).width() > 0.0);
    let position = text_rect(&output, "\u{00d7}").center();
    let paid = campaign.economy.players[0].resources;
    for (time, pressed) in [(0.2, true), (0.3, false)] {
        render(
            &ctx,
            &mut campaign,
            size,
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
    assert_eq!(campaign.economy.provinces[0].construction.as_ref().unwrap().progress().0, 1.0);
    confirm_yes(&ctx, &mut campaign, size, 1.0, 0.31);
    assert_eq!(campaign.economy.provinces[0].construction.as_ref().unwrap().progress().0, 0.0);
    assert_eq!(campaign.economy.provinces[0].construction_queue.len(), 1);
    assert_eq!(campaign.economy.players[0].resources, paid);
    let (next, _) = render(&ctx, &mut campaign, size, 1.0, "", 0.4, vec![]);
    text_rect(&next, "0%");
    assert_eq!(progress_fill(&next).width(), 0.0);
    ctx.data_mut(|data| {
        data.insert_temp(egui::Id::new("campaign-construction-month-fraction"), 0.9_f32)
    });
    let (later, _) = render(&ctx, &mut campaign, size, 1.0, "", 0.5, vec![]);
    assert!(progress_fill(&later).width() > 0.0);
    campaign.economy.advance_month(&Default::default());
    ctx.data_mut(|data| {
        data.insert_temp(egui::Id::new("campaign-construction-month-fraction"), 0.0_f32)
    });
    let (next_month, _) = render(&ctx, &mut campaign, size, 1.0, "", 0.6, vec![]);
    assert!(progress_fill(&next_month).width() > progress_fill(&later).width());
}
