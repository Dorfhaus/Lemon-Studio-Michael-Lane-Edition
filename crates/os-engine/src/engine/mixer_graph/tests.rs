use super::*;
use crate::engine::{FxSlot, pdc};
use crate::{HostedEffect, PlayMode, SampleLibrary};
use os_model::{MixerTrack, Project};
use std::sync::{
    Mutex,
    atomic::{AtomicBool, AtomicUsize, Ordering},
};

struct Effect {
    gain: f32,
    pad: pdc::Pad,
    calls: usize,
}
impl HostedEffect for Effect {
    fn process_fx(&mut self, _: u32, _: &mut [f32], _: &mut [f32]) {
        unreachable!();
    }
    fn process_fx_keyed(&mut self, _: u32, l: &mut [f32], r: &mut [f32], key: (&[f32], &[f32])) {
        self.calls += 1;
        let mut stereo: Vec<_> = l
            .iter()
            .zip(&*r)
            .zip(key.0.iter().zip(key.1))
            .flat_map(|((&l, &r), (&kl, &kr))| {
                [l * self.gain + kl * 0.03, r * self.gain - kr * 0.02]
            })
            .collect();
        self.pad.run(&mut stereo);
        for ((l, r), pair) in l.iter_mut().zip(r).zip(stereo.chunks_exact(2)) {
            *l = pair[0];
            *r = pair[1];
        }
    }
    fn reported_delay_fx(&self) -> u32 {
        self.pad.len() as u32
    }
    fn param_fx(&mut self, _: u32, _: u32, value: f64) {
        self.gain = value as f32;
    }
}

struct Aux {
    samples: Vec<f32>,
    gain: f32,
    target: usize,
}
impl crate::effects::Effect for Aux {
    fn process(&mut self, buf: &mut [f32]) {
        self.samples.clear();
        self.samples.extend(buf.iter().map(|s| s * self.gain));
        for s in buf {
            *s *= 0.7;
        }
    }
    fn extra(&self) -> Option<(usize, &[f32])> {
        Some((self.target, &self.samples))
    }
}

fn project(count: u16) -> Project {
    Project {
        mixer: (0..count).map(MixerTrack::new).collect(),
        ..Project::default()
    }
}
fn engine(p: &Project, parallel: bool) -> Engine {
    let mut e = Engine::new(p, &mut SampleLibrary::default(), 48000, PlayMode::Song);
    e.set_processing_threads(false, parallel);
    e.set_worker_limit(4);
    e
}
fn force_cost(e: &mut Engine) {
    e.parallel.cost = vec![Duration::from_micros(500); e.tracks.len()];
}
fn fill(e: &mut Engine, frames: usize, block: usize) {
    for (t, b) in e.bus.iter_mut().enumerate() {
        b.resize(frames * 2, 0.);
        for (i, s) in b.iter_mut().enumerate() {
            *s = ((t * 17 + i * 7 + block * 31) % 103) as f32 * 0.0007 - 0.03;
        }
    }
}
fn mix(e: &mut Engine, frames: usize, block: usize) {
    force_cost(e);
    fill(e, frames, block);
    e.mix(frames, &e.transport_at((frames * block) as u64));
}
fn assert_same(a: &Engine, b: &Engine) {
    assert_eq!(a.bus, b.bus, "every track must be sample-identical");
    assert_eq!(a.key_out, b.key_out);
    assert_eq!(a.capture, b.capture);
    assert_eq!(a.pdc.cap_skip, b.pdc.cap_skip);
    assert_eq!(a.pdc.due, b.pdc.due);
    for (a, b) in a.tracks.iter().zip(&b.tracks) {
        assert_eq!(a.peak, b.peak);
    }
}

#[test]
fn smart_disable_reaches_serial_and_graph_effects_but_never_exports() {
    struct Observe { enabled: bool, seen: Arc<Mutex<Vec<bool>>> }
    impl HostedEffect for Observe {
        fn set_smart_disable_fx(&mut self, enabled: bool) { self.enabled = enabled; }
        fn process_fx(&mut self, _: u32, _: &mut [f32], _: &mut [f32]) {
            self.seen.lock().unwrap().push(self.enabled);
        }
    }
    for parallel in [false, true] {
        let p = project(5); let mut e = engine(&p, parallel);
        let seen = Arc::new(Mutex::new(Vec::new()));
        for t in 1..5 {
            e.set_effect(t, 0, Arc::new(Mutex::new(Observe { enabled: false, seen: seen.clone() })));
        }
        for (block, (enabled, export)) in [(false, false), (true, false), (true, true), (false, false), (true, false)].into_iter().enumerate() {
            e.set_smart_disable(enabled); e.exporting = export;
            seen.lock().unwrap().clear(); mix(&mut e, 64, block);
            assert_eq!(*seen.lock().unwrap(), vec![enabled && !export; 4]);
            if parallel && mix_workers::available() {
                assert!(e.parallel.done.iter().any(|done| *done), "graph really dispatched");
            }
        }
    }
}

#[test]
fn graph_preserves_diamonds_keys_aux_pdc_automation_and_capture() {
    let mut p = project(17);
    // Two trees with overlapping sends, parallel buses and a final join. Routes
    // deliberately differ from numerical order; zero-level sends still carry keys.
    for t in 5..17 {
        p.mixer[t].routes = vec![1 + (t % 4) as u16, 0];
    }
    p.mixer[4].routes = vec![2, 1];
    p.mixer[3].routes = vec![1, 0];
    p.mixer[2].routes = vec![1];
    p.mixer[5].send_levels = vec![(2, 0.0), (0, 0.17)];
    let make = |parallel| {
        let mut e = engine(&p, parallel);
        for t in 0..17 {
            let mut pad = pdc::Pad::default();
            pad.set_len(t * 3);
            e.set_effect(
                t,
                0,
                Arc::new(Mutex::new(Effect {
                    gain: 0.8,
                    pad,
                    calls: 0,
                })),
            );
            e.set_effect_ports(t, 0, if t == 2 { 1 } else { 0 }, if t < 5 { 1 } else { 0 });
            // Two separate aux additions must not be collapsed before summation.
            for (slot, gain) in [(1, 2000.), (2, -2800.)] {
                e.tracks[t].effects.push(FxSlot::new(
                    slot,
                    true,
                    12800,
                    Fx::Own(Box::new(Aux {
                        samples: Vec::new(),
                        gain,
                        target: 1 + t % 2,
                    })),
                ));
            }
        }
        e.capture = Some(vec![Vec::new(); 17]);
        e.plan_delays();
        e
    };
    let (mut a, mut b) = (make(false), make(true));
    for block in 0..48 {
        let frames = [1, 31, 64, 128, 240, 256, 512][block % 7];
        for e in [&mut a, &mut b] {
            e.exporting = block >= 24;
            e.master_fx = block % 4 != 0;
            e.insert_fx = block % 5 != 0;
            e.tracks[3].slots_enabled = block % 3 != 0;
            e.tracks[4].set_slot_on(0, block % 2 != 0);
            e.tracks[6].set_slot_mix(0, (block % 5) as i32 * 3200);
            e.tracks[5].routes[0].gain = (block % 3) as f32 * 0.15;
            e.tracks[2].vol = 6000 + block as i32 * 100;
            e.tracks[2].matrix = e.tracks[2].compute_matrix();
            e.params.events = vec![super::super::param_auto::ParamEvent {
                dest: super::super::param_auto::Dest::Slot(2, 0),
                offset: 0,
                index: 0,
                value: 0.3 + block as f64 * 0.01,
            }];
            if block == 12 {
                e.set_effect_ports(2, 0, 0, 2);
            }
            if block % 6 == 0 {
                e.plan_delays();
            }
            mix(e, frames, block);
        }
        assert_same(&a, &b);
        if mix_workers::enabled() {
            assert!(b.parallel.done.iter().all(|done| *done));
        }
    }
    assert!(b.parallel.jobs.iter().all(Option::is_some));
}

#[test]
fn graph_matches_random_dags_and_reuses_the_plan_and_buffers() {
    for seed in 1..9 {
        let mut p = project(32);
        let mut rand = seed as u32;
        for t in 1..32 {
            p.mixer[t].routes.clear();
            for d in 0..t {
                rand = rand.wrapping_mul(1664525).wrapping_add(1013904223);
                if rand % 5 == 0 {
                    p.mixer[t].routes.push(d as u16);
                }
            }
        }
        let (mut a, mut b) = (engine(&p, false), engine(&p, true));
        for block in 0..8 {
            mix(&mut a, 128, block);
            mix(&mut b, 128, block);
            assert_same(&a, &b);
        }
        if mix_workers::enabled() {
            assert_eq!(b.parallel.plan.rebuilds, 1);
            let capacity = |e: &Engine| {
                e.parallel
                    .jobs
                    .iter()
                    .flatten()
                    .map(|j| {
                        (
                            j.buf.capacity(),
                            j.additions.capacity(),
                            j.additions
                                .iter()
                                .map(|a| a.samples.capacity())
                                .collect::<Vec<_>>(),
                        )
                    })
                    .collect::<Vec<_>>()
            };
            let before = capacity(&b);
            for block in 8..24 {
                mix(&mut b, 128, block);
            }
            assert_eq!(
                before,
                capacity(&b),
                "steady topology and block size reuse storage"
            );
        }
    }
}

#[test]
fn graph_updates_routes_and_ports_and_falls_back_for_feedback() {
    let mut p = project(8);
    let (mut a, mut b) = (engine(&p, false), engine(&p, true));
    for block in 0..6 {
        match block {
            1 => p.mixer[6].routes = vec![5],
            2 => p.mixer[5].routes = vec![6], // feedback, retain existing order
            3 => p.mixer[5].routes = vec![4], // valid again
            4 => p.mixer[6].routes = vec![0, 5],
            _ => (),
        }
        for e in [&mut a, &mut b] {
            e.set_mixer_sends(5, &p.mixer[5]);
            e.set_mixer_sends(6, &p.mixer[6]);
            mix(e, 64, block);
        }
        assert_same(&a, &b);
        if mix_workers::enabled() {
            assert_eq!(b.parallel.done.iter().any(|done| *done), block != 2);
        }
    }
}

#[test]
fn graph_orders_shared_instance_calls_and_parameter_delivery() {
    struct Shared {
        events: Arc<Mutex<Vec<f64>>>,
        value: f64,
        calls: usize,
    }
    impl HostedEffect for Shared {
        fn param_fx(&mut self, _: u32, _: u32, value: f64) {
            self.value = value;
        }
        fn process_fx(&mut self, _: u32, l: &mut [f32], r: &mut [f32]) {
            self.events.lock().unwrap().push(self.value);
            self.calls += 1;
            for s in l.iter_mut().chain(r) {
                *s *= self.value as f32 + self.calls as f32 * 0.01;
            }
        }
    }
    let p = project(6);
    let make = |parallel| {
        let events = Arc::new(Mutex::new(Vec::new()));
        let plugin = Arc::new(Mutex::new(Shared {
            events: events.clone(),
            value: 1.,
            calls: 0,
        }));
        let mut e = engine(&p, parallel);
        for t in [1, 2, 4] {
            e.set_effect(t, 0, plugin.clone());
        }
        (e, events)
    };
    let ((mut a, ae), (mut b, be)) = (make(false), make(true));
    for block in 0..12 {
        for e in [&mut a, &mut b] {
            e.params.events = [1, 2, 4]
                .into_iter()
                .map(|t| super::super::param_auto::ParamEvent {
                    dest: super::super::param_auto::Dest::Slot(t, 0),
                    offset: 0,
                    index: 0,
                    value: t as f64 * 0.1,
                })
                .collect();
            mix(e, 128, block);
        }
        assert_same(&a, &b);
        assert_eq!(*ae.lock().unwrap(), *be.lock().unwrap());
    }
}

#[test]
fn graph_releases_a_bus_before_an_unrelated_slow_branch_finishes() {
    if !mix_workers::enabled() {
        return;
    }
    let released = Arc::new(AtomicBool::new(false));
    struct Wait(Arc<AtomicBool>);
    impl HostedEffect for Wait {
        fn process_fx(&mut self, _: u32, _: &mut [f32], _: &mut [f32]) {
            let until = Instant::now() + Duration::from_secs(2);
            while !self.0.load(Ordering::SeqCst) && Instant::now() < until {
                std::thread::yield_now();
            }
            assert!(
                self.0.load(Ordering::SeqCst),
                "bus must run without a whole-batch barrier"
            );
        }
    }
    struct Release(Arc<AtomicBool>);
    impl HostedEffect for Release {
        fn process_fx(&mut self, _: u32, _: &mut [f32], _: &mut [f32]) {
            self.0.store(true, Ordering::SeqCst);
        }
    }
    let mut p = project(5);
    p.mixer[3].routes = vec![2];
    let mut e = engine(&p, true);
    e.set_worker_limit(2);
    let wait = Arc::new(Mutex::new(Wait(released.clone())));
    e.set_effect(4, 0, wait.clone());
    e.set_effect(2, 0, Arc::new(Mutex::new(Release(released))));
    mix(&mut e, 64, 0);
    assert!(!wait.is_poisoned());
}

#[test]
fn graph_respects_worker_caps_and_restores_owned_state_after_panic() {
    if !mix_workers::enabled() {
        return;
    }
    struct Busy {
        active: Arc<AtomicUsize>,
        peak: Arc<AtomicUsize>,
        fail: bool,
    }
    impl HostedEffect for Busy {
        fn process_fx(&mut self, _: u32, l: &mut [f32], _: &mut [f32]) {
            let n = self.active.fetch_add(1, Ordering::SeqCst) + 1;
            self.peak.fetch_max(n, Ordering::SeqCst);
            std::thread::sleep(Duration::from_millis(2));
            self.active.fetch_sub(1, Ordering::SeqCst);
            if self.fail {
                l.fill(f32::NAN);
                panic!("test plugin panic");
            }
        }
    }
    for cap in [1, 2, 4] {
        let active = Arc::new(AtomicUsize::new(0));
        let peak = Arc::new(AtomicUsize::new(0));
        let mut e = engine(&project(10), true);
        e.set_worker_limit(cap);
        for t in 1..10 {
            e.set_effect(
                t,
                0,
                Arc::new(Mutex::new(Busy {
                    active: active.clone(),
                    peak: peak.clone(),
                    fail: cap > 1 && t == 3,
                })),
            );
        }
        mix(&mut e, 64, 0);
        assert_eq!(
            peak.load(Ordering::SeqCst),
            cap.min(mix_workers::worker_count())
        );
        assert_eq!(e.tracks.iter().map(|t| t.effects.len()).sum::<usize>(), 9);
        assert!(e.bus.iter().flatten().all(|s| s.is_finite()));
        assert!(e.parallel.jobs.iter().all(Option::is_some));
        if cap > 1 {
            assert_eq!(e.missed, 1);
        }
    }
}

#[test]
fn graph_keeps_simultaneous_engine_results_isolated() {
    if !mix_workers::enabled() {
        return;
    }
    let expected: Vec<_> = (0..4)
        .map(|id| {
            let mut e = engine(&project(12), false);
            for block in 0..20 {
                mix(&mut e, 128, block + id);
            }
            std::mem::take(&mut e.bus)
        })
        .collect();
    let handles: Vec<_> = (0..4)
        .map(|id| {
            std::thread::spawn(move || {
                let mut e = engine(&project(12), true);
                for block in 0..20 {
                    mix(&mut e, 128, block + id);
                }
                std::mem::take(&mut e.bus)
            })
        })
        .collect();
    for (handle, output) in handles.into_iter().zip(expected) {
        assert_eq!(handle.join().unwrap(), output);
    }
}

#[test]
fn graph_avoids_overhead_for_a_single_heavy_chain() {
    let mut p = project(8);
    for t in 1..8 {
        p.mixer[t].routes = vec![(t - 1) as u16];
    }
    let mut e = engine(&p, true);
    mix(&mut e, 128, 0);
    assert!(e.parallel.done.iter().all(|done| !done));
    assert!(e.parallel.jobs.is_empty());
}

#[test]
fn graph_busy_plugin_returns_state_and_recovers_next_block() {
    if !mix_workers::enabled() {
        return;
    }
    let mut e = engine(&project(8), true);
    let plugin = Arc::new(Mutex::new(Effect {
        gain: 0.5,
        pad: Default::default(),
        calls: 0,
    }));
    e.set_effect(3, 0, plugin.clone());
    let guard = plugin.lock().unwrap();
    e.busy_wait = Duration::ZERO;
    mix(&mut e, 64, 0);
    assert_eq!(e.take_missed(), 1);
    assert_eq!(guard.calls, 0);
    assert_eq!(e.tracks[3].effects.len(), 1);
    drop(guard);
    mix(&mut e, 64, 1);
    assert_eq!(e.take_missed(), 0);
    assert_eq!(plugin.lock().unwrap().calls, 1);
}

#[test]
fn graph_playback_preserves_seek_rebuild_and_dynamic_latency() {
    struct Signal;
    impl crate::engine::MixerInput for Signal {
        fn render(&mut self, _: u32, tracks: &mut [Vec<f32>]) {
            for (t, b) in tracks.iter_mut().enumerate().skip(4) {
                b.fill(t as f32 * 0.03);
            }
        }
    }
    let mut p = project(8);
    for t in 4..8 {
        p.mixer[t].routes = vec![1 + (t % 3) as u16];
    }
    let make = |parallel| {
        let mut e = engine(&p, parallel);
        let mut handles = Vec::new();
        for t in 1..8 {
            let mut pad = pdc::Pad::default();
            pad.set_len(t * 7);
            let handle = Arc::new(Mutex::new(Effect {
                gain: 0.7,
                pad,
                calls: 0,
            }));
            e.set_effect(t, 0, handle.clone());
            e.set_effect_ports(t, 0, 0, 1);
            handles.push(handle);
        }
        e.clear_schedule();
        e.set_awake(true);
        e.set_mixer_input(Box::new(Signal));
        (e, handles)
    };
    let ((mut a, mut ah), (mut b, mut bh)) = (make(false), make(true));
    let mut used_graph = false;
    for block in 0..100 {
        if block == 30 {
            a.seek_tick(12);
            b.seek_tick(12);
        }
        if block == 60 {
            let ((mut na, nah), (mut nb, nbh)) = (make(false), make(true));
            na.continue_from(&mut a);
            nb.continue_from(&mut b);
            (a, b, ah, bh) = (na, nb, nah, nbh);
        }
        if block % 17 == 0 {
            ah[0].lock().unwrap().pad.set_len(13 + block);
            bh[0].lock().unwrap().pad.set_len(13 + block);
        }
        let frames = [31, 64, 128, 256, 441][block % 5];
        let mut sa = vec![0.; frames * 2];
        let mut sb = sa.clone();
        force_cost(&mut a);
        force_cost(&mut b);
        a.process(&mut sa);
        b.process(&mut sb);
        assert_eq!(sa, sb, "playback block {block}");
        assert_eq!(a.tick_position(), b.tick_position());
        assert_eq!(a.pdc.lag, b.pdc.lag);
        used_graph |= b.parallel.done.iter().any(|done| *done);
    }
    if mix_workers::enabled() {
        assert!(used_graph);
    }
}

/// No devices or microphone. Deliberately ignored in routine tests because timings
/// depend on the machine and other workloads. Uses CPU work, never sleeps.
#[test]
#[ignore]
fn benchmark_routed_mixer_scheduler() {
    struct Work(usize);
    impl HostedEffect for Work {
        fn process_fx(&mut self, _: u32, l: &mut [f32], r: &mut [f32]) {
            for s in l.iter_mut().chain(r) {
                let mut x = *s;
                for _ in 0..self.0 {
                    x = std::hint::black_box(x * 0.99991 + 0.00001);
                }
                *s = x;
            }
        }
    }
    for &(name, passes) in &[("light", 0), ("routed", 160)] {
        let mut p = project(19);
        for t in 11..19 {
            p.mixer[t].routes = vec![(t - 8) as u16];
        }
        for t in 3..11 {
            p.mixer[t].routes = vec![1 + (t % 2) as u16];
        }
        for frames in [64, 128, 256] {
            let make = |parallel| {
                let mut e = engine(&p, parallel);
                e.set_worker_limit(usize::MAX);
                if passes != 0 {
                    // Most work is on dependent buses, excluded by the old worker path.
                    for t in 1..11 {
                        e.set_effect(t, 0, Arc::new(Mutex::new(Work(passes))));
                    }
                    for t in 3..11 {
                        e.set_effect_ports(t, 0, 0, 1);
                    }
                }
                e
            };
            let (mut serial, mut parallel) = (make(false), make(true));
            let mut times = [Vec::new(), Vec::new()];
            let mut graph_blocks = 0;
            // Alternate execution order to reduce frequency/warm-cache bias.
            for block in 0..380 {
                for which in [block % 2, 1 - block % 2] {
                    let e = if which == 0 {
                        &mut serial
                    } else {
                        &mut parallel
                    };
                    fill(e, frames, block);
                    let song = e.transport_at((frames * block) as u64);
                    let start = Instant::now();
                    e.mix(frames, &song);
                    let elapsed = start.elapsed();
                    if block >= 80 {
                        times[which].push(elapsed.as_secs_f64() * 1000.);
                        if which == 1 && e.parallel.done.iter().any(|done| *done) {
                            graph_blocks += 1;
                        }
                    }
                }
                assert_eq!(serial.bus, parallel.bus);
            }
            for (mode, times) in ["serial", "graph"].into_iter().zip(&mut times) {
                times.sort_by(f64::total_cmp);
                let deadline = frames as f64 / 48.;
                let missed = times.iter().filter(|&&t| t > deadline).count();
                println!(
                    "{name} {frames} {mode}: median={:.3}ms p95={:.3}ms p99={:.3}ms max={:.3}ms over_block_budget={missed}/{} graph_blocks={graph_blocks}",
                    times[times.len() / 2],
                    times[times.len() * 95 / 100],
                    times[times.len() * 99 / 100],
                    times.last().unwrap(),
                    times.len()
                );
            }
        }
    }
}
