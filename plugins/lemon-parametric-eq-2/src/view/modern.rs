//! Graph-led editor. Window dimensions and interface scale are independent.
use super::*;
#[derive(Clone, Copy, Debug)]
pub struct Layout {
    pub size: (f32, f32),
    pub plot: Rect,
    pub scale: f32,
    pub range: f32,
}
/// Nominal Q of the stored bell octave bandwidth (multi-section/shelf shapes retain their own design).
pub fn q_of(w: f64) -> f64 {
    eq::dynamic::q_of_width(w)
}
pub fn width_of_q(q: f64) -> f64 {
    ((2. * (0.5 / q.clamp(q_of(1.), q_of(0.))).asinh() / std::f64::consts::LN_2 - 0.1) / 4.9)
        .clamp(0., 1.)
}
impl Layout {
    pub fn new(size: (f32, f32), scale: f32) -> Self {
        let (w, h) = if size.0 > 0. && size.1 > 0. {
            size
        } else {
            (W as f32, H as f32)
        };
        let s = if scale > 0. {
            scale.clamp(0.8, 1.5)
        } else {
            1.
        };
        let s = s.min(w / 620.);
        Self {
            size: (w, h),
            plot: Rect::new(
                20. * s,
                44. * s,
                (w - 72. * s).max(100.),
                (h - 100. * s).max(100.),
            ),
            scale: s,
            range: 18.,
        }
    }
    pub fn of(v: &ViewState) -> Self {
        let mut l = Self::new(v.size, v.ui_scale);
        if v.range_db > 0. {
            l.range = v.range_db;
        }
        l
    }
    pub fn x(&self, n: f64) -> f32 {
        self.plot.x + n.clamp(0., 1.) as f32 * self.plot.w
    }
    pub fn n(&self, x: f32) -> f64 {
        f64::from(((x - self.plot.x) / self.plot.w).clamp(0., 1.))
    }
    pub fn y(&self, db: f64) -> f32 {
        self.plot.cy() - db as f32 / self.range * self.plot.h * 0.5
    }
    pub fn db(&self, y: f32) -> f64 {
        f64::from((self.plot.cy() - y) * self.range * 2. / self.plot.h)
    }
    pub fn node(&self, s: &Settings, k: usize) -> (f32, f32) {
        (
            self.x(s.bands[k].freq),
            self.y(if has_level(s.bands[k].kind) {
                s.bands[k].db
            } else {
                0.
            }),
        )
    }
    pub fn panel(&self, n: f64, expanded: bool) -> Rect {
        let s = self
            .scale
            .min((self.plot.w - 8.) / if expanded { 610. } else { 452. });
        let w = if expanded { 610. } else { 452. } * s;
        Rect::new(
            (self.x(n) - w * 0.5).clamp(
                self.plot.x + 2.,
                (self.plot.right() - w - 2.).max(self.plot.x + 2.),
            ),
            self.plot.bottom() - 130. * s,
            w,
            116. * s,
        )
    }
    pub fn panel_for(&self, s: &Settings, v: &ViewState, expanded: bool) -> Rect {
        let mut r = self.panel(s.bands[v.selected.min(6)].freq, expanded);
        let node = self.node(s, v.selected.min(6));
        let z = r.w / if expanded { 610. } else { 452. };
        if !v.harsh_open && r.outset(12. * z).contains(node.0, node.1) {
            r.y = (node.1 - r.h - 24. * z).max(self.plot.y + 38. * z);
        }
        r
    }
    pub fn ring(&self, s: &Settings, v: &ViewState, expanded: bool) -> (f32, f32, f32) {
        let r = self.panel_for(s, v, expanded);
        let z = r.w / if expanded { 610. } else { 452. };
        (r.x + 229. * z, r.y + 65. * z, 34. * z)
    }
}
pub fn available(s: &Settings, k: usize) -> bool {
    let b = s.bands[k];
    b.kind == 0
        || (b.db.abs() < 1e-5
            && (b.freq - eq::default_freq(k)).abs() < 1e-5
            && (b.width - eq::default_width(b.kind)).abs() < 1e-5
            && b.kind == eq::DEFAULT_KINDS[k])
}
pub fn visible(s: &Settings, v: &ViewState, k: usize) -> bool {
    s.bands[k].kind != 0
        && (!available(s, k)
            || v.created & (1 << k) != 0
            || v.selection & (1 << k) != 0
            || v.active == Some(k)
            || (v.show_dynamics && v.selected == k))
}
fn font(n: f32, s: f32) -> Font {
    Font::new((n * s).round() as u16)
}
fn txt(c: &mut Canvas, r: Rect, n: f32, s: f32, color: Color, text: &str) {
    c.text_center(font(n, s), r, color, text);
}
fn button(c: &mut Canvas, h: &mut Vec<Hit>, r: Rect, t: Target, text: &str, on: bool, s: f32) {
    c.fill_rrect(
        Rect::new(r.x, r.y + s, r.w, r.h),
        4. * s,
        Color::rgb(0x090e10).with_alpha(100),
    );
    c.fill_rect_gradient(
        r,
        Color::rgb(if on { 0xf4e486 } else { 0x3b454a }),
        Color::rgb(if on { 0xc6af37 } else { 0x272f33 }),
        4. * s,
    );
    c.stroke_rrect(
        r.inset(0.5 * s),
        4. * s,
        0.8 * s,
        Color::rgb(if on { 0xfff0a5 } else { 0x8e9da3 }).with_alpha(if on { 125 } else { 45 }),
    );
    c.line(
        r.x + 5. * s,
        r.y + s,
        r.right() - 5. * s,
        r.y + s,
        0.7 * s,
        Color::rgb(0xffffff).with_alpha(if on { 100 } else { 30 }),
    );
    txt(
        c,
        r,
        10.,
        s,
        if on {
            Color::rgb(0x171b1e)
        } else {
            Color::rgb(0xc1ccd2)
        },
        text,
    );
    h.push(Hit { r, t });
}
fn knob(
    c: &mut Canvas,
    h: &mut Vec<Hit>,
    x: f32,
    y: f32,
    r: f32,
    n: f64,
    t: Target,
    label: &str,
    value: &str,
    color: Color,
    s: f32,
) {
    c.fill_circle(
        x,
        y + 2. * s,
        r + 2. * s,
        Color::rgb(0x080d10).with_alpha(185),
    );
    c.fill_rect_gradient(
        Rect::new(
            x - r - 0.8 * s,
            y - r - 0.8 * s,
            2. * r + 1.6 * s,
            2. * r + 1.6 * s,
        ),
        Color::rgb(0x85949a),
        Color::rgb(0x11191e),
        r + s,
    );
    c.fill_rect_gradient(
        Rect::new(x - r, y - r, 2. * r, 2. * r),
        Color::rgb(0x404e55),
        Color::rgb(0x1e272d),
        r,
    );
    c.stroke_circle(
        x,
        y,
        r - 1. * s,
        0.8 * s,
        Color::rgb(0xd4e2e6).with_alpha(45),
    );
    c.arc(
        x,
        y,
        r - 2. * s,
        -PI * 0.91,
        PI * 0.78,
        0.8 * s,
        Color::rgb(0xffffff).with_alpha(50),
    );
    let angle = -PI * 1.25 + PI * 1.5 * n.clamp(0., 1.) as f32;
    c.line(
        x + angle.cos() * r * 0.57,
        y + angle.sin() * r * 0.57,
        x + angle.cos() * (r - 6. * s),
        y + angle.sin() * (r - 6. * s),
        1.5 * s,
        color.with_alpha(220),
    );
    c.arc(
        x,
        y,
        r - 4. * s,
        -PI * 1.25,
        PI * 1.5 * n.clamp(0., 1.) as f32,
        2. * s,
        color,
    );
    txt(
        c,
        Rect::new(x - r - 8. * s, y - r - 19. * s, 2. * r + 16. * s, 15. * s),
        10.,
        s,
        Color::rgb(0xaab7c1),
        value,
    );
    txt(
        c,
        Rect::new(x - r - 10. * s, y + r + 5. * s, 2. * r + 20. * s, 14. * s),
        11.,
        s,
        Color::rgb(0xd5dfe6),
        label,
    );
    h.push(Hit {
        r: Rect::new(
            x - r - 4. * s,
            y - r - 16. * s,
            2. * r + 8. * s,
            2. * r + 38. * s,
        ),
        t,
    });
}
pub fn paint(c: &mut Canvas, look: &Look, v: &ViewState) -> Vec<Hit> {
    let mut v = v.clone();
    v.size = c.size();
    let l = Layout::of(&v);
    let (w, h) = l.size;
    let s = l.scale;
    let mut hits = vec![];
    let settings = &look.s;
    let cfg = eq::dynamic::Config::load(look.extra, false);
    for k in 0..7 {
        if cfg.bands[k][0] != 0. || cfg.spectral & (1 << k) != 0 {
            v.created |= 1 << k;
        }
    }
    c.begin_frame(None);
    c.clear(Color::rgb(0x1c2024));
    c.fill_rect_gradient(
        Rect::new(0., 0., w, h),
        Color::rgb(0x252a2f),
        Color::rgb(0x171a1e),
        0.,
    );
    c.fill_rect(Rect::new(0., 0., w, 34. * s), Color::rgb(0x252a2d));
    lemon::wordmark(
        c,
        13. * s,
        5. * s,
        true,
        if w < 950. * s { "EQ" } else { "PARAMETRIC EQ" },
        font(17., s),
        font(8., s).bold(),
    );
    button(
        c,
        &mut hits,
        Rect::new(w - 350. * s, 6. * s, 44. * s, 22. * s),
        Target::Undo,
        "Undo",
        false,
        s,
    );
    button(
        c,
        &mut hits,
        Rect::new(w - 300. * s, 6. * s, 44. * s, 22. * s),
        Target::Redo,
        "Redo",
        false,
        s,
    );
    button(
        c,
        &mut hits,
        Rect::new(w - 244. * s, 6. * s, 47. * s, 22. * s),
        Target::Store,
        "A > B",
        false,
        s,
    );
    button(
        c,
        &mut hits,
        Rect::new(w - 189. * s, 6. * s, 47. * s, 22. * s),
        Target::Flip,
        "A / B",
        false,
        s,
    );
    button(
        c,
        &mut hits,
        Rect::new(w - 134. * s, 6. * s, 55. * s, 22. * s),
        Target::Options,
        "Options",
        false,
        s,
    );
    button(
        c,
        &mut hits,
        Rect::new(w - 71. * s, 6. * s, 54. * s, 22. * s),
        Target::UiScale,
        &format!(
            "{}%",
            (if v.ui_scale > 0. { v.ui_scale } else { 1. } * 100.).round()
        ),
        false,
        s,
    );
    c.fill_rrect(l.plot.outset(1. * s), 5. * s, Color::rgb(0x070d10));
    c.fill_rect_gradient(l.plot, Color::rgb(0x253137), Color::rgb(0x141b1f), 4. * s);
    c.push_clip(l.plot.inset(1. * s));
    c.fill_polygon(
        &[
            (l.plot.x, l.plot.y),
            (l.plot.right(), l.plot.y),
            (l.plot.right(), l.plot.y + l.plot.h * 0.1),
            (l.plot.x, l.plot.y + l.plot.h * 0.32),
        ],
        Color::rgb(0xc7e0e7).with_alpha(5),
    );
    c.pop_clip();
    c.stroke_rrect(
        l.plot.inset(0.5 * s),
        4. * s,
        0.8 * s,
        Color::rgb(0x9cafb4).with_alpha(38),
    );
    hits.push(Hit {
        r: l.plot,
        t: Target::Graph,
    });
    for hz in [
        20., 30., 50., 100., 200., 500., 1000., 2000., 5000., 10000., 20000.,
    ] {
        let x = l.x(eq::freq_pos(hz));
        c.line(
            x,
            l.plot.y,
            x,
            l.plot.bottom(),
            0.7,
            Color::rgb(if hz == 100. || hz == 1000. || hz == 10000. {
                0x353b43
            } else {
                0x292f36
            }),
        );
        let label = if hz >= 1000. {
            format!("{}k", hz / 1000.)
        } else {
            format!("{hz:.0}")
        };
        txt(
            c,
            Rect::new(x - 20. * s, l.plot.bottom() + 3. * s, 40. * s, 14. * s),
            9.,
            s,
            Color::rgb(0x9daab4),
            &label,
        );
    }
    for frac in [-1., -0.5, 0., 0.5, 1.] {
        let db = f64::from(frac * l.range);
        let y = l.y(db);
        c.line(
            l.plot.x,
            y,
            l.plot.right(),
            y,
            if frac == 0. { 1.2 } else { 0.6 },
            Color::rgb(if frac == 0. { 0x52545b } else { 0x30363d }),
        );
        txt(
            c,
            Rect::new(l.plot.right() + 3. * s, y - 7. * s, 32. * s, 14. * s),
            9.,
            s,
            Color::rgb(0xdac66f),
            &format!("{db:+.0}"),
        );
    }
    let display = Rect::new(
        l.plot.right() - 52. * s,
        l.plot.y + 5. * s,
        46. * s,
        20. * s,
    );
    button(
        c,
        &mut hits,
        display,
        Target::DisplayRange,
        &format!("{} dB", l.range),
        false,
        s,
    );
    let harsh = Rect::new(l.plot.x + 8. * s, l.plot.y + 7. * s, 94. * s, 22. * s);
    button(
        c,
        &mut hits,
        harsh,
        Target::HarshPanel,
        "De-harsh",
        eq::dynamic::Config::load(look.extra, false).harsh[0] > 0.,
        s,
    );
    c.push_clip(l.plot);
    if let Some(a) = look.spectrum.filter(|a| !a.is_empty()) {
        let mut poly = Vec::with_capacity(a.len() + 2);
        poly.push((l.plot.x, l.plot.bottom()));
        for (i, db) in a.iter().enumerate() {
            let x = l.plot.x + i as f32 / (a.len() - 1).max(1) as f32 * l.plot.w;
            let range =
                [60., 90., 120.][(params::opt(look.extra, Opt::Range) - 1).clamp(0, 2) as usize];
            let y = l.plot.bottom() - ((db + range) / range).clamp(0., 1.) * l.plot.h;
            poly.push((x, y));
        }
        poly.push((l.plot.right(), l.plot.bottom()));
        if flag(look.extra, params::F_HISTOGRAM) {
            c.fill_polygon(&poly, Color::rgb(0xa9b3be).with_alpha(26));
        }
        c.polyline(
            &poly[1..poly.len() - 1],
            0.9 * s,
            Color::rgb(0xabb6c1).with_alpha(140),
        );
    }
    if flag(look.extra, params::F_HEATMAP) {
        if let Some(a) = look.spectrum.filter(|a| !a.is_empty()) {
            let range =
                [60., 90., 120.][(params::opt(look.extra, Opt::Range) - 1).clamp(0, 2) as usize];
            let (top, height, alpha) = match params::opt(look.extra, Opt::HeatmapPosition) {
                0 => (l.plot.y, 12. * s, 220.),
                1 => (l.plot.bottom() - 12. * s, 12. * s, 220.),
                _ => (l.plot.y, l.plot.h, 35.),
            };
            for (i, db) in a.iter().enumerate() {
                let v = (1. + db / range).clamp(0., 1.);
                let color = Color::rgb(if v < 0.3 {
                    0x31518a
                } else if v < 0.65 {
                    0x559e91
                } else {
                    0xe4bd58
                });
                c.fill_cell(
                    Rect::new(
                        l.plot.x + i as f32 / a.len() as f32 * l.plot.w,
                        top,
                        l.plot.w / a.len() as f32 + 0.5,
                        height,
                    ),
                    color.with_alpha((alpha * v) as u8),
                );
            }
        }
    }
    let cfg = eq::dynamic::Config::load(look.extra, false);
    let design = Options::from_image(look.extra).design(look.rate.max(22050.));
    let curve = settings.curve(design);
    let count = (l.plot.w as usize).clamp(300, 1800);
    for k in 0..BANDS {
        if !visible(settings, &v, k) {
            continue;
        }
        let selected = v.selection & (1 << k) != 0 || (v.show_dynamics && v.selected == k);
        if selected || v.active == Some(k) {
            let bc = settings.bands[k].coefs(design);
            let mut points = Vec::with_capacity(count + 3);
            for j in 0..count {
                let n = j as f64 / (count - 1) as f64;
                let hz = eq::freq_hz(n);
                points.push((l.x(n), l.y(eq::sections_db(&bc, hz, design.rate()))));
            }
            let mut fill = points.clone();
            fill.push((l.plot.right(), l.y(0.)));
            fill.push((l.plot.x, l.y(0.)));
            c.fill_polygon(
                &fill,
                band_color(k).with_alpha(if selected { 36 } else { 18 }),
            );
            c.polyline(&points, 1. * s, band_color(k).with_alpha(150));
            if cfg.bands[k][0] != 0. {
                let mut envelope = Vec::with_capacity(count + 3);
                for j in 0..count {
                    let n = j as f64 / (count - 1) as f64;
                    let hz = eq::freq_hz(n);
                    let db = eq::sections_db(&bc, hz, design.rate())
                        + eq::dynamic::response_db(
                            hz,
                            look.rate,
                            settings.bands[k],
                            f64::from(cfg.bands[k][0]),
                        );
                    envelope.push((l.x(n), l.y(db)));
                }
                let mut fill = envelope.clone();
                fill.extend(points.iter().rev());
                c.fill_polygon(&fill, band_color(k).with_alpha(35));
                c.polyline(&envelope, 0.8 * s, band_color(k).with_alpha(130));
            }
        }
    }
    let mut points = Vec::with_capacity(count);
    for j in 0..count {
        let n = j as f64 / (count - 1) as f64;
        let hz = eq::freq_hz(n);
        let mut db = curve.db(hz);
        for k in 0..BANDS {
            if settings.audible(k) && settings.bands[k].kind != 0 && cfg.spectral & (1 << k) == 0 {
                db += eq::dynamic::response_db(
                    hz,
                    look.rate,
                    settings.bands[k],
                    f64::from(v.reduction[k]),
                );
            }
        }
        if !v.spectral.is_empty() {
            let f = n * (v.spectral.len() - 1) as f64;
            let i = f.floor() as usize;
            let t = (f - i as f64) as f32;
            db += f64::from(
                v.spectral[i] * (1. - t) + v.spectral[(i + 1).min(v.spectral.len() - 1)] * t,
            );
        }
        points.push((l.x(n), l.y(db)));
    }
    c.polyline(&points, 4.8 * s, Color::rgb(0xf3d967).with_alpha(20));
    c.polyline(&points, 1.8 * s, Color::rgb(0xf5dc73));
    for k in 0..BANDS {
        if !visible(settings, &v, k) {
            continue;
        }
        let (x, y) = l.node(settings, k);
        let col = band_color(k);
        let selected = v.selection & (1 << k) != 0 || (v.show_dynamics && v.selected == k);
        let radius = if selected { 6. } else { 4.5 } * s;
        c.fill_circle(x, y, radius + 2. * s, Color::rgb(0x171c21));
        c.fill_circle(
            x,
            y,
            radius,
            if settings.audible(k) {
                col
            } else {
                Color::rgb(0x59616b)
            },
        );
        if selected {
            c.stroke_circle(x, y, radius + 3. * s, 1.2 * s, col.with_alpha(170));
        }
        hits.push(Hit {
            r: Rect::new(x - 10. * s, y - 10. * s, 20. * s, 20. * s),
            t: Target::Token(k),
        });
        if cfg.bands[k][0] != 0. {
            let ry = l.y(settings.bands[k].db + f64::from(cfg.bands[k][0]));
            kit::triangle(
                c,
                x,
                ry,
                4. * s,
                if cfg.bands[k][0] < 0. {
                    kit::Dir::Up
                } else {
                    kit::Dir::Down
                },
                col,
            );
            hits.push(Hit {
                r: Rect::new(x - 8. * s, ry - 7. * s, 16. * s, 14. * s),
                t: Target::Dynamic(k * 4),
            });
        }
    }
    if let Some(r) = v.marquee {
        c.fill_rect(r, Color::rgb(0xf0d465).with_alpha(22));
        c.stroke_rrect(r, 0., 1., Color::rgb(0xf0d465));
    }
    c.pop_clip();
    if v.show_dynamics || v.harsh_open {
        panel(c, look, &v, &l, &mut hits);
    }
    // Familiar Lemon footer, kept slim so the graph remains the main editing surface.
    let fy = h - 26. * s;
    button(
        c,
        &mut hits,
        Rect::new(15. * s, fy, 53. * s, 20. * s),
        Target::Lin,
        "LIN",
        flag(look.extra, params::F_LIN),
        s,
    );
    button(
        c,
        &mut hits,
        Rect::new(76. * s, fy, 47. * s, 20. * s),
        Target::Hq,
        "HQ",
        byte(look.extra, params::HQ_BYTE) & 1 != 0,
        s,
    );
    for i in 0..3 {
        button(
            c,
            &mut hits,
            Rect::new(145. * s + i as f32 * 53. * s, fy, 47. * s, 20. * s),
            Target::Monitor(i),
            ["Off", "Pre", "Post"][i as usize],
            byte(look.extra, params::MONITOR_BYTE) == i,
            s,
        );
    }
    button(
        c,
        &mut hits,
        Rect::new(w - 121. * s, fy, 99. * s, 20. * s),
        Target::Fader(MAIN_FADER),
        &format!("Out {:+.1} dB", settings.main_db),
        false,
        s,
    );
    for d in [2., 6., 10.] {
        c.line(w - d, h - 2., w - 2., h - d, 1., Color::rgb(0xa1adb5));
    }
    hits
}
fn panel(c: &mut Canvas, look: &Look, v: &ViewState, l: &Layout, hits: &mut Vec<Hit>) {
    let k = v.selected.min(6);
    let cfg = eq::dynamic::Config::load(look.extra, false);
    let expanded = if v.harsh_open {
        false
    } else {
        !cfg.auto[k] && cfg.bands[k][0] != 0.
    };
    let r = l.panel_for(&look.s, v, expanded);
    let s = r.w / if expanded { 610. } else { 452. };
    c.fill_rrect(
        Rect::new(r.x - 3. * s, r.y + 3. * s, r.w + 6. * s, r.h + 4. * s),
        12. * s,
        Color::rgb(0x050b0e).with_alpha(150),
    );
    c.fill_rect_gradient(
        r,
        Color::rgb(0x3b484f).with_alpha(248),
        Color::rgb(0x202a30).with_alpha(250),
        9. * s,
    );
    let color = if v.harsh_open {
        Color::rgb(0xe6d267)
    } else {
        band_color(k)
    };
    c.stroke_rrect(r, 9. * s, 1. * s, color.with_alpha(105));
    c.stroke_rrect(
        r.inset(1.5 * s),
        8. * s,
        0.7 * s,
        Color::rgb(0xe0eef2).with_alpha(35),
    );
    c.push_clip(r.inset(3. * s));
    c.fill_polygon(
        &[
            (r.x, r.y),
            (r.right(), r.y),
            (r.right(), r.y + 17. * s),
            (r.x, r.y + 32. * s),
        ],
        Color::rgb(0xd7eff7).with_alpha(9),
    );
    c.pop_clip();
    c.line(
        r.x + 12. * s,
        r.y + 1.5 * s,
        r.right() - 12. * s,
        r.y + 1.5 * s,
        0.8 * s,
        Color::rgb(0xe4eff1).with_alpha(65),
    );
    if v.harsh_open {
        txt(
            c,
            Rect::new(r.x + 14. * s, r.y + 6. * s, 200. * s, 15. * s),
            11.,
            s,
            color,
            "SPECTRAL DE-HARSH",
        );
        button(
            c,
            hits,
            Rect::new(r.x + 218. * s, r.y + 2. * s, 60. * s, 16. * s),
            Target::Dynamic(39),
            "AUTO",
            cfg.harsh_auto,
            s,
        );
        knob(
            c,
            hits,
            r.x + 65. * s,
            r.y + 61. * s,
            23. * s,
            f64::from(cfg.harsh[0] / 12.),
            Target::Dynamic(28),
            "AMOUNT",
            &format!("{:.1} dB", cfg.harsh[0]),
            color,
            s,
        );
        knob(
            c,
            hits,
            r.x + 165. * s,
            r.y + 61. * s,
            23. * s,
            f64::from((cfg.harsh[1] + 72.) / 72.),
            Target::Dynamic(29),
            "THRESHOLD",
            &format!("{:.0} dBFS", cfg.harsh[1]),
            color,
            s,
        );
        knob(
            c,
            hits,
            r.x + 266. * s,
            r.y + 61. * s,
            23. * s,
            f64::from(cfg.density),
            Target::Dynamic(37),
            "DENSITY",
            &format!("{:.0}%", cfg.density * 100.),
            color,
            s,
        );
        button(
            c,
            hits,
            Rect::new(r.right() - 80. * s, r.y + 39. * s, 67. * s, 24. * s),
            Target::Dynamic(38),
            "3dB TILT",
            cfg.tilt,
            s,
        );
        txt(
            c,
            Rect::new(r.right() - 101. * s, r.y + 75. * s, 90. * s, 17. * s),
            10.,
            s,
            color,
            &format!("{:.1} dB", v.reduction[7]),
        );
        return;
    }
    button(
        c,
        hits,
        Rect::new(r.x + 8. * s, r.y + 7. * s, 25. * s, 20. * s),
        Target::Switch(k),
        "On",
        look.s.audible(k),
        s,
    );
    button(
        c,
        hits,
        Rect::new(r.x + 9. * s, r.y + 39. * s, 86. * s, 25. * s),
        Target::BandMenu(k),
        params::TYPE_NAMES[look.s.bands[k].kind.clamp(0, 7) as usize],
        false,
        s,
    );
    button(
        c,
        hits,
        Rect::new(r.x + 9. * s, r.y + 72. * s, 86. * s, 20. * s),
        Target::OrderMark(k),
        params::ORDER_NAMES[(look.s.bands[k].order + 3).clamp(0, 6) as usize],
        false,
        s,
    );
    let b = look.s.bands[k];
    let gainx = r.x + 229. * s;
    let cy = r.y + 65. * s;
    knob(
        c,
        hits,
        r.x + 140. * s,
        cy,
        23. * s,
        b.freq,
        Target::Freq(k),
        "FREQ",
        &params::hz_text(b.hz()),
        color,
        s,
    );
    knob(
        c,
        hits,
        gainx,
        cy,
        28. * s,
        (b.db + 18.) / 36.,
        Target::Fader(k),
        "GAIN",
        &params::db_text(b.db),
        color,
        s,
    );
    let ring = 34. * s;
    let range = cfg.bands[k][0] / 18.;
    let dynamic_on = range.abs() > 0.001;
    c.stroke_circle(
        gainx,
        cy,
        ring,
        if dynamic_on { 1.6 * s } else { 0.8 * s },
        if dynamic_on {
            Color::rgb(0xf0d768).with_alpha(175)
        } else {
            Color::rgb(0x59606a).with_alpha(85)
        },
    );
    if dynamic_on {
        c.stroke_circle(
            gainx,
            cy,
            ring + 1.5 * s,
            3. * s,
            Color::rgb(0xf0d768).with_alpha(20),
        );
    }
    if range.abs() > 0.001 {
        c.arc(
            gainx,
            cy,
            ring,
            -PI * 0.5 + range.min(0.) * PI,
            range.abs() * PI,
            4. * s,
            Color::rgb(0xf5da65),
        );
        let actual = v.reduction[k] / 18.;
        c.arc(
            gainx,
            cy,
            ring - 4. * s,
            -PI * 0.5 + actual.min(0.) * PI,
            actual.abs() * PI,
            2. * s,
            Color::rgb(0xf0d768),
        );
    }
    button(
        c,
        hits,
        Rect::new(gainx - 110. * s, r.y - 5. * s, 76. * s, 18. * s),
        Target::DynamicToggle,
        "Dynamic",
        dynamic_on,
        s,
    );
    button(
        c,
        hits,
        Rect::new(gainx - 28. * s, r.y - 5. * s, 56. * s, 18. * s),
        Target::SpectralBand,
        "Spectral",
        dynamic_on && cfg.spectral & (1 << k) != 0,
        s,
    );
    button(
        c,
        hits,
        Rect::new(gainx + 40. * s, r.y - 4. * s, 26. * s, 19. * s),
        Target::ExpandDynamics,
        if expanded { "<<" } else { ">>" },
        expanded,
        s,
    );
    let qx = r.x + if expanded { 455. } else { 315. } * s;
    knob(
        c,
        hits,
        qx,
        cy,
        23. * s,
        b.width,
        Target::Q(k),
        "Q",
        &format!("{:.2}", q_of(b.width)),
        color,
        s,
    );
    if expanded {
        let tx = r.x + 300. * s;
        let tr = Rect::new(tx, r.y + 23. * s, 15. * s, 76. * s);
        c.fill_rrect(tr, 3. * s, Color::rgb(0x131a21));
        let value = (cfg.bands[k][1] + 72.) / 72.;
        c.fill_rect(
            Rect::new(
                tr.x + 3. * s,
                tr.bottom() - (v.trigger[k] + 72.).clamp(0., 72.) / 72. * tr.h,
                9. * s,
                (v.trigger[k] + 72.).clamp(0., 72.) / 72. * tr.h,
            ),
            color.with_alpha(100),
        );
        c.line(
            tr.x - 3. * s,
            tr.bottom() - value * tr.h,
            tr.right() + 3. * s,
            tr.bottom() - value * tr.h,
            2. * s,
            color,
        );
        txt(
            c,
            Rect::new(tx - 19. * s, r.y + 100. * s, 53. * s, 13. * s),
            9.,
            s,
            color,
            &format!("{:.0}dB", cfg.bands[k][1]),
        );
        hits.push(Hit {
            r: Rect::new(tx - 10. * s, tr.y - 4. * s, 35. * s, 96. * s),
            t: Target::Dynamic(k * 4 + 1),
        });
        button(
            c,
            hits,
            Rect::new(tx - 4. * s, r.y + 3. * s, 24. * s, 16. * s),
            Target::AutoDynamics,
            "A",
            cfg.auto[k],
            s,
        );
        knob(
            c,
            hits,
            r.x + 350. * s,
            r.y + 72. * s,
            14. * s,
            f64::from(cfg.bands[k][2] / 200.),
            Target::Dynamic(k * 4 + 2),
            "A",
            &format!("{:.0}ms", cfg.bands[k][2]),
            color,
            s,
        );
        knob(
            c,
            hits,
            r.x + 395. * s,
            r.y + 72. * s,
            14. * s,
            f64::from(cfg.bands[k][3] / 1500.),
            Target::Dynamic(k * 4 + 3),
            "R",
            &format!("{:.0}ms", cfg.bands[k][3]),
            color,
            s,
        );
    }
    let end = r.right() - 94. * s;
    button(
        c,
        hits,
        Rect::new(end, r.y + 41. * s, 84. * s, 22. * s),
        Target::BandMenu(k),
        eq::routed::NAMES[eq::routed::routes(look.extra, false)[k] as usize],
        false,
        s,
    );
    button(
        c,
        hits,
        Rect::new(r.right() - 92. * s, r.y + 7. * s, 22. * s, 18. * s),
        Target::PrevBand,
        "<",
        false,
        s,
    );
    txt(
        c,
        Rect::new(r.right() - 68. * s, r.y + 7. * s, 24. * s, 18. * s),
        10.,
        s,
        color,
        &(k + 1).to_string(),
    );
    button(
        c,
        hits,
        Rect::new(r.right() - 43. * s, r.y + 7. * s, 22. * s, 18. * s),
        Target::NextBand,
        ">",
        false,
        s,
    );
    button(
        c,
        hits,
        Rect::new(r.right() - 19. * s, r.y + 7. * s, 15. * s, 18. * s),
        Target::DeleteBand,
        "x",
        false,
        s,
    );
    button(
        c,
        hits,
        Rect::new(end + 17. * s, r.y + 78. * s, 50. * s, 21. * s),
        Target::Solo(k),
        "Solo",
        false,
        s,
    );
}
