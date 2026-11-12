// SPDX-License-Identifier: GPL-3.0-or-later
//! Bounded voice workflows: validate destinations before editing, then wait for real plugin loads.
use serde_json::Value;
use std::{collections::VecDeque, time::{Duration,Instant}};
use crate::{app::App, ai_commands};

pub struct Plan {
    steps: VecDeque<Value>,
    last_channel: Option<u16>,
    waiting: Option<(u16, Instant)>,
    waiting_effect: Option<(usize,Instant)>,
    completed: usize,
}
impl Plan {
    pub fn prepare(app: &App, value: Value) -> Result<Self,String> {
        let steps=if let Some(steps)=value.get("steps") { steps.as_array().ok_or("Invalid command plan")?.clone() } else {vec![value]};
        if steps.is_empty() || steps.len()>12 {return Err("Please ask for between one and twelve actions.".into());}
        let tools=ai_commands::voice_tools();
        let mut has_channel=false;
        for step in &steps {
            let tool=step["tool"].as_str().ok_or("Missing command name")?;
            if !tools.iter().any(|t| t["name"]==tool) {return Err("That command is not supported.".into());}
            let args=step["arguments"].as_object().ok_or("Invalid command arguments")?;
            for key in ["track","destination"] {
                if let Some(target)=args.get(key) {
                    // Playlist rows have their own index space; do not treat them as mixer inserts.
                    if tool=="lemon_clips" || tool=="lemon_menu" {continue;}
                    let t=target.as_u64().filter(|v| *v <= u16::MAX as u64).ok_or("Invalid mixer track")? as u16;
                    if crate::mixer::track(&app.state,t).is_none() || t>crate::mixer::max_insert(&app.state.project) {
                        return Err(format!("Mixer track {t} does not exist. No actions were applied."));
                    }
                    if crate::mixer::locked(&app.state,t) { return Err(format!("Mixer track {t} is locked.")); }
                }
            }
            if tool=="lemon_instrument" {
                ai_commands::instrument_index(app,args.get("name").and_then(Value::as_str).ok_or("Missing instrument name")?)?;
                has_channel=true;
            }
            if tool=="lemon_plugins" && step["arguments"]["op"]=="add_instrument" {has_channel=true;}
            if args.get("row").and_then(Value::as_str)==Some("last") && !has_channel {
                return Err("Which channel should I use? Name it or select it first.".into());
            }
        }
        Ok(Self {steps:steps.into(),last_channel:None,waiting:None,waiting_effect:None,completed:0})
    }
    /// Runs at most one host operation per UI tick. The next step sees the actual load result.
    pub fn advance(&mut self, app: &mut App) -> Result<bool,String> {
        if let Some((channel,since))=self.waiting {
            let row=app.state.project.channels.iter().position(|c|c.index==channel).ok_or("The new channel was removed.")?;
            if app.state.pending_plugin.is_some_and(|(r,_)|r==row) {
                if since.elapsed()>Duration::from_secs(30) {return Err("Instrument loading timed out; remaining steps stopped.".into());}
                return Ok(false);
            }
            if !app.state.hosted_rows.contains(&row) {return Err("Instrument loading failed; remaining steps stopped.".into());}
            self.waiting=None;
        }
        if let Some((key,since))=self.waiting_effect {
            if !app.plugins.hosted.contains_key(&key) {
                if since.elapsed()>Duration::from_secs(30) {return Err("Effect did not finish loading; remaining actions stopped.".into());}
                return Ok(false);
            }
            if let Some((track,slot))=crate::plugins::fx_of(key) {app.open_fx(track,slot);}
            self.waiting_effect=None;
        }
        let Some(mut step)=self.steps.pop_front() else {return Ok(true)};
        let tool=step["tool"].as_str().unwrap_or("").to_owned();
        if let Some(reference)=step["arguments"]["row"].as_str() {
            let row=match reference {
                "last" => self.last_channel.and_then(|id|app.state.project.channels.iter().position(|c|c.index==id)).ok_or("The instrument from the previous step is unavailable.")?,
                "selected" => app.state.selected_channel,
                _ => return Err("Unknown channel reference.".into()),
            };
            step["arguments"]["row"]=serde_json::json!(row);
        }
        let before=app.state.project.channels.len();
        let result=ai_commands::call_voice(app,&tool,&step["arguments"])
            .map_err(|e|format!("Stopped after {} completed actions: {e}",self.completed))?;
        self.completed+=1;
        if tool=="lemon_instrument" || (tool=="lemon_plugins" && step["arguments"]["op"]=="add_instrument") {
            let row=result["row"].as_u64().map(|r|r as usize).unwrap_or(before);
            let channel=app.state.project.channels.get(row).ok_or("Instrument did not create a channel.")?.index;
            self.last_channel=Some(channel);
            if app.state.pending_plugin.is_some_and(|(r,_)|r==row) {self.waiting=Some((channel,Instant::now()));}
        }
        if tool=="lemon_effect" {self.waiting_effect=result["hosted_key"].as_u64().map(|key|(key as usize,Instant::now()));}
        if !self.steps.is_empty() && app.modal_open() && self.steps.front().is_none_or(|s|s["tool"]!="lemon_text") {
            return Err("Complete the opened dialog before continuing with the remaining actions.".into());
        }
        Ok(self.steps.is_empty() && self.waiting.is_none() && self.waiting_effect.is_none())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    fn run(app:&mut App,value:Value) {
        let mut plan=Plan::prepare(app,value).unwrap();
        let end=Instant::now()+Duration::from_secs(30);
        while !plan.advance(app).unwrap() {
            app.plugin_tick();
            assert!(Instant::now()<end,"plugin load timeout: {:?}",app.state.message);
            std::thread::sleep(Duration::from_millis(10));
        }
    }
    #[test]
    fn voice_plan_routes_real_wave_and_reuses_eq() {
        let mut app=App::new(None);
        app.plugins=crate::plugins::Plugins::builtins();
        app.state.plugdb.rebuild(&app.plugins);
        let n=app.state.project.channels.len();
        run(&mut app,json!({"steps":[
            {"tool":"lemon_transport","arguments":{"op":"show","window":"rack"}},
            {"tool":"lemon_instrument","arguments":{"mode":"load","name":"ML Wave"}},
            {"tool":"lemon_channel","arguments":{"op":"route","row":"last","track":8}}
        ]}));
        assert_eq!(app.state.project.channels.len(),n+1);
        assert_eq!(app.state.project.channels[n].mixer_track,8);
        assert_eq!(app.plugins.descriptor(n).unwrap().name,"ML Wave");
        assert!(app.state.hosted_rows.contains(&n));
        let eq=json!({"steps":[{"tool":"lemon_eq","arguments":{"track":13}}]});
        run(&mut app,eq.clone());
        let count=app.plugins.hosted.len();
        let slots=crate::mixer::track(&app.state,13).unwrap().slots.clone();
        run(&mut app,eq);
        assert_eq!(app.state.selected_mixer,13);
        assert_eq!(app.plugins.hosted.len(),count,"opening EQ twice must not duplicate it");
        assert!(slots[crate::mini_eq::SLOT as usize].is_some());
        assert_eq!(crate::mixer::track(&app.state,13).unwrap().slots,slots);
        let invalid=json!({"steps":[{"tool":"lemon_instrument","arguments":{"mode":"load","name":"ML Wave"}},
            {"tool":"lemon_channel","arguments":{"op":"route","row":"last","track":99999}}]});
        assert!(Plan::prepare(&app,invalid).is_err());
        assert_eq!(app.state.project.channels.len(),n+1,"invalid later destination must be caught before adding a channel");
    }
    #[test]
    #[ignore="Uses downloaded local model, no microphone"]
    fn voice_plan_real_language_model() {
        let root=std::path::PathBuf::from(std::env::var_os("ML_VOICE_ASSETS").unwrap());
        let cancel=std::sync::atomic::AtomicBool::new(false);
        let mut app=App::new(None);
        app.plugins=crate::plugins::Plugins::builtins();
        app.state.plugdb.rebuild(&app.plugins);
        for request in ["Open the channel rack, load up ML Wave and then route it to track 8 in the mixer", "Open up EQ on track 13", "Could you bring up the EQ for mixer track thirteen please?"] {
            let before=app.state.project.channels.len();
            let context=ai_commands::voice_context(&mut app);
            let start=Instant::now();
            let plan=crate::voice_runtime::choose_plan(&root,request,&cancel,&context).unwrap().expect("action plan");
            eprintln!("{request} -> {plan} ({:?})",start.elapsed());
            run(&mut app,plan);
            if request.contains("ML Wave") {
                assert_eq!(app.state.project.channels.len(),before+1);
                assert_eq!(app.state.project.channels[before].mixer_track,8);
                assert_eq!(app.plugins.descriptor(before).unwrap().name,"ML Wave");
            } else {
                assert_eq!(app.state.project.channels.len(),before);
                assert_eq!(app.state.selected_mixer,13);
                assert!(crate::mixer::track(&app.state,13).unwrap().slots[crate::mini_eq::SLOT as usize].is_some());
            }
        }
    }
    #[test]
    #[ignore="Uses synthetic speech and downloaded models; never opens microphone"]
    fn voice_plan_synthetic_speech() {
        let root=std::path::PathBuf::from(std::env::var_os("ML_VOICE_ASSETS").unwrap());
        let fixture=std::path::PathBuf::from(std::env::var_os("ML_VOICE_FIXTURE").unwrap()).join("wave-route.wav");
        let wave=std::fs::read(fixture).unwrap();
        let offset=wave.windows(4).position(|w|w==b"data").unwrap();
        let count=u32::from_le_bytes(wave[offset+4..offset+8].try_into().unwrap()) as usize;
        let samples:Vec<_>=wave[offset+8..offset+8+count].chunks_exact(2).map(|v|i16::from_le_bytes([v[0],v[1]])).collect();
        let mut app=App::new(None); app.plugins=crate::plugins::Plugins::builtins(); app.state.plugdb.rebuild(&app.plugins);
        let n=app.state.project.channels.len();
        let context=ai_commands::voice_context(&mut app);
        let now=Instant::now();
        let intent=crate::voice_runtime::interpret(&root,&samples,16000,&std::sync::atomic::AtomicBool::new(false),&context).unwrap();
        eprintln!("Synthetic speech {:?} -> {:?} in {:?}",intent.transcript,intent.command,now.elapsed());
        run(&mut app,intent.command.unwrap());
        assert_eq!(app.state.project.channels.len(),n+1);
        assert_eq!(app.plugins.descriptor(n).unwrap().name,"ML Wave");
        assert_eq!(app.state.project.channels[n].mixer_track,8);
    }

}
