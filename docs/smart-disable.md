# Smart Disable

Settings > Audio > Advanced > CPU > Smart disable is optional and off by
default. It saves idle plugin DSP work during live playback and monitoring.
The control can be hidden in Settings > Visibility without changing its value.
Its per-profile preference is `smart_disable_v1`; the old inert `smart_disable`
preference does not enable it.

Compatible VST2/VST3 instances can sleep after four seconds of exact silence
plus their reported finite tail and latency. Effects require silent main AND
sidechain input. There is no silence threshold: quiet decays, noise and nonfinite
output continue to process. Sleeping does not reset/deactivate the plugin or
discard its state. Host clocks, routing, compensation pads and mixer processing
continue. This works with serial and multicore generator/mixer processing.

Notes, MIDI, automation, parameter/preset/state edits, channel control changes,
editor opening, rate changes, external plugin requests and transport changes
wake an instance. Queued future events prevent sleeping. Export always processes
fully, regardless of the live preference.

Unknown/infinite-tail plugins, active voices, held MIDI notes/pedals, auxiliary-output instruments,
ARA playback and open/busy editors stay awake. Unsupported native formats and
built-in mixer effects keep their existing processing. CLAP retains its existing
plugin-requested sleep/wake protocol. Accurate tail reporting is required;
turn Smart disable off if a plugin's internal sequencer or modulation behaves
differently after an idle gap. It cannot predict undocumented autonomous output.

Automated checks use synthetic plugins for long delayed tails, sample-accurate
wake-up, main/key input, edits, transport, sample-rate changes, clocks, off/default
behavior, rendering and worker propagation. These do not replace listening with
third-party plugins and the user's interface. Check a normal project, reverb and
delay tails, MIDI after a long pause, automation, sidechains and an export before
using the preference in an important session.
