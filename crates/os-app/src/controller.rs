// SPDX-License-Identifier: GPL-3.0-or-later
//! Per-input control-surface modes. MCU note numbers follow the Control Surface
//! library's published MCU constants. Select MCU on the controller's DAW port.
use crate::app::{Action, App};
use crate::midi_in::{Ev, Msg};
use crate::playlist::{Cmd, Mark, PItem, Popup};

pub const VISIBILITY: &str = "visible_midi-controller-setup";
pub const MODES: &[&str] = &[
    "Generic MIDI / MIDI Learn",
    "Mackie Control (MCU)",
    "Komplete Kontrol DAW (MCU)",
    "Arturia KeyLab DAW (MCU)",
    "Behringer X-Touch (MCU)",
    "Korg nanoKONTROL2 (factory CC)",
];
fn key(device: &str) -> String {
    format!("midi_surface:{}", device.replace('=', "_"))
}
pub fn mode(device: &str) -> usize {
    crate::settings_file::get(&key(device)).and_then(|v| v.parse().ok())
        .filter(|&i| i < MODES.len()).unwrap_or(0)
}
pub fn set_mode(device: &str, value: usize) {
    if value < MODES.len() { crate::settings_file::write(&key(device), &value.to_string()); }
}
pub fn label(device: &str) -> &'static str { MODES[mode(device)] }

impl App {
    pub(crate) fn controller_menu(&mut self, x: f32, y: f32) {
        let Some(device) = self.state.midi.selected_input().map(|d| d.name.clone()) else { return };
        let current = mode(&device);
        let items = MODES.iter().enumerate().map(|(i, label)|
            PItem::new(*label, Cmd::Controller(device.clone(), i)).mark(Mark::Radio(i == current))).collect();
        self.state.playlist.popup = Some(Popup { x, y, items, sub: None, scroll: [0; 2], cascade: Vec::new() });
        self.invalidate();
    }
    pub(crate) fn controller_event(&mut self, ev: &Ev) -> bool {
        let profile = mode(&ev.dev);
        if profile == 0 { return false; }
        let bank = *self.midi_play.surface_banks.entry(ev.dev.to_string()).or_default();
        let track = |i: u8| bank + u16::from(i) + 1;
        if profile == 5 {
            if let Msg::Cc { n, v, .. } = ev.msg {
                match n {
                    0..=7 => self.surface_fader(track(n), f32::from(v) / 127.0),
                    16..=23 => self.surface_pan(track(n - 16), f32::from(v) / 127.0),
                    32..=39 if v >= 64 => { self.surface_solo(track(n - 32)); }
                    48..=55 if v >= 64 => self.surface_button(track(n - 48), false),
                    64..=71 if v >= 64 => self.surface_button(track(n - 64), true),
                    41 if v >= 64 => self.surface_play(),
                    42 if v >= 64 => self.act(Action::Stop),
                    45 if v >= 64 => self.act(Action::Record),
                    58 if v >= 64 => self.surface_bank(&ev.dev, -8),
                    59 if v >= 64 => self.surface_bank(&ev.dev, 8),
                    32..=39 | 48..=55 | 64..=71 | 41 | 42 | 45 | 58 | 59 => {},
                    _ => return false,
                }
                return true;
            }
            return false;
        }
        match ev.msg {
            Msg::On { ch: 0, key, vel: 1..=127 } => match key {
                0..=7 => self.surface_button(track(key), true),
                8..=15 => self.surface_solo(track(key - 8)),
                16..=23 => self.surface_button(track(key - 16), false),
                24..=31 => {
                    let t = track(key - 24);
                    if t <= crate::mixer::inserts(&self.state) { crate::mixer::select(&mut self.state, t, false); }
                }
                0x2e => self.surface_bank(&ev.dev, -8),
                0x2f => self.surface_bank(&ev.dev, 8),
                0x30 => self.surface_bank(&ev.dev, -1),
                0x31 => self.surface_bank(&ev.dev, 1),
                0x5d => self.act(Action::Stop),
                0x5e => self.surface_play(),
                0x5f => self.act(Action::Record),
                _ => {}
            },
            Msg::Bend { ch: ch @ 0..=8, v } => {
                self.surface_fader(if ch == 8 { 0 } else { track(ch) }, f32::from(v) / 16383.0);
            }
            Msg::Cc { ch: 0, n: n @ 16..=23, v } => {
                let t = track(n - 16);
                let delta = f32::from(v & 63) / 127.0 * if v & 64 != 0 { -1.0 } else { 1.0 };
                let current = crate::history::knob_value(&self.state, crate::knobs::KnobId::MixPan(t)) as f32;
                self.surface_pan(t, current + delta);
            }
            _ => {}
        }
        // A dedicated MCU port carries control messages, including releases and
        // unsupported buttons. Never send these to the selected instrument.
        self.invalidate();
        true
    }
    fn surface_play(&mut self) {
        if self.transport.state() != crate::transport::State::Playing { self.act(Action::Play); }
    }
    fn surface_bank(&mut self, device: &str, delta: i32) {
        let bank = self.midi_play.surface_banks.entry(device.to_string()).or_default();
        *bank = (i32::from(*bank) + delta).clamp(0, i32::from(crate::mixer::inserts(&self.state).saturating_sub(1))) as u16;
        self.state.message = Some(format!("Controller mixer bank starts at Insert {}", *bank + 1));
    }
    fn surface_fader(&mut self, track: u16, value: f32) {
        if track > crate::mixer::inserts(&self.state) || crate::mixer::locked(&self.state, track) { return; }
        crate::history::fader(&mut self.state, track, true);
        if let Some(t) = crate::mixer::track_mut(&mut self.state, track) { t.volume = Some(f64::from(value.clamp(0.0, 1.0))); }
        self.sync_live();
        self.invalidate();
    }
    fn surface_pan(&mut self, track: u16, value: f32) {
        if track > crate::mixer::inserts(&self.state) || crate::mixer::locked(&self.state, track) { return; }
        let knob = crate::knobs::KnobId::MixPan(track);
        crate::history::knob(&mut self.state, knob, true);
        self.state.set_knob(knob, value.clamp(0.0, 1.0));
        self.record_knob(knob);
        self.sync_live();
        self.invalidate();
    }
    fn surface_button(&mut self, track: u16, arm: bool) {
        if track > crate::mixer::inserts(&self.state) || crate::mixer::locked(&self.state, track) { return; }
        self.act(Action::Mix(if arm { crate::mixer::MHit::Disk(track) } else { crate::mixer::MHit::Led(track) }));
    }
    fn surface_solo(&mut self, track: u16) {
        if track > crate::mixer::inserts(&self.state) || crate::mixer::locked(&self.state, track) { return; }
        crate::mixer::run(self, crate::mixer::MCmd::Solo(track));
        self.sync_live();
        self.invalidate();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn event(device: &str, msg: Msg) -> Ev {
        Ev { dev: std::sync::Arc::from(device), msg }
    }
    #[test]
    fn controller_ports_keep_keys_separate_and_mcu_releases_do_not_play_notes() {
        let mut app = App::new(None);
        app.state.project = crate::app::test_project();
        set_mode("Controller DAW", 1);
        let note = Msg::On { ch: 0, key: 60, vel: 100 };
        assert!(!app.controller_event(&event("Controller MIDI", note)));
        assert!(app.controller_event(&event("Controller DAW", note)));
        app.midi_events(vec![event("Controller DAW", note)]);
        assert_eq!(app.midi_play.lit().count(), 0);
        app.midi_events(vec![event("Controller MIDI", note)]);
        assert_eq!(app.midi_play.lit().count(), 1);
        app.midi_events(vec![event("Controller MIDI", Msg::Off { ch: 0, key: 60, vel: 0 })]);
        assert_eq!(app.midi_play.lit().count(), 0);
        assert_eq!(mode("Controller DAW"), 1);
        assert_eq!(mode("Controller MIDI"), 0);
    }
    #[test]
    fn controller_korg_factory_fader_and_button_release() {
        let mut app = App::new(None);
        app.state.project = crate::app::test_project();
        set_mode("Korg test", 5);
        app.midi_events(vec![event("Korg test", Msg::Cc { ch: 0, n: 0, v: 127 })]);
        assert_eq!(crate::mixer::track(&app.state, 1).unwrap().volume, Some(1.0));
        let enabled = crate::mixer::track(&app.state, 1).unwrap().enabled;
        app.midi_events(vec![event("Korg test", Msg::Cc { ch: 0, n: 48, v: 127 }),
                            event("Korg test", Msg::Cc { ch: 0, n: 48, v: 0 })]);
        assert_eq!(crate::mixer::track(&app.state, 1).unwrap().enabled, !enabled);
    }
    #[test]
    fn controller_faders_respect_locks_and_banks_are_per_device() {
        let mut app = App::new(None);
        app.state.project = crate::app::test_project();
        set_mode("Surface A", 1);
        set_mode("Surface B", 1);
        app.controller_event(&event("Surface A", Msg::Bend { ch: 0, v: 16383 }));
        assert_eq!(crate::mixer::track(&app.state, 1).unwrap().volume, Some(1.0));
        crate::mixer::run(&mut app, crate::mixer::MCmd::ToggleLock(1));
        app.controller_event(&event("Surface A", Msg::Bend { ch: 0, v: 0 }));
        assert_eq!(crate::mixer::track(&app.state, 1).unwrap().volume, Some(1.0));
        app.controller_event(&event("Surface A", Msg::On { ch: 0, key: 0x2f, vel: 127 }));
        app.controller_event(&event("Surface B", Msg::Off { ch: 0, key: 0x2f, vel: 0 }));
        assert!(app.midi_play.surface_banks["Surface A"] > 0);
        assert_eq!(app.midi_play.surface_banks["Surface B"], 0);
    }
}
