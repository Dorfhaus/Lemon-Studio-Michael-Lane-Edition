// SPDX-License-Identifier: GPL-3.0-or-later
//! Shared miniature mixer destination picker for audio clips and channel TRACK controls.
use crate::{
    app::App,
    dialog::{Body, Field},
};
pub const COLS: usize = 12;
pub const ROWS: usize = 4;
pub struct Track {
    pub index: u16,
    pub name: String,
    pub color: Option<os_model::Rgb>,
    pub level: f64,
}
pub struct Form {
    pub rack: Option<(Vec<usize>, bool)>,
    pub row: usize,
    pub channel: String,
    pub current: u16,
    pub tracks: Vec<Track>,
    pub shown: Vec<usize>,
    pub search: Field,
    pub first: usize,
}
impl Form {
    pub fn filter(&mut self) {
        let q = self.search.text.to_lowercase();
        self.shown = self
            .tracks
            .iter()
            .enumerate()
            .filter(|(_, t)| {
                q.split_whitespace()
                    .all(|w| format!("{} {}", t.index, t.name).to_lowercase().contains(w))
            })
            .map(|(i, _)| i)
            .collect();
        self.first = 0;
    }
    pub fn scroll(&mut self, rows: isize) {
        let max = (self.shown.len().div_ceil(COLS)).saturating_sub(ROWS);
        self.first = (self.first / COLS).saturating_add_signed(rows).min(max) * COLS;
    }
}
pub fn open(app: &mut App, row: usize) {
    let Some(ch) = app.state.project.channels.get(row) else {
        return;
    };
    let mut tracks: Vec<_> = app
        .state
        .project
        .mixer
        .iter()
        .filter(|m| m.index <= crate::mixer::max_insert(&app.state.project))
        .map(|m| Track {
            index: m.index,
            name: crate::mixer::name(&app.state, m.index),
            color: m.color,
            level: m.volume.unwrap_or(0.8),
        })
        .collect();
    tracks.sort_by_key(|t| t.index);
    let mut f = Form {
        rack: None,
        row,
        channel: ch
            .name
            .clone()
            .unwrap_or_else(|| format!("Channel {}", row + 1)),
        current: ch.mixer_track,
        tracks,
        shown: Vec::new(),
        search: Field::default(),
        first: 0,
    };
    f.filter();
    if let Some(i) = f.tracks.iter().position(|t| t.index == f.current) {
        f.scroll((i / COLS) as isize);
    }
    app.state.playlist.popup = None;
    app.show_form(Body::RoutePicker(f));
}

pub fn open_rack(app: &mut App, row: usize, spread: bool) {
    let rows = crate::rack_controls::rows(&app.state, row);
    if rows.is_empty() { return; }
    open(app, row);
    if rows.len() == 1 && !spread { return; }
    if let Some(crate::dialog::Modal { body: Body::RoutePicker(f), .. }) = app.state.dialogs.last_mut() {
        if rows.len() > 1 {
            f.channel = format!("{} selected channels{}", rows.len(), if spread { " - consecutive tracks" } else { "" });
        }
        f.rack = Some((rows, spread));
    }
}

#[cfg(plugin_windows)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SamplerTrackCommand {
    Pick(usize),
    Show(usize),
    Master(usize),
    ToggleDrag,
}

#[cfg(plugin_windows)]
pub(crate) fn sampler_track_menu(s: &crate::app::State, row: usize, x: f32, y: f32) -> Option<crate::playlist::Popup> {
    use crate::playlist::{Cmd, Mark, PItem, Popup};
    use SamplerTrackCommand::*;

    let ch = s.project.channels.get(row).filter(|ch| ch.kind.is_sampler())?;
    let track = ch.mixer_track;
    let step = crate::settings_file::flag(crate::plugin_window::SAMPLER_TRACK_DRAG, false);
    let mut master = PItem::new("Route to Master", Cmd::SamplerTrack(Master(row)));
    if track == 0 || crate::mixer::track(s, 0).is_none() || crate::mixer::locked(s, track) || crate::mixer::locked(s, 0) {
        master = master.off();
    }
    Some(Popup {
        x, y,
        items: vec![
            PItem::new("Choose Mixer track...", Cmd::SamplerTrack(Pick(row))),
            PItem::new("Show current Mixer track", Cmd::SamplerTrack(Show(row))),
            master,
            PItem::sep(),
            PItem::new("Drag up/down to change Mixer track", Cmd::SamplerTrack(ToggleDrag)).mark(Mark::Check(step)),
        ],
        sub: None,
        scroll: [0; 2],
        cascade: Vec::new(),
    })
}

#[cfg(plugin_windows)]
pub(crate) fn run_sampler_track_menu(app: &mut App, command: SamplerTrackCommand) -> Option<String> {
    use SamplerTrackCommand::*;

    let row = match command {
        Pick(row) | Show(row) | Master(row) => row,
        ToggleDrag => {
            let key = crate::plugin_window::SAMPLER_TRACK_DRAG;
            crate::settings_file::set_flag(key, !crate::settings_file::flag(key, false));
            app.invalidate();
            return None;
        }
    };
    let track = app.state.project.channels.get(row).filter(|ch| ch.kind.is_sampler())?.mixer_track;
    match command {
        Pick(_) => app.open_plugin_route_picker(row),
        Show(_) => {
            crate::mixer::track(&app.state, track)?;
            app.state.raise(crate::app::Win::Mixer);
            app.reveal_mixer_track(track);
        }
        Master(_) => return app.commit_sampler_track_step(row, track, 0),
        ToggleDrag => {}
    }
    None
}
