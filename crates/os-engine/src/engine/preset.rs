//! Prepared preset handoff. No plugin loading or destruction runs in the renderer.
use super::*;
use crate::SharedEffect;

#[derive(Clone, Copy, Default)]
pub(super) struct Smooth {
    pub last: [f32; 2],
    from: [f32; 2],
    left: u32,
    total: u32,
}
impl Smooth {
    fn begin(&mut self, rate: u32) {
        self.from = self.last;
        self.total = (rate / 200).max(1); // 5 ms to suppress the switch discontinuity.
        self.left = self.total;
    }
    pub fn frame(&mut self, mut value: [f32; 2]) -> [f32; 2] {
        if self.left > 0 {
            let mix = self.left as f32 / self.total as f32;
            for (i, v) in value.iter_mut().enumerate() { *v += (self.from[i] - *v) * mix; }
            self.left -= 1;
        }
        self.last = value;
        value
    }
}

/// MIDI already delivered to an instance. Fixed storage: controller traffic never allocates.
pub(super) struct Midi {
    pub manager: Option<u8>,
    notes: [[u8; 128]; 16],
    released: [u128; 16],
    cc: [[u8; 120]; 16],
    bend: [Option<[u8; 3]>; 16],
    pressure: [Option<[u8; 3]>; 16],
    poly: [[u8; 128]; 16],
}
impl Default for Midi {
    fn default() -> Self {
        Self { manager: None, notes: [[0; 128]; 16], released: [0; 16], cc: [[255; 120]; 16],
            bend: [None; 16], pressure: [None; 16], poly: [[255; 128]; 16] }
    }
}
impl Midi {
    pub fn release(&mut self) {
        self.notes = [[0;128];16]; self.released = [0;16];
        self.poly = [[255;128];16];
        for cc in &mut self.cc { cc[64] = 0; }
    }
    pub fn sent(&mut self, b: [u8; 3]) {
        let c = (b[0] & 15) as usize;
        let k = (b[1] & 127) as usize;
        match b[0] & 0xf0 {
            0x90 if b[2] != 0 => { self.notes[c][k] = b[2]; self.released[c] &= !(1 << k); }
            0x80 | 0x90 => {
                let pedal = |ch: usize| self.cc[ch][64] != 255 && self.cc[ch][64] >= 64;
                if pedal(c) || self.manager.is_some_and(|m| pedal(m as usize)) { self.released[c] |= 1 << k; }
                else { self.notes[c][k] = 0; }
            }
            0xb0 => {
                if k < 120 { self.cc[c][k] = b[2]; }
                if k == 64 && b[2] < 64 {
                    // MPE manager pedal can hold voices on member channels.
                    let range = if self.manager == Some(c as u8) { 0..16 } else { c..c+1 };
                    for ch in range { for key in 0..128 {
                        if self.released[ch] >> key & 1 != 0 { self.notes[ch][key] = 0; }
                    } self.released[ch] = 0; }
                }
                if k == 120 { self.notes[c].fill(0); self.released[c] = 0; }
                if k == 123 { for key in 0..128 { self.sent([0x80 | c as u8, key as u8, 0]); } }
                if k == 121 {
                    self.cc[c].fill(255); self.bend[c] = None; self.pressure[c] = None; self.poly[c].fill(255);
                    for key in 0..128 { if self.released[c] >> key & 1 != 0 { self.notes[c][key] = 0; } }
                    self.released[c] = 0;
                }
            }
            0xe0 => self.bend[c] = Some(b),
            0xd0 => self.pressure[c] = Some(b),
            0xa0 => self.poly[c][k] = b[2],
            _ => {}
        }
    }
    fn replay(&self, out: &mut Vec<(usize, [u8; 3])>, row: usize) {
        for c in 0..16 {
            // Bank/program selectors would overwrite the newly chosen preset. RPN/NRPN
            // transactions cannot be reconstructed from individual last CC values.
            for k in 1..120 {
                if matches!(k, 6 | 32 | 38 | 96..=101) { continue; }
                if self.cc[c][k] != 255 { out.push((row, [0xb0 | c as u8, k as u8, self.cc[c][k]])); }
            }
            for b in [self.bend[c], self.pressure[c]].into_iter().flatten() { out.push((row, b)); }
        }
        // All managers' pedals are established before member-channel notes.
        for c in 0..16 {
            for k in 0..128 {
                if self.notes[c][k] == 0 { continue; }
                out.push((row, [0x90 | c as u8, k as u8, self.notes[c][k]]));
                if self.poly[c][k] != 255 { out.push((row, [0xa0 | c as u8, k as u8, self.poly[c][k]])); }
                if self.released[c] >> k & 1 != 0 { out.push((row, [0x80 | c as u8, k as u8, 0])); }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preset_midi_chases_sustain_expression_and_pending_release() {
        let mut midi = Midi { manager: Some(15), ..Midi::default() };
        for b in [[0xbf,64,127], [0xe1,12,70], [0xd1,90,0], [0x91,60,83], [0x81,60,0]] { midi.sent(b); }
        let mut out = vec![]; midi.replay(&mut out, 2);
        let bytes: Vec<_> = out.iter().map(|v| v.1).collect();
        let at = |b| bytes.iter().position(|v| *v == b).unwrap();
        assert!(at([0xbf,64,127]) < at([0x91,60,83]));
        assert!(at([0xe1,12,70]) < at([0x91,60,83]));
        assert!(at([0x91,60,83]) < at([0x81,60,0]));
        midi.sent([0xbf,64,0]); out.clear(); midi.replay(&mut out, 2);
        assert!(!out.iter().any(|v| v.1[0] & 0xf0 == 0x90));
        let mut ordinary = Midi::default(); ordinary.sent([0xb0,64,127]);
        ordinary.sent([0x91,60,83]); ordinary.sent([0x81,60,0]);
        out.clear(); ordinary.replay(&mut out, 0);
        assert!(!out.iter().any(|v| v.1[0] == 0x91), "ordinary channels have independent pedals");
        ordinary.sent([0x90,62,90]); ordinary.release(); out.clear(); ordinary.replay(&mut out,0);
        assert!(!out.iter().any(|v| v.1[0] & 0xf0 == 0x90), "stop/pause cannot resurrect notes");
    }

    #[test]
    fn preset_smoothing_is_continuous_and_finishes_in_five_ms() {
        let mut s = Smooth { last: [0.4,-0.4], ..Smooth::default() }; s.begin(48000);
        assert_eq!(s.frame([-0.4,0.4]), [0.4,-0.4]);
        for _ in 1..240 { let previous = s.last; let now = s.frame([-0.4,0.4]); assert!((now[0]-previous[0]).abs() < 0.004); }
        assert_eq!(s.frame([-0.4,0.4]), [-0.4,0.4]);
    }
}

#[derive(Default)]
pub(super) struct State {
    pub midi: Midi,
    pub outputs: [Smooth; 16],
}

impl Engine {
    /// Control-thread handoff at an engine boundary. The caller retains both instances
    /// until the UI can retire the old one; this method never loads plugin state.
    pub fn replace_instrument_preset(&mut self, row: usize, plugin: SharedInstrument) {
        if self.frozen_instrument_rows.contains(&row) || row >= self.channels.len() { return; }
        self.set_instrument(row, plugin);
        self.params.resend_channel(row);
        if let Some(t) = self.inst_tuning.get_mut(row) { *t = InstTuning::default(); }
        if let Some(v) = self.note_voices.get(row) { self.inst_missed.extend(v.restart().map(|n| (row, n))); }
        for s in self.inst_shapes.iter_mut().filter(|s| s.channel == row) { s.sent = None; }
        let mut replay = Vec::new();
        for (parent, p) in &self.mpe {
            if *parent == row || self.channels.get(*parent).is_some_and(|c| c.stack.contains(&row)) {
                replay.extend(p.config.setup().into_iter().map(|b| (row, b)));
            }
        }
        if let Some(s) = self.preset.get_mut(row) {
            s.midi.replay(&mut replay, row);
            for o in &mut s.outputs { o.begin(self.rate); }
        }
        // Pending note-offs must follow the chased notes, including an off received
        // while the UI was preparing the preset.
        replay.append(&mut self.inst_midi);
        self.inst_midi = replay;
        self.tail_done = false;
    }

    pub fn replace_effect_preset(&mut self, track: usize, slot: usize, plugin: SharedEffect) {
        if let Some(fx) = self.tracks.get_mut(track).and_then(|t| t.effects.iter_mut().find(|f| f.slot == slot)) {
            fx.fx = Fx::Hosted(plugin);
            fx.fresh = false;
            fx.smooth.begin(self.rate);
            // Keep slot bypass, mix, ports, sidechain and delay-pad histories.
            self.pdc.due = true;
            self.params.resend_slot(track, slot);
        } else { self.set_effect(track, slot, plugin); }
    }
}
