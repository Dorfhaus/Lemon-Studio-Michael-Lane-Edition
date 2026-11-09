// SPDX-License-Identifier: GPL-3.0-or-later
//! Editable audio timing anchors, rendered off the audio/UI threads with the source kept intact.
use crate::{
    app::{Action, App, State, Ui},
    playlist as pl,
};
use os_model::{AudioWarp, ClipSource};
use os_ui::{Color, Font, Rect};
use std::{
    cell::Cell,
    path::PathBuf,
    sync::{Arc, Mutex},
};
pub struct Loading {
    pub original: os_model::Clip,
    arrangement: usize,
    job: Arc<Mutex<Option<Result<Form, String>>>>,
}
type Job = Arc<Mutex<Option<Result<PathBuf, String>>>>;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Command {
    Source,
    Select,
    Marker(usize),
    Delete,
    Reset,
    Fit,
    Snap,
    Apply,
    Toggle(u16),
    Close,
    Analyze,
    Quantize,
    Grid,
    Strength,
    Undo,
    Redo,
    Next,
}
pub struct Form {
    pub arrangement: usize,
    pub transients: Vec<f64>,
    pub analysis: Option<Arc<Mutex<Option<Vec<f64>>>>>,
    pub grid: usize,
    pub strength: usize,
    pub offset: f64,
    pub undo: Vec<Vec<(f64, f64)>>,
    pub redo: Vec<Vec<(f64, f64)>>,
    pub clip: usize,
    pub original: os_model::Clip,
    pub channel: os_model::Channel,
    pub edit: AudioWarp,
    pub sample: Arc<os_engine::Sample>,
    pub selected: Option<usize>,
    pub select_mode: bool,
    pub selection: Vec<f64>,
    selection_start: f64,
    selection_base: Vec<f64>,
    pub amplitude: f32,
    pub wave: Cell<Rect>,
    pub snap: bool,
    pub tempo: f64,
    pub pending: Option<Job>,
    pub message: String,
}
pub const EXTRA: f32 = 248.0;
const GRIDS: [f64; 6] = [1.0, 0.5, 0.25, 0.125, 1.0 / 3.0, 1.0 / 6.0];
const GRID_NAMES: [&str; 6] = ["1/4", "1/8", "1/16", "1/32", "1/8 triplet", "1/16 triplet"];
pub fn expanded(s: &State, row: u16) -> bool {
    if !crate::workspace_visibility::shown("visible_playlist-warp") {
        return false;
    }
    s.playlist
        .warp
        .as_ref()
        .is_some_and(|f| f.original.track == row)
        || s.playlist
            .warp_loading
            .as_ref()
            .is_some_and(|f| f.original.track == row)
}
fn interpolate(points: &[(f64, f64)], x: f64, inverse: bool) -> f64 {
    let axis = |p: (f64, f64)| if inverse { (p.1, p.0) } else { p };
    let i = points
        .partition_point(|&p| axis(p).0 <= x)
        .clamp(1, points.len() - 1);
    let (a, b) = axis(points[i - 1]);
    let (c, d) = axis(points[i]);
    b + (x - a) / (c - a) * (d - b)
}
impl Form {
    fn select_to(&mut self, x: f32) {
        let r = self.wave.get();
        let beat = ((x - r.x) / r.w).clamp(0., 1.) as f64 * self.end();
        let tolerance = if (beat - self.selection_start).abs() * r.w as f64 / self.end() < 6. {
            6. * self.end() / r.w as f64
        } else {
            0.
        };
        let lo = beat.min(self.selection_start) - tolerance;
        let hi = beat.max(self.selection_start) + tolerance;
        self.selection = self.selection_base.clone();
        for src in self
            .transients
            .iter()
            .copied()
            .chain(self.edit.points.iter().map(|p| p.0))
        {
            let dst = interpolate(&self.edit.points, src, false);
            if dst >= lo && dst <= hi && !self.selection.contains(&src) {
                self.selection.push(src);
            }
        }
        self.message = format!(
            "{} points selected  -  Quantize applies only to these points. Shift-drag adds more.",
            self.selection.len()
        );
    }
    fn checkpoint(&mut self) {
        if self.undo.last() != Some(&self.edit.points) {
            self.undo.push(self.edit.points.clone());
        }
        if self.undo.len() > 64 {
            self.undo.remove(0);
        }
        self.redo.clear();
    }

    fn end(&self) -> f64 {
        self.edit.points.last().unwrap().1
    }
    fn range(&self) -> (f64, f64) {
        (self.edit.points[0].0, self.edit.points.last().unwrap().0)
    }
    fn moved(&mut self, i: usize, x: f32) {
        if self.pending.is_some() || i == 0 || i + 1 >= self.edit.points.len() {
            return;
        }
        let r = self.wave.get();
        let mut beat = ((x - r.x) / r.w).clamp(0., 1.) as f64 * self.end();
        if self.snap {
            let step = GRIDS[self.grid];
            beat = ((beat + self.offset) / step).round() * step - self.offset;
        }
        let gap = ((self.edit.points[i + 1].1 - self.edit.points[i - 1].1) * 0.25).min(0.001);
        self.edit.points[i].1 = beat.clamp(
            self.edit.points[i - 1].1 + gap,
            self.edit.points[i + 1].1 - gap,
        );
    }
}
pub fn open(app: &mut App, i: usize) -> Result<(), String> {
    app.drag = None;
    let clip = pl::clips(&app.state)
        .get(i)
        .cloned()
        .ok_or("Select an audio clip")?;
    let ClipSource::Channel(id) = clip.source else {
        return Err("Select an audio clip".into());
    };
    let ch = app
        .state
        .project
        .channel(id)
        .cloned()
        .ok_or("Missing channel")?;
    if crate::ara::is(&ch) {
        return Err("Use the ARA editor for this clip".into());
    }
    let path = ch.sample_path.as_deref().ok_or("The clip has no sample")?;
    if ch.sound.start_offset() != 0.
        || ch.sound.fx.start() != 0.
        || ch.sound.fx.length() != 1.
        || ch.sound.fx.trim != 0
        || ch.sound.fx.flags & os_model::SampleFx::REVERSE != 0
    {
        return Err("Render this clip to audio before warping its sampler trim, offset or reverse processing".into());
    }
    // A trimmed render is a new source window; keep that audio intact rather than
    // incorrectly mapping its milliseconds onto the earlier unwarped source.
    let resolved = crate::plugins::resolve_sample(path);
    let saved = app
        .state
        .project
        .audio_warps
        .iter()
        .find(|w| {
            clip.window.is_none()
                && (w.rendered == path || crate::plugins::resolve_sample(&w.rendered) == resolved)
        })
        .cloned();
    let tempo = app.state.project.tick_tempo(app.state.tempo);
    let ppq = app.state.project.ppq;
    let arrangement = app.state.project.arrangement_index();
    let done = Arc::new(Mutex::new(None));
    app.state.playlist.vocal.row = None;
    app.state.playlist.warp = None;
    app.state.playlist.warp_loading = Some(Loading {
        original: clip.clone(),
        arrangement,
        job: done.clone(),
    });
    std::thread::spawn(move || {
        let result = prepare(i, clip, ch, saved, tempo, ppq, arrangement);
        if let Ok(mut out) = done.lock() {
            *out = Some(result);
        }
    });
    app.invalidate();
    Ok(())
}
fn prepare(
    i: usize,
    clip: os_model::Clip,
    ch: os_model::Channel,
    saved: Option<AudioWarp>,
    tempo: f64,
    ppq: u16,
    arrangement: usize,
) -> Result<Form, String> {
    let source = saved
        .as_ref()
        .map_or(ch.sample_path.as_deref().unwrap(), |w| w.source.as_str());
    let source = crate::plugins::resolve_sample(source);
    if std::fs::metadata(&source).is_ok_and(|m| m.len() > 128 * 1024 * 1024) {
        return Err("Split this large recording into smaller clips before warping".into());
    }
    let sample =
        os_engine::sample::load_shared(std::path::Path::new(&source)).map_err(|e| e.to_string())?;
    let secs = sample.frames() as f64 / sample.rate.max(1) as f64;
    let (a, b) = clip
        .window
        .map(|(a, b)| (a / 1000., b / 1000.))
        .unwrap_or((0., secs));
    let end = clip.length as f64 / ppq.max(1) as f64;
    if end <= 0. || b.min(secs) - a.max(0.) < 0.01 {
        return Err("The clip is too short to warp".into());
    }
    let mut edit = saved.unwrap_or(AudioWarp {
        source: source.clone(),
        rendered: String::new(),
        points: vec![(a.max(0.), 0.), (b.min(secs), end)],
    });
    if edit.points.len() < 2
        || edit.points.len() > 1024
        || edit
            .points
            .iter()
            .any(|p| !p.0.is_finite() || !p.1.is_finite() || p.0 < 0.0 || p.0 > secs)
        || edit.points.last().is_some_and(|p| p.1 <= 0.0)
        || edit.points.windows(2).any(|p| {
            !p[0].0.is_finite() || !p[0].1.is_finite() || p[1].0 <= p[0].0 || p[1].1 <= p[0].1
        })
    {
        return Err("This saved warp has invalid anchors".into());
    }
    edit.source = source;
    let scale = end / edit.points.last().unwrap().1;
    for p in &mut edit.points {
        p.1 *= scale;
    }
    Ok(Form {
        arrangement,
        transients: vec![],
        analysis: None,
        grid: 2,
        strength: 4,
        offset: clip.position as f64 / ppq.max(1) as f64,
        undo: vec![],
        redo: vec![],
        clip: i,
        original: clip,
        channel: ch,
        edit,
        sample,
        selected: None,
        select_mode: false,
        selection: vec![],
        selection_start: 0.,
        selection_base: vec![],
        amplitude: 1.,
        wave: Cell::new(Rect::new(0., 0., 1., 1.)),
        snap: true,
        tempo,
        pending: None,
        message: "Detecting transients...".into(),
    })
}
pub fn command(app: &mut App, c: Command) {
    if c == Command::Fit {
        let Some(f) = app.state.playlist.warp.as_ref() else {
            return;
        };
        let width = app.pl_grid().map_or(800.0, |r| r.w);
        let beats = f.end();
        let offset = f.original.position as f32 / app.state.project.ppq.max(1) as f32;
        app.state.playlist_bar_w = ((width - 32.).max(80.) * 4. / beats as f32).clamp(4., 48000.);
        app.state.playlist_scroll.0 = (offset * app.state.playlist_bar_w / 4. - 16.).max(0.);
        app.invalidate();
        return;
    }

    if let Command::Toggle(row) = c {
        if expanded(&app.state, row) {
            app.state.playlist.warp = None;
            app.state.playlist.warp_loading = None;
            app.invalidate();
            return;
        }
        let clips = pl::clips(&app.state);
        let audio = |i: usize| {
            clips.get(i).is_some_and(|clip|clip.track==row && matches!(clip.source,ClipSource::Channel(id) if app.state.project.channel(id).is_some_and(|ch|ch.sample_path.is_some())))
        };
        let i = app
            .state
            .playlist
            .sel
            .iter()
            .copied()
            .find(|&i| audio(i))
            .or_else(|| (0..clips.len()).find(|&i| audio(i)));
        app.state.message = match i {
            Some(i) => open(app, i).err(),
            None => Some("Choose a track containing audio to warp".into()),
        };
        app.invalidate();
        return;
    }
    if c == Command::Close {
        app.drag = None;
        app.state.playlist.warp = None;
        app.state.playlist.warp_loading = None;
        app.invalidate();
        return;
    }
    if c == Command::Next {
        let Some(f) = app.state.playlist.warp.as_ref() else {
            return;
        };
        let clips = pl::clips(&app.state);
        let row = f.original.track;
        let current = f.clip;
        let next=(1..=clips.len()).map(|n|(current+n)%clips.len()).find(|&i|clips[i].track==row && matches!(clips[i].source,ClipSource::Channel(id) if app.state.project.channel(id).is_some_and(|ch|ch.sample_path.is_some())));
        if let Some(i) = next {
            app.state.message = open(app, i).err();
        }
        app.invalidate();
        return;
    }
    let x = app.mouse.0;
    let Some(f) = app.state.playlist.warp.as_mut() else {
        return;
    };
    if f.pending.is_some() {
        return;
    }
    if matches!(
        c,
        Command::Source | Command::Marker(_) | Command::Delete | Command::Reset | Command::Quantize
    ) {
        f.checkpoint();
    }
    match c {
        Command::Toggle(_) | Command::Close | Command::Next | Command::Fit => {}
        Command::Select => {
            f.select_mode = !f.select_mode;
            f.selection.clear();
            f.selected = None;
            f.message = if f.select_mode {"Drag across points to select; Shift-drag adds. Then Quantize."} else {"Click to add a marker; drag its handle. Ctrl+wheel zooms time; Alt+wheel enlarges waveform."}.into();
        }
        Command::Grid => f.grid = (f.grid + 1) % GRIDS.len(),
        Command::Strength => f.strength = (f.strength + 1) % 5,
        Command::Undo => {
            if let Some(points) = f.undo.pop() {
                f.redo.push(std::mem::replace(&mut f.edit.points, points));
                f.selected = None;
            }
        }
        Command::Redo => {
            if let Some(points) = f.redo.pop() {
                f.undo.push(std::mem::replace(&mut f.edit.points, points));
                f.selected = None;
            }
        }
        Command::Analyze => {
            if f.analysis.is_some() {
                return;
            }
            let done = Arc::new(Mutex::new(None));
            f.analysis = Some(done.clone());
            let sample = f.sample.clone();
            let (a, b) = f.range();
            f.message = "Detecting transients...".into();
            std::thread::spawn(move || {
                let start = (a * sample.rate as f64).round() as usize;
                let end = ((b * sample.rate as f64).round() as usize).min(sample.frames());
                let times = crate::vocal_align::transient_times(sample, start, end)
                    .into_iter()
                    .take(1022)
                    .map(|t| a + t)
                    .collect();
                if let Ok(mut result) = done.lock() {
                    *result = Some(times);
                }
            });
        }
        Command::Quantize => {
            if f.analysis.is_some() {
                f.message = "Wait for transient detection to finish".into();
                return;
            }
            if f.select_mode && f.selection.is_empty() {
                f.message = "Select points first, or switch to Edit to quantize all.".into();
                return;
            }
            let (a, b) = f.range();
            let mut sources: Vec<_> = f.edit.points.iter().map(|p| p.0).collect();
            sources.extend(
                f.transients
                    .iter()
                    .copied()
                    .filter(|src| !f.select_mode || f.selection.contains(src)),
            );
            sources.sort_by(f64::total_cmp);
            sources.dedup_by(|a, b| (*a - *b).abs() < 0.000001);
            let mut points: Vec<_> = sources
                .into_iter()
                .filter(|src| *src >= a && *src <= b)
                .map(|src| (src, interpolate(&f.edit.points, src, false)))
                .collect();
            let step = GRIDS[f.grid];
            let offset = f.offset;
            let mut count = 0;
            for i in 1..points.len() - 1 {
                if f.select_mode && !f.selection.contains(&points[i].0) {
                    continue;
                }
                let beat = points[i].1;
                let snapped = ((beat + offset) / step).round() * step - offset;
                let dst = beat + (snapped - beat) * f.strength as f64 / 4.;
                // Keep every other anchor intact and prevent crossings.
                let lo = points[i - 1].1 + 0.000001;
                let hi = points[i + 1].1 - 0.000001;
                if lo < hi {
                    points[i].1 = dst.clamp(lo, hi);
                    count += 1;
                }
            }
            f.edit.points = points;
            f.selected = None;
            f.message = format!("{count} points quantized. Apply to hear, or Undo to compare.");
        }
        Command::Source => {
            if f.select_mode {
                let r = f.wave.get();
                f.selection_start = ((x - r.x) / r.w).clamp(0., 1.) as f64 * f.end();
                f.selection_base = if app.shift {
                    f.selection.clone()
                } else {
                    vec![]
                };
                f.select_to(x);
                app.drag = Some(crate::app::Drag::WarpMarker(usize::MAX));
                app.invalidate();
                return;
            }
            let r = f.wave.get();
            let beat = ((x - r.x) / r.w).clamp(0., 1.) as f64 * f.end();
            let mut src = interpolate(&f.edit.points, beat, true);
            if let Some(&near) = f.transients.iter().min_by(|a, b| {
                (interpolate(&f.edit.points, **a, false) - beat)
                    .abs()
                    .total_cmp(&(interpolate(&f.edit.points, **b, false) - beat).abs())
            }) {
                if ((interpolate(&f.edit.points, near, false) - beat) / f.end() * r.w as f64).abs()
                    < 8.0
                {
                    src = near;
                }
            }
            if f.edit.points.len() >= 1024 {
                return;
            }
            let i = f.edit.points.partition_point(|p| p.0 < src);
            if i == 0
                || i == f.edit.points.len()
                || src - f.edit.points[i - 1].0 < 0.005
                || f.edit.points[i].0 - src < 0.005
            {
                return;
            }
            let (l, r) = (f.edit.points[i - 1], f.edit.points[i]);
            let dst = l.1 + (src - l.0) / (r.0 - l.0) * (r.1 - l.1);
            f.edit.points.insert(i, (src, dst));
            f.selected = Some(i);
            app.drag = Some(crate::app::Drag::WarpMarker(i));
        }
        Command::Marker(i) => {
            if f.select_mode {
                let src = f.edit.points[i].0;
                if !app.shift {
                    f.selection.clear();
                }
                if let Some(at) = f.selection.iter().position(|p| *p == src) {
                    f.selection.remove(at);
                } else {
                    f.selection.push(src);
                }
                app.invalidate();
                return;
            }
            f.selected = Some(i);
            if i > 0 && i + 1 < f.edit.points.len() {
                app.drag = Some(crate::app::Drag::WarpMarker(i));
            }
        }
        Command::Delete => {
            if f.select_mode {
                let (a, b) = f.range();
                f.edit
                    .points
                    .retain(|p| p.0 == a || p.0 == b || !f.selection.contains(&p.0));
                f.selection.clear();
            }
            if let Some(i) = f
                .selected
                .take()
                .filter(|&i| i > 0 && i + 1 < f.edit.points.len())
            {
                f.edit.points.remove(i);
            }
        }
        Command::Reset => {
            let (a, b) = f.range();
            let end = f.end();
            f.edit.points = vec![(a, 0.), (b, end)];
            f.selected = None;
        }
        Command::Snap => f.snap = !f.snap,
        Command::Apply => {
            let points: Vec<_> = f
                .edit
                .points
                .iter()
                .map(|&(src, beat)| {
                    (
                        (src * f.sample.rate as f64).round() as usize,
                        (beat * 60. / f.tempo * f.sample.rate as f64).round() as usize,
                    )
                })
                .collect();
            let sample = f.sample.clone();
            let done = Arc::new(Mutex::new(None));
            f.pending = Some(done.clone());
            f.message = "Rendering warp... Original audio is preserved.".into();
            let dir = crate::pattern_render::folder(&app.state).join("Warped");
            std::thread::spawn(move || {
                let result = (|| {
                    let (l, r) =
                        os_dsp::stretch::warp(&sample.left, &sample.right, sample.rate, &points)?;
                    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
                    let stamp = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_nanos();
                    let path = dir.join(format!("Warp-{stamp}.wav"));
                    let mut file = std::fs::OpenOptions::new()
                        .write(true)
                        .create_new(true)
                        .open(&path)
                        .map_err(|e| e.to_string())?;
                    let stereo: Vec<_> = l.into_iter().zip(r).flat_map(|(l, r)| [l, r]).collect();
                    os_engine::wav::write_f32_stereo(&mut file, sample.rate, &stereo)
                        .map_err(|e| e.to_string())?;
                    Ok(path)
                })();
                if let Ok(mut out) = done.lock() {
                    *out = Some(result);
                }
            });
        }
    }
    app.invalidate();
}
pub fn drag(app: &mut App, i: usize, x: f32) {
    if let Some(f) = app.state.playlist.warp.as_mut() {
        if i == usize::MAX {
            f.select_to(x);
        } else {
            f.moved(i, x);
        }
        app.invalidate();
    }
}
pub fn wheel(app: &mut App, dx: f32, dy: f32, ctrl: bool) -> bool {
    if !crate::workspace_visibility::shown("visible_playlist-warp") {
        return false;
    }
    if !matches!(
        app.top_hit(app.mouse.0, app.mouse.1).map(|h| &h.action),
        Some(Action::Warp(_))
    ) {
        return false;
    }
    let Some(f) = app.state.playlist.warp.as_mut() else {
        return false;
    };
    if !f.wave.get().contains(app.mouse.0, app.mouse.1) || (!ctrl && !app.alt) {
        return false;
    }
    if app.drag.is_some() {
        return true;
    }
    let factor = 1.18f32.powf((dx + dy).clamp(-12., 12.));
    if app.alt {
        f.amplitude = (f.amplitude * factor).clamp(0.25, 16.);
    } else {
        crate::vocal::navigation::zoom_x(app, factor, app.mouse.0);
    }
    app.invalidate();
    true
}
pub fn poll(app: &mut App) {
    if let Some(load) = app.state.playlist.warp_loading.as_ref() {
        let valid = load.arrangement == app.state.project.arrangement_index()
            && pl::clips(&app.state).contains(&load.original);
        let result = load.job.lock().ok().and_then(|mut job| job.take());
        if !valid {
            app.state.playlist.warp_loading = None;
            app.invalidate();
        } else if let Some(result) = result {
            app.state.playlist.warp_loading = None;
            match result {
                Ok(form) => {
                    app.state.playlist.warp = Some(form);
                    command(app, Command::Analyze);
                    command(app, Command::Fit);
                }
                Err(error) => app.state.message = Some(error),
            }
            app.invalidate();
        }
    }
    if app.state.playlist.warp.as_ref().is_some_and(|f| {
        f.arrangement != app.state.project.arrangement_index()
            || pl::clips(&app.state).get(f.clip) != Some(&f.original)
            || app.state.project.channel(f.channel.index) != Some(&f.channel)
    }) {
        app.state.playlist.warp = None;
        app.state.playlist.warp_loading = None;
        app.invalidate();
        return;
    }
    let Some(f) = app.state.playlist.warp.as_mut() else {
        return;
    };
    let detected = f.analysis.as_ref().and_then(|job| job.lock().ok()?.take());
    if let Some(times) = detected {
        f.transients = times;
        f.analysis = None;
        f.message = format!(
            "{} transients detected. Drag markers or Quantize, then Apply to hear.",
            f.transients.len()
        );
        app.invalidate();
    }
    let Some(f) = app.state.playlist.warp.as_mut() else {
        return;
    };
    let result = f.pending.as_ref().and_then(|p| p.lock().ok()?.take());
    let Some(result) = result else { return };
    f.pending = None;
    let path = match result {
        Ok(p) => p,
        Err(e) => {
            f.message = e;
            app.invalidate();
            return;
        }
    };
    let Some(mut f) = app.state.playlist.warp.take() else {
        return;
    };
    if app.state.project.arrangement_index() != f.arrangement
        || pl::clips(&app.state).get(f.clip) != Some(&f.original)
    {
        app.state.message = Some("The clip changed; warp was rendered but not applied".into());
        return;
    }
    let Some(index) = app
        .state
        .project
        .channels
        .iter()
        .map(|c| c.index)
        .max()
        .unwrap_or(0)
        .checked_add(1)
    else {
        app.state.message = Some("No channel IDs available".into());
        return;
    };
    let row = app.state.project.channels.len();
    let before = crate::history::Snap::Several(vec![
        crate::history::Snap::Rack(Box::new(crate::history::RackSide::now(&app.state, row))),
        crate::history::Snap::AudioWarps(app.state.project.audio_warps.clone()),
    ]);
    pl::snapshot_with(&mut app.state, "Warp audio", before);
    f.channel.index = index;
    f.channel.sample_path = Some(path.to_string_lossy().into_owned());
    f.channel.plugin_state = None;
    f.channel.plugin = None;
    f.channel.kind = os_model::ChannelKind::AudioClip;
    let end = f.end();
    f.channel.sound.stretch[0] = 0;
    f.channel.sound.stretch[2] = 0;
    crate::loop_fit::set_fitted_time(&mut f.channel.sound, end);
    f.channel.name = Some(format!(
        "{} (warped)",
        f.channel.name.as_deref().unwrap_or("Audio")
    ));
    crate::rack::push_channel(&mut app.state, f.channel);
    f.edit.rendered = path.to_string_lossy().into_owned();
    app.state.project.audio_warps.push(f.edit.clone());
    let ppq = app.state.project.ppq.max(1);
    if let Some(c) = pl::arr_mut(&mut app.state).clips.get_mut(f.clip) {
        c.source = ClipSource::Channel(index);
        c.length = (end * ppq as f64).round().max(1.) as u32;
        c.window = None;
    }
    app.state.piano.dirty = true;
    app.state.message = Some(
        "Warp applied. Reopen Audio timing to edit; Ctrl+Z restores the original clip.".into(),
    );
    app.invalidate();
}
pub fn key_input(app: &mut App, key: &winit::keyboard::Key, ctrl: bool, alt: bool) -> bool {
    if !crate::workspace_visibility::shown("visible_playlist-warp") {
        return false;
    }
    use winit::keyboard::{Key, NamedKey};
    if alt
        || app.state.playlist.warp.is_none()
        || app.state.z.last() != Some(&crate::app::Win::Playlist)
        || app.state.playlist.popup.is_some()
    {
        return false;
    }
    let action = match key {
        Key::Named(NamedKey::Escape) => Some(Command::Close),
        Key::Named(NamedKey::Delete | NamedKey::Backspace) if !ctrl => Some(Command::Delete),
        Key::Character(c) if ctrl && c.eq_ignore_ascii_case("z") => Some(if app.shift {
            Command::Redo
        } else {
            Command::Undo
        }),
        Key::Character(c) if ctrl && c.eq_ignore_ascii_case("y") => Some(Command::Redo),
        _ => None,
    };
    if let Some(action) = action {
        command(app, action);
        true
    } else {
        false
    }
}

pub fn head(ui: &mut Ui, s: &State, row: u16, r: Rect) {
    if !crate::workspace_visibility::shown("visible_playlist-warp") {
        return;
    }
    let on = expanded(s, row);
    let hot = ui.hit(
        r,
        Action::Warp(Command::Toggle(row)),
        "Audio timing: detect transients and warp this track",
    );
    let color = if on || hot {
        ui.t.accent
    } else {
        ui.t.text_dim
    };
    if ui.t.style.vintage_2005 {
        crate::vintage::metal(ui.c, r, ui.t.button);
    }
    // A movable timing pin with outward arrows.
    let x = r.cx();
    let y = r.cy();
    ui.c.line(x, r.y + 2., x, r.bottom() - 2., 1.5, color);
    ui.c.fill_polygon(
        &[(x - 3., r.y + 2.), (x + 3., r.y + 2.), (x, r.y + 5.)],
        color,
    );
    ui.c.line(r.x + 2., y, r.right() - 2., y, 1., color);
    for (tip, dir) in [(r.x + 1., 1.), (r.right() - 1., -1.)] {
        ui.c.line(tip, y, tip + dir * 3., y - 3., 1., color);
        ui.c.line(tip, y, tip + dir * 3., y + 3., 1., color);
    }
}
pub fn inline(ui: &mut Ui, s: &State, g: &pl::PGeom) {
    if !crate::workspace_visibility::shown("visible_playlist-warp") {
        return;
    }
    if let Some(load) = s.playlist.warp_loading.as_ref() {
        let y = g.y_of(load.original.track) + g.rows.base_height(load.original.track);
        let r = Rect::new(g.grid.x, y, g.grid.w, EXTRA).intersect(&g.grid);
        ui.c.fill_rect(r, ui.t.window_title);
        ui.c.text_clip(
            Font::new(13),
            r.x + 12.,
            r.y + 12.,
            r.w - 24.,
            ui.t.text,
            "Loading audio for transient detection...",
        );
        return;
    }
    let Some(f) = s.playlist.warp.as_ref() else {
        return;
    };
    let y = g.y_of(f.original.track) + g.rows.base_height(f.original.track);
    let r = Rect::new(g.grid.x, y, g.grid.w, EXTRA);
    ui.c.push_clip(r.intersect(&g.grid));
    ui.c.fill_rect(r, ui.t.window_title);
    let mut x = r.x + 8.;
    let mut top = r.y + 7.;
    let buttons = [
        ("Detect".to_string(), Command::Analyze),
        ("Next clip".into(), Command::Next),
        ("Fit".into(), Command::Fit),
        (format!("Grid: {}", GRID_NAMES[f.grid]), Command::Grid),
        (format!("Strength: {}%", f.strength * 25), Command::Strength),
        (
            if f.select_mode {
                "Select: on".into()
            } else {
                "Select".into()
            },
            Command::Select,
        ),
        (
            if f.select_mode {
                format!("Quantize ({})", f.selection.len())
            } else {
                "Quantize".into()
            },
            Command::Quantize,
        ),
        (
            if f.snap {
                "Snap on".into()
            } else {
                "Snap off".into()
            },
            Command::Snap,
        ),
        ("Remove".into(), Command::Delete),
        ("Reset".into(), Command::Reset),
        ("Undo".into(), Command::Undo),
        ("Redo".into(), Command::Redo),
        ("Apply".into(), Command::Apply),
        ("Close".into(), Command::Close),
    ];
    for (label, cmd) in buttons {
        let width = ui.c.text_width(Font::new(11), &label) + 18.;
        if x + width > r.right() - 8. {
            x = r.x + 8.;
            top += 28.;
        }
        let b = Rect::new(x, top, width, 23.);
        x += width + 4.;
        let hot = if f.pending.is_none() || cmd == Command::Close {
            ui.hit(b, Action::Warp(cmd), label.clone())
        } else {
            false
        };
        let face = if cmd == Command::Apply {
            ui.t.accent
        } else if hot {
            ui.t.button_hi
        } else {
            ui.t.button
        };
        if ui.t.style.vintage_2005 {
            crate::vintage::metal(ui.c, b, face);
        } else {
            ui.c.fill_rrect(b, 3., face);
        }
        ui.c.text_center(Font::new(11), b, ui.t.text, &label);
    }
    let wave = Rect::new(
        g.x_of(f.original.position as f32),
        top + 32.,
        (g.x_of(f.original.position as f32 + f.end() as f32 * s.project.ppq as f32)
            - g.x_of(f.original.position as f32))
        .max(1.),
        (r.bottom() - top - 62.).max(30.),
    );
    f.wave.set(wave);
    let visible = wave.intersect(&Rect::new(g.grid.x, wave.y, g.grid.w, wave.h));
    ui.c.fill_rect(visible, Color::rgb(0x172025));
    ui.hit(
        visible,
        Action::Warp(Command::Source),
        if f.select_mode {"Drag to select points; Shift-drag adds. Ctrl+wheel: time zoom; Alt+wheel: waveform zoom"} else {"Click to add a marker; drag handle. Ctrl+wheel: time zoom; Alt+wheel: waveform zoom"},
    );
    ui.c.push_clip(visible);
    let rate = f.sample.rate as f64;
    for col in 0..visible.w.ceil() as usize {
        let x = visible.x + col as f32;
        let dst = (x - wave.x) as f64 / wave.w as f64 * f.end();
        let src = interpolate(&f.edit.points, dst, true);
        let next = interpolate(&f.edit.points, dst + f.end() / wave.w as f64, true);
        let lo = (src * rate).max(0.) as usize;
        let hi = ((next * rate) as usize)
            .max(lo.saturating_add(1))
            .min(f.sample.frames());
        let peak = f
            .sample
            .left
            .get(lo..hi)
            .unwrap_or(&[])
            .iter()
            .chain(f.sample.right.get(lo..hi).unwrap_or(&[]).iter())
            .fold(0f32, |p, v| p.max(v.abs()))
            .min(1.);
        ui.c.line(
            x,
            wave.cy() - (peak * f.amplitude).min(1.) * wave.h * 0.42,
            x,
            wave.cy() + (peak * f.amplitude).min(1.) * wave.h * 0.42,
            1.,
            Color::rgb(0x91b4bd),
        );
    }
    for &src in &f.transients {
        let x = wave.x + (interpolate(&f.edit.points, src, false) / f.end()) as f32 * wave.w;
        ui.c.line(
            x,
            wave.y,
            x,
            wave.bottom(),
            1.,
            if f.selection.contains(&src) {
                Color::rgb(0x70dfff)
            } else {
                Color::rgb(0x79949b).with_alpha(120)
            },
        );
    }
    for (i, &(src, beat)) in f.edit.points.iter().enumerate() {
        let x = wave.x + (beat / f.end()) as f32 * wave.w;
        let color = if f.selection.contains(&src) {
            Color::rgb(0x70dfff)
        } else if f.selected == Some(i) {
            Color::rgb(0xffdd70)
        } else {
            Color::rgb(0xdfac50)
        };
        ui.c.line(x, wave.y, x, wave.bottom(), 2., color);
        let handle = Rect::new(x - 5., wave.y, 10., 15.);
        ui.c.fill_polygon(
            &[(x - 5., wave.y), (x + 5., wave.y), (x, wave.y + 10.)],
            color,
        );
        ui.hit(
            handle.intersect(&visible),
            Action::Warp(Command::Marker(i)),
            "Drag timing marker; Remove deletes the selected marker",
        );
    }
    ui.c.pop_clip();
    ui.c.text_clip(
        Font::new(11),
        r.x + 8.,
        r.bottom() - 22.,
        r.w - 16.,
        ui.t.text,
        &f.message,
    );
    ui.c.pop_clip();
}

#[cfg(test)]
mod tests {
    use super::*;
    fn wait_loaded(app: &mut App) {
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
        while app.state.playlist.warp_loading.is_some() && std::time::Instant::now() < deadline {
            poll(app);
            std::thread::sleep(std::time::Duration::from_millis(2));
        }
        assert!(app.state.playlist.warp.is_some(), "{:?}", app.state.message);
    }

    #[test]
    fn inline_warp_transients_quantize_cancel_and_visibility() {
        let dir = std::env::temp_dir().join(format!("lemon-inline-warp-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let source = dir.join("attacks.wav");
        let rate = 16000u32;
        let mut audio = vec![0f32; rate as usize * 4 * 2];
        let attacks = [0.13, 0.61, 1.12, 1.64, 2.13, 2.62, 3.14, 3.62];
        for time in attacks {
            let start = (time * rate as f64) as usize;
            for i in 0..1600 {
                let value = 0.7 * (-(i as f32) / 350.).exp() * (i as f32 * 0.43).cos();
                audio[(start + i) * 2] = value;
                audio[(start + i) * 2 + 1] = -value;
            }
        }
        os_engine::wav::write_f32_stereo(
            &mut std::fs::File::create(&source).unwrap(),
            rate,
            &audio,
        )
        .unwrap();
        let mut app = App::new(None);
        app.state.project = crate::app::test_project();
        app.state.tempo = 120.;
        let ppq = app.state.project.ppq;
        let ch = &mut app.state.project.channels[0];
        ch.sample_path = Some(source.to_string_lossy().into_owned());
        let id = ch.index;
        pl::arr_mut(&mut app.state).clips = vec![os_model::Clip::new(
            ppq as u32 / 8,
            ppq as u32 * 8,
            0,
            ClipSource::Channel(id),
        )];
        let before = app.state.project.clone();
        command(&mut app, Command::Toggle(0));
        wait_loaded(&mut app);
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        while app.state.playlist.warp.as_ref().unwrap().analysis.is_some()
            && std::time::Instant::now() < deadline
        {
            poll(&mut app);
            std::thread::sleep(std::time::Duration::from_millis(2));
        }
        assert_eq!(
            app.state.playlist.warp.as_ref().unwrap().transients.len(),
            attacks.len()
        );
        command(&mut app, Command::Quantize);
        let points = app
            .state
            .playlist
            .warp
            .as_ref()
            .unwrap()
            .edit
            .points
            .clone();
        assert!(points.len() > 4);
        for &(_, beat) in &points[1..points.len() - 1] {
            assert!(((beat + 0.125) * 4. - ((beat + 0.125) * 4.).round()).abs() < 0.00001);
        }
        command(&mut app, Command::Undo);
        assert_eq!(
            app.state.playlist.warp.as_ref().unwrap().edit.points.len(),
            2
        );
        command(&mut app, Command::Redo);
        assert_eq!(
            app.state.playlist.warp.as_ref().unwrap().edit.points,
            points
        );
        // Selection-only quantize must leave every unselected anchor unchanged.
        command(&mut app, Command::Select);
        {
            let f = app.state.playlist.warp.as_mut().unwrap();
            f.edit.points[2].1 += 0.04;
            f.selection = vec![f.edit.points[2].0];
        }
        let selected_before = app
            .state
            .playlist
            .warp
            .as_ref()
            .unwrap()
            .edit
            .points
            .clone();
        command(&mut app, Command::Quantize);
        let selected_after = &app.state.playlist.warp.as_ref().unwrap().edit.points;
        assert_ne!(selected_before[2], selected_after[2]);
        for i in 0..selected_before.len() {
            if i != 2 {
                assert_eq!(selected_before[i], selected_after[i]);
            }
        }
        command(&mut app, Command::Undo);
        assert_eq!(
            app.state.playlist.warp.as_ref().unwrap().edit.points,
            selected_before
        );
        {
            let f = app.state.playlist.warp.as_mut().unwrap();
            f.wave.set(Rect::new(0., 0., 800., 100.));
            f.selection_start = 0.;
            f.selection_base.clear();
            f.select_to(400.);
            assert!(f.selection.len() > 2);
            f.edit.points = points.clone();
        }
        command(&mut app, Command::Select);
        for vintage in [false, true] {
            app.state.vintage_2005 = vintage;
            app.state.z = vec![crate::app::Win::Playlist];
            let mut canvas = os_ui::Canvas::new(1280, 800, 1.).unwrap();
            app.paint(&mut canvas, None);
            assert!(app
                .hits
                .iter()
                .any(|h| h.action == Action::Warp(Command::Toggle(0))));
            assert!(app
                .hits
                .iter()
                .any(|h| h.action == Action::Warp(Command::Quantize)));
            let wave_hit = app
                .hits
                .iter()
                .find(|h| h.action == Action::Warp(Command::Source))
                .unwrap()
                .rect;
            app.mouse = (wave_hit.x + wave_hit.w * 0.5, wave_hit.cy());
            app.alt = true;
            assert!(wheel(&mut app, 0., 1., false));
            assert!(app.state.playlist.warp.as_ref().unwrap().amplitude > 1.);
            app.alt = false;
            let old_zoom = app.state.playlist_bar_w;
            assert!(wheel(&mut app, 0., 1., true));
            assert!(app.state.playlist_bar_w > old_zoom);
            if let Some(dir) = std::env::var_os("OS_SHOT_DIR") {
                canvas
                    .pixmap
                    .save_png(PathBuf::from(dir).join(format!("inline-warp-{vintage}.png")))
                    .unwrap();
            }
        }
        crate::settings_file::write("visible_playlist-warp", "0");
        assert!(!expanded(&app.state, 0));
        assert_eq!(
            app.state.playlist.warp.as_ref().unwrap().edit.points,
            points
        );
        crate::settings_file::write("visible_playlist-warp", "1");
        assert!(expanded(&app.state, 0));
        command(&mut app, Command::Close);
        assert_eq!(app.state.project, before);
        assert!(app.state.playlist.warp.is_none());
    }
    #[test]
    fn community_warp_roundtrip_undo_and_render() {
        let dir = std::env::temp_dir().join(format!("lemon-warp-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let source = dir.join("source.wav");
        let rate = 8000u32;
        let audio: Vec<f32> = (0..16000)
            .flat_map(|i| {
                let x = (i as f32 * 440. * std::f32::consts::TAU / rate as f32).sin() * 0.2;
                [x, x]
            })
            .collect();
        os_engine::wav::write_f32_stereo(
            &mut std::fs::File::create(&source).unwrap(),
            rate,
            &audio,
        )
        .unwrap();
        let original_bytes = std::fs::read(&source).unwrap();
        let mut app = App::new(None);
        app.state.project = crate::app::test_project();
        app.state
            .project
            .arrangements
            .push(os_flp::project::default_arrangement());
        app.state.tempo = 120.;
        app.state.project.data_folder = Some(os_model::DataFolder {
            choice: 2,
            folder: dir.to_string_lossy().into_owned(),
        });
        let ch = &mut app.state.project.channels[0];
        ch.sample_path = Some(source.to_string_lossy().into_owned());
        let id = ch.index;
        let ppq = app.state.project.ppq;
        pl::arr_mut(&mut app.state).clips.push(os_model::Clip::new(
            0,
            ppq as u32 * 4,
            0,
            ClipSource::Channel(id),
        ));
        let before = app.state.project.clone();
        open(&mut app, 0).unwrap();
        wait_loaded(&mut app);
        let Some(f) = app.state.playlist.warp.as_mut() else {
            panic!()
        };
        f.edit.points.insert(1, (1., 1.5));
        f.wave.set(Rect::new(0., 0., 400., 100.));
        f.moved(1, 200.);
        assert_eq!(f.edit.points[1].1, 2.);
        f.edit.points[1].1 = 1.5;
        if let Some(dir) = std::env::var_os("OS_SHOT_DIR") {
            let mut c = os_ui::Canvas::new(1280, 900, 1.).unwrap();
            app.paint(&mut c, None);
            c.pixmap
                .save_png(std::path::PathBuf::from(dir).join("warp-editor.png"))
                .unwrap();
        }
        command(&mut app, Command::Apply);
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
        while app.state.playlist.warp.is_some() && std::time::Instant::now() < deadline {
            poll(&mut app);
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        assert!(app.state.playlist.warp.is_none());
        assert_eq!(std::fs::read(&source).unwrap(), original_bytes);
        let after = app.state.project.clone();
        let warped = &after.audio_warps[0];
        let rendered =
            os_engine::sample::load_shared(std::path::Path::new(&warped.rendered)).unwrap();
        assert_eq!(rendered.frames(), 16000);
        let file = os_flp::project::create(&after).unwrap();
        let loaded = os_flp::project::load(&file).unwrap();
        assert_eq!(loaded.audio_warps, after.audio_warps);
        crate::history::undo(&mut app.state);
        assert_eq!(app.state.project, before);
        crate::history::redo(&mut app.state);
        assert_eq!(app.state.project, after);
        open(&mut app, 0).unwrap();
        wait_loaded(&mut app);
        let Some(f) = app.state.playlist.warp.as_ref() else {
            panic!()
        };
        assert_eq!(f.edit.points, warped.points);
    }
}
