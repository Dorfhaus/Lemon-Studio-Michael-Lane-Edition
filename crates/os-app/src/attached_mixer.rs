// SPDX-License-Identifier: GPL-3.0-or-later
//! A second presentation of the existing mixer. No audio graph or clip relocation on attach.
use std::{cell::RefCell, collections::{BTreeMap, BTreeSet, HashMap}, rc::Rc};
use os_model::{ChannelKind, ClipSource};
use crate::{app::{App, State, Win, WinState}, mixer, playlist as pl};

pub const VISIBILITY: &str = "visible_playlist-attached-mixer";
pub const BUTTON_VISIBILITY: &str = "visible_mixer-attach-to-playlist";
pub(crate) const FULL_INSPECTOR_SETTING: &str = "attached_inspector_full_strip";
pub const RECORD_VISIBILITY: &str = "visible_playlist-track-record";
pub const WIDTH: f32 = 192.;

pub(crate) const EQ_VISIBILITY: &str = "visible_attached-inspector-eq";
pub(crate) const EFFECTS_VISIBILITY: &str = "visible_attached-inspector-inserts";
pub(crate) const SENDS_VISIBILITY: &str = "visible_attached-inspector-sends";
pub(crate) const FADER_VISIBILITY: &str = "visible_attached-inspector-fader";
pub(crate) const INPUT_VISIBILITY: &str = "visible_attached-inspector-input";
pub(crate) const ADD_SLOT_VISIBILITY: &str = "visible_attached-inspector-add-slot";
pub(crate) const ROUTING_VISIBILITY: &str = "visible_attached-inspector-routing";
pub(crate) const FX_VISIBILITY: &str = "visible_attached-inspector-fx";
pub(crate) const INSPECTOR_VISIBILITY: &[(&str, &str)] = &[
    ("Inspector EQ", EQ_VISIBILITY),
    ("Inspector insert effects", EFFECTS_VISIBILITY),
    ("Inspector sends", SENDS_VISIBILITY),
    ("Inspector fader and strip", FADER_VISIBILITY),
    ("Inspector input selector", INPUT_VISIBILITY),
    ("Inspector add slot button", ADD_SLOT_VISIBILITY),
    ("Inspector routing button", ROUTING_VISIBILITY),
    ("Inspector FX bypass button", FX_VISIBILITY),
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
pub enum Section {
    Eq,
    Effects,
    Sends,
}

pub(crate) const EFFECT_ROW_HEIGHT: u16 = 18;
pub(crate) const SEND_ROW_HEIGHT: u16 = 21;
pub(crate) const LIST_HEADER_HEIGHT: u16 = 22;

impl Section {
    pub(crate) const ALL: [Self; 3] = [Self::Eq, Self::Effects, Self::Sends];

    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::Eq => "EQ",
            Self::Effects => "Insert Effects",
            Self::Sends => "Sends",
        }
    }

    pub(crate) fn shown(self) -> bool {
        use crate::workspace_visibility::shown;
        match self {
            Self::Eq => shown(EQ_VISIBILITY) && shown("visible_mixer-eq"),
            Self::Effects => shown(EFFECTS_VISIBILITY),
            Self::Sends => shown(SENDS_VISIBILITY),
        }
    }

    pub(crate) fn min_height(self) -> u16 {
        match self {
            Self::Eq => 58,
            Self::Effects => LIST_HEADER_HEIGHT + EFFECT_ROW_HEIGHT,
            Self::Sends => LIST_HEADER_HEIGHT + SEND_ROW_HEIGHT,
        }
    }

    pub(crate) fn constrain(self, height: f32, limit: u16) -> u16 {
        let min = self.min_height();
        let max = limit.min(match self { Self::Eq => 440, Self::Effects => 238, Self::Sends => 358 }).max(min);
        let height = height.round().clamp(f32::from(min), f32::from(max));
        let row = match self {
            Self::Effects => EFFECT_ROW_HEIGHT,
            Self::Sends => SEND_ROW_HEIGHT,
            Self::Eq => return height as u16,
        };
        let rows = ((height - f32::from(min)) / f32::from(row)).round() as u16;
        min + rows.min((max-min)/row) * row
    }
}

#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)]
pub(crate) struct InspectorLayout {
    // Keep the fourth legacy entry so existing saved section heights still load.
    heights: [u16; 4],
    order: [Section; 3],
    pub effects_collapsed: bool,
    pub sends_collapsed: bool,
}

impl Default for InspectorLayout {
    fn default() -> Self {
        Self { heights: [0;4], order: Section::ALL, effects_collapsed: false, sends_collapsed: false }
    }
}

impl InspectorLayout {
    pub fn height(&self, section: Section, full: bool) -> u16 {
        let custom = self.heights[section as usize];
        if custom != 0 { return section.constrain(f32::from(custom), u16::MAX); }
        match section {
            Section::Eq => if full { 84 } else { 186 },
            Section::Effects => 76,
            Section::Sends => 127,
        }
    }

    pub fn order(&self) -> [Section; 3] {
        if Section::ALL.iter().all(|part| self.order.contains(part)) { self.order } else { Section::ALL }
    }

    pub fn move_section(&mut self, section: Section, delta: i8) {
        let mut order = self.order();
        if let Some(index) = order.iter().position(|&part| part == section) {
            let target = index.saturating_add_signed(isize::from(delta)).min(order.len()-1);
            order.swap(index, target);
        }
        self.order = order;
    }

    pub fn resize(&mut self, section: Section, height: u16) {
        self.heights[section as usize] = height;
    }
}

pub(crate) struct InspectorLayouts([InspectorLayout; 2]);

impl Default for InspectorLayouts {
    fn default() -> Self {
        Self([false, true].map(|full| {
            crate::prefs::text(Self::key(full))
                .and_then(|value| serde_json::from_str(&value).ok())
                .unwrap_or_default()
        }))
    }
}

impl InspectorLayouts {
    fn key(full: bool) -> &'static str {
        if full { "attached_inspector_full_layout" } else { "attached_inspector_traditional_layout" }
    }

    pub fn get(&self, full: bool) -> &InspectorLayout {
        &self.0[usize::from(full)]
    }

    pub fn get_mut(&mut self, full: bool) -> &mut InspectorLayout {
        &mut self.0[usize::from(full)]
    }

    pub fn save(&self, full: bool) {
        if let Ok(value) = serde_json::to_string(self.get(full)) {
            crate::prefs::set_text(Self::key(full), &value);
        }
    }
}

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct Window {
    rect: [f32;4], maxed: bool, mini: bool, shown: bool, detached: bool,
}
impl Window {
    fn capture(s:&State,w:Win)->Self {let v=s.wins[w.slot().unwrap()];Self {rect:[v.rect.x,v.rect.y,v.rect.w,v.rect.h],maxed:v.maxed,mini:v.mini,shown:s.shown(w),detached:s.native_detached.contains(&w)}}
    fn restore(&self,s:&mut State,w:Win) {
        let [x,y,width,height]=self.rect;
        if self.rect.iter().all(|n|n.is_finite() && n.abs()<100000.) && width>=100. && height>=50. {
            s.wins[w.slot().unwrap()]=WinState{rect:os_ui::Rect::new(x,y,width,height),maxed:self.maxed,mini:self.mini};
        }
        s.native_detached.retain(|v|*v!=w);
        if self.detached{s.native_detached.push(w);}
        if self.shown{s.raise(w);}else{s.hide(w);}
    }
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct Attached {
    playlist: Window, mixer: Window, scroll: (f32,f32),
    #[serde(skip)] pub sends_scroll: std::cell::Cell<usize>,
    #[serde(skip)] cache: RefCell<Option<Rc<Projection>>>,
}
#[derive(Debug, Clone, PartialEq)]
pub struct Projection {
    pub arrangement: Option<u16>,
    pub order: Vec<u16>,
    pub owners: Vec<Option<u16>>,
    pub buses: Vec<u16>,
}
pub fn active(s:&State)->bool{s.playlist.attached.is_some()}
pub fn full_strip(s: &State) -> bool {
    active(s) && prefers_full_strip()
}
pub(crate) fn prefers_full_strip() -> bool {
    crate::settings_file::flag(FULL_INSPECTOR_SETTING, false)
}
pub fn width(s:&State)->f32{if active(s)&&crate::workspace_visibility::shown(VISIBILITY){WIDTH}else{0.}}
pub fn clear(s:&State){if let Some(a)=&s.playlist.attached{a.cache.borrow_mut().take();}}
pub fn projection(s:&State)->Rc<Projection>{
    let Some(a)=&s.playlist.attached else{return Rc::new(Projection::build(s));};
    let mut cache=a.cache.borrow_mut();
    if cache.as_ref().is_none_or(|p|p.arrangement!=s.project.arrangement().map(|a|a.number)) {*cache=Some(Rc::new(Projection::build(s)));}
    cache.as_ref().unwrap().clone()
}
pub fn owner(s:&State,row:u16)->Option<u16>{active(s).then(||projection(s).owners.get(row as usize).copied().flatten()).flatten()}
pub fn lane(s:&State,mixer:u16)->Option<u16>{
    if !active(s){return crate::playlist_tracks::bound_lane(s,mixer);}
    let p=projection(s);
    p.order.iter().copied().find(|r|p.owners[*r as usize]==Some(mixer) && !pl::track_info(s,*r).is_some_and(|t|t.grouped))
}

impl Projection {
    /// One pass through source assignments, then stable grouped rows sorted by mixer number.
    /// Mixed-source rows stay together in the ordinary arrangement section. Never split a pattern.
    pub fn build(s:&State)->Self {
        let mut p=Self{arrangement:s.project.arrangement().map(|a|a.number),order:Vec::with_capacity(pl::TRACKS as usize),owners:vec![None;pl::TRACKS as usize],buses:vec![0]};
        let channels:HashMap<_,_>=s.project.channels.iter().filter(|c|c.kind!=ChannelKind::Automation).map(|c|(c.index,c.mixer_track)).collect();
        let patterns:HashMap<_,BTreeSet<_>>=s.project.patterns.iter().map(|pat|(pat.number,pat.notes.iter().filter_map(|n|channels.get(&n.channel).copied()).collect())).collect();
        let mut routes=vec![BTreeSet::new();pl::TRACKS as usize];
        let mut occupied=vec![false;pl::TRACKS as usize];
        for c in pl::clips(s) {
            let row=c.track as usize;if row>=routes.len(){continue;}occupied[row]=true;
            match c.source {
                ClipSource::Channel(ch)=>{if let Some(&m)=channels.get(&ch){routes[row].insert(m);}},
                ClipSource::Pattern(n)=>{if let Some(ms)=patterns.get(&n){routes[row].extend(ms);}},
            }
        }
        for &(ch,arr,row,_) in &s.project.instrument_lanes {
            if Some(arr)==p.arrangement && row<pl::TRACKS {occupied[row as usize]=true;if let Some(&m)=channels.get(&ch){routes[row as usize].insert(m);}}
        }
        let mut blocks:BTreeMap<u16,Vec<u16>>=BTreeMap::new();let mut fallback=Vec::new();let mut free=Vec::new();
        let mut first=0;
        while first<pl::TRACKS {
            let end=(first+1..pl::TRACKS).find(|r|!pl::track_info(s,*r).is_some_and(|t|t.grouped)).unwrap_or(pl::TRACKS);
            let mut owners=BTreeSet::new();let mut used=end>first+1;
            for row in first..end {
                owners.extend(&routes[row as usize]);
                if let Some(t)=pl::track_info(s,row){if let Some(m)=t.attached_mixer.or(t.comp){owners.insert(m);}
                    used|=t.name.is_some()||t.folder||t.frozen.is_some()||t.locked||t.attached_mixer.is_some()||t.comp.is_some();}
                used|=occupied[row as usize];
            }
            if owners.len()==1 && owners.first().is_some_and(|m|*m>0&&*m<=mixer::inserts(s)&&mixer::track(s,*m).is_some_and(|t|!t.clip_fx)) {
                let m=*owners.first().unwrap();blocks.entry(m).or_default().extend(first..end);
                for row in first..end{p.owners[row as usize]=Some(m);}
            }else if used {fallback.extend(first..end);}else{free.extend(first..end);}
            first=end;
        }
        let mut free=free.into_iter();
        let direct:BTreeSet<_>=channels.values().copied().chain(blocks.keys().copied()).chain(s.mixer.recording.inputs.keys().copied()).collect();
        let receives:BTreeSet<_>=s.project.mixer.iter().flat_map(|m|m.routes.iter().copied()).collect();
        for n in 1..=mixer::inserts(s) {
            if mixer::track(s,n).is_some_and(|m|m.clip_fx){continue;}
            if receives.contains(&n) && !direct.contains(&n) {p.buses.push(n);continue;}
            if let Some(rows)=blocks.remove(&n){p.order.extend(rows);}
            else if let Some(row)=free.next(){p.owners[row as usize]=Some(n);p.order.push(row);}
        }
        p.order.extend(fallback);p.order.extend(free);
        // Reserved clip-FX rows and malformed legacy bindings must still be reachable.
        let used:BTreeSet<_>=p.order.iter().copied().collect();p.order.extend((0..pl::TRACKS).filter(|r|!used.contains(r)));
        p
    }
}

pub fn toggle(app:&mut App) {
    let s=&mut app.state;
    if let Some(a)=s.playlist.attached.take(){
        a.playlist.restore(s,Win::Playlist);a.mixer.restore(s,Win::Mixer);
        s.playlist_scroll=a.scroll;
    }else{
        s.playlist.attached=Some(Attached{playlist:Window::capture(s,Win::Playlist),mixer:Window::capture(s,Win::Mixer),scroll:s.playlist_scroll,sends_scroll:Default::default(),cache:RefCell::default()});
        s.native_detached.retain(|w|*w!=Win::Playlist);
        // Keep the Mixer in its current stacking position. A newly shown
        // Playlist goes behind it when both windows occupy the workspace.
        if !s.shown(Win::Playlist) {
            let position = s.z.iter().position(|&win| win == Win::Mixer).unwrap_or(s.z.len());
            s.z.insert(position, Win::Playlist);
        }
        s.wins[Win::Playlist.slot().unwrap()].mini = false;
        s.playlist_scroll.1=0.;
    }
    s.native_layout_generation=s.native_layout_generation.wrapping_add(1);
    s.playlist.popup=None;app.invalidate();
}

/// Persist only real editing/recording bindings; viewing an old project is non-mutating.
pub fn bind(s:&mut State,row:u16,mixer:u16) {
    let a=pl::arr_mut(s);while a.tracks.len()<=row as usize{a.tracks.push(os_model::PlaylistTrack::new(a.tracks.len() as u32+1));}
    a.tracks[row as usize].attached_mixer=Some(mixer);clear(s);
}
pub fn select_lane(s:&mut State,row:u16) {
    let Some(n)=owner(s,row) else{return};mixer::select(s,n,false);
    if let Some(ch)=s.project.channels.iter().position(|c|c.mixer_track==n&&matches!(c.kind,ChannelKind::Plugin|ChannelKind::Sampler)) {
        s.selected_channel=ch;s.rack.select_only(ch);
    }
}

pub fn instrument(s:&State,n:u16)->Option<usize>{s.project.channels.iter().position(|c|c.mixer_track==n&&matches!(c.kind,ChannelKind::Plugin|ChannelKind::Sampler))}
pub fn lane_name(s:&State,n:u16)->String{
    mixer::track(s,n).and_then(|m|m.name.clone()).or_else(||instrument(s,n).and_then(|i|s.project.channels[i].name.clone())).unwrap_or_else(||mixer::name(s,n))
}
pub fn record_armed(s:&State,row:u16)->bool{
    if !active(s){return crate::playlist_tracks::armed(s,row);}
    let Some(n)=owner(s,row)else{return false};
    if s.mixer.recording.inputs.contains_key(&n){return s.mixer.recording.armed.contains(&n);}
    instrument(s,n).is_some_and(|ch|ch==s.selected_channel&&s.rec.armed&&crate::record::Opt::Notes.on())
}
pub fn record_lane(app:&mut App,row:u16)->Option<String>{
    if !active(&app.state){return crate::playlist_tracks::record(app,row);}
    let s=&app.state;let n=owner(s,row)?;
    if mixer::locked(s,n)||pl::track_info(s,row).is_some_and(|t|t.locked||t.frozen.is_some()){return Some("Unlock or unfreeze this lane before recording".into());}
    let notes=(!s.mixer.recording.inputs.contains_key(&n)).then(||instrument(s,n)).flatten();
    if let Some(ch)=notes {
        let armed=record_armed(s,row);app.end_take();
        let s=&mut app.state;select_lane(s,row);s.selected_channel=ch;s.playlist.track=row;s.song_mode=true;
        crate::mixer_recording::flag(crate::record::Opt::Notes,!armed);
        crate::mixer_recording::flag(crate::record::Opt::Smart,true);
        s.rec.armed=!armed||!s.mixer.recording.armed.is_empty();
        app.invalidate();return None;
    }
    select_lane(&mut app.state,row);app.arm_mixer_disk(n)
}

pub(crate) fn record_button_visible(s: &State, row: u16) -> bool {
    (!active(s) || crate::workspace_visibility::shown(RECORD_VISIBILITY))
        && crate::playlist_tracks::buttons(s,row)
        && (!active(s) || !pl::track_info(s,row).is_some_and(|t|t.grouped))
}

pub fn record_button(ui:&mut crate::app::Ui,s:&State,row:u16,r:os_ui::Rect){
    if !record_button_visible(s,row) { return; }
    let armed = record_armed(s,row);
    let recording = s.rec.note_record_row == Some(row) || s.mixer.recording.live.iter().any(|take| take.row == row);
    let hint = if recording {
        "Recording this track"
    } else {
        "Arm this track for recording, then press Play. Right-click to choose an audio input."
    };
    let hover = ui.hit(r, crate::app::Action::Mix(mixer::MHit::AttachedRecord(row)), hint);
    let (ring, fill) = if recording {
        (0xff9395, 0xff292d)
    } else if armed {
        (0xe96870, 0xff707b)
    } else if hover {
        (0xe96870, 0xe96870)
    } else {
        (0xe96870, 0xa63f48)
    };
    ui.c.stroke_circle(r.cx(), r.cy(), 6.1, 1., os_ui::Color::rgb(ring));
    ui.c.fill_circle(r.cx(), r.cy(), 4.5, os_ui::Color::rgb(fill));
}

pub fn colored_lane(s:&State,row:u16)->bool{
    owner(s,row).and_then(|n|mixer::track(s,n)).and_then(|m|m.color).is_some()
        || pl::track_info(s,pl::group_track(s,row).unwrap_or(row)).and_then(|t|t.color).is_some()
}
pub fn lane_icon(s:&State,row:u16,hover:bool)->Option<os_ui::Color>{
    colored_lane(s,row).then_some(if hover{os_ui::Color::rgb(0x93999d)}else{os_ui::Color::rgb(0xffffff)})
}

pub fn add_instrument(app:&mut App,index:usize)->Option<String>{
    if app.plugins.available.get(index).is_none(){return None;}
    if app.state.rec.recording()||app.audio_rec.is_some(){return Some("Finish recording before inserting an instrument track".into());}
    clear(&app.state);
    let p=projection(&app.state);
    let row=(0..pl::TRACKS).find(|r|p.owners[*r as usize].is_none_or(|m|!app.state.project.channels.iter().any(|c|c.mixer_track==m)&&!app.state.mixer.recording.inputs.contains_key(&m)) && !pl::clips(&app.state).iter().any(|c|c.track==*r) && pl::track_info(&app.state,*r).is_none_or(|t|!t.locked&&!t.grouped&&!t.folder&&t.name.is_none()&&t.comp.is_none()&&t.frozen.is_none()&&t.attached_mixer.is_none()));
    let Some(row)=row else{return Some("No empty Playlist lane is available for another instrument".into());};
    add_instrument_at(app,index,row)
}
pub fn add_instrument_at(app:&mut App,index:usize,row:u16)->Option<String>{
    if row>=pl::TRACKS||app.plugins.available.get(index).is_none(){return None;}
    if pl::track_info(&app.state,row).is_some_and(|t|t.locked||t.frozen.is_some()){return Some("Unlock or unfreeze this lane before inserting an instrument".into());}
    if app.state.rec.recording()||app.audio_rec.is_some(){return Some("Finish recording before inserting an instrument".into());}
    if mixer::inserts(&app.state)>=4093{return Some("The mixer has reached its track limit".into());}
    let snapshot=pl::capture(&app.state);let selected=app.state.selected_mixer;
    let count=app.state.project.channels.len();
    let previous=app.state.history.begin_sequence();
    let result=crate::plugin_menu::add_generator(app,index,None);
    if app.state.project.channels.len()==count{app.state.history.end_sequence_named(previous,"Insert instrument track");return result;}
    let s=&mut app.state;let ch=s.selected_channel;
    let n=mixer::add_track(s,None);
    let name=pl::track_info(s,row).and_then(|t|t.name.clone()).unwrap_or_else(||crate::rack::channel_name(s,ch));let color=pl::track_info(s,row).and_then(|t|t.color).or(s.project.channels[ch].color);
    if let Some(m)=mixer::track_mut(s,n){m.name=Some(name.clone());m.color=color;}
    s.project.channels[ch].mixer_track=n;
    bind(s,row,n);let arr=pl::arr_mut(s);let number=arr.number;arr.tracks[row as usize].name=Some(name);arr.tracks[row as usize].color=color;
    s.project.instrument_lanes.push((s.project.channels[ch].index,number,row,s.pattern.max(1)));
    s.playlist.track=row;s.selected_mixer=n;
    s.history.wrap_attached_instrument(snapshot,n,selected);
    s.history.end_sequence_named(previous,"Insert instrument track");
    clear(s);s.playlist_scroll.1=pl::Rows::new(s).top(row);app.invalidate();result
}

pub fn insert_on_lane(app:&mut App,index:usize,n:u16,row:u16)->Option<String>{
    if app.plugins.available.get(index).is_none()||mixer::track(&app.state,n).is_none(){return None;}
    if mixer::locked(&app.state,n)||pl::track_info(&app.state,row).is_some_and(|t|t.locked||t.frozen.is_some()){return Some("Unlock or unfreeze this lane before inserting an instrument".into());}
    if app.state.rec.recording()||app.audio_rec.is_some(){return Some("Finish recording before inserting an instrument".into());}
    let previous=app.state.history.begin_sequence();pl::snapshot(&mut app.state);
    let result=crate::plugin_menu::add_generator(app,index,None);
    let s=&mut app.state;let ch=s.selected_channel;s.project.channels[ch].mixer_track=n;
    bind(s,row,n);let arr=pl::arr_mut(s).number;
    s.project.instrument_lanes.push((s.project.channels[ch].index,arr,row,s.pattern.max(1)));
    s.selected_mixer=n;s.history.end_sequence_named(previous,"Insert instrument on mixer track");app.invalidate();result
}
pub fn piano(app:&mut App,row:u16,right_click:bool)->Option<String>{
    if !active(&app.state){return crate::playlist_tracks::piano(app,row,right_click);}
    let n=owner(&app.state,row)?;
    if right_click {
        crate::playlist_tracks::open_instrument_menu(app, crate::plugin_menu::Target::LaneInstrument(n, row));
        return None;
    }
    if instrument(&app.state,n).is_none(){
        let i=app.plugins.available.iter().position(|p|p.desc.name==crate::plugins::LEMON_SAMPLER)?;
        let message=insert_on_lane(app,i,n,row);
        if instrument(&app.state,n).is_none(){return message;}
        app.state.pending_open=None;
    }
    if let Some(ch)=instrument(&app.state,n){app.act(crate::app::Action::RackRoll(ch));}
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::Action;
    use os_model::{Clip, MixerTrack, PlaylistTrack};
    fn app()->App{
        let mut a=App::new(None);a.state.project=crate::app::test_project();
        a.state.project.mixer=(0..8).map(MixerTrack::new).collect();
        let arr=pl::arr_mut(&mut a.state);arr.clips.clear();arr.tracks=(1..=pl::TRACKS).map(|n|PlaylistTrack::new(n.into())).collect();
        a.state.project.instrument_lanes.clear();a.state.project.patterns.iter_mut().for_each(|p|p.notes.clear());
        a.state.project.channels[0].mixer_track=3;a.state.project.channels[1].mixer_track=1;
        a.state.project.channels[2].mixer_track=5;a.state.project.channels[3].mixer_track=0;a.state.project.channels[4].mixer_track=0;
        a.state.project.mixer[1].routes=vec![4];a.state.project.mixer[3].routes=vec![4,5];
        a.state.project.mixer[1].name=Some("Lead vocal".into());a.state.project.mixer[3].name=Some("Guitar".into());a.state.project.mixer[4].name=Some("Mix bus".into());a.state.project.mixer[5].name=Some("Keys + return".into());
        a.state.project.mixer[1].color=Some(os_model::Rgb::new(170,110,20));a.state.project.mixer[4].color=Some(os_model::Rgb::new(50,130,240));
        pl::arr_mut(&mut a.state).clips=vec![Clip::new(0,192,0,ClipSource::Channel(0)),Clip::new(0,192,7,ClipSource::Channel(1)),Clip::new(0,192,11,ClipSource::Channel(0)),Clip::new(192,192,11,ClipSource::Channel(1))];
        a.state.z=vec![Win::Playlist];a
    }
    #[test]
    fn separator_is_visual_and_existing_lane_insertion_is_one_undo(){
        let mut a=app();a.plugins=crate::plugins::Plugins::builtins();a.state.plugdb.rebuild(&a.plugins);
        a.state.project.mixer[6].separator=true;toggle(&mut a);
        let row=lane(&a.state,6).expect("separator remains a normal track");
        assert!(!projection(&a.state).buses.contains(&6));assert!(projection(&a.state).buses.contains(&4));
        let before=a.state.project.clone();let steps=a.state.history.done();
        piano(&mut a,row,true);
        assert!(a.state.playlist.popup.as_ref().unwrap().items.iter().any(|item|
            item.cmd == pl::Cmd::Plugin(crate::plugin_menu::PlugCmd::Picker(crate::plugin_menu::Target::LaneInstrument(6, row)))));
        let i=a.plugins.available.iter().position(|p|p.desc.name==crate::plugins::LEMON_SAMPLER).unwrap();
        insert_on_lane(&mut a,i,6,row);let after=a.state.project.clone();
        assert_eq!(after.mixer,before.mixer);assert_eq!(after.channels.last().unwrap().mixer_track,6);assert_eq!(a.state.history.done(),steps+1);
        crate::history::undo(&mut a.state);assert_eq!(a.state.project,before);
        crate::history::redo(&mut a.state);assert_eq!(a.state.project,after);
    }
    #[test]
    fn inspector_routes_scroll_and_all_source_lanes_have_piano_controls(){
        let mut a=app();toggle(&mut a);a.state.selected_mixer=1;
        a.state.project.mixer=(0..24).map(MixerTrack::new).collect();
        a.state.project.mixer[1].routes=(2..24).collect();
        a.state.project.mixer[1].color=Some(os_model::Rgb{r:255,g:210,b:0});
        clear(&a.state);let mut c=os_ui::Canvas::new(1440,900,1.).unwrap();a.paint(&mut c,None);
        assert!(a.hits.iter().any(|h|h.action==Action::Mix(mixer::MHit::AttachedPiano(7))));
        let mut seen=BTreeSet::new();
        for _ in 0..24{
            for h in &a.hits{if let Action::Mix(mixer::MHit::AttachedSelect(n))=h.action{seen.insert(n);}}
            let bar=a.bar(crate::app::Scroll::AttachedSends).unwrap();
            a.mouse_move(bar.track.cx(),bar.track.cy());
            a.wheel(0.,-1.,false,false);
            a.paint(&mut c,None);
        }
        assert!((2..24).all(|n|seen.contains(&n)),"all sends remain reachable");
    }
    #[test]
    fn attached_layout_is_reversible_without_audio_or_project_mutations(){
        let mut a=app();
        let playlist = Win::Playlist.slot().unwrap();
        a.state.wins[playlist].maxed = false;
        a.state.raise(Win::Mixer);
        let before=a.state.project.clone();let windows=a.state.wins.clone();let scroll=(87.,33.);a.state.playlist_scroll=scroll;
        let pending=a.state.mixer.pending.len();let history=a.state.history.done();
        a.transport.pretend_at(83.,true);let generation=a.transport.watch().generation();
        toggle(&mut a);
        assert!(!a.state.is_max(Win::Playlist));
        assert_eq!(a.state.wins[playlist].rect, windows[playlist].rect);
        assert!(a.state.shown(Win::Mixer));assert_eq!(a.state.project,before);
        let p=projection(&a.state);assert!(p.buses.contains(&4));assert!(!p.buses.contains(&5));assert_eq!(p.owners[7],Some(1));assert_eq!(p.owners[0],Some(3));assert_eq!(p.owners[11],None);
        assert!(p.order.iter().position(|r|*r==7)<p.order.iter().position(|r|*r==0));
        assert!(Rc::ptr_eq(&p,&projection(&a.state)),"meter frames reuse the lane projection");
        a.state.raise(Win::Mixer);assert!(active(&a.state));toggle(&mut a);
        assert_eq!(a.state.project,before);assert_eq!(a.state.playlist_scroll,scroll);
        for (now,old) in a.state.wins.iter().zip(windows){assert_eq!((now.rect,now.maxed,now.mini),(old.rect,old.maxed,old.mini));}
        assert_eq!(a.state.mixer.pending.len(),pending);assert_eq!(a.state.history.done(),history);
        assert_eq!(a.transport.watch().generation(),generation);assert_eq!(a.transport.tick(),83.);
        a.state.wins[playlist].maxed = true;
        a.state.hide(Win::Playlist);
        toggle(&mut a);
        assert!(a.state.shown(Win::Playlist));
        assert_eq!(a.state.docked(), Some(Win::Mixer));
    }
    #[test]
    fn attached_slice_and_selection_follow_display_order_without_rerouting(){
        let mut a=app();toggle(&mut a);let rows=pl::Rows::new(&a.state);
        for row in 0..pl::TRACKS {if rows.height(row)>0.{assert_eq!(rows.at(rows.top(row)+1.),row);}}
        assert!(rows.between(7,7,0));assert!(rows.between(0,7,0));assert!(!rows.between(11,7,0));
        let routing:Vec<_>=a.state.project.channels.iter().map(|c|c.mixer_track).collect();
        pl::snapshot(&mut a.state);assert_eq!(pl::slice(&mut a.state,96,7,0),2);
        assert_eq!(pl::clips(&a.state).iter().filter(|c|c.track==11).count(),2);
        assert_eq!(a.state.project.channels.iter().map(|c|c.mixer_track).collect::<Vec<_>>(),routing);
        assert!(crate::history::undo(&mut a.state).is_some());assert_eq!(pl::clips(&a.state).len(),4);
    }
    #[test]
    fn attached_instrument_is_one_undo_action_and_survives_detach_and_file_roundtrip(){
        let mut a=app();a.plugins=crate::plugins::Plugins::builtins();a.state.plugdb.rebuild(&a.plugins);toggle(&mut a);
        pl::arr_mut(&mut a.state).tracks[1].folder=true;clear(&a.state);
        let i=a.plugins.available.iter().position(|p|p.desc.name==crate::plugins::LEMON_SAMPLER).unwrap();
        let before=a.state.project.clone();let steps=a.state.history.done();
        add_instrument(&mut a,i);let after=a.state.project.clone();let n=a.state.selected_mixer;let row=a.state.playlist.track;
        assert_ne!(row,1,"preserve an existing empty folder");
        assert_eq!(a.state.history.done(),steps+1);assert_eq!(after.mixer.len(),before.mixer.len()+1);
        assert_eq!(after.channels.last().unwrap().mixer_track,n);assert_eq!(after.arrangement().unwrap().tracks[row as usize].attached_mixer,Some(n));
        assert_eq!(crate::history::undo(&mut a.state),Some("Insert instrument track"));assert_eq!(a.state.project,before);
        assert_eq!(crate::history::redo(&mut a.state),Some("Insert instrument track"));assert_eq!(a.state.project,after);
        let mut file=os_flp::project::create(&a.state.project).unwrap();crate::layout::write_project_layout(&mut file,crate::layout::project_snapshot(&a.state,&a.plugins));
        let loaded=os_flp::project::load(&file).unwrap();assert_eq!(loaded.arrangement().unwrap().tracks[row as usize].attached_mixer,Some(n));
        let mut reopened=app();reopened.state.project=loaded;reopened.state.file=Some(file);crate::layout::restore_project_layout(&mut reopened.state);
        assert!(active(&reopened.state));assert_eq!(lane(&reopened.state,n),Some(row));
        assert_eq!(full_strip(&reopened.state),full_strip(&a.state));
        toggle(&mut reopened);assert_eq!(pl::track_info(&reopened.state,row).unwrap().attached_mixer,Some(n));
        toggle(&mut a);assert_eq!(a.state.project,after);
    }
    #[test]
    fn attach_and_detach_buttons_are_blue_in_each_mixer_view() {
        for style in [mixer::WIDE, mixer::TRADITIONAL, crate::rack_mixer::VIEW] {
            let mut a = app();
            a.state.mixer.layout = style;
            a.state.raise(Win::Mixer);
            let mut c = os_ui::Canvas::new(1440,900,1.).unwrap();
            for attached in [false, true] {
                if attached {
                    toggle(&mut a);
                    a.state.raise(Win::Playlist);
                }
                a.invalidate(); a.paint(&mut c,None);
                let r = a.hits.iter().find(|h|h.action==Action::Mix(mixer::MHit::AttachPlaylist)).unwrap().rect;
                let pixel = c.pixmap.pixels()[(r.y as usize+4)*1440+r.x as usize+4];
                assert!(pixel.blue()>pixel.red().saturating_add(80) && pixel.blue()>pixel.green().saturating_add(30), "blue primary action: {style}, attached {attached}");
            }
            a.act(Action::Mix(mixer::MHit::AttachedInspector));
            assert_eq!(width(&a.state), 0.0);
            a.state.raise(Win::Mixer);
            a.invalidate();
            a.paint(&mut c, None);
            assert!(a.hits.iter().any(|h| h.action == Action::Mix(mixer::MHit::AttachPlaylist)));
            crate::settings_file::set_flag(BUTTON_VISIBILITY, false);
            a.invalidate();
            a.paint(&mut c, None);
            assert!(!a.hits.iter().any(|h| h.action == Action::Mix(mixer::MHit::AttachPlaylist)));
            crate::settings_file::set_flag(BUTTON_VISIBILITY, true);
            crate::settings_file::set_flag(VISIBILITY, true);
        }
    }

    #[test]
    fn attached_offscreen_controls_work_in_all_mixer_styles(){
        let mut a=app();a.state.project.mixer[1].channel_layout=1;a.state.playlist.picker_visible=true;let mut canvas=os_ui::Canvas::new(1440,900,1.).unwrap();
        for (style,name) in [(mixer::WIDE,"full"),(mixer::TRADITIONAL,"traditional"),(crate::rack_mixer::VIEW,"strips")] {
            a.state.mixer.layout=style;toggle(&mut a);a.state.selected_mixer=1;a.invalidate();a.paint(&mut canvas,None);
            assert!(!full_strip(&a.state), "all Mixer views default to the Traditional inspector");
            for action in [Action::Mix(mixer::MHit::AttachPlaylist),Action::Mix(mixer::MHit::Input(1)),Action::Mix(mixer::MHit::AttachedRecord(7)),Action::Fader(1),Action::PlAddInstrument,Action::Warp(crate::warp::Command::Toggle(7)),Action::Vocal(crate::vocal::Command::Toggle(7))]{assert!(a.hits.iter().any(|h|h.action==action),"missing {action:?}");}
            let h=a.hits.iter().find(|h|h.action==Action::PlHeader(7)).unwrap().rect;
            let other=a.hits.iter().find(|h|h.action==Action::PlHeader(0)).unwrap().rect;assert!(h.y<other.y);
            if let Some(dir)=std::env::var_os("LEMON_ATTACHED_PREVIEW"){let dir=std::path::PathBuf::from(dir);std::fs::create_dir_all(&dir).unwrap();canvas.pixmap.save_png(dir.join(format!("attached-{name}.png"))).unwrap();}
            a.act(Action::ToggleWin(Win::Mixer));a.paint(&mut canvas,None);assert!(a.state.shown(Win::Mixer));assert!(active(&a.state));a.state.hide(Win::Mixer);toggle(&mut a);
        }
    }
    #[test]
    fn inspector_style_visibility_and_shared_eq_preserve_the_project(){
        let mut a=app();a.plugins=crate::plugins::Plugins::builtins();a.state.project.mixer[1].channel_layout=1;
        a.state.mixer.layout=mixer::WIDE;toggle(&mut a);a.state.selected_mixer=1;
        assert!(!full_strip(&a.state));
        mixer::run(&mut a, mixer::MCmd::AttachedInspectorStyle(true));
        let before=a.state.project.clone();let mut c=os_ui::Canvas::new(1440,900,1.).unwrap();a.paint(&mut c,None);
        let rect=|a:&App,action:Action|a.hits.iter().find(|h|h.action==action).unwrap().rect;
        let eq=rect(&a,Action::Mix(mixer::MHit::EqGraph));let fader=rect(&a,Action::Fader(1));
        assert!(eq.bottom()<fader.y);assert!(a.hits.iter().any(|h|h.action==Action::Knob(crate::knobs::KnobId::Strip(1,0))));
        a.state.mixer.layout=mixer::TRADITIONAL;assert!(full_strip(&a.state),"the saved inspector style is independent of the Mixer view");
        a.act(Action::Mix(mixer::MHit::AttachedInspector));a.paint(&mut c,None);assert_eq!(width(&a.state),0.);assert_eq!(a.state.project,before);
        assert!(!a.hits.iter().any(|h|h.action==Action::Mix(mixer::MHit::EqGraph)));
        assert!(a.hits.iter().any(|h|h.action==Action::Mix(mixer::MHit::AttachedInspector)));
        a.act(Action::Mix(mixer::MHit::AttachedInspector));a.paint(&mut c,None);assert_eq!(width(&a.state),WIDTH);
        assert_eq!(crate::mini_eq::ensure(&mut a,1),Some(10));let instances=a.plugins.hosted.len();
        a.paint(&mut c,None);let g=rect(&a,Action::Mix(mixer::MHit::EqGraph));
        let settings=crate::mini_eq::settings(&a.state,1);let b=&settings.bands[3];let x=g.x+1.+b.freq as f32*(g.w-2.);
        a.mouse_move(x,g.cy());a.mouse_down();a.mouse_move(x,g.cy()-12.);a.mouse_up();
        assert!(crate::mini_eq::settings(&a.state,1).bands[3].db>0.);
        a.state.raise(Win::Mixer);assert_eq!(crate::mini_eq::ensure(&mut a,1),Some(10));assert_eq!(a.plugins.hosted.len(),instances,"one shared EQ instance");
    }
    #[test]
    fn attached_inspector_fits_short_windows_and_anchors_the_fader() {
        use crate::app::{Live, Scroll};
        for (full, style) in [(true,mixer::WIDE),(false,mixer::TRADITIONAL)] {
            let mut a = app();
            a.state.mixer.layout = style;
            crate::settings_file::set_flag(FULL_INSPECTOR_SETTING, full);
            a.state.project.mixer[1].channel_layout = 2;
            toggle(&mut a);
            a.state.selected_mixer = 1;
            let before = a.state.project.clone();
            let history = a.state.history.done();
            assert!(mixer::right(&mut a,mixer::MHit::AttachedInspector));
            assert!(a.state.playlist.popup.as_ref().unwrap().items.iter().any(|item| item.label == "Section order"));
            a.state.playlist.popup = None;
            for height in [900,820,720,600,480] {
                let mut c = os_ui::Canvas::new(1440,height,1.).unwrap();
                a.invalidate();
                a.paint(&mut c,None);
                let panel = a.live_rect(Live::AttachedInspector).unwrap();
                let fader = a.hits.iter().find(|h| h.action == Action::Fader(1) && h.rect.h > 0.).unwrap().rect;
                assert!(fader.bottom() <= panel.bottom() && panel.bottom()-fader.bottom() < 42., "{style} at {height}: {fader:?} vs {panel:?}");
                assert!(a.hits.iter().any(|h| h.action==Action::Mix(mixer::MHit::AttachedChannels) && h.rect.h>0.));
                let panel_bars = a.hits.iter().filter(|h| h.rect.h>0. && panel.contains(h.rect.cx(),h.rect.cy()));
                for hit in panel_bars {
                    if let Action::Scrollbar(axis) = hit.action {
                        assert!(matches!(axis, Scroll::AttachedEffects | Scroll::AttachedSends));
                    }
                }
                if full && height >= 600 {
                    assert!(a.hits.iter().any(|h|h.action==Action::Knob(crate::knobs::KnobId::Strip(1,8)) && h.rect.h>0.), "Vox tuning at {height}");
                }
                if let Some(dir) = std::env::var_os("LEMON_ATTACHED_PREVIEW") {
                    c.pixmap.save_png(std::path::PathBuf::from(dir).join(format!("inspector-fixed-{style}-{height}.png"))).unwrap();
                }
                assert_eq!(a.state.project,before);
                assert_eq!(a.state.history.done(),history);
            }
        }
    }

    #[test]
    fn inspector_section_drags_and_disclosures_persist_per_view() {
        let mut a = app();
        let mut canvas = os_ui::Canvas::new(1440, 1100, 1.).unwrap();
        let sections = Section::ALL;
        let mut expected = [[0; 3]; 2];
        for (full, style) in [(true,mixer::WIDE),(false,mixer::TRADITIONAL)] {
            a.state.mixer.layout = style;
            crate::settings_file::set_flag(FULL_INSPECTOR_SETTING, full);
            toggle(&mut a);
            a.state.selected_mixer = 1;
            let project = a.state.project.clone();
            let history = a.state.history.done();
            a.paint(&mut canvas, None);
            for section in sections {
                let hit = a.hits.iter().find(|h| matches!(h.action, Action::InspectorResize(which, ..) if which == section)).unwrap();
                let Action::InspectorResize(_, old, _) = hit.action else { unreachable!() };
                let r = hit.rect;
                a.mouse_move(r.cx(), r.cy());
                a.mouse_down();
                a.mouse_move(r.cx(), r.cy()+22.);
                a.mouse_up();
                a.paint(&mut canvas, None);
                let size = a.state.playlist.inspector_layouts.get(full).height(section,full);
                assert!(size > old, "{style} {section:?}");
                expected[usize::from(full)][section as usize] = size;
            }
            for section in [Section::Effects, Section::Sends] {
                let r = a.hits.iter().find(|h|h.action == Action::InspectorCollapse(section)).unwrap().rect;
                a.mouse_move(r.cx(),r.cy());
                a.mouse_down();
                a.mouse_up();
                a.paint(&mut canvas, None);
                assert!(!a.hits.iter().any(|h|matches!(h.action,Action::InspectorResize(which, ..) if which == section)));
            }
            assert!(a.bar(crate::app::Scroll::AttachedEffects).is_none());
            assert!(a.bar(crate::app::Scroll::AttachedSends).is_none());
            let fx = a.hits.iter().filter(|h|h.action==Action::Mix(mixer::MHit::Effects(1)) && h.rect.h>0.).collect::<Vec<_>>();
            assert_eq!(fx.len(),1);
            let fader = a.hits.iter().find(|h|h.action==Action::Fader(1)&&h.rect.h>0.).unwrap();
            assert!(fx[0].rect.bottom()<fader.rect.y);
            mixer::run(&mut a, mixer::MCmd::AttachedInspectorMove(Section::Eq,1));
            a.state.selected_mixer = 3;
            a.paint(&mut canvas, None);
            let eq = a.hits.iter().find(|h|h.action==Action::Mix(mixer::MHit::EqGraph)).unwrap().rect;
            let effects = a.hits.iter().find(|h|h.action==Action::InspectorCollapse(Section::Effects)).unwrap().rect;
            assert!(effects.bottom() < eq.y);
            assert!(a.hits.iter().any(|h| matches!(h.action,Action::InspectorResize(Section::Eq,size,_) if size==expected[usize::from(full)][0])));
            mixer::run(&mut a, mixer::MCmd::AttachedInspectorShow(EQ_VISIBILITY));
            a.paint(&mut canvas, None);
            assert!(!a.hits.iter().any(|h|h.action==Action::Mix(mixer::MHit::EqGraph)));
            mixer::run(&mut a, mixer::MCmd::AttachedInspectorShow(EQ_VISIBILITY));
            assert_eq!(a.state.project,project);
            assert_eq!(a.state.history.done(),history);
            toggle(&mut a);
        }
        let reopened = InspectorLayouts::default();
        for full in [false,true] {
            let layout = reopened.get(full);
            assert!(layout.effects_collapsed && layout.sends_collapsed);
            assert_eq!(layout.order(),[Section::Effects,Section::Eq,Section::Sends]);
            for section in sections {
                assert_eq!(layout.height(section,full),expected[usize::from(full)][section as usize]);
            }
            crate::prefs::set_text(InspectorLayouts::key(full),"");
        }
    }

    #[test]
    fn option_c_sends_meter_and_native_controls_preserve_selection_and_routing(){
        use crate::app::{Live, Scroll};
        for style in [mixer::WIDE,mixer::TRADITIONAL]{
            let mut a=app();a.state.mixer.layout=style;a.state.project.mixer[1].channel_layout=1;
            crate::settings_file::set_flag(FULL_INSPECTOR_SETTING, style == mixer::WIDE);
            a.state.project.mixer[1].routes=vec![0,2,3,4,5,6];
            for (n,name) in [(2,"Short Verb"),(3,"Long Verb"),(4,"Delay"),(5,"Vocal Mix"),(6,"Slap Delay")]{
                let m=&mut a.state.project.mixer[n];m.name=Some(name.into());m.color=Some(if n==5{os_model::Rgb::new(70,150,240)}else{os_model::Rgb::new(235,200,65)});
            }
            toggle(&mut a);a.state.selected_mixer=1;let before=a.state.project.clone();let steps=a.state.history.done();
            a.state.mixer.meters.resize(8,Default::default());
            a.state.mixer.meters[1].db=[-10.,-13.];
            a.state.mixer.meters[1].hold=[-6.,-9.];
            let mut c=os_ui::Canvas::new(1440,900,1.).unwrap();a.paint(&mut c,None);
            if let Some(dir)=std::env::var_os("LEMON_ATTACHED_PREVIEW"){c.pixmap.save_png(std::path::PathBuf::from(dir).join(format!("option-c-sends-{style}.png"))).unwrap();}
            if style==mixer::TRADITIONAL{assert!(a.live_rect(Live::Meter(1)).unwrap().w>32.,"stereo meter bars need positive width");}
            let click=|a:&mut App,c:&mut os_ui::Canvas,action:Action|{
                let r=a.hits.iter().find(|h|h.action==action&&h.rect.h>0.).unwrap().rect;
                a.mouse_move(r.cx(),r.cy());a.mouse_down();a.mouse_up();a.paint(c,None);
            };
            assert_eq!(a.hits.iter().filter(|h|matches!(h.action,Action::Mix(mixer::MHit::AttachedSendLabel(_)))&&h.rect.h>0.).count(),5);
            click(&mut a,&mut c,Action::Mix(mixer::MHit::AttachedSendLabel(0)));
            assert_eq!(a.state.selected_mixer,1);assert_eq!(a.state.project,before);
            let sends=a.bar(Scroll::AttachedSends).unwrap();
            assert_eq!(sends.max,1.);
            a.mouse_move(sends.track.cx(),sends.track.y+sends.thumb/2.);
            a.mouse_down();
            a.mouse_move(sends.track.cx(),sends.track.bottom()-sends.thumb/2.);
            a.mouse_up();
            a.paint(&mut c,None);
            assert!(a.hits.iter().any(|h|h.action==Action::Mix(mixer::MHit::AttachedSendLabel(6))&&h.rect.h>0.));
            a.mouse_move(sends.track.cx(),sends.track.cy());
            a.wheel(0.,1.,false,false);
            a.paint(&mut c,None);
            assert_eq!(a.scroll_get(Scroll::AttachedSends),0.);
            if style==mixer::WIDE{
                let pixels=c.pixmap.data().to_vec();
                let fader=a.hits.iter().find(|h|h.action==Action::Fader(1)).unwrap().rect;
                click(&mut a,&mut c,Action::Mix(mixer::MHit::Meter(1)));
                assert!(a.state.mixer.peak_view.contains(&1));assert_ne!(c.pixmap.data(),pixels);
                assert_eq!(a.hits.iter().find(|h|h.action==Action::Fader(1)).unwrap().rect,fader);
                assert!(!a.hits.iter().any(|h|matches!(h.action,Action::Mix(mixer::MHit::Jump(..)))&&h.rect.w>0.));
            }
            assert_eq!(a.state.selected_mixer,1);assert_eq!(a.state.project,before);assert_eq!(a.state.history.done(),steps);
            click(&mut a,&mut c,Action::Mix(mixer::MHit::AttachedSelect(0)));
            assert_eq!(a.state.selected_mixer,0);assert!(a.live_rect(Live::AttachedInspector).is_some());assert_eq!(a.state.project,before);
            a.state.selected_mixer=1;a.paint(&mut c,None);
            let r=a.hits.iter().find(|h|h.action==Action::Fader(1)).unwrap().rect;
            let travel=a.live_rect(Live::AttachedFader(1)).unwrap().h;
            let initial=a.state.project.mixer[1].volume.unwrap_or(0.8);
            a.mouse_move(r.cx(),r.cy());a.mouse_down();a.mouse_move(r.cx(),r.cy()+travel*0.1);a.mouse_up();
            let volume=a.state.project.mixer[1].volume.unwrap();assert!((volume-(initial-0.1)).abs()<0.003,"{style}: {volume}, travel {travel}");
        }
    }
    #[test]
    fn narrow_inspector_effects_scroll_through_every_slot(){
        let mut a=app();a.state.mixer.layout=mixer::WIDE;toggle(&mut a);a.state.selected_mixer=1;
        let mut c=os_ui::Canvas::new(1440,900,1.).unwrap();let mut seen=BTreeSet::new();
        for _ in 0..14{
            a.paint(&mut c,None);
            for h in &a.hits{if let Action::Mix(mixer::MHit::AttachedSlot(k,3))=h.action{if h.rect.h>0.{seen.insert(k);}}}
            let r=a.hits.iter().find(|h|matches!(h.action,Action::Mix(mixer::MHit::AttachedSlot(_,3)))&&h.rect.h>0.).unwrap().rect;
            a.mouse_move(r.cx(),r.cy());a.wheel(0.,-1.,false,false);
        }
        assert_eq!(seen.len(),mixer::slot_count(&a.state,1));
        let axis=crate::app::Scroll::AttachedEffects;
        let bar=a.bar(axis).unwrap();
        assert!(bar.max>0.);
        a.mouse_move(bar.track.cx(),bar.track.bottom()-bar.thumb/2.);
        a.mouse_down();
        a.mouse_move(bar.track.cx(),bar.track.y+bar.thumb/2.);
        a.mouse_up();
        a.paint(&mut c,None);
        assert_eq!(a.scroll_get(axis),0.);
    }
    #[test]
    fn ordinary_playlist_buttons_are_opt_in_and_instrument_insertion_is_undoable(){
        let mut a=app();a.plugins=crate::plugins::Plugins::builtins();a.state.plugdb.rebuild(&a.plugins);
        a.state.z=vec![Win::Playlist];a.state.wins[Win::Playlist.slot().unwrap()].maxed=true;
        assert!(!crate::playlist_tracks::enabled());let mut c=os_ui::Canvas::new(1440,900,1.).unwrap();a.paint(&mut c,None);
        assert!(!a.hits.iter().any(|h|matches!(h.action,Action::Mix(mixer::MHit::AttachedPiano(_)|mixer::MHit::AttachedRecord(_)))));
        let before=a.state.project.clone();crate::workspace_visibility::toggle(crate::playlist_tracks::VISIBILITY);a.paint(&mut c,None);
        assert_eq!(a.state.project,before);let row=5;
        for action in [Action::Mix(mixer::MHit::AttachedPiano(row)),Action::Mix(mixer::MHit::AttachedRecord(row))]{assert!(a.hits.iter().any(|h|h.action==action));}
        let rec=a.hits.iter().find(|h|h.action==Action::Mix(mixer::MHit::AttachedRecord(row))).unwrap().rect;
        for action in [Action::PlLed(row),Action::PlAutomationToggle(row),Action::Warp(crate::warp::Command::Toggle(row))]{let r=a.hits.iter().find(|h|h.action==action).unwrap().rect;assert!(rec.intersect(&r).is_empty());}
        assert!(mixer::right(&mut a, mixer::MHit::AttachedPiano(row)));
        assert!(!a.state.plugin_picker.open);
        let toggle_picker = pl::Cmd::Plugin(crate::plugin_menu::PlugCmd::TogglePlaylistKeyboardPicker);
        assert!(a.state.playlist.popup.as_ref().unwrap().items.iter().any(|item| item.cmd == toggle_picker));
        pl::run(&mut a, toggle_picker.clone());
        assert!(mixer::right(&mut a, mixer::MHit::AttachedPiano(row)));
        assert!(a.state.plugin_picker.open);
        assert_eq!(a.state.plugin_picker.target, crate::plugin_menu::Target::PlaylistInstrument(row));
        crate::plugin_picker::close(&mut a);
        assert!(pl::options_menu(&a.state, 0.0, 0.0).items.iter().any(|item| item.cmd == toggle_picker));
        pl::run(&mut a, toggle_picker);
        assert_eq!(a.state.project,before);
        let index=a.plugins.available.iter().position(|p|p.desc.name==crate::plugins::LEMON_SAMPLER).unwrap();let steps=a.state.history.done();
        crate::plugin_menu::run(&mut a,crate::plugin_menu::PlugCmd::PlaylistInstrument{index,row});
        let after=a.state.project.clone();let n=a.state.selected_mixer;assert_eq!(a.state.history.done(),steps+1);
        assert_eq!(after.mixer.len(),before.mixer.len()+1);assert_eq!(after.channels.last().unwrap().mixer_track,n);
        assert_eq!(lane(&a.state,n),Some(row));assert!(!active(&a.state));
        crate::history::undo(&mut a.state);assert_eq!(a.state.project,before);crate::history::redo(&mut a.state);assert_eq!(a.state.project,after);
        let file=os_flp::project::create(&a.state.project).unwrap();let loaded=os_flp::project::load(&file).unwrap();
        assert_eq!(loaded.arrangement().unwrap().tracks[row as usize].attached_mixer,Some(n));
        assert!(loaded.instrument_lanes.iter().any(|v|v.2==row));
        piano(&mut a,row,false);assert_eq!(a.state.selected_channel,a.state.project.channels.len()-1);
        record_lane(&mut a,row);assert!(record_armed(&a.state,row));assert_eq!(crate::smart_recording::instrument_lane(&a.state).unwrap().0,row);
        assert!(a.audio_rec.is_none());record_lane(&mut a,row);assert!(!record_armed(&a.state,row));
        a.state.z=vec![Win::Playlist];a.paint(&mut c,None);
        if let Some(dir)=std::env::var_os("LEMON_ATTACHED_PREVIEW"){c.pixmap.save_png(std::path::PathBuf::from(dir).join("ordinary-playlist-buttons.png")).unwrap();}
        let before=a.state.project.clone();crate::workspace_visibility::toggle(crate::playlist_tracks::VISIBILITY);a.paint(&mut c,None);
        assert_eq!(a.state.project,before);assert!(!a.hits.iter().any(|h|matches!(h.action,Action::Mix(mixer::MHit::AttachedPiano(_)|mixer::MHit::AttachedRecord(_)))));
    }
    #[test]
    fn ordinary_playlist_audio_recording_uses_clicked_lane_and_retains_binding(){
        let _lock=crate::capture::TEST_RECORDING.lock().unwrap_or_else(|e|e.into_inner());
        let mut a=app();let row=5;crate::settings_file::set_flag(crate::playlist_tracks::VISIBILITY,true);
        let before=a.state.project.clone();let steps=a.state.history.done();record_lane(&mut a,row);
        assert!(a.state.playlist.popup.is_some());a.state.playlist.popup=None;assert_eq!(a.state.project,before);assert_eq!(a.state.history.done(),steps);
        let input=crate::capture::Input{device:"Test ASIO".into(),asio:true,first:0,stereo:false};
        crate::playlist_tracks::set_input(&mut a,row,Some(input.clone()));let n=a.state.selected_mixer;let after=a.state.project.clone();
        assert_eq!(a.state.history.done(),steps+1);assert_eq!(lane(&a.state,n),Some(row));assert!(record_armed(&a.state,row));assert!(a.audio_rec.is_none());
        crate::history::undo(&mut a.state);assert_eq!(a.state.project,before);assert!(!a.state.mixer.recording.armed.contains(&n));
        crate::history::redo(&mut a.state);assert_eq!(a.state.project,after);crate::playlist_tracks::set_input(&mut a,row,Some(input));
        crate::settings_file::set_flag(crate::playlist_tracks::VISIBILITY,false);
        a.prepare_mixer_take().unwrap();assert_eq!(a.state.mixer.recording.live[0].row,row);
        a.transport.pretend_at(0.,true);a.one_click_tick();for _ in 0..4{crate::capture::push_inputs(true,600,|_,_|0.2);}
        a.transport.stop();for _ in 0..500{a.one_click_tick();if a.audio_rec.is_none(){break;}std::thread::sleep(std::time::Duration::from_millis(5));}
        assert!(a.audio_rec.is_none(),"{:?}",a.state.message);let ch=a.state.project.channels.last().unwrap();assert_eq!(ch.mixer_track,n);
        assert!(pl::clips(&a.state).iter().any(|c|c.track==row&&c.source==ClipSource::Channel(ch.index)));
        let path=ch.sample_path.clone().unwrap();assert_eq!(pl::track_info(&a.state,row).unwrap().attached_mixer,Some(n));
        let result=a.state.project.clone();toggle(&mut a);toggle(&mut a);assert_eq!(a.state.project,result);let _=std::fs::remove_file(path);
    }
    #[test]
    fn attached_recording_uses_the_source_lane_with_synthetic_input(){
        let _lock=crate::capture::TEST_RECORDING.lock().unwrap_or_else(|e|e.into_inner());
        let mut a=app();toggle(&mut a);
        a.set_mixer_input(1,Some(crate::capture::Input{device:"Test ASIO".into(),asio:true,first:0,stereo:false}));
        clear(&a.state);let row=lane(&a.state,1).unwrap();assert_eq!(row,7);
        a.prepare_mixer_take().unwrap();assert_eq!(a.state.mixer.recording.live[0].row,row);
        a.transport.pretend_at(0.,true);a.one_click_tick();for _ in 0..4{crate::capture::push_inputs(true,600,|_,_|0.2);}
        a.transport.stop();for _ in 0..500{a.one_click_tick();if a.audio_rec.is_none(){break;}std::thread::sleep(std::time::Duration::from_millis(5));}
        assert!(a.audio_rec.is_none(),"{:?}",a.state.message);clear(&a.state);
        let channel=a.state.project.channels.last().unwrap();assert_eq!(channel.mixer_track,1);let path=channel.sample_path.clone().unwrap();
        assert_eq!(crate::comping::for_mixer(&a.state,1),Some(row));assert_eq!(pl::track_info(&a.state,row).unwrap().attached_mixer,Some(1));
        let result=a.state.project.clone();toggle(&mut a);assert_eq!(a.state.project,result);let _=std::fs::remove_file(path);
    }
    #[test]
    fn detaching_during_a_recording_keeps_its_original_empty_source_lane(){
        let _lock=crate::capture::TEST_RECORDING.lock().unwrap_or_else(|e|e.into_inner());
        let mut a=app();toggle(&mut a);
        a.set_mixer_input(6,Some(crate::capture::Input{device:"Test ASIO".into(),asio:true,first:0,stereo:false}));
        clear(&a.state);let row=lane(&a.state,6).unwrap();assert_ne!(row,1,"exercise a lane beyond the first empty physical row");
        a.prepare_mixer_take().unwrap();assert_eq!(a.state.mixer.recording.live[0].row,row);
        a.transport.pretend_at(0.,true);a.one_click_tick();for _ in 0..4{crate::capture::push_inputs(true,600,|_,_|0.2);}
        toggle(&mut a);assert!(a.transport.playing());
        a.transport.stop();for _ in 0..500{a.one_click_tick();if a.audio_rec.is_none(){break;}std::thread::sleep(std::time::Duration::from_millis(5));}
        assert!(a.audio_rec.is_none(),"{:?}",a.state.message);
        let channel=a.state.project.channels.last().unwrap();assert_eq!(channel.mixer_track,6);let path=channel.sample_path.clone().unwrap();
        assert!(pl::clips(&a.state).iter().any(|c|c.track==row&&c.source==ClipSource::Channel(channel.index)));
        assert_eq!(pl::track_info(&a.state,row).unwrap().attached_mixer,Some(6));
        toggle(&mut a);assert_eq!(lane(&a.state,6),Some(row));let _=std::fs::remove_file(path);
    }
    #[test]
    fn attached_lane_record_button_arms_notes_and_audio_without_an_inspector(){
        let mut a=app();toggle(&mut a);crate::settings_file::set_flag(VISIBILITY,false);
        let mut c=os_ui::Canvas::new(1440,900,1.).unwrap();a.paint(&mut c,None);
        let action=Action::Mix(mixer::MHit::AttachedRecord(7));let r=a.hits.iter().find(|h|h.action==action).unwrap().rect;
        let led=a.hits.iter().find(|h|h.action==Action::PlLed(7)).unwrap().rect;
        assert!(r.w>led.w);assert!(r.right()<=led.x);assert!(r.intersect(&led).is_empty());
        let automation=a.hits.iter().find(|h|h.action==Action::PlAutomationToggle(7)).unwrap().rect;
        assert!(r.intersect(&automation).is_empty(),"record and automation targets must not overlap");
        for action in [Action::Warp(crate::warp::Command::Toggle(7)),Action::Vocal(crate::vocal::Command::Toggle(7)),Action::PlAutomation(7)]{
            let icon=a.hits.iter().find(|h|h.action==action).unwrap().rect;
            assert!((icon.y as u32..icon.bottom() as u32).any(|y|(icon.x as u32..icon.right() as u32).any(|x|c.pixmap.pixel(x,y).is_some_and(|p|p.red()>210&&p.green()>210&&p.blue()>210))),"colored lane icon should be white: {action:?}");
        }
        a.mouse_move(r.cx(),r.cy());a.mouse_down();a.mouse_up();
        assert!(record_armed(&a.state,7));assert!(crate::record::Opt::Notes.on());assert_eq!(a.state.selected_channel,1);
        assert!(a.audio_rec.is_none(),"arming does not start microphone capture");
        a.act(action.clone());assert!(!record_armed(&a.state,7));
        a.state.mixer.recording.inputs.insert(1,crate::capture::Input{device:"Test ASIO".into(),asio:true,first:0,stereo:false});
        a.act(action.clone());assert!(a.state.mixer.recording.armed.contains(&1));assert!(record_armed(&a.state,7));assert!(a.audio_rec.is_none());
        a.act(action);assert!(!a.state.mixer.recording.armed.contains(&1));
        crate::settings_file::set_flag(VISIBILITY,true);
    }
    #[test]
    fn attached_reordering_remaps_bindings_but_preserves_arrangement_rows(){
        let mut a=app();bind(&mut a.state,0,3);let before=a.state.project.clone();toggle(&mut a);
        mixer::reorder(&mut a.state,3,1,true);clear(&a.state);
        assert_eq!(pl::track_info(&a.state,0).unwrap().attached_mixer,Some(1));assert_eq!(a.state.project.channels[0].mixer_track,1);
        assert_eq!(pl::clips(&a.state),before.arrangement().unwrap().clips.as_slice());
        crate::history::undo(&mut a.state);assert_eq!(a.state.project,before);
    }
    #[test]
    fn attached_projection_is_bounded_and_cached_for_dense_projects(){
        let mut a=app();a.state.project.mixer=(0..130).map(MixerTrack::new).collect();
        pl::arr_mut(&mut a.state).clips=(0..10000).map(|i|Clip::new(i*96,96,(i%400) as u16,ClipSource::Channel((i%5) as u16))).collect();
        toggle(&mut a);let start=std::time::Instant::now();let p=projection(&a.state);let build=start.elapsed();
        let start=std::time::Instant::now();for _ in 0..10000{assert!(Rc::ptr_eq(&p,&projection(&a.state)));}let reuse=start.elapsed();
        assert_eq!(p.order.len(),pl::TRACKS as usize);assert_eq!(p.owners.len(),pl::TRACKS as usize);
        eprintln!("attached projection: 10000 clips / 128 inserts: build {build:?}; 10000 cached lookups {reuse:?}");
    }
    #[test]
    fn attached_fader_pan_and_meter_partial_repaints_match_full_frames(){
        let mut a=app();toggle(&mut a);a.state.selected_mixer=1;
        let mut partial=os_ui::Canvas::new(1440,900,1.).unwrap();let mut full=os_ui::Canvas::new(1440,900,1.).unwrap();
        a.paint(&mut partial,None);a.take_damage();
        for action in [Action::Fader(1),Action::Knob(crate::knobs::KnobId::MixPan(1))] {
            let r=a.hits.iter().find(|h|h.action==action).unwrap().rect;a.mouse_move(r.cx(),r.cy());a.mouse_down();
            a.paint(&mut partial,None);a.take_damage();
            a.mouse_move(r.cx(),r.cy()+14.);
            if let Some(d)=a.take_damage(){a.paint_region(&mut partial,d.as_deref());}
            a.paint(&mut full,None);
            assert!(partial.pixmap.data().iter().zip(full.pixmap.data()).all(|(a,b)|a.abs_diff(*b)<=3),"{action:?} repaint");a.mouse_up();a.paint(&mut partial,None);a.take_damage();
        }
        let rect=a.live_rect(crate::app::Live::Meter(1)).unwrap();
        a.state.mixer.meters.resize(8,Default::default());a.state.mixer.meters[1].db=[-6.,-9.];
        a.paint(&mut partial,Some(rect));a.paint(&mut full,None);assert!(partial.pixmap.data().iter().zip(full.pixmap.data()).all(|(a,b)|a.abs_diff(*b)<=3),"meter repaint");
    }
    #[test]
    fn compact_lane_record_sits_between_tuning_and_mute() {
        let mut a = app();
        toggle(&mut a);
        a.state.playlist.row_h = 22.;
        let mut canvas = os_ui::Canvas::new(1440,900,1.).unwrap();
        a.paint(&mut canvas,None);
        let rect = |action| a.hits.iter().find(|h|h.action==action && h.rect.h>0.).unwrap().rect;
        let timing = rect(Action::Warp(crate::warp::Command::Toggle(7)));
        let tuning = rect(Action::Vocal(crate::vocal::Command::Toggle(7)));
        let record = rect(Action::Mix(mixer::MHit::AttachedRecord(7)));
        let mute = rect(Action::PlLed(7));
        assert!(timing.right() <= tuning.x && tuning.right() <= record.x && record.right() <= mute.x);
    }

    #[test]
    fn attached_existing_instrument_records_midi_in_its_projected_lane(){
        let mut a=app();toggle(&mut a);record_lane(&mut a,7);
        assert_eq!(crate::smart_recording::instrument_lane(&a.state).unwrap().0,7);
        let mut canvas = os_ui::Canvas::new(1440, 900, 1.).unwrap();
        a.paint(&mut canvas, None);
        let button = a.hits.iter().find(|h| h.action==Action::Mix(mixer::MHit::AttachedRecord(7))).unwrap().rect;
        let center = (button.cx() as u32, button.cy() as u32);
        let armed = canvas.pixmap.pixel(center.0,center.1).unwrap();
        a.transport.pretend_at(0.,true);a.record_tick();
        assert_eq!(a.state.rec.note_record_row, Some(7));
        a.paint(&mut canvas, None);
        let active = canvas.pixmap.pixel(center.0,center.1).unwrap();
        assert!(active.red()>230 && active.green()<60 && active!=armed);
        a.record_down_with(crate::step_edit::Src::Midi(60),60,100,crate::record::Extra::default());
        a.transport.pretend_at(96.,true);a.record_up_with(crate::step_edit::Src::Midi(60),None);a.transport.stop();a.record_tick();
        assert_eq!(a.state.rec.note_record_row, None);
        assert!(a.state.project.pattern(a.state.pattern).unwrap().notes.iter().any(|n|n.channel==1&&n.key==60));
        assert!(pl::clips(&a.state).iter().any(|c|c.track==7&&c.source==ClipSource::Pattern(a.state.pattern)));
        let recorded=a.state.project.clone();toggle(&mut a);assert_eq!(a.state.project,recorded);
    }
}
