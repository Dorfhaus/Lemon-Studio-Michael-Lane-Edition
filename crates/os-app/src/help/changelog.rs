// SPDX-License-Identifier: GPL-3.0-or-later
use super::{Hit, button, hit};
use crate::app::{Action, Scroll, State, Ui};
use crate::views::settings::{self, SAct};
use os_ui::{Color, Font, Rect};

fn changelog_rect(s: &State) -> Rect {
    let (width, height) = s.screen;
    let w = (width - 24.).clamp(220., 840.);
    let h = (height - 36.).clamp(180., 740.);
    Rect::new(
        ((width - w) / 2.).max(0.).round(),
        ((height - h) / 2.).max(0.).round(),
        w,
        h,
    )
}

pub fn changelog_body(s: &State) -> Rect {
    let r = changelog_rect(s);
    Rect::new(r.x + 16., r.y + 117., r.w - 38., r.h - 130.)
}

pub(super) fn paint(ui: &mut Ui, s: &State) {
    let r = changelog_rect(s);
    let body = changelog_body(s);
    let st = &s.settings;
    let view = &st.help;
    let t = ui.t;

    ui.hit(r, Action::Settings(SAct::Body), "");
    ui.c.fill_rrect(r.outset(4.), 10., Color::rgb(0x000000).with_alpha(80));
    ui.c.fill_rrect(r, 8., t.window_title);
    ui.c.fill_rrect(r.inset(1.), 7., t.s(0x25282b));

    ui.c.text_clip(
        Font::new(21).bold(),
        r.x + 16.,
        r.y + 12.,
        r.w - 98.,
        t.text,
        crate::change_log::LABEL,
    );
    button(
        ui,
        Rect::new(r.right() - 75., r.y + 12., 59., 26.),
        "Close",
        Hit::Close,
    );
    ui.c.text_clip(
        Font::new(12),
        r.x + 16.,
        r.y + 41.,
        r.w - 32.,
        t.text_dim,
        "Michael Lane Edition release history · available offline",
    );

    let search = Rect::new(r.x + 16., r.y + 61., r.w - 100., 30.);
    let hover = ui.hit(search, hit(Hit::Search), "Search the Change Log");
    crate::views::sunken(ui, search, t.sunken);
    settings::cell_text(
        ui,
        search,
        &view.text.text,
        view.focused.then_some(&view.text),
        hover,
    );
    if view.text.text.is_empty() {
        ui.c.text_clip(
            Font::new(12),
            search.x + 7.,
            search.y + 7.,
            search.w - 14.,
            t.text_dim,
            "Search by date or change",
        );
    }
    button(
        ui,
        Rect::new(search.right() + 7., search.y, 61., search.h),
        "Clear",
        Hit::Clear,
    );

    let offset = st
        .scroll
        .clamp(0., (view.content_height.get() - body.h).max(0.));
    ui.c.push_clip(body);
    let total = crate::change_log::paint_rows(ui, body, offset, &view.text.text);
    ui.c.pop_clip();
    view.content_height.set(total);
    ui.c.text_clip(
        Font::new(12),
        body.x,
        r.y + 98.,
        body.w,
        t.text_dim,
        "Newest changes first",
    );
    if total > body.h {
        crate::views::scrollbar(
            ui,
            Rect::new(body.right() + 7., body.y, 8., body.h),
            true,
            offset.min(total - body.h),
            body.h,
            total,
            Scroll::Settings,
        );
    }
}
