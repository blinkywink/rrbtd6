//! Native UI trees authored by mods; geometry rebuilds only when script UI changes.
use crate::{game::Game,hud::{Hud,Action},uitree::{Node,Ctx,R,draw_tree,block_input}};
use macroquad::prelude::*;
use std::path::Path;
pub struct ModUi {
    revision:Option<(u64,u64)>,
    widgets:Vec<(String,Node,std::collections::BTreeMap<String,String>,bool)>,
    popup:Option<Node>,open:bool,
}
impl ModUi {
    pub fn load(root:&Path)->Self{
        let popup=Node::load(&root.join("ui/prefabs/CommonPopup.json")).map(|mut n|{
            n.hide_named(&["Jewel","CashInfo","UserId","GameVersion","Cross","Tick","Spinner"]);
            n.set_active("AnimatedObject/Layout/Buttons/CancelButton",false);
            n.set_text("AnimatedObject/Layout/Buttons/ConfirmButton/ConfirmText","OK");n
        });
        Self{revision:None,widgets:Vec::new(),popup,open:false}
    }
    pub fn sync(&mut self,g:&Game){
        if self.revision==Some((g.mod_runtime.session,g.mod_runtime.revision)){return;}
        self.revision=Some((g.mod_runtime.session,g.mod_runtime.revision));
        self.widgets=g.mod_runtime.widgets.iter().map(|w|(w.owner.clone(),Node::from_json(&w.widget.tree),w.widget.actions.clone(),w.widget.modal)).collect();
        self.open=g.mod_runtime.popup.is_some();
        if let (Some(n),Some((title,body)))=(&mut self.popup,&g.mod_runtime.popup){
            n.set_text("AnimatedObject/Layout/Banner/Title",title);
            n.set_text("AnimatedObject/Layout/Body",body);
        }
    }
    pub fn modal(&self)->bool{self.open||self.widgets.iter().any(|(_,n,_,m)|*m&&n.active)}
    pub fn blocks(&self,p:Vec2)->bool{self.modal()||self.widgets.iter().any(|(_,n,_,_)|n.active&&n.screen.contains(p))}
}
impl Hud {
    pub(crate) fn draw_mod_ui(&mut self,g:&Game,full:R,canvas_h:f32,s:f32,out:&mut Vec<Action>){
        self.mod_ui.sync(g);
        block_input(false);
        let mut ctx=Ctx{sprites:&mut self.sprites,fonts:&self.fonts,scale:s,canvas_h,clip:None};
        let top_modal=self.mod_ui.widgets.iter().rposition(|(_,n,_,m)|*m&&n.active);
        for (i,(owner,node,bindings,_)) in self.mod_ui.widgets.iter_mut().enumerate(){
            block_input(self.mod_ui.open||top_modal.is_some_and(|top|i!=top));
            draw_tree(node,full,&mut ctx);
            for (path,event) in bindings.iter(){if node.clicked(path){out.push(Action::ModEvent(owner.clone(),event.clone()));}}
        }
        if self.mod_ui.open {
            draw_rectangle(0.0,0.0,screen_width(),screen_height(),Color::new(0.0,0.0,0.0,0.45));
            block_input(false);
            if let Some(node)=self.mod_ui.popup.as_mut(){
                if let Some(a)=node.find_mut("AnimatedObject"){a.rt.scale=Vec2::ONE;}
                draw_tree(node,full,&mut ctx);
                if node.clicked("AnimatedObject/Layout/Buttons/ConfirmButton")||is_key_pressed(KeyCode::Escape){out.push(Action::ModDismiss);}
            }else{out.push(Action::ModDismiss);}
        }
        block_input(false);
    }
}
