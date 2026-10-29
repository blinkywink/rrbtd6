//! Native menu widgets for the client's data-only mod catalogue.
use crate::{modding::{self,Catalog,SandboxRules},uitree::Node};
use macroquad::prelude::*;
use std::{collections::HashSet,path::{Path,PathBuf}};
const MODS_BUTTON_SPRITE:&str="MainMenuUiAtlas/CreateChallengeIcon";
const BASE:&str="Canvas/JukeboxScreen";
const LIST:&str="Canvas/JukeboxScreen/AvailableTracks/Scroll/Container/Content";
const DETAILS:&str="Canvas/JukeboxScreen/Playlist/Scroll/Container/Content";
const FOOTER:&str="Canvas/JukeboxScreen/TrackPlayer";
const MUTED:Color=Color::new(0.66,0.82,0.82,1.0);
const GOOD:Color=Color::new(0.57,1.0,0.25,1.0);
const BAD:Color=Color::new(1.0,0.69,0.36,1.0);
#[derive(Clone)]
struct Card {key:String,id:Option<String>,name:String,version:String,description:String,error:Option<String>,rules:SandboxRules,missing:bool}
fn cards(catalog:&Catalog,enabled:&HashSet<String>)->Vec<Card>{
    let mut found=HashSet::new();
    let mut rows:Vec<_>=catalog.entries.iter().map(|e|match &e.manifest{
        Ok(m)=>{found.insert(m.id.clone());Card{key:m.id.clone(),id:Some(m.id.clone()),name:m.name.clone(),version:m.version.clone(),
            description:m.description.clone(),error:None,rules:m.on_new_sandbox.clone(),missing:false}},
        Err(error)=>Card{key:format!("invalid:{}",e.folder),id:None,name:e.folder.clone(),version:String::new(),description:String::new(),
            error:Some(error.clone()),rules:Default::default(),missing:false},
    }).collect();
    let mut missing:Vec<_>=enabled.difference(&found).cloned().collect();missing.sort();
    for id in missing{rows.push(Card{key:format!("missing:{id}"),id:Some(id.clone()),name:id,version:String::new(),description:String::new(),
        error:Some("This enabled mod is missing or invalid. Restore its files, or remove it from the enabled list below.".into()),rules:Default::default(),missing:true});}
    rows
}
pub(super) struct Browser {
    root:PathBuf,catalog:Catalog,selected:HashSet<String>,rows:Vec<Card>,focus:Option<usize>,
    template:Node,button:Node,body:Node,title:Node,notice:Option<String>,pub reset_details:bool,pub pending:Option<Vec<String>>,
}
fn text_all(node:&mut Node,text:&str){if let Some(t)=&mut node.txt{t.text=text.into();}for child in &mut node.children{text_all(child,text);}}
fn place(node:&mut Node,anchor:Vec2,pivot:Vec2,pos:Vec2,size:Vec2){
    node.rt.amin=anchor;node.rt.amax=anchor;node.rt.pivot=pivot;node.rt.pos=pos;node.rt.size=size;node.rt.scale=Vec2::ONE;
}
fn block()->Node{Node::from_json(&serde_json::json!({"name":"Block","active":true}))}
impl Browser {
    pub fn load(root:&Path,node:&mut Node)->Option<Self>{
        let template=Node::load(&root.join("ui/prefabs/JukeboxTrack.json"))?;
        let button=node.find(&format!("{BASE}/AvailableTracks/AddAllBtn/AddAllBtn"))?.clone();
        let body=template.find("TrackName")?.clone();let title=node.find(&format!("{BASE}/AvailableTracks/Title"))?.clone();
        for path in ["BlockClicks","JewelBar","Playlist/EditTrack","AvailableTracks/FoundNothingTxt","Playlist/FoundNothingTxt"]{node.set_active(&format!("{BASE}/{path}"),false);}
        node.set_text(&format!("{BASE}/TopInfo/Title"),"MODS");
        node.set_text(&format!("{BASE}/AvailableTracks/Title"),"INSTALLED MODS");
        node.set_text(&format!("{BASE}/Playlist/Title"),"MOD DETAILS");
        for (path,text) in [("AvailableTracks/AddAllBtn/AddAllBtn","Open Mods Folder"),("Playlist/RemoveAllBtn/RemoveAllBtn","Reload Mods")]{
            if let Some(b)=node.find_mut(&format!("{BASE}/{path}")){text_all(b,text);b.rt.size.x=600.0;
                if let Some(t)=b.find_mut("Text"){t.rt.size.x=550.0;}}
        }
        let mut browser=Self{root:root.to_owned(),catalog:Catalog::scan(&modding::directory(root)),selected:modding::enabled(root),
            rows:Vec::new(),focus:None,template,button,body,title,notice:None,reset_details:false,pending:None};
        browser.rebuild(node);Some(browser)
    }
    pub fn reopen(&mut self,node:&mut Node){
        self.catalog=Catalog::scan(&modding::directory(&self.root));self.selected=modding::enabled(&self.root);
        self.notice=None;self.rebuild(node);
    }
    fn text(&self,name:&str,value:&str,height:f32,title:bool)->Node{
        let mut n=if title{self.title.clone()}else{self.body.clone()};n.name=name.into();n.active=true;
        place(&mut n,vec2(0.5,1.0),vec2(0.5,1.0),Vec2::ZERO,vec2(1600.0,height));
        n.le_pref=vec2(1600.0,height);n.le_min=vec2(-1.0,height);n.le_flex=vec2(-1.0,-1.0);
        if let Some(t)=&mut n.txt{t.text=value.into();t.h=1;t.v=512;t.wrap=true;t.auto=false;t.size=if title{72.0}else{55.0};t.color=if title{WHITE}else{MUTED};}
        n
    }
    fn rebuild(&mut self,node:&mut Node){
        let key=self.focus.and_then(|i|self.rows.get(i)).map(|r|r.key.clone());
        self.rows=cards(&self.catalog,&self.selected);
        self.focus=key.and_then(|k|self.rows.iter().position(|r|r.key==k)).or_else(||(!self.rows.is_empty()).then_some(0));
        let mut list=Vec::new();
        for (i,card) in self.rows.iter().enumerate(){
            let mut row=self.template.clone();row.name=format!("Mod{i}");row.active=true;
            row.le_pref.y=250.0;row.rt.size.y=250.0;
            for path in ["TrackEQ","TrophyLockBtn","AddButton","RemoveButton"]{row.set_active(path,false);}
            row.set_active("SelectTrackButton/BtnImg",self.focus==Some(i));
            row.set_sprite("TrackIcon",Some(MODS_BUTTON_SPRITE));
            if let Some(icon)=row.find_mut("TrackIcon"){icon.rt.pos.x=120.0;icon.rt.size=vec2(145.0,145.0);}
            row.set_text("TrackName",&card.name);
            if let Some(name)=row.find_mut("TrackName"){
                place(name,vec2(0.0,0.5),vec2(0.0,0.5),vec2(235.0,50.0),vec2(1370.0,135.0));
                if let Some(t)=&mut name.txt{t.size=62.0;t.auto=true;t.min=42.0;t.max=62.0;t.wrap=true;}
            }
            let on=card.id.as_ref().is_some_and(|id|self.selected.contains(id));
            let status=if card.error.is_some(){"Needs attention"}else if on{"Enabled"}else{"Disabled"};
            let mut state=self.body.clone();state.name="State".into();
            place(&mut state,vec2(0.0,0.5),vec2(0.0,0.5),vec2(235.0,-70.0),vec2(1370.0,70.0));
            if let Some(t)=&mut state.txt{t.text=status.into();t.size=43.0;t.auto=false;t.color=if card.error.is_some(){BAD}else if on{GOOD}else{MUTED};}
            row.children.push(state);list.push(row);
        }
        if list.is_empty(){list.push(self.text("Empty","No mods installed yet.\nOpen the Mods folder, add a mod, then press Reload Mods.",360.0,false));}
        if let Some(content)=node.find_mut(LIST){content.children=list;}
        self.details(node);
        self.footer(node);
    }
    fn details(&self,node:&mut Node){
        let mut children=Vec::new();
        if let Some(card)=self.focus.and_then(|i|self.rows.get(i)){
            let mut header=block();header.name="Header".into();header.rt.size=vec2(1600.0,300.0);header.le_pref=vec2(1600.0,300.0);
            if let Some(mut icon)=self.template.find("TrackIcon").cloned(){
                icon.name="Icon".into();
                if let Some(img)=&mut icon.img{img.sprite=Some(MODS_BUTTON_SPRITE.into());}
                place(&mut icon,vec2(0.0,1.0),vec2(0.5,0.5),vec2(150.0,-145.0),vec2(230.0,230.0));header.children.push(icon);
            }
            let mut title=self.text("Name",&card.name,130.0,true);
            place(&mut title,vec2(0.0,1.0),vec2(0.0,0.5),vec2(310.0,-110.0),vec2(1270.0,180.0));
            if let Some(t)=&mut title.txt{t.auto=true;t.min=42.0;t.max=78.0;t.wrap=true;}
            header.children.push(title);
            let version=if card.version.is_empty(){"Local mod".into()}else{format!("Version {}  ·  Sandbox",card.version)};
            let mut sub=self.text("Version",&version,90.0,false);
            place(&mut sub,vec2(0.0,1.0),vec2(0.0,0.5),vec2(310.0,-245.0),vec2(1270.0,90.0));header.children.push(sub);children.push(header);
            let on=card.id.as_ref().is_some_and(|id|self.selected.contains(id));
            let mut state=self.text("Status",if card.error.is_some(){"Needs attention"}else if on{"Enabled for your next Sandbox"}else{"Disabled"},100.0,false);
            if let Some(t)=&mut state.txt{t.color=if card.error.is_some(){BAD}else if on{GOOD}else{MUTED};}children.push(state);
            let description=card.error.as_deref().unwrap_or(&card.description);
            if !description.is_empty(){
                let height=((description.chars().count()/45+1) as f32*70.0).max(160.0);
                let mut desc=self.text("Description",description,height,false);if let Some(t)=&mut desc.txt{t.v=256;}if card.error.is_some(){if let Some(t)=&mut desc.txt{t.color=BAD;}}children.push(desc);
            }
            if card.error.is_none(){
                children.push(self.text("EffectsTitle","WHAT IT CHANGES",110.0,true));
                let mut effects=Vec::new();
                if let Some(cash)=card.rules.starting_cash{effects.push(format!("Starting cash: ${}",super::account_preview::number(cash)));}
                if let Some(lives)=card.rules.starting_lives{effects.push(format!("Starting lives: {}",super::account_preview::number(lives)));}
                if effects.is_empty(){effects.push("No gameplay changes in this mod.".into());}
                let height=effects.len() as f32*85.0;children.push(self.text("Effects",&effects.join("\n"),height,false));
            }
        }else{children.push(self.text("EmptyDetails","Select a mod to see its description and effects.",300.0,false));}
        if let Some(content)=node.find_mut(DETAILS){content.children=children;}
    }
    fn footer(&self,node:&mut Node){
        let installed=self.catalog.entries.len();
        let invalid=self.catalog.entries.iter().filter(|e|e.manifest.is_err()).count();
        let issue=self.notice.clone().or_else(||self.catalog.compile(&self.selected).err());
        let status=issue.clone().unwrap_or_else(||format!("{installed} installed  ·  {} enabled{}\nChanges apply when you start a new Sandbox.",self.selected.len(),if invalid>0{format!("  ·  {invalid} needs attention")}else{String::new()}));
        let mut status_node=self.text("Status",&status,210.0,false);
        place(&mut status_node,vec2(0.0,0.5),vec2(0.0,0.5),vec2(75.0,0.0),vec2(1900.0,220.0));
        if let Some(t)=&mut status_node.txt{t.size=48.0;if issue.is_some(){t.color=BAD;}}
        let mut action=self.button.clone();action.name="Action".into();action.active=false;
        place(&mut action,vec2(1.0,0.5),vec2(1.0,0.5),vec2(-40.0,0.0),vec2(800.0,180.0));
        if let Some(text)=action.find_mut("Text"){text.rt.size.x=740.0;if let Some(t)=&mut text.txt{t.size=64.0;t.min=48.0;t.max=64.0;t.auto=true;}}
        if let Some(card)=self.focus.and_then(|i|self.rows.get(i)){
            let enabled=card.id.as_ref().is_some_and(|id|self.selected.contains(id));
            action.active=card.id.is_some()&&(card.error.is_none()||enabled);
            text_all(&mut action,if card.missing{"Remove Missing Mod"}else if enabled{"Disable Mod"}else{"Enable Mod"});
            if let Some(img)=&mut action.img{img.sprite=Some(if enabled{"RedBtnLong"}else{"GreenBtnLong"}.into());}
        }
        if let Some(footer)=node.find_mut(FOOTER){footer.children=vec![status_node,action];}
    }
    pub fn click(&mut self,node:&mut Node,pointer:Vec2)->bool{
        let hit=|path:&str|node.rect(path).is_some_and(|r|r.contains(pointer));
        if hit(&format!("{BASE}/AvailableTracks/AddAllBtn/AddAllBtn")){
            self.notice=open_folder(&modding::directory(&self.root)).err();self.footer(node);return true;
        }
        if hit(&format!("{BASE}/Playlist/RemoveAllBtn/RemoveAllBtn")){
            self.catalog=Catalog::scan(&modding::directory(&self.root));self.notice=None;self.reset_details=true;self.rebuild(node);return true;
        }
        if hit(&format!("{FOOTER}/Action")){
            let Some(id)=self.focus.and_then(|i|self.rows.get(i)).and_then(|r|r.id.clone()) else{return true;};
            if !self.selected.remove(&id){self.selected.insert(id);}
            let mut ids:Vec<_>=self.selected.iter().cloned().collect();ids.sort();self.pending=Some(ids);self.notice=None;self.rebuild(node);return true;
        }
        if let Some(i)=(0..self.rows.len()).find(|i|hit(&format!("{LIST}/Mod{i}"))){self.reset_details=self.focus!=Some(i);self.focus=Some(i);self.rebuild(node);return true;}
        false
    }
}
fn open_folder(path:&Path)->Result<(),String>{
    std::fs::create_dir_all(path).map_err(|_|"Could not create the Mods folder.".to_owned())?;
    #[cfg(target_os="windows")] let mut command=std::process::Command::new("explorer.exe");
    #[cfg(target_os="macos")] let mut command=std::process::Command::new("open");
    #[cfg(not(any(target_os="windows",target_os="macos")))] let mut command=std::process::Command::new("xdg-open");
    command.arg(path).spawn().map(|_|()).map_err(|_|"Could not open the Mods folder. It is next to BTD6.exe.".into())
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
    fn fixture(entries:Vec<Entry>,selected:HashSet<String>)->(Browser,Node){
        fn chain(parts:&[&str])->serde_json::Value{
            let mut n=json!({"name":parts[0],"active":true});
            if parts.len()>1{n["children"]=json!([chain(&parts[1..])]);}n
        }
        let mut canvas=chain(&["Scene","Canvas","JukeboxScreen"]);
        canvas["children"][0]["children"][0]["children"]=json!([
            chain(&["AvailableTracks","Scroll","Container","Content"]),chain(&["Playlist","Scroll","Container","Content"]),
            {"name":"TrackPlayer","active":true}]);
        let node=Node::from_json(&canvas);
        let body=Node::from_json(&json!({"name":"Text","active":true,"comps":[{"type":"TextMeshProUGUI","enabled":true,"m_text":"","m_fontSize":55}]}));
        let mut template=block();template.children=vec![body.clone()];template.children[0].name="TrackName".into();
        let button=block();
        let mut browser=Browser{root:PathBuf::new(),catalog:Catalog{entries},selected,rows:Vec::new(),focus:None,
            template,button,body:body.clone(),title:body,notice:None,reset_details:false,pending:None};
        let mut node=node;browser.rebuild(&mut node);(browser,node)
    }
    fn entry(id:&str)->Entry{
        Entry{folder:id.into(),manifest:Manifest::parse(&serde_json::to_vec(&json!({"api_version":1,"id":id,"name":id,"version":"1",
            "on_new_sandbox":{"starting_cash":100000}})).unwrap())}
    }
    fn click(browser:&mut Browser,node:&mut Node,path:&str){
        node.find_mut(path).unwrap().screen=Rect::new(10.0,10.0,40.0,40.0);
        assert!(browser.click(node,vec2(20.0,20.0)));
        // A real layout replaces every rect after the action.
        if let Some(n)=node.find_mut(path){n.screen=Rect::default();}
    }
    #[test]
    fn selecting_a_mod_is_separate_from_enabling_it_and_conflicts_are_visible(){
        let (mut browser,mut node)=fixture(vec![entry("a"),entry("b")],HashSet::new());
        click(&mut browser,&mut node,&format!("{LIST}/Mod1"));
        assert_eq!(browser.focus,Some(1));assert!(std::mem::take(&mut browser.reset_details));assert!(browser.pending.is_none());assert!(browser.selected.is_empty());
        click(&mut browser,&mut node,&format!("{FOOTER}/Action"));
        assert_eq!(browser.pending.take().unwrap(),vec!["b"]);
        click(&mut browser,&mut node,&format!("{LIST}/Mod0"));
        click(&mut browser,&mut node,&format!("{FOOTER}/Action"));
        assert_eq!(browser.pending.take().unwrap(),vec!["a","b"]);
        assert!(node.find(&format!("{FOOTER}/Status")).unwrap().txt.as_ref().unwrap().text.contains("More than one"));
        click(&mut browser,&mut node,&format!("{FOOTER}/Action"));
        assert_eq!(browser.pending.take().unwrap(),vec!["b"]);
        assert!(!node.find(&format!("{FOOTER}/Status")).unwrap().txt.as_ref().unwrap().text.contains("More than one"));
    }
    #[test]
    fn missing_enabled_mod_can_be_removed_and_invalid_mod_cannot_be_enabled(){
        let (mut browser,mut node)=fixture(vec![],HashSet::from(["gone".into()]));
        assert!(browser.rows[0].missing);assert!(node.find(&format!("{FOOTER}/Action")).unwrap().active);
        click(&mut browser,&mut node,&format!("{FOOTER}/Action"));
        assert!(browser.pending.take().unwrap().is_empty());assert!(browser.rows.is_empty());
        let (_,node)=fixture(vec![Entry{folder:"broken".into(),manifest:Err("Invalid JSON".into())}],HashSet::new());
        assert!(!node.find(&format!("{FOOTER}/Action")).unwrap().active);
    }
}
