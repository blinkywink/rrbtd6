//! Versioned, data-only local mods. No work runs on the frame/simulation loop.
use serde::{Deserialize, Serialize};
use std::{collections::{HashSet,BTreeMap}, path::{Path, PathBuf}};

#[path="mod_content.rs"]
pub mod content;

pub const API_VERSION:u32=1;
const MAX_MANIFEST:u64=64*1024;
const MAX_MODS:usize=256;

#[derive(Clone,Debug,Deserialize,Serialize)]
#[serde(deny_unknown_fields)]
pub struct Manifest {
    pub api_version:u32,
    pub id:String,
    pub name:String,
    pub version:String,
    #[serde(default)] pub description:String,
    #[serde(default)] pub on_new_sandbox:SandboxRules,
    #[serde(default)] pub settings:SandboxSettings,
    #[serde(default)] pub content:Vec<content::Edit>,
}
#[derive(Clone,Debug,Default,Deserialize,Serialize,PartialEq)]
#[serde(deny_unknown_fields)]
pub struct SandboxRules {
    pub starting_cash:Option<f64>,
    pub starting_lives:Option<f64>,
}
/// Optional editor controls for the existing Sandbox hooks, not new simulation hooks.
#[derive(Clone,Debug,Default,Deserialize,Serialize)]
#[serde(deny_unknown_fields)]
pub struct SandboxSettings {
    pub starting_cash:Option<NumberSetting>,
    pub starting_lives:Option<NumberSetting>,
}
#[derive(Clone,Debug,Deserialize,Serialize)]
#[serde(deny_unknown_fields)]
pub struct NumberSetting {pub min:f64,pub max:f64,pub step:f64}
pub type Overrides=BTreeMap<String,serde_json::Value>;
pub fn overrides(root:&Path)->Overrides {
    serde_json::from_value(crate::read_settings(root)["mod_settings"].clone()).unwrap_or_default()
}
impl Manifest {
    pub fn resolved(&self,overrides:&Overrides)->Result<SandboxRules,String>{
        let Some(value)=overrides.get(&self.id) else{return Ok(self.on_new_sandbox.clone());};
        let saved:SandboxRules=serde_json::from_value(value.clone()).map_err(|_|format!("{}: invalid settings. Reset this mod's settings.",self.name))?;
        let mut rules=self.on_new_sandbox.clone();
        for (key,control,value,target) in [
            ("starting_cash",&self.settings.starting_cash,saved.starting_cash,&mut rules.starting_cash),
            ("starting_lives",&self.settings.starting_lives,saved.starting_lives,&mut rules.starting_lives)] {
            if let Some(value)=value {
                let valid=control.as_ref().is_some_and(|c|c.accepts(value));
                if !valid{return Err(format!("{}: {key} settings are no longer valid. Reset this mod's settings.",self.name));}
                *target=Some(value);
            }
        }
        Ok(rules)
    }
}
impl NumberSetting {
    pub fn accepts(&self,value:f64)->bool {
        value.is_finite()&&value.fract()==0.0&&value>=self.min&&value<=self.max
            &&(value-self.min)%self.step==0.0
    }
    pub fn adjust(&self,value:f64,direction:f64)->f64 {(value+direction*self.step).clamp(self.min,self.max)}
}
impl SandboxRules {
    fn apply(&self,local:bool,sandbox:bool,cash:&mut f64,lives:&mut f64){
        if !local||!sandbox{return;}
        if let Some(value)=self.starting_cash{*cash=value;}
        if let Some(value)=self.starting_lives{*lives=value;}
    }
}
impl Manifest {
    pub fn parse(bytes:&[u8])->Result<Self,String>{
        if bytes.len() as u64>MAX_MANIFEST{return Err("Manifest exceeds 64 KiB".into());}
        let m:Self=serde_json::from_slice(bytes).map_err(|e|format!("Invalid manifest: {e}"))?;
        if m.api_version!=API_VERSION{return Err(format!("Requires mod API {}; this client supports {API_VERSION}",m.api_version));}
        if m.id.is_empty()||m.id.len()>64||!m.id.bytes().all(|b|b.is_ascii_lowercase()||b.is_ascii_digit()||b"._-".contains(&b))
            ||!m.id.as_bytes()[0].is_ascii_alphanumeric(){return Err("ID must be 1-64 lowercase letters, digits, dots, hyphens or underscores; start with a letter or digit".into());}
        for (key,value,max) in [("Name",m.name.as_str(),80),("Version",m.version.as_str(),32),("Description",m.description.as_str(),1024)]{
            if value.len()>max||value.chars().any(char::is_control)||(key!="Description"&&value.trim().is_empty()){
                return Err(format!("{key} is empty, too long, or contains control characters"));
            }
        }
        for (key,value,min,max) in [("starting_cash",m.on_new_sandbox.starting_cash,0.0,1e12),
            ("starting_lives",m.on_new_sandbox.starting_lives,1.0,1e9)]{
            if value.is_some_and(|v|!v.is_finite()||v<min||v>max||v.fract()!=0.0){return Err(format!("{key} must be a whole number from {min} to {max}"));}
        }
        for (key,control,default,min,max) in [
            ("starting_cash",&m.settings.starting_cash,m.on_new_sandbox.starting_cash,0.0,1e12),
            ("starting_lives",&m.settings.starting_lives,m.on_new_sandbox.starting_lives,1.0,1e9)] {
            if let Some(c)=control {
                if ![c.min,c.max,c.step].iter().all(|v|v.is_finite()&&v.fract()==0.0)
                    ||c.min<min||c.max>max||c.min>c.max||c.step<1.0||c.step>max
                    ||(c.max-c.min)%c.step!=0.0||!default.is_some_and(|v|c.accepts(v)) {
                    return Err(format!("{key} settings need whole-number bounds and a positive step; bounds and hook default must align with the step"));
                }
            }
        }
        if m.content.len()>256{return Err("A mod may contain at most 256 content edits".into());}
        for edit in &m.content{edit.validate()?;}
        Ok(m)
    }
}
pub struct Entry {pub folder:String,pub manifest:Result<Manifest,String>}
pub struct Catalog {pub entries:Vec<Entry>}
pub fn directory(root:&Path)->PathBuf {root.parent().unwrap_or(root).join("mods")}
pub fn enabled(root:&Path)->HashSet<String>{
    let settings=crate::read_settings(root);
    settings["mods"].as_array().into_iter().flatten().filter_map(|v|v.as_str().map(str::to_owned)).collect()
}
impl Catalog {
    pub fn scan(dir:&Path)->Self{
        let mut folders:Vec<_>=match std::fs::read_dir(dir){
            Ok(read)=>read.filter_map(Result::ok).filter(|e|e.file_type().is_ok_and(|t|t.is_dir())).map(|e|e.path()).collect(),
            Err(e) if e.kind()==std::io::ErrorKind::NotFound=>return Self{entries:Vec::new()},
            Err(e)=>return Self{entries:vec![Entry{folder:"Mods folder".into(),manifest:Err(format!("Cannot read mods folder: {}",e.kind()))}]},
        };
        folders.sort();
        let overflow=folders.len()>MAX_MODS;
        let mut entries:Vec<_>=folders.into_iter().take(MAX_MODS).map(|folder|{
            let path=folder.join("mod.json");
            let manifest=std::fs::metadata(&path).map_err(|e|format!("Cannot read mod.json: {}",e.kind())).and_then(|m|{
                if m.len()>MAX_MANIFEST{return Err("Manifest exceeds 64 KiB".into());}
                std::fs::read(&path).map_err(|e|format!("Cannot read mod.json: {}",e.kind())).and_then(|b|Manifest::parse(&b))
            });
            Entry{folder:folder.file_name().unwrap_or_default().to_string_lossy().into_owned(),manifest}
        }).collect();
        let mut seen=HashSet::new();let mut duplicates=HashSet::new();
        for m in entries.iter().filter_map(|e|e.manifest.as_ref().ok()){if !seen.insert(m.id.clone()){duplicates.insert(m.id.clone());}}
        for e in &mut entries{if let Ok(m)=&e.manifest{if duplicates.contains(&m.id){e.manifest=Err(format!("Duplicate mod ID: {}",m.id));}}}
        if overflow{entries.push(Entry{folder:"Limit reached".into(),manifest:Err("Only the first 256 mod folders are loaded".into())});}
        Self{entries}
    }
    /// Conflicts fail explicitly rather than depending on directory enumeration order.
    #[cfg(test)]
    pub fn compile(&self,enabled:&HashSet<String>)->Result<SandboxRules,String>{
        self.compile_with(enabled,&Overrides::new())
    }
    pub fn compile_with(&self,enabled:&HashSet<String>,overrides:&Overrides)->Result<SandboxRules,String>{
        let mut rules=SandboxRules::default();let mut found=HashSet::new();
        for m in self.entries.iter().filter_map(|e|e.manifest.as_ref().ok()).filter(|m|enabled.contains(&m.id)){
            found.insert(m.id.clone());
            let effective=m.resolved(overrides)?;
            for (name,target,value) in [("starting cash",&mut rules.starting_cash,effective.starting_cash),
                ("starting lives",&mut rules.starting_lives,effective.starting_lives)]{
                if let Some(value)=value{if target.replace(value).is_some(){return Err(format!("More than one enabled mod sets {name}. Disable one to apply mods."));}}
            }
        }
        if enabled.iter().any(|id|!found.contains(id)){return Err("An enabled mod is missing or invalid. Disable it or fix its manifest to apply mods.".into());}
        Ok(rules)
    }
}

/// Compile at the game-loading boundary, never in a simulation/render frame.
/// Explicit manifest IDs define stable ordering, independent of folder enumeration.
pub fn game_json(root:&Path,json:&str,local:bool)->Result<Option<String>,String>{
    if !local{return Ok(None);}
    let ids=enabled(root);if ids.is_empty(){return Ok(None);}
    let catalog=Catalog::scan(&directory(root));
    if ids.iter().any(|id|!catalog.entries.iter().any(|e|e.manifest.as_ref().is_ok_and(|m|m.id==*id))){return Err("An enabled mod is missing or invalid".into());}
    let mut mods:Vec<_>=catalog.entries.iter().filter_map(|e|e.manifest.as_ref().ok()).filter(|m|ids.contains(&m.id)).collect();
    mods.sort_by(|a,b|a.id.cmp(&b.id));
    if mods.iter().all(|m|m.content.is_empty()){return Ok(None);}
    content::compile(json,&mods).map(Some)
}

pub fn custom_mode_buttons(root:&Path,node:&mut crate::uitree::Node)->Vec<String>{
    use macroquad::prelude::*;
    let ids=enabled(root);if ids.is_empty(){return Vec::new();}
    let catalog=Catalog::scan(&directory(root));
    let mut modes:Vec<_>=catalog.entries.iter().filter_map(|e|e.manifest.as_ref().ok()).filter(|m|ids.contains(&m.id))
        .flat_map(|m|m.content.iter()).filter_map(|e|if let content::Edit::Mode{id,..}=e{Some(id.clone())}else{None}).collect();
    modes.sort();modes.dedup();if modes.is_empty(){return modes;}
    let Some(juke)=crate::uitree::Node::load(&root.join("ui/prefabs/JukeboxUI.json"))else{return Vec::new();};
    let Some(template)=juke.find("Canvas/JukeboxScreen/AvailableTracks/AddAllBtn/AddAllBtn")else{return Vec::new();};
    const BASE:&str="ModeSelectCanvas/ModeSelectScreen";
    for (name,label,x,width) in [("CustomPrev","<",-800.0,180.0),("CustomPlay","Custom Modes",0.0,1350.0),("CustomNext",">",800.0,180.0)]{
        let mut button=template.clone();button.name=name.into();button.active=true;
        button.rt.amin=vec2(0.5,0.0);button.rt.amax=button.rt.amin;button.rt.pivot=vec2(0.5,0.0);
        button.rt.pos=vec2(x,90.0);button.rt.size=vec2(width,170.0);button.rt.scale=Vec2::ONE;
        button.set_text("Text",label);
        if let Some(text)=button.find_mut("Text"){text.rt.size=vec2(width-30.0,150.0);if let Some(t)=&mut text.txt{t.auto=true;t.min=35.0;t.max=65.0;}}
        if let Some(parent)=node.find_mut(BASE){parent.children.push(button);}
    }
    modes
}

pub fn menu_edits(root:&Path,page:&str,node:&mut crate::uitree::Node){
    let ids=enabled(root);if ids.is_empty(){return;}
    let catalog=Catalog::scan(&directory(root));
    let mut manifests:Vec<_>=catalog.entries.iter().filter_map(|e|e.manifest.as_ref().ok()).filter(|m|ids.contains(&m.id)).collect();
    manifests.sort_by(|a,b|a.id.cmp(&b.id));
    for manifest in manifests{for edit in &manifest.content{
        if let content::Edit::Menu{page:target,path,text,visible}=edit{
            if target!=page{continue;}
            if node.find(path).is_none(){eprintln!("{}: menu node not found: {page}/{path}",manifest.name);continue;}
            if let Some(text)=text{node.set_text(path,text);}
            if let Some(visible)=visible{node.set_active(path,*visible);}
        }
    }}
}

pub fn room_packages(root:&Path,ids:&[String])->Result<Vec<rrbtd6_coop::ModPackage>,String>{
    let catalog=Catalog::scan(&directory(root));let overrides=overrides(root);let mut packages=Vec::new();
    let mut sorted=ids.to_vec();sorted.sort();sorted.dedup();
    for id in sorted{
        let mut manifest=catalog.entries.iter().filter_map(|e|e.manifest.as_ref().ok()).find(|m|m.id==id)
            .ok_or_else(||format!("Missing or invalid room mod: {id}"))?.clone();
        manifest.on_new_sandbox=manifest.resolved(&overrides)?;
        packages.push(rrbtd6_coop::ModPackage::new(serde_json::to_string(&manifest).map_err(|e|e.to_string())?)?);
    }
    rrbtd6_coop::mods::validate(&packages)?;
    if !packages.is_empty(){
        let original=std::fs::read_to_string(root.join("game/game.json")).map_err(|_|"Cannot validate room mods without base game data")?;
        room_json(&original,&packages)?;
    }
    Ok(packages)
}
pub fn room_json(json:&str,packages:&[rrbtd6_coop::ModPackage])->Result<Option<String>,String>{
    rrbtd6_coop::mods::validate(packages)?;if packages.is_empty(){return Ok(None);}
    let manifests:Vec<_>=packages.iter().map(|p|Manifest::parse(p.manifest.as_bytes())).collect::<Result<_,_>>()?;
    content::compile(json,&manifests.iter().collect::<Vec<_>>()).map(Some)
}

/// Called only while creating a fresh local Sandbox. Events and co-op never enter it.
pub fn apply_new_sandbox(root:&Path,game:&mut crate::game::Game,local:bool){
    if !local||!game.sandbox{return;}
    let selected=enabled(root);if selected.is_empty(){return;}
    match Catalog::scan(&directory(root)).compile_with(&selected,&overrides(root)){
        Ok(rules)=>{
            rules.apply(local,game.sandbox,&mut game.cash,&mut game.lives);
            eprintln!("mods: {} enabled; new local Sandbox cash={} lives={}",selected.len(),game.cash,game.lives);
        },
        Err(error)=>eprintln!("Mods not applied: {error}"),
    }
}

#[cfg(test)]
mod tests{
    use super::*;
    use serde_json::json;
    fn manifest(id:&str)->serde_json::Value{json!({"api_version":1,"id":id,"name":"Practice","version":"1.0","on_new_sandbox":{"starting_cash":100000}})}
    #[test]
    fn rejects_unknown_api_fields_invalid_ids_and_rules(){
        for key in ["api","id","negative","fractional","too_large","unknown"]{
            let mut m=manifest("practice");
            match key{"api"=>m["api_version"]=json!(2),"id"=>m["id"]=json!("../escape"),"negative"=>m["on_new_sandbox"]["starting_cash"]=json!(-1),
                "fractional"=>m["on_new_sandbox"]["starting_lives"]=json!(1.5),"too_large"=>m["on_new_sandbox"]["starting_cash"]=json!(1e13),_=>m["on_tick"]=json!({})}
            assert!(Manifest::parse(&serde_json::to_vec(&m).unwrap()).is_err(),"{key}");
        }
    }
    #[test]
    fn disabled_mods_do_nothing_conflicts_and_missing_ids_are_reported(){
        let make=|id:&str|Entry{folder:id.into(),manifest:Manifest::parse(&serde_json::to_vec(&manifest(id)).unwrap())};
        let c=Catalog{entries:vec![make("a"),make("b")]};
        assert_eq!(c.compile(&HashSet::new()).unwrap(),SandboxRules::default());
        assert_eq!(c.compile(&HashSet::from(["a".into()])).unwrap().starting_cash,Some(100000.0));
        assert!(c.compile(&HashSet::from(["a".into(),"b".into()])).is_err());
        assert!(c.compile(&HashSet::from(["missing".into()])).is_err());
    }
    #[test]
    fn hook_only_changes_new_local_sandbox_values(){
        let rules=SandboxRules{starting_cash:Some(100000.0),starting_lives:Some(200.0)};
        for (local,sandbox) in [(false,false),(false,true),(true,false),(true,true)]{
            let (mut cash,mut lives)=(650.0,100.0);rules.apply(local,sandbox,&mut cash,&mut lives);
            assert_eq!((cash,lives),if local&&sandbox{(100000.0,200.0)}else{(650.0,100.0)});
        }
        let (mut cash,mut lives)=(650.0,100.0);
        SandboxRules{starting_cash:Some(42.0),starting_lives:None}.apply(true,true,&mut cash,&mut lives);
        assert_eq!((cash,lives),(42.0,100.0));
    }

    fn configurable()->Manifest {
        Manifest::parse(include_bytes!("../../mods/practice-start/mod.json")).unwrap()
    }
    #[test]
    fn mod_settings_use_one_resolver_for_preview_and_runtime_and_reset_to_defaults(){
        let m=configurable();let enabled=HashSet::from([m.id.clone()]);
        let mut saved=Overrides::from([(m.id.clone(),json!({"starting_cash":120000,"starting_lives":321}))]);
        let expected=m.resolved(&saved).unwrap();
        let catalog=Catalog{entries:vec![Entry{folder:"practice".into(),manifest:Ok(m.clone())}]};
        assert_eq!(catalog.compile_with(&enabled,&saved).unwrap(),expected);
        assert_eq!(expected,SandboxRules{starting_cash:Some(120000.0),starting_lives:Some(321.0)});
        let roundtrip:Overrides=serde_json::from_slice(&serde_json::to_vec(&saved).unwrap()).unwrap();
        assert_eq!(m.resolved(&roundtrip).unwrap(),expected);
        saved.remove(&m.id);assert_eq!(m.resolved(&saved).unwrap(),m.on_new_sandbox);
    }
    #[test]
    fn mod_settings_reject_invalid_definitions_and_stale_overrides(){
        for settings in [json!({"starting_cash":{"min":0,"max":1000000,"step":0}}),
            json!({"starting_cash":{"min":0,"max":999999,"step":10000}}),
            json!({"starting_lives":{"min":0,"max":1000,"step":1}}),
            json!({"starting_lives":{"min":1,"max":1000,"step":1}}),
            json!({"damage":{"min":0,"max":10,"step":1}})] {
            let mut v=manifest("practice");v["settings"]=settings;
            assert!(Manifest::parse(&serde_json::to_vec(&v).unwrap()).is_err());
        }
        let m=configurable();
        for saved in [json!({"starting_cash":100001}),json!({"starting_lives":0}),json!({"damage":2}),json!({"starting_cash":"oops"})] {
            let overrides=Overrides::from([(m.id.clone(),saved)]);
            assert!(m.resolved(&overrides).is_err());
            let c=Catalog{entries:vec![Entry{folder:"practice".into(),manifest:Ok(m.clone())}]};
            assert_eq!(c.compile_with(&HashSet::new(),&overrides).unwrap(),SandboxRules::default());
        }
        assert_eq!(m.settings.starting_cash.as_ref().unwrap().adjust(0.0,-1.0),0.0);
        assert_eq!(m.settings.starting_lives.as_ref().unwrap().adjust(1000.0,1.0),1000.0);
    }
    #[test]
    fn duplicate_ids_are_all_rejected(){
        let dir=std::env::temp_dir().join(format!("rusttd6-mod-test-{}",std::process::id()));
        for name in ["one","two"]{let p=dir.join(name);std::fs::create_dir_all(&p).unwrap();std::fs::write(p.join("mod.json"),serde_json::to_vec(&manifest("duplicate")).unwrap()).unwrap();}
        let catalog=Catalog::scan(&dir);assert_eq!(catalog.entries.len(),2);assert!(catalog.entries.iter().all(|e|e.manifest.is_err()));
        for name in ["one","two"]{std::fs::remove_file(dir.join(name).join("mod.json")).unwrap();std::fs::remove_dir(dir.join(name)).unwrap();}std::fs::remove_dir(dir).unwrap();
    }
}
