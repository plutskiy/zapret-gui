use std::cmp::Reverse;
use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, Sender, channel};
use std::sync::Arc;
use std::time::Duration;

use egui::{Color32, RichText};

use crate::settings::{ProxyKind, Settings};
use crate::sites::{self, ListKind};
use crate::tester::{self, Check, DpiStatus, Kind, Outcome, Params, Report};
use crate::zapret::{self, ConfigEntry};
use crate::{sys, telegram};

const GREEN: Color32 = Color32::from_rgb(64, 214, 142);
const RED: Color32 = Color32::from_rgb(240, 98, 98);
const YELLOW: Color32 = Color32::from_rgb(240, 192, 72);
const GRAY: Color32 = Color32::from_rgb(140, 148, 165);
const ACCENT: Color32 = Color32::from_rgb(92, 140, 255);
const BG: Color32 = Color32::from_rgb(16, 19, 27);
const CARD: Color32 = Color32::from_rgb(25, 29, 40);
const CARD_HI: Color32 = Color32::from_rgb(34, 40, 55);

fn apply_style(ctx: &egui::Context) {
    let mut v = egui::Visuals::dark();
    v.panel_fill = BG;
    v.window_fill = CARD;
    v.extreme_bg_color = Color32::from_rgb(12, 14, 20);
    v.faint_bg_color = Color32::from_rgb(22, 26, 36);
    v.selection.bg_fill = ACCENT.gamma_multiply(0.55);
    v.hyperlink_color = ACCENT;
    let r = egui::CornerRadius::same(8);
    for w in [&mut v.widgets.noninteractive, &mut v.widgets.inactive, &mut v.widgets.hovered, &mut v.widgets.active, &mut v.widgets.open] {
        w.corner_radius = r;
    }
    v.widgets.inactive.bg_fill = CARD_HI;
    v.widgets.inactive.weak_bg_fill = CARD_HI;
    v.widgets.hovered.bg_fill = Color32::from_rgb(46, 54, 74);
    v.widgets.hovered.weak_bg_fill = Color32::from_rgb(46, 54, 74);
    v.widgets.noninteractive.bg_stroke = egui::Stroke::new(1.0, Color32::from_rgb(38, 44, 58));
    ctx.set_visuals(v);
    ctx.global_style_mut(|s| {
        s.spacing.item_spacing = egui::vec2(8.0, 8.0);
        s.spacing.button_padding = egui::vec2(12.0, 6.0);
        s.spacing.interact_size.y = 28.0;
    });
}

fn card<R>(ui: &mut egui::Ui, add: impl FnOnce(&mut egui::Ui) -> R) -> R {
    egui::Frame::new()
        .fill(CARD)
        .corner_radius(12)
        .inner_margin(egui::Margin::same(14))
        .stroke(egui::Stroke::new(1.0, Color32::from_rgb(36, 42, 56)))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            add(ui)
        })
        .inner
}

fn accent_button(text: impl Into<String>) -> egui::Button<'static> {
    egui::Button::new(RichText::new(text.into()).color(Color32::WHITE).strong()).fill(ACCENT)
}

#[derive(Clone, Copy, PartialEq)]
enum Power {
    Off,
    Working,
    On,
}

fn power_button(ui: &mut egui::Ui, state: Power, enabled: bool) -> egui::Response {
    let size = 176.0;
    let (rect, resp) = ui.allocate_exact_size(egui::vec2(size, size), egui::Sense::click());
    let t = ui.input(|i| i.time) as f32;
    let hover = enabled && resp.hovered();
    let color = match state {
        Power::Off => GRAY,
        Power::Working => ACCENT,
        Power::On => GREEN,
    };
    let p = ui.painter();
    let c = rect.center();
    if state == Power::On {
        for (k, a) in [(1.0, 0.08), (0.88, 0.12)] {
            p.circle_filled(c, size / 2.0 * k, GREEN.gamma_multiply(a));
        }
    }
    let r = size / 2.0 - 18.0;
    let fill = if hover { CARD_HI } else { CARD };
    p.circle(c, r, fill, egui::Stroke::new(4.0, color.gamma_multiply(if enabled { 1.0 } else { 0.4 })));
    if state == Power::Working {
        let n = 28;
        let start = t * 4.0;
        let pts: Vec<egui::Pos2> = (0..=n)
            .map(|i| {
                let a = start + i as f32 / n as f32 * 1.6;
                c + egui::vec2(a.cos(), a.sin()) * (r + 9.0)
            })
            .collect();
        p.add(egui::Shape::line(pts, egui::Stroke::new(4.0, ACCENT)));
        ui.ctx().request_repaint();
    }
    // power icon: arc with a gap at the top and a vertical bar
    let ir = r * 0.42;
    let a0 = -std::f32::consts::FRAC_PI_2 + 0.65;
    let a1 = -std::f32::consts::FRAC_PI_2 + std::f32::consts::TAU - 0.65;
    let pts: Vec<egui::Pos2> = (0..=40)
        .map(|i| {
            let a = a0 + (a1 - a0) * i as f32 / 40.0;
            c + egui::vec2(a.cos(), a.sin()) * ir
        })
        .collect();
    let stroke = egui::Stroke::new(6.0, color.gamma_multiply(if enabled { 1.0 } else { 0.4 }));
    p.add(egui::Shape::line(pts, stroke));
    p.line_segment([c + egui::vec2(0.0, -ir * 1.15), c + egui::vec2(0.0, -ir * 0.1)], stroke);
    if enabled {
        resp.on_hover_cursor(egui::CursorIcon::PointingHand)
    } else {
        resp
    }
}

fn badge(ui: &mut egui::Ui, text: impl Into<String>, color: Color32) {
    egui::Frame::new()
        .fill(color.gamma_multiply(0.18))
        .corner_radius(6)
        .inner_margin(egui::Margin::symmetric(7, 2))
        .show(ui, |ui| {
            ui.label(RichText::new(text.into()).color(color).small().strong());
        });
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Tab {
    Home,
    Telegram,
    Settings,
}

struct OpResult {
    message: String,
    error: bool,
    started: Option<String>,
}

#[derive(Default)]
struct TestState {
    checked: HashSet<String>,
    handle: Option<tester::Handle>,
    reports: Vec<Report>,
    ran_kind: Option<Kind>,
    current: String,
    config_index: (usize, usize),
    target_progress: (usize, usize),
    saved: Option<String>,
    auto: bool,
    best: Option<String>,
}

struct SitesState {
    input: String,
    kind: ListKind,
    results: Vec<(sites::SiteResult, bool)>,
    handle: Option<sites::Handle>,
    expected: usize,
    pause_zapret: bool,
    user_list: Vec<String>,
    restart_hint: bool,
}

#[derive(Default)]
struct TelegramState {
    probe: Option<Receiver<Result<u128, String>>>,
    probe_result: Option<Result<u128, String>>,
}

pub struct App {
    ctx: egui::Context,
    settings: Settings,
    dirty: bool,
    data: Option<PathBuf>,
    configs: Vec<ConfigEntry>,
    selected: Option<usize>,
    tab: Tab,
    show_log: bool,
    elevated: bool,
    running: Arc<AtomicBool>,
    service_conflict: Arc<AtomicBool>,
    busy: bool,
    op_tx: Sender<OpResult>,
    op_rx: Receiver<OpResult>,
    log: Vec<String>,
    test: TestState,
    sites: SitesState,
    tg: TelegramState,
}

impl App {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let ctx = cc.egui_ctx.clone();
        apply_style(&ctx);
        let settings = Settings::load();
        let data = zapret::find_data_dir(&settings.data_dir);
        let configs = data.as_deref().map(zapret::list_configs).unwrap_or_default();
        if let Some(d) = &data {
            tester::restore_leftover_ipset(d);
            zapret::ensure_user_lists(d);
        }

        let running = Arc::new(AtomicBool::new(false));
        let service_conflict = Arc::new(AtomicBool::new(false));
        {
            let (running, conflict, ctx) = (running.clone(), service_conflict.clone(), ctx.clone());
            std::thread::spawn(move || {
                loop {
                    running.store(zapret::is_running(), Ordering::Relaxed);
                    conflict.store(sys::service_running("zapret"), Ordering::Relaxed);
                    ctx.request_repaint();
                    std::thread::sleep(Duration::from_secs(3));
                }
            });
        }

        let (op_tx, op_rx) = channel();
        let selected = configs.iter().position(|c| c.name == settings.last_config);
        let user_list = data.as_deref().map(|d| sites::read_list(d, ListKind::Proxy)).unwrap_or_default();

        let mut app = Self {
            ctx,
            settings,
            dirty: false,
            data,
            configs,
            selected,
            tab: Tab::Home,
            show_log: false,
            elevated: sys::has_required_privileges(),
            running,
            service_conflict,
            busy: false,
            op_tx,
            op_rx,
            log: Vec::new(),
            test: TestState::default(),
            sites: SitesState {
                input: String::new(),
                kind: ListKind::Proxy,
                results: Vec::new(),
                handle: None,
                expected: 0,
                pause_zapret: true,
                user_list,
                restart_hint: false,
            },
            tg: TelegramState::default(),
        };

        let autostart = app.settings.autostart_last_config && app.elevated;
        match (autostart, app.selected) {
            (true, Some(i)) => app.start_config(i),
            _ => {
                if app.settings.telegram.on_app_start {
                    let tg = app.settings.telegram.clone();
                    let tx = app.op_tx.clone();
                    let ctx = app.ctx.clone();
                    std::thread::spawn(move || {
                        std::thread::sleep(Duration::from_secs(1));
                        let _ = tx.send(tg_result(telegram::apply(&tg)));
                        ctx.request_repaint();
                    });
                }
            }
        }
        app
    }

    fn log(&mut self, s: impl Into<String>) {
        self.log.push(s.into());
        if self.log.len() > 500 {
            self.log.drain(..100);
        }
    }

    fn is_running(&self) -> bool {
        self.running.load(Ordering::Relaxed)
    }

    fn start_config(&mut self, index: usize) {
        let (Some(data), Some(cfg)) = (self.data.clone(), self.configs.get(index).cloned()) else { return };
        if self.busy {
            return;
        }
        self.busy = true;
        self.selected = Some(index);
        self.log(format!("Запуск: {}", cfg.name));
        let tg = self.settings.telegram.clone();
        let (tx, ctx) = (self.op_tx.clone(), self.ctx.clone());
        std::thread::spawn(move || {
            let result = zapret::start_config(&data, &cfg);
            let ok = result.is_ok();
            let _ = tx.send(match result {
                Ok(()) => OpResult { message: format!("Запущена конфигурация {}", cfg.name), error: false, started: Some(cfg.name.clone()) },
                Err(e) => OpResult { message: e, error: true, started: None },
            });
            ctx.request_repaint();
            if ok && tg.on_zapret_start {
                std::thread::sleep(Duration::from_millis(1500));
                let _ = tx.send(tg_result(telegram::apply(&tg)));
                ctx.request_repaint();
            }
        });
    }

    fn stop_zapret(&mut self) {
        if self.busy {
            return;
        }
        self.busy = true;
        let (tx, ctx) = (self.op_tx.clone(), self.ctx.clone());
        std::thread::spawn(move || {
            zapret::stop();
            let _ = tx.send(OpResult { message: "zapret остановлен".into(), error: false, started: None });
            ctx.request_repaint();
        });
    }

    fn restart_zapret(&mut self) {
        let idx = self.configs.iter().position(|c| c.name == self.settings.last_config).or(self.selected);
        if let Some(i) = idx {
            self.start_config(i);
        }
    }

    fn poll(&mut self) {
        while let Ok(op) = self.op_rx.try_recv() {
            self.busy = false;
            let prefix = if op.error { "ОШИБКА: " } else { "" };
            self.log(format!("{prefix}{}", op.message));
            if let Some(name) = op.started {
                self.settings.last_config = name;
                self.dirty = true;
                self.sites.restart_hint = false;
            }
        }
        self.poll_tests();
        self.poll_sites();
        if let Some(rx) = &self.tg.probe {
            if let Ok(r) = rx.try_recv() {
                self.tg.probe_result = Some(r);
                self.tg.probe = None;
            }
        }
    }

    fn poll_tests(&mut self) {
        let Some(handle) = &self.test.handle else { return };
        let mut events = Vec::new();
        while let Ok(e) = handle.rx.try_recv() {
            events.push(e);
        }
        let mut finished = false;
        let mut was_cancelled = false;
        for e in events {
            match e {
                tester::Event::Log(s) => self.log(s),
                tester::Event::ConfigStart { index, total, name } => {
                    self.test.current = name;
                    self.test.config_index = (index, total);
                }
                tester::Event::Progress { done, total } => self.test.target_progress = (done, total),
                tester::Event::Report(r) => self.test.reports.push(r),
                tester::Event::Finished { cancelled } => {
                    finished = true;
                    was_cancelled = cancelled;
                    self.log(if cancelled { "Тест остановлен" } else { "Тест завершён" });
                }
            }
        }
        if finished {
            self.test.handle = None;
            self.test.best = self.best_config();
            if self.test.auto {
                self.test.auto = false;
                if !was_cancelled {
                    match self.test.best.clone().and_then(|n| self.configs.iter().position(|c| c.name == n)) {
                        Some(i) => {
                            self.log("Подобрана рабочая конфигурация — подключаюсь");
                            self.start_config(i);
                        }
                        None => self.log("ОШИБКА: ни одна конфигурация не прошла тест — подключение невозможно"),
                    }
                }
            }
            if let (Some(data), Some(kind)) = (self.data.clone(), self.test.ran_kind) {
                if !self.test.reports.is_empty() {
                    match tester::save_report(&data, kind, &self.test.reports) {
                        Ok(p) => {
                            self.log(format!("Отчёт сохранён: {}", p.display()));
                            self.test.saved = Some(p.display().to_string());
                        }
                        Err(e) => self.log(format!("Не удалось сохранить отчёт: {e}")),
                    }
                }
            }
        }
    }

    fn poll_sites(&mut self) {
        let Some(handle) = &self.sites.handle else { return };
        let mut done = false;
        while let Ok(e) = handle.rx.try_recv() {
            match e {
                sites::Event::Result(r) => self.sites.results.push((r, false)),
                sites::Event::Done => done = true,
            }
        }
        if done {
            self.sites.handle = None;
        }
    }

    fn best_config(&self) -> Option<String> {
        self.test
            .reports
            .iter()
            .filter(|r| r.error.is_none() && r.summary().ok > 0)
            .max_by_key(|r| r.score())
            .map(|r| r.config.clone())
    }

    /// Большая кнопка: подключить / отключить / отменить подбор.
    fn toggle_power(&mut self) {
        if self.test.auto {
            if let Some(h) = &self.test.handle {
                h.cancel.store(true, Ordering::Relaxed);
            }
            return;
        }
        if self.is_running() {
            self.stop_zapret();
            return;
        }
        if self.busy {
            return;
        }
        let cached = self.test.best.clone().and_then(|n| self.configs.iter().position(|c| c.name == n));
        if let Some(i) = cached {
            self.start_config(i);
            return;
        }
        self.test.auto = true;
        let all: Vec<ConfigEntry> = self.configs.clone();
        self.start_tests_for(Kind::Standard, all);
        if self.test.handle.is_none() {
            self.test.auto = false;
        }
    }

    fn start_tests(&mut self, kind: Kind) {
        let selected: Vec<ConfigEntry> =
            self.configs.iter().filter(|c| self.test.checked.contains(&c.name)).cloned().collect();
        self.start_tests_for(kind, selected);
    }

    fn start_tests_for(&mut self, kind: Kind, selected: Vec<ConfigEntry>) {
        let Some(data) = self.data.clone() else { return };
        if selected.is_empty() {
            self.log("Выберите хотя бы одну конфигурацию для теста");
            return;
        }
        let resume = if self.is_running() && !self.test.auto {
            self.configs.iter().find(|c| c.name == self.settings.last_config).cloned()
        } else {
            None
        };
        self.test.reports.clear();
        self.test.saved = None;
        self.test.ran_kind = Some(kind);
        self.test.target_progress = (0, 0);
        let params = Params {
            kind,
            parallel: self.settings.test_parallel,
            timeout: self.settings.test_timeout,
            dpi_range: self.settings.dpi_range,
        };
        self.log(format!("Запуск теста: {} конфигураций", selected.len()));
        self.test.handle = Some(tester::spawn(data, selected, params, resume, self.ctx.clone()));
    }

    fn start_site_check(&mut self) {
        let Some(data) = self.data.clone() else { return };
        let domains = sites::parse_input(&self.sites.input);
        if domains.is_empty() {
            self.log("Введите домены для проверки (например, youtube.com)");
            return;
        }
        let pause = (self.sites.pause_zapret && self.is_running()).then(|| sites::Pause {
            data,
            resume: self.configs.iter().find(|c| c.name == self.settings.last_config).cloned(),
        });
        self.sites.results.clear();
        self.sites.expected = domains.len();
        self.sites.handle = Some(sites::spawn_check(
            domains,
            self.settings.site_parallel,
            self.settings.site_timeout,
            pause,
            self.ctx.clone(),
        ));
    }

    fn reload_user_list(&mut self) {
        if let Some(d) = &self.data {
            self.sites.user_list = sites::read_list(d, self.sites.kind);
        }
    }

    fn tab_home(&mut self, ui: &mut egui::Ui) {
        let Some(_data) = self.data.clone() else {
            card(ui, |ui| {
                ui.colored_label(RED, "Папка data (с general*.bat и lists) не найдена.");
                ui.label("Укажите её в разделе «Настройки».");
                if ui.button("Открыть настройки").clicked() {
                    self.tab = Tab::Settings;
                }
            });
            return;
        };
        let testing = self.test.handle.is_some();
        let running = self.is_running();
        let auto = self.test.auto && testing;
        let state = if auto || self.busy {
            Power::Working
        } else if running {
            Power::On
        } else {
            Power::Off
        };
        let can_power = self.elevated && !self.configs.is_empty() && (auto || !testing);
        let mut start: Option<usize> = None;
        let mut start_test: Option<Kind> = None;
        let mut stop_test = false;
        let mut toggle = false;

        egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
            ui.vertical_centered(|ui| {
                ui.add_space(14.0);
                if power_button(ui, state, can_power).clicked() && can_power {
                    toggle = true;
                }
                ui.add_space(6.0);
                let (title, color) = match state {
                    Power::On => ("Подключено", GREEN),
                    Power::Working if auto => ("Подбор конфигурации…", ACCENT),
                    Power::Working => ("Подождите…", ACCENT),
                    Power::Off => ("Отключено", GRAY),
                };
                ui.label(RichText::new(title).size(24.0).strong().color(color));
                let sub = if auto {
                    let (idx, total) = self.test.config_index;
                    format!("Проверяем «{}» — {}/{}", self.test.current, idx + 1, total)
                } else if running && !self.settings.last_config.is_empty() {
                    format!("Конфигурация: {}", self.settings.last_config)
                } else if !self.elevated {
                    "Нужны права администратора (root)".into()
                } else {
                    "Нажмите на кнопку — программа сама подберёт рабочую конфигурацию".into()
                };
                ui.label(RichText::new(sub).color(GRAY));
                if auto {
                    let (idx, total) = self.test.config_index;
                    let (done, targets) = self.test.target_progress;
                    let inner = if targets > 0 { done as f32 / targets as f32 } else { 0.0 };
                    let frac = (idx as f32 + inner) / total.max(1) as f32;
                    ui.add(egui::ProgressBar::new(frac).desired_width(280.0));
                    ui.label(RichText::new("Нажмите на кнопку ещё раз, чтобы отменить").small().color(GRAY));
                }
                if self.service_conflict.load(Ordering::Relaxed) {
                    ui.colored_label(YELLOW, "Запущена служба zapret — удалите её, иначе проверка будет некорректной");
                }
                if let Err(e) = zapret::engine_path(self.data.as_deref().unwrap()) {
                    ui.colored_label(RED, e);
                }
                ui.add_space(10.0);
            });

            // ---- конфигурации и тест ----
            card(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new(format!("Конфигурации ({})", self.configs.len())).strong().size(16.0));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if testing && !auto {
                            if ui.button("■ Остановить").clicked() {
                                stop_test = true;
                            }
                        } else if !testing {
                            let n = self.test.checked.len();
                            let label = if n > 0 { format!("▶ Тест ({n})") } else { "▶ Тест всех".to_string() };
                            if ui.add_enabled(self.elevated && !self.busy, accent_button(label)).clicked() {
                                start_test = Some(Kind::Standard);
                            }
                            ui.menu_button("⋯", |ui| {
                                if ui.button("DPI-тест (заморозка 16–20 КБ)").clicked() {
                                    start_test = Some(Kind::Dpi);
                                    ui.close();
                                }
                                if ui.button("Выбрать все").clicked() {
                                    self.test.checked = self.configs.iter().map(|c| c.name.clone()).collect();
                                    ui.close();
                                }
                                if ui.button("Снять выбор").clicked() {
                                    self.test.checked.clear();
                                    ui.close();
                                }
                            });
                        }
                    });
                });
                if testing && !auto {
                    let (idx, total) = self.test.config_index;
                    let (done, targets) = self.test.target_progress;
                    let inner = if targets > 0 { done as f32 / targets as f32 } else { 0.0 };
                    let frac = (idx as f32 + inner) / total.max(1) as f32;
                    ui.add(egui::ProgressBar::new(frac).text(format!(
                        "{} — {}/{}, цели {done}/{targets}",
                        self.test.current,
                        idx + 1,
                        total
                    )));
                }
                if !self.elevated {
                    ui.colored_label(RED, "Для тестов нужны права администратора (root)");
                }
                if let Some(path) = &self.test.saved {
                    ui.label(RichText::new(format!("Отчёт: {path}")).color(GRAY).small());
                }
                ui.separator();

                let kind = self.test.ran_kind.unwrap_or(Kind::Standard);
                let best = self.test.best.clone();
                let mut order: Vec<usize> = (0..self.configs.len()).collect();
                let rank_of = |name: &str, reports: &[Report]| reports.iter().find(|r| r.config == name).map(|r| r.score());
                order.sort_by_key(|&i| Reverse(rank_of(&self.configs[i].name, &self.test.reports)));
                let idle = !testing && !self.busy;
                for i in order {
                    let name = self.configs[i].name.clone();
                    let active = running && name == self.settings.last_config;
                    let report = self.test.reports.iter().find(|r| r.config == name);
                    ui.horizontal(|ui| {
                        let mut on = self.test.checked.contains(&name);
                        if ui.add_enabled(!testing, egui::Checkbox::without_text(&mut on)).changed() {
                            if on {
                                self.test.checked.insert(name.clone());
                            } else {
                                self.test.checked.remove(&name);
                            }
                        }
                        let text = RichText::new(&name).color(if active { GREEN } else { Color32::WHITE });
                        ui.label(if active { text.strong() } else { text });
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.add_enabled(idle && self.elevated, egui::Button::new("▶").small()).on_hover_text("Запустить эту конфигурацию").clicked() {
                                start = Some(i);
                            }
                            if let Some(r) = report {
                                if let Some(_) = &r.error {
                                    badge(ui, "ошибка", RED);
                                } else {
                                    let s = r.summary();
                                    let (txt, col) = match kind {
                                        Kind::Standard => (format!("{} OK · {} ERR · ping {}/{}", s.ok, s.fail, s.ping_ok, s.ping_ok + s.ping_fail), if s.fail == 0 { GREEN } else if s.ok > s.fail { YELLOW } else { RED }),
                                        Kind::Dpi => (format!("{} OK · {} FAIL · {} BLOCK", s.ok, s.fail, s.blocked), if s.fail + s.blocked == 0 { GREEN } else if s.ok > s.fail { YELLOW } else { RED }),
                                    };
                                    badge(ui, txt, col);
                                }
                            } else if testing && self.test.current == name {
                                ui.spinner();
                            }
                            if best.as_deref() == Some(name.as_str()) {
                                badge(ui, "★ лучшая", GREEN);
                            }
                        });
                    });
                }
            });

            if !self.test.reports.is_empty() {
                ui.add_space(4.0);
                egui::CollapsingHeader::new(RichText::new("Подробные результаты").color(GRAY)).default_open(false).show(ui, |ui| {
                    let mut order: Vec<usize> = (0..self.test.reports.len()).collect();
                    order.sort_by_key(|&i| Reverse(self.test.reports[i].score()));
                    for i in order {
                        let r = &self.test.reports[i];
                        if let Some(err) = &r.error {
                            ui.colored_label(RED, format!("✖ {} — не запустилась: {err}", r.config));
                            continue;
                        }
                        egui::CollapsingHeader::new(&r.config).id_salt(("report", &r.config)).show(ui, |ui| report_details(ui, r));
                    }
                });
            }
            ui.add_space(8.0);
        });

        if toggle {
            self.toggle_power();
        }
        if stop_test {
            if let Some(h) = &self.test.handle {
                h.cancel.store(true, Ordering::Relaxed);
            }
        }
        if let Some(kind) = start_test {
            if self.test.checked.is_empty() {
                self.test.checked = self.configs.iter().map(|c| c.name.clone()).collect();
            }
            self.start_tests(kind);
        }
        if let Some(i) = start {
            self.start_config(i);
        }
    }

    fn sites_section(&mut self, ui: &mut egui::Ui) {
        let Some(data) = self.data.clone() else {
            ui.colored_label(RED, "Папка data не найдена");
            return;
        };
        let checking = self.sites.handle.is_some();
        let mut check = false;
        let mut add_selected = false;
        let mut remove: Option<String> = None;
        let mut kind_changed = false;

        ui.label(RichText::new("Свои списки сайтов для zapret").color(GRAY));
        ui.horizontal(|ui| {
            kind_changed |= ui.selectable_value(&mut self.sites.kind, ListKind::Proxy, "Обходить").changed();
            kind_changed |= ui.selectable_value(&mut self.sites.kind, ListKind::Exclude, "Исключения").changed();
        });
        ui.label(
            RichText::new(match self.sites.kind {
                ListKind::Proxy => "list-general-user.txt — сайты, трафик которых обрабатывает zapret",
                ListKind::Exclude => "list-exclude-user.txt — сайты, которые zapret не трогает",
            })
            .color(GRAY)
            .small(),
        );
        egui::ScrollArea::vertical().id_salt("user_list").max_height(130.0).auto_shrink([false, true]).show(ui, |ui| {
            for d in &self.sites.user_list {
                ui.horizontal(|ui| {
                    if ui.small_button("✖").clicked() {
                        remove = Some(d.clone());
                    }
                    ui.label(d);
                });
            }
            if self.sites.user_list.is_empty() {
                ui.label(RichText::new("Список пуст").color(GRAY));
            }
        });
        ui.separator();
        ui.label("Проверка доступности (домены через пробел, запятую или с новой строки):");
        ui.add_enabled(
            !checking,
            egui::TextEdit::multiline(&mut self.sites.input)
                .desired_rows(3)
                .desired_width(f32::INFINITY)
                .hint_text("youtube.com\ndiscord.com\nhttps://example.org/page"),
        );
        ui.horizontal_wrapped(|ui| {
            if !checking {
                if ui.add(accent_button("▶ Проверить")).clicked() {
                    check = true;
                }
            } else if ui.button("■ Остановить").clicked() {
                if let Some(h) = &self.sites.handle {
                    h.cancel.store(true, Ordering::Relaxed);
                }
            }
            ui.add_enabled_ui(!checking, |ui| {
                ui.checkbox(&mut self.sites.pause_zapret, "Без zapret (временно остановить)");
            });
        });
        ui.horizontal(|ui| {
            ui.label("Потоков:");
            self.dirty |= ui.add_enabled(!checking, egui::Slider::new(&mut self.settings.site_parallel, 1..=64)).changed();
        });
        if checking {
            let done = self.sites.results.len();
            let frac = done as f32 / self.sites.expected.max(1) as f32;
            ui.add(egui::ProgressBar::new(frac).text(format!("{done}/{}", self.sites.expected)));
        }
        if !self.sites.results.is_empty() {
            ui.horizontal_wrapped(|ui| {
                if ui.button("Недоступные").clicked() {
                    for (r, sel) in &mut self.sites.results {
                        *sel = !r.ok;
                    }
                }
                if ui.button("Все").clicked() {
                    for (_, sel) in &mut self.sites.results {
                        *sel = true;
                    }
                }
                if ui.button("Снять").clicked() {
                    for (_, sel) in &mut self.sites.results {
                        *sel = false;
                    }
                }
                let n = self.sites.results.iter().filter(|(_, s)| *s).count();
                if ui.add_enabled(n > 0, egui::Button::new(format!("➕ Добавить выбранные ({n})"))).clicked() {
                    add_selected = true;
                }
            });
        }
        if self.sites.restart_hint {
            ui.horizontal(|ui| {
                ui.colored_label(YELLOW, "Список изменён — перезапустите zapret.");
                if self.is_running() && ui.button("⟳ Перезапустить").clicked() {
                    self.restart_zapret();
                }
            });
        }
        egui::ScrollArea::vertical().id_salt("site_results_scroll").max_height(240.0).auto_shrink([false, true]).show(ui, |ui| {
            egui::Grid::new("site_results").striped(true).num_columns(4).show(ui, |ui| {
                for (r, sel) in &mut self.sites.results {
                    ui.checkbox(sel, "");
                    ui.label(&r.domain);
                    ui.colored_label(if r.ok { GREEN } else { RED }, if r.ok { "доступен" } else { "недоступен" });
                    ui.label(RichText::new(format!("{} мс · {}", r.ms, r.detail)).color(GRAY));
                    ui.end_row();
                }
            });
        });

        if kind_changed {
            self.reload_user_list();
        }
        if let Some(d) = remove {
            if let Err(e) = sites::remove_domain(&data, self.sites.kind, &d) {
                self.log(format!("Ошибка записи списка: {e}"));
            }
            self.reload_user_list();
            self.sites.restart_hint = true;
        }
        if check {
            self.start_site_check();
        }
        if add_selected {
            let chosen: Vec<String> = self.sites.results.iter().filter(|(_, s)| *s).map(|(r, _)| r.domain.clone()).collect();
            match sites::add_domains(&data, self.sites.kind, &chosen) {
                Ok(n) => self.log(format!("В список добавлено доменов: {n}")),
                Err(e) => self.log(format!("Ошибка записи списка: {e}")),
            }
            self.reload_user_list();
            self.sites.restart_hint = true;
        }
    }

    fn tab_settings(&mut self, ui: &mut egui::Ui) {
        egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
            card(ui, |ui| {
                ui.label(RichText::new("Общие").strong().size(16.0));
                ui.label("Папка data (с general*.bat, lists, bin)");
                ui.horizontal(|ui| {
                    let r = ui.add(egui::TextEdit::singleline(&mut self.settings.data_dir).desired_width(ui.available_width() - 100.0).hint_text("автоопределение"));
                    if r.changed() {
                        self.dirty = true;
                    }
                    if ui.button("Применить").clicked() {
                        self.data = zapret::find_data_dir(&self.settings.data_dir);
                        self.configs = self.data.as_deref().map(zapret::list_configs).unwrap_or_default();
                        self.selected = self.configs.iter().position(|c| c.name == self.settings.last_config);
                        self.test.best = None;
                        self.reload_user_list();
                    }
                });
                match &self.data {
                    Some(d) => ui.colored_label(GREEN, format!("Найдена: {}", d.display())),
                    None => ui.colored_label(RED, "Не найдена"),
                };
                self.dirty |= ui.checkbox(&mut self.settings.autostart_last_config, "Запускать последнюю конфигурацию при старте программы").changed();
                if self.elevated {
                    ui.colored_label(GREEN, "Права администратора: есть");
                } else {
                    ui.colored_label(RED, "Права администратора: нет — движок не запустится");
                }
            });
            ui.add_space(4.0);
            card(ui, |ui| {
                ui.label(RichText::new("Тестирование конфигураций").strong().size(16.0));
                ui.horizontal(|ui| {
                    ui.label("Параллельно:");
                    self.dirty |= ui.add(egui::Slider::new(&mut self.settings.test_parallel, 1..=32)).changed();
                });
                ui.horizontal(|ui| {
                    ui.label("Таймаут, с:");
                    self.dirty |= ui.add(egui::Slider::new(&mut self.settings.test_timeout, 2..=15)).changed();
                });
                ui.horizontal(|ui| {
                    ui.label("Объём DPI-запроса, байт:");
                    self.dirty |= ui.add(egui::DragValue::new(&mut self.settings.dpi_range).range(1024..=1_048_576).speed(1024)).changed();
                });
                ui.horizontal(|ui| {
                    ui.label("Таймаут проверки сайтов, с:");
                    self.dirty |= ui.add(egui::Slider::new(&mut self.settings.site_timeout, 2..=20)).changed();
                });
            });
            ui.add_space(4.0);
            card(ui, |ui| {
                egui::CollapsingHeader::new(RichText::new("Сайты").strong().size(16.0)).default_open(true).show(ui, |ui| self.sites_section(ui));
            });
        });
    }
    fn tab_telegram(&mut self, ui: &mut egui::Ui) {
        let mut changed = false;
        let t = &mut self.settings.telegram;
        ui.label("Telegram Desktop подключается к прокси через ссылку tg://. При первом применении Telegram покажет окно подтверждения — нажмите «Включить».");
        ui.add_space(6.0);
        ui.horizontal(|ui| {
            ui.label("Тип:");
            changed |= ui.selectable_value(&mut t.kind, ProxyKind::Socks5, "SOCKS5").changed();
            changed |= ui.selectable_value(&mut t.kind, ProxyKind::MtProto, "MTProto").changed();
        });
        egui::Grid::new("tg_grid").num_columns(2).spacing([10.0, 6.0]).show(ui, |ui| {
            ui.label("Адрес:");
            changed |= ui.text_edit_singleline(&mut t.host).changed();
            ui.end_row();
            ui.label("Порт:");
            changed |= ui.add(egui::DragValue::new(&mut t.port).range(1..=65535)).changed();
            ui.end_row();
            match t.kind {
                ProxyKind::Socks5 => {
                    ui.label("Логин:");
                    changed |= ui.text_edit_singleline(&mut t.user).changed();
                    ui.end_row();
                    ui.label("Пароль:");
                    changed |= ui.add(egui::TextEdit::singleline(&mut t.pass).password(true)).changed();
                    ui.end_row();
                }
                ProxyKind::MtProto => {
                    ui.label("Secret:");
                    changed |= ui.text_edit_singleline(&mut t.secret).changed();
                    ui.end_row();
                }
            }
        });
        ui.add_space(6.0);
        changed |= ui.checkbox(&mut t.on_zapret_start, "Подключать Telegram к прокси при запуске zapret").changed();
        changed |= ui.checkbox(&mut t.on_app_start, "Подключать Telegram к прокси при запуске программы (если zapret не запускается автоматически)").changed();
        self.dirty |= changed;

        let t = self.settings.telegram.clone();
        ui.add_space(6.0);
        ui.horizontal(|ui| {
            if ui.add_enabled(self.tg.probe.is_none(), egui::Button::new("Проверить прокси")).clicked() {
                let (tx, rx) = channel();
                let ctx = self.ctx.clone();
                let t = t.clone();
                std::thread::spawn(move || {
                    let _ = tx.send(telegram::probe(&t));
                    ctx.request_repaint();
                });
                self.tg.probe = Some(rx);
                self.tg.probe_result = None;
            }
            if ui.button("Подключить Telegram сейчас").clicked() {
                let r = telegram::apply(&t);
                let r = tg_result(r);
                self.log(r.message);
            }
            if self.tg.probe.is_some() {
                ui.spinner();
            }
        });
        match &self.tg.probe_result {
            Some(Ok(ms)) => ui.colored_label(GREEN, format!("Прокси отвечает ({ms} мс)")),
            Some(Err(e)) => ui.colored_label(RED, e),
            None => ui.label(""),
        };
        if let Ok(link) = telegram::link(&t) {
            ui.add_space(6.0);
            ui.label(RichText::new("Ссылка:").color(GRAY));
            ui.horizontal(|ui| {
                ui.label(RichText::new(&link).monospace());
                if ui.small_button("Копировать").clicked() {
                    ui.ctx().copy_text(link.clone());
                }
            });
        }
    }
}

fn tg_result(r: Result<(), String>) -> OpResult {
    match r {
        Ok(()) => OpResult { message: "Telegram: ссылка прокси отправлена (подтвердите в окне Telegram)".into(), error: false, started: None },
        Err(e) => OpResult { message: format!("Telegram: {e}"), error: true, started: None },
    }
}

fn check_color(c: Check) -> Color32 {
    match c {
        Check::Ok => GREEN,
        Check::Error | Check::Ssl => RED,
    }
}

fn report_details(ui: &mut egui::Ui, r: &Report) {
    let standard = r.outcomes.iter().any(|o| matches!(o, Outcome::Std(_)));
    if standard {
        egui::Grid::new(("grid", &r.config)).striped(true).show(ui, |ui| {
            for o in &r.outcomes {
                let Outcome::Std(s) = o else { continue };
                ui.label(&s.name);
                for (label, c) in &s.checks {
                    ui.label(RichText::new(format!("{label}:{}", c.label())).color(check_color(*c)).monospace());
                }
                if s.checks.is_empty() {
                    for _ in 0..3 {
                        ui.label("");
                    }
                }
                match s.ping {
                    Some(ms) => ui.label(RichText::new(format!("{ms} мс")).color(GREEN)),
                    None => ui.label(RichText::new("timeout").color(YELLOW)),
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
        ui.label(
            RichText::new(format!("[{}] {} ({}) — {}", d.country, d.id, d.provider, parts.join(" ")))
                .color(if blocked { RED } else { YELLOW }),
        );
    }
    ui.label(RichText::new(format!("Целей без проблем: {good}")).color(GREEN));
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.poll();

        egui::Panel::top("top").frame(egui::Frame::new().fill(BG).inner_margin(egui::Margin::symmetric(16, 10))).show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new("Zapret").size(20.0).strong());
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    for (tab, name) in [(Tab::Settings, "Настройки"), (Tab::Telegram, "Telegram"), (Tab::Home, "Главная")] {
                        let sel = self.tab == tab;
                        let b = egui::Button::new(RichText::new(name).color(if sel { Color32::WHITE } else { GRAY }))
                            .fill(if sel { ACCENT } else { Color32::TRANSPARENT });
                        if ui.add(b).clicked() {
                            self.tab = tab;
                        }
                    }
                });
            });
        });

        egui::Panel::bottom("log").frame(egui::Frame::new().fill(Color32::from_rgb(12, 14, 20)).inner_margin(egui::Margin::symmetric(16, 6))).show(ui, |ui| {
            ui.horizontal(|ui| {
                let arrow = if self.show_log { "▾" } else { "▸" };
                if ui.add(egui::Button::new(RichText::new(format!("{arrow} Журнал")).color(GRAY)).frame(false)).clicked() {
                    self.show_log = !self.show_log;
                }
                if !self.show_log {
                    if let Some(last) = self.log.last() {
                        ui.label(RichText::new(last).monospace().small().color(GRAY));
                    }
                }
            });
            if self.show_log {
                egui::ScrollArea::vertical().max_height(130.0).stick_to_bottom(true).auto_shrink([false, true]).show(ui, |ui| {
                    for line in &self.log {
                        ui.label(RichText::new(line).monospace().small());
                    }
                });
            }
        });

        egui::CentralPanel::default().frame(egui::Frame::new().fill(BG).inner_margin(egui::Margin::symmetric(16, 4))).show(ui, |ui| {
            let w = ui.available_width().min(640.0);
            let pad = (ui.available_width() - w) / 2.0;
            ui.horizontal_top(|ui| {
                ui.add_space(pad);
                ui.vertical(|ui| {
                    ui.set_width(w);
                    match self.tab {
                        Tab::Home => self.tab_home(ui),
                        Tab::Telegram => egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| card(ui, |ui| self.tab_telegram(ui))).inner,
                        Tab::Settings => self.tab_settings(ui),
                    }
                });
            });
        });

        if self.dirty {
            self.settings.save();
            self.dirty = false;
        }
    }
}

impl Drop for App {
    fn drop(&mut self) {
        self.settings.save();
        zapret::stop();
    }
}
