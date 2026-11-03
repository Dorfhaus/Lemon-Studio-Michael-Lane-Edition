// SPDX-License-Identifier: GPL-3.0-or-later
//! Dedicated Lemon Wave instrument view. Uses the same hit targets and host gestures
//! as the shared editor; graphs are parameter-derived, never decorative fake meters.
use crate::{assets::WaveParams, controls::*, reference_oscillator::Warp};
use lemon_small_fx::view::{Hit, Model, Target, ViewState};
use os_ui::{Canvas, Color, Font, Rect};
fn font(size: u16) -> Font {
    Font::light(size + 1, 0).natural()
}
pub(crate) const BG: Color = Color::rgb(0x101319);
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
pub const PRESET_NAMES: [&str; 14] = [
    "Init",
    "Bend Bass",
    "Mirror Keys",
    "Quantize Motion",
    "Wide Lead",
    "Dual Sweep",
    "Wide Sky",
    "Moving Edge",
    "Velvet Pluck",
    "Held Bloom",
    "Resonant Band",
    "Glass Tide",
    "Clockwork",
    "Four Corners",
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
    live: crate::modulation::Live,
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
        if let Some(target) = mod_target(k) {
            let routed = ROUTES.iter().any(|slot| {
                self.m.plain(SETTINGS[*slot].id) > 0.
                    && self.m.plain(SETTINGS[slot + 1].id) as usize == target
            });
            if routed {
                let depth: f32 = ROUTES
                    .iter()
                    .filter(|slot| {
                        self.m.plain(SETTINGS[**slot].id) > 0.
                            && self.m.plain(SETTINGS[**slot + 1].id) as usize == target
                    })
                    .map(|slot| self.m.plain(SETTINGS[slot + 2].id).abs() / 1000.)
                    .sum();
                self.c.polyline(
                    &arc((n - depth).max(0.), (n + depth).min(1.), 24.),
                    1.,
                    Color::rgb(0x67537f),
                );
                if self.live.active {
                    let offset = self.live.amounts[target];
                    let v = (if target == 7 {
                        n * (1. + offset)
                    } else if target == 6 {
                        n + offset * 0.5
                    } else {
                        n + offset
                    })
                    .clamp(0., 1.);
                    self.c.polyline(&arc(n.min(v), n.max(v), 24.), 2., MOD);
                }
            }
        }

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
        let reference = self.m.plain("env_engine") > 0.5;
        let key = if base == ATTACK { AMP_HOLD } else { ENV_HOLD };
        let a = seconds(self.m.plain(SETTINGS[base].id));
        let d = seconds(self.m.plain(SETTINGS[base + 1].id));
        let mut sustain = self.m.plain(SETTINGS[base + 2].id) / 1000.;
        if reference {
            sustain *= sustain;
        }
        let release = seconds(self.m.plain(SETTINGS[base + 3].id));
        let hold = if reference {
            hold_seconds(self.m.plain(SETTINGS[key].id))
        } else {
            0.
        };
        let gate = (a + d + release).max(0.1) * 0.3;
        let total = a + hold + d + gate + release;
        let shape = |i: usize| {
            if reference {
                self.m.plain(SETTINGS[key + 1 + i].id) as f64 / 1000.
            } else {
                0.5
            }
        };
        let mut pts = Vec::new();
        for (start, length, from, to, curve) in [
            (0., a, 0., 1., shape(0)),
            (a, hold, 1., 1., 0.5),
            (a + hold, d, 1., sustain, shape(1)),
            (a + hold + d, gate, sustain, sustain, 0.5),
            (a + hold + d + gate, release, sustain, 0., shape(2)),
        ] {
            for i in 0..=40 {
                let t = i as f64 / 40.;
                let v = to + (from - to) * crate::reference_envelope::falling(curve, t);
                pts.push((
                    r.x + 8. + (r.w - 16.) * (start + length * t as f32) / total,
                    r.bottom() - 9. - (r.h - 18.) * v,
                ));
            }
        }
        self.c.polyline(&pts, 1.8, color);
        let i = if base == ATTACK { 0 } else { 1 };
        let stage = self.live.env_stage[i];
        let elapsed = self.live.env_elapsed[i];
        if self.live.active && stage != 0 {
            let time = match stage {
                1 => elapsed.min(a),
                5 => a + elapsed.min(hold),
                2 => a + hold + elapsed.min(d),
                3 => a + hold + d + gate * 0.5,
                4 => a + hold + d + gate + elapsed.min(release),
                _ => 0.,
            };
            let x = r.x + 8. + (r.w - 16.) * time / total;
            self.c
                .polyline(&[(x, r.y + 4.), (x, r.bottom() - 4.)], 1., INK);
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
        live: params.telemetry.read(),
    };
    u.c.begin_frame(None);
    u.c.clear(BG);
    u.c.fill_rect(Rect::new(0., 0., 1180., 66.), Color::rgb(0x1b212b));
    u.label(23., 17., "LEMON", 12, A);
    u.label(23., 32., "WAVE", 23, INK);
    let page = params.page.load(std::sync::atomic::Ordering::Relaxed);
    for (i, label) in ["SYNTH", "MODULATION"].iter().enumerate() {
        let r = page_button(i);
        u.c.fill_rrect(r, 4., if page == i { LINE } else { WELL });
        u.c.text_center(font(11), r, if page == i { A } else { DIM }, label);
    }

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
    u.select(ENV_ENGINE, Rect::new(850., 14., 210., 27.), MOD);
    u.label(878., 45., "ENVELOPES 1 + 2 / NEXT NOTE", 9, DIM);
    u.label(1080., 27., "LEMON STUDIO", 9, DIM);
    if page == 1 {
        paint_modulation(&mut u, params);
        return u.hits;
    }
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
        let browse = crate::sound_browser::button(o);
        u.c.fill_rrect(browse, 4., Color::rgb(0x39472d));
        u.c.text_center(font(11), browse, INK, "BROWSE SOUNDS ...");
        let reference = m.plain(SETTINGS[source].id) > 0.5;
        let table = &tables.oscillators[o];
        let sample = &tables.samples[o];
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
                let y = if let Some(s) = sample.as_ref().filter(|_| !reference) {
                    s.source.samples[((p * (s.source.samples.len() - 1) as f32) as usize)
                        .min(s.source.samples.len() - 1)]
                } else if reference {
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
        let name = if let Some(s) = sample.as_ref().filter(|_| !reference) {
            format!("SAMPLE: {} | Root {:.1}", s.source.name, s.source.root)
        } else if reference {
            "INIT SAW  /  WARP PREVIEW".into()
        } else {
            table
                .as_ref()
                .map(|t| format!("{}  ·  {} frames", t.source.name, t.source.frames()))
                .unwrap_or_else(|| {
                    format!("WAVETABLE: {}", crate::controls::text(base, bank as i32))
                })
        };
        u.c.text_clip(font(9), r.x + 7., r.y + 4., r.w - 14., DIM, &name);
        u.select(source + 1, Rect::new(x + 12., 232., 174., 24.), color);
        if sample.is_some() && !reference {
            for (i, label) in ["-12", "-1", "+1", "+12"].iter().enumerate() {
                let r = crate::sound_browser::root_button(o, i);
                u.c.fill_rrect(r, 3., WELL);
                u.c.text_center(font(10), r, INK, label);
            }
        } else {
            u.select(base, Rect::new(x + 198., 232., 188., 24.), DIM);
        }

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
    u.select(FILTER, Rect::new(850., 112., 145., 25.), A);
    u.select(FILTER_MODEL, Rect::new(1005., 112., 145., 25.), A);
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
            let mut mag = match mode {
                1 => 1. / denom,
                2 => w / denom,
                3 => w * w / denom,
                _ => 1.,
            };
            if m.plain("filter_model") > 1.5 {
                mag *= mag;
            }
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
        u.env(base, Rect::new(x + 12., 490., 254., 62.), color);
        let count = if base == ATTACK { 4 } else { 5 };
        let step = 254. / count as f32;
        for j in 0..count {
            let k = if j == 4 { ENV_AMOUNT } else { base + j };
            u.knob(
                k,
                x + 12. + step * (j as f32 + 0.5),
                580.,
                ["Attack", "Decay", "Sustain", "Release", "To filter"][j],
                color,
            );
        }
        let hold = if base == ATTACK { AMP_HOLD } else { ENV_HOLD };
        for (j, caption) in ["Hold", "Attack curve", "Decay curve", "Release curve"]
            .iter()
            .enumerate()
        {
            let k = hold + j;
            let r = Rect::new(
                x + 12. + (j % 2) as f32 * 130.,
                638. + (j / 2) as f32 * 23.,
                124.,
                20.,
            );
            u.c.fill_rrect(r, 3., WELL);
            let color = if m.plain("env_engine") > 0.5 {
                color
            } else {
                DIM
            };
            let value = m.text(SETTINGS[k].id);
            u.c.text_center(font(9), r, color, &format!("{caption} {value}"));
            u.hit(r, Target::Knob(SETTINGS[k].id.into()));
        }
    }
    u.panel(Rect::new(596., 454., 228., 230.), "LFO 1", MOD, "");
    u.select(
        if m.plain("lfo_draw") > 0.5 {
            LFO_DRAW
        } else {
            LFO_SHAPE
        },
        Rect::new(608., 490., 100., 24.),
        MOD,
    );
    u.select(LFO_SYNC, Rect::new(714., 490., 98., 24.), MOD);
    let r = Rect::new(608., 522., 204., 44.);
    u.grid(r);
    let shape = m.plain("lfo_shape") as usize;
    let drawn = *params.curves.read().unwrap();
    let pts: Vec<_> = (0..192)
        .map(|i| {
            let p = i as f32 / 191.;
            let v = if m.plain("lfo_draw") > 0.5 {
                drawn.sample(0, p)
            } else {
                match shape {
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
                }
            };
            (r.x + 6. + i as f32, r.cy() - v * 15.)
        })
        .collect();
    u.c.polyline(&pts, 1.6, MOD);
    if u.live.active {
        let x = r.x + 6. + 191. * u.live.phases[0];
        u.c.polyline(&[(x, r.y), (x, r.bottom())], 1., INK);
    }
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
    u.panel(Rect::new(16., 696., 808., 92.), "DRAG SOURCE", MOD, "");
    for (source, r, label) in source_buttons(0) {
        u.c.fill_rrect(r, 3., WELL);
        u.c.text_center(font(9), r, MOD, label);
        let _ = source;
    }
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
        p[ENV_ENGINE] = 1.;
        p[AMP_HOLD] = 300.;
        p[AMP_ATK_CURVE] = 700.;
        p[AMP_DEC_CURVE] = 650.;
        p[ENV_DEC_CURVE] = 750.;
        p[FILTER_MODEL] = 2.;
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
        for (page, scale) in [(0, 1.), (0, 1.5), (1, 1.), (1, 1.5)] {
            params
                .page
                .store(page, std::sync::atomic::Ordering::Relaxed);
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
            let mut all: Vec<_> = hits.iter().map(|h| h.r).collect();
            all.extend([page_button(0), page_button(1)]);
            all.extend(source_buttons(page).into_iter().map(|(_, r, _)| r));
            if page == 1 {
                for i in 0..4 {
                    all.push(lfo_graph(i));
                    for j in 0..5 {
                        all.push(shape_button(i, j));
                    }
                }
                for i in 0..8 {
                    all.push(clear_route(i));
                }
            }
            for (i, a) in all.iter().enumerate() {
                assert!(a.x >= 0. && a.y >= 0. && a.right() <= 1180. && a.bottom() <= 820.);
                for b in &all[i + 1..] {
                    let w = a.right().min(b.right()) - a.x.max(b.x);
                    let h = a.bottom().min(b.bottom()) - a.y.max(b.y);
                    assert!(w <= 0.01 || h <= 0.01, "overlap page{page}: {a:?} {b:?}");
                }
            }
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
                    .save_png(format!("{dir}/modulation-page{page}-{scale}.png"))
                    .unwrap();
            }
        }
    }
}

pub fn page_button(page: usize) -> Rect {
    Rect::new(130. + page as f32 * 124., 18., 116., 32.)
}
pub fn lfo_panel(i: usize) -> Rect {
    Rect::new(
        16. + (i % 2) as f32 * 580.,
        80. + (i / 2) as f32 * 198.,
        568.,
        186.,
    )
}
pub fn lfo_graph(i: usize) -> Rect {
    let p = lfo_panel(i);
    Rect::new(p.x + 12., p.y + 49., 330., 100.)
}
pub fn source_buttons(page: usize) -> Vec<(usize, Rect, &'static str)> {
    if page == 0 {
        return [1, 5, 6, 7, 12, 2, 8, 9, 10, 11]
            .into_iter()
            .zip([
                "LFO 1", "LFO 2", "LFO 3", "LFO 4", "ENV 1", "ENV 2", "M1", "M2", "M3", "M4",
            ])
            .enumerate()
            .map(|(i, (s, l))| (s, Rect::new(185. + i as f32 * 62., 700., 58., 22.), l))
            .collect();
    }
    let mut v: Vec<_> = [1, 5, 6, 7]
        .into_iter()
        .enumerate()
        .map(|(i, s)| {
            let p = lfo_panel(i);
            (
                s,
                Rect::new(p.x + 435., p.y + 10., 120., 24.),
                ["Drag LFO 1", "Drag LFO 2", "Drag LFO 3", "Drag LFO 4"][i],
            )
        })
        .collect();
    for i in 0..4 {
        v.push((
            8 + i,
            Rect::new(
                870. + (i % 2) as f32 * 148.,
                506. + (i / 2) as f32 * 125.,
                120.,
                22.,
            ),
            [
                "Drag Macro 1",
                "Drag Macro 2",
                "Drag Macro 3",
                "Drag Macro 4",
            ][i],
        ));
    }
    v
}
pub fn shape_button(i: usize, shape: usize) -> Rect {
    let p = lfo_panel(i);
    Rect::new(p.x + 12. + shape as f32 * 66., p.y + 157., 62., 21.)
}
pub fn clear_route(i: usize) -> Rect {
    Rect::new(790., 510. + i as f32 * 32., 22., 24.)
}
fn paint_modulation(u: &mut Ui, params: &WaveParams) {
    let curves = *params.curves.read().unwrap();
    for i in 0..4 {
        let p = lfo_panel(i);
        u.panel(p, ["LFO 1", "LFO 2", "LFO 3", "LFO 4"][i], MOD, "");
        let r = lfo_graph(i);
        u.grid(r);
        let pts: Vec<_> = (0..=128)
            .map(|j| {
                let phase = j as f32 / 128.;
                let y = if i == 0 && u.m.plain("lfo_draw") < 0.5 {
                    match u.m.plain("lfo_shape") as usize {
                        1 => 1. - 4. * (phase - 0.5).abs(),
                        2 => phase * 2. - 1.,
                        3 => {
                            if phase < 0.5 {
                                1.
                            } else {
                                -1.
                            }
                        }
                        _ => (phase * std::f32::consts::TAU).sin(),
                    }
                } else {
                    curves.sample(i, phase)
                };
                (r.x + phase * r.w, r.y + (1. - y) * r.h * 0.5)
            })
            .collect();
        u.c.polyline(&pts, 1.6, MOD);
        if u.live.active {
            let x = r.x + r.w * u.live.phases[i];
            u.c.polyline(&[(x, r.y), (x, r.bottom())], 1., INK);
        }
        let c = LFO_CONTROLS[i];
        u.knob(c[0], p.x + 384., p.y + 82., "Free Hz", MOD);
        for (j, k) in c[1..].iter().enumerate() {
            u.select(
                *k,
                Rect::new(p.x + 430., p.y + 49. + j as f32 * 32., 126., 25.),
                MOD,
            );
        }
        for (j, label) in ["Sine", "Triangle", "Ramp", "Square", "Undo"]
            .iter()
            .enumerate()
        {
            let r = shape_button(i, j);
            u.c.fill_rrect(r, 3., WELL);
            u.c.text_center(font(9), r, DIM, label);
        }
        if i == 0 {
            u.select(LFO_DRAW, Rect::new(p.x + 350., p.y + 157., 206., 21.), MOD);
        } else {
            u.label(p.x + 356., p.y + 161., "DRAW IN THE GRAPH", 9, DIM);
        }
    }
    u.panel(
        Rect::new(16., 476., 808., 312.),
        "MODULATION MATRIX",
        MOD,
        "",
    );
    for (i, slot) in ROUTES.into_iter().enumerate() {
        let y = 510. + i as f32 * 32.;
        u.label(28., y + 5., &format!("{}", i + 1), 10, DIM);
        u.select(slot, Rect::new(48., y, 178., 24.), MOD);
        u.select(slot + 1, Rect::new(238., y, 278., 24.), INK);
        let r = Rect::new(528., y, 250., 24.);
        u.c.fill_rrect(r, 4., WELL);
        let v = u.m.plain(SETTINGS[slot + 2].id) / 1000.;
        u.c.text_center(font(11), r, MOD, &format!("Depth {:+.1}%", v * 100.));
        u.c.fill_rect(
            Rect::new(
                r.x + 125_f32.min(125. + v * 114.),
                y + 20.,
                (v * 114.).abs().max(1.),
                2.,
            ),
            MOD,
        );
        u.hit(r, Target::Knob(SETTINGS[slot + 2].id.into()));
        let r = clear_route(i);
        u.c.fill_rrect(r, 3., WELL);
        u.c.text_center(font(12), r, DIM, "×");
    }
    u.panel(Rect::new(836., 476., 328., 312.), "MACROS", A, "");
    for i in 0..4 {
        u.knob(
            MACRO1 + i,
            930. + (i % 2) as f32 * 148.,
            560. + (i / 2) as f32 * 125.,
            ["Macro 1", "Macro 2", "Macro 3", "Macro 4"][i],
            A,
        );
    }
    for (_, r, label) in source_buttons(1) {
        u.c.fill_rrect(r, 3., WELL);
        u.c.text_center(font(10), r, MOD, label);
    }
    u.label(20.,800.,"Draw a shape • Drag a source onto a synth knob (hover SYNTH while dragging) • Drag Depth to adjust",10,DIM);
}
