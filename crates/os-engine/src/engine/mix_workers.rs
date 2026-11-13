//! Persistent workers shared by instrument rendering and the mixer dependency graph.
//! Every job owns its mutable DSP state. Reply queues belong to one engine instance.
use super::{MixTrack, TransportInfo, finish_mix_audio, run_slot, send_mix_audio, strip_process};
use std::sync::{Arc, Mutex, OnceLock, mpsc};
use std::time::{Duration, Instant};

/// The host supplies the same platform priority setter it uses for the feeder.
/// Cache it per persistent worker, not globally: engines with different settings
/// may share the pool. An offline/default job restores Normal after a live job.
#[derive(Clone, Copy, Default)]
pub(super) struct Priority(Option<(fn(usize), usize)>);
impl Priority {
    pub fn new(level: usize, setter: fn(usize)) -> Self {
        Self(Some((setter, level)))
    }
    fn apply(self) {
        thread_local! { static LAST: std::cell::Cell<Priority> = const { std::cell::Cell::new(Priority(None)) }; }
        LAST.with(|last| {
            let same = match (last.get().0, self.0) {
                (Some((a, x)), Some((b, y))) => std::ptr::fn_addr_eq(a, b) && x == y,
                (None, None) => true,
                _ => false,
            };
            if same {
                return;
            }
            match self.0 {
                Some((setter, level)) => setter(level),
                None => {
                    if let Some((setter, _)) = last.get().0 {
                        setter(0);
                    }
                }
            }
            last.set(self);
        });
    }
}

pub(super) struct State {
    pub enabled: bool,
    pub done: Vec<bool>,
    pub cost: Vec<Duration>,
    pub plan: super::mixer_graph::Plan,
    pub jobs: Vec<Option<Box<Job>>>,
    pub tx: mpsc::Sender<Box<Job>>,
    pub rx: mpsc::Receiver<Box<Job>>,
}
impl State {
    pub fn new() -> Self {
        let (tx, rx) = mpsc::channel();
        Self {
            enabled: enabled(),
            done: Vec::new(),
            cost: Vec::new(),
            plan: Default::default(),
            jobs: Vec::new(),
            tx,
            rx,
        }
    }
}

#[derive(Default)]
pub(super) struct Addition {
    pub dest: usize,
    pub samples: Vec<f32>,
}
fn append(additions: &mut Vec<Addition>, count: &mut usize, dest: usize, signal: &[f32]) {
    if *count == additions.len() {
        additions.push(Addition::default());
    }
    let add = &mut additions[*count];
    add.dest = dest;
    add.samples.clear();
    add.samples.extend_from_slice(signal);
    *count += 1;
}

pub(super) struct Job {
    pub track: usize,
    pub mix: MixTrack,
    pub buf: Vec<f32>,
    pub keys: Vec<(usize, Vec<f32>)>,
    pub additions: Vec<Addition>,
    pub addition_count: usize,
    pub rate: u32,
    pub song: TransportInfo,
    pub until: Option<Instant>,
    pub wait: Duration,
    pub export: bool,
    pub smart_disable: bool,
    pub priority: Priority,
    pub slots_enabled: bool,
    pub moved: bool,
    pub missed: u64,
    pub elapsed: Duration,
    dry: Vec<f32>,
    left: Vec<f32>,
    right: Vec<f32>,
    backup: Vec<f32>,
    key: Vec<f32>,
    main: Vec<f32>,
    padded: Vec<f32>,
}
impl Default for Job {
    fn default() -> Self {
        Self {
            track: 0,
            mix: MixTrack::unity(),
            buf: Vec::new(),
            keys: Vec::new(),
            additions: Vec::new(),
            addition_count: 0,
            rate: 48000,
            song: TransportInfo {
                playing: false,
                tempo: 120.0,
                frame: 0,
                beats: 0.0,
                bar_start: 0.0,
                bar: 0,
                numerator: 4,
                denominator: 4,
                looping: None,
            },
            until: None,
            wait: Duration::ZERO,
            export: false,
            smart_disable: false,
            priority: Priority::default(),
            slots_enabled: true,
            moved: false,
            missed: 0,
            elapsed: Duration::ZERO,
            dry: Vec::new(),
            left: Vec::new(),
            right: Vec::new(),
            backup: Vec::new(),
            key: Vec::new(),
            main: Vec::new(),
            padded: Vec::new(),
        }
    }
}
impl Job {
    pub fn run(&mut self) {
        let _audio_float = os_dsp::audio_float::AudioFloat::enter();
        let start = Instant::now();
        self.moved = false;
        self.missed = 0;
        self.addition_count = 0;
        self.backup.clear();
        self.backup.extend_from_slice(&self.buf);
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            if self.track != 0 {
                strip_process(&mut self.mix, &mut self.buf);
            }
            let limit = os_dsp::glide::engine_limit(self.rate);
            for fx in &mut self.mix.effects {
                if fx.slot != 10 && Some(fx.slot) != self.mix.strip_slot && !self.slots_enabled {
                    continue;
                }
                let feed = |id| {
                    self.keys
                        .iter()
                        .find(|(s, b)| Some(*s) == id && b.len() == self.buf.len())
                        .map(|(_, b)| b.as_slice())
                };
                let key = feed(fx.key).map(|k| (k, &mut self.key));
                let main = feed(fx.main).map(|k| (k, &mut self.main));
                let mut add = |dest, signal: &[f32]| {
                    append(&mut self.additions, &mut self.addition_count, dest, signal)
                };
                run_slot(
                    fx,
                    self.smart_disable && !self.export,
                    &mut self.buf,
                    self.rate,
                    limit,
                    &self.song,
                    &mut self.dry,
                    &mut self.left,
                    &mut self.right,
                    (&mut self.until, self.wait, self.export, &mut self.missed),
                    &mut self.moved,
                    (self.track, &self.mix.routes, &mut add),
                    (key, main),
                );
            }
        }));
        // Never strand the engine waiting for a panicking plugin. Return the owned
        // state and bypass this chain for the block; discard its partial aux output.
        if result.is_err() {
            self.buf.copy_from_slice(&self.backup);
            self.addition_count = 0;
            self.missed += 1;
        }
        let frames = self.buf.len() / 2;
        finish_mix_audio(&mut self.mix, &mut self.buf, frames, self.rate);
        let mut add = |dest, signal: &[f32]| {
            append(&mut self.additions, &mut self.addition_count, dest, signal)
        };
        send_mix_audio(
            &mut self.mix,
            &self.buf,
            frames,
            self.rate,
            &mut self.padded,
            &mut add,
        );
        self.elapsed = start.elapsed();
    }
}

enum Work {
    Mixer {
        job: Box<Job>,
        reply: mpsc::Sender<Box<Job>>,
    },
    Instrument {
        job: super::instrument_workers::Job,
        reply: mpsc::Sender<super::instrument_workers::Job>,
    },
}
impl Work {
    fn run(self) {
        let _audit = os_dsp::rt_audit::Scope::enter(os_dsp::rt_audit::Role::Worker);
        match self {
            Self::Mixer { mut job, reply } => {
                job.priority.apply();
                job.run();
                let _ = reply.send(job);
            }
            Self::Instrument { mut job, reply } => {
                job.priority.apply();
                job.run();
                let _ = reply.send(job);
            }
        }
    }
}
struct Pool {
    tx: mpsc::Sender<Work>,
    workers: usize,
}
fn pool() -> &'static Pool {
    static POOL: OnceLock<Pool> = OnceLock::new();
    POOL.get_or_init(|| {
        let (tx, rx) = mpsc::channel::<Work>();
        let rx = Arc::new(Mutex::new(rx));
        let wanted = std::thread::available_parallelism()
            .map_or(1, usize::from)
            .saturating_sub(1)
            .min(12);
        let mut workers = 0;
        for i in 0..wanted {
            let rx = rx.clone();
            if std::thread::Builder::new()
                .name(format!("Lemon mixer {}", i + 1))
                .spawn(move || {
                    loop {
                        let work = rx.lock().unwrap_or_else(|e| e.into_inner()).recv();
                        let Ok(work) = work else { break };
                        work.run();
                    }
                })
                .is_ok()
            {
                workers += 1;
            }
        }
        Pool { tx, workers }
    })
}
pub(super) fn worker_count() -> usize {
    pool().workers
}
pub(super) fn available() -> bool {
    pool().workers >= 2
}
pub(super) fn enabled() -> bool {
    std::env::var_os("LEMON_AUDIO_SINGLE_CORE").is_none() && available()
}
pub(super) fn submit(job: Box<Job>, reply: mpsc::Sender<Box<Job>>) {
    if let Err(e) = pool().tx.send(Work::Mixer { job, reply }) {
        e.0.run();
    }
}

pub(super) fn submit_instrument(
    job: super::instrument_workers::Job,
    reply: mpsc::Sender<super::instrument_workers::Job>,
) {
    if let Err(error) = pool().tx.send(Work::Instrument { job, reply }) {
        error.0.run();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    thread_local! { static OBSERVED: std::cell::Cell<(usize, usize)> = const { std::cell::Cell::new((0, 0)) }; }
    fn record(level: usize) {
        OBSERVED.with(|v| v.set((level, v.get().1 + 1)));
    }

    #[test]
    fn worker_priority_follows_each_engine_and_resets_for_offline_work() {
        std::thread::spawn(|| {
            let (reply, results) = mpsc::channel();
            for (priority, expected) in [
                (Priority::new(2, record), (2, 1)),
                (Priority::new(2, record), (2, 1)),
                (Priority::new(1, record), (1, 2)),
                (Priority::default(), (0, 3)),
            ] {
                let job = Box::new(Job {
                    priority,
                    ..Default::default()
                });
                Work::Mixer {
                    job,
                    reply: reply.clone(),
                }
                .run();
                results.recv().unwrap();
                assert_eq!(OBSERVED.with(|v| v.get()), expected);
            }
        })
        .join()
        .unwrap();
    }
}
