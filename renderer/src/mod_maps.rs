//! Custom map registration and geometry validation. No catalogue scans during gameplay.
use std::{collections::BTreeMap,path::{Path,PathBuf}};
use serde::{Deserialize,Serialize};
use serde_json::Value;

#[derive(Clone,Debug,Deserialize,Serialize)]
#[serde(deny_unknown_fields)]
pub struct Definition {
    pub name:String,
    pub difficulty:usize,
    pub model:String,
    pub data:Value,
}
impl Definition {
    pub fn validate(&self)->Result<(),String>{
        if self.name.trim().is_empty()||self.name.chars().count()>80||self.name.chars().any(char::is_control)||self.difficulty>3||crate::mod_assets::segments(&self.model).is_none(){return Err("Map needs a name, difficulty 0–3 and a mod-owned model".into());}
        let map:crate::data::MapData=serde_json::from_value(self.data.clone()).map_err(|e|format!("Invalid map geometry: {e}"))?;
        let point=|p:&[f32;2]|p.iter().all(|v|v.is_finite()&&v.abs()<=1000.);
        fn ring(p:&[[f32;2]])->bool{
            if !(3..=512).contains(&p.len())||p.iter().flatten().any(|v|!v.is_finite()||v.abs()>1000.){return false;}
            let area:f32=p.iter().zip(p.iter().cycle().skip(1)).take(p.len()).map(|(a,b)|a[0]*b[1]-b[0]*a[1]).sum();
            area.abs()>0.001
        }
        let path=|p:&[[f32;2]]| (2..=2048).contains(&p.len())&&p.iter().all(point)&&p.windows(2).any(|p|p[0]!=p[1]);
        if !path(&map.path)||map.paths.len()>32||map.areas.is_empty()||map.areas.len()>128{return Err("Map needs a nonzero path and 1–128 placement areas; at most 32 routes".into());}
        let bounds=map.terrain.as_ref().ok_or("Custom maps need terrain min/max bounds")?;
        if !point(&bounds.min)||!point(&bounds.max)||bounds.min[0]>=bounds.max[0]||bounds.min[1]>=bounds.max[1]
            ||bounds.min[0]< -150.||bounds.max[0]>150.||bounds.min[1]< -116.||bounds.max[1]>116.{return Err("Custom map bounds must fit the current 300 × 232 game arena".into());}
        let mut ids=std::collections::HashSet::new();
        for p in &map.paths{
            if !ids.insert(p.id)||!path(&p.points)||(!p.dists.is_empty()&&(p.dists.len()!=p.points.len()||p.dists.iter().any(|v|!v.is_finite()||*v<0.)||p.dists.windows(2).any(|v|v[0]>v[1])))
                ||(!p.speeds.is_empty()&&(p.speeds.len()!=p.points.len()||p.speeds.iter().any(|v|!v.is_finite()||*v<0.)))
                ||!p.quick.is_empty()||!p.quick_rot.is_empty()||(!p.no_target.is_empty()&&p.no_target.len()!=p.points.len()){
                return Err("Invalid custom path IDs, point data or cache fields; supply control points, not native quick caches".into());
            }
        }
        if !map.paths.is_empty()&&(!map.paths.iter().any(|p|p.active)||map.paths.iter().any(|p|p.entry.iter().chain(&p.exit).any(|id|!ids.contains(id)))){return Err("Map routes need an active entry and valid splitter references".into());}
        let mut vertices=0usize;
        for a in &map.areas{
            vertices+=a.points.len()+a.holes.iter().map(Vec::len).sum::<usize>();
            if !(0..=5).contains(&a.kind)||!ring(&a.points)||a.holes.len()>32||a.holes.iter().any(|h|!ring(h))||!a.height.is_finite()||a.height.abs()>1000.
                ||a.blockers.len()>256||a.blockers.iter().any(|b|b.iter().any(|v|!v.is_finite())||b[2]<0.){return Err("Invalid placement polygon, hole, area type or blocker".into());}
        }
        if vertices>8192{return Err("Too many placement polygon vertices".into());}
        Ok(())
    }
}
pub(crate) fn catalog(root:&Path)->BTreeMap<String,Definition>{
    let enabled=crate::modding::enabled(root);
    let catalog=crate::modding::Catalog::scan(&crate::modding::directory(root));
    let mut result=BTreeMap::new();
    for manifest in catalog.entries.iter().filter_map(|e|e.manifest.as_ref().ok()).filter(|m|enabled.contains(&m.id)){
        for edit in &manifest.content{if let crate::modding::content::Edit::Map{id,definition}=edit{
            if !id.starts_with(&format!("{}-",manifest.id)){continue;}
            let dir=crate::mod_assets::model(root,&definition.model);
            if dir.join("meta.json").is_file()&&dir.join("preview.png").is_file(){result.insert(id.clone(),definition.clone());}
            else{eprintln!("{}: map {id} needs its baked model and preview.png",manifest.name);}
        }}
    }
    result
}
pub(crate) fn model_dir(root:&Path,id:&str)->PathBuf{
    catalog(root).get(id).map(|d|crate::mod_assets::model(root,&d.model)).unwrap_or_else(||root.join("models").join(format!("map_{id}")))
}
pub(crate) fn terrain_file(root:&Path,id:&str)->PathBuf{
    catalog(root).get(id).map(|d|crate::mod_assets::model(root,&d.model).join("preview.png")).unwrap_or_else(||root.join("maps").join(id).join("terrain.png"))
}

#[cfg(test)] mod tests {
    use super::*;
    fn example()->Definition{Definition{name:"Example".into(),difficulty:0,model:"mod:example/arena".into(),data:serde_json::json!({"path":[[-150,0],[150,0]],"areas":[{"type":2,"points":[[-150,-116],[150,-116],[150,116],[-150,116]]}],"terrain":{"min":[-150,-116],"max":[150,116]}})}}
    #[test]fn rejects_invalid_paths_polygons_bounds_and_asset_names(){
        assert!(example().validate().is_ok());
        for (pointer,value) in [("/path",serde_json::json!([[1,1],[1,1]])),("/areas/0/points",serde_json::json!([[0,0],[1,1],[2,2]])),("/terrain/max",serde_json::json!([151,116]))]{
            let mut d=example();*d.data.pointer_mut(pointer).unwrap()=value;assert!(d.validate().is_err());
        }
        let mut d=example();d.model="mod:../escape".into();assert!(d.validate().is_err());
    }
}
