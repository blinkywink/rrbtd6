//! Native settings widgets for the client's data-only mod catalogue.
use crate::{modding::{self,Catalog},uitree::Node};
use macroquad::prelude::*;
use std::{collections::HashSet,path::{Path,PathBuf}};
const MODS_BUTTON_SPRITE:&str="MainMenuUiAtlas/CreateChallengeIcon";
const CONTENT:&str="Canvas/AccessibilitySettings/Panel/Scroll/Viewport/Content";
pub(super) struct Browser {
    root:PathBuf,catalog:Catalog,selected:HashSet<String>,rows:Vec<Option<String>>,
    template:Node,reload:Node,pub pending:Option<Vec<String>>,
}
fn text_all(node:&mut Node,text:&str){if let Some(t)=&mut node.txt{t.text=text.into();}for child in &mut node.children{text_all(child,text);}}
impl Browser {
    pub fn load(root:&Path,node:&mut Node)->Option<Self>{
        let template=node.find(&format!("{CONTENT}/CashSpent"))?.clone();
        let jukebox=Node::load(&root.join("ui/prefabs/JukeboxUI.json"))?;
        let mut reload=jukebox.find("Canvas/JukeboxScreen/AvailableTracks/AddAllBtn/AddAllBtn")?.clone();
        reload.name="Reload".into();reload.active=true;text_all(&mut reload,"Reload Mods");
        reload.rt.amin=vec2(0.5,0.5);reload.rt.amax=reload.rt.amin;reload.rt.pivot=vec2(0.5,0.5);
        reload.rt.pos=Vec2::ZERO;reload.rt.size=vec2(600.0,160.0);
        let mut browser=Self{root:root.to_owned(),catalog:Catalog::scan(&modding::directory(root)),selected:modding::enabled(root),
            rows:Vec::new(),template,reload,pending:None};
        if let Some(panel)=node.find_mut("Canvas/AccessibilitySettings").and_then(|n|n.common.as_mut()){panel.heading="Mods".into();}
        node.set_active("Canvas/AccessibilitySettings/Panel/Scroll/Viewport",true);
        if let Some(content)=node.find_mut(CONTENT){content.rt.amin.y=1.0;content.rt.amax.y=1.0;}
        browser.rebuild(node);Some(browser)
    }
    fn row(&self,name:&str,label:&str,desc:&str,on:bool,toggle:bool)->Node{
        let mut row=self.template.clone();row.name=name.into();row.active=true;
        row.set_text("ToggleGroup/Anim/TxtOn",label);row.set_text("Description/Text",desc);
        super::advanced_toggle(&mut row,"ToggleGroup/Anim",on);
        if !toggle{row.set_text_color("ToggleGroup/Anim/TxtOn",WHITE);row.set_active("ToggleGroup/Anim/HitBox",false);row.set_active("ToggleGroup/Anim/Bg",false);row.set_active("ToggleGroup/Anim/BtnOn",false);}
        row
    }
    fn rebuild(&mut self,node:&mut Node){
        let status=match self.catalog.compile(&self.selected){
            Ok(_)=>format!("{} enabled. Changes apply to your next local Sandbox. Normal games, events and co-op are unchanged.",self.selected.len()),
            Err(error)=>format!("Mods will not apply: {error}"),
        };
        let mut children=vec![self.row("Summary","MODS · API 1",&status,false,false)];
        let mut toolbar=Node::from_json(&serde_json::json!({"name":"Toolbar","active":true,"rect":{"size":[2400,180],"scale":[1,1],"pivot":[0.5,0.5]},
            "comps":[{"type":"LayoutElement","m_PreferredHeight":180}]}));
        toolbar.children.push(self.reload.clone());children.push(toolbar);self.rows.clear();
        let mut found=HashSet::new();
        for (i,entry) in self.catalog.entries.iter().enumerate(){
            let (label,desc,id)=match &entry.manifest{
                Ok(m)=>{found.insert(m.id.clone());(format!("{}  v{}",m.name,m.version),format!("{}  |  {}",m.id,m.description),Some(m.id.clone()))},
                Err(error)=>(entry.folder.clone(),error.clone(),None),
            };
            let on=id.as_ref().is_some_and(|id|self.selected.contains(id));
            children.push(self.row(&format!("Mod{i}"),&label,&desc,on,id.is_some()));self.rows.push(id);
        }
        let mut missing:Vec<_>=self.selected.difference(&found).cloned().collect();missing.sort();
        for id in missing{let i=self.rows.len();children.push(self.row(&format!("Mod{i}"),&id,"Missing or invalid. Turn this off to remove it from the enabled list.",true,true));self.rows.push(Some(id));}
        if self.catalog.entries.is_empty(){children.push(self.row("Empty","No mods installed","Place each mod's mod.json in its own folder inside mods, next to BTD6.exe. Press Reload Mods after adding or editing files.",false,false));}
        if let Some(content)=node.find_mut(CONTENT){content.children=children;}
    }
    pub fn click(&mut self,node:&mut Node,pointer:Vec2)->bool{
        let hit=|path:&str|node.rect(path).is_some_and(|r|r.contains(pointer));
        if hit(&format!("{CONTENT}/Toolbar/Reload")){
            self.catalog=Catalog::scan(&modding::directory(&self.root));self.rebuild(node);return true;
        }
        let row=(0..self.rows.len()).find(|i|hit(&format!("{CONTENT}/Mod{i}/ToggleGroup/Anim/HitBox")));
        let Some(id)=row.and_then(|i|self.rows[i].clone()) else{return false;};
        if !self.selected.remove(&id){self.selected.insert(id);}
        let mut ids:Vec<_>=self.selected.iter().cloned().collect();ids.sort();self.pending=Some(ids);self.rebuild(node);true
    }
}

/// Add one native-looking menu button once; no images are decoded per frame.
pub(super) fn add_home_button(node:&mut Node,runtime:&mut serde_json::Value){
    const MAIN:&str="MainMenuCanvas/MainMenu";
    if node.find(&format!("{MAIN}/Mods")).is_some(){return;}
    let Some(mut button)=node.find(&format!("{MAIN}/Settings")).cloned() else{return;};
    button.name="Mods".into();button.set_active("Flag",false);button.set_active("NewTowerDot",false);
    // Selected original clipboard/pencil artwork, with native hover/press feedback.
    button.rt.amin=vec2(1.0,0.0);button.rt.amax=button.rt.amin;button.rt.pivot=vec2(1.0,0.0);
    button.rt.pos=vec2(-60.0,75.0);button.rt.size=vec2(315.0,315.0);
    let button_size=button.rt.size;
    if let Some(image)=button.find_mut("Button"){
        image.rt.amin=vec2(0.5,0.5);image.rt.amax=image.rt.amin;image.rt.pivot=vec2(0.5,0.5);
        image.rt.pos=Vec2::ZERO;image.rt.size=button_size;
    }
    button.set_sprite("Button",Some(MODS_BUTTON_SPRITE));
    // Use the same outlined title font and material as the other home captions.
    if let Some(mut label)=node.find(&format!("{MAIN}/BottomButtonGroup/Knowledge/KnowledgeAnim/Button/Text")).cloned(){
        label.name="Text".into();label.active=true;
        label.rt.amin=vec2(0.5,0.0);label.rt.amax=label.rt.amin;label.rt.pivot=vec2(0.5,0.5);
        // Keep the native 90-unit font, 200-unit text box, bottom alignment and
        // character spacing. Fit the caption beside Knowledge on the same baseline.
        label.rt.pos=vec2(0.0,52.5);label.rt.size=vec2(360.0,200.0);
        label.rt.scale=node.find(&format!("{MAIN}/BottomButtonGroup")).map(|n|n.rt.scale).unwrap_or(Vec2::ONE);
        if let Some(text)=&mut label.txt{text.text="MODS".into();}
        if let Some(image)=button.find_mut("Button"){image.children.push(label);}
    }
    button.active=false;
    if let Some(main)=node.find_mut(MAIN){main.children.push(button);}
    // Reuse the authored hover/press clips too; alias bindings just once at load.
    if let Some(animators)=runtime["animators"].as_array_mut(){
        let from=format!("{MAIN}/Settings/Button");let to=format!("{MAIN}/Mods/Button");
        let copies:Vec<_>=animators.iter().filter(|a|a["path"].as_str()==Some(from.as_str())).cloned().map(|mut a|{
            a["path"]=serde_json::json!(to);
            for clip in a["clips"].as_array_mut().into_iter().flatten(){
                for binding in clip["bindings"].as_array_mut().into_iter().flatten(){
                    if let Some(path)=binding["path"].as_str().filter(|p|p.starts_with(&from)){
                        binding["path"]=serde_json::json!(format!("{to}{}",&path[from.len()..]));
                    }
                }
            }
            a
        }).collect();animators.extend(copies);
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use crate::modding::{Entry,Manifest};
    use serde_json::json;
    #[test]
    fn selected_button_is_slightly_larger_in_bottom_right_corner(){
        let mut node=Node::from_json(&json!({"name":"Scene","active":true,"children":[{"name":"MainMenuCanvas","active":true,"children":[
            {"name":"MainMenu","active":true,"children":[{"name":"Settings","active":true,"children":[
                {"name":"Button","active":true,"comps":[{"type":"Image","enabled":true,"sprite":"OptionsBtn"}]}]}]}]}]}));
        add_home_button(&mut node,&mut json!({"animators":[]}));
        let button=node.find("MainMenuCanvas/MainMenu/Mods").unwrap();
        assert_eq!(button.rt.amin,vec2(1.0,0.0));assert_eq!(button.rt.pivot,vec2(1.0,0.0));
        assert_eq!(button.rt.pos,vec2(-60.0,75.0));assert_eq!(button.rt.size,vec2(315.0,315.0));
        let image=button.find("Button").unwrap();assert_eq!(image.rt.size,button.rt.size);
        assert_eq!(image.img.as_ref().unwrap().sprite.as_deref(),Some(MODS_BUTTON_SPRITE));
    }
    #[test]
    fn toggling_native_row_emits_sorted_local_preferences(){
        let mut node=Node::from_json(&json!({"name":"Scene","active":true,"children":[{"name":"Canvas","active":true,"children":[
            {"name":"AccessibilitySettings","active":true,"children":[{"name":"Panel","active":true,"children":[
                {"name":"Scroll","active":true,"children":[{"name":"Viewport","active":true,"children":[{"name":"Content","active":true}]}]}]}]}]}]}));
        let template=Node::from_json(&json!({"name":"Row","active":true,"children":[{"name":"ToggleGroup","active":true,"children":[
            {"name":"Anim","active":true,"children":[{"name":"HitBox","active":true}]}]}]}));
        let manifest=Manifest::parse(br#"{"api_version":1,"id":"practice","name":"Practice","version":"1"}"#).unwrap();
        let mut browser=Browser{root:PathBuf::new(),catalog:Catalog{entries:vec![Entry{folder:"practice".into(),manifest:Ok(manifest)}]},
            selected:HashSet::new(),rows:Vec::new(),template,reload:Node::from_json(&json!({"name":"Reload","active":true})),pending:None};
        browser.rebuild(&mut node);
        let path=format!("{CONTENT}/Mod0/ToggleGroup/Anim/HitBox");
        for expected in [vec!["practice".to_owned()],vec![]]{
            node.find_mut(&path).unwrap().screen=Rect::new(10.0,10.0,40.0,40.0);
            assert!(browser.click(&mut node,vec2(20.0,20.0)));
            assert_eq!(browser.pending.take().unwrap(),expected);
        }
    }
}
