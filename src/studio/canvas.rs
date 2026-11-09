use super::*;

pub(super) struct Canvas {
    pub(super) p: Painter,
    pub(super) s: f32,
    pub(super) o: Pos2,
}
impl Canvas {
    pub(super) fn pos(&self, x: f32, y: f32) -> Pos2 {
        self.o + vec2(x, y) * self.s
    }
    pub(super) fn point(&self, p: Pos2) -> Pos2 {
        self.pos(p.x, p.y)
    }
    pub(super) fn rect(&self, r: Rect) -> Rect {
        Rect::from_min_max(self.point(r.min), self.point(r.max))
    }
    pub(super) fn fill(&self, r: Rect, round: f32, col: Color32) {
        self.p.rect_filled(self.rect(r), round * self.s, col);
    }
    pub(super) fn outline(&self, r: Rect, round: f32, col: Color32, width: f32) {
        self.p.rect_stroke(
            self.rect(r),
            round * self.s,
            Stroke::new(width * self.s, col),
            StrokeKind::Inside,
        );
    }
    pub(super) fn line(&self, a: Pos2, b: Pos2, col: Color32, w: f32) {
        self.p
            .line_segment([self.point(a), self.point(b)], Stroke::new(w * self.s, col));
    }
    pub(super) fn text(&self, p: Pos2, t: impl ToString, size: f32, col: Color32, align: Align2) {
        self.p.text(
            self.point(p),
            align,
            t.to_string(),
            FontId::proportional(size * self.s),
            col,
        );
    }
    pub(super) fn hit(
        &self,
        ui: &mut Ui,
        r: Rect,
        id: impl std::hash::Hash + std::fmt::Debug,
    ) -> Response {
        ui.interact(self.rect(r), Id::unique(id), Sense::click_and_drag())
    }
    pub(super) fn button(
        &self,
        ui: &mut Ui,
        r: Rect,
        t: &str,
        selected: bool,
        size: f32,
    ) -> Response {
        let response = self.hit(ui, r, (t, r.min.x.to_bits(), r.min.y.to_bits()));
        self.fill(
            r,
            5.,
            if selected {
                ACCENT
            } else if response.hovered() {
                Color32::from_gray(65)
            } else {
                SURFACE
            },
        );
        self.text(
            r.center(),
            t,
            size,
            if !ui.is_enabled() {
                MUTED
            } else if selected {
                INK
            } else {
                FG
            },
            Align2::CENTER_CENTER,
        );
        response
    }
    pub(super) fn button_enabled(
        &self,
        ui: &mut Ui,
        r: Rect,
        label: &str,
        size: f32,
        enabled: bool,
    ) -> Response {
        ui.add_enabled_ui(enabled, |ui| self.button(ui, r, label, false, size))
            .inner
    }
    pub(super) fn checkbox(&self, ui: &mut Ui, p: Pos2, label: &str, v: &mut bool) -> bool {
        let r = Rect::from_min_size(p, vec2(18., 18.));
        self.outline(r, 0., MUTED, 1.5);
        if *v {
            self.text(r.center(), "✓", 16., ACCENT, Align2::CENTER_CENTER);
        }
        self.text(p + vec2(32., 9.), label, 16., FG, Align2::LEFT_CENTER);
        if self
            .hit(ui, Rect::from_min_size(p, vec2(116., 24.)), label)
            .clicked()
        {
            *v = !*v;
            true
        } else {
            false
        }
    }
    #[allow(
        clippy::too_many_arguments,
        reason = "Canvas controls share one positional drawing API"
    )]
    pub(super) fn slider(
        &self,
        ui: &mut Ui,
        id: &str,
        p: Pos2,
        w: f32,
        v: &mut f32,
        min: f32,
        max: f32,
        col: Color32,
    ) -> bool {
        let r = Rect::from_min_max(p - vec2(10., 12.), p + vec2(w + 10., 12.));
        let response = self.hit(ui, r, id);
        let old = *v;
        if (response.dragged() || response.clicked())
            && let Some(pointer) = response.interact_pointer_pos()
        {
            *v = (min + (pointer.x - self.point(p).x) / (w * self.s) * (max - min)).clamp(min, max);
        }
        let t = (*v - min) / (max - min);
        self.line(p, p + vec2(w, 0.), Color32::from_gray(88), 8.);
        self.line(p, p + vec2(w * t, 0.), col, 8.);
        self.p
            .circle_filled(self.point(p + vec2(w * t, 0.)), 10. * self.s, MUTED);
        *v != old
    }
}
