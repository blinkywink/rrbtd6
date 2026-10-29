//! Load-time content API. Authors write manifests; gameplay uses native compiled models.
use serde::{Deserialize,Serialize};
use serde_json::{Value,json};
use std::collections::HashSet;

#[derive(Clone,Debug,Deserialize,Serialize)]
#[serde(tag="op",rename_all="snake_case",deny_unknown_fields)]
pub enum Edit {
    /// Clone a complete tower including tiers; model/portrait references stay shared.
    Tower {id:String,base:String,#[serde(default)] patch:Value},
    Bloon {id:String,base:String,#[serde(default)] patch:Value},
    Mode {id:String,base:String,#[serde(default)] patch:Value},
    /// Strict JSON-pointer replacement. Includes weapon/projectile/upgrade/round data.
    Set {path:String,value:Value},
    Copy {from:String,path:String},
    AllModes {patch:Value},
    Menu {page:String,path:String,#[serde(default)] text:Option<String>,#[serde(default)] visible:Option<bool>},
}
fn identifier(id:&str)->bool {
    !id.is_empty()&&id.len()<=96&&id.as_bytes()[0].is_ascii_alphanumeric()&&id.bytes().all(|b|b.is_ascii_alphanumeric()||b"._-".contains(&b))
}
fn merge(target:&mut Value,patch:&Value)->Result<(),String>{
    let Some(fields)=patch.as_object() else {if patch.is_null(){return Ok(());}return Err("patch must be an object".into());};
    for (key,value) in fields {
        let slot=target.get_mut(key).ok_or_else(||format!("Unknown field {key}"))?;
        if value.is_object()&&slot.is_object(){merge(slot,value)?;}else{*slot=value.clone();}
    }
    Ok(())
}
impl Edit {
    pub fn validate(&self)->Result<(),String>{
        match self {
            Self::Tower{id,base,patch}|Self::Bloon{id,base,patch}|Self::Mode{id,base,patch}=>{
                if !identifier(id)||base.is_empty()||base.len()>96{return Err("Content needs a short unique ID and a base ID".into());}
                if !patch.is_null()&&!patch.is_object(){return Err("Content patch must be an object".into());}
            },
            Self::Copy{from,path}=>{
                Self::Set{path:from.clone(),value:Value::Null}.validate()?;
                Self::Set{path:path.clone(),value:Value::Null}.validate()?;
            },
            Self::Set{path,..}=>{
                let root=path.split('/').nth(1).unwrap_or("");
                if path.len()>512||!path.starts_with('/')||!matches!(root,"towers"|"tower_tiers"|"bloons"|"modes"|"start"|"rounds"|"round_sets"|"freeplay"){
                    return Err("Set path must address game content (towers, tower_tiers, bloons, modes, start or rounds)".into());
                }
            },
            Self::Menu{page,path,text,..}=>{if !identifier(page)||path.is_empty()||path.len()>512||text.as_ref().is_some_and(|t|t.len()>1024){return Err("Invalid menu edit".into());}},
            Self::AllModes{patch}=>{if !patch.is_object(){return Err("all_modes needs an object patch".into());}},
        }
        Ok(())
    }
    fn apply(&self,game:&mut Value)->Result<(),String>{
        self.validate()?;
        match self {
            Self::Menu{..}=>{},
            Self::Copy{from,path}=>{
                let value=game.pointer(from).ok_or_else(||format!("Copy source does not exist: {from}"))?.clone();
                *game.pointer_mut(path).ok_or_else(||format!("Copy target does not exist: {path}"))?=value;
            },
            Self::Set{path,value}=>{
                let target=game.pointer_mut(path).ok_or_else(||format!("Content path does not exist: {path}"))?;
                *target=value.clone();
            },
            Self::AllModes{patch}=>{for mode in game["modes"].as_object_mut().ok_or("No modes found")?.values_mut(){merge(mode,patch)?;}},
            Self::Tower{id,base,patch}=>{
                if !game["towers"][id].is_null(){return Err(format!("Tower {id} already exists"));}
                let mut tower=game["towers"].get(base).ok_or_else(||format!("Unknown base tower {base}"))?.clone();
                merge(&mut tower,patch)?;
                let model=tower["model"].as_str().unwrap_or(base).to_owned();tower["model"]=json!(model);
                let tiers=game["tower_tiers"].get(base).ok_or("Base tower has no tiers")?.clone();
                let mut shop=game["shop"].as_array().and_then(|s|s.iter().find(|s|s["id"]==*base)).ok_or("Base tower is not in the shop")?.clone();
                if shop["hero"]==true{return Err("Use a non-hero base for custom towers".into());}
                shop["id"]=json!(id);shop["cost"]=tower["cost"].clone();shop["icon"]=tower["icon"].clone();
                game["towers"][id]=tower;game["tower_tiers"][id]=tiers;
                game["shop"].as_array_mut().unwrap().push(shop);
            },
            Self::Bloon{id,base,patch}|Self::Mode{id,base,patch}=>{
                let group=if matches!(self,Self::Bloon{..}){"bloons"}else{"modes"};
                if !game[group][id].is_null(){return Err(format!("{group} ID {id} already exists"));}
                let mut item=game[group].get(base).ok_or_else(||format!("Unknown {group} base {base}"))?.clone();
                merge(&mut item,patch)?;if group=="bloons"{item["modded"]=json!(true);}
                game[group][id]=item;
            },
        }
        Ok(())
    }
}
/// Transactional: the caller keeps the original JSON on *any* validation failure.
pub fn compile(json:&str,mods:&[&super::Manifest])->Result<String,String>{
    let mut game:Value=serde_json::from_str(json).map_err(|e|e.to_string())?;
    let mut writes=HashSet::new();
    for manifest in mods {
        for edit in &manifest.content {
            let key=match edit{Edit::Set{path,..}|Edit::Copy{path,..}=>Some(path.clone()),_=>None};
            if let Some(key)=key{if !writes.insert(key.clone()){return Err(format!("Conflicting content edits at {key}"));}}
            edit.apply(&mut game).map_err(|e|format!("{}: {e}",manifest.name))?;
        }
    }
    // Deserialize before returning so field types and required values cannot poison a game.
    let data:crate::data::GameData=serde_json::from_value(game.clone()).map_err(|e|format!("Invalid game content: {e}"))?;
    if data.bloons.len()>4096||data.towers.len()>512||data.modes.len()>256{return Err("Content exceeds the current definition limits".into());}
    for (id,bloon) in &data.bloons {
        if bloon.max_health<1||!bloon.speed.is_finite()||bloon.speed<0.0||!bloon.radius.is_finite()||bloon.radius<0.0
            ||bloon.children.len()>256||bloon.children.iter().any(|c|!data.bloons.contains_key(c)){return Err(format!("Invalid bloon {id}"));}
    }
    // A cycle in the child tree would recurse forever during RBE construction.
    fn visit<'a>(id:&'a str,d:&'a crate::data::GameData,visiting:&mut HashSet<&'a str>,done:&mut HashSet<&'a str>)->Result<(),String>{
        if done.contains(id){return Ok(());}if !visiting.insert(id){return Err(format!("Bloon child cycle at {id}"));}
        for child in &d.bloons[id].children{visit(child,d,visiting,done)?;}
        visiting.remove(id);done.insert(id);Ok(())
    }
    let mut done=HashSet::new();for id in data.bloons.keys(){visit(id,&data,&mut HashSet::new(),&mut done)?;}
    for (id,tower) in &data.towers {
        if !tower.cost.is_finite()||tower.cost<0.0||!tower.range.is_finite()||tower.range<0.0||!tower.radius.is_finite()||tower.radius<0.0
            ||tower.model.as_ref().is_some_and(|m|!identifier(m)){return Err(format!("Invalid tower {id}"));}
    }
    for tiers in data.tower_tiers.values(){for tier in tiers.tiers.values(){for attack in &tier.attacks{for weapon in &attack.weapons{
        if !weapon.rate.is_finite()||weapon.rate<0.0{return Err("Weapon rate cannot be negative".into());}
    }}}}
    serde_json::to_string(&game).map_err(|e|e.to_string())
}

#[cfg(test)]
mod tests{
    use super::*;
    #[test]
    fn patches_reject_typos_and_unsupported_roots(){
        let mut target=json!({"cash":650,"nested":{"speed":1}});
        assert!(merge(&mut target,&json!({"cahs":1})).is_err());
        merge(&mut target,&json!({"nested":{"speed":2}})).unwrap();assert_eq!(target["nested"]["speed"],2);
        assert!(Edit::Set{path:"/account/password".into(),value:json!(1)}.validate().is_err());
    }
    #[test]
    fn clones_share_models_and_add_shop_entries(){
        let mut game=json!({"towers":{"DartMonkey":{"cost":200,"range":32,"icon":"Dart"}},
            "tower_tiers":{"DartMonkey":{"tiers":{"000":{"range":32}}}},"shop":[{"id":"DartMonkey","cost":200,"hero":false,"icon":"Dart"}]});
        Edit::Tower{id:"example-dart".into(),base:"DartMonkey".into(),patch:json!({"cost":100})}.apply(&mut game).unwrap();
        assert_eq!(game["towers"]["example-dart"]["model"],"DartMonkey");
        assert_eq!(game["shop"][1]["id"],"example-dart");assert_eq!(game["shop"][1]["cost"],100);
        assert!(Edit::Tower{id:"example-dart".into(),base:"DartMonkey".into(),patch:Value::Null}.apply(&mut game).is_err());
    }
    #[test]
    #[ignore="requires locally extracted game data"]
    fn authored_content_compiles_and_runs_with_the_native_simulation(){
        let path=std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../local_extracted/game/game.json");
        let original=std::fs::read_to_string(path).unwrap();
        let manifest=super::super::Manifest::parse(&serde_json::to_vec(&json!({"api_version":1,"id":"example","name":"Example","version":"1",
            "content":[{"op":"tower","id":"example-dart","base":"DartMonkey","patch":{"cost":100}},
            {"op":"set","path":"/tower_tiers/example-dart/tiers/000/attacks/0/weapons/0/projectile/damage","value":3},
            {"op":"bloon","id":"example-red","base":"Red","patch":{"maxHealth":5}},
            {"op":"mode","id":"example-mode","base":"Medium","patch":{"cash":1234}}]})).unwrap()).unwrap();
        let compiled=compile(&original,&[&manifest]).unwrap();
        let mut data:crate::data::GameData=serde_json::from_str(&compiled).unwrap();data.mode="example-mode".into();
        assert_eq!(data.tower_tiers["example-dart"].tiers["000"].attacks[0].weapons[0].projectile.damage,3.0);
        let mut game=crate::game::Game::new(data);assert_eq!(game.cash,1234.0);
        assert!(game.shop.iter().any(|s|s.id=="example-dart"));game.debug_spawn("example-red",1,0.0);
        assert_eq!(game.bloons[0].hp,5);
        let def=game.tower_defs.iter().position(|(id,_)|id=="example-dart").unwrap();assert_eq!(game.tower_model(def),"DartMonkey");
        for _ in 0..10{game.step();}
        let mut cycle=manifest.clone();cycle.content=vec![Edit::Set{path:"/bloons/Red/children".into(),value:json!(["Red"])}];
        assert!(compile(&original,&[&cycle]).unwrap_err().contains("cycle"));
        assert_eq!(serde_json::from_str::<Value>(&original).unwrap()["towers"]["example-dart"],Value::Null);
    }

}
