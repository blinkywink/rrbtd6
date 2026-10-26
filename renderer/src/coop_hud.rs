//! Original CoopManager prefab; connection state stays outside the UI tree.
use super::*;
const GROUP:&str="SliderPanel/SliderGroup";
const PLAYERS:&str="SliderPanel/SliderGroup/PlayerSlider/Players";
pub(super) struct CoopHud{tree:Option<Node>,open:bool,active:bool,runtime:serde_json::Value,changed:Option<f64>}
impl CoopHud{
    pub fn load(dir:&Path)->Self{
        let mut tree=Node::load(&dir.join("CoopManager.json"));
        if let Some(n)=&mut tree{
            for p in ["AreaIdicators","EmoteManager","CoopPopups","SliderPanel/CoopButton/ButtonGlow","SliderPanel/CoopButton/Notice"]{n.set_active(p,false);}
            n.set_active(&format!("{GROUP}/EmoteSliderPanel"),false);
            n.set_active(&format!("{GROUP}/EmoticButton"),false);
            n.set_active(&format!("{GROUP}/PlayersButton"),false);
            n.set_active(&format!("{GROUP}/PlayerSlider/CashRequestIndicator"),false);
        }
        let mut runtime:serde_json::Value=std::fs::read_to_string(dir.join("../runtime/CoopManager.json")).ok().and_then(|s|serde_json::from_str(&s).ok()).unwrap_or_default();
        for animator in runtime["animators"].as_array_mut().into_iter().flatten(){
            for clip in animator["clips"].as_array_mut().into_iter().flatten(){
                for binding in clip["bindings"].as_array_mut().into_iter().flatten(){
                    if let Some(path)=binding["path"].as_str().and_then(|p|p.strip_prefix("CoopManager/")){binding["path"]=path.to_owned().into();}
                }
            }
        }
        Self{tree,open:std::env::var_os("RRBTD6_COOP_PANEL").is_some(),active:false,runtime,changed:None}
    }
    pub fn blocks(&self,p:Vec2)->bool{
        self.active&&self.tree.as_ref().is_some_and(|n|n.rect("SliderPanel/CoopButton/Button").is_some_and(|r|r.contains(p))||
            (self.open&&n.rect(GROUP).is_some_and(|r|r.contains(p))))
    }
}
impl Hud{
    pub fn coop_blocks(&self,p:Vec2)->bool{self.coop.blocks(p)}
    pub fn draw_coop(&mut self,game:&Game,setup:Option<&rrbtd6_coop::MatchStart>,slot:u8,requests:Option<&[Option<u64>;4]>,frame:u64,out:&mut Vec<Action>){
        self.coop.active=setup.is_some();let Some(setup)=setup else{return;};
        let Some(n)=&mut self.coop.tree else{return;};
        let was_open=self.coop.open;
        if n.clicked("SliderPanel/CoopButton/Button"){self.coop.open=!self.coop.open;}
        if self.coop.open&&n.clicked(&format!("{GROUP}/CloseButton")){self.coop.open=false;}
        if was_open!=self.coop.open{self.coop.changed=Some(get_time());}
        if self.coop.open&&self.tsm.active{out.push(Action::Close);}
        n.set_active(GROUP,self.coop.open);
        let elapsed=self.coop.changed.map(|t|(get_time()-t) as f32);
        if crate::frontend::apply_animation(n,&self.coop.runtime,"CoopPlayersSlideOut",if self.coop.open{"4PSlideOutOpen"}else{"4PSlideOutClose"},elapsed){self.coop.changed=None;}
        let cash=game.coop_wallets.as_ref().map(|w|w.balances).unwrap_or([0.0;4]);
        let amount=rrbtd6_coop::economy::donation_amount(cash[slot as usize]);
        for player in 0..4u8{
            let path=format!("{PLAYERS}/Player{}Info",player+1);
            let entry=setup.room.players.iter().find(|p|p.slot==player);
            n.set_active(&path,entry.is_some()&&player!=slot);let Some(entry)=entry else{continue;};
            n.set_text(&format!("{path}/PlayerName"),&entry.name);
            n.set_sprite(&format!("{path}/PlayerId"),Some(&format!("CoopPlayer{}Icon",player+1)));
            n.set_active(&format!("{path}/IsHost"),player==0);
            for child in ["CoopDisconnectedPanel","MuteButton","MonkeysRemaining"]{n.set_active(&format!("{path}/{child}"),false);}
            let detail=format!("{path}/CoopActiveDetail");
            n.set_text(&format!("{detail}/GoldAmountDetail/Amount"),&money(cash[player as usize]));
            let give=format!("{detail}/GoldGiveBtn");
            n.set_active(&give,player!=slot);n.set_text(&format!("{give}/Txt"),&format!("Give {}",money(amount)));
            n.set_active(&format!("{detail}/Notice"),requests.and_then(|r|r[player as usize]).is_some_and(|at|frame.saturating_sub(at)<50));
            if self.coop.open&&player!=slot&&amount>0.0&&n.clicked(&give){out.push(Action::Coop(rrbtd6_coop::Action::Donate{to:player,amount}));}
        }
        let request=format!("{PLAYERS}/RequestActionButton/Button");n.set_text(&format!("{request}/Txt"),"Request Cash");
        if self.coop.open&&n.clicked(&request){out.push(Action::Coop(rrbtd6_coop::Action::RequestCash));}
        let s=Self::canvas_scale();let canvas=vec2(screen_width()/s,screen_height()/s);
        let map=R{min:Vec2::ZERO,size:vec2(canvas.x-SHOP_W,canvas.y)};
        let mut ctx=Ctx{sprites:&mut self.sprites,fonts:&self.fonts,scale:s,canvas_h:canvas.y,clip:None};
        draw_tree(n,map,&mut ctx);
    }
}
