// SPDX-License-Identifier: GPL-3.0-or-later
use crate::{
    assets::{FIELD, WaveParams},
    wavetable::{self, CYCLES},
};
use lemon_small_fx::{editor::Extension, view::Model};
use nih_plug::prelude::*;
use os_ui::{Canvas, Rect};
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

pub struct TableEditor {
    pub params: Arc<WaveParams>,
    cycles: [AtomicUsize; 2],
    preset: AtomicUsize,
}
impl TableEditor {
    pub fn new(params: Arc<WaveParams>) -> Self {
        Self {
            params,
            cycles: [AtomicUsize::new(5), AtomicUsize::new(5)],
            preset: AtomicUsize::new(usize::MAX),
        }
    }
    pub fn buttons(o: usize) -> [Rect; 3] {
        crate::instrument_view::table_buttons(o)
    }
    pub fn factory_state(index: usize) -> PluginState {
        const FILES: [&[u8]; 8] = [
            include_bytes!("../presets/Reference - Init.ospreset"),
            include_bytes!("../presets/Reference - Bend Bass.ospreset"),
            include_bytes!("../presets/Reference - Mirror Keys.ospreset"),
            include_bytes!("../presets/Reference - Quantize Motion.ospreset"),
            include_bytes!("../presets/Reference - Wide Lead.ospreset"),
            include_bytes!("../presets/Upgrade - Dual Sweep.ospreset"),
            include_bytes!("../presets/Upgrade - Wide Sky.ospreset"),
            include_bytes!("../presets/Upgrade - Moving Edge.ospreset"),
        ];
        let body = FILES[index % FILES.len()]
            .splitn(3, |b| *b == b'\n')
            .nth(2)
            .unwrap();
        let mut state: PluginState =
            serde_json::from_slice(&body[8..]).expect("bundled factory state");
        crate::LemonWave::filter_state(&mut state);
        state
    }
    fn commit(&self, sources: crate::assets::Sources, context: Option<&Arc<dyn GuiContext>>) {
        if let Some(context) = context {
            let mut state = context.get_state();
            state
                .fields
                .insert(FIELD.into(), serde_json::to_string(&sources).unwrap());
            context.set_state(state);
        } else if let Err(e) = self.params.replace(sources) {
            self.params.message(e);
        }
    }
}
impl Extension for TableEditor {
    fn render(
        &self,
        c: &mut Canvas,
        model: &dyn Model,
        state: &lemon_small_fx::view::ViewState,
        hint: &str,
    ) -> Option<Vec<lemon_small_fx::view::Hit>> {
        let selected = self.preset.load(Ordering::Relaxed);
        Some(crate::instrument_view::paint(
            c,
            model,
            state,
            hint,
            &self.params,
            self.cycles.each_ref().map(|x| x.load(Ordering::Relaxed)),
            (selected < 8).then_some(selected),
        ))
    }
    fn paint(&self, c: &mut Canvas, model: &dyn Model) {
        let _ = self.render(c, model, &Default::default(), "");
    }
    fn click(&self, x: f32, y: f32, context: Option<&Arc<dyn GuiContext>>) -> bool {
        use crate::instrument_view::{PRESET_INIT, PRESET_NEXT, PRESET_PREV};
        if [PRESET_PREV, PRESET_NEXT, PRESET_INIT]
            .iter()
            .any(|r| r.contains(x, y))
        {
            let current = self.preset.load(Ordering::Relaxed);
            let index = if PRESET_INIT.contains(x, y) || current >= 8 {
                0
            } else if PRESET_NEXT.contains(x, y) {
                (current + 1) % 8
            } else {
                (current + 7) % 8
            };
            if let Some(context) = context {
                context.set_state(Self::factory_state(index));
                self.preset.store(index, Ordering::Relaxed);
                self.params.message(format!(
                    "Factory preset: {}",
                    crate::instrument_view::PRESET_NAMES[index]
                ));
            }
            return true;
        }
        for o in 0..2 {
            let Some(action) = Self::buttons(o).iter().position(|r| r.contains(x, y)) else {
                continue;
            };
            match action {
                0 => {
                    if let Some(path) = rfd::FileDialog::new()
                        .set_title("Import periodic wavetable WAV")
                        .add_filter("WAV wavetable", &["wav"])
                        .pick_file()
                    {
                        match wavetable::import(
                            &path,
                            CYCLES[self.cycles[o].load(Ordering::Relaxed)],
                        ) {
                            Ok(source) => {
                                let mut sources = self.params.sources();
                                sources.oscillators[o] = Some(source);
                                self.commit(sources, context);
                                self.params.message("Imported table is embedded. Choose Classic source to play it; Use built-in restores the Bank knob.");
                            }
                            Err(e) => self
                                .params
                                .message(format!("Import failed: {e}. Current sound kept.")),
                        }
                    }
                }
                1 => {
                    let v = self.cycles[o].load(Ordering::Relaxed);
                    self.cycles[o].store((v + 1) % CYCLES.len(), Ordering::Relaxed);
                    self.params.message("Cycle length applies to the next import. Stereo WAVs are averaged to mono; maximum 128 frames.");
                }
                _ => {
                    let mut sources = self.params.sources();
                    sources.oscillators[o] = None;
                    self.commit(sources, context);
                    self.params
                        .message("Built-in bank restored for this oscillator.");
                }
            }
            return true;
        }
        false
    }
}
