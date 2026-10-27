//! Portable local progression, independent of the encrypted Ninja Kiwi snapshot.
use std::{path::Path, sync::{Mutex,OnceLock}};
use serde::{Serialize,Deserialize};
use serde_json::{Value,json};

#[derive(Clone,Serialize,Deserialize)]
#[serde(default)]
pub(crate) struct Profile {
    pub name:String, pub avatar:String, pub banner:String,
    pub visible_stats:std::collections::BTreeMap<String,bool>,
    pub wins:u64,pub losses:u64,pub highest_round:usize,pub revision:u64,
}
impl Default for Profile {
    fn default()->Self{Self{visible_stats:Default::default(),name:"Player".into(),avatar:"ProfileAvatar01".into(),banner:"BannerDefault".into(),wins:0,losses:0,highest_round:0,revision:1}}
}
static LOCAL:OnceLock<Mutex<Profile>>=OnceLock::new();
fn state(root:&Path)->&'static Mutex<Profile>{LOCAL.get_or_init(||Mutex::new(load(root)))}
fn load(root:&Path)->Profile {
    for file in ["local-profile.json","local-profile.backup.json"] {
        if let Ok(bytes)=std::fs::read(root.join(file)) {
            if let Ok(profile)=serde_json::from_slice::<Profile>(&bytes){return profile;}
        }
    }
    Profile::default()
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
    if name.is_empty()||name.chars().count()>24||name.chars().any(char::is_control){return Err("Use a name with 1–24 characters.".into());}
    update(root,|p|p.name=name.to_owned())
}
pub(crate) fn preview(root:&Path)->Value {
    preview_of(get(root))
}
fn preview_of(p:Profile)->Value {
    json!({"schema":1,"localProfile":true,"readOnly":true,"uploadsEnabled":false,
        "displayName":p.name,"downloadedAt":p.revision,"profileArtwork":{"profileAvatar":p.avatar,"profileBanner":p.banner},
        "profile":{"rank":1,"veteranRank":0,"monkeyMoney":0,"trophies":0,"knowledgePoints":0,
            "completedGame":p.wins,"highestSeenRound":p.highest_round,"highestSeenRoundCurrentVersion":p.highest_round,
            "unlockedTowers":[],"unlockedHeroes":[],"acquiredUpgrades":[],"viewedUpgrades":[],"mapInfo":{"maps":{}}},
        "stats":{"gamesPlayed":p.wins+p.losses,"gamesLost":p.losses},
        "rankProgress":{"current":0,"required":1,"fraction":0.0,"max":false},
        "medals":{"SPMedals":{},"CoopMedals":{},"SpecialMedals":{}},
        "top":{"heroes":[],"towers":[]},"heroCollection":{"owned":0,"total":0},"achievements":[],"visibleStats":p.visible_stats})
}

#[cfg(test)] mod tests {
    use super::*;
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
