// SPDX-License-Identifier: GPL-3.0-or-later
//! Saved track roles for attached Playlist editing. Toggling the view never edits routing.
use crate::{
    app::{App, State},
    attached_mixer as attached,
    history::{Snap, Tail},
    mixer, playlist as pl,
};
use os_model::{ChannelKind, ClipSource, PlaylistTrackMode as Mode};
use std::collections::BTreeSet;

#[derive(Debug, Clone, PartialEq)]
pub enum Command {
    Audio(u16, u16),
    Instrument(u16, u16),
    Free(u16),
    LinkExisting,
    Sort,
    AddAudio(u16),
    AddInstrument(u16),
    Retarget(usize),
}

pub fn root(s: &State, row: u16) -> u16 {
    // Only audio take/comp lanes inherit routing. A visual folder is not a bus.
    if let Some(parent) = pl::group_track(s, row)
        .filter(|&p| pl::track_info(s, p).is_some_and(|t| !t.folder && t.mode == Mode::Audio))
    {
        if pl::track_info(s, row)
            .is_none_or(|t| t.mode == Mode::Legacy && t.attached_mixer.is_none())
        {
            return parent;
        }
    }
    row
}
pub fn mode(s: &State, row: u16) -> Mode {
    pl::track_info(s, root(s, row)).map_or(Mode::Legacy, |t| t.mode)
}
pub fn instrument(s: &State, row: u16) -> Option<usize> {
    let arr = s.project.arrangement()?.number;
    s.project
        .instrument_lanes
        .iter()
        .rev()
        .filter(|v| v.1 == arr && v.2 == row)
        .find_map(|v| {
            s.project.channels.iter().position(|c| {
                c.index == v.0 && matches!(c.kind, ChannelKind::Plugin | ChannelKind::Sampler)
            })
        })
}
pub fn linked_output(s: &State, row: u16) -> Option<u16> {
    let row = root(s, row);
    let n = if mode(s, row) == Mode::Instrument {
        s.project.channels.get(instrument(s, row)?)?.mixer_track
    } else {
        pl::track_info(s, row).and_then(|t| t.attached_mixer.or(t.comp))?
    };
    mixer::track(s, n).filter(|m| !m.clip_fx).map(|_| n)
}
pub fn sources(s: &State, row: u16) -> BTreeSet<u16> {
    let mut channels = BTreeSet::new();
    for c in pl::clips(s).iter().filter(|c| c.track == row) {
        match c.source {
            ClipSource::Channel(id) => {
                if s.project
                    .channel(id)
                    .is_some_and(|c| c.kind != ChannelKind::Automation)
                {
                    channels.insert(id);
                }
            }
            ClipSource::Pattern(n) => {
                if let Some(p) = s.project.pattern(n) {
                    channels.extend(p.notes.iter().map(|n| n.channel));
                }
            }
        }
    }
    channels
}
pub fn outputs(s: &State, row: u16) -> BTreeSet<u16> {
    attached::projection(s)
        .outputs
        .get(row as usize)
        .cloned()
        .unwrap_or_default()
}
pub fn inspect_output(s: &State, row: u16) -> Option<u16> {
    attached::owner(s, row)
}
pub fn menu(s: &State, row: u16) -> Vec<pl::PItem> {
    use pl::{Cmd, Mark, PItem};
    let item = |label: &str, cmd| PItem::new(label.to_owned(), Cmd::TrackLink(cmd));
    let mut items = vec![
        item("Free arrangement track / Unlink", Command::Free(row))
            .mark(Mark::Radio(mode(s, row) == Mode::Free)),
        PItem::sub(
            "Audio track output",
            s.project
                .mixer
                .iter()
                .filter(|m| m.index > 0 && !m.clip_fx)
                .map(|m| {
                    item(
                        &format!("{}  {}", m.index, mixer::name(s, m.index)),
                        Command::Audio(row, m.index),
                    )
                    .mark(Mark::Radio(
                        mode(s, row) == Mode::Audio && linked_output(s, row) == Some(m.index),
                    ))
                })
                .collect(),
        ),
        PItem::sub(
            "Instrument track",
            s.project
                .channels
                .iter()
                .filter(|c| matches!(c.kind, ChannelKind::Plugin | ChannelKind::Sampler))
                .map(|c| {
                    item(
                        c.name.as_deref().unwrap_or("Instrument"),
                        Command::Instrument(row, c.index),
                    )
                    .mark(Mark::Radio(
                        mode(s, row) == Mode::Instrument
                            && instrument(s, row)
                                .is_some_and(|i| s.project.channels[i].index == c.index),
                    ))
                })
                .collect(),
        ),
        PItem::sep(),
        item("Insert audio track above", Command::AddAudio(row)),
        item(
            "Insert instrument on this track...",
            Command::AddInstrument(row),
        ),
        PItem::sep(),
        item("Link tracks using existing routing", Command::LinkExisting),
        item("Arrange tracks by Mixer order", Command::Sort),
    ];
    let outputs = outputs(s, row);
    if !outputs.is_empty() {
        items.push(PItem::sub(
            "Inspect current clip outputs",
            outputs
                .into_iter()
                .map(|n| {
                    PItem::new(
                        format!("{n}  {}", mixer::name(s, n)),
                        Cmd::Mixer(mixer::MCmd::AttachedSelect(n)),
                    )
                })
                .collect(),
        ));
    }
    items
}
fn editable(s: &State, row: u16) -> bool {
    row < pl::TRACKS
        && !pl::track_info(s, row).is_some_and(|t| t.locked || t.frozen.is_some() || t.folder)
}
fn set_instrument(s: &mut State, row: u16, id: u16) {
    let arr = pl::arr_mut(s).number;
    s.project
        .instrument_lanes
        .retain(|v| v.1 != arr || v.2 != row);
    s.project
        .instrument_lanes
        .push((id, arr, row, s.pattern.max(1)));
    let n = s.project.channel(id).unwrap().mixer_track;
    let t = &mut pl::arr_mut(s).tracks[row as usize];
    t.mode = Mode::Instrument;
    t.attached_mixer = Some(n);
}
pub fn snapshot(s: &mut State, name: &'static str) {
    pl::snapshot_with(
        s,
        name,
        Snap::Several(vec![
            Snap::Tail(Box::new(Tail::now(s, s.project.channels.len()))),
            Snap::Vocals(s.project.vocal_edits.clone()),
            Snap::AudioEdits(s.project.audio_edits.clone()),
            Snap::Patterns {
                patterns: s.project.patterns.clone(),
                current: s.pattern,
            },
            Snap::ClipFxTargets(
                s.project.automations.clone(),
                s.project.init_controls.clone(),
                s.project.peak_links.clone(),
            ),
        ]),
    );
}
/// Keep source automation audible on a routed copy; the original targets stay intact.
pub fn clone_controls(s: &mut State, from: u16, to: u16) {
    use os_model::target::{self, Target};
    let remap = |t: u32| match target::decode(t) {
        Target::ChannelVolume(c)
        | Target::ChannelPan(c)
        | Target::ChannelPitch(c)
        | Target::ChannelLevel(c, _)
        | Target::ChannelParam(c, _)
            if c == from =>
        {
            Some((u32::from(to) << 16) | (t & 0xffff))
        }
        _ => None,
    };
    for a in &mut s.project.automations {
        let extra: Vec<_> = a.targets.iter().filter_map(|t| remap(*t)).collect();
        for t in extra {
            if !a.targets.contains(&t) {
                a.targets.push(t);
            }
        }
    }
    let extra: Vec<_> = s
        .project
        .init_controls
        .iter()
        .filter_map(|v| {
            remap(v.target).map(|target| os_model::InitControl {
                target,
                ..v.clone()
            })
        })
        .collect();
    s.project.init_controls.extend(extra);
    let extra: Vec<_> = s
        .project
        .peak_links
        .iter()
        .filter_map(|v| {
            remap(v.target).map(|target| os_model::PeakLink {
                target,
                ..v.clone()
            })
        })
        .collect();
    s.project.peak_links.extend(extra);
    for p in &mut s.project.patterns {
        let extra: Vec<_> = p
            .controls
            .iter()
            .filter(|c| c.channel() == from)
            .map(|c| os_model::ControlPoint::new(c.position, to, c.param(), c.value))
            .collect();
        p.controls.extend(extra);
    }
}

pub fn has_controls(s: &State, id: u16) -> bool {
    let refers = |raw: u32| (raw >> 16) == u32::from(id);
    s.project
        .automations
        .iter()
        .any(|a| a.targets.iter().any(|&t| refers(t)))
        || s.project.init_controls.iter().any(|v| refers(v.target))
        || s.project.peak_links.iter().any(|v| refers(v.target))
        || s.project
            .patterns
            .iter()
            .any(|p| p.controls.iter().any(|c| c.channel() == id))
}
pub fn run(app: &mut App, cmd: Command) -> Option<String> {
    if !attached::active(&app.state) {
        return Some("Attach the Mixer to edit attached track links".into());
    }
    if app.state.rec.recording() || app.audio_rec.is_some() {
        return Some("Finish recording before changing track links".into());
    }
    app.state.playlist.popup = None;
    let s = &mut app.state;
    attached::clear(s);
    let row = match cmd {
        Command::Audio(r, _)
        | Command::Instrument(r, _)
        | Command::Free(r)
        | Command::AddAudio(r)
        | Command::AddInstrument(r) => Some(r),
        _ => None,
    };
    if row.is_some_and(|r| !editable(s, r)) {
        return Some("Choose an unlocked, unfrozen track".into());
    }
    match cmd {
        Command::Audio(row, n) => {
            if n == 0 || mixer::track(s, n).is_none_or(|m| m.clip_fx) || mixer::locked(s, n) {
                return Some("Choose an unlocked mixer output".into());
            }
            let indices: Vec<_> = pl::clips(s)
                .iter()
                .enumerate()
                .filter(|(_, c)| c.track == row)
                .map(|(i, _)| i)
                .collect();
            if indices.iter().any(|&i| route_blocked(s, i, n)) {
                return Some("This track contains audio with private clip effects or an ARA editor. Keep its existing routing or remove that attachment before linking.".into());
            }
            snapshot(s, "Link audio track");
            let arr = pl::arr_mut(s).number;
            s.project
                .instrument_lanes
                .retain(|v| v.1 != arr || v.2 != row);
            attached::bind(s, row, n);
            for i in indices {
                route_placement(s, i);
            }
        }
        Command::Instrument(row, id) => {
            if !s
                .project
                .channel(id)
                .is_some_and(|c| matches!(c.kind, ChannelKind::Plugin | ChannelKind::Sampler))
            {
                return None;
            }
            pl::snapshot_as(s, "Link instrument track");
            set_instrument(s, row, id);
        }
        Command::Free(row) => {
            pl::snapshot_as(s, "Unlink Playlist track");
            let arr = pl::arr_mut(s).number;
            let t = &mut pl::arr_mut(s).tracks[row as usize];
            t.mode = Mode::Free;
            t.attached_mixer = None;
            s.project
                .instrument_lanes
                .retain(|v| v.1 != arr || v.2 != row);
        }
        Command::AddAudio(row) => return attached::insert_lane(s, row),
        Command::AddInstrument(row) => {
            crate::playlist_tracks::open_instrument_menu(
                app,
                crate::plugin_menu::Target::PlaylistInstrument(row),
            );
            return None;
        }
        Command::LinkExisting => {
            pl::snapshot_as(s, "Link tracks using existing routing");
            let mut count = 0;
            for row in 0..pl::TRACKS {
                if !editable(s, row) || !matches!(mode(s, row), Mode::Legacy | Mode::Free) {
                    continue;
                }
                let ids = sources(s, row);
                let outputs = outputs(s, row);
                if outputs.len() != 1 || ids.is_empty() {
                    continue;
                }
                let n = *outputs.first().unwrap();
                if mixer::track(s, n).is_none_or(|m| m.clip_fx) {
                    continue;
                }
                if ids.len() == 1
                    && s.project.channel(*ids.first().unwrap()).is_some_and(|c| {
                        matches!(c.kind, ChannelKind::Plugin | ChannelKind::Sampler)
                    })
                {
                    set_instrument(s, row, *ids.first().unwrap());
                    count += 1;
                } else if ids.iter().all(|id| {
                    s.project
                        .channel(*id)
                        .is_some_and(|c| c.kind == ChannelKind::AudioClip)
                }) {
                    attached::bind(s, row, n);
                    count += 1;
                }
            }
            if count == 0 {
                crate::history::unrecord(s);
            }
            attached::clear(s);
            return Some(format!("Linked {count} tracks; existing routing preserved"));
        }
        Command::Sort => {
            let p = attached::projection(s);
            let mut families: Vec<_> = p
                .order
                .iter()
                .copied()
                .filter(|&r| !pl::track_info(s, r).is_some_and(|t| t.grouped))
                .collect();
            families.sort_by_key(|&r| inspect_output(s, r).unwrap_or(u16::MAX));
            let order: Vec<_> = families
                .into_iter()
                .flat_map(|r| attached::lane_family(s, r))
                .collect();
            if order != p.order {
                pl::snapshot_as(s, "Arrange tracks by Mixer order");
                pl::arr_mut(s).attached_order = order;
            }
        }
        Command::Retarget(i) => return retarget(s, i),
    }
    attached::clear(s);
    if let Some(row) = row {
        attached::select_lane(s, row);
    }
    s.piano.dirty = true;
    app.invalidate();
    None
}

pub fn route_blocked(s: &State, i: usize, n: u16) -> bool {
    let Some(c) = pl::clips(s).get(i) else {
        return false;
    };
    let ClipSource::Channel(id) = c.source else {
        return false;
    };
    s.project.channel(id).is_some_and(|ch| {
        ch.kind.is_sampler()
            && ch.mixer_track != n
            && (crate::ara::is(ch) || s.project.clip_fx_channels.contains(&id))
    })
}
pub fn route_placement(s: &mut State, i: usize) {
    let Some(c) = pl::clips(s).get(i) else { return };
    let row = c.track;
    if !attached::active(s) || mode(s, row) == Mode::Free {
        return;
    }
    let Some(n) = linked_output(s, row).or_else(|| attached::owner(s, row)) else {
        return;
    };
    if route_blocked(s, i, n) {
        s.message = Some(
            "Audio keeps its private clip-effects / ARA routing. Inspect current clip outputs in the track routing menu."
                .into(),
        );
        return;
    }
    attached::route_audio_placement(s, i, n);
}
fn retarget(s: &mut State, i: usize) -> Option<String> {
    let c = pl::clips(s).get(i)?.clone();
    if !editable(s, c.track) {
        return Some("Unlock or unfreeze this track first".into());
    }
    if mode(s, c.track) != Mode::Instrument {
        return Some("Link an instrument to this track first".into());
    }
    let target = s.project.channels.get(instrument(s, c.track)?)?.index;
    let ClipSource::Pattern(number) = c.source else {
        return Some("Choose a single-instrument pattern".into());
    };
    let p = s.project.pattern(number)?;
    let ids: BTreeSet<_> = p.notes.iter().map(|n| n.channel).collect();
    if ids.len() != 1 {
        return Some(
            "Split this mixed pattern by channel before assigning another instrument".into(),
        );
    }
    let original = *ids.first().unwrap();
    if original == target {
        return None;
    }
    if !p.controls.is_empty() {
        return Some("This pattern contains event automation. Separate that automation before changing instruments.".into());
    }
    snapshot(s, "Use track instrument");
    let next = pl::make_unique(s, i)?;
    let p = s.project.patterns.iter_mut().find(|p| p.number == next)?;
    for note in &mut p.notes {
        note.channel = target;
    }
    for (channel, _) in &mut p.loops {
        if *channel == original {
            *channel = target;
        }
    }
    attached::clear(s);
    s.piano.dirty = true;
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use os_model::{Clip, MixerTrack, PlaylistTrack};
    fn app() -> App {
        let mut app = App::new(None);
        let s = &mut app.state;
        s.project = crate::app::test_project();
        s.project.channels.truncate(2);
        s.project.instrument_lanes.clear();
        s.project.automations.clear();
        s.project.init_controls.clear();
        s.project.peak_links.clear();
        for (i, c) in s.project.channels.iter_mut().enumerate() {
            c.kind = ChannelKind::AudioClip;
            c.mixer_track = i as u16 + 1;
            c.sample_path = Some("linked-source.wav".into());
        }
        s.project.mixer = (0..8).map(MixerTrack::new).collect();
        let a = pl::arr_mut(s);
        a.clips = vec![
            Clip::new(0, 384, 0, ClipSource::Channel(0)),
            Clip::new(384, 384, 0, ClipSource::Channel(0)),
        ];
        a.tracks = (1..=pl::TRACKS)
            .map(|n| PlaylistTrack::new(n.into()))
            .collect();
        a.attached_order.clear();
        a.attached_hidden.clear();
        s.playlist.track = 0;
        s.playlist.source = Some(ClipSource::Channel(0));
        s.z = vec![crate::app::Win::Playlist];
        app
    }
    fn route(s: &State, i: usize) -> u16 {
        let ClipSource::Channel(id) = pl::clips(s)[i].source else {
            panic!()
        };
        s.project.channel(id).unwrap().mixer_track
    }
    #[test]
    fn linked_audio_moves_and_pastes_are_independent_and_one_undo() {
        let mut a = app();
        attached::toggle(&mut a);
        attached::bind(&mut a.state, 1, 2);
        let before = a.state.project.clone();
        pl::snapshot_placement(&mut a.state, "Place audio", [ClipSource::Channel(0)]);
        let i = pl::place(&mut a.state, 1, 768);
        assert_eq!(route(&a.state, i), 2);
        assert_eq!(route(&a.state, 0), 1);
        assert_eq!(route(&a.state, 1), 1);
        assert_ne!(pl::clips(&a.state)[i].source, pl::clips(&a.state)[0].source);
        let after = a.state.project.clone();
        crate::history::undo(&mut a.state);
        assert_eq!(a.state.project, before);
        crate::history::redo(&mut a.state);
        assert_eq!(a.state.project, after);
        a.state.playlist.sel = vec![0];
        pl::copy(&mut a.state);
        a.state.playlist.track = 1;
        a.state.playlist.pos = 1152;
        let before = a.state.project.clone();
        pl::run(&mut a, pl::Cmd::Paste);
        let i = pl::clips(&a.state).len() - 1;
        assert_eq!((pl::clips(&a.state)[i].track, route(&a.state, i)), (1, 2));
        crate::history::undo(&mut a.state);
        assert_eq!(a.state.project, before);
        a.state.playlist.sel = vec![0];
        let rows = pl::Rows::new(&a.state);
        let delta = i32::from(rows.rank(1)) - i32::from(rows.rank(0));
        pl::run(&mut a, pl::Cmd::ShiftTracks(delta));
        assert_eq!((pl::clips(&a.state)[0].track, route(&a.state, 0)), (1, 2));
        assert_eq!(route(&a.state, 1), 1);
        crate::history::undo(&mut a.state);
        assert_eq!(a.state.project, before);
    }
    #[test]
    fn unattached_editing_paste_layout_and_menus_keep_the_original_behavior() {
        let mut a = app();
        let original = a.state.project.clone();
        attached::toggle(&mut a);
        assert_eq!(a.state.project, original);
        attached::bind(&mut a.state, 1, 2);
        pl::snapshot_placement(&mut a.state, "Place audio", [ClipSource::Channel(0)]);
        let i = pl::place(&mut a.state, 1, 768);
        assert_eq!(route(&a.state, i), 2);
        let linked = a.state.project.clone();
        attached::toggle(&mut a);
        assert_eq!(a.state.project, linked);
        assert_eq!(
            pl::Rows::new(&a.state).order,
            (0..pl::TRACKS).collect::<Vec<_>>()
        );
        a.state.playlist.source = Some(ClipSource::Channel(0));
        let i = pl::place(&mut a.state, 1, 1152);
        assert_eq!(
            route(&a.state, i),
            1,
            "ordinary placement retains source output even on a saved linked lane"
        );
        a.state.playlist.sel = vec![0];
        pl::copy(&mut a.state);
        a.state.playlist.track = 1;
        a.state.playlist.pos = 1536;
        pl::run(&mut a, pl::Cmd::Paste);
        assert_eq!(
            pl::clips(&a.state).last().unwrap().track,
            0,
            "ordinary paste stays on its original row"
        );
        assert!(
            !pl::track_menu(&a.state, 1, 0., 0.)
                .items
                .iter()
                .any(|i| i.label == "Track type and routing")
        );
        let before = a.state.project.clone();
        attached::toggle(&mut a);
        assert_eq!(
            a.state.project, before,
            "reattaching never repairs or reroutes clips edited while detached"
        );
        assert_eq!(outputs(&a.state, 1), BTreeSet::from([1, 2]));
    }
    #[test]
    fn audio_import_uses_destination_only_while_attached() {
        let dir = std::env::temp_dir().join(format!("track-links-import-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("audio.wav");
        os_engine::wav::write_f32_stereo(
            &mut std::fs::File::create(&path).unwrap(),
            48000,
            &vec![0.2; 9600],
        )
        .unwrap();
        let mut a = app();
        attached::toggle(&mut a);
        attached::bind(&mut a.state, 1, 2);
        let before = a.state.project.clone();
        let i = pl::add_audio_clip(&mut a.state, &path.to_string_lossy(), 1, 0).unwrap();
        assert_eq!(route(&a.state, i), 2);
        crate::history::undo(&mut a.state);
        assert_eq!(a.state.project, before);
        attached::toggle(&mut a);
        let i = pl::add_audio_clip(&mut a.state, &path.to_string_lossy(), 1, 0).unwrap();
        assert_eq!(
            route(&a.state, i),
            0,
            "ordinary import retains Master default"
        );
        std::fs::remove_file(path).unwrap();
        std::fs::remove_dir(dir).unwrap();
    }
    #[test]
    fn track_conversion_and_unlink_preserve_other_placements_and_roundtrip() {
        let mut a = app();
        attached::toggle(&mut a);
        let before = a.state.project.clone();
        assert!(run(&mut a, Command::Audio(0, 3)).is_none());
        assert_eq!(route(&a.state, 0), 3);
        assert_eq!(a.state.project.channels[0].mixer_track, 1);
        let after = a.state.project.clone();
        crate::history::undo(&mut a.state);
        assert_eq!(a.state.project, before);
        crate::history::redo(&mut a.state);
        assert_eq!(a.state.project, after);
        let file = os_flp::project::create(&a.state.project).unwrap();
        let back = os_flp::project::load(&file).unwrap();
        assert_eq!(back.arrangement().unwrap().tracks[0].mode, Mode::Audio);
        assert_eq!(
            back.arrangement().unwrap().tracks[0].attached_mixer,
            Some(3)
        );
        let channels = a.state.project.channels.clone();
        run(&mut a, Command::Free(0));
        assert_eq!(a.state.project.channels, channels);
        assert_eq!(mode(&a.state, 0), Mode::Free);
        assert_eq!(attached::owner(&a.state, 0), None);
        a.state.playlist.source = Some(ClipSource::Channel(0));
        let i = pl::place(&mut a.state, 0, 1152);
        assert_eq!(route(&a.state, i), 1);
        attached::toggle(&mut a);
        attached::toggle(&mut a);
        assert_eq!(mode(&a.state, 0), Mode::Free);
    }
    #[test]
    fn patterns_keep_instruments_until_explicit_retarget_and_follow_instrument_output() {
        let mut a = app();
        for c in &mut a.state.project.channels {
            c.kind = ChannelKind::Sampler;
        }
        a.state.project.patterns[0].notes = vec![os_model::Note::step(0, 0)];
        let p = a.state.project.patterns[0].number;
        attached::toggle(&mut a);
        run(&mut a, Command::Instrument(1, 1));
        a.state.playlist.source = Some(ClipSource::Pattern(p));
        let i = pl::place(&mut a.state, 1, 0);
        assert_eq!(pl::clips(&a.state)[i].source, ClipSource::Pattern(p));
        assert_eq!(a.state.project.pattern(p).unwrap().notes[0].channel, 0);
        let before = a.state.project.clone();
        assert!(run(&mut a, Command::Retarget(i)).is_none());
        let ClipSource::Pattern(next) = pl::clips(&a.state)[i].source else {
            panic!()
        };
        assert_ne!(next, p);
        assert_eq!(a.state.project.pattern(next).unwrap().notes[0].channel, 1);
        assert_eq!(a.state.project.pattern(p).unwrap().notes[0].channel, 0);
        crate::history::undo(&mut a.state);
        assert_eq!(a.state.project, before);
        a.state.project.channels[0].mixer_track = 2;
        attached::clear(&a.state);
        attached::record_lane(&mut a, 1);
        assert_eq!(a.state.selected_channel, 1);
        assert!(attached::record_armed(&a.state, 1));
        attached::record_lane(&mut a, 1);
        assert!(!attached::record_armed(&a.state, 1));
        a.state.project.channels[1].mixer_track = 4;
        attached::clear(&a.state);
        assert_eq!(attached::owner(&a.state, 1), Some(4));
        a.state.project.channels[1].mixer_track = 0;
        attached::clear(&a.state);
        assert_eq!(
            attached::owner(&a.state, 1),
            Some(0),
            "explicit instrument can feed Master without changing its identity"
        );
        a.state.project.patterns[0]
            .notes
            .push(os_model::Note::step(1, 0));
        let before = a.state.project.clone();
        assert!(run(&mut a, Command::Retarget(i)).unwrap().contains("Split"));
        assert_eq!(a.state.project, before);
    }
    #[test]
    fn cloned_audio_retains_automation_and_pitch_timing_history() {
        let (mut a, dir, ci) = crate::audio_edits::tests::fixture("linked-routing");
        attached::toggle(&mut a);
        let src = pl::clips(&a.state)[ci].source;
        let ClipSource::Channel(id) = src else {
            panic!()
        };
        let n = a.state.project.channels.len();
        let auto_id = a
            .state
            .project
            .channels
            .iter()
            .map(|c| c.index)
            .max()
            .unwrap()
            + 1;
        let mut auto = a.state.project.channels[0].clone();
        auto.kind = ChannelKind::Automation;
        auto.index = auto_id;
        a.state.project.channels.push(auto);
        let target = os_model::target::channel(id, os_model::target::CHANNEL_VOLUME);
        a.state.project.automations.push(os_model::Automation {
            channel: auto_id,
            targets: vec![target],
            points: vec![os_model::AutoPoint::new(0., 0.5)],
            header: vec![],
            tail: vec![],
        });
        a.state.project.init_controls.push(os_model::InitControl {
            head: 0,
            target,
            value: 6400,
        });
        attached::bind(&mut a.state, 5, 2);
        a.state.playlist.source = Some(src);
        let before = a.state.project.clone();
        pl::snapshot_placement(&mut a.state, "Route edited audio", [src]);
        let next = pl::place(&mut a.state, 5, 0);
        let ClipSource::Channel(new) = pl::clips(&a.state)[next].source else {
            panic!()
        };
        assert_ne!(new, id);
        assert_eq!(route(&a.state, next), 2);
        assert_eq!(a.state.project.channels[n].index, auto_id);
        assert!(
            a.state
                .project
                .automations
                .last()
                .unwrap()
                .targets
                .contains(&os_model::target::channel(
                    new,
                    os_model::target::CHANNEL_VOLUME
                ))
        );
        let old = a
            .state
            .project
            .audio_edits
            .iter()
            .find(|h| h.channel == id)
            .unwrap();
        let copy = a
            .state
            .project
            .audio_edits
            .iter()
            .find(|h| h.channel == new)
            .unwrap();
        assert_eq!(old.pitch, copy.pitch);
        assert_eq!(old.timing, copy.timing);
        assert_ne!(old.clip, copy.clip);
        let after = a.state.project.clone();
        crate::history::undo(&mut a.state);
        assert_eq!(a.state.project, before);
        crate::history::redo(&mut a.state);
        assert_eq!(a.state.project, after);
        std::fs::remove_dir_all(dir).unwrap();
    }
    #[test]
    fn protected_audio_keeps_processing_and_routed_copies_do_not_adopt_unrelated_automation() {
        for ara in [false, true] {
            let mut a = app();
            if ara {
                a.state.project.channels[0].plugin = Some(crate::ara::MARKER.into());
            } else {
                a.state.project.clip_fx_channels.push(0);
            }
            attached::toggle(&mut a);
            attached::bind(&mut a.state, 1, 2);
            let before = a.state.project.clone();
            assert!(
                run(&mut a, Command::Audio(0, 2))
                    .unwrap()
                    .contains("private clip effects")
            );
            assert_eq!(a.state.project, before);
            let i = pl::place(&mut a.state, 1, 768);
            assert_eq!(route(&a.state, i), 1);
            assert_eq!(a.state.project.channels, before.channels);
            assert_eq!(outputs(&a.state, 1), BTreeSet::from([1]));
        }
        let mut a = app();
        // This otherwise identical source has different automation and must not be reused.
        let mut candidate = a.state.project.channels[0].clone();
        candidate.index = 1;
        candidate.mixer_track = 2;
        a.state.project.channels[1] = candidate;
        a.state.project.init_controls.push(os_model::InitControl {
            head: 0,
            target: os_model::target::channel(1, os_model::target::CHANNEL_VOLUME),
            value: 0,
        });
        attached::toggle(&mut a);
        attached::bind(&mut a.state, 1, 2);
        let i = pl::place(&mut a.state, 1, 768);
        assert_eq!(route(&a.state, i), 2);
        assert_ne!(pl::clips(&a.state)[i].source, ClipSource::Channel(1));
        assert_eq!(a.state.project.init_controls.len(), 1);
    }
    fn render(p: &os_model::Project) -> Vec<f32> {
        let mut lib = os_engine::SampleLibrary::new(None);
        lib.insert_buffer(
            "linked-source.wav".into(),
            std::sync::Arc::new(os_engine::Sample {
                rate: 48000,
                left: vec![0.2; 48000],
                right: vec![0.2; 48000],
            }),
        );
        let mut engine = os_engine::Engine::new(p, &mut lib, 48000, os_engine::PlayMode::Song);
        let mut out = vec![0.; 9600];
        engine.process(&mut out);
        out
    }
    #[test]
    fn attachment_is_sonically_inert_and_audio_move_reaches_its_new_output() {
        let mut a = app();
        a.state.project.mixer[1].pan = Some(-1.);
        a.state.project.mixer[2].pan = Some(1.);
        let original = render(&a.state.project);
        assert!(original.iter().any(|v| v.abs() > 0.001));
        attached::toggle(&mut a);
        assert_eq!(render(&a.state.project), original);
        attached::bind(&mut a.state, 1, 2);
        a.state.playlist.source = Some(ClipSource::Channel(0));
        let i = pl::place(&mut a.state, 1, 0);
        let mixed = render(&a.state.project);
        let energy = |v: &[f32], side| v.iter().skip(side).step_by(2).map(|x| x.abs()).sum::<f32>();
        assert!(energy(&mixed, 1) > energy(&original, 1) + 1.);
        assert_eq!(route(&a.state, 0), 1);
        assert_eq!(route(&a.state, i), 2);
        attached::toggle(&mut a);
        assert_eq!(render(&a.state.project), mixed);
        attached::toggle(&mut a);
        assert_eq!(render(&a.state.project), mixed);
    }

    #[test]
    fn attached_pointer_move_cancel_and_header_menu_are_consistent() {
        use crate::app::{Action, Win};
        let mut a = app();
        attached::toggle(&mut a);
        attached::bind(&mut a.state, 1, 2);
        a.state.wins[Win::Playlist.slot().unwrap()].maxed = true;
        let mut canvas = os_ui::Canvas::new(1440, 900, 1.).unwrap();
        a.paint(&mut canvas, None);
        let grid = a.pl_grid().unwrap();
        let g = pl::PGeom::new(&a.state, grid);
        let r = g.clip_rect(&pl::clips(&a.state)[0]);
        let from = (r.x + r.w * 0.4, r.y + 25.);
        let to = (from.0, g.y_of(1) + 25.);
        let before = a.state.project.clone();
        a.mouse_move(from.0, from.1);
        a.mouse_down();
        a.mouse_move(to.0, to.1);
        assert_eq!(pl::clips(&a.state)[0].track, 1);
        a.key(
            &winit::keyboard::Key::Named(winit::keyboard::NamedKey::Escape),
            false,
            false,
        );
        assert_eq!(a.state.project, before);
        assert!(a.drag.is_none());
        a.paint(&mut canvas, None);
        a.mouse_move(from.0, from.1);
        a.mouse_down();
        a.mouse_move(to.0, to.1);
        a.mouse_up();
        assert_eq!(route(&a.state, 0), 2);
        assert_eq!(route(&a.state, 1), 1);
        crate::history::undo(&mut a.state);
        assert_eq!(a.state.project, before);
        a.paint(&mut canvas, None);
        let button = a
            .hits
            .iter()
            .find(|h| h.action == Action::PlHeader(1))
            .unwrap()
            .rect;
        a.mouse_move(button.x + 35., button.y + 5.);
        a.mouse_down_right();
        a.mouse_up();
        assert!(
            a.state
                .playlist
                .popup
                .as_ref()
                .unwrap()
                .items
                .iter()
                .any(|i| i.label == "Track type and routing")
        );
        assert_eq!(a.state.project, before);
        a.key(
            &winit::keyboard::Key::Named(winit::keyboard::NamedKey::Escape),
            false,
            false,
        );
        assert_eq!(a.state.project, before);
        for scale in [1., 1.25, 1.5] {
            let mut image = os_ui::Canvas::new(1600, 1000, scale).unwrap();
            a.paint(&mut image, None);
            if let Some(dir) = std::env::var_os("OS_SHOT_DIR") {
                image
                    .pixmap
                    .save_png(std::path::PathBuf::from(dir).join(format!("routing-{scale}.png")))
                    .unwrap();
            }
        }
        assert_eq!(a.state.project, before);
        attached::toggle(&mut a);
        assert!(
            !pl::track_menu(&a.state, 1, 0., 0.)
                .items
                .iter()
                .any(|i| i.label == "Track type and routing")
        );
    }
}
