// SPDX-License-Identifier: GPL-3.0-or-later
use super::content::{Block, menu_groups};
use super::{Hit, Page, View, button, chapters, hit, results, topics, wrap_lines};
use crate::app::{Action, Scroll, State, Ui};
use crate::views::settings::SAct;
use os_ui::{Color, Font, Rect};
use std::sync::OnceLock;

const BG: Color = Color::rgb(0x171b22);
const SIDEBAR: Color = Color::rgb(0x1e242e);
const PANEL: Color = Color::rgb(0x232b36);
const HOVER: Color = Color::rgb(0x2d3b4b);
const SELECTED: Color = Color::rgb(0x29425a);
const TEXT: Color = Color::rgb(0xf3f6fa);
const COPY: Color = Color::rgb(0xd9e1eb);
const MUTED: Color = Color::rgb(0xaab9ca);
const LINK: Color = Color::rgb(0x99caff);
const RULE: Color = Color::rgb(0x344154);
const HEADER: f32 = 69.;
const SIDE: f32 = 221.;
const READING_WIDTH: f32 = 792.;

fn rect((w, h): (f32, f32), detached: bool) -> Rect {
    if detached {
        return Rect::new(0., 0., w, h);
    }
    let width = (w - 24.).clamp(270., 1180.);
    let height = (h - 36.).clamp(205., 860.);
    Rect::new(
        ((w - width) / 2.).max(0.).round(),
        ((h - height) / 2.).max(0.).round(),
        width,
        height,
    )
}

fn sidebar(r: Rect) -> f32 {
    if r.w >= 820. && r.h >= HEADER + 77. + 28. * chapters().len() as f32 {
        SIDE
    } else {
        0.
    }
}

pub(super) fn body(screen: (f32, f32), detached: bool) -> Rect {
    let r = rect(screen, detached);
    let available = (r.w - sidebar(r) - 52.).max(60.);
    let width = available.min(READING_WIDTH);
    let x = r.x + sidebar(r) + 22. + (available - width) * 0.5;
    Rect::new(x, r.y + HEADER + 15., width, (r.h - HEADER - 30.).max(1.))
}

type Picture = (usize, usize, Vec<u32>);
type PictureEntry = (&'static str, &'static [u8], OnceLock<Option<Picture>>);
macro_rules! shot {
    ($key:literal, $file:literal) => {
        ($key, include_bytes!($file) as &[u8], OnceLock::new())
    };
}

fn image(name: &str) -> Option<&'static Picture> {
    static SHOTS: [PictureEntry; 14] = [
        shot!("workspace", "../../../../docs/assets/guide/workspace.jpg"),
        shot!("audio", "../../../../docs/assets/guide/audio.jpg"),
        shot!("midi", "../../../../docs/assets/guide/midi.jpg"),
        shot!("browser", "../../../../docs/assets/guide/browser.jpg"),
        shot!("controls", "../../../../docs/assets/guide/controls.jpg"),
        shot!("visibility", "../../../../docs/assets/guide/visibility.jpg"),
        shot!("mixer", "../../../../docs/assets/guide/mixer.jpg"),
        shot!("piano", "../../../../docs/assets/guide/piano.jpg"),
        shot!("playlist", "../../../../docs/assets/guide/playlist.jpg"),
        shot!("rack", "../../../../docs/assets/guide/rack.jpg"),
        shot!("picker", "../../../../docs/assets/guide/picker.jpg"),
        shot!("warp", "../../../../docs/assets/guide/warp.jpg"),
        shot!(
            "playlist-menu",
            "../../../../docs/assets/guide/playlist-menu.jpg"
        ),
        shot!("piano-menu", "../../../../docs/assets/guide/piano-menu.jpg"),
    ];
    SHOTS
        .iter()
        .find(|(key, ..)| *key == name)
        .and_then(|(_, bytes, cache)| {
            cache
                .get_or_init(|| crate::wallpaper::decode(bytes))
                .as_ref()
        })
        .or_else(|| crate::plugin_artwork::stock(name))
}

pub(super) fn picture_for(name: &str) -> Option<&'static Picture> {
    image(name.split_once(" | ").map_or(name, |(key, _)| key))
}

fn caption(name: &str) -> &str {
    name.split_once(" | ").map_or(name, |(_, caption)| caption)
}

pub(super) fn paint(ui: &mut Ui, s: &State, detached: bool) {
    let view = &s.guide;
    let r = rect(s.screen, detached);
    let body = body(s.screen, detached);
    let side = sidebar(r);
    ui.hit(r, Action::Settings(SAct::Body), "");
    if !detached {
        ui.c.fill_rrect(r.outset(5.), 12., Color::rgb(0).with_alpha(105));
    }
    ui.c.fill_rrect(r, if detached { 0. } else { 8. }, BG);
    ui.c.fill_rect(Rect::new(r.x, r.y + HEADER - 1., r.w, 1.), RULE);
    header(ui, view, r, detached);

    if side > 0. {
        let rail = Rect::new(r.x + 1., r.y + HEADER, side - 1., r.h - HEADER - 1.);
        ui.c.fill_rect(rail, SIDEBAR);
        ui.c.fill_rect(Rect::new(rail.right(), rail.y, 1., rail.h), RULE);
        navigation(ui, view, rail);
    }

    let offset = view
        .scroll
        .clamp(0., (view.search.content_height.get() - body.h).max(0.));
    ui.c.push_clip(body);
    ui.clip_hit_rect(body, |ui| {
        let mut reader = Reader {
            ui,
            body,
            y: body.y - offset,
        };
        if view.searching() {
            reader.search_results(view);
        } else {
            match view.page {
                Page::Home => reader.home(),
                Page::Chapter(index) => reader.chapter(index),
                Page::Topic(index) => reader.article(index),
            }
        }
        view.search
            .content_height
            .set((reader.y - body.y + offset + 6.).max(0.));
    });
    ui.c.pop_clip();

    let total = view.search.content_height.get();
    if total > body.h {
        crate::views::scrollbar(
            ui,
            Rect::new(r.right() - 14., body.y, 8., body.h),
            true,
            offset.min(total - body.h),
            body.h,
            total,
            Scroll::Guide,
        );
    }
    if view.picture
        && let Some(name) = view.topic().and_then(|index| topics()[index].picture)
    {
        lightbox(ui, r, name);
    }
}

fn header(ui: &mut Ui, view: &View, r: Rect, detached: bool) {
    let roomy = r.w >= 660.;
    let left = r.x + 16.;
    let close = Rect::new(r.right() - 68., r.y + 11., 52., 26.);
    let detach = Rect::new(close.x - 79., close.y, 71., close.h);
    button(ui, close, "Close", Hit::Close);
    let show_detach = detached || crate::workspace_visibility::shown(super::DETACH_VISIBILITY);
    if show_detach {
        button(
            ui,
            detach,
            if detached { "Dock" } else { "Detach" },
            Hit::Detach,
        );
    }
    let home = Rect::new(
        left,
        r.y + 10.,
        if roomy { 185. } else { (r.w - 162.).max(75.) },
        29.,
    );
    ui.hit(home, hit(Hit::Home), "User Guide home");
    ui.c.text_clip(
        Font::new(if roomy { 22 } else { 19 }).bold(),
        left,
        home.y - 1.,
        home.w,
        TEXT,
        "User Guide",
    );
    let sx = if roomy {
        r.x + sidebar(r).max(180.) + 17.
    } else {
        left
    };
    let sy = r.y + if roomy { 15. } else { 40. };
    let right = if roomy {
        if show_detach {
            detach.x - 17.
        } else {
            close.x - 17.
        }
    } else {
        r.right() - 16.
    };
    let search = Rect::new(
        sx,
        sy,
        (right - sx).max(60.).min(625.),
        if roomy { 37. } else { 23. },
    );
    let clear = Rect::new(search.right() - 33., search.y, 33., search.h);
    ui.hit(search, hit(Hit::Search), "Search the User Guide (Ctrl+K)");
    ui.c.fill_rrect(search, 5., if view.search.focused { LINK } else { RULE });
    ui.c.fill_rrect(search.inset(1.), 4., SIDEBAR);
    let font = Font::new(14);
    let line = ui.c.line_height(font);
    let y = (search.cy() - line / 2.).round();
    let field = &view.search.text;
    let empty = field.text.is_empty();
    let text = Rect::new(
        search.x + 9.,
        search.y + 1.,
        search.w - if empty { 18. } else { 42. },
        search.h - 2.,
    );
    ui.c.push_clip(text);
    if empty {
        ui.c.text_clip(
            font,
            text.x,
            y,
            text.w,
            MUTED,
            "Search articles, tools and shortcuts",
        );
    } else {
        let caret = ui.c.text_width(font, field.before_caret());
        let x = text.x - (caret - text.w + 2.).max(0.);
        if view.search.focused && field.all {
            let width = ui.c.text_width(font, &field.text);
            ui.c.fill_rect(Rect::new(x, y, width, line), SELECTED);
        }
        ui.c.text(font, x, y, TEXT, &field.text);
        if view.search.focused && !field.all {
            ui.c.fill_rect(Rect::new(x + caret, y, 1., line), TEXT);
        }
    }
    if empty && view.search.focused {
        ui.c.fill_rect(Rect::new(text.x, y, 1., line), LINK);
    }
    ui.c.pop_clip();
    if !empty {
        button(ui, clear.inset(2.), "×", Hit::Clear);
    }
}

fn navigation(ui: &mut Ui, view: &View, rail: Rect) {
    let mut row = Rect::new(rail.x + 10., rail.y + 9., rail.w - 20., 27.);
    nav_row(
        ui,
        row,
        "Home",
        Hit::Home,
        view.page == Page::Home && !view.searching(),
    );
    row.y += 38.;
    ui.c.text_clip(
        Font::new(11).bold(),
        row.x + 8.,
        row.y,
        row.w - 16.,
        MUTED,
        "CONTENTS",
    );
    row.y += 19.;
    let current = match view.page {
        Page::Home => None,
        Page::Chapter(index) => Some(index),
        Page::Topic(index) => Some(topics()[index].chapter),
    };
    for (index, chapter) in chapters().iter().enumerate() {
        nav_row(
            ui,
            row,
            chapter.title,
            Hit::Section(index),
            current == Some(index),
        );
        row.y += 28.;
    }
}

fn nav_row(ui: &mut Ui, r: Rect, title: &'static str, action: Hit, active: bool) {
    let hover = ui.hit(r, hit(action), title);
    if active || hover {
        ui.c.fill_rrect(r, 5., if active { SELECTED } else { HOVER });
    }
    if active {
        ui.c.fill_rrect(Rect::new(r.x, r.y + 5., 2., r.h - 10.), 1., LINK);
    }
    ui.c.text_clip(
        Font::new(13),
        r.x + 8.,
        r.cy() - 8.,
        r.w - 13.,
        if active { TEXT } else { COPY },
        title,
    );
}

struct Reader<'a, 'ui> {
    ui: &'a mut Ui<'ui>,
    body: Rect,
    y: f32,
}

impl Reader<'_, '_> {
    fn text(&mut self, value: &str, font: Font, color: Color, indent: f32, leading: f32) {
        let width = (self.body.w - indent).max(1.);
        for line in wrap_lines(self.ui, font, value, width) {
            if self.y + leading > self.body.y && self.y < self.body.bottom() {
                self.ui
                    .c
                    .text_clip(font, self.body.x + indent, self.y, width, color, &line);
            }
            self.y += leading;
        }
    }

    fn title(&mut self, title: &str) {
        self.text(title, Font::new(28).bold(), TEXT, 0., 33.);
        self.y += 8.;
    }

    fn copy(&mut self, value: &str) {
        self.text(
            &value.replace("**", "").replace('`', ""),
            Font::new(15),
            COPY,
            0.,
            21.,
        );
        self.y += 12.;
    }

    fn label(&mut self, value: &str) {
        self.text(value, Font::new(13), MUTED, 0., 18.);
        self.y += 6.;
    }

    fn card(&mut self, r: Rect, title: &str, description: &str, action: Hit) {
        if r.bottom() <= self.body.y || r.y >= self.body.bottom() {
            return;
        }
        let hover = self.ui.hit(r, hit(action), title.to_owned());
        self.ui.c.fill_rrect(r, 6., RULE);
        self.ui
            .c
            .fill_rrect(r.inset(1.), 5., if hover { HOVER } else { PANEL });
        let font = Font::new(15).bold();
        let title_lines = wrap_lines(self.ui, font, title, r.w - 25.);
        for (index, line) in title_lines.iter().take(2).enumerate() {
            self.ui.c.text_clip(
                font,
                r.x + 12.,
                r.y + 7. + 18. * index as f32,
                r.w - 24.,
                LINK,
                line,
            );
        }
        let dy = 9. + 18. * title_lines.len().min(2) as f32;
        let small = Font::new(12);
        let lines = wrap_lines(self.ui, small, description, r.w - 25.);
        for (index, line) in lines
            .iter()
            .take(((r.h - dy - 5.) / 16.).max(0.) as usize)
            .enumerate()
        {
            self.ui.c.text_clip(
                small,
                r.x + 12.,
                r.y + dy + 16. * index as f32,
                r.w - 24.,
                MUTED,
                line,
            );
        }
    }

    fn row(&mut self, index: usize, description: &str) {
        let topic = &topics()[index];
        let height = if self.body.w < 425. { 70. } else { 53. };
        let row = Rect::new(self.body.x, self.y, self.body.w, height);
        self.card(row, topic.title, description, Hit::Topic(index));
        self.y += height + 7.;
    }

    fn home(&mut self) {
        self.title("Learn ML Studio");
        self.copy("Make a first beat, record a performance or look up a tool while you work. The whole guide is available offline.");
        let columns = if self.body.w >= 555. { 2 } else { 1 };
        let width = (self.body.w - 10. * (columns - 1) as f32) / columns as f32;
        for (index, chapter) in chapters().iter().enumerate() {
            let x = self.body.x + (index % columns) as f32 * (width + 10.);
            let y = self.y + (index / columns) as f32 * 84.;
            self.card(
                Rect::new(x, y, width, 75.),
                chapter.title,
                chapter.intro,
                Hit::Section(index),
            );
        }
        self.y += chapters().len().div_ceil(columns) as f32 * 84.;
    }

    fn chapter(&mut self, index: usize) {
        let chapter = &chapters()[index];
        self.toolbar(&[("‹ Home", Hit::Home)]);
        self.title(chapter.title);
        self.copy(chapter.intro);
        for index in chapter.topics.clone() {
            self.row(index, topics()[index].keywords);
        }
    }

    fn search_results(&mut self, view: &View) {
        let matches = results(&view.search.text.text);
        self.title("Search results");
        if matches.is_empty() {
            self.copy("No matching articles. Try a shorter phrase, a control name or a shortcut.");
            return;
        }
        let suffix = if matches.len() == 1 {
            "article"
        } else {
            "articles"
        };
        self.label(&format!(
            "{} {suffix} · Enter opens the first result",
            matches.len()
        ));
        for index in matches {
            self.row(index, chapters()[topics()[index].chapter].title);
        }
    }

    fn toolbar(&mut self, buttons: &[(&str, Hit)]) {
        let mut x = self.body.x;
        for &(label, action) in buttons {
            let width =
                (self.ui.c.text_width(Font::new(13), label) + 18.).min(self.body.right() - x);
            if width < 25. {
                break;
            }
            if self.y + 25. > self.body.y && self.y < self.body.bottom() {
                button(self.ui, Rect::new(x, self.y, width, 25.), label, action);
            }
            x += width + 8.;
        }
        self.y += 33.;
    }

    fn article(&mut self, index: usize) {
        let topic = &topics()[index];
        let chapter = &chapters()[topic.chapter];
        let show = topic.destination.is_some() && super::spotlight::enabled();
        let buttons = [("‹ Back", Hit::Back), ("Show me where", Hit::ShowWhere)];
        self.toolbar(&buttons[..if show { 2 } else { 1 }]);
        self.label(chapter.title);
        self.title(topic.title);
        for (position, block) in topic.blocks.iter().enumerate() {
            match *block {
                Block::Paragraph(value) => self.copy(value),
                Block::Heading(value) => {
                    self.y += 5.;
                    self.text(value, Font::new(18).bold(), TEXT, 0., 23.);
                    self.y += 5.;
                }
                Block::ListItem(value, numbered) => {
                    let value = value.replace("**", "").replace('`', "");
                    let (marker, text) = if numbered {
                        value
                            .split_once(". ")
                            .map_or((String::new(), value.as_str()), |(n, text)| {
                                (format!("{n}."), text)
                            })
                    } else {
                        ("•".to_owned(), value.as_str())
                    };
                    if self.y + 22. > self.body.y && self.y < self.body.bottom() {
                        self.ui.c.text_clip(
                            Font::new(14).bold(),
                            self.body.x + 1.,
                            self.y,
                            23.,
                            LINK,
                            &marker,
                        );
                    }
                    self.text(text, Font::new(15), COPY, 25., 21.);
                    self.y += 5.;
                }
                Block::MenuReference(filter) => {
                    self.text("Command reference", Font::new(18).bold(), TEXT, 0., 27.);
                    for (_, menu, items) in
                        menu_groups().filter(|(index, ..)| filter.is_none_or(|menu| menu == *index))
                    {
                        if filter.is_none() {
                            self.text(menu, Font::new(16).bold(), TEXT, 0., 24.);
                        }
                        for item in items {
                            let shortcut = if item.shortcut.is_empty() {
                                String::new()
                            } else {
                                format!("  ·  {}", item.shortcut)
                            };
                            self.text(
                                &format!("{}{}", item.label, shortcut),
                                Font::new(14),
                                COPY,
                                8.,
                                20.,
                            );
                        }
                        self.y += 14.;
                    }
                }
            }
            if position == 0
                && let Some(name) = topic.picture
            {
                self.figure(name, 342.);
            }
        }
        let links: Vec<_> = topic
            .related
            .iter()
            .filter_map(|title| topics().iter().position(|other| other.title == *title))
            .collect();
        if !links.is_empty() {
            self.y += 12.;
            self.text("Related articles", Font::new(18).bold(), TEXT, 0., 27.);
            for related in links {
                self.row(related, chapters()[topics()[related].chapter].title);
            }
        }
        self.y += 8.;
        let mut neighbors = Vec::new();
        if index > chapter.topics.start {
            neighbors.push(("‹ Previous article", Hit::Topic(index - 1)));
        }
        if index + 1 < chapter.topics.end {
            neighbors.push(("Next article ›", Hit::Topic(index + 1)));
        }
        if !neighbors.is_empty() {
            self.toolbar(&neighbors);
        }
    }

    fn figure(&mut self, name: &str, limit: f32) {
        let Some(picture) = picture_for(name) else {
            return;
        };
        let (w, h, pixels) = picture;
        let scale = (self.body.w / *w as f32).min(limit / *h as f32).min(1.);
        let width = (*w as f32 * scale).round();
        let height = (*h as f32 * scale).round();
        let r = Rect::new(self.body.x, self.y + 2., width, height);
        if r.bottom() > self.body.y && r.y < self.body.bottom() {
            let hover = self.ui.hit(r, hit(Hit::Picture), "Enlarge picture");
            self.ui
                .c
                .fill_rect(r.outset(1.), if hover { LINK } else { RULE });
            self.ui.c.image(r, *w, *h, pixels);
        }
        self.y += height + 9.;
        self.label(&format!("{} · Click to enlarge", caption(name)));
        self.y += 9.;
    }
}

fn lightbox(ui: &mut Ui, r: Rect, name: &str) {
    let Some((w, h, pixels)) = picture_for(name) else {
        return;
    };
    ui.hit(r, hit(Hit::Picture), "Close picture");
    ui.c.fill_rect(r, BG);
    let scale = ((r.w - 36.) / *w as f32).min((r.h - 86.) / *h as f32);
    let width = (*w as f32 * scale).round();
    let height = (*h as f32 * scale).round();
    let image = Rect::new(
        r.cx() - width / 2.,
        r.cy() - height / 2. + 2.,
        width,
        height,
    );
    ui.c.fill_rect(image.outset(1.), RULE);
    ui.c.image(image, *w, *h, pixels);
    ui.c.text_clip(
        Font::new(13),
        r.x + 16.,
        r.bottom() - 26.,
        r.w - 32.,
        COPY,
        caption(name),
    );
    button(
        ui,
        Rect::new(r.right() - 114., r.y + 9., 98., 26.),
        "Close picture",
        Hit::Picture,
    );
}
