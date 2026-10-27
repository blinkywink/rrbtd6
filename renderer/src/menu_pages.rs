//! Lazily loaded original menu scene layouts. Transactions and online services remain unwired.
use super::{apply_animation, scene_node, ScrollInput};
use macroquad::prelude::*;
use std::collections::HashMap;
use crate::uitree::Node;
use std::path::Path;

pub(super) struct Page {
    first_presentation:bool,
    pub node: Node,
    pub(super) season_second:Option<u64>,
    buttons:super::button_motion::ButtonMotion,
    profile:Option<super::profile_browser::Browser>,
    pub(super) jukebox:Option<super::jukebox::Browser>,
    event:Option<super::event_details::Detail>,
    goals:Option<super::season_goals::Browser>,
    coop:Option<super::coop_menu::Lobby>,
    scrolls: Vec<ScrollRegion>,
    trophy_items: Vec<serde_json::Value>,
    trophy_selected: Option<usize>,
    trophy: Option<TrophyState>,
    pointer_start: Option<Vec2>,
    pointer_dragged: bool,
    animation: Option<PageAnimation>,
    powers: Option<PowerBrowser>,
    search: Option<AchievementSearch>,
    knowledge: Option<KnowledgeBrowser>,
    legends: Option<LegendsBrowser>,
    join_code: Option<JoinCode>,
    social:Option<super::social_pages::SocialPage>,
    pub(super) home_requested: bool,
}
const JOIN_INPUT:&str="CoopJoinMatchCanvas/CoopJoinScreen/ButtonGroup/BgPanel/Code/CodeInput";
struct JoinCode {text:String,focused:bool,limit:usize,submit:bool}
struct LegendsBrowser {data:serde_json::Value,selected:usize,about:bool,animation:Option<PageAnimation>,runtime:serde_json::Value}
struct KnowledgeBrowser {
    items:Vec<serde_json::Value>,selected:Option<usize>,panel_animation:Option<PageAnimation>,
    /// KnowledgeSkillBtn.currState per item (0 = not yet set, 1 available, 2 locked,
    /// 3 purchased, 4 disabled) and when its arrows began fading to the purchased colour.
    states:Vec<u8>,arrow_fade:Vec<Option<f64>>,
    owned:std::collections::HashSet<String>,disabled:std::collections::HashSet<String>,points:i64,
    /// downloadedAt of the account the states were computed from.
    account:Option<f64>,
    /// CircleVfxAnimation "pulse" animators of the available nodes (KnowledgeSkillBtn.availableEffect).
    pulse:serde_json::Value,runtime:serde_json::Value,
}
const KNOWLEDGE_PANEL:&str="Canvas/KnowledgeSkillTree/SelectedPanel";
struct AchievementSearch {terms:Vec<String>,query:String,focused:bool}
const ACHIEVEMENT_GRID:&str="Canvas/AchievementsScreen/Scroll/Viewport/Content";
const ACHIEVEMENT_SEARCH:&str="Canvas/AchievementsScreen/TopInfo/SearchPanel/SearchBar";
struct PowerPortrait {node:Node,runtime:serde_json::Value}
struct PowerBrowser {
    items:Vec<serde_json::Value>,selected:usize,began:f64,dir:std::path::PathBuf,runtime_dir:std::path::PathBuf,
    portraits:HashMap<usize,Option<PowerPortrait>>,
    /// Whether the account preview was applied (it loads in the background).
    bound:bool,
    /// GameModel.GetModdedPowers cost and Btd6Player.GetPowerData quantity per item.
    costs:Vec<i64>,quantities:Vec<Option<i64>>,
    /// ProfileModel.seenPowers (PowerSelectButton.SetSelected adds to it).
    seen:std::collections::HashSet<String>,
    /// PowersProPowerSaveData per Pro power id: xp, unlockedTier, seenUnlockPro, seenUnlockTier.
    pro:HashMap<String,(f64,f64,bool,f64)>,
    /// CommonBackgroundScreen.AnimateChangeBackgroundColourTint: from, to, start time.
    tint:(Color,Color,f64),
    /// Buttons SetupPowerButtons created (SkuSettings.CanShowPower).
    shown:Vec<bool>,
}
const POWER_TABS:&str="Canvas/PowersSelectScreen/SafeArea/Selections/LayoutGroup/ScrollGroup/Tabs";
/// PowersSelectScreen themes (commonBgColour, descriptionTextColour, objects) from the PowersSelectUI scene.
const POWER_THEMES:[([f32;3],[f32;3],&[&str]);3]=[
    ([0.0,0.52156866,1.0],[0.11372549,0.19215687,0.24705882],&["StandardPanel"]),
    ([0.0,0.3137255,0.83137256],[1.0,1.0,1.0],&["ProPanel","ProUpgradesButton"]),
    ([0.40784314,0.16862746,0.7019608],[1.0,1.0,1.0],&["StorePanel"]),
];
impl PowerBrowser {
    fn tint_now(&self)->Color {
        // Lerps from the current colour (alpha forced to 1) over 0.25 s of unscaled time.
        let (from,to,began)=self.tint;
        let f=((get_time()-began)/0.25).clamp(0.0,1.0) as f32;
        Color::new(from.r+(to.r-from.r)*f,from.g+(to.g-from.g)*f,from.b+(to.b-from.b)*f,from.a+(to.a-from.a)*f)
    }
    fn id(&self,index:usize)->&str {self.items.get(index).and_then(|i|i["id"].as_str()).unwrap_or("")}
    /// PowersProDataHelper.IsProUnlockedForPower: unlockedTier > 0.
    fn pro_unlocked(&self,id:&str)->bool {self.pro.get(id).is_some_and(|p|p.1>0.0)}
    /// PowersProDataHelper.GetXpForPowerProCurrentUpgrade over the path-0 upgrade costs, and
    /// GameModel.GetXpCostForProUpgrade(id, 0).
    fn pro_xp(&self,id:&str)->(i64,i64) {
        let Some(item)=self.items.iter().find(|i|i["id"].as_str()==Some(id)) else {return (0,-1)};
        let costs:Vec<i64>=item["proXp"].as_array().into_iter().flatten().filter_map(|v|v.as_i64()).collect();
        let mut xp=self.pro.get(id).map(|p|p.0 as i64).unwrap_or(0);
        if xp>0 {for &cost in &costs {if xp<cost {break;} xp-=cost;}}
        (xp,costs.first().copied().unwrap_or(-1))
    }
    /// PowerSelectButton.UpdateNewRibbon.
    fn ribbon(&self,index:usize)->bool {
        let item=&self.items[index];
        if item["showAsNew"]!=true {return false;}
        let id=self.id(index);
        if !self.seen.contains(id) {return true;}
        item["pro"]==true && self.pro.get(id).is_some_and(|&(_,tier,seen_pro,seen_tier)|tier>0.0 && (!seen_pro || seen_tier<tier))
    }
}
const POWER_GRID:&str="Canvas/PowersSelectScreen/SafeArea/Selections/LayoutGroup/ScrollGroup/Viewport/Powers";
const POWER_SIDE:&str="Canvas/PowersSelectScreen/SafeArea/Selections/LayoutGroup/SideInfo";
const POWER_TOP:&str="Canvas/PowersSelectScreen/SafeArea/Selections/LayoutGroup/SideInfo/TopContainer";
struct PageAnimation {data:serde_json::Value,controller:&'static str,clip:&'static str,began:Option<f64>}
const TROPHY_BASE:&str="Canvas/TrophyStoreScreen/Container";
const TROPHY_GRID:&str="Canvas/TrophyStoreScreen/Container/Panel/Scroll/LayoutGroup/Viewport/Content";
/// TrophyStoreScreen state. tab: TrophyStoreFilter (0-4 categories, 5 Featured, 6 Owned), 7 = Favourites.
struct TrophyState {
    inventory:bool,tab:i64,live:Option<String>,account:Option<f64>,cards:Vec<TrophyCard>,day:String,days:String,
    root:std::path::PathBuf,
}
/// TrophyItemBinding / TrophyItemPanel flags for one catalogue entry.
#[derive(Clone,Default)]
struct TrophyCard {
    visible:bool,purchased:bool,enabled:bool,favorite:bool,can_enable:bool,featured:bool,on_sale:bool,
    is_new:bool,limited_end:Option<u64>,quest:bool,nexus:bool,price:i64,base:i64,
}
const TROPHY_TAB_IDS:[(&str,i64);8]=[("Featured",5),("All",6),("Heroes",1),("Monkeys",0),("Bloons",2),("Co-Op",3),("GameAndUI",4),("Favourites",7)];
const TROPHY_TABS:[(&str,Option<i64>);6]=[("All",None),("Heroes",Some(1)),("Monkeys",Some(0)),
    ("Bloons",Some(2)),("Co-Op",Some(3)),("GameAndUI",Some(4))];
struct ScrollRegion {
    view: String, content: String, origin: Vec2, axes: Vec2, offset: Vec2, velocity: Vec2,
    sensitivity: f32, drag: Option<Vec2>,
}
impl Page {
    pub(super) fn coop_settings(&self,map:String)->rrbtd6_coop::Settings{self.coop.as_ref().map(|c|c.settings(map.clone())).unwrap_or(rrbtd6_coop::Settings{map,..Default::default()})}
    pub(super) fn coop_changed(&mut self)->bool{self.coop.as_mut().is_some_and(|c|c.take_changed())}
    pub(super) fn bind_coop(&mut self,session:&super::coop_session::Session){if let Some(coop)=&mut self.coop{coop.bind_session(&mut self.node,session);}}
    pub(super) fn join_text(&self)->Option<&str>{self.join_code.as_ref().map(|c|c.text.as_str())}
    pub(super) fn take_join_submit(&mut self)->bool{self.join_code.as_mut().is_some_and(|c|std::mem::take(&mut c.submit))}
    pub(super) fn coop_modal_open(&self)->bool {self.coop.as_ref().and_then(|c|c.modal_path()).is_some()}
    pub(super) fn dismiss_search(&mut self)->bool {
        let stats="Canvas/DailyChallengeScreen/Content/StatsPopup";
        if self.event.is_some()&&self.node.find(stats).is_some_and(|n|n.active){self.node.set_active(stats,false);return true;}

        if self.coop.as_mut().is_some_and(|p|p.dismiss(&mut self.node)){return true;}
        if self.profile.as_mut().is_some_and(|p|p.dismiss(&mut self.node)){return true;}
        if self.social.as_mut().is_some_and(|s|s.dismiss(&mut self.node)){return true;}

        if let Some(browser)=&self.legends {
            if browser.about {let selected=browser.selected;self.show_legend(selected,false);return true;}
        }
        if let Some(code)=&mut self.join_code {if code.focused {code.focused=false;return true;}}
        let Some(search)=&mut self.search else {return false};
        let focused=search.focused;search.focused=false;focused
    }
    pub(super) fn warm(&mut self,sprites:&mut crate::ui::Sprites){if self.first_presentation{super::button_motion::warm_visible(&self.node,sprites);}}
    pub(super) fn presented(&mut self){if let Some(profile)=&mut self.profile{profile.presented();}if self.first_presentation{self.first_presentation=false;if let Some(animation)=&mut self.animation{animation.began=Some(get_time());}}}
    pub(super) fn reopen(&mut self) {
        self.first_presentation=true;
        self.buttons.reset();
        if let Some(animation)=&mut self.animation {animation.began=Some(get_time());}
    }
    pub(super) fn animate(&mut self) {
        self.buttons.update(&mut self.node);
        if let Some(goals)=&mut self.goals{if goals.bind(&mut self.node){
            self.scrolls=goals.regions.iter().filter_map(|(view,content)|Some(ScrollRegion{view:view.clone(),content:content.clone(),origin:self.node.find(content)?.rt.pos,
                axes:vec2(0.0,1.0),offset:Vec2::ZERO,velocity:Vec2::ZERO,sensitivity:150.0,drag:None})).collect();
        }}
        if let Some(event)=&mut self.event{event.bind(&mut self.node);}
        if let Some(profile)=&mut self.profile{profile.animate(&mut self.node);}
        if let Some(social)=&mut self.social{social.animate(&mut self.node);}

        if let Some(animation)=&mut self.animation {
            if let Some(began)=animation.began {
                if apply_animation(&mut self.node,&animation.data,animation.controller,animation.clip,Some(if self.first_presentation{0.0}else{(get_time()-began) as f32})) {
                    animation.began=None;
                }
            }
        }
        if let Some(browser)=&mut self.legends {
            if let Some(animation)=&mut browser.animation {
                if let Some(began)=animation.began {
                    if apply_animation(&mut self.node,&animation.data,animation.controller,animation.clip,Some((get_time()-began) as f32)) {animation.began=None;}
                }
            }
        }
        if let Some(browser)=&mut self.knowledge {
            if let Some(animation)=&mut browser.panel_animation {
                if let Some(began)=animation.began {
                    if apply_animation(&mut self.node,&animation.data,animation.controller,animation.clip,Some((get_time()-began) as f32)) {
                        animation.began=None;
                        if animation.clip=="SelectedPanelClose" {self.node.set_active(KNOWLEDGE_PANEL,false);}
                    }
                }
            }
        }
        self.refresh_knowledge();
        self.refresh_trophies();
        if self.powers.as_ref().is_some_and(|b|!b.bound) && super::account_preview::power_account().is_some() {self.bind_powers();}
        if let Some(browser)=&self.powers {
            // Live launch packs have no image/product bound in an offline catalog.
            self.node.set_active("Canvas/PowersSelectScreen/SafeArea/IapPackButton",false);
            if let Some(Some(portrait))=browser.portraits.get(&browser.selected) {
                if let Some(container)=self.node.find_mut(POWER_TOP) {
                    for animator in portrait.runtime["animators"].as_array().into_iter().flatten() {
                        if let Some(clip)=animator["clips"].as_array().and_then(|c|c.first()) {
                            let duration=(clip["stop"].as_f64().unwrap_or(0.0)-clip["start"].as_f64().unwrap_or(0.0)).max(0.001);
                            let elapsed=get_time()-browser.began;
                            let elapsed=if clip["loop"].as_bool().unwrap_or(false){elapsed.rem_euclid(duration)}else{elapsed.min(duration)};
                            apply_animation(container,&portrait.runtime,animator["controller"].as_str().unwrap_or(""),
                                clip["name"].as_str().unwrap_or(""),Some(elapsed as f32));
                        }
                    }
                }
            }
        }
    }
    pub(super) fn update_scroll(&mut self, input: ScrollInput) {
        if let Some(social)=&mut self.social{social.input(&mut self.node,&input);}

        if let Some(code)=&mut self.join_code {
            if input.pressed {code.focused=self.node.rect(JOIN_INPUT).is_some_and(|r|r.contains(input.pointer));}
            if code.focused {
                while let Some(c)=get_char_pressed(){if !c.is_control() && code.text.chars().count()<code.limit {code.text.push(c);}}
                if is_key_pressed(KeyCode::Backspace){code.text.pop();}
                if (is_key_down(KeyCode::LeftControl)||is_key_down(KeyCode::RightControl))&&is_key_pressed(KeyCode::V){
                    if let Some(text)=macroquad::miniquad::window::clipboard_get(){code.text=text.trim().chars().filter(|c|!c.is_control()).take(code.limit).collect();}
                }
                if is_key_pressed(KeyCode::Enter){code.focused=false;code.submit=true;}
                self.node.set_text(&format!("{JOIN_INPUT}/Text Area/Text"),&code.text);
            }
        }
        let mut search_changed=false;
        if let Some(search)=&mut self.search {
            if input.pressed {search.focused=self.node.rect(ACHIEVEMENT_SEARCH).is_some_and(|r|r.contains(input.pointer));}
            if search.focused {
                while let Some(c)=get_char_pressed() {
                    if !c.is_control() && search.query.len()<256 {search.query.push(c);search_changed=true;}
                }
                if is_key_pressed(KeyCode::Backspace) {search_changed|=search.query.pop().is_some();}
                if is_key_pressed(KeyCode::Enter) {search.focused=false;}
            }
        }
        if search_changed {self.filter_achievements();}
        if input.pressed {self.pointer_start=Some(input.pointer);self.pointer_dragged=false;}
        if self.pointer_start.is_some_and(|p|p.distance(input.pointer)>5.0) {self.pointer_dragged=true;}
        let mut taken = false;
        for scroll in &mut self.scrolls {
            if self.coop.as_ref().and_then(|c|c.modal_path()).is_some_and(|p|!scroll.view.starts_with(&p)){scroll.drag=None;scroll.velocity=Vec2::ZERO;continue;}
            let stats="Canvas/DailyChallengeScreen/Content/StatsPopup";
            if self.event.is_some()&&self.node.find(stats).is_some_and(|n|n.active)&&!scroll.view.starts_with(stats){scroll.drag=None;scroll.velocity=Vec2::ZERO;continue;}
            if self.profile.as_ref().and_then(|p|p.modal()).is_some_and(|p|!scroll.view.starts_with(p)){scroll.drag=None;scroll.velocity=Vec2::ZERO;continue;}
            if self.social.as_ref().and_then(|s|s.modal_path()).is_some_and(|p|!scroll.view.starts_with(p)){scroll.drag=None;scroll.velocity=Vec2::ZERO;continue;}
            let Some(view) = self.node.rect(&scroll.view).filter(|v| v.w>0.0 && v.h>0.0) else { continue };
            let Some(content) = self.node.find(&scroll.content) else { continue };
            let limit = ((content.unclipped_screen.size()-view.size())/input.scale).max(Vec2::ZERO);
            let inside = view.contains(input.pointer);
            if scroll.drag.is_some() || (!taken && inside) {
                taken = true;
                if input.pressed { scroll.drag=Some(input.pointer); scroll.velocity=Vec2::ZERO; }
                if let Some(last) = scroll.drag {
                    let delta=(last-input.pointer)/input.scale*scroll.axes;
                    scroll.offset+=delta;
                    let dt=input.dt.clamp(0.0001,0.1);
                    scroll.velocity+=(delta/dt-scroll.velocity)*(dt*10.0).min(1.0);
                    scroll.drag=if input.held {Some(input.pointer)} else {None};
                } else if input.wheel!=0.0 {
                    let axis=if scroll.axes.y>0.0 {vec2(0.0,1.0)} else {vec2(1.0,0.0)};
                    scroll.offset-=axis*input.wheel*scroll.sensitivity;
                    scroll.velocity=Vec2::ZERO;
                }
            }
            if scroll.drag.is_none() {
                scroll.velocity*=0.135_f32.powf(input.dt.clamp(0.0,0.1));
                scroll.offset+=scroll.velocity*input.dt.clamp(0.0,0.1);
            }
            let bounded=scroll.offset.clamp(Vec2::ZERO,limit)*scroll.axes;
            if bounded.x!=scroll.offset.x { scroll.velocity.x=0.0; }
            if bounded.y!=scroll.offset.y { scroll.velocity.y=0.0; }
            scroll.offset=bounded;
            if let Some(content)=self.node.find_mut(&scroll.content) {
                content.rt.pos=scroll.origin+vec2(-bounded.x,bounded.y);
            }
        }
        if input.released {
            let clicked=self.pointer_start.take().is_some() && !self.pointer_dragged;
            if clicked && self.jukebox.as_mut().is_some_and(|j|j.click(&mut self.node,input.pointer)){return;}
            if clicked && self.profile.as_mut().is_some_and(|p|p.click(&mut self.node,input.pointer)){
                if self.profile.as_mut().is_some_and(|p|std::mem::take(&mut p.stats_changed)){
                    for scroll in &mut self.scrolls{if scroll.content.contains("/OverallStats/"){
                        scroll.offset=Vec2::ZERO;scroll.velocity=Vec2::ZERO;scroll.drag=None;
                        if let Some(content)=self.node.find_mut(&scroll.content){content.rt.pos=scroll.origin;}
                    }}
                }
                return;
            }
            if clicked {
                if self.social.as_mut().is_some_and(|s|s.click(&mut self.node,input.pointer)) {
                    for scroll in &mut self.scrolls {scroll.offset=Vec2::ZERO;scroll.velocity=Vec2::ZERO;scroll.drag=None;
                        if let Some(content)=self.node.find_mut(&scroll.content){content.rt.pos=scroll.origin;}}
                }
            }
            if clicked && self.legends.is_some() {self.click_legends(input.pointer);}
            if clicked && !self.trophy_items.is_empty() {self.click_trophies(input.pointer);}
            if clicked {if let Some(coop)=&mut self.coop{coop.click(&mut self.node,input.pointer);}}
            if clicked {if let Some(goals)=&self.goals{goals.click(&mut self.node,input.pointer);}}
            if clicked && self.powers.is_some() {
                if let Some(tab)=[("AllTab",0),("Powers",1),("Pro",2)].iter().find(|(name,_)|
                    self.node.rect(&format!("{POWER_TABS}/{name}")).is_some_and(|r|r.contains(input.pointer))).map(|t|t.1) {
                    self.filter_powers(tab);
                }
                let count=self.powers.as_ref().unwrap().items.len();
                if let Some(index)=(0..count).find(|i|self.node.rect(&format!("{POWER_GRID}/Power{i}")).is_some_and(|r|r.contains(input.pointer))) {
                    self.select_power(index);
                }
            }
            if clicked && self.knowledge.is_some() {
                let index=self.knowledge.as_ref().unwrap().items.iter().position(|item|
                    item["path"].as_str().and_then(|p|self.node.rect(p)).is_some_and(|r|r.contains(input.pointer)));
                if index.is_some() || !self.node.rect(KNOWLEDGE_PANEL).is_some_and(|r|r.contains(input.pointer)) {
                    self.select_knowledge(index);
                }
            }
        }
    }

    fn show_legend(&mut self,index:usize,about:bool) {
        let Some(browser)=&mut self.legends else {return};
        let Some(slides)=browser.data["slides"].as_array() else {return};
        if slides.is_empty(){return;}
        browser.selected=index%slides.len();browser.about=about;
        for (i,path) in slides.iter().enumerate() {
            if let Some(path)=path.as_str(){self.node.set_active(path,!about && i==browser.selected);}
            self.node.set_active(&format!("Canvas/LegendsSelectScreen/PagePips/Pip{i}/On"),i==browser.selected);
        }
        if let Some(path)=browser.data["infoPanel"].as_str(){self.node.set_active(path,about);}
        self.node.set_active("Canvas/LegendsSelectScreen/ArrowButtons",!about);
        self.node.set_active("Canvas/LegendsSelectScreen/PagePips",!about);
        let path=if about {browser.data["infoPanel"].as_str()} else {slides[browser.selected].as_str()};
        let animators:Vec<_>=browser.runtime["animators"].as_array().into_iter().flatten()
            .filter(|a|a["controller"]=="PageTransition" && a["path"].as_str()==path).cloned().collect();
        browser.animation=Some(PageAnimation {data:serde_json::json!({"animators":animators}),controller:"PageTransition",clip:"PageTransition",began:Some(get_time())});
    }
    fn click_legends(&mut self,pointer:Vec2) {
        let Some(browser)=&self.legends else {return};
        let hit=|key:&str|browser.data[key].as_str().and_then(|p|self.node.rect(p)).is_some_and(|r|r.contains(pointer));
        let count=browser.data["slides"].as_array().map_or(0,Vec::len);
        if count==0{return;}
        let selected=browser.selected;
        if !browser.about && selected==0 && self.node.rect("Canvas/LegendsSelectScreen/SlideSubPanels/Home/Buttons/HomeButton").is_some_and(|r|r.contains(pointer)) {self.home_requested=true;return;}
        if browser.about {if hit("infoClose"){self.show_legend(selected,false);}return;}
        if hit("infoButton"){self.show_legend(selected,true);}
        else if hit("prevBtn"){self.show_legend((selected+count-1)%count,false);}
        else if hit("nextBtn"){self.show_legend((selected+1)%count,false);}
        else if let Some(index)=(0..count).find(|i|self.node.rect(&format!("Canvas/LegendsSelectScreen/PagePips/Pip{i}")).is_some_and(|r|r.contains(pointer))) {self.show_legend(index,false);}
    }

    pub(super) fn sync_knowledge_category(&mut self,category:usize) {
        if self.knowledge.as_ref().is_some_and(|k|k.selected.is_some_and(|i|k.items[i]["category"].as_u64()!=Some(category as u64))) {
            self.select_knowledge(None);
        }
    }

    /// KnowledgeSkillTree.UpdateButtonStates whenever the account changes, then each frame
    /// KnowledgeSkillBtn.Update's arrow fade and the available nodes' pulse.
    fn refresh_knowledge(&mut self) {
        let Some(browser)=&mut self.knowledge else {return};
        let node=&mut self.node;
        let now=get_time();
        let account=super::account_preview::knowledge_profile();
        let stamp=account.as_ref().map(|a|a.3);
        if browser.states.iter().all(|&s|s==0) || stamp!=browser.account {
            browser.account=stamp;
            let (owned,disabled,points)=account.map(|(o,d,p,_)|(o.into_iter().collect(),d.into_iter().collect(),p)).unwrap_or_default();
            browser.owned=owned;browser.disabled=disabled;browser.points=points;
            for i in 0..browser.items.len() {
                let state=knowledge_state(browser,i);
                let previous=browser.states[i];
                if state==previous {continue;}
                browser.states[i]=state;
                let item=&browser.items[i];
                let selected=browser.selected==Some(i);
                // SetState: selectedEffect / availableEffect, icon material, arrows, strikethrough.
                set_knowledge_selected(node,item,selected,state);
                if let Some(icon)=item["path"].as_str().and_then(|p|node.find_mut(&format!("{p}/Icon"))).and_then(|n|n.img.as_mut()) {
                    icon.gray=state!=3;
                }
                let locked=crate::ui::color(&item["lockedArrowColor"]).unwrap_or(WHITE);
                let purchased=crate::ui::color(&item["purchasedArrowColor"]).unwrap_or(WHITE);
                let arrows=match state {
                    1|2=>Some(locked),
                    3 if previous==0||previous==4=>{browser.arrow_fade[i]=None;Some(purchased)}
                    3=>{browser.arrow_fade[i]=Some(now);Some(locked)}
                    _=>None,
                };
                if let Some(color)=arrows {set_arrow_color(node,item,color);}
                if let Some(path)=item["path"].as_str(){node.set_active(&format!("{path}/StrikethroughBg"),state==4);}
            }
            let pulses:std::collections::HashSet<String>=browser.items.iter().zip(&browser.states).filter(|(_,s)|**s==1)
                .filter_map(|(item,_)|item["path"].as_str().map(|p|format!("{p}/Highlight Pulse"))).collect();
            let animators:Vec<_>=browser.runtime["animators"].as_array().into_iter().flatten()
                .filter(|a|a["controller"]=="CircleVfxAnimation"&&a["path"].as_str().is_some_and(|p|pulses.contains(p))).cloned().collect();
            browser.pulse=serde_json::json!({"animators":animators});
            if let Some(i)=browser.selected {fill_knowledge_panel(node,browser,i);}
        }
        for (i,fade) in browser.arrow_fade.iter_mut().enumerate() {
            let Some(began)=*fade else {continue};
            let item=&browser.items[i];
            let length=item["arrowFade"].as_f64().unwrap_or(0.0).max(1e-6);
            let f=((now-began)/length).min(1.0) as f32;
            let a=crate::ui::color(&item["lockedArrowColor"]).unwrap_or(WHITE);
            let b=crate::ui::color(&item["purchasedArrowColor"]).unwrap_or(WHITE);
            set_arrow_color(node,item,Color::new(a.r*(1.0-f)+b.r*f,a.g*(1.0-f)+b.g*f,a.b*(1.0-f)+b.b*f,a.a*(1.0-f)+b.a*f));
            if f>=1.0 {*fade=None;}
        }
        // CircleVfxAnimation "pulse" (1 s) loops on each available node.
        apply_animation(node,&browser.pulse,"CircleVfxAnimation","pulse",Some((now%1.0) as f32));
    }

    fn select_knowledge(&mut self,index:Option<usize>) {
        let Some(browser)=&mut self.knowledge else {return};
        let previous=browser.selected;
        if let Some(previous)=browser.selected {
            set_knowledge_selected(&mut self.node,&browser.items[previous],false,browser.states[previous]);
        }
        browser.selected=index;
        if index.is_none() {
            if previous.is_some() {
                if let Some(animation)=&mut browser.panel_animation {
                    animation.clip="SelectedPanelClose";animation.began=Some(get_time());
                }else{self.node.set_active(KNOWLEDGE_PANEL,false);}
            }
            return;
        }
        self.node.set_active(KNOWLEDGE_PANEL,true);
        let Some(i)=index.filter(|&i|i<browser.items.len()) else {return};
        set_knowledge_selected(&mut self.node,&browser.items[i],true,browser.states[i]);
        fill_knowledge_panel(&mut self.node,browser,i);
        if let Some(animation)=&mut browser.panel_animation {animation.clip="SelectedPanelOpen";animation.began=Some(get_time());}
    }

    /// PowersSelectScreen.SetupPowerButtons / PowerSelectButton.UpdatePowerDisplay(false) with the
    /// account preview, then ShowScreen's first-button selection.
    fn bind_powers(&mut self) {
        let Some(browser)=&mut self.powers else {return};
        let account=super::account_preview::power_account();
        browser.bound=account.is_some();
        browser.seen=account.as_ref().map(|a|a.seen.clone()).unwrap_or_default();
        browser.pro=account.as_ref().map(|a|a.pro.clone()).unwrap_or_default();
        let mut first=None;
        for i in 0..browser.items.len() {
            let item=&browser.items[i];
            let id=item["id"].as_str().unwrap_or("");
            // GameModel.GetModdedPowers: enabled, owned Monkey Knowledge lowers the cost (PowerModModel.Mutate).
            let mut cost=item["cost"].as_i64().unwrap_or(0);
            if let Some(a)=account.as_ref().filter(|a|!a.knowledge_disabled) {
                for (knowledge,decrease) in item["discounts"].as_object().into_iter().flatten() {
                    if a.knowledge.contains(knowledge) {cost-=decrease.as_f64().unwrap_or(0.0) as i64;}
                }
            }
            let data=account.as_ref().and_then(|a|a.data.get(id)).copied();
            // SkuSettings.CanShowPower; offline there is no store, so IsPowerIapInStore is false.
            let achievement=item["achievement"].as_i64().unwrap_or(0);
            let store=item["store"].as_str().unwrap_or("");
            let shown=item["hidden"]!=true
                && (achievement==0 || account.as_ref().is_some_and(|a|a.claimed.contains(&achievement)))
                && (cost>0 || store.is_empty() || data.is_some());
            browser.costs[i]=cost;
            browser.shown[i]=shown;
            browser.quantities[i]=data.map(|d|d.0 as i64);
            let quantity=data.map(|d|d.0 as i64).unwrap_or(0);
            let path=format!("{POWER_GRID}/Power{i}");
            self.node.set_active(&path,shown);
            self.node.set_active(&format!("{path}/PowerCountIcon"),quantity>0);
            self.node.set_text(&format!("{path}/PowerCountIcon/Count"),&quantity.to_string());
            self.node.set_active(&format!("{path}/Notify"),data.is_some_and(|d|d.1));
            self.node.set_active(&format!("{path}/SelectedImage"),false);
            self.node.set_active(&format!("{path}/NewRibbon"),browser.ribbon(i));
            if shown && first.is_none() {first=Some(i);}
        }
        let preview=std::env::var("RRBTD6_POWER_PREVIEW").ok().and_then(|name|browser.items.iter().position(|p|p["id"].as_str()==Some(&name)));
        // Open: the All filter toggle is on, the others off.
        self.filter_powers(std::env::var("RRBTD6_POWER_TAB").ok().and_then(|t|t.parse().ok()).unwrap_or(0));
        if let Some(index)=preview.or(first) {self.select_power(index);}
    }

    /// PowersSelectScreen.OnFilterAll / OnFilterPower / OnFilterPowerPro: every created button,
    /// only non-Pro ones, or only Pro ones; the selection is left as it is.
    fn filter_powers(&mut self,filter:usize) {
        let Some(browser)=&self.powers else {return};
        for (name,tab) in [("AllTab",0),("Powers",1),("Pro",2)] {
            self.node.set_active(&format!("{POWER_TABS}/{name}/BgSelected"),tab==filter);
        }
        let visible:Vec<bool>=browser.items.iter().zip(&browser.shown).map(|(item,&shown)|{
            let pro=item["pro"]==true;
            shown && match filter {1=>!pro,2=>pro,_=>true}
        }).collect();
        for (i,on) in visible.into_iter().enumerate() {self.node.set_active(&format!("{POWER_GRID}/Power{i}"),on);}
    }

    /// The CommonBackgroundScreen colour tint PowersSelectScreen.UpdateTheme animates.
    pub(super) fn background_tint(&self)->Option<Color> {self.powers.as_ref().map(PowerBrowser::tint_now)}

    fn select_power(&mut self,index:usize) {
        let Some(browser)=&mut self.powers else {return};
        let Some(item)=browser.items.get(index).cloned() else {return};
        self.node.set_active(&format!("{POWER_GRID}/Power{}/SelectedImage",browser.selected),false);
        self.node.set_active(&format!("{POWER_GRID}/Power{index}/SelectedImage"),true);
        browser.selected=index;browser.began=get_time();
        let id=browser.id(index).to_owned();
        // PowerSelectButton.SetSelected: selecting marks the power seen, clearing its NEW ribbon.
        browser.seen.insert(id.clone());
        self.node.set_active(&format!("{POWER_GRID}/Power{index}/NewRibbon"),browser.ribbon(index));
        let pro=item["pro"]==true;
        let store=item["store"].as_str().is_some_and(|s|!s.is_empty());
        let info=format!("{POWER_SIDE}/Info");
        // PowersSelectScreen.UpdateTheme.
        let theme=if pro {1} else if store {2} else {0};
        for (i,(_,_,objects)) in POWER_THEMES.iter().enumerate() {
            if i!=theme {for name in *objects {self.node.set_active(&format!("{info}/{name}"),false);}}
        }
        let (bg,text,objects)=POWER_THEMES[theme];
        for name in objects {self.node.set_active(&format!("{info}/{name}"),true);}
        self.node.set_text_color(&format!("{info}/Description"),Color::new(text[0],text[1],text[2],1.0));
        let current=browser.tint_now();
        browser.tint=(current,Color::new(bg[0],bg[1],bg[2],1.0),get_time());
        // PowersSelectScreen.DisplayPower.
        let cost=browser.costs[index];
        let quantity=browser.quantities[index];
        self.node.set_text(&format!("{info}/Title"),item["title"].as_str().unwrap_or(""));
        self.node.set_text(&format!("{info}/Description"),item["description"].as_str().unwrap_or(""));
        self.node.set_text(&format!("{info}/AddButton/CostContainer/TextCount"),&super::account_preview::number(cost as f64));
        self.node.set_active(&format!("{info}/AddButton/CostContainer"),cost>0);
        self.node.set_active(&format!("{info}/AddButton"),cost>0);
        self.node.set_active(&format!("{info}/IapStoreButton"),false);
        self.node.set_active(&format!("{info}/Count"),quantity.is_some_and(|q|q!=0));
        self.node.set_text(&format!("{info}/Count/Count"),&quantity.unwrap_or(0).to_string());
        let linked=item["linked"].as_str().unwrap_or("").to_owned();
        let xp_text=|xp:i64,cost:i64|format!("XP: {}/{}",super::account_preview::number(xp as f64),super::account_preview::number(cost as f64));
        let base_item=browser.items.iter().find(|i|i["id"].as_str()==Some(&linked)).cloned();
        if pro {
            // UpdateProXpDisplays: a locked Pro power shows how to unlock it from its base power.
            let (xp,cost)=browser.pro_xp(&id);
            let locked=format!("{info}/ProLockedMessage/LockedObject");
            self.node.set_text(&format!("{locked}/Description"),&format!("Unlock Pro with: {}",
                base_item.as_ref().and_then(|b|b["title"].as_str()).unwrap_or("")));
            if let Some(bar)=self.node.find_mut(&format!("{locked}/Fillbar/Bar")).and_then(|n|n.img.as_mut()) {bar.fill_amount=xp as f32/cost as f32;}
            self.node.set_active(&format!("{locked}/Fillbar/XpText"),true);
            self.node.set_text(&format!("{locked}/Fillbar/XpText"),&xp_text(xp,cost));
            self.node.set_sprite(&format!("{locked}/Power/Icon"),base_item.as_ref().and_then(|b|b["icon"].as_str()).and_then(|s|s.strip_suffix(".png")));
            let unlocked=browser.pro_unlocked(&id);
            self.node.set_active(&format!("{info}/AddButton"),unlocked);
            self.node.set_active(&format!("{info}/ProLockedMessage"),!unlocked);
            self.node.set_active(&format!("{info}/ProUpgradesButton/NewRibbon"),false);
            self.node.set_active(&format!("{info}/LockedTiersBar"),false);
        } else {
            // UpdateLinkedProXpDisplays: base powers never show the Pro lock or next-tier bar here.
            if !linked.is_empty() {self.node.set_active(&format!("{info}/AddButton"),true);}
            self.node.set_active(&format!("{info}/ProLockedMessage"),false);
            self.node.set_active(&format!("{info}/ProUpgradesButton/NewRibbon"),false);
            self.node.set_active(&format!("{info}/LockedTiersBar"),false);
        }
        // UpdateShortProXpDisplay: a base power whose Pro version is still locked shows its XP.
        let short=!pro && !linked.is_empty() && !browser.pro_unlocked(&linked);
        self.node.set_active(&format!("{info}/ProUnlockRegular"),short);
        if short {
            let (xp,cost)=browser.pro_xp(&linked);
            let bar=format!("{info}/ProUnlockRegular/LockedObject/Fillbar");
            if let Some(fill)=self.node.find_mut(&format!("{bar}/Bar")).and_then(|n|n.img.as_mut()) {fill.fill_amount=xp as f32/cost as f32;}
            self.node.set_text(&format!("{bar}/XpText"),&xp_text(xp,cost));
        }
        let item=&item;
        if !browser.portraits.contains_key(&index) {
            let portrait=item["animated_icon"].as_str().and_then(|file|file.strip_suffix(".prefab")).and_then(|name| {
                Some(PowerPortrait {node:Node::load(&browser.dir.join(format!("{name}.json")))?,
                    runtime:serde_json::from_str(&std::fs::read_to_string(browser.runtime_dir.join(format!("{name}.json"))).ok()?).ok()?})
            });
            browser.portraits.insert(index,portrait);
        }
        let portrait=browser.portraits.get(&index).and_then(|p|p.as_ref());
        if let Some(container)=self.node.find_mut(POWER_TOP) {
            container.children=portrait.map(|p|vec![p.node.clone()]).unwrap_or_default();
        }
        self.node.set_active(&format!("{info}/Icon"),portrait.is_none());
        self.node.set_sprite(&format!("{info}/Icon"),item["icon"].as_str().and_then(|s|s.strip_suffix(".png")));
    }

    fn filter_achievements(&mut self) {
        let Some(search)=&self.search else {return};
        let query=search.query.trim().to_lowercase();
        let mut count=0;
        if let Some(content)=self.node.find_mut(ACHIEVEMENT_GRID) {
            for (card,term) in content.children.iter_mut().zip(&search.terms) {
                card.active=query.is_empty() || term.contains(&query);
                count+=usize::from(card.active);
            }
        }
        self.node.set_text(&format!("{ACHIEVEMENT_SEARCH}/Text Area/SearchText"),&search.query);
        self.node.set_active(&format!("{ACHIEVEMENT_SEARCH}/Text Area/SearchPlaceholder"),search.query.is_empty());
        self.node.set_active("Canvas/AchievementsScreen/Scroll/FoundNothingTxt",count==0);
        for scroll in &mut self.scrolls {
            if scroll.content==ACHIEVEMENT_GRID {
                scroll.offset=Vec2::ZERO;scroll.velocity=Vec2::ZERO;scroll.drag=None;
                if let Some(content)=self.node.find_mut(ACHIEVEMENT_GRID){content.rt.pos=scroll.origin;}
            }
        }
    }

    fn select_trophy(&mut self,index:usize) {
        let Some(item)=self.trophy_items.get(index) else {return};
        if let Some(previous)=self.trophy_selected {
            self.node.set_active(&format!("{TROPHY_GRID}/Trophy{previous}/Selected"),false);
        }
        self.trophy_selected=Some(index);
        self.node.set_active(&format!("{TROPHY_GRID}/Trophy{index}/Selected"),true);
        let base=format!("{TROPHY_BASE}/BottomInfo");
        self.node.set_sprite(&format!("{base}/Icon"),item["icon"].as_str().and_then(|s|s.strip_suffix(".png")));
        self.node.set_text(&format!("{base}/Title"),item["title"].as_str().unwrap_or(""));
        self.node.set_text(&format!("{base}/Description"),item["description"].as_str().unwrap_or(""));
        // TrophyStoreScreen.RefreshBottomBar.
        if let Some(card)=self.trophy.as_ref().and_then(|t|t.cards.get(index)).cloned() {
            let owned=card.purchased;
            self.node.set_active(&base,true);
            self.node.set_active(&format!("{base}/AddButton"),!owned&&!card.quest&&!card.nexus);
            self.node.set_text(&format!("{base}/AddButton/TextCount"),&card.price.to_string());
            self.node.set_active(&format!("{base}/QuestButton"),!owned&&card.quest);
            self.node.set_active(&format!("{base}/PurchasedGroup"),true);
            self.node.set_active(&format!("{base}/PurchasedGroup/EnableCheck"),owned&&card.can_enable);
            self.node.set_active(&format!("{base}/PurchasedGroup/EnableCheck/Tick"),card.enabled);
            self.node.set_active(&format!("{base}/PurchasedGroup/FavouriteCheck"),owned);
            self.node.set_active(&format!("{base}/PurchasedGroup/FavouriteCheck/Tick"),card.favorite);
            self.node.set_active(&format!("{base}/PurchasedGroup/BoughtObj"),owned&&!card.can_enable);
        }
    }

    /// TrophyStoreScreen.InitTrophyStore / InstantiateItem / SortPanels whenever the live store
    /// or the account changes, and each frame TrophyItemPanel.Update's limited-time countdown.
    fn refresh_trophies(&mut self) {
        let Some(state)=&mut self.trophy else {return};
        let live=super::home_events::trophy_live(&state.root);
        let account=super::account_preview::trophy_account();
        let live_key=live.as_ref().map(|l|l["checkedAt"].to_string());
        let account_key=account.as_ref().map(|a|a.1);
        let now=std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_millis() as u64;
        if state.cards.is_empty() || live_key!=state.live || account_key!=state.account {
            state.live=live_key;state.account=account_key;
            let live=live.unwrap_or_default();
            let ids=|v:&serde_json::Value|v.as_array().into_iter().flatten().filter_map(|s|s.as_str().map(str::to_owned)).collect::<std::collections::HashSet<_>>();
            let hidden=ids(&live["sku"]["hiddenItems"]);let new=ids(&live["sku"]["newItems"]);
            let mut featured=std::collections::HashSet::new();let mut on_sale=std::collections::HashSet::new();
            let mut limited:HashMap<String,u64>=HashMap::new();
            for event in live["events"].as_array().into_iter().flatten() {
                featured.extend(ids(&event["metadata"]["featured"]));on_sale.extend(ids(&event["metadata"]["onSale"]));
                for id in ids(&event["metadata"]["enabled"]) {limited.insert(id,event["end"].as_u64().unwrap_or(0));}
            }
            let rows=account.map(|a|a.0).unwrap_or_default();
            state.cards=self.trophy_items.iter().enumerate().map(|(i,item)| {
                let raw=&item["raw"];
                let original=item["id"].as_str().unwrap_or("").to_owned();
                // TrophyStoreItem.get_Id: walletIdOverride when set.
                let id=raw["walletIdOverride"].as_str().filter(|s|!s.is_empty()).map(str::to_owned).unwrap_or(original.clone());
                let has=|set:&std::collections::HashSet<String>|set.contains(&id)||set.contains(&original);
                let [purchased,enabled,favorite,can_enable]=rows.get(i).copied().unwrap_or([false,false,false,true]);
                let limited_end=limited.get(&id).or_else(||limited.get(&original)).copied();
                let price=|key:&str|live["prices"][key].as_i64().unwrap_or(9999);
                let base=price(&id);let sale=price(&format!("{id}_sale"));
                let on_sale=has(&on_sale);
                let loot=raw["lootSetFilter"].as_i64().unwrap_or(0);
                // InstantiateItem.isVisible (IsIapStoreItem needs item-target data the export lacks).
                let visible=purchased||(raw["hideUnlessUnlocked"]!=1&&(limited_end.is_some()||!has(&hidden)));
                TrophyCard {visible,purchased,enabled,favorite,can_enable,featured:has(&featured),on_sale,
                    is_new:has(&new)&&limited_end.is_none(),limited_end:limited_end.filter(|_|!purchased),
                    quest:loot==1,nexus:loot==2,price:if on_sale{sale}else{base},base}
            }).collect();
            let cards=state.cards.clone();
            if let Some(content)=self.node.find_mut(TROPHY_GRID) {
                for (i,card) in cards.iter().enumerate() {
                    let Some(node)=content.children.iter_mut().find(|c|c.name==format!("Trophy{i}")) else {continue};
                    // TrophyItemPanel.Bind / RefreshDisplay.
                    node.set_active("StandardFrame",true);
                    node.set_active("BoughtFrame",card.purchased);
                    node.set_active("Purchased",card.purchased);
                    node.set_active("NotPurchasedGroup",!card.purchased);
                    node.set_active("Purchased/EnabledTick",card.purchased&&card.can_enable&&card.enabled);
                    node.set_active("Purchased/FavouriteToggle/Background/Checkmark",card.favorite);
                    node.set_active("NotPurchasedGroup/Cost",!card.quest&&!card.nexus);
                    node.set_text("NotPurchasedGroup/Cost/TextCount",&card.price.to_string());
                    node.set_text("NotPurchasedGroup/Sale/SaleOrigPrice",&card.base.to_string());
                    node.set_active("NotPurchasedGroup/Sale",card.on_sale);
                    node.set_active("NotPurchasedGroup/NewRibbon",card.is_new);
                    node.set_active("NotPurchasedGroup/QuestLock",card.quest);
                    node.set_active("NotPurchasedGroup/Container",!card.quest&&!card.nexus);
                    node.set_active("NotPurchasedGroup/LimitedTime",card.limited_end.is_some());
                    node.set_active("EmoteIconText",false);
                }
                // SortPanels: owned last, new first; then limited-time (not new), then limited-time new,
                // each moved to the front in catalogue order.
                let mut order:Vec<usize>=(0..cards.len()).collect();
                let front=|order:&mut Vec<usize>,i:usize|{order.retain(|&x|x!=i);order.insert(0,i);};
                for i in 0..cards.len() {
                    if cards[i].purchased {order.retain(|&x|x!=i);order.push(i);}
                    else if cards[i].is_new {front(&mut order,i);}
                }
                for i in 0..cards.len() {if cards[i].limited_end.is_some()&&!cards[i].is_new {front(&mut order,i);}}
                for i in 0..cards.len() {if cards[i].limited_end.is_some()&&cards[i].is_new {front(&mut order,i);}}
                let mut by_name:HashMap<String,Node>=content.children.drain(..).map(|c|(c.name.clone(),c)).collect();
                content.children=order.iter().filter_map(|i|by_name.remove(&format!("Trophy{i}"))).collect();
                content.children.extend(by_name.into_values());
            }
            let (inventory,tab)=(state.inventory,state.tab);
            self.show_trophy_tab(inventory,tab);
        }
        let Some(state)=&self.trophy else {return};
        let (day,days)=(state.day.clone(),state.days.clone());
        let ends:Vec<(usize,u64)>=state.cards.iter().enumerate().filter_map(|(i,c)|c.limited_end.map(|e|(i,e))).collect();
        for (i,end) in ends {
            self.node.set_text(&format!("{TROPHY_GRID}/Trophy{i}/NotPurchasedGroup/LimitedTime/TimeCount"),
                &super::home_events::event_time_remaining(end.saturating_sub(now)/1000,&day,&days));
        }
    }

    /// OnMainToggleChanged (store / inventory) and DisplayTabItems / DisplayFilteredSubs with IsActive.
    fn show_trophy_tab(&mut self,inventory:bool,tab:i64) {
        let Some(state)=&mut self.trophy else {return};
        state.inventory=inventory;state.tab=tab;
        let cards=state.cards.clone();
        let shows=|card:&TrophyCard,category:i64|card.visible&&match tab {
            5=>card.featured,
            6=>card.purchased,
            7=>card.purchased&&card.favorite,
            t=>category==t&&(!inventory||card.purchased),
        };
        let mut first=None;
        if let Some(content)=self.node.find_mut(TROPHY_GRID) {
            for card_node in &mut content.children {
                let Some(i)=card_node.name.strip_prefix("Trophy").and_then(|s|s.parse::<usize>().ok()) else {continue};
                let category=self.trophy_items.get(i).and_then(|t|t["category"].as_i64()).unwrap_or(-1);
                card_node.active=cards.get(i).is_some_and(|c|shows(c,category));
                card_node.set_active("Selected",false);
                if card_node.active&&first.is_none(){first=Some(i);}
            }
        }
        let tabs=format!("{TROPHY_BASE}/Panel/TrophyStoreBg/Tabs");
        self.node.set_active(&format!("{tabs}/Featured"),!inventory);
        self.node.set_active(&format!("{tabs}/All"),inventory);
        for (name,id) in TROPHY_TAB_IDS {self.node.set_active(&format!("{tabs}/{name}/BgSelected"),id==tab);}
        self.node.set_active(&format!("{TROPHY_BASE}/TopInfo/TopTabs/TrophyStoreTab/BgSelected"),!inventory);
        self.node.set_active(&format!("{TROPHY_BASE}/TopInfo/TopTabs/InventoryTab/BgSelected"),inventory);
        self.node.set_active(&format!("{TROPHY_BASE}/Panel/Scroll/FoundNothingTxt"),first.is_none());
        for scroll in &mut self.scrolls {
            if scroll.content==TROPHY_GRID {
                scroll.offset=Vec2::ZERO;scroll.velocity=Vec2::ZERO;scroll.drag=None;
                if let Some(content)=self.node.find_mut(TROPHY_GRID){content.rt.pos=scroll.origin;}
            }
        }
        self.trophy_selected=None;
        self.node.set_active(&format!("{TROPHY_BASE}/BottomInfo"),first.is_some());
        if let Some(first)=first {self.select_trophy(first);}
    }

    fn filter_trophies(&mut self,category:Option<i64>) {
        self.filter_trophies_by(category,false);
    }
    fn filter_trophies_by(&mut self,category:Option<i64>,favorites:bool) {
        let mut first=None;
        if let Some(content)=self.node.find_mut(TROPHY_GRID) {
            for (i,(card,item)) in content.children.iter_mut().zip(&self.trophy_items).enumerate() {
                card.active=if favorites{super::account_preview::trophy_favorite(i)}else{category.is_none() || item["category"].as_i64()==category};
                if card.active && first.is_none(){first=Some(i);}
            }
        }
        for (name,value) in TROPHY_TABS {
            self.node.set_active(&format!("{TROPHY_BASE}/Panel/TrophyStoreBg/Tabs/{name}/BgSelected"),!favorites && value==category);
        }
        for name in ["Featured","Favourites"] {
            self.node.set_active(&format!("{TROPHY_BASE}/Panel/TrophyStoreBg/Tabs/{name}/BgSelected"),favorites && name=="Favourites");
        }
        for scroll in &mut self.scrolls {
            if scroll.content==TROPHY_GRID {
                scroll.offset=Vec2::ZERO;scroll.velocity=Vec2::ZERO;scroll.drag=None;
                if let Some(content)=self.node.find_mut(TROPHY_GRID){content.rt.pos=scroll.origin;}
            }
        }
        self.node.set_active(&format!("{TROPHY_BASE}/BottomInfo"),first.is_some());
        if let Some(first)=first {self.select_trophy(first);}
    }

    fn click_trophies(&mut self,pointer:Vec2) {
        if let Some(state)=&self.trophy {
            let (inventory,tab)=(state.inventory,state.tab);
            let top=format!("{TROPHY_BASE}/TopInfo/TopTabs");
            if self.node.rect(&format!("{top}/TrophyStoreTab")).is_some_and(|r|r.contains(pointer)) {if inventory{self.show_trophy_tab(false,5);}return;}
            if self.node.rect(&format!("{top}/InventoryTab")).is_some_and(|r|r.contains(pointer)) {if !inventory{self.show_trophy_tab(true,6);}return;}
            for (name,id) in TROPHY_TAB_IDS {
                let path=format!("{TROPHY_BASE}/Panel/TrophyStoreBg/Tabs/{name}");
                if self.node.find(&path).is_some_and(|n|n.active)&&self.node.rect(&path).is_some_and(|r|r.contains(pointer)) {
                    if id!=tab {self.show_trophy_tab(inventory,id);}
                    return;
                }
            }
            let clicked=(0..self.trophy_items.len()).find(|i|
                self.node.find(&format!("{TROPHY_GRID}/Trophy{i}")).is_some_and(|n|n.active)&&
                self.node.rect(&format!("{TROPHY_GRID}/Trophy{i}")).is_some_and(|r|r.contains(pointer)));
            if let Some(index)=clicked {self.select_trophy(index);}
            return;
        }
        if self.node.rect(&format!("{TROPHY_BASE}/Panel/TrophyStoreBg/Tabs/Favourites")).is_some_and(|r|r.contains(pointer)) {
            self.filter_trophies_by(None,true);return;
        }
        for (name,category) in TROPHY_TABS {
            if self.node.rect(&format!("{TROPHY_BASE}/Panel/TrophyStoreBg/Tabs/{name}")).is_some_and(|r|r.contains(pointer)) {
                self.filter_trophies(category);return;
            }
        }
        let clicked=(0..self.trophy_items.len()).find(|i|
            self.node.rect(&format!("{TROPHY_GRID}/Trophy{i}")).is_some_and(|r|r.contains(pointer)));
        if let Some(index)=clicked {self.select_trophy(index);}
    }
}

fn scroll_regions(data: &serde_json::Value, node: &Node) -> Vec<ScrollRegion> {
    fn walk<'a>(n: &'a serde_json::Value, path: String, ids: &mut HashMap<i64,String>, refs: &mut Vec<(&'a serde_json::Value,String)>) {
        if let Some(id)=n["transform_id"].as_i64() { ids.insert(id,path.clone()); }
        for c in n["comps"].as_array().into_iter().flatten() { if c["type"]=="ScrollRect" { refs.push((c,path.clone())); } }
        for child in n["children"].as_array().into_iter().flatten() {
            let name=child["name"].as_str().unwrap_or("");
            walk(child,if path.is_empty(){name.to_owned()}else{format!("{path}/{name}")},ids,refs);
        }
    }
    let mut ids=HashMap::new(); let mut refs=Vec::new(); walk(data,String::new(),&mut ids,&mut refs);
    let mut regions=Vec::new();
    for (c,owner) in refs {
        // Unity uses the ScrollRect's own RectTransform when viewport is unassigned.
        let view=if c["m_Viewport"]["m_PathID"].as_i64().unwrap_or(0)==0 {&owner}
            else {let Some(view)=c["m_Viewport"]["m_PathID"].as_i64().and_then(|id|ids.get(&id)) else {continue};view};
        let Some(content)=c["m_Content"]["m_PathID"].as_i64().and_then(|id|ids.get(&id)) else {continue};
        let Some(target)=node.find(content) else {continue};
        regions.push(ScrollRegion {view:view.clone(),content:content.clone(),origin:target.rt.pos,
            axes:vec2(c["m_Horizontal"].as_i64().unwrap_or(0) as f32,c["m_Vertical"].as_i64().unwrap_or(0) as f32),
            offset:Vec2::ZERO,velocity:Vec2::ZERO,sensitivity:c["m_ScrollSensitivity"].as_f64().unwrap_or(150.0) as f32,drag:None});
    }
    regions.sort_by_key(|r|std::cmp::Reverse(r.view.matches('/').count()));
    regions
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum MenuPage { CoopLobby,SocialGoals,SocialSeason,DailyStandard,DailyAdvanced,DailyCoop,Race,DailyStandardRules,DailyAdvancedRules,DailyCoopRules,RaceRules,BossRules,BossEliteRules,Profile, Knowledge, KnowledgeTree, Social, Powers, Achievements, Store, TrophyStore, Legends, Mailbox, QuickMatch, JoinMatch, Friends, Teams, Perks, Quests, Gift, Events, Television, Jukebox }

impl MenuPage {
    pub(super) fn event_spec(self)->Option<(&'static str,bool)>{
        Some(match self {
            Self::DailyStandard=>("daily_standard",false),Self::DailyAdvanced=>("daily_advanced",false),Self::DailyCoop=>("daily_coop",false),Self::Race=>("raceEvent",false),
            Self::DailyStandardRules=>("daily_standard",true),Self::DailyAdvancedRules=>("daily_advanced",true),Self::DailyCoopRules=>("daily_coop",true),Self::RaceRules=>("raceEvent",true),Self::BossRules=>("bossBloon",true),Self::BossEliteRules=>("bossElite",true),_=>return None
        })
    }
    pub(super) fn rules_page(self)->Option<Self>{Some(match self{Self::DailyStandard=>Self::DailyStandardRules,Self::DailyAdvanced=>Self::DailyAdvancedRules,Self::DailyCoop=>Self::DailyCoopRules,Self::Race=>Self::RaceRules,_=>return None})}
    pub(super) fn event_parent(self)->Self{match self{Self::DailyStandardRules=>Self::DailyStandard,Self::DailyAdvancedRules=>Self::DailyAdvanced,Self::DailyCoopRules=>Self::DailyCoop,Self::RaceRules=>Self::Race,_=>Self::Events}}

    pub fn from_name(name: &str) -> Option<Self> {
        Some(match name {
            "daily"=>Self::DailyStandard,"daily_advanced"=>Self::DailyAdvanced,"daily_coop"=>Self::DailyCoop,"race"=>Self::Race,
            "daily_rules"=>Self::DailyStandardRules,"advanced_rules"=>Self::DailyAdvancedRules,"race_rules"=>Self::RaceRules,"boss_rules"=>Self::BossRules,"boss_elite_rules"=>Self::BossEliteRules,
            "profile"=>Self::Profile,"jukebox"=>Self::Jukebox,
            "quests"=>Self::Quests,"gift"=>Self::Gift,"events"=>Self::Events,"tv"=>Self::Television,
            "coop_lobby"=>Self::CoopLobby,"social_goals"=>Self::SocialGoals,"social_season"=>Self::SocialSeason,"friends"=>Self::Friends,"teams"=>Self::Teams,"perks"=>Self::Perks,
            "knowledge" => Self::Knowledge, "knowledge_tree" => Self::KnowledgeTree,
            "quick_match" => Self::QuickMatch, "join_match" => Self::JoinMatch,
            "social" => Self::Social, "powers" => Self::Powers, "achievements" => Self::Achievements,
            "store" => Self::Store, "trophy_store" => Self::TrophyStore,
            "legends" => Self::Legends, "mailbox" => Self::Mailbox, _ => return None,
        })
    }
    pub(super) fn spec(self) -> (&'static str, &'static str, &'static str) {
        match self {
            Self::DailyStandard|Self::DailyAdvanced|Self::DailyCoop=>("DailyChallengeUI","DailyChallengeAnimation","Open"),
            Self::Race=>("RaceEventUI","DailyChallengeAnimation","Open"),
            Self::DailyStandardRules|Self::DailyAdvancedRules|Self::DailyCoopRules|Self::RaceRules|Self::BossRules|Self::BossEliteRules=>("ChallengeRulesUI","ChallengeRulesAnim","ChallengeRulesOpen"),
            Self::CoopLobby => ("CoopLobbyUI","LobbyMatchAnimation2","LobbyPanelOpen2"),
            Self::SocialGoals => ("SocialSeasonsGoalsUI","SocialSeasonsGoalsUI","GoalsOpen"),
            Self::SocialSeason => ("SocialSeasonsEventUI","SocialSeasonsEventUI","Open"),
            Self::Profile => ("PlayerStatsUI","PlayerStats","Open"),
            Self::Jukebox => ("JukeboxUI","JukeboxAnimation","Open"),
            Self::Quests => ("QuestBrowserUI","QuestBrowserAnim","Open"),
            Self::Gift => ("TowerGiftBoxUI","TowerGiftBoxPopUp","Open"),
            Self::Events => ("GameEventsUI","GameEventsAnimation","Open"),
            Self::Television => ("BloonsMonkeyTalesPopup","SceneBasicAnim","SceneBasicOpen"),
            Self::Friends => ("LinkFriendsUI","SceneBasicAnim","SceneBasicOpen"),
            Self::Teams => ("TeamsBrowserUI","SceneBasicAnim","SceneBasicOpen"),
            Self::Perks => ("PerksUI","PerksScene","PerksSceneOpen"),
            Self::Knowledge => ("KnowledgeMainUI", "KnowledgeScreenTransitions", "Open"),
            Self::KnowledgeTree => ("KnowledgeSkillTreeUI", "KnowledgeSkillTreeTransitions", "Open"),
            Self::QuickMatch => ("CoopQuickMatchUI", "CreateMatchAnimation", "MatchPanelOpen"),
            Self::JoinMatch => ("CoopJoinMatchUI", "JoinMatchAnimation", "JoinPanelOpen"),
            Self::Social => ("PlaySocialUI", "PlaySocialScreen", "PlaySocialScreenOpen"),
            Self::Powers => ("PowersSelectUI", "PowersSelectScreen", "PowersMenuOpen"),
            Self::Achievements => ("AchievementsUI", "AchievementsAnimation", "Open"),
            Self::Store => ("StoreUI", "StoreAnimation", "StoreOpen"),
            Self::TrophyStore => ("TrophyStoreUI", "TrophyStoreAnimation", "Open"),
            Self::Legends => ("LegendsSelectUI", "LegendsSelect", "LegendsSelectOpen"),
            Self::Mailbox => ("MailboxUI", "SceneBasicAnim", "SceneBasicOpen"),
        }
    }
    pub(super) fn background_key(self)->&'static str {
        if self==Self::Powers {"InstaPowersCollectionUI"}else{self.spec().0}
    }
    pub(super) fn load(self, dir: &Path, root: &Path) -> Option<Page> {
        let (file, controller, clip) = self.spec();
        let data: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(dir.join(format!("{file}.json"))).ok()?).ok()?;
        let mut node = if self==Self::Television {Node::from_json(&data)} else {scene_node(Node::from_json(&data))};
        let mut trophy_items=Vec::new();
        let mut power_items=Vec::new();
        let mut search=None;
        let mut knowledge_animation=None;
        let mut animation=None;
        let mut buttons=super::button_motion::ButtonMotion::default();
        // Retain only this opening clip, not all controllers in a scene.
        if let Some(data) = std::fs::read_to_string(root.join(format!("ui/runtime/{file}.json"))).ok()
            .and_then(|s| serde_json::from_str::<serde_json::Value>(&s).ok()) {
            buttons=super::button_motion::ButtonMotion::from_runtime(&data);
            apply_animation(&mut node, &data, controller, clip, None);
            if self==Self::KnowledgeTree {
                let animators:Vec<_>=data["animators"].as_array().into_iter().flatten().filter(|a|a["controller"]=="SelectedPanel").cloned().collect();
                if !animators.is_empty(){knowledge_animation=Some(PageAnimation {data:serde_json::json!({"animators":animators}),
                    controller:"SelectedPanel",clip:"SelectedPanelOpen",began:None});}
            }
            let animators:Vec<_>=data["animators"].as_array().into_iter().flatten()
                .filter(|a|a["controller"].as_str()==Some(controller)).filter_map(|a| {
                    let clips:Vec<_>=a["clips"].as_array().into_iter().flatten().filter(|c|c["name"].as_str()==Some(clip)).cloned().collect();
                    (!clips.is_empty()).then(||serde_json::json!({"controller":controller,"clips":clips}))
                }).collect();
            if !animators.is_empty() {animation=Some(PageAnimation {data:serde_json::json!({"animators":animators}),controller,clip,began:Some(get_time())});}
        }
        match self {
            Self::Profile => {
                let base="Canvas/PlayerStatsScreen";
                for path in ["Stats/Others","Stats/FollowerCount","Stats/Own/OnlineStatus","SettingsMenu","LoadingWheel"] {
                    node.set_active(&format!("{base}/{path}"),false);
                }
                node.set_text(&format!("{base}/Stats/TopInfo/PlayerName"),"Loading account...");
                for path in ["TopInfo/PlayerRank","TopInfo/VeteranPlayerRank","OverallStats/PublicStats",
                    "Medals/MedalScroll/Viewport/MedalContainer/MedalPrefab","Medals/MedalScroll/Viewport/MedalContainer/SPMedals",
                    "Medals/MedalScroll/Viewport/MedalContainer/CoopMedals","Medals/MedalScroll/Viewport/MedalContainer/SpecialMedals"] {
                    node.set_active(&format!("{base}/Stats/{path}"),false);
                }
            }
            Self::Gift => {node.hide_named(&["DebugForceUnlockButton","DebugResetButton"]);}
            Self::Television => {node.hide_named(&["BloonsTube"]);}
            Self::Store => {
                if let Some(catalog)=std::fs::read_to_string(root.join("ui/runtime/store_menu.json")).ok()
                    .and_then(|s|serde_json::from_str::<serde_json::Value>(&s).ok()) {
                    for label in catalog["labels"].as_array().into_iter().flatten() {
                        if let (Some(path),Some(text))=(label["path"].as_str(),label["text"].as_str()){node.set_text(path,text);}
                    }
                    const CONTENT:&str="Canvas/StoreScreen/Products/Viewport/Content";
                    if let Some(template)=Node::load(&dir.join("StoreProductSmall.json")) {
                        if let Some(container)=node.find_mut(&format!("{CONTENT}/ContentSmallTemplate")) {
                            container.fit[0]=2;
                            container.children=catalog["items"].as_array().into_iter().flatten()
                                .filter(|p|p["visible"]==true && p["product_type"]==0).enumerate().map(|(i,item)| {
                                    let mut card=template.clone();card.name=format!("Product{i}");card.active=true;
                                    card.set_text("PopupButton/Anim/Title",item["title"].as_str().unwrap_or(""));
                                    card.set_sprite("PopupButton/Anim/ImagePosition/Image",item["icon"].as_str().and_then(|s|s.strip_suffix(".png")));
                                    // Product service stock/prices and account entitlement are not catalog data.
                                    card.hide_named(&["BuyGroup","TimeRemainingBg","LimitedTime","SaleImage","BonusImage","DlcImage","LootPanel","Glow","Info"]);
                                    card
                                }).collect();
                        }
                    }
                    if let Some(content)=node.find_mut(CONTENT){content.fit[0]=2;}
                    node.set_active(&format!("{CONTENT}/ContentLargeTemplate"),false);
                }
                for path in ["Products/Viewport/LoadingWheel","TaxMessage","FilterTabs/TabLegends/NewRibbon",
                    "ButtonLayout/NexusButtonContainer/Tick","ButtonLayout/NexusButtonContainer/Cross","JewelBar"] {
                    node.set_active(&format!("Canvas/StoreScreen/{path}"),false);
                }
                for tab in ["TabPopular","TabMonkeyMoney","TabHeroesAndMonkeys","TabModesAndMore","TabLegends"] {
                    node.set_active(&format!("Canvas/StoreScreen/FilterTabs/{tab}/Highlight"),tab=="TabPopular");
                }
            }
            Self::QuickMatch => {
                node.set_active("CoopQuickMatchCanvas/CoopQuickMatchScreen/LoadingObject",false);
                for name in ["QuickGame","MediumGame","LongGame"] {
                    node.set_active(&format!("CoopQuickMatchCanvas/CoopQuickMatchScreen/GameType/{name}/Button/SelectGlow"),name=="QuickGame");
                }
            }
            Self::JoinMatch => {
                node.set_active("CoopJoinMatchCanvas/CoopJoinScreen/LoadingObject",false);
            }
            Self::Knowledge => {
                node.set_active("Canvas/KnowledgeMain/JewelBar",false);
                node.set_active("Canvas/KnowledgeMain/LevelLockInfo", false);
                if let Some(counts)=std::fs::read_to_string(root.join("ui/runtime/knowledge_counts.json")).ok()
                    .and_then(|s|serde_json::from_str::<serde_json::Value>(&s).ok()) {
                    for (i,category) in ["Primary","Military","Magic","Support","Heroes","Powers"].iter().enumerate() {
                        let total=counts[format!("Page{}",i+1)].as_u64().unwrap_or(0);
                        node.set_text(&format!("Canvas/KnowledgeMain/Buttons/{category}Btn/Button/CompletedTxt"),&format!("0/{total}"));
                    }
                }
            }
            Self::KnowledgeTree => {
                node.set_active(KNOWLEDGE_PANEL,false);
                node.set_active("Canvas/KnowledgeSkillTree/JewelBar",false);
            }
            Self::Powers => {
                node.set_active("Canvas/InstaTowerScreen", false);
                let base="Canvas/PowersSelectScreen/SafeArea";
                // An offline catalog has no live IAP launch pack; never draw its unbound white image.
                node.set_active(&format!("{base}/IapPackButton"),false);
                for name in ["AllTab","Powers","Pro"] {
                    node.set_active(&format!("{base}/Selections/LayoutGroup/ScrollGroup/Tabs/{name}/BgSelected"),name=="AllTab");
                }
                let grid=format!("{base}/Selections/LayoutGroup/ScrollGroup/Viewport/Powers");
                if let Some(powers)=std::fs::read_to_string(root.join("ui/runtime/powers.json")).ok()
                    .and_then(|s|serde_json::from_str::<serde_json::Value>(&s).ok()) {
                    if let Some(container)=node.find_mut(&grid) {
                        if let Some(template)=container.children.first().cloned() {
                            container.children=powers.as_array().into_iter().flatten().enumerate().map(|(i,p)| {
                                // Shown by bind_powers (SkuSettings.CanShowPower).
                                let mut button=template.clone();button.name=format!("Power{i}");button.active=false;
                                button.set_sprite("Icon",p["icon"].as_str().and_then(|s|s.strip_suffix(".png")));
                                button.set_text("Text",p["title"].as_str().unwrap_or(""));
                                button.set_active("SelectedImage",false);
                                // PowerSelectButton.SetPower: BgImage when it costs Monkey Money, BgImageIAP
                                // otherwise; UpdatePowerDisplay then turns on BgImagePro for a Pro power and
                                // BgImageIAP (store power) or BgImage for the rest.
                                let pro=p["pro"]==true;
                                let store=p["store"].as_str().is_some_and(|s|!s.is_empty());
                                let paid=p["cost"].as_i64().unwrap_or(0)>0;
                                button.set_active("BgImage",paid || (!pro && !store));
                                button.set_active("BgImageIAP",!paid || (!pro && store));
                                button.set_active("BgImagePro",pro);
                                button
                            }).collect();
                        }
                    }
                    power_items=powers.as_array().cloned().unwrap_or_default();
                }
            }
            Self::Achievements => {
                node.hide_named(&["GooglePlay", "GameCenter"]);
                let base="Canvas/AchievementsScreen";
                let template_path=format!("{base}/Scroll/Viewport/AchievementPanelBlue");
                let template=node.find(&template_path).cloned();
                node.set_active(&template_path,false);
                if let (Some(template),Some(entries))=(template,read_catalog(root,"achievements")) {
                    let loot_template=Node::load(&dir.join("SmallBaseLootItem.json"));
                    let insta_template=Node::load(&dir.join("SmallInstaTowerLootItem.json"));
                    if let Some(content)=node.find_mut(&format!("{base}/Scroll/Viewport/Content")) {
                        content.fit[1]=2;
                        content.children=entries.iter().enumerate().map(|(i,a)| {
                            let mut card=template.clone();card.name=format!("Achievement{i}");card.active=true;
                            card.set_text("Title",a["title"].as_str().unwrap_or(""));
                            card.set_text("Description",a["description"].as_str().unwrap_or(""));
                            card.set_sprite("Achievements/Icon",a["icon"].as_str().and_then(|s|s.strip_suffix(".png")));
                            card.set_text("Achievements/Text",a["subscript"].as_str().unwrap_or(""));
                            card.set_active("InProgressGroup",true);
                            card.set_text("InProgressGroup/ProgressBar/ProgressText","0%");
                            if let Some(bar)=card.find_mut("InProgressGroup/ProgressBar/Mask/InnerBar") {bar.rt.amax.x=0.0;}
                            card.hide_named(&["ClaimGroup","ClaimedGroup","iconGlow"]);
                            card.set_active("RewardsGroup",true);
                            if let Some(container)=card.find_mut("RewardsGroup/LootPanel") {
                                container.children=a["rewards"].as_array().into_iter().flatten().enumerate().filter_map(|(j,reward)| {
                                    let mut item=if reward["kind"]=="InstaMonkey" {insta_template.clone()} else {loot_template.clone()}?;
                                    item.name=format!("Reward{j}");item.active=true;
                                    item.set_sprite("Icon",reward["icon"].as_str().and_then(|s|s.strip_suffix(".png")));
                                    item.set_text("Description",reward["label"].as_str().unwrap_or(""));
                                    item.hide_named(&["Bonus","AddOne","LimitedTime"]);
                                    Some(item)
                                }).collect();
                            }
                            card
                        }).collect();
                    }
                    node.set_text(&format!("{base}/TopInfo/Count"),&format!("0/{}",entries.len()));
                    node.hide_named(&["NothingFound"]);
                    search=Some(AchievementSearch {terms:entries.iter().map(|a|format!("{}\n{}",
                        a["title"].as_str().unwrap_or(""),a["description"].as_str().unwrap_or("")).to_lowercase()).collect(),
                        query:std::env::var("RRBTD6_ACHIEVEMENT_SEARCH").unwrap_or_default(),focused:false});
                }
            }
            Self::TrophyStore => {
                node.set_active("Canvas/SortBy", false);
                let base="Canvas/TrophyStoreScreen/Container";
                node.set_text(&format!("{base}/TrophiesEarned/TrophiesQtyTxt"),"0");
                node.set_active(&format!("{base}/BottomInfo/PurchasedGroup"),false);
                node.set_active(&format!("{base}/BottomInfo/TrackEQ"),false);
                node.set_active(&format!("{base}/BottomInfo/QuestButton"),false);
                node.set_active(&format!("{base}/BottomInfo/AddButton"),true);
                if let (Some(template),Some(entries))=(Node::load(&dir.join("TrophyStoreCard.json")),read_catalog(root,"trophy_items")) {
                    if let Some(content)=node.find_mut(&format!("{base}/Panel/Scroll/LayoutGroup/Viewport/Content")) {
                        content.children=entries.iter().enumerate().map(|(i,item)| {
                            let mut card=template.clone();card.name=format!("Trophy{i}");card.active=true;
                            card.set_sprite("Icon",item["icon"].as_str().and_then(|s|s.strip_suffix(".png")));
                            card.set_sprite("ItemType",item["type_icon"].as_str().and_then(|s|s.strip_suffix(".png")));
                            card.set_text("ItemName",item["short_name"].as_str().unwrap_or(""));
                            card.set_active("StandardFrame",true);card.set_active("Selected",i==0);
                            card
                        }).collect();
                    }
                    if let Some(item)=entries.first() {
                        node.set_sprite(&format!("{base}/BottomInfo/Icon"),item["icon"].as_str().and_then(|s|s.strip_suffix(".png")));
                        node.set_text(&format!("{base}/BottomInfo/Title"),item["title"].as_str().unwrap_or(""));
                        node.set_text(&format!("{base}/BottomInfo/Description"),item["description"].as_str().unwrap_or(""));
                        node.set_text(&format!("{base}/BottomInfo/AddButton/TextCount"),"");
                    }
                    trophy_items=entries;
                }
            }
            _ => {}
        }
        let legends=if self==Self::Legends {
            let bindings=std::fs::read_to_string(root.join("ui/runtime/legends_menu.json")).ok().and_then(|s|serde_json::from_str::<serde_json::Value>(&s).ok());
            bindings.map(|bindings| {
                for text in bindings["text"].as_array().into_iter().flatten() {
                    if let (Some(path),Some(text))=(text["path"].as_str(),text["text"].as_str()){node.set_text(path,text);}
                }
                node.set_active("Canvas/LegendsSelectScreen/JewelBar",false);
                // Read-only expansion previews use the source information panel. Product prices,
                // sale flags and ownership are supplied by services and are not bound here.
                for panel in bindings["panels"].as_array().into_iter().flatten() {
                    if let Some(path)=panel["mainPanel"].as_str(){node.set_active(path,true);}
                    if let Some(path)=panel["purchasePanel"].as_str(){node.set_active(path,false);}
                }
                // Source artwork has an unbound empty release banner; live release state is absent.
                node.hide_named(&["UpdatedLegendBanner"]);
                if let Some(pips)=node.find_mut("Canvas/LegendsSelectScreen/PagePips") {
                    if let Some(template)=pips.children.first().cloned() {
                        pips.children=bindings["slides"].as_array().into_iter().flatten().enumerate().map(|(i,_)| {
                            let mut pip=template.clone();pip.name=format!("Pip{i}");pip
                        }).collect();
                    }
                }
                let mut runtime:serde_json::Value=std::fs::read_to_string(root.join("ui/runtime/LegendsSelectUI.json")).ok()
                    .and_then(|s|serde_json::from_str(&s).ok()).unwrap_or_default();
                if let Some(animators)=runtime["animators"].as_array_mut(){animators.retain(|a|a["controller"]=="PageTransition");}
                LegendsBrowser {data:bindings,selected:0,about:false,animation:None,runtime}
            })
        }else{None};
        let coop=if self==Self::CoopLobby{Some(super::coop_menu::Lobby::new(&mut node))}else{None};
        let goals=if self==Self::SocialGoals{super::season_goals::Browser::new(root,&mut node)}else{None};
        let social=super::social_pages::SocialPage::load(root,file,&mut node);
        let profile=if self==Self::Profile{super::profile_browser::Browser::load(root,&mut node)}else{None};
        let jukebox=if self==Self::Jukebox{super::jukebox::Browser::load(root,&mut node,crate::menus::load_loc(&root.join("text/English.xml")))}else{None};
        let scrolls=scroll_regions(&data,&node);
        let powers=(!power_items.is_empty()).then(||PowerBrowser {costs:vec![0;power_items.len()],shown:vec![false;power_items.len()],quantities:vec![None;power_items.len()],
            items:power_items,selected:0,began:get_time(),dir:dir.to_path_buf(),
            runtime_dir:root.join("ui/runtime"),portraits:HashMap::new(),bound:false,seen:Default::default(),pro:HashMap::new(),
            // CommonBackgroundScreen starts from the MenuBackgrounds colour (white for InstaPowersCollectionUI).
            tint:(WHITE,WHITE,get_time())});
        let knowledge=if self==Self::KnowledgeTree {read_catalog(root,"knowledge_items").map(|items|KnowledgeBrowser {
            states:vec![0;items.len()],arrow_fade:vec![None;items.len()],items,selected:None,panel_animation:knowledge_animation,
            owned:Default::default(),disabled:Default::default(),points:0,account:None,pulse:serde_json::Value::Null,
            runtime:std::fs::read_to_string(root.join("ui/runtime/KnowledgeSkillTreeUI.json")).ok()
                .and_then(|s|serde_json::from_str(&s).ok()).unwrap_or_default()})}else{None};
        let join_code=if self==Self::JoinMatch {
            fn input_limit(n:&serde_json::Value)->Option<usize> {
                n["comps"].as_array().into_iter().flatten().find_map(|c|c["m_CharacterLimit"].as_u64().map(|n|n as usize))
                    .or_else(||n["children"].as_array()?.iter().find_map(input_limit))
            }
            let _native_limit=input_limit(&data);
            let limit=160;
            let text=std::env::var("RRBTD6_JOIN_CODE").unwrap_or_default().chars().take(limit).collect::<String>();
            node.set_text(&format!("{JOIN_INPUT}/Text Area/Text"),&text);
            Some(JoinCode {text,focused:false,limit,submit:false})
        }else{None};
        let event=self.event_spec().map(|(key,rules)|super::event_details::Detail::new(root,key,rules));
        // TrophyStoreScreen opens in store mode on the Featured tab (OnMainToggleChanged(false)).
        let trophy=(self==Self::TrophyStore&&!trophy_items.is_empty()).then(|| {
            let text=std::fs::read_to_string(root.join("text/English.xml")).unwrap_or_default();
            let entry=|id:&str|text.split(&format!("<T id=\"{id}\">")).nth(1).and_then(|s|s.split('<').next()).unwrap_or(id).to_owned();
            TrophyState {inventory:false,tab:5,live:None,account:None,cards:Vec::new(),day:entry("day"),days:entry("days"),root:root.to_owned()}
        });
        let mut page=Page {first_presentation:true,node,season_second:None,buttons,profile,jukebox,event,goals,coop,scrolls,trophy_items,trophy_selected:Some(0),trophy,pointer_start:None,pointer_dragged:false,animation,powers,search,knowledge,legends,join_code,social,home_requested:false};
        if self==Self::Legends {
            let preview=std::env::var("RRBTD6_LEGENDS_PREVIEW").unwrap_or_default();
            let index=page.legends.as_ref().and_then(|b|b.data["slides"].as_array())
                .and_then(|slides|slides.iter().position(|p|p.as_str().is_some_and(|p|p.rsplit('/').next()==Some(preview.as_str())))).unwrap_or(0);
            page.show_legend(index,preview.eq_ignore_ascii_case("about"));
        }
        if self==Self::KnowledgeTree {
            if let Some(browser)=&page.knowledge {
                for item in &browser.items {
                    if let Some(icon)=item["nodeIcon"].as_str(){page.node.set_active(icon,false);}
                }
            }
            page.refresh_knowledge();
            if let Some(index)=std::env::var("RRBTD6_KNOWLEDGE_PREVIEW").ok().and_then(|id|
                page.knowledge.as_ref()?.items.iter().position(|k|k["id"].as_str()==Some(&id))) {page.select_knowledge(Some(index));}
        }
        if page.search.is_some(){page.filter_achievements();}
        if self==Self::Powers {page.bind_powers();}
        if self==Self::TrophyStore {
            let category=std::env::var("RRBTD6_TROPHY_CATEGORY").ok().and_then(|name|
                TROPHY_TABS.iter().find(|(label,_)|label.eq_ignore_ascii_case(&name)).and_then(|(_,category)|*category));
            page.filter_trophies(category);
        }
        Some(page)
    }
}

fn read_catalog(root: &Path, name: &str) -> Option<Vec<serde_json::Value>> {
    serde_json::from_str(&std::fs::read_to_string(root.join(format!("ui/runtime/{name}.json"))).ok()?).ok()
}

#[cfg(test)]
mod catalog_tests {
    use super::*;
    use serde_json::json;
    #[test]
    fn scroll_rect_without_viewport_uses_its_own_bounds() {
        let data=json!({"name":"Scene","children":[{"name":"Canvas","children":[
            {"name":"List","transform_id":10,"comps":[{"type":"ScrollRect",
                "m_Content":{"m_PathID":20},"m_Viewport":{"m_PathID":0},"m_Vertical":1}],
             "children":[{"name":"Content","transform_id":20}]}]}]});
        let node=Node::from_json(&data);
        let regions=scroll_regions(&data,&node);
        assert_eq!(regions.len(),1);
        assert_eq!(regions[0].view,"Canvas/List");
        assert_eq!(regions[0].content,"Canvas/List/Content");
        assert_eq!(regions[0].axes,vec2(0.0,1.0));
    }
    #[test]
    fn trophy_category_zero_filters_and_switching_selects_a_visible_item() {
        let mut tree=json!({"name":"Content","children":[
            {"name":"Trophy0","children":[{"name":"Selected"}]},
            {"name":"Trophy1","children":[{"name":"Selected"}]},
            {"name":"Trophy2","children":[{"name":"Selected"}]}
        ]});
        for name in TROPHY_GRID.split('/').rev().skip(1) {tree=json!({"name":name,"children":[tree]});}
        let mut page=Page {first_presentation:true,season_second:None,node:Node::from_json(&json!({"name":"Scene","children":[tree]})),
            buttons:Default::default(),
            scrolls:Vec::new(),trophy_items:vec![json!({"category":1}),json!({"category":0}),json!({"category":2})],
            profile:None,jukebox:None,event:None,goals:None,coop:None,trophy_selected:Some(0),trophy:None,pointer_start:None,pointer_dragged:false,animation:None,powers:None,search:None,knowledge:None,legends:None,join_code:None,social:None,home_requested:false};
        page.filter_trophies(Some(0));
        assert_eq!(page.trophy_selected,Some(1));
        let cards=&page.node.find(TROPHY_GRID).unwrap().children;
        assert_eq!(cards.iter().map(|c|c.active).collect::<Vec<_>>(),[false,true,false]);
        page.filter_trophies(Some(2));
        assert_eq!(page.trophy_selected,Some(2));
        page.filter_trophies(None);
        assert!(page.node.find(TROPHY_GRID).unwrap().children.iter().all(|c|c.active));
    }
}

fn knowledge_learnt(browser:&KnowledgeBrowser,id:&str)->bool {browser.owned.contains(id)||browser.disabled.contains(id)}

/// KnowledgeHelper.GetCategoryLearntTotal: owned or disabled knowledge in a category.
fn knowledge_category_learnt(browser:&KnowledgeBrowser,category:&serde_json::Value)->i64 {
    browser.items.iter().filter(|k|&k["category"]==category&&k["id"].as_str().is_some_and(|id|knowledge_learnt(browser,id))).count() as i64
}

/// KnowledgeSkillTree.UpdateButtonStates: purchased, disabled, available
/// (KnowledgeHelper.IsKnowledgePurchasable) or locked.
fn knowledge_state(browser:&KnowledgeBrowser,i:usize)->u8 {
    let item=&browser.items[i];
    let id=item["id"].as_str().unwrap_or("");
    if browser.owned.contains(id) {return 3;}
    if browser.disabled.contains(id) {return 4;}
    let prerequisites=item["prerequisites"].as_array().into_iter().flatten().all(|p|p.as_str().is_some_and(|p|knowledge_learnt(browser,p)));
    let invested=knowledge_category_learnt(browser,&item["category"])>=item["required"].as_i64().unwrap_or(0);
    if browser.points>0&&prerequisites&&invested {1} else {2}
}

/// KnowledgeSkillBtn.SetSelected: the selected glow, and the pulse only while available and not selected.
fn set_knowledge_selected(node:&mut Node,item:&serde_json::Value,selected:bool,state:u8) {
    let Some(path)=item["path"].as_str() else {return};
    node.set_active(&format!("{path}/Highlight"),selected);
    node.set_active(&format!("{path}/Highlight Pulse"),!selected&&state==1);
}

fn set_arrow_color(node:&mut Node,item:&serde_json::Value,color:Color) {
    for path in item["arrows"].as_array().into_iter().flatten().filter_map(|p|p.as_str()) {
        if let Some(img)=node.find_mut(path).and_then(|n|n.img.as_mut()){img.color=color;}
    }
}

/// KnowledgeSkillTree.UpdateSelectedSkill for the selected node's state.
fn fill_knowledge_panel(node:&mut Node,browser:&KnowledgeBrowser,i:usize) {
    let item=&browser.items[i];
    let state=browser.states[i];
    let id=item["id"].as_str().unwrap_or("");
    let panel=format!("{KNOWLEDGE_PANEL}/Panel");
    node.set_sprite(&format!("{panel}/SkillIcon"),item["icon"].as_str());
    node.set_text(&format!("{panel}/TitleTxt"),item["title"].as_str().unwrap_or(""));
    node.set_text(&format!("{panel}/LayoutGroup/DescTxt"),item["description"].as_str().unwrap_or(""));
    node.set_active(&format!("{panel}/LayoutGroup/EnableCheck"),state==3||state==4);
    node.set_active(&format!("{panel}/LayoutGroup/EnableCheck/Tick"),!browser.disabled.contains(id));
    node.set_active(&format!("{panel}/LayoutGroup/AddPointButton"),state==1);
    let cost=item["cost"].as_i64().unwrap_or(0);
    if state==1 {
        node.set_active(&format!("{panel}/LayoutGroup/AddPointButton/MonkeyMoney"),cost>0);
        node.set_text(&format!("{panel}/LayoutGroup/AddPointButton/MonkeyMoney/TextCount"),&cost.to_string());
    }
    // KnowledgeHelper.IsKnowledgePurchasableReason: missing prerequisites, then investment.
    let reason=if state==2 {
        let missing:Vec<&str>=item["prerequisites"].as_array().into_iter().flatten().filter_map(|p|p.as_str())
            .filter(|p|!knowledge_learnt(browser,p)).collect();
        let learnt=knowledge_category_learnt(browser,&item["category"]);
        let required=item["required"].as_i64().unwrap_or(0);
        if !missing.is_empty() {
            let mut text=item["needsText"].as_str().unwrap_or("Needs").to_owned();
            for p in missing {
                let name=browser.items.iter().find(|k|k["id"]==p).and_then(|k|k["title"].as_str()).unwrap_or(p);
                text.push(' ');text.push_str(name);text.push(',');
            }
            text.pop();text
        } else if learnt<required {
            item["requirementText"].as_str().unwrap_or("").replace("{0}",&(required-learnt).to_string())
                .replace("{1}",item["categoryName"].as_str().unwrap_or(""))
        } else {String::new()}
    } else {String::new()};
    node.set_active(&format!("{panel}/PrereqTxt"),true);
    node.set_text(&format!("{panel}/PrereqTxt"),&reason);
}
