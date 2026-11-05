// SPDX-License-Identifier: GPL-3.0-or-later
//! Resolve the personal full-mixer finish once per frame. Layout and input stay shared.
use super::{analog, clean, original, Color, KnobId, Rect, Ui, PI};

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Finish {
    Original,
    CleanModern,
    Analog,
}
impl Finish {
    pub(crate) fn current() -> Self {
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
            }
        }
    };
}
painter!(panel(r: Rect, base: Color, selected: bool) -> ());
painter!(well(r: Rect) -> ());
painter!(plate(r: Rect) -> ());
painter!(lamp(x: f32, y: f32, on: bool, col: Color, r: f32) -> ());
painter!(button(r: Rect, on: bool, hover: bool, col: Color) -> Color);
painter!(signal_switch(r: Rect, on: bool, hover: bool, swap: bool) -> ());
painter!(rocker(r: Rect, on: bool, hover: bool) -> ());
painter!(fader(x: f32, y: f32, selected: bool, hover: bool) -> ());
painter!(vu(r: Rect, levels: [f32; 2], clipped: bool, selected: bool) -> ());
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
    match ui.mixer_finish {
        Finish::Original => {
            original::knob(ui, x, y, r, value, cap, hover);
            return;
        }
        Finish::CleanModern => clean::knob(ui, x, y, r, value, cap, hover),
        Finish::Analog => analog::knob(ui, x, y, r, value, Color::rgb(0x484848), hover),
    }
    let neutral = if (id.default_value() - 0.5).abs() < 0.0001 {
        0.5
    } else {
        0.0
    };
    let value = value.clamp(0., 1.);
    if (value - neutral).abs() > 0.0001 {
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
            Color::rgb(0xe8c74f),
        );
    }
}
