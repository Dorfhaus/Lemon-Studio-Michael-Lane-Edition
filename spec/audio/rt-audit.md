# Real-time allocation and restore diagnostics

The optional `os-app/rt-audit` Cargo feature installs a counting wrapper around
Rust's system allocator. Scoped counters distinguish the device renderer, feeder
and mixer/instrument worker jobs. Ordinary builds compile the scopes away and do
not replace the allocator. The counters do not cover third-party native heaps.
They count allocation attempts, deallocations and reallocations (as both), calls
and maximum scope duration. Concurrent phase boundaries are approximate. No
logging, allocation or mutex is introduced by the counter itself.

Build with the normal validated native bundles and storage preflight, adding
`cargo test -p os-app --release --features rt-audit --no-run`. Run diagnostics
serially, with isolated profile/cache/temp directories, and without concurrent
compilation. Audit builds have overhead; their timing is not a production benchmark.

* `steady_mix_allocation_audit`: warm-up followed by 1,000 blocks of continuous
  synthetic input at normal, half and double tape speed. The simple engine has no
  scheduled note events or hosted plugins and meter
  collection is disabled. This deliberately narrow regression requires zero
  allocation/deallocation in both renderer and feeder; it does not certify every
  project or edit path.
* `installed_editor_with_app_parameter_tick_and_queue --ignored`: set
  `LEMON_STRESS_HEADLESS=1` to omit editor creation. Otherwise explicit GUI
  authorization is required. Set `LEMON_STRESS_PLUGIN` to an installed plugin;
  descriptor version and path are printed. `LEMON_STRESS_LIFECYCLE=1` and
  `LEMON_STRESS_STEP=5` restore saved state during playback. The middle phase
  performs the operation even in headless mode. Other phases are playback controls.
* Set `LEMON_STRESS_SYNTHETIC_MS=0`, `8`, or `20` instead of a plugin path for a
  native-independent generator with a controlled state-restore delay. A failure
  here is evidence of host contention without depending on a vendor/version.
* `sustained_low_buffer_stress --ignored` with `LEMON_STRESS_ROUTED_MIXER=1`
  exercises actual mixer workers, live control edits and small callback sizes.

The lifecycle probe asserts zero reported dropouts. Deliberately slow synthetic
restoration currently fails that assertion: it is a retained reproduction, not
a passing regression or an expected failure silently counted as success. In-place
state restore holds the instance mutex; native call duration can exceed the queue
deadline. Do not remove synchronization or hide missed ticks to make it pass.

Meter vectors can still grow until their consumer returns storage. The lifecycle
fixture does not run the full meter UI, so its feeder allocation totals include
that retention. Worker channels, routing changes, plugin state transitions and
native plugin memory require further coverage. Physical-driver, open-editor,
Mac and Linux acceptance are separate from these synthetic Windows checks.
