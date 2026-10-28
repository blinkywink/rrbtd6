//! Versioned, data-only local mods. No work runs on the frame/simulation loop.
use serde::{Deserialize, Serialize};
use std::{collections::HashSet, path::{Path, PathBuf}};

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
}
#[derive(Clone,Debug,Default,Deserialize,Serialize,PartialEq)]
#[serde(deny_unknown_fields)]
pub struct SandboxRules {
    pub starting_cash:Option<f64>,
    pub starting_lives:Option<f64>,
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
    pub fn compile(&self,enabled:&HashSet<String>)->Result<SandboxRules,String>{
        let mut rules=SandboxRules::default();let mut found=HashSet::new();
        for m in self.entries.iter().filter_map(|e|e.manifest.as_ref().ok()).filter(|m|enabled.contains(&m.id)){
            found.insert(m.id.clone());
            for (name,target,value) in [("starting cash",&mut rules.starting_cash,m.on_new_sandbox.starting_cash),
                ("starting lives",&mut rules.starting_lives,m.on_new_sandbox.starting_lives)]{
                if let Some(value)=value{if target.replace(value).is_some(){return Err(format!("More than one enabled mod sets {name}. Disable one to apply mods."));}}
            }
        }
        if enabled.iter().any(|id|!found.contains(id)){return Err("An enabled mod is missing or invalid. Disable it or fix its manifest to apply mods.".into());}
        Ok(rules)
    }
}

/// Called only while creating a fresh local Sandbox. Events and co-op never enter it.
pub fn apply_new_sandbox(root:&Path,game:&mut crate::game::Game,local:bool){
    if !local||!game.sandbox{return;}
    let selected=enabled(root);if selected.is_empty(){return;}
    match Catalog::scan(&directory(root)).compile(&selected){
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
    #[test]
    fn duplicate_ids_are_all_rejected(){
        let dir=std::env::temp_dir().join(format!("rusttd6-mod-test-{}",std::process::id()));
        for name in ["one","two"]{let p=dir.join(name);std::fs::create_dir_all(&p).unwrap();std::fs::write(p.join("mod.json"),serde_json::to_vec(&manifest("duplicate")).unwrap()).unwrap();}
        let catalog=Catalog::scan(&dir);assert_eq!(catalog.entries.len(),2);assert!(catalog.entries.iter().all(|e|e.manifest.is_err()));
        for name in ["one","two"]{std::fs::remove_file(dir.join(name).join("mod.json")).unwrap();std::fs::remove_dir(dir.join(name)).unwrap();}std::fs::remove_dir(dir).unwrap();
    }
}
