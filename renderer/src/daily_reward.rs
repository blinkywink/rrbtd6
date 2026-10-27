//! Native daily chest calendar. Claims are session previews, never account writes.
use crate::uitree::Node;
use macroquad::prelude::*;
use serde_json::Value;
use std::{collections::HashMap,path::{Path,PathBuf},time::{SystemTime,UNIX_EPOCH}};
const BASE:&str="Canvas/DailyRewardScreen/BgPanel";
pub(super) struct Browser {
    root:PathBuf,today:Node,upcoming:Node,loot:Node,powers:Vec<Value>,loc:HashMap<String,String>,
    revision:Value,second:u64,index:u64,next:i64,offset:i64,rewards:Vec<String>,selected:usize,
    claimed:bool,ready:bool,descriptions:Vec<String>,runtime:Value,claimed_at:Option<f64>,
}
impl Browser {
    pub fn load(root:&Path,node:&mut Node)->Option<Self>{
        let today=node.find(&format!("{BASE}/TodayLootTemplate"))?.clone();
        let upcoming=node.find(&format!("{BASE}/LootTemplate"))?.clone();
        let loot=Node::load(&root.join("ui/prefabs/MediumBaseLootItem.json"))?;
        let powers=std::fs::read(root.join("ui/runtime/powers.json")).ok().and_then(|b|serde_json::from_slice(&b).ok()).unwrap_or_default();
        node.set_active(&format!("{BASE}/TodayLootTemplate"),false);
        node.set_active(&format!("{BASE}/LootTemplate"),false);
        if let Some(list)=node.find_mut(&format!("{BASE}/LootSet")){list.children.clear();list.img=None;}
        node.set_text(&format!("{BASE}/RewardInfo"),"Loading daily rewards…");
        let mut runtime:Value=std::fs::read(root.join("ui/runtime/DailyRewardUI.json")).ok()
            .and_then(|b|serde_json::from_slice(&b).ok()).unwrap_or_default();
        if let Some(animators)=runtime["animators"].as_array_mut(){
            animators.retain(|a|matches!(a["controller"].as_str(),Some("GodrayVfxAnimation"|"TickDropAC")));
            for animator in animators{for clip in animator["clips"].as_array_mut().into_iter().flatten(){
                for binding in clip["bindings"].as_array_mut().into_iter().flatten(){
                    if let Some(path)=binding["path"].as_str(){binding["path"]=Value::String(path.replace("/TodayLootTemplate/","/LootSet/Day0/"));}
                }
            }}
        }
        Some(Self{root:root.into(),today,upcoming,loot,powers,loc:crate::menus::load_loc(&root.join("text/English.xml")),
            revision:Value::Null,second:u64::MAX,index:0,next:0,offset:0,rewards:Vec::new(),selected:0,claimed:false,ready:false,descriptions:Vec::new(),runtime,claimed_at:None})
    }
    pub fn animate(&mut self,node:&mut Node){
        self.bind(node);
        super::apply_animation(node,&self.runtime,"GodrayVfxAnimation","rotate",Some(get_time().rem_euclid(3.8333332538604736) as f32));
        if !self.ready{super::apply_animation(node,&self.runtime,"TickDropAC","TickDrop",self.claimed_at.map(|t|(get_time()-t) as f32));}
    }
    fn label(&self,key:&str)->String{self.loc.get(key).cloned().unwrap_or_else(||key.to_owned())}
    pub fn bind(&mut self,node:&mut Node){
        let now=SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs() as i64;
        if self.second==now as u64{return;}self.second=now as u64;
        let Some((rewards,offset))=super::home_events::daily_rewards(&self.root) else{
            node.set_text(&format!("{BASE}/RewardInfo"),"Daily reward configuration is unavailable. No reward has been claimed.");return;
        };
        let Some((index,date,revision))=super::account_preview::daily_reward() else{return;};
        let changed=self.revision!=revision||self.rewards!=rewards||self.offset!=offset;
        if changed {
            self.revision=revision;self.index=index;self.offset=offset;self.rewards=rewards;
            self.next=if date.is_empty(){0}else if let Some(next)=parse_date(&date){next}else{
                node.set_text(&format!("{BASE}/RewardInfo"),"The saved daily reward time could not be read. No reward has been claimed.");
                self.revision=Value::Null;return;
            };
            self.claimed=false;self.claimed_at=None;self.selected=0;
        }
        // BTD6TimeUtil.GetDailyTimeNow = UTC now - dailyResetUTCOffset.
        // OpenChest.IsChestReady uses GetDailyTimeRemaining(next) < TimeSpan.Zero.
        let ready=available(self.next,now,self.offset);
        if changed||self.ready!=ready{self.ready=ready;self.rebuild(node);}
        self.info(node,now);
        if std::env::var_os("RRBTD6_DAILY_PREVIEW_OPEN").is_some()&&self.ready&&!self.claimed{
            self.open_preview(now);self.rebuild(node);self.info(node,now);
        }
    }
    fn reward(&self,raw:&str)->Option<(String,String,String)>{
        let (kind,value)=raw.split_once(':')?;let parts:Vec<_>=value.split(',').collect();
        let count=parts.get(1).copied().unwrap_or("1");
        Some(match kind {
            "MonkeyMoney"=>("BloonjaminsIcon".into(),value.into(),format!("{} {}",value,self.label("Monkey Money"))),
            "Power"=>{let power=self.powers.iter().find(|p|p["id"].as_str()==parts.first().copied())?;
                (power["icon"].as_str()?.trim_end_matches(".png").into(),format!("x{count}"),format!("{count} × {}",power["title"].as_str().unwrap_or(parts[0])))},
            "RandomInstaMonkey"=>(format!("InstaRandomTier{}",parts.first()?),format!("x{count}"),format!("{count} × Tier {} Insta Monkey",parts[0])),
            "RacesPass"=>("RacePassIconSmall".into(),format!("x{value}"),format!("{value} × Race Pass")),
            _=>return None,
        })
    }
    fn rebuild(&mut self,node:&mut Node){
        let mut cards=Vec::new();self.descriptions.clear();
        // DailyRewardsScreen.Open uses the previous reward after a claim, otherwise today's.
        let first=if self.ready{self.index}else{self.index.saturating_sub(1)};
        for day in 0..5 {
            let mut card=if day==0{self.today.clone()}else{self.upcoming.clone()};
            card.name=format!("Day{day}");card.active=true;card.rt.pos=Vec2::ZERO;
            let label=if day==0{self.label(if self.ready{"Today"}else{"Claimed"})}else{
                self.label("DailyRewardDay").replace("{0}",&(first+day as u64+1).to_string())};
            card.set_text("Date",&label);card.set_active("Tick",day==0&&!self.ready);
            card.set_active("ActiveButton",day==self.selected);
            let raw=&self.rewards[((first+day as u64)%self.rewards.len() as u64) as usize];
            let description=if let Some((icon,count,description))=self.reward(raw){
                let mut item=self.loot.clone();item.name="Reward".into();item.active=true;
                item.rt.amin=vec2(0.5,0.5);item.rt.amax=item.rt.amin;item.rt.pos=Vec2::ZERO;
                item.set_sprite("Icon",Some(&icon));item.set_text("ShortDesc",&count);
                if let Some(container)=card.find_mut("ItemContainer"){container.children=vec![item];}else{card.children.push(item);}
                description
            }else{"Reward preview unavailable".into()};
            self.descriptions.push(description);cards.push(card);
        }
        if let Some(list)=node.find_mut(&format!("{BASE}/LootSet")){list.children=cards;}
    }
    fn info(&self,node:&mut Node,now:i64){
        let description=self.descriptions.get(self.selected).map(String::as_str).unwrap_or("");
        let status=if self.ready{"Available — click today's reward to preview opening. No rewards are credited.".into()}
            else{let remaining=self.next.saturating_sub(now-self.offset).max(0) as u64;
                format!("{}Next chest in {}. No rewards credited.",if self.claimed{"Preview opened. "}else{""},super::home_events::remaining(remaining))};
        node.set_text(&format!("{BASE}/RewardInfo"),&format!("{description}\n{status}"));
    }
    pub fn click(&mut self,node:&mut Node,pointer:Vec2)->bool{
        let Some(day)=(0..5).find(|i|node.rect(&format!("{BASE}/LootSet/Day{i}")).is_some_and(|r|r.contains(pointer)))else{return false;};
        self.selected=day;
        if day==0&&self.ready {
            let now=SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs() as i64;
            self.open_preview(now);
        }
        self.second=u64::MAX;self.rebuild(node);self.bind(node);true
    }
    fn open_preview(&mut self,now:i64){
        self.index+=1;self.next=next_reset(now,self.offset);self.claimed=true;self.ready=false;self.claimed_at=Some(get_time());
    }
}
fn available(next:i64,now:i64,offset:i64)->bool{next<now-offset}
fn next_reset(now:i64,offset:i64)->i64{(now-offset).div_euclid(86400)*86400+86400}
/// Profile DateTime values are on the game's shifted daily clock, without a timezone suffix.
fn parse_date(value:&str)->Option<i64>{
    let date=value.get(..19)?;let b=date.as_bytes();
    if b[4]!=b'-'||b[7]!=b'-'||b[10]!=b'T'||b[13]!=b':'||b[16]!=b':'{return None;}
    let n=|a,b|date.get(a..b)?.parse::<i64>().ok();
    let (year,month,day,hour,minute,second)=(n(0,4)?,n(5,7)?,n(8,10)?,n(11,13)?,n(14,16)?,n(17,19)?);
    if !(1..=9999).contains(&year)||!(1..=12).contains(&month)||!(0..24).contains(&hour)||!(0..60).contains(&minute)||!(0..60).contains(&second){return None;}
    let leap=year%4==0&&(year%100!=0||year%400==0);
    let days=[31,if leap{29}else{28},31,30,31,30,31,31,30,31,30,31];
    if day<1||day>days[(month-1) as usize]{return None;}
    let y=year-i64::from(month<=2);let era=y.div_euclid(400);let yoe=y-era*400;
    let m=month+if month>2{-3}else{9};let doy=(153*m+2)/5+day-1;
    let days=era*146097+yoe*365+yoe/4-yoe/100+doy-719468;
    Some(days*86400+hour*3600+minute*60+second)
}
#[cfg(test)]mod tests{
    use super::*;
    #[test]fn shifted_profile_clock_is_not_parsed_as_local_time(){
        assert_eq!(parse_date("1970-01-01T00:00:00"),Some(0));
        assert_eq!(parse_date("2026-10-15T16:00:00"),Some(1792080000));
        assert!(parse_date("2026-02-29T00:00:00").is_none());
        assert!(parse_date("2024-02-29T00:00:00").is_some());
        assert!(parse_date("garbage").is_none());
    }
    #[test]fn chest_uses_shifted_utc_day_and_does_not_skip_missed_rewards(){
        let next=parse_date("2026-10-15T16:00:00").unwrap();let offset=8*3600;
        assert!(!available(next,next+offset,offset));
        assert!(available(next,next+offset+1,offset));
        let now=next+offset+20*86400;
        assert!(available(next,now,offset));
        let reset=next_reset(now,offset);
        assert!(!available(reset,now,offset));
        assert_eq!(reset+offset-now,8*3600);
        assert!(available(reset,reset+offset+1,offset));
    }
}
