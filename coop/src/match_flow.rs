//! Host-ordered input batches. The renderer must apply a batch before acknowledging it.
//! No render rate or wall clock enters the shared command order.
use super::*;
use std::collections::VecDeque;

pub const STEPS_PER_FRAME:u8=6;
pub const IN_FLIGHT:usize=8;
const INPUTS_PER_PLAYER:usize=32;

#[derive(Clone,Debug,Serialize,Deserialize,PartialEq)]
pub enum Action{
    Place{kind:String,at:[f32;2]},Upgrade{tower:u64,path:u8},Sell{tower:u64},
    Target{tower:u64,mode:u8},Point{tower:u64,at:[f32;2]},
    Ability{tower:u64,index:u16,at:Option<[f32;2]>,target:Option<u64>},StartRound,
    Pointer{at:[f32;2]},Collect{at:[f32;2]},
    Speed{value:u8},AutoStart{value:bool},Freeplay,
    Special{tower:u64},
    Donate{to:u8,amount:f64},RequestCash,
}
impl Action{
    pub fn valid(&self)->bool{
        let point=|p:&[f32;2]|p.iter().all(|v|v.is_finite()&&v.abs()<=4096.0);
        match self{
            Self::Donate{to,amount}=>*to<MAX_PLAYERS as u8&&amount.is_finite()&&*amount>0.0,
            Self::Place{kind,at}=>!kind.is_empty()&&kind.len()<=80&&kind.bytes().all(|b|b.is_ascii_alphanumeric()||b==b'_')&&point(at),
            Self::Upgrade{path,..}=>*path<3,Self::Target{mode,..}=>*mode<32,
            Self::Point{at,..}|Self::Pointer{at}|Self::Collect{at}=>point(at),
            Self::Ability{index,at,target,..}=>*index<256&&at.as_ref().is_none_or(point)&&!(at.is_some()&&target.is_some()),Self::Speed{value}=>matches!(value,1|3),_=>true,
        }
    }
}
#[derive(Clone,Debug,Serialize,Deserialize,PartialEq)]
pub struct Input{pub slot:u8,pub sequence:u64,pub action:Action}
#[derive(Clone,Debug,Serialize,Deserialize,PartialEq)]
pub struct Frame{pub number:u64,pub actions:Vec<Input>}
impl Frame{
    pub(crate) fn valid(&self,room:&Room)->bool{
        self.actions.len()<=MAX_PLAYERS*INPUTS_PER_PLAYER&&self.actions.iter().all(|i|i.action.valid()&&room.players.iter().any(|p|p.slot==i.slot))
    }
}
#[derive(Clone,Debug,Serialize,Deserialize,PartialEq)]
pub struct MatchStart{pub room:Room,pub steps_per_frame:u8,pub seed:i32}

#[derive(Default)]
pub(crate) struct Flow{
    pub start:Option<MatchStart>,pub running:bool,
    loaded:[Option<[u8;32]>;MAX_PLAYERS],sequences:[u64;MAX_PLAYERS],
    pending:Vec<Input>,next:u64,receipts:VecDeque<(u64,[Option<[u8;32]>;MAX_PLAYERS])>,
    pub progress:u64,
}
impl Flow{
    pub fn begin(&mut self,room:&Room)->Result<MatchStart,String>{
        if self.start.is_some(){return Err("Match has already started".into());}
        if !room.valid()||room.players.len()<2||room.players.iter().any(|p|!p.ready||!p.connected){return Err("At least two connected players must be ready".into());}
        let mut bytes=[0;4];getrandom::fill(&mut bytes).map_err(|_|"Could not generate a shared simulation seed")?;
        let seed=(u32::from_le_bytes(bytes)%2147483646) as i32+1;
        let start=MatchStart{room:room.clone(),steps_per_frame:STEPS_PER_FRAME,seed};
        self.start=Some(start.clone());self.next=1;Ok(start)
    }
    fn slots(&self)->Result<Vec<usize>,String>{
        Ok(self.start.as_ref().ok_or("Match has not started")?.room.players.iter().map(|p|p.slot as usize).collect())
    }
    pub fn waiting(&self)->bool{self.start.is_some()&&(!self.running||!self.receipts.is_empty())}
    pub fn loaded(&mut self,slot:u8,digest:[u8;32])->Result<bool,String>{
        let slots=self.slots()?;
        if self.running||!slots.contains(&(slot as usize)){return Err("Invalid loading acknowledgement".into());}
        if self.loaded[slot as usize].is_some(){return Err("Duplicate loading acknowledgement".into());}
        if self.loaded.iter().flatten().any(|d|*d!=digest){return Err("Initial simulation states differ".into());}
        self.loaded[slot as usize]=Some(digest);
        self.progress+=1;
        self.running=slots.iter().all(|&s|self.loaded[s].is_some());Ok(self.running)
    }
    pub fn input(&mut self,slot:u8,sequence:u64,action:Action)->Result<(),String>{
        let slots=self.slots()?;
        if !self.running||!slots.contains(&(slot as usize))||!action.valid(){return Err("Invalid match input".into());}
        if sequence!=self.sequences[slot as usize]+1{return Err("Input sequence is out of order".into());}
        if self.pending.iter().filter(|i|i.slot==slot).count()>=INPUTS_PER_PLAYER{return Err("Too many pending inputs".into());}
        self.sequences[slot as usize]=sequence;self.pending.push(Input{slot,sequence,action});Ok(())
    }
    pub fn advance(&mut self)->Result<Option<Frame>,String>{
        if !self.running{return Err("Players are still loading".into());}
        if self.receipts.len()>=IN_FLIGHT{return Ok(None);}
        let frame=Frame{number:self.next,actions:std::mem::take(&mut self.pending)};
        self.next+=1;self.receipts.push_back((frame.number,[None;MAX_PLAYERS]));Ok(Some(frame))
    }
    pub fn ack(&mut self,slot:u8,frame:u64,digest:[u8;32])->Result<(),String>{
        let slots=self.slots()?;
        if !self.running||!slots.contains(&(slot as usize)){return Err("Invalid frame acknowledgement".into());}
        let (_,acks)=self.receipts.iter_mut().find(|(n,_)|*n==frame).ok_or("Unknown simulation frame")?;
        if acks[slot as usize].is_some(){return Err("Duplicate frame acknowledgement".into());}
        if acks.iter().flatten().any(|d|*d!=digest){return Err("Simulation diverged; match stopped".into());}
        acks[slot as usize]=Some(digest);
        self.progress+=1;
        while self.receipts.front().is_some_and(|(_,a)|slots.iter().all(|&s|a[s].is_some())){self.receipts.pop_front();}
        Ok(())
    }
}

#[cfg(test)]mod tests{
    use super::*;
    #[test]fn invalid_donations_are_rejected_before_sequencing(){
        let mut f=running();
        for action in [Action::Donate{to:1,amount:f64::NAN},Action::Donate{to:1,amount:f64::INFINITY},Action::Donate{to:1,amount:-1.0},Action::Donate{to:4,amount:100.0}]{assert!(f.input(0,1,action).is_err());}
        f.input(0,1,Action::Donate{to:1,amount:100.0}).unwrap();assert_eq!(f.advance().unwrap().unwrap().actions.len(),1);
    }
    #[test]fn ability_target_shape_is_unambiguous(){
        assert!(Action::Ability{tower:1,index:0,at:Some([2.0,3.0]),target:None}.valid());
        assert!(Action::Ability{tower:1,index:0,at:None,target:Some(2)}.valid());
        assert!(!Action::Ability{tower:1,index:0,at:Some([2.0,3.0]),target:Some(2)}.valid());
    }
    fn room()->Room{let mut s=State::new(Settings::default(),"Host").unwrap();s.join("Guest").unwrap();s.ready(0,true).unwrap();s.ready(1,true).unwrap();s.room}
    fn running()->Flow{let mut f=Flow::default();f.begin(&room()).unwrap();assert!(!f.loaded(0,[7;32]).unwrap());assert!(f.loaded(1,[7;32]).unwrap());f}
    #[test]fn loading_barrier_and_initial_digest(){let mut f=Flow::default();let mut r=room();r.players[1].ready=false;assert!(f.begin(&r).is_err());r.players[1].ready=true;f.begin(&r).unwrap();assert!(f.advance().is_err());f.loaded(0,[1;32]).unwrap();assert!(f.loaded(1,[2;32]).is_err());assert!(!f.running);assert!(f.loaded(1,[1;32]).unwrap());}
    #[test]fn host_assigns_slots_and_orders_inputs(){let mut f=running();f.input(1,1,Action::StartRound).unwrap();f.input(0,1,Action::Sell{tower:5}).unwrap();assert!(f.input(1,1,Action::StartRound).is_err());assert!(f.input(3,1,Action::StartRound).is_err());let frame=f.advance().unwrap().unwrap();assert_eq!(frame.number,1);assert_eq!(frame.actions.iter().map(|i|i.slot).collect::<Vec<_>>(),vec![1,0]);assert!(frame.valid(&room()));}
    #[test]fn slow_peer_bounds_memory_and_hash_mismatch_stops_progress(){let mut f=running();for _ in 0..IN_FLIGHT{assert!(f.advance().unwrap().is_some());}assert!(f.advance().unwrap().is_none());f.ack(0,1,[1;32]).unwrap();assert!(f.ack(1,1,[2;32]).is_err());assert!(f.advance().unwrap().is_none());f.ack(1,1,[1;32]).unwrap();assert!(f.advance().unwrap().is_some());assert!(f.ack(1,1,[1;32]).is_err());}
    #[test]fn invalid_coordinates_and_per_player_input_cap(){let mut f=running();assert!(f.input(1,1,Action::Pointer{at:[f32::NAN,0.0]}).is_err());for n in 1..=INPUTS_PER_PLAYER{f.input(1,n as u64,Action::StartRound).unwrap();}assert!(f.input(1,33,Action::StartRound).is_err());}
}
