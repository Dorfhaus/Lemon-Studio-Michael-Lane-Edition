//! Conservative suspension for plugins that report a finite tail. Exact silence
//! preserves sample equality and keeps noise generators and decaying tails awake.
use os_engine::TransportInfo;

#[derive(Default)]
pub(crate) struct SmartDisable {
    enabled: bool,
    quiet_frames: u64,
    rate: u32,
    song: Option<TransportInfo>,
    frames: usize,
    midi_notes: [u128; 16],
    // Sustain (low 16 bits) and sostenuto (high 16), one bit per MIDI channel.
    sustain: u32,
}
impl SmartDisable {
    pub fn enabled(&self) -> bool {
        self.enabled
    }
    pub fn set_enabled(&mut self, enabled: bool) {
        if self.enabled != enabled {
            self.wake();
            self.enabled = enabled;
        }
    }
    pub fn wake(&mut self) {
        self.quiet_frames = 0;
    }
    pub fn reset(&mut self) {
        self.wake();
        self.midi_notes.fill(0);
        self.sustain = 0;
    }
    pub fn midi(&mut self, bytes: [u8; 3]) {
        self.wake();
        let channel = usize::from(bytes[0] & 15);
        let bit = 1u128 << (bytes[1] & 127);
        match bytes[0] & 0xF0 {
            0x90 if bytes[2] != 0 => self.midi_notes[channel] |= bit,
            0x80 | 0x90 => self.midi_notes[channel] &= !bit,
            0xB0 => match bytes[1] {
                64 | 66 => {
                    let pedal = channel + if bytes[1] == 66 { 16 } else { 0 };
                    if bytes[2] >= 64 {
                        self.sustain |= 1 << pedal;
                    } else {
                        self.sustain &= !(1 << pedal);
                    }
                }
                120 => {
                    self.midi_notes[channel] = 0;
                    self.sustain &= !(0x10001 << channel);
                }
                121 => self.sustain &= !(0x10001 << channel),
                123 => self.midi_notes[channel] = 0,
                _ => {}
            },
            _ => {}
        }
    }
    pub fn transport(&mut self, info: &TransportInfo) {
        if self.song.is_none_or(|old| {
            old.playing != info.playing
                || old.tempo != info.tempo
                || old.numerator != info.numerator
                || old.denominator != info.denominator
                || old.looping != info.looping
                || (info.frame != old.frame.saturating_add(self.frames as u64)
                    && (old.playing || info.frame != old.frame))
        }) {
            self.wake();
        }
        self.song = Some(*info);
    }
    pub fn skip(&mut self, rate: u32, frames: usize, quiet_input: bool, tail: Option<u64>) -> bool {
        if self.rate != rate {
            self.wake();
            self.rate = rate;
        }
        self.frames = frames;
        if !quiet_input
            || tail.is_none()
            || self.sustain != 0
            || self.midi_notes.iter().any(|&n| n != 0)
        {
            self.wake();
            return false;
        }
        self.enabled
            && quiet_input
            && tail
                .is_some_and(|tail| self.quiet_frames >= tail.saturating_add(u64::from(rate) * 4))
    }
    pub fn rendered(&mut self, frames: usize, quiet: bool) {
        if quiet {
            self.quiet_frames = self.quiet_frames.saturating_add(frames as u64);
        } else {
            self.wake();
        }
    }
}

pub(crate) fn silent(left: &[f32], right: &[f32]) -> bool {
    left.iter().chain(right).all(|&sample| sample == 0.0)
}

#[cfg(test)]
mod tests;
