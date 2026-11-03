# Wave sound browser - 3 November 2026

On SYNTH, click BROWSE SOUNDS at the top of oscillator A or B.

- Basic Waves: Sine, Saw, Square, Triangle, Pulse 25%, Pulse 12.5%.
- Wavetables: Basic (sine/saw/square morph), Hollow, Glass and Vowel. Position scans their frames. Import WAV still loads periodic tables using the selected cycle size.
- My Samples: local recorded synth notes, grouped by instrument and original patch. Select a group on the left and a recording on the right. Groups/Sounds arrows browse further pages.

Loading a sound automatically selects Wavetable / Sample mode. Reference Saw remains a separate synthesis mode. Basic waves and imported tables show their name above the waveform. Selecting a wavetable bank from the browser clears the previous custom asset; the small bank selector alone does not replace an imported asset.

Recorded samples follow keyboard pitch and play once, preserving the recording's development over time. They are not periodic wavetables and are not automatically looped. The four buttons under their waveform adjust Root by -12, -1, +1, +12 semitones. Raising Root makes playback lower at the same key. Root estimates can need correction on complex/detuned recordings. These are individually selected recordings, not automatic multisample key zones. Position/Bend/phase controls do not transform recorded playback; tune/unison/detune, envelopes, filter and effects remain available. Stereo input is downmixed to mono.

Sample/table selection and Root edits restore plugin state and can interrupt held notes. Make these edits between notes. Loaded audio is embedded in saved native patches/projects; loading an older patch clears samples. Maximum recording length 12 seconds and WAV size 16 MB; PCM8/16/24/32 and float32 WAV are supported. Playback uses prepared low-pass octave levels and interpolation for pitched playback, plus short edge fades. No sample file reads occur in the audio callback.

My Samples reads a user-local JSON catalogue at LOCALAPPDATA/Lemon Studio/Lemon Wave/library.json (HOME fallback on other platforms). Each entry has group, name, path, and root (MIDI pitch, fractional allowed). This Windows installation indexes 260 recordings in 38 groups from Mini-Moog Multis and 800DV 28 multi patches. These names are library labels, not independently verified recording provenance. The original files remain on E:. The catalogue and third-party recordings are not included in application source or public factory distributions. Only the user's selected audio is embedded in their saved sound.

Checks: 25 Wave tests and 29 shared-editor tests passed (one preexisting shared test ignored); actual CLAP/VST3 synthetic recorded-note + table/state recall; all 260 local recordings decoded and rendered finite/non-silent, with sampled state round trips. A captured prior 109-control Glass Tide patch renders bit-identically. Offscreen browser/main editor layouts reviewed at 1x with bounds checked at 1x/1.5x. These checks do not certify pitch estimates, recording provenance, live interaction or listening quality. User hands-on listening and visual acceptance remain.
