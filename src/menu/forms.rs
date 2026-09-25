//! Menu form cards and clipboard input.

use super::*;

pub(super) fn menu_form(
    ui: &mut egui::Ui,
    id: &str,
    title: &str,
    contents: impl FnOnce(&mut egui::Ui),
) {
    ui.spacing_mut().item_spacing.y = 0.0;
    let height = (logical_content_rect(ui).height() - 128.0).max(160.0);
    ui.set_max_height(height);
    egui::ScrollArea::vertical()
        .id_salt(id)
        .auto_shrink([false, true])
        .max_height((height - MENU_ACTION_HEIGHT - FORM_ACTION_GAP).max(80.0))
        .show(ui, |ui| {
            ui.vertical_centered(|ui| {
                ui.heading(egui::RichText::new(title).size(MENU_TITLE_TEXT_SIZE));
                ui.add_space(FORM_TITLE_GAP);
                ui.scope(|ui| {
                    ui.spacing_mut().item_spacing.y = FORM_CARD_GAP;
                    contents(ui);
                });
            });
        });
    ui.add_space(FORM_ACTION_GAP);
}

pub(super) fn card_frame() -> egui::Frame {
    egui::Frame::new()
        .fill(egui::Color32::from_rgba_unmultiplied(31, 20, 18, 232))
        .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgba_unmultiplied(190, 143, 94, 115)))
        .corner_radius(6.0)
        .inner_margin(egui::Margin::symmetric(18, 12))
}

pub(super) fn code_card_heading(
    ui: &mut egui::Ui,
    label: &str,
    tooltip: &str,
    copy_value: Option<&str>,
) -> bool {
    let mut copied = false;
    ui.horizontal(|ui| {
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if let Some(value) = copy_value {
                copied = copy_icon_button(ui, value, label);
            }
            code_info_icon(ui, tooltip);
            ui.with_layout(egui::Layout::top_down(egui::Align::Min), |ui| {
                ui.label(
                    egui::RichText::new(label.to_uppercase())
                        .size(MENU_CARD_LABEL_TEXT_SIZE)
                        .strong()
                        .color(GOLD),
                );
            });
        });
    });
    copied
}

pub(super) fn copy_icon_button(ui: &mut egui::Ui, value: &str, label: &str) -> bool {
    let (rect, response) = ui.allocate_exact_size(egui::vec2(28.0, 24.0), egui::Sense::click());
    let response = response.on_hover_cursor(egui::CursorIcon::PointingHand);
    response.widget_info(|| {
        egui::WidgetInfo::labeled(egui::WidgetType::Button, true, format!("Copy {label}"))
    });
    let icon_color = if response.is_pointer_button_down_on() {
        GOLD
    } else if response.hovered() || response.has_focus() {
        CREAM
    } else {
        MUTED_TEXT
    };
    let icon = egui::Rect::from_center_size(rect.center(), egui::vec2(16.0, 17.0));
    let back = egui::Rect::from_min_size(icon.min, egui::vec2(11.0, 12.0));
    let front = egui::Rect::from_min_size(icon.min + egui::vec2(5.0, 5.0), egui::vec2(11.0, 12.0));
    let stroke = egui::Stroke::new(1.5, icon_color);
    ui.painter().rect_stroke(back, 1.0, stroke, egui::StrokeKind::Inside);
    ui.painter().rect_stroke(front, 1.0, stroke, egui::StrokeKind::Inside);
    if response.clicked() {
        #[cfg(target_arch = "wasm32")]
        let _ = call_browser_clipboard_function("augustusCopyText", Some(value));
        ui.ctx().copy_text(value.to_owned());
    }
    response.clicked()
}

pub(super) fn code_info_icon(ui: &mut egui::Ui, tooltip: &str) {
    let (rect, response) = ui.allocate_exact_size(egui::vec2(28.0, 24.0), egui::Sense::hover());
    let center = rect.center();
    let color = if response.hovered() {
        CREAM
    } else {
        MUTED_TEXT
    };
    ui.painter().circle_stroke(center, 8.0, egui::Stroke::new(1.5, color));
    ui.painter().text(
        center,
        egui::Align2::CENTER_CENTER,
        "i",
        egui::FontId::proportional(14.0),
        color,
    );
    response.on_hover_ui(|ui| {
        ui.set_max_width(300.0);
        ui.label(egui::RichText::new(tooltip).size(MENU_TOOLTIP_TEXT_SIZE));
    });
}

pub(super) fn form_option_card(
    ui: &mut egui::Ui,
    label: &str,
    tooltip: &str,
    contents: impl FnOnce(&mut egui::Ui),
) {
    let frame = card_frame();
    let inner_width = (ui.available_width() - frame.total_margin().sum().x).max(1.0);
    frame.show(ui, |ui| {
        ui.set_width(inner_width);
        ui.spacing_mut().item_spacing.y = 0.0;
        code_card_heading(ui, label, tooltip, None);
        ui.add_space(FORM_CARD_GAP);
        contents(ui);
    });
}

pub(super) fn editable_form_card(
    ui: &mut egui::Ui,
    label: &str,
    value: &mut String,
    hint: &str,
    tooltip: &str,
    char_limit: Option<usize>,
) {
    form_option_card(ui, label, tooltip, |ui| {
        let mut editor = egui::TextEdit::singleline(value)
            .id_salt(label)
            .horizontal_align(egui::Align::Center)
            .vertical_align(egui::Align::Center)
            .font(egui::FontId::proportional(MENU_CONTROL_TEXT_SIZE))
            .hint_text(egui::RichText::new(hint).size(MENU_CONTROL_TEXT_SIZE))
            .margin(egui::vec2(12.0, 6.0));
        if let Some(limit) = char_limit {
            editor = editor.char_limit(limit);
        }
        let response = ui.add_sized(egui::vec2(ui.available_width(), MENU_CONTROL_HEIGHT), editor);
        response.context_menu(|ui| {
            if ui.button("Paste").on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                request_menu_paste(ui, response.id);
                ui.close();
            }
        });
    });
}

pub(super) fn menu_submit_pressed(ui: &egui::Ui) -> bool {
    ui.input_mut(|input| {
        let mut pressed = false;
        input.events.retain(|event| {
            if let egui::Event::Key {
                key: egui::Key::Enter,
                pressed: true,
                repeat,
                modifiers,
                ..
            } = event
            {
                if modifiers.is_none() {
                    pressed |= !repeat;
                    return false;
                }
            }
            true
        });
        pressed
    })
}

pub(super) fn menu_paste_target_id() -> egui::Id {
    egui::Id::new("augustus_menu_paste_target")
}

pub(super) fn request_menu_paste(ui: &egui::Ui, target: egui::Id) {
    #[cfg(target_arch = "wasm32")]
    if !call_browser_clipboard_function("augustusRequestPaste", None)
        .and_then(|result| result.as_bool())
        .unwrap_or(false)
    {
        return;
    }
    ui.ctx().data_mut(|data| data.insert_temp(menu_paste_target_id(), target));
}

pub(super) fn apply_pending_menu_paste(context: &egui::Context, _clipboard: &mut EguiClipboard) {
    let Some(target) = context.data(|data| data.get_temp::<egui::Id>(menu_paste_target_id()))
    else {
        return;
    };

    #[cfg(not(target_arch = "wasm32"))]
    let result = Some(_clipboard.get_text().ok_or(()));
    #[cfg(target_arch = "wasm32")]
    let result = take_browser_paste_result();

    let Some(result) = result else {
        return;
    };
    context.data_mut(|data| data.remove::<egui::Id>(menu_paste_target_id()));
    let Ok(text) = result else {
        return;
    };
    context.memory_mut(|memory| memory.request_focus(target));
    context.input_mut(|input| input.events.push(egui::Event::Paste(text)));
}

#[cfg(target_arch = "wasm32")]
pub(super) fn take_browser_paste_result() -> Option<Result<String, ()>> {
    let result = call_browser_clipboard_function("augustusTakePaste", None)?;
    if let Some(text) = result.as_string() {
        Some(Ok(text))
    } else if result.as_bool() == Some(false) {
        Some(Err(()))
    } else {
        None
    }
}

#[cfg(target_arch = "wasm32")]
pub(super) fn call_browser_clipboard_function(
    name: &str,
    argument: Option<&str>,
) -> Option<wasm_bindgen::JsValue> {
    use wasm_bindgen::{JsCast, JsValue};
    let window = web_sys::window()?;
    let function = js_sys::Reflect::get(window.as_ref(), &JsValue::from_str(name))
        .ok()?
        .dyn_into::<js_sys::Function>()
        .ok()?;
    match argument {
        Some(argument) => function.call1(window.as_ref(), &JsValue::from_str(argument)).ok(),
        None => function.call0(window.as_ref()).ok(),
    }
}
