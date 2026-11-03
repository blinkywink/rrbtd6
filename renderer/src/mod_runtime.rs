//! Opt-in event scripts. Compiled once per match, never loaded from disk during a tick.
use std::collections::{BTreeMap, BTreeSet};
use rhai::{Array, AST, CallFnOptions, Dynamic, Engine, Scope};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use crate::game::Game;

pub const DAMAGE:u8=1;
pub const FIRE:u8=2;
pub const PLACE:u8=4;
pub const SELL:u8=8;
pub const UPGRADE:u8=16;
#[derive(Clone,Debug,Default,Deserialize)]
#[serde(default,deny_unknown_fields)]
pub struct Decision {
    pub cancel:bool,pub unsubscribe:bool,pub damage:Option<f64>,pub count:Option<u32>,
    pub speed:Option<f32>,pub pierce:Option<f32>,pub direction:Option<f32>,
}
impl Decision {
    fn validate(&self,event:&str)->Result<(),String>{
        if self.damage.is_some_and(|v|!v.is_finite()||!(0.0..=1e9).contains(&v))
            ||self.count.is_some_and(|n|!(1..=256).contains(&n))
            ||self.speed.is_some_and(|v|!v.is_finite()||!(0.0..=1e5).contains(&v))
            ||self.pierce.is_some_and(|v|!v.is_finite()||!(0.0..=1e9).contains(&v))
            ||self.direction.is_some_and(|v|!v.is_finite()||v.abs()>1e4)
            ||event!="before_fire"&&(self.count.is_some()||self.speed.is_some()||self.pierce.is_some()||self.direction.is_some())
            ||!matches!(event,"before_fire"|"before_damage")&&self.damage.is_some(){return Err("Invalid interception decision fields".into());}
        Ok(())
    }
}
// JSON preserves integer/float coercion for optional numeric fields in Rhai maps.
fn decode<T:serde::de::DeserializeOwned>(value:&Dynamic)->Result<T,String>{
    let json:Value=rhai::serde::from_dynamic(value).map_err(|e|e.to_string())?;
    if serde_json::to_vec(&json).map_err(|e|e.to_string())?.len()>262144{return Err("Callback output exceeds 256 KiB".into());}
    serde_json::from_value(json).map_err(|e|e.to_string())
}
fn state_size(state:&Dynamic)->Result<(),String>{
    let state:Value=rhai::serde::from_dynamic(state).map_err(|e|e.to_string())?;
    if serde_json::to_vec(&state).map_err(|e|e.to_string())?.len()>65536{return Err("Script state exceeds 64 KiB".into());}Ok(())
}

fn context(g:&Game,event:&Value)->Dynamic{
    #[derive(Serialize)]
    struct Context<'a>{cash:f64,lives:f64,round:usize,tick:u64,popped:u64,tower_count:usize,bloon_count:usize,sandbox:bool,event:&'a Value}
    rhai::serde::to_dynamic(Context{cash:g.cash,lives:g.lives,round:g.round,tick:g.sim_elapsed,popped:g.popped,
        tower_count:g.towers.len(),bloon_count:g.bloons.len(),sandbox:g.sandbox,event}).unwrap()
}

pub fn interval()->u32 {60}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Definition {
    pub id:String, pub source:String,
    #[serde(default="interval")] pub tick_interval:u32,
}
fn engine()->Engine {
    let mut e=Engine::new();
    e.set_module_resolver(rhai::module_resolvers::StaticModuleResolver::new());
    e.set_max_operations(30_000).set_max_call_levels(16).set_max_expr_depths(32,32)
        .set_max_string_size(8192).set_max_array_size(8192).set_max_map_size(8192);
    e.disable_symbol("eval");
    e.on_print(|_|{});e.on_debug(|_,_,_|{});
    e
}
impl Definition {
    pub fn validate(&self)->Result<(),String>{
        if self.id.is_empty()||self.id.len()>160||!self.id.bytes().all(|b|b.is_ascii_alphanumeric()||b"-_:".contains(&b))
            ||self.source.len()>32768||!(1..=3600).contains(&self.tick_interval){return Err("Invalid script ID, size or tick interval".into());}
        engine().compile(&self.source).map_err(|e|format!("Script {}: {e}",self.id))?;
        Ok(())
    }
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Widget {
    pub id:String, pub tree:Value,
    #[serde(default)] pub actions:BTreeMap<String,String>,
    #[serde(default)] pub modal:bool,
}
impl Widget {
    fn validate(&self)->Result<(),String>{
        fn walk(v:&Value,depth:usize,count:&mut usize)->bool{
            *count+=1;
            if depth>12||*count>128||!v.is_object(){return false;}
            if !v["name"].as_str().is_some_and(|s|!s.is_empty()&&s.len()<=96&&!s.contains('/')){return false;}
            v.get("children").is_none_or(|c|c.as_array().is_some_and(|a|a.iter().all(|n|walk(n,depth+1,count))))
        }
        if self.id.is_empty()||self.id.len()>64||!walk(&self.tree,0,&mut 0)||self.actions.len()>32
            ||self.actions.iter().any(|(p,e)|p.len()>512||!event_name(e)){
            return Err("Invalid widget tree, ID or event binding".into());
        }
        Ok(())
    }
}
fn event_name(s:&str)->bool{!s.is_empty()&&s.len()<=64&&s.bytes().all(|b|b.is_ascii_alphanumeric()||b==b'_')}
#[derive(Clone,Debug,Deserialize)]
#[serde(tag="op",rename_all="snake_case",deny_unknown_fields)]
enum Command {
    Listen {event:String,enabled:bool},
    Inspect {query:crate::mod_entities::Query},
    EntityPatch {patch:crate::mod_entities::Patch},
    DamageBloon {id:u64,#[serde(default)]layer:Option<u64>,amount:i32},
    Cash {amount:f64}, Lives {value:f64},
    Spawn {name:String,count:u32,spacing:f32},
    ClearBloons, ResetCooldowns,
    Ui {widget:Widget}, RemoveUi {id:String},
    Popup {title:String,body:String},
}
impl Command {
    fn validate(&self,g:&Game)->Result<(),String>{
        let valid=match self {
            Self::Listen{event,..}=>event_name(event),
            Self::Inspect{query}=>return query.validate(),
            Self::EntityPatch{patch}=>return patch.validate(),
            Self::DamageBloon{id,amount,..}=>*id>0&&(0..=1_000_000_000).contains(amount),
            Self::Cash{amount}=>amount.is_finite()&&amount.abs()<=1e9,
            Self::Lives{value}=>value.is_finite()&&(0.0..=1e9).contains(value),
            Self::Spawn{name,count,spacing}=>*count<=256&&spacing.is_finite()&&(0.0..=60.0).contains(spacing)&&g.mod_has_bloon(name),
            Self::Ui{widget}=>return widget.validate(),
            Self::RemoveUi{id}=>id.len()<=64,
            Self::Popup{title,body}=>title.len()<=160&&body.len()<=4096,
            _=>true,
        };
        if valid {Ok(())}else{Err("Invalid script command".into())}
    }
}
struct Script {id:String,ast:AST,state:Dynamic,events:BTreeSet<String>,muted:BTreeSet<String>,interval:u32,disabled:bool}
#[derive(Clone)]
pub struct OwnedWidget {pub owner:String,pub widget:Widget}
#[derive(Default)]
pub struct Runtime {
    engine:Option<Engine>,scripts:Vec<Script>,
    pub widgets:Vec<OwnedWidget>,pub revision:u64,pub session:u64,
    pub popup:Option<(String,String)>,
    pub last_towers:usize,
    queries:Vec<(String,crate::mod_entities::Query)>,
    damage:Vec<(u64,Option<u64>,i32)>,
    events:BTreeSet<String>,
    pub intercepts:u8,
    fuel:Option<std::sync::Arc<std::sync::atomic::AtomicU64>>,fuel_tick:u64,
}
impl Runtime {
    pub fn new(defs:Vec<Definition>)->Self{
        if defs.is_empty(){return Self::default();}
        let mut e=engine();let mut scripts=Vec::new();
        let fuel=std::sync::Arc::new(std::sync::atomic::AtomicU64::new(0));let used=fuel.clone();
        e.on_progress(move |_|{
            (used.fetch_add(1,std::sync::atomic::Ordering::Relaxed)>100_000).then(||Dynamic::from("Shared mod operation budget exceeded"))
        });
        for d in defs {
            match e.compile(&d.source){
                Ok(ast)=>{let events=ast.iter_functions().filter(|f|f.params.len()==1).map(|f|f.name.to_owned()).collect();
                    scripts.push(Script{id:d.id,ast,state:rhai::Map::new().into(),events,muted:BTreeSet::new(),interval:d.tick_interval,disabled:false});},
                Err(err)=>eprintln!("Mod script {} disabled: {err}",d.id),
            }
        }
        static SESSION:std::sync::atomic::AtomicU64=std::sync::atomic::AtomicU64::new(1);
        let mut runtime=Self{engine:Some(e),scripts,fuel:Some(fuel),session:SESSION.fetch_add(1,std::sync::atomic::Ordering::Relaxed),..Self::default()};
        runtime.reindex();runtime
    }
    pub fn pending_entities(&self)->bool{!self.queries.is_empty()||!self.damage.is_empty()}
    pub fn active(&self)->bool{self.scripts.iter().any(|s|!s.disabled)}
    pub fn listens(&self,event:&str)->bool{self.events.contains(event)}
    fn prepare_budget(&mut self,tick:u64){
        if self.fuel_tick!=tick{self.fuel_tick=tick;if let Some(f)=&self.fuel{f.store(0,std::sync::atomic::Ordering::Relaxed);}}
    }
    fn reindex(&mut self){
        self.events=self.scripts.iter().filter(|s|!s.disabled).flat_map(|s|s.events.difference(&s.muted).cloned()).collect();
        let active:BTreeSet<_>=self.scripts.iter().filter(|s|!s.disabled).map(|s|s.id.as_str()).collect();
        let old=self.widgets.len();
        self.widgets.retain(|w|active.contains(w.owner.as_str()));
        self.queries.retain(|(owner,_)|active.contains(owner.as_str()));
        if old!=self.widgets.len(){self.revision+=1;}
        self.intercepts=0;
        for (name,mask) in [("before_damage",DAMAGE),("before_fire",FIRE),("before_place",PLACE),("before_sell",SELL),("before_upgrade",UPGRADE)]{
            if self.events.contains(name){self.intercepts|=mask;}
        }
    }
    pub fn dispatch(&mut self,g:&mut Game,event:&str,owner:Option<&str>,payload:Value){
        self.prepare_budget(g.sim_elapsed);
        let Some(e)=&self.engine else{return};
        let mut reindex=false;
        for script in &mut self.scripts {
            if script.disabled||(!script.events.contains(event)||script.muted.contains(event))||owner.is_some_and(|o|o!=script.id)
                ||event=="on_tick"&&g.sim_elapsed%script.interval as u64!=0 {continue;}
            // Small snapshots only. No serialization of models/entities on each tick.
            let ctx=context(g,&payload);
            let options=CallFnOptions::new().eval_ast(false).bind_this_ptr(&mut script.state);
            let result=e.call_fn_with_options::<Array>(options,&mut Scope::new(),&script.ast,event,(ctx,))
                .map_err(|e|e.to_string()).and_then(|a|decode::<Vec<Command>>(&a.into()));
            let commands=result.and_then(|commands|{
                state_size(&script.state)?;
                if commands.len()>32{return Err("At most 32 commands per callback".into());}
                for command in &commands{
                    command.validate(g)?;
                    if let Command::Listen{event,..}=command {
                        if !script.events.contains(event){return Err(format!("No one-argument callback named {event}"));}
                    }
                }
                if self.queries.len()+commands.iter().filter(|c|matches!(c,Command::Inspect{..})).count()>64
                    ||self.damage.len()+commands.iter().filter(|c|matches!(c,Command::DamageBloon{..})).count()>256{
                    return Err("Pending entity command limit exceeded".into());
                }
                if commands.iter().any(|c|matches!(c,Command::Ui{..}|Command::RemoveUi{..})){
                    let mut ids:BTreeSet<_>=self.widgets.iter().map(|w|(w.owner.clone(),w.widget.id.clone())).collect();
                    for command in &commands{match command{
                        Command::Ui{widget}=>{ids.insert((script.id.clone(),widget.id.clone()));},
                        Command::RemoveUi{id}=>{ids.remove(&(script.id.clone(),id.clone()));},_=>{},
                    }}
                    if ids.len()>64{return Err("At most 64 active mod widgets".into());}
                }
                Ok(commands)
            });
            let commands=match commands {Ok(c)=>c,Err(err)=>{script.disabled=true;reindex=true;eprintln!("Mod script {} disabled in {event}: {err}",script.id);continue;}};
            for command in commands {match command {
                Command::Listen{event,enabled}=>{
                    reindex|=if enabled{script.muted.remove(&event)}else{script.muted.insert(event)};
                },
                Command::Inspect{query}=>self.queries.push((script.id.clone(),query)),
                Command::EntityPatch{patch}=>patch.apply(g),
                Command::DamageBloon{id,layer,amount}=>self.damage.push((id,layer,amount)),
                Command::Cash{amount}=>g.cash=(g.cash+amount).clamp(0.0,1e12),
                Command::Lives{value}=>{g.lives=value;g.last_set_health=value;},
                Command::Spawn{name,count,spacing}=>{g.sandbox_spawn(&name,false,false,false,count,spacing);},
                Command::ClearBloons=>g.clear_bloons(),Command::ResetCooldowns=>g.reset_cooldowns(),
                Command::Ui{widget}=>{self.widgets.retain(|w|w.owner!=script.id||w.widget.id!=widget.id);self.widgets.push(OwnedWidget{owner:script.id.clone(),widget});self.revision+=1;},
                Command::RemoveUi{id}=>{self.widgets.retain(|w|w.owner!=script.id||w.widget.id!=id);self.revision+=1;},
                Command::Popup{title,body}=>{self.popup=Some((title,body));self.revision+=1;},
            }}
        }
        if reindex{self.reindex();}
    }
    pub fn intercept(&mut self,g:&Game,event:&str,mut payload:Value)->Decision{
        self.prepare_budget(g.sim_elapsed);
        let Some(e)=&self.engine else{return Decision::default()};
        let mut result=Decision::default();let mut reindex=false;
        for script in &mut self.scripts{
            if script.disabled||(!script.events.contains(event)||script.muted.contains(event)){continue;}
            let ctx=context(g,&payload);
            let options=CallFnOptions::new().eval_ast(false).bind_this_ptr(&mut script.state);
            let decision=e.call_fn_with_options::<Dynamic>(options,&mut Scope::new(),&script.ast,event,(ctx,))
                .map_err(|e|e.to_string()).and_then(|v|decode::<Decision>(&v))
                .and_then(|d|{d.validate(event)?;state_size(&script.state)?;Ok(d)});
            let d=match decision{Ok(d)=>d,Err(err)=>{script.disabled=true;reindex=true;eprintln!("Mod script {} disabled in {event}: {err}",script.id);continue;}};
            // Only this owner and current hook are muted, after its final decision applies.
            if d.unsubscribe{reindex|=script.muted.insert(event.to_owned());}
            if d.damage.is_some(){result.damage=d.damage;payload["damage"]=json!(d.damage);}
            if d.count.is_some(){result.count=d.count;payload["count"]=json!(d.count);}
            if d.speed.is_some(){result.speed=d.speed;payload["speed"]=json!(d.speed);}
            if d.pierce.is_some(){result.pierce=d.pierce;payload["pierce"]=json!(d.pierce);}
            if d.direction.is_some(){result.direction=d.direction;payload["direction"]=json!(d.direction);}
            if d.cancel{result.cancel=true;break;}
        }
        if reindex{self.reindex();}result
    }
}
impl Game {
    pub fn mod_intercept(&mut self,event:&str,payload:Value)->Decision{
        let mut runtime=std::mem::take(&mut self.mod_runtime);
        let result=runtime.intercept(self,event,payload);self.mod_runtime=runtime;result
    }
    pub fn mod_event(&mut self,event:&str,owner:Option<&str>){
        self.mod_event_payload(event,owner,Value::Null);
    }
    pub fn mod_event_payload(&mut self,event:&str,owner:Option<&str>,payload:Value){
        if !self.mod_runtime.listens(event){return;}
        let mut runtime=std::mem::take(&mut self.mod_runtime);
        runtime.dispatch(self,event,owner,payload);
        self.mod_runtime=runtime;
    }
    pub fn mod_entity_commands(&mut self){
        // Take the current batch. Commands requested by query callbacks wait for the next step.
        let n=self.mod_runtime.queries.len().min(8);
        let queries:Vec<_>=self.mod_runtime.queries.drain(..n).collect();
        let damage=std::mem::take(&mut self.mod_runtime.damage);
        for (owner,query) in queries {let payload=query.snapshot(self);self.mod_event_payload("on_query",Some(&owner),payload);}
        for (id,layer,amount) in damage {self.mod_damage_bloon(id,layer,amount);}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn runtime_cannot_import_files_and_limits_infinite_loops(){
        let e=engine();let mut scope=Scope::new();
        assert!(e.eval::<Dynamic>("loop {}").is_err());
        assert!(e.eval::<Dynamic>("import \"C:/Windows/win.ini\" as x;").is_err());
        let ast=e.compile("fn run(ctx) { this.n = 5; [#{op: \"cash\", amount: this.n * 2}] }").unwrap();
        let mut state:rhai::Dynamic=rhai::Map::new().into();
        let out=e.call_fn_with_options::<Array>(CallFnOptions::new().eval_ast(false).bind_this_ptr(&mut state),&mut scope,&ast,"run",(rhai::Map::new(),)).unwrap();
        let commands:Vec<Command>=rhai::serde::from_dynamic(&out.into()).unwrap();
        assert!(matches!(commands[0],Command::Cash{amount} if amount==10.0));
        assert_eq!(state.cast::<rhai::Map>()["n"].as_int().unwrap(),5);
    }
    #[test]fn decisions_are_specific_to_the_interception_phase(){
        assert!(Decision{damage:Some(2.0),..Default::default()}.validate("before_damage").is_ok());
        assert!(Decision{damage:Some(2.0),..Default::default()}.validate("before_place").is_err());
        assert!(Decision{count:Some(0),..Default::default()}.validate("before_fire").is_err());
        assert!(Decision{speed:Some(f32::NAN),..Default::default()}.validate("before_fire").is_err());
        let mut r=Runtime::new(vec![Definition{id:"fuel".into(),tick_interval:1,source:"fn before_damage(ctx) { #{} }".into()}]);
        assert_eq!(r.intercepts,DAMAGE);
        r.fuel.as_ref().unwrap().store(100_001,std::sync::atomic::Ordering::Relaxed);
        assert!(r.engine.as_ref().unwrap().eval::<Dynamic>("let a=1; a+2").is_err());
        r.prepare_budget(1);
        assert!(r.engine.as_ref().unwrap().eval::<Dynamic>("let a=1; a+2").is_ok());
        r.widgets.push(OwnedWidget{owner:"fuel".into(),widget:Widget{id:"modal".into(),tree:json!({"name":"Modal","active":true}),actions:Default::default(),modal:true}});
        r.scripts[0].disabled=true;r.reindex();
        assert!(r.widgets.is_empty(),"a failed script must not leave an undismissable modal");
        assert_eq!(r.intercepts,0);assert!(r.revision>0);
    }
    #[test]fn muted_hooks_keep_owner_widgets_and_other_subscribers(){
        let defs=["a","b"].map(|id|Definition{id:id.into(),tick_interval:60,
            source:"fn before_fire(ctx) { #{} } fn on_open(ctx) { [] }".into()});
        let mut r=Runtime::new(defs.into());
        r.widgets.push(OwnedWidget{owner:"a".into(),widget:Widget{id:"launcher".into(),tree:json!({"name":"Open"}),actions:Default::default(),modal:false}});
        r.scripts[0].muted.insert("before_fire".into());r.reindex();
        assert_eq!(r.intercepts,FIRE);assert_eq!(r.widgets.len(),1);
        r.scripts[1].muted.insert("before_fire".into());r.reindex();
        assert_eq!(r.intercepts,0);assert!(r.listens("on_open"));assert_eq!(r.widgets.len(),1);
        r.scripts[0].muted.remove("before_fire");r.reindex();assert_eq!(r.intercepts,FIRE);
    }
    #[test]
    #[ignore="requires locally extracted game data"]
    fn subscriptions_preserve_state_ui_and_other_owners(){
        let root=std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../local_extracted/game/game.json");
        let mut g=Game::new(serde_json::from_slice(&std::fs::read(root).unwrap()).unwrap());
        g.mod_runtime=Runtime::new(vec![Definition{id:"switch".into(),tick_interval:1,source:r#"
            fn on_new_game(ctx) {
                this.hits=0;
                [#{op:"listen",event:"before_damage",enabled:false},
                 #{op:"ui",widget:#{id:"panel",tree:#{name:"Panel"},modal:true}}]
            }
            fn on_enable(ctx) { [#{op:"listen",event:"before_damage",enabled:true}] }
            fn before_damage(ctx) {this.hits+=1;#{cancel:true,unsubscribe:true}}
            fn on_report(ctx) { [#{op:"cash",amount:this.hits}] }
            fn on_invalid(ctx) { [#{op:"cash",amount:100},#{op:"listen",event:"not_defined",enabled:true}] }
        "#.into()}]);
        g.mod_event("on_new_game",None);
        assert_eq!(g.mod_runtime.intercepts,0);
        assert!(g.mod_runtime.active());assert_eq!(g.mod_runtime.widgets.len(),1);
        assert!(!g.mod_intercept("before_damage",json!({"damage":1})).cancel);
        for expected in 1..=2 {
            g.mod_event("on_enable",Some("switch"));
            assert_eq!(g.mod_runtime.intercepts,DAMAGE);
            assert!(g.mod_intercept("before_damage",json!({"damage":1})).cancel);
            assert_eq!(g.mod_runtime.intercepts,0);
            let before=g.cash;g.mod_event("on_report",None);assert_eq!(g.cash,before+expected as f64);
        }
        let before=g.cash;g.mod_event("on_invalid",None);
        assert_eq!(g.cash,before,"invalid subscriptions reject the whole command batch");
        assert!(g.mod_runtime.widgets.is_empty());assert!(!g.mod_runtime.active());

        g.mod_runtime=Runtime::new(vec![
            Definition{id:"first".into(),tick_interval:1,source:"fn before_damage(ctx) { #{damage:2,unsubscribe:true} }".into()},
            Definition{id:"second".into(),tick_interval:1,source:"fn before_damage(ctx) { #{damage:ctx.event.damage*3} }".into()},
        ]);
        assert_eq!(g.mod_intercept("before_damage",json!({"damage":1})).damage,Some(6.0));
        assert_eq!(g.mod_runtime.intercepts,DAMAGE,"other owners remain subscribed");
        assert_eq!(g.mod_intercept("before_damage",json!({"damage":1})).damage,Some(3.0));
    }
    #[test]
    #[ignore="requires locally extracted game data"]
    fn entity_lab_features_unsubscribe_when_idle(){
        let root=std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
        let mut g=Game::new(serde_json::from_slice(&std::fs::read(root.join("local_extracted/game/game.json")).unwrap()).unwrap());
        g.mod_runtime=Runtime::new(vec![Definition{id:"lab".into(),tick_interval:60,
            source:std::fs::read_to_string(root.join("tools/mod_examples/entity-lab/entities.rhai")).unwrap()}]);
        g.mod_event("on_new_game",None);
        assert_eq!(g.mod_runtime.intercepts,0);assert!(!g.mod_runtime.listens("on_tick"));
        g.mod_event("on_open",None);assert!(g.mod_runtime.listens("on_tick"));
        g.mod_event("on_boost",None);assert_eq!(g.mod_runtime.intercepts,FIRE);
        g.mod_event("on_boost",None);assert_eq!(g.mod_runtime.intercepts,0);
        g.mod_event("on_shield",None);
        for _ in 0..5 {assert!(g.mod_intercept("before_damage",json!({"damage":1})).cancel);}
        assert_eq!(g.mod_runtime.intercepts,0);
        g.mod_event("on_close",None);assert!(!g.mod_runtime.listens("on_tick"));
        assert_eq!(g.mod_runtime.widgets.len(),1);
        g.mod_event("on_open",None);assert_eq!(g.mod_runtime.widgets.len(),2);
        assert!(g.mod_runtime.active());
    }
    #[test]
    #[ignore="requires locally extracted game data"]
    fn widget_replacement_at_capacity_and_failure_cleanup(){
        let root=std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../local_extracted/game/game.json");
        let mut g=Game::new(serde_json::from_slice(&std::fs::read(root).unwrap()).unwrap());
        g.mod_runtime=Runtime::new(vec![Definition{id:"limits".into(),tick_interval:60,source:r#"
            fn batch(start) {
                let out=[];
                for n in start..(start+32) {out.push(#{op:"ui",widget:#{id:"w"+n,tree:#{name:"Panel",active:true},modal:true}});}
                out
            }
            fn on_new_game(ctx) { batch(0) }
            fn on_fill(ctx) { batch(32) }
            fn on_update(ctx) { [#{op:"ui",widget:#{id:"w0",tree:#{name:"Updated",active:true},modal:true}}] }
            fn on_overflow(ctx) { [#{op:"ui",widget:#{id:"extra",tree:#{name:"Overflow",active:true},modal:true}}] }
        "#.into()}]);
        g.mod_event("on_new_game",None);g.mod_event("on_fill",None);
        assert_eq!(g.mod_runtime.widgets.len(),64);
        g.mod_event("on_update",None);
        assert_eq!(g.mod_runtime.widgets.len(),64);
        assert!(g.mod_runtime.active());
        g.mod_event("on_overflow",None);
        assert!(!g.mod_runtime.active());
        assert!(g.mod_runtime.widgets.is_empty(),"failed modal owner must release input");
    }
    #[test]
    #[ignore="requires locally extracted game data"]
    fn workshop_runs_ui_mechanics_and_isolates_bad_callbacks(){
        let root=std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../local_extracted");
        let data=std::fs::read_to_string(root.join("game/game.json")).unwrap();
        let manifest=crate::modding::Manifest::parse(&std::fs::read(root.join("../mods/workshop/mod.json")).unwrap()).unwrap();
        let compiled=crate::modding::content::compile(&data,&[&manifest]).unwrap();
        let mut game=Game::new(serde_json::from_str(&compiled).unwrap());
        assert_eq!(game.mod_runtime.widgets.len(),1);
        let owner="workshop:mechanics";
        game.mod_event("on_open",Some(owner));
        assert_eq!(game.mod_runtime.widgets.len(),2);
        assert!(game.mod_runtime.widgets[1].widget.modal);
        game.mod_event("on_bonus",Some(owner));
        let before=game.cash;
        game.mod_event("on_round_end",Some(owner));
        assert_eq!(game.cash,before+150.0);
        game.mod_event("on_wave",Some(owner));
        assert_eq!(game.mod_runtime.popup.as_ref().unwrap().0,"Custom wave");
        game.mod_event("on_close",Some(owner));
        assert_eq!(game.mod_runtime.widgets.len(),1);
        let sn=game.shop.iter().position(|s|s.id=="SniperMonkey").unwrap();
        assert_eq!(game.shop[sn+1].id,"DartMonkey");
        game.mod_runtime=Runtime::new(vec![Definition{id:"broken".into(),tick_interval:1,
            source:"fn on_tick(ctx) { [#{op:\"cash\",amount:999}, #{op:\"lives\",value:-1}] }".into()}]);
        let before=game.cash;
        game.mod_event("on_tick",None);
        assert_eq!(game.cash,before,"invalid batches must not apply earlier valid commands");
        assert!(!game.mod_runtime.active());
        game.mod_runtime=Runtime::new(vec![Definition{id:"loop".into(),tick_interval:1,
            source:"fn on_tick(ctx) { loop {} [] }".into()}]);
        game.mod_event("on_tick",None);
        assert!(!game.mod_runtime.active());
    }
}
