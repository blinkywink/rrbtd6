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
        // Races, Boss Rush, Contested Territory, Social Seasons and Odyssey are removed (DEVIATIONS.md).
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
        "bossBloon"=>EventKind::Boss,_=>return None,
    })
}
#[derive(Clone,Copy,PartialEq,Eq,Debug)]
pub(crate) enum EventKind {Boss}
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
                // The tools venv's interpreter (Scripts/python.exe on Windows, bin/python elsewhere).
                let python=project.join(if cfg!(windows){"tools/.venv/Scripts/python.exe"}else{"tools/.venv/bin/python"});
                let mut command=std::process::Command::new(if python.is_file(){python}else if cfg!(windows){"python".into()}else{"python3".into()});
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
        held:is_mouse_button_down(MouseButton::Left),released:is_mouse_button_released(MouseButton::Left),wheel:crate::hud::wheel_notches(),
        dt:get_frame_time(),scale};
    let Events {scroll,drag,..}=&mut *state;
    let clicked=scroll.update(drag,0,area,limit,input);
    if let Some(content)=node.find_mut(CONTENT){content.rt.pos=state.origin.unwrap_or(content.rt.pos)+vec2(0.0,state.scroll.offset);}
    if !clicked || !area.contains(input.pointer){return None;}
    let id=&node.find(CONTENT)?.children.iter().find(|c|c.active && c.screen.contains(input.pointer))?.name;
    let row=state.data["live"]["events"].as_array()?.iter().find(|r|r["id"].as_str()==Some(id.as_str()))?;
    use super::menu_pages::MenuPage;
    (row["type"]=="bossBloon").then_some(MenuPage::BossRules)
}

/// Trophy Store live state: TrophyStoreBankData prices, the SKU's trophyStore hidden / new
/// lists and the active trophyStore events (TrophyEventMetadata onSale / featured / enabled).
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
    let row=state.data["live"]["events"].as_array()?.iter().find(|r|r["type"]==if key=="bossElite"{"bossBloon"}else{key}&&active(r,now))?;
    let body=&state.data["nativeEventData"][row["id"].as_str()?]["body"];
    let challenge=if key=="bossElite"{&body["eliteDcm"]}else{&body["normalDcm"]};
    Some(serde_json::json!({"event":row,"body":body,"challenge":challenge,"title":row["name"],"revision":state.data["checkedAt"]}))
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
