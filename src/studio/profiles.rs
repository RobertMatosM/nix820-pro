use super::*;
use crate::remap::{KeyCode, Layer};

impl Studio {
    pub(super) fn keys(&mut self, ui: &mut Ui, c: &Canvas) {
        for (i, (label, tip)) in [
            ("⊞", "New profile"),
            ("×", "Delete profile"),
            ("✎", "Rename profile"),
            ("⇤", "Import profile"),
            ("⇧", "Export profile"),
        ]
        .iter()
        .enumerate()
        {
            if c.button(
                ui,
                Rect::from_min_size(pos2(100. + i as f32 * 35., 64.), vec2(28., 25.)),
                label,
                false,
                18.,
            )
            .on_hover_text(*tip)
            .clicked()
            {
                match i {
                    0 => {
                        self.saved
                            .profiles
                            .push(Profile::new(self.saved.profiles.len() + 1));
                        self.saved.selected = self.saved.profiles.len() - 1;
                        self.save();
                    }
                    1 => {
                        if self.saved.profiles.len() > 1 {
                            self.saved.profiles.remove(self.saved.selected);
                            self.saved.selected = 0;
                            self.save();
                        }
                    }
                    2 => {
                        self.rename_value = self.p().name.clone();
                        self.rename = true;
                    }
                    3 => self.open_file("Import profile"),
                    _ => self.open_file("Export profile"),
                }
            }
        }
        for i in 0..self.saved.profiles.len() {
            let y = 107. + i as f32 * 32.;
            let r = Rect::from_min_size(pos2(66., y - 16.), vec2(239., 32.));
            if i == self.saved.selected {
                c.fill(r, 5., SELECTION)
            }
            c.p.circle_stroke(c.pos(89., y), 10. * c.s, Stroke::new(2. * c.s, FG));
            if i == self.saved.selected {
                c.p.circle_filled(c.pos(89., y), 10. * c.s, FG);
            }
            c.text(
                pos2(118., y),
                &self.saved.profiles[i].name,
                16.,
                FG,
                Align2::LEFT_CENTER,
            );
            if c.hit(ui, r, (i, "profile")).clicked() {
                self.saved.selected = i;
                self.save();
            }
        }
        for (i, label) in ["Base Layer", "Fn Layer"].iter().enumerate() {
            if c.button(
                ui,
                Rect::from_min_size(pos2(640. + 160. * i as f32, 81.), vec2(150., 30.)),
                label,
                self.layer == (i == 1),
                16.,
            )
            .clicked()
            {
                self.layer = i == 1;
                if self
                    .key
                    .as_deref()
                    .is_some_and(|key| !Layer::from_fn(self.layer).allows(key))
                {
                    self.key = None;
                }
            }
        }
        let layer = Layer::from_fn(self.layer);
        let receiver = self.receiver_selected();
        let apply_label = if receiver {
            "Apply selected key".into()
        } else {
            format!("Apply {} layer", layer.display_name())
        };
        self.apply_button(ui, c, &apply_label, 1);
        if c.button(
            ui,
            Rect::from_min_size(pos2(975., 82.), vec2(126., 30.)),
            if receiver && self.layer { "Use normal key" } else { "Restore key" },
            false,
            13.,
        )
        .on_hover_text(if receiver && self.layer {
            "Assign the printed physical key to this Fn position locally, then Apply selected key. This does not restore its factory Fn shortcut."
        } else if receiver {
            "Restore this key locally, then Apply selected key to send its original action."
        } else { "Restore the selected key locally, then Apply this layer." })
        .clicked()
        {
            self.restore_selected_key();
        }
        if c.button_enabled(
            ui,
            Rect::from_min_size(pos2(1110., 82.), vec2(135., 30.)),
            &format!("Default {} layer", layer.display_name()),
            13.,
            !receiver,
        )
        .on_hover_text(
            "Clear this profile's selected layer overrides locally, then Apply this layer.",
        )
        .on_disabled_hover_text(
            "Whole-layer defaults require wired USB. Wireless Apply changes one selected key.",
        )
        .clicked()
        {
            self.pm().restore_layer(layer);
            self.notify(format!(
                "{}-layer defaults saved locally. Click Apply {} layer to send them.",
                layer.display_name(),
                layer.display_name()
            ));
            self.save();
        }
        self.keyboard(ui, c);
        self.destination_picker(ui, c);
        let selection = self
            .key
            .as_ref()
            .map(|key| {
                format!(
                    "Selected: {key} → {}",
                    self.p().key_label(key, self.layer).unwrap_or_else(|| {
                        if receiver && self.layer {
                            "No explicit action".into()
                        } else {
                            "Default".into()
                        }
                    })
                )
            })
            .unwrap_or_else(|| "Select a source key, then a destination below.".into());
        c.text(pos2(326., 126.), selection, 12., MUTED, Align2::LEFT_CENTER);
        c.text(pos2(326., 142.), if receiver && self.layer {
            "Wireless: sends one selected key. Use normal key assigns its printed action; factory Fn shortcuts require wired USB."
        } else if receiver {
            "Wireless: sends only the selected key. Restore key prepares its original action; click Apply selected key to send."
        } else if self.layer {
            "Fn reserves Esc, Backspace, Home, Win, Fn and Backslash/Pipe. Apply replaces this layer’s overrides."
        } else {
            "Apply replaces all Base-layer overrides with this profile. Edits stay local until Apply."
        }, 11., MUTED, Align2::LEFT_CENTER);
    }
    fn assign_key(&mut self, destination: KeyCode) {
        let layer = Layer::from_fn(self.layer);
        let Some(source) = self.key.clone() else {
            self.notify("Select a key on the keyboard first.");
            return;
        };
        match self.pm().assign_layer(layer, &source, Some(destination)) {
            Ok(()) => {
                let apply = if self.receiver_selected() {
                    "Apply selected key".into()
                } else {
                    format!("Apply {} layer", layer.display_name())
                };
                self.notify(format!(
                    "Saved {source} → {} locally. Click {apply} to send.",
                    destination.label()
                ));
                self.save();
            }
            Err(e) => self.notify(e),
        }
    }
    fn restore_selected_key(&mut self) {
        let layer = Layer::from_fn(self.layer);
        let Some(source) = self.key.clone() else {
            self.notify("Select a key to restore first.");
            return;
        };
        let normal_fn = self.receiver_selected() && layer == Layer::Fn;
        let destination = if normal_fn {
            let Some(key) = crate::remap::base_key(&source) else {
                self.notify("No normal action is available for this key.");
                return;
            };
            Some(key)
        } else {
            None
        };
        match self.pm().assign_layer(layer, &source, destination) {
            Ok(()) => {
                if normal_fn {
                    self.notify(format!("Fn+{source} will use its printed key. Click Apply selected key to send; factory Fn shortcuts are not restored."));
                } else if self.receiver_selected() {
                    self.notify(format!("Original action for {source} saved locally. Click Apply selected key to send."));
                } else {
                    self.notify(format!(
                        "Default for {source} saved locally. Click Apply {} layer to send.",
                        layer.display_name()
                    ));
                }
                self.save();
            }
            Err(e) => self.notify(e),
        }
    }
}

impl Studio {
    fn destination_picker(&mut self, ui: &mut Ui, c: &Canvas) {
        let area = c.rect(Rect::from_min_size(pos2(330., 446.), vec2(870., 220.)));
        let mut chosen = None;
        ui.scope_builder(UiBuilder::new().max_rect(area), |ui| {
            ui.heading("Assign a destination");
            ui.label("Select a physical key above, then choose its new action.");
            ui.horizontal(|ui| {
                for (index, label) in ["All", "Keyboard", "Modifiers", "Media", "Browser"]
                    .iter()
                    .enumerate()
                {
                    ui.selectable_value(&mut self.destination_category, index, *label);
                }
                ui.add(
                    TextEdit::singleline(&mut self.destination_query)
                        .hint_text("Search keys and actions…")
                        .desired_width(230. * c.s),
                );
            });
            ui.separator();
            let choices = destination_choices(self.destination_category, &self.destination_query);
            ScrollArea::vertical()
                .id_salt("destinations")
                .max_height(140. * c.s)
                .show(ui, |ui| {
                    if choices.is_empty() {
                        ui.label("No matching destinations.");
                    }
                    ui.horizontal_wrapped(|ui| {
                        for key in choices {
                            let response =
                                ui.add_enabled(self.key.is_some(), Button::new(key.label()));
                            if response.on_hover_text(format!("{key:?}")).clicked() {
                                chosen = Some(key);
                            }
                        }
                    });
                });
        });
        if let Some(key) = chosen {
            self.assign_key(key);
        }
    }
}
fn destination_choices(category: usize, query: &str) -> Vec<KeyCode> {
    let query = query.trim().to_lowercase();
    KeyCode::ALL
        .iter()
        .copied()
        .filter(|key| {
            let record = key.record();
            let group = if record[0] == 3 {
                if record[2] >= 2 { 4 } else { 3 }
            } else if record[1] != 0 {
                2
            } else {
                1
            };
            (category == 0 || category == group)
                && format!("{} {key:?}", key.label())
                    .to_lowercase()
                    .contains(&query)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    fn step(
        app: &mut Studio,
        ctx: &Context,
        events: Vec<Event>,
    ) -> Vec<egui::epaint::ClippedShape> {
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
                app.keys(ui, &c);
            },
        );
        output.textures_delta.clear();
        output.shapes
    }
    fn click(app: &mut Studio, ctx: &Context, pos: Pos2) {
        step(app, ctx, vec![Event::PointerMoved(pos)]);
        for pressed in [true, false] {
            step(
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
    fn find_button(shapes: &[egui::epaint::ClippedShape], label: &str) -> Pos2 {
        fn find(shape: &Shape, label: &str) -> Option<Pos2> {
            match shape {
                Shape::Text(t) if t.galley.text() == label => {
                    Some(t.pos + t.galley.rect.size() * 0.5)
                }
                Shape::Vec(children) => children.iter().find_map(|shape| find(shape, label)),
                _ => None,
            }
        }
        shapes
            .iter()
            .find_map(|shape| find(&shape.shape, label))
            .unwrap_or_else(|| panic!("Button not rendered: {label}"))
    }
    #[test]
    fn receiver_routes_one_selected_key_and_keeps_fn_restoration_explicit() {
        let ctx = Context::default();
        let mut app = Studio::create(&ctx, Some(PathBuf::from("unused.png")), 1);
        app.keyboards = vec![crate::device::Keyboard {
            connection: crate::device::Connection::Receiver,
            feature: "/nonexistent/receiver".into(),
            display: None,
            label: "test receiver".into(),
        }];
        app.pm()
            .assign_layer(Layer::Top, "A", Some(KeyCode::B))
            .unwrap();
        app.pm()
            .assign_layer(Layer::Top, "W", Some(KeyCode::Mute))
            .unwrap();
        app.pm()
            .assign_layer(Layer::Fn, "I", Some(KeyCode::Insert))
            .unwrap();
        assert!(!app.can_apply_map());
        app.key = Some("A".into());
        let check = |app: &Studio, layer: Layer, source: &str, destination: KeyCode| {
            let crate::device::Job::Remap {
                layer: actual_layer,
                source: actual_source,
                destination: actual_destination,
            } = app.map_job().unwrap()
            else {
                panic!("must create a single-key job")
            };
            assert_eq!(
                (actual_layer, actual_source.as_str(), actual_destination),
                (layer, source, destination)
            );
        };
        let shapes = step(&mut app, &ctx, vec![]);
        find_button(&shapes, "Apply selected key");
        check(&app, Layer::Top, "A", KeyCode::B);
        let saved = serde_json::to_string(&app.saved).unwrap();
        click(&mut app, &ctx, pos2(1170., 96.)); // Receiver whole-layer defaults are disabled.
        assert_eq!(serde_json::to_string(&app.saved).unwrap(), saved);
        click(&mut app, &ctx, pos2(1035., 96.)); // Top Restore key prepares normal A.
        check(&app, Layer::Top, "A", KeyCode::A);
        assert_eq!(app.p().top_keys.get("W"), Some(&Some(KeyCode::Mute)));
        click(&mut app, &ctx, pos2(875., 96.)); // Switch to Fn with A still selected.
        assert!(!app.can_apply_map()); // Unset Fn defaults must never become an ordinary A implicitly.
        app.key = Some("I".into());
        check(&app, Layer::Fn, "I", KeyCode::Insert);
        let shapes = step(&mut app, &ctx, vec![]);
        find_button(&shapes, "Use normal key");
        click(&mut app, &ctx, pos2(1035., 96.));
        check(&app, Layer::Fn, "I", KeyCode::I);
        assert_eq!(app.p().fn_keys.get("I"), Some(&Some(KeyCode::I)));
        let saved = serde_json::to_string(&app.saved).unwrap();
        click(&mut app, &ctx, pos2(1170., 96.));
        assert_eq!(serde_json::to_string(&app.saved).unwrap(), saved);
        assert!(app.transfer.is_none(), "local restore must wait for Apply");
        app.keyboards[0].connection = crate::device::Connection::Wired;
        assert!(matches!(app.map_job().unwrap(), crate::device::Job::Map(_)));
        click(&mut app, &ctx, pos2(1035., 96.)); // Wired Restore still restores firmware defaults.
        assert_eq!(app.p().fn_keys.get("I"), Some(&None));
    }
    #[test]
    fn searchable_picker_assigns_fn_insert_and_restores_without_touching_base() {
        let ctx = Context::default();
        let mut app = Studio::create(&ctx, Some(PathBuf::from("unused.png")), 1);
        app.pm().assign_top("A", Some(KeyCode::B)).unwrap();
        click(&mut app, &ctx, pos2(875., 96.)); // Fn.
        click(&mut app, &ctx, pos2(821., 271.)); // Physical I.
        app.destination_query = "Insert".into();
        let shapes = step(&mut app, &ctx, vec![]);
        let target = find_button(&shapes, "Ins");
        click(&mut app, &ctx, target);
        assert_eq!(app.p().fn_keys.get("I"), Some(&Some(KeyCode::Insert)));
        let fixture: serde_json::Value = serde_json::from_str(include_str!(
            "../../tests/fixtures/fn-layer-remap-test-fixtures.json"
        ))
        .unwrap();
        let map = app.p().map_for_layer(Layer::Fn).unwrap();
        assert_eq!(
            map.as_bytes()
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect::<String>(),
            fixture["transactions"][0]["body"]
        );
        click(&mut app, &ctx, pos2(1035., 96.));
        assert_eq!(
            app.p().map_for_layer(Layer::Fn).unwrap(),
            crate::remap::KeyMap::new(Layer::Fn)
        );
        assert_eq!(app.p().top_keys.get("A"), Some(&Some(KeyCode::B)));
        assert!(app.transfer.is_none());
    }
    #[test]
    fn picker_keeps_every_destination_distinct_and_searchable_by_category() {
        assert_eq!(destination_choices(0, "").len(), KeyCode::ALL.len());
        for key in KeyCode::ALL {
            assert!(destination_choices(0, &format!("{key:?}")).contains(key));
        }
        assert!(destination_choices(2, "shift").contains(&KeyCode::RightShift));
        assert!(!destination_choices(1, "shift").contains(&KeyCode::RightShift));
        assert_eq!(
            destination_choices(4, "search"),
            vec![KeyCode::BrowserSearch]
        );
        assert_eq!(destination_choices(3, "mute"), vec![KeyCode::Mute]);
        assert_eq!(destination_choices(1, "Numpad7"), vec![KeyCode::Numpad7]);
    }
    #[test]
    fn reserved_fn_keys_cannot_be_selected_and_switching_layers_clears_reserved_selection() {
        let ctx = Context::default();
        let mut app = Studio::create(&ctx, Some(PathBuf::from("unused.png")), 1);
        click(&mut app, &ctx, pos2(498., 185.));
        assert_eq!(app.key.as_deref(), Some("Esc"));
        click(&mut app, &ctx, pos2(875., 96.));
        assert!(app.key.is_none());
        for position in [
            pos2(498., 185.),
            pos2(1009., 233.),
            pos2(1082., 233.),
            pos2(550., 385.),
            pos2(916., 385.),
            pos2(1018., 271.),
        ] {
            click(&mut app, &ctx, position);
            assert!(app.key.is_none());
        }
        click(&mut app, &ctx, pos2(821., 271.));
        assert_eq!(app.key.as_deref(), Some("I"));
    }
}
