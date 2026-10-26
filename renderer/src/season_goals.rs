//! Native goal/phase/tier templates populated from the current season configuration.
use crate::uitree::Node;
use serde_json::Value;
use std::{path::{Path,PathBuf},time::{SystemTime,UNIX_EPOCH}};

pub(super) const BASE:&str="Canvas/SocialSeasonsGoalsUI";
const PHASES:&str="Canvas/SocialSeasonsGoalsUI/CompetitionActive/SocialSeasonPhasesContainer";
pub(super) struct Browser {
    root:PathBuf,catalog:Value,template:Node,revision:Option<Value>,second:u64,
    pub regions:Vec<(String,String)>,stages:Vec<String>,
}
impl Browser {
    pub fn new(root:&Path,node:&mut Node)->Option<Self>{
        let template=node.find(&format!("{PHASES}/SocialSeasonPhase"))?.clone();
        let catalog=std::fs::read(root.join("ui/runtime/season_goals.json")).ok()
            .and_then(|b|serde_json::from_slice(&b).ok()).unwrap_or_default();
        node.hide_named(&["Loading","Jewel","TeamTrophies","CurrentPlayerStats","TierDescriptionPopup"]);
        node.set_active(&format!("{BASE}/CompetitionActive"),false);
        Some(Self{root:root.into(),catalog,template,revision:None,second:u64::MAX,regions:Vec::new(),stages:Vec::new()})
    }
    /// Returns true only when layout contents changed; scrolling state survives timer ticks.
    pub fn bind(&mut self,node:&mut Node)->bool{
        let now=SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs();
        if self.second==now{return false;}self.second=now;
        let Some(data)=super::home_events::season_snapshot(&self.root) else{return false};
        let event=&data["event"];
        node.set_text(&format!("{BASE}/TopInfo/Countdown/Time"),&super::home_events::remaining(event["end"].as_u64().unwrap_or(now*1000).saturating_sub(now*1000)/1000));
        let suffix=super::home_events::theme_suffix(data["theme"].as_str());
        for (path,prefix) in [("TopInfo/BannerMask/BannerImage","ThemeBannerFull"),("TopInfo/Inner/PortraitIcon","ThemeMonkey")] {
            node.set_active(&format!("{BASE}/{path}"),suffix.is_some());
            if let Some(suffix)=suffix{node.set_sprite(&format!("{BASE}/{path}"),Some(&format!("{prefix}{suffix}")));}
        }
        let rev=serde_json::json!([event["id"],data["settings"]]);
        if self.revision.as_ref()==Some(&rev){self.phase_locks(node,&data,now);return false;}self.revision=Some(rev);
        let Some(phases)=data["settings"]["phasesSettings"].as_array()else{return false};
        self.regions.clear();self.stages.clear();let mut children=Vec::new();
        for (pi,goals) in phases.iter().enumerate(){
            let mut phase=self.template.clone();phase.name=format!("Phase{pi}");phase.active=true;
            phase.set_text("StagesGroup/Header/Title",&format!("{} {}",self.catalog["phase"].as_str().unwrap_or("Phase"),pi+1));
            // Goal progress is not available from the account cache. Do not invent lock/completion states.
            phase.set_active("StagesGroup/PhaseLocked",false);
            let Some(template)=phase.find("StagesGroup/Scroll/Viewport/Stages/Stage").cloned()else{continue};
            let mut stages=Vec::new();
            for (gi,goal) in goals.as_array().into_iter().flatten().enumerate(){
                let mut stage=template.clone();stage.name=format!("Goal{gi}");stage.active=true;
                let id=goal["displayId"].as_str().unwrap_or("");let entry=&self.catalog["goals"][id];
                stage.set_text("TaskHeader/Title",entry["label"].as_str().unwrap_or("Goal"));
                stage.set_sprite("TaskHeader/Image",entry["icon"].as_str());
                stage.hide_named(&["Complete","ProgressLocked","Flash","Tick","TierComplete","TierLocked","InfoBtn"]);
                let tier_template=stage.find("AllTiers/TierGroupTemplate").cloned();
                if let (Some(template),Some(list))=(tier_template,stage.find_mut("AllTiers")){
                    list.children=goal["tierConditions"].as_array().into_iter().flatten().enumerate().map(|(ti,condition)|{
                        let mut tier=template.clone();tier.name=format!("Tier{ti}");tier.active=true;
                        tier.hide_named(&["TierComplete","TierLocked","Flash","InfoBtn"]);
                        tier.set_text("Tier1Details/TierDescription",&format!("{} {}",self.catalog["tier"].as_str().unwrap_or("Tier"),ti+1));
                        let target=condition["scoreTarget"].as_u64().or_else(||condition["count"].as_u64());
                        tier.set_text("Progress/ProgressText",&target.map(|n|format!("— / {}",commas(n))).unwrap_or_else(||"—".into()));
                        tier.set_text("Progress/Stars/Star1/StarIcon/Amount",&goal["tierRewards"][ti].as_u64().unwrap_or(0).to_string());
                        let trophies=goal["tierTeamTrophyRewards"][ti].as_u64().unwrap_or(0);
                        tier.set_active("Progress/Stars/Star1/StarIcon/TrophyIcon",trophies>0);
                        tier.set_text("Progress/Stars/Star1/StarIcon/TrophyIcon/TrophyAmount",&trophies.to_string());
                        clear_fills(&mut tier);tier
                    }).collect();
                }
                stage.set_active("AllTiers",pi==0&&gi==0&&std::env::var_os("RRBTD6_GOALS_EXPAND").is_some());
                // The header's stars are tier rewards, not an account progress sample.
                if let (Some(template),Some(stars))=(stage.find("TaskHeader/Progress/Stars/Star1").cloned(),stage.find_mut("TaskHeader/Progress/Stars")){
                    stars.children=goal["tierRewards"].as_array().into_iter().flatten().enumerate().map(|(i,r)|{
                        let mut star=template.clone();star.name=format!("Reward{i}");star.active=true;
                        star.set_text("StarIcon/Amount",&r.as_u64().unwrap_or(0).to_string());
                        star.set_active("StarIcon/TrophyIcon",false);star
                    }).collect();
                }
                clear_fills(&mut stage);
                self.stages.push(format!("{PHASES}/Phase{pi}/StagesGroup/Scroll/Viewport/Stages/Goal{gi}"));
                stages.push(stage);
            }
            if let Some(list)=phase.find_mut("StagesGroup/Scroll/Viewport/Stages"){list.children=stages;}
            let view=format!("{PHASES}/Phase{pi}/StagesGroup/Scroll/Viewport");
            self.regions.push((view.clone(),format!("{view}/Stages")));children.push(phase);
        }
        if let Some(group)=node.find_mut(PHASES){group.children=children;}
        node.set_active(&format!("{BASE}/CompetitionActive"),true);self.phase_locks(node,&data,now);true
    }
    fn phase_locks(&self,node:&mut Node,data:&Value,now:u64){
        let interval=timespan(data["settings"]["phaseUnlockInterval"].as_str().unwrap_or(""));
        let start=data["event"]["start"].as_u64().unwrap_or(0)/1000;
        for pi in 0..self.regions.len(){
            let path=format!("{PHASES}/Phase{pi}/StagesGroup/PhaseLocked");
            let unlock=start.saturating_add(interval.saturating_mul(pi as u64));
            node.set_active(&path,now<unlock);
            node.set_active(&format!("{path}/UnlockText"),false);
            node.set_text(&format!("{path}/UnlockTimer/Text"),&format!("Phase {} unlocks in",pi+1));
            node.set_text(&format!("{path}/UnlockTimer/Time"),&super::home_events::remaining(unlock.saturating_sub(now)));
        }
    }
    pub fn click(&self,node:&mut Node,pointer:macroquad::prelude::Vec2){
        for stage in &self.stages{
            if let Some((phase,_))=stage.split_once("/StagesGroup/"){
                if node.find(&format!("{phase}/StagesGroup/PhaseLocked")).is_some_and(|n|n.active){continue;}
            }
            if node.rect(&format!("{stage}/TaskHeader")).is_some_and(|r|r.contains(pointer)){
                let path=format!("{stage}/AllTiers");let open=node.find(&path).is_some_and(|n|n.active);node.set_active(&path,!open);break;
            }
        }
    }
}
fn clear_fills(n:&mut Node){if n.name=="InnerBar"{n.active=false;}if let Some(i)=&mut n.img{if i.kind==3{i.fill_amount=0.0;}}for c in &mut n.children{clear_fills(c);}}
fn timespan(s:&str)->u64{
    let (days,clock)=s.split_once('.').map(|(d,c)|(d.parse::<u64>().unwrap_or(0),c)).unwrap_or((0,s));
    let values:Vec<u64>=clock.split(':').map(|v|v.parse().unwrap_or(0)).collect();
    if values.len()!=3{return 0;}days*86400+values[0]*3600+values[1]*60+values[2]
}
fn commas(n:u64)->String{let s=n.to_string();let len=s.len();s.chars().enumerate().flat_map(|(i,c)|if i>0&&(len-i)%3==0{vec![',',c]}else{vec![c]}).collect()}

#[cfg(test)] mod tests{use super::*;#[test]fn exact_integer_goal_targets(){assert_eq!(commas(1500000000),"1,500,000,000");assert_eq!(commas(12),"12");assert_eq!(timespan("7.00:00:00"),604800);}}
