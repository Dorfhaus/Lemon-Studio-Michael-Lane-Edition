// SPDX-License-Identifier: GPL-3.0-or-later
//! Headless verification through Lemon Studio's actual CLAP/VST3 hosts.
use os_host::Library;
use std::{path::Path, sync::Arc};
fn verify(lib: Arc<dyn Library>) {
    let desc = lib
        .plugins()
        .iter()
        .find(|d| d.name == "Lemon Wave")
        .expect("instrument descriptor")
        .clone();
    assert!(desc.is_instrument());
    let shared = lib.create(&desc.id).expect("create");
    let mut p = shared.lock().unwrap();
    assert!(p.has_editor());
    assert!(p.activate(48000));
    let infos: Vec<_> = (0..p.param_count())
        .filter_map(|i| p.param_info(i))
        .collect();
    for spec in lemon_wave::controls::SETTINGS {
        assert!(infos.iter().any(|i| i.name == spec.name), "{}", spec.name);
    }
    let gain = infos.iter().find(|i| i.name == "Output").unwrap().id;
    let mut flush_l = [0.; 128];
    let mut flush_r = flush_l;
    assert!(p.set_param(gain, 0.37));
    p.process(48000, &mut flush_l, &mut flush_r);
    let state = p.save_state().expect("save state");
    assert!(p.set_param(gain, 0.9));
    p.process(48000, &mut flush_l, &mut flush_r);
    assert!(p.load_state(&state));
    p.idle();
    assert!(
        (p.param_value(gain).unwrap() - 0.37).abs() < 0.002,
        "restored {}",
        p.param_value(gain).unwrap()
    );
    p.reset();
    p.note_on(37, 69, 0.8);
    let mut l = [0.; 512];
    let mut r = l;
    p.process(48000, &mut l, &mut r);
    assert!(l[..37].iter().all(|x| x.abs() < 1e-9), "early MIDI event");
    assert!(l[100..].iter().any(|x| x.abs() > 0.001), "no sound");
    let release = infos.iter().find(|i| i.name == "Release").unwrap().id;
    assert!(p.set_param(release, 0.));
    p.note_off(0, 69);
    for _ in 0..32 {
        p.process(48000, &mut l, &mut r);
    }
    assert!(l.iter().chain(&r).all(|x| x.abs() < 1e-6));
    p.reset();
    p.process(48000, &mut l, &mut r);
    assert!(l.iter().chain(&r).all(|x| *x == 0.));
    println!(
        "{}: descriptor, 44 controls, state restore, MIDI offset, note release and reset PASS",
        desc.uid()
    );
}
fn main() {
    let dir = std::env::args().nth(1).expect("bundle directory");
    verify(os_clap::Library::load(&Path::new(&dir).join("Lemon Wave.clap")).expect("CLAP load"));
    verify(os_vst3::Module::load(&Path::new(&dir).join("Lemon Wave.vst3")).expect("VST3 load"));
}
