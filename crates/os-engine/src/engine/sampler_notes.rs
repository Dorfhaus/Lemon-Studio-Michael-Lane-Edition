//! Note generation for the sampler Misc page, on the same tick clock as the Piano Roll.
use super::Trigger;
use crate::{laws, NoteLevels};
use os_model::{Arpeggio, ChannelMisc, Echo, Tracking};

const MAX_HELD: usize = 128;

#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct Settings {
    pub tracking: [Tracking; 2],
    pub arp: Arpeggio,
    pub echo: Echo,
}

impl Settings {
    pub fn of(misc: &ChannelMisc) -> Option<Self> {
        let settings = Self { tracking: misc.tracking, arp: misc.arp, echo: misc.echo };
        let arp = (1..=5).contains(&settings.arp.direction);
        let echo = settings.echo.echoes != 0 && (settings.echo.feed > 0.0 || settings.echo.fat);
        let tracking = settings.tracking.iter().any(|t| t.pan != 0.0 || t.mod_x != 0.0 || t.mod_y != 0.0);
        (arp || echo || tracking).then_some(settings)
    }

    pub fn track(self, note: &mut Trigger) {
        let [velocity, key] = self.tracking;
        let v = (f32::from(note.velocity) - f32::from(velocity.mid)) / 128.0;
        let k = (note.key as f32 - f32::from(key.mid)) / 127.0;
        let apply = |level: f32, velocity: f32, key: f32| {
            (level + v * velocity + k * key).clamp(-1.0, 1.0)
        };
        note.levels.pan = apply(note.levels.pan, velocity.pan, key.pan);
        note.levels.mod_x = apply(note.levels.mod_x, velocity.mod_x, key.mod_x);
        note.levels.mod_y = apply(note.levels.mod_y, velocity.mod_y, key.mod_y);
        note.pan = f64::from(note.levels.pan);
    }

    pub fn echoes(self, note: Trigger, ppq: u16) -> impl Iterator<Item = (u64, Trigger)> {
        let echo = self.echo;
        let count = if echo.feed > 0.0 || echo.fat { echo.echoes.min(10) } else { 0 };
        let spacing = if echo.fat { 0.035 * f64::from(ppq) } else { 2.0 * f64::from(echo.time) * f64::from(ppq) };
        let feedback = if echo.fat && echo.feed == 0.0 { 0.5 } else { echo.feed * 2.0 };
        (1..=count).filter_map(move |i| {
            let gain = feedback.powi(i as i32);
            let velocity = (f32::from(note.velocity) * gain).round().clamp(0.0, 128.0) as u8;
            if velocity == 0 { return None; }
            let step = i as f32;
            let side = if i % 2 == 0 { -1.0 } else { 1.0 };
            let mut pan = note.levels.pan + step * echo.pan;
            if echo.fat {
                pan += side * 0.85;
            } else if echo.ping_pong {
                let spread = if echo.pan == 0.0 { 0.6 } else { step * echo.pan.abs() };
                pan = side * (note.levels.pan.abs() + spread).clamp(0.0, 1.0)
                    * if echo.pan < 0.0 { -1.0 } else { 1.0 };
            }
            let pitch = (12.0 * step * echo.pitch + if echo.fat { 0.09 * side } else { 0.0 }).clamp(-48.0, 48.0);
            let levels = NoteLevels {
                pan: pan.clamp(-1.0, 1.0),
                mod_x: (note.levels.mod_x + step * echo.mod_x).clamp(-1.0, 1.0),
                mod_y: (note.levels.mod_y + step * echo.mod_y).clamp(-1.0, 1.0),
                echo_pitch: pitch,
                ..note.levels
            };
            let delay = (spacing * f64::from(i)).round().max(u64::from(i) as f64) as u64;
            Some((delay, Trigger {
                velocity,
                vel: laws::velocity_gain(velocity),
                pan: f64::from(levels.pan),
                rate: note.rate * 2f64.powf(f64::from(pitch) / 12.0),
                levels,
                // Echoes may overlap the original even when Cut self is enabled.
                cuts: Some((0, 0)),
                porta: false,
                ..note
            }))
        })
    }
}

#[derive(Debug)]
struct Held {
    note: Trigger,
    expires: Option<u64>,
    live: bool,
}

#[derive(Debug, Default)]
pub(super) struct Arp {
    held: Vec<Held>,
    step: u64,
    next: u64,
    previous: Option<u16>,
}

impl Arp {
    pub fn active(&self) -> bool { !self.held.is_empty() }

    pub fn add(&mut self, note: Trigger, now: u64, duration: Option<u64>, live: bool) {
        self.expire(now);
        if !self.active() {
            (self.step, self.next, self.previous) = (0, now, None);
        }
        if live {
            self.remove_live(note.key);
        }
        if self.held.len() == MAX_HELD {
            self.held.remove(0);
        }
        self.held.push(Held { note, expires: duration.map(|n| now + n), live });
    }

    pub fn remove_live(&mut self, key: u16) -> bool {
        let before = self.held.len();
        self.held.retain(|h| !h.live || h.note.key != key);
        self.held.len() != before
    }

    pub fn clear_sequence(&mut self) {
        self.held.retain(|h| h.live);
    }

    pub fn expire(&mut self, now: u64) {
        self.held.retain(|h| h.expires.is_none_or(|end| end > now));
    }

    pub fn next(&mut self, arp: Arpeggio, now: u64, ppq: u16, random: &mut super::layers::Mt19937) -> Option<(Trigger, u64, bool)> {
        self.expire(now);
        if !self.active() || now < self.next {
            return None;
        }
        let mut notes = [None; 128];
        for h in &self.held {
            notes[usize::from(h.note.key.min(127))] = Some(&h.note);
        }
        let root = *notes.iter().flatten().next().copied()?;
        let mut keys = [0u16; 128];
        let count = if arp.chord < 0 {
            let mut count = 0;
            for note in notes.iter().flatten() {
                keys[count] = note.key.min(127);
                count += 1;
            }
            count
        } else {
            keys[0] = root.key.min(127);
            let mut count = 1;
            if let Some(chord) = os_model::chords::entry(arp.chord as usize) {
                for &interval in chord.intervals {
                    let key = keys[count - 1] + u16::from(interval);
                    if key > 127 { break; }
                    keys[count] = key;
                    count += 1;
                }
            }
            count
        };
        let repeat = arp.repeat.clamp(1, 6) as usize;
        let span = count * repeat * arp.range.clamp(1, 6) as usize;
        let step = self.step as usize;
        let pos = match (arp.direction, span) {
            (_, 1) => 0,
            (2, _) => span - 1 - step % span,
            (3, _) => {
                let i = step % (2 * span - 2);
                i.min(2 * span - 2 - i)
            }
            (4, _) => {
                let i = step % (2 * span);
                i.min(2 * span - 1 - i)
            }
            (5, _) => super::layers::place(random.next_u32(), span),
            _ => step % span,
        };
        let index = pos % count;
        let octave = if arp.range == 0 { index } else { pos / (count * repeat) };
        let key = (keys[index] + 12 * octave as u16).min(127);
        let source = if arp.chord < 0 { notes[usize::from(keys[index])].copied().unwrap_or(root) } else { root };
        let hold = arp.time >= 1.0;
        let spacing = (f64::from(ppq) / 4.0 * 2f64.powf(6.0 * f64::from(arp.time - Arpeggio::TIME_DEFAULT)))
            .round().max(1.0) as u64;
        self.next = if hold { u64::MAX } else { now + spacing };
        self.step = self.step.wrapping_add(1);
        let slide = arp.slide && self.previous.is_some_and(|previous| previous != key);
        self.previous = Some(key);
        let gate = (spacing as f64 * f64::from(arp.gate.clamp(0.0, 1.0))).round().max(1.0) as u64;
        let remaining = self.held.iter().map(|h| h.expires.unwrap_or(u64::MAX)).max().unwrap_or(now + 1).saturating_sub(now);
        let duration = if hold { remaining } else { gate.min(remaining).max(1) };
        Some((Trigger { key, rate: source.rate * 2f64.powf((f64::from(key) - f64::from(source.key)) / 12.0),
            buf: if key == source.key { source.buf } else { 0 }, ..source }, duration, slide))
    }
}

#[derive(Debug, Clone, Copy)]
pub(super) struct Tap {
    pub note: Trigger,
    pub duration: u64,
}

#[derive(Debug, Clone, Copy)]
pub(super) struct Release {
    pub due: u64,
    pub channel: usize,
    pub key: u8,
    pub id: u32,
    pub choke: bool,
    pub echo: bool,
    pub live_key: Option<u16>,
    pub arp_hold: bool,
}
