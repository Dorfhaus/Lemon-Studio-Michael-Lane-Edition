// SPDX-License-Identifier: GPL-3.0-or-later
use crate::{assets::WaveParams, controls::*, instrument_view as view, modulation::POINTS};
use lemon_small_fx::view::Hit;
use nih_plug::prelude::*;
use std::sync::{Arc, Mutex, atomic::Ordering};
#[derive(Clone, Copy)]
enum Gesture {
    Draw {
        lfo: usize,
        last: (f32, f32),
        before: [f32; POINTS],
    },
    Route {
        source: usize,
        x: f32,
        y: f32,
    },
}
pub struct ModEditor {
    params: Arc<WaveParams>,
    gesture: Mutex<Option<Gesture>>,
    undo: Mutex<[Option<[f32; POINTS]>; 4]>,
    hits: Mutex<Vec<Hit>>,
}
impl ModEditor {
    pub fn new(params: Arc<WaveParams>) -> Self {
        Self {
            params,
            gesture: Mutex::new(None),
            undo: Mutex::new([None; 4]),
            hits: Mutex::new(Vec::new()),
        }
    }
    fn edit(&self, edits: &[(usize, f32)], context: Option<&Arc<dyn GuiContext>>) {
        let mut v = os_plugkit::vals::Vals::new(
            &*self.params.base,
            self.params.base.live.clone(),
            context.cloned(),
        );
        for (k, x) in edits {
            v.edit_plain(SETTINGS[*k].id, *x);
        }
    }
    fn commit(&self, context: Option<&Arc<dyn GuiContext>>) {
        if let Some(c) = context {
            let mut state = c.get_state();
            state.fields.insert(
                crate::modulation::FIELD.into(),
                self.params.curves.read().unwrap().encode(),
            );
            c.set_state(state);
        }
    }
    fn point(i: usize, x: f32, y: f32) -> (f32, f32) {
        let r = view::lfo_graph(i);
        (
            ((x - r.x) / r.w).clamp(0., 1.),
            (1. - 2. * (y - r.y) / r.h).clamp(-1., 1.),
        )
    }
    pub fn observe(&self, hits: &[Hit], canvas: &mut os_ui::Canvas) {
        *self.hits.lock().unwrap() = hits.to_vec();
        if let Some(Gesture::Route { source, x, y }) = *self.gesture.lock().unwrap() {
            let color = os_ui::Color::rgb(0xbda4fb);
            canvas.fill_rect(
                os_ui::Rect::new(16., 794., 1148., 24.),
                os_ui::Color::rgb(0x101319),
            );
            canvas.text(
                os_ui::Font::light(11, 0).natural(),
                20.,
                800.,
                color,
                &format!(
                    "Dragging {} - release on a destination knob; hover SYNTH to change page",
                    ALL_MOD_SOURCES[source]
                ),
            );
            if let Some(h) = hits
                .iter()
                .find(|h| h.r.contains(x, y) && SPEC.index(h.t.id()).and_then(mod_target).is_some())
            {
                let r = h.r;
                canvas.polyline(
                    &[
                        (r.x, r.y),
                        (r.right(), r.y),
                        (r.right(), r.bottom()),
                        (r.x, r.bottom()),
                        (r.x, r.y),
                    ],
                    1.5,
                    color,
                );
            }
        }
    }
    pub fn click(&self, x: f32, y: f32, context: Option<&Arc<dyn GuiContext>>) -> bool {
        for i in 0..2 {
            if view::page_button(i).contains(x, y) {
                self.params.page.store(i, Ordering::Relaxed);
                return true;
            }
        }
        let page = self.params.page.load(Ordering::Relaxed);
        for (source, r, _) in view::source_buttons(page) {
            if r.contains(x, y) {
                *self.gesture.lock().unwrap() = Some(Gesture::Route { source, x, y });
                return true;
            }
        }
        if page == 0 {
            return false;
        }
        for i in 0..4 {
            if view::lfo_graph(i).contains(x, y) {
                let before = self.params.curves.read().unwrap().samples[i];
                let point = Self::point(i, x, y);
                self.params.curves.write().unwrap().draw(i, point, point);
                *self.gesture.lock().unwrap() = Some(Gesture::Draw {
                    lfo: i,
                    last: point,
                    before,
                });
                if i == 0 {
                    self.edit(&[(LFO_DRAW, 1.)], context);
                }
                return true;
            }
            for j in 0..5 {
                if view::shape_button(i, j).contains(x, y) {
                    let before = self.params.curves.read().unwrap().samples[i];
                    if j == 4 {
                        if let Some(old) = self.undo.lock().unwrap()[i] {
                            self.params.curves.write().unwrap().samples[i] = old;
                        }
                    } else {
                        self.params.curves.write().unwrap().preset(i, j);
                    }
                    self.undo.lock().unwrap()[i] = Some(before);
                    if i == 0 {
                        self.edit(&[(LFO_DRAW, 1.)], context);
                    }
                    self.commit(context);
                    return true;
                }
            }
        }
        for (i, slot) in ROUTES.into_iter().enumerate() {
            if view::clear_route(i).contains(x, y) {
                self.edit(&[(slot, 0.), (slot + 2, 0.)], context);
                return true;
            }
        }
        false
    }
    pub fn dragging(&self) -> bool {
        self.gesture.lock().unwrap().is_some()
    }
    pub fn moved(&self, x: f32, y: f32) -> bool {
        let mut g = self.gesture.lock().unwrap();
        match *g {
            Some(Gesture::Draw { lfo, last, before }) => {
                let point = Self::point(lfo, x, y);
                self.params.curves.write().unwrap().draw(lfo, last, point);
                *g = Some(Gesture::Draw {
                    lfo,
                    last: point,
                    before,
                });
                true
            }
            Some(Gesture::Route { source, .. }) => {
                for i in 0..2 {
                    if view::page_button(i).contains(x, y) {
                        self.params.page.store(i, Ordering::Relaxed);
                    }
                }
                *g = Some(Gesture::Route { source, x, y });
                true
            }
            None => false,
        }
    }
    pub fn release(&self, context: Option<&Arc<dyn GuiContext>>) -> bool {
        let g = self.gesture.lock().unwrap().take();
        match g {
            Some(Gesture::Draw { lfo, before, .. }) => {
                self.undo.lock().unwrap()[lfo] = Some(before);
                self.commit(context);
                self.params.message(
                    "Shape saved with the patch. Undo restores the last stroke for this LFO.",
                );
                true
            }
            Some(Gesture::Route { source, x, y }) => {
                let target = self
                    .hits
                    .lock()
                    .unwrap()
                    .iter()
                    .find(|h| h.r.contains(x, y))
                    .and_then(|h| SPEC.index(h.t.id()))
                    .and_then(mod_target);
                if let Some(t) = target {
                    self.assign(source, t, context);
                } else {
                    self.params
                        .message("Drop cancelled: choose an oscillator, filter or output knob.");
                }
                true
            }
            None => false,
        }
    }
    fn assign(&self, source: usize, target: usize, context: Option<&Arc<dyn GuiContext>>) {
        if ROUTES.iter().any(|s| {
            self.params.base.v(*s) == source as i32 && self.params.base.v(*s + 1) == target as i32
        }) {
            self.params
                .message("Route already exists. Adjust its Depth in Modulation.");
            return;
        }
        let slot = ROUTES[2..].iter().chain(&ROUTES[..2]).copied().find(|s| {
            (*s >= MOD3_SOURCE || (source <= 4 && target < 8)) && self.params.base.v(*s) == 0
        });
        if let Some(s) = slot {
            self.edit(
                &[(s + 1, target as f32), (s + 2, 250.), (s, source as f32)],
                context,
            );
            self.params.message(format!(
                "{} -> {}: depth +25%; edit in Modulation",
                ALL_MOD_SOURCES[source], ALL_MOD_TARGETS[target]
            ));
        } else {
            self.params
                .message("No compatible empty route. Clear a route in Modulation first.");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lemon_small_fx::view::Model;
    struct ModelOf(Arc<WaveParams>);
    impl Model for ModelOf {
        fn plain(&self, id: &str) -> f32 {
            self.0.base.v(SPEC.index(id).unwrap()) as f32
        }
        fn norm(&self, id: &str) -> f32 {
            let s = &SETTINGS[SPEC.index(id).unwrap()];
            (self.plain(id) - s.range.0 as f32) / (s.range.1 - s.range.0) as f32
        }
        fn text(&self, id: &str) -> String {
            crate::controls::text(SPEC.index(id).unwrap(), self.plain(id) as i32)
        }
    }
    #[test]
    fn draw_undo_cross_page_drop_and_state_recall() {
        let params = Arc::new(WaveParams::default());
        let editor = ModEditor::new(params.clone());
        params.page.store(1, Ordering::Relaxed);
        let before = *params.curves.read().unwrap();
        let graph = view::lfo_graph(2);
        assert!(editor.click(graph.x + 1., graph.y + 1., None));
        assert!(editor.dragging());
        editor.moved(graph.right() - 1., graph.bottom() - 1.);
        editor.release(None);
        assert_ne!(*params.curves.read().unwrap(), before);
        let saved = params.serialize_fields();
        let restored = WaveParams::default();
        restored.deserialize_fields(&saved);
        assert_eq!(
            *params.curves.read().unwrap(),
            *restored.curves.read().unwrap()
        );
        let undo = view::shape_button(2, 4);
        editor.click(undo.x + 2., undo.y + 2., None);
        assert_eq!(*params.curves.read().unwrap(), before);
        let source = view::source_buttons(1)
            .into_iter()
            .find(|(s, _, _)| *s == 6)
            .unwrap()
            .1;
        editor.click(source.x + 2., source.y + 2., None);
        let tab = view::page_button(0);
        editor.moved(tab.x + 2., tab.y + 2.);
        assert_eq!(params.page.load(Ordering::Relaxed), 0);
        let mut canvas = os_ui::Canvas::new(1180, 820, 1.).unwrap();
        let hits = view::paint(
            &mut canvas,
            &ModelOf(params.clone()),
            &Default::default(),
            "",
            &params,
            [5; 2],
            None,
        );
        let target = hits.iter().find(|h| h.t.id() == "cutoff").unwrap().r;
        editor.observe(&hits, &mut canvas);
        editor.moved(target.x + 10., target.y + 10.);
        editor.release(None);
        assert_eq!(params.base.v(MOD3_SOURCE), 6);
        assert_eq!(params.base.v(MOD3_TARGET), 1);
        assert_eq!(params.base.v(MOD3_AMOUNT), 250);
        params.page.store(1, Ordering::Relaxed);
        let clear = view::clear_route(2);
        editor.click(clear.x + 2., clear.y + 2., None);
        assert_eq!(params.base.v(MOD3_SOURCE), 0);
        restored.deserialize_fields(&std::collections::BTreeMap::new());
        assert_eq!(
            *restored.curves.read().unwrap(),
            crate::modulation::Curves::default()
        );
    }
}
