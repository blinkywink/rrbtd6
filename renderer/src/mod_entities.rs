//! Bounded snapshots and edits by stable ID. No live references escape into scripts.
use serde::{Deserialize,Serialize};
use serde_json::{Value,json};
use crate::game::{Game,TargetMode};
#[derive(Clone,Copy,Debug,Deserialize,Serialize,PartialEq)]
#[serde(rename_all="snake_case")]
pub enum Kind {Towers,Bloons,Projectiles}
#[derive(Clone,Debug,Deserialize,Serialize)]
#[serde(deny_unknown_fields)]
pub struct Query {
    pub kind:Kind, #[serde(default)] pub offset:usize,
    pub limit:usize, #[serde(default)] pub tag:String,
}
impl Query {
    pub fn validate(&self)->Result<(),String>{
        if self.offset>1_000_000||!(1..=128).contains(&self.limit)||self.tag.len()>80{return Err("Invalid entity query page/tag".into());}Ok(())
    }
    pub fn snapshot(&self,g:&Game)->Value{
        let total=match self.kind{Kind::Towers=>g.towers.len(),Kind::Bloons=>g.bloons.len(),Kind::Projectiles=>g.projectiles.len()};
        let items:Vec<_>=(self.offset.min(total)..self.offset.saturating_add(self.limit).min(total)).map(|i|match self.kind{
            Kind::Towers=>tower(g,i),Kind::Bloons=>bloon(g,i),Kind::Projectiles=>projectile(g,i),
        }).collect();
        json!({"kind":self.kind,"tag":self.tag,"offset":self.offset,"total":total,"items":items})
    }
}
pub fn tower(g:&Game,i:usize)->Value{
    let t=&g.towers[i];
    json!({"id":t.id,"tower":g.tower_defs[t.def].0,"position":t.pos,"tiers":t.tiers,
        "pops":t.pops,"damage":t.damage,"target":t.target.label(),"point":t.point})
}
pub fn bloon(g:&Game,i:usize)->Value{
    let b=&g.bloons[i];
    json!({"id":b.id,"layer":b.layer,"bloon":g.mod_bloon_name(b.kind),"hp":b.hp,"position":b.pos,"distance":b.dist,"path":b.path})
}
pub fn projectile(g:&Game,i:usize)->Value{
    let p=&g.projectiles[i];
    json!({"id":p.uid,"owner":p.owner,"position":p.pos,"velocity":p.vel,
        "damage":p.spec.damage,"pierce":p.pierce,"remaining":p.frames_left/60.0})
}
#[derive(Clone,Debug,Default,Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Fields {
    pub hp:Option<i32>, pub damage:Option<f32>,pub pierce:Option<f32>,
    pub velocity:Option<[f32;2]>,pub remaining:Option<f32>,
    pub target:Option<String>,pub point:Option<[f32;2]>,
}
#[derive(Clone,Debug,Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Patch {pub kind:Kind,pub id:u64,#[serde(default)]pub layer:Option<u64>,pub fields:Fields}
fn target(s:&str)->Option<TargetMode>{Some(match s{
    "First"=>TargetMode::First,"Last"=>TargetMode::Last,"Close"=>TargetMode::Close,
    "Strong"=>TargetMode::Strong,"Point"=>TargetMode::Point,_=>return None,
})}
impl Patch {
    pub fn validate(&self)->Result<(),String>{
        let f=&self.fields;
        let has_tower=f.target.is_some()||f.point.is_some();
        let has_proj=f.damage.is_some()||f.pierce.is_some()||f.velocity.is_some()||f.remaining.is_some();
        if self.id==0||self.layer.is_some()&&self.kind!=Kind::Bloons
            ||f.hp.is_some_and(|v|v<1||v>1_000_000_000)
            ||[f.damage,f.pierce].into_iter().flatten().any(|v|!v.is_finite()||!(0.0..=1e9).contains(&v))
            ||f.remaining.is_some_and(|v|!v.is_finite()||!(0.0..=3600.0).contains(&v))
            ||f.velocity.is_some_and(|v|v.iter().any(|x|!x.is_finite()||x.abs()>1e5))
            ||f.point.is_some_and(|v|v.iter().any(|x|!x.is_finite()||x.abs()>1000.0))
            ||f.target.as_deref().is_some_and(|s|target(s).is_none())
            ||match self.kind{Kind::Towers=>f.hp.is_some()||has_proj,Kind::Bloons=>has_tower||has_proj,Kind::Projectiles=>has_tower||f.hp.is_some()}{
            return Err("Invalid entity patch fields for this entity kind".into());
        }Ok(())
    }
    pub fn apply(&self,g:&mut Game){
        let f=&self.fields;
        match self.kind{
            Kind::Towers=>{
                let Some(i)=g.towers.iter().position(|t|t.id==self.id)else{return};
                if let Some(mode)=f.target.as_deref().and_then(target){
                    if g.target_modes(i).contains(&mode){g.towers[i].target=mode;}
                }
                if let Some(point)=f.point{if g.uses_point(i)||g.has_point_attack(i){g.towers[i].point=Some(point);}}
            },
            Kind::Bloons=>if let Some(b)=g.bloons.iter_mut().find(|b|b.id==self.id&&self.layer.is_none_or(|l|l==b.layer)){
                if let Some(hp)=f.hp{b.hp=hp;}
            },
            Kind::Projectiles=>if let Some(p)=g.projectiles.iter_mut().find(|p|p.uid==self.id){
                if let Some(damage)=f.damage{p.spec.damage=damage;}
                if let Some(pierce)=f.pierce{p.pierce=pierce;}
                if let Some(velocity)=f.velocity{
                    p.vel=velocity;let length=velocity[0].hypot(velocity[1]);
                    p.dir=(length>0.0).then(||[velocity[0]/length,velocity[1]/length]);
                }
                if let Some(seconds)=f.remaining{p.frames_left=seconds*60.0;}
            },
        }
    }
}
#[cfg(test)]
mod tests{
    use super::*;
    #[test]fn patches_reject_mismatched_fields_and_unbounded_values(){
        for v in [json!({"kind":"towers","id":1,"fields":{"hp":5}}),json!({"kind":"bloons","id":1,"fields":{"hp":0}}),
            json!({"kind":"projectiles","id":1,"fields":{"remaining":4000}}),json!({"kind":"towers","id":1,"fields":{"target":"Unknown"}})]{
            assert!(serde_json::from_value::<Patch>(v).unwrap().validate().is_err());
        }
        assert!(Query{kind:Kind::Bloons,offset:0,limit:129,tag:String::new()}.validate().is_err());
    }
}
