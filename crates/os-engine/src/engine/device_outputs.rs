// SPDX-License-Identifier: GPL-3.0-or-later

use super::pdc::Pad;
use crate::varispeed::Varispeed;
use os_model::{HardwareOutput, MixerTrack};

#[derive(Default)]
pub(super) struct DeviceOutputs {
    device: String,
    routes: Vec<Option<(usize, Pad)>>,
    pairs: Vec<(Vec<f32>, Varispeed)>,
    scratch: Vec<f32>,
    pub(super) interleaved: Vec<f32>,
}

impl DeviceOutputs {
    pub(super) fn configure(&mut self, device: String, pairs: usize, tracks: &[MixerTrack]) {
        self.device = device;
        self.pairs.resize_with(pairs, Default::default);
        self.routes.clear();
        if pairs == 0 { return; }
        for track in tracks {
            self.set_route(usize::from(track.index), track.hardware_output.as_ref());
        }
    }

    pub(super) fn set_route(&mut self, track: usize, output: Option<&HardwareOutput>) {
        if self.pairs.is_empty() { return; }
        let pair = output
            .filter(|output| output.device == self.device && output.first >= 2 && output.first % 2 == 0)
            .map(|output| usize::from(output.first / 2 - 1))
            .filter(|&pair| pair < self.pairs.len());
        if pair.is_none() && track >= self.routes.len() { return; }
        if track >= self.routes.len() { self.routes.resize_with(track + 1, || None); }
        if self.routes[track].as_ref().map(|route| route.0) != pair {
            self.routes[track] = pair.map(|pair| (pair, Pad::default()));
        }
    }

    pub(super) fn continue_from(&mut self, old: &mut Self) {
        if self.device != old.device || self.pairs.len() != old.pairs.len() { return; }
        std::mem::swap(&mut self.pairs, &mut old.pairs);
        std::mem::swap(&mut self.interleaved, &mut old.interleaved);
        std::mem::swap(&mut self.scratch, &mut old.scratch);
        for (next, previous) in self.routes.iter_mut().zip(&mut old.routes) {
            if let (Some((pair, pad)), Some((old_pair, old_pad))) = (next, previous)
                && pair == old_pair
            {
                std::mem::swap(pad, old_pad);
            }
        }
    }

    pub(super) fn sounding(&self, buses: &[Vec<f32>]) -> bool {
        self.interleaved.iter().any(|sample| sample.abs() >= super::QUIET)
            || self.routes.iter().zip(buses).any(|(route, bus)| {
                route.is_some() && bus.iter().any(|sample| sample.abs() >= super::QUIET)
            })
    }

    /// Buses contain the post-fader track signal. Apply the same head trim and
    /// tape-speed clock as the main output before putting all pairs in one block.
    pub(super) fn render(
        &mut self, buses: &[Vec<f32>], window: std::ops::Range<usize>,
        master_delay: u64, leaving: &[i64], clock: &Varispeed, ratio: f64,
    ) {
        self.interleaved.clear();
        if !self.routes.iter().any(Option::is_some) { return; }
        for (audio, _) in &mut self.pairs {
            audio.clear();
            audio.resize(window.len(), 0.0);
        }
        for (index, route) in self.routes.iter_mut().enumerate() {
            let Some((pair, pad)) = route else { continue };
            let Some(bus) = buses.get(index).filter(|bus| window.end <= bus.len()) else { continue };
            let latency = leaving.get(index).copied().unwrap_or_default();
            pad.set_len((master_delay as i64).saturating_sub(latency.max(0)).max(0) as usize);
            self.scratch.clear();
            self.scratch.extend_from_slice(bus);
            pad.run(&mut self.scratch);
            for (out, &sample) in self.pairs[*pair].0.iter_mut().zip(&self.scratch[window.clone()]) {
                *out += sample;
            }
        }
        let mut samples = 0;
        for (audio, speed) in &mut self.pairs {
            speed.sync_clock(clock);
            speed.process(audio, ratio);
            samples = audio.len();
        }
        let width = self.pairs.len() * 2;
        self.interleaved.resize(samples / 2 * width, 0.0);
        for (pair, (audio, _)) in self.pairs.iter().enumerate() {
            for (out, stereo) in self.interleaved.chunks_exact_mut(width).zip(audio.chunks_exact(2)) {
                out[pair * 2..pair * 2 + 2].copy_from_slice(stereo);
            }
        }
    }
}
