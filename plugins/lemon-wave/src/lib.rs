// SPDX-License-Identifier: GPL-3.0-or-later
//! Lemon Wave: original two-oscillator wavetable instrument, stage one.
#![allow(semicolon_in_expressions_from_non_local_macros)]
pub mod controls;
pub mod dsp;
use lemon_small_fx::{Shared, params::FxParams};
use nih_plug::prelude::*;
use std::num::NonZeroU32;
use std::sync::{Arc, atomic::Ordering};

pub struct LemonWave {
    params: Arc<FxParams>,
    shared: Arc<Shared>,
    engine: dsp::Engine,
}
impl Default for LemonWave {
    fn default() -> Self {
        Self {
            params: Arc::new(FxParams::new(controls::SPEC)),
            shared: Arc::new(Shared::default()),
            engine: dsp::Engine::new(44100.),
        }
    }
}
impl Plugin for LemonWave {
    const NAME: &'static str = "Lemon Wave";
    const VENDOR: &'static str = "Lemon Studio";
    const URL: &'static str = "";
    const EMAIL: &'static str = "";
    const VERSION: &'static str = env!("CARGO_PKG_VERSION");
    const AUDIO_IO_LAYOUTS: &'static [AudioIOLayout] = &[AudioIOLayout {
        main_input_channels: None,
        main_output_channels: NonZeroU32::new(2),
        ..AudioIOLayout::const_default()
    }];
    const MIDI_INPUT: MidiConfig = MidiConfig::MidiCCs;
    const SAMPLE_ACCURATE_AUTOMATION: bool = true;
    type SysExMessage = ();
    type BackgroundTask = ();
    fn params(&self) -> Arc<dyn Params> {
        self.params.clone()
    }
    fn editor(&mut self, _: AsyncExecutor<Self>) -> Option<Box<dyn Editor>> {
        Some(Box::new(lemon_small_fx::editor::FxEditor::new(
            controls::SPEC,
            self.params.clone(),
            self.shared.clone(),
        )))
    }
    fn initialize(
        &mut self,
        _: &AudioIOLayout,
        c: &BufferConfig,
        _: &mut impl InitContext<Self>,
    ) -> bool {
        if !c.sample_rate.is_finite() || c.sample_rate < 8000. || c.sample_rate > 384000. {
            return false;
        }
        self.engine = dsp::Engine::new(c.sample_rate);
        self.engine
            .set(std::array::from_fn(|i| self.params.v(i) as f32));
        self.engine.reset();
        self.shared
            .rate
            .store(c.sample_rate as u32, Ordering::Relaxed);
        true
    }
    fn reset(&mut self) {
        self.engine.reset();
    }
    fn process(
        &mut self,
        buffer: &mut Buffer,
        _: &mut AuxiliaryBuffers,
        ctx: &mut impl ProcessContext<Self>,
    ) -> ProcessStatus {
        self.engine
            .set(std::array::from_fn(|i| self.params.v(i) as f32));
        let tempo = ctx.transport().tempo.unwrap_or(120.) as f32;
        self.engine.tempo = if tempo.is_finite() {
            tempo.clamp(20., 400.)
        } else {
            120.
        };
        let len = buffer.samples();
        let out = buffer.as_slice();
        for ch in out.iter_mut() {
            ch.fill(0.);
        }
        if out.len() < 2 {
            return ProcessStatus::Normal;
        }
        let (l, r) = out.split_at_mut(1);
        let mut at = 0;
        while let Some(ev) = ctx.next_event() {
            let t = (ev.timing() as usize).min(len).max(at);
            if t > at {
                self.engine.render(&mut l[0][at..t], &mut r[0][at..t]);
                at = t;
            }
            match ev {
                NoteEvent::NoteOn {
                    note,
                    velocity,
                    channel,
                    voice_id,
                    ..
                } => self.engine.on(note, velocity, channel, voice_id),
                NoteEvent::NoteOff {
                    note,
                    channel,
                    voice_id,
                    ..
                } => self.engine.off(note, channel, voice_id),
                NoteEvent::Choke {
                    note,
                    channel,
                    voice_id,
                    ..
                } => self.engine.choke(note, channel, voice_id),
                NoteEvent::PolyTuning {
                    note,
                    channel,
                    voice_id,
                    tuning,
                    ..
                } => self.engine.tune(note, channel, voice_id, tuning),
                NoteEvent::MidiPitchBend { channel, value, .. } => self.engine.bend(channel, value),
                NoteEvent::MidiCC {
                    channel,
                    cc: 64,
                    value,
                    ..
                } => self.engine.sustain(channel, value >= 0.5),
                NoteEvent::MidiCC {
                    channel, cc: 120, ..
                } => self.engine.all_off(channel, true),
                NoteEvent::MidiCC {
                    channel, cc: 123, ..
                } => self.engine.all_off(channel, false),
                _ => {}
            }
        }
        if at < len {
            self.engine.render(&mut l[0][at..len], &mut r[0][at..len]);
        }
        for (note, channel, voice_id) in self.engine.take_ended() {
            ctx.send_event(NoteEvent::VoiceTerminated {
                timing: (len as u32).saturating_sub(1),
                note,
                channel,
                voice_id,
            });
        }
        if self.engine.active() {
            ProcessStatus::KeepAlive
        } else {
            ProcessStatus::Normal
        }
    }
}
impl ClapPlugin for LemonWave {
    const CLAP_ID: &'static str = "org.lemonstudio.lemon-wave";
    const CLAP_DESCRIPTION: Option<&'static str> = Some("Polyphonic wavetable synthesizer");
    const CLAP_MANUAL_URL: Option<&'static str> = None;
    const CLAP_SUPPORT_URL: Option<&'static str> = None;
    const CLAP_FEATURES: &'static [ClapFeature] = &[
        ClapFeature::Instrument,
        ClapFeature::Synthesizer,
        ClapFeature::Stereo,
    ];
}
impl Vst3Plugin for LemonWave {
    const VST3_CLASS_ID: [u8; 16] = *b"LemonWaveSynth01";
    const VST3_SUBCATEGORIES: &'static [Vst3SubCategory] =
        &[Vst3SubCategory::Instrument, Vst3SubCategory::Synth];
}
#[cfg(not(feature = "linked"))]
nih_export_clap!(LemonWave);
#[cfg(not(feature = "linked"))]
nih_export_vst3!(LemonWave);
os_plugkit::linked_clap_entry!(LemonWave);

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn parameter_identity_state_and_editor_layout() {
        let params = FxParams::new(controls::SPEC);
        let map = params.param_map();
        assert_eq!(map.len(), controls::COUNT);
        let mut ids = std::collections::HashSet::new();
        for (id, _, _) in &map {
            assert!(ids.insert(id));
        }
        let values: Vec<i32> = controls::SETTINGS.iter().map(|s| s.range.1).collect();
        let bytes = lemon_small_fx::params::to_fl(controls::SPEC, &values, &params.raw_copy());
        let restored = FxParams::new(controls::SPEC);
        restored.set_raw(bytes.clone());
        let fields = restored.serialize_fields();
        params.deserialize_fields(&fields);
        assert_eq!(params.raw_copy(), bytes);
        for (i, v) in values.iter().enumerate() {
            assert_eq!(
                lemon_small_fx::params::setting(controls::SPEC, &bytes, i),
                *v
            );
        }
        struct Model;
        impl lemon_small_fx::view::Model for Model {
            fn plain(&self, id: &str) -> f32 {
                controls::SETTINGS[controls::SPEC.index(id).unwrap()].start as f32
            }
            fn norm(&self, id: &str) -> f32 {
                let s = &controls::SETTINGS[controls::SPEC.index(id).unwrap()];
                (s.start - s.range.0) as f32 / (s.range.1 - s.range.0) as f32
            }
            fn text(&self, id: &str) -> String {
                let i = controls::SPEC.index(id).unwrap();
                controls::text(i, controls::SETTINGS[i].start)
            }
        }
        for scale in [1., 1.5] {
            let mut c = os_ui::Canvas::new(1100, 720, scale).unwrap();
            let hits = lemon_small_fx::view::paint(
                controls::SPEC,
                &mut c,
                &os_theme::design::palette(os_theme::Design::Lemon, None),
                &Model,
                &Default::default(),
                "",
            );
            for setting in controls::SETTINGS {
                assert!(
                    hits.iter().any(|h| h.t.id() == setting.id),
                    "{}",
                    setting.id
                );
            }
            for h in &hits {
                assert!(h.r.x >= 0. && h.r.y >= 0. && h.r.right() <= 1100. && h.r.bottom() <= 720.);
            }
            if let Ok(dir) = std::env::var("LEMON_WAVE_PREVIEW") {
                std::fs::create_dir_all(&dir).unwrap();
                c.pixmap
                    .save_png(format!("{dir}/lemon-wave-{scale}.png"))
                    .unwrap();
            }
        }
    }
}
