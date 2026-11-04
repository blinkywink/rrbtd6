//! Native menu widgets for the client's content and event-script catalogue.
use crate::{modding::{self,Catalog,SandboxRules,SandboxSettings,Overrides},uitree::Node};
use macroquad::prelude::*;
use std::{collections::HashSet,path::{Path,PathBuf}};
const MODS_BUTTON_SPRITE:&str="MainMenuUiAtlas/CreateChallengeIcon";
const BASE:&str="Canvas/JukeboxScreen";
const LIST:&str="Canvas/JukeboxScreen/AvailableTracks/Scroll/Container/Content";
const DETAILS:&str="Canvas/JukeboxScreen/Playlist/Scroll/Container/Content";
const TOOLS:&str="Canvas/JukeboxScreen/AvailableTracks/BrowserTools";
const FOOTER:&str="Canvas/JukeboxScreen/TrackPlayer";
pub const CONTINUE:&str="Canvas/JukeboxScreen/ContinueBtn";
/// Height taken from the bottom of the details list for the Enable All / Disable All buttons.
const BULK_STRIP:f32=180.0;
const BULK:[(&str,&str,f32);2]=[("EnableAll","Enable All",-390.0),("DisableAll","Disable All",390.0)];
const MUTED:Color=Color::new(0.91,0.96,1.0,1.0);
const GOOD:Color=WHITE;
const BAD:Color=Color::new(1.0,0.69,0.36,1.0);
#[derive(Clone)]
struct Card {key:String,id:Option<String>,name:String,version:String,description:String,error:Option<String>,rules:SandboxRules,settings:SandboxSettings,changes:usize,missing:bool,icon:Option<PathBuf>}
fn cards(root:&Path,catalog:&Catalog,enabled:&HashSet<String>,overrides:&Overrides)->Vec<Card>{
    let mut found=HashSet::new();
    let mut rows:Vec<_>=catalog.entries.iter().map(|e|match &e.manifest{
        Ok(m)=>{found.insert(m.id.clone());Card{key:m.id.clone(),id:Some(m.id.clone()),name:m.name.clone(),version:m.version.clone(),
            description:m.description.clone(),error:None,rules:m.resolved(overrides).unwrap_or_else(|_|m.on_new_sandbox.clone()),settings:m.settings.clone(),changes:m.content.len(),missing:false,icon:mod_icon(root,&e.folder)}},
        Err(error)=>Card{key:format!("invalid:{}",e.folder),id:None,name:e.folder.clone(),version:String::new(),description:String::new(),
            error:Some(error.clone()),rules:Default::default(),settings:Default::default(),changes:0,missing:false,icon:None},
    }).collect();
    let mut missing:Vec<_>=enabled.difference(&found).cloned().collect();missing.sort();
    for id in missing{rows.push(Card{key:format!("missing:{id}"),id:Some(id.clone()),name:id,version:String::new(),description:String::new(),
        error:Some("This enabled mod is missing or invalid. Restore its files, or remove it from the enabled list below.".into()),rules:Default::default(),settings:Default::default(),changes:0,missing:true,icon:None});}
    rows
}
#[derive(Clone,Copy,PartialEq,Eq)]
enum ModFilter{All,Enabled,Disabled}
struct NumberEdit {id:String,key:String,text:String,select_all:bool}
pub(super) struct Browser {
    root:PathBuf,catalog:Catalog,selected:HashSet<String>,overrides:Overrides,rows:Vec<Card>,focus:Option<usize>,
    query:String,search_focus:bool,search_select_all:bool,filter:ModFilter,pub reset_list:bool,edit:Option<NumberEdit>,icons_dirty:bool,template:Node,button:Node,body:Node,title:Node,notice:Option<String>,pub reset_details:bool,pub pending:Option<Vec<String>>,pub pending_settings:Option<Overrides>,room:Option<(bool,Vec<String>)>,
}
fn text_all(node:&mut Node,text:&str){if let Some(t)=&mut node.txt{t.text=text.into();}for child in &mut node.children{text_all(child,text);}}
fn place(node:&mut Node,anchor:Vec2,pivot:Vec2,pos:Vec2,size:Vec2){
    node.rt.amin=anchor;node.rt.amax=anchor;node.rt.pivot=pivot;node.rt.pos=pos;node.rt.size=size;node.rt.scale=Vec2::ONE;
}
fn replace_children(node:&mut Node,mut children:Vec<Node>){
    fn retain_rect(old:&Node,new:&mut Node){new.screen=old.screen;
        for child in &mut new.children{if let Some(previous)=old.children.iter().find(|n|n.name==child.name){retain_rect(previous,child);}}
    }
    for child in &mut children{if let Some(old)=node.children.iter().find(|n|n.name==child.name){retain_rect(old,child);}}
    node.children=children;
}
fn block()->Node{Node::from_json(&serde_json::json!({"name":"Block","active":true}))}
fn panel(name:&str,size:Vec2,color:Color)->Node{
    Node::from_json(&serde_json::json!({"name":name,"active":true,"rect":{"anchor_min":[0.5,0.5],"anchor_max":[0.5,0.5],"pivot":[0.5,0.5],"size":[size.x,size.y],"scale":[1,1]},
        "comps":[{"type":"Image","enabled":true,"sprite":"SmallSquareWhiteRound","m_Type":1,"color":[color.r,color.g,color.b,color.a]}]}))
}
/// Original Extras toggle geometry and ToggleOn/ToggleOff endpoint colors.
fn native_switch(on:bool)->Node{
    let mut toggle=block();toggle.name="Toggle".into();
    let mut track=panel("Bg",vec2(250.0,85.0),if on{Color::from_rgba(129,255,0,255)}else{Color::from_rgba(56,32,15,255)});
    if let Some(img)=&mut track.img{img.sprite=Some("SmallSquareWhite".into());}
    let mut knob=panel("BtnOn",vec2(150.0,150.0),WHITE);
    if let Some(img)=&mut knob.img{img.sprite=Some("BlueBtnCircleSmall".into());img.kind=0;img.preserve=true;}
    knob.rt.pos.x=if on{80.0}else{-80.0};toggle.children=vec![track,knob];toggle
}
fn native_button(node:&mut Node){
    if let Some(img)=&mut node.img{img.sprite=Some("BlueBtnLong".into());img.color=WHITE;img.kind=1;}
    if let Some(label)=node.find_mut("Text").and_then(|n|n.txt.as_mut()){label.color=WHITE;}
}
fn card_sprite(card:&Card)->String{if card.icon.is_some(){format!("mod-browser/{}/icon",card.key)}else{MODS_BUTTON_SPRITE.into()}}
/// Convention: optional icon.png beside mod.json. Never read outside the mod folder.
fn mod_icon(root:&Path,folder:&str)->Option<PathBuf>{
    let base=modding::directory(root).canonicalize().ok()?;
    let dir=base.join(folder).canonicalize().ok()?;if !dir.starts_with(&base){return None;}
    let path=dir.join("icon.png").canonicalize().ok()?;if !path.starts_with(&dir){return None;}
    use std::io::Read;
    let mut file=std::fs::File::open(&path).ok()?;if file.metadata().ok()?.len()>1024*1024{return None;}
    let mut header=[0u8;24];file.read_exact(&mut header).ok()?;
    if &header[..8]!=b"\x89PNG\r\n\x1a\n"||&header[12..16]!=b"IHDR"{return None;}
    let w=u32::from_be_bytes(header[16..20].try_into().ok()?);let h=u32::from_be_bytes(header[20..24].try_into().ok()?);
    if w==0||h==0||w>512||h>512{return None;}Some(path)
}
fn short_preview(text:&str,limit:usize)->String{
    let text=text.split_whitespace().collect::<Vec<_>>().join(" ");
    if text.chars().count()<=limit{return text;}
    let mut preview:String=text.chars().take(limit.saturating_sub(1)).collect();
    if let Some(space)=preview.rfind(' '){if space>preview.len()/2{preview.truncate(space);}}
    preview.push('\u{2026}');preview
}
fn native_theme(node:&mut Node){
    for path in ["Panel (1)","Panel (2)"]{node.set_active(&format!("{BASE}/{path}"),false);}
    for (path,sprite) in [("JukeboxBg","MainBGPanelBlue"),("TopInfo/Panel","MainBGPanelBlue"),
        ("AvailableTracks/InnerBg","BlueInsertPanelRound"),("Playlist/InnerBg","BlueInsertPanelRound"),("TrackPlayer","BlueInsertPanelRound")]{
        node.set_sprite(&format!("{BASE}/{path}"),Some(sprite));
        if let Some(img)=node.find_mut(&format!("{BASE}/{path}")).and_then(|n|n.img.as_mut()){img.color=WHITE;img.kind=1;}
    }
    node.set_active(&format!("{BASE}/TopInfo/Panel"),false);
    if let Some(header)=node.find_mut(&format!("{BASE}/TopInfo")){header.rt.pos.y-=75.0;}
    if let Some(title)=node.find_mut(&format!("{BASE}/TopInfo/Title")){title.rt.pos.x=80.0;title.rt.size.x=650.0;}
    for path in ["AvailableTracks","Playlist"]{
        if let Some(pane)=node.find_mut(&format!("{BASE}/{path}")){pane.rt.size.y+=260.0;pane.rt.pos.y-=130.0;}
        for child in ["InnerBg","Scroll"]{if let Some(n)=node.find_mut(&format!("{BASE}/{path}/{child}")){n.rt.size.y+=260.0;n.rt.pos.y-=130.0;}}
    }
    for path in ["AvailableTracks/AddAllBtn","Playlist/RemoveAllBtn"]{if let Some(n)=node.find_mut(&format!("{BASE}/{path}")){n.rt.pos.y+=55.0;}}
    if let Some(scroll)=node.find_mut(&format!("{BASE}/AvailableTracks/Scroll")){scroll.rt.size.y-=150.0;scroll.rt.pos.y-=75.0;}
    // The details list gives its bottom up to the Enable All / Disable All buttons.
    if let Some(scroll)=node.find_mut(&format!("{BASE}/Playlist/Scroll")){scroll.rt.size.y-=BULK_STRIP;scroll.rt.pos.y+=BULK_STRIP*0.5;}
    if let Some(footer)=node.find_mut(FOOTER){footer.rt.size=vec2(3100.0,155.0);footer.rt.pos.y=-960.0;}
    if let Some(base)=node.find_mut(BASE){
        let mut icon=panel("ModsIcon",vec2(220.0,220.0),WHITE);
        if let Some(img)=&mut icon.img{img.sprite=Some(MODS_BUTTON_SPRITE.into());img.preserve=true;}
        place(&mut icon,vec2(0.5,0.5),vec2(0.5,0.5),vec2(-185.0,775.0),vec2(150.0,150.0));base.children.push(icon);
    }
    if let Some(title)=node.find_mut(&format!("{BASE}/TopInfo/Title")).and_then(|n|n.txt.as_mut()){title.color=GOOD;}
    for part in ["AvailableTracks/Title","Playlist/Title"]{
        if let Some(t)=node.find_mut(&format!("{BASE}/{part}")).and_then(|n|n.txt.as_mut()){t.color=GOOD;t.size=66.0;}
    }
}
impl Browser {
    pub fn load(root:&Path,node:&mut Node)->Option<Self>{
        let template=Node::load(&root.join("ui/prefabs/JukeboxTrack.json"))?;
        let mut button=node.find(&format!("{BASE}/AvailableTracks/AddAllBtn/AddAllBtn"))?.clone();native_button(&mut button);
        let body=template.find("TrackName")?.clone();let title=node.find(&format!("{BASE}/AvailableTracks/Title"))?.clone();
        for path in ["BlockClicks","JewelBar","Playlist/EditTrack","AvailableTracks/FoundNothingTxt","Playlist/FoundNothingTxt"]{node.set_active(&format!("{BASE}/{path}"),false);}
        node.set_text(&format!("{BASE}/TopInfo/Title"),"MODS");
        node.set_text(&format!("{BASE}/AvailableTracks/Title"),"INSTALLED");
        node.set_text(&format!("{BASE}/Playlist/Title"),"DETAILS");
        for (path,text) in [("AvailableTracks/AddAllBtn/AddAllBtn","Open Mods Folder"),("Playlist/RemoveAllBtn/RemoveAllBtn","Reload Mods")]{
            if let Some(b)=node.find_mut(&format!("{BASE}/{path}")){text_all(b,text);native_button(b);b.rt.size.x=600.0;
                if let Some(t)=b.find_mut("Text"){t.rt.size.x=550.0;}}
        }
        native_theme(node);
        // Enable All / Disable All, side by side in the strip under the details list.
        let bottom=node.find(&format!("{BASE}/Playlist/Scroll")).map(|s|s.rt.pos.y-s.rt.size.y*0.5).unwrap_or(-1212.85);
        if let Some(side)=node.find_mut(&format!("{BASE}/Playlist")){
            side.children.retain(|c|!BULK.iter().any(|(name,_,_)|c.name==*name));
            for (name,text,x) in BULK{
                let mut b=button.clone();b.name=name.into();b.active=true;text_all(&mut b,text);native_button(&mut b);
                place(&mut b,vec2(0.5,1.0),vec2(0.5,0.5),vec2(x,bottom-BULK_STRIP*0.5),vec2(700.0,140.0));
                if let Some(t)=b.find_mut("Text"){t.rt.size.x=640.0;}
                side.children.push(b);
            }
        }
        let mut browser=Self{root:root.to_owned(),catalog:Catalog::scan(&modding::directory(root)),selected:modding::enabled(root),overrides:modding::overrides(root),
            rows:Vec::new(),focus:None,query:String::new(),search_focus:false,search_select_all:false,filter:ModFilter::All,reset_list:false,edit:None,icons_dirty:true,template,button,body,title,notice:None,reset_details:false,pending:None,pending_settings:None,room:None};
        browser.rebuild(node);Some(browser)
    }
    /// Create Match (Mods): choosing the mods of a room that doesn't exist yet, finished with
    /// a Continue button (`picked`, then the map select). Nothing is enabled locally.
    pub fn begin_pick(&mut self,node:&mut Node){
        self.bind_room(node,Some((&[],true)));
        node.set_text(&format!("{BASE}/TopInfo/Title"),"GAME MODS");
        if node.find(CONTINUE).is_none(){
            let mut b=self.button.clone();b.name="ContinueBtn".into();b.active=true;text_all(&mut b,"Continue");native_button(&mut b);
            place(&mut b,vec2(1.0,0.0),vec2(1.0,0.0),vec2(-90.0,70.0),vec2(600.0,150.0));
            if let Some(t)=b.find_mut("Text"){t.rt.size.x=550.0;}
            if let Some(base)=node.find_mut(BASE){base.children.push(b);}
        }
        node.set_active(CONTINUE,true);
    }
    pub fn end_pick(&mut self,node:&mut Node){node.set_active(CONTINUE,false);}
    /// The mods ticked while picking, sorted.
    pub fn picked(&self)->Vec<String>{let mut ids:Vec<_>=self.selected.iter().cloned().collect();ids.sort();ids}
    pub fn reopen(&mut self,node:&mut Node){
        node.set_text(&format!("{BASE}/TopInfo/Title"),"MODS");
        node.set_text(&format!("{BASE}/AvailableTracks/Title"),"INSTALLED");
        node.set_active(&format!("{BASE}/AvailableTracks/AddAllBtn"),true);node.set_active(&format!("{BASE}/Playlist/RemoveAllBtn"),true);
        self.catalog=Catalog::scan(&modding::directory(&self.root));self.selected=modding::enabled(&self.root);self.overrides=modding::overrides(&self.root);
        self.edit=None;self.search_focus=false;self.reset_list=true;self.room=None;self.notice=None;self.icons_dirty=true;self.rebuild(node);
    }
    pub fn bind_room(&mut self,node:&mut Node,room:Option<(&[rrbtd6_coop::ModPackage],bool)>){
        let Some((packages,host))=room else{
            if self.room.take().is_some(){self.reopen(node);}return;
        };
        let signature=(host,packages.iter().map(|p|p.digest.clone()).collect::<Vec<_>>());
        if self.room.as_ref()==Some(&signature){return;}
        self.edit=None;self.room=Some(signature);self.overrides=Overrides::new();
        self.catalog=if host{Catalog::scan(&modding::directory(&self.root))}else{Catalog{entries:packages.iter().map(|p|crate::modding::Entry{
            folder:p.id.clone(),manifest:crate::modding::Manifest::parse(p.manifest.as_bytes())}).collect()}};
        self.selected=packages.iter().map(|p|p.id.clone()).collect();self.notice=None;self.icons_dirty=true;self.rebuild(node);
        node.set_text(&format!("{BASE}/TopInfo/Title"),"MODS");
        node.set_text(&format!("{BASE}/AvailableTracks/Title"),if host{"AVAILABLE MODS"}else{"HOST'S MODS"});
        node.set_active(&format!("{BASE}/AvailableTracks/AddAllBtn"),host);
        node.set_active(&format!("{BASE}/Playlist/RemoveAllBtn"),host);
    }
    pub fn report_error(&mut self,node:&mut Node,error:String){self.notice=Some(error);self.footer(node);}
    fn editable(&self)->bool{self.room.as_ref().is_none_or(|(host,_)|*host)}
    fn text(&self,name:&str,value:&str,height:f32,title:bool)->Node{
        let mut n=if title{self.title.clone()}else{self.body.clone()};n.name=name.into();n.active=true;
        place(&mut n,vec2(0.5,1.0),vec2(0.5,1.0),Vec2::ZERO,vec2(1600.0,height));
        n.le_pref=vec2(1600.0,height);n.le_min=vec2(-1.0,height);n.le_flex=vec2(-1.0,-1.0);
        if let Some(t)=&mut n.txt{t.text=value.into();t.h=1;t.v=512;t.wrap=true;t.auto=false;t.size=if title{66.0}else{55.0};t.color=if title{GOOD}else{MUTED};if !title{t.style=Default::default();}}
        n
    }
    fn matches(&self,card:&Card)->bool{
        let enabled=card.id.as_ref().is_some_and(|id|self.selected.contains(id));
        if (self.filter==ModFilter::Enabled&&!enabled)||(self.filter==ModFilter::Disabled&&enabled){return false;}
        if self.query.trim().is_empty(){return true;}
        let haystack=format!("{} {} {}",card.name,card.id.as_deref().unwrap_or(""),card.description).to_lowercase();
        self.query.to_lowercase().split_whitespace().all(|term|haystack.contains(term))
    }
    fn ensure_toolbar(&self,node:&mut Node){
        if node.find(TOOLS).is_some(){return;}
        let mut tools=block();tools.name="BrowserTools".into();
        place(&mut tools,vec2(0.5,1.0),vec2(0.5,0.5),vec2(0.0,-233.65),vec2(1714.8,120.0));
        let mut search=panel("Search",vec2(750.0,110.0),Color::from_rgba(118,172,230,255));search.clip=true;
        place(&mut search,vec2(0.0,0.5),vec2(0.0,0.5),vec2(20.0,0.0),vec2(750.0,110.0));
        if let Some(img)=&mut search.img{img.ppum=3.0;}
        let mut backing=panel("Backing",Vec2::ZERO,Color::from_rgba(42,76,120,255));
        backing.rt.amin=Vec2::ZERO;backing.rt.amax=Vec2::ONE;backing.rt.size=vec2(-8.0,-8.0);
        if let Some(img)=&mut backing.img{img.ppum=3.0;}search.children.push(backing);
        let mut icon=panel("Icon",vec2(55.0,55.0),WHITE);
        if let Some(img)=&mut icon.img{img.sprite=Some("SearchIcon".into());img.kind=0;img.preserve=true;}
        place(&mut icon,vec2(0.0,0.5),vec2(0.5,0.5),vec2(55.0,0.0),vec2(55.0,55.0));search.children.push(icon);
        let mut caption=self.text("Text","Search mods...",85.0,false);
        place(&mut caption,vec2(0.0,0.5),vec2(0.0,0.5),vec2(100.0,0.0),vec2(550.0,85.0));
        if let Some(t)=&mut caption.txt{t.size=43.0;t.min=32.0;t.max=43.0;t.auto=true;t.wrap=false;}search.children.push(caption);
        let mut clear=panel("Clear",vec2(55.0,55.0),WHITE);
        if let Some(img)=&mut clear.img{img.sprite=Some("CloseIcon".into());img.kind=0;img.preserve=true;}
        place(&mut clear,vec2(1.0,0.5),vec2(0.5,0.5),vec2(-45.0,0.0),vec2(55.0,55.0));clear.active=false;search.children.push(clear);tools.children.push(search);
        for (name,label,sprite,x,width) in [("All","All",MODS_BUTTON_SPRITE,786.0,230.0),("Enabled","Enabled","TickGreenIcon",1032.0,310.0),("Disabled","Disabled","CloseIcon",1358.0,310.0)]{
            let mut button=self.control_button(name,label,vec2(width,105.0));
            place(&mut button,vec2(0.0,0.5),vec2(0.0,0.5),vec2(x,0.0),vec2(width,105.0));
            if let Some(label)=button.find_mut("Text"){
                label.rt.pos.x=25.0;label.rt.size=vec2(width-90.0,85.0);
                if let Some(t)=&mut label.txt{t.size=42.0;t.min=30.0;t.max=42.0;}
            }
            let mut icon=panel("Icon",vec2(50.0,50.0),WHITE);
            if let Some(img)=&mut icon.img{img.sprite=Some(sprite.into());img.kind=0;img.preserve=true;}
            place(&mut icon,vec2(0.0,0.5),vec2(0.5,0.5),vec2(50.0,0.0),vec2(50.0,50.0));button.children.push(icon);tools.children.push(button);
        }
        if let Some(pane)=node.find_mut(&format!("{BASE}/AvailableTracks")){pane.children.push(tools);}
    }
    fn paint_toolbar(&self,node:&mut Node){
        // Keep the full query for matching; only the displayed portion scrolls/truncates.
        let count=self.query.chars().count();
        let shown=if self.search_focus&&count>32{format!("...{}",self.query.chars().skip(count-29).collect::<String>())}else{short_preview(&self.query,32)};
        let text=if self.query.is_empty(){"Search mods...".into()}else if self.search_focus{format!("{shown} |")}else{shown};
        node.set_text(&format!("{TOOLS}/Search/Text"),&text);
        node.set_active(&format!("{TOOLS}/Search/Clear"),!self.query.is_empty());
        node.set_img_color(&format!("{TOOLS}/Search"),if self.search_focus{YELLOW}else{Color::from_rgba(118,172,230,255)});
        for (name,filter) in [("All",ModFilter::All),("Enabled",ModFilter::Enabled),("Disabled",ModFilter::Disabled)]{
            node.set_sprite(&format!("{TOOLS}/{name}"),Some(if self.filter==filter{"GreenBtnLong"}else{"BlueBtnLong"}));
        }
    }
    fn search_text(&mut self,node:&mut Node,text:&str,erase:bool,select_all:bool){
        let previous=self.query.clone();
        if select_all{self.search_select_all=true;}
        if erase{if self.search_select_all{self.query.clear();}else{self.query.pop();}self.search_select_all=false;}
        for c in text.chars().filter(|c|!c.is_control()){
            if self.search_select_all{self.query.clear();self.search_select_all=false;}
            if self.query.chars().count()<128{self.query.push(c);}
        }
        if self.query!=previous{self.reset_list=true;self.refresh_list(node);}else{self.paint_toolbar(node);}
    }
    pub fn dismiss(&mut self,node:&mut Node)->bool{
        if self.cancel_edit(node){return true;}
        if self.search_focus{self.search_focus=false;self.paint_toolbar(node);return true;}false
    }
    fn rebuild(&mut self,node:&mut Node){
        let key=self.focus.and_then(|i|self.rows.get(i)).map(|r|r.key.clone());
        self.rows=cards(&self.root,&self.catalog,&self.selected,&self.overrides);
        self.focus=key.and_then(|k|self.rows.iter().position(|r|r.key==k)).or_else(||(!self.rows.is_empty()).then_some(0));
        self.refresh_list(node);
    }
    fn refresh_list(&mut self,node:&mut Node){
        self.ensure_toolbar(node);self.paint_toolbar(node);
        let visible:Vec<_>=self.rows.iter().enumerate().filter(|(_,card)|self.matches(card)).map(|(i,_)|i).collect();
        let previous=self.focus;
        if self.focus.is_none_or(|i|!visible.contains(&i)){self.focus=visible.first().copied();}
        self.reset_details|=previous!=self.focus;
        let mut list=Vec::new();
        for &i in &visible{let card=&self.rows[i];
            let mut row=self.template.clone();row.name=format!("Mod{i}");row.active=true;
            row.le_pref.y=230.0;row.rt.size.y=230.0;
            if let Some(img)=&mut row.img{img.sprite=Some("MainBGPanelBlue".into());img.color=WHITE;img.kind=1;}

            if self.focus==Some(i){
                let mut inset=panel("SelectionFill",Vec2::ZERO,WHITE);
                inset.rt.amin=Vec2::ZERO;inset.rt.amax=Vec2::ONE;inset.rt.size=vec2(-14.0,-14.0);
                if let Some(img)=&mut inset.img{img.sprite=Some("MainBGPanelBlue".into());}
                if let Some(img)=&mut row.img{img.sprite=Some("MainBGPanelYellow".into());}
                row.children.insert(0,inset);
            }
            for path in ["TrackEQ","TrophyLockBtn","AddButton","RemoveButton"]{row.set_active(path,false);}
            row.set_active("SelectTrackButton/BtnImg",false);
            if let Some(img)=row.find_mut("SelectTrackButton/BtnImg").and_then(|n|n.img.as_mut()){img.color=GOOD;}
            row.set_sprite("TrackIcon",Some(&card_sprite(card)));
            if let Some(icon)=row.find_mut("TrackIcon"){icon.rt.pos.x=120.0;icon.rt.size=vec2(145.0,145.0);if let Some(img)=&mut icon.img{img.preserve=true;}}
            row.set_text("TrackName",&short_preview(&card.name,52));
            if let Some(name)=row.find_mut("TrackName"){
                place(name,vec2(0.0,0.5),vec2(0.0,0.5),vec2(235.0,if card.description.trim().is_empty()&&card.error.is_none(){0.0}else{45.0}),vec2(1040.0,85.0));
                if let Some(t)=&mut name.txt{t.style=Default::default();t.size=60.0;t.auto=true;t.min=36.0;t.max=60.0;t.wrap=false;t.color=WHITE;}
            }
            if card.error.is_none()&&!card.description.trim().is_empty(){
                let mut description=self.text("Summary",&short_preview(&card.description,110),100.0,false);
                place(&mut description,vec2(0.0,0.5),vec2(0.0,0.5),vec2(235.0,-48.0),vec2(1040.0,100.0));
                if let Some(t)=&mut description.txt{t.size=42.0;t.auto=false;t.wrap=true;t.color=Color::from_rgba(212,230,250,255);}
                row.children.push(description);
            }
            let on=card.id.as_ref().is_some_and(|id|self.selected.contains(id));
            if card.error.is_some(){
                let mut warning=self.text("Warning","Needs attention",65.0,false);
                place(&mut warning,vec2(0.0,0.5),vec2(0.0,0.5),vec2(235.0,-65.0),vec2(1040.0,65.0));
                if let Some(t)=&mut warning.txt{t.color=BAD;t.size=40.0;}row.children.push(warning);
            }
            let mut switch=native_switch(on);
            place(&mut switch,vec2(1.0,0.5),vec2(1.0,0.5),vec2(-55.0,0.0),vec2(300.0,165.0));
            switch.button=self.editable()&&card.id.is_some()&&(card.error.is_none()||on);
            if !self.editable()||card.id.is_none()||(card.error.is_some()&&!on){switch.alpha=0.35;}
            row.children.push(switch);list.push(row);
        }
        if list.is_empty(){let message=if self.rows.is_empty(){"No mods installed yet.\nOpen the Mods folder, add a mod, then press Reload Mods."}else if !self.query.trim().is_empty(){"No mods match your search."}else if self.filter==ModFilter::Enabled{"No enabled mods."}else{"No disabled mods."};list.push(self.text("Empty",message,360.0,false));}
        if let Some(content)=node.find_mut(LIST){replace_children(content,list);}
        self.details(node);
        self.footer(node);
    }
    fn details(&self,node:&mut Node){
        let mut children=Vec::new();
        if let Some(card)=self.focus.and_then(|i|self.rows.get(i)){
            let can_reset=self.room.is_none()&&card.error.is_none()&&(card.settings.starting_cash.is_some()||card.settings.starting_lives.is_some()||card.id.as_ref().is_some_and(|id|self.overrides.contains_key(id)));
            let mut header=block();header.name="Header".into();header.rt.size=vec2(1600.0,300.0);header.le_pref=vec2(1600.0,300.0);
            if let Some(mut icon)=self.template.find("TrackIcon").cloned(){
                icon.name="Icon".into();
                if let Some(img)=&mut icon.img{img.sprite=Some(card_sprite(card));img.preserve=true;}
                place(&mut icon,vec2(0.0,1.0),vec2(0.5,0.5),vec2(150.0,-145.0),vec2(230.0,230.0));header.children.push(icon);
            }
            let mut title=self.text("Name",&card.name,130.0,true);
            place(&mut title,vec2(0.0,1.0),vec2(0.0,0.5),vec2(310.0,-110.0),vec2(1270.0,180.0));
            if let Some(t)=&mut title.txt{t.auto=true;t.min=42.0;t.max=78.0;t.wrap=true;}
            header.children.push(title);
            let version=if card.version.is_empty(){"Local mod".into()}else{format!("Version {}",card.version)};
            let mut sub=self.text("Version",&version,90.0,false);
            place(&mut sub,vec2(0.0,1.0),vec2(0.0,0.5),vec2(310.0,-245.0),vec2(if can_reset{700.0}else{1270.0},90.0));header.children.push(sub);
            if can_reset{
                let mut reset=self.control_button("ResetSettings","Reset to Defaults",vec2(540.0,115.0));
                place(&mut reset,vec2(0.0,1.0),vec2(0.5,0.5),vec2(1320.0,-245.0),vec2(540.0,115.0));
                if let Some(t)=reset.find_mut("Text").and_then(|n|n.txt.as_mut()){t.size=48.0;t.min=36.0;t.max=48.0;}
                header.children.push(reset);
            }
            children.push(header);
            let description=card.error.as_deref().unwrap_or(&card.description);
            if !description.is_empty(){
                let height=((description.chars().count()/60+1) as f32*70.0).max(110.0);
                let mut desc=self.text("Description",description,height,false);if let Some(t)=&mut desc.txt{t.v=256;}if card.error.is_some(){if let Some(t)=&mut desc.txt{t.color=BAD;}}children.push(desc);
            }
            if card.error.is_none(){

                let mut effects=Vec::new();
                if let Some(cash)=card.rules.starting_cash.filter(|_|card.settings.starting_cash.is_none()){effects.push(format!("Starting cash: ${}",super::account_preview::number(cash)));}
                if let Some(lives)=card.rules.starting_lives.filter(|_|card.settings.starting_lives.is_none()){effects.push(format!("Starting lives: {}",super::account_preview::number(lives)));}
                if effects.is_empty()&&card.rules==SandboxRules::default()&&card.changes==0{effects.push("No gameplay changes in this mod.".into());}
                if !effects.is_empty(){
                    children.push(self.text("EffectsTitle","WHAT IT CHANGES",110.0,true));
                    let height=effects.len() as f32*85.0;children.push(self.text("Effects",&effects.join("\n"),height,false));
                }
                let controls=[("starting_cash","Starting cash",&card.settings.starting_cash,card.rules.starting_cash),
                    ("starting_lives","Starting lives",&card.settings.starting_lives,card.rules.starting_lives)];
                if self.room.is_none()&&(controls.iter().any(|(_,_,control,_)|control.is_some())||card.id.as_ref().is_some_and(|id|self.overrides.contains_key(id))) {
                    children.push(self.text("SettingsTitle","SETTINGS",90.0,true));
                    if let Some(error)=self.catalog.entries.iter().filter_map(|e|e.manifest.as_ref().ok())
                        .find(|m|Some(&m.id)==card.id.as_ref()).and_then(|m|m.resolved(&self.overrides).err()) {
                        let mut warning=self.text("SettingsError",&error,260.0,false);
                        if let Some(t)=&mut warning.txt{t.color=BAD;}children.push(warning);
                    }

                    for (key,label,control,value) in controls {
                        let (Some(control),Some(value))=(control,value) else{continue;};
                        let mut row=block();row.name=key.into();row.rt.size=vec2(1600.0,165.0);row.le_pref=row.rt.size;
                        let mut label=self.text("Value",label,160.0,false);
                        place(&mut label,vec2(0.0,0.5),vec2(0.0,0.5),vec2(0.0,0.0),vec2(580.0,160.0));row.children.push(label);
                        let mut input=panel("Input",vec2(600.0,140.0),Color::from_rgba(118,172,230,255));
                        if let Some(img)=&mut input.img{img.ppum=3.0;}
                        place(&mut input,vec2(0.0,0.5),vec2(0.5,0.5),vec2(900.0,0.0),vec2(600.0,140.0));
                        let mut backing=panel("Backing",Vec2::ZERO,Color::from_rgba(42,76,120,255));
                        backing.rt.amin=Vec2::ZERO;backing.rt.amax=Vec2::ONE;backing.rt.size=vec2(-10.0,-10.0);
                        if let Some(img)=&mut backing.img{img.ppum=3.0;}input.children.push(backing);
                        let mut number=self.text("Text",&super::account_preview::number(value),120.0,false);
                        place(&mut number,vec2(0.5,0.5),vec2(0.5,0.5),Vec2::ZERO,vec2(550.0,120.0));
                        if let Some(t)=&mut number.txt{t.h=2;t.auto=true;t.min=36.0;t.max=60.0;}
                        input.children.push(number);row.children.push(input);
                        for (name,text,x,allowed) in [("Minus","-",1300.0,value>control.min),("Plus","+",1470.0,value<control.max)] {
                            let mut button=self.control_button(name,text,vec2(140.0,140.0));
                            place(&mut button,vec2(0.0,0.5),vec2(0.5,0.5),vec2(x,0.0),vec2(140.0,140.0));
                            if !allowed{button.alpha=0.45;}row.children.push(button);
                        }
                        children.push(row);
                    }
                }

            }
        }else{children.push(self.text("EmptyDetails","Select a mod to see its description and effects.",300.0,false));}
        if let Some(content)=node.find_mut(DETAILS){replace_children(content,children);}
    }
    fn control_button(&self,name:&str,text:&str,size:Vec2)->Node {
        let mut button=self.button.clone();button.name=name.into();button.active=true;
        place(&mut button,vec2(0.5,1.0),vec2(0.5,1.0),Vec2::ZERO,size);
        text_all(&mut button,text);
        native_button(&mut button);
        if matches!(name,"Minus"|"Plus"){if let Some(img)=&mut button.img{img.sprite=Some("BlueBtnSquareSmall".into());}}
        if let Some(label)=button.find_mut("Text") {
            label.rt.size=vec2(size.x-40.0,size.y-20.0);
            if let Some(t)=&mut label.txt{t.size=64.0;t.min=42.0;t.max=64.0;t.auto=true;}
        }
        button
    }
    fn footer(&self,node:&mut Node){
        let issue=self.notice.clone().or_else(||if self.room.is_none(){self.catalog.compile_with(&self.selected,&self.overrides).err()}else{None});
        let mut status=self.text("Status",issue.as_deref().unwrap_or(""),150.0,false);
        place(&mut status,vec2(0.5,0.5),vec2(0.5,0.5),Vec2::ZERO,vec2(3000.0,150.0));
        if let Some(t)=&mut status.txt{t.size=42.0;t.color=BAD;}
        let mut action=block();action.name="Action".into();action.active=false;
        if let Some(footer)=node.find_mut(FOOTER){footer.active=issue.is_some();footer.children=vec![status,action];}
    }
    pub fn warm_icons(&mut self,sprites:&mut crate::ui::Sprites){
        if !std::mem::take(&mut self.icons_dirty){return;}
        for card in &self.rows{if let Some(path)=&card.icon{sprites.register_file(&card_sprite(card),path,MODS_BUTTON_SPRITE);}}
    }
    fn begin_edit(&mut self,node:&mut Node,key:&str){
        if self.room.is_some(){return;}
        self.search_focus=false;self.paint_toolbar(node);
        let Some(card)=self.focus.and_then(|i|self.rows.get(i)) else{return;};
        let Some(id)=card.id.clone() else{return;};
        let value=match key{"starting_cash"=>card.rules.starting_cash,_=>card.rules.starting_lives};
        let Some(value)=value else{return;};
        self.edit=Some(NumberEdit{id,key:key.into(),text:format!("{value:.0}"),select_all:true});
        self.notice=None;self.paint_edit(node);self.footer(node);
    }
    fn paint_edit(&self,node:&mut Node){
        if let Some(edit)=&self.edit{
            let field=format!("{DETAILS}/{}/Input",edit.key);
            node.set_img_color(&field,Color::from_rgba(255,213,56,255));
            let path=format!("{field}/Text");
            node.set_text(&path,&format!("{} |",edit.text));
            if let Some(t)=node.find_mut(&path).and_then(|n|n.txt.as_mut()){t.color=YELLOW;}
        }
    }
    fn edit_text(&mut self,node:&mut Node,text:&str,erase:bool,select_all:bool){
        let Some(edit)=&mut self.edit else{return;};
        if select_all{edit.select_all=true;}
        if erase{if edit.select_all{edit.text.clear();}else{edit.text.pop();}edit.select_all=false;}
        for c in text.chars().filter(char::is_ascii_digit){
            if edit.select_all{edit.text.clear();edit.select_all=false;}
            if edit.text.len()<20{edit.text.push(c);}
        }
        self.paint_edit(node);
    }
    pub fn cancel_edit(&mut self,node:&mut Node)->bool{
        if self.edit.take().is_none(){return false;}
        self.notice=None;self.details(node);self.footer(node);true
    }
    fn commit_edit(&mut self,node:&mut Node)->bool{
        let Some(edit)=&self.edit else{return true;};
        let Some(card)=self.focus.and_then(|i|self.rows.get(i)).filter(|c|c.id.as_deref()==Some(edit.id.as_str())) else{return self.cancel_edit(node);};
        let control=match edit.key.as_str(){"starting_cash"=>card.settings.starting_cash.as_ref(),_=>card.settings.starting_lives.as_ref()};
        let Some(control)=control else{return self.cancel_edit(node);};
        let value=edit.text.parse::<f64>().ok().filter(|v|control.accepts(*v));
        let Some(value)=value else{
            self.notice=Some(format!("Enter a whole number from {} to {}, in steps of {}. Press Esc to cancel.",control.min,control.max,control.step));
            self.footer(node);return false;
        };
        let mut rules=card.rules.clone();
        match edit.key.as_str(){"starting_cash"=>rules.starting_cash=Some(value),_=>rules.starting_lives=Some(value)}
        let saved=serde_json::json!({"starting_cash":card.settings.starting_cash.as_ref().and(rules.starting_cash),
            "starting_lives":card.settings.starting_lives.as_ref().and(rules.starting_lives)});
        self.overrides.insert(edit.id.clone(),saved);self.edit=None;
        self.pending_settings=Some(self.overrides.clone());self.notice=None;self.rebuild(node);true
    }
    pub fn input(&mut self,node:&mut Node){
        if self.search_focus{
            let ctrl=is_key_down(KeyCode::LeftControl)||is_key_down(KeyCode::RightControl);
            let mut text=String::new();while let Some(c)=get_char_pressed(){if !ctrl{text.push(c);}}
            if ctrl&&is_key_pressed(KeyCode::V){text=macroquad::miniquad::window::clipboard_get().unwrap_or_default();}
            let erase=is_key_pressed(KeyCode::Backspace)||is_key_pressed(KeyCode::Delete);
            let all=ctrl&&is_key_pressed(KeyCode::A);
            if !text.is_empty()||erase||all{self.search_text(node,&text,erase,all);}
            if is_key_pressed(KeyCode::Enter)||is_key_pressed(KeyCode::KpEnter){self.search_focus=false;self.paint_toolbar(node);}return;
        }
        if self.edit.is_none(){return;}
        let ctrl=is_key_down(KeyCode::LeftControl)||is_key_down(KeyCode::RightControl);
        let mut typed=String::new();while let Some(c)=get_char_pressed(){if !ctrl{typed.push(c);}}
        if ctrl&&is_key_pressed(KeyCode::V){
            let pasted=macroquad::miniquad::window::clipboard_get().unwrap_or_default();
            if pasted.chars().any(|c|!c.is_ascii_digit()&&!c.is_whitespace()&&c!=','){
                self.notice=Some("Paste a whole number, without decimals or other text.".into());self.footer(node);return;
            }
            typed=pasted;
        }
        let erase=is_key_pressed(KeyCode::Backspace)||is_key_pressed(KeyCode::Delete);
        if !typed.is_empty()||erase||(ctrl&&is_key_pressed(KeyCode::A)){self.edit_text(node,&typed,erase,ctrl&&is_key_pressed(KeyCode::A));}
        if is_key_pressed(KeyCode::Enter)||is_key_pressed(KeyCode::KpEnter){self.commit_edit(node);}
    }
    fn toggle(&mut self,node:&mut Node,index:usize){
        if !self.editable(){return;}
        let Some(card)=self.rows.get(index) else{return;};
        let Some(id)=card.id.clone() else{return;};
        if card.error.is_some()&&!self.selected.contains(&id){return;}
        if !self.selected.remove(&id){self.selected.insert(id);}
        let mut ids:Vec<_>=self.selected.iter().cloned().collect();ids.sort();
        self.pending=Some(ids);self.notice=None;self.reset_list|=self.filter!=ModFilter::All;self.rebuild(node);
    }
    pub fn click(&mut self,node:&mut Node,pointer:Vec2)->bool{
        if node.rect(&format!("{DETAILS}/Header/ResetSettings")).is_some_and(|r|r.contains(pointer)){self.edit=None;}
        let input=["starting_cash","starting_lives"].into_iter().find(|key|node.rect(&format!("{DETAILS}/{key}/Input")).is_some_and(|r|r.contains(pointer)));
        if let Some(edit)=&self.edit{
            if input==Some(edit.key.as_str()){return true;}
            if !self.commit_edit(node){return true;}
        }
        if let Some(key)=input{self.begin_edit(node,key);return true;}
        if node.rect(&format!("{TOOLS}/Search/Clear")).is_some_and(|r|r.contains(pointer)){
            self.query.clear();self.search_focus=true;self.search_select_all=true;self.reset_list=true;self.refresh_list(node);return true;
        }
        if node.rect(&format!("{TOOLS}/Search")).is_some_and(|r|r.contains(pointer)){
            self.search_focus=true;self.search_select_all=true;self.paint_toolbar(node);return true;
        }
        if self.search_focus{self.search_focus=false;self.paint_toolbar(node);}
        for (name,filter) in [("All",ModFilter::All),("Enabled",ModFilter::Enabled),("Disabled",ModFilter::Disabled)]{
            if node.rect(&format!("{TOOLS}/{name}")).is_some_and(|r|r.contains(pointer)){
                self.filter=filter;self.reset_list=true;self.refresh_list(node);return true;
            }
        }
        let hit=|path:&str|node.rect(path).is_some_and(|r|r.contains(pointer));
        if self.editable()&&hit(&format!("{BASE}/AvailableTracks/AddAllBtn/AddAllBtn")){
            self.notice=open_folder(&modding::directory(&self.root)).err();self.footer(node);return true;
        }
        if self.editable()&&hit(&format!("{BASE}/Playlist/RemoveAllBtn/RemoveAllBtn")){
            self.catalog=Catalog::scan(&modding::directory(&self.root));self.notice=None;self.reset_details=true;self.reset_list=true;self.icons_dirty=true;
            let mut ids:Vec<_>=self.selected.iter().cloned().collect();ids.sort();self.pending=Some(ids);
            self.rebuild(node);return true;
        }
        if let Some(i)=(0..self.rows.len()).find(|i|hit(&format!("{LIST}/Mod{i}/Toggle"))){self.toggle(node,i);return true;}
        if let Some(enable)=[true,false].into_iter().find(|&on|hit(&format!("{BASE}/Playlist/{}",if on{"EnableAll"}else{"DisableAll"}))){
            if !self.editable(){return true;}
            // Every mod that loads (as a toggle allows), or none.
            if enable{self.selected.extend(self.rows.iter().filter(|c|c.error.is_none()).filter_map(|c|c.id.clone()));}else{self.selected.clear();}
            let mut ids:Vec<_>=self.selected.iter().cloned().collect();ids.sort();
            self.pending=Some(ids);self.notice=None;self.reset_list|=self.filter!=ModFilter::All;self.rebuild(node);return true;
        }
        if let Some(card)=self.focus.and_then(|i|self.rows.get(i)).cloned().filter(|c|c.error.is_none()) {
            if hit(&format!("{DETAILS}/Header/ResetSettings")) {
                if let Some(id)=&card.id{self.overrides.remove(id);}
                self.pending_settings=Some(self.overrides.clone());self.notice=None;self.rebuild(node);return true;
            }
            for (key,control,value) in [("starting_cash",&card.settings.starting_cash,card.rules.starting_cash),
                ("starting_lives",&card.settings.starting_lives,card.rules.starting_lives)] {
                let (Some(control),Some(value))=(control,value) else{continue;};
                for (action,direction) in [("Minus",-1.0),("Plus",1.0)] {
                    if hit(&format!("{DETAILS}/{key}/{action}")) {
                        let next=control.adjust(value,direction);
                        if next!=value {
                            let id=card.id.as_ref().unwrap();
                            // Normalize stale settings through the same resolver used by gameplay.
                            let mut rules=card.rules.clone();
                            match key{"starting_cash"=>rules.starting_cash=Some(next),_=>rules.starting_lives=Some(next)}
                            let saved=serde_json::json!({
                                "starting_cash":card.settings.starting_cash.as_ref().and(rules.starting_cash),
                                "starting_lives":card.settings.starting_lives.as_ref().and(rules.starting_lives)});
                            self.overrides.insert(id.clone(),saved);
                            self.pending_settings=Some(self.overrides.clone());self.notice=None;self.rebuild(node);
                        }
                        return true;
                    }
                }
            }
        }
        if self.editable()&&hit(&format!("{FOOTER}/Action")){
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
        // Home captions rise in with the authored Knowledge bounce. Offset the curve
        // to our bottom-right resting position; keep its timing and tangents intact.
        for animator in animators.iter_mut().filter(|a|a["controller"]=="HomeScreen") {
            for clip in animator["clips"].as_array_mut().into_iter().flatten(){
                let copies:Vec<_>=clip["bindings"].as_array().into_iter().flatten()
                    .filter(|b|b["path"]==format!("{MAIN}/BottomButtonGroup/Knowledge/KnowledgeAnim") && b["property"]=="m_AnchoredPosition.y")
                    .cloned().map(|mut b|{
                        b["path"]=serde_json::json!(format!("{MAIN}/Mods"));
                        for key in ["start","end"] {for v in b[key].as_array_mut().into_iter().flatten(){if let Some(x)=v.as_f64(){*v=serde_json::json!(x+75.0);}}}
                        for curve in b["curves"].as_array_mut().into_iter().flatten(){for key in curve.as_array_mut().into_iter().flatten(){if let Some(x)=key[4].as_f64(){key[4]=serde_json::json!(x+75.0);}}}
                        b
                    }).collect();
                if let Some(bindings)=clip["bindings"].as_array_mut(){bindings.extend(copies);}
            }
        }

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
    fn search_matches_case_insensitive_names_ids_and_descriptions_without_changing_enabled_mods(){
        let mut alpha=entry("alpha");alpha.manifest.as_mut().unwrap().description="Healing towers and custom rounds".into();
        let (mut browser,mut node)=fixture(vec![alpha,entry("beta")],HashSet::from(["alpha".into()]));
        browser.search_focus=true;browser.search_text(&mut node,"HEALING rounds",false,true);
        assert!(node.find(&format!("{LIST}/Mod0")).is_some());assert!(node.find(&format!("{LIST}/Mod1")).is_none());
        assert_eq!(browser.focus,Some(0));assert!(browser.reset_list);assert!(browser.pending.is_none());
        browser.search_text(&mut node,"BETA",false,true);
        assert_eq!(browser.focus,Some(1));assert!(node.find(&format!("{LIST}/Mod0")).is_none());
        browser.search_text(&mut node,"no such mod",false,true);assert!(browser.focus.is_none());
        assert!(node.find(&format!("{LIST}/Empty")).is_some());
        click(&mut browser,&mut node,&format!("{TOOLS}/Search/Clear"));
        assert!(browser.query.is_empty());assert_eq!(browser.focus,Some(0));
        assert_eq!(browser.selected,HashSet::from(["alpha".into()]));assert!(browser.pending.is_none());
        let long="x".repeat(100);browser.search_text(&mut node,&long,false,true);
        assert_eq!(browser.query,long);assert!(node.find(&format!("{TOOLS}/Search/Text")).unwrap().txt.as_ref().unwrap().text.chars().count()<=36);
    }
    #[test]
    fn status_filters_combine_with_search_and_toggling_does_not_drop_hidden_enabled_mods(){
        let (mut browser,mut node)=fixture(vec![entry("alpha"),entry("beta"),entry("gamma")],HashSet::from(["alpha".into(),"gamma".into()]));
        click(&mut browser,&mut node,&format!("{TOOLS}/Enabled"));
        assert!(node.find(&format!("{LIST}/Mod1")).is_none());assert!(node.find(&format!("{LIST}/Mod2")).is_some());
        browser.search_text(&mut node,"alpha",false,true);
        click(&mut browser,&mut node,&format!("{LIST}/Mod0/Toggle"));
        assert_eq!(browser.pending.take().unwrap(),vec!["gamma"]);assert!(browser.focus.is_none());
        click(&mut browser,&mut node,&format!("{TOOLS}/Disabled"));
        assert_eq!(browser.focus,Some(0));assert!(node.find(&format!("{LIST}/Mod1")).is_none());
        browser.search_text(&mut node,"",true,true);
        assert!(node.find(&format!("{LIST}/Mod0")).is_some());assert!(node.find(&format!("{LIST}/Mod1")).is_some());
        assert!(node.find(&format!("{LIST}/Mod2")).is_none());
        click(&mut browser,&mut node,&format!("{TOOLS}/All"));assert!(node.find(&format!("{LIST}/Mod2")).is_some());
    }
    #[test]
    fn leaving_search_for_a_number_field_moves_keyboard_focus_without_clearing_the_query(){
        let manifest=Manifest::parse(include_bytes!("../../mods/practice-start/mod.json")).unwrap();
        let (mut browser,mut node)=fixture(vec![Entry{folder:"practice".into(),manifest:Ok(manifest)}],HashSet::new());
        click(&mut browser,&mut node,&format!("{TOOLS}/Search"));browser.search_text(&mut node,"practice",false,false);
        assert!(browser.search_focus);assert!(browser.dismiss(&mut node));assert!(!browser.search_focus);
        assert_eq!(browser.query,"practice");
        click(&mut browser,&mut node,&format!("{TOOLS}/Search"));
        click(&mut browser,&mut node,&format!("{DETAILS}/starting_cash/Input"));
        assert!(!browser.search_focus);assert!(browser.edit.is_some());assert_eq!(browser.query,"practice");
    }
    #[test]
    fn clicking_another_field_or_switch_commits_once_and_performs_the_click(){
        let manifest=Manifest::parse(include_bytes!("../../mods/practice-start/mod.json")).unwrap();
        let (mut browser,mut node)=fixture(vec![Entry{folder:"practice".into(),manifest:Ok(manifest)}],HashSet::new());
        click(&mut browser,&mut node,&format!("{DETAILS}/starting_cash/Input"));
        browser.edit_text(&mut node,"250000",false,false);
        click(&mut browser,&mut node,&format!("{DETAILS}/starting_lives/Input"));
        assert_eq!(browser.edit.as_ref().unwrap().key,"starting_lives");
        assert_eq!(browser.pending_settings.take().unwrap()["practice-start"]["starting_cash"],json!(250000.0));
        browser.edit_text(&mut node,"300",false,false);
        click(&mut browser,&mut node,&format!("{LIST}/Mod0/Toggle"));
        assert!(browser.edit.is_none());assert_eq!(browser.pending.take().unwrap(),vec!["practice-start"]);
        assert_eq!(browser.pending_settings.take().unwrap()["practice-start"]["starting_lives"],json!(300.0));
    }
    #[test]
    fn typed_settings_validate_commit_and_cancel_without_enabling_the_mod(){
        let manifest=Manifest::parse(include_bytes!("../../mods/practice-start/mod.json")).unwrap();
        let (mut browser,mut node)=fixture(vec![Entry{folder:"practice".into(),manifest:Ok(manifest)}],HashSet::new());
        browser.begin_edit(&mut node,"starting_cash");browser.edit_text(&mut node,"250000",false,false);
        assert!(browser.commit_edit(&mut node));
        assert_eq!(browser.pending_settings.take().unwrap()["practice-start"]["starting_cash"],json!(250000.0));
        assert!(browser.selected.is_empty());assert!(browser.pending.is_none());
        browser.begin_edit(&mut node,"starting_cash");browser.edit_text(&mut node,"250001",false,false);
        assert!(!browser.commit_edit(&mut node));assert!(browser.pending_settings.is_none());assert!(browser.edit.is_some());
        assert!(node.find(FOOTER).unwrap().active);assert!(browser.cancel_edit(&mut node));
        assert_eq!(browser.rows[0].rules.starting_cash,Some(250000.0));
        browser.begin_edit(&mut node,"starting_lives");browser.edit_text(&mut node,"",true,false);
        assert!(!browser.commit_edit(&mut node));browser.edit_text(&mut node,"300",false,true);
        assert!(browser.commit_edit(&mut node));assert_eq!(browser.rows[0].rules.starting_lives,Some(300.0));
        browser.begin_edit(&mut node,"starting_cash");browser.edit_text(&mut node,"250001",false,false);
        assert!(!browser.commit_edit(&mut node));
        click(&mut browser,&mut node,&format!("{DETAILS}/Header/ResetSettings"));
        assert!(browser.edit.is_none());assert!(browser.overrides.is_empty());
        assert!(browser.pending_settings.take().unwrap().is_empty());
    }
    #[test]
    fn local_icons_require_bounded_png_inside_the_mod_folder(){
        let unique=std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos();
        let base=std::env::temp_dir().join(format!("rusttd6-mod-icon-{}-{unique}",std::process::id()));
        let root=base.join("local_extracted");let folder=base.join("mods/sample");
        std::fs::create_dir_all(&root).unwrap();std::fs::create_dir_all(&folder).unwrap();
        let file=folder.join("icon.png");
        assert!(mod_icon(&root,"sample").is_none());
        let png:&[u8]=&[137,80,78,71,13,10,26,10,0,0,0,13,73,72,68,82,0,0,0,1,0,0,0,1,8,6,0,0,0,31,21,196,137,0,0,0,13,73,68,65,84,120,156,99,80,120,80,240,31,0,5,4,2,112,97,176,11,51,0,0,0,0,73,69,78,68,174,66,96,130];
        std::fs::write(&file,png).unwrap();assert_eq!(mod_icon(&root,"sample"),Some(file.canonicalize().unwrap()));
        assert!(mod_icon(&root,"..").is_none());
        let mut oversized=png.to_vec();oversized[16..20].copy_from_slice(&513u32.to_be_bytes());
        std::fs::write(&file,oversized).unwrap();assert!(mod_icon(&root,"sample").is_none());
        std::fs::write(&file,b"not a png").unwrap();assert!(mod_icon(&root,"sample").is_none());
        std::fs::remove_file(file).unwrap();std::fs::remove_dir(folder).unwrap();
        std::fs::remove_dir(base.join("mods")).unwrap();std::fs::remove_dir(root).unwrap();std::fs::remove_dir(base).unwrap();
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
        let mut browser=Browser{root:PathBuf::new(),catalog:Catalog{entries},selected,overrides:Overrides::new(),rows:Vec::new(),focus:None,
            query:String::new(),search_focus:false,search_select_all:false,filter:ModFilter::All,reset_list:false,edit:None,icons_dirty:true,template,button,body:body.clone(),title:body,notice:None,reset_details:false,pending:None,pending_settings:None,room:None};
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
        click(&mut browser,&mut node,&format!("{LIST}/Mod1/Toggle"));
        assert_eq!(browser.pending.take().unwrap(),vec!["b"]);
        click(&mut browser,&mut node,&format!("{LIST}/Mod0"));
        click(&mut browser,&mut node,&format!("{LIST}/Mod0/Toggle"));
        assert_eq!(browser.pending.take().unwrap(),vec!["a","b"]);
        assert!(node.find(&format!("{FOOTER}/Status")).unwrap().txt.as_ref().unwrap().text.contains("More than one"));
        click(&mut browser,&mut node,&format!("{LIST}/Mod0/Toggle"));
        assert_eq!(browser.pending.take().unwrap(),vec!["b"]);
        assert!(!node.find(&format!("{FOOTER}/Status")).unwrap().txt.as_ref().unwrap().text.contains("More than one"));
    }
    #[test]
    fn row_switch_is_instant_and_does_not_change_the_inspected_mod(){
        let (mut browser,mut node)=fixture(vec![entry("a"),entry("b")],HashSet::new());
        assert_eq!(browser.focus,Some(0));
        assert!(node.find(&format!("{LIST}/Mod0/Toggle/Label")).is_none());
        assert!(node.find(&format!("{LIST}/Mod0/State")).is_none());
        assert!(node.find(&format!("{DETAILS}/Status")).is_none());
        assert!(!node.find(FOOTER).unwrap().active);
        click(&mut browser,&mut node,&format!("{LIST}/Mod1/Toggle"));
        assert_eq!(browser.pending.take().unwrap(),vec!["b"]);assert_eq!(browser.focus,Some(0));
        assert!(node.find(&format!("{LIST}/Mod0/Toggle/Label")).is_none());
        assert!(node.find(&format!("{LIST}/Mod0/State")).is_none());
        assert!(node.find(&format!("{DETAILS}/Status")).is_none());
        assert!(!node.find(FOOTER).unwrap().active);
        click(&mut browser,&mut node,&format!("{LIST}/Mod1/Toggle"));
        assert!(browser.pending.take().unwrap().is_empty());assert_eq!(browser.focus,Some(0));
    }
    #[test]
    fn missing_enabled_mod_can_be_removed_and_invalid_mod_cannot_be_enabled(){
        let (mut browser,mut node)=fixture(vec![],HashSet::from(["gone".into()]));
        assert!(browser.rows[0].missing);assert!(node.find(&format!("{LIST}/Mod0/Toggle")).is_some());
        click(&mut browser,&mut node,&format!("{LIST}/Mod0/Toggle"));
        assert!(browser.pending.take().unwrap().is_empty());assert!(browser.rows.is_empty());
        let (_,node)=fixture(vec![Entry{folder:"broken".into(),manifest:Err("Invalid JSON".into())}],HashSet::new());
        assert!(!node.find(&format!("{FOOTER}/Action")).unwrap().active);
    }
    #[test]
    fn mod_settings_buttons_update_preview_and_emit_persistence_without_enabling(){
        let manifest=Manifest::parse(include_bytes!("../../mods/practice-start/mod.json")).unwrap();
        let (mut browser,mut node)=fixture(vec![Entry{folder:"practice".into(),manifest:Ok(manifest.clone())}],HashSet::new());
        click(&mut browser,&mut node,&format!("{DETAILS}/starting_cash/Plus"));
        assert!(browser.pending.is_none());assert!(browser.selected.is_empty());
        let saved=browser.pending_settings.take().unwrap();
        assert_eq!(saved["practice-start"]["starting_cash"].as_f64(),Some(110000.0));
        assert_eq!(browser.rows[0].rules,manifest.resolved(&saved).unwrap());
        assert!(node.find(&format!("{DETAILS}/starting_cash/Input/Text")).unwrap().txt.as_ref().unwrap().text.contains("110,000"));
        click(&mut browser,&mut node,&format!("{DETAILS}/starting_lives/Minus"));
        assert_eq!(browser.pending_settings.take().unwrap()["practice-start"]["starting_lives"].as_f64(),Some(199.0));
        click(&mut browser,&mut node,&format!("{DETAILS}/Header/ResetSettings"));
        assert!(browser.pending_settings.take().unwrap().is_empty());
        assert_eq!(browser.rows[0].rules,manifest.on_new_sandbox);
    }
    #[test]
    fn mod_settings_can_recover_after_author_removes_controls(){
        let (mut browser,mut node)=fixture(vec![entry("a")],HashSet::from(["a".into()]));
        browser.overrides.insert("a".into(),json!({"starting_cash":200000}));browser.rebuild(&mut node);
        assert!(node.find(&format!("{DETAILS}/SettingsError")).is_some());
        click(&mut browser,&mut node,&format!("{DETAILS}/Header/ResetSettings"));
        assert!(browser.overrides.is_empty());
        assert!(browser.catalog.compile_with(&browser.selected,&browser.overrides).is_ok());
    }

    #[test]
    fn guest_room_mod_manager_is_read_only_and_does_not_persist_local_ids(){
        let manifest=Manifest::parse(include_bytes!("../../mods/practice-start/mod.json")).unwrap();
        let package=rrbtd6_coop::ModPackage::new(serde_json::to_string(&manifest).unwrap()).unwrap();
        let (mut browser,mut node)=fixture(vec![],HashSet::new());
        browser.bind_room(&mut node,Some((&[package],false)));
        assert_eq!(browser.rows.len(),1);assert!(!browser.editable());assert!(!node.find(&format!("{FOOTER}/Action")).unwrap().active);
        assert!(browser.selected.contains("practice-start"));assert!(browser.pending.is_none());assert!(browser.pending_settings.is_none());
        click(&mut browser,&mut node,&format!("{LIST}/Mod0/Toggle"));
        assert!(browser.selected.contains("practice-start"));assert!(browser.pending.is_none());
    }

}
