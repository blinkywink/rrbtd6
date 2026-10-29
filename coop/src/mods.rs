//! Bounded data-only room mods, transferred directly in the authenticated room snapshot.
//! No filesystem paths, executables or native code cross this boundary.
use serde::{Deserialize,Serialize};
use std::collections::HashSet;
#[derive(Clone,Debug,Serialize,Deserialize,PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ModPackage{pub id:String,pub version:String,pub digest:String,pub manifest:String}
impl ModPackage{
    pub fn new(manifest:String)->Result<Self,String>{
        let value:serde_json::Value=serde_json::from_str(&manifest).map_err(|_|"Invalid mod JSON")?;
        let id=value["id"].as_str().ok_or("Mod has no ID")?.to_owned();
        let version=value["version"].as_str().ok_or("Mod has no version")?.to_owned();
        let package=Self{id,version,digest:blake3::hash(manifest.as_bytes()).to_hex().to_string(),manifest};
        validate(std::slice::from_ref(&package))?;Ok(package)
    }
}
pub fn validate(mods:&[ModPackage])->Result<(),String>{
    if mods.len()>16||mods.iter().map(|m|m.manifest.len()).sum::<usize>()>24*1024{return Err("Room mods exceed the current 16-mod / 24 KiB data limit".into());}
    let mut ids=HashSet::new();
    for m in mods{
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
}
