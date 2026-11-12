# Mixer dependency scheduler

Implemented in November 2026 in `os-engine`. The Audio settings' existing
Multithreaded mixer processing switch enables it. Processing work limit bounds
simultaneous jobs, including work performed by the feeder; one selects serial
processing. Generator multithreading remains independently selectable. The host
passes its selected processing priority to both kinds of DSP worker.

## Execution

The render-ahead feeder and device callback are unchanged. At each mixer block,
the engine can schedule complete track DSP chains, including native strip
processing, hosted and built-in slots, faders, meters, and compensated sends.
The master follows its dependencies. Instrument rendering still precedes mixing.

The cached plan includes every route, including zero-level sidechain sends,
resolved sidechain and alternate main inputs, and ordering edges between tracks
that use the same hosted plugin instance. Its exact signature detects route,
port, track, and plugin identity edits at a block boundary. Gains and bypass
switches use the existing runtime state without rebuilding topology. Existing
automation and PDC planning remain authoritative.

Ready work is prioritized by estimated remaining branch cost. A completed source
releases its dependents immediately, without waiting for unrelated jobs in a
batch. Small tasks and a single ready chain can execute on the feeder. Initial
blocks learn costs serially; total cost below 300 microseconds or an estimated
ideal parallel saving below 80 microseconds keeps serial execution. Feedback or
backward-input graphs also retain the existing serial behavior, before any job
is dispatched. The scheduler never breaks a cycle by dropping a route.

## Correctness and ownership

Each job exclusively owns its track state and audio buffer until completion.
There are no raw shared mutable DSP pointers. Sidechain/main input snapshots and
per-contribution send buffers are reused. For every destination, the feeder adds
source contributions in the original mixer order, with each auxiliary output
and send added separately. No nondeterministic atomic summation or pre-summing
is used. This preserves floating-point addition order as well as routing.

Slot parameter delivery waits for the same dependencies as processing. Shared
plugin instances retain their original inter-track invocation order. PDC pads,
gain glides, bypass behavior, master/insert effects options, track capture,
meters, and latency-change notifications are preserved. A caught Rust plugin
panic returns the job, restores the block's input, discards partial auxiliary
outputs and records a missed block. This does not contain native crashes or
hung plugins. Existing plugin-lock wait budgets still apply.

The persistent pool and its instrument jobs are reused. Per-engine reply queues
isolate concurrent renders. DSP jobs and buffers are retained per track. Changes
in topology or block size may allocate on the feeder/worker side; the upgrade
does not claim an allocation-free render engine. No scheduling or new allocation
was added to the device callback.

## Verification

The scheduler tests compare every track sample against serial rendering across
random DAGs, diamonds, multiple sends, keyed/alternate inputs, native auxiliary
outputs, PDC, parameter automation, bypass/mix edits, live route edits, capture,
variable blocks, engine replacement, seek and changing latency. They also check
shared plugin ordering, worker caps, buffer reuse, busy/panicking plugins,
priority switching, concurrent engines and immediate downstream release.

The opt-in CPU benchmark uses 19 tracks with most plugin work on dependent buses,
which the previous independent-insert worker path excluded. It alternates serial
and parallel execution order, warms up 80 blocks, measures 300 blocks, and checks
sample equality. It reports median, p95, p99, maximum and blocks over the nominal
buffer budget. These are synthetic CPU measurements, not driver underrun counts
or a comparison against the running Gridline application.

Windows desktop result on 11 November 2026, in release mode (milliseconds).

| Frames | Serial median | Graph median | Serial p99 | Graph p99 |
| --- | ---: | ---: | ---: | ---: |
| 64 | 0.449 | 0.172 | 0.478 | 0.294 |
| 128 | 0.895 | 0.334 | 1.064 | 0.508 |
| 256 | 1.766 | 0.677 | 2.068 | 0.917 |

The light workload remained serial with 0.004, 0.007 and 0.012 ms medians.
No measured block exceeded its nominal buffer budget. The 10-second-per-buffer
routed queue stress run also recorded zero underruns, silent gaps, invalid
samples or late synthetic callbacks during live routing/control edits. Actual
worker invocation counts were 13,299, 14,172 and 14,896 respectively.

Run the storage guard for the actual Cargo target directory before compiling.

```powershell
cargo test --locked --release -p os-engine --lib -- mixer_graph mix_workers --test-threads=1
cargo test --locked --release -p os-engine --lib benchmark_routed_mixer_scheduler -- --ignored --nocapture
```

The app's opt-in queue stress test uses synthetic signal input, dependent CPU
effects, actual feeder workers, paced callbacks and live mixer edits. It checks
that workers actually process the buses, and rejects queue underruns, silent
gaps and nonfinite output at 64, 128 and 256 frames. It opens no audio device.

```powershell
$env:LEMON_STRESS_SECONDS='10'
$env:LEMON_STRESS_ROUTED_MIXER='1'
cargo test --locked --release -p os-app --bin lemonstudio sustained_low_buffer_stress -- --ignored --nocapture
```

Use the project's normal personal engine-bundle environment when building the
app. Physical interface/listening checks and native Mac/Linux validation remain
separate from these Windows synthetic checks. This scheduler upgrade adds no
new visible feature, so it uses the existing Audio/Workspace Visibility controls.
