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

const GREEN: Color32 = Color32::from_rgb(80, 200, 120);
const RED: Color32 = Color32::from_rgb(235, 90, 90);
const YELLOW: Color32 = Color32::from_rgb(230, 190, 70);
const GRAY: Color32 = Color32::from_rgb(150, 150, 150);

#[derive(Clone, Copy, PartialEq, Eq)]
enum Tab {
    Control,
    Tests,
    Sites,
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
            tab: Tab::Control,
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
                    self.log(if cancelled { "Тест остановлен" } else { "Тест завершён" });
                }
            }
        }
        if finished {
            self.test.handle = None;
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

    fn start_tests(&mut self, kind: Kind) {
        let Some(data) = self.data.clone() else { return };
        let selected: Vec<ConfigEntry> =
            self.configs.iter().filter(|c| self.test.checked.contains(&c.name)).cloned().collect();
        if selected.is_empty() {
            self.log("Выберите хотя бы одну конфигурацию для теста");
            return;
        }
        let resume = if self.is_running() {
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

    fn tab_control(&mut self, ui: &mut egui::Ui) {
        let Some(data) = self.data.clone() else {
            ui.colored_label(RED, "Папка data (с general*.bat и lists) не найдена. Укажите её во вкладке «Настройки».");
            return;
        };
        ui.label(RichText::new(format!("Папка: {}", data.display())).color(GRAY));
        if let Err(e) = zapret::engine_path(&data) {
            ui.colored_label(RED, e);
        }
        ui.add_space(4.0);

        let mut start = None;
        let (mut stop, mut restart) = (false, false);
        ui.horizontal(|ui| {
            let can = !self.busy && self.selected.is_some();
            if ui.add_enabled(can, egui::Button::new("▶ Запустить")).clicked() {
                start = self.selected;
            }
            if ui.add_enabled(!self.busy, egui::Button::new("■ Остановить")).clicked() {
                stop = true;
            }
            if ui.add_enabled(!self.busy && self.is_running(), egui::Button::new("⟳ Перезапустить")).clicked() {
                restart = true;
            }
            if self.busy {
                ui.spinner();
            }
        });
        self.dirty |= ui.checkbox(&mut self.settings.autostart_last_config, "Запускать последнюю конфигурацию при старте программы").changed();
        ui.separator();

        ui.label(format!("Конфигурации ({}):", self.configs.len()));
        egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
            for (i, cfg) in self.configs.iter().enumerate() {
                let active = self.is_running() && cfg.name == self.settings.last_config;
                let text = if active { format!("● {}", cfg.name) } else { cfg.name.clone() };
                let r = ui.selectable_label(self.selected == Some(i), text);
                if r.clicked() {
                    self.selected = Some(i);
                }
                if r.double_clicked() && !self.busy {
                    start = Some(i);
                }
            }
        });

        if let Some(i) = start {
            self.start_config(i);
        } else if restart {
            self.restart_zapret();
        } else if stop {
            self.stop_zapret();
        }
    }

    fn tab_tests(&mut self, ui: &mut egui::Ui) {
        if self.data.is_none() {
            ui.colored_label(RED, "Папка data не найдена");
            return;
        }
        let testing = self.test.handle.is_some();
        let mut start = None;
        let mut stop_test = false;
        let mut run_cfg: Option<String> = None;

        egui::Panel::left("test_configs").resizable(true).default_size(250.0).show(ui, |ui| {
            ui.label(RichText::new("Конфигурации для теста").strong());
            ui.horizontal(|ui| {
                if ui.add_enabled(!testing, egui::Button::new("Все")).clicked() {
                    self.test.checked = self.configs.iter().map(|c| c.name.clone()).collect();
                }
                if ui.add_enabled(!testing, egui::Button::new("Ничего")).clicked() {
                    self.test.checked.clear();
                }
                ui.label(format!("выбрано {}", self.test.checked.len()));
            });
            egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
                ui.add_enabled_ui(!testing, |ui| {
                    for cfg in &self.configs {
                        let mut on = self.test.checked.contains(&cfg.name);
                        if ui.checkbox(&mut on, &cfg.name).changed() {
                            if on {
                                self.test.checked.insert(cfg.name.clone());
                            } else {
                                self.test.checked.remove(&cfg.name);
                            }
                        }
                    }
                });
            });
        });

        egui::CentralPanel::default().show(ui, |ui| {
            ui.add_enabled_ui(!testing, |ui| {
                ui.horizontal(|ui| {
                    ui.label("Параллельно:");
                    self.dirty |= ui.add(egui::Slider::new(&mut self.settings.test_parallel, 1..=32)).changed();
                    ui.label("Таймаут, с:");
                    self.dirty |= ui.add(egui::Slider::new(&mut self.settings.test_timeout, 2..=15)).changed();
                });
            });
            ui.horizontal(|ui| {
                if !testing {
                    if ui.add_enabled(self.elevated, egui::Button::new("▶ Тест: HTTP / TLS / ping")).clicked() {
                        start = Some(Kind::Standard);
                    }
                    if ui.add_enabled(self.elevated, egui::Button::new("▶ Тест: DPI (заморозка 16-20 КБ)")).clicked() {
                        start = Some(Kind::Dpi);
                    }
                } else if ui.button("■ Остановить тест").clicked() {
                    stop_test = true;
                }
            });
            if !self.elevated {
                ui.colored_label(RED, "Для тестов нужны права администратора (root)");
            }
            if self.service_conflict.load(Ordering::Relaxed) {
                ui.colored_label(YELLOW, "Установлена и запущена служба zapret — удалите её, иначе тест будет некорректным");
            }
            if testing {
                let (idx, total) = self.test.config_index;
                let (done, targets) = self.test.target_progress;
                let inner = if targets > 0 { done as f32 / targets as f32 } else { 0.0 };
                let frac = (idx as f32 + inner) / total.max(1) as f32;
                ui.add(egui::ProgressBar::new(frac).text(format!(
                    "{} — конфиг {}/{}, цели {done}/{targets}",
                    self.test.current,
                    idx + 1,
                    total
                )));
            }
            ui.separator();
            self.test_results(ui, &mut run_cfg);
        });

        if stop_test {
            if let Some(h) = &self.test.handle {
                h.cancel.store(true, Ordering::Relaxed);
            }
        }
        if let Some(kind) = start {
            self.start_tests(kind);
        }
        if let Some(name) = run_cfg {
            if let Some(i) = self.configs.iter().position(|c| c.name == name) {
                self.start_config(i);
            }
        }
    }

    fn test_results(&mut self, ui: &mut egui::Ui, run_cfg: &mut Option<String>) {
        if self.test.reports.is_empty() {
            ui.label(RichText::new("Результатов пока нет").color(GRAY));
            return;
        }
        let kind = self.test.ran_kind.unwrap_or(Kind::Standard);
        let mut order: Vec<usize> = (0..self.test.reports.len()).collect();
        order.sort_by_key(|&i| Reverse(self.test.reports[i].score()));

        if let Some(&best) = order.iter().find(|&&i| self.test.reports[i].error.is_none()) {
            let r = &self.test.reports[best];
            ui.horizontal(|ui| {
                ui.label(RichText::new(format!("Лучшая: {}", r.config)).color(GREEN).strong());
                if ui.add_enabled(self.test.handle.is_none() && !self.busy, egui::Button::new("▶ Запустить лучшую")).clicked() {
                    *run_cfg = Some(r.config.clone());
                }
            });
        }
        if let Some(path) = &self.test.saved {
            ui.label(RichText::new(format!("Отчёт: {path}")).color(GRAY).small());
        }

        egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
            for (rank, &i) in order.iter().enumerate() {
                let r = &self.test.reports[i];
                if let Some(err) = &r.error {
                    ui.colored_label(RED, format!("✖ {} — не запустилась: {err}", r.config));
                    continue;
                }
                let s = r.summary();
                let title = match kind {
                    Kind::Standard => format!(
                        "#{}  {}   OK {} | ERR {} | ping {}/{}",
                        rank + 1, r.config, s.ok, s.fail, s.ping_ok, s.ping_ok + s.ping_fail
                    ),
                    Kind::Dpi => format!(
                        "#{}  {}   OK {} | FAIL {} | BLOCKED {}",
                        rank + 1, r.config, s.ok, s.fail, s.blocked
                    ),
                };
                let color = if s.fail + s.blocked == 0 { GREEN } else if s.ok > s.fail { YELLOW } else { RED };
                egui::CollapsingHeader::new(RichText::new(title).color(color))
                    .id_salt(("report", &r.config))
                    .show(ui, |ui| {
                        if ui.add_enabled(self.test.handle.is_none() && !self.busy, egui::Button::new("▶ Запустить")).clicked() {
                            *run_cfg = Some(r.config.clone());
                        }
                        report_details(ui, r);
                    });
            }
        });
    }

    fn tab_sites(&mut self, ui: &mut egui::Ui) {
        let Some(data) = self.data.clone() else {
            ui.colored_label(RED, "Папка data не найдена");
            return;
        };
        let checking = self.sites.handle.is_some();
        let mut check = false;
        let mut add_selected = false;
        let mut remove: Option<String> = None;
        let mut kind_changed = false;

        egui::Panel::right("sites_list").resizable(true).default_size(280.0).show(ui, |ui| {
            ui.label(RichText::new("Список для zapret").strong());
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
            egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
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
        });

        egui::CentralPanel::default().show(ui, |ui| {
            ui.label("Домены для проверки (через пробел, запятую или с новой строки):");
            ui.add_enabled(
                !checking,
                egui::TextEdit::multiline(&mut self.sites.input)
                    .desired_rows(4)
                    .desired_width(f32::INFINITY)
                    .hint_text("youtube.com\ndiscord.com\nhttps://example.org/page"),
            );
            ui.horizontal(|ui| {
                if !checking {
                    if ui.button("▶ Проверить параллельно").clicked() {
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
                ui.label("Потоков:");
                self.dirty |= ui.add_enabled(!checking, egui::Slider::new(&mut self.settings.site_parallel, 1..=64)).changed();
            });
            if checking {
                let done = self.sites.results.len();
                let frac = done as f32 / self.sites.expected.max(1) as f32;
                ui.add(egui::ProgressBar::new(frac).text(format!("{done}/{}", self.sites.expected)));
            }
            ui.separator();

            if !self.sites.results.is_empty() {
                ui.horizontal(|ui| {
                    if ui.button("Выбрать недоступные").clicked() {
                        for (r, sel) in &mut self.sites.results {
                            *sel = !r.ok;
                        }
                    }
                    if ui.button("Выбрать все").clicked() {
                        for (_, sel) in &mut self.sites.results {
                            *sel = true;
                        }
                    }
                    if ui.button("Снять выбор").clicked() {
                        for (_, sel) in &mut self.sites.results {
                            *sel = false;
                        }
                    }
                    let n = self.sites.results.iter().filter(|(_, s)| *s).count();
                    if ui.add_enabled(n > 0, egui::Button::new(format!("➕ Добавить выбранные ({n}) в список"))).clicked() {
                        add_selected = true;
                    }
                });
            }
            if self.sites.restart_hint {
                ui.horizontal(|ui| {
                    ui.colored_label(YELLOW, "Список изменён — перезапустите zapret, чтобы применить.");
                    if self.is_running() && ui.button("⟳ Перезапустить").clicked() {
                        self.restart_zapret();
                    }
                });
            }
            egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
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

    fn tab_settings(&mut self, ui: &mut egui::Ui) {
        ui.label(RichText::new("Папка data (с general*.bat, lists, bin)").strong());
        ui.horizontal(|ui| {
            let r = ui.add(egui::TextEdit::singleline(&mut self.settings.data_dir).desired_width(420.0).hint_text("автоопределение"));
            if r.changed() {
                self.dirty = true;
            }
            if ui.button("Применить").clicked() {
                self.data = zapret::find_data_dir(&self.settings.data_dir);
                self.configs = self.data.as_deref().map(zapret::list_configs).unwrap_or_default();
                self.selected = self.configs.iter().position(|c| c.name == self.settings.last_config);
                self.reload_user_list();
            }
        });
        match &self.data {
            Some(d) => ui.colored_label(GREEN, format!("Найдена: {}", d.display())),
            None => ui.colored_label(RED, "Не найдена"),
        };
        ui.separator();
        ui.horizontal(|ui| {
            ui.label("Объём DPI-запроса, байт:");
            self.dirty |= ui.add(egui::DragValue::new(&mut self.settings.dpi_range).range(1024..=1_048_576).speed(1024)).changed();
        });
        ui.horizontal(|ui| {
            ui.label("Таймаут проверки сайтов, с:");
            self.dirty |= ui.add(egui::Slider::new(&mut self.settings.site_timeout, 2..=20)).changed();
        });
        ui.separator();
        if self.elevated {
            ui.colored_label(GREEN, "Права администратора: есть");
        } else {
            ui.colored_label(RED, "Права администратора: нет — движок не запустится");
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

        egui::Panel::top("top").show(ui, |ui| {
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                let running = self.is_running();
                let (text, color) = if running { ("● zapret работает", GREEN) } else { ("● zapret остановлен", RED) };
                ui.label(RichText::new(text).color(color).strong());
                if running && !self.settings.last_config.is_empty() {
                    ui.label(RichText::new(format!("({})", self.settings.last_config)).color(GRAY));
                }
                if !self.elevated {
                    ui.colored_label(RED, "· нет прав администратора");
                }
                if self.service_conflict.load(Ordering::Relaxed) {
                    ui.colored_label(YELLOW, "· запущена служба zapret");
                }
            });
            ui.horizontal(|ui| {
                for (tab, name) in [
                    (Tab::Control, "Управление"),
                    (Tab::Tests, "Тест конфигов"),
                    (Tab::Sites, "Сайты"),
                    (Tab::Telegram, "Telegram"),
                    (Tab::Settings, "Настройки"),
                ] {
                    ui.selectable_value(&mut self.tab, tab, name);
                }
            });
            ui.add_space(2.0);
        });

        egui::Panel::bottom("log").resizable(true).default_size(110.0).show(ui, |ui| {
            egui::ScrollArea::vertical().stick_to_bottom(true).auto_shrink([false, false]).show(ui, |ui| {
                for line in &self.log {
                    ui.label(RichText::new(line).monospace().small());
                }
            });
        });

        egui::CentralPanel::default().show(ui, |ui| match self.tab {
            Tab::Control => self.tab_control(ui),
            Tab::Tests => self.tab_tests(ui),
            Tab::Sites => self.tab_sites(ui),
            Tab::Telegram => self.tab_telegram(ui),
            Tab::Settings => self.tab_settings(ui),
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
