# Lemon Wave

An original wavetable instrument for Lemon Studio, developed in stages at Michael's request. The first stage is a Windows CLAP/VST3 instrument, not a complete Serum clone. No Serum code, factory tables, presets or artwork are included.

## Stage 1

Two oscillators, each with four analytic wavetable families (five interpolated frames), table position, gain, semitone tuning, 1–7 unison voices, detune, pan and phase bend. Twenty-four polyphonic voices, sub and noise sources, stereo spread, a resonant state-variable filter, drive, amplitude/filter ADSRs, four LFO shapes with four destinations and host-tempo rate divisions, stereo chorus and ping-pong delay. Eight original factory patches. Forty-four stable host parameters; CLAP and VST3 state recall and MIDI sample offsets.

The engine precomputes harmonic-limited table levels outside audio processing. It uses interpolation and 10 ms continuous-control smoothing. Phase bend and nonlinear drive/output can still introduce aliasing; oversampling and audio-reference comparisons are future work. LFO tempo sync means tempo-relative rate with note retrigger, not host-song phase locking. Fixed voice stealing can produce a transient at the polyphony limit. Delay-time changes can pitch the tail. These are explicit first-stage limitations.

The editor uses Lemon Studio's existing parameter gestures, automation hooks and preset wrapper. Factory `.ospreset` files select the CLAP identity. The VST3 build can save its own host presets, but does not load CLAP preset headers directly.

## Build and verify

`cargo test --locked -p lemon-wave --lib`

`cargo build --locked --release -p lemon-wave`

Package `lemon_wave.dll` as `Lemon Wave.clap` and as `Lemon Wave.vst3/Contents/x86_64-win/Lemon Wave.vst3` on Windows. `tools/bundle-plugins` also knows the module.

`cargo run --locked -p lemon-wave --example verify_host -- BUNDLE_DIRECTORY` tests both actual Lemon Studio hosting layers without opening a GUI or audio device. `render_presets` takes bundle, preset and output directories and validates/renders all patches offline. `LEMON_WAVE_PREVIEW` selects an offscreen test image directory.

## Next stages (not implemented)

1. Wavetable WAV import, embedded per-project tables, frame editing/drawing, spectral editing, table export, higher-quality warp/oversampling, phase/randomization controls.
2. Drag-and-drop modulation matrix, multiple editable LFOs/envelopes, macros, MPE/aftertouch/velocity routing, modulation visualization, mono/legato/glide.
3. Expanded filter/routing and reorderable effects rack: EQ, distortion modes, compression, reverb, phaser/flanger, delay synchronization and additional chorus modes.
4. Serum 2-style broader synthesis options: third oscillator, sample/multisample, granular and spectral engines; arpeggiation/sequencing; larger preset browser and sound library.
5. Listening, spectral/aliasing tests, CPU profiling, automation stress and user-guided interaction refinement. Mac/Linux builds require their own verification.

## Reference investigation

Installed target: Xfer Serum 2.1.5, native Windows x64 VST3, 19,236,864 bytes. SHA-256 `33369e196ae66d05d20485f3c5e800707dc34a5ad89c27959250d60387b2ea55`. Version and classes are observed in `Contents/Resources/moduleinfo.json`. REA 6.8.0/Ghidra 12.1.4 performs static inspection only; it has not run Serum or established audio equivalence. The initial cold import exceeded 330 seconds. Evidence/results from the extended attempt are recorded in the project handoff and Desktop research folder.

Xfer's official feature description establishes that Serum 2 includes wavetable, multisample, sample, granular and spectral modes: https://xferrecords.com/web-manual/serum-2/exploring-sound-design-in-serum . Its routing documentation describes three oscillator sources plus sub/noise and two filters: https://xferrecords.com/web-manual/serum-2/routing-an-oscillator-or-filter . These are published behavior descriptions, not proof of recovered internal algorithms. Lemon Wave's DSP is an independent implementation; remaining parity claims are unknown.
