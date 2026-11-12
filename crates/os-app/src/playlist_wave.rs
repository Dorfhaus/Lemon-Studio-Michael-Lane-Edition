// SPDX-License-Identifier: GPL-3.0-or-later
//! One background analysis job per Playlist, with a bounded, deduplicated request queue.
use super::*;
use std::{
    collections::VecDeque,
    sync::{Arc, mpsc},
};

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Kind {
    Peaks,
    Tones,
    Fine,
}
enum Data {
    Peaks(Wave),
    Tones(os_dsp::tone::Tones),
    Fine(Arc<os_engine::Sample>),
}
struct Job {
    path: String,
    kind: Kind,
    rx: mpsc::Receiver<(bool, Option<Data>)>,
}
#[derive(Default)]
pub(super) struct Jobs {
    running: Option<Job>,
    queue: VecDeque<(String, Kind)>,
    failed_fine: std::collections::HashSet<String>,
}
impl Jobs {
    pub(super) fn request(&mut self, path: &str, kind: Kind) {
        if kind == Kind::Fine && self.failed_fine.contains(path) {
            return;
        }
        if self
            .running
            .as_ref()
            .is_some_and(|j| j.path == path && j.kind == kind)
            || self.queue.iter().any(|(p, k)| p == path && *k == kind)
            || self.queue.len() >= 64
        {
            return;
        }
        self.queue.push_back((path.to_owned(), kind));
        self.start();
    }
    fn start(&mut self) {
        if self.running.is_some() {
            return;
        }
        let Some((path, kind)) = self.queue.pop_front() else {
            return;
        };
        let resolved = crate::plugins::resolve_sample(&path);
        let (tx, rx) = mpsc::channel();
        self.running = Some(Job { path, kind, rx });
        std::thread::spawn(move || {
            let stamp = || {
                std::fs::metadata(&resolved)
                    .ok()
                    .map(|m| (m.len(), m.modified().ok()))
            };
            let before = stamp();
            let data = os_engine::sample::load_shared(std::path::Path::new(&resolved))
                .ok()
                .map(|sample| match kind {
                    Kind::Peaks => Data::Peaks(peaks(&sample)),
                    Kind::Tones => Data::Tones(os_dsp::tone::Tones::analyse_stereo(
                        &sample.left,
                        &sample.right,
                        sample.rate,
                    )),
                    Kind::Fine => Data::Fine(sample),
                });
            let _ = tx.send((before == stamp(), data));
        });
    }
}
pub(super) fn peaks(sample: &os_engine::Sample) -> Wave {
    let per = 64;
    let peaks = sample
        .left
        .chunks(per)
        .zip(sample.right.chunks(per))
        .map(|(l, r)| {
            l.iter()
                .zip(r)
                .map(|(a, b)| (a + b) * 0.5)
                .fold((f32::MAX, f32::MIN), |(lo, hi), v| (lo.min(v), hi.max(v)))
        })
        .collect();
    Wave {
        fine: RefCell::new(None),
        rate: sample.rate,
        frames: sample.frames(),
        per,
        peaks,
    }
}
impl PlaylistState {
    pub fn request_wave(&self, path: &str) -> Option<Rc<Wave>> {
        if let Some(w) = self.waves.borrow().get(path) {
            return w.clone();
        }
        if !self.wave_wait {
            self.wave_jobs.borrow_mut().request(path, Kind::Peaks);
        }
        None
    }
    pub fn request_fine(&self, path: &str) {
        self.wave_jobs.borrow_mut().request(path, Kind::Fine);
    }
    pub fn poll_waves(&self) -> bool {
        let mut jobs = self.wave_jobs.borrow_mut();
        let result = jobs.running.as_ref().and_then(|j| match j.rx.try_recv() {
            Ok(r) => Some(r),
            Err(mpsc::TryRecvError::Disconnected) => Some((true, None)),
            Err(mpsc::TryRecvError::Empty) => None,
        });
        let Some((stable, data)) = result else {
            return false;
        };
        let job = jobs.running.take().unwrap();
        if stable {
            match data {
                Some(Data::Peaks(w)) => {
                    self.waves.borrow_mut().insert(job.path, Some(Rc::new(w)));
                }
                Some(Data::Tones(t)) => {
                    self.tones.borrow_mut().insert(job.path, Some(Rc::new(t)));
                }
                Some(Data::Fine(s)) => {
                    if let Some(Some(w)) = self.waves.borrow().get(&job.path) {
                        *w.fine.borrow_mut() = Some(s);
                    }
                }
                None => match job.kind {
                    Kind::Peaks => {
                        self.waves.borrow_mut().insert(job.path, None);
                    }
                    Kind::Tones => {
                        self.tones.borrow_mut().insert(job.path, None);
                    }
                    Kind::Fine => {
                        jobs.failed_fine.insert(job.path);
                    }
                },
            }
        }
        jobs.start();
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn drawing_queues_analysis_and_publishes_matching_peaks() {
        let path = std::env::temp_dir().join(format!("playlist-worker-{}.wav", std::process::id()));
        let sample = os_engine::Sample {
            rate: 24000,
            left: vec![0.25; 48000],
            right: vec![0.5; 48000],
        };
        crate::vocal::incremental::write_audio(&path, &sample).unwrap();
        let playlist = PlaylistState::default();
        let path_text = path.to_str().unwrap();
        assert!(playlist.request_wave(path_text).is_none());
        for _ in 0..100 {
            assert!(playlist.request_wave(path_text).is_none());
        }
        assert!(playlist.wave_jobs.borrow().queue.is_empty());
        let until = std::time::Instant::now() + std::time::Duration::from_secs(10);
        while !playlist.poll_waves() {
            assert!(std::time::Instant::now() < until);
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        let wave = playlist.request_wave(path_text).unwrap();
        assert_eq!(wave.peaks, peaks(&sample).peaks);
        assert_eq!(wave.frames, sample.frames());
        let _ = std::fs::remove_file(path);
    }
}
