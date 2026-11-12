// SPDX-License-Identifier: GPL-3.0-or-later
//! Per-placement edit history. Analysis and immutable audio stay in `vocal::Edit`.
use crate::vocal::{Edit, Note};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

pub type Timing = Vec<(f64, f64)>;
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct History<T> {
    pub current: Arc<T>,
    pub undo: Vec<Arc<T>>,
    pub redo: Vec<Arc<T>>,
}
impl<T: Clone + PartialEq> History<T> {
    pub fn new(current: T) -> Self {
        Self {
            current: Arc::new(current),
            undo: vec![],
            redo: vec![],
        }
    }
    pub fn commit(&mut self, next: T) -> bool {
        if *self.current == next {
            return false;
        }
        self.undo.push(std::mem::replace(&mut self.current, Arc::new(next)));
        if self.undo.len() > 64 {
            self.undo.remove(0);
        }
        self.redo.clear();
        true
    }
    pub fn step(&mut self, redo: bool) -> bool {
        let (from, to) = if redo {
            (&mut self.redo, &mut self.undo)
        } else {
            (&mut self.undo, &mut self.redo)
        };
        let Some(next) = from.pop() else { return false };
        to.push(std::mem::replace(&mut self.current, next));
        true
    }
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Pitch {
    pub notes: Vec<Note>,
    pub curve: Vec<(u32, f32)>,
    pub enabled: bool,
    pub strength: f32,
    pub formant: f32,
}
impl Pitch {
    pub fn read(d: &Edit) -> Self {
        Self {
            notes: d.notes.clone(),
            curve: d.curve.clone(),
            enabled: d.enabled,
            strength: d.strength,
            formant: d.formant,
        }
    }
    pub fn put(&self, d: &mut Edit) {
        d.notes = self.notes.clone();
        d.curve = self.curve.clone();
        d.enabled = self.enabled;
        d.strength = self.strength;
        d.formant = self.formant;
    }
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Document {
    pub channel: u16,
    pub clip: u32,
    pub arrangement: u16,
    pub source_range: (f64, f64),
    pub output_range: (f64, f64),
    pub analyzed: bool,
    pub pitch: History<Pitch>,
    pub timing: History<Timing>,
}
impl Document {
    pub fn new(d: &Edit, clip: u32, arrangement: u16, range: (f64, f64), analyzed: bool) -> Self {
        Self {
            channel: d.channel,
            clip,
            arrangement,
            source_range: (d.original(range.0), d.original(range.1)),
            output_range: range,
            analyzed,
            pitch: History::new(Pitch::read(d)),
            timing: History::new(d.timing.clone()),
        }
    }
    pub fn commit(&mut self, d: &Edit) -> bool {
        let p = self.pitch.commit(Pitch::read(d));
        self.timing.commit(d.timing.clone()) || p
    }
    pub fn restore(&self, d: &mut Edit) {
        self.pitch.current.put(d);
        d.timing = (*self.timing.current).clone();
    }
    pub fn valid(&self, d: &Edit) -> bool {
        self.channel == d.channel
            && self.clip != 0
            && self.source_range.0.is_finite()
            && self.source_range.1.is_finite()
            && self.source_range.0 >= 0.
            && self.source_range.1 > self.source_range.0
            && self.source_range.1 <= d.frames as f64 / d.rate.max(1) as f64 + 0.000001
            && self.output_range.1 <= d.frames as f64 / d.rate.max(1) as f64 + 0.000001
            && self.output_range.0.is_finite()
            && self.output_range.1.is_finite()
            && self.output_range.0 >= 0.
            && self.output_range.1 > self.output_range.0
            && self.pitch.undo.len() <= 64
            && self.pitch.redo.len() <= 64
            && self.timing.undo.len() <= 64
            && self.timing.redo.len() <= 64
            && std::iter::once(&self.pitch.current)
                .chain(&self.pitch.undo)
                .chain(&self.pitch.redo)
                .all(|p| {
                    let mut v = d.clone();
                    p.put(&mut v);
                    v.valid()
                })
            && std::iter::once(&self.timing.current)
                .chain(&self.timing.undo)
                .chain(&self.timing.redo)
                .all(|t| {
                    let mut v = d.clone();
                    v.timing = (**t).clone();
                    v.valid()
                })
    }
    /// Restrict every historical map to the *current* source slice; fix its audible boundaries.
    pub fn restrict(&mut self, range: (f64, f64)) {
        let source = (
            map(&self.timing.current, range.0, true),
            map(&self.timing.current, range.1, true),
        );
        for t in std::iter::once(&mut self.timing.current)
            .chain(&mut self.timing.undo)
            .chain(&mut self.timing.redo)
        {
            let (a, b) = (map(t, source.0, false), map(t, source.1, false));
            let duration = t.last().unwrap().0;
            let mut next = vec![(0., 0.)];
            if source.0 > 0. {
                next.push((source.0, range.0));
            }
            next.extend(
                t.iter()
                    .filter(|p| p.0 > source.0 && p.0 < source.1)
                    .map(|&(x, y)| (x, range.0 + (y - a) / (b - a) * (range.1 - range.0))),
            );
            if source.1 < duration {
                next.push((source.1, range.1));
            }
            next.push((duration, duration));
            *t = Arc::new(next);
        }
        let lo = (source.0 / crate::vocal::STEP).ceil() as u32;
        let hi = (source.1 / crate::vocal::STEP).ceil() as u32;
        for p in std::iter::once(&mut self.pitch.current)
            .chain(&mut self.pitch.undo)
            .chain(&mut self.pitch.redo)
        {
            let p = Arc::make_mut(p);
            p.notes.retain_mut(|n| {
                n.start = n.start.max(lo);
                n.end = n.end.min(hi);
                n.start < n.end
            });
            p.curve.retain(|(f, _)| *f >= lo && *f < hi);
        }
        self.source_range = source;
        self.output_range = range;
    }
}
pub fn map(points: &[(f64, f64)], at: f64, inverse: bool) -> f64 {
    if points.len() < 2 {
        return at;
    }
    let pair = |p: (f64, f64)| if inverse { (p.1, p.0) } else { p };
    let i = points
        .partition_point(|p| pair(*p).0 < at)
        .clamp(1, points.len() - 1);
    let (a, b) = (pair(points[i - 1]), pair(points[i]));
    a.1 + (at - a.0) / (b.0 - a.0) * (b.1 - a.1)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn snapshots_share_history_and_branch_independently() {
        let mut original = History::new(vec![0; 10000]);
        for i in 1..=64 { original.commit(vec![i; 10000]); }
        let mut copy = original.clone();
        assert!(Arc::ptr_eq(&original.current, &copy.current));
        assert!(original.undo.iter().zip(&copy.undo).all(|(a,b)| Arc::ptr_eq(a,b)));
        copy.step(false);
        copy.commit(vec![99; 10000]);
        assert_eq!(original.current[0], 64);
        assert_eq!(copy.current[0], 99);
        assert!(original.redo.is_empty());
    }
    #[test]
    fn bounded_independent_history() {
        let mut h = History::new(0);
        for i in 1..=80 {
            assert!(h.commit(i));
        }
        assert_eq!(h.undo.len(), 64);
        assert!(!h.commit(80));
        assert!(h.step(false));
        assert_eq!(*h.current, 79);
        assert!(h.step(true));
        assert_eq!(*h.current, 80);
        h.step(false);
        h.commit(81);
        assert!(h.redo.is_empty());
    }
}
