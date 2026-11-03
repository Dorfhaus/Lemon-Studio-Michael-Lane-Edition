// SPDX-License-Identifier: GPL-3.0-or-later
use os_ui::{Canvas, Color, Font, Rect};
use serde::{Deserialize, Serialize};
#[derive(Clone, Serialize, Deserialize)]
pub struct Entry {
    pub group: String,
    pub name: String,
    pub path: std::path::PathBuf,
    pub root: f32,
}
#[derive(Default)]
pub struct Browser {
    pub oscillator: Option<usize>,
    kind: usize,
    group: usize,
    gpage: usize,
    page: usize,
    entries: Vec<Entry>,
    pub message: String,
}
pub enum Choice {
    Basic(usize),
    Bank(usize),
    Sample(Entry),
}
pub const BASIC: [&str; 6] = [
    "Sine",
    "Saw",
    "Square",
    "Triangle",
    "Pulse 25%",
    "Pulse 12.5%",
];
pub fn button(o: usize) -> Rect {
    Rect::new(214. + 410. * o as f32, 112., 188., 25.)
}
fn tab(i: usize) -> Rect {
    Rect::new(70. + i as f32 * 220., 146., 208., 34.)
}
fn group_row(i: usize) -> Rect {
    Rect::new(70., 208. + i as f32 * 34., 280., 30.)
}
fn row(i: usize) -> Rect {
    Rect::new(370., 208. + i as f32 * 34., 730., 30.)
}
const CLOSE: Rect = Rect::new(1000., 98., 100., 30.);
const GP: Rect = Rect::new(70., 636., 130., 30.);
const GN: Rect = Rect::new(210., 636., 140., 30.);
const PREV: Rect = Rect::new(370., 636., 130., 30.);
const NEXT: Rect = Rect::new(510., 636., 130., 30.);
pub fn catalogue() -> std::path::PathBuf {
    std::env::var_os("LOCALAPPDATA")
        .or_else(|| std::env::var_os("HOME"))
        .map(std::path::PathBuf::from)
        .unwrap_or_default()
        .join("Lemon Studio/Lemon Wave/library.json")
}
impl Browser {
    pub fn open(&mut self, o: usize) {
        self.oscillator = Some(o);
        self.entries.clear();
        self.message.clear();
        match std::fs::read(catalogue()) {
            Ok(bytes) if bytes.len() <= 4 * 1024 * 1024 => {
                match serde_json::from_slice::<Vec<Entry>>(&bytes) {
                    Ok(e) => {
                        self.entries = e;
                        self.entries
                            .sort_by(|a, b| (&a.group, &a.name).cmp(&(&b.group, &b.name)));
                    }
                    Err(_) => self.message = "Cannot read My Samples catalogue".into(),
                }
            }
            _ => {
                self.message =
                    "My Samples catalogue is empty. Basic Waves and Wavetables are ready.".into()
            }
        };
        self.group = 0;
        self.gpage = 0;
        self.page = 0;
    }
    fn groups(&self) -> Vec<String> {
        let mut g: Vec<_> = self.entries.iter().map(|e| e.group.clone()).collect();
        g.sort();
        g.dedup();
        g
    }
    pub fn click(&mut self, x: f32, y: f32) -> Option<Choice> {
        if CLOSE.contains(x, y) {
            self.oscillator = None;
            return None;
        }
        for i in 0..3 {
            if tab(i).contains(x, y) {
                self.kind = i;
                self.page = 0;
                return None;
            }
        }
        let groups = self.groups();
        let list: Vec<_> = self
            .entries
            .iter()
            .filter(|e| groups.get(self.group) == Some(&e.group))
            .cloned()
            .collect();
        if self.kind == 2 {
            for i in 0..12 {
                if group_row(i).contains(x, y) && self.gpage * 12 + i < groups.len() {
                    self.group = self.gpage * 12 + i;
                    self.page = 0;
                    return None;
                }
            }
            if GP.contains(x, y) {
                self.gpage = self.gpage.saturating_sub(1)
            }
            if GN.contains(x, y) && (self.gpage + 1) * 12 < groups.len() {
                self.gpage += 1
            }
            if PREV.contains(x, y) {
                self.page = self.page.saturating_sub(1)
            }
            if NEXT.contains(x, y) && (self.page + 1) * 12 < list.len() {
                self.page += 1
            }
        }
        for i in 0..12 {
            if row(i).contains(x, y) {
                return match self.kind {
                    0 if i < BASIC.len() => Some(Choice::Basic(i)),
                    1 if i < 4 => Some(Choice::Bank(i)),
                    2 => list.get(self.page * 12 + i).cloned().map(Choice::Sample),
                    _ => None,
                };
            }
        }
        None
    }
    pub fn paint(&self, c: &mut Canvas) {
        let Some(o) = self.oscillator else { return };
        let ink = Color::rgb(0xe8edf3);
        let dim = Color::rgb(0x9ba9ba);
        let accent = Color::rgb(0xd3ed76);
        let f = Font::light(13, 0).natural();
        c.fill_rect(Rect::new(42., 80., 1096., 664.), Color::rgb(0x19212c));
        c.text(
            f.bold(),
            70.,
            105.,
            ink,
            &format!(
                "SOUND BROWSER  /  OSCILLATOR {}",
                if o == 0 { "A" } else { "B" }
            ),
        );
        let mut b = |r: Rect, t: &str, selected: bool| {
            c.fill_rrect(
                r,
                4.,
                if selected {
                    Color::rgb(0x3b4931)
                } else {
                    Color::rgb(0x263242)
                },
            );
            c.text_clip(
                f,
                r.x + 9.,
                r.y + 8.,
                r.w - 18.,
                if selected { accent } else { ink },
                t,
            );
        };
        b(CLOSE, "Close", false);
        for (i, t) in ["Basic Waves", "Wavetables", "My Samples"]
            .iter()
            .enumerate()
        {
            b(tab(i), t, self.kind == i)
        }
        if self.kind == 2 {
            let groups = self.groups();
            for (i, g) in groups.iter().skip(self.gpage * 12).take(12).enumerate() {
                b(group_row(i), g, self.group == self.gpage * 12 + i)
            }
            let list: Vec<_> = self
                .entries
                .iter()
                .filter(|e| groups.get(self.group) == Some(&e.group))
                .collect();
            for (i, e) in list.iter().skip(self.page * 12).take(12).enumerate() {
                b(
                    row(i),
                    &format!("{}   |   estimated root {:.1}", e.name, e.root),
                    false,
                )
            }
            b(GP, "< Groups", false);
            b(GN, "Groups >", false);
            b(PREV, "< Sounds", false);
            b(NEXT, "Sounds >", false);
        } else {
            let labels: &[&str] = if self.kind == 0 {
                &BASIC
            } else {
                &[
                    "Basic - Sine / Saw / Square morph",
                    "Hollow - odd harmonics",
                    "Glass - bright harmonics",
                    "Vowel - formant sweep",
                ]
            };
            for (i, t) in labels.iter().enumerate() {
                b(row(i), t, false)
            }
        }
        c.text(f,70.,690.,dim,if self.kind==2{"Recorded notes: play once, follow keyboard pitch. Root pitch is estimated; adjust it on Synth."}else{"Select a sound to load it. Wavetable Position morphs frames; Import WAV loads your own tables."});
        c.text_clip(f, 70., 716., 1020., dim, &self.message);
    }
}
pub fn basic(index: usize) -> crate::wavetable::Source {
    let samples = (0..crate::wavetable::SIZE)
        .map(|i| {
            let p = i as f32 / crate::wavetable::SIZE as f32;
            match index {
                0 => (p * std::f32::consts::TAU).sin(),
                1 => 2. * p - 1.,
                2 => {
                    if p < 0.5 {
                        1.
                    } else {
                        -1.
                    }
                }
                3 => 1. - 4. * (p - 0.5).abs(),
                4 => {
                    if p < 0.25 {
                        1.
                    } else {
                        -1.
                    }
                }
                _ => {
                    if p < 0.125 {
                        1.
                    } else {
                        -1.
                    }
                }
            }
        })
        .collect();
    crate::wavetable::Source {
        name: BASIC[index.min(5)].into(),
        samples,
    }
}
pub fn root_button(o: usize, i: usize) -> Rect {
    Rect::new(220. + 410. * o as f32 + i as f32 * 45., 232., 42., 24.)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn browser_bounds_paging_and_preview() {
        let mut b = Browser::default();
        b.open(0);
        for scale in [1., 1.5] {
            for kind in 0..3 {
                b.kind = kind;
                let mut c = Canvas::new(1180, 820, scale).unwrap();
                b.paint(&mut c);
                if let Ok(dir) = std::env::var("LEMON_WAVE_PREVIEW") {
                    std::fs::create_dir_all(&dir).unwrap();
                    c.pixmap
                        .save_png(format!("{dir}/browser-{kind}-{scale}.png"))
                        .unwrap();
                }
            }
        }
        let mut rects = vec![CLOSE, GP, GN, PREV, NEXT];
        rects.extend((0..3).map(tab));
        rects.extend((0..12).map(group_row));
        rects.extend((0..12).map(row));
        for (i, a) in rects.iter().enumerate() {
            assert!(a.right() < 1180. && a.bottom() < 820.);
            for b in &rects[i + 1..] {
                assert!(
                    a.right().min(b.right()) <= a.x.max(b.x)
                        || a.bottom().min(b.bottom()) <= a.y.max(b.y)
                );
            }
        }
        b.kind = 0;
        assert!(matches!(b.click(500., 223.), Some(Choice::Basic(0))));
        b.kind = 1;
        assert!(matches!(b.click(500., 223.), Some(Choice::Bank(0))));
    }
}
