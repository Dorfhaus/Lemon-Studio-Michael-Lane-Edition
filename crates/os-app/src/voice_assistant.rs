// SPDX-License-Identifier: GPL-3.0-or-later

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::path::{Path, PathBuf};
use std::sync::{
    Arc, Mutex, mpsc,
    atomic::{AtomicBool, AtomicU64, Ordering::Relaxed},
};
use std::time::{Duration, Instant};

use crate::app::App;
use crate::dialog::Body;
use crate::voice_runtime::{self, DOWNLOAD_BYTES, Intent};

pub const VISIBILITY: &str = "visible_voice-assistant";
const MAX_SECONDS: usize = 20;
const SILENCE: Duration = Duration::from_millis(850);

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Button { Download, Listen, Finish, External, Local, Choose, Dismiss, KeepActive, HideDock }

#[derive(Default)]
pub struct UiState {
    pub screen: Screen,
    pub phase: f32,
    pub last_frame: Option<Instant>,
    pub level: f32,
    pub progress: u8,
    pub status: String,
    pub transcript: String,
    pub conversation: Vec<String>,
    pub keep_active: bool,
    pub dock_progress: f32,
}

#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub enum Screen {
    #[default]
    Choose,
    Setup,
    Download,
    Ready,
    Listening,
    Processing,
    Result,
}

pub enum Session {
    Listening(Listening),
    Task(Task),
    Plan(crate::voice_plan::Plan),
}

pub struct Task {
    cancel: Arc<AtomicBool>,
    progress: Arc<AtomicU64>,
    receiver: mpsc::Receiver<Result<TaskResult, String>>,
}

enum TaskResult { Installed, Command(Intent) }

impl Drop for Task {
    fn drop(&mut self) { self.cancel.store(true, Relaxed); }
}

impl Task {
    fn start(job: impl FnOnce(&AtomicBool, &AtomicU64) -> Result<TaskResult, String> + Send + 'static) -> Self {
        let cancel = Arc::new(AtomicBool::new(false));
        let progress = Arc::new(AtomicU64::new(0));
        let (sender, receiver) = mpsc::channel();
        let (stop, counter) = (cancel.clone(), progress.clone());
        std::thread::spawn(move || { let _ = sender.send(job(&stop, &counter)); });
        Self { cancel, progress, receiver }
    }
}

pub fn available() -> bool {
    !voice_runtime::LOCAL_AI_ON_HOLD && cfg!(all(windows, target_arch = "x86_64"))
        && crate::workspace_visibility::shown(VISIBILITY)
}

pub fn open(app: &mut App) {
    if !available() || crate::settings_file::read(|s| s.get("ai_assistant_mode") == Some("mcp")) {
        crate::ai_control::open(app);
        return;
    }
    if app.modal_open() { return; }
    let conversation=std::mem::take(&mut app.state.voice_ui.conversation);
    let keep_active=app.state.voice_ui.keep_active;
    app.state.voice_ui = UiState {conversation,keep_active,..Default::default()};
    app.show_form(Body::VoiceAssistant);
    if crate::settings_file::read(|s| s.get("ai_assistant_mode") == Some("local")) {
        listen(app);
    }
}

pub fn choose(app: &mut App) {
    if voice_runtime::LOCAL_AI_ON_HOLD { crate::ai_control::open(app); return; }
    if app.modal_open() { return; }
    let conversation=std::mem::take(&mut app.state.voice_ui.conversation);
    app.state.voice_ui = UiState {conversation,..Default::default()};
    app.show_form(Body::VoiceAssistant);
}

pub fn button(app: &mut App, button: Button) {
    if voice_runtime::LOCAL_AI_ON_HOLD { return; }
    if button==Button::HideDock {
        app.state.voice_ui.keep_active=false;
        app.invalidate();
        return;
    }
    if !is_open(app) { return; }
    match button {
        Button::KeepActive => {
            app.state.voice_ui.keep_active=true;
            app.state.voice_ui.dock_progress=0.;
            close(app);
            app.state.dialogs.pop();
            status(app,Screen::Ready,"");
            app.modal_changed();
        }
        Button::Choose => { close(app); status(app, Screen::Choose, ""); }
        Button::Dismiss => { close(app); app.state.dialogs.pop(); app.modal_changed(); }
        Button::Local => {
            if !available() { status(app, Screen::Choose, "Local AI is currently available on Windows x64."); return; }
            crate::settings_file::write("ai_assistant_mode", "local");
            crate::ai_control::disable(app);
            listen(app);
        }
        Button::Download if app.voice_session.is_none() => {
            let Some(root) = voice_runtime::root() else {
                status(app, Screen::Setup, "Could not find the AI Assistant folder.");
                return;
            };
            app.voice_session = Some(Session::Task(Task::start(move |cancel, progress| {
                voice_runtime::install(&root, cancel, progress)?;
                Ok(TaskResult::Installed)
            })));
            status(app, Screen::Download, "");
        }
        Button::Listen if app.voice_session.is_none() => listen(app),
        Button::Finish => finish(app),
        Button::External => {
            crate::settings_file::write("ai_assistant_mode", "mcp");
            close(app);
            app.state.dialogs.pop();
            app.modal_changed();
            crate::ai_control::open(app);
        }
        _ => {}
    }
}

pub fn animating(app: &App) -> bool {
    (is_open(app) && matches!(app.state.voice_ui.screen, Screen::Listening | Screen::Processing | Screen::Ready | Screen::Result)) || (available() && app.state.voice_ui.keep_active && !app.modal_open())
}

fn is_open(app: &App) -> bool {
    app.state.dialogs.last().is_some_and(|dialog| matches!(dialog.body, Body::VoiceAssistant))
}

fn listen(app: &mut App) {
    if voice_runtime::LOCAL_AI_ON_HOLD { return; }
    if !voice_runtime::root().is_some_and(|root| voice_runtime::ready(&root)) {
        status(app, Screen::Setup, "Download the voice models to get started.");
        return;
    }
    app.state.voice_ui.transcript.clear();
    if let Some(root)=voice_runtime::root() {voice_runtime::warm(root);}
    match Listening::open() {
        Ok(recording) => {
            app.voice_session = Some(Session::Listening(recording));
            status(app, Screen::Listening, "");
        }
        Err(error) => status(app, Screen::Result, &error),
    }
}

pub fn close(app: &mut App) {
    app.voice_session = None;
}

fn status(app: &mut App, screen: Screen, message: &str) {
    app.state.voice_ui.screen = screen;
    app.state.voice_ui.status = message.to_owned();
    app.state.voice_ui.level = 0.0;
    app.invalidate();
}

fn finish(app: &mut App) {
    if !matches!(app.voice_session, Some(Session::Listening(_))) { return; }
    let Some(Session::Listening(listening)) = app.voice_session.take() else { return };
    let rate = listening.rate;
    match listening.finish() {
        Ok(samples) => {
            if let Some(root) = voice_runtime::root() {
                start_inference(app, root, samples, rate);
            } else {
                status(app, Screen::Result, "Could not find the AI Assistant folder.");
            }
        }
        Err(error) => status(app, Screen::Result, &error),
    }
}

fn start_inference(app: &mut App, root: PathBuf, samples: Vec<i16>, rate: u32) {
    let mut context = crate::ai_commands::voice_context(app);
    context["recent_requests"]=serde_json::json!(app.state.voice_ui.conversation);
    app.voice_session = Some(Session::Task(Task::start(move |cancel, _| {
        voice_runtime::interpret(&root, &samples, rate, cancel, &context).map(TaskResult::Command)
    })));
    status(app, Screen::Processing, "");
}

fn complete(app: &mut App, result: Result<TaskResult, String>) {
    match result {
        Ok(TaskResult::Installed) => listen(app),
        Ok(TaskResult::Command(intent)) => {
            app.state.voice_ui.conversation.push(intent.transcript.clone());
            while app.state.voice_ui.conversation.len()>6 {app.state.voice_ui.conversation.remove(0);}
            app.state.voice_ui.transcript = intent.transcript;
            if let Some(question)=intent.question {
                app.state.voice_ui.conversation.push(format!("Assistant asks: {question}"));
                status(app,Screen::Result,&question);
                return;
            }
            match intent.command {
                Some(command) if app.project_loading().is_none() && !app.state.collaborating => {
                    match crate::voice_plan::Plan::prepare(app,command) {
                        Ok(plan) => { app.voice_session=Some(Session::Plan(plan)); status(app,Screen::Processing,""); }
                        Err(error) => status(app,Screen::Result,&error),
                    }
                }
                Some(_) => status(app, Screen::Result, "Try again with a local project open."),
                None => status(app, Screen::Result, "I could not map that request. Try one specific ML Studio action."),
            }
        }
        Err(error) => {
            let screen = if app.state.voice_ui.screen == Screen::Download { Screen::Setup } else { Screen::Result };
            status(app, screen, &error);
        }
    }
}

pub fn tick(app: &mut App) {
    voice_runtime::maintain(available() && app.state.voice_ui.keep_active);
    let now = Instant::now();
    let elapsed = app.state.voice_ui.last_frame.replace(now)
        .map_or(1.0 / 60.0, |before| now.duration_since(before).as_secs_f32().min(0.1));
    if app.state.voice_ui.keep_active && !is_open(app) {
        app.state.voice_ui.dock_progress=(app.state.voice_ui.dock_progress+elapsed*3.5).min(1.);
    }
    if animating(app) {
        app.state.voice_ui.phase = (app.state.voice_ui.phase + elapsed * 1.4) % (std::f32::consts::TAU * 100.0);
        app.invalidate();
    }
    if app.voice_session.is_none() { return; }
    if !is_open(app) {
        close(app);
        return;
    }
    if matches!(app.voice_session,Some(Session::Plan(_))) {
        let Some(Session::Plan(mut plan))=app.voice_session.take() else {return};
        let orb=app.state.dialogs.pop();
        let result=plan.advance(app);
        match result {
            Ok(true) => { app.state.voice_ui.conversation.push("Assistant: requested actions completed.".into()); app.state.voice_ui.dock_progress=0.; app.modal_changed(); }
            Ok(false) => {
                if let Some(orb)=orb { app.state.dialogs.push(orb); }
                app.voice_session=Some(Session::Plan(plan));
                app.invalidate();
            }
            Err(error) => {
                if !app.modal_open() {
                    if let Some(orb)=orb {app.state.dialogs.push(orb);}
                    status(app,Screen::Result,&error);
                } else {app.state.message=Some(error); app.modal_changed();}
            }
        }
        return;
    }
    match app.voice_session.as_ref().unwrap() {
        Session::Plan(_) => unreachable!(),
        Session::Listening(listening) => {
            let (level, end) = listening.activity();
            if end { finish(app); }
            else {
                let old = app.state.voice_ui.level;
                let speed = if level > old { 20.0 } else { 5.0 };
                app.state.voice_ui.level += (level - old) * (1.0 - (-elapsed * speed).exp());
                if (old - app.state.voice_ui.level).abs() > 0.0001 { app.invalidate(); }
            }
        }
        Session::Task(task) => {
            let result = task.receiver.try_recv();
            if app.state.voice_ui.screen == Screen::Download {
                let progress = (100 * task.progress.load(Relaxed) / DOWNLOAD_BYTES).min(100) as u8;
                if progress != app.state.voice_ui.progress {
                    app.state.voice_ui.progress = progress;
                    app.invalidate();
                }
            }
            match result {
                Ok(result) => {
                    app.voice_session = None;
                    complete(app, result);
                }
                Err(mpsc::TryRecvError::Disconnected) => {
                    app.voice_session = None;
                    complete(app, Err("The Voice Assistant stopped unexpectedly.".into()));
                }
                Err(mpsc::TryRecvError::Empty) => {}
            }
        }
    }
}

/// Assistant-only input conditioning. Raw (pre-gain) energy determines speech;
/// gain cannot turn its own amplified room noise into an endless utterance.
struct VoiceInput {
    dc: f32,
    power: f32,
    gain: f32,
    noise: f32,
    voiced: bool,
    speech_samples: u32,
    window: u32,
    count: u32,
    smoothing: f32,
}
impl VoiceInput {
    fn new(rate: u32) -> Self {
        Self { dc: 0., power: 0., gain: 8., noise: 0.00025, voiced: false,
            speech_samples: 0, window: (rate / 100).max(1), count: 0,
            smoothing: 1. - (-1. / (rate.max(1) as f32 * 0.08)).exp() }
    }
    fn process(&mut self, sample: f32) -> f32 {
        let sample = if sample.is_finite() { sample.clamp(-1.,1.) } else { 0. };
        self.dc += self.smoothing * (sample-self.dc);
        let clean = sample-self.dc;
        self.power += clean*clean;
        self.count += 1;
        if self.count >= self.window {
            let rms=(self.power / self.count as f32).sqrt();
            let threshold=(self.noise*3.).max(0.0008);
            if rms > threshold {
                self.speech_samples=(self.speech_samples+self.count).min(self.window*10);
            } else {
                self.speech_samples=self.speech_samples.saturating_sub(self.count*2);
                self.noise += 0.03*(rms-self.noise);
            }
            self.voiced=self.speech_samples >= self.window*4;
            // Up to +30 dB for quiet speech; bounded gain keeps silence controlled.
            let target=if rms > threshold { (0.10 / rms.max(0.0001)).clamp(0.5,32.) } else { self.gain.min(8.) };
            self.gain += if target < self.gain { 0.45 } else { 0.12 } * (target-self.gain);
            self.power=0.; self.count=0;
        }
        let amplified=clean*self.gain;
        // Soft-knee compression above -8 dBFS, with headroom for recognition.
        let magnitude=amplified.abs();
        let compressed=if magnitude <= 0.4 { magnitude } else { 0.4+(magnitude-0.4)/(1.+(magnitude-0.4)/0.5) };
        amplified.signum()*compressed.min(0.9)
    }
}

struct Capture {
    mono: Vec<i16>,
    level: f32,
    last_voice: Option<Instant>,
    failed: bool,
    input: VoiceInput,
}

pub struct Listening {
    stream: cpal::Stream,
    capture: Arc<Mutex<Capture>>,
    start: Instant,
    rate: u32,
}

impl Listening {
    fn open() -> Result<Self, String> {
        let host = cpal::default_host();
        let mic = host.default_input_device().ok_or("Choose a default microphone in Windows Sound settings.")?;
        let supported = mic.default_input_config().map_err(|e| format!("Could not open the microphone: {e}"))?;
        let format = supported.sample_format();
        let config: cpal::StreamConfig = supported.into();
        let rate = config.sample_rate;
        let capture = Arc::new(Mutex::new(Capture {
            mono: Vec::with_capacity(rate as usize * MAX_SECONDS),
            level: 0.0,
            last_voice: None,
            failed: false,
            input: VoiceInput::new(rate),
        }));
        let stream = match format {
            cpal::SampleFormat::F32 => mic_stream::<f32>(&mic, config, &capture),
            cpal::SampleFormat::I16 => mic_stream::<i16>(&mic, config, &capture),
            cpal::SampleFormat::U16 => mic_stream::<u16>(&mic, config, &capture),
            cpal::SampleFormat::I24 => mic_stream::<cpal::I24>(&mic, config, &capture),
            cpal::SampleFormat::I32 => mic_stream::<i32>(&mic, config, &capture),
            cpal::SampleFormat::F64 => mic_stream::<f64>(&mic, config, &capture),
            _ => return Err("The default microphone uses an unsupported sample format.".into()),
        }?;
        stream.play().map_err(|e| format!("Could not start the microphone: {e}"))?;
        Ok(Self { stream, capture, start: Instant::now(), rate })
    }

    fn activity(&self) -> (f32, bool) {
        let Ok(data) = self.capture.try_lock() else { return (0.0, false) };
        let finished = self.start.elapsed() >= Duration::from_secs(MAX_SECONDS as u64)
            || data.failed
            || data.last_voice.map_or_else(
                || self.start.elapsed() > Duration::from_secs(10),
                |last| last.elapsed() > SILENCE,
            );
        (data.level, finished)
    }

    fn finish(self) -> Result<Vec<i16>, String> {
        drop(self.stream);
        let mut data = self.capture.lock().map_err(|e| e.to_string())?;
        if data.failed { return Err("The microphone disconnected.".into()); }
        if data.last_voice.is_none() || data.mono.len() < self.rate as usize / 4 {
            return Err("I didn’t hear a command. Try speaking a little closer to the microphone.".into());
        }
        Ok(std::mem::take(&mut data.mono))
    }
}

fn mic_stream<T>(
    mic: &cpal::Device, config: cpal::StreamConfig, capture: &Arc<Mutex<Capture>>,
) -> Result<cpal::Stream, String>
where
    T: cpal::SizedSample,
    f32: cpal::FromSample<T>,
{
    let channels = usize::from(config.channels).max(1);
    let samples = capture.clone();
    let errors = capture.clone();
    mic.build_input_stream::<T, _, _>(
        config,
        move |frames, _| {
            let Ok(mut capture) = samples.try_lock() else { return };
            let room = capture.mono.capacity() - capture.mono.len();
            let mut sum = 0.0;
            let mut count = 0;
            let mut heard = false;
            for frame in frames.chunks_exact(channels).take(room) {
                let sample = frame.iter().map(|x| x.to_sample::<f32>()).sum::<f32>() / channels as f32;
                let sample = if sample.is_finite() { sample.clamp(-1.0, 1.0) } else { 0.0 };
                let sample = capture.input.process(sample);
                heard |= capture.input.voiced;
                sum += sample * sample;
                count += 1;
                capture.mono.push((sample * 32767.0) as i16);
            }
            if count > 0 {
                capture.level = (sum / count as f32).sqrt();
                if heard { capture.last_voice = Some(Instant::now()); }

            }
        },
        move |_| {
            if let Ok(mut capture) = errors.lock() { capture.failed = true; }
        },
        None,
    ).map_err(|e| format!("Could not open the microphone: {e}"))
}

pub fn models_location() -> String {
    voice_runtime::root().as_deref().map(Path::display).map(|path| path.to_string()).unwrap_or_default()
}

#[cfg(all(test, windows))]
mod tests {
    use super::*;
    use crate::app::Action;
    use crate::dialog::DHit;

    fn samples(path: &Path) -> Vec<i16> {
        let wave = std::fs::read(path).unwrap();
        assert_eq!(&wave[..4], b"RIFF");
        let offset = wave[12..].windows(4).position(|id| id == b"data").unwrap() + 12;
        let bytes = u32::from_le_bytes(wave[offset + 4..offset + 8].try_into().unwrap()) as usize;
        wave[offset + 8..offset + 8 + bytes].chunks_exact(2)
            .map(|sample| i16::from_le_bytes(sample.try_into().unwrap())).collect()
    }

    #[test]
    fn local_ai_hold_routes_to_mcp_and_blocks_downloads() {
        assert!(voice_runtime::LOCAL_AI_ON_HOLD);
        assert!(!available());
        let mut app=App::new(None);
        open(&mut app);
        assert!(matches!(app.state.dialogs.last().unwrap().body,Body::Ai));
        assert!(app.voice_session.is_none());
        app.state.dialogs.clear();
        choose(&mut app);
        assert!(matches!(app.state.dialogs.last().unwrap().body,Body::Ai));
        button(&mut app,Button::Local);
        assert!(app.voice_session.is_none());
        let root=std::env::temp_dir().join("ml-local-ai-held-no-download");
        assert!(voice_runtime::install(&root,&AtomicBool::new(false),&AtomicU64::new(0)).unwrap_err().contains("on hold"));
    }

    #[test]
    #[ignore = "Local AI experiment is on hold"]
    fn voice_dock_keeps_workspace_free_without_recording() {
        let mut app=App::new(None);
        app.show_form(Body::VoiceAssistant);
        app.state.voice_ui.screen=Screen::Ready;
        let mut canvas=os_ui::Canvas::new(1280,800,1.).unwrap();
        app.paint(&mut canvas,None);
        assert!(app.hits.iter().any(|h|h.action==Action::Dlg(DHit::Voice(Button::KeepActive))));
        button(&mut app,Button::KeepActive);
        assert!(!app.modal_open());
        assert!(app.state.voice_ui.keep_active);
        assert!(app.voice_session.is_none());
        assert!(animating(&app));
        app.state.voice_ui.dock_progress=1.;
        app.paint(&mut canvas,None);
        let dock=app.hits.iter().find(|h|h.action==Action::Voice(Button::HideDock)).expect("dismiss control");
        assert!(dock.rect.x>1100. && dock.rect.y>650.);
        if let Some(out)=std::env::var_os("OS_SHOT_DIR") {
            let out=std::path::PathBuf::from(out); std::fs::create_dir_all(&out).unwrap();
            canvas.pixmap.save_png(out.join("assistant-dock.png")).unwrap();
        }
        // A successful command returns to the companion without restarting capture.
        app.show_form(Body::VoiceAssistant);
        app.voice_session=Some(Session::Plan(crate::voice_plan::Plan::prepare(&app,serde_json::json!({"steps":[{"tool":"lemon_transport","arguments":{"op":"tempo","value":127}}]})).unwrap()));
        tick(&mut app);
        assert!(!app.modal_open());
        assert_eq!(app.state.tempo,127.);
        assert!(app.state.voice_ui.keep_active && app.voice_session.is_none());
        app.act(Action::Voice(Button::HideDock));
        assert!(!app.state.voice_ui.keep_active);
        assert!(!animating(&app));
    }

    #[test]
    #[ignore = "Local AI experiment is on hold"]
    fn voice_upgrade_quiet_input_and_live_animation() {
        let mut input = VoiceInput::new(48000);
        let mut square = 0.0;
        let mut detected = false;
        for i in 0..48000 {
            let out = input.process(0.002*(i as f32*std::f32::consts::TAU*220./48000.).sin());
            if i>24000 { square += out*out; }
            detected |= input.voiced;
        }
        assert!(detected, "quiet speech must cross the raw speech detector");
        assert!((square/24000.0_f32).sqrt()>0.025, "quiet audio must be amplified");
        for _ in 0..48000 { assert!(input.process(1.).abs()<=0.9); }
        for _ in 0..48000 { input.process(0.); }
        assert!(!input.voiced, "silence must finish the command after gain");
        let mut quiet = VoiceInput::new(48000);
        for i in 0..48000 { quiet.process(if i%2==0 {0.0001} else {-0.0001}); }
        assert!(!quiet.voiced);
        assert!(quiet.process(f32::NAN).is_finite());
        let mut app=App::new(None);
        app.state.cables.borrow_mut().animate(false);
        app.show_form(Body::VoiceAssistant);
        app.state.voice_ui.screen=Screen::Listening;
        assert!(app.animating(), "orb needs frame pacing with global animations off");
        let phase=app.state.voice_ui.phase;
        tick(&mut app);
        assert!(app.state.voice_ui.phase>phase);
        let mut canvas=os_ui::Canvas::new(1280,800,1.).unwrap();
        app.paint(&mut canvas,None);
        assert!(!app.hits.iter().any(|h| matches!(h.action, Action::Dlg(DHit::Voice(Button::Choose | Button::External | Button::Download)))));
        assert!(app.voice_session.is_none());
        button(&mut app,Button::Dismiss);
        assert!(!is_open(&app));
    }

    #[test]
    #[ignore = "Requires downloaded local command model; never opens microphone"]
    fn voice_upgrade_local_command_model() {
        let root=PathBuf::from(std::env::var_os("ML_VOICE_ASSETS").unwrap());
        let mut app=App::new(None);
        let context=crate::ai_commands::voice_context(&mut app);
        for (words, tool, op) in [
            ("Show the mixer", "lemon_transport", "show"),
            ("Set the tempo to 128 BPM", "lemon_transport", "tempo"),
            ("Mute the first channel", "lemon_channel", "mute"),
            ("Undo the last change", "lemon_history", "undo"),
        ] {
            let result=voice_runtime::choose_command(&root,words,&AtomicBool::new(false),&context).unwrap().expect(words);
            eprintln!("{words} -> {result}");
            assert_eq!(result["tool"],tool);
            assert_eq!(result["arguments"]["op"],op);
            if op=="show" { assert_eq!(result["arguments"]["window"],"mixer"); }
            if op=="tempo" { assert_eq!(result["arguments"]["value"],128); }
            if op=="mute" { assert_eq!(result["arguments"]["row"],0); assert_eq!(result["arguments"]["enabled"],true); }
            crate::ai_commands::call_voice(&mut app,tool,&result["arguments"]).unwrap();
            assert!(app.ai_runtime.is_none(), "local commands must not enable external MCP");
        }
    }

    #[test]
    #[ignore = "Requires pinned Windows runtimes, models and a synthetic speech fixture"]
    fn voice_e2e_sets_real_project_tempo_and_undoes() {
        let root = PathBuf::from(std::env::var_os("ML_VOICE_ASSETS").unwrap());
        let fixture = PathBuf::from(std::env::var_os("ML_VOICE_FIXTURE").unwrap());
        voice_runtime::install(&root, &AtomicBool::new(false), &AtomicU64::new(0)).unwrap();
        let mut app = App::new(None);
        app.act(Action::Ai);
        assert!(is_open(&app));
        assert!(app.voice_session.is_none(), "isolated test profile never opens a real microphone");
        let mut canvas = os_ui::Canvas::new(1280, 720, 1.0).unwrap();
        app.paint(&mut canvas, None);
        assert!(app.hits.iter().any(|hit| hit.action == Action::Dlg(DHit::Voice(Button::Local))));
        let initial = app.state.tempo;
        let speak = |app: &mut App, file: &str| {
            start_inference(app, root.clone(), samples(&fixture.join(file)), 16_000);
            let limit = Instant::now() + Duration::from_secs(40);
            while app.voice_session.is_some() {
                tick(app);
                assert!(Instant::now() < limit);
                std::thread::sleep(Duration::from_millis(10));
            }
            eprintln!("{file}: {:?} -> {}", app.state.voice_ui.transcript, app.state.voice_ui.status);
        };
        speak(&mut app, "unsupported.wav");
        assert_eq!(app.state.tempo, initial);
        assert!(!is_open(&app));
        app.show_form(Body::VoiceAssistant);
        speak(&mut app, "tempo-128.wav");
        assert_eq!(app.state.tempo, 128.0, "{}", app.state.voice_ui.status);
        assert_eq!(crate::history::undo(&mut app.state), Some("Tempo"));
        assert_eq!(app.state.tempo, initial);
        app.dialog_click(DHit::Close);
        assert!(!is_open(&app));
    }
}
