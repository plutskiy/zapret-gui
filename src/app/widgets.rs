use super::*;

// ---------------------------------------------------------------- widgets local to the app

pub(super) struct RowOut {
    pub(super) toggled: bool,
    pub(super) play: bool,
}

pub(super) fn config_row(
    ui: &mut Ui,
    v: &RowView,
    checkbox_enabled: bool,
    play_enabled: bool,
) -> RowOut {
    let w = ui.available_width();
    let (rect, resp) = ui.allocate_exact_size(vec2(w, 46.0), Sense::click());
    let p = ui.painter().clone();
    if v.active || v.selected {
        p.rect_filled(rect, cr(10.0), TEAL_TINT);
    } else if resp.hovered() {
        p.rect_filled(rect, cr(10.0), HOVER);
    }
    let cy = rect.center().y;
    paint_checkbox(
        ui,
        pos2(rect.left() + 10.0 + 8.0, cy),
        v.checked,
        checkbox_enabled,
    );

    let play_rect = Rect::from_center_size(pos2(rect.right() - 8.0 - 15.0, cy), vec2(30.0, 30.0));
    let play_resp = icon_button_at(
        ui,
        play_rect,
        Icon::Play,
        0.55,
        play_enabled,
        ui.scope_id().with(("play", &v.name)),
    );
    let mut x = play_rect.left() - 12.0;
    if let Some((text, tone)) = &v.result {
        let bw = badge_width(ui, text, true);
        paint_badge(
            ui,
            Rect::from_min_size(pos2(x - bw, cy - 8.5), vec2(bw, 17.0)),
            text,
            *tone,
            true,
        );
        x -= bw + 12.0;
    }
    if v.best {
        let bw = badge_width(ui, "★ лучшая", false);
        paint_badge(
            ui,
            Rect::from_min_size(pos2(x - bw, cy - 8.5), vec2(bw, 17.0)),
            "★ лучшая",
            Tone::Accent,
            false,
        );
        x -= bw + 12.0;
    }
    if v.spinner {
        spinner(ui, pos2(x - 7.0 - 8.0, cy), 14.0);
        x -= 14.0 + 8.0 + 12.0;
    }
    let left = rect.left() + 10.0 + 16.0 + 12.0;
    let (f, c) = if v.active || v.selected {
        (font(14.0, W::Semi), ACCENT)
    } else {
        (font(14.0, W::Med), TEXT)
    };
    text_trunc(&p, pos2(left, cy), &v.name, f, c, x - left);

    let play = play_resp.clicked();
    RowOut {
        toggled: resp
            .on_hover_cursor(egui::CursorIcon::PointingHand)
            .clicked()
            && checkbox_enabled
            && !play,
        play,
    }
}

pub(super) fn icon_button_at(
    ui: &mut Ui,
    rect: Rect,
    icon: Icon,
    opacity: f32,
    enabled: bool,
    id: egui::Id,
) -> egui::Response {
    let resp = ui.interact(
        rect,
        id,
        if enabled {
            Sense::click()
        } else {
            Sense::hover()
        },
    );
    let hover = enabled && resp.hovered();
    let painter = ui.painter();
    if resp.is_pointer_button_down_on() && enabled {
        painter.circle_filled(rect.center(), rect.width() / 2.0, CARD);
    } else if hover {
        painter.circle_filled(rect.center(), rect.width() / 2.0, HOVER);
    }
    let color = if hover { TEXT } else { TEXT2 };
    let icon_color = if enabled {
        color.gamma_multiply(if hover { 1.0 } else { opacity })
    } else {
        color.gamma_multiply(0.4)
    };
    icon_at(ui, rect.center(), 17.0, icon, icon_color);
    if enabled {
        resp.on_hover_cursor(egui::CursorIcon::PointingHand)
    } else {
        resp
    }
}

pub(super) fn paint_badge(ui: &Ui, rect: Rect, text: &str, tone: Tone, mono: bool) {
    let (bg, fg, border) = match tone {
        Tone::Neutral => (INK_700, TEXT2, LINE),
        Tone::Accent => (TEAL_TINT, ACCENT_HOVER, Color32::TRANSPARENT),
        Tone::Success => (SUCCESS.gamma_multiply(0.16), SUCCESS, Color32::TRANSPARENT),
        Tone::Warning => (WARNING.gamma_multiply(0.16), WARNING, Color32::TRANSPARENT),
        Tone::Danger => (DANGER.gamma_multiply(0.16), DANGER, Color32::TRANSPARENT),
    };
    let font = font(11.0, if mono { W::Mono500 } else { W::Semi });
    let painter = ui.painter();
    painter.rect_filled(rect, cr(6.0), bg);
    if border != Color32::TRANSPARENT {
        painter.rect_stroke(
            rect,
            cr(6.0),
            Stroke::new(1.0, border),
            egui::StrokeKind::Inside,
        );
    }
    painter.text(rect.center(), Align2::CENTER_CENTER, text, font, fg);
}

pub(super) fn nav_item(ui: &mut Ui, icon: Icon, label: &str, active: bool) -> egui::Response {
    let (rect, resp) = ui.allocate_exact_size(vec2(ui.available_width(), 40.0), Sense::click());
    let p = ui.painter().clone();
    if active {
        p.rect_filled(rect, cr(10.0), TEAL_TINT);
        p.rect_filled(
            Rect::from_min_max(
                pos2(rect.left(), rect.top() + 10.0),
                pos2(rect.left() + 3.0, rect.bottom() - 10.0),
            ),
            cr(2.0),
            ACCENT,
        );
    } else if resp.hovered() {
        p.rect_filled(rect, cr(10.0), HOVER);
    }
    let color = if active { ACCENT } else { TEXT2 };
    icon_at(
        ui,
        pos2(rect.left() + 14.0 + 9.0, rect.center().y),
        18.0,
        icon,
        color,
    );
    p.text(
        pos2(rect.left() + 14.0 + 18.0 + 12.0, rect.center().y),
        Align2::LEFT_CENTER,
        label,
        font(14.0, W::Med),
        color,
    );
    resp.on_hover_cursor(egui::CursorIcon::PointingHand)
}

pub(super) fn field(ui: &mut Ui, label: &str, width: f32, add: impl FnOnce(&mut Ui, f32)) {
    ui.allocate_ui_with_layout(vec2(width, 0.0), Layout::top_down(Align::Min), |ui| {
        ui.spacing_mut().item_spacing.y = 8.0;
        overline(ui, label);
        add(ui, width);
    });
}

pub(super) fn slider_row(
    ui: &mut Ui,
    label: &str,
    value: &mut u32,
    min: u32,
    max: u32,
    slider_w: f32,
) -> bool {
    let w = ui.available_width();
    let label_w = text_w(ui, label, &font(14.0, W::Med));
    let inline_w = w - label_w - 44.0 - 16.0 - 16.0;
    let stacked = inline_w < 80.0;
    let slider_w = if stacked {
        (w - 44.0 - 16.0).max(60.0)
    } else {
        slider_w.min(inline_w)
    };
    let (rect, _) =
        ui.allocate_exact_size(vec2(w, if stacked { 72.0 } else { 46.0 }), Sense::hover());
    ui.painter().rect_filled(
        Rect::from_min_size(pos2(rect.left(), rect.bottom() - 1.0), vec2(w, 1.0)),
        0,
        LINE_SOFT,
    );
    let cy = if stacked {
        rect.top() + 18.0
    } else {
        rect.center().y - 0.5
    };
    let sy = if stacked { rect.top() + 48.0 } else { cy };
    ui.painter().text(
        pos2(rect.left(), cy),
        Align2::LEFT_CENTER,
        label,
        font(14.0, W::Med),
        TEXT,
    );
    ui.painter().text(
        pos2(rect.right(), cy),
        Align2::RIGHT_CENTER,
        value.to_string(),
        font(13.0, W::Mono500),
        TEXT,
    );
    let sx = if stacked {
        rect.left()
    } else {
        rect.right() - 44.0 - 16.0 - slider_w
    };
    let sr = Rect::from_min_size(pos2(sx, sy - 6.5), vec2(slider_w, 13.0));
    ui.scope_builder(UiBuilder::new().max_rect(sr), |ui| {
        slider(ui, value, min, max, slider_w)
    })
    .inner
}

pub(super) fn status_line(ui: &mut Ui, icon: Option<Icon>, text: &str, color: Color32) {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 8.0;
        if let Some(ic) = icon {
            let (r, _) = ui.allocate_exact_size(vec2(16.0, 16.0), Sense::hover());
            icon_at(ui, r.center(), 16.0, ic, color);
        }
        ui.add(egui::Label::new(rt(text, 13.0, W::Med, color)).selectable(false));
    });
}

pub(super) fn status_line_mono(ui: &mut Ui, icon: Option<Icon>, text: &str, color: Color32) {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 8.0;
        if let Some(ic) = icon {
            let (r, _) = ui.allocate_exact_size(vec2(14.0, 14.0), Sense::hover());
            icon_at(ui, r.center(), 14.0, ic, color);
        }
        ui.add(
            egui::Label::new(rt(text, 12.0, W::Mono, color))
                .selectable(false)
                .wrap_mode(egui::TextWrapMode::Wrap),
        );
    });
}

pub(super) fn banner_row(ui: &mut Ui, text: &str, add_right: impl FnOnce(&mut Ui)) {
    egui::Frame::new()
        .fill(rgba(251, 191, 36, 26))
        .stroke(Stroke::new(1.0, rgba(251, 191, 36, 61)))
        .corner_radius(12)
        .inner_margin(Margin::symmetric(14, 10))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal(|ui| {
                ui.add(egui::Label::new(rt(text, 13.0, W::Med, WARNING)).selectable(false));
                ui.with_layout(Layout::right_to_left(Align::Center), add_right);
            });
        });
}

pub(super) fn two_col<T>(
    ui: &mut Ui,
    wide: bool,
    ctx: &mut T,
    left: impl FnOnce(&mut T, &mut Ui),
    right: impl FnOnce(&mut T, &mut Ui),
) {
    if wide && ui.available_width() >= TWO_COL_MIN {
        let w = (ui.available_width() - 16.0) / 2.0;
        ui.horizontal_top(|ui| {
            ui.spacing_mut().item_spacing.x = 16.0;
            ui.allocate_ui_with_layout(vec2(w, 0.0), Layout::top_down(Align::Min), |ui| {
                left(ctx, ui)
            });
            ui.allocate_ui_with_layout(vec2(w, 0.0), Layout::top_down(Align::Min), |ui| {
                right(ctx, ui)
            });
        });
    } else {
        ui.spacing_mut().item_spacing.y = 16.0;
        left(ctx, ui);
        right(ctx, ui);
    }
}

pub(super) fn check_color(c: Check) -> Color32 {
    match c {
        Check::Ok => SUCCESS,
        Check::Error | Check::Ssl => DANGER,
    }
}

pub(super) fn report_details(ui: &mut Ui, r: &Report) {
    let standard = r.outcomes.iter().any(|o| matches!(o, Outcome::Std(_)));
    if standard {
        egui::Grid::new(("grid", &r.config))
            .striped(true)
            .show(ui, |ui| {
                for o in &r.outcomes {
                    let Outcome::Std(s) = o else { continue };
                    ui.label(rt(&s.name, 12.0, W::Reg, TEXT2));
                    for (label, c) in &s.checks {
                        ui.label(
                            RichText::new(format!("{label}:{}", c.label()))
                                .color(check_color(*c))
                                .font(font(12.0, W::Mono)),
                        );
                    }
                    if s.checks.is_empty() {
                        for _ in 0..3 {
                            ui.label("");
                        }
                    }
                    match s.ping {
                        Some(ms) => ui.label(rt(format!("{ms} мс"), 12.0, W::Mono, SUCCESS)),
                        None => ui.label(rt("timeout", 12.0, W::Mono, WARNING)),
                    };
                    ui.end_row();
                }
            });
        return;
    }
    let mut good = 0;
    for o in &r.outcomes {
        let Outcome::Dpi(d) = o else { continue };
        if d.lines.iter().all(|l| l.status == DpiStatus::Ok) {
            good += 1;
            continue;
        }
        let parts: Vec<String> = d
            .lines
            .iter()
            .map(|l| {
                let s = match l.status {
                    DpiStatus::Ok => "OK",
                    DpiStatus::Fail => "FAIL",
                    DpiStatus::LikelyBlocked => "BLOCKED",
                };
                format!("{}:{s}", l.label)
            })
            .collect();
        let blocked = d.lines.iter().any(|l| l.status == DpiStatus::LikelyBlocked);
        ui.label(rt(
            format!(
                "[{}] {} ({}) — {}",
                d.country,
                d.id,
                d.provider,
                parts.join(" ")
            ),
            12.0,
            W::Mono,
            if blocked { DANGER } else { WARNING },
        ));
    }
    ui.label(rt(
        format!("Целей без проблем: {good}"),
        12.0,
        W::Mono,
        SUCCESS,
    ));
}
