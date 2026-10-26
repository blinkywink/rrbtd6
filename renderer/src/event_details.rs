//! Original challenge/race layouts populated from live service responses.
use crate::uitree::Node;
use serde_json::Value;
use std::{path::{Path,PathBuf},time::{SystemTime,UNIX_EPOCH},sync::OnceLock};
static CATALOG:OnceLock<Value>=OnceLock::new();
pub(super) struct Detail {key:&'static str,rules:bool,root:PathBuf,revision:Option<f64>,second:u64}
impl Detail {
    pub fn new(root:&Path,key:&'static str,rules:bool)->Self{
        CATALOG.get_or_init(||{
            let data:Value=std::fs::read(root.join("game/game.json")).ok().and_then(|b|serde_json::from_slice(&b).ok()).unwrap_or_default();
            let maps:Value=std::fs::read(root.join("ui/mapset.json")).ok().and_then(|b|serde_json::from_slice(&b).ok()).unwrap_or_default();
            let mut shop=data["shop"].as_array().cloned().unwrap_or_default();
            for hero in data["heroes"].as_array().into_iter().flatten(){if !shop.iter().any(|r|r["id"]==hero["id"]){shop.push(hero.clone());}}
            serde_json::json!({"shop":shop,"maps":maps})
        });
        Self{key,rules,root:root.to_owned(),revision:None,second:u64::MAX}
    }
    pub fn bind(&mut self,node:&mut Node){
        let now=SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs();
        if self.second==now{return;}self.second=now;
        let Some(data)=super::home_events::detail(&self.root,self.key)else{return};
        let base=if self.rules{"Canvas/ChallengeRulesScreen/Content"}else if self.key=="raceEvent"{"Canvas/RaceEventScreen/Content"}else{"Canvas/DailyChallengeScreen/Content"};
        if let Some(end)=data["event"]["end"].as_u64(){node.set_text(&format!("{base}/Time Remaining"),&super::home_events::remaining(end.saturating_sub(now*1000)/1000));}
        if !self.rules{if let Some(id)=data["event"]["id"].as_str(){
            let done=super::account_preview::event_progress(self.key,id).is_some_and(|p|p["hasCompletedOneRun"]==true);
            node.set_active(&format!("{base}/ClaimedPanel"),done);
        }}
        let revision=data["revision"].as_f64();
        if self.revision.is_some()&&self.revision==revision{return;}self.revision=revision;
        let challenge=&data["challenge"];
        let title=data["title"].as_str().unwrap_or("Event details").replace('_'," ");
        let title_path=if self.rules{format!("{base}/TopInfo/Title")}else{format!("{}/TopInfo/Title",base.trim_end_matches("/Content"))};
        node.set_text(&title_path,&title);
        node.hide_named(&["ClickBlock","LoadingObject","StatsPopup","LatestWinner","FirstWinner","ModifierPopUpPanel","RelicPopupInfo","RelicPopupPoint","RelicsScroll","FreeEntryTimerObject","FreeEntriesObj","RacePassActiveObj","TooLateToStartObj","RaceTicketsBtn"]);
        if !challenge.is_object(){node.set_text(&title_path,"Event details are currently unavailable");return;}
        let start=&challenge["startRules"];
        if self.rules{
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
        }else{
            let map=challenge["map"].as_str().unwrap_or("");
            node.set_text(&format!("{base}/MapName"),map);
            node.set_text(&format!("{base}/Difficulty"),challenge["difficulty"].as_str().unwrap_or(""));
            let sprite=CATALOG.get().and_then(|c|c["maps"].as_array()).and_then(|maps|maps.iter().find(|m|m["id"]==map)).and_then(|m|m["sprite"].as_str());
            node.set_sprite(&format!("{base}/MapMask/MapImage"),sprite);
            node.set_active(&format!("{base}/ClaimedPanel"),challenge["id"].as_str().is_some_and(super::account_preview::daily_completed));
            let limit=challenge["maxTowers"].as_i64().unwrap_or(-1);
            node.set_active(&format!("{base}/MaxTowersBanner"),limit>=0);
            node.set_text(&format!("{base}/MaxTowersBanner/Txt"),&limit.to_string());
            // No invented saved games, reward claims, race entries or leaderboard submissions.
            node.set_active(&format!("{base}/Buttons/ContinueBtn"),false);
            node.set_active(&format!("{base}/CoopButtons"),false);
            node.set_active(&format!("{base}/PlayBtn"),self.key=="raceEvent");
            node.set_text(&format!("{base}/PlayBtn/Text"),"Practice");
            node.set_active(&format!("{base}/PlayBtn/RetryCost"),false);
            node.set_active(&format!("{base}/Buttons/PlayBtn"),self.key!="raceEvent");
            node.set_text(&format!("{base}/Buttons/PlayBtn/Text"),if self.key=="daily_coop"{"Solo Practice"}else{"Play"});
            node.set_active(&format!("{base}/Buttons/PlayBtn/Lock"),false);
            node.set_active(&format!("{base}/Buttons/PlayBtn/LockedButtonPopUpPanel"),false);
            node.set_active(&format!("{base}/RaceTicketsCount"),false);
            node.set_active(&format!("{base}/TrophiesEarned"),false);
            self.rewards(node,base,challenge["rewards"].as_str().unwrap_or(""));
            node.set_text(&format!("{base}/Date"),challenge["mode"].as_str().unwrap_or("Standard"));
            for (part,key) in [("Plays","plays"),("Wins","wins"),("Fails","losses"),("UniquePlayers","playsUnique"),("Victorious players","winsUnique")] {
                node.set_text(&format!("{base}/StatsPopup/StatsGroup/{part}/Value"),&challenge[key].as_u64().map(|n|n.to_string()).unwrap_or_else(||"—".into()));
            }
        }
        self.towers(node,base,challenge);
    }
    fn rewards(&self,node:&mut Node,base:&str,rewards:&str){
        if self.key=="raceEvent"{return;} // Authored badge/trophy strip uses fixed native positions.
        let Some(template)=Node::load(&self.root.join("ui/prefabs/SmallBaseLootItem.json"))else{return};
        let path=if self.key=="raceEvent"{format!("{base}/Badges/TowerContainer")}else{format!("{base}/LootPanel")};
        if let Some(container)=node.find_mut(&path){
            container.active=true;
            container.children=rewards.split('#').filter_map(|token|{
                let (kind,value)=token.split_once(':')?;let parts:Vec<_>=value.split(',').collect();
                let (icon,count)=match kind{
                    "MonkeyMoney"=>("BloonjaminsIcon".to_owned(),value.to_owned()),
                    "Trophy"=>("TrophyIcon".to_owned(),value.to_owned()),
                    "RandomPower"=>("PowersIcon".to_owned(),parts.get(1).unwrap_or(&"1").to_string()),
                    "RandomInstaMonkey"=>(format!("InstaRandomTier{}",parts.first().unwrap_or(&"1")),parts.get(1).unwrap_or(&"1").to_string()),
                    _=>return None
                };
                let mut item=template.clone();item.name=kind.to_owned();item.active=true;
                item.set_sprite("Icon",Some(&icon));item.set_text("Description",&count);
                item.hide_named(&["AddOne","Bonus","LimitedTime"]);Some(item)
            }).collect();
        }
    }
    fn towers(&self,node:&mut Node,base:&str,challenge:&Value){
        let base=if self.rules{format!("{base}/Towers")}else{base.to_owned()};
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
