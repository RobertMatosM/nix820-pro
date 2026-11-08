use super::*;

const SECTIONS: &[(usize, &str)] = &[
    (0, "Overview"),
    (1, "Keymap"),
    (3, "Lighting"),
    (4, "Display"),
    (8, "Device"),
];

impl eframe::App for Studio {
    fn ui(&mut self, root_ui: &mut Ui, _frame: &mut eframe::Frame) {
        let ctx = root_ui.ctx().clone();
        self.commit_navigation();
        self.poll_transfer();
        if self.transfer.is_some() && ctx.input(|i| i.viewport().close_requested()) {
            ctx.send_viewport_cmd(ViewportCommand::CancelClose);
            self.notify("Wait for the keyboard transfer to finish before closing.");
        }
        self.update_section_transition();
        let progress = self
            .transition_started
            .map(|t| (t.elapsed().as_secs_f32() / 0.22).min(1.))
            .unwrap_or(1.);
        let eased = 1. - (1. - progress).powi(3);
        CentralPanel::default()
            .frame(Frame::NONE.fill(BG))
            .show(root_ui, |ui| {
                let rect = ui.max_rect();
                let scale = (rect.width() / 1480.).min(rect.height() / 800.);
                let origin = rect.min
                    + vec2(
                        (rect.width() - 1480. * scale) / 2.,
                        (rect.height() - 800. * scale) / 2.,
                    );
                let c = Canvas {
                    p: ui.painter().clone(),
                    s: scale,
                    o: origin,
                };
                c.fill(Rect::from_min_size(Pos2::ZERO, vec2(1480., 800.)), 0., BG);
                c.fill(
                    Rect::from_min_size(Pos2::ZERO, vec2(204., 800.)),
                    0.,
                    SIDEBAR,
                );
                for i in 0..3 {
                    c.fill(
                        Rect::from_min_size(
                            pos2(22. + i as f32 * 9., 24.),
                            vec2(6., 14. - i as f32 * 3.),
                        ),
                        2.,
                        ACCENT,
                    );
                }
                c.text(pos2(61., 31.), "Nix820 Pro", 17., FG, Align2::LEFT_CENTER);
                c.text(
                    pos2(226., 31.),
                    SECTIONS
                        .iter()
                        .find(|(page, _)| *page == self.page)
                        .map(|(_, name)| *name)
                        .unwrap_or("Overview"),
                    21.,
                    FG,
                    Align2::LEFT_CENTER,
                );
                c.text(
                    pos2(1235., 31.),
                    if self.receiver_selected() {
                        "AK820 Pro · 2.4 GHz"
                    } else {
                        "AK820 Pro · USB"
                    },
                    13.,
                    MUTED,
                    Align2::RIGHT_CENTER,
                );
                if c.button_enabled(
                    ui,
                    Rect::from_min_size(pos2(1260., 16.), vec2(192., 30.)),
                    "Connection",
                    13.,
                    self.transfer.is_none(),
                )
                .clicked()
                {
                    self.scan();
                    self.hardware_open = true;
                }
                c.line(pos2(204., 58.), pos2(1480., 58.), BORDER, 1.);
                self.sidebar(ui, &c);
                let mut painter = c.p.clone();
                painter.set_opacity(eased);
                let content = Canvas {
                    p: painter,
                    s: scale,
                    o: origin + vec2((154. + 8. * (1. - eased)) * scale, 0.),
                };
                match self.page {
                    1 => self.keys(ui, &content),
                    3 => self.lighting(ui, &content),
                    4 => self.screen(ui, &content, &ctx),
                    8 => self.settings_page(ui, &content),
                    _ => self.home(ui, &c, &ctx),
                }
                c.text(
                    pos2(226., 771.),
                    "Apply sends local edits. Wired USB: all features. 2.4 GHz: selected keys, lighting, settings and resets.",
                    12.,
                    MUTED,
                    Align2::LEFT_CENTER,
                );
                if let Some((message, time)) = &self.toast
                    && time.elapsed().as_secs() < 6
                {
                    let r = Rect::from_min_size(pos2(480., 714.), vec2(945., 40.));
                    c.fill(r, 6., SELECTION);
                    c.text(r.center(), message, 12., FG, Align2::CENTER_CENTER);
                }
            });
        self.hardware_window(&ctx);
        self.reset_window(&ctx);
        self.image_picker_window(&ctx);
        self.about_window(&ctx);
        if let Some(op) = self.file_dialog.clone() {
            let mut open = true;
            Window::new(&op)
                .open(&mut open)
                .collapsible(false)
                .resizable(false)
                .show(&ctx, |ui| {
                    ui.label("File on this computer:");
                    ui.add(TextEdit::singleline(&mut self.file_path).desired_width(560.));
                    ui.horizontal(|ui| {
                        if ui.button("Continue").clicked() {
                            let path = self.file_path.clone();
                            match self.file_op(&ctx, &op, &path) {
                                Ok(message) => {
                                    self.notify(message);
                                    self.file_dialog = None;
                                }
                                Err(error) => self.notify(error),
                            }
                        }
                        if ui.button("Cancel").clicked() {
                            self.file_dialog = None;
                        }
                    });
                });
            if !open {
                self.file_dialog = None;
            }
        }
        if self.rename {
            let mut open = true;
            Window::new("Rename profile")
                .open(&mut open)
                .collapsible(false)
                .show(&ctx, |ui| {
                    ui.text_edit_singleline(&mut self.rename_value);
                    if ui.button("Save").clicked() && !self.rename_value.trim().is_empty() {
                        self.pm().name = self.rename_value.trim().into();
                        self.save();
                        self.rename = false;
                    }
                });
            if !open {
                self.rename = false;
            }
        }
        self.frames += 1;
        if self.frames == 30
            && let Some(path) = self.screenshot.clone()
        {
            let context = ctx.clone();
            ctx.request_screenshot(move |image| {
                let bytes: Vec<_> = image.pixels.iter().flat_map(|p| p.to_array()).collect();
                if let Err(error) = image::save_buffer(
                    &path,
                    &bytes,
                    image.width() as u32,
                    image.height() as u32,
                    image::ColorType::Rgba8,
                ) {
                    eprintln!("Screenshot failed: {error}");
                }
                context.send_viewport_cmd(ViewportCommand::Close);
            });
        }
        if progress < 1.
            || self.pending_mode.is_some()
            || self.transfer.is_some()
            || self.image_picker.is_some()
            || self.preview
            || (self.page == 3 && self.screenshot.is_none())
            || (self.screenshot.is_some() && self.frames < 35)
            || self
                .toast
                .as_ref()
                .is_some_and(|(_, time)| time.elapsed().as_secs() < 6)
        {
            ctx.request_repaint_after(std::time::Duration::from_millis(16));
        }
    }
}
impl Studio {
    fn commit_navigation(&mut self) {
        if let Some(change) = self.pending_mode.take() {
            match change {
                ModeChange::Page(page) => self.page = page,
                ModeChange::Lighting(mode) => {
                    self.pm().mode = mode;
                    self.save();
                }
            }
        }
    }
    fn update_section_transition(&mut self) {
        if self.page != self.transition_page {
            self.transition_page = self.page;
            self.transition_started = self.screenshot.is_none().then(Instant::now);
        }
    }
    pub(super) fn sidebar(&mut self, ui: &mut Ui, c: &Canvas) {
        for (index, (page, label)) in SECTIONS.iter().enumerate() {
            if c.button(
                ui,
                Rect::from_min_size(pos2(16., 84. + index as f32 * 46.), vec2(172., 36.)),
                label,
                self.page == *page,
                15.,
            )
            .clicked()
            {
                self.pending_mode = Some(ModeChange::Page(*page));
            }
        }
        c.text(
            pos2(22., 687.),
            "Independent project",
            12.,
            MUTED,
            Align2::LEFT_CENTER,
        );
        c.text(
            pos2(22., 710.),
            format!("v{} · 0BSD", env!("CARGO_PKG_VERSION")),
            12.,
            MUTED,
            Align2::LEFT_CENTER,
        );
        if c.button(
            ui,
            Rect::from_min_size(pos2(16., 742.), vec2(172., 32.)),
            "About",
            false,
            13.,
        )
        .clicked()
        {
            self.about_open = true;
        }
    }
    fn about_window(&mut self, ctx: &Context) {
        if !self.about_open {
            return;
        }
        Window::new("About Nix820 Pro").open(&mut self.about_open).resizable(false).show(ctx,|ui| {
            ui.heading("Nix820 Pro");
            ui.label("Independent AK820 Pro configurator for Linux.");
            ui.label("Not affiliated with or endorsed by AJAZZ. AJAZZ and AK820 Pro identify the supported device.");
            ui.separator();
            ui.label("Original project code: BSD Zero Clause (0BSD). Bundled fonts and dependencies retain their own licenses; see THIRD_PARTY_NOTICES.md.");
            ui.label("Supported: wired keymaps (Base/Fn), media/browser actions, lighting, display uploads, clock sync, settings and resets.");
            ui.label("2.4 GHz receiver: selected-key remapping, lighting, device settings and resets. Bluetooth configuration, macros, custom per-key lighting and audio-driven effects are not implemented.");
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn section_transition_ignores_lighting_and_profile_changes() {
        let ctx = Context::default();
        let marker = Some(PathBuf::from("unused.png"));
        let mut app = Studio::create(&ctx, marker.clone(), 3);
        app.pending_mode = Some(ModeChange::Lighting(6));
        app.commit_navigation();
        app.screenshot = None;
        app.update_section_transition();
        assert!(app.transition_started.is_none());
        app.saved.selected = 1;
        app.update_section_transition();
        assert!(app.transition_started.is_none());
        app.pending_mode = Some(ModeChange::Page(4));
        app.commit_navigation();
        app.update_section_transition();
        let started = app.transition_started.unwrap();
        app.pending_mode = Some(ModeChange::Page(4));
        app.commit_navigation();
        app.update_section_transition();
        assert_eq!(app.transition_started, Some(started));
        app.screenshot = marker;
    }
    #[test]
    fn release_navigation_contains_only_implemented_sections() {
        assert_eq!(
            SECTIONS.iter().map(|(page, _)| *page).collect::<Vec<_>>(),
            vec![0, 1, 3, 4, 8]
        );
        let ctx = Context::default();
        let mut app = Studio::create(&ctx, Some(PathBuf::from("unused.png")), 1);
        let pos = pos2(90., 84. + 4. * 46. + 18.);
        for pressed in [None, Some(true), Some(false)] {
            let events = pressed
                .map(|pressed| {
                    vec![Event::PointerButton {
                        pos,
                        pressed,
                        button: PointerButton::Primary,
                        modifiers: Modifiers::NONE,
                    }]
                })
                .unwrap_or_else(|| vec![Event::PointerMoved(pos)]);
            let mut output = ctx.run_ui(
                RawInput {
                    screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(1480., 800.))),
                    events,
                    ..Default::default()
                },
                |ui| {
                    let c = Canvas {
                        p: ui.painter().clone(),
                        s: 1.,
                        o: Pos2::ZERO,
                    };
                    app.sidebar(ui, &c);
                },
            );
            output.textures_delta.clear();
        }
        assert!(matches!(app.pending_mode, Some(ModeChange::Page(8))));
        assert!(app.transfer.is_none());
    }
}
