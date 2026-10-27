//! Native Home event buttons, backed by cached/live read-only configuration.
use std::{path::Path, sync::{mpsc,Mutex,OnceLock},time::{SystemTime,UNIX_EPOCH}};
use macroquad::prelude::*;
use serde_json::Value;
use crate::uitree::Node;
use super::{ScrollDrag,ScrollInput,ScrollState,scroll_limit};

const BASE:&str="MainMenuCanvas/MainMenu/EventButtonGroup";
const VIEW:&str="MainMenuCanvas/MainMenu/EventButtonGroup/Viewport";
const CONTENT:&str="MainMenuCanvas/MainMenu/EventButtonGroup/Viewport/Content";
struct Events { rx:mpsc::Receiver<Value>, data:Value, keys:Vec<String>,scroll:ScrollState,drag:Option<ScrollDrag>,origin:Option<Vec2>,second:u64,hidden:Vec<EventKind> }
static EVENTS:OnceLock<Mutex<Events>>=OnceLock::new();

fn template(row:&Value)->Option<&'static str> {
    Some(match row["type"].as_str()? {
        "raceEvent"=>"HomeRace","odysseyEvent"=>"HomeOdyssey","bossRush"=>"HomeBossRush",
        "ct"|"contestedTerritory"=>"HomeCt","socialseason"=>"HomeSocialSeason",
        "bossBloon"=>match boss(row)? {
            "Bloonarius"=>"BloonariusEventPanelBtn","Vortex"=>"VortexEventPanelBtn",
            "Lych"=>"LychEventPanelBtn","Dreadbloon"=>"DreadbloonEventPanelBtn",
            "Phayze"=>"PhayzeEventPanelBtn","Blastapopoulos"=>"BlastapopoulosEventPanelBtn",
            "Diamondback"=>"DiamondbackEventPanelBtn",_=>return None,
        },
        _=>return None,
    })
}
/// Event kind, for the main menu toggles in Advanced settings (DEVIATIONS.md 3).
pub(crate) fn kind(row:&Value)->Option<EventKind> {
    Some(match row["type"].as_str()? {
        "raceEvent"=>EventKind::Race,"odysseyEvent"=>EventKind::Odyssey,"bossRush"=>EventKind::BossRush,
        "ct"|"contestedTerritory"=>EventKind::ContestedTerritory,"socialseason"=>EventKind::SocialSeason,
        "bossBloon"=>EventKind::Boss,_=>return None,
    })
}
#[derive(Clone,Copy,PartialEq,Eq,Debug)]
pub(crate) enum EventKind {Race,Boss,BossRush,Odyssey,ContestedTerritory,SocialSeason}
fn boss(row:&Value)->Option<&'static str> {
    let name=row["name"].as_str()?;
    ["Bloonarius","Vortex","Lych","Dreadbloon","Phayze","Blastapopoulos","Diamondback"]
        .into_iter().find(|boss|name.starts_with(boss))
}
fn active(row:&Value,now:u64)->bool {
    let (Some(start),Some(end))=(row["start"].as_u64(),row["end"].as_u64()) else{return false};
    let platform=row["skusSelected"].as_array().is_none_or(|a|a.is_empty()||a.iter().any(|s|s.as_u64()==Some(1136)));
    platform && start<=now && now<end
}

/// Ninja Kiwi services are contacted only when RRBTD6_NK_ONLINE=1 is set explicitly.
pub(super) fn nk_online()->bool {std::env::var("RRBTD6_NK_ONLINE").is_ok_and(|v|v=="1")}

fn state(root:&Path)->&'static Mutex<Events> {
    EVENTS.get_or_init(|| {
        let (tx,rx)=mpsc::channel();let root=root.to_owned();
        std::thread::spawn(move|| {
            let path=root.join("online/events.json");
            let read=||std::fs::read(&path).ok().and_then(|s|serde_json::from_slice::<Value>(&s).ok());
            if let Some(data)=read(){let _=tx.send(data);}
            // Offline by default: the cached events.json is used as-is.
            if !nk_online(){return;}
            loop {
                let project=root.parent().unwrap_or(&root);
                let python=project.join("tools/.venv/Scripts/python.exe");
                let mut command=std::process::Command::new(if python.is_file(){python}else{"python".into()});
                command.arg("-I").arg(project.join("tools/assets/fetch_live_events.py")).arg(&root);
                #[cfg(windows)] {use std::os::windows::process::CommandExt;command.creation_flags(0x08000000);}
                if command.output().is_ok(){if let Some(data)=read(){if tx.send(data).is_err(){break;}}}
                if std::env::var_os("RRBTD6_AUTOTEST").is_some(){break;}
                std::thread::sleep(std::time::Duration::from_secs(900));
            }
        });
        Mutex::new(Events {rx,data:Value::Null,keys:Vec::new(),scroll:Default::default(),drag:None,origin:None,second:0,hidden:Vec::new()})
    })
}

/// `hidden`: event kinds the player turned off; the remaining buttons close up (VerticalLayoutGroup).
pub fn update(root:&Path,node:&mut Node,scale:f32,hidden:&[EventKind])->Option<super::menu_pages::MenuPage> {
    let state=state(root);
    let Ok(mut state)=state.try_lock() else{return None};
    let update=state.rx.try_iter().last();
    let now=SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_millis() as u64;
    let changed=update.is_some();if let Some(data)=update{state.data=data;}
    if changed || state.second!=now/1000 || state.hidden.as_slice()!=hidden {
        state.hidden=hidden.to_vec();
        state.second=now/1000;
        let rows:Vec<_>=state.data["live"]["events"].as_array().into_iter().flatten()
            .filter(|row|active(row,now)).filter(|row|kind(row).is_none_or(|k|!hidden.contains(&k)))
            .filter_map(|row|Some((row["id"].as_str()?.to_owned(),template(row)?))).collect();
        let keys:Vec<_>=rows.iter().map(|(id,_)|id.clone()).collect();
        if keys!=state.keys {
            if let Some(content)=node.find_mut(CONTENT) {
                if state.origin.is_none(){state.origin=Some(content.rt.pos);}
                content.children=rows.iter().filter_map(|(id,file)| {
                    let mut button=Node::load(&root.join(format!("ui/prefabs/{file}.json")))?;
                    button.name=id.clone();button.active=true;
                    button.hide_named(&["Glow","Pip","TicketsAvailable"]);
                    Some(button)
                }).collect();
            }
            state.keys=keys;state.scroll=Default::default();state.drag=None;
        }
    }
    let any=node.find(CONTENT).is_some_and(|n|!n.children.is_empty());
    node.set_active(BASE,any);
    let area=node.rect(VIEW).unwrap_or_default();let limit=scroll_limit(node,VIEW,CONTENT,scale);
    let input=ScrollInput {pointer:mouse_position().into(),pressed:is_mouse_button_pressed(MouseButton::Left),
        held:is_mouse_button_down(MouseButton::Left),released:is_mouse_button_released(MouseButton::Left),wheel:mouse_wheel().1,
        dt:get_frame_time(),scale};
    let Events {scroll,drag,..}=&mut *state;
    let clicked=scroll.update(drag,0,area,limit,input);
    if let Some(content)=node.find_mut(CONTENT){content.rt.pos=state.origin.unwrap_or(content.rt.pos)+vec2(0.0,state.scroll.offset);}
    if !clicked || !area.contains(input.pointer){return None;}
    let id=&node.find(CONTENT)?.children.iter().find(|c|c.active && c.screen.contains(input.pointer))?.name;
    let row=state.data["live"]["events"].as_array()?.iter().find(|r|r["id"].as_str()==Some(id.as_str()))?;
    use super::menu_pages::MenuPage;
    Some(match row["type"].as_str()? {
        "raceEvent"=>MenuPage::Race,
        "bossBloon"=>MenuPage::BossRules,
        "socialseason"=>MenuPage::SocialSeason,
        _=>MenuPage::Events,
    })
}

pub fn bind_browser(root:&Path,node:&mut Node) {
    const SCREEN:&str="Canvas/GameEventsScreen";
    if node.find(SCREEN).is_none(){return;}
    let Ok(mut state)=state(root).try_lock() else{return};
    if let Some(data)=state.rx.try_iter().last(){state.data=data;}
    let now=SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_millis() as u64;
    let rows=state.data["live"]["events"].as_array();
    let content=format!("{SCREEN}/Content/RotatingEvents/Scroll/LayoutGroup/Content");
    for (name,kind) in [("SocialSeasons","socialseason"),("Race","raceEvent"),("Odyssey","odysseyEvent"),
        ("BossEvent","bossBloon"),("ContestedTerritory","ct"),("BossRush","bossRush"),("CollectionEvent","collectableEvent"),("Sweepstakes","sweepstakes")] {
        let event=rows.into_iter().flatten().find(|e|e["type"]==kind && active(e,now));
        let path=format!("{content}/{name}");node.set_active(&path,event.is_some());
        if let Some(event)=event {
            if kind=="socialseason" {node.set_text(&format!("{path}/Title"),event["name"].as_str().unwrap_or("Social Season"));}
            if kind=="bossBloon" {
                if let Some(boss)=boss(event) {
                    node.set_sprite(&format!("{path}/BannerImage"),Some(&format!("EventBannerSmall{boss}")));
                    node.set_sprite(&format!("{path}/Icon"),Some(&format!("{boss}EventIcon")));
                }
            }
            for part in ["NotRunning","NextTimer","EventEnded","Rewards"]{node.set_active(&format!("{path}/{part}"),false);}
            node.set_active(&format!("{path}/Timer"),true);
            let end=event["end"].as_u64().unwrap_or(now);
            node.set_text(&format!("{path}/Timer/TimeRemaining"),&remaining(end.saturating_sub(now)/1000));
        }
    }
    for (daily,key) in [("Standard","standard"),("Advanced","advanced")] {
        let done=state.data["live"]["dailyIds"][key].as_str().is_some_and(super::account_preview::daily_completed);
        node.set_active(&format!("{SCREEN}/Content/DailyChallengesGroup/{daily}/ClaimedPanel"),done);
    }
    let coop=rows.into_iter().flatten().find(|e|e["type"]=="coopChallenge" && active(e,now));
    let base=format!("{SCREEN}/Content/DailyChallengesGroup/Coop");
    node.set_active(&format!("{base}/TimeBanner"),coop.is_some());
    let coop_complete=state.data["metadata"]["current_daily_coop"]["body"]["id"].as_str().is_some_and(super::account_preview::daily_completed)
        || coop.and_then(|e|e["id"].as_str()).is_some_and(|id|super::account_preview::daily_completed(&format!("coop{id}")));
    node.set_active(&format!("{base}/ClaimedPanel"),coop_complete);
    if let Some(end)=coop.and_then(|e|e["end"].as_u64()){node.set_text(&format!("{base}/TimeBanner/Time Remaining"),&remaining(end.saturating_sub(now)/1000));}
    node.set_active(&format!("{SCREEN}/Content/FeaturedBanner/LoadingStatsSpinner"),false);
    node.set_active(&format!("{SCREEN}/Content/FeaturedBanner/Buttons"),false);
    if let Some(hours)=state.data["live"]["response"]["body"]["settings"]["settings"]["dailyResetUTCOffset"].as_u64() {
        let seconds=now/1000;let offset=hours*3600;
        let reset=((seconds.saturating_sub(offset))/86400+1)*86400+offset;
        node.set_text(&format!("{SCREEN}/Content/DailyChallengesGroup/TimeBanner/Time Remaining"),&remaining(reset.saturating_sub(seconds)));
    }
}

/// Trophy Store live state: TrophyStoreBankData prices, the SKU's trophyStore hidden / new
/// lists and the active trophyStore events (TrophyEventMetadata onSale / featured / enabled).
/// TowerGoalUnlockManager goals: the SKU's towerUnlocks (pops needed per gift tower), from the cache.
pub(super) fn tower_unlock_goals(root:&Path)->std::collections::HashMap<String,f64> {
    let Ok(mut state)=state(root).try_lock() else {return Default::default()};
    if let Some(data)=state.rx.try_iter().last(){state.data=data;}
    state.data["live"]["response"]["body"]["settings"]["settings"]["towerUnlocks"].as_object().into_iter().flatten()
        .filter_map(|(k,v)|Some((k.clone(),v["pops"].as_f64()?))).collect()
}
/// Reuses the background events cache; no parsing or network request on the UI thread.
pub(super) fn daily_rewards(root:&Path)->Option<(Vec<String>,i64)> {
    let mut state=state(root).try_lock().ok()?;
    if let Some(data)=state.rx.try_iter().last(){state.data=data;}
    let settings=&state.data["live"]["response"]["body"]["settings"];
    let rewards=settings["gameEvents"]["dailyRewards"].as_array()?.iter().filter_map(|v|v.as_str().map(str::to_owned)).collect::<Vec<_>>();
    if rewards.is_empty(){return None;}
    Some((rewards,settings["settings"]["dailyResetUTCOffset"].as_i64()?*3600))
}

pub(super) fn trophy_live(root:&Path)->Option<Value> {
    let mut state=state(root).try_lock().ok()?;
    if let Some(data)=state.rx.try_iter().last(){state.data=data;}
    let now=SystemTime::now().duration_since(UNIX_EPOCH).ok()?.as_millis() as u64;
    let events:Vec<_>=state.data["live"]["events"].as_array().into_iter().flatten()
        .filter(|row|row["type"]=="trophyStore"&&active(row,now)).cloned().collect();
    Some(serde_json::json!({"prices":state.data["trophyBank"]["prices"],"sku":state.data["live"]["response"]["body"]["settings"]["trophyStore"],
        "events":events,"checkedAt":state.data["checkedAt"]}))
}

/// Helpers.ToEventTimeRemainingDisplay(span, true): under 1.5 days "{hours+24*days:00}:{mm}:{ss}",
/// otherwise "{whole days} day" / "days".
pub(super) fn event_time_remaining(seconds:u64,day:&str,days:&str)->String {
    if (seconds as f64)/86400.0<1.5 {
        format!("{:02}:{:02}:{:02}",seconds/3600,seconds/60%60,seconds%60)
    } else {
        let whole=seconds/86400;
        format!("{whole} {}",if whole<2{day}else{days})
    }
}

pub(super) fn remaining(seconds:u64)->String {
    let days=seconds/86400;let hours=seconds/3600%24;let minutes=seconds/60%60;let seconds=seconds%60;
    if days>0 {format!("{days}d {hours:02}:{minutes:02}:{seconds:02}")}else{format!("{hours:02}:{minutes:02}:{seconds:02}")}
}

pub(super) fn detail(root:&Path,key:&str)->Option<Value>{
    let mut state=state(root).try_lock().ok()?;
    if let Some(data)=state.rx.try_iter().last(){state.data=data;}
    let now=SystemTime::now().duration_since(UNIX_EPOCH).ok()?.as_millis() as u64;
    if let Some(category)=key.strip_prefix("daily_"){
        let body=if state.data["dailyDetails"][category].is_object(){&state.data["dailyDetails"][category]}else{&state.data["metadata"][format!("current_daily_{category}")]["body"]};
        let offset=state.data["live"]["response"]["body"]["settings"]["settings"]["dailyResetUTCOffset"].as_u64().unwrap_or(8)*3600;
        let mut end=((now/1000).saturating_sub(offset)/86400+1)*86400+offset;
        if category=="coop"{if let Some(row)=state.data["live"]["events"].as_array().and_then(|rows|rows.iter().find(|r|r["type"]=="coopChallenge"&&active(r,now))){end=row["end"].as_u64().unwrap_or(end*1000)/1000;}}
        return body.is_object().then(||serde_json::json!({"challenge":body,"event":{"end":end*1000},"title":body["name"],"revision":state.data["checkedAt"]}));
    }
    let row=state.data["live"]["events"].as_array()?.iter().find(|r|r["type"]==if key=="bossElite"{"bossBloon"}else{key}&&active(r,now))?;
    let body=&state.data["nativeEventData"][row["id"].as_str()?]["body"];
    let challenge=if key=="bossElite"{&body["eliteDcm"]}else if key=="bossBloon"{&body["normalDcm"]}else{&body["challenge"]};
    Some(serde_json::json!({"sendDelay":state.data["live"]["response"]["body"]["settings"]["gameEvents"]["races"]["settings"]["raceSendDelay"],"event":row,"body":body,"challenge":challenge,"title":row["name"],"revision":state.data["checkedAt"]}))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn expired_upcoming_or_wrong_platform_is_not_active(){
        let row=serde_json::json!({"start":100,"end":200,"skusSelected":[]});
        assert!(!active(&row,99));assert!(active(&row,100));assert!(!active(&row,200));
        assert!(!active(&serde_json::json!({"start":100,"end":200,"skusSelected":[1108]}),150));
    }
    #[test] fn timer_changes_at_day_boundary(){
        assert_eq!(remaining(86400),"1d 00:00:00");assert_eq!(remaining(86399),"23:59:59");
        assert_eq!(remaining(0),"00:00:00");
    }
}

/// Bind the original season layout without presenting prefab sample scores as live data.
pub fn bind_social_season(root:&Path,node:&mut Node){
    const BASE:&str="Canvas/SocialSeasonsEventScreen";
    let Ok(mut state)=state(root).try_lock() else{return};
    if let Some(data)=state.rx.try_iter().last(){state.data=data;}
    let now=SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_millis() as u64;
    let event=state.data["live"]["events"].as_array().into_iter().flatten().find(|e|e["type"]=="socialseason"&&active(e,now));
    node.hide_named(&["Loading","RewardsPopup","RewardsPopupRightHanded","Complete","PhaseUnlocked","StarsGained"]);
    for path in ["TopInfo/TeamTrophies","CompetitionActive/TeamsSection/Info","CompetitionActive/TeamsSection/TeamScoresContainer","CompetitionActive/ProgressBarsContainer/AggregateGoal/Limit"] {node.set_active(&format!("{BASE}/{path}"),false);}
    let Some(event)=event else{node.set_text(&format!("{BASE}/TopInfo/Inner/Title"),"Social Seasons");node.set_active(&format!("{BASE}/CompetitionActive"),false);return;};
    node.set_active(&format!("{BASE}/CompetitionActive"),true);
    let theme=state.data["live"]["events"].as_array().into_iter().flatten()
        .find(|e|e["type"]=="socialseasonperks"&&e["metadata"]["socialSeasonEventId"]==event["id"])
        .and_then(|e|e["metadata"]["themeId"].as_str());
    let suffix=theme_suffix(theme);
    node.set_active(&format!("{BASE}/TopInfo/BannerMask/BannerImage"),suffix.is_some());
    node.set_active(&format!("{BASE}/TopInfo/Inner/PortraitIcon"),suffix.is_some());
    if let Some(suffix)=suffix {
        node.set_sprite(&format!("{BASE}/TopInfo/BannerMask/BannerImage"),Some(&format!("ThemeBannerFull{suffix}")));
        node.set_sprite(&format!("{BASE}/TopInfo/Inner/PortraitIcon"),Some(&format!("ThemeMonkey{suffix}")));
        node.set_sprite(&format!("{BASE}/CompetitionActive/TeamsSection/TeamGoal/Container/Progress/FinalRewardButton/BadgeIcon"),Some(&format!("SocialSeasonsUltimateMedal{suffix}")));
    }
    node.set_active(&format!("{BASE}/CompetitionActive/ProgressBarsContainer/AggregateGoal/Container/Progress/FinalRewardButton/CosmeticReward"),false);
    node.set_sprite(&format!("{BASE}/Buttons/TeamIsland/TeamIslandBtn"),Some("TeamIslandBtn"));
    node.set_active(&format!("{BASE}/Buttons/TeamIsland/TeamIslandBtn/Jewel"),false);
    node.set_active(&format!("{BASE}/Buttons/Perks/PerksBtn/Jewel"),false);
    let number=event["metadata"]["seasonNumber"].as_u64().unwrap_or(0);
    node.set_text(&format!("{BASE}/TopInfo/Inner/Title"),&format!("Social Season {number}"));
    node.set_text(&format!("{BASE}/TopInfo/Inner/SubTitle"),"");
    node.set_text(&format!("{BASE}/TopInfo/Countdown/Title"),"Event ends in:");
    node.set_text(&format!("{BASE}/TopInfo/Countdown/Time"),&remaining(event["end"].as_u64().unwrap_or(now).saturating_sub(now)/1000));
    let settings=&state.data["live"]["response"]["body"]["settings"]["socialSeasons"]["eventSettings"];
    let setting=event["metadata"]["settingsId"].as_str().unwrap_or("default");
    let config=settings.get(setting).or_else(||settings.get("default"));
    let target=config.and_then(|c|c["globalGoal"]["milestoneRewards"].as_array()).and_then(|r|r.last()).and_then(|r|r["score"].as_u64());
    let goal=format!("{BASE}/CompetitionActive/ProgressBarsContainer/AggregateGoal/Container");
    node.set_text(&format!("{goal}/CommunityGoalTitle"),"Community Goal");
    node.set_text(&format!("{goal}/Progress/ProgressBar/ProgressText"),&target.map(|n|format!("— / {n}")).unwrap_or_else(||"—".into()));
    let team=format!("{BASE}/CompetitionActive/TeamsSection");
    node.set_text(&format!("{team}/TeamGoal/Container/Progress/ProgressBar/TotalPoints/Count"),"—");
    node.set_text(&format!("{team}/TeamScoresBackground/NoScoresText"),"Team scores are unavailable");
    // Source progress bars are authored at sample completion; no live aggregate is available yet.
    fn clear_fill(n:&mut Node){if let Some(img)=&mut n.img{if img.kind==3{img.fill_amount=0.0;}}for c in &mut n.children{clear_fill(c);}}
    if let Some(n)=node.find_mut(&format!("{BASE}/CompetitionActive")){clear_fill(n);}
}

pub(super) fn theme_suffix(theme:Option<&str>)->Option<&'static str>{match theme{Some("dark_magic_club")=>Some("DarkMagicClub"),Some("monkey_science")=>Some("Science"),Some("quincys_roadtrip")=>Some("QuincysRoadTrip"),Some("strikers_vacation")=>Some("StrikersVacation"),_=>None}}
pub(super) fn season_snapshot(root:&Path)->Option<Value>{
    let mut state=state(root).try_lock().ok()?;
    if let Some(data)=state.rx.try_iter().last(){state.data=data;}
    let now=SystemTime::now().duration_since(UNIX_EPOCH).ok()?.as_millis() as u64;
    let rows=state.data["live"]["events"].as_array()?;
    let event=rows.iter().find(|e|e["type"]=="socialseason"&&active(e,now))?;
    let theme=rows.iter().find(|e|e["type"]=="socialseasonperks"&&e["metadata"]["socialSeasonEventId"]==event["id"]).map(|e|&e["metadata"]["themeId"]);
    let settings=&state.data["live"]["response"]["body"]["settings"]["socialSeasons"]["eventSettings"];
    let id=event["metadata"]["settingsId"].as_str().unwrap_or("default");
    Some(serde_json::json!({"event":event,"settings":settings.get(id).or_else(||settings.get("default"))?,"theme":theme}))
}
