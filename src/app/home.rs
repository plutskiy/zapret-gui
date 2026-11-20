use super::*;

impl App {
    // ------------------------------------------------------------ derived UI state

    pub(super) fn hero_state(&self) -> Hero {
        let auto = self.test.auto && self.test.handle.is_some();
        if auto || self.busy {
            Hero::Working
        } else if self.is_running() {
            Hero::On
        } else if self.home_error.is_some() {
            Hero::Error
        } else {
            Hero::Off
        }
    }

    pub(super) fn auto_testing(&self) -> bool {
        self.test.auto && self.test.handle.is_some()
    }

    pub(super) fn test_fraction(&self) -> f32 {
        let (idx, total) = self.test.config_index;
        let (done, targets) = self.test.target_progress;
        let inner = if targets > 0 {
            done as f32 / targets as f32
        } else {
            0.0
        };
        (idx as f32 + inner) / total.max(1) as f32
    }

    pub(super) fn hero_texts(&self, state: Hero) -> (String, String) {
        match state {
            Hero::Off => (
                "Отключено".into(),
                if !self.elevated {
                    "Нужны права администратора (root)".into()
                } else if let Some(c) = self.selected.and_then(|i| self.configs.get(i)) {
                    format!("Выбрана «{}» — нажмите, чтобы запустить", c.name)
                } else {
                    "Конфигурация не выбрана — нажмите, и программа проверит все".into()
                },
            ),
            Hero::Working => {
                if self.auto_testing() {
                    let (idx, total) = self.test.config_index;
                    (
                        "Подбор конфигурации…".into(),
                        format!("Проверяем «{}» — {}/{}", self.test.current, idx + 1, total),
                    )
                } else {
                    ("Подождите…".into(), "Выполняется операция".into())
                }
            }
            Hero::On => {
                let sub = if self.settings.last_config.is_empty() {
                    String::new()
                } else {
                    format!("Конфигурация: {}", self.settings.last_config)
                };
                ("Подключено".into(), sub)
            }
            Hero::Error => (
                "Не удалось подключиться".into(),
                self.home_error.clone().unwrap_or_default(),
            ),
        }
    }

    pub(super) fn default_log_line(state: Hero) -> &'static str {
        match state {
            Hero::Off => "Zapret остановлен",
            Hero::Working => "Подбор конфигурации…",
            Hero::On => "Zapret запущен",
            Hero::Error => "ОШИБКА",
        }
    }

    // ------------------------------------------------------------ home

    pub(super) fn hero_block(&mut self, ui: &mut Ui, s: f32, state: Hero, acts: &mut Acts) {
        let can_power = self.elevated
            && !self.configs.is_empty()
            && (self.auto_testing() || self.test.handle.is_none());
        let (title, sub) = self.hero_texts(state);
        let progress = self.test_fraction();
        ui.vertical_centered(|ui| {
            ui.spacing_mut().item_spacing = vec2(0.0, 0.0);
            if power_hero(ui, state, s, progress, can_power).clicked() && can_power {
                acts.toggle = true;
            }
            ui.add_space(14.0 * s);
            let tcol = match state {
                Hero::Off => TEXT,
                Hero::Error => DANGER,
                _ => ACCENT,
            };
            ui.add(egui::Label::new(rt(title, 28.0 * s, W::D800, tcol)).selectable(false));
            ui.add_space(8.0 * s);
            ui.scope(|ui| {
                ui.set_max_width(360.0 * s);
                if !sub.is_empty() {
                    ui.add(egui::Label::new(rt(sub, 14.0 * s, W::Reg, TEXT2)).selectable(false));
                }
            });
            if state == Hero::Working && self.auto_testing() {
                ui.add_space(16.0 * s);
                progress_bar(ui, progress, 280.0 * s);
                ui.add_space(10.0 * s);
                ui.add(
                    egui::Label::new(rt(
                        "Нажмите на кнопку ещё раз, чтобы отменить",
                        12.0 * s,
                        W::Reg,
                        TEXT3,
                    ))
                    .selectable(false),
                );
            }
            if self.service_conflict.load(Ordering::Relaxed) {
                ui.add_space(16.0 * s);
                banner(
                    ui,
                    "Запущена служба zapret — удалите её, иначе проверка будет некорректной",
                    400.0,
                    |_| {},
                );
            }
            if let Some(data) = self.data.as_deref() {
                if let Err(e) = zapret::engine_path(data) {
                    ui.add_space(10.0);
                    ui.add(egui::Label::new(rt(e, 12.0, W::Reg, DANGER)).selectable(false));
                }
            }
        });
    }

    pub(super) fn row_views(&self) -> Vec<RowView> {
        let testing = self.test.handle.is_some();
        let running = self.is_running();
        let kind = self.test.ran_kind.unwrap_or(Kind::Standard);
        let mut order: Vec<usize> = (0..self.configs.len()).collect();
        let rank_of = |name: &str, reports: &[Report]| {
            reports.iter().find(|r| r.config == name).map(|r| r.score())
        };
        order.sort_by_key(|&i| Reverse(rank_of(&self.configs[i].name, &self.test.reports)));
        order
            .into_iter()
            .map(|i| {
                let name = self.configs[i].name.clone();
                let report = self.test.reports.iter().find(|r| r.config == name);
                let result = report.map(|r| {
                    if r.error.is_some() {
                        return ("ошибка".to_string(), Tone::Danger);
                    }
                    let s = r.summary();
                    let (txt, bad) = match kind {
                        Kind::Standard => (
                            format!(
                                "{} OK · {} ERR · ping {}/{}",
                                s.ok,
                                s.fail,
                                s.ping_ok,
                                s.ping_ok + s.ping_fail
                            ),
                            s.fail,
                        ),
                        Kind::Dpi => (
                            format!("{} OK · {} FAIL · {} BLOCK", s.ok, s.fail, s.blocked),
                            s.fail + s.blocked,
                        ),
                    };
                    let tone = if bad == 0 {
                        Tone::Success
                    } else if s.ok > s.fail {
                        Tone::Warning
                    } else {
                        Tone::Danger
                    };
                    (txt, tone)
                });
                RowView {
                    active: running && name == self.settings.last_config,
                    selected: self.selected == Some(i),
                    checked: self.test.checked.contains(&name),
                    spinner: report.is_none() && testing && self.test.current == name,
                    best: self.test.best.as_deref() == Some(name.as_str()),
                    result,
                    name,
                }
            })
            .collect()
    }

    pub(super) fn config_card(&mut self, ui: &mut Ui, h: f32, acts: &mut Acts) {
        let testing = self.test.handle.is_some();
        let auto = self.auto_testing();
        let idle = !testing && !self.busy;
        let rows = self.row_views();
        let w = ui.available_width();
        let mut toggle_name: Option<String> = None;
        let mut play: Option<usize> = None;

        let fr = egui::Frame::new()
            .fill(CARD)
            .stroke(Stroke::new(1.0, LINE_SOFT))
            .corner_radius(16)
            .show(ui, |ui| {
                ui.set_width(w - 2.0);
                ui.set_height(h - 2.0);
                ui.spacing_mut().item_spacing = vec2(8.0, 0.0);

                egui::Frame::new()
                    .inner_margin(Margin {
                        left: 14,
                        right: 12,
                        top: 14,
                        bottom: 12,
                    })
                    .show(ui, |ui| {
                        ui.set_width(ui.available_width());
                        ui.spacing_mut().interact_size.y = 30.0;
                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing.x = 10.0;
                            h_title(ui, "Конфигурации");
                            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                if testing {
                                    if button(ui, "Остановить", Variant::Secondary, None, true)
                                        .clicked()
                                    {
                                        acts.stop_test = true;
                                    }
                                } else {
                                    if !testing {
                                        let more = icon_button(ui, Icon::More, 1.0, true);
                                        egui::Popup::menu(&more).show(|ui| {
                                            if ui.button("DPI-тест (заморозка 16–20 КБ)").clicked()
                                            {
                                                acts.start_test = Some(Kind::Dpi);
                                                ui.close();
                                            }
                                            if ui.button("Выбрать все").clicked() {
                                                self.test.checked = self
                                                    .configs
                                                    .iter()
                                                    .map(|c| c.name.clone())
                                                    .collect();
                                                ui.close();
                                            }
                                            if ui.button("Снять выбор").clicked() {
                                                self.test.checked.clear();
                                                ui.close();
                                            }
                                        });
                                    }
                                    let n = self.test.checked.len();
                                    let label = if n > 0 {
                                        format!("Тест ({n})")
                                    } else {
                                        "Тест всех".to_string()
                                    };
                                    if button(
                                        ui,
                                        &label,
                                        Variant::Primary,
                                        Some(Icon::Play),
                                        self.elevated && idle && !auto,
                                    )
                                    .clicked()
                                    {
                                        acts.start_test = Some(Kind::Standard);
                                    }
                                }
                            });
                        });
                        if testing && !auto {
                            let (idx, total) = self.test.config_index;
                            let (done, targets) = self.test.target_progress;
                            ui.add_space(10.0);
                            ui.add(
                                egui::Label::new(rt(
                                    format!(
                                        "{} — {}/{}, цели {done}/{targets}",
                                        self.test.current,
                                        idx + 1,
                                        total
                                    ),
                                    12.0,
                                    W::Mono,
                                    TEXT3,
                                ))
                                .selectable(false),
                            );
                            ui.add_space(6.0);
                            progress_bar(ui, self.test_fraction(), ui.available_width());
                        }
                        if !self.elevated {
                            ui.add_space(8.0);
                            ui.add(
                                egui::Label::new(rt(
                                    "Для тестов нужны права администратора (root)",
                                    12.0,
                                    W::Reg,
                                    DANGER,
                                ))
                                .selectable(false),
                            );
                        }
                        if let Some(path) = &self.test.saved {
                            ui.add_space(8.0);
                            ui.add(
                                egui::Label::new(rt(
                                    format!("Отчёт: {path}"),
                                    12.0,
                                    W::Mono,
                                    TEXT3,
                                ))
                                .selectable(false),
                            );
                        }
                    });

                ui.horizontal(|ui| {
                    ui.add_space(8.0);
                    let (r, _) = ui
                        .allocate_exact_size(vec2(ui.available_width() - 8.0, 1.0), Sense::hover());
                    ui.painter().rect_filled(r, 0, LINE_SOFT);
                });

                egui::Frame::new()
                    .inner_margin(Margin::same(8))
                    .show(ui, |ui| {
                        let avail = ui.available_height();
                        let out = egui::ScrollArea::vertical()
                            .id_salt("cfg_list")
                            .max_height(avail)
                            .auto_shrink([false, false])
                            .show(ui, |ui| {
                                ui.spacing_mut().item_spacing = vec2(0.0, 0.0);
                                for row in &rows {
                                    let r = config_row(ui, row, !testing, !testing);
                                    if r.toggled {
                                        toggle_name = Some(row.name.clone());
                                    }
                                    if r.play {
                                        play = self.configs.iter().position(|c| c.name == row.name);
                                    }
                                }
                                if !self.test.reports.is_empty() {
                                    ui.add_space(8.0);
                                    let label = if self.show_details {
                                        "Скрыть подробные результаты"
                                    } else {
                                        "Подробные результаты"
                                    };
                                    if button(ui, label, Variant::Ghost, None, true).clicked() {
                                        self.show_details = !self.show_details;
                                    }
                                    if self.show_details {
                                        ui.add_space(6.0);
                                        ui.spacing_mut().item_spacing = vec2(8.0, 6.0);
                                        let mut order: Vec<usize> =
                                            (0..self.test.reports.len()).collect();
                                        order.sort_by_key(|&i| {
                                            Reverse(self.test.reports[i].score())
                                        });
                                        for i in order {
                                            let r = &self.test.reports[i];
                                            if let Some(err) = &r.error {
                                                ui.add(egui::Label::new(rt(
                                                    format!(
                                                        "✖ {} — не запустилась: {err}",
                                                        r.config
                                                    ),
                                                    12.0,
                                                    W::Reg,
                                                    DANGER,
                                                )));
                                                continue;
                                            }
                                            egui::CollapsingHeader::new(rt(
                                                &r.config,
                                                13.0,
                                                W::Med,
                                                TEXT2,
                                            ))
                                            .id_salt(("report", &r.config))
                                            .show(ui, |ui| report_details(ui, r));
                                        }
                                    }
                                }
                            });
                        if out.content_size.y > out.inner_rect.height() + 1.0 {
                            let fade = Rect::from_min_max(
                                pos2(out.inner_rect.left() - 8.0, out.inner_rect.bottom() - 40.0),
                                pos2(out.inner_rect.right() + 8.0, out.inner_rect.bottom() + 8.0),
                            );
                            vgradient_rect(ui.painter(), fade, Color32::TRANSPARENT, CARD);
                        }
                    });
            });
        card_top_edge(ui.painter(), fr.response.rect);

        if let Some(name) = toggle_name {
            if !self.test.checked.remove(&name) {
                self.test.checked.insert(name);
            }
        }
        if play.is_some() {
            acts.select = play;
        }
    }

    pub(super) fn tab_home(&mut self, ui: &mut Ui, wide: bool) {
        let Some(_data) = self.data.clone() else {
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
            egui::Frame::new().inner_margin(pad).show(ui, |ui| {
                card_frame().show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.spacing_mut().item_spacing.y = 10.0;
                    ui.add(egui::Label::new(rt(
                        "Папка data (с configs и lists) не найдена.",
                        14.0,
                        W::Med,
                        DANGER,
                    )));
                    ui.add(egui::Label::new(rt(
                        "Укажите её в разделе «Настройки».",
                        14.0,
                        W::Reg,
                        TEXT2,
                    )));
                    if button(ui, "Открыть настройки", Variant::Secondary, None, true).clicked()
                    {
                        self.tab = Tab::Settings;
                    }
                });
            });
            return;
        };
        let state = self.hero_state();
        let mut acts = Acts::default();
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

        egui::Frame::new().inner_margin(pad).show(ui, |ui| {
            let avail = ui.available_size();
            if wide {
                let mut est = 240.0 + 14.0 + 31.0 + 8.0 + 20.0;
                if state == Hero::Working && self.auto_testing() {
                    est += 42.0;
                }
                if self.service_conflict.load(Ordering::Relaxed) {
                    est += 60.0;
                }
                let top = ((avail.y - est) / 2.0).max(0.0);
                ui.horizontal_top(|ui| {
                    ui.spacing_mut().item_spacing.x = 28.0;
                    ui.allocate_ui_with_layout(
                        vec2(320.0, avail.y),
                        Layout::top_down(Align::Center),
                        |ui| {
                            ui.add_space(top);
                            self.hero_block(ui, 1.0, state, &mut acts);
                        },
                    );
                    let w = ui.available_width();
                    ui.allocate_ui_with_layout(
                        vec2(w, avail.y),
                        Layout::top_down(Align::Min),
                        |ui| {
                            self.config_card(ui, avail.y, &mut acts);
                        },
                    );
                });
            } else {
                egui::ScrollArea::vertical()
                    .id_salt("home_scroll")
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        let top = ui.next_widget_position().y;
                        self.hero_block(ui, 0.86, state, &mut acts);
                        ui.add_space(16.0);
                        let used = ui.next_widget_position().y - top;
                        let h = (avail.y - used).max(320.0);
                        self.config_card(ui, h, &mut acts);
                    });
            }
        });

        if acts.toggle {
            self.toggle_power();
        }
        if acts.stop_test {
            if let Some(h) = &self.test.handle {
                h.cancel.store(true, Ordering::Relaxed);
            }
        }
        if let Some(kind) = acts.start_test {
            if self.test.checked.is_empty() {
                self.test.checked = self.configs.iter().map(|c| c.name.clone()).collect();
            }
            self.start_tests(kind);
        }
        if let Some(i) = acts.select {
            self.selected = if self.selected == Some(i) {
                None
            } else {
                Some(i)
            };
        }
    }
}
