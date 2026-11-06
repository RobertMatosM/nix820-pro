use super::*;

impl Studio {
    pub(super) fn keyboard(&mut self, ui: &mut Ui, c: &Canvas) {
        let case = Rect::from_min_size(pos2(466., 153.), vec2(648., 269.));
        c.fill(case, 10., SURFACE);
        c.outline(case, 10., BORDER, 1.);
        let mut keys: Vec<(String, f32, f32, f32)> = vec![];
        let row0 = [
            "Esc", "F1", "F2", "F3", "F4", "F5", "F6", "F7", "F8", "F9", "F10", "F11", "F12", "Del",
        ];
        for (i, k) in row0.iter().enumerate() {
            keys.push((
                k.to_string(),
                480. + i as f32 * 40. + if i > 0 { 8. } else { 0. },
                167.,
                36.,
            ));
        }
        let rows: Vec<Vec<(&str, f32)>> = vec![
            vec![
                ("` ~", 1.),
                ("1 !", 1.),
                ("2 @", 1.),
                ("3 #", 1.),
                ("4 $", 1.),
                ("5 %", 1.),
                ("6 ^", 1.),
                ("7 &", 1.),
                ("8 *", 1.),
                ("9 (", 1.),
                ("0 )", 1.),
                ("- _", 1.),
                ("= +", 1.),
                ("Back", 1.9),
            ],
            vec![
                ("Tab", 1.5),
                ("Q", 1.),
                ("W", 1.),
                ("E", 1.),
                ("R", 1.),
                ("T", 1.),
                ("Y", 1.),
                ("U", 1.),
                ("I", 1.),
                ("O", 1.),
                ("P", 1.),
                ("[ {", 1.),
                ("] }", 1.),
                ("\\ |", 1.4),
            ],
            vec![
                ("Caps", 1.75),
                ("A", 1.),
                ("S", 1.),
                ("D", 1.),
                ("F", 1.),
                ("G", 1.),
                ("H", 1.),
                ("J", 1.),
                ("K", 1.),
                ("L", 1.),
                ("; :", 1.),
                ("' \"", 1.),
                ("Enter", 2.15),
            ],
            vec![
                ("Shift", 2.25),
                ("Z", 1.),
                ("X", 1.),
                ("C", 1.),
                ("V", 1.),
                ("B", 1.),
                ("N", 1.),
                ("M", 1.),
                (", <", 1.),
                (". >", 1.),
                ("/ ?", 1.),
                ("Shift R", 1.65),
                ("↑", 1.),
            ],
            vec![
                ("Ctrl", 1.25),
                ("Win", 1.25),
                ("Alt", 1.25),
                ("Space", 6.25),
                ("Alt R", 1.),
                ("Fn", 1.),
                ("Ctrl R", 1.),
                ("←", 1.),
                ("↓", 1.),
                ("→", 1.),
            ],
        ];
        for (r, row) in rows.iter().enumerate() {
            let mut x = 480.;
            for (k, w) in row {
                let width = w * 38.;
                keys.push((k.to_string(), x, 215. + r as f32 * 38., width - 3.));
                x += width;
            }
        }
        for (i, k) in ["Home", "PgUp", "PgDn"].iter().enumerate() {
            keys.push((k.to_string(), 1064., 215. + i as f32 * 38., 35.));
        }
        let rgb = if self.p().mode == 19 {
            Color32::from_gray(85)
        } else {
            self.color().gamma_multiply(self.p().brightness / 5.)
        };
        let label_col = if self.page == 3 { rgb } else { FG };
        for (name, x, y, w) in keys {
            let r = Rect::from_min_size(pos2(x, y), vec2(w, 36.));
            let is_sel = self.key.as_ref() == Some(&name) && self.page == 1;
            let top = r.shrink(1.5);
            let dark = [
                "Tab", "Caps", "Shift", "Shift R", "Ctrl", "Win", "Alt", "Alt R", "Fn", "Ctrl R",
                "F5", "F6", "F7", "F8",
            ]
            .contains(&name.as_str());
            let yellow = ["Esc", "Enter", "Space"].contains(&name.as_str());
            let fill = if self.page == 3 {
                SURFACE
            } else if is_sel {
                SELECTION
            } else if yellow {
                ACCENT
            } else if dark {
                KEY_DARK
            } else {
                KEY_LIGHT
            };
            let ink = if self.page == 3 {
                label_col
            } else if is_sel || dark {
                FG
            } else {
                INK
            };
            c.fill(top, 4., fill);
            c.outline(
                top,
                4.,
                if is_sel { ACCENT } else { BORDER },
                if is_sel { 3. } else { 1. },
            );
            let label_col = if self.page == 3 && self.screenshot.is_none() && self.p().mode != 19 {
                let phase = self.started.elapsed().as_secs_f32() * self.p().speed * 0.6;
                let rainbow = self.p().colorful || [5, 7].contains(&self.p().mode);
                let base = if rainbow {
                    Color32::from(egui::ecolor::Hsva::new(
                        (x / 648. + phase * 0.15).fract(),
                        1.,
                        1.,
                        1.,
                    ))
                    .gamma_multiply(self.p().brightness / 5.)
                } else {
                    label_col
                };
                let pulse = if [0, 5, 7].contains(&self.p().mode) {
                    1.
                } else {
                    0.35 + 0.65 * ((phase - x / 200. + y / 250.).sin() * 0.5 + 0.5)
                };
                base.gamma_multiply(pulse)
            } else {
                label_col
            };
            let key_color = if self.page == 3 { label_col } else { ink };
            let assignment = (self.page == 1)
                .then(|| self.p().key_label(&name, self.layer))
                .flatten();
            let label = assignment
                .as_deref()
                .map(|label| {
                    crate::remap::MEDIA_CHOICES
                        .iter()
                        .find(|(key, _)| key.label() == label)
                        .map(|(_, icon)| *icon)
                        .unwrap_or(label)
                })
                .unwrap_or(match name.as_str() {
                    "Shift R" => "Shift",
                    "Alt R" => "Alt",
                    "Ctrl R" => "Ctrl",
                    "Back" => "⟵",
                    "Space" => "━━━━",
                    _ => &name,
                });
            c.text(
                top.center(),
                label,
                if label.len() > 8 { 7.5 } else { 10. },
                key_color,
                Align2::CENTER_CENTER,
            );
            if self.page == 1 {
                let allowed = crate::remap::Layer::from_fn(self.layer).allows(&name);
                if !allowed {
                    c.fill(r, 2., Color32::from_black_alpha(55));
                }
                let response = c.hit(ui, r, (&name, "key"));
                if allowed {
                    if response.clicked() {
                        self.key = Some(name.clone());
                    }
                } else {
                    response
                        .on_hover_text("Reserved source key in this layer; cannot be remapped.");
                }
            }
        }
        c.outline(
            Rect::from_min_size(pos2(1065., 332.), vec2(35., 35.)),
            4.,
            MUTED,
            1.,
        );
        c.text(pos2(1082., 349.), "LCD", 8., MUTED, Align2::CENTER_CENTER);
        c.p.circle_stroke(c.pos(1082., 186.), 15. * c.s, Stroke::new(2. * c.s, MUTED));
    }
}
