use super::*;

impl App {
    // ------------------------------------------------------------ settings

    pub(super) fn tab_settings(&mut self, ui: &mut Ui, wide: bool) {
        let pad = if wide {
            Margin {
                left: 32,
                right: 32,
                top: 8,
                bottom: 24,
            }
        } else {
            Margin {
                left: 28,
                right: 28,
                top: 4,
                bottom: 18,
            }
        };
        egui::ScrollArea::vertical()
            .id_salt("settings_scroll")
            .auto_shrink([false, false])
            .show(ui, |ui| {
                egui::Frame::new().inner_margin(pad).show(ui, |ui| {
                    ui.horizontal(|ui| {
                        page_title(ui, "Настройки");
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            segmented(
                                ui,
                                &mut self.settings_page,
                                &[
                                    (SettingsPage::General, "Общие"),
                                    (SettingsPage::Sites, "Сайты"),
                                    (SettingsPage::Logs, "Логи"),
                                ],
                                true,
                            );
                        });
                    });
                    ui.add_space(if wide { 26.0 } else { 20.0 });
                    match self.settings_page {
                        SettingsPage::General => two_col(
                            ui,
                            wide,
                            self,
                            |s, ui| s.card_general(ui),
                            |s, ui| s.card_testing(ui, wide),
                        ),
                        SettingsPage::Sites => two_col(
                            ui,
                            wide,
                            self,
                            |s, ui| s.card_user_lists(ui),
                            |s, ui| s.card_availability(ui, wide),
                        ),
                        SettingsPage::Logs => self.card_logs(ui),
                    }
                });
            });
    }

    pub(super) fn card_general(&mut self, ui: &mut Ui) {
        card_frame().show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.spacing_mut().item_spacing.y = 14.0;
            h_title(ui, "Общие");
            ui.vertical(|ui| {
                ui.spacing_mut().item_spacing.y = 8.0;
                overline(ui, "Папка data (с configs, lists, bin)");
                let mut apply = false;
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 10.0;
                    let bw = 28.0 + text_w(ui, "Применить", &font(13.0, W::Med)) + 2.0;
                    let w = ui.available_width() - bw - 10.0;
                    let r = text_input(
                        ui,
                        "data_dir",
                        &mut self.settings.data_dir,
                        w,
                        InputOpts {
                            hint: "автоопределение",
                            icon: Some(Icon::Server),
                            ..Default::default()
                        },
                    );
                    if r.changed() {
                        self.dirty = true;
                    }
                    apply = button(ui, "Применить", Variant::Secondary, None, true).clicked();
                });
                if apply {
                    self.data = zapret::find_data_dir(&self.settings.data_dir);
                    self.configs = self
                        .data
                        .as_deref()
                        .map(zapret::list_configs)
                        .unwrap_or_default();
                    self.selected = self
                        .configs
                        .iter()
                        .position(|c| c.name == self.settings.last_config);
                    self.test.best = None;
                    self.reload_user_list();
                }
                match &self.data {
                    Some(d) => status_line_mono(
                        ui,
                        Some(Icon::Check),
                        &format!("Найдена: {}", d.display()),
                        SUCCESS,
                    ),
                    None => status_line_mono(ui, None, "Не найдена", DANGER),
                }
            });
            ui.vertical(|ui| {
                ui.spacing_mut().item_spacing.y = 0.0;
                let mut ch = false;
                settings_row(
                    ui,
                    SettingsRow {
                        title: "Запускать последнюю конфигурацию",
                        desc: "При старте программы",
                        last: false,
                    },
                    |ui| {
                        ch |= switch(ui, &mut self.settings.autostart_last_config, false);
                    },
                );
                let mut autostart_toggled = None;
                if sys::autostart_supported() {
                    let mut on = self.system_autostart;
                    settings_row(
                        ui,
                        SettingsRow {
                            title: "Запускать вместе с системой",
                            desc: "При входе в Windows, с правами администратора",
                            last: false,
                        },
                        |ui| {
                            if switch(ui, &mut on, self.elevated) {
                                autostart_toggled = Some(on);
                            }
                        },
                    );
                }
                let elevated = self.elevated;
                settings_row(
                    ui,
                    SettingsRow {
                        title: "Права администратора",
                        desc: "Без них движок не запустится",
                        last: true,
                    },
                    |ui| {
                        if elevated {
                            badge(ui, "есть", Tone::Success, false);
                        } else {
                            badge(ui, "нет", Tone::Danger, false);
                        }
                    },
                );
                self.dirty |= ch;
                if let Some(on) = autostart_toggled {
                    if sys::set_autostart(on) {
                        self.system_autostart = on;
                    } else {
                        self.log(format!(
                            "Ошибка: не удалось {} автозапуск",
                            if on { "включить" } else { "отключить" }
                        ));
                    }
                }
            });
        });
    }

    pub(super) fn card_testing(&mut self, ui: &mut Ui, wide: bool) {
        let sw = if wide { 200.0 } else { 170.0 };
        card_frame().show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.spacing_mut().item_spacing.y = 0.0;
            h_title(ui, "Тестирование конфигураций");
            ui.add_space(6.0);
            let mut ch = false;
            let mut v = self.settings.test_parallel as u32;
            if slider_row(ui, "Параллельно", &mut v, 1, 32, sw) {
                self.settings.test_parallel = v as _;
                ch = true;
            }
            let mut v = self.settings.test_timeout as u32;
            if slider_row(ui, "Таймаут, с", &mut v, 2, 15, sw) {
                self.settings.test_timeout = v as _;
                ch = true;
            }
            let mut v = self.settings.site_timeout as u32;
            if slider_row(ui, "Таймаут проверки сайтов, с", &mut v, 2, 20, sw)
            {
                self.settings.site_timeout = v as _;
                ch = true;
            }
            ui.add_space(12.0);
            let dpi_label = rt("Объём DPI-запроса, байт", 14.0, W::Med, TEXT);
            let narrow = ui.available_width() < 190.0 + 150.0 + 24.0;
            let mut dpi_row = |ui: &mut Ui| {
                let mut v = self.settings.dpi_range as u32;
                if stepper(ui, &mut v, 1024, 1_048_576, 1024, 150.0) {
                    self.settings.dpi_range = v as _;
                    ch = true;
                }
            };
            if narrow {
                ui.add(egui::Label::new(dpi_label).selectable(false));
                ui.add_space(6.0);
                dpi_row(ui);
            } else {
                ui.horizontal(|ui| {
                    ui.add(egui::Label::new(dpi_label).selectable(false));
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| dpi_row(ui));
                });
            }
            ui.add_space(2.0);
            self.dirty |= ch;
        });
    }

    pub(super) fn card_logs(&mut self, ui: &mut Ui) {
        let mut clear = false;
        let mut copy = false;
        let mut open = false;
        if self.log_source == LogSource::TgWs
            && self
                .tg_log_at
                .is_none_or(|t| t.elapsed() >= Duration::from_secs(1))
        {
            self.tg_log = crate::tg_ws::read_log(1000);
            self.tg_log_at = Some(std::time::Instant::now());
        }
        if self.log_source == LogSource::TgWs {
            ui.ctx().request_repaint_after(Duration::from_secs(1));
        }
        card_frame().show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.spacing_mut().item_spacing.y = 12.0;
            ui.horizontal(|ui| {
                h_title(ui, "Логи");
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if self.log_source == LogSource::TgWs {
                        if button(ui, "Файл", Variant::Ghost, None, true).clicked() {
                            open = true;
                        }
                        if button(ui, "Диагностика", Variant::Ghost, None, true).clicked()
                        {
                            crate::tg_ws::diagnose();
                        }
                    }
                    if button(ui, "Очистить", Variant::Ghost, None, true).clicked() {
                        clear = true;
                    }
                    if button(ui, "Копировать", Variant::Secondary, None, true).clicked()
                    {
                        copy = true;
                    }
                });
            });
            segmented(
                ui,
                &mut self.log_source,
                &[
                    (LogSource::App, "Приложение"),
                    (LogSource::TgWs, "Telegram WS"),
                ],
                true,
            );
            let lines: &Vec<String> = match self.log_source {
                LogSource::App => &self.log,
                LogSource::TgWs => &self.tg_log,
            };
            if copy {
                ui.ctx().copy_text(lines.join("\n"));
            }
            egui::ScrollArea::vertical()
                .id_salt(("log_scroll", self.log_source as u8))
                .max_height(460.0)
                .stick_to_bottom(true)
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.spacing_mut().item_spacing.y = 3.0;
                    if lines.is_empty() {
                        ui.add(egui::Label::new(rt("Пока пусто", 12.0, W::Mono, TEXT3)));
                    }
                    for line in lines {
                        let lower = line.to_lowercase();
                        let color = if line.starts_with("ОШИБКА") || lower.contains("error") {
                            DANGER
                        } else if lower.contains("failed") || lower.contains("timeout") {
                            WARNING
                        } else {
                            TEXT2
                        };
                        ui.add(egui::Label::new(rt(line, 12.0, W::Mono, color)));
                    }
                });
        });
        if clear {
            match self.log_source {
                LogSource::App => self.log.clear(),
                LogSource::TgWs => {
                    crate::tg_ws::clear_log();
                    self.tg_log.clear();
                }
            }
        }
        if open {
            sys::open_uri(&crate::tg_ws::log_path().to_string_lossy());
        }
    }

    pub(super) fn card_user_lists(&mut self, ui: &mut Ui) {
        let mut remove: Option<String> = None;
        let mut kind_changed = false;
        let mut restart = false;
        card_frame().show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.spacing_mut().item_spacing.y = 14.0;
            ui.horizontal(|ui| {
                h_title(ui, "Свои списки сайтов");
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    kind_changed |= segmented(
                        ui,
                        &mut self.sites.kind,
                        &[
                            (ListKind::Proxy, "Обходить"),
                            (ListKind::Exclude, "Исключения"),
                        ],
                        true,
                    );
                });
            });
            let desc = match self.sites.kind {
                ListKind::Proxy => {
                    "list-general-user.txt — сайты, трафик которых обрабатывает zapret"
                }
                ListKind::Exclude => "list-exclude-user.txt — сайты, которые zapret не трогает",
            };
            ui.add(egui::Label::new(rt(desc, 12.0, W::Mono, TEXT3)).selectable(false));
            egui::ScrollArea::vertical()
                .id_salt("user_list")
                .max_height(170.0)
                .auto_shrink([false, true])
                .show(ui, |ui| {
                    ui.horizontal_wrapped(|ui| {
                        ui.spacing_mut().item_spacing = vec2(8.0, 8.0);
                        for d in &self.sites.user_list {
                            if chip(ui, d) {
                                remove = Some(d.clone());
                            }
                        }
                    });
                    if self.sites.user_list.is_empty() {
                        ui.add(
                            egui::Label::new(rt("Список пуст", 13.0, W::Reg, TEXT3))
                                .selectable(false),
                        );
                    }
                });
            if self.sites.restart_hint {
                let running = self.is_running();
                banner_row(
                    ui,
                    "Список изменён — перезапустите zapret.",
                    |ui| {
                        if running
                            && button(ui, "Перезапустить", Variant::Secondary, None, true).clicked()
                        {
                            restart = true;
                        }
                    },
                );
            }
        });
        if kind_changed {
            self.reload_user_list();
        }
        if restart {
            self.restart_zapret();
        }
        if let (Some(d), Some(data)) = (remove, self.data.clone()) {
            if let Err(e) = sites::remove_domain(&data, self.sites.kind, &d) {
                self.log(format!("Ошибка записи списка: {e}"));
            }
            self.reload_user_list();
            self.sites.restart_hint = true;
        }
    }

    pub(super) fn card_availability(&mut self, ui: &mut Ui, wide: bool) {
        let checking = self.sites.handle.is_some();
        let mut check = false;
        let mut add_selected = false;
        card_frame().show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.spacing_mut().item_spacing.y = 12.0;
            h_title(ui, "Проверка доступности");
            let w = ui.available_width();
            text_area(
                ui,
                "sites_input",
                &mut self.sites.input,
                w,
                if wide { 68.0 } else { 56.0 },
                "youtube.com\ndiscord.com, instagram.com",
                !checking,
            );
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 14.0;
                if !checking {
                    if button(
                        ui,
                        "Проверить",
                        Variant::Primary,
                        Some(Icon::Play),
                        self.data.is_some(),
                    )
                    .clicked()
                    {
                        check = true;
                    }
                } else if button(ui, "Остановить", Variant::Secondary, None, true).clicked()
                {
                    if let Some(h) = &self.sites.handle {
                        h.cancel.store(true, Ordering::Relaxed);
                    }
                }
                ui.add_enabled_ui(!checking, |ui| {
                    switch(ui, &mut self.sites.pause_zapret, true);
                });
                ui.add(
                    egui::Label::new(rt("Без zapret (временно остановить)", 13.0, W::Reg, TEXT2))
                        .selectable(false),
                );
            });
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 14.0;
                ui.add(egui::Label::new(rt("Потоков", 13.0, W::Reg, TEXT2)).selectable(false));
                let sw = ui.available_width() - 28.0 - 14.0;
                let mut v = self.settings.site_parallel as u32;
                if ui
                    .add_enabled_ui(!checking, |ui| slider(ui, &mut v, 1, 64, sw))
                    .inner
                {
                    self.settings.site_parallel = v as _;
                    self.dirty = true;
                }
                let (r, _) = ui.allocate_exact_size(vec2(28.0, 16.0), Sense::hover());
                ui.painter().text(
                    r.right_center(),
                    Align2::RIGHT_CENTER,
                    v.to_string(),
                    font(13.0, W::Mono500),
                    TEXT,
                );
            });
            if checking {
                let done = self.sites.results.len();
                let frac = done as f32 / self.sites.expected.max(1) as f32;
                progress_bar(ui, frac, ui.available_width());
            }
            let n = self.sites.results.iter().filter(|(_, s)| *s).count();
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing = vec2(6.0, 6.0);
                if button(ui, "Недоступные", Variant::Ghost, None, true).clicked() {
                    for (r, sel) in &mut self.sites.results {
                        *sel = !r.ok;
                    }
                }
                if button(ui, "Все", Variant::Ghost, None, true).clicked() {
                    for (_, sel) in &mut self.sites.results {
                        *sel = true;
                    }
                }
                if button(ui, "Снять", Variant::Ghost, None, true).clicked() {
                    for (_, sel) in &mut self.sites.results {
                        *sel = false;
                    }
                }
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if button(
                        ui,
                        &format!("Добавить выбранные ({n})"),
                        Variant::Secondary,
                        Some(Icon::Plus),
                        n > 0,
                    )
                    .clicked()
                    {
                        add_selected = true;
                    }
                });
            });
            if !self.sites.results.is_empty() {
                egui::ScrollArea::vertical()
                    .id_salt("site_results_scroll")
                    .max_height(240.0)
                    .auto_shrink([false, true])
                    .show(ui, |ui| {
                        ui.spacing_mut().item_spacing.y = 0.0;
                        for (r, sel) in &mut self.sites.results {
                            let w = ui.available_width();
                            let (rect, resp) =
                                ui.allocate_exact_size(vec2(w, 40.0), Sense::click());
                            let p = ui.painter().clone();
                            p.rect_filled(
                                Rect::from_min_size(rect.min, vec2(w, 1.0)),
                                0,
                                LINE_SOFT,
                            );
                            if resp.hovered() {
                                p.rect_filled(
                                    rect.shrink2(vec2(0.0, 1.0)),
                                    6,
                                    rgba(255, 255, 255, 5),
                                );
                            }
                            let cy = rect.center().y;
                            paint_checkbox(ui, pos2(rect.left() + 8.0, cy), *sel, true);
                            let meta = format!("{} мс · {}", r.ms, r.detail);
                            p.text(
                                pos2(rect.right(), cy),
                                Align2::RIGHT_CENTER,
                                meta,
                                font(12.0, W::Mono),
                                TEXT3,
                            );
                            let label = if r.ok {
                                "доступен"
                            } else {
                                "недоступен"
                            };
                            let lx = rect.right() - 132.0 - 12.0;
                            p.text(
                                pos2(lx, cy),
                                Align2::RIGHT_CENTER,
                                label,
                                font(13.0, W::Med),
                                if r.ok { SUCCESS } else { DANGER },
                            );
                            let lw = text_w(ui, label, &font(13.0, W::Med));
                            text_trunc(
                                &p,
                                pos2(rect.left() + 8.0 + 8.0 + 12.0, cy),
                                &r.domain,
                                font(14.0, W::Med),
                                TEXT,
                                lx - lw - 12.0 - (rect.left() + 28.0),
                            );
                            if resp
                                .on_hover_cursor(egui::CursorIcon::PointingHand)
                                .clicked()
                            {
                                *sel = !*sel;
                            }
                        }
                    });
            }
        });
        if check {
            self.start_site_check();
        }
        if add_selected {
            if let Some(data) = self.data.clone() {
                let chosen: Vec<String> = self
                    .sites
                    .results
                    .iter()
                    .filter(|(_, s)| *s)
                    .map(|(r, _)| r.domain.clone())
                    .collect();
                match sites::add_domains(&data, self.sites.kind, &chosen) {
                    Ok(n) => self.log(format!("В список добавлено доменов: {n}")),
                    Err(e) => self.log(format!("Ошибка записи списка: {e}")),
                }
                self.reload_user_list();
                self.sites.restart_hint = true;
            }
        }
    }
}
