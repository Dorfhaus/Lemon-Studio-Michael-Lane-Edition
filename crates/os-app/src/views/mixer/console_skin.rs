// SPDX-License-Identifier: GPL-3.0-or-later
//! Resolve the personal full-mixer finish once per frame. Layout and input stay shared.
use super::{Color, KnobId, PI, Rect, Ui, analog, clean, original, themed};

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Finish {
    Original,
    CleanModern,
    Analog,
    Theme,
}
impl Finish {
    pub(crate) fn current(appearance: os_theme::appearance::Appearance) -> Self {
        use os_theme::appearance::Mixer;
        if let Some(mixer) = appearance.mixer {
            return match mixer {
                Mixer::Original => Self::Original,
                Mixer::CleanModern => Self::CleanModern,
                Mixer::Analog => Self::Analog,
                Mixer::Theme => Self::Theme,
            };
        }
        match crate::prefs::index(&crate::prefs::MIXER_STYLE) {
            1 => Self::CleanModern,
            2 => Self::Analog,
            _ => Self::Original,
        }
    }
}

macro_rules! painter {
    ($name:ident ($($arg:ident : $ty:ty),*) -> $ret:ty) => {
        pub(super) fn $name(ui: &mut Ui, $($arg:$ty),*) -> $ret {
            match ui.mixer_finish {
                Finish::Original => original::$name(ui,$($arg),*),
                Finish::CleanModern => clean::$name(ui,$($arg),*),
                Finish::Analog => analog::$name(ui,$($arg),*),
                Finish::Theme => themed::$name(ui,$($arg),*),
            }
        }
    };
}
pub(super) fn panel(ui: &mut Ui, r: Rect, base: Color, selected: bool) {
    if ui.mixer_skin.uses(crate::mixer_skin::Part::Panel) {
        let t = ui.mixer_skin.palette(ui.t);
        return crate::mixer_skin::panel(ui.c, t, ui.mixer_skin, r, selected);
    }
    match ui.mixer_finish {
        Finish::Original => original::panel(ui, r, base, selected),
        Finish::CleanModern => clean::panel(ui, r, base, selected),
        Finish::Analog => analog::panel(ui, r, base, selected),
        Finish::Theme => themed::panel(ui, r, base, selected),
    }
}
painter!(well(r: Rect) -> ());
painter!(plate(r: Rect) -> ());
painter!(lamp(x: f32, y: f32, on: bool, col: Color, r: f32) -> ());
pub(super) fn button(ui: &mut Ui, r: Rect, on: bool, hover: bool, col: Color) -> Color {
    if ui.mixer_skin.uses(crate::mixer_skin::Part::Button) {
        let t = ui.mixer_skin.palette(ui.t);
        let state = os_ui::kit::State::new(hover, ui.pressed_here(r), on);
        return crate::mixer_skin::button(ui.c, t, ui.mixer_skin, r, state);
    }
    match ui.mixer_finish {
        Finish::Original => original::button(ui, r, on, hover, col),
        Finish::CleanModern => clean::button(ui, r, on, hover, col),
        Finish::Analog => analog::button(ui, r, on, hover, col),
        Finish::Theme => themed::button(ui, r, on, hover, col),
    }
}
painter!(signal_switch(r: Rect, on: bool, hover: bool, swap: bool) -> ());
painter!(rocker(r: Rect, on: bool, hover: bool) -> ());
pub(super) fn fader(ui: &mut Ui, x: f32, y: f32, selected: bool, hover: bool) {
    if ui.mixer_skin.uses(crate::mixer_skin::Part::Fader) {
        let t = ui.mixer_skin.palette(ui.t);
        return crate::mixer_skin::fader(
            ui.c,
            t,
            ui.mixer_skin,
            Rect::new(x - 8.0, y - 13.0, 16.0, 26.0),
            selected,
            hover,
        );
    }
    match ui.mixer_finish {
        Finish::Original => original::fader(ui, x, y, selected, hover),
        Finish::CleanModern => clean::fader(ui, x, y, selected, hover),
        Finish::Analog => analog::fader(ui, x, y, selected, hover),
        Finish::Theme => themed::fader(ui, x, y, selected, hover),
    }
}
pub(super) fn vu(ui: &mut Ui, r: Rect, levels: [f32; 2], clipped: bool, selected: bool) {
    if ui.mixer_skin.uses(crate::mixer_skin::Part::Meter) {
        let t = ui.mixer_skin.palette(ui.t);
        return crate::mixer_skin::meter(
            ui.c,
            t,
            ui.mixer_skin,
            r,
            levels.map(|v| crate::meters::vu_fraction(crate::meters::vu_db(v))),
            clipped,
        );
    }
    match ui.mixer_finish {
        Finish::Original => original::vu(ui, r, levels, clipped, selected),
        Finish::CleanModern => clean::vu(ui, r, levels, clipped, selected),
        Finish::Analog => analog::vu(ui, r, levels, clipped, selected),
        Finish::Theme => themed::vu(ui, r, levels, clipped, selected),
    }
}
painter!(separator(r: Rect) -> ());

pub(super) fn knob(
    ui: &mut Ui,
    id: KnobId,
    x: f32,
    y: f32,
    r: f32,
    value: f32,
    cap: Color,
    hover: bool,
) {
    knob_impl(ui, id, x, y, r, value, cap, hover, false);
}

fn knob_impl(ui: &mut Ui, id: KnobId, x: f32, y: f32, r: f32, value: f32,
    cap: Color, hover: bool, body_only: bool) {
    if ui.t.style.vintage_2005 {
        let size = (r * 2.0 + 2.0).round().clamp(7.0, 22.0);
        let bounds = Rect::new(x - size / 2.0, y - size / 2.0, size, size);
        crate::vintage_rack_art::knob(ui.c, bounds, value);
        return;
    }
    if ui.mixer_skin.uses(crate::mixer_skin::Part::Knob) {
        let t = ui.mixer_skin.palette(ui.t);
        let bipolar = (id.default_value() - 0.5).abs() < 0.0001;
        return crate::mixer_skin::knob_body(ui.c, t, ui.mixer_skin, x, y, r, value, bipolar, hover, body_only);
    }
    match ui.mixer_finish {
        Finish::Theme => {
            let bipolar = (id.default_value() - 0.5).abs() < 0.0001;
            let style =
                ui.t.appearance
                    .knob
                    .unwrap_or(os_theme::appearance::Knob::Ring);
            let mut appearance = ui.t.appearance;
            appearance.knob_body_only = body_only;
            os_ui::kit::styled_knob_appearance(ui.c, ui.t, x, y, r, value, bipolar, hover, style, &appearance);
            return;
        }
        Finish::Original => {
            original::knob(ui, x, y, r, value, cap, hover);
            return;
        }
        Finish::CleanModern => clean::knob(ui, x, y, r, value, cap, hover),
        Finish::Analog => analog::knob_body(ui, x, y, r, value, Color::rgb(0x484848), hover, body_only),
    }
    let neutral = if (id.default_value() - 0.5).abs() < 0.0001 {
        0.5
    } else {
        0.0
    };
    let value = value.clamp(0., 1.);
    if !body_only && (value - neutral).abs() > 0.0001 {
        let lo = value.min(neutral);
        let start = (-1.25 + 1.5 * lo) * PI;
        let radius = if ui.mixer_finish == Finish::Analog {
            r + 2.1
        } else {
            r
        };
        ui.c.arc(
            x,
            y,
            radius,
            start,
            1.5 * PI * (value - neutral).abs(),
            1.5,
            Color::rgb(0x2096f3),
        );
    }
}

/// Traditional insert slots retain their old rendering until explicitly customized.
/// Cap artwork is shared with Full Mixer; decorations are independent and bounded.
pub(crate) fn insert_knob(ui: &mut Ui, id: KnobId, x: f32, y: f32, q: f32, value: f32) {
    let a = ui.t.appearance;
    if !a.insert_custom {
        super::mixer_knob(ui, x, y, 10.5 * q, 12.0 * q, value, Some(false));
        ui.c.stroke_circle(x, y, 11.8 * q, 1.5, ui.t.s(0x55595c));
        return;
    }
    let hover = ui.hovering(Rect::new(x - 13.0 * q, y - 13.0 * q, 26.0 * q, 26.0 * q));
    if a.insert_full_knob {
        knob_impl(ui, id, x, y, 7.5 * q, value, Color::rgb(0xc2c7c9), hover, true);
    } else {
        let mut appearance = a;
        appearance.knob_body_only = true;
        if a.insert_traditional_knob { appearance.knob = None; }
        if let Some(style) = a.insert_knob { appearance.knob = Some(style); }
        os_ui::kit::knob_appearance(ui.c, ui.t, x, y, 8.5 * q, value, false,
            os_ui::kit::KnobLook::Mixer { halo: 10.0 * q, arc: false }, os_ui::kit::State::hover(hover), &appearance);
        // The legacy Console ball uses its arc as the value indicator. Keep a
        // pointer readable when the user hides that arc and its surround.
        if appearance.knob.is_none() && !ui.t.style.lemon() && !ui.t.style.vintage_2005 {
            let angle = (0.75 + 1.5 * value.clamp(0.0, 1.0)) * PI;
            ui.c.line(x + 2.0 * q * angle.cos(), y + 2.0 * q * angle.sin(),
                x + 6.5 * q * angle.cos(), y + 6.5 * q * angle.sin(), 1.2 * q, ui.t.glyph);
        }
    }
    let ring_width = a.insert_ring_width.unwrap_or(1.0) * q;
    if a.insert_ring != Some(false) && ring_width > 0.0 && value > 0.0 {
        ui.c.arc(x, y, 10.6 * q, 0.75 * PI, 1.5 * PI * value.clamp(0.0, 1.0), ring_width, ui.t.insert_knob_ring);
    }
    let rim_width = a.insert_rim_width.unwrap_or(1.0) * q;
    if a.insert_rim != Some(false) && rim_width > 0.0 {
        ui.c.stroke_circle(x, y, 12.8 * q, rim_width, ui.t.insert_knob_rim);
    }
}
