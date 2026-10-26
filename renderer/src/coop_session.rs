//! UI-owned connection lifetime. No sockets, hashing or relay lookups on render frames.
use rrbtd6_coop::{Client,Command,Event,Invite,Options,Room,Settings,Action,Frame,MatchStart};
use std::path::Path;

#[derive(Default)]
pub(super) struct Session{
    client:Option<Client>,pub room:Option<Room>,pub slot:Option<u8>,pub invite:Option<String>,
    pub status:String,pub error:bool,pub host:bool,pub revision:u64,pub latency:Option<u32>,
    pub prepared:Option<MatchStart>,pub running:bool,pub frames:std::collections::VecDeque<Frame>,sequence:u64,
    start_requested:bool,test_readied:bool,
}
impl Session{
    fn options(root:&Path)->Options{
        let mut options=Options::new(super::account_preview::player_name().unwrap_or_else(||"Player".into()),"rrbtd6-private-coop-1");
        options.identity_files.push(root.join("game/game.json"));
        if let Ok(exe)=std::env::current_exe(){options.identity_files.push(exe);}
        options.force_relay=std::env::var_os("RRBTD6_COOP_RELAY_ONLY").is_some();options
    }
    pub fn active(&self)->bool{self.client.is_some()}
    pub fn host_or_update(&mut self,root:&Path,settings:Settings){
        if self.active()&&self.host{self.send(Command::Settings(settings));return;}
        self.leave();self.host=true;self.status="Opening private room...".into();self.error=false;
        self.client=Some(Client::host(Self::options(root),settings));self.revision+=1;
    }
    pub fn join(&mut self,root:&Path,code:&str)->bool{
        let invite=match Invite::parse(code){Ok(i)=>i,Err(e)=>{self.status=e;self.error=true;return false;}};
        self.leave();self.invite=Some(invite.encode());self.status="Connecting to private room...".into();self.error=false;
        self.client=Some(Client::join(Self::options(root),invite));self.revision+=1;true
    }
    pub fn leave(&mut self){
        self.client=None;self.room=None;self.slot=None;self.invite=None;self.host=false;
        self.error=false;self.status.clear();self.latency=None;self.revision+=1;
        self.prepared=None;self.running=false;self.frames.clear();self.sequence=0;
        self.start_requested=false;self.test_readied=false;
    }
    pub fn send(&mut self,command:Command){
        if let Some(client)=&self.client{if let Err(e)=client.send(command){self.status=e;self.error=true;self.revision+=1;}}
    }
    pub fn ready(&mut self){
        if self.host&&self.room.as_ref().is_some_and(|r|r.players.len()>1&&r.players.iter().all(|p|p.ready)){
            // Keep unsupported rule variants out of this initial playable prototype.
            if self.room.as_ref().is_some_and(|r|r.settings.division!=4||r.settings.mode!=0){self.status="This prototype requires Standard mode and Free For All areas".into();self.error=true;self.revision+=1;return;}
            if !self.start_requested{self.send(Command::Start);self.start_requested=true;}return;
        }
        if let (Some(slot),Some(room))=(self.slot,&self.room){
            let ready=room.players.iter().find(|p|p.slot==slot).is_some_and(|p|p.ready);
            self.send(Command::Ready(!ready));
        }
    }
    pub fn input(&mut self,action:Action){
        if !self.running||self.error{return;}
        if let Some(client)=&self.client{
            let sequence=self.sequence+1;
            match client.send(Command::Input{sequence,action}){Ok(())=>self.sequence=sequence,Err(e)=>{self.status=e;self.error=true;self.revision+=1;}}
        }
    }
    pub fn poll(&mut self){
        let events:Vec<_>=self.client.as_ref().map(|c|c.poll().collect()).unwrap_or_default();
        for event in events{
            match event{
                Event::Hosting{invite}=>{
                    if let Some(path)=std::env::var_os("RRBTD6_COOP_INVITE_OUT"){let _=std::fs::write(path,&invite);}
                    self.invite=Some(invite);
                },
                Event::Joined{slot}=>{self.slot=Some(slot);self.status="Connected".into();self.error=false;},
                Event::Room(room)=>{self.room=Some(room);self.status="Connected".into();self.error=false;},
                Event::Error(e)=>{self.status=e;self.error=true;},
                Event::Disconnected(e)=>{self.client=None;self.slot=None;self.room=None;self.invite=None;self.status=e;self.error=true;},
                Event::Latency(ms)=>{self.latency=Some(ms);continue;},
                Event::Prepare(setup)=>{self.prepared=Some(setup);self.status="Loading private co-op...".into();},
                Event::Running=>{self.running=true;self.status="Private co-op: individual wallets".into();},
                Event::Frame(frame)=>{self.frames.push_back(frame);},
            }
            self.revision+=1;
        }
        if self.client.as_ref().is_some_and(Client::is_closed){
            self.client=None;self.room=None;self.slot=None;self.invite=None;
            if !self.error{self.status="Connection closed".into();self.error=true;}
            self.revision+=1;
        }
        if std::env::var_os("RRBTD6_COOP_PLAYTEST").is_some()&&!self.error&&self.prepared.is_none()&&!self.running{
            let players=std::env::var("RRBTD6_COOP_TEST_PLAYERS").ok().and_then(|s|s.parse::<usize>().ok()).unwrap_or(2).clamp(2,4);
            if self.room.as_ref().is_some_and(|r|r.players.len()==players){
                if !self.test_readied{self.ready();self.test_readied=true;}
                else if self.host&&self.room.as_ref().is_some_and(|r|r.players.iter().all(|p|p.ready)){self.ready();}
            }
        }
    }
}
