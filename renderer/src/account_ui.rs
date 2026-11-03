//! Local identity and progression controls.
use crate::{hud::Hud,uitree::{Node,Rt,Ctx,R,draw_tree}};
use macroquad::prelude::*;
use std::path::{Path,PathBuf};
#[derive(Default,PartialEq)] enum Focus{ Name,#[default]None }
#[derive(Default)]
pub(crate) struct AccountUi {
    root:PathBuf,account:Option<Node>,reset_pending:bool,account_status:String,
    local_name:String,focus:Focus,
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
            text("Heading","Your local profile",0.0,690.0,2700.0,150.0,100.0,true),
            text("Subtitle","Co-op, mods and experiments. Everything is ready to play.",0.0,510.0,2700.0,100.0,55.0,false),
            text("LocalHeading","Player name",0.0,300.0,1600.0,100.0,70.0,true),
            placed(input.clone(),"NameBox",0.0,140.0,1600.0,150.0),
            text("Name","",0.0,140.0,1500.0,130.0,65.0,false),
            btn("SaveName","Save name",0.0,-50.0,700.0),
            text("Notice","Your name, level and map medals are saved on this computer.\nAll towers, upgrades, heroes and maps are available immediately.\nNo Ninja Kiwi account or sign-in is used.",0.0,-290.0,2500.0,290.0,53.0,false),
            btn("Reset","Reset game data",0.0,-510.0,1050.0),
            text("Status","",-200.0,-690.0,2000.0,125.0,48.0,false),
            btn("Done","Done",1130.0,-690.0,430.0),
        ];
        let mut root=Node::from_json(&serde_json::json!({"name":"Account","active":true,"children":[]}));root.children.push(card);
        self.local_name=crate::local_profile::get(&self.root).name;
        self.reset_pending=false;self.account_status.clear();self.focus=Focus::None;self.backdrop=None;self.account=Some(root);
    }
    pub fn draw_account(&mut self,hud:&mut Hud,_player:Option<&str>){
        let node=self.account.as_mut().unwrap();
        if is_key_pressed(KeyCode::Tab){self.focus=Focus::Name;}
        if self.focus==Focus::Name {
            while let Some(c)=get_char_pressed(){if !c.is_control() && self.local_name.chars().count()<24{self.local_name.push(c);}}
            if is_key_pressed(KeyCode::Backspace){self.local_name.pop();}
            if (is_key_down(KeyCode::LeftControl)||is_key_down(KeyCode::RightControl))&&is_key_pressed(KeyCode::V){
                if let Some(value)=macroquad::miniquad::window::clipboard_get(){self.local_name=value.trim().chars().filter(|c|!c.is_control()).take(24).collect();}
            }
        }
        let cursor=if self.focus==Focus::Name && get_time()%1.0<0.5{"|"}else{""};
        node.set_text("Card/Name",&format!("{}{}",self.local_name,cursor));
        node.set_text("Card/Reset/Label",if self.reset_pending{"Confirm reset"}else{"Reset game data"});
        node.set_text("Card/Status",if self.reset_pending{"Reset level, stats and solo/co-op medals? Click Confirm reset. Done cancels."}else{&self.account_status});
        draw_rectangle(0.0,0.0,screen_width(),screen_height(),Color::new(0.01,0.025,0.055,0.8));
        let scale=Hud::canvas_scale();let canvas=vec2(screen_width()/scale,screen_height()/scale);
        let mut ctx=Ctx{sprites:&mut hud.sprites,fonts:&hud.fonts,scale,canvas_h:canvas.y,clip:None};
        draw_tree(node,R{min:Vec2::ZERO,size:canvas},&mut ctx);
        if is_mouse_button_pressed(MouseButton::Left){self.focus=if node.clicked("Card/NameBox"){Focus::Name}else{Focus::None};}
        if node.clicked("Card/SaveName") || (self.focus==Focus::Name&&is_key_pressed(KeyCode::Enter)){
            self.reset_pending=false;
            match crate::local_profile::rename(&self.root,&self.local_name){
                Ok(())=>{crate::frontend::account_preview::refresh_local(&self.root);self.generation+=1;self.account_status="Name saved.".into();},
                Err(error)=>self.account_status=error,
            }
        }
        if node.clicked("Card/Reset") {
            if self.reset_pending {
                self.reset_pending=false;
                match crate::local_profile::reset(&self.root){
                    Ok(())=>{crate::frontend::account_preview::refresh_local(&self.root);self.generation+=1;self.account_status="Game data reset. Your name, settings and mods are kept.".into();},
                    Err(error)=>self.account_status=error,
                }
            }else{self.reset_pending=true;}
        }
        if node.clicked("Card/Done")||is_key_pressed(KeyCode::Escape){self.account=None;self.reset_pending=false;self.backdrop=None;}
    }
}
