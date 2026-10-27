//! Shared account card: independent local identity and optional cloud sign-in.
use crate::{hud::Hud,uitree::{Node,Rt,Ctx,R,draw_tree}};
use macroquad::prelude::*;
use serde_json::Value;
use std::path::{Path,PathBuf};
#[path="sign_in.rs"] mod sign_in;
#[derive(Default,PartialEq)] enum Focus{ Name,Code,#[default]None }
#[derive(Default)]
pub(crate) struct AccountUi {
    root:PathBuf,account:Option<Node>,account_code:String,account_status:String,
    local_name:String,focus:Focus,account_job:Option<std::sync::mpsc::Receiver<Value>>,
    pub backdrop:Option<crate::menus::Backdrop>,pub generation:u64,
}
fn placed(mut n:Node,name:&str,x:f32,y:f32,w:f32,h:f32)->Node{
    n.name=name.into();n.active=true;n.children.clear();
    n.rt=Rt{amin:Vec2::splat(0.5),amax:Vec2::splat(0.5),pos:vec2(x,y),size:vec2(w,h),..Default::default()};n
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
    fn open_account(&mut self){
        let Some(source)=Node::load(&self.root.join("ui/prefabs/LinkAccountUI.json")) else{return};
        let base="Canvas/LinkAccountScreen/Login/AnimatedObject";
        let Some(background)=source.find(&format!("{base}/Background")) else{return};
        let Some(title)=source.find(&format!("{base}/Title")) else{return};
        let Some(input)=source.find(&format!("{base}/EmailBox")) else{return};
        let Some(button)=source.find(&format!("{base}/OKButton")) else{return};
        let title_source=Node::load(&self.root.join("ui/prefabs/TitleScreenUI.json"));
        let body=title_source.as_ref().and_then(|n|n.find("Canvas/ScreenBoxer/TitleScreen/PlayerInfo/PlayerNameTxt")).unwrap_or(title);
        let mut card=placed(background.clone(),"Card",0.0,0.0,3000.0,1660.0);
        let text=|name:&str,value:&str,x:f32,y:f32,w:f32,h:f32,size:f32,heading:bool|{
            let mut n=placed(if heading{title.clone()}else{body.clone()},name,x,y,w,h);
            n.set_text("",value);
            if let Some(t)=n.txt.as_mut(){t.size=size;t.auto=false;t.h=2;t.v=512;t.wrap=true;t.case=0;t.color=WHITE;}
            n
        };
        let btn=|name:&str,label:&str,x:f32,y:f32,w:f32|{
            let mut n=placed(button.clone(),name,x,y,w,140.0);
            n.children.push(text("Label",label,0.0,0.0,w-60.0,120.0,64.0,true));n
        };
        card.children=vec![
            text("Heading","Your RustTD6 account",0.0,690.0,2700.0,150.0,100.0,true),
            text("Subtitle","Play locally, or connect a Ninja Kiwi account.",0.0,550.0,2700.0,100.0,55.0,false),
            text("LocalHeading","Local profile",-650.0,350.0,1250.0,100.0,70.0,true),
            placed(input.clone(),"NameBox",-650.0,210.0,1250.0,150.0),
            text("Name","",-650.0,210.0,1150.0,130.0,65.0,false),
            btn("SaveName","Save name",-650.0,35.0,700.0),
            text("OnlineHeading","Ninja Kiwi account",-650.0,-170.0,1250.0,110.0,70.0,true),
            placed(input.clone(),"CodeBox",-650.0,-310.0,1250.0,150.0),
            text("Code","",-650.0,-310.0,1150.0,130.0,65.0,false),
            text("Identity","",-650.0,-310.0,1250.0,140.0,60.0,false),
            btn("Submit","Sign in",-650.0,-490.0,700.0),
            placed(input.clone(),"NoticeBox",820.0,-80.0,1100.0,1040.0),
            text("NoticeHeading","You choose how to play",820.0,350.0,970.0,100.0,58.0,true),
            text("Notice", "RustTD6 is open source.\n\nOptional sign-in contacts Ninja Kiwi to sign in and download account data. Client updates come from GitHub.\n\nIf you are concerned about account flags, use a separate Ninja Kiwi account. Account safety is not guaranteed.\n\nYou can stay signed out and use your local profile. Local progress is stored on this computer and is never uploaded to Ninja Kiwi.",820.0,-190.0,940.0,850.0,48.0,false),
            text("Status","",-350.0,-690.0,1950.0,125.0,48.0,false),
            btn("Done","Done",1130.0,-690.0,430.0),
        ];
        let mut root=Node::from_json(&serde_json::json!({"name":"Account","active":true,"children":[]}));root.children.push(card);
        self.local_name=crate::local_profile::get(&self.root).name;
        self.account_code.clear();self.account_status.clear();self.focus=Focus::None;self.backdrop=None;self.account=Some(root);
    }
    pub fn draw_account(&mut self,hud:&mut Hud,player:Option<&str>){
        let busy=self.account_job.is_some();
        let node=self.account.as_mut().unwrap();
        if !busy {
            if is_key_pressed(KeyCode::Tab){self.focus=if self.focus==Focus::Name && player.is_none(){Focus::Code}else{Focus::Name};}
            let field=match self.focus{Focus::Name=>Some(&mut self.local_name),Focus::Code if player.is_none()=>Some(&mut self.account_code),_=>None};
            if let Some(field)=field{
                let code=self.focus==Focus::Code;let limit=if code{25}else{24};
                while let Some(c)=get_char_pressed(){if !c.is_control() && (!code||c.is_ascii_alphanumeric()||c=='-') && field.chars().count()<limit{field.push(if code{c.to_ascii_uppercase()}else{c});}}
                if is_key_pressed(KeyCode::Backspace){field.pop();}
                if (is_key_down(KeyCode::LeftControl)||is_key_down(KeyCode::RightControl))&&is_key_pressed(KeyCode::V){
                    if let Some(value)=macroquad::miniquad::window::clipboard_get(){*field=value.trim().chars().filter(|c|!c.is_control()&&(!code||c.is_ascii_alphanumeric()||*c=='-')).take(limit).map(|c|if code{c.to_ascii_uppercase()}else{c}).collect();}
                }
            }
        }
        let cursor=|focus:bool|if focus && get_time()%1.0<0.5{"|"}else{""};
        node.set_text("Card/Name",&format!("{}{}",self.local_name,cursor(self.focus==Focus::Name)));
        node.set_text("Card/Code",&format!("{}{}",if self.account_code.is_empty(){"Paste linking code"}else{&self.account_code},cursor(self.focus==Focus::Code)));
        node.set_active("Card/Code",player.is_none());node.set_active("Card/CodeBox",player.is_none());
        node.set_active("Card/Identity",player.is_some());node.set_text("Card/Identity",&format!("Signed in as {}",player.unwrap_or("")));
        node.set_text("Card/Submit/Label",if busy{"Working..."}else if player.is_some(){"Log out"}else{"Sign in"});
        node.set_text("Card/Status",if busy{"Connecting..."}else if self.account_status.is_empty(){"Local name, cosmetics and progress are saved on this computer."}else{&self.account_status});
        draw_rectangle(0.0,0.0,screen_width(),screen_height(),Color::new(0.01,0.025,0.055,0.8));
        let scale=Hud::canvas_scale();let canvas=vec2(screen_width()/scale,screen_height()/scale);
        let mut ctx=Ctx{sprites:&mut hud.sprites,fonts:&hud.fonts,scale,canvas_h:canvas.y,clip:None};
        draw_tree(node,R{min:Vec2::ZERO,size:canvas},&mut ctx);
        if is_mouse_button_pressed(MouseButton::Left){self.focus=if node.clicked("Card/NameBox"){Focus::Name}else if player.is_none()&&node.clicked("Card/CodeBox"){Focus::Code}else{Focus::None};}
        if !busy && (node.clicked("Card/SaveName") || (self.focus==Focus::Name&&is_key_pressed(KeyCode::Enter))){
            match crate::local_profile::rename(&self.root,&self.local_name){
                Ok(())=>{crate::frontend::account_preview::refresh_local(&self.root);self.generation+=1;self.account_status="Local name saved.".into();},
                Err(error)=>self.account_status=error,
            }
        }
        if !busy && (node.clicked("Card/Submit") || (self.focus==Focus::Code&&is_key_pressed(KeyCode::Enter))){
            if player.is_some(){if std::env::var_os("RRBTD6_AUTOTEST").is_none(){self.account_job=Some(sign_in::logout(&self.root));}}
            else if !self.account_code.contains('-')||self.account_code.len()<5{self.account_status="Enter the linking code, including its hyphen.".into();}
            else if std::env::var_os("RRBTD6_AUTOTEST").is_none(){self.account_job=Some(sign_in::start(&self.root,std::mem::take(&mut self.account_code)));self.focus=Focus::None;}
        }
        if node.clicked("Card/Done")||is_key_pressed(KeyCode::Escape){self.account=None;self.account_code.clear();self.backdrop=None;}
    }
}
