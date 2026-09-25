//! Map resource strip and detail panels.

use super::*;

pub(super) fn load_hud_resource_icon(ctx: &egui::Context, index: usize) -> egui::TextureHandle {
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

pub(super) fn load_pop_class_icon(ctx: &egui::Context, index: usize) -> egui::TextureHandle {
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

pub(super) fn hud_delta_color(delta: f64) -> egui::Color32 {
    if delta > 0.0 {
        egui::Color32::from_rgb(43, 133, 67)
    } else if delta < 0.0 {
        egui::Color32::from_rgb(174, 54, 45)
    } else {
        egui::Color32::from_rgb(35, 35, 32)
    }
}

pub(super) fn format_hud_number(value: f64) -> String {
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

pub(super) fn format_hud_delta(value: f64) -> String {
    if value >= 0.0 {
        format!("+{}", format_hud_number(value))
    } else {
        format_hud_number(value)
    }
}

pub(super) fn paint_hud_resources(
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

pub(super) fn hud_resource_positions() -> [f32; 7] {
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

pub(super) fn draw_map_resources(
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
    if *game == ActiveGame::LocalPractice {
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
