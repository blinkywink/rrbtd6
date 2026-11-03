//! Portable local identity and cosmetic progression. Nothing is uploaded.
use std::{path::Path, sync::{Mutex,OnceLock}};
use serde::{Serialize,Deserialize};
use serde_json::{Value,json};

#[derive(Clone,Serialize,Deserialize)]
#[serde(default)]
pub(crate) struct Profile {
    pub name:String, pub avatar:String, pub banner:String,
    pub visible_stats:std::collections::BTreeMap<String,bool>,
    pub experience:u64, pub maps:Value,
    pub wins:u64,pub losses:u64,pub highest_round:usize,pub revision:u64,
    /// Trophies to spend in the Trophy Store (DEVIATIONS.md: earned from maps and rank levels).
    pub trophies:u64,
    /// Trophy Store items bought, by wallet id: (enabled, favourite).
    pub trophy_items:std::collections::BTreeMap<String,(bool,bool)>,
    /// Powers bought with trophies, and how many are held.
    pub powers:std::collections::BTreeMap<String,u64>,
    /// Monkey Knowledge bought with trophies.
    pub knowledge:std::collections::BTreeSet<String>,
}
impl Default for Profile {
    fn default()->Self{Self{experience:0,maps:json!({}),visible_stats:Default::default(),name:"Player".into(),avatar:"ProfileAvatar01".into(),banner:"BannerDefault".into(),wins:0,losses:0,highest_round:0,revision:1,trophies:0,trophy_items:Default::default(),powers:Default::default(),knowledge:Default::default()}}
}
static LOCAL:OnceLock<Mutex<Profile>>=OnceLock::new();
fn state(root:&Path)->&'static Mutex<Profile>{LOCAL.get_or_init(||Mutex::new(load(root)))}
fn load(root:&Path)->Profile {
    for file in ["local-profile.json","local-profile.backup.json"] {
        if let Ok(bytes)=std::fs::read(root.join(file)) {
            if let Ok(profile)=serde_json::from_slice::<Profile>(&bytes){return profile;}
        }
    }
    let mut profile=Profile::default();
    if let Ok(bytes)=std::fs::read(root.join("player-name.json")) {
        if let Ok(seed)=serde_json::from_slice::<Value>(&bytes) {
            if let Some(name)=seed["name"].as_str().filter(|n|valid_name(n)){profile.name=name.trim().to_owned();}
        }
    }
    profile
}
pub(crate) fn get(root:&Path)->Profile{state(root).lock().unwrap().clone()}
pub(crate) fn update(root:&Path,change:impl FnOnce(&mut Profile))->Result<(),String>{
    let mut current=state(root).lock().unwrap();
    let mut next=current.clone();change(&mut next);next.revision+=1;
    save(root,&next).map_err(|_|"Could not save the local profile. Your previous profile is unchanged.".to_owned())?;
    *current=next;Ok(())
}
fn save(root:&Path,profile:&Profile)->std::io::Result<()> {
    let file=root.join("local-profile.json");let temporary=root.join("local-profile.tmp");
    let bytes=serde_json::to_vec_pretty(profile)?;
    {use std::io::Write;let mut output=std::fs::File::create(&temporary)?;output.write_all(&bytes)?;output.sync_all()?;}
    if file.exists(){std::fs::copy(&file,root.join("local-profile.backup.json"))?;}
    std::fs::rename(temporary,file)
}
pub(crate) fn rename(root:&Path,name:&str)->Result<(),String>{
    let name=name.trim();
    if !valid_name(name){return Err("Use a name with 1–24 characters.".into());}
    update(root,|p|p.name=name.to_owned())
}
fn valid_name(name:&str)->bool { !name.trim().is_empty()&&name.chars().count()<=24&&!name.chars().any(char::is_control) }
pub(crate) fn reset(root:&Path)->Result<(),String>{
    update(root,reset_progress)
}
fn reset_progress(p:&mut Profile){let name=p.name.clone();let revision=p.revision;*p=Profile{name,revision,..Default::default()};}
pub(crate) fn map_id(root:&Path,key:&str)->String {
    std::fs::read(root.join("ui/mapset.json")).ok().and_then(|b|serde_json::from_slice::<Vec<Value>>(&b).ok())
        .and_then(|rows|rows.into_iter().filter_map(|r|r["id"].as_str().map(str::to_owned)).find(|id|crate::frontend::map_key(id)==key))
        .unwrap_or_else(||key.to_owned())
}
/// Native mode medals are local mementos, including runs with mods. Custom modes have separate IDs.
fn medal_mode(mode:&str)->Option<(&str,&str,&str)> {
    Some(match mode {
        "Easy"=>("Easy","Standard","Easy"),"Medium"=>("Medium","Standard","Medium"),"Hard"=>("Hard","Standard","Hard"),
        "Impoppable"=>("Hard","Impoppable","Impoppable"),"CHIMPS"=>("Hard","Clicks","CHIMPS"),
        other=>{let (difficulty,variant)=other.split_once(' ')?;
            if !matches!((difficulty,variant),("Easy","PrimaryOnly"|"Deflation")|("Medium","MilitaryOnly"|"Apopalypse"|"Reverse")|("Hard","MagicOnly"|"DoubleMoabHealth"|"HalfCash"|"AlternateBloonsRounds")){return None;}
            (difficulty,variant,variant)
        }
    })
}
/// Monkey Money in trophies: what makes a first Easy win on a beginner map 10 (DEVIATIONS.md).
pub(crate) const MONEY_PER_TROPHY:f64=7.5;
/// A Power's price: its Monkey Money cost in trophies, rounded up.
pub(crate) fn power_price(cost:i64)->u64{((cost.max(0) as f64/MONEY_PER_TROPHY).ceil() as u64).max(1)}
/// A Monkey Knowledge's price: its point (a rank level, so TROPHIES_PER_LEVEL) plus any Monkey Money.
pub(crate) fn knowledge_price(cost:i64)->u64{TROPHIES_PER_LEVEL+(cost.max(0) as f64/MONEY_PER_TROPHY).round() as u64}
pub(crate) fn buy_power(root:&Path,id:&str,price:u64)->bool{
    if get(root).trophies<price{return false;}
    update(root,|p|{p.trophies-=price;*p.powers.entry(id.to_owned()).or_insert(0)+=1;}).is_ok()
}
pub(crate) fn buy_knowledge(root:&Path,id:&str,price:u64)->bool{
    let p=get(root);
    if p.knowledge.contains(id)||p.trophies<price{return false;}
    update(root,|p|{p.trophies-=price;p.knowledge.insert(id.to_owned());}).is_ok()
}
/// Our trophy reward for a win (DEVIATIONS.md): the game's Monkey Money for the difficulty and
/// mode (MapInfoManager's 75 / 125 / 200, 300 for Impoppable and CHIMPS) times the map tier + 1
/// (beginner 0 .. expert 3), a fifth on a replay, divided by 7.5: 10 for a beginner map on Easy.
pub(crate) fn trophy_reward(p:&Profile,map:&str,mode:&str,coop:bool,tier:usize)->u64{
    let Some((difficulty,variant,_))=medal_mode(mode) else{return 0};
    let base=match (difficulty,variant){("Hard","Impoppable"|"Clicks")=>300.0,("Hard",_)=>200.0,("Medium",_)=>125.0,_=>75.0};
    let packed=p.maps[map]["difficult"][difficulty][if coop{"coopModes"}else{"modes"}][variant].as_u64().unwrap_or(0);
    let first=((packed>>20)&65535)+((packed>>36)&65535)==0;
    let money=base*(tier.min(3)+1) as f64/if first{1.0}else{5.0};
    (money/MONEY_PER_TROPHY).round() as u64
}
/// Trophies for each rank level gained (DEVIATIONS.md).
pub(crate) const TROPHIES_PER_LEVEL:u64=10;
/// Adds rank XP, with TROPHIES_PER_LEVEL for every level it crosses (a level is 1000 XP).
pub(crate) fn add_experience(p:&mut Profile,xp:u64){
    let before=p.experience/1000;
    p.experience=p.experience.saturating_add(xp);
    p.trophies=p.trophies.saturating_add((p.experience/1000-before)*TROPHIES_PER_LEVEL);
}
/// Buys Trophy Store item `id` for `price` trophies (enabled, as a purchase is); false when it's
/// owned already or not affordable.
pub(crate) fn buy_trophy_item(root:&Path,id:&str,price:u64)->bool{
    let p=get(root);
    if p.trophy_items.contains_key(id)||p.trophies<price{return false;}
    update(root,|p|{p.trophies-=price;p.trophy_items.insert(id.to_owned(),(true,false));}).is_ok()
}
/// Sets an owned item's enabled / favourite flags.
pub(crate) fn set_trophy_item(root:&Path,id:&str,enabled:Option<bool>,favourite:Option<bool>){
    let _=update(root,|p|{if let Some(o)=p.trophy_items.get_mut(id){
        if let Some(e)=enabled{o.0=e;}if let Some(f)=favourite{o.1=f;}
    }});
}
pub(crate) fn record_result(p:&mut Profile,map:&str,mode:&str,coop:bool,won:bool,round:usize){
    p.highest_round=p.highest_round.max(round);
    if won {p.wins=p.wins.saturating_add(1);}else{p.losses=p.losses.saturating_add(1);}
    let Some((difficulty,variant,_))=medal_mode(mode).filter(|_|won) else{return};
    // One bit represents completion. Replaying a map does not multiply medal counts.
    // Black CHIMPS is intentionally not awarded without a verified no-retry run.
    p.maps[map]["difficult"][difficulty][if coop{"coopModes"}else{"modes"}][variant]=json!(1u64<<20);
}
pub(crate) fn preview(root:&Path)->Value {
    let p=get(root);
    let mut v=preview_of(p.clone());
    // The Trophy Store's ownership rows, in its catalogue order (ui/runtime/trophy_items.json).
    static ORDER:OnceLock<Vec<String>>=OnceLock::new();
    let order=ORDER.get_or_init(||std::fs::read(root.join("ui/runtime/trophy_items.json")).ok()
        .and_then(|b|serde_json::from_slice::<Vec<Value>>(&b).ok()).unwrap_or_default().iter()
        .map(|t|t["raw"]["walletIdOverride"].as_str().filter(|s|!s.is_empty()).or(t["id"].as_str()).unwrap_or("").to_owned()).collect());
    v["trophyOwnership"]=order.iter().map(|id|{
        let own=p.trophy_items.get(id);
        json!({"owned":own.is_some(),"enabled":own.is_some_and(|o|o.0),"favorite":own.is_some_and(|o|o.1),"canEnable":true})
    }).collect();
    v["profile"]["trophyStorePurchasedItems"]=p.trophy_items.keys().map(|id|(id.clone(),json!(true))).collect::<serde_json::Map<_,_>>().into();
    v
}

fn preview_of(p:Profile)->Value {
    let mut medals=json!({"SPMedals":{},"CoopMedals":{},"SpecialMedals":{}});
    for map in p.maps.as_object().into_iter().flatten().map(|(_,m)|m){
        for mode in ["Easy","Easy PrimaryOnly","Easy Deflation","Medium","Medium MilitaryOnly","Medium Apopalypse","Medium Reverse","Hard","Hard MagicOnly","Hard DoubleMoabHealth","Hard HalfCash","Hard AlternateBloonsRounds","Impoppable","CHIMPS"]{
            let (difficulty,variant,label)=medal_mode(mode).unwrap();
            for (key,group) in [("modes","SPMedals"),("coopModes","CoopMedals")]{
                let packed=map["difficult"][difficulty][key][variant].as_u64().unwrap_or(0);
                if ((packed>>20)&65535)+((packed>>36)&65535)>0 {
                    let label=if variant=="Clicks"&&packed&8!=0{"CHIMPS-BLACK"}else{label};
                    medals[group][label]=json!(medals[group][label].as_u64().unwrap_or(0)+1);
                }
            }
        }
    }
    let level=1+p.experience/1000;
    json!({"schema":1,"localProfile":true,"readOnly":true,"uploadsEnabled":false,
        "displayName":p.name,"downloadedAt":p.revision,"profileArtwork":{"profileAvatar":p.avatar,"profileBanner":p.banner},
        "profile":{"rank":level,"veteranRank":0,"monkeyMoney":0,"trophies":p.trophies,"knowledgePoints":0,
            "acquiredKnowledge":p.knowledge,
            "powersData":p.powers.iter().map(|(id,n)|(id.clone(),json!({"quantity":n,"isNew":false}))).collect::<serde_json::Map<_,_>>(),
            "completedGame":p.wins,"highestSeenRound":p.highest_round,"highestSeenRoundCurrentVersion":p.highest_round,
            "unlockedTowers":[],"unlockedHeroes":[],"acquiredUpgrades":[],"viewedUpgrades":[],"mapInfo":{"maps":p.maps}},
        "stats":{"gamesPlayed":p.wins.saturating_add(p.losses),"gamesLost":p.losses},
        "rankProgress":{"current":p.experience%1000,"required":1000,"fraction":(p.experience%1000) as f64/1000.0,"max":false},
        "medals":medals,"top":{"heroes":[],"towers":[]},"heroCollection":{"owned":0,"total":0},"visibleStats":p.visible_stats})
}

#[cfg(test)] mod tests {
    use super::*;
    #[test] fn prices_are_in_trophies(){
        assert_eq!(power_price(375),50);
        assert_eq!(power_price(100),14);
        assert_eq!(power_price(1),1);
        assert_eq!(knowledge_price(0),10);
        assert_eq!(knowledge_price(750),110);
    }
    #[test] fn old_profiles_load_with_nothing_bought(){
        let p:Profile=serde_json::from_str(r#"{"trophies":5}"#).unwrap();
        assert_eq!(p.trophies,5);
        assert!(p.powers.is_empty()&&p.knowledge.is_empty());
    }
    #[test] fn bought_things_reach_the_screens(){
        let mut p=Profile::default();
        p.powers.insert("MoabMine".into(),2);p.knowledge.insert("MoreCash".into());p.trophies=9;
        let v=preview_of(p);
        assert_eq!(v["profile"]["powersData"]["MoabMine"]["quantity"],2);
        assert_eq!(v["profile"]["acquiredKnowledge"][0],"MoreCash");
        assert_eq!(v["profile"]["trophies"],9);
    }
    #[test] fn invalid_names_do_not_touch_the_profile(){
        for name in ["", "   ", "bad\nname", "1234567890123456789012345"]{assert!(rename(Path::new("unused-profile-path"),name).is_err());}
    }
    #[test] fn offline_preview_has_complete_profile_sections_and_no_remote_identity(){
        let p=preview_of(Profile{name:"Offline".into(),wins:3,losses:2,highest_round:98,..Default::default()});
        assert_eq!(p["localProfile"],true);assert_eq!(p["displayName"],"Offline");
        assert_eq!(p["stats"]["gamesPlayed"],5);assert_eq!(p["profile"]["completedGame"],3);
        assert_eq!(p["profile"]["highestSeenRound"],98);
        assert!(p["medals"]["SPMedals"].is_object());assert!(p["top"]["towers"].is_array());
        assert_eq!(p["uploadsEnabled"],false);
    }
    #[test] fn local_profile_roundtrip_and_backup_recovery_preserve_identity_and_progress(){
        let root=std::env::temp_dir().join(format!("rrbtd6-profile-{}-{}",std::process::id(),std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
        std::fs::create_dir(&root).unwrap();
        let p=Profile{name:"Local Test".into(),wins:2,highest_round:140,..Default::default()};
        save(&root,&p).unwrap();assert_eq!(load(&root).name,"Local Test");assert_eq!(load(&root).highest_round,140);
        save(&root,&Profile{name:"Next".into(),..p}).unwrap();
        std::fs::write(root.join("local-profile.json"),b"interrupted").unwrap();
        assert_eq!(load(&root).name,"Local Test");assert_eq!(load(&root).wins,2);
        for file in ["local-profile.json","local-profile.backup.json"]{std::fs::remove_file(root.join(file)).unwrap();}
        std::fs::remove_dir(root).unwrap();
    }
}

#[cfg(test)] mod progression_tests {
    use super::*;
    #[test] fn solo_and_coop_medals_are_independent_and_replays_do_not_duplicate_them(){
        let mut p=Profile::default();
        record_result(&mut p,"MonkeyMeadow","Easy",false,true,40);
        record_result(&mut p,"MonkeyMeadow","Easy",false,true,40);
        let data=preview_of(p.clone());
        assert_eq!(data["medals"]["SPMedals"]["Easy"],1);
        assert!(data["medals"]["CoopMedals"]["Easy"].is_null());
        record_result(&mut p,"MonkeyMeadow","Hard HalfCash",true,true,80);
        let data=preview_of(p);
        assert_eq!(data["medals"]["CoopMedals"]["HalfCash"],1);
        assert_eq!(data["profile"]["mapInfo"]["maps"]["MonkeyMeadow"]["difficult"]["Hard"]["coopModes"]["HalfCash"],1u64<<20);
    }
    #[test] fn losses_and_custom_modes_do_not_award_native_medals_or_black_chimps(){
        let mut p=Profile::default();
        record_result(&mut p,"Logs","Hard",true,false,79);
        record_result(&mut p,"Logs","modded-mode",false,true,200);
        assert_eq!(preview_of(p.clone())["medals"]["SPMedals"],json!({}));
        record_result(&mut p,"Logs","CHIMPS",true,true,100);
        let data=preview_of(p);assert_eq!(data["medals"]["CoopMedals"]["CHIMPS"],1);
        assert!(data["medals"]["CoopMedals"]["CHIMPS-BLACK"].is_null());
    }
    #[test] fn level_is_cosmetic_and_old_profiles_migrate(){
        let p:Profile=serde_json::from_str(r#"{"name":"Existing","wins":5}"#).unwrap();
        assert_eq!(p.experience,0);assert_eq!(p.maps,json!({}));
        let data=preview_of(Profile{experience:1250,..p});
        assert_eq!(data["profile"]["rank"],2);assert_eq!(data["rankProgress"]["current"],250);
    }
    #[test] fn reset_clears_progress_but_keeps_the_chosen_identity(){
        let mut p=Profile{name:"Keep me".into(),experience:10000,wins:5,losses:2,highest_round:200,revision:7,..Default::default()};
        record_result(&mut p,"Tutorial","Easy",true,true,40);
        reset_progress(&mut p);
        assert_eq!(p.name,"Keep me");assert_eq!(p.revision,7);
        assert_eq!((p.experience,p.wins,p.losses,p.highest_round),(0,0,0,0));assert_eq!(p.maps,json!({}));
    }
    #[test] fn installer_seed_never_overwrites_a_saved_profile(){
        let root=std::env::temp_dir().join(format!("rrbtd6-name-seed-{}",std::process::id()));
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(root.join("player-name.json"),br#"{"name":"Imported"}"#).unwrap();
        assert_eq!(load(&root).name,"Imported");
        save(&root,&Profile{name:"Chosen".into(),..Default::default()}).unwrap();
        assert_eq!(load(&root).name,"Chosen");
        for file in ["player-name.json","local-profile.json"]{std::fs::remove_file(root.join(file)).unwrap();}
        std::fs::remove_dir(root).unwrap();
    }
}
