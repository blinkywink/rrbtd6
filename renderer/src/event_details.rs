//! The boss event rules layout (ChallengeRulesUI) populated from live service responses.
use crate::uitree::Node;
use serde_json::Value;
use std::{path::{Path,PathBuf},time::{SystemTime,UNIX_EPOCH},sync::OnceLock};
static CATALOG:OnceLock<Value>=OnceLock::new();
pub(super) struct Detail {key:&'static str,root:PathBuf,revision:Option<f64>,second:u64}
impl Detail {
    pub fn new(root:&Path,key:&'static str)->Self{
        CATALOG.get_or_init(||{
            let data:Value=std::fs::read(root.join("game/game.json")).ok().and_then(|b|serde_json::from_slice(&b).ok()).unwrap_or_default();
            let mut shop=data["shop"].as_array().cloned().unwrap_or_default();
            for hero in data["heroes"].as_array().into_iter().flatten(){if !shop.iter().any(|r|r["id"]==hero["id"]){shop.push(hero.clone());}}
            serde_json::json!({"shop":shop})
        });
        Self{key,root:root.to_owned(),revision:None,second:u64::MAX}
    }
    pub fn bind(&mut self,node:&mut Node){
        let now=SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs();
        if self.second==now{return;}self.second=now;
        let Some(data)=super::home_events::detail(&self.root,self.key)else{return};
        let base="Canvas/ChallengeRulesScreen/Content";
        if let Some(end)=data["event"]["end"].as_u64(){node.set_text(&format!("{base}/Time Remaining"),&super::home_events::remaining(end.saturating_sub(now*1000)/1000));}
        let revision=data["revision"].as_f64();
        if self.revision.is_some()&&self.revision==revision{return;}self.revision=revision;
        let challenge=&data["challenge"];
        let title_path=format!("{base}/TopInfo/Title");
        node.set_text(&title_path,&data["title"].as_str().unwrap_or("Event details").replace('_'," "));
        node.hide_named(&["ClickBlock","RelicPopupInfo","RelicPopupPoint","RelicsScroll"]);
        if !challenge.is_object(){node.set_text(&title_path,"Event details are currently unavailable");return;}
        let start=&challenge["startRules"];
        for (row,key,native) in [("StartingCashDetail","startingCash","cash"),("StartingLivesDetails","lives","lives"),("StartRoundDetails","startRound","round"),("EndRoundDetails","endRound","endRound")] {
            let v=challenge[key].as_i64().or_else(||start[native].as_i64());
            node.set_text(&format!("{base}/StartingRules/{row}/Value"),&v.map(|v|v.to_string()).unwrap_or_else(||"Default".into()));
        }
        let limit=challenge["maxTowers"].as_i64().unwrap_or(-1);
        node.set_text(&format!("{base}/StartingRules/MaxMonkeysDetail/Value"),&if limit<0{"Unlimited".into()}else{limit.to_string()});
        let tiers=challenge["leastTiersUsed"].as_i64().filter(|n|*n>=0);
        let cash=challenge["leastCashUsed"].as_i64().filter(|n|*n>=0);
        node.set_active(&format!("{base}/StartingRules/LeastCashOrTierDetails"),tiers.or(cash).is_some());
        node.set_text(&format!("{base}/StartingRules/LeastCashOrTierDetails/Lbl"),if tiers.is_some(){"Maximum tiers"}else{"Maximum cash"});
        node.set_text(&format!("{base}/StartingRules/LeastCashOrTierDetails/Value"),&tiers.or(cash).unwrap_or(0).to_string());
        self.rule_rows(node,base,challenge);
        self.towers(node,&format!("{base}/Towers"),challenge);
    }
    fn towers(&self,node:&mut Node,base:&str,challenge:&Value){
        let template=node.find(&format!("{base}/TowerPrefab")).or_else(||node.find(&format!("{base}/TowerScrollRect/TowerContainer/TowerContent/TowerPrefab"))).cloned();
        let Some(template)=template else{return};
        let list=format!("{base}/TowerScrollRect/TowerContainer/TowerContent");
        if let Some(content)=node.find_mut(&list){
            content.children=challenge["towers"].as_array().or_else(||challenge["_towers"].as_array()).into_iter().flatten().filter(|t|t["max"].as_i64()!=Some(0)).enumerate().map(|(i,t)|{
                let mut card=template.clone();card.name=format!("EventTower{i}");card.active=true;
                let id=t["tower"].as_str().unwrap_or("");let id=if id=="ChosenPrimaryHero"{"Quincy"}else{id};
                let shop=CATALOG.get().and_then(|c|c["shop"].as_array()).and_then(|rows|rows.iter().find(|r|r["id"]==id));
                let icon=shop.and_then(|r|r["icon"].as_str()).unwrap_or(id);
                card.set_sprite("Icon",Some(icon));
                let category=match shop.and_then(|r|r["set"].as_i64()){Some(1)=>"Primary",Some(2)=>"Military",Some(4)=>"Magic",Some(8)=>"Support",_=>"Hero"};
                if let Some(img)=card.img.as_mut(){img.sprite=Some(format!("TowerContainer{category}"));}
                card.set_active("ExcludedIcon",false);
                let count=t["max"].as_i64().unwrap_or(-1);card.set_active("TowerCountIcon",count>=0);
                card.set_text("TowerCountIcon/Text",&count.to_string());
                let blocked:[i64;3]=std::array::from_fn(|p|t[format!("path{}NumBlockedTiers",p+1)].as_i64().unwrap_or(0));
                card.set_active("PathNumber",blocked.iter().any(|n|*n>0));
                card.set_text("PathNumber",&format!("{}-{}-{}",5-blocked[0],5-blocked[1],5-blocked[2]));card
            }).collect();
        }
        node.set_active(&format!("{base}/TowerPrefab"),false);
        node.set_active(&format!("{base}/SelectedHeroPrefab"),false);
    }
    fn rule_rows(&self,node:&mut Node,base:&str,c:&Value){
        let Some(template)=node.find(&format!("{base}/RulesModDetailsTemplate")).cloned()else{return};
        let mut flags=Vec::new();
        for (key,label,invert) in [("disableMK","Monkey Knowledge",true),("disablePowers","Powers",true),("disableSelling","Selling",true),("noContinues","Continues",true),("disableDoubleCash","Double Cash",true)]{
            if c[key].as_bool()==Some(true){
                let icon=match key{"disableMK"=>"NoKnowledgeIcon","disablePowers"=>"PowersDisabledIcon","disableSelling"=>"SellingDisabledIcon","noContinues"=>"NoContinuesIcon",_=>"NoDoubleCashIcon"};
                flags.push((label.to_owned(),if invert{"Disabled"}else{"Enabled"}.into(),icon.to_owned()));
            }
        }
        let m=if c["bloonModifiers"].is_object(){&c["bloonModifiers"]}else{&c["_bloonModifiers"]};
        let mut modifiers=Vec::new();
        for (key,label) in [("speedMultiplier","Bloon speed"),("moabSpeedMultiplier","MOAB speed"),("bossSpeedMultiplier","Boss speed"),("regrowRateMultiplier","Regrow rate")] {
            if let Some(v)=m[key].as_f64().filter(|v|(*v-1.0).abs()>0.00001){
                let icon=match key{"speedMultiplier"=>if v>1.0{"FasterBloonsIcon"}else{"SlowerBloonsIcon"},"moabSpeedMultiplier"=>if v>1.0{"FasterMoabIcon"}else{"SlowerMoabIcon"},"bossSpeedMultiplier"=>if v>1.0{"FasterBossIcon"}else{"SlowerBossIcon"},_=>if v>1.0{"RegrowRateIncreaseIcon"}else{"RegrowRateDecreaseIcon"}};
                modifiers.push((label.to_owned(),format!("{:.0}%",v*100.0),icon.to_owned()));
            }
        }
        for (key,label) in [("bloons","Bloon health"),("moabs","MOAB health"),("boss","Boss health")] {
            if let Some(v)=m["healthMultipliers"][key].as_f64().filter(|v|(*v-1.0).abs()>0.00001){
                let icon=match (key,v>1.0){("bloons",true)=>"BloonBoostIcon",("bloons",false)=>"BloonDecreaseHPIcon",("moabs",true)=>"MoabBoostIcon",("moabs",false)=>"MoabDecreaseHPIcon",(_,true)=>"BossBoostIcon",_=>"BossDecreaseHPIcon"};
                modifiers.push((label.to_owned(),format!("{:.0}%",v*100.0),icon.to_owned()));
            }
        }
        for (key,label) in [("abilityCooldownReductionMultiplier","Ability cooldown"),("removeableCostMultiplier","Obstacle cost")] {
            if let Some(v)=c[key].as_f64().filter(|v|(*v-1.0).abs()>0.00001){
                let icon=match (key,v>1.0){("abilityCooldownReductionMultiplier",true)=>"AbilityCooldownReductionIncreaseIcon",("abilityCooldownReductionMultiplier",false)=>"AbilityCooldownReductionDecreaseIcon",(_,true)=>"RemovableCostIncreaseIcon",_=>"RemovableCostDecreaseIcon"};
                modifiers.push((label.to_owned(),format!("{:.0}%",v*100.0),icon.to_owned()));
            }
        }
        for (key,label) in [("allCamo","All camo"),("allRegen","All regrow")] {
            if m[key].as_bool()==Some(true){flags.push((label.to_owned(),"On".into(),if key=="allCamo"{"AllCamoIcon"}else{"AllRegenIcon"}.to_owned()));}
        }
        for (path,rows) in [("RightPanel/RulesScroll/Viewport/Rules",flags),("ModifiersScroll/Viewport/Modifiers",modifiers)] {
            if let Some(content)=node.find_mut(&format!("{base}/{path}")){
                content.children=rows.iter().enumerate().map(|(i,(label,value,icon))|{
                    let mut row=template.clone();row.name=format!("Rule{i}");row.active=true;
                    row.set_text("Layout/Lbl",label);row.set_text("Layout/Value",value);row.set_sprite("Icon",Some(icon));row
                }).collect();
            }
        }
        node.set_active(&format!("{base}/RulesModDetailsTemplate"),false);
    }
}
