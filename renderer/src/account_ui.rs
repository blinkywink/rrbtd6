//! Shared title / Settings account panel. Network work stays on the worker.
use crate::{hud::Hud,uitree::{Node,Rt,Ctx,R,draw_tree}};
use macroquad::prelude::*;
use serde_json::Value;
use std::path::{Path,PathBuf};
#[path="sign_in.rs"] mod sign_in;
#[derive(Default)]
pub(crate) struct AccountUi {
    root:PathBuf,account:Option<Node>,account_code:String,account_status:String,
    account_focus:bool,account_job:Option<std::sync::mpsc::Receiver<Value>>,
    pub backdrop:Option<crate::menus::Backdrop>, pub generation:u64,
}
impl AccountUi {
    pub fn is_open(&self)->bool{self.account.is_some()}
    pub fn open(&mut self,root:&Path){self.root=root.to_owned();self.open_account();}
    pub fn poll(&mut self){
        if !self.root.as_os_str().is_empty(){crate::frontend::account_preview::ensure(&self.root);}
        if let Some(job)=&self.account_job {
            let result=match job.try_recv(){
                Ok(result)=>Some(result),
                Err(std::sync::mpsc::TryRecvError::Disconnected)=>Some(serde_json::json!({"ok":false,"message":"Sign-in helper stopped. No automatic retry was made."})),
                Err(std::sync::mpsc::TryRecvError::Empty)=>None,
            };
            if let Some(result)=result {
                self.account_job=None;
                self.account_status=result["message"].as_str().unwrap_or("Sign-in failed.").to_owned();
                if result["ok"]==true {self.generation+=1;crate::frontend::account_preview::signed_in(&self.root,result["preview"].clone());}
            }
        }
    }
    fn open_account(&mut self) {
        let Some(mut node)=Node::load(&self.root.join("ui/prefabs/LinkAccountUI.json")) else{return};
        for child in &mut node.children {child.rt=Rt::default();}
        const BASE:&str="Canvas/LinkAccountScreen/Login/AnimatedObject";
        node.set_active("Canvas/LinkAccountScreen/AccountInfo",false);
        node.set_active("Canvas/LinkAccountScreen/HomeBgBlur",false);
        for name in ["PasswordBox","PasswordTxtField"] {node.set_active(&format!("{BASE}/{name}"),false);}
        node.set_text(&format!("{BASE}/Title"),"Account");
        node.set_active(&format!("{BASE}/EmailTitle"),false);
        node.set_text(&format!("{BASE}/CancelButton/Text"),"Back");
        if let Some(status)=node.find_mut(&format!("{BASE}/PasswordTitle")) {
            status.rt.pos.x=0.0;status.rt.size.x=-200.0;status.rt.size.y=230.0;
            if let Some(text)=status.txt.as_mut(){text.h=2;text.wrap=true;text.auto=true;text.max=55.0;text.min=30.0;}
        }
        self.account_code.clear();self.account_focus=true;self.account_status.clear();self.backdrop=None;
        self.account=Some(node);
    }

    pub fn draw_account(&mut self,hud:&mut Hud,player:Option<&str>) {
        const BASE:&str="Canvas/LinkAccountScreen/Login/AnimatedObject";
        let node=self.account.as_mut().unwrap();
        let busy=self.account_job.is_some();
        if !busy && player.is_none() && self.account_focus {
            while let Some(c)=get_char_pressed(){if c.is_ascii_alphanumeric() || c=='-' {if self.account_code.len()<25{self.account_code.push(c.to_ascii_uppercase());}}}
            if is_key_pressed(KeyCode::Backspace){self.account_code.pop();}
            if (is_key_down(KeyCode::LeftControl)||is_key_down(KeyCode::RightControl)) && is_key_pressed(KeyCode::V) {
                if let Some(text)=macroquad::miniquad::window::clipboard_get(){self.account_code=text.trim().chars().filter(|c|c.is_ascii_alphanumeric()||*c=='-').take(25).map(|c|c.to_ascii_uppercase()).collect();}
            }
        }
        let placeholder=if self.account_focus{"Linking code |"}else{"Linking code"};
        node.set_text(&format!("{BASE}/Title"),player.unwrap_or("Sign in"));
        node.set_active(&format!("{BASE}/EmailBox"),player.is_none());
        node.set_active(&format!("{BASE}/EmailTxtField"),player.is_none());
        if let Some(field)=node.find_mut(&format!("{BASE}/EmailTxtField")){field.rt.pos.x=0.0;}
        if let Some(field)=node.find_mut(&format!("{BASE}/EmailBox")){field.rt.pos.x=0.0;}
        node.set_text(&format!("{BASE}/EmailTxtField"),if self.account_code.is_empty(){placeholder}else{&self.account_code});
        let status=if busy {"Please wait...".to_owned()}
            else if !self.account_status.is_empty(){self.account_status.clone()}
            else {player.map(|_|"Signed in".to_owned())
                .unwrap_or_else(||"Enter a linking code from the official game.".to_owned())};
        node.set_text(&format!("{BASE}/PasswordTitle"),&status);
        node.set_text(&format!("{BASE}/OKButton/Text"),if busy{"Please wait..."}else if player.is_some(){"Log out"}else{"Sign in"});
        draw_rectangle(0.0,0.0,screen_width(),screen_height(),Color::new(0.0,0.0,0.0,0.65));
        let scale=Hud::canvas_scale();let canvas=vec2(screen_width()/scale,screen_height()/scale);
        let mut ctx=Ctx{sprites:&mut hud.sprites,fonts:&hud.fonts,scale,canvas_h:canvas.y,clip:None};
        draw_tree(node,R{min:Vec2::ZERO,size:canvas},&mut ctx);
        if is_mouse_button_pressed(MouseButton::Left) {
            let (x,y)=mouse_position();self.account_focus=node.rect(&format!("{BASE}/EmailBox")).is_some_and(|r|r.contains(vec2(x,y)));
        }
        if !busy && (node.clicked(&format!("{BASE}/OKButton")) || (self.account_focus && is_key_pressed(KeyCode::Enter))) {
            if player.is_some() {
                if std::env::var_os("RRBTD6_AUTOTEST").is_none(){self.account_job=Some(sign_in::logout(&self.root));}
            } else if !self.account_code.contains('-') || self.account_code.len()<5 {
                self.account_status="Enter the full linking code, including its hyphen.".to_owned();
            } else if std::env::var_os("RRBTD6_AUTOTEST").is_none() {
                self.account_job=Some(sign_in::start(&self.root,std::mem::take(&mut self.account_code)));
                self.account_focus=false;
            }
        }
        if node.clicked(&format!("{BASE}/CancelButton")) || is_key_pressed(KeyCode::Escape) {
            self.account_code.clear();self.account=None;self.backdrop=None;
        }
    }

}
