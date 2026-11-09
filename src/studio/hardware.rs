use super::*;
use crate::{device::Job, protocol};

impl Studio {
    pub(super) fn receiver_selected(&self) -> bool {
        self.keyboards
            .get(self.keyboard_index)
            .is_some_and(|keyboard| keyboard.connection == crate::device::Connection::Receiver)
    }
    pub(super) fn wired_actions_available(&self) -> bool {
        !self.receiver_selected() && self.transfer.is_none()
    }
    pub(super) fn reset_description(&self, kind: protocol::ResetKind) -> &'static str {
        if self.receiver_selected() {
            match kind {
                protocol::ResetKind::Keyboard => {
                    "Reset the connected keyboard over 2.4 GHz. Keyboard-side changes may be lost. Saved profiles and display presets on this computer are kept."
                }
                protocol::ResetKind::Factory => {
                    "Restore the connected keyboard's factory defaults over 2.4 GHz. Keyboard-side changes may be lost. Saved profiles and display presets on this computer are kept."
                }
            }
        } else {
            kind.description()
        }
    }
    fn begin_transfer(&mut self, job: Job) {
        if self.screenshot.is_some() {
            return;
        }
        if self.transfer.is_some() {
            self.notify("A keyboard transfer is already running.");
            return;
        }
        let Some(keyboard) = self.keyboards.get(self.keyboard_index).cloned() else {
            self.transfer_status = "No supported keyboard found. Connect by wired USB or use the 2.4 GHz receiver for lighting or settings, then refresh in Connection.".into();
            self.notify(self.transfer_status.clone());
            self.hardware_open = true;
            return;
        };
        if let Err(error) = job.check_connection(keyboard.connection) {
            self.transfer_status = error;
            self.notify(self.transfer_status.clone());
            self.hardware_open = true;
            return;
        }
        let (tx, rx) = std::sync::mpsc::channel();
        self.transfer = Some(rx);
        self.transfer_status = "Transferring to keyboard…".into();
        std::thread::spawn(move || {
            let _ = tx.send(job.run(&keyboard));
        });
    }
    pub(super) fn reset_window(&mut self, ctx: &Context) {
        let Some(kind) = self.pending_reset else {
            return;
        };
        let mut open = true;
        let can_reset = self.transfer.is_none() && !self.keyboards.is_empty();
        Window::new(kind.label()).open(&mut open).collapsible(false).resizable(false)
            .default_width(430.).show(ctx, |ui| {
                ui.label(self.reset_description(kind));
                if let Some(keyboard) = self.keyboards.get(self.keyboard_index) {
                    ui.label(format!("Keyboard: {}", keyboard.label));
                } else {
                    ui.label("Connect a supported keyboard or 2.4 GHz receiver to reset it.");
                }
                ui.label("Saved profiles and TFT presets on this computer will be kept. The editor is not a live view of the keyboard; Apply sends its saved values again.");
                ui.horizontal(|ui| {
                    if ui.add_enabled(can_reset, egui::Button::new(kind.label())).clicked() {
                        self.pending_reset = None;
                        self.begin_transfer(Job::Reset(kind));
                    }
                    if ui.button("Cancel").clicked() { self.pending_reset = None; }
                });
            });
        if !open {
            self.pending_reset = None;
        }
    }
    pub(super) fn poll_transfer(&mut self) {
        let result = self.transfer.as_ref().and_then(|rx| match rx.try_recv() {
            Ok(r) => Some(r),
            Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                Some(Err("Transfer worker stopped unexpectedly.".into()))
            }
            Err(_) => None,
        });
        if let Some(r) = result {
            self.transfer = None;
            self.transfer_status = match r {
                Ok(s) => s,
                Err(e) => format!("Transfer stopped: {e}"),
            };
            self.notify(self.transfer_status.clone());
            self.hardware_open = true;
        }
    }
    fn start_lighting(&mut self) {
        let p = self.p();
        match protocol::lighting(p.mode, p.rgb, p.colorful, p.brightness, p.speed) {
            Ok(report) => self.begin_transfer(Job::Lighting(report)),
            Err(e) => self.notify(e),
        }
    }
    pub(super) fn map_job(&self) -> Result<Job, String> {
        let layer = crate::remap::Layer::from_fn(self.layer);
        if self.receiver_selected() {
            let source = self.key.as_deref().ok_or("Select a physical key first.")?;
            let destination = self.p().destination_for_key(layer, source)?;
            let destination = destination.or_else(|| {
                (layer == crate::remap::Layer::Top).then(|| crate::remap::base_key(source)).flatten()
            }).ok_or("Choose an explicit Fn destination, or click Use normal key. Factory Fn defaults require wired USB.")?;
            protocol::wireless::remap(layer, source, destination)?;
            Ok(Job::Remap {
                layer,
                source: source.into(),
                destination,
            })
        } else {
            self.p()
                .map_for_layer(layer)
                .map(|map| Job::Map(Box::new(map)))
        }
    }
    pub(super) fn can_apply_map(&self) -> bool {
        self.transfer.is_none() && (!self.receiver_selected() || self.map_job().is_ok())
    }
    pub(super) fn start_map(&mut self) {
        match self.map_job() {
            Ok(job) => self.begin_transfer(job),
            Err(error) => self.notify(error),
        }
    }

    pub(super) fn start_settings(&mut self) {
        let e = &self.saved.extras;
        let report = e
            .response
            .checked_add(1)
            .ok_or_else(|| "Invalid response level.".to_string())
            .and_then(|level| {
                protocol::settings(
                    level,
                    e.sleep,
                    e.disable_win,
                    e.disable_alt_f4,
                    e.disable_alt_tab,
                )
            });
        match report {
            Ok(report) => self.begin_transfer(Job::Settings(report)),
            Err(e) => self.notify(e),
        }
    }
    pub(super) fn start_tft(&mut self) {
        self.delays[self.frame_index] = self.delay;
        let encoded = if self.pixels.len() == 1 && self.delay == 50 {
            protocol::tft_image(&self.pixels[0], 160, 128)
        } else {
            protocol::tft_animation(&self.pixels, &self.delays, 160, 128)
        };
        match encoded {
            Ok(blocks) => self.begin_transfer(Job::Tft(blocks)),
            Err(e) => self.notify(e),
        }
    }
    pub(super) fn start_time_sync(&mut self) {
        let result = std::process::Command::new("date")
            .arg("+%y %m %d %H %M %S %u")
            .output()
            .map_err(|e| e.to_string())
            .and_then(|out| {
                if !out.status.success() {
                    return Err("Could not read local time".into());
                }
                let fields = String::from_utf8_lossy(&out.stdout)
                    .split_whitespace()
                    .map(|s| s.parse::<u8>().map_err(|e| e.to_string()))
                    .collect::<Result<Vec<_>, _>>()?;
                protocol::time_report(fields.try_into().map_err(|_| "Invalid local time")?)
            });
        match result {
            Ok(report) => self.begin_transfer(Job::TimeSync(report)),
            Err(e) => self.notify(e),
        }
    }
    pub(super) fn apply_button(&mut self, ui: &mut Ui, c: &Canvas, label: &str, page: usize) {
        if c.button_enabled(
            ui,
            Rect::from_min_size(pos2(330., 81.), vec2(188., 30.)),
            label,
            14.,
            if page == 1 { self.can_apply_map() } else { self.transfer.is_none() },
        )
        .on_hover_text(if page == 1 && self.receiver_selected() {
            "Sends only the selected key's local action over 2.4 GHz. Other device mappings are kept."
        } else if page == 1 {
            "Replaces all overrides in the selected layer with this profile. Other device remaps in that layer are not merged."
        } else {
            "Apply lighting over wired USB or the 2.4 GHz receiver. Changes stay local until you click Apply."
        })
        .on_disabled_hover_text(if self.transfer.is_some() { "Wait for the current transfer to finish.".into() }
            else if page == 1 { self.map_job().err().unwrap_or_else(|| "Select a key and destination.".into()) }
            else { "Wait for the current transfer to finish.".into() })
        .clicked()
        {
            if page == 1 {
                self.start_map();
            } else {
                self.start_lighting();
            }
        }
    }
    pub(super) fn hardware_window(&mut self, ctx: &Context) {
        if !self.hardware_open {
            return;
        }
        let mut open = true;
        Window::new("Keyboard connection").open(&mut open).default_width(620.).resizable(false).show(ctx, |ui| {
            ui.label("AK820 Pro · wired USB or 2.4 GHz receiver");
            ui.label("2.4 GHz supports selected-key remapping, lighting, device settings and resets. Whole-layer defaults and display transfers need wired USB.");
            if ui.add_enabled(self.transfer.is_none(), egui::Button::new("Refresh devices")).clicked() { self.scan(); }
            if self.keyboards.is_empty() { ui.label("No supported HID interfaces are visible."); }
            for (i,k) in self.keyboards.iter().enumerate() {
                ui.add_enabled_ui(self.transfer.is_none(), |ui| { ui.radio_value(&mut self.keyboard_index, i, &k.label); });
                if !k.feature.exists() { ui.label("Device node is unavailable in this environment."); }
            }
            ui.separator();
            ui.label("Lighting: all twenty modes from Static through LED Off.");
            if self.receiver_selected() { ui.label("Receiver selected: RGB, colorful, brightness and speed are available. Device settings and resets are available too. Check the physical keyboard after a transfer."); }
            ui.label("TFT: upload the selected preset (up to 140 frames) at 128 × 128, or sync the local clock.");
            ui.label("Base and Fn keymaps, lighting, display uploads and device settings are supported over wired USB.");
            ui.label(if self.receiver_selected() { "Apply selected key sends one action. Fn factory defaults are unavailable; Use normal key assigns the printed key." } else { "Applying a keymap replaces that layer’s overrides with the selected local profile." });
            ui.label("Fn reserves Esc, Backspace, Home, Win, Fn and Backslash/Pipe.");
            ui.label("The editor shows saved local values; it does not read the keyboard’s current configuration.");
            ui.add_enabled_ui(self.transfer.is_none(), |ui| {
                ui.horizontal(|ui| {
                    if ui.button("Apply lighting").clicked() { self.start_lighting(); }
                    if ui.add_enabled(self.can_apply_map(), egui::Button::new(if self.receiver_selected() { "Apply selected key" } else if self.layer { "Apply Fn layer" } else { "Apply Base layer" })).clicked() { self.start_map(); }
                    if ui.add_enabled(!self.receiver_selected(), egui::Button::new("Upload TFT preset")).clicked() { self.start_tft(); }
                });
            });
            ui.separator();
            ui.label(&self.transfer_status);
        });
        self.hardware_open = open;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn step(app: &mut Studio, ctx: &Context, page: usize, events: Vec<Event>) {
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
                match page {
                    1 => app.apply_button(ui, &c, "Apply Base layer", 1),
                    4 => app.screen(ui, &c, ctx),
                    _ => app.settings_page(ui, &c),
                }
            },
        );
        output.textures_delta.clear();
    }
    fn click(app: &mut Studio, ctx: &Context, page: usize, pos: Pos2) {
        step(app, ctx, page, vec![Event::PointerMoved(pos)]);
        for pressed in [true, false] {
            step(
                app,
                ctx,
                page,
                vec![Event::PointerButton {
                    pos,
                    pressed,
                    button: PointerButton::Primary,
                    modifiers: Modifiers::NONE,
                }],
            );
        }
    }
    #[test]
    fn receiver_blocks_layer_maps_and_display_but_allows_selected_key_settings_and_reset() {
        let ctx = Context::default();
        let mut app = Studio::create(&ctx, Some(PathBuf::from("unused.png")), 0);
        assert!(
            app.keyboards.is_empty(),
            "screenshot mode must not scan host devices"
        );
        app.keyboards = vec![crate::device::Keyboard {
            connection: crate::device::Connection::Receiver,
            feature: "/nonexistent/receiver".into(),
            display: None,
            label: "test receiver".into(),
        }];
        // Detect accidental clicks via the transfer status, without allowing persistence.
        app.screenshot = None;
        app.persistence_ready = false;
        let status = app.transfer_status.clone();
        for (page, pos) in [
            (1, pos2(420., 96.)),
            (4, pos2(1140., 597.)),
            (4, pos2(1140., 702.)),
        ] {
            click(&mut app, &ctx, page, pos);
            assert!(app.transfer.is_none());
            assert!(app.pending_reset.is_none());
            assert_eq!(app.transfer_status, status);
        }
        click(&mut app, &ctx, 4, pos2(1140., 632.));
        assert_eq!(app.file_dialog.as_deref(), Some("Export PNG"));
        assert!(app.receiver_selected());
        assert!(!app.wired_actions_available());
        app.file_dialog = None;
        app.key = Some("A".into());
        app.pm()
            .assign_layer(
                crate::remap::Layer::Top,
                "A",
                Some(crate::remap::KeyCode::B),
            )
            .unwrap();
        click(&mut app, &ctx, 1, pos2(420., 96.));
        assert!(
            app.transfer.is_some(),
            "Apply selected key must start a receiver job"
        );
        app.transfer = None;
        click(&mut app, &ctx, 8, pos2(420., 96.));
        assert!(
            app.transfer.is_some(),
            "Apply settings must start a receiver job"
        );
        app.transfer = None;
        for (pos, kind) in [
            (pos2(183., 618.), protocol::ResetKind::Keyboard),
            (pos2(183., 668.), protocol::ResetKind::Factory),
        ] {
            click(&mut app, &ctx, 8, pos);
            assert_eq!(app.pending_reset, Some(kind));
            assert!(app.transfer.is_none(), "Reset must wait for confirmation");
            assert!(app.reset_description(kind).contains("2.4 GHz"));
            app.pending_reset = None;
        }
    }
}
