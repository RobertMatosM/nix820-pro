use super::*;

impl Studio {
    pub(super) fn screen(&mut self, ui: &mut Ui, c: &Canvas, ctx: &Context) {
        c.text(
            pos2(70., 77.),
            "LED screen preview:",
            16.,
            FG,
            Align2::LEFT_CENTER,
        );
        c.text(
            pos2(320., 77.),
            "Image preview:",
            16.,
            FG,
            Align2::LEFT_CENTER,
        );
        c.checkbox(ui, pos2(467., 68.), "Grid", &mut self.grid);
        for (i, l) in ["Editing", "Preview"].iter().enumerate() {
            let rect = Rect::from_min_size(pos2(562. + 95. * i as f32, 64.), vec2(87., 27.));
            if c.hit(ui, rect, l).clicked() {
                self.preview = i == 1;
                self.dirty_texture = true;
                if self.preview {
                    self.started = Instant::now();
                }
            }
            c.text(
                pos2(575. + 95. * i as f32, 77.),
                *l,
                16.,
                if self.preview == (i == 1) { ACCENT } else { FG },
                Align2::LEFT_CENTER,
            );
        }
        c.text(
            pos2(1030., 77.),
            "Color settings:",
            16.,
            FG,
            Align2::LEFT_CENTER,
        );
        self.delays[self.frame_index] = self.delay;
        let displayed = if self.preview {
            let total: u128 = self.delays.iter().map(|d| *d as u128).sum();
            let mut time = self.started.elapsed().as_millis() % total;
            self.delays
                .iter()
                .position(|d| {
                    if time < *d as u128 {
                        true
                    } else {
                        time -= *d as u128;
                        false
                    }
                })
                .unwrap_or(0)
        } else {
            self.frame_index
        };
        if self.dirty_texture {
            self.thumb_textures.clear();
        }
        if self.dirty_texture || self.preview {
            let mut bytes = self.pixels[displayed].clone();
            for a in bytes.chunks_mut(4) {
                a[3] = 255;
            }
            let im = ColorImage::from_rgba_unmultiplied([160, 128], &bytes);
            if let Some(t) = self.texture.as_mut() {
                t.set(im, TextureOptions::NEAREST)
            } else {
                self.texture = Some(ctx.load_texture("screen", im, TextureOptions::NEAREST))
            }
            self.dirty_texture = false;
        }
        let preset_area = Rect::from_min_size(pos2(80., 104.), vec2(210., 567.));
        scroll_track(
            ui,
            c,
            preset_area,
            self.presets.len() as f32 * 148.,
            &mut self.preset_scroll,
            "presets",
            false,
        );
        let pc = Canvas {
            p: c.p.with_clip_rect(c.rect(preset_area)),
            s: c.s,
            o: c.o,
        };
        let mut choose = None;
        for i in 0..self.presets.len() {
            let r = Rect::from_min_size(
                pos2(90., 112. + i as f32 * 148. - self.preset_scroll),
                vec2(174., 130.),
            );
            if !r.intersects(preset_area) {
                continue;
            }
            pc.fill(r, 12., Color32::from_gray(24));
            pc.outline(
                r,
                12.,
                if i == self.preset_index {
                    ACCENT
                } else {
                    MUTED
                },
                1.,
            );
            let t = if i == self.preset_index {
                self.texture.clone()
            } else {
                Some(
                    self.preset_textures
                        .entry(i)
                        .or_insert_with(|| {
                            let preset = &self.presets[i];
                            let mut bytes = preset.frames[preset.selected].clone();
                            for p in bytes.chunks_mut(4) {
                                p[3] = 255;
                            }
                            ctx.load_texture(
                                format!("preset{i}"),
                                ColorImage::from_rgba_unmultiplied([160, 128], &bytes),
                                TextureOptions::NEAREST,
                            )
                        })
                        .clone(),
                )
            };
            if let Some(t) = t {
                pc.p.image(
                    t.id(),
                    pc.rect(Rect::from_min_size(r.min + vec2(37., 6.), vec2(100., 100.))),
                    Rect::from_min_max(Pos2::ZERO, pos2(1., 1.)),
                    Color32::WHITE,
                );
            }
            pc.fill(
                Rect::from_min_size(r.min + vec2(0., 108.), vec2(174., 22.)),
                3.,
                if i == self.preset_index {
                    ACCENT
                } else {
                    Color32::from_gray(70)
                },
            );
            let count = if i == self.preset_index {
                self.pixels.len()
            } else {
                self.presets[i].frames.len()
            };
            pc.text(
                r.min + vec2(87., 119.),
                format!("Preset {} : {}", i + 1, count),
                14.,
                if i == self.preset_index { INK } else { FG },
                Align2::CENTER_CENTER,
            );
            if c.hit(ui, r.intersect(preset_area), ("preset", i)).clicked() {
                choose = Some(i);
            }
        }
        if let Some(i) = choose {
            self.select_preset(i);
        }
        let canvas = Rect::from_min_size(pos2(310., 92.), vec2(694., 433.));
        c.fill(canvas, 0., Color32::BLACK);
        if let Some(t) = &self.texture {
            c.p.image(
                t.id(),
                c.rect(canvas),
                Rect::from_min_max(Pos2::ZERO, pos2(1., 1.)),
                Color32::WHITE,
            );
        }
        if self.grid {
            for i in 1..40 {
                let x = canvas.left() + canvas.width() * i as f32 / 40.;
                c.line(pos2(x, 92.), pos2(x, 525.), Color32::from_gray(45), 0.5);
            }
            for i in 1..32 {
                let y = 92. + 433. * i as f32 / 32.;
                c.line(pos2(310., y), pos2(1004., y), Color32::from_gray(45), 0.5);
            }
        }
        let hit = c.hit(ui, canvas, "paint");
        if !self.preview && (hit.drag_started() || hit.clicked()) {
            self.undo.push(self.pixels[self.frame_index].clone());
            if self.undo.len() > 20 {
                self.undo.remove(0);
            }
        }
        if !self.preview
            && (hit.dragged() || hit.clicked())
            && let Some(pos) = hit.interact_pointer_pos()
        {
            let local = (pos - c.point(canvas.min)) / c.s;
            let x = (local.x / canvas.width() * 160.).clamp(0., 159.) as i32;
            let y = (local.y / canvas.height() * 128.).clamp(0., 127.) as i32;
            for dy in -1..=1 {
                for dx in -1..=1 {
                    let xx = (x + dx).clamp(0, 159) as usize;
                    let yy = (y + dy).clamp(0, 127) as usize;
                    let idx = (yy * 160 + xx) * 4;
                    let col = if self.eraser {
                        [0, 0, 0]
                    } else {
                        self.draw_color
                    };
                    self.pixels[self.frame_index][idx..idx + 4]
                        .copy_from_slice(&[col[0], col[1], col[2], 255]);
                }
            }
            self.dirty_texture = true;
        }
        let strip = Rect::from_min_size(pos2(310., 538.), vec2(694., 133.));
        // Stacked frame controls follow the selected frame; all other frames keep their order.
        let content = self.pixels.len() as f32 * 205. + 72.;
        scroll_track(
            ui,
            c,
            strip,
            content,
            &mut self.thumb_offset,
            "frames",
            true,
        );
        let visible = Rect::from_min_size(strip.min, vec2(strip.width(), 114.));
        let tc = Canvas {
            p: c.p.with_clip_rect(c.rect(visible)),
            s: c.s,
            o: c.o,
        };
        let mut selected = None;
        let mut insert = false;
        let mut remove = false;
        for i in 0..self.pixels.len() {
            let x = 315. + i as f32 * 205. + if i > self.frame_index { 72. } else { 0. }
                - self.thumb_offset;
            let r = Rect::from_min_size(pos2(x, 542.), vec2(184., 107.));
            if i == self.frame_index {
                for (y, label, id, hint, clicked) in [
                    (
                        548.,
                        "+",
                        "insert-frame",
                        "Add a blank frame after the selected frame",
                        &mut insert,
                    ),
                    (
                        599.,
                        "−",
                        "remove-frame",
                        "Delete selected frame",
                        &mut remove,
                    ),
                ] {
                    let button = Rect::from_min_size(pos2(r.right() + 10., y), vec2(52., 44.));
                    if button.intersects(visible) {
                        let response = c.hit(ui, button.intersect(visible), id).on_hover_text(hint);
                        tc.fill(
                            button,
                            6.,
                            Color32::from_gray(if response.hovered() { 60 } else { 36 }),
                        );
                        tc.text(button.center(), label, 32., MUTED, Align2::CENTER_CENTER);
                        *clicked = response.clicked();
                    }
                }
            }
            if !r.intersects(visible) {
                continue;
            }
            if i == self.frame_index {
                tc.outline(r, 4., ACCENT, 1.);
            }
            tc.fill(r.shrink(4.), 0., Color32::from_gray(23));
            let t = self.thumb_textures.entry(i).or_insert_with(|| {
                let mut bytes = self.pixels[i].clone();
                for p in bytes.chunks_mut(4) {
                    p[3] = 255;
                }
                ctx.load_texture(
                    format!("thumb{i}"),
                    ColorImage::from_rgba_unmultiplied([160, 128], &bytes),
                    TextureOptions::NEAREST,
                )
            });
            tc.p.image(
                t.id(),
                tc.rect(r.shrink2(vec2(40., 4.))),
                Rect::from_min_max(Pos2::ZERO, pos2(1., 1.)),
                Color32::WHITE,
            );
            tc.fill(
                Rect::from_min_size(pos2(r.right() - 42., 628.), vec2(42., 21.)),
                3.,
                ACCENT,
            );
            tc.text(
                pos2(r.right() - 21., 638.),
                self.delays[i],
                13.,
                Color32::BLACK,
                Align2::CENTER_CENTER,
            );
            if c.hit(ui, r.intersect(visible), (i, "frame")).clicked() {
                selected = Some(i);
            }
        }
        if let Some(i) = selected {
            self.frame_index = i;
            self.delay = self.delays[i];
            self.undo.clear();
            self.preview = false;
            self.dirty_texture = true;
        }
        if insert {
            self.insert_frame(false);
        }
        if remove {
            if self.pixels.len() > 1 {
                self.pixels.remove(self.frame_index);
                self.delays.remove(self.frame_index);
                self.frame_index = self.frame_index.min(self.pixels.len() - 1);
                self.delay = self.delays[self.frame_index];
                self.undo.clear();
                self.preview = false;
                self.dirty_texture = true;
                let left = self.frame_index as f32 * 205.;
                self.thumb_offset = self.thumb_offset.clamp((left + 240. - 684.).max(0.), left);
            } else {
                self.notify("Keep at least one frame in each preset.");
            }
        }
        let sw = Rect::from_min_size(pos2(1050., 135.), vec2(195., 180.));
        let hsv = egui::ecolor::Hsva::from(Color32::from_rgb(
            self.draw_color[0],
            self.draw_color[1],
            self.draw_color[2],
        ));
        for y in 0..60 {
            for x in 0..65 {
                let col = Color32::from(egui::ecolor::Hsva::new(
                    hsv.h,
                    x as f32 / 64.,
                    1. - y as f32 / 85.,
                    1.,
                ));
                c.fill(
                    Rect::from_min_size(
                        pos2(sw.min.x + x as f32 * 3., sw.min.y + y as f32 * 3.),
                        vec2(3., 3.),
                    ),
                    0.,
                    col,
                );
            }
        }
        let pick = c.hit(ui, sw, "screen-color");
        if (pick.dragged() || pick.clicked())
            && let Some(p) = pick.interact_pointer_pos()
        {
            let d = (p - c.point(sw.min)) / c.s;
            let col = Color32::from(egui::ecolor::Hsva::new(
                hsv.h,
                (d.x / 195.).clamp(0., 1.),
                1. - d.y / 255.,
                1.,
            ));
            self.draw_color = [col.r(), col.g(), col.b()];
        }
        for i in 0..215 {
            let col = Color32::from(egui::ecolor::Hsva::new(i as f32 / 215., 1., 1., 1.));
            c.fill(
                Rect::from_min_size(pos2(1040. + i as f32, 346.), vec2(1., 20.)),
                0.,
                col,
            );
        }
        let hue = c.hit(
            ui,
            Rect::from_min_size(pos2(1040., 346.), vec2(215., 20.)),
            "hue",
        );
        if (hue.dragged() || hue.clicked())
            && let Some(p) = hue.interact_pointer_pos()
        {
            let h = ((p.x - c.pos(1040., 0.).x) / c.s / 215.).clamp(0., 1.);
            let col = Color32::from(egui::ecolor::Hsva::new(h, 1., 1., 1.));
            self.draw_color = [col.r(), col.g(), col.b()];
        }
        for i in 0..3 {
            c.text(
                pos2(1040. + i as f32 * 73., 396.),
                ["R", "G", "B"][i],
                16.,
                FG,
                Align2::LEFT_CENTER,
            );
            ui.put(
                c.rect(Rect::from_min_size(
                    pos2(1056. + i as f32 * 73., 383.),
                    vec2(51., 25.),
                )),
                DragValue::new(&mut self.draw_color[i]).range(0..=255),
            );
        }
        for (i, col) in [
            [85, 36, 120],
            [221, 180, 60],
            [0, 213, 0],
            [0, 205, 211],
            [230, 10, 0],
            [213, 47, 180],
        ]
        .iter()
        .enumerate()
        {
            let r = Rect::from_min_size(pos2(1040. + i as f32 * 37., 427.), vec2(27., 27.));
            c.fill(r, 0., Color32::from_rgb(col[0], col[1], col[2]));
            c.outline(r, 0., Color32::from_gray(100), 2.);
            if c.hit(ui, r, (i, "tft-swatch")).clicked() {
                self.draw_color = *col;
            }
        }
        for (i, label) in ["Time Sync", "Save PNG", "Export GIF", "Upload to keyboard"]
            .iter()
            .enumerate()
        {
            if c.button_enabled(
                ui,
                Rect::from_min_size(pos2(1040., 582. + i as f32 * 35.), vec2(215., 30.)),
                label,
                16.,
                matches!(i, 1 | 2) || self.wired_actions_available(),
            )
            .on_disabled_hover_text(
                "Display transfers require wired USB, or another transfer is running.",
            )
            .clicked()
            {
                match i {
                    0 => {
                        self.start_time_sync();
                    }
                    1 => self.open_file("Export PNG"),
                    2 => self.open_file("Export GIF"),
                    _ => self.start_tft(),
                }
            }
        }
        for (i, label) in ["New", "Import", "Delete"].iter().enumerate() {
            if c.button(
                ui,
                Rect::from_min_size(pos2(74. + i as f32 * 68., 687.), vec2(68., 30.)),
                label,
                false,
                16.,
            )
            .clicked()
            {
                match i {
                    0 => self.new_preset(),
                    1 => self.open_file("Import image"),
                    _ => {
                        if self.presets.len() > 1 {
                            self.presets.remove(self.preset_index);
                            self.preset_index = self.preset_index.min(self.presets.len() - 1);
                            let p = &self.presets[self.preset_index];
                            self.pixels = p.frames.clone();
                            self.delays = p.delays.clone();
                            self.frame_index = p.selected;
                            self.delay = self.delays[self.frame_index];
                            self.reset_screen_view();
                        } else {
                            self.notify("Keep at least one preset. Use − to delete a frame.");
                        }
                    }
                }
            }
        }
        c.text(pos2(320., 701.), "Text:", 16., FG, Align2::LEFT_CENTER);
        let text = ui.put(
            c.rect(Rect::from_min_size(pos2(369., 687.), vec2(280., 27.))),
            TextEdit::singleline(&mut self.screen_text).hint_text("Type a caption and press Enter"),
        );
        if text.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter)) {
            self.undo.push(self.pixels[self.frame_index].clone());
            draw_caption(
                &mut self.pixels[self.frame_index],
                &self.screen_text,
                self.draw_color,
            );
            self.screen_text.clear();
            self.dirty_texture = true;
        }

        c.text(pos2(675., 701.), "Delay", 13., FG, Align2::LEFT_CENTER);
        ui.put(
            c.rect(Rect::from_min_size(pos2(719., 687.), vec2(60., 27.))),
            DragValue::new(&mut self.delay)
                .range(10..=5000)
                .speed(2)
                .suffix(" ms"),
        );
        if c.button(
            ui,
            Rect::from_min_size(pos2(800., 686.), vec2(40., 28.)),
            "Copy",
            false,
            12.,
        )
        .clicked()
        {
            self.insert_frame(true);
        }
        if c.button(
            ui,
            Rect::from_min_size(pos2(844., 686.), vec2(44., 28.)),
            "Undo",
            false,
            12.,
        )
        .clicked()
            && let Some(old) = self.undo.pop()
        {
            self.pixels[self.frame_index] = old;
            self.dirty_texture = true;
        }
        if c.button(
            ui,
            Rect::from_min_size(pos2(895., 686.), vec2(60., 28.)),
            "Eraser",
            self.eraser,
            12.,
        )
        .clicked()
        {
            self.eraser = !self.eraser;
        }
    }
}
/// Original 5x7 pixel lettering for captions in exported screen frames.
pub(super) fn draw_caption(pixels: &mut [u8], text: &str, color: [u8; 3]) {
    let letters = "ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789!?- .";
    let rows = [
        "01110100011000111111100011000110001",
        "11110100011000111110100011000111110",
        "01111100001000010000100001000001111",
        "11110100011000110001100011000111110",
        "11111100001000011110100001000011111",
        "11111100001000011110100001000010000",
        "01111100001000010111100011000101111",
        "10001100011000111111100011000110001",
        "11111001000010000100001000010011111",
        "00111000100001000010100101001001100",
        "10001100101010011000101001001010001",
        "10000100001000010000100001000011111",
        "10001110111010110101100011000110001",
        "10001110011010110011100011000110001",
        "01110100011000110001100011000101110",
        "11110100011000111110100001000010000",
        "01110100011000110001101011001001101",
        "11110100011000111110101001001010001",
        "01111100001000001110000010000111110",
        "11111001000010000100001000010000100",
        "10001100011000110001100011000101110",
        "10001100011000110001100010101000100",
        "10001100011000110101101011101110001",
        "10001100010101000100010101000110001",
        "10001100010101000100001000010000100",
        "11111000010001000100010001000011111",
        "01110100011001110101110011000101110",
        "00100011000010000100001000010001110",
        "01110100010000100010001000100011111",
        "11110000010000101110000010000111110",
        "00010001100101010010111110001000010",
        "11111100001000011110000010000111110",
        "01110100001000011110100011000101110",
        "11111000010001000100010000100001000",
        "01110100011000101110100011000101110",
        "01110100011000101111000010000101110",
        "00100001000010000100001000000000100",
        "01110100010000100010001000000000100",
        "00000000000000011111000000000000000",
        "00000000000000000000000000000000000",
        "00000000000000000000000000000000100",
    ];
    let chars: Vec<char> = text.to_uppercase().chars().take(25).collect();
    let start = 80usize.saturating_sub(chars.len() * 3);
    for (i, ch) in chars.iter().enumerate() {
        let index = letters.chars().position(|c| c == *ch).unwrap_or(39);
        for (j, b) in rows[index].bytes().enumerate() {
            if b == b'1' {
                let x = start + i * 6 + j % 5;
                let y = 60 + j / 5;
                if x < 160 && y < 128 {
                    let k = (y * 160 + x) * 4;
                    pixels[k..k + 4].copy_from_slice(&[color[0], color[1], color[2], 255]);
                }
            }
        }
    }
}

/// Pixel-based scrolling preserves trackpad deltas and clamps both wheel and scrollbar drag.
fn scroll_track(
    ui: &mut Ui,
    c: &Canvas,
    area: Rect,
    content: f32,
    offset: &mut f32,
    id: &str,
    horizontal: bool,
) {
    let span = if horizontal {
        area.width()
    } else {
        area.height()
    };
    let max = (content - span).max(0.);
    if ui.rect_contains_pointer(c.rect(area)) {
        let delta = ui.input_mut(|i| {
            let d = i.smooth_scroll_delta;
            i.smooth_scroll_delta = Vec2::ZERO;
            if horizontal { d.x + d.y } else { d.y }
        });
        *offset = (*offset - delta / c.s).clamp(0., max);
        if delta != 0. {
            ui.ctx()
                .request_repaint_after(std::time::Duration::from_millis(16));
        }
    }
    *offset = offset.clamp(0., max);
    if max <= 0. {
        return;
    }
    let track = if horizontal {
        Rect::from_min_size(
            pos2(area.left() + 5., area.bottom() - 10.),
            vec2(area.width() - 10., 6.),
        )
    } else {
        Rect::from_min_size(pos2(area.right() - 7., area.top()), vec2(5., area.height()))
    };
    let len = if horizontal {
        track.width()
    } else {
        track.height()
    };
    let thumb = (len * span / content).max(24.);
    let response = c.hit(ui, track.expand(4.), ("scrollbar", id));
    let grab_id = Id::unique(("scrollbar-grab", id));
    if response.drag_started()
        && let Some(p) = ui.input(|i| i.pointer.press_origin())
    {
        let local = (p - c.point(track.min)) / c.s;
        let position = if horizontal { local.x } else { local.y };
        let start = (*offset / max) * (len - thumb);
        let grab = if (start..=start + thumb).contains(&position) {
            position - start
        } else {
            thumb / 2.
        };
        ui.ctx().data_mut(|d| d.insert_temp(grab_id, grab));
    }
    if (response.dragged() || response.clicked())
        && let Some(p) = response.interact_pointer_pos()
    {
        let local = (p - c.point(track.min)) / c.s;
        let position = if horizontal { local.x } else { local.y };
        let grab = if response.dragged() {
            ui.ctx()
                .data(|d| d.get_temp::<f32>(grab_id))
                .unwrap_or(thumb / 2.)
        } else {
            thumb / 2.
        };
        *offset = ((position - grab) / (len - thumb) * max).clamp(0., max);
    }
    c.fill(track, 3., Color32::from_gray(45));
    let start = (*offset / max) * (len - thumb);
    let r = if horizontal {
        Rect::from_min_size(track.min + vec2(start, 0.), vec2(thumb, 6.))
    } else {
        Rect::from_min_size(track.min + vec2(0., start), vec2(5., thumb))
    };
    c.fill(r, 3., Color32::from_rgb(190, 177, 0));
}

#[cfg(test)]
mod tests {
    use super::*;
    fn step(app: &mut Studio, ctx: &Context, events: Vec<Event>) {
        let input = RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(1288., 724.))),
            events,
            ..Default::default()
        };
        let mut output = ctx.run_ui(input, |ui| {
            let c = Canvas {
                p: ui.painter().clone(),
                s: 1.,
                o: Pos2::ZERO,
            };
            app.screen(ui, &c, ctx);
        });
        output.textures_delta.clear();
    }
    fn click(app: &mut Studio, ctx: &Context, pos: Pos2) {
        step(app, ctx, vec![Event::PointerMoved(pos)]);
        step(
            app,
            ctx,
            vec![Event::PointerButton {
                pos,
                button: PointerButton::Primary,
                pressed: true,
                modifiers: Modifiers::NONE,
            }],
        );
        step(
            app,
            ctx,
            vec![Event::PointerButton {
                pos,
                button: PointerButton::Primary,
                pressed: false,
                modifiers: Modifiers::NONE,
            }],
        );
    }
    #[test]
    fn preset_and_frame_buttons_keep_independent_content_and_undo() {
        let ctx = Context::default();
        let mut app = Studio::create(&ctx, Some(PathBuf::from("unused.png")), 4);
        app.pixels[0][0] = 123;
        click(&mut app, &ctx, pos2(535., 570.));
        assert_eq!(app.pixels.len(), 5);
        assert_eq!(app.frame_index, 1);
        assert_eq!(app.pixels[1][0], 0);
        assert_eq!(app.pixels[0][0], 123);
        click(&mut app, &ctx, pos2(740., 621.));
        assert_eq!(app.pixels.len(), 4);
        assert_eq!(app.delays.len(), 4);
        assert_eq!(app.pixels[0][0], 123);
        click(&mut app, &ctx, pos2(740., 570.));
        assert_eq!(app.pixels.len(), 5);
        click(&mut app, &ctx, pos2(105., 702.));
        assert_eq!(app.presets.len(), 2);
        assert_eq!(app.pixels.len(), 1);
        click(&mut app, &ctx, pos2(535., 621.));
        assert_eq!(app.pixels.len(), 1);
        app.toast = None;
        app.pixels[0][0] = 77;
        app.undo.push(vec![1; 160 * 128 * 4]);
        click(&mut app, &ctx, pos2(170., 170.));
        assert_eq!(app.pixels.len(), 5);
        assert_eq!(app.pixels[0][0], 123);
        assert!(app.undo.is_empty());
        click(&mut app, &ctx, pos2(170., 310.));
        assert_eq!(app.pixels[0][0], 77);
        let stored = serde_json::to_vec(&app.presets).unwrap();
        assert!(
            serde_json::from_slice::<Vec<model::ScreenPreset>>(&stored)
                .unwrap()
                .iter()
                .all(model::ScreenPreset::valid)
        );
    }
    #[test]
    fn frame_scroll_uses_fractional_deltas_and_scrollbar_reaches_both_ends() {
        let ctx = Context::default();
        let mut app = Studio::create(&ctx, Some(PathBuf::from("unused.png")), 4);
        for _ in 0..12 {
            app.insert_frame(false);
        }
        app.thumb_offset = 0.;
        step(&mut app, &ctx, vec![Event::PointerMoved(pos2(600., 600.))]);
        step(
            &mut app,
            &ctx,
            vec![Event::MouseWheel {
                unit: egui::MouseWheelUnit::Point,
                phase: TouchPhase::Move,
                source: MouseWheelSource::Unknown,
                delta: vec2(0., -0.5),
                modifiers: Modifiers::NONE,
            }],
        );
        assert!(
            app.thumb_offset > 0. && app.thumb_offset < 1.,
            "{}",
            app.thumb_offset
        );
        click(&mut app, &ctx, pos2(997., 664.));
        assert_eq!(
            app.thumb_offset,
            app.pixels.len() as f32 * 205. + 72. - 694.
        );
        click(&mut app, &ctx, pos2(316., 664.));
        assert_eq!(app.thumb_offset, 0.);
    }
}
