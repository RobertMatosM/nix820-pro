use super::*;

impl Studio {
    pub(super) fn settings_page(&mut self, ui: &mut Ui, c: &Canvas) {
        self.keyboard(ui, c);
        if c.button(ui, Rect::from_min_size(pos2(330., 81.), vec2(188., 30.)),
                    "Apply settings", false, 14.)
            .on_hover_text("Sends response time, sleep and the three shortcut locks over wired USB. Changes save locally until applied.")
            .clicked() {
            self.start_settings();
        }
        c.text(
            pos2(535., 96.),
            "Wired USB · experimental",
            12.,
            MUTED,
            Align2::LEFT_CENTER,
        );
        c.text(
            pos2(83., 105.),
            "Device behavior",
            19.,
            FG,
            Align2::LEFT_CENTER,
        );
        c.text(
            pos2(83., 136.),
            "Changes apply over wired USB.",
            12.,
            MUTED,
            Align2::LEFT_CENTER,
        );
        c.text(
            pos2(83., 229.),
            "Key Response Time",
            16.,
            FG,
            Align2::LEFT_CENTER,
        );
        c.text(
            pos2(93., 244.),
            response_info(self.saved.extras.response),
            11.5,
            MUTED,
            Align2::LEFT_TOP,
        );
        if combo(
            ui,
            c,
            "response",
            pos2(83., 423.),
            200.,
            &["1 Level", "2 Level", "3 Level", "4 Level", "5 Level"],
            &mut self.saved.extras.response,
        ) {
            self.save();
        }
        c.text(pos2(83., 489.), "Sleep Time", 16., FG, Align2::LEFT_CENTER);
        if combo(
            ui,
            c,
            "sleep",
            pos2(83., 503.),
            200.,
            &["No sleep", "1min", "5min", "30min"],
            &mut self.saved.extras.sleep,
        ) {
            self.save();
        }
        for i in 0..3 {
            let mut v = match i {
                0 => self.saved.extras.disable_win,
                1 => self.saved.extras.disable_alt_f4,
                _ => self.saved.extras.disable_alt_tab,
            };
            if c.checkbox(
                ui,
                pos2(370., 499. + 40. * i as f32),
                [
                    "Disable Windows Key",
                    "Disable ALT + F4",
                    "Disable ALT + TAB",
                ][i],
                &mut v,
            ) {
                match i {
                    0 => self.saved.extras.disable_win = v,
                    1 => self.saved.extras.disable_alt_f4 = v,
                    _ => self.saved.extras.disable_alt_tab = v,
                };
                self.save();
            }
        }
        c.text(
            pos2(370., 640.),
            "Changes save locally until you click Apply settings.",
            13.,
            MUTED,
            Align2::LEFT_CENTER,
        );
        for (i, kind) in [
            crate::protocol::ResetKind::Keyboard,
            crate::protocol::ResetKind::Factory,
        ]
        .iter()
        .enumerate()
        {
            if c.button(
                ui,
                Rect::from_min_size(pos2(83., 603. + 50. * i as f32), vec2(200., 30.)),
                kind.label(),
                false,
                16.,
            )
            .on_hover_text(kind.description())
            .clicked()
            {
                self.pending_reset = Some(*kind);
            }
        }
    }
}

fn combo(
    ui: &mut Ui,
    c: &Canvas,
    id: &str,
    pos: Pos2,
    width: f32,
    options: &[&str],
    selected: &mut usize,
) -> bool {
    *selected = (*selected).min(options.len() - 1);
    let old = *selected;
    ui.scope_builder(
        UiBuilder::new().max_rect(c.rect(Rect::from_min_size(pos, vec2(width, 30.)))),
        |ui| {
            ui.style_mut()
                .text_styles
                .insert(TextStyle::Button, FontId::proportional(16. * c.s));
            ui.style_mut()
                .text_styles
                .insert(TextStyle::Body, FontId::proportional(16. * c.s));
            ui.spacing_mut().interact_size.y = 30. * c.s;
            ui.visuals_mut().widgets.inactive.corner_radius = CornerRadius::ZERO;
            ui.visuals_mut().widgets.hovered.corner_radius = CornerRadius::ZERO;
            ui.visuals_mut().widgets.active.corner_radius = CornerRadius::ZERO;
            ComboBox::from_id_salt(id)
                .width(width * c.s)
                .selected_text(options[*selected])
                .show_ui(ui, |ui| {
                    for (i, label) in options.iter().enumerate() {
                        ui.selectable_value(selected, i, *label);
                    }
                });
        },
    );
    old != *selected
}

// Vendor timings supplied by the operator, indexed by the saved 0-based level.
fn response_info(level: usize) -> String {
    let ranges = [
        ("2–3", "5–6", "12–13"),
        ("5–6", "7–9", "15–16"),
        ("8–9", "10–12", "18–19"),
        ("13–14", "15–17", "23–24"),
        ("17–18", "19–21", "27–28"),
    ];
    let Some((wired, radio, bluetooth)) = ranges.get(level) else {
        return "Choose a response level from 1 to 5.".into();
    };
    format!(
        "*Delay setting parameters for Level {}:\nWired: approximately {wired} ms\n2.4GHz: approximately {radio} ms\nBluetooth: approximately {bluetooth} ms\nNote: Parameters may vary due to\ndifferent devices and surrounding\nenvironmental interference.\nDifferent setting types may lead to key\nchatter if the delay setting is too low.\nIf key chatter occurs, please increase\nthe delay setting.",
        level + 1
    )
}

#[cfg(test)]
mod settings_ui_tests {
    use super::*;
    fn step(app: &mut Studio, ctx: &Context, events: Vec<Event>) -> egui::FullOutput {
        let mut output = ctx.run_ui(
            RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(1288., 724.))),
                events,
                ..Default::default()
            },
            |ui| {
                let c = Canvas {
                    p: ui.painter().clone(),
                    s: 1.,
                    o: Pos2::ZERO,
                };
                app.settings_page(ui, &c);
            },
        );
        output.textures_delta.clear();
        output
    }
    fn click(app: &mut Studio, ctx: &Context, pos: Pos2) {
        let _ = step(app, ctx, vec![Event::PointerMoved(pos)]);
        for pressed in [true, false] {
            let _ = step(
                app,
                ctx,
                vec![Event::PointerButton {
                    pos,
                    pressed,
                    button: PointerButton::Primary,
                    modifiers: Modifiers::NONE,
                }],
            );
        }
    }
    fn text(shapes: &[egui::epaint::ClippedShape]) -> String {
        fn collect(shape: &egui::Shape, result: &mut String) {
            match shape {
                egui::Shape::Text(t) => {
                    result.push_str(t.galley.text());
                    result.push('\n');
                }
                egui::Shape::Vec(children) => {
                    for child in children {
                        collect(child, result)
                    }
                }
                _ => {}
            }
        }
        let mut result = String::new();
        for shape in shapes {
            collect(&shape.shape, &mut result);
        }
        result
    }
    #[test]
    fn selected_levels_render_all_three_supplied_timings_and_reset_clicks_only_open_confirmation() {
        let ctx = Context::default();
        let mut app = Studio::create(&ctx, Some(PathBuf::from("unused.png")), 8);
        for (level, wired, radio, bluetooth) in [
            (0, "2–3", "5–6", "12–13"),
            (1, "5–6", "7–9", "15–16"),
            (2, "8–9", "10–12", "18–19"),
            (3, "13–14", "15–17", "23–24"),
            (4, "17–18", "19–21", "27–28"),
        ] {
            app.saved.extras.response = level;
            let output = step(&mut app, &ctx, vec![]);
            let rendered = text(&output.shapes);
            assert!(rendered.contains(&format!("parameters for Level {}:", level + 1)));
            assert!(rendered.contains(&format!("Wired: approximately {wired} ms")));
            assert!(rendered.contains(&format!("2.4GHz: approximately {radio} ms")));
            assert!(rendered.contains(&format!("Bluetooth: approximately {bluetooth} ms")));
        }
        let before = serde_json::to_string(&app.saved).unwrap();
        click(&mut app, &ctx, pos2(183., 618.));
        assert_eq!(
            app.pending_reset,
            Some(crate::protocol::ResetKind::Keyboard)
        );
        assert!(app.transfer.is_none());
        app.pending_reset = None;
        click(&mut app, &ctx, pos2(183., 668.));
        assert_eq!(app.pending_reset, Some(crate::protocol::ResetKind::Factory));
        assert!(app.transfer.is_none());
        assert_eq!(serde_json::to_string(&app.saved).unwrap(), before);
    }
}
