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
    browser: std::sync::Mutex<crate::sound_browser::Browser>,
    modulation: crate::modulation_editor::ModEditor,
    pub params: Arc<WaveParams>,
    cycles: [AtomicUsize; 2],
    preset: AtomicUsize,
}
impl TableEditor {
    pub fn new(params: Arc<WaveParams>) -> Self {
        Self {
            browser: Default::default(),
            modulation: crate::modulation_editor::ModEditor::new(params.clone()),
            params,
            cycles: [AtomicUsize::new(5), AtomicUsize::new(5)],
            preset: AtomicUsize::new(usize::MAX),
        }
    }
    pub fn buttons(o: usize) -> [Rect; 3] {
        crate::instrument_view::table_buttons(o)
    }
    pub fn factory_state(index: usize) -> PluginState {
        const FILES: [&[u8]; 14] = [
            include_bytes!("../presets/Reference - Init.ospreset"),
            include_bytes!("../presets/Reference - Bend Bass.ospreset"),
            include_bytes!("../presets/Reference - Mirror Keys.ospreset"),
            include_bytes!("../presets/Reference - Quantize Motion.ospreset"),
            include_bytes!("../presets/Reference - Wide Lead.ospreset"),
            include_bytes!("../presets/Upgrade - Dual Sweep.ospreset"),
            include_bytes!("../presets/Upgrade - Wide Sky.ospreset"),
            include_bytes!("../presets/Upgrade - Moving Edge.ospreset"),
            include_bytes!("../presets/Curves - Velvet Pluck.ospreset"),
            include_bytes!("../presets/Curves - Held Bloom.ospreset"),
            include_bytes!("../presets/Curves - Resonant Band.ospreset"),
            include_bytes!("../presets/Motion - Glass Tide.ospreset"),
            include_bytes!("../presets/Motion - Clockwork.ospreset"),
            include_bytes!("../presets/Motion - Four Corners.ospreset"),
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
    fn activate_classic(
        &self,
        o: usize,
        bank: Option<usize>,
        context: Option<&Arc<dyn GuiContext>>,
    ) {
        use crate::controls::*;
        let mut v = os_plugkit::vals::Vals::new(
            &*self.params.base,
            self.params.base.live.clone(),
            context.cloned(),
        );
        v.edit_plain(SETTINGS[if o == 0 { A_SOURCE } else { B_SOURCE }].id, 0.);
        if let Some(bank) = bank {
            v.edit_plain(SETTINGS[o * 8].id, bank as f32);
        }
    }
    fn commit(
        &self,
        o: usize,
        bank: Option<usize>,
        sources: crate::assets::Sources,
        context: Option<&Arc<dyn GuiContext>>,
    ) {
        if let Some(context) = context {
            let mut state = context.get_state();
            state
                .fields
                .insert(FIELD.into(), serde_json::to_string(&sources).unwrap());
            use crate::controls::*;
            let source = if o == 0 { A_SOURCE } else { B_SOURCE };
            let mut raw = self.params.base.fl_state();
            for (k, value) in [
                (source, 0),
                (
                    o * 8,
                    bank.map(|b| b as i32).unwrap_or(self.params.base.v(o * 8)),
                ),
            ] {
                state.params.insert(
                    SETTINGS[k].id.into(),
                    nih_plug::wrapper::state::ParamValue::I32(value),
                );
                raw[4 + k * 4..8 + k * 4].copy_from_slice(&value.to_le_bytes());
            }
            let hex: String = raw.iter().map(|b| format!("{b:02x}")).collect();
            state
                .fields
                .insert("state".into(), serde_json::to_string(&hex).unwrap());
            context.set_state(state);
        } else {
            match self.params.replace(sources) {
                Ok(()) => self.activate_classic(o, bank, None),
                Err(e) => self.params.message(e),
            }
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
        let hits = crate::instrument_view::paint(
            c,
            model,
            state,
            hint,
            &self.params,
            self.cycles.each_ref().map(|x| x.load(Ordering::Relaxed)),
            (selected < crate::instrument_view::PRESET_NAMES.len()).then_some(selected),
        );
        self.modulation.observe(&hits, c);
        let browser = self.browser.lock().unwrap();
        if browser.oscillator.is_some() {
            browser.paint(c);
            Some(Vec::new())
        } else {
            Some(hits)
        }
    }
    fn dragging(&self) -> bool {
        self.modulation.dragging()
    }
    fn moved(&self, x: f32, y: f32) -> bool {
        self.modulation.moved(x, y)
    }
    fn release(&self, c: Option<&Arc<dyn GuiContext>>) -> bool {
        self.modulation.release(c)
    }
    fn paint(&self, c: &mut Canvas, model: &dyn Model) {
        let _ = self.render(c, model, &Default::default(), "");
    }
    fn click(&self, x: f32, y: f32, context: Option<&Arc<dyn GuiContext>>) -> bool {
        {
            let mut browser = self.browser.lock().unwrap();
            if let Some(o) = browser.oscillator {
                if let Some(choice) = browser.click(x, y) {
                    use crate::sound_browser::Choice;
                    let mut sources = self.params.sources();
                    let mut bank = None;
                    let result = match choice {
                        Choice::Basic(i) => {
                            sources.oscillators[o] = Some(crate::sound_browser::basic(i));
                            sources.samples[o] = None;
                            Ok(())
                        }
                        Choice::Bank(i) => {
                            sources.oscillators[o] = None;
                            sources.samples[o] = None;
                            bank = Some(i);
                            Ok(())
                        }
                        Choice::Sample(e) => {
                            crate::sample::Sample::import(&e.path, e.root).map(|sample| {
                                sources.samples[o] = Some(sample);
                                sources.oscillators[o] = None;
                            })
                        }
                    };
                    match result {
                        Ok(()) => {
                            self.commit(o, bank, sources, context);
                            browser.oscillator = None;
                            self.params.message("Sound loaded and embedded in this patch. Recorded samples play once; Root adjusts tuning.");
                        }
                        Err(e) => {
                            browser.message = format!("Load failed: {e}. Current sound kept.");
                            self.params.message(browser.message.clone());
                        }
                    }
                }
                return true;
            }
        }
        if self.params.page.load(Ordering::Relaxed) == 0 {
            for o in 0..2 {
                if crate::sound_browser::button(o).contains(x, y) {
                    self.browser.lock().unwrap().open(o);
                    return true;
                }
                for (i, delta) in [-12., -1., 1., 12.].iter().enumerate() {
                    if crate::sound_browser::root_button(o, i).contains(x, y) {
                        let mut sources = self.params.sources();
                        if let Some(sample) = &mut sources.samples[o] {
                            sample.root = (sample.root + delta).clamp(0., 127.);
                            self.commit(o, None, sources, context);
                            return true;
                        }
                    }
                }
            }
        }
        if self.modulation.click(x, y, context) {
            return true;
        }
        use crate::instrument_view::{PRESET_INIT, PRESET_NEXT, PRESET_PREV};
        if [PRESET_PREV, PRESET_NEXT, PRESET_INIT]
            .iter()
            .any(|r| r.contains(x, y))
        {
            let current = self.preset.load(Ordering::Relaxed);
            let index = if PRESET_INIT.contains(x, y)
                || current >= crate::instrument_view::PRESET_NAMES.len()
            {
                0
            } else if PRESET_NEXT.contains(x, y) {
                (current + 1) % crate::instrument_view::PRESET_NAMES.len()
            } else {
                (current + crate::instrument_view::PRESET_NAMES.len() - 1)
                    % crate::instrument_view::PRESET_NAMES.len()
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
        if self.params.page.load(Ordering::Relaxed) == 1 {
            return false;
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
                                sources.samples[o] = None;
                                self.commit(o, None, sources, context);
                                self.params.message("Wavetable imported and active. Position scans frames; Browse Sounds selects another source.");
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
                    sources.samples[o] = None;
                    self.commit(o, None, sources, context);
                    self.params
                        .message("Built-in bank restored. Browse Sounds opens the wavetable list.");
                }
            }
            return true;
        }
        false
    }
}
