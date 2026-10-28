//! Canonical diagnostic state for replay checks. Excludes visual/audio queues.
//! It covers primary combat state and clocks, not every nested mutator yet.
use super::*;
trait Feed{fn feed(&self,h:&mut blake3::Hasher);}
macro_rules! ints{($($t:ty),*)=>{$(impl Feed for $t{fn feed(&self,h:&mut blake3::Hasher){h.update(&self.to_le_bytes());}})*};}
ints!(u64,u32,u8,i32);
impl Feed for usize{fn feed(&self,h:&mut blake3::Hasher){(*self as u64).feed(h);}}
impl Feed for f32{fn feed(&self,h:&mut blake3::Hasher){self.to_bits().feed(h);}}
impl Feed for f64{fn feed(&self,h:&mut blake3::Hasher){self.to_bits().feed(h);}}
impl Feed for bool{fn feed(&self,h:&mut blake3::Hasher){(*self as u8).feed(h);}}
impl Feed for str{fn feed(&self,h:&mut blake3::Hasher){self.len().feed(h);h.update(self.as_bytes());}}
impl Feed for String{fn feed(&self,h:&mut blake3::Hasher){self.as_str().feed(h);}}
impl<T:Feed+?Sized> Feed for &T{fn feed(&self,h:&mut blake3::Hasher){T::feed(*self,h);}}
impl<T:Feed> Feed for Option<T>{fn feed(&self,h:&mut blake3::Hasher){self.is_some().feed(h);if let Some(v)=self{v.feed(h);}}}
impl<T:Feed> Feed for Vec<T>{fn feed(&self,h:&mut blake3::Hasher){self.len().feed(h);for v in self{v.feed(h);}}}
impl<T:Feed,const N:usize> Feed for [T;N]{fn feed(&self,h:&mut blake3::Hasher){for v in self{v.feed(h);}}}
impl<A:Feed,B:Feed> Feed for (A,B){fn feed(&self,h:&mut blake3::Hasher){self.0.feed(h);self.1.feed(h);}}
impl<A:Feed,B:Feed,C:Feed> Feed for (A,B,C){fn feed(&self,h:&mut blake3::Hasher){self.0.feed(h);self.1.feed(h);self.2.feed(h);}}
impl<A:Feed,B:Feed,C:Feed,D:Feed> Feed for (A,B,C,D){fn feed(&self,h:&mut blake3::Hasher){self.0.feed(h);self.1.feed(h);self.2.feed(h);self.3.feed(h);}}
impl<K:Feed,V:Feed> Feed for std::collections::BTreeMap<K,V>{fn feed(&self,h:&mut blake3::Hasher){self.len().feed(h);for (key,value) in self{key.feed(h);value.feed(h);}}}
macro_rules! feed{($h:expr,$($value:expr),+)=>{$($value.feed($h);)+};}
impl Game{
    pub(crate) fn coop_ability(&mut self,t:usize,index:usize,point:Option<[f32;2]>,target:Option<usize>)->bool{
        if !self.round_active{return false;}
        let Some(ability)=self.tier(t).and_then(|tier|tier.abilities.get(index)).cloned()else{return false;};
        if self.towers[t].ability_cd.get(&ability.name).copied().unwrap_or(0.0)>0.0{return false;}
        if let Some(oc)=&ability.overclock{
            let Some(other)=target.filter(|&i|i!=t&&i<self.towers.len()&&self.towers[i].sub.is_none())else{return false;};
            if point.is_some(){return false;}
            let top=*self.towers[other].tiers.iter().max().unwrap_or(&0) as usize;
            let mult=oc.tier_mult.get(top).copied().unwrap_or(1.0);let tw=&mut self.towers[other];
            tw.overclock=Some((oc.rate,oc.village_range,oc.range,oc.frames*mult));
            if let Some(st)=&oc.permanent{tw.ultra=((tw.ultra.0+1).min(st.max_stacks),st.rate,st.village_range);}
        }else if let Some(pa)=&ability.place_at{
            let Some(p)=point else{return false;};if target.is_some(){return false;}
            let (tiers,owner)=(self.towers[t].tiers,self.towers[t].id);
            let single=Emission{kind:"Single".into(),count:1,angle:0.0,spread:0.0,offset_start:0.0,target_spread:0.0,markers:Vec::new(),secondary:None,random_angle:0.0,offset:0.0,dir_off:None};
            let first=self.projectiles.len();self.emit(p,0.0,&single,&pa.projectile,None,tiers,owner);
            for pr in &mut self.projectiles[first..]{pr.pos=p;pr.vel=[0.0;2];pr.wait=pa.delay*FPS;}
        }else if let Some(ds)=&ability.darkshift{
            let Some(p)=point else{return false;};if target.is_some(){return false;}
            let from=self.towers[t].pos;
            if ds.restrict&&(p[0]-from[0]).hypot(p[1]-from[1])>ds.radius{return false;}
            self.towers[t].pos=[f32::MAX;2];let ok=self.can_place(self.towers[t].def,p);self.towers[t].pos=from;
            if !ok{return false;}self.towers[t].pos=p;
        }else{
            if point.is_some()||target.is_some(){return false;}
            self.towers[t].ability_cd.insert(ability.name.clone(),ability.cooldown*FPS);self.use_ability(t,&ability);return true;
        }
        self.towers[t].ability_cd.insert(ability.name.clone(),ability.cooldown*FPS);self.sound(&ability.sounds,None);true
    }
    pub(crate) fn coop_pointer(&self,id:u64)->Option<[f32;2]>{
        if self.coop_pointers.is_empty(){return self.pointer;}
        if let Some(point)=self.coop_pointers.get(&id){return *point;}
        self.towers.iter().find(|t|t.id==id).and_then(|t|t.sub.as_ref()).and_then(|s|self.coop_pointers.get(&s.parent).copied().flatten())
    }
    pub(crate) fn coop_digest(&self)->[u8;32]{
        let mut hasher=blake3::Hasher::new();let h=&mut hasher;
        let (cash,debt,rate)=self.coop_wallets.as_ref().map_or((self.cash,self.debt,self.debt_rate),|w|(w.total(),w.debt(),0.0));
        self.coop_wallets.is_some().feed(h);if let Some(w)=&self.coop_wallets{feed!(h,w.players,w.balances,w.debts,w.repayment,w.actor,w.owners);}
        feed!(h,self.mode_name,cash,debt,rate,self.lives,self.round,self.round_active,self.round_frame,self.steps,
            self.rng,self.freeplay_seed,self.next_id,self.next_uid,self.pending,self.sandbox_queue,self.popped,self.game_over,self.won,self.freeplay,
            self.routs,self.danger,self.leaked,self.syphon,self.typed_made,self.bounds);
        self.tracks.len().feed(h);for track in &self.tracks{feed!(h,track.pts,track.cum);}
        self.towers.len().feed(h);
        for t in &self.towers{
            feed!(h,t.id,t.def,t.pos,t.tiers,t.invested,t.target.label(),t.point,t.weapons.iter().map(|w|(w.last as u64,w.anim as u64,w.missed,w.round_shots)).collect::<Vec<_>>(),t.shots,t.bank,t.pops,t.xp,t.auto_t,t.focus,t.mute,t.ultra,t.overclock,t.range_up,t.rage,t.angle);
            let mut abilities:Vec<_>=t.ability_cd.iter().collect();abilities.sort_unstable_by_key(|(key,_)|*key);abilities.feed(h);
            t.air.is_some().feed(h);if let Some(a)=&t.air{feed!(h,a.pos,a.vel,a.angle,a.phase,a.height,a.dive,a.grab,a.wait,a.tick,a.on_path,a.bank,a.pitch,a.scale,a.tt,a.age);}
        }
        self.bloons.len().feed(h);
        for b in &self.bloons{
            feed!(h,b.id,b.kind,b.dist,b.pos,b.hp,b.path,b.base,b.decamo,b.grow_t,b.grow_ceiling,b.effects.len());
            for e in &b.effects{
                feed!(h,e.id,e.frames_left,e.layers,e.owner);
                match &e.kind{
                    EffectKind::Slow{mult,glue}=>{feed!(h,0u8,mult,glue);},EffectKind::Freeze=>1u8.feed(h),
                    EffectKind::Dot{damage,interval,tick,immune,..}=>{feed!(h,2u8,damage,interval,tick,immune);},
                    EffectKind::Brittle{damage}=>{feed!(h,3u8,damage);},
                    EffectKind::Emit{interval,tick,..}=>{feed!(h,4u8,interval,tick);},EffectKind::PopEmit{..}=>5u8.feed(h),EffectKind::DestroyEmit{..}=>13u8.feed(h),
                    EffectKind::Mark{damage_mult,cash_mult}=>{feed!(h,6u8,damage_mult,cash_mult);},EffectKind::Strip=>7u8.feed(h),
                    EffectKind::Worth{per_tier}=>{feed!(h,8u8,per_tier);},
                    EffectKind::Trance{centre,radius,angle,rate}=>{feed!(h,9u8,centre,radius,angle,rate);},EffectKind::Cooldown=>10u8.feed(h),
                    EffectKind::Unarmor{bits}=>{feed!(h,11u8,bits);},
                    EffectKind::Knock{m}=>{feed!(h,12u8,m.to_bits());},
                    EffectKind::Carry{uids,..}=>{feed!(h,14u8,uids);},
                }
            }
        }
        self.projectiles.len().feed(h);
        for p in &self.projectiles{
            feed!(h,p.uid,p.owner,p.pos,p.vel,p.frames_left,p.pierce,p.target,p.origin,p.wait,p.bounces,p.camo,p.fwd,p.path_d,p.on_track,p.clear_t,p.seek,p.moving,p.rounds_left,p.value,p.refresh_t,p.interval_t,
                p.cash_base,p.spec.damage,p.spec.pierce,p.spec.radius,p.spec.immune,p.spec.speed,p.spec.lifespan,p.spec.pierce_cost);
            if p.hit.big.is_empty(){p.hit.small.feed(h);}else{let mut ids:Vec<_>=p.hit.big.iter().copied().collect();ids.sort_unstable();ids.feed(h);}
        }
        *hasher.finalize().as_bytes()
    }
}
