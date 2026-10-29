//! Private client-to-client rooms over encrypted QUIC, with automatic NAT traversal/relay.
//! Network work starts explicitly, off the renderer thread. No official account credentials.
mod transport;
pub mod mods;
pub use mods::ModPackage;
mod match_flow;
pub mod economy;
pub use match_flow::{Action,Input,Frame,MatchStart,STEPS_PER_FRAME,IN_FLIGHT};
pub use transport::{Client, Options};
use serde::{Deserialize,Serialize};

pub const VERSION:u32=8;
pub const MAX_PLAYERS:usize=4;
pub const MAX_FRAME:usize=64*1024;
pub const ALPN:&[u8]=b"rrbtd6/private-coop/1";

#[derive(Clone,Debug,Serialize,Deserialize,PartialEq)]
pub struct Settings {
    #[serde(default)]pub custom_mode:Option<String>,
    pub map:String,pub difficulty:u8,pub mode:u8,pub division:u8,pub closed:[bool;MAX_PLAYERS],
}
impl Default for Settings{fn default()->Self{Self{custom_mode:None,map:"Tutorial".into(),difficulty:1,mode:0,division:4,closed:[false;4]}}}
impl Settings{
    pub fn valid(&self)->bool{
        self.custom_mode.as_ref().is_none_or(|id|!id.is_empty()&&id.len()<=96&&id.bytes().all(|b|b.is_ascii_alphanumeric()||b"._-".contains(&b)))&&
        !self.map.is_empty()&&self.map.len()<=80&&self.map.bytes().all(|b|b.is_ascii_alphanumeric()||b==b'_')&&
        self.difficulty<3&&self.mode<[3,4,7][self.difficulty.min(2) as usize]&&self.division<7&&!self.closed[0]
    }
}
#[derive(Clone,Debug,Serialize,Deserialize,PartialEq)]
pub struct Player{pub slot:u8,pub name:String,pub ready:bool,pub connected:bool}
#[derive(Clone,Debug,Serialize,Deserialize,PartialEq)]
pub struct Room{pub revision:u64,pub settings:Settings,pub players:Vec<Player>,#[serde(default)]pub mods:Vec<ModPackage>}
impl Room{
    fn valid(&self)->bool{
        if mods::validate(&self.mods).is_err(){return false;}
        if self.settings.custom_mode.as_ref().is_some_and(|id|!mods::mode_ids(&self.mods).contains(id)){return false;}
        if !self.settings.valid()||self.players.is_empty()||self.players.len()>MAX_PLAYERS{return false;}
        let mut seen=[false;MAX_PLAYERS];
        for p in &self.players{
            if p.slot as usize>=MAX_PLAYERS||seen[p.slot as usize]||p.name.chars().count()>32||p.name.chars().any(char::is_control){return false;}
            seen[p.slot as usize]=true;
        }
        seen[0]
    }
}

#[derive(Clone,Debug)]
pub enum Command{Settings(Settings),Mods(Vec<ModPackage>),Ready(bool),Leave,Start,Loaded([u8;32]),Input{sequence:u64,action:Action},Advance,Ack{frame:u64,digest:[u8;32]}}
#[derive(Clone,Debug)]
pub enum Event{
    Hosting{invite:String},Joined{slot:u8},Room(Room),Error(String),Disconnected(String),
    Latency(u32),
    Prepare(MatchStart),Running,Frame(Frame),
}

#[derive(Clone,Debug,Serialize,Deserialize)]
enum Wire{
    Hello{version:u32,token:String,identity:String,name:String},
    Welcome{version:u32,slot:u8,room:Room},Room(Room),Settings(Settings),Ready(bool),
    Ping(u64),Pong(u64),Reject(String),
    Prepare(MatchStart),Loaded([u8;32]),Running,Input{sequence:u64,action:Action},Frame(Frame),Ack{frame:u64,digest:[u8;32]},
}
#[derive(Clone,Debug)]
pub struct Invite{pub endpoint:iroh::EndpointId,token:String}
impl Invite{
    pub fn parse(text:&str)->Result<Self,String>{
        let text=text.trim().strip_prefix("rrbtd6://join/").unwrap_or(text.trim());
        if text.len()>160{return Err("Invalid invite code".into());}
        let mut parts=text.split('-');
        if parts.next()!=Some("RR1"){return Err("Paste an RR1 invite code from this client".into());}
        let endpoint=parts.next().ok_or("Invalid invite code")?.parse().map_err(|_|"Invalid invite address")?;
        let token=parts.next().ok_or("Invalid invite code")?;
        if token.len()!=32||!token.bytes().all(|b|b.is_ascii_hexdigit())||parts.next().is_some(){return Err("Invalid invite code".into());}
        Ok(Self{endpoint,token:token.to_ascii_lowercase()})
    }
    pub fn encode(&self)->String{format!("RR1-{}-{}",self.endpoint,self.token)}
}
fn name(text:&str)->String{
    let n:String=text.chars().filter(|c|!c.is_control()).take(32).collect();
    if n.trim().is_empty(){"Player".into()}else{n}
}

/// Pure room rules. Connections supply the sender slot; the payload cannot impersonate a host.
struct State{room:Room}
impl State{
    fn new(settings:Settings,host:&str)->Result<Self,String>{
        if !settings.valid(){return Err("Invalid room settings".into());}
        Ok(Self{room:Room{revision:1,settings,players:vec![Player{slot:0,name:name(host),ready:false,connected:true}],mods:Vec::new()}})
    }
    fn join(&mut self,player:&str)->Result<u8,String>{
        let slot=(1..MAX_PLAYERS).find(|&s|!self.room.settings.closed[s]&&!self.room.players.iter().any(|p|p.slot as usize==s)).ok_or("Room is full")?;
        self.room.players.push(Player{slot:slot as u8,name:name(player),ready:false,connected:true});
        self.room.revision+=1;Ok(slot as u8)
    }
    fn leave(&mut self,slot:u8){self.room.players.retain(|p|p.slot!=slot);self.room.revision+=1;}
    fn settings(&mut self,slot:u8,settings:Settings)->Result<(),String>{
        if slot!=0{return Err("Only the host can change the room".into());}
        if !settings.valid(){return Err("Invalid room settings".into());}
        if settings.custom_mode.as_ref().is_some_and(|id|!mods::mode_ids(&self.room.mods).contains(id)){return Err("Selected mode is not provided by the room mods".into());}
        if self.room.players.iter().any(|p|settings.closed[p.slot as usize]){return Err("A connected player's slot cannot be closed".into());}
        if self.room.settings!=settings{self.room.settings=settings;for p in &mut self.room.players{p.ready=false;}self.room.revision+=1;}
        Ok(())
    }
    fn mods(&mut self,slot:u8,mods:Vec<ModPackage>)->Result<(),String>{
        if slot!=0{return Err("Only the host can select room mods".into());}
        mods::validate(&mods)?;
        if self.room.mods!=mods{
            if self.room.settings.custom_mode.as_ref().is_some_and(|id|!mods::mode_ids(&mods).contains(id)){self.room.settings.custom_mode=None;self.room.settings.mode=0;}
            self.room.mods=mods;for p in &mut self.room.players{p.ready=false;}self.room.revision+=1;}
        Ok(())
    }
    fn ready(&mut self,slot:u8,ready:bool)->Result<(),String>{
        let p=self.room.players.iter_mut().find(|p|p.slot==slot).ok_or("Player has left")?;
        if p.ready!=ready{p.ready=ready;self.room.revision+=1;}Ok(())
    }
}

#[cfg(test)] mod tests{
    use super::*;
    #[test]fn host_permissions_capacity_and_setting_changes_clear_ready(){
        let mut s=State::new(Settings::default(),"Host").unwrap();
        for i in 1..4{assert_eq!(s.join("Guest").unwrap(),i);}
        assert!(s.join("Extra").is_err());assert!(s.settings(1,Settings::default()).is_err());
        s.ready(1,true).unwrap();let mut settings=s.room.settings.clone();settings.difficulty=2;s.settings(0,settings).unwrap();
        assert!(s.room.players.iter().all(|p|!p.ready));
        s.leave(2);assert_eq!(s.join("Replacement").unwrap(),2);
    }
    #[test]fn invalid_and_occupied_slot_settings_are_rejected(){
        let mut s=State::new(Settings::default(),"Host").unwrap();s.join("Guest").unwrap();
        let mut settings=s.room.settings.clone();settings.closed[1]=true;assert!(s.settings(0,settings).is_err());
        let mut settings=s.room.settings.clone();settings.map="../save".into();assert!(s.settings(0,settings).is_err());
        assert!(Invite::parse("123456").is_err());assert!(Invite::parse(&"x".repeat(1000)).is_err());
    }
    #[test]fn remote_rooms_require_unique_slots_and_a_host(){
        let mut s=State::new(Settings::default(),"Host").unwrap();
        s.join("Guest").unwrap();assert!(s.room.valid());
        let original=s.room.clone();
        s.room.players[1].slot=0;assert!(!s.room.valid());
        s.room=original.clone();s.room.players.remove(0);assert!(!s.room.valid());
        s.room=original.clone();s.room.players[1].slot=4;assert!(!s.room.valid());
        s.room=original;s.room.players[1].name="Hidden\nname".into();assert!(!s.room.valid());
    }
    #[test]fn closed_slots_are_skipped_and_invalid_ready_requests_do_not_mutate(){
        let mut settings=Settings::default();settings.closed[1]=true;
        let mut s=State::new(settings,"Host").unwrap();assert_eq!(s.join("Guest").unwrap(),2);
        let revision=s.room.revision;assert!(s.ready(3,true).is_err());assert_eq!(s.room.revision,revision);
        s.ready(2,true).unwrap();let revision=s.room.revision;
        s.settings(0,s.room.settings.clone()).unwrap();assert_eq!(s.room.revision,revision);
        assert!(s.room.players.iter().find(|p|p.slot==2).unwrap().ready);
    }
    #[test]
    fn room_mod_changes_are_host_only_and_reset_readiness(){
        let mut state=State::new(Settings::default(),"Host").unwrap();state.join("Guest").unwrap();
        state.ready(0,true).unwrap();state.ready(1,true).unwrap();
        let package=ModPackage::new(r#"{"api_version":1,"id":"example","version":"1"}"#.into()).unwrap();
        assert!(state.mods(1,vec![package.clone()]).is_err());
        state.mods(0,vec![package]).unwrap();assert!(state.room.players.iter().all(|p|!p.ready));assert!(state.room.valid());
    }

    #[test]
    fn custom_room_mode_requires_its_mod_and_removal_returns_to_standard(){
        let mut state=State::new(Settings::default(),"Host").unwrap();
        let mut settings=state.room.settings.clone();settings.custom_mode=Some("test-mode".into());
        assert!(state.settings(0,settings.clone()).is_err());
        let package=ModPackage::new(r#"{"api_version":1,"id":"test","version":"1","content":[{"op":"mode","id":"test-mode","base":"Medium"}]}"#.into()).unwrap();
        state.mods(0,vec![package]).unwrap();state.settings(0,settings).unwrap();assert!(state.room.valid());
        state.mods(0,vec![]).unwrap();assert!(state.room.settings.custom_mode.is_none());assert_eq!(state.room.settings.mode,0);
    }

}
