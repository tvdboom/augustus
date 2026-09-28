//! Headless layout checks run real campaign panel bodies at desktop/compact widths.

use super::super::campaign_notifications::NoticeSeverity;
use super::*;
use crate::game::economy::{EconomicProvince, EconomyWorld, Terrain};
use crate::game::politics::espionage::{Scandal, ScandalKind, ScandalTarget, Severity};
use crate::game::politics::senate::PoliticalProfile;
use crate::game::politics::PoliticalRank;

/// Deterministic representative states cover ownership, independence, vassals and troops.
fn fixture() -> Campaign {
    let names = ["Italia", "Africa Proconsularis", "Achaia", "Cappadocia", "Sardinia et Corsica"];
    let mut provinces: Vec<_> = names
        .iter()
        .map(|name| {
            EconomicProvince::new(
                *name,
                60.0,
                Terrain::Farmland,
                true,
                [1.4, 0.8, 0.6],
                [8.0, 22.0, 33.0, 37.0],
                2,
            )
        })
        .collect();
    provinces[0].owner = Some(0);
    provinces[0].wonder_sites = vec![0];
    provinces[1].owner = Some(1);
    provinces[3].overlord = Some(0);
    provinces[4].overlord = Some(1);
    let graph = vec![vec![1, 2], vec![0, 3], vec![0, 3, 4], vec![1, 2, 4], vec![2, 3]];
    let mut campaign = Campaign::default();
    campaign.active = true;
    campaign.economy = EconomyWorld::new(2, provinces, graph.clone());
    for wallet in &mut campaign.economy.players {
        wallet.coin = 5000.0;
        wallet.influence = 1000.0;
        wallet.resources = [2000.0; 3];
    }
    campaign.actors = vec![
        PoliticalPlayer {
            coin: 5000.0,
            influence: 1000.0,
            rank: PoliticalRank::Aedile,
            consul_until: None,
            ..Default::default()
        };
        2
    ];
    campaign.politics = vec![
        ProvincePolitics::owned(2, 0),
        ProvincePolitics::owned(2, 1),
        ProvincePolitics::independent(2),
        ProvincePolitics::independent(2),
        ProvincePolitics::independent(2),
    ];
    campaign.politics[2].state = PoliticalState::Independent {
        local: 15.0,
        shares: vec![55.0, 30.0],
    };
    campaign.politics[3].state = PoliticalState::Vassal {
        overlord: 0,
        control: 72.0,
        tribute: Tribute::High,
    };
    campaign.politics[4].state = PoliticalState::Vassal {
        overlord: 1,
        control: 40.0,
        tribute: Tribute::Normal,
    };
    campaign.military = MilitaryWorld::new(5);
    campaign.graph = graph
        .into_iter()
        .map(|neighbors| MilitaryProvince {
            terrain: MilitaryTerrain::Plains,
            area: 100.0,
            road_level: 1,
            neighbors,
        })
        .collect();
    for unit_type in UnitType::ALL {
        campaign.military.seed_unit(0, ForceOwner::Player(0), unit_type).unwrap();
    }
    campaign.military.seed_unit(1, ForceOwner::Player(1), UnitType::HeavyInfantry).unwrap();
    campaign.wars = vec![vec![false; 2]; 2];
    campaign.npc_wars = vec![vec![false; 5]; 2];
    campaign.invitations = vec![vec![true; 2]; 2];
    campaign.profiles = vec![PoliticalProfile::default(); 2];
    campaign.recent_victories = vec![0.0; 2];
    campaign.actors[1].rank = PoliticalRank::Consul;
    campaign.actors[1].consul_until = Some(48);
    campaign.espionage.scandals = vec![
        Scandal {
            id: 1,
            holder: 0,
            target: ScandalTarget::Province(2),
            kind: ScandalKind::MilitaryIncompetence,
            severity: Severity::Major,
            province: Some(2),
            source_id: 10,
            acquired: 0,
            expires: 24,
            reserved_for_motion: false,
        },
        Scandal {
            id: 2,
            holder: 0,
            target: ScandalTarget::Player(1),
            kind: ScandalKind::FriendlyAttack,
            severity: Severity::Medium,
            province: Some(1),
            source_id: 11,
            acquired: 0,
            expires: 24,
            reserved_for_motion: false,
        },
    ];
    campaign.notifications.province_notice(0, 1, 0, NoticeSeverity::Warning, super::super::campaign_notifications::NoticeKind::HostileRelation, "Relations with Africa Proconsularis have become hostile", "A foreign political campaign reduced our relationship. Inspect the provincial diplomacy panel for its full breakdown.");
    campaign.economy.refresh_npc_markets(&campaign.inputs());
    campaign
}

/// Exercise egui's actual layout, including expanded sections, without a desktop window.
fn layout_context() -> egui::Context {
    let ctx = egui::Context::default();
    ctx.set_global_style(augustus_ui_style());
    ctx.add_font(FontInsert::new(
        "firasans",
        egui::FontData::from_static(include_bytes!("../../assets/fonts/FiraSans-Bold.ttf")),
        vec![InsertFontFamily {
            family: egui::FontFamily::Proportional,
            priority: FontPriority::Highest,
        }],
    ));
    ctx.memory_mut(|memory| memory.set_everything_is_visible(true));
    ctx
}

fn hover_card_fixture(own_all: bool) -> (Campaign, ProvinceOwnership) {
    let mut ownership = ProvinceOwnership::default();
    ownership.start_game(&[PLAYER_COLORS[0]]);
    if own_all {
        for (id, seed) in ownership.campaign_seeds().into_iter().enumerate() {
            ownership.sync_campaign_province(id, Some(0), seed.population, seed.potential, 1.0);
        }
    }
    let mut campaign = Campaign::default();
    campaign.start(&ownership, 1);
    (campaign, ownership)
}

fn hover_card_textures(
    ctx: &egui::Context,
) -> ([egui::TextureHandle; 7], [egui::TextureHandle; 4]) {
    let load = |name| {
        ctx.load_texture(
            name,
            egui::ColorImage::from_rgba_unmultiplied([1, 1], &[255, 255, 255, 255]),
            egui::TextureOptions::LINEAR,
        )
    };
    (
        std::array::from_fn(|index| load(format!("hover-resource-{index}"))),
        std::array::from_fn(|index| load(format!("hover-class-{index}"))),
    )
}

#[allow(clippy::too_many_arguments)]
fn render_hover_cards(
    ctx: &egui::Context,
    campaign: &Campaign,
    ownership: &ProvinceOwnership,
    icons: &[egui::TextureHandle; 7],
    class_icons: &[egui::TextureHandle; 4],
    hover: &mut super::super::resource_hud::HudHoverState,
    screen: egui::Rect,
    scale: f32,
    time: f64,
    pointer: egui::Pos2,
) -> egui::FullOutput {
    let mut output = ctx.run_ui(
        egui::RawInput {
            screen_rect: Some(screen),
            time: Some(time),
            events: vec![egui::Event::PointerMoved(pointer)],
            ..Default::default()
        },
        |_| {
            super::super::map_menu::draw_map_menu_hitboxes(ctx, scale);
            super::super::resource_hud::show_hud_hover_cards(
                ctx,
                screen,
                scale,
                super::super::map_menu::map_resource_strip_right(screen, scale)
                    - MAP_DATE_SECTION_WIDTH,
                0,
                ownership,
                Some(campaign),
                HudResource {
                    amount: ownership.total_population_for(0),
                    monthly_delta: 0.0,
                },
                icons,
                class_icons,
                hover,
            );
        },
    );
    output.textures_delta.clear();
    output
}

fn hover_card_rect(output: &egui::FullOutput) -> Option<egui::Rect> {
    output.shapes.iter().find_map(|shape| match &shape.shape {
        egui::Shape::Rect(rect) if rect.fill == egui::Color32::from_rgb(236, 231, 216) => {
            Some(rect.rect)
        },
        _ => None,
    })
}

#[test]
fn resource_hover_cards_open_over_the_drag_guard_and_stay_open_on_the_card() {
    let (campaign, ownership) = hover_card_fixture(false);
    let widths = [350.0, 265.0, 265.0, 430.0, 430.0, 350.0];
    for scale in [1.0, 0.85] {
        let ctx = layout_context();
        ctx.memory_mut(|memory| memory.set_everything_is_visible(false));
        let (icons, class_icons) = hover_card_textures(&ctx);
        let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1600.0, 900.0));
        let mut hover = super::super::resource_hud::HudHoverState::default();
        let mut frame = 0;
        let mut render = |position| {
            frame += 1;
            let output = render_hover_cards(
                &ctx,
                &campaign,
                &ownership,
                &icons,
                &class_icons,
                &mut hover,
                screen,
                scale,
                frame as f64 * 0.1,
                position,
            );
            let active = hover.resource.or_else(|| {
                if hover.coin {
                    Some(3)
                } else if hover.influence {
                    Some(4)
                } else if hover.population {
                    Some(5)
                } else {
                    None
                }
            });
            (active, hover_card_rect(&output))
        };
        for (index, x) in hud_resource_positions().into_iter().enumerate() {
            let icon = egui::pos2(x + 35.0, 22.0) * scale;
            render(icon);
            let (active, rect) = render(icon);
            assert_eq!(active, Some(index));
            let rect = rect.expect("Hover must paint the original illustrated card");
            assert!((rect.width() - widths[index] * scale).abs() <= 1.0);
            assert!((rect.top() - 45.0 * scale).abs() <= 1.0);
            assert!(screen.contains_rect(rect));
            assert_eq!(
                render(rect.center()).0,
                Some(index),
                "Card must remain usable for scrolling"
            );
            assert_eq!(render(egui::pos2(1550.0, 850.0)).0, None);
        }
    }
}

#[test]
fn original_governance_card_retains_its_banner_buttons_badges_and_close() {
    let ctx = layout_context();
    ctx.memory_mut(|memory| memory.set_everything_is_visible(false));
    let (icons, _) = hover_card_textures(&ctx);
    let mut governance = Governance::default();
    let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1600.0, 900.0));
    let mut time = 0.0;
    let mut render = |events| {
        time += 0.1;
        let mut changed = false;
        let mut closed = false;
        let mut output = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(screen),
                time: Some(time),
                events,
                ..Default::default()
            },
            |_| {
                (changed, closed) = super::super::governance_panel::show(
                    &ctx,
                    1.0,
                    &icons[0],
                    &icons,
                    PLAYER_COLORS[0],
                    &mut governance,
                    false,
                );
            },
        );
        output.textures_delta.clear();
        (output, changed, closed, governance)
    };
    render(vec![]);
    let (output, _, _, _) = render(vec![]);
    let panel = output
        .shapes
        .iter()
        .find_map(|shape| match &shape.shape {
            egui::Shape::Rect(rect) if rect.fill == egui::Color32::from_rgb(238, 233, 219) => {
                Some(rect.rect)
            },
            _ => None,
        })
        .expect("The original parchment governance card must be painted");
    assert_eq!(panel, map_corner_panel_rect(screen, 1.0, egui::vec2(500.0, 570.0)));
    for label in [
        "Governance",
        "LABOR & AGRARIAN",
        "ECONOMY",
        "Food Rations",
        "Slave Labor",
        "Noble Taxes",
        "Army Wages",
    ] {
        assert!(output.shapes.iter().any(|shape| matches!(&shape.shape, egui::Shape::Text(text) if text.galley.job.text == label)), "Missing original governance label {label}");
    }
    let high = output
        .shapes
        .iter()
        .filter_map(|shape| match &shape.shape {
            egui::Shape::Text(text) if text.galley.job.text == "High" => {
                Some(egui::Rect::from_min_size(text.pos, text.galley.size()).center())
            },
            _ => None,
        })
        .min_by(|a, b| a.y.total_cmp(&b.y))
        .expect("Food rations must retain their High button");
    for pressed in [true, false] {
        let (_, changed, _, selected) = render(vec![
            egui::Event::PointerMoved(high),
            egui::Event::PointerButton {
                pos: high,
                button: egui::PointerButton::Primary,
                pressed,
                modifiers: egui::Modifiers::NONE,
            },
        ]);
        if !pressed {
            assert!(changed);
            assert_eq!(selected.food_rations, crate::map::EdictLevel::High);
        }
    }
    let close = panel.right_top() + egui::vec2(-25.0, 25.0);
    for pressed in [true, false] {
        let (_, _, closed, _) = render(vec![
            egui::Event::PointerMoved(close),
            egui::Event::PointerButton {
                pos: close,
                button: egui::PointerButton::Primary,
                pressed,
                modifiers: egui::Modifiers::NONE,
            },
        ]);
        if !pressed {
            assert!(closed, "Original governance close button must remain clickable");
        }
    }
}

fn run_province_header(
    ctx: &egui::Context,
    campaign: &Campaign,
    view: &mut CampaignUi,
    time: f64,
    events: Vec<egui::Event>,
) -> egui::FullOutput {
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
        |_| {
            egui::Area::new(egui::Id::new("header-interaction-test"))
                .fixed_pos(egui::pos2(30.0, 30.0))
                .movable(false)
                .show(ctx, |ui| {
                    panel_style(ui, 1.0);
                    panel_frame(1.0).show(ui, |ui| {
                        ui.set_width(570.0);
                        panel_header(ui, "Italia", 1.0, &campaign.economy.provinces, 0, view);
                    });
                });
        },
    );
    output.textures_delta.clear();
    output
}

#[test]
fn province_banner_is_flush_and_its_title_search_selects_by_typing() {
    let ctx = layout_context();
    ctx.memory_mut(|memory| memory.set_everything_is_visible(false));
    let campaign = fixture();
    let mut view = CampaignUi {
        open: Some(CampaignTab::Province),
        province: Some(0),
        ..Default::default()
    };
    run_province_header(&ctx, &campaign, &mut view, 0.0, vec![]);
    let output = run_province_header(&ctx, &campaign, &mut view, 0.1, vec![]);
    assert!(!output.shapes.iter().any(
        |shape| matches!(&shape.shape, egui::Shape::Text(text) if text.galley.job.text == "Trade")
    ));
    assert!(output.shapes.iter().any(|shape| matches!(&shape.shape, egui::Shape::Text(text) if text.galley.job.text == "Notifications")));
    let banner = output
        .shapes
        .iter()
        .find_map(|shape| match &shape.shape {
            egui::Shape::Rect(rect) if rect.fill == PLAYER_COLORS[0] => Some(rect.rect),
            _ => None,
        })
        .expect("Owner banner must be painted");
    assert_eq!(banner.left_top(), egui::pos2(30.0, 30.0));
    assert!((banner.width() - 570.0).abs() <= 1.0);
    let title = output
        .shapes
        .iter()
        .find_map(|shape| match &shape.shape {
            egui::Shape::Text(text)
                if text.galley.job.text == "Italia" && banner.contains(text.pos) =>
            {
                Some(text.clone())
            },
            _ => None,
        })
        .unwrap();
    let position = title.pos + title.galley.size() * 0.5;
    for (time, pressed) in [(0.2, true), (0.3, false)] {
        run_province_header(
            &ctx,
            &campaign,
            &mut view,
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
    assert!(view.province_selector_open, "Clicking the province name must open the dropdown");
    let editing = run_province_header(&ctx, &campaign, &mut view, 0.4, vec![]);
    let placeholder = editing
        .shapes
        .iter()
        .find_map(|shape| match &shape.shape {
            egui::Shape::Text(text)
                if text.galley.job.text == "Italia" && banner.contains(text.pos) =>
            {
                Some(text)
            },
            _ => None,
        })
        .expect("The current province remains the title placeholder");
    assert!(
        title.pos.distance(placeholder.pos) < 1.0,
        "Editing must preserve the title position: {:?} -> {:?}",
        title.pos,
        placeholder.pos
    );
    assert!((title.galley.size() - placeholder.galley.size()).length() < 1.0);
    assert_eq!(
        title.galley.job.sections[0].format.font_id,
        placeholder.galley.job.sections[0].format.font_id
    );
    assert_eq!(
        title.galley.job.sections[0].format.color.gamma_multiply(0.5),
        placeholder.galley.job.sections[0].format.color,
        "The current name must recede while the search field is editable"
    );
    assert!(editing.shapes.iter().any(|shape| matches!(&shape.shape, egui::Shape::Text(text) if text.galley.job.text == "Cappadocia" && text.pos.y >= banner.bottom())), "Other provinces appear below the title");
    run_province_header(&ctx, &campaign, &mut view, 0.5, vec![egui::Event::Text("cAPpa".into())]);
    assert_eq!(view.province_search, "cAPpa");
    run_province_header(
        &ctx,
        &campaign,
        &mut view,
        0.6,
        vec![egui::Event::Key {
            key: egui::Key::Enter,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: egui::Modifiers::NONE,
        }],
    );
    assert_eq!(view.province, Some(3));
    assert!(!view.province_selector_open);
    assert!(!ctx.egui_wants_keyboard_input());
}

#[test]
fn province_search_list_aligns_with_the_field_and_accepts_clicks_past_the_name() {
    let ctx = layout_context();
    ctx.memory_mut(|memory| memory.set_everything_is_visible(false));
    let campaign = fixture();
    let mut view = CampaignUi {
        open: Some(CampaignTab::Province),
        province: Some(0),
        province_selector_open: true,
        province_search: "Achaia".into(),
        ..Default::default()
    };
    run_province_header(&ctx, &campaign, &mut view, 0.0, vec![]);
    let output = run_province_header(&ctx, &campaign, &mut view, 0.1, vec![]);
    let field = output
        .shapes
        .iter()
        .find_map(|shape| match &shape.shape {
            egui::Shape::Rect(rect) if rect.fill == egui::Color32::from_black_alpha(18) => {
                Some(rect.rect)
            },
            _ => None,
        })
        .expect("The editable title must have its own background");
    // Popup frames group their shadow and parchment rectangle in a shape vector.
    fn paper_below(shape: &egui::Shape, bottom: f32) -> Option<egui::Rect> {
        match shape {
            egui::Shape::Rect(rect)
                if rect.fill == province_panel::PAPER && rect.rect.top() >= bottom =>
            {
                Some(rect.rect)
            },
            egui::Shape::Vec(shapes) => shapes.iter().find_map(|shape| paper_below(shape, bottom)),
            _ => None,
        }
    }
    let dropdown = output
        .shapes
        .iter()
        .find_map(|shape| paper_below(&shape.shape, field.bottom()))
        .expect("The province list must appear below the search field");
    assert!((dropdown.left() - field.left()).abs() < 1.0);
    assert!((dropdown.top() - field.bottom()).abs() < 1.0);
    assert!((dropdown.width() - field.width()).abs() < 1.0);
    let row = output
        .shapes
        .iter()
        .find_map(|shape| match &shape.shape {
            egui::Shape::Text(text)
                if text.galley.job.text == "Achaia" && dropdown.contains(text.pos) =>
            {
                Some(egui::Rect::from_min_size(text.pos, text.galley.size()))
            },
            _ => None,
        })
        .expect("The filtered province must be the dropdown's only result");
    assert!(!output.shapes.iter().any(|shape| matches!(
        &shape.shape, egui::Shape::Text(text) if text.galley.job.text == "Type in the province title to search"
    )));
    let position = egui::pos2(dropdown.right() - 12.0, row.center().y);
    assert!(position.x > row.right() + 100.0, "Exercise the empty part of the row");
    for (time, pressed) in [(0.2, true), (0.3, false)] {
        run_province_header(
            &ctx,
            &campaign,
            &mut view,
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
    assert_eq!(view.province, Some(2));
    assert!(!view.province_selector_open);
    assert!(view.province_search.is_empty());
}

#[test]
fn province_close_button_changes_color_on_hover_and_press() {
    let mut feedback = Vec::new();
    for color in PLAYER_COLORS.into_iter().chain([province_panel::NEUTRAL]) {
        let ctx = layout_context();
        ctx.data_mut(|data| data.insert_temp(egui::Id::new("campaign-owner-color"), color));
        ctx.memory_mut(|memory| memory.set_everything_is_visible(false));
        let campaign = fixture();
        let mut view = CampaignUi {
            open: Some(CampaignTab::Province),
            province: Some(0),
            ..Default::default()
        };
        run_province_header(&ctx, &campaign, &mut view, 0.0, vec![]);
        let normal = run_province_header(&ctx, &campaign, &mut view, 0.1, vec![]);
        let close_circle = |output: &egui::FullOutput| {
            output
                .shapes
                .iter()
                .find_map(|shape| match &shape.shape {
                    egui::Shape::Circle(circle)
                        if (circle.radius - 12.0).abs() < 0.1
                            && circle.center.x > 550.0
                            && circle.center.y < 75.0 =>
                    {
                        Some(circle.clone())
                    },
                    _ => None,
                })
                .expect("Close button circle")
        };
        let normal = close_circle(&normal);
        let hover = run_province_header(
            &ctx,
            &campaign,
            &mut view,
            0.2,
            vec![egui::Event::PointerMoved(normal.center)],
        );
        let hover = close_circle(&hover);
        let pressed = run_province_header(
            &ctx,
            &campaign,
            &mut view,
            0.3,
            vec![egui::Event::PointerButton {
                pos: normal.center,
                button: egui::PointerButton::Primary,
                pressed: true,
                modifiers: egui::Modifiers::NONE,
            }],
        );
        let pressed = close_circle(&pressed);
        assert_ne!(normal.fill, hover.fill);
        assert_ne!(hover.fill, pressed.fill);
        assert_eq!(normal.stroke.color, hover.stroke.color, "Hover must retain contrasting ink");
        assert_eq!(hover.stroke.color, pressed.stroke.color, "Press must retain contrasting ink");
        feedback.push((hover.fill, pressed.fill));
    }
    assert!(
        feedback.windows(2).all(|colors| colors[0] != colors[1]),
        "Close feedback must change with the owner banner"
    );
}

#[test]
fn province_tabs_offer_policies_only_to_the_direct_owner_and_trade_to_everyone_else() {
    let campaign = fixture();
    for province in 0..5 {
        let ctx = layout_context();
        ctx.memory_mut(|memory| memory.set_everything_is_visible(false));
        let mut view = CampaignUi {
            open: Some(CampaignTab::Province),
            province: Some(province),
            section: 1,
            ..Default::default()
        };
        run_province_header(&ctx, &campaign, &mut view, 0.0, vec![]);
        let output = run_province_header(&ctx, &campaign, &mut view, 0.1, vec![]);
        let labels: Vec<_> = output
            .shapes
            .iter()
            .filter_map(|shape| match &shape.shape {
                egui::Shape::Text(text) => Some(text.galley.job.text.as_str()),
                _ => None,
            })
            .collect();
        let owned = campaign.economy.provinces[province].owner == Some(0);
        assert_eq!(labels.contains(&"Policies"), owned);
        assert_eq!(labels.contains(&"Trade"), !owned);
    }
}

#[test]
fn top_bar_draws_civic_power_before_resources_with_whole_population_counts() {
    let ctx = layout_context();
    let (icons, _) = hover_card_textures(&ctx);
    let mut resources = [HudResource {
        amount: 0.0,
        monthly_delta: 0.0,
    }; 7];
    for (index, value) in [110.0, 220.0, 330.0, 440.0, 550.0, 1660.9].into_iter().enumerate() {
        resources[index].amount = value;
    }
    let mut output = ctx.run_ui(
        egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1600.0, 900.0),
            )),
            ..Default::default()
        },
        |root| {
            super::super::resource_hud::paint_hud_resources(
                root.painter(),
                egui::Pos2::ZERO,
                1.0,
                1200.0,
                &resources,
                &icons,
            );
        },
    );
    output.textures_delta.clear();
    let left = |label: &str| {
        output
            .shapes
            .iter()
            .find_map(|shape| match &shape.shape {
                egui::Shape::Text(text) if text.galley.job.text == label => Some(text.pos.x),
                _ => None,
            })
            .unwrap_or_else(|| panic!("Missing top-bar value {label}"))
    };
    let positions = ["550", "440", "110", "220", "330", "1660"].map(left);
    assert!(positions.windows(2).all(|pair| pair[0] < pair[1]));
}

#[test]
fn overview_groups_and_badges_keep_floor_counts_alignment_and_warning_colors() {
    for warning in [false, true] {
        let ctx = layout_context();
        ctx.memory_mut(|memory| memory.set_everything_is_visible(false));
        let mut campaign = fixture();
        campaign.advance_month();
        campaign.economy.provinces[0].population = [11.9, 27.9, 53.9, 31.9];
        campaign.economy.provinces[0].capacity_area = if warning {
            0.0
        } else {
            1000.0
        };
        campaign.economy.last_report.province_reports[0].food_supply_ratio = if warning {
            0.996
        } else {
            1.0
        };
        let capacity = campaign.economy.provinces[0].capacity(&campaign.economy.config);
        let mut output = ctx.run_ui(egui::RawInput::default(), |root| {
            egui::CentralPanel::default().show(root, |ui| {
                ui.set_width(546.0);
                panel_style(ui, 1.0);
                campaign_economy::overview(ui, &campaign.economy, 0, 1.0);
            });
        });
        output.textures_delta.clear();
        let texts: Vec<_> = output
            .shapes
            .iter()
            .filter_map(|shape| match &shape.shape {
                egui::Shape::Text(text) => Some(text),
                _ => None,
            })
            .collect();
        let text = |label: &str| {
            *texts
                .iter()
                .find(|text| text.galley.job.text == label)
                .unwrap_or_else(|| panic!("Missing {label} in overview"))
        };
        let total = text(&format!("125 / {:.0}", capacity.floor()));
        assert!(text("Population").pos.y < total.pos.y);
        let supplied = text(if warning {
            "99%"
        } else {
            "100%"
        });
        assert!(text("Food supplied").pos.y < supplied.pos.y);
        for value in [total, supplied] {
            assert_eq!(
                value.galley.job.sections[0].format.color,
                if warning {
                    super::super::resource_hud::hud_delta_color(-1.0)
                } else {
                    province_panel::INK
                }
            );
        }
        assert!(text("Civic Power").pos.y < text("Influence").pos.y);
        assert!(text("Influence").pos.y < text("Coin").pos.y);
        assert!(text("Coin").pos.y < text("Resources").pos.y);
        assert!(text("Resources").pos.y < text("Food").pos.y);
        for (name, count) in
            [("Nobles", "11"), ("Citizens", "27"), ("Plebeians", "53"), ("Slaves", "31")]
        {
            assert!(texts.iter().any(|value| value.galley.job.text == count
                && (value.pos.y - text(name).pos.y).abs() < 1.0));
            assert!(
                (text(name).pos.x - text("Food").pos.x).abs() < 1.0,
                "Names must share a left edge after their icons"
            );
        }
    }
}

#[test]
fn all_hud_ledgers_fit_with_a_full_atlas_of_owned_provinces() {
    let (campaign, ownership) = hover_card_fixture(true);
    for scale in [1.0, 0.85] {
        let ctx = layout_context();
        ctx.memory_mut(|memory| memory.set_everything_is_visible(false));
        let (icons, class_icons) = hover_card_textures(&ctx);
        let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1200.0, 800.0));
        let mut hover = super::super::resource_hud::HudHoverState::default();
        for (index, x) in hud_resource_positions().into_iter().enumerate() {
            let date_left = super::super::map_menu::map_resource_strip_right(screen, scale)
                - MAP_DATE_SECTION_WIDTH;
            if x + HUD_RESOURCE_WIDTH > date_left - HUD_RESOURCE_GROUP_PADDING {
                continue;
            }
            let pointer = egui::pos2(x + 35.0, 22.0) * scale;
            for frame in 0..2 {
                let output = render_hover_cards(
                    &ctx,
                    &campaign,
                    &ownership,
                    &icons,
                    &class_icons,
                    &mut hover,
                    screen,
                    scale,
                    (index * 2 + frame) as f64 * 0.1,
                    pointer,
                );
                if frame > 0 {
                    let rect = hover_card_rect(&output).expect("Original HUD card must be painted");
                    assert!(
                        screen.contains_rect(rect),
                        "HUD card {index} exceeds the viewport: {rect:?}"
                    );
                }
            }
        }
    }
}

/// Measure actual panel bodies without substituting implementation-shaped mock widgets.
fn assert_body_fits(label: &str, width: f32, scale: f32, mut render: impl FnMut(&mut egui::Ui)) {
    let ctx = layout_context();
    let mut used_width: f32 = 0.0;
    let mut used_height: f32 = 0.0;
    // A second frame resolves font/layout cache and CollapsingHeader animation state.
    for frame in 0..2 {
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1600.0, 12_000.0),
            )),
            time: Some(frame as f64),
            ..Default::default()
        };
        let mut output = ctx.run_ui(input, |root| {
            root.scope_builder(
                egui::UiBuilder::new()
                    .max_rect(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(width, 12_000.0),
                    ))
                    .layout(egui::Layout::top_down(egui::Align::Min)),
                |ui| {
                    ui.set_max_width(width);
                    panel_style(ui, scale);
                    let start = ui.next_widget_position().x;
                    render(ui);
                    used_width = used_width.max(ui.min_rect().right() - start);
                    used_height = used_height.max(ui.min_rect().height());
                },
            );
        });
        // Headless layout intentionally has no GPU; acknowledge atlas/icon deltas.
        output.textures_delta.clear();
        assert!(!output.shapes.is_empty(), "{label}: empty output would not verify a real panel");
    }
    assert!(used_height > 25.0, "{label}: the actual content was not laid out");
    assert!(
        used_width <= width + 1.5,
        "{label} overflow: used {used_width:.1}px inside {width:.1}px at scale {scale:.2}"
    );
}

#[test]
fn complete_panel_shell_bounds_long_headers_and_status_with_scrolled_content() {
    for (width, height, scale) in [(570.0, 720.0, 1.0), (404.0, 540.0, 0.85), (404.0, 400.0, 1.0)] {
        for tab in [CampaignTab::Province, CampaignTab::Senate, CampaignTab::Trade] {
            for status in [String::new(), "Political action completed. The affected province's relationship and control have changed; review the detailed modifiers and the next monthly resolution before committing more resources. ".repeat(20)] {
                let ctx = layout_context();
                let mut campaign = fixture();
                let mut view = CampaignUi { open: Some(tab), province: Some(4), ..Default::default() };
                let rect = egui::Rect::from_min_size(egui::pos2(30.0, 30.0), egui::vec2(width, height));
                for frame in 0..2 {
                    let input = egui::RawInput { screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1200.0, 900.0))), time: Some(frame as f64), ..Default::default() };
                    let mut outer = rect;
                    let mut output = ctx.run_ui(input, |root| {
                        root.scope_builder(egui::UiBuilder::new().max_rect(rect), |ui| {
                            panel_style(ui, scale);
                            outer = panel_frame(scale).show(ui, |ui| {
                                ui.set_width(width);
                                ui.set_min_height(height);
                                panel_header(ui, "Sardinia et Corsica · provincial administration", scale, &campaign.economy.provinces, 0, &mut view);
                                scroll_body_with_status(ui, rect.bottom() - 12.0 * scale - 1.2, scale, &status, "shell_test_body", |ui| {
                                    if tab == CampaignTab::Province {
                                        diplomacy(ui, &mut campaign, 4, 0);
                                    } else if tab == CampaignTab::Trade {
                                        campaign_trade::show(ui, &mut campaign, 0, scale);
                                    } else {
                                        campaign_politics::show(ui, &mut campaign.senate, &mut campaign.actors, &campaign.profiles, &campaign.senate_config, 0, &mut campaign.espionage, &campaign.espionage_config, &[PLAYER_COLORS[0], PLAYER_COLORS[1]]);
                                    }
                                });
                            }).response.rect;
                        });
                    });
                    output.textures_delta.clear();
                    assert!(outer.right() <= rect.right() + 1.0, "{tab:?}: header/frame horizontal overflow at {width}px: {outer:?}");
                    assert!(outer.bottom() <= rect.bottom() + 1.0, "{tab:?}: frame/footer vertical overflow at {width}×{height}px: {outer:?}");
                }
            }
        }
    }
}

#[test]
fn diplomacy_legal_states_fit_desktop_and_compact_panels() {
    for (width, scale) in [(500.0, 1.0), (380.0, 0.85), (380.0, 1.0)] {
        for province in 0..5 {
            let mut campaign = fixture();
            assert_body_fits(&format!("Diplomacy province {province}"), width, scale, |ui| {
                diplomacy(ui, &mut campaign, province, 0);
            });
        }
    }
}

#[test]
fn senate_neutral_loyalty_and_consul_states_fit_desktop_and_compact_panels() {
    for (width, scale) in [(500.0, 1.0), (380.0, 0.85), (380.0, 1.0)] {
        for phase in 0..3 {
            let mut campaign = fixture();
            if phase > 0 {
                for (id, senator) in campaign.senate.senators.iter_mut().enumerate() {
                    senator.allegiance = if id % 3 == 0 {
                        None
                    } else {
                        Some(id % 2)
                    };
                }
            }
            if phase == 2 {
                campaign.actors[0].rank = PoliticalRank::Consul;
                campaign.actors[0].consul_until = Some(24);
            }
            assert_body_fits(&format!("Senate phase {phase}"), width, scale, |ui| {
                campaign_politics::show(
                    ui,
                    &mut campaign.senate,
                    &mut campaign.actors,
                    &campaign.profiles,
                    &campaign.senate_config,
                    0,
                    &mut campaign.espionage,
                    &campaign.espionage_config,
                    &[PLAYER_COLORS[0], PLAYER_COLORS[1]],
                );
            });
        }
    }
}

#[test]
fn rome_city_and_province_selection_open_senate_and_other_selections_stay_contextual() {
    let mut c = fixture();
    c.economy.provinces[2].name = "Latium".into();
    c.politics[2] = ProvincePolitics::rome(2);
    let mut view = CampaignUi::default();
    for selected in [MapDetail::Province(2), MapDetail::City(2)] {
        view.select_map_detail(selected, &c);
        assert_eq!(view.open, Some(CampaignTab::Senate));
        assert_eq!(view.province, Some(2));
        assert_eq!(view.last_detail, Some(selected));
    }
    view.select_map_detail(MapDetail::Province(0), &c);
    assert_eq!(view.open, Some(CampaignTab::Province));
    assert_eq!(view.section, 0);
    view.select_map_detail(MapDetail::City(0), &c);
    assert_eq!(view.section, 2);
}

#[test]
fn economy_overview_policies_buildings_and_trade_fit_compact_panels() {
    for (width, scale) in [(500.0, 1.0), (380.0, 0.85), (380.0, 1.0)] {
        for section in 0..4 {
            let mut campaign = fixture();
            assert_body_fits(
                &format!("Economy section {section}"),
                width,
                scale,
                |ui| match section {
                    0 => province_overview(ui, &campaign, 0, 0, scale),
                    1 => campaign_economy::policies(ui, &mut campaign.economy, 0, 0),
                    2 => {
                        campaign_economy::buildings(ui, &mut campaign.economy, 0, 0, scale);
                    },
                    _ => {
                        campaign_trade::show(ui, &mut campaign, 0, scale);
                    },
                },
            );
        }
    }
}

#[test]
fn national_trade_routes_composer_and_market_fit_compact_widths() {
    use crate::game::economy::{TradeAgreement, TradeBundle, TradeFrequency, TradeParty};
    for (width, scale) in [(546.0, 1.0), (380.0, 0.85), (380.0, 1.0)] {
        for page in 0..3 {
            let mut campaign = fixture();
            let agreement = |party| {
                TradeAgreement::new(
                    TradeParty::Player(0),
                    party,
                    TradeBundle {
                        coin: 5.0,
                        ..Default::default()
                    },
                    TradeBundle {
                        resources: [0.0, 1.0, 0.0],
                        ..Default::default()
                    },
                    TradeFrequency::Monthly,
                )
            };
            let id = campaign.propose_national_trade(agreement(TradeParty::Npc(2))).unwrap();
            campaign.end_trade(0, id, true).unwrap();
            campaign.propose_national_trade(agreement(TradeParty::Npc(3))).unwrap();
            let id = campaign.propose_national_trade(agreement(TradeParty::Player(1))).unwrap();
            campaign.accept_national_trade(1, id).unwrap();
            campaign.propose_national_trade(agreement(TradeParty::Player(1))).unwrap();
            assert_body_fits(&format!("National trade page {page}"), width, scale, |ui| {
                campaign_trade::select_page(ui.ctx(), 0, page);
                campaign_trade::show(ui, &mut campaign, 0, scale);
            });
        }
    }
}

#[test]
fn province_trade_composer_fits_and_targets_the_selected_owner_or_npc() {
    for (width, scale) in [(546.0, 1.0), (380.0, 0.85), (380.0, 1.0)] {
        for province in 1..5 {
            let mut campaign = fixture();
            assert_body_fits(&format!("Province trade {province}"), width, scale, |ui| {
                campaign_trade::show_province(ui, &mut campaign, province, 0, scale);
            });
        }
    }
}

#[test]
fn national_trade_cancellation_updates_authoritative_relation_and_keeps_control() {
    use crate::game::economy::{TradeAgreement, TradeBundle, TradeFrequency, TradeParty};
    let mut campaign = fixture();
    let relation = campaign.politics[2].relation(0);
    let control = campaign.politics[2].independent_control(0);
    let id = campaign
        .propose_national_trade(TradeAgreement::new(
            TradeParty::Player(0),
            TradeParty::Npc(2),
            TradeBundle {
                coin: 5.0,
                ..Default::default()
            },
            TradeBundle {
                resources: [0.0, 1.0, 0.0],
                ..Default::default()
            },
            TradeFrequency::Monthly,
        ))
        .unwrap();
    campaign.end_trade(0, id, true).unwrap();
    assert_eq!(campaign.politics[2].relation(0), relation);
    campaign.end_trade(0, id, false).unwrap();
    assert_eq!(
        campaign.politics[2].relation(0),
        relation - campaign.economy.config.trade.cancellation_relation_penalty
    );
    assert_eq!(
        campaign.economy.provinces[2].relation_by_player[0],
        campaign.politics[2].relation(0)
    );
    assert_eq!(campaign.politics[2].independent_control(0), control);
}

#[test]
fn route_icon_buttons_give_notice_then_apply_the_immediate_cancellation_penalty() {
    use crate::game::economy::{
        TradeAgreement, TradeBundle, TradeFrequency, TradeParty, TradeStatus,
    };
    fn render(
        ctx: &egui::Context,
        campaign: &mut Campaign,
        time: f64,
        events: Vec<egui::Event>,
    ) -> egui::FullOutput {
        let mut output = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(1000.0, 900.0),
                )),
                time: Some(time),
                events,
                ..Default::default()
            },
            |root| {
                root.scope_builder(
                    egui::UiBuilder::new().max_rect(egui::Rect::from_min_size(
                        egui::pos2(20.0, 20.0),
                        egui::vec2(380.0, 600.0),
                    )),
                    |ui| {
                        panel_style(ui, 1.0);
                        campaign_trade::show(ui, campaign, 0, 1.0);
                    },
                );
            },
        );
        output.textures_delta.clear();
        output
    }
    let ctx = layout_context();
    ctx.memory_mut(|memory| memory.set_everything_is_visible(false));
    let mut campaign = fixture();
    campaign
        .propose_national_trade(TradeAgreement::new(
            TradeParty::Player(0),
            TradeParty::Npc(2),
            TradeBundle {
                coin: 5.0,
                ..Default::default()
            },
            TradeBundle {
                resources: [0.0, 1.0, 0.0],
                ..Default::default()
            },
            TradeFrequency::Monthly,
        ))
        .unwrap();
    let output = render(&ctx, &mut campaign, 0.0, vec![]);
    let buttons: Vec<_> = output
        .shapes
        .iter()
        .filter_map(|shape| match &shape.shape {
            egui::Shape::Rect(rect)
                if rect.fill == province_panel::PAPER
                    && rect.rect.size() == egui::vec2(30.0, 30.0) =>
            {
                Some(rect.rect.center())
            },
            _ => None,
        })
        .collect();
    assert_eq!(buttons.len(), 2, "Each open route must have two icon actions");
    let relation = campaign.politics[2].relation(0);
    for (action, position) in [(0, buttons[1]), (1, buttons[0])] {
        for pressed in [true, false] {
            render(
                &ctx,
                &mut campaign,
                0.1 + f64::from(action) * 0.2
                    + if pressed {
                        0.0
                    } else {
                        0.1
                    },
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
        if action == 0 {
            assert_eq!(campaign.economy.trades[0].cancellation_month, Some(6));
            assert_eq!(campaign.politics[2].relation(0), relation);
        } else {
            assert_eq!(campaign.economy.trades[0].status, TradeStatus::Cancelled);
            assert_eq!(campaign.politics[2].relation(0), relation - 10.0);
        }
    }
}

#[test]
fn province_overview_fits_each_political_state_at_compact_widths() {
    let campaign = fixture();
    for (width, scale) in [(500.0, 1.0), (380.0, 0.85), (380.0, 1.0)] {
        for province in 0..campaign.politics.len() {
            assert_body_fits("Province overview", width, scale, |ui| {
                province_overview(ui, &campaign, province, 0, scale);
            });
        }
    }
}

#[test]
fn complete_province_overview_fits_without_scrolling_and_aligns_badges_and_ledgers() {
    use crate::game::economy::{BuildingProject, BuildingType, ConstructionProject};
    for (width, height, scale) in [(570.0, 720.0, 1.0), (404.0, 540.0, 0.85), (404.0, 400.0, 1.0)] {
        for province in 0..5 {
            for construction in [false, true] {
                let ctx = layout_context();
                ctx.memory_mut(|memory| memory.set_everything_is_visible(false));
                let mut campaign = fixture();
                campaign.advance_month();
                if construction {
                    campaign.economy.provinces[province].construction =
                        Some(ConstructionProject::Building(BuildingProject {
                            building: BuildingType::Granary,
                            target_level: 1,
                            progress: 1.0,
                            required_progress: 4.0,
                        }));
                    if !campaign.administers_province(0, province) {
                        campaign.espionage.missions.push(
                            crate::game::politics::espionage::SpyMission {
                                owner: 0,
                                province,
                                months_active: 3,
                            },
                        );
                        campaign.refresh_intelligence_reports();
                    }
                }
                let mut view = CampaignUi {
                    open: Some(CampaignTab::Province),
                    province: Some(province),
                    ..Default::default()
                };
                let rect =
                    egui::Rect::from_min_size(egui::pos2(30.0, 30.0), egui::vec2(width, height));
                for frame in 0..2 {
                    let mut outer = rect;
                    let mut body = rect;
                    let mut output = ctx.run_ui(
                        egui::RawInput {
                            screen_rect: Some(egui::Rect::from_min_size(
                                egui::Pos2::ZERO,
                                egui::vec2(1200.0, 900.0),
                            )),
                            time: Some(frame as f64),
                            ..Default::default()
                        },
                        |root| {
                            root.scope_builder(egui::UiBuilder::new().max_rect(rect), |ui| {
                                panel_style(ui, scale);
                                outer = panel_frame(scale)
                                    .show(ui, |ui| {
                                        ui.set_width(width);
                                        ui.set_min_height(height);
                                        panel_header(
                                            ui,
                                            &campaign.economy.provinces[province].name,
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
                                                province_overview_body(
                                                    ui,
                                                    rect.bottom() - 12.0 * scale,
                                                    &campaign,
                                                    province,
                                                    0,
                                                    scale,
                                                );
                                                body = ui.min_rect();
                                            });
                                    })
                                    .response
                                    .rect;
                            });
                        },
                    );
                    output.textures_delta.clear();
                    assert!(outer.right() <= rect.right() + 1.0 && outer.bottom() <= rect.bottom() + 1.0, "Overview exceeds panel {width}×{height}, province {province}, construction={construction}: {outer:?}");
                    let text_rect = |label: &str| {
                        output
                            .shapes
                            .iter()
                            .find_map(|shape| {
                                if let egui::Shape::Text(text) = &shape.shape {
                                    (text.galley.job.text == label && text.pos.y >= body.top())
                                        .then_some(egui::Rect::from_min_size(
                                            text.pos,
                                            text.galley.size(),
                                        ))
                                } else {
                                    None
                                }
                            })
                            .unwrap_or_else(|| panic!("Missing overview text {label}"))
                    };
                    if campaign.economy.provinces[province].owner == Some(0) {
                        let demand = text_rect("Food demand");
                        let supplied = text_rect("Food supplied");
                        assert!(
                            (demand.center().y - supplied.center().y).abs() < 1.0
                                && demand.right() < supplied.left(),
                            "Summary badges must stay on one horizontal row"
                        );
                        let row = text_rect("Nobles");
                        let values: Vec<_> = output
                            .shapes
                            .iter()
                            .filter_map(|shape| {
                                if let egui::Shape::Text(text) = &shape.shape {
                                    let rect =
                                        egui::Rect::from_min_size(text.pos, text.galley.size());
                                    ((rect.center().y - row.center().y).abs() < 1.0)
                                        .then_some((rect.center().x, text.galley.job.text.as_str()))
                                } else {
                                    None
                                }
                            })
                            .collect();
                        assert_eq!(values.len(), 4, "Population row must contain a name, total, change and happiness: {values:?}");
                        let mut values = values;
                        values.sort_by(|a, b| a.0.total_cmp(&b.0));
                        assert!(
                            values[2].1.starts_with(['+', '-']),
                            "Monthly change must precede happiness: {values:?}"
                        );
                        assert!(
                            values[3].1.ends_with('%'),
                            "Happiness must be the last column: {values:?}"
                        );
                        assert!(
                            values[3].0 > body.left() + body.width() * 0.85,
                            "Ledger must use the right side of the panel"
                        );
                    } else {
                        text_rect("Population");
                        if !campaign.administers_province(0, province) && !construction {
                            text_rect("?");
                        }
                    }
                    for shape in &output.shapes {
                        if let egui::Shape::Text(text) = &shape.shape {
                            if text.pos.y >= body.top() {
                                let text_bounds =
                                    egui::Rect::from_min_size(text.pos, text.galley.size());
                                assert!(
                                    text_bounds.right() <= body.right() + 1.0
                                        && text_bounds.bottom() <= rect.bottom() - 10.0 * scale,
                                    "Overview text must remain visible: {} {text_bounds:?}",
                                    text.galley.job.text
                                );
                                assert_ne!(text.galley.job.text, "Recent notifications");
                            }
                        }
                    }
                    output.textures_delta.clear();
                }
            }
        }
    }
}

#[test]
fn campaign_symbols_use_transparent_art_without_circle_frames() {
    let ctx = layout_context();
    let symbols = [
        Icon::Amount,
        Icon::Delta,
        Icon::Change,
        Icon::Notifications,
        Icon::Control,
        Icon::Relation,
        Icon::Diplomacy,
        Icon::Policies,
        Icon::Construction,
        Icon::Cancel,
        Icon::Notice,
        Icon::Confirm,
    ];
    let mut output = ctx.run_ui(egui::RawInput::default(), |ui| {
        ui.horizontal(|ui| {
            for symbol in symbols {
                icon(ui, symbol, 32.0);
            }
        });
    });
    assert!(
        !output.shapes.iter().any(|shape| matches!(shape.shape, egui::Shape::Circle(_))),
        "Campaign icon rendering must not add circle frames"
    );
    let images: Vec<_> = output
        .textures_delta
        .set
        .iter()
        .flat_map(|(_, deltas)| deltas.iter())
        .filter_map(|delta| {
            let egui::ImageData::Color(image) = &delta.image;
            (image.size == [64, 64]).then_some(image)
        })
        .collect();
    assert_eq!(images.len(), symbols.len(), "Each semantic icon must load its own prepared art");
    for image in images {
        for corner in [0, 63, 64 * 63, 64 * 64 - 1] {
            assert_eq!(image.pixels[corner].a(), 0, "Icon corners must remain transparent");
        }
        assert!(image.pixels.iter().any(|pixel| pixel.a() > 128), "Icon art must be visible");
    }
    output.textures_delta.clear();
}

#[test]
fn province_policy_choices_apply_only_for_the_direct_owner() {
    use crate::game::economy::ResourceFocus;
    for owner in [0, 1] {
        let ctx = layout_context();
        ctx.memory_mut(|memory| memory.set_everything_is_visible(false));
        let mut campaign = fixture();
        campaign.economy.provinces[0].owner = Some(owner);
        let mut render = |time, events| {
            ctx.run_ui(
                egui::RawInput {
                    time: Some(time),
                    events,
                    ..Default::default()
                },
                |root| {
                    root.scope_builder(
                        egui::UiBuilder::new().max_rect(egui::Rect::from_min_size(
                            egui::pos2(20.0, 20.0),
                            egui::vec2(546.0, 480.0),
                        )),
                        |ui| {
                            panel_style(ui, 1.0);
                            campaign_economy::policies(ui, &mut campaign.economy, 0, 0);
                        },
                    );
                },
            )
        };
        let mut output = render(0.0, vec![]);
        let position = output
            .shapes
            .iter()
            .find_map(|shape| {
                if let egui::Shape::Text(text) = &shape.shape {
                    (text.galley.job.text == "Food").then_some(text.pos + egui::vec2(4.0, 4.0))
                } else {
                    None
                }
            })
            .expect("Food policy choice");
        output.textures_delta.clear();
        for (time, pressed) in [(0.1, true), (0.2, false)] {
            let mut output = render(
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
            output.textures_delta.clear();
        }
        drop(render);
        assert_eq!(
            campaign.economy.provinces[0].policies.focus,
            if owner == 0 {
                ResourceFocus::Food
            } else {
                ResourceFocus::Balanced
            }
        );
    }
}

#[test]
fn province_policy_sections_and_new_choices_are_available_only_to_the_direct_owner() {
    use crate::game::economy::{
        CivicSpending, ConstructionPace, ManumissionPolicy, RecruitmentEffort, ResourceFocus,
    };
    for owner in [0, 1] {
        let ctx = layout_context();
        ctx.memory_mut(|memory| memory.set_everything_is_visible(false));
        let mut campaign = fixture();
        campaign.economy.provinces[0].owner = Some(owner);
        campaign.economy.provinces[0].policies.focus = ResourceFocus::Food;
        let mut render = |time, events| {
            let mut output = ctx.run_ui(
                egui::RawInput {
                    time: Some(time),
                    events,
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(1200.0, 1100.0),
                    )),
                    ..Default::default()
                },
                |root| {
                    root.scope_builder(
                        egui::UiBuilder::new().max_rect(egui::Rect::from_min_size(
                            egui::pos2(20.0, 20.0),
                            egui::vec2(546.0, 900.0),
                        )),
                        |ui| {
                            panel_style(ui, 1.0);
                            campaign_economy::policies(ui, &mut campaign.economy, 0, 0);
                        },
                    );
                },
            );
            output.textures_delta.clear();
            output
        };
        let output = render(0.0, vec![]);
        let labels: Vec<_> = output
            .shapes
            .iter()
            .filter_map(|s| match &s.shape {
                egui::Shape::Text(text) => Some(text.galley.job.text.as_str()),
                _ => None,
            })
            .collect();
        for label in [
            "ECONOMY & WORKS",
            "POPULATION & WELFARE",
            "MILITARY",
            "Resource Focus",
            "Construction Pace",
            "Migration Focus",
            "Civic Spending",
            "Manumission",
            "Recruitment Effort",
            "Balanced",
        ] {
            assert!(labels.contains(&label), "Missing local policy label: {label}");
        }
        assert!(!labels.contains(&"All"));
        assert!(!labels.contains(&"PROVINCE POLICIES"));
        for (index, label) in
            ["Balanced", "Urgent", "Generous", "Encouraged", "High"].into_iter().enumerate()
        {
            let output = render(index as f64 + 0.1, vec![]);
            let position = output
                .shapes
                .iter()
                .find_map(|shape| match &shape.shape {
                    egui::Shape::Text(text) if text.galley.job.text == label => {
                        Some(text.pos + egui::vec2(3.0, 3.0))
                    },
                    _ => None,
                })
                .unwrap_or_else(|| panic!("Missing choice: {label}"));
            for (offset, pressed) in [(0.2, true), (0.3, false)] {
                render(
                    index as f64 + offset,
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
        }
        drop(render);
        let policies = campaign.economy.provinces[0].policies;
        if owner == 0 {
            assert_eq!(policies.focus, ResourceFocus::Balanced);
            assert_eq!(policies.construction, ConstructionPace::Urgent);
            assert_eq!(policies.civic_spending, CivicSpending::Generous);
            assert_eq!(policies.manumission, ManumissionPolicy::Encouraged);
            assert_eq!(policies.recruitment, RecruitmentEffort::High);
        } else {
            assert_eq!(policies.focus, ResourceFocus::Food);
            assert_eq!(policies.construction, ConstructionPace::Normal);
            assert_eq!(policies.civic_spending, CivicSpending::Frugal);
            assert_eq!(policies.manumission, ManumissionPolicy::Normal);
            assert_eq!(policies.recruitment, RecruitmentEffort::Normal);
        }
    }
}

#[test]
fn province_policy_scroll_reaches_military_without_shrinking_the_cards() {
    let ctx = layout_context();
    ctx.memory_mut(|memory| memory.set_everything_is_visible(false));
    let mut campaign = fixture();
    let rect = egui::Rect::from_min_size(egui::pos2(20.0, 20.0), egui::vec2(380.0, 240.0));
    let mut render = |time, events| {
        let mut output = ctx.run_ui(
            egui::RawInput {
                time: Some(time),
                events,
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(1200.0, 900.0),
                )),
                ..Default::default()
            },
            |root| {
                root.scope_builder(egui::UiBuilder::new().max_rect(rect), |ui| {
                    panel_style(ui, 1.0);
                    campaign_economy::policies(ui, &mut campaign.economy, 0, 0);
                    assert!(ui.min_rect().bottom() <= rect.bottom() + 1.0);
                });
            },
        );
        output.textures_delta.clear();
        output
    };
    let visible = |output: &egui::FullOutput, label: &str| {
        output.shapes.iter().any(|s| match &s.shape {
            egui::Shape::Text(text) => {
                text.galley.job.text == label
                    && s.clip_rect
                        .intersects(egui::Rect::from_min_size(text.pos, text.galley.size()))
            },
            _ => false,
        })
    };
    let first = render(0.0, vec![]);
    assert!(!visible(&first, "Recruitment Effort"));
    render(
        0.1,
        vec![
            egui::Event::PointerMoved(rect.center()),
            egui::Event::MouseWheel {
                phase: egui::TouchPhase::Move,
                unit: egui::MouseWheelUnit::Point,
                delta: egui::vec2(0.0, -1000.0),
                modifiers: egui::Modifiers::NONE,
            },
        ],
    );
    let mut last = render(0.2, vec![]);
    for frame in 3..15 {
        last = render(frame as f64 * 0.1, vec![]);
    }
    assert!(visible(&last, "Recruitment Effort"), "Scrolling must expose the final policy");
}

#[test]
fn province_policy_cards_fit_and_keep_nationwide_edicts_out_of_local_controls() {
    for (width, height, scale) in [(546.0, 480.0, 1.0), (380.0, 360.0, 0.85), (380.0, 240.0, 1.0)] {
        let ctx = layout_context();
        ctx.memory_mut(|memory| memory.set_everything_is_visible(false));
        let mut campaign = fixture();
        let rect = egui::Rect::from_min_size(egui::pos2(20.0, 20.0), egui::vec2(width, height));
        let mut used = rect;
        let mut output = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(1200.0, 900.0),
                )),
                ..Default::default()
            },
            |root| {
                root.scope_builder(egui::UiBuilder::new().max_rect(rect), |ui| {
                    panel_style(ui, scale);
                    campaign_economy::policies(ui, &mut campaign.economy, 0, 0);
                    used = ui.min_rect();
                });
            },
        );
        output.textures_delta.clear();
        assert!(
            used.right() <= rect.right() + 1.0 && used.bottom() <= rect.bottom() + 1.0,
            "Province policy cards overflow {width}×{height}: {used:?}"
        );
        let labels: Vec<_> = output
            .shapes
            .iter()
            .filter_map(|shape| match &shape.shape {
                egui::Shape::Text(text) => Some(text.galley.job.text.as_str()),
                _ => None,
            })
            .collect();
        for title in ["Food Rations", "Slave Labor", "Noble Taxes", "Army Wages"] {
            assert!(!labels.contains(&title), "Nationwide edict must stay in Governance: {title}");
        }
        for title in ["Resource Focus", "Migration Focus"] {
            assert!(labels.contains(&title), "Missing local policy: {title}");
        }
        assert!(!labels.contains(&"MONTHLY PREVIEW"), "Policies must not show a preview");
        for shape in &output.shapes {
            if let egui::Shape::Text(text) = &shape.shape {
                let bounds = egui::Rect::from_min_size(text.pos, text.galley.size());
                assert!(
                    bounds.left() >= rect.left() - 1.0 && bounds.right() <= rect.right() + 1.0,
                    "Policy text must fit: {} {bounds:?}",
                    text.galley.job.text
                );
            }
        }
    }
}

#[test]
fn province_notice_cards_filter_history_and_click_through_to_the_related_panel() {
    use super::super::campaign_notifications::NoticeKind;
    for (kind, section) in [
        (NoticeKind::ConstructionStarted, 2),
        (NoticeKind::BuildingCompleted, 2),
        (NoticeKind::RecruitmentCompleted, 3),
        (NoticeKind::HostileRelation, 4),
        (NoticeKind::TradeInterrupted, 5),
        (NoticeKind::FoodShortage, 1),
    ] {
        let ctx = layout_context();
        ctx.memory_mut(|memory| memory.set_everything_is_visible(false));
        let mut campaign = fixture();
        campaign.notifications.province_notice(
            0,
            0,
            1,
            NoticeSeverity::Warning,
            kind,
            "Visible notice",
            "Details about this province's event.",
        );
        campaign.notifications.province_notice(
            1,
            0,
            1,
            NoticeSeverity::Info,
            kind,
            "Private notice",
            "Another player's private event.",
        );
        campaign.notifications.province_notice(
            0,
            1,
            1,
            NoticeSeverity::Info,
            kind,
            "Elsewhere notice",
            "An event from another province.",
        );
        let rect = egui::Rect::from_min_size(egui::pos2(20.0, 20.0), egui::vec2(380.0, 260.0));
        let mut selected = None;
        let mut render = |time, events| {
            ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(1000.0, 700.0),
                    )),
                    time: Some(time),
                    events,
                    ..Default::default()
                },
                |root| {
                    root.scope_builder(egui::UiBuilder::new().max_rect(rect), |ui| {
                        panel_style(ui, 1.0);
                        if let Some(notice) = egui::ScrollArea::vertical()
                            .max_height(rect.height())
                            .show(ui, |ui| province_notifications(ui, &campaign, 0, 0, 1.0))
                            .inner
                        {
                            selected = Some(notice);
                        }
                    });
                },
            )
        };
        let mut output = render(0.0, vec![]);
        output.textures_delta.clear();
        let labels: Vec<_> = output
            .shapes
            .iter()
            .filter_map(|shape| {
                if let egui::Shape::Text(text) = &shape.shape {
                    Some(text.galley.job.text.as_str())
                } else {
                    None
                }
            })
            .collect();
        assert!(labels.contains(&"Visible notice") && labels.contains(&"Feb 60 AD"));
        assert!(!labels.contains(&"Private notice") && !labels.contains(&"Elsewhere notice"));
        let position = output
            .shapes
            .iter()
            .find_map(|shape| {
                if let egui::Shape::Text(text) = &shape.shape {
                    (text.galley.job.text == "Visible notice")
                        .then_some(text.pos + egui::vec2(5.0, 5.0))
                } else {
                    None
                }
            })
            .unwrap();
        for (time, pressed) in [(0.1, true), (0.2, false)] {
            let mut output = render(
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
            output.textures_delta.clear();
        }
        let notice = selected.expect("The entire notification card must be clickable");
        let mut view = CampaignUi {
            open: Some(CampaignTab::Province),
            province: Some(0),
            section: 5,
            ..Default::default()
        };
        let mut detail = ProvincePanelOpen(Some(MapDetail::Province(0)));
        let mut map = MapView::default();
        open_notification(&mut view, &notice, &campaign, &mut detail, &mut map);
        if kind == NoticeKind::FoodShortage {
            assert_eq!(view.open, Some(CampaignTab::Governance));
        } else if kind == NoticeKind::TradeInterrupted {
            assert_eq!(view.open, Some(CampaignTab::Trade));
            assert_eq!(detail.0, None);
        } else {
            assert_eq!(view.section, section);
            assert_eq!(view.last_detail, detail.0);
        }
    }
}

#[test]
fn overview_meters_follow_live_control_relation_and_ownership() {
    let ctx = layout_context();
    let mut politics = ProvincePolitics::independent(2);
    let states = [
        (
            PoliticalState::Independent {
                local: 100.0,
                shares: vec![0.0, 0.0],
            },
            0.0,
            vec!["0/100"],
        ),
        (
            PoliticalState::Independent {
                local: 15.0,
                shares: vec![55.0, 30.0],
            },
            24.5,
            vec!["55/100", "24/100"],
        ),
        (
            PoliticalState::Vassal {
                overlord: 0,
                control: 72.5,
                tribute: Tribute::Normal,
            },
            83.0,
            vec!["72/100", "83/100"],
        ),
        (
            PoliticalState::Vassal {
                overlord: 1,
                control: 40.0,
                tribute: Tribute::Normal,
            },
            61.0,
            vec!["40/100", "61/100"],
        ),
        (
            PoliticalState::Owned {
                owner: 0,
            },
            83.0,
            vec!["100/100"],
        ),
        (
            PoliticalState::Owned {
                owner: 1,
            },
            45.0,
            vec!["100/100", "45/100"],
        ),
    ];
    for (state, relation, expected) in states {
        politics.state = state;
        politics.relations[0] = relation;
        let mut output = ctx.run_ui(egui::RawInput::default(), |root| {
            egui::CentralPanel::default().show(root, |ui| {
                ui.set_width(500.0);
                panel_style(ui, 1.0);
                overview_politics(ui, &politics, 0, 1.0);
            });
        });
        let labels: Vec<_> = output
            .shapes
            .iter()
            .filter_map(|shape| {
                if let egui::Shape::Text(text) = &shape.shape {
                    Some(text.galley.job.text.as_str())
                } else {
                    None
                }
            })
            .collect();
        for label in expected {
            assert!(labels.contains(&label), "Missing {label:?} in {labels:?}");
        }
        if matches!(
            politics.state,
            PoliticalState::Owned {
                owner: 0
            }
        ) {
            assert!(
                labels.iter().filter(|label| **label == "100/100").count() == 2,
                "Domestic relations must give way to happiness"
            );
        }
        assert!(!labels.contains(&"Your province") && !labels.contains(&"Domestic province"));
        output.textures_delta.clear();
    }
}

#[test]
fn terrain_city_badges_keep_their_sides_and_city_opens_buildings() {
    let ctx = layout_context();
    ctx.memory_mut(|memory| memory.set_everything_is_visible(false));
    let campaign = fixture();
    let mut view = CampaignUi {
        open: Some(CampaignTab::Province),
        province: Some(1),
        ..Default::default()
    };
    run_province_header(&ctx, &campaign, &mut view, 0.0, vec![]);
    let output = run_province_header(&ctx, &campaign, &mut view, 0.1, vec![]);
    let text_position = |label: &str| {
        output
            .shapes
            .iter()
            .find_map(|shape| {
                if let egui::Shape::Text(text) = &shape.shape {
                    (text.galley.job.text == label).then_some(text.pos)
                } else {
                    None
                }
            })
            .unwrap_or_else(|| panic!("Missing landscape badge {label}"))
    };
    let terrain = text_position("Farmland");
    let city = text_position("Carthage");
    assert!(terrain.x < city.x);
    assert!((terrain.y - city.y).abs() < 2.0);
    let position = city + egui::vec2(5.0, 5.0);
    for (time, pressed) in [(0.2, true), (0.3, false)] {
        run_province_header(
            &ctx,
            &campaign,
            &mut view,
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
    assert_eq!(view.section, 2, "City badge must open the buildings inspector");
}

#[test]
fn military_roster_and_active_battle_fit_desktop_and_compact_panels() {
    for (width, scale) in [(500.0, 1.0), (380.0, 0.85), (380.0, 1.0)] {
        for in_battle in [false, true] {
            let mut campaign = fixture();
            let province = if in_battle {
                1
            } else {
                0
            };
            if in_battle {
                campaign
                    .military
                    .seed_unit(1, ForceOwner::Player(0), UnitType::LightCavalry)
                    .unwrap();
                campaign.military.seed_unit(1, ForceOwner::Player(0), UnitType::Archers).unwrap();
                campaign
                    .military
                    .start_battle(
                        1,
                        &[ForceOwner::Player(0)],
                        &[ForceOwner::Player(1)],
                        Some(1),
                        Some(0),
                        MilitaryTerrain::Hills,
                        2,
                        9,
                    )
                    .unwrap();
            }
            assert_body_fits(&format!("Military battle={in_battle}"), width, scale, |ui| {
                campaign_military::show(
                    ui,
                    &campaign.military,
                    &campaign.economy,
                    &campaign.graph,
                    province,
                    0,
                    |_| true,
                    |_| None,
                    |_, _| MilitaryAccess::Peaceful,
                    |a, b| campaign.forces_hostile(a, b),
                );
            });
        }
    }
}

#[test]
fn military_panel_actions_drive_paid_drafting_movement_locked_battle_and_retreat() {
    use campaign_military::MilitaryUiAction as Action;
    let mut c = fixture();
    let owner = ForceOwner::Player(0);
    c.military.config.units[UnitType::LightInfantry as usize].recruitment_months = 1.;
    let population = c.economy.provinces[0].population[2];
    let metal = c.economy.players[0].resources[1];
    assert_eq!(
        apply_military_action(&mut c, 0, 0, Action::Recruit(UnitType::LightInfantry)),
        "Order accepted."
    );
    assert_eq!(c.economy.provinces[0].population[2], population - 10.);
    assert_eq!(c.economy.players[0].resources[1], metal - 12.);
    assert_eq!(apply_military_action(&mut c, 0, 0, Action::CancelRecruitment), "Order accepted.");
    assert_eq!(
        c.economy.provinces[0].population[2],
        population - 10.,
        "cancel must not refund drafted population"
    );
    assert_eq!(
        apply_military_action(&mut c, 0, 0, Action::Recruit(UnitType::LightInfantry)),
        "Order accepted."
    );
    c.advance_month();
    let recruit = c.military.provinces[0].forces[&owner].last().unwrap().clone();
    let civilians = c.economy.provinces[0].population[2];
    assert_eq!(apply_military_action(&mut c, 0, 0, Action::Disband(recruit.id)), "Order accepted.");
    assert_eq!(c.economy.provinces[0].population[2], civilians + recruit.current_manpower);
    let plan = BattlePlan {
        tactic: CombatTactic::Envelopment,
        ..Default::default()
    };
    assert_eq!(apply_military_action(&mut c, 0, 0, Action::SavePlan(plan)), "Order accepted.");
    let unit = c.military.provinces[0].forces[&owner]
        .iter()
        .find(|unit| unit.unit_type == UnitType::HeavyInfantry)
        .unwrap()
        .id;
    c.declare_hostility(0, 1);
    assert_eq!(
        apply_military_action(
            &mut c,
            0,
            0,
            Action::Move {
                destination: 1,
                units: vec![unit],
                route: vec![1],
                plan
            }
        ),
        "Order accepted."
    );
    assert!(c.military.provinces[0].forces[&owner].iter().all(|cohort| cohort.id != unit));
    let moving = c.military.movements[0].id;
    let override_plan = BattlePlan {
        tactic: CombatTactic::ShockAction,
        ..plan
    };
    assert_eq!(
        apply_military_action(
            &mut c,
            0,
            0,
            Action::SaveMovementPlan {
                order: moving,
                plan: override_plan
            }
        ),
        "Order accepted."
    );
    assert_eq!(
        c.military.provinces[0].plans[&owner], plan,
        "moving override must preserve stationary default"
    );
    c.military.config.base_manpower_damage = 0.;
    c.military.config.base_morale_damage = 0.;
    c.advance_month();
    let battle = &c.military.battles[0];
    let battle_id = battle.id;
    assert_eq!(battle.attackers.plans[&owner], override_plan);
    assert_ne!(apply_military_action(&mut c, 1, 0, Action::SavePlan(plan)), "Order accepted.");
    assert_eq!(
        apply_military_action(
            &mut c,
            1,
            0,
            Action::Retreat {
                battle: battle_id,
                attacker: true
            }
        ),
        "Order accepted."
    );
    c.advance_month();
    assert!(c.military.battles.is_empty());
    assert!(c.military.provinces[0].forces[&owner].iter().any(|cohort| cohort.id == unit));
    assert_eq!(c.economy.provinces[1].owner, Some(1));
}

#[test]
fn senate_appointment_notice_opens_the_chamber_for_each_local_player() {
    use super::super::campaign_notifications::{NoticeAction, NoticeKind};
    let mut c = fixture();
    for senator in c.senate.senators.iter_mut().take(12) {
        senator.allegiance = Some(0);
    }
    let event = c.senate.promote(0, &mut c.actors, &c.senate_config).unwrap();
    c.record_senate_event(&event);
    assert_eq!(c.actors[0].rank, PoliticalRank::Praetor);
    for player in 0..2 {
        let result = c
            .notifications
            .history_for(player)
            .find(|notice| notice.kind == NoticeKind::SenateOfficeAppointed)
            .unwrap();
        assert_eq!(result.action, NoticeAction::OpenSenate);
        assert_eq!(result.province, None);
    }
}
