use super::*;

impl App {
    // ------------------------------------------------------------ telegram

    pub(super) fn tab_telegram(&mut self, ui: &mut Ui, wide: bool) {
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
                top: 0,
                bottom: 4,
            }
        };
        egui::ScrollArea::vertical().id_salt("tg_scroll").auto_shrink([false, false]).show(ui, |ui| {
            egui::Frame::new().inner_margin(pad).show(ui, |ui| {
                ui.spacing_mut().item_spacing.y = 10.0;
                ui.vertical(|ui| {
                    ui.spacing_mut().item_spacing.y = 6.0;
                    page_title(ui, "Telegram");
                    ui.add(egui::Label::new(rt("Подключение к прокси через ссылку tg:// — подтвердите её в окне Telegram.", 14.0, W::Reg, TEXT2)).selectable(false));
                });
                two_col(ui, wide, self, |s, ui| s.tg_proxy_card(ui), |s, ui| s.tg_side(ui));
            });
        });
    }

    pub(super) fn tg_proxy_card(&mut self, ui: &mut Ui) {
        card_frame().show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.spacing_mut().item_spacing.y = 14.0;
            let mut changed = false;
            let t = &mut self.settings.telegram;
            ui.horizontal(|ui| {
                h_title(ui, "Прокси");
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    changed |= segmented(
                        ui,
                        &mut t.kind,
                        &[
                            (ProxyKind::Socks5, "SOCKS5"),
                            (ProxyKind::MtProto, "MTProto"),
                            (ProxyKind::TgWs, "TG WS"),
                        ],
                        true,
                    );
                });
            });
            let full = ui.available_width();
            let gap = 12.0;
            ui.horizontal_top(|ui| {
                ui.spacing_mut().item_spacing.x = gap;
                field(ui, "Адрес", full - 112.0 - gap, |ui, w| {
                    changed |=
                        text_input(ui, "tg_host", &mut t.host, w, InputOpts::default()).changed()
                });
                field(ui, "Порт", 112.0, |ui, w| {
                    let mut port = t.port.to_string();
                    if text_input(ui, "tg_port", &mut port, w, InputOpts::default()).changed() {
                        let digits: String = port.chars().filter(|c| c.is_ascii_digit()).collect();
                        if let Ok(p) = digits.parse::<u32>() {
                            t.port = p.clamp(1, 65535) as _;
                            changed = true;
                        }
                    }
                });
            });
            match t.kind {
                ProxyKind::Socks5 => {
                    let half = (full - gap) / 2.0;
                    ui.horizontal_top(|ui| {
                        ui.spacing_mut().item_spacing.x = gap;
                        field(ui, "Логин", half, |ui, w| {
                            changed |= text_input(
                                ui,
                                "tg_user",
                                &mut t.user,
                                w,
                                InputOpts {
                                    hint: "необязательно",
                                    ..Default::default()
                                },
                            )
                            .changed()
                        });
                        field(ui, "Пароль", half, |ui, w| {
                            changed |= text_input(
                                ui,
                                "tg_pass",
                                &mut t.pass,
                                w,
                                InputOpts {
                                    password: true,
                                    ..Default::default()
                                },
                            )
                            .changed()
                        });
                    });
                }
                ProxyKind::MtProto | ProxyKind::TgWs => {
                    field(ui, "Secret", full, |ui, w| {
                        changed |= text_input(
                            ui,
                            "tg_secret",
                            &mut t.secret,
                            w,
                            InputOpts {
                                mono: true,
                                hint: if t.kind == ProxyKind::TgWs {
                                    "Генерируется локально для TG WS"
                                } else {
                                    "Secret из настроек MTProto-сервера"
                                },
                                ..Default::default()
                            },
                        )
                        .changed();
                    });
                }
            }
            self.dirty |= changed;
        });
    }

    pub(super) fn tg_side(&mut self, ui: &mut Ui) {
        ui.spacing_mut().item_spacing.y = 16.0;
        let mut changed = false;
        let t = &mut self.settings.telegram;
        ui.vertical(|ui| {
            ui.spacing_mut().item_spacing.y = 12.0;
            ui.add(egui::Label::new(rt("Автоподключение", 19.0, W::Semi, TEXT)).selectable(false));
            egui::Frame::new()
                .fill(CARD)
                .stroke(Stroke::new(1.0, LINE_SOFT))
                .corner_radius(14)
                .inner_margin(Margin::symmetric(20, 4))
                .show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.spacing_mut().item_spacing.y = 0.0;
                    settings_row(
                        ui,
                        SettingsRow {
                            title: "При запуске zapret",
                            desc: "Подключать Telegram к прокси",
                            last: false,
                        },
                        |ui| {
                            changed |= switch(ui, &mut t.on_zapret_start, false);
                        },
                    );
                    settings_row(
                        ui,
                        SettingsRow {
                            title: "При запуске программы",
                            desc: "Если zapret не запускается автоматически",
                            last: true,
                        },
                        |ui| {
                            changed |= switch(ui, &mut t.on_app_start, false);
                        },
                    );
                });
        });
        self.dirty |= changed;
        ui.add_space(40.0 - 16.0);

        let t = self.settings.telegram.clone();
        let link = telegram::link(&t).ok();
        let mut probe_now = false;
        let mut apply_now = false;
        card_frame().show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.spacing_mut().item_spacing.y = 14.0;
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing = vec2(10.0, 10.0);
                if button(
                    ui,
                    "Проверить прокси",
                    Variant::Secondary,
                    None,
                    self.tg.probe.is_none(),
                )
                .clicked()
                {
                    probe_now = true;
                }
                if button(
                    ui,
                    "Подключить Telegram сейчас",
                    Variant::Primary,
                    None,
                    true,
                )
                .clicked()
                {
                    apply_now = true;
                }
                if self.tg.probe.is_some() {
                    ui.spinner();
                }
            });
            match &self.tg.probe_result {
                Some(Ok(ms)) => status_line(
                    ui,
                    Some(Icon::CheckCircle),
                    &if t.kind == ProxyKind::MtProto {
                        format!("Порт MTProto-прокси открыт ({ms} мс); Telegram принимает только secret сервера")
                    } else if t.kind == ProxyKind::TgWs {
                        format!("Локальный TG WS-прокси запущен ({ms} мс)")
                    } else {
                        format!("Прокси отвечает ({ms} мс)")
                    },
                    SUCCESS,
                ),
                Some(Err(e)) => status_line(ui, None, e, DANGER),
                None => {}
            }
            if let Some(link) = &link {
                let (rect, _) =
                    ui.allocate_exact_size(vec2(ui.available_width(), 40.0), Sense::hover());
                let p = ui.painter().clone();
                p.rect_filled(rect, cr(999.0), INK_1000);
                p.rect_stroke(
                    rect,
                    cr(999.0),
                    Stroke::new(1.0, LINE),
                    egui::StrokeKind::Inside,
                );
                let copy_w = 28.0 + text_w(ui, "Копировать", &font(13.0, W::Med)) + 2.0;
                let btn = Rect::from_min_size(
                    pos2(rect.right() - 6.0 - copy_w, rect.center().y - 14.0),
                    vec2(copy_w, 28.0),
                );
                text_trunc(
                    &p,
                    pos2(rect.left() + 14.0, rect.center().y),
                    link,
                    font(12.0, W::Mono),
                    TEXT2,
                    rect.width() - 14.0 - copy_w - 16.0,
                );
                ui.scope_builder(UiBuilder::new().max_rect(btn), |ui| {
                    if button(ui, "Копировать", Variant::Ghost, None, true).clicked() {
                        ui.ctx().copy_text(link.to_string());
                    }
                });
            }
        });
        if probe_now {
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
        if apply_now {
            let r = tg_result(telegram::apply(&t));
            self.log(r.message);
        }
    }
}
