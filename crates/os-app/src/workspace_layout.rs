//! Named, per-profile window arrangements. Loading never replaces musical project data.
use std::path::{Path, PathBuf};
use serde::{Deserialize, Serialize};
use crate::{app::{App, State, Win, WinState}, dialog::{Answer, Kind, Msg, Then}, playlist::{Cmd, PItem}};

pub const VISIBILITY: &str = "visible_workspace-layouts";
const EXT: &str = "lslayout";
const WINDOWS: [Win; 8] = [Win::Playlist, Win::Mixer, Win::PianoRoll, Win::ChannelRack, Win::Browser, Win::Perf, Win::Tapper, Win::Touch];

#[derive(Clone, Debug, PartialEq)]
pub enum Command { Save, Open, Load(PathBuf), Shortcut(PathBuf), Delete(PathBuf), Previous }

#[derive(Serialize, Deserialize)]
struct Window {
    rect: [f32; 4], max: bool, mini: bool,
}
#[derive(Serialize, Deserialize)]
struct Frame { window: usize, pos: (i32, i32), size: (u32, u32), max: bool }
#[derive(Serialize, Deserialize)]
struct Layout {
    version: u8,
    windows: [Window; 3],
    rack: [f32; 4],
    browser: (bool, f32),
    auxiliary: [(f32, f32); 3],
    open: Vec<usize>,
    detached: Vec<usize>,
    frames: Vec<Frame>,
    mixer: u8,
    attached: bool,
}
impl Layout {
    fn capture(s: &State) -> Self {
        Self {
            version: 1,
            windows: std::array::from_fn(|i| {let w = s.wins[i]; Window {rect: [w.rect.x,w.rect.y,w.rect.w,w.rect.h],max:w.maxed,mini:w.mini}}),
            rack: [s.rack_pos.0,s.rack_pos.1,s.rack_size.0,s.rack_size.1],
            browser: (s.show_browser,s.browser_w),
            auxiliary: [s.perf.pos,s.tapper.pos,s.touch.pos],
            open: s.z.iter().filter_map(|w| WINDOWS.iter().position(|v| v==w)).collect(),
            detached: s.native_detached.iter().filter_map(|w| WINDOWS.iter().position(|v| v==w)).collect(),
            frames: s.native_frames.iter().filter_map(|(w,f)| WINDOWS.iter().position(|v| v==w).map(|window|Frame{window,pos:f.pos,size:f.size,max:f.max})).collect(),
            mixer: crate::mixer::view(s),
            attached: s.playlist.attached.is_some(),
        }
    }
    fn decode(bytes: &[u8]) -> Result<Self, String> {
        if bytes.len()>64*1024 {return Err("Layout file is too large".into());}
        let v: Self = serde_json::from_slice(bytes).map_err(|e| e.to_string())?;
        let rect = |r: &[f32;4]| r.iter().all(|n|n.is_finite() && n.abs()<100000.) && r[2]>=100. && r[3]>=50.;
        if v.version!=1 || !v.windows.iter().all(|w|rect(&w.rect)) || !rect(&v.rack)
            || !matches!(v.mixer,0..=3|10..=17|32)
            || !v.browser.1.is_finite() || !(100.0..=5000.0).contains(&v.browser.1)
            || v.open.len()>WINDOWS.len() || v.open.iter().any(|&i|i>=WINDOWS.len())
            || v.detached.len()>4 || v.detached.iter().any(|i|!matches!(i,0|1|2|4))
            || v.frames.len()>4 || v.frames.iter().any(|f|!matches!(f.window,0|1|2|4) || f.pos.0.unsigned_abs()>100000 || f.pos.1.unsigned_abs()>100000 || f.size.0==0 || f.size.1==0 || f.size.0>32768 || f.size.1>32768)
            || v.auxiliary.iter().any(|p|!p.0.is_finite() || !p.1.is_finite() || p.0.abs()>100000. || p.1.abs()>100000.) {
            return Err("Invalid window layout".into());
        }
        Ok(v)
    }
    fn apply(self, app: &mut App) {
        // Use the normal attach transition so each project keeps its own track associations.
        if self.attached != app.state.playlist.attached.is_some() {crate::attached_mixer::toggle(app);}
        let s = &mut app.state;
        for (i,w) in self.windows.into_iter().enumerate() {
            let [x,y,wid,hei]=w.rect;
            s.wins[i]=WinState{rect:os_ui::Rect::new(x,y,wid,hei),maxed:w.max,mini:w.mini};
        }
        s.rack_pos=(self.rack[0],self.rack[1]); s.rack_size=(self.rack[2],self.rack[3]);
        s.show_browser=self.browser.0; s.browser_w=self.browser.1; s.browser_auto_hidden=false;
        s.perf.pos=self.auxiliary[0]; s.perf.placed=true;
        s.tapper.pos=self.auxiliary[1]; s.tapper.placed=true;
        s.touch.pos=self.auxiliary[2]; s.touch.placed=true;
        s.z.retain(|w|!WINDOWS.contains(w));
        for i in self.open {let w=WINDOWS[i];if !s.z.contains(&w){s.z.push(w);}}
        s.native_detached=self.detached.into_iter().map(|i|WINDOWS[i]).collect();
        if self.attached {s.native_detached.retain(|w|*w!=Win::Playlist);}
        s.native_frames=self.frames.into_iter().map(|f|(WINDOWS[f.window],crate::layout::MainWin{pos:f.pos,size:f.size,max:f.max})).collect();
        s.native_layout_generation=s.native_layout_generation.wrapping_add(1);
        crate::mixer::set_layout(s,self.mixer);
        s.startup_layout_pending=false;
        s.dock_work=None;
        crate::layout::save(s); crate::browser::save_width(s.browser_w);
        app.invalidate();
    }
}

fn directory() -> Option<PathBuf> {crate::app::theme_dir().map(|p|p.join("Window layouts"))}
fn name(path: &Path) -> String {path.file_stem().unwrap_or_default().to_string_lossy().into_owned()}
pub fn presets() -> Vec<PathBuf> {
    let mut paths: Vec<PathBuf> = crate::prefs::text("workspace_layout_files").and_then(|v|serde_json::from_str(&v).ok()).unwrap_or_default();
    if let Some(dir)=directory() {if let Ok(entries)=std::fs::read_dir(dir) {paths.extend(entries.flatten().map(|v|v.path()).filter(|p|p.extension().is_some_and(|e|e.eq_ignore_ascii_case(EXT))));}}
    paths.retain(|p|p.is_file()); paths.sort(); paths.dedup();
    paths.sort_by_key(|p|name(p).to_lowercase()); paths
}
fn remember(path: &Path) {
    let mut paths=presets(); if !paths.iter().any(|p|p==path){paths.push(path.to_path_buf());}
    crate::prefs::set_text("workspace_layout_files",&serde_json::to_string(&paths).unwrap());
}
pub fn actions() -> Vec<(String, String, Cmd)> {
    presets().into_iter().map(|path|(format!("workspace-layout:{}",path.to_string_lossy()),format!("Window layouts / {}",name(&path)),Cmd::WorkspaceLayout(Command::Load(path)))).collect()
}
pub fn items(_: &State) -> Vec<PItem> {
    if !crate::workspace_visibility::shown(VISIBILITY) {return Vec::new();}
    let item=|label: &str,c|PItem::new(label.to_string(),Cmd::WorkspaceLayout(c));
    let mut items=vec![item("Save custom layout as...",Command::Save),item("Load custom layout...",Command::Open)];
    let previous=item("Undo last layout change",Command::Previous);
    items.push(if crate::prefs::text("workspace_previous_layout").is_some(){previous}else{previous.off()});
    let paths=presets();
    if !paths.is_empty() {
        items.push(PItem::sep());
        items.extend(paths.iter().map(|p|item(&name(p),Command::Load(p.clone()))));
        items.push(PItem::sub("Assign layout hotkey", paths.iter().map(|p|item(&name(p),Command::Shortcut(p.clone()))).collect()));
        items.push(PItem::sub("Delete saved layout", paths.iter().map(|p|item(&name(p),Command::Delete(p.clone()))).collect()));
    }
    items
}
pub fn run(app: &mut App, command: Command) {
    match command {
        Command::Save | Command::Open => {
            let save=command==Command::Save;
            let dir=directory(); if let Some(dir)=&dir{let _=std::fs::create_dir_all(dir);}
            let dialog=crate::filedlg::FileDlg::new(if save {crate::filedlg::Mode::Save}else{crate::filedlg::Mode::Open},if save {"Save custom window layout"}else{"Load custom window layout"},"window layout",Then::WorkspaceLayout(save))
                .filter("ML Studio window layout",&[EXT]).start(dir.as_deref());
            app.file_dialog(if save {dialog.name("My workspace.lslayout")}else{dialog});
        }
        Command::Load(path) => chosen(app,&path,false),
        Command::Shortcut(path) => crate::input_bindings::layout_shortcut(app,&format!("Window layouts / {}",name(&path))),
        Command::Delete(path) => app.show_msg(Msg::new(Kind::Question,"Delete saved layout",crate::dialog::spans(&["Delete the saved layout ",&name(&path),"? The current workspace stays as it is."]),&[Answer::Yes,Answer::No],Then::DeleteWorkspaceLayout(path))),
        Command::Previous => {
            let result=crate::prefs::text("workspace_previous_layout").ok_or("No previous layout".to_string()).and_then(|v|Layout::decode(v.as_bytes()));
            match result {Ok(v)=>apply(app,v),Err(e)=>app.state.message=Some(e)}
        }
    }
}
fn apply(app: &mut App, layout: Layout) {
    if let Ok(old)=serde_json::to_string(&Layout::capture(&app.state)){crate::prefs::set_text("workspace_previous_layout",&old);}
    layout.apply(app);
}

#[derive(Serialize, Deserialize)]
struct Session {
    layout: Layout,
    project: Option<PathBuf>,
    template: Option<String>,
    editors: serde_json::Value,
}

/// Quit saves the workspace even when the musical project was not saved.
pub fn save_session(app: &App) {
    let session=Session {
        layout:Layout::capture(&app.state),
        project:app.state.path.clone(),
        template:crate::prefs::text("default_template"),
        editors:crate::layout::project_snapshot(&app.state,&app.plugins)["editors"].clone(),
    };
    if let Ok(value)=serde_json::to_string(&session) {crate::prefs::set_text("last_window_session",&value);}
}

/// Normal startup restores the last session after loading its template/project.
/// Explicit project opens continue to use their own saved layout.
pub fn restore_session(app: &mut App) {
    let Some(text)=crate::prefs::text("last_window_session").filter(|s|s.len()<=1024*1024) else {return};
    let Ok(session)=serde_json::from_str::<Session>(&text) else {return};
    let Ok(bytes)=serde_json::to_vec(&session.layout) else {return};
    let Ok(layout)=Layout::decode(&bytes) else {return};
    let same=session.project==app.state.path && (session.project.is_some() || session.template==crate::prefs::text("default_template"));
    app.startup_editor_layout=Some(serde_json::json!({"editors":if same {session.editors}else{serde_json::json!([])}}));
    layout.apply(app);
}
pub fn chosen(app: &mut App, path: &Path, save: bool) {
    let path=if save && !path.extension().is_some_and(|e|e.eq_ignore_ascii_case(EXT)){path.with_extension(EXT)}else{path.to_path_buf()};
    let result=(||->Result<(),String>{
        if save {
            let bytes=serde_json::to_vec_pretty(&Layout::capture(&app.state)).map_err(|e|e.to_string())?;
            Layout::decode(&bytes)?;
            std::fs::write(&path,bytes).map_err(|e|e.to_string())?;
        } else {
            if std::fs::metadata(&path).map_err(|e|e.to_string())?.len()>64*1024{return Err("Layout file is too large".into());}
            let bytes=std::fs::read(&path).map_err(|e|e.to_string())?;
            apply(app,Layout::decode(&bytes)?);
        }
        remember(&path); Ok(())
    })();
    match result {
        Ok(())=>app.state.message=Some(format!("Window layout {}: {}",if save{"saved"}else{"loaded"},name(&path))),
        Err(e)=>app.tell(Kind::Info,"Window layout",crate::dialog::spans(&[&format!("Could not {} layout: {e}",if save{"save"}else{"load"})])),
    }
    app.invalidate();
}
pub fn deleted(app: &mut App, path: &Path) {
    app.state.message=Some(match std::fs::remove_file(path){Ok(())=>format!("Deleted layout: {}",name(path)),Err(e)=>format!("Could not delete layout: {e}")});
    app.invalidate();
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn last_session_closed_windows_and_sizes_override_startup_project_only() {
        let mut app=App::new(None);app.state.project=crate::app::test_project();
        let mut canvas=os_ui::Canvas::new(1600,1000,1.).unwrap();app.paint(&mut canvas,None);
        let path=std::env::temp_dir().join(format!("ml-session-{}.lmn",std::process::id()));
        app.state.path=Some(path.clone());
        app.state.z=vec![Win::Playlist,Win::Mixer,Win::PianoRoll,Win::ChannelRack];
        let mut file=os_flp::project::create(&app.state.project).unwrap();
        crate::layout::write_project_layout(&mut file,crate::layout::project_snapshot(&app.state,&app.plugins));
        std::fs::write(&path,file.to_bytes().unwrap()).unwrap();
        app.key(&winit::keyboard::Key::Named(winit::keyboard::NamedKey::F12),false,false);
        assert!(app.state.z.is_empty());app.state.show_browser=false;
        app.state.rack_size=(900.,370.);app.state.rack_pos=(280.,290.);
        app.state.wins[0].rect=os_ui::Rect::new(320.,150.,1000.,650.);
        crate::layout::save(&app.state);save_session(&app);
        let mut restored=App::new(None);restored.state.project=app.state.project.clone();
        restored.state.file=Some(file);restored.state.path=Some(path.clone());
        crate::layout::restore(&mut restored.state);assert!(restored.state.shown(Win::Playlist));
        restore_session(&mut restored);restored.paint(&mut canvas,None);
        assert!(restored.state.z.is_empty());assert!(!restored.state.show_browser);
        assert_eq!(restored.state.rack_size,(900.,370.));assert_eq!(restored.state.rack_pos,(280.,290.));
        assert_eq!(restored.state.wins[0].rect,app.state.wins[0].rect);
        assert_eq!(restored.startup_editor_layout.as_ref().unwrap()["editors"],serde_json::json!([]));
        // Explicit opens clear the startup override and use the file's saved windows.
        restored.open(path.clone());assert!(restored.startup_editor_layout.is_none());assert!(restored.state.shown(Win::Playlist));
        std::fs::remove_file(path).unwrap();
        // Save/reopen a partly open workspace, preserving order and detached geometry.
        app.state.z=vec![Win::PianoRoll,Win::Mixer];app.state.native_detached=vec![Win::Mixer];
        app.state.native_frames=vec![(Win::Mixer,crate::layout::MainWin{pos:(100,200),size:(900,600),max:false})];
        save_session(&app);restore_session(&mut restored);
        assert_eq!(restored.state.z,app.state.z);assert_eq!(restored.state.native_frames,app.state.native_frames);
    }
    #[test]
    fn custom_layout_roundtrip_previous_and_invalid_are_non_destructive() {
        let mut app=App::new(None); app.state.project=crate::app::test_project();
        let mut canvas=os_ui::Canvas::new(1920,1080,1.).unwrap(); app.paint(&mut canvas,None);
        app.state.wins[0]=WinState{rect:os_ui::Rect::new(320.,400.,1000.,450.),maxed:false,mini:false};
        app.state.wins[1]=WinState{rect:os_ui::Rect::new(330.,100.,1100.,500.),maxed:false,mini:false};
        app.state.rack_pos=(600.,230.);app.state.rack_size=(630.,330.);
        app.state.z=vec![Win::Playlist,Win::Mixer,Win::ChannelRack];
        app.state.native_detached=vec![Win::Mixer];
        app.state.native_frames=vec![(Win::Mixer,crate::layout::MainWin{pos:(-1500,80),size:(1100,700),max:false})];
        app.state.browser_w=310.; app.state.show_browser=false;
        let project=app.state.project.clone();
        let saved=serde_json::to_vec(&Layout::capture(&app.state)).unwrap();
        let path=std::env::temp_dir().join(format!("ml-window-layout-{}.lslayout",std::process::id()));
        chosen(&mut app,&path,true);
        assert!(path.is_file());assert!(presets().contains(&path));
        app.state.wins[0].rect.x=700.;app.state.rack_pos=(30.,100.);app.state.show_browser=true;
        let previous=serde_json::to_value(Layout::capture(&app.state)).unwrap();
        chosen(&mut app,&path,false);
        assert_eq!(serde_json::to_value(Layout::capture(&app.state)).unwrap(),serde_json::to_value(Layout::decode(&saved).unwrap()).unwrap());
        assert_eq!(app.state.project,project);
        let mut restarted=App::new(None);restarted.state.project=project.clone();
        run(&mut restarted,Command::Load(path.clone()));
        assert_eq!(restarted.state.rack_pos,(600.,230.));assert_eq!(restarted.state.native_frames,app.state.native_frames);
        // A failed decode has no partial window edits.
        let before=serde_json::to_value(Layout::capture(&app.state)).unwrap();
        let mut bad=before.clone();bad["rack"][2]=serde_json::json!(-1);
        assert!(Layout::decode(&serde_json::to_vec(&bad).unwrap()).is_err());
        assert_eq!(serde_json::to_value(Layout::capture(&app.state)).unwrap(),before);
        // Put the previous snapshot back after the independent reopen above.
        crate::prefs::set_text("workspace_previous_layout",&previous.to_string());
        run(&mut app,Command::Previous);
        assert_eq!(serde_json::to_value(Layout::capture(&app.state)).unwrap(),previous);
        run(&mut app,Command::Previous);assert_eq!(app.state.wins[0].rect.x,320.);
        assert_eq!(app.state.project,project);
        deleted(&mut app,&path);assert!(!presets().contains(&path));
    }

    #[test]
    fn custom_layout_attach_visibility_cancel_and_about() {
        let mut app=App::new(None);app.state.project=crate::app::test_project();
        let mut canvas=os_ui::Canvas::new(1600,1000,1.25).unwrap();app.paint(&mut canvas,None);
        crate::attached_mixer::toggle(&mut app);
        let layout=serde_json::to_vec(&Layout::capture(&app.state)).unwrap();
        crate::attached_mixer::toggle(&mut app);
        let project=app.state.project.clone();
        apply(&mut app,Layout::decode(&layout).unwrap());
        assert!(app.state.playlist.attached.is_some());assert_eq!(app.state.project,project);
        run(&mut app,Command::Previous);assert!(app.state.playlist.attached.is_none());assert_eq!(app.state.project,project);
        let view=crate::views::menus::VIEW;
        let menu=crate::views::menus::submenu(view,"Arrange windows").unwrap();
        assert!(menu(&app.state).iter().any(|p|p.cmd==Cmd::WorkspaceLayout(Command::Save)));
        crate::workspace_visibility::toggle(VISIBILITY);
        assert!(items(&app.state).is_empty());assert_eq!(app.state.project,project);
        crate::dialog::fake::live(true);
        run(&mut app,Command::Save);assert!(app.modal_open());
        app.dialog_click(crate::dialog::DHit::Close);assert!(!app.modal_open());
        assert_eq!(app.state.project,project);
        app.menu_item(crate::views::menus::HELP_MENU,crate::views::menus::HELP_ABOUT);
        assert!(app.state.settings.open);assert_eq!(app.state.settings.tab,crate::views::settings::ABOUT);
        app.paint(&mut canvas,None);
        crate::dialog::fake::live(false);
        // Vintage Mixer view is stored independently of the modern view.
        app.state.vintage_2005=true;
        crate::mixer::set_layout(&mut app.state,crate::mixer::TRADITIONAL);
        let vintage=Layout::capture(&app.state);
        assert_eq!(vintage.mixer,crate::mixer::TRADITIONAL);
        crate::mixer::set_layout(&mut app.state,crate::mixer::WIDE);
        vintage.apply(&mut app);
        assert_eq!(crate::mixer::view(&app.state),crate::mixer::TRADITIONAL);
    }
}
