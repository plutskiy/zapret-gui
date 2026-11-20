use std::borrow::Cow;
use std::f32::consts::{FRAC_PI_2, TAU};
use std::sync::Arc;

use egui::epaint::{Mesh, Shadow};
use egui::text::{LayoutJob, TextFormat};
use egui::{
    Align, Align2, Color32, CornerRadius, FontData, FontDefinitions, FontFamily, FontId, Layout,
    Painter, Pos2, Rect, Response, RichText, Sense, Shape, Stroke, StrokeKind, Ui, UiBuilder, Vec2,
    pos2, vec2,
};

pub const fn rgb(r: u8, g: u8, b: u8) -> Color32 {
    Color32::from_rgb(r, g, b)
}
pub const fn rgba(r: u8, g: u8, b: u8, a: u8) -> Color32 {
    Color32::from_rgba_unmultiplied_const(r, g, b, a)
}

pub const INK_1000: Color32 = rgb(6, 6, 7);
pub const BG_WINDOW: Color32 = rgb(10, 10, 12);
pub const BG_CHROME: Color32 = rgb(14, 14, 17);
pub const CARD: Color32 = rgb(24, 24, 32);
pub const HOVER: Color32 = rgb(31, 31, 40);
pub const INK_700: Color32 = rgb(39, 39, 48);
pub const INK_600: Color32 = rgb(52, 52, 63);
pub const INK_500: Color32 = rgb(74, 74, 87);

pub const LINE_SOFT: Color32 = rgba(255, 255, 255, 15);
pub const LINE: Color32 = rgba(255, 255, 255, 26);
pub const LINE_STRONG: Color32 = rgba(255, 255, 255, 41);

pub const TEXT: Color32 = rgb(244, 245, 247);
pub const TEXT2: Color32 = rgba(244, 245, 247, 168);
pub const TEXT3: Color32 = rgba(244, 245, 247, 107);

pub const ACCENT: Color32 = rgb(0, 217, 192);
pub const ACCENT_HOVER: Color32 = rgb(46, 233, 210);
pub const ACCENT_PRESS: Color32 = rgb(0, 184, 163);
pub const ON_ACCENT: Color32 = rgb(4, 32, 28);
pub const TEAL_TINT: Color32 = rgba(0, 217, 192, 31);
pub const TEAL_GLOW: Color32 = rgba(0, 217, 192, 77);

pub const SUCCESS: Color32 = rgb(52, 211, 153);
pub const WARNING: Color32 = rgb(251, 191, 36);
pub const DANGER: Color32 = rgb(255, 90, 90);

// ---------------------------------------------------------------- fonts

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum W {
    Reg,
    Med,
    Semi,
    Bold,
    D700,
    D800,
    Mono,
    Mono500,
}

fn family(w: W) -> FontFamily {
    match w {
        W::Reg => FontFamily::Proportional,
        W::Med => FontFamily::Name("ui500".into()),
        W::Semi => FontFamily::Name("ui600".into()),
        W::Bold => FontFamily::Name("ui700".into()),
        W::D700 => FontFamily::Name("disp700".into()),
        W::D800 => FontFamily::Name("disp800".into()),
        W::Mono => FontFamily::Monospace,
        W::Mono500 => FontFamily::Name("mono500".into()),
    }
}

pub fn font(size: f32, w: W) -> FontId {
    FontId::new(size, family(w))
}

pub fn rt(text: impl Into<String>, size: f32, w: W, color: Color32) -> RichText {
    RichText::new(text.into()).font(font(size, w)).color(color)
}

pub fn install_fonts(ctx: &egui::Context) {
    let mut fonts = FontDefinitions::default();
    let base_prop = fonts
        .families
        .get(&FontFamily::Proportional)
        .cloned()
        .unwrap_or_default();
    let base_mono = fonts
        .families
        .get(&FontFamily::Monospace)
        .cloned()
        .unwrap_or_default();

    let inter: &'static [u8] = include_bytes!("../assets/fonts/InterVar.ttf");
    let tight: &'static [u8] = include_bytes!("../assets/fonts/InterTightVar.ttf");
    let jb: &'static [u8] = include_bytes!("../assets/fonts/JetBrainsMono-Regular.ttf");
    let jb500: &'static [u8] = include_bytes!("../assets/fonts/JetBrainsMono-Medium.ttf");

    let var = |bytes: &'static [u8], wght: f32| {
        let mut d = FontData::from_static(bytes);
        d.tweak.coords = egui::epaint::text::VariationCoords::new([(b"wght", wght)]);
        Arc::new(d)
    };
    let entries: [(&str, Arc<FontData>); 8] = [
        ("inter400", var(inter, 400.0)),
        ("inter500", var(inter, 500.0)),
        ("inter600", var(inter, 600.0)),
        ("inter700", var(inter, 700.0)),
        ("tight700", var(tight, 700.0)),
        ("tight800", var(tight, 800.0)),
        ("jb400", Arc::new(FontData::from_static(jb))),
        ("jb500", Arc::new(FontData::from_static(jb500))),
    ];
    for (name, data) in entries {
        fonts.font_data.insert(name.to_owned(), data);
    }

    let mut put = |fam: FontFamily, first: &str, base: &[String]| {
        let mut v = vec![first.to_owned()];
        v.extend(base.iter().cloned());
        fonts.families.insert(fam, v);
    };
    put(FontFamily::Proportional, "inter400", &base_prop);
    put(FontFamily::Name("ui500".into()), "inter500", &base_prop);
    put(FontFamily::Name("ui600".into()), "inter600", &base_prop);
    put(FontFamily::Name("ui700".into()), "inter700", &base_prop);
    put(FontFamily::Name("disp700".into()), "tight700", &base_prop);
    put(FontFamily::Name("disp800".into()), "tight800", &base_prop);
    put(FontFamily::Monospace, "jb400", &base_mono);
    put(FontFamily::Name("mono500".into()), "jb500", &base_mono);
    ctx.set_fonts(fonts);
}

pub fn apply_style(ctx: &egui::Context) {
    install_fonts(ctx);
    let mut v = egui::Visuals::dark();
    v.panel_fill = Color32::TRANSPARENT;
    v.window_fill = CARD;
    v.window_stroke = Stroke::new(1.0, LINE);
    v.window_corner_radius = CornerRadius::same(12);
    v.menu_corner_radius = CornerRadius::same(12);
    v.extreme_bg_color = INK_1000;
    v.faint_bg_color = HOVER;
    v.selection.bg_fill = ACCENT.gamma_multiply(0.35);
    v.selection.stroke = Stroke::new(1.0, ACCENT_HOVER);
    v.hyperlink_color = ACCENT;
    v.popup_shadow = Shadow {
        offset: [0, 12],
        blur: 32,
        spread: 0,
        color: rgba(0, 0, 0, 140),
    };
    let r = CornerRadius::same(10);
    for w in [
        &mut v.widgets.noninteractive,
        &mut v.widgets.inactive,
        &mut v.widgets.hovered,
        &mut v.widgets.active,
        &mut v.widgets.open,
    ] {
        w.corner_radius = r;
        w.fg_stroke = Stroke::new(1.0, TEXT);
    }
    v.widgets.noninteractive.bg_stroke = Stroke::new(1.0, LINE_SOFT);
    v.widgets.noninteractive.bg_fill = CARD;
    v.widgets.inactive.bg_fill = INK_700;
    v.widgets.inactive.weak_bg_fill = Color32::TRANSPARENT;
    v.widgets.inactive.bg_stroke = Stroke::NONE;
    v.widgets.hovered.bg_fill = INK_600;
    v.widgets.hovered.weak_bg_fill = HOVER;
    v.widgets.hovered.bg_stroke = Stroke::NONE;
    v.widgets.active.bg_fill = ACCENT.gamma_multiply(0.3);
    v.widgets.active.weak_bg_fill = INK_700;
    v.widgets.active.bg_stroke = Stroke::NONE;
    v.widgets.open.weak_bg_fill = HOVER;
    ctx.set_visuals(v);
    ctx.global_style_mut(|s| {
        s.spacing.item_spacing = vec2(8.0, 8.0);
        s.spacing.button_padding = vec2(12.0, 6.0);
        s.spacing.interact_size.y = 28.0;
        s.spacing.scroll = egui::style::ScrollStyle::floating();
        s.spacing.scroll.bar_width = 6.0;
        s.spacing.scroll.floating_width = 4.0;
        s.spacing.scroll.floating_allocated_width = 0.0;
        s.text_styles
            .insert(egui::TextStyle::Body, font(14.0, W::Reg));
        s.text_styles
            .insert(egui::TextStyle::Button, font(13.0, W::Med));
        s.text_styles
            .insert(egui::TextStyle::Small, font(12.0, W::Reg));
        s.text_styles
            .insert(egui::TextStyle::Monospace, font(13.0, W::Mono));
    });
}

// ---------------------------------------------------------------- icons

#[derive(Clone, Copy)]
pub enum Icon {
    Home,
    Settings,
    Send,
    Play,
    More,
    Server,
    X,
    XThin,
    Plus,
    ChevR,
    ChevL,
    ChevD,
    Power,
    Warn,
    Check,
    CheckBox,
    CheckCircle,
}

macro_rules! stroke_svg {
    ($sw:literal, $body:literal) => {
        concat!(
            r##"<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="#fff" stroke-width=""##,
            $sw,
            r##"" stroke-linecap="round" stroke-linejoin="round">"##,
            $body,
            "</svg>"
        )
    };
}

impl Icon {
    fn svg(self) -> (&'static str, &'static str) {
        match self {
            Icon::Home => (
                "bytes://i-home.svg",
                stroke_svg!(
                    "1.9",
                    r#"<path d="M3 9.5 12 3l9 6.5"/><path d="M5 10v10h14V10"/><path d="M9 20v-6h6v6"/>"#
                ),
            ),
            Icon::Settings => (
                "bytes://i-settings.svg",
                stroke_svg!(
                    "1.9",
                    r#"<circle cx="12" cy="12" r="3"/><path d="M12 2v3M12 19v3M2 12h3M19 12h3M5 5l2 2M17 17l2 2M19 5l-2 2M7 17l-2 2"/>"#
                ),
            ),
            Icon::Send => (
                "bytes://i-send.svg",
                stroke_svg!(
                    "1.9",
                    r#"<path d="m22 2-7 20-4-9-9-4Z"/><path d="M22 2 11 13"/>"#
                ),
            ),
            Icon::Play => (
                "bytes://i-play.svg",
                r##"<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="#fff"><path d="M7 4.5v15a1 1 0 0 0 1.5.86l13-7.5a1 1 0 0 0 0-1.72l-13-7.5A1 1 0 0 0 7 4.5Z"/></svg>"##,
            ),
            Icon::More => (
                "bytes://i-more.svg",
                stroke_svg!(
                    "1.9",
                    r#"<circle cx="5" cy="12" r="1.4"/><circle cx="12" cy="12" r="1.4"/><circle cx="19" cy="12" r="1.4"/>"#
                ),
            ),
            Icon::Server => (
                "bytes://i-server.svg",
                stroke_svg!(
                    "1.9",
                    r#"<rect x="3" y="4" width="18" height="7" rx="2"/><rect x="3" y="13" width="18" height="7" rx="2"/><path d="M7 7.5h.01M7 16.5h.01"/>"#
                ),
            ),
            Icon::X => (
                "bytes://i-x.svg",
                stroke_svg!("1.9", r#"<path d="M6 6 18 18"/><path d="M18 6 6 18"/>"#),
            ),
            Icon::XThin => (
                "bytes://i-x24.svg",
                stroke_svg!("2.4", r#"<path d="M6 6 18 18"/><path d="M18 6 6 18"/>"#),
            ),
            Icon::Plus => (
                "bytes://i-plus.svg",
                stroke_svg!("1.9", r#"<path d="M12 5v14"/><path d="M5 12h14"/>"#),
            ),
            Icon::ChevR => (
                "bytes://i-chevr.svg",
                stroke_svg!("1.9", r#"<path d="m9 6 6 6-6 6"/>"#),
            ),
            Icon::ChevL => (
                "bytes://i-chevl.svg",
                stroke_svg!("1.9", r#"<path d="m15 6-6 6 6 6"/>"#),
            ),
            Icon::ChevD => (
                "bytes://i-chevd.svg",
                stroke_svg!("1.9", r#"<path d="m6 9 6 6 6-6"/>"#),
            ),
            Icon::Power => (
                "bytes://i-power.svg",
                stroke_svg!(
                    "1.7",
                    r#"<path d="M18.36 6.64a9 9 0 1 1-12.73 0"/><path d="M12 2v10"/>"#
                ),
            ),
            Icon::Warn => (
                "bytes://i-warn.svg",
                stroke_svg!(
                    "1.9",
                    r#"<path d="M10.3 3.9 1.8 18a2 2 0 0 0 1.7 3h17a2 2 0 0 0 1.7-3L13.7 3.9a2 2 0 0 0-3.4 0Z"/><path d="M12 9v4"/><path d="M12 17h.01"/>"#
                ),
            ),
            Icon::Check => (
                "bytes://i-check.svg",
                stroke_svg!("2.2", r#"<path d="m20 6-11 11-5-5"/>"#),
            ),
            Icon::CheckBox => (
                "bytes://i-checkbox.svg",
                stroke_svg!("3.2", r#"<path d="m20 6-11 11-5-5"/>"#),
            ),
            Icon::CheckCircle => (
                "bytes://i-checkc.svg",
                stroke_svg!(
                    "2",
                    r#"<circle cx="12" cy="12" r="9"/><path d="m8.5 12 2.5 2.5 4.5-5"/>"#
                ),
            ),
        }
    }
}

pub fn paint_icon(ui: &Ui, rect: Rect, icon: Icon, color: Color32) {
    let (uri, svg) = icon.svg();
    let src = egui::ImageSource::Bytes {
        uri: Cow::Borrowed(uri),
        bytes: egui::load::Bytes::Static(svg.as_bytes()),
    };
    egui::Image::new(src)
        .tint(color)
        .fit_to_exact_size(rect.size())
        .paint_at(ui, rect);
}

pub fn icon_at(ui: &Ui, center: Pos2, size: f32, icon: Icon, color: Color32) {
    paint_icon(
        ui,
        Rect::from_center_size(center, Vec2::splat(size)),
        icon,
        color,
    );
}

// ---------------------------------------------------------------- helpers

pub fn cr(r: f32) -> CornerRadius {
    CornerRadius::same(r.round().clamp(0.0, 255.0) as u8)
}

pub fn text_w(ui: &Ui, s: &str, font: &FontId) -> f32 {
    ui.painter()
        .layout_no_wrap(s.to_owned(), font.clone(), Color32::WHITE)
        .size()
        .x
}

fn dim(c: Color32, enabled: bool) -> Color32 {
    if enabled { c } else { c.gamma_multiply(0.4) }
}

pub fn text_trunc(
    painter: &Painter,
    left_center: Pos2,
    s: &str,
    font: FontId,
    color: Color32,
    max_w: f32,
) {
    let mut job = LayoutJob::single_section(s.to_owned(), TextFormat::simple(font, color));
    job.wrap.max_width = max_w.max(1.0);
    job.wrap.max_rows = 1;
    job.wrap.break_anywhere = true;
    job.wrap.overflow_character = Some('…');
    let galley = painter.layout_job(job);
    let pos = pos2(left_center.x, left_center.y - galley.size().y / 2.0);
    painter.galley(pos, galley, color);
}

pub fn card_frame() -> egui::Frame {
    egui::Frame::new()
        .fill(CARD)
        .stroke(Stroke::new(1.0, LINE_SOFT))
        .corner_radius(16)
        .inner_margin(egui::Margin::same(16))
}

pub fn card_top_edge(painter: &Painter, rect: Rect) {
    painter.line_segment(
        [
            pos2(rect.left() + 16.0, rect.top() + 1.5),
            pos2(rect.right() - 16.0, rect.top() + 1.5),
        ],
        Stroke::new(1.0, rgba(255, 255, 255, 13)),
    );
}

pub fn h_title(ui: &mut Ui, s: &str) -> Response {
    ui.add(egui::Label::new(rt(s, 18.0, W::D700, TEXT)).selectable(false))
}

pub fn page_title(ui: &mut Ui, s: &str) -> Response {
    ui.add(egui::Label::new(rt(s, 28.0, W::D800, TEXT)).selectable(false))
}

pub fn divider(ui: &mut Ui) {
    let (r, _) = ui.allocate_exact_size(vec2(ui.available_width(), 1.0), Sense::hover());
    ui.painter().rect_filled(r, 0, LINE_SOFT);
}

/// 11px / 600 / uppercase / 0.14em tracking label.
pub fn overline(ui: &mut Ui, s: &str) {
    let f = font(11.0, W::Semi);
    let upper = s.to_uppercase();
    let spacing = 11.0 * 0.14;
    let mut x = 0.0;
    let mut glyphs = Vec::new();
    for ch in upper.chars() {
        let w = text_w(ui, &ch.to_string(), &f);
        glyphs.push((ch, x));
        x += w + spacing;
    }
    let (rect, _) = ui.allocate_exact_size(
        vec2(x.min(ui.available_width()).max(1.0), 13.0),
        Sense::hover(),
    );
    let painter = ui.painter().with_clip_rect(rect.expand(2.0));
    for (ch, gx) in glyphs {
        painter.text(
            pos2(rect.left() + gx, rect.center().y),
            Align2::LEFT_CENTER,
            ch,
            f.clone(),
            TEXT3,
        );
    }
}

// ---------------------------------------------------------------- buttons

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Variant {
    Primary,
    Secondary,
    Ghost,
}

pub fn button(
    ui: &mut Ui,
    label: &str,
    variant: Variant,
    icon: Option<Icon>,
    enabled: bool,
) -> Response {
    let h = 28.0;
    let f = font(
        13.0,
        if variant == Variant::Primary {
            W::Bold
        } else {
            W::Med
        },
    );
    let tw = text_w(ui, label, &f);
    let iw = if icon.is_some() { 15.0 + 6.0 } else { 0.0 };
    let w = 28.0 + iw + tw + 2.0;
    let (rect, resp) = ui.allocate_exact_size(
        vec2(w, h),
        if enabled {
            Sense::click()
        } else {
            Sense::hover()
        },
    );
    let hover = enabled && resp.hovered();
    let press = enabled && resp.is_pointer_button_down_on();
    let (bg, fg, border) = match variant {
        Variant::Primary => (
            if press {
                ACCENT_PRESS
            } else if hover {
                ACCENT_HOVER
            } else {
                ACCENT
            },
            ON_ACCENT,
            Color32::TRANSPARENT,
        ),
        Variant::Secondary => (
            if press {
                HOVER
            } else if hover {
                INK_600
            } else {
                INK_700
            },
            TEXT,
            LINE,
        ),
        Variant::Ghost => (
            if press {
                CARD
            } else if hover {
                HOVER
            } else {
                Color32::TRANSPARENT
            },
            TEXT2,
            Color32::TRANSPARENT,
        ),
    };
    let p = ui.painter();
    if variant == Variant::Primary && enabled {
        p.add(
            Shadow {
                offset: [0, 2],
                blur: 14,
                spread: 0,
                color: TEAL_GLOW,
            }
            .as_shape(rect, cr(14.0)),
        );
    }
    p.rect_filled(rect, cr(14.0), dim(bg, enabled));
    if border != Color32::TRANSPARENT {
        p.rect_stroke(
            rect,
            cr(14.0),
            Stroke::new(1.0, dim(border, enabled)),
            StrokeKind::Inside,
        );
    }
    let mut x = rect.center().x - (iw + tw) / 2.0;
    if let Some(ic) = icon {
        icon_at(
            ui,
            pos2(x + 7.5, rect.center().y),
            15.0,
            ic,
            dim(fg, enabled),
        );
        x += iw;
    }
    ui.painter().text(
        pos2(x, rect.center().y),
        Align2::LEFT_CENTER,
        label,
        f,
        dim(fg, enabled),
    );
    if enabled {
        resp.on_hover_cursor(egui::CursorIcon::PointingHand)
    } else {
        resp
    }
}

pub fn icon_button(ui: &mut Ui, icon: Icon, opacity: f32, enabled: bool) -> Response {
    let (rect, resp) = ui.allocate_exact_size(
        Vec2::splat(30.0),
        if enabled {
            Sense::click()
        } else {
            Sense::hover()
        },
    );
    let hover = enabled && resp.hovered();
    let p = ui.painter();
    if resp.is_pointer_button_down_on() && enabled {
        p.circle_filled(rect.center(), 15.0, CARD);
    } else if hover {
        p.circle_filled(rect.center(), 15.0, HOVER);
    }
    let color = if hover { TEXT } else { TEXT2 };
    icon_at(
        ui,
        rect.center(),
        17.0,
        icon,
        dim(
            color.gamma_multiply(if hover { 1.0 } else { opacity }),
            enabled,
        ),
    );
    if enabled {
        resp.on_hover_cursor(egui::CursorIcon::PointingHand)
    } else {
        resp
    }
}

// ---------------------------------------------------------------- badge

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Tone {
    Neutral,
    Accent,
    Success,
    Warning,
    Danger,
}

pub fn badge(ui: &mut Ui, text: &str, tone: Tone, mono: bool) -> Response {
    let (bg, fg, bd) = match tone {
        Tone::Neutral => (INK_700, TEXT2, LINE),
        Tone::Accent => (TEAL_TINT, ACCENT_HOVER, Color32::TRANSPARENT),
        Tone::Success => (SUCCESS.gamma_multiply(0.16), SUCCESS, Color32::TRANSPARENT),
        Tone::Warning => (WARNING.gamma_multiply(0.16), WARNING, Color32::TRANSPARENT),
        Tone::Danger => (DANGER.gamma_multiply(0.16), DANGER, Color32::TRANSPARENT),
    };
    let f = font(11.0, if mono { W::Mono500 } else { W::Semi });
    let tw = text_w(ui, text, &f);
    let (rect, resp) = ui.allocate_exact_size(vec2(tw + 20.0, 17.0), Sense::hover());
    let p = ui.painter();
    p.rect_filled(rect, cr(6.0), bg);
    if bd != Color32::TRANSPARENT {
        p.rect_stroke(rect, cr(6.0), Stroke::new(1.0, bd), StrokeKind::Inside);
    }
    p.text(rect.center(), Align2::CENTER_CENTER, text, f, fg);
    resp
}

pub fn badge_width(ui: &Ui, text: &str, mono: bool) -> f32 {
    text_w(
        ui,
        text,
        &font(11.0, if mono { W::Mono500 } else { W::Semi }),
    ) + 20.0
}

// ---------------------------------------------------------------- controls

pub fn segmented<T: PartialEq + Copy>(
    ui: &mut Ui,
    value: &mut T,
    items: &[(T, &str)],
    small: bool,
) -> bool {
    let h = if small { 30.0 } else { 36.0 };
    let f_on = font(13.0, W::Semi);
    let f_off = font(13.0, W::Med);
    let seg_w = items
        .iter()
        .map(|(_, n)| text_w(ui, n, &f_on).max(text_w(ui, n, &f_off)))
        .fold(0.0, f32::max)
        + 32.0;
    let total = seg_w * items.len() as f32 + 6.0;
    let (rect, _) = ui.allocate_exact_size(vec2(total, h), Sense::hover());
    let p = ui.painter().clone();
    p.rect_filled(rect, cr(999.0), INK_1000);
    p.rect_stroke(rect, cr(999.0), Stroke::new(1.0, LINE), StrokeKind::Inside);
    let mut changed = false;
    for (i, (v, name)) in items.iter().enumerate() {
        let seg = Rect::from_min_size(
            pos2(rect.left() + 3.0 + seg_w * i as f32, rect.top() + 3.0),
            vec2(seg_w, h - 6.0),
        );
        let on = *value == *v;
        let resp = ui.interact(seg, ui.scope_id().with(("seg", i)), Sense::click());
        if on {
            p.rect_filled(seg, cr(999.0), INK_700);
            p.line_segment(
                [
                    pos2(seg.left() + 10.0, seg.top() + 0.5),
                    pos2(seg.right() - 10.0, seg.top() + 0.5),
                ],
                Stroke::new(1.0, rgba(255, 255, 255, 13)),
            );
        } else if resp.hovered() {
            p.rect_filled(seg, cr(999.0), rgba(255, 255, 255, 8));
        }
        let (f, c) = if on {
            (f_on.clone(), TEXT)
        } else {
            (f_off.clone(), if resp.hovered() { TEXT2 } else { TEXT3 })
        };
        p.text(seg.center(), Align2::CENTER_CENTER, *name, f, c);
        if resp
            .on_hover_cursor(egui::CursorIcon::PointingHand)
            .clicked()
            && !on
        {
            *value = *v;
            changed = true;
        }
    }
    changed
}

pub fn switch(ui: &mut Ui, on: &mut bool, small: bool) -> bool {
    let (w, h) = if small { (36.0, 20.0) } else { (44.0, 26.0) };
    let knob = h - 6.0;
    let (rect, resp) = ui.allocate_exact_size(vec2(w, h), Sense::click());
    let mut changed = false;
    if resp.clicked() {
        *on = !*on;
        changed = true;
    }
    let t = ui.ctx().animate_bool_with_time(resp.id, *on, 0.2);
    let p = ui.painter();
    p.rect_filled(rect, cr(999.0), INK_600.lerp_to_gamma(ACCENT, t));
    let kx = egui::lerp(
        rect.left() + 3.0 + knob / 2.0..=rect.right() - 3.0 - knob / 2.0,
        t,
    );
    p.add(
        Shadow {
            offset: [0, 1],
            blur: 3,
            spread: 0,
            color: rgba(0, 0, 0, 100),
        }
        .as_shape(
            Rect::from_center_size(pos2(kx, rect.center().y), Vec2::splat(knob)),
            cr(knob),
        ),
    );
    p.circle_filled(pos2(kx, rect.center().y), knob / 2.0, Color32::WHITE);
    resp.on_hover_cursor(egui::CursorIcon::PointingHand);
    changed
}

pub fn slider(ui: &mut Ui, value: &mut u32, min: u32, max: u32, width: f32) -> bool {
    let thumb = 13.0;
    let (rect, resp) = ui.allocate_exact_size(vec2(width, thumb), Sense::click_and_drag());
    let mut changed = false;
    if (resp.dragged() || resp.is_pointer_button_down_on())
        && let Some(pos) = resp.interact_pointer_pos()
    {
        let ratio = ((pos.x - rect.left()) / rect.width()).clamp(0.0, 1.0);
        let v = (min as f32 + ratio * (max - min) as f32).round() as u32;
        if v != *value {
            *value = v;
            changed = true;
        }
    }
    let ratio = (*value - min.min(*value)) as f32 / (max - min).max(1) as f32;
    let hot = resp.hovered() || resp.dragged();
    let track = Rect::from_center_size(rect.center(), vec2(width, 4.0));
    let p = ui.painter();
    p.rect_filled(track, cr(999.0), INK_700);
    let fill = Rect::from_min_size(track.min, vec2(track.width() * ratio, 4.0));
    p.rect_filled(fill, cr(999.0), if hot { ACCENT_HOVER } else { ACCENT });
    let c = pos2(rect.left() + width * ratio, rect.center().y);
    p.add(
        Shadow {
            offset: [0, 2],
            blur: 8,
            spread: 0,
            color: rgba(0, 0, 0, 128),
        }
        .as_shape(Rect::from_center_size(c, Vec2::splat(thumb)), cr(thumb)),
    );
    p.circle_filled(c, thumb / 2.0, Color32::WHITE);
    resp.on_hover_cursor(egui::CursorIcon::PointingHand);
    changed
}

pub fn stepper(ui: &mut Ui, value: &mut u32, min: u32, max: u32, step: u32, width: f32) -> bool {
    let (rect, _) = ui.allocate_exact_size(vec2(width, 36.0), Sense::hover());
    let p = ui.painter().clone();
    p.rect_filled(rect, cr(10.0), INK_1000);
    p.rect_stroke(rect, cr(10.0), Stroke::new(1.0, LINE), StrokeKind::Inside);
    let mut changed = false;
    for (i, (icon, delta)) in [(Icon::ChevL, -1i64), (Icon::ChevR, 1i64)]
        .into_iter()
        .enumerate()
    {
        let zone = if i == 0 {
            Rect::from_min_size(rect.min, vec2(30.0, rect.height()))
        } else {
            Rect::from_min_size(
                pos2(rect.right() - 30.0, rect.top()),
                vec2(30.0, rect.height()),
            )
        };
        let at_limit = if delta < 0 {
            *value <= min
        } else {
            *value >= max
        };
        let resp = ui.interact(
            zone,
            ui.scope_id().with(("step", i)),
            if at_limit {
                Sense::hover()
            } else {
                Sense::click()
            },
        );
        let color = if resp.hovered() && !at_limit {
            TEXT
        } else {
            TEXT2
        };
        icon_at(
            ui,
            zone.center(),
            16.0,
            icon,
            if at_limit {
                color.gamma_multiply(0.3)
            } else {
                color
            },
        );
        if resp.clicked() {
            *value = (*value as i64 + delta * step as i64).clamp(min as i64, max as i64) as u32;
            changed = true;
        }
    }
    p.text(
        rect.center(),
        Align2::CENTER_CENTER,
        value.to_string(),
        font(13.0, W::Mono500),
        TEXT,
    );
    changed
}

pub fn paint_checkbox(ui: &Ui, center: Pos2, checked: bool, enabled: bool) {
    let r = Rect::from_center_size(center, Vec2::splat(16.0));
    let p = ui.painter();
    if checked {
        p.rect_filled(r, cr(5.0), dim(ACCENT, enabled));
        icon_at(ui, center, 12.0, Icon::CheckBox, ON_ACCENT);
    } else {
        p.rect_stroke(
            r.shrink(0.75),
            cr(5.0),
            Stroke::new(1.5, dim(LINE_STRONG, enabled)),
            StrokeKind::Inside,
        );
    }
}

pub fn spinner(ui: &Ui, center: Pos2, size: f32) {
    let t = ui.input(|i| i.time) as f32;
    let r = size / 2.0 - 1.0;
    let p = ui.painter();
    p.circle_stroke(center, r, Stroke::new(2.0, rgba(255, 255, 255, 31)));
    p.add(Shape::line(
        arc_points(center, r, t * 6.0, 1.6),
        Stroke::new(2.0, ACCENT),
    ));
    ui.ctx().request_repaint();
}

pub fn progress_bar(ui: &mut Ui, frac: f32, width: f32) {
    let (rect, _) = ui.allocate_exact_size(vec2(width, 4.0), Sense::hover());
    let p = ui.painter();
    p.rect_filled(rect, cr(2.0), INK_700);
    let fill = Rect::from_min_size(rect.min, vec2(rect.width() * frac.clamp(0.0, 1.0), 4.0));
    p.add(
        Shadow {
            offset: [0, 0],
            blur: 12,
            spread: 0,
            color: TEAL_GLOW,
        }
        .as_shape(fill, cr(2.0)),
    );
    p.rect_filled(fill, cr(2.0), ACCENT);
}

pub fn chip(ui: &mut Ui, label: &str) -> bool {
    let f = font(13.0, W::Med);
    let tw = text_w(ui, label, &f);
    let (rect, _) =
        ui.allocate_exact_size(vec2(14.0 + tw + 8.0 + 18.0 + 8.0, 32.0), Sense::hover());
    let p = ui.painter().clone();
    p.rect_filled(rect, cr(999.0), INK_700);
    p.text(
        pos2(rect.left() + 14.0, rect.center().y),
        Align2::LEFT_CENTER,
        label,
        f,
        TEXT,
    );
    let close = Rect::from_center_size(
        pos2(rect.right() - 8.0 - 9.0, rect.center().y),
        Vec2::splat(18.0),
    );
    let resp = ui.interact(close, ui.scope_id().with(("chipx", label)), Sense::click());
    p.circle_filled(
        close.center(),
        9.0,
        if resp.hovered() {
            rgba(255, 255, 255, 26)
        } else {
            rgba(255, 255, 255, 20)
        },
    );
    icon_at(ui, close.center(), 11.0, Icon::XThin, TEXT2);
    resp.on_hover_cursor(egui::CursorIcon::PointingHand)
        .clicked()
}

pub fn banner(ui: &mut Ui, text: &str, max_w: f32, add_right: impl FnOnce(&mut Ui)) {
    egui::Frame::new()
        .fill(rgba(251, 191, 36, 26))
        .stroke(Stroke::new(1.0, rgba(251, 191, 36, 61)))
        .corner_radius(12)
        .inner_margin(egui::Margin::symmetric(14, 10))
        .show(ui, |ui| {
            ui.set_max_width(max_w);
            ui.horizontal(|ui| {
                let (r, _) = ui.allocate_exact_size(vec2(18.0, 18.0), Sense::hover());
                icon_at(ui, r.center(), 18.0, Icon::Warn, WARNING);
                ui.add(egui::Label::new(rt(text, 13.0, W::Med, WARNING)));
                add_right(ui);
            });
        });
}

pub struct InputOpts<'a> {
    pub hint: &'a str,
    pub icon: Option<Icon>,
    pub password: bool,
    pub mono: bool,
}

impl Default for InputOpts<'_> {
    fn default() -> Self {
        Self {
            hint: "",
            icon: None,
            password: false,
            mono: false,
        }
    }
}

pub fn text_input(
    ui: &mut Ui,
    salt: &str,
    text: &mut String,
    width: f32,
    opts: InputOpts,
) -> Response {
    let h = 28.0;
    let id = ui.scope_id().with(("input", salt));
    let (rect, _) = ui.allocate_exact_size(vec2(width, h), Sense::hover());
    let focused = ui.memory(|m| m.has_focus(id));
    let p = ui.painter().clone();
    if focused {
        p.rect_stroke(
            rect,
            cr(14.0),
            Stroke::new(3.0, TEAL_GLOW),
            StrokeKind::Outside,
        );
    }
    p.rect_filled(rect, cr(14.0), INK_1000);
    p.rect_stroke(
        rect,
        cr(14.0),
        Stroke::new(1.0, if focused { ACCENT } else { LINE }),
        StrokeKind::Inside,
    );
    let mut left = rect.left() + 14.0;
    if let Some(ic) = opts.icon {
        icon_at(
            ui,
            pos2(left + 9.0, rect.center().y),
            18.0,
            ic,
            if focused { TEXT2 } else { TEXT3 },
        );
        left += 28.0;
    }
    let inner = Rect::from_min_max(
        pos2(left, rect.top()),
        pos2(rect.right() - 14.0, rect.bottom()),
    );
    let mut child = ui.new_child(
        UiBuilder::new()
            .max_rect(inner)
            .layout(Layout::left_to_right(Align::Center)),
    );
    child.set_clip_rect(inner.intersect(ui.clip_rect()));
    let f = font(14.0, if opts.mono { W::Mono } else { W::Reg });
    let te = egui::TextEdit::singleline(text)
        .id(id)
        .frame(egui::Frame::NONE)
        .margin(egui::Margin::ZERO)
        .font(f.clone())
        .text_color(TEXT)
        .password(opts.password)
        .desired_width(inner.width())
        .hint_text(rt(opts.hint, 14.0, W::Reg, TEXT3));
    child.add(te)
}

pub fn text_area(
    ui: &mut Ui,
    salt: &str,
    text: &mut String,
    width: f32,
    height: f32,
    hint: &str,
    enabled: bool,
) -> Response {
    let id = ui.scope_id().with(("area", salt));
    let (rect, _) = ui.allocate_exact_size(vec2(width, height), Sense::hover());
    let focused = ui.memory(|m| m.has_focus(id));
    let p = ui.painter().clone();
    if focused {
        p.rect_stroke(
            rect,
            cr(12.0),
            Stroke::new(3.0, TEAL_GLOW),
            StrokeKind::Outside,
        );
    }
    p.rect_filled(rect, cr(12.0), INK_1000);
    p.rect_stroke(
        rect,
        cr(12.0),
        Stroke::new(1.0, if focused { ACCENT } else { LINE }),
        StrokeKind::Inside,
    );
    let inner = rect.shrink2(vec2(14.0, 10.0));
    let mut child = ui.new_child(
        UiBuilder::new()
            .max_rect(inner)
            .layout(Layout::top_down(Align::Min)),
    );
    child.set_clip_rect(inner.intersect(ui.clip_rect()));
    child.add_enabled(
        enabled,
        egui::TextEdit::multiline(text)
            .id(id)
            .frame(egui::Frame::NONE)
            .margin(egui::Margin::ZERO)
            .font(font(13.0, W::Mono))
            .text_color(TEXT2)
            .desired_width(inner.width())
            .desired_rows(1)
            .hint_text(rt(hint, 13.0, W::Mono, TEXT3)),
    )
}

pub struct SettingsRow<'a> {
    pub title: &'a str,
    pub desc: &'a str,
    pub last: bool,
}

/// Label on the left, control (added by `add_control`) on the right.
pub fn settings_row(ui: &mut Ui, row: SettingsRow, add_control: impl FnOnce(&mut Ui)) {
    let w = ui.available_width();
    let r = ui.allocate_ui_with_layout(vec2(w, 0.0), Layout::left_to_right(Align::Center), |ui| {
        ui.add_space(0.0);
        ui.spacing_mut().item_spacing = vec2(24.0, 0.0);
        ui.vertical(|ui| {
            ui.spacing_mut().item_spacing.y = 3.0;
            ui.add_space(16.0);
            ui.add(egui::Label::new(rt(row.title, 16.0, W::Semi, TEXT)).selectable(false));
            if !row.desc.is_empty() {
                ui.add(egui::Label::new(rt(row.desc, 12.0, W::Reg, TEXT3)).selectable(false));
            }
            ui.add_space(13.0);
        });
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| add_control(ui));
    });
    if !row.last {
        let y = r.response.rect.bottom();
        ui.painter().rect_filled(
            Rect::from_min_size(pos2(r.response.rect.left(), y), vec2(w, 1.0)),
            0,
            LINE_SOFT,
        );
        ui.add_space(1.0);
    }
}

// ---------------------------------------------------------------- geometry

pub fn arc_points(c: Pos2, r: f32, start: f32, sweep: f32) -> Vec<Pos2> {
    let n = ((sweep.abs() / TAU) * 96.0).ceil().max(2.0) as usize;
    (0..=n)
        .map(|i| {
            let a = start + sweep * i as f32 / n as f32;
            c + vec2(a.cos(), a.sin()) * r
        })
        .collect()
}

pub fn gradient_disc(painter: &Painter, c: Pos2, r: f32, color_at: impl Fn(Pos2) -> Color32) {
    let (rings, segs) = (16usize, 96usize);
    let mut m = Mesh::default();
    m.colored_vertex(c, color_at(c));
    for i in 1..=rings {
        let rr = r * i as f32 / rings as f32;
        for j in 0..segs {
            let a = TAU * j as f32 / segs as f32;
            let p = c + vec2(a.cos(), a.sin()) * rr;
            m.colored_vertex(p, color_at(p));
        }
    }
    for j in 0..segs {
        m.add_triangle(0, (1 + j) as u32, (1 + (j + 1) % segs) as u32);
    }
    for i in 1..rings {
        let base = 1 + (i - 1) * segs;
        let next = 1 + i * segs;
        for j in 0..segs {
            let (a, b) = (base + j, base + (j + 1) % segs);
            let (c2, d) = (next + j, next + (j + 1) % segs);
            m.add_triangle(a as u32, c2 as u32, b as u32);
            m.add_triangle(b as u32, c2 as u32, d as u32);
        }
    }
    painter.add(Shape::mesh(m));
}

pub fn vgradient_rect(painter: &Painter, rect: Rect, top: Color32, bottom: Color32) {
    let mut m = Mesh::default();
    m.colored_vertex(rect.left_top(), top);
    m.colored_vertex(rect.right_top(), top);
    m.colored_vertex(rect.right_bottom(), bottom);
    m.colored_vertex(rect.left_bottom(), bottom);
    m.add_triangle(0, 1, 2);
    m.add_triangle(0, 2, 3);
    painter.add(Shape::mesh(m));
}

/// Soft tinted ellipse at the top of the window (the "wash").
pub fn top_wash(painter: &Painter, window: Rect, center_x_frac: f32, rgb: (u8, u8, u8)) {
    let center = pos2(window.left() + window.width() * center_x_frac, window.top());
    let rx = window.width() * 0.70 * 0.70;
    let ry = 420.0 * 0.70;
    let mut m = Mesh::default();
    m.colored_vertex(center, rgba(rgb.0, rgb.1, rgb.2, 36));
    let n = 72;
    for i in 0..n {
        let a = TAU * i as f32 / n as f32;
        m.colored_vertex(
            center + vec2(a.cos() * rx, a.sin() * ry),
            Color32::TRANSPARENT,
        );
    }
    for i in 0..n {
        m.add_triangle(0, 1 + i as u32, 1 + ((i + 1) % n) as u32);
    }
    painter.with_clip_rect(window).add(Shape::mesh(m));
}

// ---------------------------------------------------------------- power hero

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Hero {
    Off,
    Working,
    On,
    Error,
}

impl Hero {
    pub fn tone(self) -> Color32 {
        match self {
            Hero::Off => rgba(244, 245, 247, 133),
            Hero::Working | Hero::On => ACCENT,
            Hero::Error => DANGER,
        }
    }
    pub fn glow_rgb(self) -> Option<(u8, u8, u8)> {
        match self {
            Hero::Off => None,
            Hero::Error => Some((255, 90, 90)),
            _ => Some((0, 217, 192)),
        }
    }
}

pub fn power_hero(ui: &mut Ui, state: Hero, s: f32, progress: f32, enabled: bool) -> Response {
    let size = 240.0 * s;
    let (rect, resp) = ui.allocate_exact_size(
        vec2(size, size),
        if enabled {
            Sense::click()
        } else {
            Sense::hover()
        },
    );
    let hover = enabled && resp.hovered();
    let c = rect.center();
    let big_r = size / 2.0;
    let disc_r = big_r - 16.0 * s;
    let disc_rect = Rect::from_center_size(c, Vec2::splat(disc_r * 2.0));
    let p = ui.painter();

    // shadow / glow
    match state.glow_rgb() {
        Some((r, g, b)) => {
            p.add(
                Shadow {
                    offset: [0, 0],
                    blur: (80.0 * s) as u8,
                    spread: 0,
                    color: rgba(r, g, b, 71),
                }
                .as_shape(disc_rect, cr(255.0)),
            );
        }
        None => {
            p.add(
                Shadow {
                    offset: [0, (16.0 * s) as i8],
                    blur: (40.0 * s) as u8,
                    spread: 0,
                    color: rgba(0, 0, 0, 128),
                }
                .as_shape(disc_rect, cr(255.0)),
            );
        }
    }

    // outer ring
    let ring_w = 4.6 * s;
    let ring_r = big_r - ring_w / 2.0 - 0.2;
    let track = rgba(255, 255, 255, 26);
    match state {
        Hero::Off => {
            p.circle_stroke(c, ring_r, Stroke::new(ring_w, track));
        }
        Hero::On => {
            p.circle_stroke(c, ring_r, Stroke::new(ring_w, ACCENT));
        }
        Hero::Error => {
            p.circle_stroke(c, ring_r, Stroke::new(ring_w, DANGER));
        }
        Hero::Working => {
            p.circle_stroke(c, ring_r, Stroke::new(ring_w, track));
            let sweep = TAU * progress.clamp(0.03, 1.0);
            p.add(Shape::line(
                arc_points(c, ring_r, -FRAC_PI_2, sweep),
                Stroke::new(ring_w, ACCENT),
            ));
        }
    }

    // disc
    let focus = pos2(c.x, disc_rect.top() + disc_r * 2.0 * 0.25);
    let stop = ((disc_r * 2.0 / 2.0).powi(2) + (disc_r * 2.0 * 0.75).powi(2)).sqrt() * 0.70;
    let (c0, c1) = ((0x23u8, 0x23u8, 0x2du8), (0x13u8, 0x13u8, 0x17u8));
    gradient_disc(p, c, disc_r, |pt| {
        let t = ((pt - focus).length() / stop).clamp(0.0, 1.0);
        let l = |a: u8, b: u8| (a as f32 + (b as f32 - a as f32) * t).round() as u8;
        Color32::from_rgb(l(c0.0, c1.0), l(c0.1, c1.1), l(c0.2, c1.2))
    });
    let border = if hover && state == Hero::Off {
        ACCENT.gamma_multiply(0.45)
    } else {
        LINE
    };
    p.circle_stroke(c, disc_r - 0.5, Stroke::new(1.0, border));
    p.add(Shape::line(
        arc_points(c, disc_r - 1.5, -FRAC_PI_2 - 0.9, 1.8),
        Stroke::new(
            1.0,
            rgba(255, 255, 255, if state == Hero::Off { 15 } else { 20 }),
        ),
    ));

    icon_at(ui, c, 76.0 * s, Icon::Power, state.tone());
    if enabled {
        resp.on_hover_cursor(egui::CursorIcon::PointingHand)
    } else {
        resp
    }
}
