use super::*;

impl Studio {
    pub(super) fn file_op(
        &mut self,
        ctx: &Context,
        op: &str,
        path: &str,
    ) -> Result<String, String> {
        let p = PathBuf::from(path);
        match op {
            "Export profile" => {
                std::fs::write(
                    &p,
                    serde_json::to_vec_pretty(self.p()).map_err(|e| e.to_string())?,
                )
                .map_err(|e| e.to_string())?;
            }
            "Import profile" => {
                let mut v: Profile =
                    serde_json::from_slice(&std::fs::read(&p).map_err(|e| e.to_string())?)
                        .map_err(|e| e.to_string())?;
                if !v.valid() {
                    return Err("Invalid profile settings".into());
                }
                v.migrate_key_maps();
                self.saved.profiles.push(v);
                self.saved.selected = self.saved.profiles.len() - 1;
                self.save();
            }
            "Import image" => {
                if p.extension()
                    .is_some_and(|e| e.to_string_lossy().eq_ignore_ascii_case("gif"))
                {
                    let (frames, delays) = gif_io::read_animation(&p)?;
                    self.pixels = frames;
                    self.delays = delays;
                    self.delay = self.delays[0];
                    self.frame_index = 0;
                    self.thumb_offset = 0.;
                } else {
                    if self.pixels.len() >= 140 {
                        return Err("The screen project can hold up to 140 frames. Delete a frame before importing.".into());
                    }
                    let im = image::open(&p)
                        .map_err(|e| e.to_string())?
                        .resize_exact(160, 128, image::imageops::FilterType::Triangle)
                        .into_rgba8();
                    self.insert_frame(false);
                    self.pixels[self.frame_index] = im.into_raw();
                }
                self.undo.clear();
                self.preview = false;
                self.dirty_texture = true;
            }
            "Export GIF" => {
                self.delays[self.frame_index] = self.delay;
                gif_io::write_animation(&p, &self.pixels, &self.delays)?;
            }
            "Export PNG" => {
                let mut px = self.pixels[self.frame_index].clone();
                for c in px.chunks_mut(4) {
                    c[3] = 255;
                }
                image::save_buffer(&p, &px, 160, 128, image::ColorType::Rgba8)
                    .map_err(|e| e.to_string())?;
            }
            _ => return Err("Unknown operation".into()),
        }
        ctx.request_repaint();
        Ok(format!("{op} complete"))
    }
    pub(super) fn open_file(&mut self, op: &str) {
        if op == "Import image" {
            self.select_image();
            return;
        }
        self.file_dialog = Some(op.into());
        self.file_path = self
            .paths
            .data
            .clone()
            .join(match op {
                "Export profile" => "profile.json",
                "Export GIF" => "screen.gif",
                "Export PNG" => "screen.png",
                "Export analysis" => "hid-report.md",
                "Export macro" => "macro.json",
                "Export local effect" => "lighting-effect.json",
                _ => "",
            })
            .to_string_lossy()
            .into_owned();
    }
}

fn image_file(path: &std::path::Path) -> bool {
    path.extension().is_some_and(|ext| {
        ["png", "jpg", "jpeg", "gif"]
            .iter()
            .any(|e| ext.to_string_lossy().eq_ignore_ascii_case(e))
    })
}

impl Studio {
    fn select_image(&mut self) {
        if self.image_picker.is_some() || self.image_browser.is_some() {
            return;
        }
        let (tx, rx) = std::sync::mpsc::channel();
        self.image_picker = Some(rx);
        std::thread::spawn(move || {
            let result = std::process::Command::new("zenity")
                .args([
                    "--file-selection",
                    "--title=Import TFT image",
                    "--file-filter=Images | *.png *.jpg *.jpeg *.gif *.PNG *.JPG *.JPEG *.GIF",
                ])
                .output()
                .map_err(|e| e.to_string())
                .and_then(|out| {
                    if out.status.success() {
                        let path = String::from_utf8(out.stdout).map_err(|e| e.to_string())?;
                        let path = path.trim_end_matches(['\r', '\n']);
                        Ok((!path.is_empty()).then(|| PathBuf::from(path)))
                    } else if out.status.code() == Some(1)
                        && !String::from_utf8_lossy(&out.stderr)
                            .to_lowercase()
                            .contains("cannot open display")
                        && !String::from_utf8_lossy(&out.stderr)
                            .to_lowercase()
                            .contains("failed")
                        && !String::from_utf8_lossy(&out.stderr)
                            .to_lowercase()
                            .contains("error")
                    {
                        Ok(None) // Cancel leaves the editor untouched.
                    } else {
                        Err("Native file selector unavailable".into())
                    }
                });
            let _ = tx.send(result);
        });
    }

    pub(super) fn image_picker_window(&mut self, ctx: &Context) {
        let result = self
            .image_picker
            .as_ref()
            .and_then(|rx| match rx.try_recv() {
                Ok(r) => Some(r),
                Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                    Some(Err("File selector closed".into()))
                }
                Err(_) => None,
            });
        if let Some(result) = result {
            self.image_picker = None;
            match result {
                Ok(Some(path)) => self.import_selected_image(ctx, path),
                Ok(None) => {}
                Err(_) => {
                    self.image_browser = Some(
                        std::env::var_os("HOME")
                            .map(PathBuf::from)
                            .unwrap_or_else(|| self.paths.data.clone()),
                    );
                    self.image_selection = None;
                }
            }
        }
        // Built-in fallback for desktops without a native chooser.
        let Some(directory) = self.image_browser.clone() else {
            return;
        };
        let mut open = true;
        let mut chosen = None;
        Window::new("Import TFT image")
            .open(&mut open)
            .default_width(620.)
            .resizable(false)
            .collapsible(false)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    if ui.button("Home").clicked() {
                        self.image_browser = Some(
                            std::env::var_os("HOME")
                                .map(PathBuf::from)
                                .unwrap_or_else(|| self.paths.data.clone()),
                        );
                        self.image_selection = None;
                    }
                    if ui
                        .add_enabled(directory.parent().is_some(), Button::new("Up one folder"))
                        .clicked()
                    {
                        self.image_browser = directory.parent().map(std::path::Path::to_path_buf);
                        self.image_selection = None;
                    }
                    if ui.button("Pictures").clicked() {
                        let home = std::env::var_os("HOME")
                            .map(PathBuf::from)
                            .unwrap_or_else(|| self.paths.data.clone());
                        self.image_browser = Some(home.join("Pictures"));
                        self.image_selection = None;
                    }
                });
                ui.label(directory.to_string_lossy());
                ui.separator();
                match std::fs::read_dir(&directory) {
                    Ok(entries) => {
                        let mut entries: Vec<PathBuf> = entries
                            .flatten()
                            .map(|e| e.path())
                            .filter(|p| {
                                !p.file_name()
                                    .is_some_and(|n| n.to_string_lossy().starts_with('.'))
                                    && (p.is_dir() || image_file(p))
                            })
                            .collect();
                        entries.sort_by_key(|p| {
                            (
                                !p.is_dir(),
                                p.file_name().map(|n| n.to_string_lossy().to_lowercase()),
                            )
                        });
                        ScrollArea::vertical()
                            .min_scrolled_height(300.)
                            .max_height(300.)
                            .show(ui, |ui| {
                                if entries.is_empty() {
                                    ui.label("No images in this folder.");
                                }
                                for path in entries {
                                    let folder = path.is_dir();
                                    let label = format!(
                                        "{} {}",
                                        if folder { "▸" } else { "▧" },
                                        path.file_name().unwrap_or_default().to_string_lossy()
                                    );
                                    let response = ui.selectable_label(
                                        self.image_selection.as_ref() == Some(&path),
                                        label,
                                    );
                                    if response.clicked() {
                                        if folder {
                                            self.image_browser = Some(path.clone());
                                            self.image_selection = None;
                                        } else {
                                            self.image_selection = Some(path.clone());
                                        }
                                    }
                                    if !folder && response.double_clicked() {
                                        chosen = Some(path);
                                    }
                                }
                            });
                    }
                    Err(e) => {
                        ui.label(format!("Could not open folder: {e}"));
                    }
                }
                ui.separator();
                ui.label("PNG, JPEG or GIF · fitted to 160 × 128 pixels");
                ui.horizontal(|ui| {
                    if ui
                        .add_enabled(self.image_selection.is_some(), Button::new("Open image"))
                        .clicked()
                    {
                        chosen = self.image_selection.clone();
                    }
                    if ui.button("Cancel").clicked() {
                        self.image_browser = None;
                    }
                });
            });
        if !open {
            self.image_browser = None;
        }
        if let Some(path) = chosen {
            self.import_selected_image(ctx, path);
        }
    }

    fn import_selected_image(&mut self, ctx: &Context, path: PathBuf) {
        match self.file_op(ctx, "Import image", &path.to_string_lossy()) {
            Ok(msg) => {
                self.image_browser = None;
                self.image_selection = None;
                self.notify(msg);
            }
            Err(e) => self.notify(format!("Could not import image: {e}")),
        }
    }
}
