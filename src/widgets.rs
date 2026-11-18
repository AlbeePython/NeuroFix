use eframe::egui::{
    self, Align2, Color32, CursorIcon, FontId, Rect, Response, RichText, Sense, Ui, Vec2,
};

use crate::icons::{self, Icon};
use crate::theme;

pub fn mix(a: Color32, b: Color32, t: f32) -> Color32 {
    let t = t.clamp(0.0, 1.0);
    let f = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * t).round() as u8;
    Color32::from_rgba_premultiplied(
        f(a.r(), b.r()),
        f(a.g(), b.g()),
        f(a.b(), b.b()),
        f(a.a(), b.a()),
    )
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum PillKind {
    Filled,
    Tonal,
    Outlined,
    Text,
    Danger,
}

pub fn button(
    ui: &mut Ui,
    label: &str,
    icon: Option<Icon>,
    kind: PillKind,
    enabled: bool,
) -> Response {
    let font = FontId::new(14.0, theme::bold());
    let galley = ui
        .painter()
        .layout_no_wrap(label.to_owned(), font.clone(), theme::ON_PRIMARY);
    let text_width = galley.size().x;
    let icon_space = if icon.is_some() { 26.0 } else { 0.0 };
    let width = text_width + icon_space + 40.0;
    let height = 40.0;
    let (rect, response) = ui.allocate_exact_size(Vec2::new(width, height), Sense::click());

    let hovered = enabled && response.hovered();
    let pressed = enabled && response.is_pointer_button_down_on();

    let (base, fg) = match kind {
        PillKind::Filled => (theme::PRIMARY, theme::ON_PRIMARY),
        PillKind::Tonal => (theme::SECONDARY_CONTAINER, theme::ON_SECONDARY_CONTAINER),
        PillKind::Outlined => (Color32::TRANSPARENT, theme::PRIMARY),
        PillKind::Text => (Color32::TRANSPARENT, theme::PRIMARY),
        PillKind::Danger => (Color32::TRANSPARENT, theme::ERROR),
    };

    let (base, fg) = if enabled {
        (base, fg)
    } else {
        (
            mix(base, theme::SURFACE, 0.55),
            mix(fg, theme::SURFACE, 0.62),
        )
    };

    let painter = ui.painter();
    if base.a() > 0 {
        let fill = if pressed {
            mix(base, fg, 0.16)
        } else if hovered {
            mix(base, fg, 0.08)
        } else {
            base
        };
        painter.rect_filled(rect, 20.0, fill);
    } else if hovered {
        painter.rect_filled(
            rect,
            20.0,
            Color32::from_rgba_unmultiplied(fg.r(), fg.g(), fg.b(), 26),
        );
    }

    if kind == PillKind::Outlined {
        painter.rect_stroke(
            rect,
            20.0,
            egui::Stroke::new(1.0_f32, theme::OUTLINE_VARIANT),
        );
    }

    let content_left = rect.center().x - (text_width + icon_space) * 0.5;
    if let Some(icon) = icon {
        let icon_rect = Rect::from_center_size(
            egui::pos2(content_left + 9.0, rect.center().y),
            Vec2::splat(18.0),
        );
        icons::paint(painter, icon_rect, icon, fg, 1.9);
    }
    painter.text(
        egui::pos2(content_left + icon_space, rect.center().y),
        Align2::LEFT_CENTER,
        label,
        font,
        fg,
    );

    if enabled {
        response.on_hover_cursor(CursorIcon::PointingHand)
    } else {
        response
    }
}

pub fn chip(ui: &mut Ui, label: &str, bg: Color32, fg: Color32) -> Response {
    let font = FontId::new(11.5, theme::bold());
    let galley = ui
        .painter()
        .layout_no_wrap(label.to_owned(), font.clone(), fg);
    let width = galley.size().x + 20.0;
    let height = 24.0;
    let (rect, response) = ui.allocate_exact_size(Vec2::new(width, height), Sense::hover());
    ui.painter().rect_filled(rect, 12.0, bg);
    ui.painter()
        .text(rect.center(), Align2::CENTER_CENTER, label, font, fg);
    response
}

pub fn card<R>(ui: &mut Ui, fill: Color32, padding: f32, add: impl FnOnce(&mut Ui) -> R) -> R {
    egui::Frame::none()
        .fill(fill)
        .rounding(theme::RADIUS_L)
        .inner_margin(egui::Margin::same(padding))
        .show(ui, add)
        .inner
}

pub fn nav_item(ui: &mut Ui, icon: Icon, label: &str, active: bool, enabled: bool) -> Response {
    let width = ui.available_width();
    let (rect, response) = ui.allocate_exact_size(Vec2::new(width, 54.0), Sense::click());
    let hovered = enabled && response.hovered();

    let fg = if !enabled {
        mix(theme::ON_SURFACE, theme::SURFACE, 0.55)
    } else if active {
        theme::ON_SECONDARY_CONTAINER
    } else {
        theme::ON_SURFACE_VARIANT
    };

    if active {
        ui.painter()
            .rect_filled(rect, 27.0, theme::SECONDARY_CONTAINER);
    } else if hovered {
        ui.painter().rect_filled(
            rect,
            27.0,
            Color32::from_rgba_unmultiplied(
                theme::ON_SURFACE.r(),
                theme::ON_SURFACE.g(),
                theme::ON_SURFACE.b(),
                18,
            ),
        );
    }

    let icon_rect = Rect::from_center_size(
        egui::pos2(rect.left() + 27.0, rect.center().y),
        Vec2::splat(21.0),
    );
    icons::paint(ui.painter(), icon_rect, icon, fg, 1.8);

    ui.painter().text(
        egui::pos2(rect.left() + 52.0, rect.center().y),
        Align2::LEFT_CENTER,
        label,
        FontId::new(15.0, theme::bold()),
        fg,
    );

    if enabled {
        response.on_hover_cursor(CursorIcon::PointingHand)
    } else {
        response
    }
}

pub fn search_field(ui: &mut Ui, text: &mut String, hint: &str) -> Response {
    egui::Frame::none()
        .fill(theme::SURFACE_CONTAINER)
        .rounding(26.0)
        .inner_margin(egui::Margin::symmetric(20.0, 0.0))
        .show(ui, |ui| {
            ui.set_height(52.0);
            ui.horizontal_centered(|ui| {
                let (icon_rect, _) = ui.allocate_exact_size(Vec2::splat(19.0), Sense::hover());
                icons::paint(
                    ui.painter(),
                    icon_rect,
                    Icon::Search,
                    theme::ON_SURFACE_VARIANT,
                    1.9,
                );
                ui.add_space(8.0);
                ui.add(
                    egui::TextEdit::singleline(text)
                        .frame(false)
                        .text_color(theme::ON_SURFACE)
                        .hint_text(
                            RichText::new(hint)
                                .color(theme::ON_SURFACE_VARIANT)
                                .font(FontId::new(14.5, egui::FontFamily::Proportional)),
                        )
                        .font(FontId::new(14.5, egui::FontFamily::Proportional))
                        .desired_width(ui.available_width()),
                )
            })
            .inner
        })
        .inner
}

pub fn switch(ui: &mut Ui, value: &mut bool) -> Response {
    let (rect, mut response) = ui.allocate_exact_size(Vec2::new(52.0, 32.0), Sense::click());
    if response.clicked() {
        *value = !*value;
        response.mark_changed();
    }

    let t = ui.ctx().animate_bool_with_time(response.id, *value, 0.16);
    let track = mix(theme::SURFACE_HIGHEST, theme::PRIMARY, t);
    let border = if *value {
        theme::PRIMARY
    } else {
        theme::OUTLINE
    };
    ui.painter().rect_filled(rect, 16.0, track);
    ui.painter()
        .rect_stroke(rect, 16.0, egui::Stroke::new(1.5_f32, border));

    let radius = 10.0;
    let cx = rect.left() + 16.0 + (rect.width() - 32.0) * t;
    ui.painter().circle_filled(
        egui::pos2(cx, rect.center().y),
        radius,
        if *value {
            theme::ON_PRIMARY
        } else {
            theme::OUTLINE
        },
    );

    response.on_hover_cursor(CursorIcon::PointingHand)
}

pub fn status_dot(ui: &mut Ui, color: Color32, size: f32) {
    let (rect, _) = ui.allocate_exact_size(Vec2::splat(size), Sense::hover());
    ui.painter()
        .circle_filled(rect.center(), size * 0.32, color);
}
