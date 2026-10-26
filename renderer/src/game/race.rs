//! Race round queues run concurrently; each round keeps its own bloon ownership.
use super::*;

pub struct Race {
    pub event_id:String,
    pub name:String,
    pub elapsed:u64,
    pub sent:usize,
    pub started:bool,
    pub requested:usize,
    pub next_send:u64,
    pub send_delay:u64,
    pub spawn_round:usize,
    pending:Vec<(u64,usize,u32,usize)>,
    active:Vec<usize>,
    emissions:Vec<usize>,
    completed:Vec<bool>,
    limits:HashMap<String,(i64,[u8;3])>,
    max_towers:i64,
}
impl Race{
    pub fn time_text(&self)->String{
        let cs=self.elapsed*100/FPS as u64;
        format!("{:02}:{:02}.{:02}",cs/6000,cs/100%60,cs%100)
    }
}
impl Game {
    pub fn validate_race_rules(event:&serde_json::Value)->Result<(),String>{
        let c=&event["challenge"];
        if !c.is_object(){return Err("Race rules are unavailable".into());}
        let m=&c["bloonModifiers"];
        for key in ["speedMultiplier","moabSpeedMultiplier","bossSpeedMultiplier","regrowRateMultiplier"]{
            if m[key].as_f64().is_some_and(|v|(v-1.0).abs()>0.00001){return Err(format!("Race modifier not implemented yet: {key}"));}
        }
        if m["allCamo"]==true||m["allRegen"]==true{return Err("Race camo/regrow override is not implemented yet".into());}
        for key in ["bloons","moabs","boss"]{if m["healthMultipliers"][key].as_f64().is_some_and(|v|(v-1.0).abs()>0.00001){return Err("Race health override is not implemented yet".into());}}
        for key in ["abilityCooldownReductionMultiplier","removeableCostMultiplier"]{if c[key].as_f64().is_some_and(|v|(v-1.0).abs()>0.00001){return Err(format!("Race modifier not implemented yet: {key}"));}}
        if c["roundSets"].as_array().is_some_and(|r|r.iter().any(|v|v!="default")){return Err("This race uses an unsupported custom round set".into());}
        Ok(())
    }
    pub fn configure_race(&mut self,event:&serde_json::Value)->Result<(),String>{
        Self::validate_race_rules(event)?;
        let c=&event["challenge"];
        if !c.is_object(){return Err("Race rules are unavailable".into());}
        let first=c["startRules"]["round"].as_u64().unwrap_or(1) as usize;
        let end=c["startRules"]["endRound"].as_u64().unwrap_or(80) as usize;
        if first==0||end<first||end>self.rounds.len(){return Err("Unsupported race round set".into());}
        let mut limits=HashMap::new();
        for t in c["towers"].as_array().into_iter().flatten(){
            let Some(id)=t["tower"].as_str()else{continue};
            let max=t["max"].as_i64().unwrap_or(0);
            if max==0{continue;}
            let caps=std::array::from_fn(|p|(5-t[format!("path{}NumBlockedTiers",p+1)].as_i64().unwrap_or(0)).clamp(0,5) as u8);
            limits.insert(id.to_owned(),(max,caps));
        }
        // The current event supplies explicit hero choices. Keep those in the shop too.
        for hero in &self.heroes{if limits.contains_key(&hero.id)&&!self.shop.iter().any(|s|s.id==hero.id){self.shop.insert(0,hero.clone());}}
        self.shop.retain(|s|limits.contains_key(&s.id));
        self.locked=self.tower_defs.iter().filter(|(id,_)|!limits.contains_key(id)).map(|(id,_)|id.clone()).collect();
        self.cash=c["startRules"]["cash"].as_f64().unwrap_or(650.0);
        self.lives=c["startRules"]["lives"].as_f64().unwrap_or(150.0);
        self.start_lives=self.lives;
        self.start_round=first-1;self.round=first-1;self.end_round=end;
        self.no_sell=c["disableSelling"].as_bool().unwrap_or(false);
        self.apopalypse=false;self.sandbox=false;self.freeplay=false;
        self.race=Some(Race{name:event["title"].as_str().or_else(||event["event"]["name"].as_str()).unwrap_or("Race").replace('_'," "),event_id:event["event"]["id"].as_str().unwrap_or("race").to_owned(),elapsed:0,sent:first-1,started:false,requested:0,next_send:0,
            send_delay:(event["sendDelay"].as_f64().unwrap_or(0.2)*FPS as f64).round().max(1.0) as u64,spawn_round:0,
            pending:Vec::new(),active:vec![0;end+1],emissions:vec![0;end+1],completed:vec![false;end+1],limits,max_towers:c["maxTowers"].as_i64().unwrap_or(-1)});
        Ok(())
    }
    pub fn configure_front_event(&mut self,event:&serde_json::Value)->Result<(),String>{
        if event["localDaily"]==true{self.configure_daily(event)}else{self.configure_race(event)}
    }
    pub fn validate_daily_rules(event:&serde_json::Value)->Result<(),String>{
        let mut supported=event.clone();
        let m=&mut supported["challenge"]["bloonModifiers"];
        m["allRegen"]=serde_json::json!(false);
        for key in ["speedMultiplier","moabSpeedMultiplier","bossSpeedMultiplier","regrowRateMultiplier"]{m[key]=serde_json::json!(1.0);}
        for key in ["bloons","moabs","boss"]{m["healthMultipliers"][key]=serde_json::json!(1.0);}
        m["allCamo"]=serde_json::json!(false);
        let cooldown=event["challenge"]["abilityCooldownReductionMultiplier"].as_f64().unwrap_or(1.0);
        if !cooldown.is_finite()||cooldown<=0.0{return Err("Invalid ability cooldown multiplier".into());}
        supported["challenge"]["abilityCooldownReductionMultiplier"]=serde_json::json!(1.0);
        Self::validate_race_rules(&supported)
    }
    pub fn configure_daily(&mut self,event:&serde_json::Value)->Result<(),String>{
        Self::validate_daily_rules(event)?;
        let mut setup=event.clone();
        setup["challenge"]["bloonModifiers"]=serde_json::json!({});
        setup["challenge"]["abilityCooldownReductionMultiplier"]=serde_json::json!(1.0);
        self.configure_race(&setup)?;
        self.challenge_limits=self.race.take();
        let m=&event["challenge"]["bloonModifiers"];
        let cooldown=event["challenge"]["abilityCooldownReductionMultiplier"].as_f64().unwrap_or(1.0) as f32;
        for tt in self.tower_tiers.iter_mut().flatten(){for tier in tt.tiers.values_mut(){for ability in &mut tier.abilities{ability.cooldown*=cooldown;}}}
        let regrow_names:HashMap<String,String>=if m["allRegen"]==true{
            self.names.iter().filter_map(|name|{
                if name.contains("Regrow"){return None;}
                let end=name.find("Fortified").or_else(||name.find("Camo")).unwrap_or(name.len());
                let variant=format!("{}Regrow{}",&name[..end],&name[end..]);
                self.by_name.get(&variant).filter(|&&k|self.defs[k].grow.is_some()).map(|_|(name.clone(),variant))
            }).collect()
        }else{HashMap::new()};
        for round in &mut self.rounds{for group in round{if let Some(variant)=regrow_names.get(&group.bloon){group.bloon=variant.clone();}}}
        let original=self.defs.clone();
        for (i,d) in self.defs.iter_mut().enumerate(){
            let moab=d.tags.iter().any(|t|t=="Moabs");
            let speed=m[if moab{"moabSpeedMultiplier"}else{"speedMultiplier"}].as_f64().unwrap_or(1.0) as f32;
            let hp=m["healthMultipliers"][if moab{"moabs"}else{"bloons"}].as_f64().unwrap_or(1.0) as f32;
            for child in &mut d.children{if let Some(variant)=regrow_names.get(child){*child=variant.clone();}}
            d.speed*=speed;d.max_health=(d.max_health as f32*hp).max(1.0) as i32;
            if let Some(g)=&mut d.grow{g.rate/=m["regrowRateMultiplier"].as_f64().unwrap_or(1.0).max(0.0001) as f32;}
            if m["allCamo"]==true {
                if let Some(&camo)=self.by_name.get(&format!("{}Camo",self.names[i])) {
                    d.is_camo=true;d.sprite=original[camo].sprite.clone();d.display=original[camo].display.clone();
                }
            }
        }
        Ok(())
    }
    pub fn race_can_place(&self,def:usize)->bool{
        let Some(race)=self.race.as_ref().or(self.challenge_limits.as_ref()) else{return true};
        let Some((max,_))=race.limits.get(&self.tower_defs[def].0)else{return false};
        let roots=self.towers.iter().filter(|t|t.sub.is_none());
        if race.max_towers>=0&&roots.clone().count()>=race.max_towers as usize{return false;}
        *max<0||roots.filter(|t|t.def==def).count()<*max as usize
    }
    pub fn race_upgrade_allowed(&self,t:usize,path:usize)->bool{
        if self.hero_def(self.towers[t].def).is_some(){return true;}
        self.race.as_ref().or(self.challenge_limits.as_ref()).is_none_or(|r|r.limits.get(&self.tower_defs[self.towers[t].def].0).is_some_and(|(_,caps)|path<3&&self.towers[t].tiers[path]<caps[path]))
    }
    pub fn request_race_round(&mut self){
        let Some(r)=&mut self.race else{return};
        if self.game_over||self.won||r.sent+r.requested>=self.end_round{return;}
        r.requested+=1;
        if self.steps>=r.next_send{self.send_race_round();}
    }
    fn send_race_round(&mut self){
        let r=self.race.as_mut().unwrap();
        if r.requested==0||r.sent>=self.end_round{return;}
        r.requested-=1;r.sent+=1;r.started=true;r.next_send=self.steps+r.send_delay;
        let round=r.sent;self.round=round-1;self.round_active=true;
        let groups=self.rounds[round-1].clone();
        for g in groups{
            let Some(&kind)=self.by_name.get(&g.bloon)else{continue};
            let start=self.round_scale*g.start*FPS;let span=self.round_scale*g.duration*FPS;
            let gap=if g.count>1{span/(g.count-1) as f32}else{0.0};
            for i in 0..g.count{
                let due=self.steps+(start+if g.count==1{span*0.5}else{i as f32*gap}).ceil().max(0.0) as u64;
                let emission=r.emissions[round] as u32;r.emissions[round]+=1;
                r.pending.push((due,kind,emission,round));
            }
        }
        r.pending.sort_by_key(|q|std::cmp::Reverse((q.0,q.3,q.2)));
        self.progressive_difficulty(round as i32);
        for tower in &mut self.towers{tower.weapons.iter_mut().for_each(|w|w.round_shots=0);}
    }
    pub(super) fn race_step_start(&mut self){
        let Some(r)=&self.race else{return};
        if r.requested>0&&self.steps>=r.next_send{self.send_race_round();}
        while let Some((due,kind,emission,round))=self.race.as_ref().and_then(|r|r.pending.last().copied()){
            if due>self.steps{break;}
            let r=self.race.as_mut().unwrap();r.pending.pop();r.emissions[round]-=1;r.spawn_round=round;
            let path=self.spawn_tracks[emission as usize%self.spawn_tracks.len()] as u8;
            self.spawn_emitted(kind,path);
        }
    }
    pub(super) fn race_step_end(&mut self){
        let Some(r)=&mut self.race else{return};
        if !r.started||self.won||self.game_over{return;}
        r.elapsed+=1;
        let sent=r.sent;
        for round in self.start_round+1..=sent{
            let r=self.race.as_mut().unwrap();
            if !r.completed[round]&&r.emissions[round]==0&&r.active[round]==0{
                r.completed[round]=true;
                let display=self.round;self.round=round-1;self.finish_round();self.round=display;self.round_active=true;
            }
        }
        if sent==self.end_round&&self.race.as_ref().unwrap().completed[self.start_round+1..=sent].iter().all(|v|*v){
            self.won=true;self.round_active=false;self.round=self.end_round;
            self.sound(&["VictoryCampaign".to_owned()],None);
        }
    }
    pub(super) fn race_bloon_spawn(&mut self)->usize{
        let Some(r)=&mut self.race else{return 0};let n=r.spawn_round;
        if n>0{r.active[n]+=1;}n
    }
    pub(super) fn race_bloon_remove(&mut self,round:usize){if let Some(r)=&mut self.race{if round>0{r.active[round]=r.active[round].saturating_sub(1);}}}
}

#[cfg(test)]
mod tests{
    use super::*;
    use serde_json::json;
    fn setup()->Game{
        let mut g=super::super::tests_support::game();
        g.rounds[0]=serde_json::from_value(json!([{"bloon":"Red","count":1,"start":0.0,"duration":0.0}])).unwrap();
        g.rounds[1]=g.rounds[0].clone();
        g.configure_race(&json!({"event":{"id":"synthetic-race"},"sendDelay":0.2,"challenge":{
            "startRules":{"round":1,"endRound":2,"cash":650,"lives":150},"maxTowers":2,
            "towers":[{"tower":"DartMonkey","max":1,"path1NumBlockedTiers":3,"path2NumBlockedTiers":5,"path3NumBlockedTiers":0}]}})).unwrap();g
    }
    #[test]fn sent_rounds_overlap_without_erasing_previous_bloons_and_complete_individually(){
        let mut g=setup();g.request_race_round();g.step();assert_eq!(g.bloons.len(),1);
        g.request_race_round();for _ in 0..15{g.step();}
        assert_eq!(g.race.as_ref().unwrap().sent,2);assert_eq!(g.bloons.len(),2);
        assert_eq!((g.bloons[0].race_round,g.bloons[1].race_round),(1,2));
        g.remove_bloon(0);g.step();assert!(!g.won);assert!(g.race.as_ref().unwrap().completed[1]);
        g.remove_bloon(0);g.step();assert!(g.won);assert!(!g.round_active);
        let time=g.race.as_ref().unwrap().elapsed;g.step();assert_eq!(time,g.race.as_ref().unwrap().elapsed);
    }
    #[test]fn daily_uses_normal_round_flow_and_applies_modifiers_and_caps(){
        let mut g=super::super::tests_support::game();
        let red=g.by_name["Red"];let speed=g.defs[red].speed;
        let event=json!({"localDaily":true,"challenge":{
            "startRules":{"round":8,"endRound":10,"cash":1250,"lives":2},
            "bloonModifiers":{"speedMultiplier":2.0,"moabSpeedMultiplier":5.0,"regrowRateMultiplier":15.0,"allCamo":true,"healthMultipliers":{"bloons":0.23,"moabs":0.05}},
            "maxTowers":1,"towers":[{"tower":"DartMonkey","max":1,"path1NumBlockedTiers":3,"path2NumBlockedTiers":5,"path3NumBlockedTiers":0}]}});
        g.configure_front_event(&event).unwrap();
        assert!(g.race.is_none());assert!(g.challenge_limits.is_some());
        assert_eq!((g.round,g.end_round,g.cash,g.lives),(7,10,1250.0,2.0));
        assert_eq!(g.defs[red].speed,speed*2.0);assert!(g.defs[red].is_camo);
        g.start_round();assert!(g.round_active);assert!(g.race.is_none());
        let mut unsupported=event.clone();unsupported["challenge"]["removeableCostMultiplier"]=json!(12.0);
        assert!(Game::validate_daily_rules(&unsupported).is_err());
    }
    #[test]fn daily_regrow_override_and_cooldown_scale_use_existing_definitions(){
        let mut g=super::super::tests_support::game();
        let before=g.tower_tiers.iter().flatten().flat_map(|t|t.tiers.values()).flat_map(|t|&t.abilities).find(|a|a.cooldown>0.0).map(|a|(a.name.clone(),a.cooldown)).unwrap();
        g.rounds[0]=serde_json::from_value(json!([{"bloon":"Red","count":1,"start":0.0,"duration":0.0}])).unwrap();
        g.configure_daily(&json!({"challenge":{"startRules":{"round":1,"endRound":1},"bloonModifiers":{"allRegen":true},"abilityCooldownReductionMultiplier":5.0,"towers":[]}})).unwrap();
        assert_eq!(g.rounds[0][0].bloon,"RedRegrow");
        assert!(g.defs[g.by_name["RedRegrow"]].grow.is_some());
        let after=g.tower_tiers.iter().flatten().flat_map(|t|t.tiers.values()).flat_map(|t|&t.abilities).find(|a|a.name==before.0).unwrap();
        assert_eq!(after.cooldown,before.1*5.0);
    }
    #[test]fn event_purchase_and_path_caps_are_enforced(){
        let mut g=setup();let dart=g.tower_defs.iter().position(|(id,_)|id=="DartMonkey").unwrap();
        assert!(g.race_can_place(dart));
        let pos=(-140..140).step_by(5).flat_map(|x|(-110..110).step_by(5).map(move|y|[x as f32,y as f32])).find(|p|g.can_place(dart,*p)).unwrap();
        assert!(g.place(dart,pos));let t=g.towers.len()-1;
        assert!(!g.race_can_place(dart));assert!(g.next_upgrade(t,1).is_none());
        g.towers[t].tiers=[2,0,0];assert!(g.next_upgrade(t,0).is_none());assert!(g.next_upgrade(t,2).is_some());
    }
}
