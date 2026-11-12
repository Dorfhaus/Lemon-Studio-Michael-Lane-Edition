// SPDX-License-Identifier: GPL-3.0-or-later
//! Placement ownership and persistent, independently undoable audio edit layers.
use crate::{
    app::{App, State},
    history::Snap,
    playlist as pl,
};
use os_model::{
    ClipSource,
    audio_edit::{Document, Pitch},
    vocal::Edit,
};

pub fn ids(s: &mut State) {
    let mut next = s
        .project
        .arrangements
        .iter()
        .flat_map(|a| &a.clips)
        .map(|c| c.id())
        .max()
        .unwrap_or(0)
        .saturating_add(1)
        .max(1);
    let mut seen = std::collections::HashSet::new();
    for a in &mut s.project.arrangements {
        for c in &mut a.clips {
            if c.id() == 0 || !seen.insert(c.id()) {
                while seen.contains(&next) {
                    next = next.wrapping_add(1).max(1);
                }
                c.set_id(next);
                seen.insert(next);
                next = next.wrapping_add(1).max(1);
            }
        }
    }
}
pub fn channel(s: &State, ci: usize) -> Option<u16> {
    match pl::clips(s).get(ci)?.source {
        ClipSource::Channel(ch) => Some(ch),
        _ => None,
    }
}
pub fn attach(s: &mut State, ci: usize, analyzed: bool) {
    ids(s);
    let Some(ch) = channel(s, ci) else { return };
    let Some(d) = s.project.vocal_edits.iter().find(|d| d.channel == ch) else {
        return;
    };
    if s.project.audio_edits.iter().any(|h| h.channel == ch) {
        return;
    }
    let c = &pl::clips(s)[ci];
    let duration = d.frames as f64 / d.rate as f64;
    let (a, b) = pl::window(s, c);
    let range = (
        (a / 1000.).clamp(0., duration),
        (b / 1000.).clamp(0., duration),
    );
    if range.1 <= range.0 {
        return;
    }
    let arr = s.project.arrangement().map_or(0, |a| a.number);
    s.project
        .audio_edits
        .push(Document::new(d, c.id(), arr, range, analyzed));
}
/// Isolate only the placement being edited. Original samples/analysis are reusable.
pub fn isolate(s: &mut State, ci: usize) -> Option<u16> {
    let ch = channel(s, ci)?;
    let shared = s
        .project
        .arrangements
        .iter()
        .flat_map(|a| &a.clips)
        .filter(|c| c.source == ClipSource::Channel(ch))
        .count()
        > 1;
    let ch = if shared { pl::make_unique(s, ci)? } else { ch };
    ids(s);
    let c = pl::clips(s).get(ci)?.clone();
    let arr = s.project.arrangement()?.number;
    if let Some(d) = s.project.audio_edits.iter_mut().find(|d| d.channel == ch) {
        d.clip = c.id();
        d.arrangement = arr;
    }
    Some(ch)
}
pub fn clone_channel(s: &mut State, from: u16, to: u16) {
    if let Some(mut h) = s
        .project
        .audio_edits
        .iter()
        .find(|h| h.channel == from)
        .cloned()
    {
        h.channel = to;
        s.project.audio_edits.push(h);
    }
}
pub fn restrict(s: &mut State, ci: usize) {
    let Some(ch) = channel(s, ci) else { return };
    let c = pl::clips(s)[ci].clone();
    let (a, b) = pl::window(s, &c);
    let Some(h) = s.project.audio_edits.iter_mut().find(|h| h.channel == ch) else {
        return;
    };
    let range = (a / 1000., b / 1000.);
    if range.1 <= range.0 || range == h.output_range {
        return;
    }
    h.restrict(range);
    if let Some(d) = s.project.vocal_edits.iter_mut().find(|d| d.channel == ch) {
        h.restore(d);
    }
}
pub fn finish(s: &mut State) {
    for h in &mut s.project.audio_edits {
        if let Some(d) = s
            .project
            .vocal_edits
            .iter()
            .find(|d| d.channel == h.channel)
        {
            h.commit(d);
        }
    }
}
pub fn snapshot(s: &mut State, label: &'static str) {
    let mut snaps = vec![
        Snap::Vocals(s.project.vocal_edits.clone()),
        Snap::AudioEdits(s.project.audio_edits.clone()),
    ];
    for d in &s.project.vocal_edits {
        if let Some(r) = s.project.channels.iter().position(|c| c.index == d.channel) {
            if let Some(v) = Snap::sample(s, r) {
                snaps.push(v);
            }
        }
    }
    crate::history::record(s, label, Snap::Several(snaps));
}
pub fn active(s: &State) -> Option<u16> {
    s.playlist
        .vocal
        .active
        .filter(|ch| s.project.audio_edits.iter().any(|h| h.channel == *ch))
        .or_else(|| s.playlist.vocal.selected.first().map(|p| p.0))
        .or_else(|| {
            s.playlist.sel.iter().find_map(|&ci| {
                channel(s, ci).filter(|ch| s.project.audio_edits.iter().any(|h| h.channel == *ch))
            })
        })
        .or_else(|| {
            pl::clips(s)
                .iter()
                .filter(|c| Some(c.track) == s.playlist.vocal.row)
                .find_map(|c| match c.source {
                    ClipSource::Channel(ch)
                        if s.project.audio_edits.iter().any(|h| h.channel == ch) =>
                    {
                        Some(ch)
                    }
                    _ => None,
                })
        })
}
pub fn history(app: &mut App, ch: u16, timing: bool, redo: bool) {
    let Some(i) = app
        .state
        .project
        .audio_edits
        .iter()
        .position(|h| h.channel == ch)
    else {
        return;
    };
    let h = &app.state.project.audio_edits[i];
    let available = if timing {
        if redo {
            h.timing.redo.len()
        } else {
            h.timing.undo.len()
        }
    } else if redo {
        h.pitch.redo.len()
    } else {
        h.pitch.undo.len()
    };
    if available == 0 {
        return;
    }
    snapshot(
        &mut app.state,
        if timing {
            "Restore clip timing"
        } else {
            "Restore clip pitch edits"
        },
    );
    let h = &mut app.state.project.audio_edits[i];
    if timing {
        h.timing.step(redo);
    } else {
        h.pitch.step(redo);
    }
    if let Some(d) = app
        .state
        .project
        .vocal_edits
        .iter_mut()
        .find(|d| d.channel == ch)
    {
        h.restore(d);
    }
    crate::vocal::history_changed(&mut app.state);
    app.state.piano.dirty = true;
    app.invalidate();
}
pub fn pitch_changed(d: &Edit) -> bool {
    let mut p = d.clone();
    p.timing = vec![
        (0., 0.),
        (
            d.frames as f64 / d.rate as f64,
            d.frames as f64 / d.rate as f64,
        ),
    ];
    p.changed()
}
pub fn timing_changed(d: &Edit) -> bool {
    d.timing.iter().any(|(a, b)| (a - b).abs() > 0.0001)
}
pub fn changed(d: &Edit) -> bool {
    pitch_changed(d) || timing_changed(d)
}
pub fn reset_pitch(d: &mut Edit) {
    d.curve.clear();
    d.formant = 0.;
    d.strength = 1.;
    d.enabled = true;
    for n in &mut d.notes {
        n.reset();
    }
}
pub fn analyzed(s: &State, ch: u16) -> bool {
    s.project
        .audio_edits
        .iter()
        .find(|h| h.channel == ch)
        .is_none_or(|h| h.analyzed)
}
pub fn analysis_ready(s: &mut State, ch: u16) {
    if let Some(h) = s.project.audio_edits.iter_mut().find(|h| h.channel == ch) {
        if let Some(d) = s.project.vocal_edits.iter().find(|d| d.channel == ch) {
            h.analyzed = true;
            if h.pitch.current.notes.is_empty() {
                h.pitch = os_model::audio_edit::History::new(Pitch::read(d));
            } else {
                h.pitch.commit(Pitch::read(d));
            }
        }
    }
}

/// Copies receive independent control documents before the next render is scheduled.
pub fn reconcile(s: &mut State) {
    if s.project.audio_edits.is_empty() {
        return;
    }
    // Most ticks only need to verify ownership. Do not clone a document's histories.
    let owners: std::collections::HashMap<_, _> = s.project.audio_edits.iter()
        .map(|d| (d.channel, (d.clip, d.arrangement))).collect();
    let mut seen = std::collections::HashSet::new();
    if s.project.arrangements.iter().all(|a| a.clips.iter().all(|c| {
        if c.id() == 0 || !seen.insert(c.id()) { return false; }
        match c.source {
            ClipSource::Channel(ch) => owners.get(&ch).is_none_or(|&(id, arrangement)| {
                id != 0 && id == c.id() && arrangement == a.number
            }),
            _ => true,
        }
    })) { return; }
    ids(s);
    let placements: Vec<_> = s
        .project
        .arrangements
        .iter()
        .enumerate()
        .flat_map(|(ai, a)| {
            a.clips
                .iter()
                .enumerate()
                .filter_map(move |(ci, c)| match c.source {
                    ClipSource::Channel(ch) => Some((ai, ci, ch)),
                    _ => None,
                })
        })
        .collect();
    for (ai, ci, ch) in placements {
        let Some(h) = s
            .project
            .audio_edits
            .iter()
            .find(|h| h.channel == ch)
        else {
            continue;
        };
        let clip = s.project.arrangements[ai].clips[ci].id();
        let arrangement = s.project.arrangements[ai].number;
        if h.clip == clip {
            if let Some(h) = s.project.audio_edits.iter_mut().find(|h| h.channel == ch) {
                h.arrangement = arrangement;
            }
            continue;
        }
        let shared = s
            .project
            .arrangements
            .iter()
            .flat_map(|a| &a.clips)
            .filter(|c| c.source == ClipSource::Channel(ch))
            .count()
            > 1;
        let id = if shared {
            let Some(next) = s
                .project
                .channels
                .iter()
                .map(|c| c.index)
                .max()
                .unwrap_or(0)
                .checked_add(1)
            else {
                continue;
            };
            let Some(mut c) = s.project.channel(ch).cloned() else {
                continue;
            };
            c.index = next;
            s.project.channels.push(c);
            crate::vocal::clone_source(s, ch, next);
            s.project.arrangements[ai].clips[ci].source = ClipSource::Channel(next);
            s.piano.dirty = true;
            next
        } else {
            ch
        };
        if let Some(h) = s.project.audio_edits.iter_mut().find(|h| h.channel == id) {
            h.clip = clip;
            h.arrangement = arrangement;
        }
    }
}

#[derive(Debug, Clone)]
pub enum Output {
    Export(os_model::Project, crate::export::Job, bool),
    Consolidate(os_model::Project, Vec<usize>, Option<(u32, u32)>),
    Tracks(os_model::Project),
}
pub fn alive(p: &os_model::Project, ch: u16) -> bool {
    p.arrangements.iter().any(|a| {
        a.clips.iter().any(|c| {
            c.source == ClipSource::Channel(ch)
                && p.audio_edits
                    .iter()
                    .find(|h| h.channel == ch)
                    .is_none_or(|h| h.clip == c.id() && h.arrangement == a.number)
        })
    })
}
pub fn owns(p: &os_model::Project, d: &Edit) -> bool {
    p.channel(d.channel)
        .and_then(|c| c.sample_path.as_deref())
        .is_some_and(|path| path == d.source || path == d.rendered)
}
pub fn pending(p: &os_model::Project) -> bool {
    p.vocal_edits.iter().any(|d| {
        alive(p, d.channel)
            && owns(p, d)
            && if changed(d) {
                d.render_key != crate::vocal::key(d)
                    || !std::path::Path::new(&crate::plugins::resolve_sample(&d.rendered)).is_file()
                    || p.channel(d.channel).and_then(|c| c.sample_path.as_deref())
                        != Some(d.rendered.as_str())
            } else {
                p.channel(d.channel).and_then(|c| c.sample_path.as_deref())
                    != Some(d.source.as_str())
            }
    })
}
pub fn poll_requests(app: &mut App) {
    if pending(&app.state.project) {
        return;
    }
    if let Some(play) = app.state.playlist.vocal.waiting.take() {
        match play {
            crate::warp::Play::Start(b) => app.start_playback_counted(b),
            crate::warp::Play::Seek(t) => app.play_song_from(t),
            crate::warp::Play::Resume => crate::warp::resume(app),
        }
    }
    let Some(request) = app.state.playlist.vocal.pending_output.take() else {
        return;
    };
    match request {
        Output::Export(mut project, job, song) => {
            let live: std::collections::HashSet<_> = project
                .vocal_edits
                .iter()
                .filter(|d| alive(&project, d.channel) && owns(&project, d))
                .map(|d| d.channel)
                .collect();
            for d in project
                .vocal_edits
                .iter_mut()
                .filter(|d| live.contains(&d.channel))
            {
                if let Some(now) = app.state.project.vocal_edits.iter().find(|now| {
                    now.channel == d.channel
                        && owns(&app.state.project, now)
                        && crate::vocal::key(now) == crate::vocal::key(d)
                }) {
                    d.rendered = now.rendered.clone();
                    d.render_key = now.render_key.clone();
                    if let Some(c) = project.channels.iter_mut().find(|c| c.index == d.channel) {
                        c.sample_path = Some(if changed(d) {
                            d.rendered.clone()
                        } else {
                            d.source.clone()
                        });
                    }
                } else {
                    app.state.message=Some("Export cancelled because audio edits changed while preparing it. Start export again.".into());
                    return;
                }
            }
            app.start_project_export(project, job, song);
        }
        Output::Consolidate(p, sel, range) => {
            if same_request(&p, &app.state.project)
                && sel == app.state.playlist.sel
                && range == app.state.playlist.range
            {
                app.state.message = Some(pl::consolidate(app));
            } else {
                app.state.message = Some(
                    "Consolidation cancelled because the selection or project changed.".into(),
                );
            }
        }
        Output::Tracks(p) => {
            if same_request(&p, &app.state.project) {
                app.state.message = Some(pl::export_tracks(app));
            } else {
                app.state.message =
                    Some("Track export cancelled because the project changed.".into());
            }
        }
    }
    app.invalidate();
}
fn same_request(a: &os_model::Project, b: &os_model::Project) -> bool {
    a.arrangements == b.arrangements
        && a.vocal_edits.len() == b.vocal_edits.len()
        && a.vocal_edits
            .iter()
            .zip(&b.vocal_edits)
            .all(|(a, b)| crate::vocal::key(a) == crate::vocal::key(b))
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use std::{
        path::PathBuf,
        sync::atomic::AtomicBool,
        time::{Duration, Instant},
    };
    pub(crate) fn fixture(name: &str) -> (App, PathBuf, usize) {
        let dir = std::env::temp_dir().join(format!("audio-edits-{name}-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("original.wav");
        let rate = 24000u32;
        let samples: Vec<f32> = (0..rate * 2)
            .flat_map(|i| {
                let t = i as f32 / rate as f32;
                let x = 0.2 * (t * 220. * std::f32::consts::TAU).sin();
                [x, x]
            })
            .collect();
        os_engine::wav::write_f32_stereo(
            &mut std::fs::File::create(&path).unwrap(),
            rate,
            &samples,
        )
        .unwrap();
        let mut app = App::new(None);
        app.state.project = crate::app::test_project();
        app.state.tempo = 120.;
        app.state.project.tempo = Some(120.);
        app.state.project.data_folder = Some(os_model::DataFolder {
            choice: 2,
            folder: dir.to_string_lossy().into_owned(),
        });
        let ci = pl::add_audio_clip(&mut app.state, &path.to_string_lossy(), 2, 0).unwrap();
        let ch = channel(&app.state, ci).unwrap();
        let source = path.to_string_lossy().into_owned();
        let mut d = Edit {
            channel: ch,
            source: source.clone(),
            source_hash: crate::vocal::file_hash(&source).unwrap(),
            rendered: source,
            render_key: String::new(),
            rate,
            frames: rate * 2,
            f0: vec![220.; 401].into(),
            notes: vec![os_model::vocal::Note::new(1, 400, 57.)],
            curve: vec![],
            timing: vec![(0., 0.), (2., 2.)],
            enabled: true,
            strength: 1.,
            formant: 0.,
        };
        d.render_key = crate::vocal::key(&d);
        app.state.project.vocal_edits.push(d);
        attach(&mut app.state, ci, true);
        app.state.playlist.vocal.ready = Some(true);
        app.state.playlist.vocal.row = Some(2);
        app.state.playlist.sel = vec![ci];
        (app, dir, ci)
    }
    pub(crate) fn render(app: &mut App) {
        let deadline = Instant::now() + Duration::from_secs(30);
        while pending(&app.state.project) || app.vocal_job.is_some() {
            crate::vocal::tick(app);
            assert!(
                Instant::now() < deadline,
                "render did not finish: {}",
                app.state.playlist.vocal.message
            );
            std::thread::sleep(Duration::from_millis(5));
        }
    }
    #[test]
    fn audio_edits_tune_warp_slice_local_undo_save_reopen_redo() {
        let (mut app, dir, ci) = fixture("roundtrip");
        let bytes = std::fs::read(dir.join("original.wav")).unwrap();
        // This check deliberately fails, rather than skips, if the signed native engine is unavailable.
        crate::vocal::engine::Engine::load().expect("verified native vocal engine required");
        snapshot(&mut app.state, "Tune");
        app.state.project.vocal_edits[0].notes[0].shift = 3.;
        finish(&mut app.state);
        render(&mut app);
        snapshot(&mut app.state, "Warp");
        app.state.project.vocal_edits[0].timing = vec![(0., 0.), (0.5, 0.3), (1., 1.2), (2., 2.)];
        finish(&mut app.state);
        pl::snapshot(&mut app.state);
        let c = pl::clips(&app.state)[ci].clone();
        let right = pl::split(&mut app.state, ci, c.position + c.length / 2).unwrap();
        let left_ch = channel(&app.state, ci).unwrap();
        let right_ch = channel(&app.state, right).unwrap();
        assert_ne!(left_ch, right_ch);
        let placement = pl::clips(&app.state).to_vec();
        let sibling = crate::vocal::doc(&app.state, right).unwrap().clone();
        history(&mut app, left_ch, true, false);
        assert_eq!(pl::clips(&app.state), placement);
        assert_eq!(crate::vocal::doc(&app.state, right).unwrap(), &sibling);
        assert_eq!(
            crate::vocal::doc(&app.state, ci).unwrap().notes[0].shift,
            3.
        );
        let restored = app.state.project.vocal_edits.clone();
        crate::history::undo(&mut app.state);
        assert_ne!(app.state.project.vocal_edits, restored);
        crate::history::redo(&mut app.state);
        assert_eq!(app.state.project.vocal_edits, restored);
        render(&mut app);
        let p =
            os_flp::project::load(&os_flp::project::create(&app.state.project).unwrap()).unwrap();
        assert_eq!(p.audio_edits, app.state.project.audio_edits);
        app.state.project = p;
        history(&mut app, left_ch, true, true);
        render(&mut app);
        for (a, b) in pl::clips(&app.state).iter().zip(&placement) {
            assert_eq!(
                (a.id(), a.position, a.length, a.source, a.window),
                (b.id(), b.position, b.length, b.source, b.window)
            );
        }
        assert_eq!(std::fs::read(dir.join("original.wav")).unwrap(), bytes);
        assert!(
            app.state
                .project
                .vocal_edits
                .iter()
                .all(|d| d.notes[0].shift == 3.)
        );
    }
    #[test]
    fn audio_edits_order_independence_and_bypass_retains_timing() {
        let (app, _, _) = fixture("order");
        let mut d = app.state.project.vocal_edits[0].clone();
        let sample = os_engine::sample::load_shared(std::path::Path::new(&d.source)).unwrap();
        let stop = AtomicBool::new(false);
        d.notes[0].shift = 2.;
        d.timing = vec![(0., 0.), (0.8, 1.1), (2., 2.)];
        let a = crate::vocal::render_edit(sample.clone(), &d, &stop).unwrap();
        let mut reverse = app.state.project.vocal_edits[0].clone();
        reverse.timing = d.timing.clone();
        reverse.notes[0].shift = 2.;
        let b = crate::vocal::render_edit(sample.clone(), &reverse, &stop).unwrap();
        assert_eq!(a, b);
        d.enabled = false;
        let bypass = crate::vocal::render_edit(sample.clone(), &d, &stop).unwrap();
        assert!(timing_changed(&d));
        assert!(!pitch_changed(&d));
        assert_ne!(bypass.0, sample.left);
        assert!(a.0.iter().all(|v| v.is_finite()));
        let analyzed = crate::vocal::engine::Engine::load()
            .unwrap()
            .analyze(&os_engine::Sample {
                rate: d.rate,
                left: a.0,
                right: a.1,
            })
            .unwrap();
        let mut voiced: Vec<_> = analyzed
            .into_iter()
            .skip(60)
            .take(240)
            .filter(|v| *v > 0.)
            .collect();
        voiced.sort_by(f32::total_cmp);
        let median = voiced[voiced.len() / 2];
        let mean = voiced.iter().sum::<f32>() / voiced.len() as f32;
        eprintln!(
            "Warped corrected pitch: median {median}, mean {mean}, bounds {}..{}",
            voiced[0],
            voiced[voiced.len() - 1]
        );
        assert!((median - 246.94).abs() < 6., "measured pitch {median}");
    }
    #[test]
    fn audio_edits_pending_export_and_source_failure_recover() {
        let (mut app, dir, _) = fixture("export");
        snapshot(&mut app.state, "Tune");
        app.state.project.vocal_edits[0].notes[0].shift = 2.;
        finish(&mut app.state);
        let out = dir.join("export.wav");
        app.start_export(crate::export::Job::wav(&out), true);
        assert!(app.export.is_none());
        assert!(app.state.playlist.vocal.pending_output.is_some());
        assert!(!out.exists());
        let hash = app.state.project.vocal_edits[0].source_hash.clone();
        app.state.project.vocal_edits[0].source_hash = "changed".into();
        let deadline = Instant::now() + Duration::from_secs(15);
        while !app
            .state
            .playlist
            .vocal
            .message
            .starts_with("Vocal tuning:")
        {
            crate::vocal::tick(&mut app);
            assert!(Instant::now() < deadline);
            std::thread::sleep(Duration::from_millis(5));
        }
        assert!(!out.exists());
        assert!(app.export.is_none());
        app.state.project.vocal_edits[0].source_hash = hash;
        crate::vocal::command(&mut app, crate::vocal::Command::Retry);
        render(&mut app);
        poll_requests(&mut app);
        assert!(app.export.is_some());
        let deadline = Instant::now() + Duration::from_secs(15);
        while app.export.is_some() {
            app.export_tick();
            assert!(Instant::now() < deadline);
            std::thread::sleep(Duration::from_millis(5));
        }
        assert!(out.is_file());
        assert!(std::fs::metadata(out).unwrap().len() > 1000);
    }
    #[test]
    fn audio_edits_save_and_consolidate_wait_for_committed_revision() {
        let (mut app, dir, _) = fixture("pending-save");
        app.state.project.vocal_edits[0].notes[0].shift = 2.;
        finish(&mut app.state);
        let path = dir.join("pending.lmn");
        app.save_copy(path.clone())
            .expect("save available")
            .expect("save completed");
        let saved =
            os_flp::project::load(&os_flp::FlpFile::parse(&std::fs::read(path).unwrap()).unwrap())
                .unwrap();
        assert_eq!(saved.audio_edits, app.state.project.audio_edits);
        assert!(pending(&saved));
        assert!(pl::consolidate(&mut app).contains("Preparing current"));
        assert!(matches!(
            app.state.playlist.vocal.pending_output,
            Some(Output::Consolidate(..))
        ));
        assert!(pl::export_tracks(&mut app).contains("Preparing current"));
        assert!(matches!(
            app.state.playlist.vocal.pending_output,
            Some(Output::Tracks(..))
        ));
        crate::vocal::command(&mut app, crate::vocal::Command::Cancel);
        assert!(app.state.playlist.vocal.pending_output.is_none());
        assert!(app.export.is_none());
        app.state.project = saved;
        app.state.playlist.vocal.ready = Some(true);
        render(&mut app);
        assert!(!pending(&app.state.project));
    }

    #[test]
    fn audio_edits_older_reader_keeps_finished_audio_without_editing_metadata() {
        let (mut app, _, ci) = fixture("old-reader");
        app.state.project.vocal_edits[0].enabled = false;
        app.state.project.vocal_edits[0].timing = vec![(0., 0.), (1., 0.7), (2., 2.)];
        finish(&mut app.state);
        render(&mut app);
        let ch = channel(&app.state, ci).unwrap();
        let path = app.state.project.channel(ch).unwrap().sample_path.clone();
        let mut file = os_flp::project::create(&app.state.project).unwrap();
        let restored = os_flp::project::load(&file).unwrap();
        assert_eq!(restored.vocal_edits, app.state.project.vocal_edits);
        assert_eq!(restored.audio_edits, app.state.project.audio_edits);
        file.chunks.retain(|c| c.tag != os_flp::Tag(*b"LSAE"));
        let old = os_flp::project::load(&file).unwrap();
        assert!(old.audio_edits.is_empty());
        assert!(old.vocal_edits.is_empty());
        assert_eq!(old.channel(ch).unwrap().sample_path, path);
    }

    #[test]
    fn audio_edits_copy_to_other_arrangement_and_sync() {
        let (mut app, _, ci) = fixture("arr-copy");
        app.state.project.vocal_edits[0].notes[0].shift = 2.;
        finish(&mut app.state);
        let original = channel(&app.state, ci).unwrap();
        let mut a = app.state.project.arrangement().unwrap().clone();
        a.number = 12;
        app.state.project.arrangements.push(a);
        reconcile(&mut app.state);
        let os_model::ClipSource::Channel(copied) =
            app.state.project.arrangements.last().unwrap().clips[ci].source
        else {
            panic!()
        };
        assert_ne!(original, copied);
        let before = app
            .state
            .project
            .vocal_edits
            .iter()
            .find(|d| d.channel == copied)
            .unwrap()
            .clone();
        app.state
            .project
            .vocal_edits
            .iter_mut()
            .find(|d| d.channel == original)
            .unwrap()
            .notes[0]
            .shift = 3.;
        finish(&mut app.state);
        assert_eq!(
            app.state
                .project
                .vocal_edits
                .iter()
                .find(|d| d.channel == copied)
                .unwrap(),
            &before
        );
        let shared = crate::collaboration::capture(&app.state.project).unwrap();
        assert_eq!(
            shared["project/audio_edits"],
            serde_json::to_value(&app.state.project.audio_edits).unwrap()
        );
    }
    #[test]
    fn audio_edits_normalized_slice_history_renders_extreme_ratios() {
        let (mut app, _, ci) = fixture("slice-limits");
        let ch = channel(&app.state, ci).unwrap();
        app.state.project.vocal_edits[0].timing = vec![(0., 0.), (0.5, 0.03125), (2., 2.)];
        finish(&mut app.state);
        app.state.project.vocal_edits[0].timing = vec![(0., 0.), (0.5, 1.9), (2., 2.)];
        finish(&mut app.state);
        let end = app.state.project.vocal_edits[0].destination(0.51);
        pl::arr_mut(&mut app.state).clips[ci].window = Some((0., end * 1000.));
        restrict(&mut app.state, ci);
        history(&mut app, ch, true, false);
        assert!(
            !crate::vocal::timing_valid(&app.state.project.vocal_edits[0]),
            "normalizing history can exceed interactive segment limits"
        );
        assert!(app.state.project.vocal_edits[0].valid());
        render(&mut app);
        assert!(!pending(&app.state.project));
    }
    #[test]
    fn audio_edits_pending_resume_preserves_pattern_mode() {
        let (mut app, _, _) = fixture("resume");
        app.state.song_mode = false;
        app.state.project.vocal_edits[0].notes[0].shift = 2.;
        finish(&mut app.state);
        crate::warp::resume(&mut app);
        assert!(matches!(
            app.state.playlist.vocal.waiting,
            Some(crate::warp::Play::Resume)
        ));
        render(&mut app);
        assert!(app.state.playlist.vocal.waiting.is_none());
        assert!(!app.state.song_mode);
        assert!(!app.transport.song());
    }

    #[test]
    fn audio_edits_play_wait_stop_undo_and_replaced_source() {
        let (mut app, _, ci) = fixture("play-gate");
        let ch = channel(&app.state, ci).unwrap();
        app.state.project.vocal_edits[0].notes[0].shift = 2.;
        finish(&mut app.state);
        assert!(crate::warp::wait_for_play(
            &mut app,
            crate::warp::Play::Start(0)
        ));
        assert!(!app.transport.playing());
        assert!(crate::warp::cancel_play(&mut app));
        assert!(app.state.playlist.vocal.waiting.is_none());
        render(&mut app);
        assert!(!app.transport.playing());
        history(&mut app, ch, false, false);
        assert!(
            pending(&app.state.project),
            "Undo must publish the original before a fresh Play"
        );
        crate::vocal::tick(&mut app);
        assert!(!pending(&app.state.project));
        assert_eq!(
            app.state
                .project
                .channel(ch)
                .unwrap()
                .sample_path
                .as_deref(),
            Some(app.state.project.vocal_edits[0].source.as_str())
        );
        history(&mut app, ch, false, true);
        app.state
            .project
            .channels
            .iter_mut()
            .find(|c| c.index == ch)
            .unwrap()
            .sample_path = Some("replacement.wav".into());
        assert!(
            !pending(&app.state.project),
            "detached metadata must not block playback"
        );
        crate::vocal::tick(&mut app);
        assert_eq!(
            app.state
                .project
                .channel(ch)
                .unwrap()
                .sample_path
                .as_deref(),
            Some("replacement.wav")
        );
        assert!(app.vocal_job.is_none());
    }
    #[test]
    fn audio_edits_history_limits_layers_and_manual_controls() {
        let (mut app, _, ci) = fixture("histories");
        let ch = channel(&app.state, ci).unwrap();
        let c = app
            .state
            .project
            .channels
            .iter_mut()
            .find(|c| c.index == ch)
            .unwrap();
        c.pitch_cents = 200;
        c.misc.fine = -25;
        crate::loop_fit::set_fitted_time(&mut c.sound, 6.);
        let sound = c.sound.clone();
        pl::arr_mut(&mut app.state).clips[ci].position = 384;
        app.state.tempo = 137.;
        app.state.project.tempo = Some(137.);
        for i in 1..=70 {
            app.state.project.vocal_edits[0].notes[0].shift = i as f32 / 10.;
            finish(&mut app.state);
        }
        app.state.project.vocal_edits[0].timing = vec![(0., 0.), (1., 0.7), (2., 2.)];
        finish(&mut app.state);
        history(&mut app, ch, true, false);
        history(&mut app, ch, false, false);
        app.state.project.vocal_edits[0].notes[0].shift = 8.;
        finish(&mut app.state);
        let h = &app.state.project.audio_edits[0];
        assert_eq!(h.pitch.undo.len(), 64);
        assert!(h.pitch.redo.is_empty());
        assert_eq!(h.timing.redo.len(), 1);
        let p =
            os_flp::project::load(&os_flp::project::create(&app.state.project).unwrap()).unwrap();
        assert_eq!(p.audio_edits, app.state.project.audio_edits);
        assert_eq!(p.channel(ch).unwrap().sound.stretch, sound.stretch);
        assert_eq!(pl::clips(&app.state)[ci].position, 384);
        app.state.project = p;
        history(&mut app, ch, true, true);
        render(&mut app);
        assert_eq!(
            app.state.project.channel(ch).unwrap().sound.stretch,
            sound.stretch
        );
        assert_eq!(app.state.project.channel(ch).unwrap().misc.fine, -25);
        assert_eq!(app.state.project.channel(ch).unwrap().pitch_cents, 200);
    }
    #[test]
    fn audio_edits_copy_trim_history_and_cancel_are_independent() {
        let (mut app, _, ci) = fixture("copy");
        snapshot(&mut app.state, "Warp");
        app.state.project.vocal_edits[0].timing = vec![(0., 0.), (1., 0.7), (2., 2.)];
        finish(&mut app.state);
        pl::snapshot(&mut app.state);
        let mut copy = pl::clips(&app.state)[ci].clone();
        copy.position += copy.length;
        copy.set_id(0);
        pl::arr_mut(&mut app.state).clips.push(copy);
        reconcile(&mut app.state);
        let copy = pl::clips(&app.state).len() - 1;
        let ch = channel(&app.state, copy).unwrap();
        assert_ne!(ch, channel(&app.state, ci).unwrap());
        let original = crate::vocal::doc(&app.state, ci).unwrap().clone();
        history(&mut app, ch, true, false);
        assert_eq!(crate::vocal::doc(&app.state, ci).unwrap(), &original);
        let before_trim = app.state.project.audio_edits.clone();
        let clip = pl::clips(&app.state)[ci].clone();
        pl::arr_mut(&mut app.state).clips[ci].window = Some((500., 1500.));
        reconcile(&mut app.state);
        pl::arr_mut(&mut app.state).clips[ci] = clip;
        reconcile(&mut app.state);
        assert_eq!(
            app.state.project.audio_edits, before_trim,
            "trimming and extending must retain hidden controls"
        );
        let before = app.state.project.audio_edits.clone();
        finish(&mut app.state);
        assert_eq!(app.state.project.audio_edits, before);
        assert_eq!(
            app.state
                .project
                .audio_edits
                .iter()
                .find(|d| d.channel == ch)
                .unwrap()
                .timing
                .redo
                .len(),
            1
        );
    }
}
