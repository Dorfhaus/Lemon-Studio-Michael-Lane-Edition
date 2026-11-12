// SPDX-License-Identifier: GPL-3.0-or-later
//! Optional track controls in the ordinary Playlist. Binding happens on an edit,
//! never while drawing headers or opening a picker.
use crate::{app::{Action,App,State},attached_mixer as attached,mixer,playlist as pl};
use os_model::{ChannelKind,ClipSource};

pub const VISIBILITY:&str="visible_playlist-direct-record-instrument";
pub(crate) const KEYBOARD_PICKER: crate::prefs::Check = crate::prefs::Check {
    key: "playlist_keyboard_right_click_picker",
    label: "Keyboard right-click opens Plugin Picker",
    default: false,
};

pub(crate) fn keyboard_picker_item() -> pl::PItem {
    pl::PItem::new(KEYBOARD_PICKER.label, pl::Cmd::Plugin(crate::plugin_menu::PlugCmd::TogglePlaylistKeyboardPicker))
        .mark(pl::Mark::Check(crate::prefs::check(&KEYBOARD_PICKER)))
}

pub(crate) fn open_instrument_menu(app: &mut App, target: crate::plugin_menu::Target) {
    if crate::prefs::check(&KEYBOARD_PICKER) {
        crate::plugin_picker::open(app, target);
    } else {
        app.open_plugin_menu(target, app.mouse);
        if let Some(menu) = &mut app.state.playlist.popup {
            menu.items.insert(1, keyboard_picker_item());
        }
    }
}

pub fn enabled()->bool{crate::workspace_visibility::shown(VISIBILITY)}
pub fn buttons(s:&State,row:u16)->bool{
    if attached::active(s){return attached::owner(s,row).is_some();}
    enabled()&&!pl::track_info(s,row).is_some_and(|t|t.folder)
}
/// Cheap explicit ownership for painting/arming; no scan through pattern notes.
pub fn owner(s:&State,row:u16)->Option<u16>{
    if attached::active(s){return attached::owner(s,row);}
    pl::track_info(s,row).and_then(|t|t.attached_mixer.or(t.comp))
        .filter(|n|*n>0&&mixer::track(s,*n).is_some_and(|m|!m.clip_fx))
}
pub fn bound_lane(s:&State,n:u16)->Option<u16>{
    let valid=|row|pl::track_info(s,row).is_some_and(|t|t.attached_mixer==Some(n)&&!t.folder&&!t.locked&&t.frozen.is_none());
    if valid(s.playlist.track){Some(s.playlist.track)}else{(0..pl::TRACKS).find(|r|valid(*r))}
}
fn writable(s:&State,row:u16)->bool{row<pl::TRACKS&&!pl::track_info(s,row).is_some_and(|t|t.locked||t.frozen.is_some()||t.folder)}
fn instrument(s:&State,row:u16)->Option<usize>{
    let eligible=|ch:u16|s.project.channels.iter().position(|c|c.index==ch&&matches!(c.kind,ChannelKind::Plugin|ChannelKind::Sampler));
    let arr=s.project.arrangement()?.number;
    if let Some(ch)=s.project.instrument_lanes.iter().rev().find(|v|v.1==arr&&v.2==row).and_then(|v|eligible(v.0)){return Some(ch);}
    if let Some(ch)=owner(s,row).and_then(|n|attached::instrument(s,n)){return Some(ch);}
    let mut channels=std::collections::BTreeSet::new();
    for clip in pl::clips(s).iter().filter(|c|c.track==row){match clip.source{
        ClipSource::Channel(ch)=>{if let Some(ch)=eligible(ch){channels.insert(ch);}},
        ClipSource::Pattern(p)=>{if let Some(p)=s.project.pattern(p){for n in &p.notes{if let Some(ch)=eligible(n.channel){channels.insert(ch);}}}},
    }}
    (channels.len()==1).then(||*channels.first().unwrap())
}
pub fn piano(app:&mut App,row:u16,right_click:bool)->Option<String>{
    if !writable(&app.state,row){return Some("Unlock or unfreeze this track before editing instruments".into());}
    if right_click {
        open_instrument_menu(app, crate::plugin_menu::Target::PlaylistInstrument(row));
        return None;
    }
    if let Some(ch)=instrument(&app.state,row){app.act(Action::RackRoll(ch));return None;}
    let index=app.plugins.available.iter().position(|p|p.desc.name==crate::plugins::LEMON_SAMPLER)?;
    let result=insert(app,index,row);
    if let Some(ch)=instrument(&app.state,row){app.state.pending_open=None;app.act(Action::RackRoll(ch));}
    result
}
pub fn insert(app:&mut App,index:usize,row:u16)->Option<String>{
    if !writable(&app.state,row){return Some("Unlock or unfreeze this track before inserting an instrument".into());}
    if let Some(n)=owner(&app.state,row){attached::insert_on_lane(app,index,n,row)}
    else{attached::add_instrument_at(app,index,row)}
}
pub fn input_menu(app:&mut App,row:u16)->Option<String>{
    if !writable(&app.state,row){return Some("Unlock or unfreeze this track before recording".into());}
    let n=owner(&app.state,row).unwrap_or(u16::MAX);
    let mut menu=crate::mixer_recording::input_menu(&app.state,n,app.mouse.0,app.mouse.1+10.);
    for item in &mut menu.items{if let pl::Cmd::Mixer(mixer::MCmd::RecordInput(_,input))=&item.cmd{item.cmd=pl::Cmd::Mixer(mixer::MCmd::PlaylistInput(row,input.clone()));}}
    app.state.playlist.popup=Some(menu);app.invalidate();None
}
pub fn record(app:&mut App,row:u16)->Option<String>{
    if !writable(&app.state,row){return Some("Unlock or unfreeze this track before recording".into());}
    let n=owner(&app.state,row);
    if let Some(n)=n{if mixer::locked(&app.state,n){return Some(mixer::LOCK_MESSAGE.into());}}
    if n.is_none_or(|n|!app.state.mixer.recording.inputs.contains_key(&n)){
        if let Some(ch)=instrument(&app.state,row){
            if mixer::locked(&app.state,app.state.project.channels[ch].mixer_track){return Some(mixer::LOCK_MESSAGE.into());}
            if app.audio_rec.is_some(){return Some("Stop the current take before changing recording tracks".into());}
            let armed=app.state.selected_channel==ch&&app.state.playlist.track==row&&app.state.rec.armed&&crate::record::Opt::Notes.on();
            app.end_take();let s=&mut app.state;let arr=pl::arr_mut(s).number;let id=s.project.channels[ch].index;
            if !s.project.instrument_lanes.iter().any(|v|v.0==id&&v.1==arr&&v.2==row){
                crate::history::record(s,"Set instrument recording track",crate::history::Snap::InstrumentLanes(s.project.instrument_lanes.clone()));
                s.project.instrument_lanes.retain(|v|v.0!=id||v.1!=arr);s.project.instrument_lanes.push((id,arr,row,s.pattern.max(1)));
            }
            s.selected_channel=ch;s.rack.select_only(ch);s.selected_mixer=s.project.channels[ch].mixer_track;s.playlist.track=row;s.song_mode=true;
            crate::mixer_recording::flag(crate::record::Opt::Notes,!armed);crate::mixer_recording::flag(crate::record::Opt::Smart,true);
            s.rec.armed=!armed||!s.mixer.recording.armed.is_empty();app.invalidate();return None;
        }
    }
    app.state.playlist.track=row;
    if let Some(n)=n.filter(|n|app.state.mixer.recording.inputs.contains_key(n)){app.arm_mixer_disk(n)}else{input_menu(app,row)}
}
pub fn armed(s:&State,row:u16)->bool{
    if let Some(n)=owner(s,row){if s.mixer.recording.inputs.contains_key(&n){return s.mixer.recording.armed.contains(&n);}}
    s.playlist.track==row&&s.rec.armed&&crate::record::Opt::Notes.on()&&s.project.channels.get(s.selected_channel).is_some_and(|ch|s.project.instrument_lanes.iter().any(|v|v.0==ch.index&&Some(v.1)==s.project.arrangement().map(|a|a.number)&&v.2==row))
}
pub fn set_input(app:&mut App,row:u16,input:Option<crate::capture::Input>)->Option<String>{
    if !writable(&app.state,row){return Some("Unlock or unfreeze this track before recording".into());}
    if app.audio_rec.is_some(){return Some("Stop the current take before changing its input".into());}
    let n=if let Some(n)=owner(&app.state,row){n}else{
        if input.is_none(){return None;}
        if mixer::inserts(&app.state)>=4093{return Some("The mixer has reached its track limit".into());}
        let s=&mut app.state;let snapshot=pl::capture(s);let selected=s.selected_mixer;let previous=s.history.begin_sequence();
        let name=pl::track_name(s,row);let color=pl::track_info(s,row).and_then(|t|t.color);
        let n=mixer::add_track(s,None);attached::bind(s,row,n);
        if let Some(m)=mixer::track_mut(s,n){m.name=Some(name);m.color=color;}
        s.history.wrap_attached_instrument(snapshot,n,selected);s.history.end_sequence_named(previous,"Create recording track");n
    };
    app.state.playlist.track=row;app.set_mixer_input(n,input)
}
