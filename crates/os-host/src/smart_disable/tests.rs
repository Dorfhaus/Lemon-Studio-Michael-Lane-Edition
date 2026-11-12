use crate::{
    Attention, Descriptor, Format, HostedEffect, Instrument, Kind, Plugin, SharedPlugin, guard,
    guarded,
};
use os_engine::TransportInfo;
use std::{
    collections::VecDeque,
    ffi::c_void,
    sync::{Arc, Mutex},
};

// Deterministic synthetic DSP: a delayed impulse, timed notes, and a sidechain
// that can generate output even with silent main input. No audio device required.
struct Probe {
    calls: usize,
    frames: usize,
    queries: usize,
    tail: Option<u64>,
    latency: u32,
    delay: VecDeque<f32>,
    notes: Vec<(usize, f32)>,
    noise: f32,
    voices: usize,
    aux: usize,
    editor: bool,
}
impl Default for Probe {
    fn default() -> Self {
        Self {
            calls: 0,
            frames: 0,
            queries: 0,
            tail: Some(0),
            latency: 0,
            delay: VecDeque::new(),
            notes: Vec::new(),
            noise: 0.,
            voices: 0,
            aux: 0,
            editor: false,
        }
    }
}
struct TestPlugin {
    desc: Descriptor,
    probe: Arc<Mutex<Probe>>,
    attention: Arc<Attention>,
}
impl Instrument for TestPlugin {
    fn note_on(&mut self, offset: u32, _: u8, velocity: f32) {
        self.probe
            .lock()
            .unwrap()
            .notes
            .push((offset as usize, velocity));
    }
    fn note_off(&mut self, _: u32, _: u8) {}
    fn process(&mut self, _: u32, l: &mut [f32], r: &mut [f32]) {
        let mut p = self.probe.lock().unwrap();
        p.calls += 1;
        p.frames += l.len();
        l.fill(p.noise);
        for &(offset, gain) in &p.notes {
            if offset < l.len() {
                l[offset] += gain;
            }
        }
        p.notes.retain_mut(|(offset, _)| {
            if *offset < l.len() {
                false
            } else {
                *offset -= l.len();
                true
            }
        });
        r.copy_from_slice(l);
    }
    fn voices(&self) -> usize {
        self.probe.lock().unwrap().voices
    }
    fn extra_outputs(&self) -> usize {
        self.probe.lock().unwrap().aux
    }
}
impl HostedEffect for TestPlugin {
    fn process_fx(&mut self, _: u32, l: &mut [f32], r: &mut [f32]) {
        let mut p = self.probe.lock().unwrap();
        p.calls += 1;
        p.frames += l.len();
        for sample in l.iter_mut() {
            p.delay.push_back(*sample);
            *sample = p.delay.pop_front().unwrap() * 0.5 + p.noise;
        }
        r.copy_from_slice(l);
    }
    fn process_fx_keyed(&mut self, rate: u32, l: &mut [f32], r: &mut [f32], key: (&[f32], &[f32])) {
        self.process_fx(rate, l, r);
        for (v, k) in l.iter_mut().zip(key.0) {
            *v += *k;
        }
        for (v, k) in r.iter_mut().zip(key.1) {
            *v += *k;
        }
    }
}
impl Plugin for TestPlugin {
    fn desc(&self) -> &Descriptor {
        &self.desc
    }
    fn activate(&mut self, _: u32) -> bool {
        true
    }
    fn idle(&mut self) {}
    fn has_editor(&self) -> bool {
        true
    }
    fn editor_open(&self) -> bool {
        self.probe.lock().unwrap().editor
    }
    fn open_editor(&mut self, _: *mut c_void, _: f32) -> Result<(u32, u32), String> {
        self.probe.lock().unwrap().editor = true;
        Ok((10, 10))
    }
    fn editor_size(&self) -> Option<(u32, u32)> {
        None
    }
    fn close_editor(&mut self) {
        self.probe.lock().unwrap().editor = false;
    }
    fn save_state(&self) -> Option<Vec<u8>> {
        Some(vec![42])
    }
    fn load_state(&mut self, _: &[u8]) -> bool {
        true
    }
    fn latency(&self) -> u32 {
        self.probe.lock().unwrap().latency
    }
    fn attention(&self) -> Option<Arc<Attention>> {
        Some(self.attention.clone())
    }
    fn smart_disable_tail(&self) -> Option<u64> {
        let mut p = self.probe.lock().unwrap();
        p.queries += 1;
        if p.editor || !p.notes.is_empty() {
            None
        } else {
            p.tail
        }
    }
    fn advance_silent(&mut self, _: u32, frames: usize) {
        self.probe.lock().unwrap().frames += frames;
    }
}
fn plugin() -> (SharedPlugin, Arc<Mutex<Probe>>) {
    let probe = Arc::new(Mutex::new(Probe::default()));
    let desc = Descriptor {
        format: Format::Vst3,
        id: "sleep-test".into(),
        name: "Sleep test".into(),
        vendor: String::new(),
        version: String::new(),
        features: vec![],
        kind: Kind::Instrument,
        path: Default::default(),
    };
    (
        guarded::share(
            TestPlugin {
                desc,
                probe: probe.clone(),
                attention: Arc::default(),
            },
            Arc::new(guard::Module::default()),
        ),
        probe,
    )
}
fn quiet(p: &mut dyn Plugin, blocks: usize) {
    for _ in 0..blocks {
        p.process(1000, &mut [0.; 10], &mut [0.; 10]);
    }
}

#[test]
fn optional_sleep_reduces_dsp_calls_but_advances_clock() {
    let (p, probe) = plugin();
    let mut p = p.lock().unwrap();
    quiet(&mut *p, 1000);
    assert_eq!(probe.lock().unwrap().calls, 1000);
    assert_eq!(
        probe.lock().unwrap().queries,
        0,
        "off does not query plugin metadata"
    );
    p.set_smart_disable(true);
    quiet(&mut *p, 1000);
    assert_eq!(
        probe.lock().unwrap().calls,
        1400,
        "six seconds of DSP skipped"
    );
    assert_eq!(
        probe.lock().unwrap().frames,
        20000,
        "host time includes skipped frames"
    );
    p.set_smart_disable(false);
    quiet(&mut *p, 3);
    assert_eq!(probe.lock().unwrap().calls, 1403);
    p.set_smart_disable(true);
    quiet(&mut *p, 1);
    assert_eq!(
        probe.lock().unwrap().calls,
        1404,
        "enable starts a fresh quiet window"
    );
}

#[test]
fn long_silent_delay_gap_and_latency_preserve_every_sample() {
    let render = |enabled| {
        let (p, probe) = plugin();
        {
            let mut s = probe.lock().unwrap();
            s.tail = Some(6000);
            s.latency = 17;
            s.delay = VecDeque::from(vec![0.; 6017]);
        }
        let mut p = p.lock().unwrap();
        p.set_smart_disable_fx(enabled);
        let mut out = Vec::new();
        for block in 0..2200 {
            let mut l = [0.; 10];
            let mut r = [0.; 10];
            if block == 0 {
                l[0] = 1.;
            }
            p.process_fx(1000, &mut l, &mut r);
            out.extend(l);
            assert_eq!(l, r);
        }
        let calls = probe.lock().unwrap().calls;
        (out, calls)
    };
    let (plain, plain_calls) = render(false);
    let (sleeping, sleeping_calls) = render(true);
    assert_eq!(sleeping, plain);
    assert_eq!(sleeping[6017], 0.5);
    assert!(sleeping_calls < plain_calls - 500);
}

#[test]
fn audio_and_sidechain_wake_on_the_same_block() {
    let (p, probe) = plugin();
    let mut p = p.lock().unwrap();
    p.set_smart_disable_fx(true);
    quiet(&mut *p, 500);
    let calls = probe.lock().unwrap().calls;
    let mut l = [0.; 10];
    let mut r = [0.; 10];
    let mut key = [0.; 10];
    key[3] = 0.25;
    p.process_fx_keyed(1000, &mut l, &mut r, (&key, &key));
    assert_eq!(l, key);
    assert_eq!(r, key);
    assert_eq!(probe.lock().unwrap().calls, calls + 1);
    quiet(&mut *p, 500);
    let calls = probe.lock().unwrap().calls;
    l.fill(0.);
    l[7] = 1.;
    p.process_fx(1000, &mut l, &mut r);
    assert_eq!(l[7], 0.5);
    assert_eq!(probe.lock().unwrap().calls, calls + 1);
}

#[test]
fn queued_notes_keep_their_sample_offsets_after_sleep() {
    let (p, probe) = plugin();
    let mut p = p.lock().unwrap();
    p.set_smart_disable(true);
    quiet(&mut *p, 500);
    p.note_on(53, 60, 0.7);
    let mut out = Vec::new();
    for _ in 0..10 {
        let mut l = [0.; 10];
        p.process(1000, &mut l, &mut [0.; 10]);
        out.extend(l);
    }
    assert_eq!(out.iter().position(|v| *v != 0.), Some(53));
    assert_eq!(out[53], 0.7);
    assert_eq!(probe.lock().unwrap().calls, 410);
}

#[test]
fn edits_midi_notes_resets_and_external_requests_wake() {
    let wakes: &[fn(&mut dyn Plugin)] = &[
        |p| p.note_off(0, 60),
        |p| p.midi(0, [0xB0, 64, 127]),
        |p| p.param(0, 0, 0.5),
        |p| p.param_fx(0, 0, 0.5),
        |p| {
            p.set_param(0, 0.5);
        },
        |p| {
            p.load_state(&[42]);
        },
        |p| {
            p.select_program(0, 0);
        },
        |p| p.reset(),
        |p| p.reset_fx(),
        |p| {
            p.attention().unwrap().raise();
        },
        |p| {
            p.channel_levels(os_engine::ChannelLevels {
                volume: 0.5,
                ..Default::default()
            });
        },
        |p| p.tune(0, 60, 1.5),
    ];
    for wake in wakes {
        let (p, probe) = plugin();
        let mut p = p.lock().unwrap();
        p.set_smart_disable(true);
        quiet(&mut *p, 500);
        let calls = probe.lock().unwrap().calls;
        wake(&mut *p);
        quiet(&mut *p, 1);
        assert_eq!(probe.lock().unwrap().calls, calls + 1);
        assert_eq!(
            p.save_state(),
            Some(vec![42]),
            "sleep never destroys plugin state"
        );
    }
}

#[test]
fn voices_editors_aux_unknown_tails_and_nonzero_output_stay_awake() {
    for case in 0..8 {
        let (p, probe) = plugin();
        {
            let mut s = probe.lock().unwrap();
            match case {
                0 => s.voices = 1,
                1 => s.editor = true,
                2 => s.aux = 1,
                3 => s.tail = None,
                4 => s.noise = 1e-30,
                5 => s.noise = f32::NAN,
                6 => s.noise = f32::INFINITY,
                _ => s.tail = Some(u64::MAX),
            }
        }
        let mut p = p.lock().unwrap();
        p.set_smart_disable(true);
        quiet(&mut *p, 1000);
        assert_eq!(probe.lock().unwrap().calls, 1000, "case {case}");
    }
}

#[test]
fn offline_rendering_never_sleeps_and_rate_changes_wake() {
    let (p, probe) = plugin();
    let mut p = p.lock().unwrap();
    p.set_smart_disable(true);
    quiet(&mut *p, 500);
    let calls = probe.lock().unwrap().calls;
    p.set_offline(true);
    quiet(&mut *p, 1000);
    assert_eq!(probe.lock().unwrap().calls, calls + 1000);
    p.set_offline(false);
    quiet(&mut *p, 500);
    let calls = probe.lock().unwrap().calls;
    p.process(2000, &mut [0.; 10], &mut [0.; 10]);
    assert_eq!(probe.lock().unwrap().calls, calls + 1);
}

#[test]
fn raw_midi_held_notes_and_sustain_prevent_sleep_until_released() {
    for (channel, pedal) in [(0, 64), (1, 64), (15, 64), (1, 66), (15, 66)] {
        let (p, probe) = plugin();
        let mut p = p.lock().unwrap();
        p.set_smart_disable(true);
        p.midi(0, [0x90 | channel, 60, 100]);
        quiet(&mut *p, 500);
        assert_eq!(
            probe.lock().unwrap().calls,
            500,
            "raw MIDI notes can sound after a silent gap"
        );
        p.midi(0, [0xB0 | channel, pedal, 127]);
        p.midi(0, [0x80 | channel, 60, 64]);
        quiet(&mut *p, 500);
        assert_eq!(
            probe.lock().unwrap().calls,
            1000,
            "sustain retains released notes"
        );
        p.midi(0, [0xB0 | channel, pedal, 0]);
        quiet(&mut *p, 500);
        assert_eq!(
            probe.lock().unwrap().calls,
            1400,
            "pedal release allows sleep after tails"
        );
        p.midi(0, [0x90 | channel, 61, 100]);
        p.reset();
        quiet(&mut *p, 500);
        assert_eq!(
            probe.lock().unwrap().calls,
            1800,
            "reset clears raw MIDI holds"
        );
    }
}

#[test]
fn transport_continuity_sleeps_but_seek_loop_tempo_and_play_changes_wake() {
    for case in 0..6 {
        let (p, probe) = plugin();
        let mut p = p.lock().unwrap();
        p.set_smart_disable(true);
        let song = TransportInfo {
            playing: true,
            tempo: 120.,
            frame: 0,
            beats: 0.,
            bar_start: 0.,
            bar: 0,
            numerator: 4,
            denominator: 4,
            looping: None,
        };
        for i in 0..500 {
            p.transport(&song.advanced(i * 10, 1000));
            quiet(&mut *p, 1);
        }
        assert_eq!(probe.lock().unwrap().calls, 400);
        let mut changed = song.advanced(5000, 1000);
        match case {
            0 => changed.frame = 2,
            1 => changed.frame = 7000,
            2 => changed.tempo = 130.,
            3 => changed.playing = false,
            4 => changed.looping = Some((0., 4.)),
            _ => changed.frame = 4995,
        }
        p.transport(&changed);
        quiet(&mut *p, 1);
        assert_eq!(probe.lock().unwrap().calls, 401);
    }
}
