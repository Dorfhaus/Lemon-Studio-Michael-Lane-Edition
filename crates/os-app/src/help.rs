// SPDX-License-Identifier: GPL-3.0-or-later
//! Bundled user guide and shared controls for the offline Change Log.
use crate::app::{Action, App, State, Ui};
use crate::views::settings::{self, SAct};
use os_ui::{Font, Rect};
use std::cell::Cell;
use winit::keyboard::{Key, NamedKey};

mod changelog;
mod content;
mod spotlight;
mod wiki;

use content::{chapters, results, topics};
pub(crate) use spotlight::{active, paint as paint_spotlight};

pub const LABEL: &str = "User Guide";
pub const VISIBILITY: &str = "visible_help-guide";
pub const SPOTLIGHT_VISIBILITY: &str = "visible_help-show-where";
pub const DETACH_VISIBILITY: &str = "visible_help-guide-detach";
pub const PAGE: usize = settings::TABS.len();
const DETACHED_PREF: &str = "user_guide_detached";

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Hit {
    Search,
    Clear,
    Topic(usize),
    Section(usize),
    Home,
    Back,
    Picture,
    Detach,
    ShowWhere,
    ReturnToGuide,
    DismissSpotlight,
    Close,
}

#[derive(Debug, Default)]
pub struct Search {
    text: crate::dialog::Field,
    focused: bool,
    pub content_height: Cell<f32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
enum Page {
    #[default]
    Home,
    Chapter(usize),
    Topic(usize),
}

#[derive(Debug, Default)]
pub struct View {
    search: Search,
    page: Page,
    back: Vec<(Page, f32)>,
    pub scroll: f32,
    picture: bool,
    detached_open: bool,
    pub focus_window: bool,
    spotlight: Option<spotlight::Visit>,
}

impl View {
    fn topic(&self) -> Option<usize> {
        match self.page {
            Page::Topic(index) => Some(index),
            _ => None,
        }
    }

    fn navigate(&mut self, page: Page) {
        if self.page != page {
            self.back.push((self.page, self.scroll));
            self.page = page;
            self.scroll = 0.;
        }
        self.picture = false;
        self.search.focused = false;
    }

    fn home(&mut self) {
        self.search.text.set("");
        self.search.focused = false;
        self.page = Page::Home;
        self.back.clear();
        self.picture = false;
        self.scroll = 0.;
    }

    fn back(&mut self) {
        self.picture = false;
        self.search.focused = false;
        if let Some((page, scroll)) = self.back.pop() {
            self.page = page;
            self.scroll = scroll;
        } else {
            self.home();
        }
    }

    fn searching(&self) -> bool {
        !matches!(self.page, Page::Topic(_)) && !self.search.text.text.trim().is_empty()
    }

    pub fn content_height(&self) -> f32 {
        self.search.content_height.get()
    }

    pub fn scroll_by(&mut self, amount: f32, height: f32) {
        let limit = (self.search.content_height.get() - height).max(0.);
        self.scroll = (self.scroll + amount).clamp(0., limit);
    }
}

pub fn is_open(s: &State) -> bool {
    s.settings.open && matches!(s.settings.tab, PAGE | crate::change_log::PAGE)
}

pub fn detached_open(s: &State) -> bool {
    s.guide.detached_open
}

pub fn open(app: &mut App) {
    if !crate::workspace_visibility::shown(VISIBILITY) {
        return;
    }
    if app.state.guide.detached_open
        || (crate::workspace_visibility::shown(DETACH_VISIBILITY)
            && crate::settings_file::flag(DETACHED_PREF, false))
    {
        app.state.guide.detached_open = true;
        app.state.guide.focus_window = true;
    } else {
        app.open_settings(PAGE);
    }
    app.invalidate();
}

pub fn close_detached(app: &mut App) {
    app.state.guide.detached_open = false;
    app.state.guide.focus_window = false;
    app.state.guide.picture = false;
    app.invalidate();
}

pub fn fallback_to_docked(app: &mut App) {
    close_detached(app);
    crate::settings_file::set_flag(DETACHED_PREF, false);
    app.open_settings(PAGE);
}

pub fn reset(search: &mut Search) {
    *search = Search {
        focused: true,
        ..Default::default()
    };
}

pub fn act(app: &mut App, action: Hit) {
    match action {
        Hit::ShowWhere => spotlight::show(app),
        Hit::ReturnToGuide => spotlight::return_to_guide(app),
        Hit::DismissSpotlight => spotlight::dismiss(app),
        _ if app.state.settings.open && app.state.settings.tab == crate::change_log::PAGE => {
            let st = &mut app.state.settings;
            match action {
                Hit::Search => focus(&mut st.help),
                Hit::Clear => {
                    st.help.text.set("");
                    st.help.focused = true;
                    st.scroll = 0.;
                }
                Hit::Close => st.open = false,
                _ => {}
            }
        }
        _ if is_open(&app.state) => guide_act(app, action, false),
        _ => return,
    }
    app.invalidate();
}

pub fn guide_act(app: &mut App, action: Hit, detached: bool) {
    match action {
        Hit::ShowWhere => return spotlight::show(app),
        Hit::Detach if detached => return fallback_to_docked(app),
        Hit::Detach if crate::workspace_visibility::shown(DETACH_VISIBILITY) => {
            app.state.settings.open = false;
            app.state.guide.detached_open = true;
            app.state.guide.focus_window = true;
            crate::settings_file::set_flag(DETACHED_PREF, true);
            app.invalidate();
        }
        Hit::Close if detached => close_detached(app),
        Hit::Close => {
            app.state.settings.open = false;
            app.state.guide.picture = false;
        }
        Hit::Search => focus(&mut app.state.guide.search),
        Hit::Clear => {
            app.state.guide.home();
            app.state.guide.search.focused = true;
        }
        Hit::Home => app.state.guide.home(),
        Hit::Section(index) if index < chapters().len() => {
            let view = &mut app.state.guide;
            view.search.text.set("");
            view.navigate(Page::Chapter(index));
        }
        Hit::Topic(index) if index < topics().len() => app.state.guide.navigate(Page::Topic(index)),
        Hit::Back => app.state.guide.back(),
        Hit::Picture => app.state.guide.picture = !app.state.guide.picture,
        _ => {}
    }
}

fn focus(search: &mut Search) {
    search.focused = true;
    search.text.select_all();
}

fn find(key: &Key, ctrl: bool, alt: bool) -> bool {
    ctrl && !alt
        && matches!(key, Key::Character(c) if c.eq_ignore_ascii_case("f") || c.eq_ignore_ascii_case("k"))
}

fn page_step(key: &Key, height: f32) -> Option<f32> {
    Some(match key {
        Key::Named(NamedKey::PageDown) => height * 0.9,
        Key::Named(NamedKey::PageUp) => -height * 0.9,
        _ => return None,
    })
}

pub fn guide_key(app: &mut App, key: &Key, ctrl: bool, alt: bool, detached: bool, height: f32) {
    let view = &mut app.state.guide;
    match key {
        Key::Named(NamedKey::Escape) if view.picture => view.picture = false,
        Key::Named(NamedKey::Escape) if !view.back.is_empty() => view.back(),
        Key::Named(NamedKey::Escape) if !view.search.text.text.is_empty() => view.home(),
        Key::Named(NamedKey::Escape) => guide_act(app, Hit::Close, detached),
        Key::Named(NamedKey::ArrowLeft) if alt => view.back(),
        Key::Named(NamedKey::Enter) if view.searching() => {
            if let Some(index) = results(&view.search.text.text).first().copied() {
                view.navigate(Page::Topic(index));
            }
        }
        _ if find(key, ctrl, alt) => focus(&mut view.search),
        _ if let Some(step) = page_step(key, height) => {
            if !view.picture {
                view.scroll_by(step, height);
            }
        }
        _ if view.search.focused => {
            let before = view.search.text.text.clone();
            if view.search.text.key(key, ctrl, alt) && view.search.text.text != before {
                view.page = Page::Home;
                view.back.clear();
                view.picture = false;
                view.scroll = 0.;
            }
        }
        _ => {}
    }
}

pub fn key(app: &mut App, key: &Key, ctrl: bool, alt: bool) -> bool {
    if !is_open(&app.state) {
        if active(&app.state)
            && *key == Key::Named(NamedKey::Escape)
            && app.state.menu_open.is_none()
            && app.state.playlist.popup.is_none()
            && app.drag.is_none()
        {
            spotlight::dismiss(app);
            return true;
        }
        return false;
    }
    if app.state.settings.tab == PAGE {
        let height = body(&app.state).h;
        guide_key(app, key, ctrl, alt, false, height);
        return true;
    }
    let height = body(&app.state).h;
    let st = &mut app.state.settings;
    let search = &mut st.help;
    match key {
        Key::Named(NamedKey::Escape) if !search.text.text.is_empty() => {
            search.text.set("");
            st.scroll = 0.;
        }
        Key::Named(NamedKey::Escape) => st.open = false,
        _ if find(key, ctrl, alt) => focus(search),
        _ if let Some(step) = page_step(key, height) => {
            st.scroll =
                (st.scroll + step).clamp(0., (search.content_height.get() - height).max(0.));
        }
        _ if search.focused => {
            let before = search.text.text.clone();
            if search.text.key(key, ctrl, alt) && search.text.text != before {
                st.scroll = 0.;
            }
        }
        _ => {}
    }
    true
}

pub fn body(s: &State) -> Rect {
    if s.settings.tab == PAGE {
        wiki::body(s.screen, false)
    } else {
        changelog::changelog_body(s)
    }
}

pub fn paint(ui: &mut Ui, s: &State) {
    if s.settings.tab == PAGE {
        wiki::paint(ui, s, false);
    } else {
        changelog::paint(ui, s);
    }
}

pub fn paint_detached(ui: &mut Ui, s: &State) {
    wiki::paint(ui, s, true);
}

pub fn scroll_detached(app: &mut App, amount: f32, height: f32) {
    if !app.state.guide.picture {
        app.state.guide.scroll_by(amount, height);
    }
}

pub fn detached_body_height(screen: (f32, f32)) -> f32 {
    wiki::body(screen, true).h
}

pub fn wheel(app: &mut App, amount: f32) -> bool {
    if !is_open(&app.state) || app.state.settings.tab != PAGE {
        return false;
    }
    let height = body(&app.state).h;
    if !app.state.guide.picture {
        app.state.guide.scroll_by(amount, height);
        app.invalidate();
    }
    true
}

fn hit(action: Hit) -> Action {
    Action::Settings(SAct::Help(action))
}

fn button(ui: &mut Ui, r: Rect, label: &str, action: Hit) {
    let hover = ui.hit(r, hit(action), label.to_owned());
    ui.c.fill_rrect(r, 5., if hover { ui.t.button_hi } else { ui.t.button });
    ui.c.text_center(Font::new(13), r, ui.t.text, label);
}

pub(crate) fn wrap_lines(ui: &mut Ui, font: Font, value: &str, width: f32) -> Vec<String> {
    let mut lines = Vec::new();
    for paragraph in value.lines() {
        let mut line = String::new();
        for word in paragraph.split_whitespace() {
            let candidate = if line.is_empty() {
                word.to_owned()
            } else {
                format!("{line} {word}")
            };
            if ui.c.text_width(font, &candidate) <= width {
                line = candidate;
                continue;
            }
            if !line.is_empty() {
                lines.push(std::mem::take(&mut line));
            }
            for ch in word.chars() {
                line.push(ch);
                if line.len() > ch.len_utf8() && ui.c.text_width(font, &line) > width {
                    line.pop();
                    lines.push(std::mem::take(&mut line));
                    line.push(ch);
                }
            }
        }
        lines.push(line);
    }
    lines
}

#[cfg(test)]
mod tests;
