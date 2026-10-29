//! Private peer lobby controls using the original lobby scene.
use crate::uitree::Node;
use macroquad::prelude::*;
pub(super) const BASE:&str="Canvas/CoopLobbyScreen";
const SETTINGS:&str="Canvas/CoopLobbyScreen/BgPanel/StandardMatchUI";
const DROPS:[&str;2]=["DiffDropdown","GameModeDropdown"];
const DIVISIONS:[&str;7]=["Horizontal","Vertical","DiagonalLR","DiagonalRL","FreeForAll","RadioActive","Stairs"];
const DIFFICULTIES:[&str;3]=["Easy","Medium","Hard"];
const MODES:[&[&str];3]=[
    &["Standard","Primary Only","Deflation"],
    &["Standard","Military Only","Apopalypse","Reverse"],
    &["Standard","Magic Only","Double HP MOABs","Half Cash","Alternate Bloons Rounds","Impoppable","CHIMPS"],
];
pub(super) struct Lobby {custom_modes:Vec<String>,difficulty:usize,mode:usize,open:Option<usize>,closed:[bool;4],division:usize,editable:bool,changed:bool,net_revision:u64,network_room:Option<rrbtd6_coop::Room>}
impl Lobby{
    pub fn new(node:&mut Node)->Self{
        node.hide_named(&["LoadingObject","DebugStartGame","DebugReadyPanel","CoopLobbyReconnectingPanel","BossSelectPanel","BossChallengeMapSelect","Jewel","HeroIconChangeFx","CopyCodeTick","GoldenBloon","MonkeyTeams","CollectionEventIcon","ChimpsGlintsFxGold","ChimpsSparklesFxHematite"]);
        for path in ["BgPanel/BossMatchUI","BgPanel/Code","BgPanel/StandardMatchUI/Rewards","SelectionButton/PublicButton","Hotkeys","BgPanel/TopRightButtonGroup/BossSelectButton","BgPanel/TopRightButtonGroup/ChallengeRules","BgPanel/MapGroup/MapInfoPane/MedalContainer","BgPanel/MapGroup/MapInfoPane/BronzeBg","BgPanel/MapGroup/MapInfoPane/SilverBg","BgPanel/CoopGameTypeBg/CoopGameTypeDropdown"]{node.set_active(&format!("{BASE}/{path}"),false);}
        node.set_active(&format!("{BASE}/BgPanel/StandardMatchUI"),true);
        node.set_text(&format!("{BASE}/BgPanel/CoopGameTypeBg/Text"),"Standard");
        node.set_text(&format!("{BASE}/SelectionButton/ReadyButton/ConfirmText"),"Not connected");
        if let Some(mut button)=node.find(&format!("{BASE}/SelectionButton/ReadyButton")).cloned(){
            button.name="RoomMods".into();button.rt.amin=vec2(0.0,0.0);button.rt.amax=button.rt.amin;
            button.rt.pivot=vec2(0.0,0.0);button.rt.pos=vec2(100.0,120.0);button.rt.size=vec2(650.0,200.0);
            button.set_text("ConfirmText","Room Mods");
            if let Some(root)=node.find_mut(BASE){root.children.push(button);}
        }
        let mut lobby=Self{custom_modes:Vec::new(),difficulty:1,mode:0,open:None,closed:[false;4],division:4,editable:true,changed:false,net_revision:u64::MAX,network_room:None};
        lobby.open=std::env::var("RRBTD6_COOP_DROPDOWN").ok().and_then(|s|s.parse::<usize>().ok()).filter(|&i|i<2);
        lobby.refresh(node);lobby
    }
    fn choices(&self,which:usize)->Vec<String>{
        if which==0{DIFFICULTIES.iter().map(|s|s.to_string()).collect()}
        else{MODES[self.difficulty].iter().map(|s|s.to_string()).chain(self.custom_modes.iter().cloned()).collect()}
    }
    fn refresh(&mut self,node:&mut Node){
        for which in 0..2{
            let path=format!("{SETTINGS}/{}",DROPS[which]);let choices=self.choices(which);
            let selected=if which==0{self.difficulty}else{self.mode};
            node.set_text(&format!("{path}/Text"),&choices[selected]);
            let content=format!("{path}/Template/Viewport/Content");
            let template=node.find(&content).and_then(|n|n.children.first()).cloned();
            if let (Some(template),Some(group))=(template,node.find_mut(&content)){
                let h=template.rt.size.y.max(1.0);group.rt.size.y=h*choices.len() as f32;
                group.children=choices.iter().enumerate().map(|(i,label)|{
                    let mut row=template.clone();row.name=format!("Option{i}");row.active=true;
                    row.rt.pos.y=-(i as f32)*h;row.set_text("Item Label",label);
                    row.set_active("Item Selected",i==selected);row
                }).collect();
            }
            node.set_active(&format!("{path}/Template"),self.open==Some(which));
        }
        // Unity puts an open TMP dropdown on an override-sorting canvas. Within this
        // absolute-positioned panel, drawing that dropdown last preserves the same overlap.
        if let Some(which)=self.open {
            if let Some(panel)=node.find_mut(SETTINGS){
                if let Some(i)=panel.children.iter().position(|n|n.name==DROPS[which]){
                    let dropdown=panel.children.remove(i);panel.children.push(dropdown);
                }
            }
        }
        let divisions=format!("{BASE}/BgPanel/MapGroup/MapInfoPane/MapMask/CoopMapDivisions");
        for (i,name) in DIVISIONS.iter().enumerate(){
            let path=format!("{divisions}/{name}");node.set_active(&path,i==self.division);
            let two=self.closed.iter().filter(|&&closed|!closed).count()==2;
            node.set_active(&format!("{path}/2P"),two);node.set_active(&format!("{path}/4P"),!two);
        }
        for i in 0..4{
            let path=format!("{BASE}/CoopPlayerInfo/PlayerSlot{}",i+1);
            // No invented remote players or server-issued lobby code.
            node.set_active(&format!("{path}/OccupiedState"),false);
            node.set_active(&format!("{path}/EmptySlot"),!self.closed[i]);
            node.set_active(&format!("{path}/ClosedSlot"),self.closed[i]);
            node.set_active(&format!("{path}/EmptySlot/LoadingIcon"),false);
            node.set_active(&format!("{path}/EmptySlot/InviteFriend"),false);
            node.set_text(&format!("{path}/EmptySlot/Text"),if i==0{"Local setup"}else{"Empty Slot"});
            node.set_active(&format!("{path}/EmptySlot/CloseSlotBtn"),i>0);
        }
    }
    pub fn settings(&self,map:String)->rrbtd6_coop::Settings{
        let custom_mode=self.mode.checked_sub(MODES[self.difficulty].len()).and_then(|i|self.custom_modes.get(i)).cloned();
        rrbtd6_coop::Settings{map,difficulty:self.difficulty as u8,mode:if custom_mode.is_some(){0}else{self.mode as u8},custom_mode,division:self.division as u8,closed:self.closed}
    }
    pub fn take_changed(&mut self)->bool{std::mem::take(&mut self.changed)}
    pub fn bind_session(&mut self,node:&mut Node,session:&super::coop_session::Session){
        if self.net_revision!=session.revision{
            self.net_revision=session.revision;self.editable=!session.active()||session.host;
            self.network_room=session.room.clone();if !self.editable{self.open=None;}
            if let Some(room)=&self.network_room{self.custom_modes=rrbtd6_coop::mods::mode_ids(&room.mods);self.difficulty=room.settings.difficulty as usize;self.mode=room.settings.custom_mode.as_ref().and_then(|id|self.custom_modes.iter().position(|m|m==id)).map_or(room.settings.mode as usize,|i|MODES[self.difficulty].len()+i);self.division=room.settings.division as usize;self.closed=room.settings.closed;}
            self.refresh(node);
        }
        node.set_active(&format!("{BASE}/RoomMods"),session.room.is_some());
        node.set_text(&format!("{BASE}/RoomMods/ConfirmText"),&format!("Room Mods ({})",session.room.as_ref().map_or(0,|r|r.mods.len())));
        let waiting=session.active()&&session.room.is_none();let failed=session.error;
        node.set_active(&format!("{BASE}/LoadingObject"),waiting||failed);
        node.set_text(&format!("{BASE}/LoadingObject/LoadingGame/Text"),&session.status);
        node.set_active(&format!("{BASE}/BgPanel/Code"),session.invite.is_some());
        if let Some(code)=&session.invite{
            let preview=if code.len()>25{format!("{}...{}",&code[..12],&code[code.len()-6..])}else{code.clone()};
            node.set_text(&format!("{BASE}/BgPanel/Code/Insert/CodeInfo"),&preview);
            node.set_text(&format!("{BASE}/BgPanel/Code/Code Text"),"Copy Invite");
            for path in ["Tick","HideCodeToggle","LobbyCodeHidden","SocialShareBtn/IosShareImg","SocialShareBtn/AndroidShareImg"]{node.set_active(&format!("{BASE}/BgPanel/Code/{path}"),false);}
        }
        let ready=session.slot.and_then(|slot|session.room.as_ref()?.players.iter().find(|p|p.slot==slot)).is_some_and(|p|p.ready);
        let can_start=session.host&&session.room.as_ref().is_some_and(|r|r.players.len()>1&&r.players.iter().all(|p|p.ready));
        node.set_text(&format!("{BASE}/SelectionButton/ReadyButton/ConfirmText"),if session.room.is_none(){"Not connected"}else if can_start{"Start!"}else if ready{"Not Ready"}else{"Ready!"});
        if let Some(room)=&self.network_room{
            for i in 0..4{
                let path=format!("{BASE}/CoopPlayerInfo/PlayerSlot{}",i+1);
                let player=room.players.iter().find(|p|p.slot as usize==i);
                node.set_active(&format!("{path}/OccupiedState"),player.is_some());
                node.set_active(&format!("{path}/EmptySlot"),player.is_none()&&!self.closed[i]);
                node.set_active(&format!("{path}/ClosedSlot"),player.is_none()&&self.closed[i]);
                node.set_active(&format!("{path}/EmptySlot/CloseSlotBtn"),i>0&&self.editable);
                node.set_active(&format!("{path}/ClosedSlot/OpenSlotBtn"),self.editable);
                if let Some(player)=player{
                    node.set_text(&format!("{path}/OccupiedState/Name"),&format!("{}{}",player.name,if player.ready{" (Ready)"}else{""}));
                    for child in ["CustomBanner","LvlStar","Difficulty","HeroIcon","NoHeroIcon","InfoButton","HeroIconChangeFx"]{node.set_active(&format!("{path}/OccupiedState/{child}"),false);}
                    node.set_active(&format!("{path}/OccupiedState/Ribbon/HostIndicator"),i==0);
                }
            }
        }
    }
    pub fn dismiss(&mut self,node:&mut Node)->bool{
        if self.open.take().is_none(){return false;}self.refresh(node);true
    }
    pub fn modal_path(&self)->Option<String>{self.open.map(|i|format!("{SETTINGS}/{}/Template",DROPS[i]))}
    /// Receives a release after the shared scroll controller ruled out a drag.
    pub fn click(&mut self,node:&mut Node,pointer:Vec2){
        let hit=|n:&Node,p:&str|n.rect(p).is_some_and(|r|r.contains(pointer));
        if let Some(which)=self.open{
            let path=format!("{SETTINGS}/{}/Template/Viewport/Content",DROPS[which]);
            let choice=(0..self.choices(which).len()).find(|i|hit(node,&format!("{path}/Option{i}")));
            if let Some(i)=choice{if which==0{self.difficulty=i;self.mode=0;}else{self.mode=i;}self.changed=true;}
            self.open=None;self.refresh(node);return;
        }
        if !self.editable{return;}
        for (i,name) in DROPS.iter().enumerate(){if hit(node,&format!("{SETTINGS}/{name}")){self.open=Some(i);self.refresh(node);return;}}
        for (name,delta) in [("LeftBtn",DIVISIONS.len()-1),("RightBtn",1)]{
            if hit(node,&format!("{BASE}/BgPanel/MapGroup/CoopDivision/{name}")){self.division=(self.division+delta)%DIVISIONS.len();self.changed=true;self.refresh(node);return;}
        }
        for i in 1..4{
            let path=format!("{BASE}/CoopPlayerInfo/PlayerSlot{}",i+1);
            if hit(node,&format!("{path}/EmptySlot/CloseSlotBtn"))||hit(node,&format!("{path}/ClosedSlot/OpenSlotBtn")){self.closed[i]=!self.closed[i];self.changed=true;self.refresh(node);return;}
        }
    }
}

#[cfg(test)]
mod tests{
    use super::*;
    use serde_json::json;
    #[test]
    fn dropdown_release_changes_difficulty_and_discards_incompatible_mode(){
        let dropdown=|name:&str|json!({"name":name,"children":[{"name":"Text"},{"name":"Template","children":[
            {"name":"Viewport","children":[{"name":"Content","children":[{"name":"Item","rect":{"size":[0,120]},"children":[{"name":"Item Label"},{"name":"Item Selected"}]}]}]}]}]});
        let mut node=Node::from_json(&json!({"name":"Scene","children":[{"name":"Canvas","children":[{"name":"CoopLobbyScreen","children":[
            {"name":"BgPanel","children":[{"name":"StandardMatchUI","children":[dropdown("DiffDropdown"),dropdown("GameModeDropdown")]}]}]}]}]}));
        fn enable(n:&mut Node){n.active=true;for c in &mut n.children{enable(c);}}
        enable(&mut node);
        let mut lobby=Lobby::new(&mut node);lobby.difficulty=2;lobby.mode=6;lobby.open=Some(0);lobby.refresh(&mut node);
        let row=format!("{SETTINGS}/DiffDropdown/Template/Viewport/Content/Option0");
        node.find_mut(&row).unwrap().screen=Rect::new(10.0,10.0,100.0,40.0);
        lobby.click(&mut node,vec2(20.0,20.0));
        assert_eq!((lobby.difficulty,lobby.mode),(0,0));assert!(lobby.open.is_none());
        assert_eq!(node.find(&format!("{SETTINGS}/GameModeDropdown/Template/Viewport/Content")).unwrap().children.len(),3);
        assert!(!node.find(&format!("{SETTINGS}/DiffDropdown/Template")).unwrap().active);
    }
}
