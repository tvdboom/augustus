//! Exercise scrolling and the actual Politics choices in the nationwide panel.
use super::*;
use egui::epaint::text::{FontInsert, FontPriority, InsertFontFamily};

#[test]
fn politics_policies_are_reachable_and_change_only_the_selected_edict() {
    for (size, scale) in [(egui::vec2(1600.0, 900.0), 1.0), (egui::vec2(960.0, 700.0), 0.85)] {
        let ctx = egui::Context::default();
        ctx.set_global_style(super::super::campaign_widgets::map_style(scale));
        ctx.add_font(FontInsert::new(
            "firasans",
            egui::FontData::from_static(include_bytes!("../../assets/fonts/FiraSans-Bold.ttf")),
            vec![InsertFontFamily {
                family: egui::FontFamily::Proportional,
                priority: FontPriority::Highest,
            }],
        ));
        let icons = load_effect_icons(&ctx);
        let mut governance = Governance::default();
        let mut selected = 0;
        let mut time = 0.0;
        let mut capture = crate::egui_capture::Capture::default();
        let mut render = |events| {
            time += 0.1;
            let mut changed = false;
            let mut output = ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, size)),
                    time: Some(time),
                    events,
                    ..Default::default()
                },
                |_| {
                    (changed, _, _) = show_overview(
                        &ctx,
                        scale,
                        &icons[0],
                        &icons,
                        egui::Color32::from_rgb(50, 126, 185),
                        &mut governance,
                        false,
                        &mut selected,
                        |_, _| None,
                    );
                },
            );
            capture.frame(&ctx, &output, &format!("vassal-policies-{}", size.x as u32));
            output.textures_delta.clear();
            (output, changed, governance)
        };
        render(vec![]);
        let (first, _, _) = render(vec![]);
        let panel = first
            .shapes
            .iter()
            .find_map(|shape| match &shape.shape {
                egui::Shape::Rect(rect) if rect.fill == egui::Color32::from_rgb(238, 233, 219) => {
                    Some(rect.rect)
                },
                _ => None,
            })
            .unwrap();
        render(vec![
            egui::Event::PointerMoved(panel.center()),
            egui::Event::MouseWheel {
                phase: egui::TouchPhase::Move,
                unit: egui::MouseWheelUnit::Point,
                delta: egui::vec2(0.0, -1500.0),
                modifiers: egui::Modifiers::NONE,
            },
        ]);
        let mut output = render(vec![]).0;
        for _ in 0..12 {
            output = render(vec![]).0;
        }
        let visible = |label: &str| {
            output.shapes.iter().any(|shape| match &shape.shape {
                egui::Shape::Text(text) => {
                    text.galley.job.text == label
                        && shape
                            .clip_rect
                            .contains_rect(egui::Rect::from_min_size(text.pos, text.galley.size()))
                },
                _ => false,
            })
        };
        for label in ["POLITICS", "Vassal Tribute", "Vassal Autonomy", "20%", "0/mo"] {
            assert!(
                visible(label),
                "Politics must be visible after scrolling at {size:?}: {label}"
            );
        }
        // The Politics choices are the last two rows. Select Low tribute and High autonomy.
        for (label, policy) in [("Low", 0), ("High", 1)] {
            let mut choices: Vec<_> = output
                .shapes
                .iter()
                .filter_map(|shape| match &shape.shape {
                    egui::Shape::Text(text) if text.galley.job.text == label => {
                        Some(text.pos + text.galley.size() * 0.5)
                    },
                    _ => None,
                })
                .collect();
            choices.sort_by(|a, b| a.y.total_cmp(&b.y));
            let position = choices[choices.len() - 2 + policy];
            for pressed in [true, false] {
                let (next, changed, current) = render(vec![
                    egui::Event::PointerMoved(position),
                    egui::Event::PointerButton {
                        pos: position,
                        button: egui::PointerButton::Primary,
                        pressed,
                        modifiers: egui::Modifiers::NONE,
                    },
                ]);
                output = next;
                if !pressed {
                    assert!(changed, "The {label} Politics choice must emit a governance change");
                    assert_eq!(current.vassal_tribute, Tribute::Low);
                    assert_eq!(
                        current.vassal_autonomy,
                        if policy == 1 {
                            VassalAutonomy::High
                        } else {
                            VassalAutonomy::Normal
                        }
                    );
                    assert_eq!(current.food_rations, crate::map::EdictLevel::Medium);
                    assert_eq!(current.slave_labor, crate::map::EdictLevel::Medium);
                    assert_eq!(current.noble_wages, crate::map::EdictLevel::Medium);
                    assert_eq!(current.army_wages, crate::map::EdictLevel::Medium);
                }
            }
        }
        render(vec![]);
    }
}
