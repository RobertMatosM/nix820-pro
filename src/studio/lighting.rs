use super::*;

impl Studio {
    pub(super) fn lighting(&mut self, ui: &mut Ui, c: &Canvas) {
        for (i, m) in MODES.iter().enumerate() {
            let y = 77. + i as f32 * 32.;
            let r = Rect::from_min_size(pos2(66., y - 16.), vec2(239., 32.));
            if self.p().mode == i {
                c.fill(r, 5., SELECTION);
            }
            c.p.circle_stroke(
                c.pos(89., y),
                10. * c.s,
                Stroke::new(2. * c.s, Color32::from_gray(213)),
            );
            if self.p().mode == i {
                c.p.circle_filled(c.pos(89., y), 10. * c.s, FG);
            }
            c.text(pos2(117., y), m, 16., FG, Align2::LEFT_CENTER);
            if c.hit(ui, r, (&m, "mode")).clicked() {
                self.pending_mode = Some(ModeChange::Lighting(i));
            }
        }
        self.apply_button(ui, c, "Apply lighting", 3);
        self.keyboard(ui, c);
        c.text(
            pos2(334., 483.),
            "Lighting Brightness",
            16.,
            FG,
            Align2::LEFT_CENTER,
        );
        let mut v = self.p().brightness;
        if c.slider(
            ui,
            "brightness",
            pos2(346., 507.),
            216.,
            &mut v,
            1.,
            5.,
            ACCENT,
        ) {
            self.pm().brightness = v.round();
            self.save();
        }
        c.text(
            pos2(454., 539.),
            format!("{}", self.p().brightness as i32),
            16.,
            FG,
            Align2::CENTER_CENTER,
        );
        c.text(
            pos2(334., 564.),
            "Lighting Speed",
            16.,
            FG,
            Align2::LEFT_CENTER,
        );
        let mut v = self.p().speed;
        if c.slider(ui, "speed", pos2(346., 586.), 216., &mut v, 1., 5., ACCENT) {
            self.pm().speed = v.round();
            self.save();
        }
        c.text(
            pos2(454., 619.),
            format!("{}", self.p().speed as i32),
            16.,
            FG,
            Align2::CENTER_CENTER,
        );
        c.fill(
            Rect::from_min_size(pos2(725., 467.), vec2(49., 39.)),
            0.,
            Color32::BLACK,
        );
        c.fill(
            Rect::from_min_size(pos2(728., 470.), vec2(43., 33.)),
            0.,
            self.color(),
        );
        c.outline(
            Rect::from_min_size(pos2(725., 467.), vec2(49., 39.)),
            0.,
            FG,
            1.,
        );
        let mut colorful = self.p().colorful;
        if c.checkbox(ui, pos2(811., 478.), "Colorful", &mut colorful) {
            self.pm().colorful = colorful;
            self.save();
        }
        c.text(pos2(724., 534.), "Palette", 16., FG, Align2::LEFT_CENTER);
        let center = pos2(810., 621.);
        let radius = 62.;
        let mut mesh = Mesh::default();
        mesh.colored_vertex(c.point(center), Color32::WHITE);
        for i in 0..=90 {
            let angle = i as f32 / 90. * std::f32::consts::TAU;
            let col = Color32::from(egui::ecolor::Hsva::new(1. - i as f32 / 90., 1., 1., 1.));
            mesh.colored_vertex(
                c.point(center + vec2(angle.cos() * radius, angle.sin() * radius)),
                col,
            );
            if i > 0 {
                mesh.add_triangle(0, i, i + 1);
            }
        }
        c.p.circle_filled(c.point(center), 65. * c.s, BG);
        c.p.add(Shape::mesh(mesh));
        let response = c.hit(
            ui,
            Rect::from_center_size(center, vec2(124., 124.)),
            "wheel",
        );
        if (response.dragged() || response.clicked())
            && let Some(p) = response.interact_pointer_pos()
        {
            let d = (p - c.point(center)) / c.s;
            let h = (-d.y.atan2(d.x)).rem_euclid(std::f32::consts::TAU) / std::f32::consts::TAU;
            let col = Color32::from(egui::ecolor::Hsva::new(
                h,
                (d.length() / radius).min(1.),
                1.,
                1.,
            ));
            self.pm().rgb = [col.r(), col.g(), col.b()];
            self.save();
        }
        let hsv = egui::ecolor::Hsva::from(self.color());
        let angle = -hsv.h * std::f32::consts::TAU;
        c.p.circle_stroke(
            c.point(center + vec2(angle.cos(), angle.sin()) * radius * hsv.s),
            4. * c.s,
            Stroke::new(c.s, Color32::WHITE),
        );
        for (i, l) in ["R", "G", "B"].iter().enumerate() {
            let y = 568. + i as f32 * 30.;
            c.text(pos2(904., y + 4.), *l, 16., FG, Align2::LEFT_CENTER);
            let mut v = self.p().rgb[i] as f32;
            if c.slider(
                ui,
                l,
                pos2(936., y),
                228.,
                &mut v,
                0.,
                255.,
                [Color32::RED, Color32::GREEN, Color32::BLUE][i],
            ) {
                self.pm().rgb[i] = v.round() as u8;
                self.save();
            }
            let r = c.rect(Rect::from_min_size(pos2(1180., y - 9.), vec2(47., 24.)));
            let mut value = self.p().rgb[i] as u32;
            ui.put(r, DragValue::new(&mut value).range(0..=255));
            if value != self.p().rgb[i] as u32 {
                self.pm().rgb[i] = value as u8;
                self.save();
            }
        }
        for (i, col) in [
            Color32::RED,
            Color32::from_rgb(69, 201, 183), // RGB shortcut, independent of the UI accent.
            Color32::YELLOW,
            Color32::GREEN,
            Color32::BLUE,
            Color32::from_rgb(0, 245, 237),
            Color32::from_rgb(240, 0, 240),
            Color32::WHITE,
            Color32::BLACK,
        ]
        .iter()
        .enumerate()
        {
            let r = Rect::from_min_size(pos2(928. + i as f32 * 35., 659.), vec2(26., 26.));
            c.fill(r, 0., *col);
            if c.hit(ui, r, (&i, "swatch")).clicked() {
                self.pm().rgb = [col.r(), col.g(), col.b()];
                self.save();
            }
        }
    }
}
