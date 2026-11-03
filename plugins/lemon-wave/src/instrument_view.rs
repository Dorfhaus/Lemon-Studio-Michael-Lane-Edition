// SPDX-License-Identifier: GPL-3.0-or-later
//! Dedicated Lemon Wave instrument view. Uses the same hit targets and host gestures
//! as the shared editor; graphs are parameter-derived, never decorative fake meters.
use crate::{assets::WaveParams, controls::*, reference_oscillator::Warp};
use lemon_small_fx::view::{Hit, Model, Target, ViewState};
use os_ui::{Canvas, Color, Font, Rect};
fn font(size: u16) -> Font {
    Font::light(size + 1, 0).natural()
}
const BG: Color = Color::rgb(0x101319);
const PANEL: Color = Color::rgb(0x1b2029);
const WELL: Color = Color::rgb(0x11161e);
const LINE: Color = Color::rgb(0x303a49);
const INK: Color = Color::rgb(0xe8edf3);
const DIM: Color = Color::rgb(0x8d9baa);
const A: Color = Color::rgb(0xd3ed76);
const B: Color = Color::rgb(0x68d7df);
const MOD: Color = Color::rgb(0xbda4fb);
pub const PRESET_PREV: Rect = Rect::new(414., 18., 30., 32.);
pub const PRESET_NEXT: Rect = Rect::new(724., 18., 30., 32.);
pub const PRESET_INIT: Rect = Rect::new(768., 18., 70., 32.);
pub const PRESET_NAMES: [&str; 8] = [
    "Init",
    "Bend Bass",
    "Mirror Keys",
    "Quantize Motion",
    "Wide Lead",
    "Dual Sweep",
    "Wide Sky",
    "Moving Edge",
];
pub fn table_buttons(o: usize) -> [Rect; 3] {
    let x = 28. + o as f32 * 410.;
    [
        Rect::new(x, 418., 105., 20.),
        Rect::new(x + 111., 418., 140., 20.),
        Rect::new(x + 257., 418., 117., 20.),
    ]
}
struct Ui<'a> {
    c: &'a mut Canvas,
    m: &'a dyn Model,
    st: &'a ViewState,
    hits: Vec<Hit>,
}
impl Ui<'_> {
    fn label(&mut self, x: f32, y: f32, s: &str, size: u32, color: Color) {
        self.c.text(
            if size >= 12 {
                font(size as _).bold()
            } else {
                font(size as _)
            },
            x,
            y,
            color,
            s,
        );
    }
    fn panel(&mut self, r: Rect, title: &str, accent: Color, subtitle: &str) {
        self.c.fill_rrect(r, 9., PANEL);
        self.c
            .fill_rrect(Rect::new(r.x + 12., r.y + 13., 3., 15.), 1.5, accent);
        self.label(r.x + 24., r.y + 12., title, 12, INK);
        if !subtitle.is_empty() {
            self.label(r.x + 24., r.y + 31., subtitle, 10, DIM);
        }
    }
    fn hit(&mut self, r: Rect, t: Target) {
        self.hits.push(Hit { r, t });
    }
    fn hover(&self, r: Rect) -> bool {
        self.st.mouse.is_some_and(|(x, y)| r.contains(x, y))
    }
    fn knob(&mut self, k: usize, x: f32, y: f32, caption: &str, color: Color) {
        let id = SETTINGS[k].id;
        let n = self.m.norm(id).clamp(0., 1.);
        let r = Rect::new(x - 23., y - 24., 46., 70.);
        let hot = self.hover(r) || self.st.pressed.as_ref().is_some_and(|t| t.id() == id);
        let arc = |from: f32, to: f32, radius: f32| -> Vec<(f32, f32)> {
            (0..=40)
                .map(|i| {
                    let a = (135. + 270. * (from + (to - from) * i as f32 / 40.)).to_radians();
                    (x + radius * a.cos(), y + radius * a.sin())
                })
                .collect()
        };
        self.c.polyline(&arc(0., 1., 20.), 2.5, LINE);
        let zero = if SETTINGS[k].range.0 < 0 { 0.5 } else { 0. };
        self.c.polyline(&arc(zero, n, 20.), 2.5, color);
        self.c.fill_rrect(
            Rect::new(x - 15., y - 15., 30., 30.),
            15.,
            Color::rgb(if hot { 0x394452 } else { 0x2a3340 }),
        );
        let a = (135. + 270. * n).to_radians();
        self.c.polyline(
            &[
                (x + 7. * a.cos(), y + 7. * a.sin()),
                (x + 13. * a.cos(), y + 13. * a.sin()),
            ],
            2.,
            INK,
        );
        self.c.text_center(
            font(10),
            Rect::new(x - 29., y + 25., 58., 14.),
            DIM,
            caption,
        );
        let value = self
            .m
            .text(id)
            .replace(" voices", "")
            .replace(" cents", " ct")
            .replace(" deg", "°");
        self.c.text_center(
            font(11),
            Rect::new(x - 32., y + 40., 64., 14.),
            if hot { INK } else { color },
            &value,
        );
        self.hit(r, Target::Knob(id.into()));
    }
    fn select(&mut self, k: usize, r: Rect, color: Color) {
        let id = SETTINGS[k].id;
        let v = self.m.plain(id).round() as i32;
        let (lo, hi) = SETTINGS[k].range;
        self.c.fill_rrect(r, 4., WELL);
        let left = Rect::new(r.x, r.y, 19., r.h);
        let right = Rect::new(r.right() - 19., r.y, 19., r.h);
        self.c.text_center(font(12), left, DIM, "‹");
        self.c.text_center(font(12), right, DIM, "›");
        self.c.text_center(
            font(11),
            Rect::new(r.x + 19., r.y, r.w - 38., r.h),
            color,
            &self.m.text(id),
        );
        self.hit(
            left,
            Target::Choice(id.into(), if v <= lo { hi } else { v - 1 }),
        );
        self.hit(
            right,
            Target::Choice(id.into(), if v >= hi { lo } else { v + 1 }),
        );
        self.hit(
            Rect::new(r.x + 19., r.y, r.w - 38., r.h),
            Target::Knob(id.into()),
        );
    }
    fn grid(&mut self, r: Rect) {
        self.c.fill_rrect(r, 5., WELL);
        for i in 1..8 {
            let x = r.x + r.w * i as f32 / 8.;
            self.c
                .fill_rect(Rect::new(x, r.y + 4., 1., r.h - 8.), Color::rgb(0x202a36));
        }
        for i in 1..4 {
            let y = r.y + r.h * i as f32 / 4.;
            self.c
                .fill_rect(Rect::new(r.x + 4., y, r.w - 8., 1.), Color::rgb(0x202a36));
        }
    }
    fn env(&mut self, base: usize, r: Rect, color: Color) {
        self.grid(r);
        // Horizontal scale is normalized across A/D/hold/R. Values and node shapes
        // follow the existing linear amplitude envelope rather than an invented curve.
        let a = seconds(self.m.plain(SETTINGS[base].id));
        let d = seconds(self.m.plain(SETTINGS[base + 1].id));
        let sustain = self.m.plain(SETTINGS[base + 2].id) / 1000.;
        let release = seconds(self.m.plain(SETTINGS[base + 3].id));
        let hold = (a + d + release).max(0.1) * 0.3;
        let total = a + d + hold + release;
        let x0 = r.x + 8.;
        let w = r.w - 16.;
        let y0 = r.bottom() - 9.;
        let h = r.h - 18.;
        let pts = [
            (x0, y0),
            (x0 + w * a / total, y0 - h),
            (x0 + w * (a + d) / total, y0 - h * sustain),
            (x0 + w * (a + d + hold) / total, y0 - h * sustain),
            (x0 + w, y0),
        ];
        self.c.polyline(&pts, 1.8, color);
        for &(x, y) in &pts[1..4] {
            self.c
                .fill_rrect(Rect::new(x - 2.5, y - 2.5, 5., 5.), 2.5, color);
        }
    }
}
pub fn paint(
    c: &mut Canvas,
    m: &dyn Model,
    st: &ViewState,
    hint: &str,
    params: &WaveParams,
    cycles: [usize; 2],
    preset: Option<usize>,
) -> Vec<Hit> {
    let mut u = Ui {
        c,
        m,
        st,
        hits: Vec::new(),
    };
    u.c.begin_frame(None);
    u.c.clear(BG);
    u.c.fill_rect(Rect::new(0., 0., 1180., 66.), Color::rgb(0x1b212b));
    u.label(23., 17., "LEMON", 12, A);
    u.label(23., 32., "WAVE", 23, INK);
    u.label(130., 26., "WAVETABLE SYNTHESIZER", 10, DIM);
    let title = preset
        .map(|i| PRESET_NAMES[i])
        .unwrap_or("Choose a factory preset");
    for (r, t) in [
        (PRESET_PREV, "‹"),
        (PRESET_NEXT, "›"),
        (PRESET_INIT, "Init"),
    ] {
        u.c.fill_rrect(r, 5., if u.hover(r) { LINE } else { WELL });
        u.c.text_center(font(13), r, INK, t);
    }
    u.c.fill_rrect(Rect::new(450., 18., 268., 32.), 5., WELL);
    u.c.text_center(font(12), Rect::new(450., 18., 268., 32.), INK, title);
    u.label(884., 20., "24 VOICES", 11, INK);
    u.label(884., 37., "STEREO SYNTH", 10, DIM);
    u.label(1080., 27., "LEMON STUDIO", 9, DIM);
    let tables = params.tables.read().unwrap();
    for o in 0..2 {
        let x = 16. + 410. * o as f32;
        let color = if o == 0 { A } else { B };
        let base = o * 8;
        let source = if o == 0 { A_SOURCE } else { B_SOURCE };
        let phase = if o == 0 { A_PHASE } else { B_PHASE };
        u.panel(
            Rect::new(x, 80., 398., 362.),
            if o == 0 {
                "OSCILLATOR A"
            } else {
                "OSCILLATOR B"
            },
            color,
            "",
        );
        u.select(source, Rect::new(x + 12., 112., 174., 25.), color);
        u.select(base, Rect::new(x + 198., 112., 188., 25.), DIM);
        let reference = m.plain(SETTINGS[source].id) > 0.5;
        let table = &tables.oscillators[o];
        let r = Rect::new(x + 12., 146., 374., 78.);
        u.grid(r);
        let position = m.plain(SETTINGS[base + 1].id) / 1000.;
        let bank = m.plain(SETTINGS[base].id) as usize;
        let mode = m.plain(SETTINGS[source + 1].id) as usize;
        let amount = (m.plain(SETTINGS[source + 2].id) / 1000.
            + m.plain(SETTINGS[base + 7].id) / 200.)
            .clamp(0., 1.);
        let warp = Warp::from_reference_id(WARP_IDS[mode.min(10)]).unwrap();
        let points: Vec<_> = (0..360)
            .map(|i| {
                let p = i as f32 / 359.;
                let y = if reference {
                    let (q, g) = warp.apply((p as f64).min(1. - f64::EPSILON), amount);
                    (((q + 0.5).fract() * 2. - 1.) as f32) * g
                } else {
                    let pivot = 0.5 + m.plain(SETTINGS[base + 7].id) / 200.;
                    let q = if p < pivot {
                        p / pivot * 0.5
                    } else {
                        0.5 + (p - pivot) / (1. - pivot) * 0.5
                    };
                    table
                        .as_ref()
                        .map(|t| t.sample(position, q.fract(), 20., 48000.))
                        .unwrap_or_else(|| crate::dsp::preview(bank, position, q.fract()))
                };
                (r.x + 7. + i as f32, r.cy() - y.clamp(-1., 1.) * 29.)
            })
            .collect();
        u.c.polyline(&points, 4., color.with_alpha(25));
        u.c.polyline(&points, 1.5, color);
        let name = if reference {
            "INIT SAW  /  WARP PREVIEW".into()
        } else {
            table
                .as_ref()
                .map(|t| format!("{}  ·  {} frames", t.source.name, t.source.frames()))
                .unwrap_or_else(|| "BUILT-IN WAVETABLE".into())
        };
        u.c.text_clip(font(9), r.x + 7., r.y + 4., r.w - 14., DIM, &name);
        u.select(source + 1, Rect::new(x + 12., 232., 174., 24.), color);
        u.label(
            x + 204.,
            238.,
            if reference {
                "BANK / POSITION IN CLASSIC"
            } else {
                "WARP MODES IN REFERENCE"
            },
            9,
            DIM,
        );
        for (j, (k, cap)) in [
            (base + 1, "Position"),
            (base + 2, "Level"),
            (base + 3, "Tune"),
            (base + 4, "Unison"),
            (base + 5, "Detune"),
            (base + 6, "Pan"),
        ]
        .into_iter()
        .enumerate()
        {
            u.knob(k, x + 36. + 65. * j as f32, 280., cap, color);
        }
        for (j, (k, cap)) in [
            (source + 2, "Warp"),
            (base + 7, "Bend"),
            (phase, "Phase"),
            (phase + 1, "Random"),
            (phase + 2, "Blend"),
        ]
        .into_iter()
        .enumerate()
        {
            u.knob(k, x + 42. + 78. * j as f32, 356., cap, color);
        }
        let labels = [
            "Import WAV".to_string(),
            format!("Cycle {}", crate::wavetable::CYCLES[cycles[o]]),
            "Built-in".into(),
        ];
        for (r, label) in table_buttons(o).into_iter().zip(labels) {
            u.c.fill_rrect(r, 4., if u.hover(r) { LINE } else { WELL });
            u.c.text_center(font(10), r, DIM, &label);
        }
    }
    drop(tables);
    u.panel(Rect::new(836., 80., 328., 362.), "FILTER / MIX", A, "");
    u.select(FILTER, Rect::new(850., 112., 300., 25.), A);
    let fr = Rect::new(850., 146., 300., 78.);
    u.grid(fr);
    let fc = cutoff(m.plain("cutoff"));
    let mode = m.plain("filter") as usize;
    let q = 1. / (2. - 1.9 * m.norm("resonance"));
    let curve: Vec<_> = (0..286)
        .map(|i| {
            let f = 20. * 1000_f32.powf(i as f32 / 285.);
            let w = f / fc;
            let denom = ((1. - w * w).powi(2) + (w / q).powi(2)).sqrt().max(1e-6);
            let mag = match mode {
                1 => 1. / denom,
                2 => w / denom,
                3 => w * w / denom,
                _ => 1.,
            };
            (
                fr.x + 7. + i as f32,
                fr.y + 17. + (-20. * mag.max(1e-4).log10() / 48.).clamp(-0.2, 1.) * 52.,
            )
        })
        .collect();
    u.c.polyline(&curve, 1.5, A);
    u.label(858., 151., "RESPONSE GUIDE", 9, DIM);
    for (j, (k, cap)) in [
        (CUTOFF, "Cutoff"),
        (RESONANCE, "Resonance"),
        (DRIVE, "Drive"),
    ]
    .into_iter()
    .enumerate()
    {
        u.knob(k, 892. + 108. * j as f32, 280., cap, A);
    }
    for (j, (k, cap)) in [
        (SUB, "Sub"),
        (NOISE, "Noise"),
        (SPREAD, "Width"),
        (GAIN, "Output"),
    ]
    .into_iter()
    .enumerate()
    {
        u.knob(k, 875. + 82. * j as f32, 356., cap, A);
    }
    for (base, x, title, color) in [
        (ATTACK, 16., "ENVELOPE 1  /  AMP", A),
        (ENV_ATTACK, 306., "ENVELOPE 2", B),
    ] {
        u.panel(Rect::new(x, 454., 278., 230.), title, color, "");
        u.env(base, Rect::new(x + 12., 490., 254., 76.), color);
        let count = if base == ATTACK { 4 } else { 5 };
        let step = 254. / count as f32;
        for j in 0..count {
            let k = if j == 4 { ENV_AMOUNT } else { base + j };
            u.knob(
                k,
                x + 12. + step * (j as f32 + 0.5),
                601.,
                ["Attack", "Decay", "Sustain", "Release", "To filter"][j],
                color,
            );
        }
        u.label(
            x + 14.,
            663.,
            if base == ATTACK {
                "NOTE GATE → AMPLITUDE"
            } else {
                "ROUTE MORE DESTINATIONS BELOW"
            },
            9,
            DIM,
        );
    }
    u.panel(Rect::new(596., 454., 228., 230.), "LFO 1", MOD, "");
    u.select(LFO_SHAPE, Rect::new(608., 490., 100., 24.), MOD);
    u.select(LFO_SYNC, Rect::new(714., 490., 98., 24.), MOD);
    let r = Rect::new(608., 522., 204., 44.);
    u.grid(r);
    let shape = m.plain("lfo_shape") as usize;
    let pts: Vec<_> = (0..192)
        .map(|i| {
            let p = i as f32 / 191.;
            let v = match shape {
                1 => 1. - 4. * (p - 0.5).abs(),
                2 => p * 2. - 1.,
                3 => {
                    if p < 0.5 {
                        1.
                    } else {
                        -1.
                    }
                }
                _ => (p * std::f32::consts::TAU).sin(),
            };
            (r.x + 6. + i as f32, r.cy() - v * 15.)
        })
        .collect();
    u.c.polyline(&pts, 1.6, MOD);
    u.knob(LFO_RATE, 654., 601., "Rate", MOD);
    u.knob(LFO_AMOUNT, 766., 601., "Direct depth", MOD);
    u.select(LFO_TARGET, Rect::new(608., 653., 204., 24.), MOD);
    u.panel(Rect::new(836., 454., 328., 334.), "EFFECTS", B, "");
    u.label(852., 489., "01   CHORUS", 10, DIM);
    for (j, (k, cap)) in [
        (CHORUS_RATE, "Rate"),
        (CHORUS_DEPTH, "Depth"),
        (CHORUS_MIX, "Mix"),
    ]
    .into_iter()
    .enumerate()
    {
        u.knob(k, 892. + 108. * j as f32, 530., cap, B);
    }
    u.c.fill_rect(Rect::new(852., 596., 296., 1.), LINE);
    u.label(852., 610., "02   STEREO DELAY", 10, DIM);
    for (j, (k, cap)) in [
        (DELAY_TIME, "Time"),
        (DELAY_FEEDBACK, "Feedback"),
        (DELAY_MIX, "Mix"),
    ]
    .into_iter()
    .enumerate()
    {
        u.knob(k, 892. + 108. * j as f32, 650., cap, B);
    }
    u.label(852., 727., "CHORUS → DELAY → OUTPUT", 10, DIM);
    u.label(852., 749., "Mix at 0% keeps the dry signal.", 10, DIM);
    u.panel(
        Rect::new(16., 696., 808., 92.),
        "MODULATION ROUTES",
        MOD,
        "",
    );
    for (slot, y) in [(MOD1_SOURCE, 728.), (MOD2_SOURCE, 759.)] {
        u.select(slot, Rect::new(30., y, 178., 24.), MOD);
        u.label(220., y + 5., "→", 12, DIM);
        u.select(slot + 1, Rect::new(246., y, 216., 24.), INK);
        let r = Rect::new(476., y, 334., 24.);
        u.c.fill_rrect(r, 4., WELL);
        let v = m.plain(SETTINGS[slot + 2].id) / 1000.;
        let center = r.x + 160.;
        u.c.fill_rect(
            Rect::new(
                center.min(center + v * 148.),
                y + 17.,
                (v * 148.).abs().max(1.),
                2.,
            ),
            MOD,
        );
        u.c.text_center(font(11), r, MOD, &format!("Depth  {:+.1}%", v * 100.));
        u.hit(r, Target::Knob(SETTINGS[slot + 2].id.into()));
    }
    let status = params.status.lock().unwrap();
    let footer = if hint.is_empty() {
        status.as_str()
    } else {
        hint
    };
    u.c.text_clip(font(10), 20., 800., 1140., DIM, footer);
    u.hits
}

#[cfg(test)]
mod tests {
    use super::*;
    struct Patch(crate::dsp::Patch);
    impl Model for Patch {
        fn plain(&self, id: &str) -> f32 {
            self.0[SPEC.index(id).unwrap()]
        }
        fn norm(&self, id: &str) -> f32 {
            let s = &SETTINGS[SPEC.index(id).unwrap()];
            (self.plain(id) - s.range.0 as f32) / (s.range.1 - s.range.0) as f32
        }
        fn text(&self, id: &str) -> String {
            crate::controls::text(SPEC.index(id).unwrap(), self.plain(id) as i32)
        }
    }
    #[test]
    fn reference_view_has_accessible_nonoverlapping_targets_at_both_scales() {
        let mut p = crate::dsp::defaults();
        p[A_SOURCE] = 1.;
        p[A_MODE] = 9.;
        p[A_AMOUNT] = 650.;
        p[A_UNISON] = 5.;
        p[A_RANDOM] = 350.;
        p[B_SOURCE] = 1.;
        p[B_LEVEL] = 400.;
        p[B_MODE] = 4.;
        p[B_AMOUNT] = 400.;
        p[CUTOFF] = 700.;
        p[FILTER] = 1.;
        p[MOD1_SOURCE] = 1.;
        p[MOD1_TARGET] = 2.;
        p[MOD1_AMOUNT] = 350.;
        p[MOD2_SOURCE] = 2.;
        p[MOD2_TARGET] = 1.;
        p[MOD2_AMOUNT] = 280.;
        let model = Patch(p);
        let params = WaveParams::default();
        for scale in [1., 1.5] {
            let mut c = Canvas::new(1180, 820, scale).unwrap();
            let hits = paint(
                &mut c,
                &model,
                &Default::default(),
                "Drag to edit · Ctrl for fine control · Right-click for automation",
                &params,
                [5; 2],
                None,
            );
            for (i, a) in hits.iter().enumerate() {
                for b in &hits[i + 1..] {
                    let w = a.r.right().min(b.r.right()) - a.r.x.max(b.r.x);
                    let h = a.r.bottom().min(b.r.bottom()) - a.r.y.max(b.r.y);
                    assert!(
                        w <= 0.01 || h <= 0.01,
                        "overlap: {} / {}",
                        a.t.id(),
                        b.t.id()
                    );
                }
            }
            if let Ok(dir) = std::env::var("LEMON_WAVE_PREVIEW") {
                std::fs::create_dir_all(&dir).unwrap();
                c.pixmap
                    .save_png(format!("{dir}/reference-redesign-{scale}.png"))
                    .unwrap();
            }
        }
    }
}
