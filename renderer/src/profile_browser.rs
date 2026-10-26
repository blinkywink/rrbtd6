//! Installed profile cosmetics in the original picker. Selection is a local preview only.
use crate::uitree::Node;
use macroquad::prelude::*;
use serde_json::Value;
use std::path::Path;
const BASE:&str="Canvas/PlayerStatsScreen";
pub(super) const MODAL:&str="Canvas/PlayerStatsScreen/SettingsMenu";
pub(super) struct Browser {catalog:Value,runtime:Value,open:bool,banners:bool,selected:Option<usize>,began:f64}
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
        let mut browser=Self{catalog,runtime,open:false,banners:false,selected:None,began:0.0};
        if let Ok(tab)=std::env::var("RRBTD6_PROFILE_PICKER"){
            browser.open=true;browser.began=get_time();node.set_active(MODAL,true);browser.tab(node,tab=="banners");
        }
        Some(browser)
    }
    pub fn modal(&self)->Option<&'static str>{self.open.then_some(MODAL)}
    pub fn dismiss(&mut self,node:&mut Node)->bool{
        if !self.open{return false;}self.open=false;node.set_active(MODAL,false);true
    }
    pub fn animate(&mut self,node:&mut Node){
        if !self.open{return;}
        super::apply_animation(node,&self.runtime,"PlayerStatsSettings","OpenSettings",Some((get_time()-self.began) as f32));
        self.show_tab(node);
        // Purchases and account writes are not implemented. Do not offer fake actions.
        for name in ["Confirm","Buy","IapBuy","Remove","Quest","GoBtn"]{node.set_active(&format!("{MODAL}/Panel/{name}"),false);}
        node.set_active(&format!("{MODAL}/Panel/InfoMessage"),true);
        node.set_text(&format!("{MODAL}/Panel/InfoMessage/Text"),"Local preview only — cloud changes are disabled.");
    }
    fn tab(&mut self,node:&mut Node,banners:bool){
        self.banners=banners;self.selected=None;
        self.show_tab(node);
    }
    fn show_tab(&self,node:&mut Node){
        let banners=self.banners;
        for (name,on) in [("Avatars",!banners),("Banners",banners)] {
            node.set_active(&format!("{MODAL}/Panel/Tabs/{name}/BgSelected"),on);
            node.set_active(&format!("{MODAL}/Panel/Tabs/{name}/Bg"),!on);
        }
        node.set_active(&format!("{MODAL}/Panel/Container/ScrollAvatar"),!banners);
        node.set_active(&format!("{MODAL}/Panel/Container/ScrollBanner"),banners);
    }
    pub fn click(&mut self,node:&mut Node,pointer:Vec2)->bool{
        let hit=|node:&Node,path:&str|node.rect(path).is_some_and(|r|r.contains(pointer));
        if !self.open {
            let avatar=hit(node,&format!("{BASE}/Stats/TopInfo/PlayerAvatar/SettingsBtn"));
            let banner=hit(node,&format!("{BASE}/Stats/TopInfo/BannerSettingsBtn"));
            if !avatar&&!banner{return false;}
            self.open=true;self.began=get_time();node.set_active(MODAL,true);self.tab(node,banner);return true;
        }
        if hit(node,&format!("{MODAL}/Panel/Cancel")){self.dismiss(node);return true;}
        for (name,banners) in [("Avatars",false),("Banners",true)]{
            if hit(node,&format!("{MODAL}/Panel/Tabs/{name}")){self.tab(node,banners);return true;}
        }
        let (category,group)=if self.banners{("banners","ScrollBanner/BannerGroup")}else{("avatars","ScrollAvatar/AvatarGroup")};
        let path=format!("{MODAL}/Panel/Container/{group}");
        let selected=self.catalog[category].as_array().into_iter().flatten().enumerate().find_map(|(i,_)|hit(node,&format!("{path}/Cosmetic{i}")).then_some(i));
        if let Some(i)=selected {
            if let Some(list)=node.find_mut(&path){for (j,card) in list.children.iter_mut().enumerate(){card.set_active("Selected",i==j);}}
            self.selected=Some(i);
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn closing_picker_consumes_back_once_without_leaving_modal_input_capture(){
        let mut browser=Browser{catalog:Value::Null,runtime:Value::Null,open:true,banners:false,selected:Some(2),began:0.0};
        let mut node=Node::from_json(&serde_json::json!({"name":"Scene","children":[{"name":"Canvas","children":[
            {"name":"PlayerStatsScreen","children":[{"name":"SettingsMenu","active":true}]}]}]}));
        assert_eq!(browser.modal(),Some(MODAL));
        assert!(browser.dismiss(&mut node));
        assert!(browser.modal().is_none());
        assert!(!node.find(MODAL).unwrap().active);
        assert!(!browser.dismiss(&mut node));
    }
}
