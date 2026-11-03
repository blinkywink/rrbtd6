//! Load-time content API. Authors write manifests; gameplay uses native compiled models.
use serde::{Deserialize,Serialize};
use serde_json::{Value,json};
use std::collections::HashSet;

#[derive(Clone,Debug,Deserialize,Serialize)]
#[serde(tag="op",rename_all="snake_case",deny_unknown_fields)]
pub enum Edit {
    Script {id:String,source:String,#[serde(default="crate::mod_runtime::interval")] tick_interval:u32},
    Shop {tower:String,#[serde(default)] before:Option<String>,#[serde(default)] after:Option<String>,#[serde(default)] index:Option<usize>,#[serde(default)] visible:Option<bool>,#[serde(default)] category:Option<u32>},
    /// Clone a complete tower including tiers; model/portrait references stay shared.
    Tower {id:String,base:String,#[serde(default)] patch:Value},
    TowerModel {tower:String,asset:String,muzzle:[f32;3],release:f32},
    UpgradePatch {tower:String,path:usize,tier:u8,#[serde(default)] cost:Option<f64>,#[serde(default)] stats:Value,#[serde(default)] weapon:Value,#[serde(default)] projectile:Value,#[serde(default)] projectile_from:Option<String>,#[serde(default)] model:Option<String>,#[serde(default)] muzzle:Option<[f32;3]>,#[serde(default)] release:Option<f32>},
    Map {id:String,definition:crate::mod_maps::Definition},
    Bloon {id:String,base:String,#[serde(default)] patch:Value},
    Mode {id:String,base:String,#[serde(default)] patch:Value},
    /// Strict JSON-pointer replacement. Includes weapon/projectile/upgrade/round data.
    Set {path:String,value:Value},
    Copy {from:String,path:String},
    Insert {path:String,index:usize,value:Value},
    Remove {path:String,index:usize},
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
            Self::Script{id,source,tick_interval}=>crate::mod_runtime::Definition{id:id.clone(),source:source.clone(),tick_interval:*tick_interval}.validate()?,
            Self::Shop{tower,before,after,index,category,..}=>{
                if !identifier(tower)||before.iter().chain(after).any(|v|!identifier(v))
                    ||usize::from(before.is_some())+usize::from(after.is_some())+usize::from(index.is_some())>1
                    ||index.is_some_and(|v|v>512)||category.is_some_and(|v|!matches!(v,1|2|4|8)) {
                    return Err("Invalid shop position/category; choose before, after, or index".into());
                }
            },
            Self::Map{id,definition}=>{if !identifier(id){return Err("Invalid custom map ID".into());}definition.validate()?;},
            Self::UpgradePatch{tower,path,tier,cost,stats,weapon,projectile,projectile_from,model,muzzle,release}=>{
                if let Some(from)=projectile_from{Self::Set{path:from.clone(),value:Value::Null}.validate()?;}
                if !identifier(tower)||*path>=3||!(1..=5).contains(tier){return Err("Upgrade edits currently support paths 0–2 and tiers 1–5".into());}
                if cost.is_some_and(|c|!c.is_finite()||c<0.0)||[stats,weapon,projectile].iter().any(|p|!p.is_null()&&!p.is_object())
                    ||model.as_ref().is_some_and(|s|crate::mod_assets::segments(s).is_none())
                    ||muzzle.is_some_and(|m|m.iter().any(|v|!v.is_finite()))||release.is_some_and(|v|!v.is_finite()||!(0.0..=5.0).contains(&v)){
                    return Err("Invalid upgrade fields, model or socket".into());
                }
            },
            Self::TowerModel{tower,asset,muzzle,release}=>{
                if !identifier(tower)||crate::mod_assets::segments(asset).is_none()||muzzle.iter().any(|v|!v.is_finite())||!release.is_finite()||!(0.0..=5.0).contains(release){return Err("Invalid custom model or release socket".into());}
            },
            Self::Tower{id,base,patch}|Self::Bloon{id,base,patch}|Self::Mode{id,base,patch}=>{
                if !identifier(id)||base.is_empty()||base.len()>96{return Err("Content needs a short unique ID and a base ID".into());}
                if !patch.is_null()&&!patch.is_object(){return Err("Content patch must be an object".into());}
            },
            Self::Insert{path,..}|Self::Remove{path,..}=>{Self::Set{path:path.clone(),value:Value::Null}.validate()?;},
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
            Self::Script{id,source,tick_interval}=>{
                if game.get("mod_scripts").is_none(){game["mod_scripts"]=json!([]);}
                game["mod_scripts"].as_array_mut().ok_or("Invalid scripts list")?.push(json!({"id":id,"source":source,"tick_interval":tick_interval}));
            },
            Self::Shop{tower,before,after,index,visible,category}=>{
                let list=game["shop"].as_array_mut().ok_or("Missing shop")?;
                let at=list.iter().position(|v|v["id"]==*tower).ok_or("Shop tower does not exist")?;
                let mut item=list.remove(at);
                if let Some(category)=category{item["set"]=json!(category);}
                let target=if let Some(before)=before{list.iter().position(|v|v["id"]==*before).ok_or("Unknown shop anchor")?}
                    else if let Some(after)=after{list.iter().position(|v|v["id"]==*after).ok_or("Unknown shop anchor")?+1}
                    else{index.unwrap_or(at).min(list.len())};
                if *visible!=Some(false){list.insert(target,item);}
                if let Some(category)=category{
                    if let Some(states)=game["tower_tiers"][tower]["tiers"].as_object_mut(){for state in states.values_mut(){state["set"]=json!(category);}}
                }
            },
            Self::Map{id,definition}=>{
                if !game["mod_maps"][id].is_null(){return Err(format!("Duplicate map ID {id}"));}
                game["mod_maps"][id]=serde_json::to_value(definition).map_err(|e|e.to_string())?;
            },
            Self::UpgradePatch{tower,path,tier,cost,stats,weapon,projectile,projectile_from,model,muzzle,release}=>{
                let copied=projectile_from.as_ref().map(|from|game.pointer(from).cloned().ok_or("Unknown projectile template")).transpose()?;
                let definitions=game["tower_tiers"].get_mut(tower).ok_or("Unknown upgrade tower")?;
                let upgrade=definitions["upgrades"].as_array_mut().ok_or("Tower has no upgrades")?.iter_mut()
                    .find(|u|u["path"].as_u64()==Some(*path as u64)&&u["tier"].as_u64()==Some((*tier-1) as u64)).ok_or("Upgrade does not exist")?;
                if let Some(cost)=cost{upgrade["cost"]=json!(cost);}
                let mut visual_keys=Vec::new();
                for (key,state) in definitions["tiers"].as_object_mut().ok_or("Tower has no tier states")?{
                    if key.len()!=3||!key.bytes().all(|b|(b'0'..=b'5').contains(&b))||key.as_bytes()[*path]-b'0'<*tier{continue;}
                    merge(state,stats)?;
                    if !weapon.is_null()||!projectile.is_null()||copied.is_some()||muzzle.is_some()||release.is_some(){
                        let primary=state.pointer_mut("/attacks/0/weapons/0").ok_or("Upgrade state has no primary weapon")?;
                        merge(primary,weapon)?;
                        if let Some(template)=&copied{primary["projectile"]=template.clone();}
                        if !projectile.is_null(){
                            merge(&mut primary["projectile"],projectile)?;
                            // An explicit display replaces the inherited damage-stage graphics.
                            if projectile.get("display").is_some() && projectile.get("pierce_sprites").is_none(){
                                if let Some(sprites)=primary["projectile"].get_mut("pierce_sprites"){*sprites=json!([]);}
                            }
                        }
                        if let Some(m)=muzzle{primary["eject"]=json!([-m[0],m[2],m[1]]);}
                        if let Some(release)=release{primary["animation_offset"]=json!(release);}
                    }
                    if model.is_some(){visual_keys.push(key.clone());}
                }
                for key in visual_keys{definitions["visuals"][key]=json!(model);}
            },
            Self::TowerModel{tower,asset,muzzle,release}=>{
                let definition=game["towers"].get_mut(tower).ok_or("Unknown custom model tower")?;
                definition["model"]=json!(asset);
                for tier in game["tower_tiers"][tower]["tiers"].as_object_mut().ok_or("Tower has no tiers")?.values_mut(){
                    // The starter replaces the primary weapon; secondary weapons retain their sockets.
                    if let Some(weapon)=tier.pointer_mut("/attacks/0/weapons/0"){
                        weapon["eject"]=json!([-muzzle[0],muzzle[2],muzzle[1]]);weapon["animation_offset"]=json!(release);
                    }
                }
            },
            Self::Insert{path,index,value}=>{
                let list=game.pointer_mut(path).and_then(Value::as_array_mut).ok_or("Insert needs an existing array")?;
                if *index>list.len()||list.len()>=4096{return Err("Insert index/array limit exceeded".into());}
                list.insert(*index,value.clone());
            },
            Self::Remove{path,index}=>{
                let list=game.pointer_mut(path).and_then(Value::as_array_mut).ok_or("Remove needs an existing array")?;
                if *index>=list.len(){return Err("Remove index out of range".into());}
                list.remove(*index);
            },
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
            if let Edit::Script{id,..}=edit{
                let key=format!("script:{}:{id}",manifest.id);
                if !writes.insert(key){return Err("Duplicate script ID".into());}
                let last=game["mod_scripts"].as_array_mut().unwrap().last_mut().unwrap();
                last["id"]=json!(format!("{}:{id}",manifest.id));
            }
        }
    }
    if let (Some(towers),Some(shop))=(game["towers"].as_object().cloned(),game["shop"].as_array_mut()){
        for item in shop{if let Some(tower)=item["id"].as_str().and_then(|id|towers.get(id)){
            item["cost"]=tower["cost"].clone();item["icon"]=tower["icon"].clone();
        }}
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
            ||tower.model.as_ref().is_some_and(|m|!identifier(m)&&crate::mod_assets::segments(m).is_none()){return Err(format!("Invalid tower {id}"));}
    }
    for tiers in data.tower_tiers.values(){
        if tiers.visuals.iter().any(|(key,model)|key.len()!=3||!key.bytes().all(|b|(b'0'..=b'5').contains(&b))||crate::mod_assets::segments(model).is_none()) {return Err("Invalid per-tier model reference".into());}
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
    fn shop_moves_and_array_edits_are_strict(){
        let mut data=json!({"shop":[{"id":"A","set":1},{"id":"B","set":1},{"id":"C","set":1}],
            "tower_tiers":{"A":{"tiers":{"000":{"set":1}}}},"rounds":[1,2]});
        Edit::Shop{tower:"A".into(),before:None,after:Some("B".into()),index:None,visible:None,category:Some(2)}.apply(&mut data).unwrap();
        assert_eq!(data["shop"][1]["id"],"A");
        assert_eq!(data["shop"][1]["set"],2);
        assert_eq!(data["tower_tiers"]["A"]["tiers"]["000"]["set"],2);
        Edit::Insert{path:"/rounds".into(),index:1,value:json!(5)}.apply(&mut data).unwrap();
        assert_eq!(data["rounds"],json!([1,5,2]));
        Edit::Remove{path:"/rounds".into(),index:0}.apply(&mut data).unwrap();
        assert_eq!(data["rounds"],json!([5,2]));
        assert!(Edit::Remove{path:"/rounds".into(),index:20}.apply(&mut data).is_err());
        assert!(Edit::Shop{tower:"A".into(),before:Some("B".into()),after:Some("C".into()),index:None,visible:None,category:None}.validate().is_err());
    }
    #[test]
    fn model_socket_is_compiled_into_all_primary_upgrade_weapons(){
        let mut game=json!({"towers":{"robot":{}},"tower_tiers":{"robot":{"tiers":{
            "000":{"attacks":[{"weapons":[{"eject":[0,0,0],"animation_offset":0}]}]},
            "100":{"attacks":[{"weapons":[{"eject":[0,0,0],"animation_offset":0}]}]}
        }}}});
        Edit::TowerModel{tower:"robot".into(),asset:"mod:example/robot".into(),muzzle:[4.,8.,6.],release:0.15}.apply(&mut game).unwrap();
        for tier in ["000","100"]{
            assert_eq!(game["tower_tiers"]["robot"]["tiers"][tier]["attacks"][0]["weapons"][0]["eject"],json!([-4.,6.,8.]));
        }
        assert_eq!(game["towers"]["robot"]["model"],"mod:example/robot");
        assert!(Edit::TowerModel{tower:"robot".into(),asset:"mod:../escape".into(),muzzle:[0.;3],release:0.15}.validate().is_err());
    }
    #[test]
    fn upgrade_edits_apply_to_later_and_crosspath_states_with_model_swaps(){
        let state=json!({"range":32,"attacks":[{"weapons":[{"rate":1,"eject":[0,0,0],"animation_offset":0,"projectile":{"damage":1,"display":"original","pierce_sprites":["old-stage"]}}]}]});
        let mut game=json!({"tower_tiers":{"robot":{"upgrades":[{"path":0,"tier":0,"cost":100}],"tiers":{}}}});
        for key in ["000","100","200","102","010"]{game["tower_tiers"]["robot"]["tiers"][key]=state.clone();}
        Edit::UpgradePatch{tower:"robot".into(),path:0,tier:1,cost:Some(150.),stats:json!({"range":40}),weapon:json!({"rate":0.5}),projectile:json!({"damage":5,"display":"mod:example/bolt"}),projectile_from:None,model:Some("mod:example/robot2".into()),muzzle:Some([4.,8.,6.]),release:Some(0.15)}.apply(&mut game).unwrap();
        assert_eq!(game["tower_tiers"]["robot"]["upgrades"][0]["cost"],150.);
        for key in ["100","200","102"]{
            let tier=&game["tower_tiers"]["robot"]["tiers"][key];
            assert_eq!(tier["range"],40);assert_eq!(tier["attacks"][0]["weapons"][0]["projectile"]["damage"],5);
            assert_eq!(tier["attacks"][0]["weapons"][0]["projectile"]["pierce_sprites"],json!([]));
            assert_eq!(game["tower_tiers"]["robot"]["visuals"][key],"mod:example/robot2");
        }
        for key in ["000","010"]{assert_eq!(game["tower_tiers"]["robot"]["tiers"][key],state);}
    }
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
