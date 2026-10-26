//! Per-player accounting and native starting/shared cash distribution arithmetic.
use super::MAX_PLAYERS;
use std::collections::BTreeMap;

/// CoopInGamePlayerPanel.GetCashTransferAmount: one fifth (floored), at least
/// 100 when available; below 100 the remaining balance is offered.
pub fn donation_amount(cash:f64)->f64{
    if !cash.is_finite()||cash<=0.0{0.0}else if cash<100.0{cash}else{(cash/5.0).floor().max(100.0)}
}

#[derive(Clone,Debug)]
pub struct Wallets{
    pub players:Vec<u8>,pub balances:[f64;MAX_PLAYERS],pub debts:[f64;MAX_PLAYERS],
    pub repayment:[f64;MAX_PLAYERS],pub actor:Option<u8>,pub owners:BTreeMap<u64,u8>,
}
impl Wallets{
    pub fn new(total:f32,players:Vec<u8>)->Result<Self,String>{
        let balances=starting_balances(total,&players,false,[false;MAX_PLAYERS])?;
        Ok(Self{players,balances,debts:[0.0;MAX_PLAYERS],repayment:[0.0;MAX_PLAYERS],actor:None,owners:BTreeMap::new()})
    }
    pub fn total(&self)->f64{self.players.iter().map(|&s|self.balances[s as usize]).sum()}
    pub fn debt(&self)->f64{self.players.iter().map(|&s|self.debts[s as usize]).sum()}
    pub fn credit(&mut self,slot:u8,amount:f64){
        let i=slot as usize;let payment=(amount*self.repayment[i]).min(self.debts[i]).max(0.0);
        self.debts[i]-=payment;self.balances[i]+=amount-payment;
    }
    pub fn shared(&mut self,amount:f64){
        let share=amount/self.players.len() as f64;
        for i in 0..self.players.len(){self.credit(self.players[i],share);}
    }
    pub fn borrow(&mut self,slot:u8,amount:f64,repayment:f64){
        let i=slot as usize;self.balances[i]+=amount;self.debts[i]+=amount;self.repayment[i]=repayment;
    }
    pub fn transfer(&mut self,from:u8,to:u8,amount:f64)->bool{
        if from==to||!self.players.contains(&from)||!self.players.contains(&to)||!amount.is_finite()||amount<=0.0||self.balances[from as usize]<amount{return false;}
        self.balances[from as usize]-=amount;self.balances[to as usize]+=amount;true
    }
}

fn players_valid(players:&[u8])->bool{
    !players.is_empty()&&players.len()<=MAX_PLAYERS&&players.iter().enumerate().all(|(i,&p)|(p as usize)<MAX_PLAYERS&&!players[..i].contains(&p))
}
/// Simulation.DistributeCash: truncate GameModel.cash to i32, divide into whole
/// shares, give remainder units in player enumeration order, then apply double cash.
pub fn starting_balances(total:f32,players:&[u8],double_allowed:bool,double_cash:[bool;MAX_PLAYERS])->Result<[f64;MAX_PLAYERS],String>{
    if !players_valid(players)||!total.is_finite()||total<0.0||total as f64>i32::MAX as f64{return Err("Invalid starting cash distribution".into());}
    let total=total.trunc() as i32;let count=players.len() as i32;
    let base=total/count;let remainder=total%count;let mut balances=[0.0;MAX_PLAYERS];
    for (index,&player) in players.iter().enumerate(){
        let share=base+(index<(remainder as usize)) as i32;
        balances[player as usize]=share as f64*if double_allowed&&double_cash[player as usize]{2.0}else{1.0};
    }
    Ok(balances)
}
/// Simulation.AddSharedCash: divide in f64 without flooring each gain. The
/// modifier flag and recipient entitlement control whether a share is doubled.
pub fn shared_credits(amount:f64,players:&[u8],double_allowed:bool,apply_double:bool,double_cash:[bool;MAX_PLAYERS])->Result<[f64;MAX_PLAYERS],String>{
    if !players_valid(players)||!amount.is_finite()||amount<0.0{return Err("Invalid shared credit".into());}
    let share=amount/players.len() as f64;let mut credits=[0.0;MAX_PLAYERS];
    for &player in players{
        credits[player as usize]=share*if double_allowed&&apply_double&&double_cash[player as usize]{2.0}else{1.0};
        if !credits[player as usize].is_finite(){return Err("Shared credit overflow".into());}
    }
    Ok(credits)
}

#[cfg(test)]mod tests{
    use super::*;
    #[test]fn native_donation_offer(){
        for (cash,expected) in [(0.0,0.0),(25.5,25.5),(100.0,100.0),(499.0,100.0),(505.0,101.0),(10009.0,2001.0)]{assert_eq!(donation_amount(cash),expected);}
        assert_eq!(donation_amount(f64::NAN),0.0);
    }
    #[test]fn loans_and_repayment_are_per_player(){
        let mut w=Wallets::new(650.0,vec![0,1]).unwrap();w.borrow(0,100.0,0.5);w.shared(20.0);
        assert_eq!(w.balances,[430.0,335.0,0.0,0.0]);assert_eq!(w.debts,[95.0,0.0,0.0,0.0]);
        w.credit(1,20.0);assert_eq!(w.balances[1],355.0);assert_eq!(w.debts[0],95.0);
        w.credit(0,500.0);assert_eq!(w.debts[0],0.0);assert_eq!(w.balances[0],835.0);
    }
    #[test]fn donations_cannot_create_money_or_repay_recipient_debt(){
        let mut w=Wallets::new(650.0,vec![0,1]).unwrap();w.borrow(1,100.0,0.5);
        assert!(!w.transfer(0,1,-1.0));assert!(!w.transfer(0,1,f64::NAN));assert!(!w.transfer(0,2,5.0));assert!(!w.transfer(0,1,326.0));
        let total=w.total();assert!(w.transfer(0,1,100.0));assert_eq!(w.total(),total);assert_eq!(w.debts[1],100.0);
    }
    #[test]fn starting_remainder_follows_player_order(){
        assert_eq!(starting_balances(650.9,&[0,1,2,3],false,[false;4]).unwrap(),[163.0,163.0,162.0,162.0]);
        assert_eq!(starting_balances(650.0,&[0,3,1],false,[false;4]).unwrap(),[217.0,216.0,0.0,217.0]);
        assert_eq!(starting_balances(650.0,&[0,1],false,[false;4]).unwrap(),[325.0,325.0,0.0,0.0]);
    }
    #[test]fn double_cash_is_per_recipient_and_shared_credits_keep_fractions(){
        let double=[true,false,false,false];
        assert_eq!(starting_balances(650.0,&[0,1,2,3],true,double).unwrap(),[326.0,163.0,162.0,162.0]);
        assert_eq!(shared_credits(1.0,&[0,1,2,3],false,true,double).unwrap(),[0.25;4]);
        assert_eq!(shared_credits(1.0,&[0,1,2,3],true,true,double).unwrap(),[0.5,0.25,0.25,0.25]);
        assert_eq!(shared_credits(1.0,&[0,1,2,3],true,false,double).unwrap(),[0.25;4]);
    }
    #[test]fn malformed_players_and_nonfinite_amounts_are_rejected(){
        for players in [&[][..],&[0,0][..],&[0,4][..]]{assert!(starting_balances(650.0,players,false,[false;4]).is_err());}
        for amount in [f32::NAN,f32::INFINITY,-1.0,2147483648.0]{assert!(starting_balances(amount,&[0,1],false,[false;4]).is_err());}
        assert!(shared_credits(f64::INFINITY,&[0,1],false,false,[false;4]).is_err());
    }
}
