//! Map resource strip and detail panels.

use super::*;

pub(in crate::app) fn load_hud_resource_icon(
    ctx: &egui::Context,
    index: usize,
) -> egui::TextureHandle {
    let image = image::load_from_memory(HUD_RESOURCE_ICONS[index])
        .expect("HUD resource PNG must be valid")
        .resize_exact(128, 128, image::imageops::FilterType::Lanczos3)
        .to_rgba8();
    ctx.load_texture(
        HUD_RESOURCE_NAMES[index],
        egui::ColorImage::from_rgba_unmultiplied(
            [image.width() as usize, image.height() as usize],
            image.as_raw(),
        ),
        egui::TextureOptions::LINEAR,
    )
}

pub(in crate::app) fn load_pop_class_icon(
    ctx: &egui::Context,
    index: usize,
) -> egui::TextureHandle {
    let image = image::load_from_memory(POP_CLASS_ICONS[index])
        .expect("Pop class PNG must be valid")
        .resize_exact(64, 64, image::imageops::FilterType::Lanczos3)
        .to_rgba8();
    ctx.load_texture(
        POP_CLASS_NAMES[index],
        egui::ColorImage::from_rgba_unmultiplied(
            [image.width() as usize, image.height() as usize],
            image.as_raw(),
        ),
        egui::TextureOptions::LINEAR,
    )
}

pub(in crate::app) fn hud_delta_color(delta: f64) -> egui::Color32 {
    if delta > 0.0 {
        egui::Color32::from_rgb(43, 133, 67)
    } else if delta < 0.0 {
        egui::Color32::from_rgb(174, 54, 45)
    } else {
        egui::Color32::from_rgb(35, 35, 32)
    }
}

pub(in crate::app) fn format_hud_number(value: f64) -> String {
    let value = value.floor();
    let magnitude = value.abs();
    if magnitude < 1_000.0 {
        return format!("{value:.0}");
    }

    // Promote values that round to 1000.0k so the unit stays readable.
    let (scaled, suffix) = if magnitude >= 999_950.0 {
        (magnitude / 1_000_000.0, "M")
    } else {
        (magnitude / 1_000.0, "k")
    };
    let rounded = (scaled * 10.0).round() / 10.0;
    let sign = if value < 0.0 {
        "-"
    } else {
        ""
    };
    format!("{sign}{rounded:.1}{suffix}")
}

pub(in crate::app) fn format_hud_delta(value: f64) -> String {
    if value >= 0.0 {
        format!("+{}", format_hud_number(value))
    } else {
        format_hud_number(value)
    }
}

pub(in crate::app) fn paint_hud_resources(
    painter: &egui::Painter,
    origin: egui::Pos2,
    scale: f32,
    date_left: f32,
    resources: &[HudResource; 7],
    icons: &[egui::TextureHandle; 7],
) {
    let p = |x: f32, y: f32| origin + egui::vec2(x, y) * scale;
    let second = MAP_RESOURCE_STRIP_LEFT + HUD_FIRST_GROUP_WIDTH;
    let third = second + HUD_SECOND_GROUP_WIDTH;
    for (index, (resource, x)) in resources.iter().zip(hud_resource_positions()).enumerate() {
        if x + HUD_RESOURCE_WIDTH > date_left - HUD_RESOURCE_GROUP_PADDING {
            break;
        }
        let amount = format_hud_number(resource.amount);
        let delta = format_hud_delta(resource.monthly_delta);
        let gap = 4.0 * scale;
        let max_text_width = (HUD_RESOURCE_WIDTH - 16.0 - 4.0 - 20.0) * scale;
        let fit = |label: String, size: f32, color: egui::Color32| {
            let mut font_size = size * scale;
            loop {
                let galley = painter.layout_no_wrap(
                    label.clone(),
                    egui::FontId::proportional(font_size),
                    color,
                );
                if galley.size().x <= max_text_width {
                    break galley;
                }
                font_size *= 0.98 * max_text_width / galley.size().x;
            }
        };
        let amount_color = if index == 6 && resource.amount < 50.0 {
            egui::Color32::from_rgb(170, 53, 45)
        } else if index == 6 && resource.amount > 50.0 {
            egui::Color32::from_rgb(42, 125, 63)
        } else {
            egui::Color32::from_rgb(35, 35, 32)
        };
        let delta_color = hud_delta_color(resource.monthly_delta);
        let amount_galley = fit(amount, 17.0, amount_color);
        let delta_galley = fit(delta, 13.5, delta_color);
        let text_width = delta_galley.size().x.max(amount_galley.size().x);
        let icon_size = ((HUD_RESOURCE_WIDTH - 16.0) * scale - gap - text_width)
            .clamp(20.0 * scale, 38.0 * scale);
        let icon_left =
            p(x + HUD_RESOURCE_WIDTH * 0.5, 0.0).x - (icon_size + gap + text_width) * 0.5;
        let text_right = icon_left + icon_size + gap + text_width;
        painter.image(
            icons[index].id(),
            egui::Rect::from_min_size(
                egui::pos2(icon_left, p(0.0, 24.0).y - icon_size * 0.5),
                egui::vec2(icon_size, icon_size),
            ),
            egui::Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(1.0, 1.0)),
            egui::Color32::WHITE,
        );
        painter.galley(
            egui::pos2(
                text_right - amount_galley.size().x,
                p(0.0, 17.0).y - amount_galley.size().y * 0.5,
            ),
            amount_galley,
            amount_color,
        );
        painter.galley(
            egui::pos2(
                text_right - delta_galley.size().x,
                p(0.0, 35.0).y - delta_galley.size().y * 0.5,
            ),
            delta_galley,
            delta_color,
        );
    }
    for separator in [second, third, third + HUD_THIRD_GROUP_WIDTH] {
        if separator < date_left - HUD_RESOURCE_GROUP_PADDING {
            painter.line_segment(
                [p(separator, 4.0), p(separator, 43.0)],
                egui::Stroke::new(scale, egui::Color32::from_rgb(192, 183, 164)),
            );
        }
    }
}

pub(in crate::app) fn hud_resource_positions() -> [f32; 7] {
    let first = MAP_RESOURCE_STRIP_LEFT;
    let second = first + HUD_FIRST_GROUP_WIDTH;
    let third = second + HUD_SECOND_GROUP_WIDTH;
    [
        first + HUD_RESOURCE_GROUP_PADDING,
        first + HUD_RESOURCE_GROUP_PADDING + HUD_RESOURCE_WIDTH,
        first + HUD_RESOURCE_GROUP_PADDING + HUD_RESOURCE_WIDTH * 2.0,
        second + HUD_RESOURCE_GROUP_PADDING,
        second + HUD_RESOURCE_GROUP_PADDING + HUD_RESOURCE_WIDTH,
        third + HUD_RESOURCE_GROUP_PADDING,
        third + HUD_RESOURCE_GROUP_PADDING + HUD_RESOURCE_WIDTH,
    ]
}

pub(in crate::app) fn draw_map_resources(
    mut contexts: EguiContexts,
    mut textures: Local<Option<[egui::TextureHandle; 7]>>,
    mut pop_textures: Local<Option<[egui::TextureHandle; 4]>>,
    mut open_resource: Local<Option<usize>>,
    mut open_coin: Local<bool>,
    mut open_influence: Local<bool>,
    mut open_population: Local<bool>,
    mut open_pop_class: Local<Option<usize>>,
    mut open_happiness: Local<bool>,
    resources: Res<HudResources>,
    game: Res<ActiveGame>,
    practice: Res<LocalPractice>,
    ownership: Res<ProvinceOwnership>,
    campaign: Res<campaign::Campaign>,
) {
    let Ok(context) = contexts.ctx_mut() else {
        return;
    };
    let screen = context.content_rect();
    let scale = viewport_ui_scale(screen.size());
    if !map_resource_strip_visible(screen, scale) {
        *open_resource = None;
        *open_coin = false;
        *open_influence = false;
        *open_population = false;
        *open_pop_class = None;
        *open_happiness = false;
        return;
    }
    let icons = textures
        .get_or_insert_with(|| std::array::from_fn(|index| load_hud_resource_icon(context, index)));
    let painter = context.layer_painter(egui::LayerId::new(
        egui::Order::Foreground,
        egui::Id::new("augustus_map_edge_frame"),
    ));
    let date_left = map_resource_strip_right(screen, scale) - MAP_DATE_SECTION_WIDTH;
    let player = if *game == ActiveGame::LocalPractice {
        practice.active_player
    } else {
        0
    };
    let mut displayed = resources.for_player(player);
    if *game == ActiveGame::LocalPractice && !campaign.active {
        for (resource, production) in
            displayed[..3].iter_mut().zip(ownership.net_production_for(player))
        {
            resource.monthly_delta = production;
        }
        displayed[3].monthly_delta = ownership.coin_delta_for(player);
        displayed[4].monthly_delta = ownership.influence_delta_for(player);
        displayed[5].amount = ownership.total_population_for(player);
        displayed[5].monthly_delta = ownership.population_change_for(
            player,
            displayed[0].amount,
            resources.famine_months.get(player).copied().unwrap_or(0),
        );
    }
    paint_hud_resources(&painter, screen.min, scale, date_left, &displayed, icons);
    if campaign.active {
        campaign_resource_details(context, screen, scale, date_left, player, &campaign);
        return;
    }
    if *game == ActiveGame::LocalPractice {
        resource_panel::show(
            context,
            screen,
            scale,
            date_left,
            player,
            &ownership,
            icons,
            &mut open_resource,
        );
        coin_panel::show(
            context,
            screen,
            scale,
            date_left,
            player,
            &ownership,
            &icons[3],
            &mut open_coin,
        );
        influence_panel::show(
            context,
            screen,
            scale,
            date_left,
            player,
            &ownership,
            &icons[4],
            &mut open_influence,
        );
        let pop_icons = pop_textures.get_or_insert_with(|| {
            std::array::from_fn(|index| load_pop_class_icon(context, index))
        });
        population_panel::show(
            context,
            screen,
            scale,
            date_left,
            player,
            &ownership,
            displayed[5],
            &icons[5],
            pop_icons,
            &mut open_population,
            &mut open_pop_class,
        );
        happiness_panel::show(
            context,
            screen,
            scale,
            date_left,
            resources.happiness_for(player),
            &icons[6],
            pop_icons,
            &mut open_happiness,
        );
    } else {
        *open_resource = None;
        *open_coin = false;
        *open_influence = false;
        *open_population = false;
        *open_pop_class = None;
        *open_happiness = false;
    }
}

// The edge art stays on the foreground layer while the map uses the full viewport.

/// Explain authoritative campaign values, including storage and proportional civilian/army supply.
fn campaign_resource_details(
    ctx: &egui::Context,
    screen: egui::Rect,
    scale: f32,
    date_left: f32,
    player: usize,
    campaign: &campaign::Campaign,
) {
    let Some(wallet) = campaign.economy.players.get(player) else {
        return;
    };
    for (index, x) in hud_resource_positions().into_iter().enumerate() {
        if x + HUD_RESOURCE_WIDTH > date_left - HUD_RESOURCE_GROUP_PADDING {
            break;
        }
        egui::Area::new(egui::Id::new(("campaign_resource_details", index)))
            .fixed_pos(screen.min + egui::vec2(x, 0.0) * scale)
            .order(egui::Order::Foreground)
            .show(ctx, |ui| {
                let (_, response) = ui.allocate_exact_size(
                    egui::vec2(HUD_RESOURCE_WIDTH, 48.0) * scale,
                    egui::Sense::hover(),
                );
                response.on_hover_ui(|ui| {
                    campaign_resource_breakdown(ui, index, player, campaign, wallet, scale)
                });
            });
    }
}

/// Bounded illustrated source ledger, shared by all seven HUD tooltips.
pub(in crate::app) fn campaign_resource_breakdown(
    ui: &mut egui::Ui,
    index: usize,
    player: usize,
    campaign: &campaign::Campaign,
    wallet: &crate::game::economy::PlayerEconomy,
    scale: f32,
) {
    use campaign_widgets::{icon, stat, Icon};
    *ui.style_mut() = campaign_widgets::map_style(scale);
    let symbols = [
        Icon::Food,
        Icon::Metal,
        Icon::Stone,
        Icon::Coin,
        Icon::Influence,
        Icon::Population,
        Icon::Happiness,
    ];
    ui.set_width(380.0 * scale);
    ui.horizontal(|ui| {
        icon(ui, symbols[index], 28.0 * scale);
        ui.strong(HUD_RESOURCE_NAMES[index]);
    });
    if index < 3 {
        ui.label(format!("{:.1} / {:.0} stored",wallet.resources[index],wallet.storage[index]))
            .on_hover_text("Production, monthly trade and consumption resolve before storage overflow is discarded. Immediate exchanges and changes of ownership enforce capacity immediately.");
        ui.add(
            egui::ProgressBar::new(
                (wallet.resources[index] / wallet.storage[index].max(1.0)).clamp(0.0, 1.0) as f32,
            )
            .desired_width(ui.available_width()),
        );
    }
    if index < 5 {
        let change =
            campaign.economy.last_report.player_delta.get(player).map_or(0.0, |d| d[index]);
        ui.label(egui::RichText::new(format!("Last month {change:+.2}")).color(hud_delta_color(change)))
            .on_hover_text("Net change from the last completed month, including production, trade, consumption, political spending and storage loss. The rows below show domestic sources.");
    }
    if index == 0 {
        let supplied =
            campaign.economy.last_report.food_supply_ratio.get(player).copied().unwrap_or(1.0);
        ui.horizontal(|ui| {
            stat(ui,Icon::Food,&format!("{:.0}%",supplied*100.0),"Last month's proportional supply. Civilian and military requests share the same ratio.");
            stat(ui,Icon::Attack,&format!("{:.1}/mo",campaign.military.food_demand(crate::game::military::ForceOwner::Player(player))),"Current army food demand scales with surviving manpower.");
        });
    }
    if index == 4 {
        ui.horizontal(|ui| {
            stat(
                ui,
                Icon::Eagle,
                &format!(
                    "+{:.1}",
                    campaign.senate_config.rank_income(campaign.actors[player].rank)
                ),
                "Monthly office income. Earlier ranks do not stack.",
            );
            let vassals: f64 = campaign
                .politics
                .iter()
                .filter_map(|p| p.vassal_income(&campaign.diplomacy_config))
                .filter(|(p, _)| *p == player)
                .map(|(_, v)| v)
                .sum();
            stat(
                ui,
                Icon::Control,
                &format!("+{vassals:.1}"),
                "Current monthly vassal Influence, based on Vassal Control.",
            );
        });
    }
    ui.separator();
    egui::ScrollArea::vertical().id_salt(("campaign-resource-ledger",index))
        .max_height((ui.ctx().content_rect().height()*0.45).min(330.0*scale)).show(ui,|ui| {
            egui::Grid::new(("campaign-resource-sources",index)).striped(true)
                .min_col_width(35.0*scale).max_col_width(150.0*scale).show(ui,|ui| {
                    ui.strong("Province");
                    match index {
                        0..=2=>{ui.strong("Output");if index==0 {ui.strong("Use");}},
                        3=>{ui.strong("Tax");},4=>{ui.strong("Domestic");},
                        5=>{ui.strong("People");ui.strong("Capacity");},
                        _=>{for (symbol,label) in [(Icon::Nobles,"Nobles"),(Icon::Citizens,"Citizens"),(Icon::Plebeians,"Plebeians"),(Icon::Slaves,"Slaves")] {icon(ui,symbol,22.0*scale).on_hover_text(label);}},
                    }
                    ui.end_row();
                    for (id,p) in campaign.economy.provinces.iter().enumerate().filter(|(_,p)|p.owner==Some(player)) {
                        ui.add(egui::Label::new(&p.name).truncate()).on_hover_text(&p.name);
                        let report=campaign.economy.last_report.province_reports.get(id);
                        match index {
                            0..=2=>{ui.label(format!("{:.1}",p.production(&campaign.economy.config).1[index]));if index==0 {ui.label(format!("{:.1}",p.food_request(&campaign.economy.config)));}},
                            3=>{ui.label(format!("{:.1}",p.tax_income(&campaign.economy.config)));},
                            4=>{ui.label(format!("{:.2}",report.map_or(0.0,|r|r.influence_income))).on_hover_text("Nobles, buildings and completed wonders in the last completed month.");},
                            5=>{ui.label(format!("{:.1}",p.total_population()));ui.label(format!("{:.1}",p.capacity(&campaign.economy.config)));},
                            _=>{for happiness in p.happiness {ui.label(egui::RichText::new(format!("{happiness:.0}")).color(hud_delta_color(happiness-50.0)));}},
                        }
                        ui.end_row();
                    }
                });
        });
}
