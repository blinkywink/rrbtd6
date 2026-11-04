//! Bounded room mods: the manifest (content edits and event scripts) travels in the
//! authenticated room snapshot; a mod's custom art (its assets/ files) is listed there by hash and
//! fetched from the host on request (Wire::AssetRequest). Paths are checked segment by segment;
//! no executables or native code cross this boundary.
use serde::{Deserialize,Serialize};
use std::collections::{HashMap,HashSet};
/// One of a mod's art files, at `path` inside the mod's folder (assets/<asset>/<file>).
#[derive(Clone,Debug,Serialize,Deserialize,PartialEq)]
#[serde(deny_unknown_fields)]
pub struct AssetFile{pub path:String,pub size:u64,pub digest:String}
#[derive(Clone,Debug,Serialize,Deserialize,PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ModPackage{pub id:String,pub version:String,pub digest:String,pub manifest:String,#[serde(default)]pub assets:Vec<AssetFile>}
/// The host's art files by hash, served to players who ask.
pub type AssetStore=HashMap<String,std::sync::Arc<Vec<u8>>>;
pub const MAX_ASSET_FILES:usize=128;
pub const MAX_ASSET_BYTES:u64=32*1024*1024;
/// Raw bytes per Wire::AssetChunk (base64 inside the 64 KiB frame).
pub const ASSET_CHUNK:usize=40*1024;
impl ModPackage{
    pub fn new(manifest:String)->Result<Self,String>{Self::with_assets(manifest,Vec::new())}
    pub fn with_assets(manifest:String,assets:Vec<AssetFile>)->Result<Self,String>{
        let value:serde_json::Value=serde_json::from_str(&manifest).map_err(|_|"Invalid mod JSON")?;
        let id=value["id"].as_str().ok_or("Mod has no ID")?.to_owned();
        let version=value["version"].as_str().ok_or("Mod has no version")?.to_owned();
        let package=Self{id,version,digest:blake3::hash(manifest.as_bytes()).to_hex().to_string(),manifest,assets};
        validate(std::slice::from_ref(&package))?;Ok(package)
    }
}
/// assets/<asset>/<file>, each segment plain (no dots-only names, separators or drive letters).
pub fn asset_path_ok(path:&str)->bool{
    let parts:Vec<&str>=path.split('/').collect();
    parts.len()==3&&parts[0]=="assets"&&parts[1..].iter().all(|s|!s.is_empty()&&s.len()<=64&&!s.bytes().all(|b|b==b'.')&&s.bytes().all(|b|b.is_ascii_alphanumeric()||b"._-".contains(&b)))
}
pub fn validate(mods:&[ModPackage])->Result<(),String>{
    if mods.len()>16||mods.iter().map(|m|m.manifest.len()).sum::<usize>()>24*1024{return Err("Room mods exceed the current 16-mod / 24 KiB data limit".into());}
    let files:Vec<&AssetFile>=mods.iter().flat_map(|m|&m.assets).collect();
    if files.len()>MAX_ASSET_FILES||files.iter().map(|f|f.size).sum::<u64>()>MAX_ASSET_BYTES{return Err("Room mod art exceeds 128 files / 32 MiB".into());}
    if files.iter().any(|f|!asset_path_ok(&f.path)||f.digest.len()!=64||!f.digest.bytes().all(|b|b.is_ascii_hexdigit())){return Err("Invalid room mod art file".into());}
    let mut ids=HashSet::new();
    for m in mods{
        let mut paths=HashSet::new();
        if m.assets.iter().any(|f|!paths.insert(&f.path)){return Err("Duplicate room mod art file".into());}
        if m.id.is_empty()||m.id.len()>64||!m.id.bytes().all(|b|b.is_ascii_lowercase()||b.is_ascii_digit()||b"._-".contains(&b))
            ||m.version.is_empty()||m.version.len()>32||!ids.insert(&m.id){return Err("Invalid or duplicate room mod".into());}
        if blake3::hash(m.manifest.as_bytes()).to_hex().as_str()!=m.digest{return Err("Room mod hash mismatch".into());}
        let v:serde_json::Value=serde_json::from_str(&m.manifest).map_err(|_|"Invalid room mod JSON")?;
        if v["id"]!=m.id||v["version"]!=m.version||v["api_version"]!=1{return Err("Room mod metadata mismatch or unsupported API".into());}
    }
    if serde_json::to_vec(mods).map_err(|e|e.to_string())?.len()>40*1024{return Err("Encoded room mod set is too large".into());}
    Ok(())
}
pub fn mode_ids(mods:&[ModPackage])->Vec<String>{
    let mut ids=Vec::new();
    for package in mods{if let Ok(v)=serde_json::from_str::<serde_json::Value>(&package.manifest){
        for op in v["content"].as_array().into_iter().flatten(){if op["op"]=="mode"{if let Some(id)=op["id"].as_str(){ids.push(id.to_owned());}}}
    }}
    ids.sort();ids.dedup();ids
}
#[cfg(test)]
mod tests{
    use super::*;
    fn package()->ModPackage{ModPackage::new(r#"{"api_version":1,"id":"example","name":"Example","version":"1"}"#.into()).unwrap()}
    #[test]
    fn packages_are_hashed_bounded_and_unique(){
        let p=package();validate(std::slice::from_ref(&p)).unwrap();
        assert!(validate(&[p.clone(),p.clone()]).is_err());
        let mut bad=p.clone();bad.manifest.push(' ');assert!(validate(&[bad]).is_err());
        let mut bad=p;bad.id="../../file".into();assert!(validate(&[bad]).is_err());
        assert!(ModPackage::new("x".repeat(25*1024)).is_err());
    }
    #[test]
    fn art_paths_stay_inside_the_mods_assets(){
        for ok in ["assets/robot/meta.json","assets/robot/mesh_0.bin"]{assert!(asset_path_ok(ok));}
        for bad in ["robot/meta.json","assets/../x/y","assets/robot/..","assets/robot","assets/a/b/c","assets/C:/x","assets//x","assets/a\\b/c"]{assert!(!asset_path_ok(bad),"{bad}");}
        let m=r#"{"api_version":1,"id":"example","name":"Example","version":"1"}"#.to_string();
        let file=|p:&str|AssetFile{path:p.into(),size:3,digest:"a".repeat(64)};
        assert!(ModPackage::with_assets(m.clone(),vec![file("assets/robot/meta.json")]).is_ok());
        assert!(ModPackage::with_assets(m.clone(),vec![file("assets/robot/meta.json"),file("assets/robot/meta.json")]).is_err());
        assert!(ModPackage::with_assets(m,vec![file("../evil/x")]).is_err());
    }
}
