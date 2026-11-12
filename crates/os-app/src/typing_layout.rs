// SPDX-License-Identifier: GPL-3.0-or-later
use crate::{
    chords,
    harmony::{Harmony, ROOTS},
    playlist::{Cmd, Mark, PItem, Popup},
};

use winit::keyboard::KeyCode;

pub const ENABLED: &str = "typing_keyboard_enabled";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Change {
    Root(u8),
    Scale(usize),
    Follow,
}

#[derive(Default, Clone, Copy, serde::Deserialize, serde::Serialize)]
pub struct Layout {
    independent: bool,
    root: u8,
    scale: usize, // 0 is chromatic, 67..=88 are the named scales in the chord library.
}

impl Layout {
    pub fn load() -> Self {
        crate::prefs::text("typing_keyboard_layout")
            .and_then(|s| serde_json::from_str::<Self>(&s).ok())
            .filter(|s| s.root < 12 && (s.scale == 0 || (67..=88).contains(&s.scale)))
            .unwrap_or_default()
    }
    pub fn set(&mut self, change: Change) {
        match change {
            Change::Root(root) if root < 12 => self.root = root,
            Change::Scale(scale) if scale == 0 || (67..=88).contains(&scale) => self.scale = scale,
            Change::Follow => {}
            _ => return,
        }
        self.independent = change != Change::Follow;
        if let Ok(value) = serde_json::to_string(self) {
            crate::prefs::set_text("typing_keyboard_layout", &value);
        }
    }
    fn scale(self, harmony: &Harmony) -> Option<(u8, usize)> {
        if self.independent {
            (self.scale != 0).then_some((self.root, self.scale))
        } else {
            harmony.typing_snap.then_some((harmony.root, harmony.scale))
        }
    }

    pub fn is_scaled(self, harmony: &Harmony) -> bool {
        self.scale(harmony).is_some()
    }

    pub fn note(self, code: KeyCode, octave: i32, harmony: &Harmony) -> Option<u16> {
        let base = 48 + octave * 12;
        let note = if let Some((root, scale)) = self.scale(harmony) {
            let (row, degree) = scale_position(code)?;
            let classes = chords::classes(scale, 0);
            let mut degrees = (0..12).filter(|&i| classes[i]);
            let count = degrees.clone().count();
            let offset = degrees.nth(degree % count)? + 12 * (degree / count);
            base + i32::from(root) + 12 * row + offset as i32
        } else {
            base + typing_semitone(code)? + if self.independent { i32::from(self.root) } else { 0 }
        };
        (0..128).contains(&note).then_some(note as u16)
    }

    pub fn menu(self, x: f32, y: f32) -> Popup {
        let item = |name: &str, change, checked| {
            PItem::new(name.to_string(), Cmd::TypingLayout(change)).mark(Mark::Check(checked))
        };
        let scale_name = |scale| {
            if scale == 0 {
                "Chromatic"
            } else {
                chords::label(scale)
            }
        };
        let roots = ROOTS
            .iter()
            .enumerate()
            .map(|(i, name)| item(name, Change::Root(i as u8), i == self.root as usize))
            .collect();
        let scales = std::iter::once(0)
            .chain(67..=88)
            .map(|i| item(scale_name(i), Change::Scale(i), i == self.scale))
            .collect();
        Popup {
            x,
            y,
            items: vec![
                PItem::sub(format!("Key: {}", ROOTS[self.root as usize]), roots),
                PItem::sub(format!("Scale: {}", scale_name(self.scale)), scales),
                PItem::sep(),
                item(
                    "Follow Piano Roll typing snap",
                    Change::Follow,
                    !self.independent,
                ),
            ],
            sub: None,
            scroll: [0; 2],
            cascade: Vec::new(),
        }
    }
}

// Use physical positions so the same rows work on QWERTY and QWERTZ keyboards.
const SCALE_ROWS: &[&[KeyCode]] = {
    use KeyCode::*;
    &[
        &[KeyZ, KeyX, KeyC, KeyV, KeyB, KeyN, KeyM, Comma, Period, Slash],
        &[KeyA, KeyS, KeyD, KeyF, KeyG, KeyH, KeyJ, KeyK, KeyL, Semicolon, Quote],
        &[KeyQ, KeyW, KeyE, KeyR, KeyT, KeyY, KeyU, KeyI, KeyO, KeyP, BracketLeft, BracketRight],
    ]
};

fn scale_position(code: KeyCode) -> Option<(i32, usize)> {
    SCALE_ROWS.iter().enumerate().find_map(|(row, keys)| {
        keys.iter().position(|&key| key == code).map(|degree| (row as i32, degree))
    })
}

/// Semitone above the lower tier's first C (C4) for a physical key (spec: ui/typing-keyboard.md).
fn typing_semitone(code: KeyCode) -> Option<i32> {
    use KeyCode::*;
    Some(match code {
        // Lower tier: white keys on the bottom row, black keys on the row above.
        KeyZ => 0, KeyS => 1, KeyX => 2, KeyD => 3, KeyC => 4, KeyV => 5, KeyG => 6, KeyB => 7,
        KeyH => 8, KeyN => 9, KeyJ => 10, KeyM => 11, Comma => 12, KeyL => 13, Period => 14,
        Semicolon => 15, Slash => 16,
        // Upper tier, one octave up.
        KeyQ => 12, Digit2 => 13, KeyW => 14, Digit3 => 15, KeyE => 16, KeyR => 17, Digit5 => 18,
        KeyT => 19, Digit6 => 20, KeyY => 21, Digit7 => 22, KeyU => 23, KeyI => 24, Digit9 => 25,
        KeyO => 26, Digit0 => 27, KeyP => 28, BracketLeft => 29, Equal => 30, BracketRight => 31,
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn selected_scale_plays_every_letter_position_and_remembers_typing() {
        use crate::app::{App, Win};
        let mut app = App::new(None);
        app.state.project = crate::app::test_project();
        app.state.z = vec![Win::Playlist];
        assert!(!app.typing_on());
        app.toggle_typing();
        app.typing_layout.set(Change::Root(2));
        app.typing_layout.set(Change::Scale(73));
        let allowed = chords::classes(73, 2);
        for (row, codes) in SCALE_ROWS.iter().enumerate() {
            let mut previous = None;
            for &code in *codes {
                assert!(app.typing_key(code, true, false, false), "{code:?}");
                let note = app.state.piano.typed.last().unwrap().1;
                assert!(allowed[(note % 12) as usize], "{code:?} => {note}");
                if let Some(previous) = previous {
                    assert!(note > previous);
                } else {
                    assert_eq!(note, 50 + row as i32 * 12);
                }
                previous = Some(note);
                assert!(app.typing_key(code, false, false, false));
            }
        }
        assert!(App::new(None).typing_on());
        app.toggle_typing();
        assert!(!App::new(None).typing_on());
    }

    #[test]
    fn toolbar_typing_layout_is_independent_and_persistent() {
        use crate::app::{Action, App};
        use winit::keyboard::KeyCode;
        let mut app = App::new(None);
        app.state.project = crate::app::test_project();
        if !app.typing_on() {
            app.toggle_typing();
        }
        let mut canvas = os_ui::Canvas::new(1920, 1080, 1.).unwrap();
        app.paint(&mut canvas, None);
        let r = app
            .hits
            .iter()
            .find(|h| h.action == Action::Typing)
            .unwrap()
            .rect;
        app.mouse_move(r.cx(), r.cy());
        app.mouse_down_right();
        let popup = app.state.playlist.popup.take().unwrap();
        assert!(popup.items[0].label.starts_with("Key:"));
        let harmony = &mut app.state.piano.harmony;
        harmony.typing_snap = true;
        harmony.root = 0;
        harmony.scale = 67;
        crate::playlist::run(&mut app, Cmd::TypingLayout(Change::Root(2)));
        crate::playlist::run(&mut app, Cmd::TypingLayout(Change::Scale(73)));
        assert!(app.typing_key(KeyCode::KeyZ, true, false, false));
        assert_eq!(app.state.piano.typed.last().unwrap().1, 50);
        assert_eq!(Layout::load().note(KeyCode::KeyX, 0, &app.state.piano.harmony), Some(52));
        assert_eq!(app.state.piano.harmony.root, 0);
        assert_eq!(app.state.piano.harmony.scale, 67);
        app.typing_layout.set(Change::Follow);
        assert!(app.typing_key(KeyCode::KeyZ, false, false, false));
        assert_eq!(Layout::load().note(KeyCode::KeyX, 0, &app.state.piano.harmony), Some(50));
    }
}
