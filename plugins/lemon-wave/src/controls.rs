// SPDX-License-Identifier: GPL-3.0-or-later
//! Original Lemon Wave controls. No Serum code, tables, skins or presets.
use lemon_small_fx::{Card, Control, Display, Mark, Setting, Sound, Spec};
use os_ui::{Canvas, Color, Rect};
pub const A_BANK: usize = 0;
pub const A_POSITION: usize = 1;
pub const A_LEVEL: usize = 2;
pub const A_TUNE: usize = 3;
pub const A_UNISON: usize = 4;
pub const A_DETUNE: usize = 5;
pub const A_PAN: usize = 6;
pub const A_WARP: usize = 7;
pub const B_BANK: usize = 8;
pub const B_POSITION: usize = 9;
pub const B_LEVEL: usize = 10;
pub const B_TUNE: usize = 11;
pub const B_UNISON: usize = 12;
pub const B_DETUNE: usize = 13;
pub const B_PAN: usize = 14;
pub const B_WARP: usize = 15;
pub const CUTOFF: usize = 16;
pub const RESONANCE: usize = 17;
pub const FILTER: usize = 18;
pub const DRIVE: usize = 19;
pub const SUB: usize = 20;
pub const NOISE: usize = 21;
pub const GAIN: usize = 22;
pub const SPREAD: usize = 23;
pub const ATTACK: usize = 24;
pub const DECAY: usize = 25;
pub const SUSTAIN: usize = 26;
pub const RELEASE: usize = 27;
pub const ENV_ATTACK: usize = 28;
pub const ENV_DECAY: usize = 29;
pub const ENV_SUSTAIN: usize = 30;
pub const ENV_RELEASE: usize = 31;
pub const ENV_AMOUNT: usize = 32;
pub const LFO_RATE: usize = 33;
pub const LFO_AMOUNT: usize = 34;
pub const LFO_SHAPE: usize = 35;
pub const LFO_TARGET: usize = 36;
pub const LFO_SYNC: usize = 37;
pub const CHORUS_RATE: usize = 38;
pub const CHORUS_DEPTH: usize = 39;
pub const CHORUS_MIX: usize = 40;
pub const DELAY_TIME: usize = 41;
pub const DELAY_FEEDBACK: usize = 42;
pub const DELAY_MIX: usize = 43;
pub const COUNT: usize = 44;
pub const SETTINGS: [Setting; COUNT] = [
    Setting {
        id: "a_bank",
        name: "A Bank",
        range: (0, 3),
        start: 0,
        magnet: None,
    },
    Setting {
        id: "a_position",
        name: "A Position",
        range: (0, 1000),
        start: 650,
        magnet: None,
    },
    Setting {
        id: "a_level",
        name: "A Level",
        range: (0, 1000),
        start: 700,
        magnet: None,
    },
    Setting {
        id: "a_tune",
        name: "A Tune",
        range: (-24, 24),
        start: 0,
        magnet: None,
    },
    Setting {
        id: "a_unison",
        name: "A Unison",
        range: (1, 7),
        start: 1,
        magnet: None,
    },
    Setting {
        id: "a_detune",
        name: "A Detune",
        range: (0, 100),
        start: 12,
        magnet: None,
    },
    Setting {
        id: "a_pan",
        name: "A Pan",
        range: (-100, 100),
        start: 0,
        magnet: None,
    },
    Setting {
        id: "a_warp",
        name: "A Bend",
        range: (-90, 90),
        start: 0,
        magnet: None,
    },
    Setting {
        id: "b_bank",
        name: "B Bank",
        range: (0, 3),
        start: 0,
        magnet: None,
    },
    Setting {
        id: "b_position",
        name: "B Position",
        range: (0, 1000),
        start: 400,
        magnet: None,
    },
    Setting {
        id: "b_level",
        name: "B Level",
        range: (0, 1000),
        start: 0,
        magnet: None,
    },
    Setting {
        id: "b_tune",
        name: "B Tune",
        range: (-24, 24),
        start: 0,
        magnet: None,
    },
    Setting {
        id: "b_unison",
        name: "B Unison",
        range: (1, 7),
        start: 1,
        magnet: None,
    },
    Setting {
        id: "b_detune",
        name: "B Detune",
        range: (0, 100),
        start: 12,
        magnet: None,
    },
    Setting {
        id: "b_pan",
        name: "B Pan",
        range: (-100, 100),
        start: 0,
        magnet: None,
    },
    Setting {
        id: "b_warp",
        name: "B Bend",
        range: (-90, 90),
        start: 0,
        magnet: None,
    },
    Setting {
        id: "cutoff",
        name: "Cutoff",
        range: (0, 1000),
        start: 1000,
        magnet: None,
    },
    Setting {
        id: "resonance",
        name: "Resonance",
        range: (0, 1000),
        start: 70,
        magnet: None,
    },
    Setting {
        id: "filter",
        name: "Filter",
        range: (0, 3),
        start: 0,
        magnet: None,
    },
    Setting {
        id: "drive",
        name: "Drive",
        range: (0, 1000),
        start: 0,
        magnet: None,
    },
    Setting {
        id: "sub",
        name: "Sub",
        range: (0, 1000),
        start: 0,
        magnet: None,
    },
    Setting {
        id: "noise",
        name: "Noise",
        range: (0, 1000),
        start: 0,
        magnet: None,
    },
    Setting {
        id: "gain",
        name: "Output",
        range: (0, 1000),
        start: 650,
        magnet: None,
    },
    Setting {
        id: "spread",
        name: "Spread",
        range: (0, 1000),
        start: 800,
        magnet: None,
    },
    Setting {
        id: "attack",
        name: "Attack",
        range: (0, 1000),
        start: 80,
        magnet: None,
    },
    Setting {
        id: "decay",
        name: "Decay",
        range: (0, 1000),
        start: 460,
        magnet: None,
    },
    Setting {
        id: "sustain",
        name: "Sustain",
        range: (0, 1000),
        start: 800,
        magnet: None,
    },
    Setting {
        id: "release",
        name: "Release",
        range: (0, 1000),
        start: 400,
        magnet: None,
    },
    Setting {
        id: "env_attack",
        name: "Mod attack",
        range: (0, 1000),
        start: 80,
        magnet: None,
    },
    Setting {
        id: "env_decay",
        name: "Mod decay",
        range: (0, 1000),
        start: 450,
        magnet: None,
    },
    Setting {
        id: "env_sustain",
        name: "Mod sustain",
        range: (0, 1000),
        start: 0,
        magnet: None,
    },
    Setting {
        id: "env_release",
        name: "Mod release",
        range: (0, 1000),
        start: 350,
        magnet: None,
    },
    Setting {
        id: "env_amount",
        name: "Env > cutoff",
        range: (-1000, 1000),
        start: 0,
        magnet: None,
    },
    Setting {
        id: "lfo_rate",
        name: "LFO rate",
        range: (0, 1000),
        start: 450,
        magnet: None,
    },
    Setting {
        id: "lfo_amount",
        name: "LFO amount",
        range: (0, 1000),
        start: 0,
        magnet: None,
    },
    Setting {
        id: "lfo_shape",
        name: "LFO shape",
        range: (0, 3),
        start: 0,
        magnet: None,
    },
    Setting {
        id: "lfo_target",
        name: "LFO target",
        range: (0, 3),
        start: 0,
        magnet: None,
    },
    Setting {
        id: "lfo_sync",
        name: "LFO sync",
        range: (0, 8),
        start: 0,
        magnet: None,
    },
    Setting {
        id: "chorus_rate",
        name: "Chorus rate",
        range: (0, 1000),
        start: 350,
        magnet: None,
    },
    Setting {
        id: "chorus_depth",
        name: "Chorus depth",
        range: (0, 1000),
        start: 400,
        magnet: None,
    },
    Setting {
        id: "chorus_mix",
        name: "Chorus mix",
        range: (0, 1000),
        start: 0,
        magnet: None,
    },
    Setting {
        id: "delay_time",
        name: "Delay time",
        range: (0, 1000),
        start: 400,
        magnet: None,
    },
    Setting {
        id: "delay_feedback",
        name: "Feedback",
        range: (0, 1000),
        start: 350,
        magnet: None,
    },
    Setting {
        id: "delay_mix",
        name: "Delay mix",
        range: (0, 1000),
        start: 0,
        magnet: None,
    },
];
pub const CONTROLS: [Control; COUNT] = [
    Control::Knob {
        id: "a_bank",
        cx: 59.0,
        cy: 164.0,
        r: 21.0,
        caption: "Bank",
        bipolar: false,
    },
    Control::Knob {
        id: "a_position",
        cx: 146.0,
        cy: 164.0,
        r: 21.0,
        caption: "Position",
        bipolar: false,
    },
    Control::Knob {
        id: "a_level",
        cx: 233.0,
        cy: 164.0,
        r: 21.0,
        caption: "Level",
        bipolar: false,
    },
    Control::Knob {
        id: "a_tune",
        cx: 320.0,
        cy: 164.0,
        r: 21.0,
        caption: "Tune",
        bipolar: true,
    },
    Control::Knob {
        id: "a_unison",
        cx: 59.0,
        cy: 255.0,
        r: 21.0,
        caption: "Unison",
        bipolar: false,
    },
    Control::Knob {
        id: "a_detune",
        cx: 146.0,
        cy: 255.0,
        r: 21.0,
        caption: "Detune",
        bipolar: false,
    },
    Control::Knob {
        id: "a_pan",
        cx: 233.0,
        cy: 255.0,
        r: 21.0,
        caption: "Pan",
        bipolar: true,
    },
    Control::Knob {
        id: "a_warp",
        cx: 320.0,
        cy: 255.0,
        r: 21.0,
        caption: "Bend",
        bipolar: true,
    },
    Control::Knob {
        id: "b_bank",
        cx: 421.0,
        cy: 164.0,
        r: 21.0,
        caption: "Bank",
        bipolar: false,
    },
    Control::Knob {
        id: "b_position",
        cx: 508.0,
        cy: 164.0,
        r: 21.0,
        caption: "Position",
        bipolar: false,
    },
    Control::Knob {
        id: "b_level",
        cx: 595.0,
        cy: 164.0,
        r: 21.0,
        caption: "Level",
        bipolar: false,
    },
    Control::Knob {
        id: "b_tune",
        cx: 682.0,
        cy: 164.0,
        r: 21.0,
        caption: "Tune",
        bipolar: true,
    },
    Control::Knob {
        id: "b_unison",
        cx: 421.0,
        cy: 255.0,
        r: 21.0,
        caption: "Unison",
        bipolar: false,
    },
    Control::Knob {
        id: "b_detune",
        cx: 508.0,
        cy: 255.0,
        r: 21.0,
        caption: "Detune",
        bipolar: false,
    },
    Control::Knob {
        id: "b_pan",
        cx: 595.0,
        cy: 255.0,
        r: 21.0,
        caption: "Pan",
        bipolar: true,
    },
    Control::Knob {
        id: "b_warp",
        cx: 682.0,
        cy: 255.0,
        r: 21.0,
        caption: "Bend",
        bipolar: true,
    },
    Control::Knob {
        id: "cutoff",
        cx: 783.0,
        cy: 164.0,
        r: 21.0,
        caption: "Cutoff",
        bipolar: false,
    },
    Control::Knob {
        id: "resonance",
        cx: 870.0,
        cy: 164.0,
        r: 21.0,
        caption: "Resonance",
        bipolar: false,
    },
    Control::Knob {
        id: "filter",
        cx: 957.0,
        cy: 164.0,
        r: 21.0,
        caption: "Filter",
        bipolar: false,
    },
    Control::Knob {
        id: "drive",
        cx: 1044.0,
        cy: 164.0,
        r: 21.0,
        caption: "Drive",
        bipolar: false,
    },
    Control::Knob {
        id: "sub",
        cx: 783.0,
        cy: 255.0,
        r: 21.0,
        caption: "Sub",
        bipolar: false,
    },
    Control::Knob {
        id: "noise",
        cx: 870.0,
        cy: 255.0,
        r: 21.0,
        caption: "Noise",
        bipolar: false,
    },
    Control::Knob {
        id: "gain",
        cx: 957.0,
        cy: 255.0,
        r: 21.0,
        caption: "Output",
        bipolar: false,
    },
    Control::Knob {
        id: "spread",
        cx: 1044.0,
        cy: 255.0,
        r: 21.0,
        caption: "Spread",
        bipolar: false,
    },
    Control::Knob {
        id: "attack",
        cx: 59.0,
        cy: 414.0,
        r: 21.0,
        caption: "Attack",
        bipolar: false,
    },
    Control::Knob {
        id: "decay",
        cx: 146.0,
        cy: 414.0,
        r: 21.0,
        caption: "Decay",
        bipolar: false,
    },
    Control::Knob {
        id: "sustain",
        cx: 233.0,
        cy: 414.0,
        r: 21.0,
        caption: "Sustain",
        bipolar: false,
    },
    Control::Knob {
        id: "release",
        cx: 320.0,
        cy: 414.0,
        r: 21.0,
        caption: "Release",
        bipolar: false,
    },
    Control::Knob {
        id: "env_attack",
        cx: 405.0,
        cy: 414.0,
        r: 21.0,
        caption: "attack",
        bipolar: false,
    },
    Control::Knob {
        id: "env_decay",
        cx: 470.0,
        cy: 414.0,
        r: 21.0,
        caption: "decay",
        bipolar: false,
    },
    Control::Knob {
        id: "env_sustain",
        cx: 535.0,
        cy: 414.0,
        r: 21.0,
        caption: "sustain",
        bipolar: false,
    },
    Control::Knob {
        id: "env_release",
        cx: 600.0,
        cy: 414.0,
        r: 21.0,
        caption: "release",
        bipolar: false,
    },
    Control::Knob {
        id: "env_amount",
        cx: 665.0,
        cy: 414.0,
        r: 21.0,
        caption: "Env > cutoff",
        bipolar: true,
    },
    Control::Knob {
        id: "lfo_rate",
        cx: 770.0,
        cy: 414.0,
        r: 21.0,
        caption: "rate",
        bipolar: false,
    },
    Control::Knob {
        id: "lfo_amount",
        cx: 835.0,
        cy: 414.0,
        r: 21.0,
        caption: "amount",
        bipolar: false,
    },
    Control::Knob {
        id: "lfo_shape",
        cx: 900.0,
        cy: 414.0,
        r: 21.0,
        caption: "shape",
        bipolar: false,
    },
    Control::Knob {
        id: "lfo_target",
        cx: 965.0,
        cy: 414.0,
        r: 21.0,
        caption: "target",
        bipolar: false,
    },
    Control::Knob {
        id: "lfo_sync",
        cx: 1030.0,
        cy: 414.0,
        r: 21.0,
        caption: "sync",
        bipolar: false,
    },
    Control::Knob {
        id: "chorus_rate",
        cx: 85.0,
        cy: 590.0,
        r: 21.0,
        caption: "Chorus rate",
        bipolar: false,
    },
    Control::Knob {
        id: "chorus_depth",
        cx: 265.0,
        cy: 590.0,
        r: 21.0,
        caption: "Chorus depth",
        bipolar: false,
    },
    Control::Knob {
        id: "chorus_mix",
        cx: 445.0,
        cy: 590.0,
        r: 21.0,
        caption: "Chorus mix",
        bipolar: false,
    },
    Control::Knob {
        id: "delay_time",
        cx: 625.0,
        cy: 590.0,
        r: 21.0,
        caption: "Delay time",
        bipolar: false,
    },
    Control::Knob {
        id: "delay_feedback",
        cx: 805.0,
        cy: 590.0,
        r: 21.0,
        caption: "Feedback",
        bipolar: false,
    },
    Control::Knob {
        id: "delay_mix",
        cx: 985.0,
        cy: 590.0,
        r: 21.0,
        caption: "Delay mix",
        bipolar: false,
    },
];
const CARDS: [Card; 7] = [
    Card {
        x0: 16.,
        x1: 362.,
        y0: 36.,
        y1: 310.,
        heading: "OSCILLATOR A",
    },
    Card {
        x0: 378.,
        x1: 724.,
        y0: 36.,
        y1: 310.,
        heading: "OSCILLATOR B",
    },
    Card {
        x0: 740.,
        x1: 1084.,
        y0: 36.,
        y1: 310.,
        heading: "FILTER / MIXER",
    },
    Card {
        x0: 16.,
        x1: 362.,
        y0: 350.,
        y1: 480.,
        heading: "AMPLITUDE ENVELOPE",
    },
    Card {
        x0: 378.,
        x1: 724.,
        y0: 350.,
        y1: 480.,
        heading: "FILTER ENVELOPE",
    },
    Card {
        x0: 740.,
        x1: 1084.,
        y0: 350.,
        y1: 480.,
        heading: "LFO / MODULATION",
    },
    Card {
        x0: 16.,
        x1: 1084.,
        y0: 530.,
        y1: 660.,
        heading: "STEREO EFFECTS",
    },
];
pub const SPEC: &Spec = &Spec {
    name: "Lemon Wave",
    word: "WAVE",
    about: "Lemon Wave — original wavetable synthesis / Stage 1",
    settings: &SETTINGS,
    version: Some(1),
    any_version: false,
    fields: &[],
    tail: &[],
    size: (1100, 720),
    cards: &CARDS,
    controls: &CONTROLS,
    display: Some(Display {
        r: Rect::new(28., 52., 684., 66.),
        draw: draw,
    }),
    mark: Mark {
        x: 1084.,
        y: 693.,
        anchor: 1.,
        stacked: false,
    },
    hint: Some(Rect::new(16., 680., 780., 26.)),
    labels: &[(910., 76., "POLYPHONIC WAVETABLE SYNTH")],
    text,
    parse,
    sound: unused_sound,
    rings: true,
};
fn unused_sound(_: u32, _: &[i32]) -> Box<dyn Sound> {
    unreachable!("Wave owns its instrument engine")
}
pub fn seconds(v: f32) -> f32 {
    0.001 * 10000.0_f32.powf(v / 1000.)
}
pub fn cutoff(v: f32) -> f32 {
    20. * 1000.0_f32.powf(v / 1000.)
}
pub fn lfo_hz(v: f32) -> f32 {
    0.05 * 400.0_f32.powf(v / 1000.)
}
pub fn text(k: usize, v: i32) -> String {
    match k {
        A_BANK | B_BANK => ["Basic", "Hollow", "Glass", "Vowel"][v.clamp(0, 3) as usize].into(),
        FILTER => ["Bypass", "Low pass", "Band pass", "High pass"][v.clamp(0, 3) as usize].into(),
        LFO_SHAPE => ["Sine", "Triangle", "Ramp", "Square"][v.clamp(0, 3) as usize].into(),
        LFO_TARGET => {
            ["Table position", "Cutoff", "Pitch", "Amplitude"][v.clamp(0, 3) as usize].into()
        }
        LFO_SYNC => [
            "Free", "4 bars", "2 bars", "1 bar", "1/2", "1/4", "1/8", "1/16", "1/32",
        ][v.clamp(0, 8) as usize]
            .into(),
        A_TUNE | B_TUNE => format!("{v:+} st"),
        A_UNISON | B_UNISON => format!("{v} voices"),
        A_DETUNE | B_DETUNE => format!("{v} cents"),
        CUTOFF => format!("{:.0} Hz", cutoff(v as f32)),
        ATTACK | DECAY | RELEASE | ENV_ATTACK | ENV_DECAY | ENV_RELEASE => {
            format!("{:.0} ms", seconds(v as f32) * 1000.)
        }
        LFO_RATE => format!("{:.2} Hz", lfo_hz(v as f32)),
        DELAY_TIME => format!("{} ms", 50 + v),
        CHORUS_RATE => format!("{:.2} Hz", 0.05 + v as f32 / 200.),
        _ => format!("{:.1}%", 100. * (v as f32 / SETTINGS[k].range.1 as f32)),
    }
}
fn parse(k: usize, s: &str) -> Option<i32> {
    let s = s.trim();
    if matches!(
        k,
        A_BANK | B_BANK | FILTER | LFO_SHAPE | LFO_TARGET | LFO_SYNC
    ) {
        return (SETTINGS[k].range.0..=SETTINGS[k].range.1)
            .find(|&v| text(k, v).eq_ignore_ascii_case(s));
    }
    let x = s
        .split_whitespace()
        .next()?
        .trim_end_matches('%')
        .parse::<f32>()
        .ok()?;
    if !x.is_finite() {
        return None;
    }
    let v = match k {
        ATTACK | DECAY | RELEASE | ENV_ATTACK | ENV_DECAY | ENV_RELEASE => {
            x.max(1.).log10() / 4. * 1000.
        }
        CUTOFF => (x.max(20.) / 20.).log10() / 3. * 1000.,
        LFO_RATE => (x.max(0.05) / 0.05).ln() / 400.0_f32.ln() * 1000.,
        CHORUS_RATE => (x - 0.05) * 200.,
        DELAY_TIME => x - 50.,
        A_TUNE | B_TUNE | A_UNISON | B_UNISON | A_DETUNE | B_DETUNE => x,
        _ => x / 100. * SETTINGS[k].range.1 as f32,
    };
    Some((v.round() as i32).clamp(SETTINGS[k].range.0, SETTINGS[k].range.1))
}
fn draw(c: &mut Canvas, r: Rect, v: &[i32]) {
    for o in 0..2 {
        let x = r.x + o as f32 * 362.;
        let b = o * 8;
        let pts: Vec<_> = (0..320)
            .map(|i| {
                let phase = i as f32 / 319.;
                let y = crate::dsp::preview(v[b] as usize, v[b + 1] as f32 / 1000., phase);
                (x + i as f32, r.cy() - y * 25.)
            })
            .collect();
        c.polyline(
            &pts,
            1.5,
            Color::rgb(if o == 0 { 0xe7dc6b } else { 0x65d4c3 }),
        );
    }
}
