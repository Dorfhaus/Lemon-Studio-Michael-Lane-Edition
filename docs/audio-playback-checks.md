# Playback release checks

Offline checks help isolate failures but do not certify a physical audio driver or another computer.

## Installed instrument and sampler diagnostic

`transport::tests::installed_vst_three_track_queue_probe` is an ignored, opt-in test. Set
`LEMON_STRESS_PLUGIN` to an installed VST3 instrument and `LEMON_STRESS_SECONDS` to the
duration per scenario (default 10 seconds, capped at 120). Use an isolated
`OPENSTUDIO_USER_DIR` and an E: temporary directory on the Windows development machine.
Run the normal storage guard before compiling.

```text
cargo test --locked --release -p os-app installed_vst_three_track_queue_probe -- --ignored --nocapture --test-threads=1
```

The test renders a synthetic 44.1 kHz sample through two actual hosted stock
samplers at 48 kHz, with and without a four-note chord in the selected instrument.
Each scenario creates a fresh engine and runs the real feeder, render-ahead queue,
and paced synthetic callback at 48, 128, 192, 512, and 1024 frames. It reports underruns,
silent blocks, nonfinite samples, callback lateness, callback execution time, and
peak published render load. It never opens an editor, microphone or audio device.
The plugin uses its initial preset. This is not the reporter's project or preset.

Do not run compilation or other timing tests concurrently. Callback pacing uses
the local scheduler and does not emulate ASIO, an older CPU, or driver buffer
switches. Sampler output can mask an instrument-specific silent gap, so also use
`installed_vst_audio_probe` to check that the instrument itself produces audio.

## Hardware acceptance

For a reproducible dropout report, retain the exact build, project and samples,
plugin format/version/preset, OS/CPU, driver version, sample rate and buffer size.
Compare a copy with the suspect instrument removed. Hold the buffer fixed, reset
the counter, and measure a continuous run separately from changing buffer sizes.
Capture CPU/render load and actual audio behavior. Where possible compare the
same workload and driver configuration with another host.

The existing Underruns total combines output-queue shortages, hosted-plugin misses
and driver overload notifications. It does not identify the source by itself or
count unique audible incidents. `OS_DEBUG=1` logs queue depth and callback spacing
every two seconds during playback; collect logs locally without private project
data in public reports. Monitor dropouts have a separate counter.

An unresolved user-reported failure stays open even when these local tests pass.
Do not call a release audio-verified until the affected setup passes its acceptance
run, or the owner explicitly changes the release decision with the limitation known.

## Paired callbacks and buffer reserve (13 November 2026)

Playback now reserves a full additional device block, with a 3 ms minimum.
At 44.1 kHz, the former 6 ms maximum left only 264 extra frames for a 1024-frame
callback; two callbacks before the feeder ran could exhaust the queue. The new
policy preserves steady latency for 16-256-frame buffers at 44.1/48 kHz. Larger
buffers intentionally add more queue latency (about 17.2 ms extra at 44.1 kHz /
1024 frames). Keep a small stable buffer for live playing and recording. Startup
prefill now uses the actual negotiated fixed buffer, rather than imposing a
1024-frame minimum on small ASIO buffers.

`playback_bursts_preserve_every_sample_and_position` exercises the real engine,
feeder and renderer across 54 rate/PPQ/buffer combinations. It denies the feeder
a turn between paired callbacks and checks exact samples and playback position.
It failed before the reserve correction. This is stronger than a queue-size model
but does not reproduce physical ASIO callbacks or an Intel/Focusrite system.

Set `LEMON_STRESS_BURSTS=1` for paired callbacks in
`installed_vst_three_track_queue_probe`; `LEMON_STRESS_RATE=44100` selects the
reporter's rate (48000 default; 96000 also supported). Sampler-only and hosted
plugin cases are separate. Effect slots are explicitly enabled; older versions
of this diagnostic left template effect groups bypassed, so their effect passes
must not be used as proof of active effect processing. Instruments were unaffected.
The test uses initial presets and closed editors; it does not claim coverage for
arbitrary state/preset changes or visible editor work.

With `OS_DEBUG=1`, existing transport diagnostic lines now report minimum AND
maximum callback spacing, queue/plugin/driver lifetime failure counts and missing
output frames. The normal resettable total is retained. Diagnostics contain
counts and timings, not captured audio. Compare deltas during fixed-buffer runs;
changing buffers/reopening devices is a separate scenario. These are debug
extensions with no new workspace UI requiring a visibility switch.

A second fix rechecks queue demand after each bounded feeder pass. If callbacks
consumed more audio during rendering, the feeder releases the engine mutex and
refills again instead of parking for up to 5 ms with a deficit. Paused, finished
or full queues still park. The bounded-pass overload protection remains intact.
