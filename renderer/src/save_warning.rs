//! Read-only imported save warning; never deletes or overwrites an official save.
use crate::uitree::{Node,Ctx,R,draw_tree,block_input};
use macroquad::prelude::*;
use std::path::Path;
const BASE:&str="AnimatedObject/Layout";
#[derive(Default)]
pub(super) struct Warning {node:Option<Node>,pending:Option<(String,String)>,began:f64,backdrop:Option<crate::menus::Backdrop>}
impl Warning {
    pub fn is_open(&self)->bool{self.pending.is_some()}
    pub fn open(&mut self,root:&Path,map:String,mode:String,saved:&serde_json::Value)->bool{
        if self.node.is_none(){self.node=Node::load(&root.join("ui/prefabs/CommonPopup.json"));}
        let Some(node)=self.node.as_mut()else{return false;};
        node.hide_named(&["Jewel","UserId","GameVersion","Cross","Tick","Spinner","CashInfo"]);
        node.set_text(&format!("{BASE}/Banner/Title"),"Saved Game");
        node.set_text(&format!("{BASE}/Body"),&format!("This map has an imported {} {} save at round {}.\n\nResuming official saves is not supported yet. Start a new local game? Your imported save will be kept.",
            saved["mapDifficulty"].as_str().unwrap_or(""),saved["modeName"].as_str().unwrap_or(""),saved["round"].as_u64().unwrap_or(0)));
        node.set_text(&format!("{BASE}/Buttons/ConfirmButton/ConfirmText"),"Start New");
        node.set_text(&format!("{BASE}/Buttons/CancelButton/CancelText"),"Cancel");
        self.pending=Some((map,mode));self.began=get_time();self.backdrop=None;true
    }
    pub fn draw(&mut self,full:R,ctx:&mut Ctx)->Option<(String,String)>{
        if !self.is_open(){return None;}
        if self.backdrop.is_none(){self.backdrop=Some(crate::menus::Backdrop::capture_sharp());}
        if let Some(bg)=&self.backdrop{bg.draw();}
        draw_rectangle(0.0,0.0,screen_width(),screen_height(),Color::new(0.0,0.0,0.0,0.35));
        block_input(false);
        let Some(node)=self.node.as_mut()else{return None;};
        let t=((get_time()-self.began)/0.25).clamp(0.0,1.0) as f32;
        if let Some(animated)=node.find_mut("AnimatedObject"){animated.rt.scale=Vec2::splat(1.0-(1.0-t).powi(3));}
        draw_tree(node,full,ctx);
        let confirm=node.clicked(&format!("{BASE}/Buttons/ConfirmButton"));
        let cancel=node.clicked(&format!("{BASE}/Buttons/CancelButton"))||is_key_pressed(KeyCode::Escape);
        if confirm||cancel{self.backdrop=None;let pending=self.pending.take();return if confirm{pending}else{None};}
        None
    }
}
