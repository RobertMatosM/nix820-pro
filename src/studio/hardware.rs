use super::*;
use crate::{device::Job, protocol};

impl Studio {
    fn begin_transfer(&mut self, job: Job) {
        if self.screenshot.is_some() {
            return;
        }
        if self.transfer.is_some() {
            self.notify("A keyboard transfer is already running.");
            return;
        }
        let Some(keyboard) = self.keyboards.get(self.keyboard_index).cloned() else {
            self.transfer_status = "No supported wired keyboard found. Connect by USB and refresh using the cable icon.".into();
            self.notify(self.transfer_status.clone());
            self.hardware_open = true;
            return;
        };
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
        Window::new(kind.label()).open(&mut open).collapsible(false).resizable(false)
            .default_width(430.).show(ctx, |ui| {
                ui.label(kind.description());
                if let Some(keyboard) = self.keyboards.get(self.keyboard_index) {
                    ui.label(format!("Keyboard: {}", keyboard.label));
                } else {
                    ui.label("Connect a supported keyboard by wired USB to reset it.");
                }
                ui.label("Saved profiles and TFT presets on this computer will be kept. The editor is not a live view of the keyboard; Apply sends its saved values again.");
                ui.horizontal(|ui| {
                    if ui.add_enabled(self.transfer.is_none() && !self.keyboards.is_empty(), egui::Button::new(kind.label())).clicked() {
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
    pub(super) fn start_map(&mut self) {
        match self
            .p()
            .map_for_layer(crate::remap::Layer::from_fn(self.layer))
        {
            Ok(map) => self.begin_transfer(Job::Map(Box::new(map))),
            Err(e) => self.notify(e),
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
        if c.button(
            ui,
            Rect::from_min_size(pos2(330., 81.), vec2(188., 30.)),
            label,
            false,
            14.,
        )
        .on_hover_text(if page == 1 {
            "Replaces all overrides in the selected layer with this profile. Other device remaps in that layer are not merged."
        } else {
            "Experimental wired USB transfer. Changes stay local until you click Apply."
        })
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
        Window::new("Wired keyboard").open(&mut open).default_width(540.).resizable(false).show(ctx, |ui| {
            ui.label("AK820 Pro · wired USB connection");
            ui.label("Connect in wired USB mode. The 2.4G receiver is not supported.");
            if ui.add_enabled(self.transfer.is_none(), egui::Button::new("Refresh devices")).clicked() { self.scan(); }
            if self.keyboards.is_empty() { ui.label("No supported wired HID interfaces are visible."); }
            for (i,k) in self.keyboards.iter().enumerate() {
                ui.radio_value(&mut self.keyboard_index, i, &k.label);
                if !k.feature.exists() { ui.label("Device node is unavailable in this environment."); }
            }
            ui.separator();
            ui.label("Lighting: all twenty modes from Static through LED Off.");
            ui.label("TFT: upload the selected preset (up to 140 frames) at 128 × 128, or sync the local clock.");
            ui.label("Base and Fn keymaps, lighting, display uploads and device settings are supported over wired USB.");
            ui.label("Applying a keymap replaces that layer’s overrides with the selected local profile.");
            ui.label("Fn reserves Esc, Backspace, Home, Win, Fn and Backslash/Pipe.");
            ui.label("The editor shows saved local values; it does not read the keyboard’s current configuration.");
            ui.add_enabled_ui(self.transfer.is_none(), |ui| {
                ui.horizontal(|ui| {
                    if ui.button("Apply lighting").clicked() { self.start_lighting(); }
                    if ui.button(if self.layer { "Apply Fn layer" } else { "Apply Base layer" }).clicked() { self.start_map(); }
                    if ui.button("Upload TFT preset").clicked() { self.start_tft(); }
                });
            });
            ui.separator();
            ui.label(&self.transfer_status);
        });
        self.hardware_open = open;
    }
}
