use std::cmp::Reverse;
use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, Sender, channel};
use std::time::Duration;

use egui::{
    Align, Align2, Color32, Layout, Margin, Rect, RichText, Sense, Stroke, Ui, UiBuilder, pos2,
    vec2,
};

use crate::settings::{ProxyKind, Settings};
use crate::sites::{self, ListKind};
use crate::tester::{self, Check, DpiStatus, Kind, Outcome, Params, Report};
use crate::theme::{self, *};
use crate::zapret::{self, ConfigEntry};
use crate::{sys, telegram};

const WIDE_BREAKPOINT: f32 = 820.0;
const TWO_COL_MIN: f32 = 760.0;
const NO_PASS_MSG: &str = "Ни одна конфигурация не прошла тест — подключение невозможно";

#[derive(Clone, Copy, PartialEq, Eq)]
enum Tab {
    Home,
    Telegram,
    Settings,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum SettingsPage {
    General,
    Sites,
    Logs,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum LogSource {
    App,
    TgWs,
}

struct OpResult {
    message: String,
    error: bool,
    started: Option<String>,
    start_failed: bool,
}

#[derive(Default)]
struct TestState {
    checked: HashSet<String>,
    handle: Option<tester::Handle>,
    draining: Option<tester::Handle>,
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

#[derive(Default)]
struct Acts {
    toggle: bool,
    select: Option<usize>,
    start_test: Option<Kind>,
    stop_test: bool,
}

struct RowView {
    name: String,
    active: bool,
    selected: bool,
    checked: bool,
    spinner: bool,
    best: bool,
    result: Option<(String, Tone)>,
}

pub struct App {
    ctx: egui::Context,
    settings: Settings,
    dirty: bool,
    data: Option<PathBuf>,
    configs: Vec<ConfigEntry>,
    selected: Option<usize>,
    tab: Tab,
    settings_page: SettingsPage,
    log_source: LogSource,
    tg_log: Vec<String>,
    tg_log_at: Option<std::time::Instant>,
    show_log: bool,
    show_details: bool,
    elevated: bool,
    system_autostart: bool,
    confirm_close: bool,
    #[cfg(windows)]
    _tray: Option<tray_icon::TrayIcon>,
    running: Arc<AtomicBool>,
    service_conflict: Arc<AtomicBool>,
    busy: bool,
    home_error: Option<String>,
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
        egui_extras::install_image_loaders(&ctx);
        theme::apply_style(&ctx);
        let settings = Settings::load();
        let data = zapret::find_data_dir(&settings.data_dir);
        let configs = data
            .as_deref()
            .map(zapret::list_configs)
            .unwrap_or_default();
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
                    let (r, c) = (zapret::is_running(), sys::service_running("zapret"));
                    let changed = running.swap(r, Ordering::Relaxed)
                        != r | (conflict.swap(c, Ordering::Relaxed) != c);
                    if changed {
                        ctx.request_repaint();
                    }
                    std::thread::sleep(Duration::from_secs(3));
                }
            });
        }

        let (op_tx, op_rx) = channel();
        let selected = configs.iter().position(|c| c.name == settings.last_config);
        let user_list = data
            .as_deref()
            .map(|d| sites::read_list(d, ListKind::Proxy))
            .unwrap_or_default();

        let mut app = Self {
            ctx,
            settings,
            dirty: false,
            data,
            configs,
            selected,
            tab: Tab::Home,
            settings_page: SettingsPage::General,
            log_source: LogSource::App,
            tg_log: Vec::new(),
            tg_log_at: None,
            show_log: false,
            show_details: false,
            elevated: sys::has_required_privileges(),
            system_autostart: sys::autostart_supported() && sys::autostart_enabled(),
            confirm_close: false,
            #[cfg(windows)]
            _tray: crate::tray::create(&cc.egui_ctx),
            running,
            service_conflict,
            busy: false,
            home_error: None,
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

    #[cfg(windows)]
    fn handle_close(&mut self, ui: &Ui) {
        let ctx = ui.ctx().clone();
        if self._tray.is_some()
            && !crate::tray::quitting()
            && ctx.input(|i| i.viewport().close_requested())
        {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            self.confirm_close = true;
        }
        if !self.confirm_close {
            return;
        }
        let (mut minimize, mut quit, mut cancel) = (false, false, false);
        let modal = egui::Modal::new(egui::Id::new("close_confirm")).show(&ctx, |ui| {
            ui.set_width(320.0);
            ui.spacing_mut().item_spacing.y = 12.0;
            h_title(ui, "Закрыть приложение?");
            ui.add(
                egui::Label::new(rt(
                    "Можно свернуть в трей — zapret продолжит работать в фоне.",
                    13.0,
                    W::Reg,
                    TEXT2,
                ))
                .selectable(false),
            );
            ui.horizontal(|ui| {
                if button(ui, "Свернуть в трей", Variant::Primary, None, true).clicked() {
                    minimize = true;
                }
                if button(ui, "Закрыть", Variant::Secondary, None, true).clicked() {
                    quit = true;
                }
                if button(ui, "Отмена", Variant::Ghost, None, true).clicked() {
                    cancel = true;
                }
            });
        });
        if modal.should_close() {
            cancel = true;
        }
        if minimize {
            self.confirm_close = false;
            crate::tray::hide(&ctx);
        } else if quit {
            crate::tray::QUITTING.store(true, Ordering::Relaxed);
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        } else if cancel {
            self.confirm_close = false;
        }
    }

    fn is_running(&self) -> bool {
        self.running.load(Ordering::Relaxed)
    }
}

mod chrome;
mod home;
mod ops;
mod settings_tab;
mod telegram_tab;
mod widgets;

use ops::tg_result;
use widgets::*;

impl eframe::App for App {
    fn ui(&mut self, ui: &mut Ui, _frame: &mut eframe::Frame) {
        self.poll();
        #[cfg(windows)]
        self.handle_close(ui);

        let wide = ui.available_width() >= WIDE_BREAKPOINT;
        let state = self.hero_state();

        if let Some(rgb) = state.glow_rgb() {
            top_wash(
                ui.painter(),
                ui.max_rect(),
                if wide { 0.32 } else { 0.5 },
                rgb,
            );
        }

        egui::Panel::bottom("log")
            .frame(egui::Frame::new().fill(INK_1000))
            .show(ui, |ui| {
                self.log_bar(ui, state);
            });

        if wide {
            egui::Panel::left("nav")
                .exact_size(208.0)
                .resizable(false)
                .frame(
                    egui::Frame::new()
                        .fill(BG_CHROME)
                        .inner_margin(Margin::same(12)),
                )
                .show(ui, |ui| {
                    let r = ui.max_rect();
                    ui.painter().rect_filled(
                        Rect::from_min_size(
                            pos2(r.right() + 12.0 - 1.0, r.top() - 12.0),
                            vec2(1.0, r.height() + 24.0),
                        ),
                        0,
                        LINE_SOFT,
                    );
                    self.sidebar(ui, state);
                });
        } else {
            egui::Panel::top("top")
                .frame(egui::Frame::new().inner_margin(Margin {
                    left: 28,
                    right: 28,
                    top: 4,
                    bottom: 12,
                }))
                .show(ui, |ui| {
                    ui.with_layout(Layout::top_down(Align::Center), |ui| {
                        let mut tab = self.tab;
                        segmented(
                            ui,
                            &mut tab,
                            &[
                                (Tab::Home, "Главная"),
                                (Tab::Telegram, "Telegram"),
                                (Tab::Settings, "Настройки"),
                            ],
                            false,
                        );
                        self.tab = tab;
                    });
                });
        }

        egui::CentralPanel::default()
            .frame(egui::Frame::new())
            .show(ui, |ui| match self.tab {
                Tab::Home => self.tab_home(ui, wide),
                Tab::Telegram => self.tab_telegram(ui, wide),
                Tab::Settings => self.tab_settings(ui, wide),
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
        crate::tg_ws::stop();
        zapret::shutdown();
    }
}
