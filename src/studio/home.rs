use super::*;

impl Studio {
    pub(super) fn home(&mut self, ui: &mut Ui, c: &Canvas, _ctx: &Context) {
        c.text(
            pos2(246., 128.),
            "Your keyboard, configured locally.",
            32.,
            FG,
            Align2::LEFT_CENTER,
        );
        c.text(
            pos2(246., 172.),
            "Independent AK820 Pro configurator for Linux",
            17.,
            MUTED,
            Align2::LEFT_CENTER,
        );
        let card = Rect::from_min_size(pos2(246., 220.), vec2(1110., 330.));
        c.fill(card, 10., Color32::from_rgb(21, 34, 47));
        c.outline(card, 10., Color32::from_rgb(50, 69, 87), 1.);
        // Original schematic illustration; no vendor images or product renders.
        let illustration = Rect::from_min_size(pos2(286., 285.), vec2(545., 180.));
        c.fill(illustration, 8., Color32::from_rgb(13, 23, 33));
        for row in 0..5 {
            for col in 0..14 {
                let r = Rect::from_min_size(
                    pos2(298. + col as f32 * 37., 297. + row as f32 * 31.),
                    vec2(31., 24.),
                );
                c.fill(
                    r,
                    3.,
                    if row == 2 && col == 3 {
                        Color32::from_rgb(29, 91, 89)
                    } else {
                        Color32::from_rgb(34, 52, 69)
                    },
                );
                c.outline(
                    r,
                    3.,
                    if row == 2 && col == 3 {
                        ACCENT
                    } else {
                        Color32::from_rgb(56, 77, 97)
                    },
                    1.,
                );
            }
        }
        c.text(pos2(890., 285.), "AK820 Pro", 28., FG, Align2::LEFT_CENTER);
        c.text(
            pos2(890., 328.),
            if self.keyboards.is_empty() {
                "Connect by wired USB to apply changes."
            } else {
                "Wired keyboard detected."
            },
            14.,
            MUTED,
            Align2::LEFT_CENTER,
        );
        c.text(
            pos2(890., 358.),
            "Edit and save profiles while disconnected.",
            14.,
            MUTED,
            Align2::LEFT_CENTER,
        );
        if c.button(
            ui,
            Rect::from_min_size(pos2(890., 408.), vec2(220., 42.)),
            "Open keymap",
            true,
            16.,
        )
        .clicked()
        {
            self.pending_mode = Some(ModeChange::Page(1));
        }
        for (index, (page, title, description)) in [
            (
                1,
                "Keymap",
                "Base and Fn layers, keyboard and media actions",
            ),
            (3, "Lighting", "Built-in effects, brightness and color"),
            (4, "Display", "Images, GIFs and keyboard clock"),
            (
                8,
                "Device",
                "Response level, sleep, shortcut locks and resets",
            ),
        ]
        .iter()
        .enumerate()
        {
            let x = 246. + index as f32 * 279.;
            let r = Rect::from_min_size(pos2(x, 590.), vec2(264., 110.));
            c.fill(r, 6., Color32::from_rgb(21, 34, 47));
            c.text(pos2(x + 14., 613.), title, 17., FG, Align2::LEFT_CENTER);
            // The full description is available without encoding vendor UI language.
            c.text(
                pos2(x + 14., 647.),
                match page {
                    1 => "Layers and key actions",
                    3 => "Effects and color",
                    4 => "Images and animation",
                    _ => "Settings and resets",
                },
                13.,
                MUTED,
                Align2::LEFT_CENTER,
            );
            if c.hit(ui, r, ("overview-section", page))
                .on_hover_text(*description)
                .clicked()
            {
                self.pending_mode = Some(ModeChange::Page(*page));
            }
        }
    }
}
