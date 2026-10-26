//! Co-op accounting and the compatibility cash/debt fields used by existing rules.
use super::*;
use rrbtd6_coop::economy::Wallets;
impl Game{
    pub(crate) fn coop_init_cash(&mut self,players:Vec<u8>)->Result<(),String>{
        if self.coop_wallets.is_none(){self.coop_wallets=Some(Wallets::new(self.cash as f32,players)?);self.coop_restore_cash();}Ok(())
    }
    fn coop_save_actor(&mut self){
        if let Some(w)=&mut self.coop_wallets{if let Some(s)=w.actor{let i=s as usize;w.balances[i]=self.cash;w.debts[i]=self.debt;w.repayment[i]=self.debt_rate;}}
    }
    pub(crate) fn coop_restore_cash(&mut self){
        if let Some(w)=&self.coop_wallets{
            if let Some(s)=w.actor{let i=s as usize;self.cash=w.balances[i];self.debt=w.debts[i];self.debt_rate=w.repayment[i];}
            else{self.cash=w.total();self.debt=w.debt();self.debt_rate=0.0;}
        }
    }
    pub(crate) fn coop_project_cash(&mut self,slot:u8){if let Some(w)=&self.coop_wallets{self.cash=w.balances[slot as usize];}}
    pub(crate) fn coop_actor(&mut self,slot:u8){
        if let Some(w)=&mut self.coop_wallets{w.actor=Some(slot);}self.coop_restore_cash();
    }
    pub(crate) fn coop_end_actor(&mut self){self.coop_save_actor();if let Some(w)=&mut self.coop_wallets{w.actor=None;}self.coop_restore_cash();}
    fn cash_owner(&self,id:u64)->Option<u8>{
        let w=self.coop_wallets.as_ref()?;
        if let Some(s)=w.owners.get(&id){return Some(*s);}
        let mut parent=id;
        for _ in 0..8{
            parent=self.towers.iter().find(|t|t.id==parent)?.sub.as_ref()?.parent;
            if let Some(s)=w.owners.get(&parent){return Some(*s);}
        }None
    }
    pub(crate) fn cash_for_tower(&self,id:u64)->f64{
        self.cash_owner(id).and_then(|s|self.coop_wallets.as_ref().map(|w|w.balances[s as usize])).unwrap_or(self.cash)
    }
    pub(crate) fn coop_credit(&mut self,owner:Option<u64>,amount:f64){
        self.coop_save_actor();let slot=owner.and_then(|id|self.cash_owner(id));
        if let Some(w)=&mut self.coop_wallets{let value=amount*self.cash_mult;if let Some(s)=slot{w.credit(s,value);}else{w.shared(value);}}
        self.coop_restore_cash();
    }
    pub(super) fn earn_for(&mut self,owner:u64,amount:f64){
        if self.coop_wallets.is_some(){self.coop_credit(Some(owner),amount);}else{self.earn(amount);}
    }
    pub(super) fn coop_loan(&mut self,id:u64,amount:f64,repayment:f64){
        self.coop_save_actor();let slot=self.cash_owner(id).or_else(||self.coop_wallets.as_ref().and_then(|w|w.actor));
        if let (Some(w),Some(s))=(&mut self.coop_wallets,slot){w.borrow(s,amount,repayment);}self.coop_restore_cash();
    }
    pub(super) fn collect_projectile_cash(&mut self,pr:&Projectile){
        if let Some(t)=self.towers.iter_mut().find(|t|t.id==pr.owner){t.cash_earned+=pr.value;}
        if self.coop_wallets.is_none(){self.earn(pr.value);return;}
        if pr.spec.cash.as_ref().is_some_and(|c|c.bonus_shared){
            self.earn_for(pr.owner,pr.cash_base);self.earn(pr.value-pr.cash_base);
        }else{self.earn_for(pr.owner,pr.value);}
    }
}

#[cfg(test)]mod tests{
    use super::*;
    #[test]fn income_and_loans_stay_with_the_producer_after_sale(){
        let mut g=tests_support::game();g.cash=650.0;g.coop_init_cash(vec![0,1]).unwrap();
        g.coop_wallets.as_mut().unwrap().owners.insert(42,1);
        g.earn(1.0);g.earn_for(42,20.0);
        assert_eq!(g.coop_wallets.as_ref().unwrap().balances,[325.5,345.5,0.0,0.0]);
        g.coop_actor(0);g.coop_loan(42,100.0,0.5);g.earn_for(42,20.0);g.coop_end_actor();
        let w=g.coop_wallets.as_ref().unwrap();
        assert_eq!(w.balances,[325.5,455.5,0.0,0.0]);assert_eq!(w.debts,[0.0,90.0,0.0,0.0]);
        assert_eq!(g.cash_for_tower(42),455.5);
    }
    #[test]fn local_hud_projection_never_changes_authoritative_money_or_checksum(){
        let mut g=tests_support::game();g.cash=650.0;g.coop_init_cash(vec![0,1]).unwrap();
        g.coop_wallets.as_mut().unwrap().owners.insert(42,1);g.earn_for(42,20.0);
        let hash=g.coop_digest();g.coop_project_cash(0);assert_eq!(g.cash,325.0);assert_eq!(g.coop_digest(),hash);
        g.coop_project_cash(1);assert_eq!(g.cash,345.0);assert_eq!(g.coop_digest(),hash);
        g.coop_actor(0);g.cash-=100.0;g.coop_end_actor();
        assert_eq!(g.coop_wallets.as_ref().unwrap().balances,[225.0,345.0,0.0,0.0]);assert_eq!(g.cash,570.0);
    }
    #[test]fn an_ally_collecting_bananas_pays_the_farm_owner_and_banks_do_too(){
        let mut g=tests_support::game();let farm=g.tower_defs.iter().position(|(n,_)|n=="BananaFarm").unwrap();
        let at=(-28..=28).flat_map(|x|(-20..=20).map(move |z|[x as f32*5.0,z as f32*5.0])).find(|&p|g.can_place(farm,p)).expect("farm placement");
        assert!(g.place(farm,at));let t=g.towers.len()-1;let id=g.towers[t].id;
        g.cash=650.0;g.coop_init_cash(vec![0,1]).unwrap();g.coop_wallets.as_mut().unwrap().owners.insert(id,1);
        g.start_round();
        for _ in 0..600{g.step();if !g.projectiles.is_empty(){break;}}
        let index=g.projectiles.iter().position(|p|p.owner==id&&p.spec.cash.is_some()).expect("farm emits a cash projectile");
        let mut pr=g.projectiles.swap_remove(index);pr.moving=0.0;let at=pr.pos;let value=pr.value;assert!(value>0.0);
        g.towers[t].bank=123.9;
        let before=g.coop_wallets.as_ref().unwrap().balances;g.coop_actor(1);g.collect_bank(t);g.coop_end_actor();
        let after=g.coop_wallets.as_ref().unwrap().balances;assert_eq!(after[0],before[0]);assert_eq!(after[1],before[1]+123.0);
        // The source can disappear before its last banana is collected.
        g.towers.clear();g.projectiles.clear();g.projectiles.push(pr);
        let before=g.coop_wallets.as_ref().unwrap().balances;g.coop_actor(0);g.collect_cash(at);g.coop_end_actor();
        let after=g.coop_wallets.as_ref().unwrap().balances;assert_eq!(after[0],before[0]);assert_eq!(after[1],before[1]+value);
    }
}
