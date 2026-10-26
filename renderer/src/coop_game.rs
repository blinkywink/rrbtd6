//! Simulation adapter for private co-op. Uses stable tower IDs, never UI indices.
//! Income and costs use per-player wallets; the HUD receives a local projection.
use crate::game::{Game,TargetMode};
use rrbtd6_coop::{Action,Frame,MAX_PLAYERS,STEPS_PER_FRAME};
use std::collections::BTreeMap;

const TARGETS:[TargetMode;17]=[
    TargetMode::First,TargetMode::Last,TargetMode::Close,TargetMode::Strong,
    TargetMode::Pointer,TargetMode::Point,TargetMode::Pursuit,TargetMode::Circle,
    TargetMode::FigureEight,TargetMode::Infinite,TargetMode::Centered,TargetMode::Track,
    TargetMode::CloseTrack,TargetMode::SmartTrack,TargetMode::AutoTrack,TargetMode::Submerge,
    TargetMode::Elite,
];
pub fn target_code(mode:TargetMode)->u8{TARGETS.iter().position(|&m|m==mode).unwrap_or(0) as u8}

pub struct Replica{
    slots:Vec<u8>,pub owners:BTreeMap<u64,u8>,pointers:[Option<[f32;2]>;MAX_PLAYERS],
    sequences:[u64;MAX_PLAYERS],pub frame:u64,
    pub speed:u8,pub auto_start:bool,
    remaining:u8,
    pub placed:Vec<(u8,u64)>,
    pub requests:[Option<u64>;MAX_PLAYERS],
}
impl Replica{
    pub fn new(slots:Vec<u8>)->Result<Self,String>{
        if slots.len()<2||slots.len()>MAX_PLAYERS||!slots.contains(&0)||slots.iter().enumerate().any(|(i,s)|*s as usize>=MAX_PLAYERS||slots[..i].contains(s)){return Err("Invalid co-op players".into());}
        Ok(Self{slots,owners:BTreeMap::new(),pointers:[None;MAX_PLAYERS],sequences:[0;MAX_PLAYERS],frame:0,speed:1,auto_start:false,remaining:0,placed:Vec::new(),requests:[None;MAX_PLAYERS]})
    }
    pub fn owns(&self,slot:u8,id:u64)->bool{self.owners.get(&id)==Some(&slot)}
    pub fn prepare(&mut self,g:&mut Game)->Result<(),String>{
        if g.coop_wallets.is_none(){g.coop_init_cash(self.slots.clone())?;}
        if let Some(w)=&mut g.coop_wallets{for t in &g.towers{if t.sub.is_none(){if let Some(slot)=self.owners.get(&t.id){w.owners.insert(t.id,*slot);}}}}
        Ok(())
    }
    pub fn ability(&self,g:&Game,slot:u8,name:&str)->Option<Action>{
        g.towers.iter().enumerate().filter(|(_,t)|self.owns(slot,t.id)).find_map(|(i,t)|{
            let tier=g.tier(i)?;let (index,_)=tier.abilities.iter().enumerate().find(|(_,a)|a.name==name)?;
            if t.ability_cd.get(name).copied().unwrap_or(0.0)>0.0{return None;}
            Some(Action::Ability{tower:t.id,index:index as u16,at:None,target:None})
        })
    }
    fn tower(&self,g:&Game,slot:u8,id:u64)->Option<usize>{
        if !self.owns(slot,id){return None;}
        g.towers.iter().position(|t|t.id==id&&t.sub.is_none())
    }
    fn inherit(&mut self,g:&mut Game){
        // Newly spawned sentries and pets inherit their creator's owner.
        for tower in &g.towers{
            if let Some(parent)=tower.sub.as_ref().and_then(|s|self.owners.get(&s.parent).copied()){
                self.owners.entry(tower.id).or_insert(parent);
            }
        }
        let alive:std::collections::HashSet<_>=g.towers.iter().map(|t|t.id).collect();
        self.owners.retain(|id,_|alive.contains(id));
        if let Some(w)=&mut g.coop_wallets{for t in &g.towers{if t.sub.is_none(){if let Some(slot)=self.owners.get(&t.id){w.owners.insert(t.id,*slot);}}}}
        g.coop_pointers.clear();
        for (&id,&slot) in &self.owners{g.coop_pointers.insert(id,self.pointers[slot as usize]);}
    }
    fn action(&mut self,g:&mut Game,slot:u8,action:&Action)->bool{
        g.coop_actor(slot);let accepted=self.apply_action(g,slot,action);g.coop_end_actor();accepted
    }
    fn apply_action(&mut self,g:&mut Game,slot:u8,action:&Action)->bool{
        match action{
            Action::Donate{to,amount}=>{
                // The actor fields are only a compatibility view. Synchronize them
                // after the transfer before the wrapper saves that view back.
                let accepted=g.coop_wallets.as_mut().is_some_and(|w|w.transfer(slot,*to,*amount));
                g.coop_restore_cash();accepted
            },
            Action::RequestCash=>{
                let previous=self.requests[slot as usize];
                if previous.is_some_and(|at|self.frame.saturating_sub(at)<50){return false;}
                self.requests[slot as usize]=Some(self.frame);true
            },
            Action::Place{kind,at}=>{
                let Some(def)=g.tower_defs.iter().position(|(id,_)|id==kind)else{return false;};
                if !g.shop.iter().any(|s|s.id==*kind){return false;}
                let before=g.towers.len();
                if !g.place(def,*at){return false;}
                for tower in &g.towers[before..]{self.owners.insert(tower.id,slot);if tower.sub.is_none(){if let Some(w)=&mut g.coop_wallets{w.owners.insert(tower.id,slot);}}}
                true
            },
            Action::Upgrade{tower,path}=>{
                let Some(t)=self.tower(g,slot,*tower)else{return false;};
                // Sacrifices and paragons need explicit cross-player ownership rules.
                if g.temple_prompt(t,*path as usize).is_some()||g.towers[t].tiers[*path as usize]>=5{return false;}
                g.upgrade(t,*path as usize)
            },
            Action::Sell{tower}=>{
                let Some(t)=self.tower(g,slot,*tower)else{return false;};
                // Selling a ship must not silently sell another player's deck towers.
                if g.towers.iter().any(|other|other.deck.is_some_and(|(id,_)|id==*tower)&&!self.owns(slot,other.id)){return false;}
                g.sell(t);!g.towers.iter().any(|t|t.id==*tower)
            },
            Action::Target{tower,mode}=>{
                let Some(t)=self.tower(g,slot,*tower)else{return false;};
                let Some(&mode)=TARGETS.get(*mode as usize)else{return false;};
                if !g.target_modes(t).contains(&mode){return false;}
                g.towers[t].target=mode;true
            },
            Action::Point{tower,at}=>{
                let Some(t)=self.tower(g,slot,*tower)else{return false;};
                if !g.uses_point(t){return false;}
                g.towers[t].point=Some(*at);true
            },
            Action::StartRound=>{g.start_round();true},
            Action::Pointer{at}=>{self.pointers[slot as usize]=Some(*at);true},
            Action::Collect{at}=>{g.collect_cash(*at);true},
            Action::Speed{value}=>{self.speed=*value;true},
            Action::AutoStart{value}=>{self.auto_start=*value;true},
            Action::Freeplay=>{g.start_freeplay();true},
            Action::Special{tower}=>{
                let Some(t)=self.tower(g,slot,*tower)else{return false;};g.toggle_stance(t);g.collect_bank(t);true
            },
            Action::Ability{tower,index,at,target}=>{
                let Some(t)=self.tower(g,slot,*tower)else{return false;};
                let other=if let Some(id)=target{let Some(index)=g.towers.iter().position(|t|t.id==*id)else{return false;};Some(index)}else{None};
                g.coop_ability(t,*index as usize,*at,other)
            },
        }
    }
    pub fn active(&self)->bool{self.remaining>0}
    pub fn begin(&mut self,g:&mut Game,frame:&Frame)->Result<Vec<bool>,String>{
        if self.active()||frame.number!=self.frame+1||frame.actions.len()>128{return Err("Simulation frame is out of order".into());}
        // Validate the entire batch before mutating anything.
        let mut sequences=self.sequences;
        for input in &frame.actions{
            if !self.slots.contains(&input.slot)||!input.action.valid(){return Err("Invalid co-op action".into());}
            let seq=&mut sequences[input.slot as usize];
            if input.sequence!=*seq+1{return Err("Co-op input sequence differs".into());}*seq=input.sequence;
        }
        self.sequences=sequences;
        self.prepare(g)?;
        let mut results=Vec::new();
        for input in &frame.actions{
            let accepted=self.action(g,input.slot,&input.action);
            if accepted&&matches!(input.action,Action::Place{..}){
                if let Some(t)=g.towers.iter().rev().find(|t|t.sub.is_none()&&self.owns(input.slot,t.id)){self.placed.push((input.slot,t.id));}
            }
            results.push(accepted);
        }
        self.inherit(g);self.remaining=STEPS_PER_FRAME;Ok(results)
    }
    pub fn step(&mut self,g:&mut Game)->bool{
        if !self.active(){return false;}
        g.coop_restore_cash();
        g.step();if self.auto_start&&g.round>0{g.start_round();}
        self.remaining-=1;
        if self.remaining==0{self.frame+=1;self.inherit(g);return true;}false
    }
    pub fn apply(&mut self,g:&mut Game,frame:&Frame)->Result<Vec<bool>,String>{
        let results=self.begin(g,frame)?;
        while self.active(){self.step(g);}Ok(results)
    }
    /// Diagnostic checksum, including private simulation clocks/RNG. This is not
    /// yet a full-state serializer and must not be treated as proof of determinism.
    pub fn digest(&self,g:&Game)->[u8;32]{
        let mut hash=blake3::Hasher::new();hash.update(&g.coop_digest());
        hash.update(&self.frame.to_le_bytes());
        hash.update(&[self.speed,self.auto_start as u8]);
        for request in self.requests{hash.update(&request.unwrap_or(u64::MAX).to_le_bytes());}
        hash.update(&serde_json::to_vec(&(&self.owners,&self.pointers,&self.sequences)).expect("finite validated replica state"));
        *hash.finalize().as_bytes()
    }
}

pub struct Runtime{pub replica:Replica,acc:f32,host_acc:f32}
impl Runtime{
    pub fn new(slots:Vec<u8>,g:&mut Game)->Result<Self,String>{let mut replica=Replica::new(slots)?;replica.prepare(g)?;Ok(Self{replica,acc:0.0,host_acc:0.0})}
    pub fn update(&mut self,g:&mut Game,front:&mut crate::frontend::FrontEnd,dt:f32)->Result<(),String>{
        front.coop_poll();
        if front.coop_failed(){return Err(front.coop_status().to_owned());}
        if !front.coop_running(){self.acc=0.0;self.host_acc=0.0;g.coop_project_cash(front.coop_slot());return Ok(());}
        g.coop_restore_cash();
        if front.coop_host(){
            self.host_acc+=dt.min(0.25)*self.replica.speed as f32;
            while self.host_acc>=STEPS_PER_FRAME as f32/crate::game::FPS{
                front.coop_send(rrbtd6_coop::Command::Advance);
                self.host_acc-=STEPS_PER_FRAME as f32/crate::game::FPS;
            }
        }
        self.acc+=dt.min(0.25)*self.replica.speed as f32;
        while self.acc>=1.0/crate::game::FPS{
            if !self.replica.active(){
                let Some(frame)=front.coop_frame()else{self.acc=0.0;break;};
                self.replica.begin(g,&frame)?;
            }
            if self.replica.step(g){
                let digest=self.replica.digest(g);
                front.coop_send(rrbtd6_coop::Command::Ack{frame:self.replica.frame,digest});
                if self.replica.frame==180&&std::env::var_os("RRBTD6_COOP_PLAYTEST").is_some(){
                    if let Some(path)=std::env::var_os("RRBTD6_COOP_RESULT_OUT"){
                        let result=serde_json::json!({"frame":self.replica.frame,"digest":digest,"owners":self.replica.owners,"cash":g.cash,"wallets":g.coop_wallets.as_ref().map(|w|w.balances),"debts":g.coop_wallets.as_ref().map(|w|w.debts),"popped":g.popped,"round":g.round,"towers":g.towers.iter().map(|t|(&g.tower_defs[t.def].0,t.id,t.tiers)).collect::<Vec<_>>()});
                        std::fs::write(path,result.to_string()).map_err(|e|e.to_string())?;
                    }
                }
            }
            self.acc-=1.0/crate::game::FPS;
        }
        g.coop_project_cash(front.coop_slot());Ok(())
    }
}

#[cfg(test)]mod tests{
    use super::*;
    use crate::game::tests_support;
    use rrbtd6_coop::Input;
    fn input(slot:u8,sequence:u64,action:Action)->Input{Input{slot,sequence,action}}
    fn spots()->[[f32;2];2]{let mut g=tests_support::game();let dart=g.tower_defs.iter().position(|(n,_)|n=="DartMonkey").unwrap();let a=tests_support::place_near(&mut g,dart,180.0);let p=g.towers[a].pos;let b=tests_support::place_near(&mut g,dart,240.0);[p,g.towers[b].pos]}
    #[test]fn ownership_survives_swap_remove_and_unowned_actions_are_rejected(){
        let mut g=tests_support::game();let mut r=Replica::new(vec![0,1]).unwrap();let p=spots();
        assert_eq!(r.apply(&mut g,&Frame{number:1,actions:(0..2).map(|s|input(s,1,Action::Place{kind:"DartMonkey".into(),at:p[s as usize]})).collect()}).unwrap(),vec![true,true]);
        let host=g.towers[0].id;let guest=g.towers[1].id;
        assert_eq!(r.apply(&mut g,&Frame{number:2,actions:vec![input(1,2,Action::Sell{tower:host}),input(1,3,Action::Upgrade{tower:host,path:0}),input(0,2,Action::Sell{tower:host})]}).unwrap(),vec![false,false,true]);
        assert_eq!(g.towers[0].id,guest);assert!(r.owns(1,guest));
        assert_eq!(r.apply(&mut g,&Frame{number:3,actions:vec![input(1,4,Action::Upgrade{tower:guest,path:0})]}).unwrap(),vec![true]);
    }
    #[test]fn invalid_batch_does_not_partially_apply(){
        let mut g=tests_support::game();let mut r=Replica::new(vec![0,1]).unwrap();let before=r.digest(&g);
        assert!(r.begin(&mut g,&Frame{number:1,actions:vec![input(0,1,Action::StartRound),input(1,2,Action::StartRound)]}).is_err());
        assert_eq!(r.digest(&g),before);assert!(!g.round_active);
    }
    #[test]fn donations_and_requests_are_ordered_and_do_not_repay_debt(){
        let mut g=tests_support::game();g.cash=650.0;let mut r=Replica::new(vec![0,1]).unwrap();r.prepare(&mut g).unwrap();
        g.coop_wallets.as_mut().unwrap().borrow(1,100.0,0.5);
        let actions=vec![input(0,1,Action::Donate{to:1,amount:100.0}),input(0,2,Action::Donate{to:1,amount:300.0}),input(1,1,Action::Donate{to:1,amount:1.0}),input(1,2,Action::Donate{to:3,amount:1.0}),input(1,3,Action::RequestCash),input(1,4,Action::RequestCash)];
        assert_eq!(r.apply(&mut g,&Frame{number:1,actions}).unwrap(),vec![true,false,false,false,true,false]);
        let w=g.coop_wallets.as_ref().unwrap();assert_eq!(w.balances,[225.0,525.0,0.0,0.0]);assert_eq!(w.debts,[0.0,100.0,0.0,0.0]);
        assert_eq!(r.requests,[None,Some(0),None,None]);
        let before=r.digest(&g);
        assert!(r.begin(&mut g,&Frame{number:2,actions:vec![input(0,3,Action::Donate{to:1,amount:f64::NAN})]}).is_err());assert_eq!(r.digest(&g),before);
    }
    #[test]fn purchases_and_refunds_cannot_spend_another_players_wallet(){
        let mut g=tests_support::game();g.cash=650.0;let mut r=Replica::new(vec![0,1]).unwrap();let p=spots();
        assert_eq!(r.apply(&mut g,&Frame{number:1,actions:vec![input(0,1,Action::Place{kind:"DartMonkey".into(),at:p[0]})]}).unwrap(),vec![true]);
        let id=g.towers[0].id;let own=g.coop_wallets.as_ref().unwrap().balances[0];assert!(own<325.0);
        assert_eq!(g.coop_wallets.as_ref().unwrap().balances[1],325.0);
        assert_eq!(r.apply(&mut g,&Frame{number:2,actions:vec![input(0,2,Action::Place{kind:"DartMonkey".into(),at:p[1]}),input(1,1,Action::Sell{tower:id})]}).unwrap(),vec![false,false]);
        let refund=(g.towers[0].invested*g.sell_fraction(0)).floor();
        assert_eq!(r.apply(&mut g,&Frame{number:3,actions:vec![input(0,3,Action::Sell{tower:id})]}).unwrap(),vec![true]);
        assert_eq!(g.coop_wallets.as_ref().unwrap().balances,[own+refund,325.0,0.0,0.0]);
    }
    #[test]fn render_pacing_and_independently_drained_effects_do_not_change_simulation(){
        let mut a=tests_support::game();let mut b=tests_support::game();b.freeplay_seed=a.freeplay_seed;let mut ra=Replica::new(vec![0,1]).unwrap();let mut rb=Replica::new(vec![0,1]).unwrap();let p=spots();
        for number in 1..=300{
            let actions=match number{1=>vec![input(0,1,Action::Place{kind:"DartMonkey".into(),at:p[0]}),input(1,1,Action::Place{kind:"DartMonkey".into(),at:p[1]})],2=>vec![input(0,2,Action::StartRound)],_=>vec![]};
            let frame=Frame{number,actions};ra.apply(&mut a,&frame).unwrap();rb.begin(&mut b,&frame).unwrap();
            while rb.active(){rb.step(&mut b);b.sfx.clear();b.pops.clear();b.bursts.clear();}
            assert_eq!(ra.digest(&a),rb.digest(&b),"frame {number}");
        }
        assert!(a.popped>0);
    }
    #[test]
    #[ignore="local performance measurement"]
    fn coop_digest_benchmark(){
        let mut g=tests_support::game();g.debug_spawn("Red",2000,0.0);
        let replica=Replica::new(vec![0,1]).unwrap();let start=std::time::Instant::now();
        for _ in 0..100{std::hint::black_box(replica.digest(&g));}
        eprintln!("Co-op checksum: {} bloons, {:.3} ms average",g.bloons.len(),start.elapsed().as_secs_f64()*10.0);
    }
    #[test]fn pointers_are_independent_and_ability_sources_are_owned(){
        let mut g=tests_support::game();let def=g.tower_defs.iter().position(|(id,_)|id=="DartMonkey").unwrap();
        let a=tests_support::place_near(&mut g,def,180.0);let id_a=g.towers[a].id;
        let b=tests_support::place_near(&mut g,def,240.0);let id_b=g.towers[b].id;
        for _ in 0..4{assert!(g.upgrade(a,1));}
        for cd in g.towers[a].ability_cd.values_mut(){*cd=0.0;}
        let mut r=Replica::new(vec![0,1]).unwrap();r.owners.insert(id_a,0);r.owners.insert(id_b,1);
        let name=g.tier(a).unwrap().abilities[0].name.clone();assert!(r.ability(&g,1,&name).is_none());
        let action=r.ability(&g,0,&name).unwrap();g.start_round();
        assert_eq!(r.apply(&mut g,&Frame{number:1,actions:vec![input(0,1,Action::Pointer{at:[10.0,20.0]}),input(1,1,Action::Pointer{at:[-10.0,-20.0]}),input(1,2,action.clone())]}).unwrap(),vec![true,true,false]);
        assert_eq!(g.coop_pointer(id_a),Some([10.0,20.0]));assert_eq!(g.coop_pointer(id_b),Some([-10.0,-20.0]));
        assert_eq!(r.apply(&mut g,&Frame{number:2,actions:vec![input(0,2,action)]}).unwrap(),vec![true]);
        assert!(g.towers[a].ability_cd[&name]>0.0);
    }
    #[test]fn targeted_overclock_matches_single_player_and_accepts_an_ally_target(){
        fn setup()->(Game,usize,usize){
            let mut g=tests_support::game();let source=g.tower_defs.iter().position(|(n,_)|n=="EngineerMonkey").unwrap();
            let target=g.tower_defs.iter().position(|(n,_)|n=="DartMonkey").unwrap();
            let a=tests_support::place_near(&mut g,source,240.0);let b=tests_support::place_near(&mut g,target,350.0);
            for _ in 0..4{assert!(g.upgrade(a,1));}for cd in g.towers[a].ability_cd.values_mut(){*cd=0.0;}g.start_round();(g,a,b)
        }
        let (mut native,a,b)=setup();let (mut peer,_,_)=setup();peer.freeplay_seed=native.freeplay_seed;
        let name=native.tier(a).unwrap().abilities.iter().find(|a|a.overclock.is_some()).unwrap().name.clone();
        let mut replica=Replica::new(vec![0,1]).unwrap();replica.owners.insert(peer.towers[a].id,0);replica.owners.insert(peer.towers[b].id,1);
        replica.prepare(&mut native).unwrap();
        let mut action=replica.ability(&peer,0,&name).unwrap();if let Action::Ability{target,..}=&mut action{*target=Some(peer.towers[b].id);}
        assert!(native.activate_ability_on(&name,b));
        assert_eq!(replica.begin(&mut peer,&Frame{number:1,actions:vec![input(0,1,action)]}).unwrap(),vec![true]);
        assert_eq!(native.coop_digest(),peer.coop_digest());
        while replica.active(){native.step();replica.step(&mut peer);}
        assert_eq!(native.coop_digest(),peer.coop_digest());assert!(peer.towers[b].overclock.is_some());
    }
    #[test]fn targeted_darkshift_matches_single_player_and_bad_targets_do_not_spend_cooldown(){
        fn setup()->(Game,usize){let mut g=tests_support::game();let d=g.tower_defs.iter().position(|(n,_)|n=="SuperMonkey").unwrap();let t=tests_support::place_near(&mut g,d,260.0);for _ in 0..3{assert!(g.upgrade(t,2));}for cd in g.towers[t].ability_cd.values_mut(){*cd=0.0;}g.start_round();(g,t)}
        let (mut native,t)=setup();let (mut peer,_)=setup();peer.freeplay_seed=native.freeplay_seed;
        let name=native.tier(t).unwrap().abilities.iter().find(|a|a.darkshift.is_some()).unwrap().name.clone();
        let mut replica=Replica::new(vec![0,1]).unwrap();replica.owners.insert(peer.towers[t].id,0);
        replica.prepare(&mut native).unwrap();
        let from=native.towers[t].pos;let def=native.towers[t].def;
        let at=(0..32).map(|k|{let a=k as f32/32.0*std::f32::consts::TAU;[from[0]+36.0*a.cos(),from[1]+36.0*a.sin()]}).find(|&p|native.can_place(def,p)).unwrap();
        let mut action=replica.ability(&peer,0,&name).unwrap();if let Action::Ability{target,..}=&mut action{*target=Some(u64::MAX);}
        assert_eq!(replica.begin(&mut peer,&Frame{number:1,actions:vec![input(0,1,action)]}).unwrap(),vec![false]);assert_eq!(peer.towers[t].ability_cd[&name],0.0);
        while replica.active(){native.step();replica.step(&mut peer);}
        let mut action=replica.ability(&peer,0,&name).unwrap();if let Action::Ability{at:point,..}=&mut action{*point=Some(at);}
        assert!(native.activate_ability_at(&name,at));
        assert_eq!(replica.begin(&mut peer,&Frame{number:2,actions:vec![input(0,2,action)]}).unwrap(),vec![true]);
        assert_eq!(native.coop_digest(),peer.coop_digest());assert_eq!(peer.towers[t].pos,at);
    }
    #[test]fn targeted_mega_mine_matches_single_player(){
        fn setup()->Game{
            let root=std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../local_extracted");
            let mut data:crate::data::GameData=serde_json::from_slice(&std::fs::read(root.join("game/game.json")).unwrap()).unwrap();
            data.map=serde_json::from_slice(&std::fs::read(root.join("maps/logs/map.json")).unwrap()).unwrap();
            let pond=data.map.areas.iter().find(|a|a.kind==1).unwrap().points.clone();
            let at=[pond.iter().map(|p|p[0]).sum::<f32>()/pond.len() as f32,pond.iter().map(|p|p[1]).sum::<f32>()/pond.len() as f32];
            let mut g=Game::new(data);g.cash=1e9;let d=g.tower_defs.iter().position(|(n,_)|n=="AdmiralBrickell").unwrap();
            assert!(g.place(d,at));g.towers[0].tiers=[10,0,0];g.towers[0].ability_cd.clear();g.start_round();g
        }
        let mut native=setup();let mut peer=setup();peer.freeplay_seed=native.freeplay_seed;
        assert_eq!(native.coop_digest(),peer.coop_digest(),"initial mine state");
        let mut replica=Replica::new(vec![0,1]).unwrap();replica.owners.insert(peer.towers[0].id,0);
        replica.prepare(&mut native).unwrap();
        let at=native.track.pos(native.track.length()*0.3);
        let mut action=replica.ability(&peer,0,"Mega Mine").unwrap();if let Action::Ability{at:point,..}=&mut action{*point=Some(at);}
        assert!(native.activate_ability_at("Mega Mine",at));
        assert_eq!(replica.begin(&mut peer,&Frame{number:1,actions:vec![input(0,1,action)]}).unwrap(),vec![true]);
        assert_eq!(native.towers[0].ability_cd,peer.towers[0].ability_cd,"mine cooldowns");
        let snapshot=|g:&Game|g.projectiles.iter().map(|p|(p.uid,p.owner,p.pos,p.vel,p.frames_left,p.pierce,p.spec.damage,p.spec.radius)).collect::<Vec<_>>();
        assert_eq!(snapshot(&native),snapshot(&peer),"mine projectiles");
        assert_eq!(native.coop_digest(),peer.coop_digest());assert!(!peer.projectiles.is_empty());
    }
}
