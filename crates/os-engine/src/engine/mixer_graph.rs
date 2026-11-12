//! Dependency scheduling inside the render engine. The device callback still only
//! drains the render-ahead queue. Jobs own their DSP state, and inputs are summed
//! in the original mixer order, independent of worker completion order.
use super::{Engine, Fx, MixTrack, TransportInfo, mix_workers};
use std::cmp::Reverse;
use std::collections::{BinaryHeap, HashMap};
use std::sync::Arc;
use std::time::{Duration, Instant};

#[cfg(test)]
mod tests;

#[derive(Default)]
pub(super) struct Plan {
    signature: Vec<usize>,
    scratch_signature: Vec<usize>,
    pub valid: bool,
    pub sources: Vec<Vec<usize>>,
    pub feeds: Vec<Vec<usize>>,
    followers: Vec<Vec<usize>>,
    counts: Vec<usize>,
    pending: Vec<usize>,
    priority: Vec<Duration>,
    ready: BinaryHeap<(Duration, Reverse<usize>, usize)>,
    positions: Vec<usize>,
    #[cfg(test)]
    pub rebuilds: usize,
}

impl Plan {
    /// Compare exact topology, not a lossy hash. Gain, bypass and automation edits
    /// do not rebuild the plan. Port, route and shared-instance edits do.
    pub fn prepare(&mut self, tracks: &[MixTrack], order: &[usize]) {
        let sig = &mut self.scratch_signature;
        sig.clear();
        sig.push(tracks.len());
        sig.extend_from_slice(order);
        for track in tracks {
            sig.push(track.routes.len());
            sig.extend(track.routes.iter().map(|r| r.dest));
            sig.push(track.effects.len());
            for fx in &track.effects {
                sig.extend([fx.key.unwrap_or(usize::MAX), fx.main.unwrap_or(usize::MAX)]);
                sig.push(match &fx.fx {
                    Fx::Hosted(h) => Arc::as_ptr(h) as *const () as usize,
                    Fx::Own(_) => 0,
                });
            }
        }
        if *sig == self.signature {
            return;
        }
        std::mem::swap(&mut self.signature, sig);
        #[cfg(test)]
        {
            self.rebuilds += 1;
        }
        let n = tracks.len();
        self.valid = order.len() == n;
        self.positions.clear();
        self.positions.resize(n, usize::MAX);
        for (i, &t) in order.iter().enumerate() {
            if t >= n || self.positions[t] != usize::MAX {
                self.valid = false;
                return;
            }
            self.positions[t] = i;
        }
        self.sources.resize_with(n, Vec::new);
        self.feeds.resize_with(n, Vec::new);
        self.followers.resize_with(n, Vec::new);
        self.counts.clear();
        self.counts.resize(n, 0);
        for list in self
            .sources
            .iter_mut()
            .chain(&mut self.feeds)
            .chain(&mut self.followers)
        {
            list.clear();
        }
        let mut last_instance = HashMap::new();
        for &t in order {
            for route in &tracks[t].routes {
                if route.dest >= n {
                    self.valid = false;
                    continue;
                }
                if !self.sources[route.dest].contains(&t) {
                    self.sources[route.dest].push(t);
                }
                self.edge(t, route.dest);
            }
            for fx in &tracks[t].effects {
                for src in [fx.key, fx.main].into_iter().flatten() {
                    if src >= n {
                        self.valid = false;
                        continue;
                    }
                    if !self.feeds[t].contains(&src) {
                        self.feeds[t].push(src);
                    }
                    self.edge(src, t);
                }
                if let Fx::Hosted(h) = &fx.fx {
                    let id = Arc::as_ptr(h) as *const () as usize;
                    if let Some(previous) = last_instance.insert(id, t).filter(|&p| p != t) {
                        // Preserve calls AND parameter delivery for a plugin used in
                        // several strips, even when those strips have no audio edge.
                        self.edge(previous, t);
                    }
                }
            }
        }
        self.pending.resize(n, 0);
        self.priority.resize(n, Duration::ZERO);
        self.ready.reserve(n);
    }

    fn edge(&mut self, source: usize, dest: usize) {
        // Feedback and unusual backwards ports retain the existing serial semantics.
        // No partial dispatch is allowed before this validation finishes.
        if self.positions[source] >= self.positions[dest] {
            self.valid = false;
            return;
        }
        if !self.followers[source].contains(&dest) {
            self.followers[source].push(dest);
            self.counts[dest] += 1;
        }
    }

    pub fn start(&mut self, order: &[usize], cost: &[Duration]) {
        self.pending.copy_from_slice(&self.counts);
        self.ready.clear();
        // Prefer the longest remaining branch, with stable mixer order for ties.
        for &t in order.iter().rev() {
            self.priority[t] = cost[t]
                + self.followers[t]
                    .iter()
                    .map(|&d| self.priority[d])
                    .max()
                    .unwrap_or_default();
        }
        for &t in order {
            if self.pending[t] == 0 {
                self.enqueue(t);
            }
        }
    }
    fn enqueue(&mut self, t: usize) {
        self.ready
            .push((self.priority[t], Reverse(self.positions[t]), t));
    }
    fn pop(&mut self) -> Option<usize> {
        self.ready.pop().map(|(_, _, t)| t)
    }
    fn complete(&mut self, t: usize) {
        for i in 0..self.followers[t].len() {
            let d = self.followers[t][i];
            self.pending[d] -= 1;
            if self.pending[d] == 0 {
                self.enqueue(d);
            }
        }
    }
}

impl Engine {
    /// Returns None before moving any state when this block should run serially.
    pub(super) fn mix_graph(&mut self, frames: usize, song: &TransportInfo) -> Option<bool> {
        let state = &mut self.parallel;
        if self.worker_limit == 1
            || !state.enabled
            || !mix_workers::available()
            || state.cost.iter().sum::<Duration>() < Duration::from_micros(300)
        {
            return None;
        }
        state.plan.prepare(&self.tracks, &self.order);
        if !state.plan.valid {
            return None;
        }
        state.plan.start(&self.order, &state.cost);
        // A single dominant chain has no useful parallel work, even when its total
        // cost is high. Avoid dispatch/copy overhead when the ideal saving is tiny.
        let critical = state.plan.ready.peek().map_or(Duration::ZERO, |r| r.0);
        if state.cost.iter().sum::<Duration>().saturating_sub(critical) < Duration::from_micros(80)
        {
            return None;
        }
        state
            .jobs
            .resize_with(self.tracks.len(), || Some(Box::default()));
        let limit = self.worker_limit.min(mix_workers::worker_count()).max(1);
        let until = Some(*self.busy_until.get_or_insert_with(|| Instant::now() + self.busy_wait));
        let mut running = 0;
        let mut finished = 0;
        let mut moved = false;
        while finished < self.tracks.len() {
            // Accept completed work before dispatching more, so a ready downstream
            // branch need not wait for an unrelated slow job or a batch barrier.
            while let Ok(job) = self.parallel.rx.try_recv() {
                running -= 1;
                moved |= self.finish_mix_job(job, frames);
                finished += 1;
            }
            while running < limit {
                let Some(t) = self.parallel.plan.pop() else {
                    break;
                };
                // Apply every source's extra outputs and sends in exactly the serial
                // order. Never pre-sum contributions (floating point is not associative).
                for &s in &self.parallel.plan.sources[t] {
                    let source = self.parallel.jobs[s].as_ref().expect("completed source");
                    for add in source
                        .additions
                        .iter()
                        .take(source.addition_count)
                        .filter(|a| a.dest == t)
                    {
                        for (d, s) in self.bus[t].iter_mut().zip(&add.samples) {
                            *d += *s;
                        }
                    }
                }
                if !self.pdc.delivered {
                    self.params.deliver_slots(t, &self.tracks[t].effects);
                }
                let mut job = self.parallel.jobs[t].take().expect("idle mixer job");
                job.track = t;
                job.keys
                    .resize_with(self.parallel.plan.feeds[t].len(), Default::default);
                for ((id, samples), &src) in job.keys.iter_mut().zip(&self.parallel.plan.feeds[t]) {
                    *id = src;
                    samples.clear();
                    if let Some(k) = self
                        .key_out
                        .get(src)
                        .and_then(Option::as_ref)
                        .filter(|k| k.len() == frames * 2)
                    {
                        samples.extend_from_slice(k);
                    }
                }
                std::mem::swap(&mut job.mix, &mut self.tracks[t]);
                std::mem::swap(&mut job.buf, &mut self.bus[t]);
                job.rate = self.rate;
                job.song = *song;
                job.until = until;
                job.wait = self.busy_wait;
                job.export = self.exporting;
                job.smart_disable = self.smart_disable && !self.exporting;
                job.priority = self.worker_priority;
                job.slots_enabled = job.mix.slots_enabled
                    && if t == 0 {
                        self.master_fx
                    } else {
                        self.insert_fx
                    };
                // Cheap work and a lone ready chain run on the feeder. The work limit
                // counts the feeder too, so a setting of two never runs three jobs.
                let local = self.parallel.cost[t] < Duration::from_micros(40)
                    || (running == 0 && self.parallel.plan.ready.is_empty());
                if local {
                    job.run();
                    moved |= self.finish_mix_job(job, frames);
                    finished += 1;
                } else {
                    mix_workers::submit(job, self.parallel.tx.clone());
                    running += 1;
                }
            }
            if running > 0 {
                let job = self
                    .parallel
                    .rx
                    .recv()
                    .expect("mixer workers return owned state even after a plugin panic");
                running -= 1;
                moved |= self.finish_mix_job(job, frames);
                finished += 1;
            } else {
                debug_assert_eq!(
                    finished,
                    self.tracks.len(),
                    "validated mixer DAG must make progress"
                );
            }
        }
        Some(moved)
    }

    fn finish_mix_job(&mut self, mut job: Box<mix_workers::Job>, frames: usize) -> bool {
        let t = job.track;
        if let Some(k) = self.key_out.get_mut(t).and_then(Option::as_mut) {
            k.clear();
            k.extend_from_slice(&job.buf);
        }
        self.capture_mix_track(t, frames, &job.buf);
        self.missed += job.missed;
        self.parallel.cost[t] = job.elapsed;
        self.parallel.done[t] = true;
        let moved = job.moved;
        std::mem::swap(&mut self.tracks[t], &mut job.mix);
        std::mem::swap(&mut self.bus[t], &mut job.buf);
        self.parallel.jobs[t] = Some(job);
        self.parallel.plan.complete(t);
        moved
    }
}
