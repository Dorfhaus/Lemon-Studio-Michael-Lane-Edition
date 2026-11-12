# ML Studio User Guide

## Getting started
### Make your first song
beat beginner first project getting started
Guide: rack-steps
1. Use File > New to start with your default template. Set the template in General settings and choose an output device in Options > Audio settings.
2. Add an instrument with the Add menu or the plugin picker. Open the Channel Rack and click steps on a drum channel to make a beat.
3. Right-click an instrument's row and open Piano Roll to draw notes. Play in Pattern mode to hear the current pattern.
4. Open the Playlist, choose the pattern and click a track to place it. Switch to Song mode to hear the arrangement.
5. Save the project, then use File > Export to create an audio file.

### Understand the main windows
workspace browser mixer channel rack piano roll playlist arrangement
The Channel Rack holds instruments, samples and step patterns. Piano Roll edits notes for one channel in the current pattern. The Playlist arranges patterns, recordings and automation over time.
The Mixer controls audio level, pan, effects and routing. The Browser finds samples, projects and presets. Use the toolbar shortcuts or the View menu to show these windows.

### Set tempo and play the right thing
bpm metronome transport playback loop song pattern space
Guide: transport
Choose PAT to hear the current pattern or SONG to hear the Playlist. Press Space to start or stop, or use the toolbar transport.
Adjust BPM in the toolbar. Enable the metronome to hear the beat and click in the Playlist ruler to set the playhead.

### Save, reopen and recover a project
lmn flp backup save as autosave portable missing files
Use File > Save to keep working or Save As to make a separate version. Keep the project's audio files and installed plugins available when reopening it.
Use the File menu's ZIP project export when you need a portable project and its audio. Third-party plugins must be installed on the destination machine.
If a project crashes or a save is lost, check the available backups before overwriting the original project.

### Import a project from FL Studio
flp compatibility import report missing sample plugin migrate
Open the FLP through File > Open. Review the Project Import Report if it lists missing files or unresolved links, and point the project to the original samples where needed.
Install compatible plugin formats before opening a project that uses third-party plugins. Check playback, automation, routing and settings against the original, then save a separate working copy.

### Undo and redo
history revert mistake ctrl z
Use Edit > Undo or Undo history to step back through project edits. Redo restores edits that were undone if you have not made a new conflicting edit.
Playlist pitch and timing editors offer separate Undo/Redo controls for the active clip. Ctrl+Z and Ctrl+Y use project history. Each completed gesture, reset, or clip-history action is one project undo step.

## Audio and recording
### Set up an audio device
sound output speakers headphones interface asio wasapi coreaudio buffer latency
Guide: audio-device
Open Options > Audio settings. Choose an output device and verify that the device's sample rate and channels match your interface settings.
Start with a buffer that plays reliably. Lower it if monitoring feels delayed, or raise it if playback crackles or the underrun count rises. On Windows, select your interface's ASIO driver if available.

### Record a microphone or instrument
record vocals guitar interface input arm monitor take
Guide: record
1. Select an empty Mixer insert and choose your interface input in its input selector.
2. Check that the meter moves and set a safe input level on the interface. Arm disk recording on the insert.
3. Enable recording and choose audio recording if prompted. Start playback, perform the take, then stop and check the recorded clip.
Use headphones when monitoring a microphone. If the sound is doubled, check whether both interface direct monitoring and software monitoring are active.

### Prevent feedback and choose a recording input
record dry mono stereo left right input monitoring bleed
Guide: mixer-input
Use a mono input for a single microphone or instrument, and a stereo pair for a stereo source. Verify the input selector on the Mixer track before arming it.
Listen on headphones and disable any unintended second monitoring path. Record a short test, then play it back to check the level, channel balance and timing.

### Record notes from a MIDI keyboard
midi controller record piano keyboard velocity
Guide: midi
Enable the device in Options > MIDI settings. Select the instrument channel, test a few notes, then enable recording and select note recording if prompted.
Start playback in the desired mode and perform. Open the pattern's Piano Roll to correct timing and velocity.

### Fix delay, crackles or no sound
latency underruns cpu stutter dropout silence device troubleshooting
Guide: audio-buffer
If nothing plays, check the output device, master level, muted tracks, song/pattern mode and routing to Master. If only one channel is silent, check its sample or plugin and its Mixer destination.
For dropouts, raise the buffer and temporarily bypass expensive effects to isolate the cause. Freeze or bounce finished parts if needed.
Plugin delay compensation aligns paths using delays reported by plugins. Input monitoring still has hardware and processing latency, so use a small stable buffer or direct monitoring when recording.

### Set up a keyboard or mixer controller
MIDI controller Komplete Kontrol KeyLab Arturia Behringer X-Touch nanoKONTROL Akai Novation Launchkey M-Audio Nektar Mackie MCU
Guide: midi
Open Settings > MIDI, select an input device, enable it and choose its Controller type. This choice is saved separately for each device in your profile.
For musical keys and pads, choose Generic MIDI / MIDI Learn. Use the input role to separate an instrument port from a dedicated DAW controls port. Use Instruments + controls when a keyboard has one shared port. Right-click a software knob, choose Link to controller, then move the hardware control. For an endless encoder, choose the Relative mode that matches its MIDI format.
Under DAW MIDI Learn, select an action and press a hardware button on the selected input. Single press mode suits CC buttons that send no release. Save Learn preset exports the actions and button behavior to a file. Load Learn preset applies that file to the selected input and replaces its learned actions.
For a controller with Mackie Control Universal support, set its hardware mode to MCU, select its dedicated DAW input in ML Studio, and choose Mackie Control or the matching named MCU profile. Profiles include Kontrol, KeyLab, X-Touch, Oxygen Pro, nanoKONTROL Studio, SSL UF1/UF8 and iCON QCon. Use the hardware's matching Mackie mode. Select the dedicated DAW output in the output list, then click Pair selected output. Hardware with compatible motor faders, button LEDs, pan rings or track-name displays receives feedback over that output.
For Launchkey MK4, select its native profile on the DAW input and pair the DAW output to enter DAW mode. Its DAW pads select and mute Mixer inserts, and its mixer encoders and available faders control pan and level. Keep the instrument port available for playing keys. The nanoKONTROL2 factory profile uses CC mode. For Akai and other controllers configured to transmit MIDI Machine Control, choose the MMC profile to receive transport commands alongside notes.
If notes work but transport does not, turn on the MIDI Monitor and check whether pressing a button generates a message on the selected DAW port. If an input shows in use, close the other application using that MIDI port. Use Refresh device list after connecting hardware.
Hide the setup controls under Settings > General > Workspace Visibility > Input customization > MIDI controller setup. Saved modes and mappings continue to operate when the controls are hidden.

### Use MIDI, typing keyboard and step entry
step edit typing keyboard controllers performance
Guide: typing
Enable Typing keyboard to piano in the toolbar or press Ctrl+T. ML Studio remembers the switch in your profile when you quit and reopen. Notes go to the selected instrument. Use numpad + and - to shift the keyboard by octaves.
Right-click the toolbar button to choose Key and Scale or Follow Piano Roll typing snap. For a selected scale, the letter rows play successive scale notes and each row starts one octave above the previous row. On a German QWERTZ keyboard, the bottom row runs from Y through minus and the home row from A through Ä. The corresponding QWERTY ranges are Z through slash and A through apostrophe. Choose Chromatic for the staggered piano-key layout with black keys on the row above.
Turn typing off to use the same unmodified keys for editing shortcuts. Text fields take text normally and modified keyboard shortcuts keep their own bindings.
Use Step Edit with playback stopped to enter notes one step at a time. Check the selected channel, current position and snap before entering notes.
Use MIDI settings to configure enabled devices and controller inputs. Save a small test project before setting up a complex controller mapping.

### Enter notes one step at a time
step edit step recording midi chord input
Guide: step
Stop playback and enable Step Edit in the toolbar. Select an instrument and set the playhead and snap.
Play a note or chord from a MIDI or typing keyboard. Each entry advances by one snap step. Turn Step Edit off before returning to live playing.

## Channel Rack and Piano Roll
### Add, replace and route an instrument
channel rack generator vst synth sampler insert instrument
Guide: rack-route
Use Add or the plugin picker to create an instrument channel. To insert another instrument from the rack, right-click a row and choose Insert instrument.
Use the rack channel's Mixer track assignment to send its sound to an insert. Name the channel and insert so they are easy to find again.

### Program steps and patterns
drum sequencer swing kick snare hi hat pattern
Guide: rack-steps
Choose or create a pattern. Click a step to turn it on or off, and play in PAT mode to audition it.
Keep variations in separate patterns so they can be arranged independently. Adjust swing to change the feel and use per-channel loop lengths where available.

### Draw, move and edit notes
piano roll pencil brush select resize velocity note properties
Guide: piano-notes
Open a channel's Piano Roll. Use Draw to add notes, drag a note to change time or pitch, and drag its edge to resize it.
Use Select to work on several notes. Right-click empty space with Draw or Brush to clear selection. Use the lower lane to edit velocity or other note values.
Double-click a note to inspect its properties. Set snap according to the grid you want before moving or resizing notes.

### Use Glide Notes and slide pitches
glide slide portamento normal note type
Guide: piano-notes
Choose Glide Note in note properties to place a glide. Select an existing normal or glide note to pick up that placement type.
Place a glide against a sounding note on a compatible instrument and adjust its length to control the transition. Check the result on the instrument you are using, since third-party plugins can handle note slides differently.

### Zoom, navigate and place patterns
piano roll alt wheel vertical zoom place pattern playlist toolbar
Guide: place-pattern
Use Alt + wheel over Piano Roll to resize note rows vertically. Use the horizontal overview to move through the pattern and the vertical controls to bring the notes into view.
Use Place Pattern to Playlist in the Piano Roll toolbar to put the current pattern at the Playlist playhead. It finds the first track with space for the clip, opens the Playlist and highlights the result.

### Quantize and shape notes
quantize timing strum arpeggio flam chop glue legato flip randomize
Guide: piano-tools
Select the notes to process, then open the relevant command in the Piano Roll Tools menu. Quantize adjusts timing toward a grid. Strum offsets chord notes, and Arpeggiate turns chord notes into a repeating sequence.
Use Chop to split notes, Glue to join suitable notes and Legato to extend notes toward the following ones. Preview the tool, apply it, or cancel to keep the original.

### Edit per-note expression with MPE
mpe pitch bend pressure slide timbre controller lanes
Guide: piano-mpe
Open MPE / Note expression in the Piano Roll menu. Enable it for the instrument and show the expression lanes. Use a compatible instrument or patch and match its pitch bend range.
Select one note to edit its expression curves. For a hardware controller, also match its zone and member channels to the instrument settings.

### Work with chord and scale tools
chord workspace harmony key scale scale highlighting chord progressions
Guide: chords
Open Chord Workspace from the Piano Roll shortcut where shown. Choose a scale or chord and audition it in the current instrument before placing notes.
Use the Piano Roll's scale guides as a reference. If the shortcut is hidden, look under Settings > Visibility > Piano Roll.

### Resize lanes and use the pattern overview
piano roll minimap scroll zoom velocity lane splitter
Guide: piano-overview
Drag the splitter above the lower event lane to give it more space. Double-click the splitter to hide or show the lane.
Use the overview scrollbar to move through a long pattern. Zoom in for small edits and out to check the whole phrase.

### Edit velocity and note expression
velocity pan release note properties ghost notes scale key event
Guide: piano-expression
Use the Piano Roll lower lane to change velocity and other available note properties. Select notes first when you only want to change part of a phrase.
Use ghost notes for reference to other channels and key or scale guides to see usable pitches. Drag the lane splitter to make the editing area taller.

## Playlist and audio editing
### Build an arrangement
playlist clips tracks markers patterns drag arrangement
Guide: clip-picker
Choose a pattern, audio clip or automation source, then place it on the Playlist. Drag a clip to move it and use its edges to trim it. Turn on snap for grid-aligned placement.
Use the Playlist tools to draw, paint, split, mute or select clips. Right-click the ruler for marker commands and use the Arrangement selector to manage versions of the song.

### Use the clips picker and loop previews
clip picker pattern picker preview audition loop stop playlist left panel
Guide: clip-picker
In the Playlist menu, enable Show clip picker panel. Its tabs separate patterns, audio and automation. Click a row to choose what to place.
Click the Play icon on the right to loop an audio clip or supported pattern. Click Stop on the same row or start another preview. For a pattern that relies on a hosted third-party instrument, render it to audio before auditioning it here.
Show or hide the preview buttons in Settings > Visibility > Playlist.

### Import, trim and fade audio
sample drag wav mp3 audio clip fades crop stretch
Guide: stretch
Drag an audio file from the Browser or your file manager into the Playlist. Use the clip edges to trim, the fade handles to ease in or out, and the clip menu for source commands.
Use Stretch mode when dragging an edge should change the clip's timing. Turn it off to return to trimming. Use the audio channel's settings for pitch or stretching method.

### Make a clip independent
make unique clone duplicate shared audio channel route
Use Make unique in the clip menu before changing a source that several clips share. Edits to a shared source can affect other instances.
After making it unique, rename the new source and make the change there. Use a unique audio channel when two instances need separate Mixer destinations.

### Set up an alternate arrangement
arrangement duplicate playlist version marker sections
Guide: arrangement
Use the Arrangement selector to manage alternate versions of the Playlist. Keep a separate arrangement for an alternate edit or song structure.
Review which arrangement is active before recording, bouncing or exporting. Patterns and channels can still be shared between arrangements.

### Edit audio timing with warp markers
warp transient flex time guitar rhythm audio timing detect quantize snap
Guide: warp
1. Click the timing-pin button on the Playlist track, or open Edit audio timing from an audio clip's menu. The editor detects transients in the clip.
2. Click a detected line to create a timing marker. Drag anywhere along a movable marker line to adjust timing. Use Snap to follow the selected grid.
3. Choose Select and drag across several points. Shift-drag adds more. Use Quantize to move the selection toward the grid, and choose Grid and Strength before quantizing.
4. Use Ctrl + wheel to zoom time, Alt + wheel to scale the waveform display, and Fit to see the whole clip. Changes render automatically after you release a drag or finish a command. Close keeps completed edits; Escape during a drag cancels that gesture. Updating means the last successful audio is still playing until the new render is ready.

### Fit a loop or use its detected tempo
detect sample tempo loop bpm stretch fit make it fit project tempo
Open an audio clip's menu and choose Detect sample tempo. The dialog displays the detected BPM. Choose Yes to set the project tempo to that value, or Cancel to keep the current tempo. The tempo change supports project Undo.
To keep the project tempo and fit the clip to it, use the Stretch / fit options. Compare automatic fitting against half-time, double-time, a known source BPM, or a specified number of bars or beats. Check both the start and end of the loop against the grid.

### Separate stems from a clip
stem separation vocals drums bass other model download retry
Open a Playlist audio clip's menu and choose Stem separation. Select the stems you want to create. On first use, confirm the download of the optional engine and model. Setup requires an internet connection and several gigabytes of space. Processing runs locally after setup.
The finished stems are separate audio files and Playlist tracks. Keep the project data folder with the session when sharing it. If setup fails, choose Stem separation again to retry. Details are written to setup.log in the Stem separation folder.

### Align multiple performances
vocal align vocalign dub guide timing takes imported vocals instruments
Select at least two audio clips, including the guide, then right-click the intended guide and choose Vocal Align to this clip.
Click Analyze and compare the guide, original and aligned views. Adjust Tightness and listen to the preview before applying. Imported clips work as well as recorded takes. Matching phrasing and comparable source sections give better results.
Use a guide that already has the timing you want. Check sustained, quiet or poorly matched sections by ear.

### Tune vocals in the Playlist
pitch correction vocal tuning melody notes formant tune
Guide: vocal
Click the microphone icon on the track with the vocal clip to expand the tuning editor. Let it detect notes, then use the editing tools to select notes and adjust pitch or timing where supported.
Pitch edits and timing share the original recording and remain independently editable. Use Undo/Redo pitch edits or Undo/Redo timing for the active clip, or Ctrl+Z/Ctrl+Y for project history. Copies and slices inherit edits and then become independent. Undo timing on a slice keeps its timeline boundaries fixed. Up to 64 steps per layer are saved with the project. Closing an editor keeps completed edits; Escape cancels the current drag. Workspace Visibility can hide editor and clip-history controls without changing playback.

Fresh playback, export, and consolidation wait for current audio edits. A failed update retains previous audio and offers Retry. Existing rendered projects remain playable; older baked edits can only be recovered when their original sources and metadata are available.

### Comp and organize takes
take lanes comping record vocals alternatives
Guide: comp
Right-click the Playlist track and choose Create take lanes / comp when available. Keep alternate performances on lanes under the same parent.
Use the comp tool to choose sections for the main take. Audition transitions and adjust boundaries or fades before bouncing a final take.

### Freeze or bounce a track
render new track replace cpu bounce audio freeze unfreeze
Guide: track-menu
Right-click a Playlist track header. Freeze track renders the track to save CPU and allows you to return to the editable sources with Unfreeze.
Use Bounce audio > Bounce audio in new track to keep the source on its own track, or Bounce and replace track to work with the rendered clip in place. Save first and compare the result, especially when effects and routing are important.

### Arrange with groups, markers and zoom
playlist overview minimap group clips ruler zoom scroll
Guide: playlist-overview
Use the overview above the Playlist to navigate. Drag the thumb to move the view and drag its ends to change the horizontal range.
Group related clips when they should move together. Use time markers to label sections and, where applicable, set start, loop or skip behavior. Use the Playlist snap choice before moving an entire section.

## Mixer, routing and effects
### Set levels, pan and routing
mixer volume pan master signal flow insert route
Guide: mixer-level
Route a Channel Rack instrument or audio clip to a named Mixer insert. Use its fader and pan controls to balance it, and check its meter.
Right-click a Mixer channel to rename it or choose Change color > Custom color. The custom swatches accept six-digit hex colors. Settings > General > Input offers optional double-click reset for faders and knobs and opening main menus on hover. Both start off.
Effects in the insert's slots run in order. Check that the track reaches Master through a direct route or a bus. Avoid clipping the Master if you are exporting a mix.

### Send a Mixer track to an interface output
hardware output cue headphone mix out extra stereo pair asio
Open a Mixer track and click Hardware out below its EQ. Choose an additional pair exposed by the active audio device, such as Outputs 3 / 4, or choose Off. The output gets the track's post-fader signal alongside its normal sends. Master plays through the device's main outputs.
The choice is saved with the project for that device and affects live playback and monitoring. For a cue mix, send the desired tracks to a bus and set that bus's hardware output. A stereo-only device offers Off. Settings > Visibility > Mixer can hide the selector while retaining the assignment.

### Route audio with the miniature Mixer picker
route picker send to mixer track track number audio clip channel rack
Guide: rack-route
Click the Channel Rack's Mixer track number or the channel window's TRACK box to open the miniature Mixer picker. For a Playlist audio clip, use Send to mixer track in its menu.
Search by insert name or number and click a strip to assign the route. Press Escape to cancel. Clips sharing the same source also share its route, so use Make unique first when they need separate assignments.

### Create sends, buses and sidechains
send triangle reverb aux bus route sidechain compressor ducking
Guide: mixer-send
Select the source Mixer track. Click the routing triangle beneath the destination track to create a send, or right-click it for routing choices. Use the send level to control the amount.
For parallel effects, keep the dry route and send some signal to a reverb or delay return. For a bus, use Route to this track only if the bus should carry the full signal.
For sidechain ducking, create a sidechain route and choose that source in the effect slot's Sidechain input menu. Check the plugin's own sidechain controls.

### Choose the Mixer layout and built-in strip
full mixer custom strip traditional gate tame comp drive low cut high mid low
Guide: mixer-layout
Use the Mixer view buttons to choose Full or Traditional. In Full Mixer, right-click an insert and choose Channel layout > Custom strip to see its built-in EQ, Low Cut, Gate, Drive, Comp and Tame. Vox adds vocal controls.
Standard (FL-style) emphasizes the meter. The overall Mixer view and each insert's channel layout are separate choices.

### Attach the Mixer to the Playlist
attach detach integrated mixer inspector instrument track recording bus arrangement
Guide: mixer-layout
Click Attach to Playlist at the top left of the Mixer. The Playlist opens at its current size, with source lanes in mixer order and a compact inspector for the selected channel. Settings > Visibility > Playlist has separate controls for the inspector and the Mixer’s Attach to Playlist button. Buses and Master stay in the inspector instead of occupying recording lanes. The separate Mixer stays open. Switch between Mixer and Playlist with the workspace buttons when their windows overlap. Both views control the same channels and effects.
The larger red button beside each source lane's mute/solo light arms that track. Instrument lanes arm MIDI notes; audio lanes use their chosen input. Right-click the red button to choose an audio input, then press transport Play to record. Selecting or arming a lane does not start recording by itself. Vocal tuning and Time Warper remain on each lane.
Attached lanes use their mixer channel colors. Every source lane has a Piano Roll icon: left-click opens its instrument's Piano Roll (an empty lane gets an empty sampler); right-click opens the instrument menu for that mixer channel. Enable Keyboard right-click opens Plugin Picker in that menu or in Playlist Options for direct access to the picker. The choice applies to attached and ordinary Playlist track controls. The inspector lists outgoing sends in their destination colors below the effects. Adjust a send with its knob and use the arrow to inspect the destination. A mixer track receiving audio from another insert is treated as a bus. Separators mark visual groups.
On enables or mutes the entire mixer channel; FX bypasses only its effects. The Playlist lane mute controls the clips on that lane, so it can differ from muting the mixer channel and everything routed through it.
The attached inspector defaults to the Traditional bottom-fader layout for every Mixer view. Right-click the inspector button or open Mixer Options > Attached inspector layout > Inspector style to choose Full channel strip. The choice is saved in your profile. Both include the same editable parametric EQ used by the Mixer, with Open EQ for its full editor. Changing views never creates another EQ instance or changes channel processing. Use the small panel button beside + above the track headers to hide or show the inspector. Drag the grip below EQ, Insert Effects, or Sends to resize it. The fader and strip fill the remaining height. Short windows can temporarily reduce the space given to the upper sections. Click the selected channel name to choose another channel, including Master. Click the Insert Effects or Sends heading to collapse it, and click again to restore its size. Drag the scrollbar thumb or wheel over either list when it overflows. Right-click the inspector button or open Mixer Options > Attached inspector layout to change section order, show controls, or reset the layout. Sizes, order, and collapsed sections save automatically for each mixer view and apply to every channel. Settings > Visibility > Attached mixer inspector has the same show/hide controls. Theme Editor > Attached mixer inspector controls the background, grips, scroll thumbs, and Traditional fader color. Send names use the destination colors. Clicking a send name keeps the current channel selected; use its arrow to inspect that destination. The native Full Mixer VU switches between VU and peak when clicked; the Traditional inspector uses a stereo peak meter. Both the layout choice and visibility are saved per profile. The Mixer EQ visibility setting also controls the inspector EQ.
Use the + above the track headers to insert an instrument. This creates its mixer channel and routes the instrument to it, as one Undo action. Existing routed instruments appear automatically. Mixed-instrument patterns and mixed-output rows remain ordinary arrangement lanes; attaching does not split or reroute them.
Detach from Playlist restores the previous Playlist row order while retaining new recordings and edits. A project created in the attached view keeps its arrangement when detached. Saving remembers the attached layout and new lane assignments. Settings > Visibility > Playlist can hide the inspector or track record buttons without changing routing, recordings or arming.
To use these track controls without attaching the Mixer, enable Settings > Visibility > Playlist > Playlist record and instrument buttons (off by default). The red button arms an instrument for notes or opens the audio input selector; right-click it to choose an audio input explicitly. Press transport Play to record. The piano button opens the track's Piano Roll, creating an empty sampler if needed; right-click it to choose an instrument. A new instrument or audio recording track gets its own mixer channel when needed. Cancelling a picker creates nothing. Track bindings survive save/reopen and hiding the buttons; instrument insertion and recording-track creation support project Undo/Redo.

### Lock a Mixer channel
lock padlock fader protect routing effects
Guide: mixer-lock
Click the padlock on a Mixer channel to protect its configuration from accidental manual changes. Click it again when you want to edit the channel.
Playback and existing automation continue while it is locked. If the icon is missing, look under Settings > Visibility > Mixer.

### Rename, color and group mixer channels
rename mixer channel custom color hex separator channels
Right-click an insert and choose Rename or Change color. In the color menu, choose a swatch or open Custom color to enter a six-digit hex color. Attached Playlist lanes use their corresponding mixer channel colors.
Use separators to mark mixer sections visually. The mixer shortcuts scroll to the corresponding sections. Use buses to sum audio and sends to feed parallel effects.

### Add, reorder and bypass effects
plugin picker effects slot vst eq insert replace bypass
Guide: effect-slot
Click an empty effect slot and choose an effect. Click a loaded slot to open its controls. Use the slot menu to replace, move, copy or remove an effect where offered.
Use the slot enable switch to compare the effect with the dry signal, and check the channel level after changes that add gain.

### Save and reuse effect chains
effect chains plugin picker save load favorites import export
Guide: effect-chain
Open the effect picker and choose Effect Chains. Right-click a Mixer insert and choose Save effect chain to capture its ordered effects and settings. To build one in the picker, Ctrl-click the effects you want and choose Save as chain.
Click a chain to select it, Ctrl/Cmd-click to toggle individual chains, or Shift-click to select a range. Click Choose to load them onto the selected Mixer insert, or drag a selected chain to the Mixer insert you want. Dragging a selected card carries the whole selection. Chains load in the order shown, with their effects in order, into consecutive free slots. One Undo removes the whole insertion.
Use the available rename, duplicate, favorite, import and export controls to organize and share chains. Recipients need the plugins used by the chain.

### Mix with Lemon Parametric EQ
eq bell shelf low pass high pass dynamic spectral tame deharsh listen solo
Guide: effect-slot
Add Lemon Parametric EQ to an effect slot. Select a band and adjust frequency, gain and Q. Use the band type controls for a bell, shelf or cut.
Use the band card to bypass or listen to an individual band. Open the band's Dynamic or Spectral controls for level-dependent changes and audition before committing to deep cuts.
The Mixer mini EQ and an EQ plugin in a slot are separate processors. Check which one is enabled when troubleshooting a tonal change.

### Use dynamics, saturation and time effects
limiter compressor noise gate clipping reverb delay drive transient chorus
Guide: effect-slot
Use a compressor to reduce level changes, a gate to control quiet sections, and a limiter or clipper to control peaks. Match the output level before comparing an effect with bypass.
Use drive and saturation for harmonics, Transient Processor to shape attack and sustain, and chorus or stereo effects to adjust width. Put reverb or delay on a send when several tracks should share it.

### Correct headphone monitoring
headphones phones autoeq monitor correction profile
Guide: phones
Click the Master strip's headphone icon, search for the exact model, and choose the matching profile. Once downloaded, the profile can be used offline.
Use the PHONES switch to compare or bypass correction. This processing applies to the listening output and is excluded from audio exports, bounce and recording.

### Master a mix with Master Assistant
master assistant loudness reference headphone correction
Guide: master-assistant
Start with a balanced mix and leave room at the Master output. Open Master Assistant from the Master strip and let it analyze representative playback before comparing its settings.
Set a target that fits the delivery format and check that the result is not clipping. Use Detailed controls to inspect or adjust EQ, multiband compression, stereo width and limiting. Compare at similar loudness before exporting.

## Plugins, instruments and presets
### Find installed plugins
plugin scan vst vst3 vst2 clap plugin manager add folders missing plugin
Guide: plugin-scan
Open Settings > Plugins. Add the folders where your plugins are installed, then click Find new plugins. Use Rescan all when the inventory needs rebuilding.
Use a plugin format and architecture that match your system. After a scan, find instruments in Add or the rack picker, and effects in a Mixer slot's picker.

### Search, favorite and manage the plugin picker
f8 plugin picker favorites blacklist installed filter
Open the picker from the rack or an effect slot to search for the right type of plugin. F8 opens the general picker. Check the target before choosing, since an effect goes to a Mixer insert and an instrument goes to the rack.
Use the installed and favorites filters to narrow the list. If a plugin is missing, check its blacklist status and scan results in Settings > Plugins.

### Recover a missing plugin or preset
missing plugin wrong format vst3 clap preset blacklisted
Guide: plugin-scan
Check the plugin name and format the project expects. Install that version when available, add its folder to the plugin manager, and run a scan.
If an effect slot opens with a missing plugin, preserve a copy of the project before replacing the slot. Compare the replacement's sound and reconnect automation where needed.

### Load and save presets
preset vstpreset fxp fxb ospreset patch bank browse previous next
Use the arrows or preset menu in the plugin wrapper's title bar to step through presets. The menu offers browsing and saving your own presets where supported.
Drop a compatible preset onto the intended plugin window or slot to load it there. Use Save preset as to preserve an edited version before switching to another preset.

### Play and shape samples in Lemon Sampler
sample sampler root note loop start end envelopes reverse pitch
Load a sample into a sampler channel. Set the start and end, pitch or root note, and loop behavior to suit the sound. Open Piano Roll and play notes at different pitches to check it.
Use the channel's envelopes and filters to shape playback. Keep long files on disk when supported if you need to reduce memory use.

### Design sounds with Lemon Wave
synth oscillator wavetable modulation lfo envelope unassign
Add Lemon Wave as an instrument and choose a preset as a starting point. Change oscillator and filter settings while playing a few notes, then shape the sound with the envelope.
Assign an LFO or envelope to a control to add movement. Right-click a modulated control to manage or remove its assignment. Save a new preset before changing patches.

### Design sounds with ML Plasma
ml plasma gridline wavetable synthesizer oscillator modulation preset
Add ML Plasma from the instrument picker. Use its oscillator and wavetable controls to shape the source, then use the filter, envelopes, and modulation to shape the movement and tone.
Save the project to keep the synth state. Use the instrument's preset controls when you want to reuse a sound in another project.

### Use Lemon Trio and Citrus
trio synthesizer oscillator citrus sytrus fm presets
Use Lemon Trio when you want to layer three oscillators. Start with one oscillator, then add detuned or octave layers and adjust the filter and envelope.
In Citrus, start from a preset and change operators or their routing in small steps. Save a copy before making large patch changes.

### Slice audio with Lemon Chop
chop slices sample beat slicer pads
Load an audio sample into Lemon Chop and check its detected slices. Adjust slice boundaries before playing them from pads or MIDI notes.
Use the slices to build a pattern in Piano Roll. Compare playback with the original loop to check the trigger positions and tempo.

### Use an ARA audio editor
ara melodyne plugin audio edit compatibility
Install a compatible ARA editor for your platform. Open it through the audio clip's ARA action where available and let it read the clip.
Complete pitch or timing edits in the editor, then save the project. Some clip operations, including inline warp, require rendering the ARA edit first.

## Automation and control
### Create an automation clip
automation parameter ramp volume filter right click last tweaked
Right-click a supported knob or plugin parameter and choose Create automation clip. For a plugin parameter that does not expose the command directly, use Tools > Last tweaked or Browse parameters to find it.
Place the automation clip in the Playlist and edit its points. Play the affected section and watch the target control to confirm the link.

### Edit curves and manage targets
automation points tension curve link move channel track
Add points on the automation line and drag them to change value and time. Use point menus or curve controls for the shape between points.
Moving the clip to another Playlist row changes where it is displayed. Check its parameter link when you want to automate a different control. Use a separate clip or link for an independent target.

### Map a MIDI knob or controller
link to controller midi learn multilink hardware
Guide: midi
Enable your controller in MIDI settings. Right-click a supported parameter and choose Link to controller, then move the hardware control when prompted.
Test the full range and check whether the link should apply to the current project or to a reusable control setup.

### Customize mouse and keyboard behavior
controls input bindings modifier shortcut marquee right drag wheel
Guide: controls
Open Settings > Controls. Choose an editor context, event and modifiers, then search for an action. Use Capture to record a key and try it in that editor.
A more specific context can override a broader one. Use Restore event to return to inherited behavior, or Do nothing to suppress the gesture.
Clicks and drags can have separate assignments. Set a left click to insert a note and left drag to select a region where those actions are available.

### Restore a shortcut that stopped working
shortcut override keyboard binding context hotkey typing keyboard
Guide: controls
Check which editor is active and whether a text field or dialog is taking keys. Turn off Typing keyboard to piano if a note key should be used for editing.
Open Settings > Controls and look for a matching event in the editor and its parent contexts. Use Restore event to remove an override and return to inherited behavior.

### Build action sequences and toolbar buttons
macro script lsa saved actions custom toolbar import export
Guide: controls
In Settings > Controls, use Append to build a sequence of compatible editing actions. Save a name to reuse it and choose Pin as toolbar button to make it accessible from the toolbar.
Use Export to back up controls. Import accepts a controls JSON file or a portable .lsa action script. Portable scripts run supported ML Studio actions and do not execute system commands.

## Themes, workspace and sharing
### Browse or search the User Guide
offline help manual collapsible sections topics find
Open Help > Search User Guide. Click a section title to expand or collapse its topics. Open an article and choose Back to return to the same list position.
Type a feature, shortcut, or task in the search field to search every section. Enter opens the first match. Where available, Show me where points to the matching control in the workspace. Esc returns to the list, clears the search, or closes the guide.

### Show or hide interface features
workspace visibility remove button toolbar settings tabs clip picker
Guide: visibility
Open Settings > General > Workspace Visibility, or choose the Visibility tab. Expand a section or search for the control, then toggle it.
These choices save to your profile and follow you between projects. Use the Playlist menu's Show clip picker panel switch to open or close the picker itself.
Restore default visibility brings back the default show/hide choices. Existing toolbar layout settings still apply.

### Move the Settings window
settings options menu move drag position title header floating
Drag the Settings window by its title bar to move it within the workspace. File, Options and other dropdown menus stay anchored to their menu buttons; they cannot be dragged.

### Customize menus
menu layout arrange submenu hide items piano roll channel rack mixer
Guide: menus
Open Settings > Menus and choose the menu to customize. Show or hide entries and use the structure controls to reorder them or place them in submenus.
Menu layouts are saved to your profile. Restore the menu's defaults if a command becomes hard to find.

### Show a window or restore your workspace
window missing channel rack mixer browser playlist layout
Use View or the toolbar to reopen the missing window. If its shortcut is hidden, use Settings > Visibility to show it.
A project can restore saved window positions. If a window is behind another one, bring it forward with View and move or resize it before saving the layout.

### Change theme, icons and cursors
theme vintage fl 2005 theme editor cursor pack toolbar colors
Guide: theme
Open Settings > Theme and choose a skin or theme. Use the theme controls to adjust colors, icons, cursor packs and Mixer appearance where available.
Use the Theme Editor to preview changes and apply or cancel them. Export the theme if you want to share or reuse it on another installation.

### Browse samples and release history
browser folders sample preview presets plugin downloads changelog
Guide: browser
Use the Browser tabs to find samples, presets, projects and plugin downloads. Add your own folders in Settings > File to keep libraries easy to find.
Use the sample preview controls to audition files before placing them. Open Help > Change Log to browse or search the bundled release history offline.

### Export a finished song, stems or MIDI
export render wav mp3 stems multitrack midi
Guide: export
Choose Song mode and confirm the arrangement. Use File > Export to open the export form.
Export audio opens in Easy mode with 24-bit WAV and 320 kbps MP3 options. Pick either or both. Easy uses stereo and the standard render settings. The Advanced tab has additional formats, source range, quality, effects, and split mixer tracks. Advanced export choices save separately.
Use WAV for a full-quality mix, MP3 when a smaller listening copy is useful, and MIDI for notes and timing. Use split Mixer tracks when you need separate audio stems.
Listen to the exported file and verify its start, end, levels and effects before sending it.

### Send a project with its audio files
share project pack zipped audio samples collaborate
Guide: export
Use File's ZIP project export to gather the project and its referenced audio files. Open a copy of the archive to check that it contains what the recipient needs.
Tell the recipient which third-party plugins and formats the project uses. For a collaborator without those plugins, include audio renders of important tracks.

### Collaborate on a project
live collaboration host join invite network transport follow recovery
Choose File > Collaborate. Save the project, stop playback, and host on a reachable address. Share the invitation privately. Your collaborator opens the same dialog and joins with that invitation. Both users need compatible ML Studio builds and their own copies of the plugins used in the project.
Reopen Collaborate for session status, playback following, conflict resolution, recovery files, and End or Leave. Each computer renders its own audio. Use a LAN or shared private VPN for direct connections.

### Connect an external assistant with MCP
AI assistant external MCP tools working orb approval
Open AI assistant and enable AI control for the session. Save the setup to the Desktop and configure it in a compatible MCP client. Approve the named client in ML Studio when it connects. Disable AI control in ML Studio to revoke access.
The blue Working orb shows when an approved MCP command is running. Hide it under Settings > Visibility > Editor feedback > MCP activity orb.

### Report a problem and check for updates
report bug build update restart troubleshooting support
Use Report a Bug and include your build, operating system, exact steps, expected result and actual result. A small project or screenshot helps when it can be shared safely.
Use the update control to check for a signed release. Save your work before restarting to apply it. You can also install a public update from the official downloads page.

### Discover free plugins
plugin downloads favorites freeware open source portable compatibility installation

Open Browser > Plugin Downloads. Show opens filters for Favorites, Native download, Portable install, Open source and Freeware. Sort switches between categories and alphabetical names. Search matches every word across names, descriptions and licences. Reset filters and search returns to the full catalog.

Click the star beside a plugin to save it to your profile's download favorites. These are separate from favorites in the installed Plugin Picker. Download cards distinguish portable packages, installers/archives and website-only entries. Get from developer opens the official website; some free plugins require a vendor account or manager. Complete the vendor's installation, then click Scan Plugins. A downloaded installer is not proof the plugin is installed.

Direct downloads are checked against the catalog checksum before opening. Cancel stops the current download. Downloads opens the package folder. Hide the discovery controls under Settings > Visibility > Browser > Plugin Downloads filters and favorites. Hiding them shows the unfiltered category browser and preserves your saved favorites and choices. Hiding the whole tab does not remove any installed plugins.

## Built-in audio effects and editors
### Tesla time stretch and pitch shift
Tesla audio editor duration pitch semitones Signalsmith stretch

In Tesla, select the audio to change, or clear the selection to process the whole sample. Open Tools > Time stretch / pitch shift, or click the stretch tool. Duration changes the length independently of pitch: 200% is twice as long; 50% is half as long. Pitch uses semitones: +12 is one octave up. Click either value to type an exact number, or use its minus and plus buttons. Duration supports 25–400%; pitch supports -24 to +24 semitones.

Apply renders in the background and shows Processing. Cancel or Escape keeps the original audio. Wait for a cancelled render to finish before opening another. On success the processed range is selected; Undo restores the previous audio and markers in one step. Source files on disk are never overwritten. Save As or send the edited audio to the Playlist when ready. Markers, regions and loops follow the time change; whole-sample changes also adjust stored sample tempo. The tool supports samples and results up to 20 minutes. Stop recording before processing.

Hide its toolbar button and Tools menu entry in Settings > Visibility > Toolbar extras > Tesla time stretch and pitch shift. Existing audio and applied edits are preserved.

### Lemon Tape: tape color and saturation
Lemon Tape Airwindows ToTape6 tape saturation soften head bump flutter

Add Lemon Tape in a Mixer effect slot. Input drives the tape model (-12 to +12 dB); Soften rounds the high frequencies; Head bump adds low-frequency resonance; Flutter introduces pitch movement. Output trims the processed level and Mix blends dry and processed audio. Start with Gentle Warmth or Soft Mix Bus. Match output loudness when judging the effect. Drum Color is more driven; Worn Cassette adds obvious softness and movement. The reel artwork illustrates the effect, not an audio meter.

Knobs support typing values, reset, automation and the usual plugin preset saving. Existing effects and old projects keep their original sound. Lemon Tape uses Chris Johnson's MIT-licensed Airwindows ToTape6 audio processor with Lemon controls; credit and licence are included. This is a creative tape model, not a claim to reproduce one specific physical machine. At very high sample rates, flutter depth is bounded to fit the processor's delay line. Below 22.05 kHz, internal coefficients use a 22.05 kHz floor for stability.

Hide it from new-plugin menus with Settings > Visibility > Toolbar extras > Lemon Tape in plugin menus. Existing instances still play, save and reopen. Plugin Manager's existing Show control also remains available.


### Lemon Hall reverb
Keywords: hall, reverb, Dragonfly, Freeverb3, early reflections, damping, pre-delay

Add **Lemon Hall** in a Mixer effect slot for a stereo hall with early reflections and a modulated reverb tail. It is an additional effect; existing Lemon Reverb instances keep their original sound. Try Vocal Hall or Short Drum Space first. On a dedicated send, use Hall Send - Fully Wet; adjust the channel's send amount to blend the effect.

Decay sets the tail length (0.20–12 seconds). Pre-delay separates the dry attack from the whole wet signal (0–250 ms). Early Balance blends early reflections at 100% with the diffuse late tail at 0%; start around 25%. Diffusion softens the tail's texture. Low Cut and High Cut filter the wet signal, leaving the dry signal intact. HF Decay sets how quickly high frequencies fade relative to the main decay: lower values give a darker tail. Modulation adds movement, Width sets stereo spread (0% is mono), and Mix blends dry and wet. Changes are smoothed; moving Pre-delay can briefly bend the wet pitch, so stop or use gentle changes if unwanted.

Drag a knob, use its wheel, or right-click for the standard value, reset and automation options. Save your settings in a plugin preset or project. The display is a settings illustration, not a live audio meter. Hall geometry and modulation speed are fixed; this is a focused Lemon interface to the Dragonfly/Freeverb3 hall engine, not every control from the original plugin.

To hide it from new plugin menus, turn off **Lemon Hall in plugin menus** in Settings > General > Workspace Visibility > Choose what to show > Toolbar extras. Saved instances still load and play. This option is saved with your profile.


### Clean up audio in Tesla
Keywords: cleanup, clean up, denoise, noise reduction, noise profile, hiss, hum, Tesla, apply to track

Right-click an audio clip in the Playlist and choose **Audio processing > Clean up audio in Tesla**. Tesla opens a copy of the clip's current source audio. It includes the full source file, so a trimmed clip can show more audio than its visible Playlist section. The clip's position, length, fades, gain, stretch settings and routing remain in the Playlist.

For the best result, close the cleanup panel, select at least 100 ms of background noise with no wanted voice or instrument, then open **Tools > Clean up audio (denoise)** and click **Learn selected noise**. Tesla remembers the noise profile for this editor session and clears the selection so the next preview cleans the whole sample. To clean only a portion, close the panel, select that portion and reopen Cleanup. If there is no noise-only section, use **Automatic estimate** and listen carefully; it is an estimate, especially on continuous music.

Start with 12 dB Reduction and 1.50x Sensitivity. Higher Reduction permits stronger attenuation; higher Sensitivity identifies more audio as noise. Click **Preview cleaned** to render and listen. A progress bar appears while processing; Close/Escape cancels processing without editing the sample. Use **Original**, **Preview cleaned** and **Stop** to compare. Preview never changes the document or saved project. Tesla temporarily suppresses its incoming mixer audio during these previews so it does not double up with the audition.

**Apply cleanup** commits the result inside Tesla with one undo step. If you continue editing afterward, the yellow **Apply to track** button at the bottom of Tesla (also in its File menu) returns the edited sample without running denoise again. **Apply to track**, available only when opened from a ML Studio clip, writes a separate cleaned WAV and updates that exact Playlist clip automatically. Other occurrences of the same sample are unchanged. Playlist Undo restores the original clip in one step; the original audio file is never overwritten. The cleaned WAV stays in the project's Rendered/Cleaned folder (or the normal user render folder for projects without a data folder) and is referenced when saving the project. Keep these files with the project when transferring it.

Cleanup uses the current source file. Previously rendered vocal tuning or audio timing is already in that file and stays baked into the cleaned copy; the copied clip does not keep a live link to those earlier editing sessions. Mixer effects are not baked. ARA clips must first be rendered to ordinary audio. If you change the Playlist, source file or source channel while cleanup is open, Apply refuses to replace anything: reopen Cleanup from the intended clip. Changing Tesla's sample length or sample rate also prevents returning it to that clip, because doing so would invalidate its timing. Return links end when the project closes or the operation succeeds.

Tesla also works separately as a **VST3** plugin in FL Studio and other VST3 hosts. Load or drag an audio file into Tesla, use the same cleanup controls, then **Apply cleanup** and **File > Save sample as**. Saved plugin state includes the edited audio. Automatic **Apply to track** is a ML Studio connection; external hosts use Tesla's normal save or drag-out workflow. This is an audio editor that processes the loaded sample, not a live noise gate or an automatic multitrack editor.

This spectral tool targets steady noise such as hiss, fans and hum. It does not separate competing voices or reliably remove individual clicks. Strong settings can damage wanted audio, so compare before applying. Supports mono/stereo, 8–384 kHz, up to 20 minutes and 120 million samples. Noise profiles are not saved across editor sessions. Hide its toolbar tool and Playlist menu action through **Settings > General > Workspace Visibility > Choose what to show > Toolbar extras > Tesla audio cleanup / Playlist cleanup action**; hiding does not change existing audio.

## Settings reference
### Audio device and latency
settings audio normal advanced driver output device mode sample rate hz buffer length latency show asio panel reset underruns auto close
Open Settings > Audio. Normal shows Output device and Latency. Use a shorter buffer when playing live and a larger buffer if playback clicks or drops out. Advanced exposes the device's sample rate (Hz) and buffer length. Under ASIO, Show ASIO panel opens the driver's own controls in both Normal and Advanced, including when latency presets are available. Shared or Exclusive mode is shown for supported Windows and Linux devices.
Use the status and underrun counters to check that the selected settings took effect. Reset underruns clears the counter so you can check a new configuration. Available options depend on your driver and platform. Greyed controls show when a choice is unavailable.

### Audio processing and recording settings
settings cpu generator mixer audio worker limit priority smart disable pdc resampling metronome preview recording format recording offset
Under Audio > Advanced > CPU, Multithreaded generator processing and Multithreaded mixer processing distribute eligible work. Audio worker limit sets the size of the worker pool. Audio processing priority selects the thread priority on supported platforms. Smart disable lets eligible idle effects sleep. Use its per-plugin override if a plugin needs continuous processing.
Mixer contains Plugin delay compensation during playback, playback Resampling quality, Browser preview track, Metronome track, and transport options. A preview or metronome route of 0 uses Master. Play truncated notes on transport controls whether sustained notes can be heard when starting mid-note. Available controls are enabled in the app.
Recording offers 32-bit float, 24-bit PCM, or 16-bit PCM. Recording offset moves recorded audio on the timeline. Positive values place it later. Measure against a reference before compensating for a hardware round trip.

### MIDI devices, controller type and sync
settings midi input output external sync ports master sync offsets curve velocity release pickup mcu generic transport learn monitor pedal
Enable the devices you use under MIDI > Input or Output. Match ports for plugins that listen to a particular port. Choose a Controller type per input. Use Generic MIDI for keys and the DAW port in MCU mode for compatible mixer and transport hardware. Use MIDI Monitor or Refresh device list to verify input. DAW MIDI Learn assigns actions to hardware buttons.
Velocity and release links and curves control how incoming dynamics are mapped. Pickup (takeover mode) helps avoid value jumps. Auto accept detected controller, Foot pedal controls note off, and Toggle on release adjust input handling. Omni preview MIDI channel, Song marker jump MIDI channel and Generator muting MIDI channel assign channels to those functions.
Set Sync type and Master sync offset for an enabled output that sends clock or sync, and External sync and its offset when following another device. Check the status before recording to a synced timeline.

### General display and language
settings general display ppi scaling high visibility smoothing transparent animations colorful waveforms thick lines force refreshes language note names black notes
General > Display contains Main GUI scaling, display PPI, Pop-ups scaling and Toolbars scaling. These controls let you match the workspace to the monitor and adjust menus and toolbars separately. Animations, Animation refresh rate and Smoothing adjust visual motion. Transparent windows, Transparent menus, Force refreshes, Thick lines, High visibility and Colorful waveforms adjust presentation.
General > Language offers English, French, Spanish, German, Brazilian Portuguese and Italian. Translation coverage varies by surface and plugins can use their own language. Note names and Black notes independently choose English, Germanic or solfege notation and sharps or flats.

### General input and Undo
settings general input mouse optimize for double click reset hover menus piano zoom keys undo warnings max levels
Under General > Input, Manage shortcuts opens Controls. Choose Optimize for, Piano Roll zoom keys, Vertical scrolling speed and Horizontal scrolling speed. Click-and-hold & special gesture functions, Use both mouse buttons in step sequencer and Invert pencil buttons adjust editor gestures. Double-click resets faders and knobs and Open main menus on hover are optional and off by default.
Under Undo history, set Maximum undo levels and choose Undo knob tweaks, Alternate undo mode, Put undone recordings in the recycle bin, Show undo history in the browser when undoing changes, and Show recent changes at the top. The clip editors also offer their own pitch and timing history within project Undo.
Manage messages restores or suppresses individual confirmations. Project data size warning controls when the app warns about large project data. System file association and Updates controls depend on your platform and installation.

### General startup and editor defaults
settings general miscellaneous advanced startup default template plugin windows focus meters sample read loading
Use Startup project and Default template to choose what opens on launch. Show startup splash screen and Silent startup control startup presentation. Auto name channels, Auto name effect slots, Auto zip empty channels and Auto-route Channel Rack samples to mixer inserts affect new channels or slots.
Auto select linked modules links selection across views. Auto-scroll mixer while routing keeps destinations reachable. Restore previous state after solo brings back the prior mute state. Alternate meter scale, Auto zoom in piano roll and Small scrollbars in editors change their displays. Monophonic painting in piano roll and Select first channel with notes when the pattern changes adjust editing.
Detach all new plugins, Open detached plugins with the main window, Hide plugin window toolbar by default, Disable typing keyboard priority by default and Limit the number of open plugin editors set editor behavior. Create automation clips at the playback position chooses where new automation starts.
Under Advanced, read sample tempo information and Read sample root note use embedded metadata when present. Fast sample preview, Auto keep long audio on disk, Threaded project sample loading and Initialize controls automatically govern loading and initial behavior. Don't limit windows to screen, Cursors scaling and Legacy scaling adjust layout and sizing. Greyed controls depend on platform or availability.

### File, backups and sample libraries
settings file autosave location maximum user data folder factory folder browser extra search folders repair optional features reset browser cache
File > Backup sets Autosave frequency, Location and Maximum retained backups. Use the Browser's backup area to reopen an earlier revision. Save deliberately before relying on an autosave as a transfer copy.
File shows the User data folder and the factory content location. Add or remove Browser extra search folders to find sample libraries. Changing a search root changes where ML Studio looks for files. File > Troubleshooting offers Repair optional features and Reset browser cache.

### Plugin manager and plugin editors
settings plugins formats search folder find new plugins rescan all rescan plugins with errors combine formats prefer vst clap hide favorite
In Settings > Plugins, enable the folders you want scanned. Add a folder for an installation outside the standard locations. Find new plugins scans additions and Rescan all rebuilds scan results. Use Rescan plugins with errors when retrying failed scans.
Combine formats of the same plugin and the format preference affect which variants appear in the picker. Use the table to filter, favorite, or hide plugins. Use the plugin window's D control to detach or attach its editor. Projects keep the plugin identity and still need a compatible installation on each computer.

### Themes, icons, cursor and mixer design
settings theme appearance skins adjustments theme editor mixer skin designer icons fonts cursor packs note colors brightness
Settings > Theme lets you choose a skin and theme and adjust colors and brightness. Audio and automation clips use note colors changes how clip colors are derived. The Playlist scrollbar style is also adjustable when its visibility option is enabled.
Open Theme Editor for individual color and shape settings or Mixer Skin Designer for mixer-specific appearance. Editing a mixer body, panel, slot or fader color selects the editable Theme colors finish automatically; the Traditional and Full layouts both use those colors. Editing a skin component color enables that custom component. Apply keeps the changes, and Cancel restores the previous appearance. Icons & Fonts lets you choose icon and cursor packs independently, override individual glyphs, and change the interface font. Save a preset or export the theme to reuse the appearance.

### Project data, time and credits
settings project data folder ask copy used samples ppq timebase signature division panning law crossfades info debug about licence
Project > Data folder controls where project audio is stored. Choose when to ask for a data folder and whether to Copy used samples to the project folder. Time settings sets Timebase (PPQ) and a time signature or time division. Changing the signature changes the bar grid. Use the tempo control to change BPM.
Project > Advanced includes Panning law and Play truncated notes in clips. Playlist contains Automatic crossfades and the default for new projects. The Info tab saves the title, author, genre, comments, web link, and Show info on opening. It also shows working time, with a reset control.
About shows version, licence and product links. Debug can be enabled in Workspace Visibility for diagnostics. Debug details can help when describing a reproducible problem.

### Workspace visibility, menus and controls
settings visibility choose what to show menus customize controls bindings portable action shortcuts theme toolbar
Visibility lists the current show/hide controls by workspace area. Search for an exact label to find its switch. Open it directly from Settings > Visibility or through General > Workspace. The settings apply per profile, and Restore default visibility restores the shipped choices.
Menus lets you organize menu entries and restore their structure. Controls manages keyboard shortcuts, mouse bindings, action sequences, and toolbar buttons. Back up your controls with Export and import a supported controls file or portable action script when moving profiles.
