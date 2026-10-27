//! Native profile cosmetics picker; cloud selections preview, local selections persist.
use crate::uitree::Node;
use macroquad::prelude::*;
use serde_json::Value;
use std::path::Path;
const BASE:&str="Canvas/PlayerStatsScreen";
pub(super) const MODAL:&str="Canvas/PlayerStatsScreen/SettingsMenu";
pub(super) struct Browser {public_stats:bool,root:std::path::PathBuf,catalog:Value,runtime:Value,open:bool,banners:bool,selected:Option<usize>,began:Option<f64>,closing:bool,equipped:[Option<String>;2],overall:Option<(Vec<Node>,Vec<Node>,String)>,named:Option<usize>,pub stats_changed:bool}
impl Browser {
    pub fn load(root:&Path,node:&mut Node)->Option<Self>{
        let catalog:Value=serde_json::from_str(&std::fs::read_to_string(root.join("ui/runtime/profile_cosmetics.json")).ok()?).ok()?;
        let mut runtime:Value=serde_json::from_str(&std::fs::read_to_string(root.join("ui/runtime/PlayerStatsUI.json")).ok()?).ok()?;
        if let Some(animators)=runtime["animators"].as_array_mut(){animators.retain(|a|a["controller"]=="PlayerStatsSettings");}
        for (category,template,group) in [("avatars","ProfileAvatarItem","ScrollAvatar/AvatarGroup"),("banners","ProfileBannerItem","ScrollBanner/BannerGroup")] {
            let template=Node::load(&root.join(format!("ui/prefabs/{template}.json")))?;
            if let Some(list)=node.find_mut(&format!("{MODAL}/Panel/Container/{group}")){
                list.children=catalog[category].as_array().into_iter().flatten().enumerate().map(|(i,item)|{
                    let mut card=template.clone();card.name=format!("Cosmetic{i}");card.active=true;
                    card.set_sprite("Image",item["icon"].as_str());
                    card.hide_named(&["Selected","EquippedTick","TrophyStoreCost","IapStoreLock","QuestLock","LimitedTime"]);
                    card
                }).collect();
            }
        }
        let mut browser=Self{public_stats:false,root:root.to_owned(),catalog,runtime,open:false,banners:false,selected:None,began:None,closing:false,equipped:[None,None],overall:None,named:None,stats_changed:false};
        if let Ok(tab)=std::env::var("RRBTD6_PROFILE_PICKER"){
            browser.open=true;node.set_active(MODAL,true);browser.tab(node,tab=="banners");
        }
        Some(browser)
    }
    pub fn modal(&self)->Option<&'static str>{self.open.then_some(MODAL)}
    pub fn dismiss(&mut self,node:&mut Node)->bool{
        if !self.open{return false;}
        if !self.closing{self.closing=true;self.began=None;}
        node.set_active(MODAL,true);true
    }
    pub fn animate(&mut self,node:&mut Node){
        self.stats_view(node);
        if !self.open{return;}
        self.sync_equipped(node);
        let clip=if self.closing{"ClosedSettings"}else{"OpenSettings"};
        let complete=super::apply_animation(node,&self.runtime,"PlayerStatsSettings",clip,
            Some(self.began.map(|t|(get_time()-t) as f32).unwrap_or(0.0)));
        if self.closing && complete && self.began.is_some(){
            self.open=false;self.closing=false;self.began=None;node.set_active(MODAL,false);return;
        }
        self.show_tab(node);
        // Cloud writes remain disabled. The native error strip is only needed when a
        // different cosmetic is selected, rather than covering the grid while browsing.
        for name in ["Confirm","Buy","IapBuy","Remove","Quest","GoBtn"]{node.set_active(&format!("{MODAL}/Panel/{name}"),false);}
        let local=super::account_preview::is_local();
        let changed=self.selected.is_some() && self.selected!=self.equipped_index();
        node.set_active(&format!("{MODAL}/Panel/Confirm"),local && changed);
        node.set_active(&format!("{MODAL}/Panel/InfoMessage"),!local && changed);
        node.set_text(&format!("{MODAL}/Panel/InfoMessage/Text"),"Preview only. Ninja Kiwi account changes are disabled.");
    }
    pub fn presented(&mut self){if self.open && self.began.is_none(){self.began=Some(get_time());}}
    fn equipped_index(&self)->Option<usize>{
        let category=if self.banners{"banners"}else{"avatars"};
        let icon=self.equipped[self.banners as usize].as_deref()?;
        self.catalog[category].as_array()?.iter().position(|item|item["icon"].as_str()==Some(icon))
    }
    fn sync_equipped(&mut self,node:&mut Node){
        let current=["PlayerAvatar/Icon","CustomBanner/Image"].map(|path|
            node.find(&format!("{BASE}/Stats/TopInfo/{path}")).and_then(|n|n.img.as_ref()).and_then(|i|i.sprite.clone()));
        if self.equipped!=current{
            self.equipped=current;self.selected=self.equipped_index();self.refresh_cards(node);
        }
    }
    fn refresh_cards(&self,node:&mut Node){
        for (i,(category,group)) in [("avatars","ScrollAvatar/AvatarGroup"),("banners","ScrollBanner/BannerGroup")].iter().enumerate(){
            if let Some(list)=node.find_mut(&format!("{MODAL}/Panel/Container/{group}")){
                for (j,card) in list.children.iter_mut().enumerate(){
                    card.set_active("EquippedTick",self.equipped[i].as_deref().is_some_and(|icon|self.catalog[category][j]["icon"].as_str()==Some(icon)));
                    card.set_active("Selected",self.banners as usize==i && self.selected==Some(j));
                }
            }
        }
    }
    fn stats_view(&self,node:&mut Node){
        node.set_active(&format!("{BASE}/Stats/OverallStats/PublicStats"),self.public_stats);
        node.set_active(&format!("{BASE}/Stats/OverallStats/ConfigureStats"),!self.public_stats);
        node.set_text(&format!("{BASE}/Stats/Own/PublicSwapButton/Text"),if self.public_stats{"All Stats"}else{"Public Stats"});
    }
    fn tab(&mut self,node:&mut Node,banners:bool){
        self.banners=banners;self.sync_equipped(node);self.selected=self.equipped_index();self.refresh_cards(node);
        self.show_tab(node);
    }
    fn show_tab(&self,node:&mut Node){
        let banners=self.banners;
        for (name,on) in [("Avatars",!banners),("Banners",banners)] {
            node.set_active(&format!("{MODAL}/Panel/Tabs/{name}/BgSelected"),on);
            node.set_active(&format!("{MODAL}/Panel/Tabs/{name}/Bg"),true);
        }
        node.set_active(&format!("{MODAL}/Panel/Container/ScrollAvatar"),!banners);
        node.set_active(&format!("{MODAL}/Panel/Container/ScrollBanner"),banners);
    }
    fn select_named(&mut self,node:&mut Node,index:usize){
        let base=format!("{BASE}/Stats/OverallStats");
        let private=format!("{base}/ConfigureStats/PlayerStats/Stats");
        let public=format!("{base}/PublicStats/PlayerStats/Stats");
        if self.overall.is_none(){
            let rows=|path:&str|node.find(path).map(|n|n.children.clone()).unwrap_or_default();
            let shared=node.find(&format!("{base}/SharedStats")).and_then(|n|n.txt.as_ref()).map(|t|t.text.clone()).unwrap_or_default();
            self.overall=Some((rows(&private),rows(&public),shared));
        }
        let mut rows=Vec::new();let mut shared_rows=Vec::new();
        let title;
        if index==0{
            let Some((all,visible,shared))=&self.overall else{return;};
            rows=all.clone();shared_rows=visible.clone();
            node.set_text(&format!("{base}/SharedStats"),shared);title="Overall Stats".to_owned();self.named=None;
        }else{
            let Some(item)=super::account_preview::named_monkey(index-1) else{return;};
            let Some(template)=Node::load(&self.root.join("ui/prefabs/ProfileStat.json")) else{return;};
            for (key,label,id) in NAMED_STATS{
                let Some(value)=item["stats"][key].as_f64() else{continue;};
                let mut row=template.clone();row.name=format!("NamedStat_{key}");row.active=true;
                row.set_text("Name",label);row.set_text("Value",&super::account_preview::number(value));
                let visible=item["visible"][format!("PlayerStats{id}")]==true;
                row.set_active("Toggle/Tick",visible);
                if visible{let mut shared=row.clone();shared.set_active("Toggle",false);shared_rows.push(shared);}
                rows.push(row);
            }
            node.set_text(&format!("{base}/SharedStats"),&format!("Shared {}/7",shared_rows.len()));
            title=item["name"].as_str().unwrap_or("Monkey Stats").to_owned();self.named=Some(index-1);
        }
        if let Some(list)=node.find_mut(&private){list.children=rows;}
        if let Some(list)=node.find_mut(&public){list.children=shared_rows;}
        node.set_text(&format!("{base}/StatCategoryDropdown/Text"),&title);
        if let Some(list)=node.find_mut(&format!("{BASE}/Stats/NamedMonkeysMenu/OptionScroll/Content")){
            for (i,row) in list.children.iter_mut().enumerate(){row.set_active("NamedMonkeyPanel/Selected",i==index);}
        }
        self.stats_changed=true;
    }
    pub fn click(&mut self,node:&mut Node,pointer:Vec2)->bool{
        if self.closing{return true;}
        let hit=|node:&Node,path:&str|node.rect(path).is_some_and(|r|r.contains(pointer));
        if !self.open{
            let path=format!("{BASE}/Stats/NamedMonkeysMenu/OptionScroll/Content");
            let selected=node.find(&path).and_then(|list|list.children.iter().enumerate().find(|(_,row)|hit(node,&format!("{path}/{}",row.name))).map(|(i,_)|i));
            if let Some(index)=selected{self.select_named(node,index);return true;}
        }
        if !self.open && hit(node,&format!("{BASE}/Stats/Own/PublicSwapButton")) {
            self.public_stats=!self.public_stats;self.stats_view(node);return true;
        }
        if !self.open && !self.public_stats && super::account_preview::is_local(){
            let list=format!("{BASE}/Stats/OverallStats/ConfigureStats/PlayerStats/Stats");
            let clicked=node.find(&list).and_then(|n|n.children.iter().find(|r|hit(node,&format!("{list}/{}/Toggle",r.name)))).map(|n|n.name.trim_start_matches("AccountStat_").to_owned());
            if let Some(key)=clicked {
                if crate::local_profile::update(&self.root,|p|{
                    // PlayerStatsScreen.Stat_OnSelect caps the shared list at seven.
                    let selected=p.visible_stats.get(&key)==Some(&true);
                    if selected || p.visible_stats.values().filter(|v|**v).count()<7{
                        p.visible_stats.insert(key,!selected);
                    }
                }).is_ok(){super::account_preview::refresh_local(&self.root);}
                return true;
            }
        }
        if !self.open {
            let avatar=hit(node,&format!("{BASE}/Stats/TopInfo/PlayerAvatar/SettingsBtn"));
            let banner=hit(node,&format!("{BASE}/Stats/TopInfo/BannerSettingsBtn"));
            if !avatar&&!banner{return false;}
            self.open=true;self.closing=false;self.began=None;node.set_active(MODAL,true);self.tab(node,banner);return true;
        }
        if super::account_preview::is_local() && hit(node,&format!("{MODAL}/Panel/Confirm")) {
            let category=if self.banners{"banners"}else{"avatars"};
            if let Some(icon)=self.selected.and_then(|i|self.catalog[category][i]["icon"].as_str()) {
                if crate::local_profile::update(&self.root,|p|if self.banners{p.banner=icon.to_owned();}else{p.avatar=icon.to_owned();}).is_ok(){
                    super::account_preview::refresh_local(&self.root);self.dismiss(node);
                }
            }
            return true;
        }
        if hit(node,&format!("{MODAL}/Panel/Cancel")){self.dismiss(node);return true;}
        for (name,banners) in [("Avatars",false),("Banners",true)]{
            if hit(node,&format!("{MODAL}/Panel/Tabs/{name}")){self.tab(node,banners);return true;}
        }
        let (category,group)=if self.banners{("banners","ScrollBanner/BannerGroup")}else{("avatars","ScrollAvatar/AvatarGroup")};
        let path=format!("{MODAL}/Panel/Container/{group}");
        let selected=self.catalog[category].as_array().into_iter().flatten().enumerate().find_map(|(i,_)|hit(node,&format!("{path}/Cosmetic{i}")).then_some(i));
        if let Some(i)=selected {
            self.selected=Some(i);self.refresh_cards(node);
        }
        true
    }
}

// NamedMonkeyStats fields and their PublicNamedMonkeyStats visibility ids.
const NAMED_STATS:[(&str,&str,&str);22]=[
    ("gamesWon","Games Won","GamesWon"),("highestRound","Highest Round","HighestRound"),
    ("timesPlaced","Times Placed","TimesPlaced"),("totalPopCount","Bloons Popped","PopCount"),
    ("totalCoopPopCount","Co-op Bloons Popped","CoopPopCount"),("camoBloonsPopped","Camo Bloons Popped","CamoPopped"),
    ("leadBloonsPopped","Lead Bloons Popped","LeadPopped"),("goldenBloonsPopped","Golden Bloons Popped","GoldenBloonsPopped"),
    ("purpleBloonsPopped","Purple Bloons Popped","PurplePopped"),("regrowBloonsPopped","Regrow Bloons Popped","RegrowPopped"),
    ("ceramicBloonsPopped","Ceramic Bloons Popped","CeramicPopped"),("moabsPopped","MOABs Popped","MoabsPopped"),
    ("bfbsPopped","BFBs Popped","BfbsPopped"),("zomgsPopped","ZOMGs Popped","ZomgsPopped"),
    ("ddtsPopped","DDTs Popped","DdtsPopped"),("badsPopped","BADs Popped","BadsPopped"),
    ("bossesPopped","Bosses Popped","BossesPopped"),("cashGenerated","Cash Generated","CashGenerated"),
    ("abilitiesUsed","Abilities Used","AbilitiesUsed"),("timesUpgraded","Times Upgraded","TimesUpgraded"),
    ("timesSacrificed","Times Sacrificed","TimesSacrificed"),("damageDoneToBosses","Damage Done to Bosses","DamageDoneToBosses")];

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn closing_picker_keeps_modal_input_capture_during_native_close_animation(){
        let mut browser=Browser{public_stats:false,root:Default::default(),catalog:Value::Null,runtime:Value::Null,open:true,banners:false,selected:Some(2),began:None,closing:false,equipped:[None,None],overall:None,named:None,stats_changed:false};
        let mut node=Node::from_json(&serde_json::json!({"name":"Scene","children":[{"name":"Canvas","children":[
            {"name":"PlayerStatsScreen","children":[{"name":"SettingsMenu","active":true}]}]}]}));
        assert_eq!(browser.modal(),Some(MODAL));
        assert!(browser.dismiss(&mut node));
        assert_eq!(browser.modal(),Some(MODAL));
        assert!(browser.closing);
        assert!(node.find(MODAL).unwrap().active);
        assert!(browser.dismiss(&mut node));
        assert!(browser.began.is_none());
    }
}
