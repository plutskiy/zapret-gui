use super::*;

impl App {
    pub(super) fn start_config(&mut self, index: usize) {
        let (Some(data), Some(cfg)) = (self.data.clone(), self.configs.get(index).cloned()) else {
            return;
        };
        if self.busy {
            return;
        }
        self.busy = true;
        self.home_error = None;
        self.selected = Some(index);
        self.log(format!("Запуск: {}", cfg.name));
        let tg = self.settings.telegram.clone();
        let (tx, ctx) = (self.op_tx.clone(), self.ctx.clone());
        std::thread::spawn(move || {
            let result = zapret::start_config(&data, &cfg);
            let ok = result.is_ok();
            let _ = tx.send(match result {
                Ok(()) => OpResult {
                    message: format!("Запущена конфигурация {}", cfg.name),
                    error: false,
                    started: Some(cfg.name.clone()),
                    start_failed: false,
                },
                Err(e) => OpResult {
                    message: e,
                    error: true,
                    started: None,
                    start_failed: true,
                },
            });
            ctx.request_repaint();
            if ok && tg.on_zapret_start {
                std::thread::sleep(Duration::from_millis(1500));
                let _ = tx.send(tg_result(telegram::apply(&tg)));
                ctx.request_repaint();
            }
        });
    }

    pub(super) fn stop_zapret(&mut self) {
        if self.busy {
            return;
        }
        self.busy = true;
        let (tx, ctx) = (self.op_tx.clone(), self.ctx.clone());
        std::thread::spawn(move || {
            zapret::stop();
            let _ = tx.send(OpResult {
                message: "zapret остановлен".into(),
                error: false,
                started: None,
                start_failed: false,
            });
            ctx.request_repaint();
        });
    }

    pub(super) fn restart_zapret(&mut self) {
        let idx = self
            .configs
            .iter()
            .position(|c| c.name == self.settings.last_config)
            .or(self.selected);
        if let Some(i) = idx {
            self.start_config(i);
        }
    }

    pub(super) fn poll(&mut self) {
        while let Ok(op) = self.op_rx.try_recv() {
            self.busy = false;
            let prefix = if op.error { "ОШИБКА: " } else { "" };
            self.log(format!("{prefix}{}", op.message));
            if op.start_failed {
                self.home_error = Some(op.message.clone());
            }
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

    pub(super) fn poll_tests(&mut self) {
        if let Some(d) = &self.test.draining {
            let mut done = false;
            while let Ok(e) = d.rx.try_recv() {
                done |= matches!(e, tester::Event::Finished { .. });
            }
            if done {
                self.test.draining = None;
            }
        }
        let Some(handle) = &self.test.handle else {
            return;
        };
        let mut events = Vec::new();
        while let Ok(e) = handle.rx.try_recv() {
            events.push(e);
        }
        let mut finished = false;
        let mut was_cancelled = false;
        let stop_requested = handle.cancel.load(Ordering::Relaxed);
        for e in events {
            match e {
                tester::Event::Log(s) => self.log(s),
                tester::Event::ConfigStart { index, total, name } => {
                    self.test.current = name;
                    self.test.config_index = (index, total);
                }
                tester::Event::Progress { done, total } => {
                    self.test.target_progress = (done, total)
                }
                tester::Event::Report(r) => self.test.reports.push(r),
                tester::Event::Finished { cancelled } => {
                    finished = true;
                    was_cancelled = cancelled;
                    self.log(if cancelled {
                        "Тест остановлен"
                    } else {
                        "Тест завершён"
                    });
                }
            }
        }
        if stop_requested && !finished {
            finished = true;
            was_cancelled = true;
            self.log("Тест остановлен");
            self.test.draining = self.test.handle.take();
        }
        if finished {
            self.test.handle = None;
            self.test.best = self.best_config();
            if self.test.auto {
                self.test.auto = false;
                if !was_cancelled {
                    match self
                        .test
                        .best
                        .clone()
                        .and_then(|n| self.configs.iter().position(|c| c.name == n))
                    {
                        Some(i) => {
                            self.log("Подобрана рабочая конфигурация — подключаюсь");
                            self.start_config(i);
                        }
                        None => {
                            self.log(format!("ОШИБКА: {}", NO_PASS_MSG));
                            self.home_error = Some(NO_PASS_MSG.into());
                        }
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

    pub(super) fn poll_sites(&mut self) {
        let Some(handle) = &self.sites.handle else {
            return;
        };
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

    pub(super) fn best_config(&self) -> Option<String> {
        self.test
            .reports
            .iter()
            .filter(|r| r.error.is_none() && r.summary().ok > 0)
            .max_by_key(|r| r.score())
            .map(|r| r.config.clone())
    }

    /// Большая кнопка: подключить / отключить / отменить подбор.
    pub(super) fn toggle_power(&mut self) {
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
        self.home_error = None;
        if let Some(i) = self.selected.filter(|&i| i < self.configs.len()) {
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

    pub(super) fn start_tests(&mut self, kind: Kind) {
        let selected: Vec<ConfigEntry> = self
            .configs
            .iter()
            .filter(|c| self.test.checked.contains(&c.name))
            .cloned()
            .collect();
        self.start_tests_for(kind, selected);
    }

    pub(super) fn start_tests_for(&mut self, kind: Kind, selected: Vec<ConfigEntry>) {
        let Some(data) = self.data.clone() else {
            return;
        };
        if self.test.draining.is_some() {
            self.log("Предыдущий тест ещё завершается, подождите секунду");
            return;
        }
        if selected.is_empty() {
            self.log("Выберите хотя бы одну конфигурацию для теста");
            return;
        }
        let resume = if self.is_running() && !self.test.auto {
            self.configs
                .iter()
                .find(|c| c.name == self.settings.last_config)
                .cloned()
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
            fast: self.test.auto,
        };
        self.log(format!("Запуск теста: {} конфигураций", selected.len()));
        self.test.handle = Some(tester::spawn(
            data,
            selected,
            params,
            resume,
            self.ctx.clone(),
        ));
    }

    pub(super) fn start_site_check(&mut self) {
        let Some(data) = self.data.clone() else {
            return;
        };
        let domains = sites::parse_input(&self.sites.input);
        if domains.is_empty() {
            self.log("Введите домены для проверки (например, youtube.com)");
            return;
        }
        let pause = (self.sites.pause_zapret && self.is_running()).then(|| sites::Pause {
            data,
            resume: self
                .configs
                .iter()
                .find(|c| c.name == self.settings.last_config)
                .cloned(),
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

    pub(super) fn reload_user_list(&mut self) {
        if let Some(d) = &self.data {
            self.sites.user_list = sites::read_list(d, self.sites.kind);
        }
    }
}

pub(super) fn tg_result(r: Result<(), String>) -> OpResult {
    match r {
        Ok(()) => OpResult {
            message: "Telegram: ссылка прокси отправлена (подтвердите в окне Telegram)".into(),
            error: false,
            started: None,
            start_failed: false,
        },
        Err(e) => OpResult {
            message: format!("Telegram: {e}"),
            error: true,
            started: None,
            start_failed: false,
        },
    }
}
