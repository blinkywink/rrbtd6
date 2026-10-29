//! Native social menu tabs, text fields and information dialogs. No online mutations.
use super::{apply_animation,ScrollInput};
use crate::uitree::Node;
use macroquad::prelude::*;
use serde_json::Value;
use std::path::Path;
struct Field {path:String,text_path:String,placeholder:Option<String>,text:String,limit:usize,focused:bool}
struct Motion {index:usize,began:f64,closing:bool}
pub(super) struct SocialPage {config:Value,runtime:Value,modal:Option<usize>,motion:Option<Motion>,fields:Vec<Field>,
    /// LinkFriendsScreen: downloadedAt of the account last bound (None until bound).
    friends:Option<f64>,file:String}
const FRIEND_CODE:&str="Canvas/LinkFriends/Container/TopBar/FriendCodePanal";
const FRIEND_REQUESTS:&str="Canvas/LinkFriends/Container/TopBar/Left/FriendRequestToggle";

/// LiNKFriends.GetMyFriendCode: upper-cased, "XXXXX-rest" once it has at least 6 characters.
fn friend_code(code:&str)->String {
    if code.chars().count()<6 {return code.to_owned();}
    let upper=code.to_uppercase();
    let (head,tail)=upper.split_at(upper.char_indices().nth(5).map_or(upper.len(),|(i,_)|i));
    format!("{head}-{tail}")
}
impl SocialPage {
    pub fn load(root:&Path,file:&str,node:&mut Node)->Option<Self> {
        if !matches!(file,"LinkFriendsUI"|"TeamsBrowserUI"|"PerksUI"){return None;}
        let controls:Value=serde_json::from_str(&std::fs::read_to_string(root.join("ui/runtime/social_controls.json")).ok()?).ok()?;
        let config=controls[file].clone();if !config.is_object(){return None;}
        for state in config["initial"].as_array().into_iter().flatten(){if let Some(path)=state["path"].as_str(){node.set_active(path,state["on"].as_bool().unwrap_or(false));}}
        for text in config["text"].as_array().into_iter().flatten(){if let (Some(path),Some(text))=(text["path"].as_str(),text["text"].as_str()){node.set_text(path,text);}}
        let mut runtime:Value=std::fs::read_to_string(root.join(format!("ui/runtime/{file}.json"))).ok().and_then(|s|serde_json::from_str(&s).ok()).unwrap_or_default();
        if let Some(animators)=runtime["animators"].as_array_mut(){animators.retain(|a|config["dialogs"].as_array().into_iter().flatten().any(|d|d["controller"]==a["controller"] && d["panel"]==a["path"]));}
        let fields=config["inputs"].as_array().into_iter().flatten().filter_map(|f| {
            let field=Field {path:f["path"].as_str()?.into(),text_path:f["text_path"].as_str()?.into(),placeholder:f["placeholder"].as_str().map(str::to_owned),text:String::new(),
                limit:f["limit"].as_u64().filter(|n|*n>0).map_or(usize::MAX,|n|n as usize),focused:false};
            node.set_text(&field.text_path,"");Some(field)
        }).collect();
        // Toggle.graphic ("Off" slash) is shown only while isOn; both scene toggles start off.
        if file=="LinkFriendsUI" {
            node.set_active(&format!("{FRIEND_CODE}/CopyBtn/IosShareImg"),false);
            node.set_active(&format!("{FRIEND_CODE}/CopyBtn/AndroidShareImg"),false);
            for toggle in [format!("{FRIEND_REQUESTS}/Off"),format!("{FRIEND_CODE}/HideCodeToggle/Off")] {node.set_active(&toggle,false);}
        }
        let mut page=Self {config,runtime,modal:None,motion:None,fields,friends:None,file:file.to_owned()};
        if let Some(tab)=std::env::var("RRBTD6_SOCIAL_TAB").ok().and_then(|s|s.parse().ok()){page.tab(node,tab);}
        if let Some(dialog)=std::env::var("RRBTD6_SOCIAL_DIALOG").ok().and_then(|s|s.parse().ok()){page.open(node,dialog);}
        Some(page)
    }
    pub fn modal_path(&self)->Option<&str>{self.config["dialogs"][self.modal?]["panel"].as_str()}
    fn tab(&mut self,node:&mut Node,index:usize) {
        let Some(tabs)=self.config["tabs"].as_array().filter(|tabs|index<tabs.len())else{return};
        for (i,tab) in tabs.iter().enumerate(){for key in ["panel","highlight"]{if let Some(path)=tab[key].as_str(){node.set_active(path,i==index);}}}
        for text in tabs[index]["text"].as_array().into_iter().flatten(){if let (Some(path),Some(text))=(text["path"].as_str(),text["text"].as_str()){node.set_text(path,text);}}
    }
    fn open(&mut self,node:&mut Node,index:usize) {
        let Some(dialog)=self.config["dialogs"].as_array().and_then(|d|d.get(index))else{return};
        let Some(panel)=dialog["panel"].as_str()else{return};
        node.set_active(panel,true);self.modal=Some(index);self.motion=Some(Motion {index,began:get_time(),closing:false});
    }
    fn close(&mut self,node:&mut Node) {
        let Some(index)=self.modal else{return};let dialog=&self.config["dialogs"][index];
        for f in &mut self.fields {f.focused=false;}
        if dialog["close_clip"].is_string(){self.motion=Some(Motion {index,began:get_time(),closing:true});}
        else{if let Some(panel)=dialog["panel"].as_str(){node.set_active(panel,false);}self.modal=None;self.motion=None;}
    }
    pub fn dismiss(&mut self,node:&mut Node)->bool {
        if self.modal.is_some(){self.close(node);return true;}
        let focused=self.fields.iter().any(|f|f.focused);for f in &mut self.fields{f.focused=false;}focused
    }
    pub fn animate(&mut self,node:&mut Node) {
        if self.file=="LinkFriendsUI" {self.bind_friends(node);}
        let Some(motion)=&self.motion else{return};let dialog=&self.config["dialogs"][motion.index];
        let controller=dialog["controller"].as_str().unwrap_or("");let clip=dialog[if motion.closing{"close_clip"}else{"open_clip"}].as_str().unwrap_or("");
        if apply_animation(node,&self.runtime,controller,clip,Some((get_time()-motion.began) as f32)) {
            if motion.closing{if let Some(panel)=dialog["panel"].as_str(){node.set_active(panel,false);}self.modal=None;}
            self.motion=None;
        }
    }
    pub fn input(&mut self,node:&mut Node,input:&ScrollInput) {
        let modal=self.modal_path().map(str::to_owned);
        for field in &mut self.fields {
            if input.pressed {field.focused=modal.as_ref().is_none_or(|p|field.path.starts_with(p)) && node.rect(&field.path).is_some_and(|r|r.contains(input.pointer));}
            if field.focused {
                while let Some(c)=get_char_pressed(){if !c.is_control() && field.text.chars().count()<field.limit{field.text.push(c);}}
                if is_key_pressed(KeyCode::Backspace){field.text.pop();}
                if is_key_pressed(KeyCode::Enter){field.focused=false;}
                node.set_text(&field.text_path,&field.text);
                if let Some(path)=&field.placeholder{node.set_active(path,field.text.is_empty());}
            }
        }
    }
    /// LinkFriendsScreen.Open: friendCodePanel / friendCodeHiddenPanel from friendScreenShowCode,
    /// friendRequestToggle.isOn from toggleFriendRequests, and the player's friend code.
    fn bind_friends(&mut self,node:&mut Node) {
        let Some((show,requests,code,stamp))=super::account_preview::friends_profile() else {return};
        if self.friends==Some(stamp) {return;}
        self.friends=Some(stamp);
        let show=show && !code.is_empty();
        node.set_active(&format!("{FRIEND_CODE}/CopyBtn"),!code.is_empty());
        node.set_active(&format!("{FRIEND_CODE}/HideCodeToggle"),!code.is_empty());
        node.set_active(&format!("{FRIEND_CODE}/FriendCodeGroup"),show);
        node.set_active(&format!("{FRIEND_CODE}/FriendCodeHidden"),!show);
        node.set_active(&format!("{FRIEND_REQUESTS}/Off"),requests);
        node.set_text(&format!("{FRIEND_CODE}/FriendCodeGroup/FriendCode"),&friend_code(&code));
    }

    pub fn click(&mut self,node:&mut Node,pointer:Vec2)->bool {
        // OnToggleYourFriendCodeVisibility: swap the code / hidden panels; the toggle's isOn
        // becomes the previous visibility. (The game also saves friendScreenShowCode.)
        if self.file=="LinkFriendsUI" && self.modal.is_none() && node.rect(&format!("{FRIEND_CODE}/HideCodeToggle")).is_some_and(|r|r.contains(pointer)) {
            let shown=node.find(&format!("{FRIEND_CODE}/FriendCodeGroup")).is_some_and(|n|n.active);
            node.set_active(&format!("{FRIEND_CODE}/FriendCodeGroup"),!shown);
            node.set_active(&format!("{FRIEND_CODE}/FriendCodeHidden"),shown);
            node.set_active(&format!("{FRIEND_CODE}/HideCodeToggle/Off"),shown);
            return false;
        }
        let hit=|v:&Value|v.as_array().into_iter().flatten().filter_map(Value::as_str).any(|p|node.rect(p).is_some_and(|r|r.contains(pointer)));
        if let Some(index)=self.modal {
            let dialog=&self.config["dialogs"][index];
            let on_surface=dialog["surface"].as_str().and_then(|p|node.rect(p)).is_some_and(|r|r.contains(pointer));
            if hit(&dialog["close"]) || (!on_surface && hit(&dialog["outside"])){self.close(node);}return false;
        }
        if let Some(index)=self.config["dialogs"].as_array().into_iter().flatten().position(|d|hit(&d["open"])){self.open(node,index);return false;}
        if let Some(index)=self.config["tabs"].as_array().into_iter().flatten().position(|t|t["button"].as_str().and_then(|p|node.rect(p)).is_some_and(|r|r.contains(pointer))) {self.tab(node,index);return true;}
        false
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    #[test]
    fn modal_content_blocks_background_dismiss_and_done_closes_it() {
        let mut node=Node::from_json(&json!({"name":"Scene","children":[{"name":"Modal","children":[
            {"name":"Background"},{"name":"Panel"},{"name":"Done"}]}]}));
        node.active=true;node.find_mut("Modal").unwrap().active=true;
        for path in ["Modal/Background","Modal/Panel","Modal/Done"]{node.find_mut(path).unwrap().active=true;}
        node.find_mut("Modal/Background").unwrap().screen=Rect::new(0.0,0.0,100.0,100.0);
        node.find_mut("Modal/Panel").unwrap().screen=Rect::new(20.0,20.0,60.0,60.0);
        node.find_mut("Modal/Done").unwrap().screen=Rect::new(40.0,70.0,20.0,10.0);
        let config=json!({"dialogs":[{"panel":"Modal","surface":"Modal/Panel","outside":["Modal/Background"],"close":["Modal/Done"],"close_clip":null}]});
        let mut page=SocialPage {config,runtime:Value::Null,modal:Some(0),motion:None,fields:Vec::new(),friends:None,file:String::new()};
        assert_eq!(friend_code("abcdefghi"),"ABCDE-FGHI");assert_eq!(friend_code("abc"),"abc");
        page.click(&mut node,vec2(30.0,30.0));assert_eq!(page.modal,Some(0));
        page.click(&mut node,vec2(45.0,75.0));assert_eq!(page.modal,None);
        assert!(node.rect("Modal").is_none());
    }
}
