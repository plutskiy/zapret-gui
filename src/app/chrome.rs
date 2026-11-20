use super::*;

impl App {
    // ------------------------------------------------------------ chrome

    pub(super) fn sidebar(&mut self, ui: &mut Ui, state: Hero) {
        let tabs = [
            (Tab::Home, Icon::Home, "Главная"),
            (Tab::Telegram, Icon::Send, "Telegram"),
            (Tab::Settings, Icon::Settings, "Настройки"),
        ];
        ui.spacing_mut().item_spacing.y = 4.0;
        for (tab, icon, name) in tabs {
            if nav_item(ui, icon, name, self.tab == tab).clicked() {
                self.tab = tab;
            }
        }
        let (title, _) = self.hero_texts(state);
        let cfg = match state {
            Hero::On if !self.settings.last_config.is_empty() => self.settings.last_config.clone(),
            Hero::Working => "подбор…".into(),
            _ => "—".into(),
        };
        let lit = state.glow_rgb();
        ui.with_layout(Layout::bottom_up(Align::Min), |ui| {
            egui::Frame::new()
                .fill(CARD)
                .stroke(Stroke::new(1.0, LINE_SOFT))
                .corner_radius(12)
                .inner_margin(Margin::symmetric(14, 12))
                .show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = 10.0;
                        let (r, _) = ui.allocate_exact_size(vec2(8.0, 8.0), Sense::hover());
                        if let Some((rr, gg, bb)) = lit {
                            ui.painter().add(
                                egui::epaint::Shadow {
                                    offset: [0, 0],
                                    blur: 10,
                                    spread: 0,
                                    color: rgba(rr, gg, bb, 200),
                                }
                                .as_shape(r, cr(4.0)),
                            );
                        }
                        ui.painter().circle_filled(r.center(), 4.0, state.tone());
                        ui.vertical(|ui| {
                            ui.spacing_mut().item_spacing.y = 3.0;
                            ui.add(
                                egui::Label::new(rt(title, 13.0, W::Semi, TEXT)).selectable(false),
                            );
                            let w = ui.available_width();
                            let (r, _) = ui.allocate_exact_size(vec2(w, 14.0), Sense::hover());
                            text_trunc(
                                ui.painter(),
                                r.left_center(),
                                &cfg,
                                font(12.0, W::Mono),
                                TEXT3,
                                w,
                            );
                        });
                    });
                });
        });
    }

    pub(super) fn log_bar(&mut self, ui: &mut Ui, state: Hero) {
        let w = ui.available_width();
        let (rect, resp) = ui.allocate_exact_size(vec2(w, 38.0), Sense::click());
        let p = ui.painter().clone();
        p.rect_filled(Rect::from_min_size(rect.min, vec2(w, 1.0)), 0, LINE_SOFT);
        icon_at(
            ui,
            pos2(rect.left() + 20.0 + 7.0, rect.center().y),
            14.0,
            if self.show_log {
                Icon::ChevD
            } else {
                Icon::ChevR
            },
            TEXT3,
        );
        let lx = rect.left() + 20.0 + 14.0 + 10.0;
        p.text(
            pos2(lx, rect.center().y),
            Align2::LEFT_CENTER,
            "Журнал",
            font(12.0, W::Med),
            TEXT2,
        );
        let jw = text_w(ui, "Журнал", &font(12.0, W::Med));
        if !self.show_log {
            let line = self
                .log
                .last()
                .cloned()
                .unwrap_or_else(|| Self::default_log_line(state).to_string());
            let color = if line.starts_with("ОШИБКА") {
                DANGER
            } else {
                TEXT3
            };
            let x = lx + jw + 10.0;
            text_trunc(
                &p,
                pos2(x, rect.center().y),
                &line,
                font(12.0, W::Mono),
                color,
                rect.right() - 20.0 - x,
            );
        }
        if resp
            .on_hover_cursor(egui::CursorIcon::PointingHand)
            .clicked()
        {
            self.show_log = !self.show_log;
        }
        if self.show_log {
            egui::Frame::new()
                .inner_margin(Margin {
                    left: 20,
                    right: 20,
                    top: 0,
                    bottom: 10,
                })
                .show(ui, |ui| {
                    egui::ScrollArea::vertical()
                        .max_height(130.0)
                        .stick_to_bottom(true)
                        .auto_shrink([false, true])
                        .show(ui, |ui| {
                            ui.set_width(ui.available_width());
                            ui.spacing_mut().item_spacing.y = 4.0;
                            for line in &self.log {
                                let c = if line.starts_with("ОШИБКА") {
                                    DANGER
                                } else {
                                    TEXT2
                                };
                                ui.add(egui::Label::new(rt(line, 12.0, W::Mono, c)));
                            }
                        });
                });
        }
    }
}
