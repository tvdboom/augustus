//! Map resource strip and detail panels.

use super::*;

const POPULATION_HUD_ICON_SIZE: f32 = 26.0;
const RESOURCE_HUD_ICON_SIZE: f32 = 38.0;

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
    if value == 0.0 {
        return "0".into();
    }
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
    let number = format_hud_number(value);
    if number == "0" {
        number
    } else if value > 0.0 {
        format!("+{number}")
    } else {
        number
    }
}

/// Food consumption uses whole units, with one minus sign and unsigned zero.
pub(in crate::app) fn format_food_demand(amount: f64) -> String {
    let magnitude = amount.abs().floor();
    if magnitude == 0.0 {
        "0".to_owned()
    } else {
        format!("−{magnitude:.0}")
    }
}

/// Civilian and soldier counts always round down, without fractional abbreviations.
pub(in crate::app) fn format_population(value: f64) -> String {
    let value = value.floor();
    if value == 0.0 {
        "0".into()
    } else {
        format!("{value:.0}")
    }
}

pub(in crate::app) fn format_population_delta(value: f64) -> String {
    let value = value.floor();
    if value == 0.0 {
        "0".into()
    } else {
        format!("{value:+.0}")
    }
}

pub(in crate::app) fn paint_hud_resources(
    painter: &egui::Painter,
    origin: egui::Pos2,
    scale: f32,
    date_left: f32,
    resources: &[HudResource; 7],
    storage: Option<&[f64; 3]>,
    icons: &[egui::TextureHandle; 7],
) {
    let p = |x: f32, y: f32| origin + egui::vec2(x, y) * scale;
    let second = MAP_RESOURCE_STRIP_LEFT + HUD_FIRST_GROUP_WIDTH;
    let third = second + HUD_SECOND_GROUP_WIDTH;
    let positions = hud_resource_positions();
    for index in [4, 3, 0, 1, 2, 5] {
        let resource = &resources[index];
        let x = positions[index];
        let width = hud_resource_width(index);
        if x + width > date_left - HUD_RESOURCE_GROUP_PADDING {
            break;
        }
        let capacity = storage.and_then(|limits| limits.get(index));
        let at_capacity = capacity.is_some_and(|limit| resource.amount >= *limit);
        let ink = hud_delta_color(0.0);
        let amount_color = if at_capacity {
            hud_delta_color(-1.0)
        } else {
            ink
        };
        let amount = capacity.map_or_else(
            || format_hud_number(resource.amount),
            |capacity| {
                format!("{} / {}", format_hud_number(resource.amount), format_hud_number(*capacity))
            },
        );
        // Full storage cannot gain more stock, but consumption can still drain it.
        let monthly_delta = if at_capacity {
            resource.monthly_delta.min(0.0)
        } else {
            resource.monthly_delta
        };
        let delta = format_hud_delta(monthly_delta);
        let gap = 4.0 * scale;
        let icon_size = if index == 5 {
            POPULATION_HUD_ICON_SIZE
        } else {
            RESOURCE_HUD_ICON_SIZE
        } * scale;
        let max_text_width = (width - 16.0) * scale - gap - icon_size;
        let fit = |mut job: egui::text::LayoutJob| loop {
            let galley = painter.layout_job(job.clone());
            if galley.size().x <= max_text_width {
                break galley;
            }
            let shrink = 0.98 * max_text_width / galley.size().x;
            for section in &mut job.sections {
                section.format.font_id.size *= shrink;
            }
        };
        let delta_color = hud_delta_color(if index == 5 {
            monthly_delta.floor()
        } else {
            monthly_delta
        });
        let amount_job = egui::text::LayoutJob::simple(
            amount,
            egui::FontId::proportional(19.0 * scale),
            amount_color,
            f32::INFINITY,
        );
        let amount_galley = if index < 3 {
            // Wider stock slots retain the same type size as the civic currencies.
            painter.layout_job(amount_job)
        } else {
            fit(amount_job)
        };
        let delta_galley = fit(egui::text::LayoutJob::simple(
            delta,
            egui::FontId::proportional(14.5 * scale),
            delta_color,
            f32::INFINITY,
        ));
        let text_width = delta_galley.size().x.max(amount_galley.size().x);
        let icon_left = p(x + width * 0.5, 0.0).x - (icon_size + gap + text_width) * 0.5;
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

pub(in crate::app) fn hud_resource_width(index: usize) -> f32 {
    if index < 3 {
        HUD_STOCK_RESOURCE_WIDTH
    } else {
        HUD_RESOURCE_WIDTH
    }
}

pub(in crate::app) fn hud_resource_positions() -> [f32; 6] {
    let first = MAP_RESOURCE_STRIP_LEFT;
    let second = first + HUD_FIRST_GROUP_WIDTH;
    let third = second + HUD_SECOND_GROUP_WIDTH;
    [
        second + HUD_RESOURCE_GROUP_PADDING,
        second + HUD_RESOURCE_GROUP_PADDING + HUD_STOCK_RESOURCE_WIDTH,
        second + HUD_RESOURCE_GROUP_PADDING + HUD_STOCK_RESOURCE_WIDTH * 2.0,
        first + HUD_RESOURCE_GROUP_PADDING + HUD_RESOURCE_WIDTH,
        first + HUD_RESOURCE_GROUP_PADDING,
        third + HUD_RESOURCE_GROUP_PADDING,
    ]
}

#[derive(Default)]
pub(in crate::app) struct HudHoverState {
    pub resource: Option<usize>,
    pub food_row: Option<usize>,
    pub coin: bool,
    pub influence: bool,
    pub population: bool,
    pub population_class: Option<usize>,
}

pub(in crate::app) fn draw_map_resources(
    mut contexts: EguiContexts,
    mut textures: Local<Option<[egui::TextureHandle; 7]>>,
    mut pop_textures: Local<Option<[egui::TextureHandle; 4]>>,
    mut hover: Local<HudHoverState>,
    resources: Res<HudResources>,
    game: Res<ActiveGame>,
    practice: Res<LocalPractice>,
    ownership: Res<ProvinceOwnership>,
    campaign: Res<campaign::Campaign>,
    terminal: Res<TerminalPresentation>,
) {
    if terminal.spectating {
        return;
    }
    let Ok(context) = contexts.ctx_mut() else {
        return;
    };
    let screen = context.content_rect();
    let scale = viewport_ui_scale(screen.size());
    if !map_resource_strip_visible(screen, scale) {
        *hover = HudHoverState::default();
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
    let storage = (*game == ActiveGame::LocalPractice && campaign.active)
        .then(|| campaign.economy.players.get(player).map(|wallet| &wallet.storage))
        .flatten();
    paint_hud_resources(&painter, screen.min, scale, date_left, &displayed, storage, icons);
    if *game == ActiveGame::LocalPractice {
        let pop_icons = pop_textures.get_or_insert_with(|| {
            std::array::from_fn(|index| load_pop_class_icon(context, index))
        });
        show_hud_hover_cards(
            context,
            screen,
            scale,
            date_left,
            player,
            &ownership,
            campaign.active.then_some(&*campaign),
            displayed[5],
            icons,
            pop_icons,
            &mut hover,
        );
    } else {
        *hover = HudHoverState::default();
    }
}

/// Use the original illustrated cards with campaign state projected into map ownership.
pub(in crate::app) fn show_hud_hover_cards(
    context: &egui::Context,
    screen: egui::Rect,
    scale: f32,
    date_left: f32,
    player: usize,
    ownership: &ProvinceOwnership,
    campaign: Option<&campaign::Campaign>,
    population: HudResource,
    icons: &[egui::TextureHandle; 7],
    pop_icons: &[egui::TextureHandle; 4],
    hover: &mut HudHoverState,
) {
    resource_panel::show(
        context,
        screen,
        scale,
        date_left,
        player,
        ownership,
        campaign,
        icons,
        &mut hover.resource,
        &mut hover.food_row,
    );
    coin_panel::show(
        context,
        screen,
        scale,
        date_left,
        player,
        ownership,
        campaign,
        &icons[3],
        &mut hover.coin,
    );
    influence_panel::show(
        context,
        screen,
        scale,
        date_left,
        player,
        ownership,
        campaign,
        &icons[4],
        &mut hover.influence,
    );
    population_panel::show(
        context,
        screen,
        scale,
        date_left,
        player,
        ownership,
        population,
        &icons[5],
        pop_icons,
        &mut hover.population,
        &mut hover.population_class,
    );
}
