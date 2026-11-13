# Local AI release hold — 12 November 2026

Michael paused the local voice/model project. LOCAL_AI_ON_HOLD in voice_runtime.rs is true for all builds; do not enable it for a release without an explicit request to resume this project. Toolbar AI assistant and its settings open existing external MCP controls. Local downloads, listening and companion orb are disabled. The retained prototype documentation below is historical until the project resumes. No local model files are included in releases.

External MCP remains available: enable AI control, export setup, configure a compatible assistant, connect and approve it in ML Studio. Commands are entered through that assistant's own interface. The local listening/analyzing orb is not wired to external MCP speech or responses.

Approved MCP tool calls show a blue **Working** orb in the bottom-right corner of the main workspace. It fades 1.5 seconds after the last call. The orb reflects command activity in ML Studio and uses no microphone or model. Hide it under **Settings > Visibility > Editor feedback > MCP activity orb**.

# AI control (local MCP)

The **AI assistant** toolbar button remembers Local AI or external MCP per profile. Change this through **Options > Settings > AI assistant settings**. Local AI is currently Windows x64. External MCP remains off at launch until explicitly enabled.

## Offline voice assistant (Windows x64)

Choose Local AI and download its pinned runtimes/models once (745 MB). Opening AI assistant starts listening immediately. The orb continuously morphs independently of the global decorative Animations setting, and reacts to microphone level. Pause to submit, or click the orb to finish. Click outside or press Esc to cancel. A Keep active button below the listening orb shrinks it into an animated bottom-right companion. Click the small orb to speak again; successful commands return to the companion. Its x dismisses it. Parking cancels any current capture and leaves the microphone off while idle; the workspace remains editable. The choice lasts for the current session and the existing Local Voice Assistant visibility control hides the companion too. Errors remain visible and clicking the orb retries. Setup/choice screens retain their necessary controls.

The default Windows input is conditioned only for the assistant. A bounded automatic gain stage (up to 32x), DC removal and soft compression raise quiet speech while preserving peak headroom. Speech detection uses pre-gain energy with an adaptive floor and a short onset hold, so gain itself cannot trigger speech. Capture waits up to 10 seconds for speech, ends after about 850 ms of silence, and is capped at 20 seconds. It does not change the Windows microphone gain, project audio or recording channels.

The local command model builds an ordered plan of up to twelve actions from semantic host tools: transport/windows, instruments, channels/routing, mixer, dedicated EQ, effects, notes, clips, exposed parameters, contextual menus, dialog text, history, scrolling and automation. It receives current project state, installed built-in/third-party plugin identities, implemented menu labels and exposed hosted-plugin parameters. Relevant plugins and parameters are retrieved from the full live catalog (up to 32 instrument/effect candidates and 64 parameters per prompt); project lists remain bounded to 32 entries. This is runtime discovery, not model fine-tuning or complete knowledge of native plugin internals. Recently spoken requests and clarification responses provide short conversation context.

Examples: “Open the channel rack, load ML Wave and then route it to track 8 in the mixer”, “Open up EQ on track 13”, “Show the mixer”, “Set tempo to 128 BPM”, and “Undo the last change”. Instrument names are resolved against actual installed identities; there is no special LM Wave alias. Open reuses a unique existing instrument/effect, while load creates a new instance. Dedicated EQ reuses the track EQ without replacing user inserts. Ambiguous plugin names are refused; duplicate formats may require manual selection. Unknown requests should ask for clarification instead of inventing commands.

Plans validate mixer destinations before beginning, apply one host operation per UI tick, wait for actual instrument/effect loading, and bind later routing to the created channel's stable identity. They stop on failure or incompatible dialogs. Existing per-edit Undo applies; a plan is not one atomic undo transaction, and cancellation does not roll back completed steps. No external MCP listener is enabled for local commands. Success dismisses the orb; errors remain visible. Model predictions can still be wrong. Arbitrary native-plugin controls, dynamic MCP control IDs, audio-quality judgments and unrestricted autonomous workflows are not supported. Some actions open normal DAW forms for manual completion.

Whisper Base and Qwen3.5-0.8B Q4_0 run locally at below-normal process priority. Speech recognition and token generation use two CPU threads; prompt ingestion uses one quarter of available logical CPUs, bounded to 2–6 threads (six on Michael's desktop). Speech recognition uses English and an ML Studio vocabulary prompt. Qwen uses an 8192-token context and constrained plan JSON. Generic routing/window words no longer retrieve unrelated plugin parameters. The JSON grammar separates executable plans from clarification questions. Its authenticated loopback service warms when listening starts and remains cached until 120 seconds idle (or stays resident while Keep active is enabled); Whisper is still launched for each utterance. No cloud inference is used. Capture is in memory; temporary WAV/transcript files are removed after use. After prompt-thread and retrieval tuning, the three text planning examples took about 2.9 seconds each (previous run roughly 6–12 seconds). Synthetic speech passed the ML Wave/load/route workflow in 2.35 seconds with a warm model and a reusable cached prompt. These inference timings exclude the speech-ending pause, and cold or larger-project requests can take longer. These are test timings, not guaranteed latency. Physical microphone sensitivity and real-project performance require hands-on evaluation.

## Connect

1. Enable AI in ML Studio.
2. Click **Save setup on Desktop**. It writes `ML Studio AI Setup/Personal MCP.json` or `Development MCP.json` plus instructions. These describe an MCP stdio server: the current Lemon executable with `--ai-mcp` and this profile's connection-file path.
3. Import/configure that server in a compatible assistant. This does not install or configure a particular AI product.
4. Connect, then approve the named client inside ML Studio within 60 seconds. The assistant cannot click its own approval or close the AI permission panel.
5. Turning AI off revokes sessions immediately. Restart the assistant connection after enabling again. Sessions expire after 15 minutes without a request; clients may ping to keep them alive.

Remember stores the client name and private connection key in this profile. It suppresses the approval only after the user manually enables AI. The name is self-declared, not a vendor certificate; trust is possession of the local key plus the approved name. Do not grant access to untrusted programs. Forget revokes the key and requires fresh approval. Personal and development profiles remain separate. The generated setup JSON does not contain the key.

External MCP does not require a bundled chatbot, model or subscription. Lemon does not transmit project data to a cloud; the selected assistant may send it to its provider. Assistant edits can modify the project and use its existing file/export dialogs. Human approval grants that scope for the session; it is not a confirmation for every edit.

## Coverage

Sixteen tools expose transport/tempo, project inspection, channel routing and levels, mixer levels/pan/mute/strip/FX/routing/Quick Sidechain, plugin insertion and exposed parameters, note editing, playlist clips, automation points, undo/redo, view scrolling, and discoverable app menu/dialog commands. Notes and clip positions use project ticks. Mixer faders use the host's normalized range (0.8 = unity); pan uses 0.5 = center. Slots are 1-based user slots, with the dedicated mini-EQ excluded from that numbering. Use returned hosted keys for plugin parameters.

`lemon_controls` returns command IDs and labels from the current interface without screenshots or mouse automation. Refresh after each edit and after the next paint. Pointer-drag/audio-keyboard surfaces are excluded: use semantic tools for notes, clips, routing and values. Existing menus provide access to additional commands; this is not a promise that every native plugin UI or every DAW feature has a dedicated semantic tool. Native plugin internals are only accessible when exposed to the host. There is no automatic audio listening or mixing-quality analysis.

Edits use the existing command and undo paths where those commands support undo. Native dialogs and loading states block incompatible edits. AI permission controls are never exposed. Frozen/comped clip moves are refused; use their existing track commands. Read current state before editing, preserve user work, and use Make Unique before changing the source of a shared clip.

## Transport and boundaries

The optional server binds IPv4 loopback on a random port and serves authenticated POST `/mcp`. A random 256-bit bearer key is required before requests reach the application. It rejects browser Origin headers, unexpected Host, duplicate headers, transfer encoding, oversized requests and wrong sessions. It has a bounded queue, at most four request workers, 20 tool requests per second, request deadlines and socket timeouts. UI-thread dispatch avoids concurrent mutation of the project. Turning off drops the queue; timed-out requests do not execute later.

The stdio adapter starts before DAW/audio initialization. It accepts newline JSON-RPC on stdin and writes only JSON-RPC on stdout. It reads the current endpoint from the profile, permits only `http://127.0.0.1:<port>/mcp`, and follows no redirects. This implements the JSON response subset of MCP 2025-06-18 Streamable HTTP, with stdio for client compatibility; no SSE subscription or remote-network server is provided.

Connection and remembered-key files live under the profile's `AI control` folder. Unix permissions are 0700 for that folder and 0600 for credential files; Windows inherits the user's profile permissions. Other programs running as the same user can read those credentials. This does not defend against a compromised OS account.

## Verification

Focused synthetic/headless tests cover HTTP authentication/Origin/session rejection, approval/decline, permission-panel isolation, command dispatch/revocation, note undo/redo, clip movement, routing selection isolation, input validation, FLP save/load and toolbar/menu discovery. Native assistant interoperability and hands-on production sessions still require user verification. No microphone or live session is used by these tests.
