//! Channel-rack group edits. Relative knob offsets survive common-limit clamping.
use crate::{app::{App,State},knobs::KnobId,history::{self,Snap}};
pub const ROUTING_VISIBILITY: &str = "visible_rack-routing-menu";

/// All selected instruments take the displayed destination. Validate the whole edit before recording it.
pub fn route(s: &mut State, rows: &[usize], destination: u16, spread: bool) -> Option<String> {
    let rows: Vec<_> = rows.iter().copied().filter(|&r| s.project.channels.get(r)
        .is_some_and(|c| c.kind != os_model::ChannelKind::Automation)).collect();
    if rows.is_empty() { return Some("Select an instrument or audio channel first.".into()); }
    let max = crate::mixer::max_insert(&s.project);
    let mut edits = Vec::new();
    for (i, &r) in rows.iter().enumerate() {
        let dst = usize::from(destination) + if spread { i } else { 0 };
        if dst > usize::from(max) || !s.project.mixer.iter().any(|m| usize::from(m.index) == dst && !m.clip_fx) {
            return Some("Not enough mixer tracks for this routing. Choose an earlier track or add inserts.".into());
        }
        let src = s.project.channels[r].mixer_track;
        if crate::mixer::locked(s, src) || crate::mixer::locked(s, dst as u16) {
            return Some(crate::mixer::LOCK_MESSAGE.into());
        }
        if src != dst as u16 { edits.push((r, src, dst as u16)); }
    }
    if edits.is_empty() { return None; }
    history::record(s, "Route selected channels", Snap::Several(edits.iter().map(|&(r, src, _)| Snap::ChannelRoute(r, src)).collect()));
    for (r, _, dst) in edits { s.project.channels[r].mixer_track = dst; }
    crate::attached_mixer::clear(s);
    s.piano.dirty = true;
    Some(format!("Routed {} channel(s) {} {}", rows.len(), if spread { "starting at" } else { "to" }, crate::mixer::name(s, destination)))
}

pub fn routing_items(s: &State, row: usize) -> Vec<crate::playlist::PItem> {
    use crate::{playlist::{Cmd, PItem}, rack::RCmd};
    if !crate::workspace_visibility::shown(ROUTING_VISIBILITY) { return Vec::new(); }
    let selected = rows(s, row).len() > 1;
    let mut items = vec![PItem::new(if selected { "Route selected channels to mixer..." } else { "Route channel to mixer..." }, Cmd::Rack(RCmd::RouteSelected(row, false)))];
    if selected { items.push(PItem::new("Route selected channels to consecutive mixer tracks...", Cmd::Rack(RCmd::RouteSelected(row, true)))); }
    items
}
pub fn rows(s:&State,row:usize)->Vec<usize>{
    let rows = if row == usize::MAX { (0..s.project.channels.len()).collect() } else { crate::rack::selection_for(s, row) };
    rows.into_iter().filter(|&r|s.project.channels.get(r).is_some_and(|c|c.kind!=os_model::ChannelKind::Automation)).collect()
}
pub fn all_follow(s:&State)->bool {let r=rows(s,usize::MAX);!r.is_empty()&&r.iter().all(|&r|s.project.channels[r].misc.main_pitch)}
pub fn toggle_follow(app:&mut App,row:usize){
    let rows=rows(&app.state,row);if rows.is_empty(){return;}
    let on=if row==usize::MAX {!all_follow(&app.state)}else{!app.state.project.channels[row].misc.main_pitch};
    let snaps=rows.iter().map(|&r|{let c=&app.state.project.channels[r];Snap::PitchFollow(r,c.misc.main_pitch,c.misc.pitch_follow_offset)}).collect();
    history::record(&mut app.state,"Global Pitch Follow",Snap::Several(snaps));
    let global=i32::from(app.state.project.main_pitch.unwrap_or(0));
    for r in rows {let c=&mut app.state.project.channels[r];if c.misc.main_pitch!=on {c.misc.pitch_follow_offset+=if on {-global}else{global};c.misc.main_pitch=on;}
        if c.kind.is_sampler(){app.plugins.set_param_named(r,"Affected by the main pitch",f64::from(on));}
    }
    app.state.piano.dirty=true;app.invalidate();
}
pub fn group(s:&State,id:KnobId)->Vec<(KnobId,f32)>{
    let row=match id {KnobId::ChanVol(r)|KnobId::ChanPan(r)|KnobId::ChanPitch(r)=>r,_=>return Vec::new()};
    rows(s,row).into_iter().map(|r|{let k=match id {KnobId::ChanVol(_)=>KnobId::ChanVol(r),KnobId::ChanPan(_)=>KnobId::ChanPan(r),_=>KnobId::ChanPitch(r)};(k,s.knob(k))}).collect()
}
fn units(s:&State,id:KnobId)->f32{match id{KnobId::ChanPitch(r)=>200.*s.project.channels[r].misc.pitch_range.max(1) as f32,_=>1.}}
pub fn apply(s:&mut State,id:KnobId,base:f32,to:f32,group:&[(KnobId,f32)]){
    if group.is_empty(){s.set_knob(id,to);return;}
    let mut delta=(to-base)*units(s,id);
    for &(k,v) in group {let u=units(s,k);delta=delta.clamp(-v*u,(1.-v)*u);}
    for &(k,v) in group {let next=v+delta/units(s,k);s.set_knob(k,next);}
}
pub fn snapshot(s:&mut State,id:KnobId,wheel:bool){
    let group=group(s,id);
    if group.len()<2 {history::knob(s,id,wheel);return;}
    if !crate::prefs::check(&crate::prefs::UNDO_KNOB_TWEAKS){return;}
    let snaps=group.into_iter().map(|(k,_)|Snap::Knob(k,history::knob_value(s,k))).collect();
    history::record(s,"Selected channel controls",Snap::Several(snaps));
}
pub fn toggle_enabled(s:&mut State,row:usize){
    let Some(c)=s.project.channels.get(row)else{return};let on=!c.enabled;
    let rows: Vec<_> = crate::rack::selection_for(s, row).into_iter().filter(|&r| r < s.project.channels.len()).collect();
    let snaps=rows.iter().map(|&r|Snap::ChannelEnabled(r,s.project.channels[r].enabled)).collect();
    history::record(s,"Selected channels on/off",Snap::Several(snaps));
    for r in rows {s.project.channels[r].enabled=on;}s.piano.dirty=true;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::{Action,Win};
    #[test]
    fn rack_additions_fit_channels_preserve_manual_sizes_and_cap_to_workspace() {
        use crate::rack::{self,AutoResize,RCmd};
        for vintage in [false,true] {
            let mut app=App::new(None);app.state.project=crate::app::test_project();app.state.vintage_2005=vintage;
            app.state.z=vec![Win::ChannelRack];app.state.show_browser=false;
            let mut canvas=os_ui::Canvas::new(1600,1000,1.).unwrap();app.paint(&mut canvas,None);
            app.state.rack_pos=(300.,700.);app.state.rack_size=(900.,200.);
            app.paint(&mut canvas,None);assert_eq!(app.state.rack_size,(900.,200.),"repaint keeps a manual size");
            for _ in 0..5 {crate::app::add_channel(&mut app.state.project);}
            app.paint(&mut canvas,None);
            let panel=crate::views::rack::panel(&app.state);
            assert!(panel.h>=crate::views::rack::content_h(&app.state));assert_eq!(app.state.rack_size.0,900.);
            assert!(crate::views::rack::rect(&app.state).bottom()<=app.state.screen.1);
            assert!(app.state.rack_pos.1<700.);
            for _ in 0..70 {crate::app::add_channel(&mut app.state.project);}
            app.paint(&mut canvas,None);
            let rect=crate::views::rack::rect(&app.state);
            assert!(rect.y>=app.state.work_rect().y && rect.bottom()<=app.state.screen.1);
            assert!(crate::views::rack::content_h(&app.state)>crate::views::rack::panel(&app.state).h);
            rack::run(&mut app,RCmd::AutoResize(AutoResize::Off));let height=app.state.rack_size.1;
            crate::app::add_channel(&mut app.state.project);app.paint(&mut canvas,None);assert_eq!(app.state.rack_size.1,height);
            assert_eq!(App::new(None).state.rack.auto_resize,AutoResize::Off);
            rack::run(&mut app,RCmd::AutoResize(AutoResize::Min));
        }
    }
    #[test]
    fn selected_rack_drag_routes_together_and_escape_cancels() {
        let mut app=App::new(None);app.state.project=crate::app::test_project();
        app.state.z=vec![Win::Mixer,Win::ChannelRack];app.state.rack.sel=[0,1].into();
        let original=app.state.project.clone();
        let mut canvas=os_ui::Canvas::new(1920,1080,1.25).unwrap();app.paint(&mut canvas,None);
        let r=app.hits.iter().find(|h|h.action==Action::RackMixer(0)).unwrap().rect;
        app.mouse_move(r.cx(),r.cy());app.mouse_down();app.mouse_move(r.cx(),r.cy()-25.);
        app.key(&winit::keyboard::Key::Named(winit::keyboard::NamedKey::Escape),false,false);
        app.mouse_up();assert_eq!(app.state.project,original);
        app.mouse_move(r.cx(),r.cy());app.mouse_down();app.mouse_move(r.cx(),r.cy()-25.);
        app.state.raise(Win::Mixer);app.paint(&mut canvas,None);
        let dst=app.hits.iter().find(|h|h.action==Action::Mix(crate::mixer::MHit::Strip(8))).unwrap().rect;
        app.mouse_move(dst.cx(),dst.y+15.);app.mouse_up();
        assert_eq!(app.state.project.channels[0].mixer_track,8);assert_eq!(app.state.project.channels[1].mixer_track,8);
        assert_eq!(app.state.project.channels[2],original.channels[2]);
        history::undo(&mut app.state);assert_eq!(app.state.project,original);
        history::redo(&mut app.state);assert_eq!(app.state.project.channels[1].mixer_track,8);
    }
    #[test]
    fn selected_rack_routes_picker_wheel_cancel_undo_and_roundtrip() {
        use crate::{dialog::{Body,DHit},playlist::Cmd,rack::RCmd};
        let mut app=App::new(None);app.state.project=crate::app::test_project();app.state.z=vec![Win::ChannelRack];app.state.show_browser=false;
        app.state.rack.select_only(0);
        let mut canvas=os_ui::Canvas::new(1600,1000,1.).unwrap();app.paint(&mut canvas,None);
        let selector=app.hits.iter().find(|h|h.action==Action::RackSelect(1)).unwrap().rect;
        app.shift=true;app.mouse_move(selector.cx(),selector.cy());app.mouse_down();app.mouse_up();app.shift=false;
        assert_eq!(app.state.rack.sel.iter().copied().collect::<Vec<_>>(),vec![0,1]);
        let original=app.state.project.clone();
        let menu=crate::rack::channel_menu(&app.state,0,0.,0.);
        let command=menu.items.iter().find(|p|p.cmd==Cmd::Rack(RCmd::RouteSelected(0,false))).unwrap().cmd.clone();
        crate::dialog::fake::live(true);
        crate::playlist::run(&mut app,command.clone());
        let Body::RoutePicker(f)=&app.state.dialogs.last().unwrap().body else {panic!("picker")};
        assert_eq!(f.rack,Some((vec![0,1],false)));
        app.dialog_click(DHit::Close);assert_eq!(app.state.project,original);
        crate::playlist::run(&mut app,command);
        app.dialog_click(DHit::RoutePick(10));
        assert_eq!(app.state.project.channels[0].mixer_track,10);assert_eq!(app.state.project.channels[1].mixer_track,10);
        assert_eq!(app.state.project.channels[2],original.channels[2]);
        history::undo(&mut app.state);assert_eq!(app.state.project,original);history::redo(&mut app.state);
        app.paint(&mut canvas,None);
        let number=app.hits.iter().find(|h|h.action==Action::RackMixer(1)).unwrap().rect;
        app.mouse_move(number.cx(),number.cy());app.wheel(0.,1.,false,false);
        assert_eq!(app.state.project.channels[0].mixer_track,11);assert_eq!(app.state.project.channels[1].mixer_track,11);
        history::undo(&mut app.state);assert_eq!(app.state.project.channels[0].mixer_track,10);
        crate::playlist::run(&mut app,Cmd::Rack(RCmd::RouteSelected(0,true)));
        app.dialog_click(DHit::RoutePick(12));
        assert_eq!(app.state.project.channels[0].mixer_track,12);assert_eq!(app.state.project.channels[1].mixer_track,13);
        let file=os_flp::project::create(&app.state.project).unwrap();
        let loaded=os_flp::project::load(&os_flp::FlpFile::parse(&file.to_bytes().unwrap()).unwrap()).unwrap();
        assert_eq!(loaded.channels[0].mixer_track,12);assert_eq!(loaded.channels[1].mixer_track,13);
        let before=app.state.project.clone();
        crate::mixer::track_mut(&mut app.state,13).unwrap().edit_locked=true;
        let locked=app.state.project.clone();let count=app.state.history.done();
        assert_eq!(route(&mut app.state,&[0,1],14,false).as_deref(),Some(crate::mixer::LOCK_MESSAGE));
        assert_eq!(app.state.project,locked);assert_eq!(app.state.history.done(),count);
        crate::mixer::track_mut(&mut app.state,13).unwrap().edit_locked=false;
        let max=crate::mixer::max_insert(&app.state.project);
        assert!(route(&mut app.state,&[0,1],max,true).unwrap().contains("Not enough"));assert_eq!(app.state.project,before);
        // A control on an unselected channel edits only that channel.
        let third=rows(&app.state,2);assert_eq!(third,vec![2]);route(&mut app.state,&third,3,false);
        assert_eq!(app.state.project.channels[0].mixer_track,12);
        crate::workspace_visibility::toggle(ROUTING_VISIBILITY);assert!(routing_items(&app.state,0).is_empty());
        assert_eq!(app.state.rack.sel.iter().copied().collect::<Vec<_>>(),vec![0,1]);
        crate::dialog::fake::live(false);
    }
    #[test]
    fn picture_batch_group_controls_and_octave_wheel(){
        let mut a=App::new(None);a.state.project=crate::app::test_project();a.state.rack.sel=[0,1].into();
        a.state.project.channels[0].misc.pitch_range=24;a.state.project.channels[1].misc.pitch_range=12;
        a.state.project.channels[0].pitch_cents=0;a.state.project.channels[1].pitch_cents=-1200;
        a.state.z=vec![Win::ChannelRack];a.state.show_browser=false;
        let mut c=os_ui::Canvas::new(1280,900,1.).unwrap();a.paint(&mut c,None);
        let id=KnobId::ChanPitch(0);let r=a.hits.iter().find(|h|h.action==Action::Knob(id)).unwrap().rect;
        a.mouse_move(r.cx(),r.cy());a.wheel(0.,1.,false,true);
        assert_eq!((a.state.project.channels[0].pitch_cents,a.state.project.channels[1].pitch_cents),(1200,0));
        history::undo(&mut a.state);assert_eq!((a.state.project.channels[0].pitch_cents,a.state.project.channels[1].pitch_cents),(0,-1200));
        history::redo(&mut a.state);assert_eq!(a.state.project.channels[0].pitch_cents,1200);
        for id in [KnobId::ChanVol(0),KnobId::ChanPan(0)] {
            a.state.set_knob(id,0.25);let other=match id{KnobId::ChanVol(_)=>KnobId::ChanVol(1),_=>KnobId::ChanPan(1)};a.state.set_knob(other,0.5);
            let g=group(&a.state,id);snapshot(&mut a.state,id,false);apply(&mut a.state,id,0.25,1.,&g);
            assert!((a.state.knob(id)-0.75).abs()<1e-6);assert_eq!(a.state.knob(other),1.);
            history::undo(&mut a.state);assert_eq!(a.state.knob(id),0.25);assert_eq!(a.state.knob(other),0.5);
        }
        a.state.project.channels[0].enabled=true;toggle_enabled(&mut a.state,0);assert!(!a.state.project.channels[0].enabled&&!a.state.project.channels[1].enabled);
        history::undo(&mut a.state);assert!(a.state.project.channels[0].enabled);
        a.state.project.main_pitch=Some(300);let before=a.state.project.channels[2].misc.main_pitch;
        toggle_follow(&mut a,0);assert!(!a.state.project.channels[0].misc.main_pitch&&!a.state.project.channels[1].misc.main_pitch);assert_eq!(a.state.project.channels[2].misc.main_pitch,before);
        assert_eq!(a.state.project.channels[0].misc.pitch_follow_offset,300);
        toggle_follow(&mut a,usize::MAX);assert!(all_follow(&a.state));assert_eq!(a.state.project.channels[0].misc.pitch_follow_offset,0);
    }
    #[test]
    fn picture_batch_layout_and_preset_drop(){
        let mut a=App::new(None);a.state.project=crate::app::test_project();a.plugins.available=crate::plugins::builtin();a.sync_samplers(None);
        a.state.z=vec![Win::ChannelRack];a.state.show_browser=true;a.state.rack_pos=(340.,120.);
        let mut c=os_ui::Canvas::new(1280,900,1.).unwrap();a.paint(&mut c,None);
        let r=a.hits.iter().find(|h|h.action==Action::ChannelSelect(0)).unwrap().rect;
        a.mouse_move(r.cx(),r.cy());a.mouse_down();a.mouse_move(100.,300.);a.mouse_up();
        assert!(a.state.message.as_deref().unwrap_or("").starts_with("Saved preset"),"{:?}",a.state.message);
        let first=a.plugins.presets[&0].clone();let bytes=std::fs::read(&first).unwrap();assert!(crate::presets::decode(&bytes).is_some());
        crate::presets::save_channel_drop(&mut a,0);let second=a.plugins.presets[&0].clone();assert_ne!(first,second);assert_eq!(std::fs::read(first).unwrap(),bytes);
        assert_eq!(a.state.browser_tab,4);
        a.state.project.channels[1].misc.main_pitch=false;
        a.paint(&mut c,None);
        if let Some(dir)=std::env::var_os("OS_SHOT_DIR"){c.pixmap.save_png(std::path::PathBuf::from(&dir).join("rack-batch.png")).unwrap();}
        crate::piano_tools::run(&mut a,crate::piano_tools::NCmd::KeyboardStyle(true));assert!(crate::piano_tools::classic_keyboard());
        a.state.z=vec![Win::PianoRoll];a.state.show_browser=false;a.state.piano.scroll_y=900.;a.paint(&mut c,None);
        if let Some(dir)=std::env::var_os("OS_SHOT_DIR"){c.pixmap.save_png(std::path::PathBuf::from(dir).join("piano-batch.png")).unwrap();}
        crate::piano_tools::run(&mut a,crate::piano_tools::NCmd::KeyboardStyle(false));assert!(!crate::piano_tools::classic_keyboard());assert_eq!(crate::prefs::text("piano_keyboard_style").as_deref(),Some("piano"));
        a.paint(&mut c,None);
        if let Some(dir)=std::env::var_os("OS_SHOT_DIR"){c.pixmap.save_png(std::path::PathBuf::from(dir).join("piano-shaped.png")).unwrap();}
        crate::piano_tools::run(&mut a,crate::piano_tools::NCmd::KeyboardStyle(true));
    }
}
