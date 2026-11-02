// SPDX-License-Identifier: GPL-3.0-or-later
//! Original wavetable engine; analytic spectra, not recovered vendor tables.
use crate::controls::*;
use std::f32::consts::{PI, TAU};
use std::sync::{Arc, OnceLock};
const SIZE: usize = 1024;
const FRAMES: usize = 5;
const MIPS: usize = 9;
pub const VOICES: usize = 24;
pub type Patch = [f32; COUNT];
pub fn defaults() -> Patch {
    std::array::from_fn(|i| SETTINGS[i].start as f32)
}

fn harmonic(bank: usize, frame: usize, n: usize) -> f32 {
    let t = frame as f32 / 4.;
    let h = n as f32;
    match bank {
        0 => {
            let sine = if n == 1 { 1. } else { 0. };
            let saw = 1. / h;
            let square = if n % 2 == 1 { 1. / h } else { 0. };
            if t <= 0.5 {
                sine + (saw - sine) * t * 2.
            } else {
                saw + (square - saw) * (t - 0.5) * 2.
            }
        }
        1 => {
            if n % 2 == 1 {
                ((h * (0.2 + t * 0.7)).cos() * 0.4 + 0.6) / h
            } else {
                0.
            }
        }
        2 => {
            let peak = 2. + t * 14.;
            (-(h - peak).powi(2) / 8.).exp() * 0.45 + if n == 1 { 0.5 } else { 0.02 / h }
        }
        _ => {
            let f = 2. + t * 9.;
            ((-(h - f).powi(2) / 3.).exp() + 0.5 * (-(h - f * 2.7).powi(2) / 8.).exp()) / h.sqrt()
                + if n == 1 { 0.3 } else { 0. }
        }
    }
}
pub fn preview(bank: usize, pos: f32, phase: f32) -> f32 {
    let f = pos.clamp(0., 1.) * 4.;
    let a = (f.floor() as usize).min(3);
    let w = f - a as f32;
    (1..=32)
        .map(|n| {
            (harmonic(bank, a, n) * (1. - w) + harmonic(bank, a + 1, n) * w)
                * (TAU * phase * n as f32).sin()
        })
        .sum::<f32>()
        * 0.5
}
struct Tables {
    data: Vec<f32>,
}
impl Tables {
    fn new() -> Self {
        let mut data = vec![0.; 4 * FRAMES * MIPS * SIZE];
        for b in 0..4 {
            for f in 0..FRAMES {
                let mut full = vec![0.; SIZE];
                for mip in 0..MIPS {
                    // Incrementally add the harmonic band for this level.
                    let first = if mip == 0 { 1 } else { (1 << (mip - 1)) + 1 };
                    for n in first..=(1 << mip) {
                        let amp = harmonic(b, f, n);
                        for (i, x) in full.iter_mut().enumerate() {
                            *x += amp * (TAU * n as f32 * i as f32 / SIZE as f32).sin();
                        }
                    }
                    let start = ((b * FRAMES + f) * MIPS + mip) * SIZE;
                    data[start..start + SIZE].copy_from_slice(&full);
                }
                // One gain for every mip, so pitch changes do not change normalization.
                let peak = full.iter().fold(1.0_f32, |a, x| a.max(x.abs()));
                let start = (b * FRAMES + f) * MIPS * SIZE;
                for x in &mut data[start..start + MIPS * SIZE] {
                    *x /= peak;
                }
            }
        }
        Self { data }
    }
    fn sample(&self, bank: usize, pos: f32, phase: f32, freq: f32, rate: f32) -> f32 {
        let safe = (rate * 0.45 / freq.max(1.)).max(1.);
        let mip = safe.log2().floor().clamp(0., (MIPS - 1) as f32) as usize;
        let blend = (safe / (1 << mip) as f32 - 1.).clamp(0., 1.);
        let frame = pos.clamp(0., 1.) * 4.;
        let f = (frame.floor() as usize).min(3);
        let w = frame - f as f32;
        let p = phase * SIZE as f32;
        let i = (p as usize) % SIZE;
        let t = p - p.floor();
        let read = |f, m| {
            let start = ((bank * FRAMES + f) * MIPS + m) * SIZE;
            self.data[start + i] * (1. - t) + self.data[start + (i + 1) % SIZE] * t
        };
        let read = |f| {
            if mip == 0 {
                read(f, 0)
            } else {
                read(f, mip - 1) * (1. - blend) + read(f, mip) * blend
            }
        };
        read(f) * (1. - w) + read(f + 1) * w
    }
}

#[derive(Clone, Copy, Default)]
struct Env {
    value: f32,
    stage: u8,
    release_step: f32,
}
impl Env {
    fn start(&mut self) {
        *self = Self {
            stage: 1,
            ..Self::default()
        };
    }
    fn off(&mut self, r: f32, rate: f32) {
        if self.stage != 0 && self.stage != 4 {
            self.stage = 4;
            self.release_step = self.value / (r * rate).max(1.);
        }
    }
    fn tick(&mut self, a: f32, d: f32, s: f32, rate: f32) -> f32 {
        match self.stage {
            1 => {
                self.value += 1. / (a * rate).max(1.);
                if self.value >= 1. {
                    self.value = 1.;
                    self.stage = 2;
                }
            }
            2 => {
                self.value -= (1. - s) / (d * rate).max(1.);
                if self.value <= s {
                    self.value = s;
                    self.stage = 3;
                }
            }
            3 => self.value = s,
            4 => {
                self.value -= self.release_step;
                if self.value <= 0. {
                    self.value = 0.;
                    self.stage = 0;
                }
            }
            _ => {}
        }
        self.value
    }
}
#[derive(Clone, Copy, Default)]
struct Filter {
    ic1: f32,
    ic2: f32,
}
impl Filter {
    fn tick(&mut self, x: f32, g: f32, k: f32, mode: usize) -> f32 {
        let a = 1. / (1. + g * (g + k));
        let v1 = a * (self.ic1 + g * (x - self.ic2));
        let v2 = self.ic2 + g * v1;
        self.ic1 = 2. * v1 - self.ic1;
        self.ic2 = 2. * v2 - self.ic2;
        match mode {
            1 => v2,
            2 => v1,
            3 => x - k * v1 - v2,
            _ => x,
        }
    }
}
#[derive(Clone, Copy, Default)]
struct Voice {
    note: u8,
    channel: u8,
    id: Option<i32>,
    age: u64,
    held: bool,
    velocity: f32,
    tune: f32,
    phase: [[f32; 7]; 2],
    sub: f32,
    lfo: f32,
    amp: Env,
    env: Env,
    filter: [Filter; 2],
}
pub struct Engine {
    tables: Arc<Tables>,
    voices: [Voice; VOICES],
    pub rate: f32,
    current: Patch,
    target: Patch,
    age: u64,
    bend: [f32; 16],
    pedal: [bool; 16],
    noise: u32,
    delay: [Vec<f32>; 2],
    chorus: [Vec<f32>; 2],
    head: usize,
    ch_head: usize,
    ch_phase: f32,
    pub tempo: f32,
    ended: Vec<(u8, u8, Option<i32>)>,
    tail: usize,
}
impl Engine {
    pub fn new(rate: f32) -> Self {
        static TABLES: OnceLock<Arc<Tables>> = OnceLock::new();
        Self {
            tables: TABLES.get_or_init(|| Arc::new(Tables::new())).clone(),
            voices: [Voice::default(); VOICES],
            rate,
            current: defaults(),
            target: defaults(),
            age: 0,
            bend: [0.; 16],
            pedal: [false; 16],
            noise: 0x53e974ab,
            delay: std::array::from_fn(|_| vec![0.; (rate * 1.2) as usize + 2]),
            chorus: std::array::from_fn(|_| vec![0.; (rate * 0.06) as usize + 2]),
            head: 0,
            ch_head: 0,
            ch_phase: 0.,
            tempo: 120.,
            ended: Vec::with_capacity(256),
            tail: 0,
        }
    }
    pub fn reset(&mut self) {
        self.voices.fill(Voice::default());
        self.bend.fill(0.);
        self.pedal.fill(false);
        self.delay.iter_mut().for_each(|x| x.fill(0.));
        self.chorus.iter_mut().for_each(|x| x.fill(0.));
        self.head = 0;
        self.ch_head = 0;
        self.ch_phase = 0.;
        self.current = self.target;
        self.tail = 0;
        self.ended.clear();
    }
    pub fn set(&mut self, p: Patch) {
        self.target = std::array::from_fn(|i| {
            if p[i].is_finite() {
                p[i].clamp(SETTINGS[i].range.0 as f32, SETTINGS[i].range.1 as f32)
            } else {
                SETTINGS[i].start as f32
            }
        });
    }
    pub fn on(&mut self, note: u8, velocity: f32, channel: u8, id: Option<i32>) {
        if channel >= 16 || note > 127 {
            return;
        }
        if velocity <= 0. {
            self.off(note, channel, id);
            return;
        }
        self.age += 1;
        let index = self
            .voices
            .iter()
            .position(|v| v.amp.stage == 0)
            .unwrap_or_else(|| {
                self.voices
                    .iter()
                    .enumerate()
                    .min_by_key(|(_, v)| (v.held, v.age))
                    .unwrap()
                    .0
            });
        let old = self.voices[index];
        if old.amp.stage != 0 && self.ended.len() < 256 {
            self.ended.push((old.note, old.channel, old.id));
        }
        let mut v = Voice {
            note,
            channel,
            id,
            age: self.age,
            held: true,
            velocity: velocity.clamp(0., 1.),
            ..Voice::default()
        };
        for o in 0..2 {
            for u in 0..7 {
                v.phase[o][u] = ((u * 37 + o * 13) as f32 / 101.).fract();
            }
        }
        v.amp.start();
        v.env.start();
        self.voices[index] = v;
    }
    pub fn off(&mut self, note: u8, channel: u8, id: Option<i32>) {
        for v in &mut self.voices {
            if v.note == note && v.channel == channel && id.is_none_or(|id| v.id == Some(id)) {
                v.held = false;
                if !self.pedal[channel.min(15) as usize] {
                    v.amp.off(seconds(self.target[RELEASE]), self.rate);
                    v.env.off(seconds(self.target[ENV_RELEASE]), self.rate);
                }
            }
        }
    }
    pub fn sustain(&mut self, channel: u8, on: bool) {
        if channel >= 16 {
            return;
        }
        self.pedal[channel as usize] = on;
        if !on {
            for v in &mut self.voices {
                if v.channel == channel && !v.held {
                    v.amp.off(seconds(self.target[RELEASE]), self.rate);
                    v.env.off(seconds(self.target[ENV_RELEASE]), self.rate);
                }
            }
        }
    }
    pub fn bend(&mut self, channel: u8, value: f32) {
        if channel < 16 && value.is_finite() {
            self.bend[channel as usize] = (value.clamp(0., 1.) * 2. - 1.) * 2.;
        }
    }
    pub fn tune(&mut self, note: u8, channel: u8, id: Option<i32>, value: f32) {
        if value.is_finite() {
            for v in &mut self.voices {
                if v.note == note && v.channel == channel && id.is_none_or(|i| v.id == Some(i)) {
                    v.tune = value.clamp(-96., 96.);
                }
            }
        }
    }
    pub fn choke(&mut self, note: u8, channel: u8, id: Option<i32>) {
        for v in &mut self.voices {
            if v.note == note && v.channel == channel && id.is_none_or(|i| v.id == Some(i)) {
                if self.ended.len() < 256 {
                    self.ended.push((v.note, v.channel, v.id));
                }
                *v = Voice::default();
            }
        }
    }
    pub fn all_off(&mut self, channel: u8, hard: bool) {
        for v in &mut self.voices {
            if v.channel == channel {
                v.held = false;
                v.amp.off(
                    if hard {
                        0.002
                    } else {
                        seconds(self.target[RELEASE])
                    },
                    self.rate,
                );
                v.env.off(0.002, self.rate);
            }
        }
    }
    pub fn take_ended(&mut self) -> impl Iterator<Item = (u8, u8, Option<i32>)> + '_ {
        self.ended.drain(..)
    }
    pub fn active(&self) -> bool {
        self.voices.iter().any(|v| v.amp.stage != 0) || self.tail > 0
    }
    pub fn render(&mut self, left: &mut [f32], right: &mut [f32]) {
        let smooth = 1. - (-1. / (0.01 * self.rate)).exp();
        for (out_l, out_r) in left.iter_mut().zip(right) {
            for i in 0..COUNT {
                self.current[i] += (self.target[i] - self.current[i]) * smooth;
            }
            let p = &self.current;
            let discrete = &self.target;
            let attack = seconds(p[ATTACK]);
            let decay = seconds(p[DECAY]);
            let ea = seconds(p[ENV_ATTACK]);
            let ed = seconds(p[ENV_DECAY]);
            let sync = discrete[LFO_SYNC] as usize;
            let hz = if sync == 0 {
                lfo_hz(p[LFO_RATE])
            } else {
                self.tempo.clamp(20., 400.)
                    / 60.
                    / [16., 8., 4., 2., 1., 0.5, 0.25, 0.125][sync - 1]
            };
            let mut output = [0.; 2];
            let mut sounding = false;
            for v in &mut self.voices {
                if v.amp.stage == 0 {
                    continue;
                }
                sounding = true;
                let amp = v.amp.tick(attack, decay, p[SUSTAIN] / 1000., self.rate) * v.velocity;
                let env = v.env.tick(ea, ed, p[ENV_SUSTAIN] / 1000., self.rate);
                if v.amp.stage == 0 && self.ended.len() < 256 {
                    self.ended.push((v.note, v.channel, v.id));
                }
                let lfo = match discrete[LFO_SHAPE] as usize {
                    1 => 1. - 4. * (v.lfo - 0.5).abs(),
                    2 => v.lfo * 2. - 1.,
                    3 => {
                        if v.lfo < 0.5 {
                            1.
                        } else {
                            -1.
                        }
                    }
                    _ => (v.lfo * TAU).sin(),
                } * p[LFO_AMOUNT]
                    / 1000.;
                v.lfo = (v.lfo + hz / self.rate).fract();
                let target = discrete[LFO_TARGET] as usize;
                let pitch = v.note as f32 - 69.
                    + v.tune
                    + self.bend[v.channel as usize]
                    + if target == 2 { lfo * 2. } else { 0. };
                let base = 440. * 2.0_f32.powf(pitch / 12.);
                let mut sig = [0.; 2];
                for o in 0..2 {
                    let b = o * 8;
                    let count = discrete[b + 4] as usize;
                    let bank = discrete[b] as usize;
                    let pos =
                        (p[b + 1] / 1000. + if target == 0 { lfo * 0.5 } else { 0. }).clamp(0., 1.);
                    for u in 0..count {
                        let offset = if count == 1 {
                            0.
                        } else {
                            u as f32 / (count - 1) as f32 * 2. - 1.
                        };
                        let freq = base * 2.0_f32.powf((p[b + 3] + offset * p[b + 5] / 100.) / 12.);
                        let phase = v.phase[o][u];
                        v.phase[o][u] = (phase + freq / self.rate).fract();
                        // Phase bend is an original two-slope warp; conservative mip selection limits added harmonics.
                        let pivot = 0.5 + p[b + 7] / 200.;
                        let warped = if phase < pivot {
                            phase / pivot * 0.5
                        } else {
                            0.5 + (phase - pivot) / (1. - pivot) * 0.5
                        };
                        let max_slope = 0.5 / pivot.min(1. - pivot);
                        let s = if freq < self.rate * 0.45 {
                            self.tables
                                .sample(bank, pos, warped, freq * max_slope, self.rate)
                        } else {
                            0.
                        };
                        let pan = (p[b + 6] / 100. + offset * p[SPREAD] / 1000.).clamp(-1., 1.);
                        let angle = (pan + 1.) * PI / 4.;
                        let gain = p[b + 2] / 1000. / (count as f32).sqrt();
                        sig[0] += s * angle.cos() * gain;
                        sig[1] += s * angle.sin() * gain;
                    }
                }
                v.sub = (v.sub + base * 0.5 / self.rate).fract();
                let sub = (v.sub * TAU).sin() * p[SUB] / 1000.;
                self.noise ^= self.noise << 13;
                self.noise ^= self.noise >> 17;
                self.noise ^= self.noise << 5;
                let noise =
                    (self.noise as f32 / u32::MAX as f32 * 2. - 1.) * p[NOISE] / 1000. * 0.3;
                let cv =
                    (p[CUTOFF] + env * p[ENV_AMOUNT] + if target == 1 { lfo * 500. } else { 0. })
                        .clamp(0., 1000.);
                let g = (PI * cutoff(cv).min(self.rate * 0.42) / self.rate).tan();
                let k = 2. - 1.9 * p[RESONANCE] / 1000.;
                let drive = 1. + p[DRIVE] / 100.;
                let am = if target == 3 {
                    1. - p[LFO_AMOUNT] / 2000. + lfo * 0.5
                } else {
                    1.
                };
                for ch in 0..2 {
                    let x = (sig[ch] + sub + noise) * 0.3;
                    let x = if p[DRIVE] > 0.1 {
                        (x * drive).tanh() / drive.sqrt()
                    } else {
                        x
                    };
                    output[ch] += v.filter[ch].tick(x, g, k, discrete[FILTER] as usize) * amp * am;
                }
            }
            if sounding {
                self.tail = (self.rate
                    * if p[DELAY_MIX] > 0.1 {
                        90.
                    } else if p[CHORUS_MIX] > 0.1 {
                        0.06
                    } else {
                        0.
                    }) as usize;
            } else {
                self.tail = self.tail.saturating_sub(1);
            }
            self.ch_phase = (self.ch_phase + (0.05 + p[CHORUS_RATE] / 200.) / self.rate).fract();
            let mut wet_delay = [0.; 2];
            for ch in 0..2 {
                let ms = 12.
                    + (self.ch_phase * TAU + ch as f32 * PI / 2.).sin() * p[CHORUS_DEPTH] / 100.;
                let wet = read_delay(&self.chorus[ch], self.ch_head, ms * 0.001 * self.rate);
                self.chorus[ch][self.ch_head] = output[ch];
                output[ch] =
                    output[ch] * (1. - p[CHORUS_MIX] / 2000.) + wet * p[CHORUS_MIX] / 2000.;
                wet_delay[ch] = read_delay(
                    &self.delay[ch],
                    self.head,
                    (50. + p[DELAY_TIME]) * 0.001 * self.rate,
                );
            }
            for ch in 0..2 {
                self.delay[ch][self.head] =
                    (output[ch] + wet_delay[1 - ch] * p[DELAY_FEEDBACK] / 1000. * 0.85).tanh();
                output[ch] += wet_delay[ch] * p[DELAY_MIX] / 1000. * 0.6;
            }
            self.head = (self.head + 1) % self.delay[0].len();
            self.ch_head = (self.ch_head + 1) % self.chorus[0].len();
            let gain = p[GAIN] / 1000.;
            *out_l = (output[0] * gain).tanh();
            *out_r = (output[1] * gain).tanh();
        }
    }
}
fn read_delay(data: &[f32], head: usize, delay: f32) -> f32 {
    let at = (head as f32 - delay.clamp(1., data.len() as f32 - 2.)).rem_euclid(data.len() as f32);
    // f32 rem_euclid can round a tiny negative position up to exactly len.
    let i = (at as usize) % data.len();
    let f = at - at.floor();
    data[i] * (1. - f) + data[(i + 1) % data.len()] * f
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn delay_wrap_interpolates_across_the_ring_boundary() {
        let mut data = vec![0.; 57602];
        data[0] = 1.;
        let v = read_delay(&data, 18000, 18000.002);
        assert!(v.is_finite() && (0.99..=1.).contains(&v));
        data[57601] = 0.5;
        assert!((read_delay(&data, 18000, 18000.5) - 0.75).abs() < 1e-5);
    }
    #[test]
    fn render_pitch_release_reset_and_extremes() {
        for rate in [44100., 48000., 96000.] {
            let mut e = Engine::new(rate);
            let mut p = defaults();
            p[A_POSITION] = 0.;
            p[ATTACK] = 0.;
            p[SUSTAIN] = 1000.;
            p[FILTER] = 0.;
            p[RELEASE] = 0.;
            e.set(p);
            e.reset();
            e.on(69, 1., 0, Some(1));
            let mut l = vec![0.; rate as usize];
            let mut r = l.clone();
            e.render(&mut l, &mut r);
            let crosses = l[2000..]
                .windows(2)
                .filter(|s| s[0] <= 0. && s[1] > 0.)
                .count();
            let hz = crosses as f32 * rate / (l.len() - 2000) as f32;
            assert!((hz - 440.).abs() < 2., "{hz}");
            assert!(l.iter().any(|x| x.abs() > 0.01));
            e.off(69, 0, Some(1));
            e.render(&mut l[..1024], &mut r[..1024]);
            assert!(l[512..1024].iter().all(|x| x.abs() < 1e-6));
            for i in 0..COUNT {
                p[i] = SETTINGS[i].range.1 as f32;
            }
            e.set(p);
            e.reset();
            for n in 24..56 {
                e.on(n, 1., 0, None);
            }
            e.render(&mut l, &mut r);
            assert!(l.iter().chain(&r).all(|x| x.is_finite() && x.abs() <= 1.));
            e.reset();
            e.render(&mut l, &mut r);
            assert!(l.iter().chain(&r).all(|x| *x == 0.));
        }
    }
    #[test]
    fn channel_note_identity_pedal_and_block_boundaries() {
        let mut a = Engine::new(48000.);
        let mut b = Engine::new(48000.);
        a.on(60, 0.8, 0, Some(1));
        b.on(60, 0.8, 0, Some(1));
        let mut l = [0.; 1024];
        let mut r = l;
        let mut l2 = l;
        let mut r2 = l;
        a.render(&mut l, &mut r);
        for (x, y) in l2.chunks_mut(73).zip(r2.chunks_mut(73)) {
            b.render(x, y);
        }
        assert_eq!(l, l2);
        assert_eq!(r, r2);
        a.on(60, 0.8, 1, Some(2));
        a.sustain(0, true);
        a.off(60, 0, Some(1));
        assert!(a.voices.iter().any(|v| v.id == Some(1) && v.amp.stage != 4));
        a.sustain(0, false);
        assert!(a.voices.iter().any(|v| v.id == Some(1) && v.amp.stage == 4));
        assert!(a.voices.iter().any(|v| v.id == Some(2) && v.held));
    }
}
