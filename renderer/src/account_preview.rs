//! Decrypt the cached read-only account off the render thread, once per process.
use std::{path::Path, sync::{mpsc,Mutex, OnceLock}};
use crate::uitree::Node;

struct Preview { rx:mpsc::Receiver<Option<serde_json::Value>>, finished:bool, data: Option<serde_json::Value>, bound:std::collections::HashSet<String> }
static PREVIEW: OnceLock<Mutex<Preview>> = OnceLock::new();

/// Starts the cached-account read (once) and returns its state.
fn preview_state(root: &Path) -> &'static Mutex<Preview> {
    PREVIEW.get_or_init(|| {
        let project = helper_project(root);
        let (tx,rx)=mpsc::channel();
        let local_test=std::env::var_os("RRBTD6_AUTOTEST").is_some() && std::env::var_os("RRBTD6_LOCAL_PROFILE").is_some();
        std::thread::spawn(move || {
            if local_test{let _=tx.send(None);return;}
            if let Some(data)=read_preview(&project,"--preview"){let _=tx.send(Some(data));}
            // Offline by default: only the cached snapshot is read. Ninja Kiwi blocked the owner's
            // IP after a burst of requests, so live reads need an explicit RRBTD6_NK_ONLINE=1.
            if std::env::var_os("RRBTD6_AUTOTEST").is_none() && super::home_events::nk_online() {
                if let Some(data)=read_preview(&project,"--refresh-preview"){let _=tx.send(Some(data));}
            }
            let _=tx.send(None);
        });
        Mutex::new(Preview {data:Some(crate::local_profile::preview(root)),finished:false,bound:Default::default(),rx})
    })
}

/// Takes any account data the reader thread has delivered.
fn poll(state: &mut Preview) {
    let mut update=None;let mut finished=false;
    for packet in state.rx.try_iter(){if let Some(data)=packet{update=Some(data);}else{finished=true;}}
    state.finished|=finished;
    if let Some(data)=update {
        if state.data.as_ref().is_none_or(|old|old["downloadedAt"]!=data["downloadedAt"]){state.bound.clear();}
        state.data=Some(data);
    }
}

/// For screens outside the main menu (the in-game upgrade screen): load the cached account
/// without binding a menu page.
pub(crate) fn ensure(root: &Path) {
    if let Ok(mut state) = preview_state(root).try_lock() {poll(&mut state);}
}

pub fn bind(root: &Path, node: &mut Node) {
    let Ok(mut state) = preview_state(root).try_lock() else { return };
    poll(&mut state);
    if state.data.is_none(){
        if state.finished{node.set_text("Canvas/PlayerStatsScreen/Stats/TopInfo/PlayerName","Profile unavailable — saved sign-in has not been removed.");}
        return;
    }
    let initializing=state.bound.insert(node.name.clone());
    let data=state.data.as_ref().unwrap();
    if initializing {
        // KnowledgeHelper.GetCategoryProgressString: "{active}/{total}", active = owned and not
        // disabled, 0 while all knowledge is off. KnowledgeMain.CheckDisabledKnowledge colours it
        // red when anything learnt in the category is disabled, or all knowledge is off.
        let all_off=data["profile"]["knowledgeDisabled"]==true;
        let disabled:std::collections::HashSet<&str>=data["profile"]["disabledKnowledge"].as_array().into_iter().flatten().filter_map(|v|v.as_str()).collect();
        let mut owned=[0;6];let mut totals=[0;6];let mut any_disabled=[false;6];
        for item in data["knowledge"].as_array().into_iter().flatten() {
            if let Some(category)=item["category"].as_u64().filter(|&i|i<6) {
                totals[category as usize]+=1;
                if item["owned"]==true{owned[category as usize]+=1;}
                let id=item["path"].as_str().and_then(|p|p.rsplit('/').nth(1)).unwrap_or("");
                if disabled.contains(id){any_disabled[category as usize]=true;}
            }
        }
        for (i,category) in ["Primary","Military","Magic","Support","Heroes","Powers"].iter().enumerate() {
            let path=format!("Canvas/KnowledgeMain/Buttons/{category}Btn/Button/CompletedTxt");
            node.set_text(&path,&format!("{}/{}",if all_off{0}else{owned[i]},totals[i]));
            if let Some(txt)=node.find_mut(&path).and_then(|n|n.txt.as_mut()) {
                txt.color=macroquad::prelude::Color::new(1.0,if all_off||any_disabled[i]{0.0}else{1.0},if all_off||any_disabled[i]{0.0}else{1.0},1.0);
            }
        }
        const ACH:&str="Canvas/AchievementsScreen";
        if let Some(content)=node.find_mut(&format!("{ACH}/Scroll/Viewport/Content")) {
            let mut claimed=0;
            for (i,card) in content.children.iter_mut().enumerate() {
                let Some(item)=data["achievements"].get(i) else {continue};
                let done=item["claimed"]==true;claimed+=usize::from(done);
                let progress=item["progress"].as_f64().unwrap_or(0.0).clamp(0.0,1.0);
                card.set_active("ClaimedGroup",done);
                card.set_active("InProgressGroup",!done);
                card.set_active("RewardsGroup",!done);
                card.set_text("InProgressGroup/ProgressBar/ProgressText",&format!("{:.0}%",100.0*progress));
                if let Some(bar)=card.find_mut("InProgressGroup/ProgressBar/Mask/InnerBar"){bar.rt.amax.x=progress as f32;}
            }
            let total=content.children.len();
            node.set_text(&format!("{ACH}/TopInfo/Count"),&format!("{claimed}/{total}"));
        }
    }
    const STATS: &str = "Canvas/PlayerStatsScreen/Stats";
    if node.find(STATS).is_some() {
        if let Some(banner)=data["profileArtwork"]["profileBanner"].as_str().or_else(||data["profile"]["profileBanner"].as_str()) {
            node.set_sprite(&format!("{STATS}/TopInfo/CustomBanner/Image"),Some(banner));
        }
        if let Some(avatar)=data["profileArtwork"]["profileAvatar"].as_str().or_else(||data["profile"]["profileAvatar"].as_str()) {
            node.set_sprite(&format!("{STATS}/TopInfo/PlayerAvatar/Icon"),Some(avatar));
        }
        node.set_active(&format!("{STATS}/TopInfo/EmptyBanner"),data["profileArtwork"]["profileBanner"].as_str().is_none_or(str::is_empty));
        node.set_active(&format!("{STATS}/TopInfo/CustomBanner"),data["profileArtwork"]["profileBanner"].as_str().is_some_and(|s|!s.is_empty()));
        let veteran=data["profile"]["veteranRank"].as_f64().unwrap_or(0.0)>0.0;
        node.set_active(&format!("{STATS}/TopInfo/PlayerRank"),!veteran);
        node.set_active(&format!("{STATS}/TopInfo/VeteranPlayerRank"),veteran);
        if let Some(position)=node.find(&format!("{STATS}/TopInfo/PlayerRank")).map(|n|n.rt.pos){
            if let Some(rank)=node.find_mut(&format!("{STATS}/TopInfo/VeteranPlayerRank")){rank.rt.pos=position;}
        }
        node.set_active(&format!("{STATS}/Own"),true);
        node.set_active(&format!("{STATS}/Others"),false);
        node.set_active(&format!("{STATS}/Own/OnlineStatus"),false);
        node.set_active(&format!("{STATS}/Own/PublicSwapButton"),true);
        node.set_active(&format!("{STATS}/FollowerCount"),false);
        node.set_active("Canvas/PlayerStatsScreen/JewelBar/ContextSwitch",false);
        node.set_active("Canvas/PlayerStatsScreen/JewelBar/ContextSwitchButton",false);

        if initializing{node.set_text(&format!("{STATS}/OverallStats/StatCategoryDropdown/Text"),"Overall Stats");}
        let medals=format!("{STATS}/Medals/MedalScroll/Viewport/MedalContainer");
        node.set_active(&format!("{medals}/MedalPrefab"),false);
        if initializing {
            if let (Some(template),Some(group))=(Node::load(&root.join("ui/prefabs/ProfileMedal.json")),node.find_mut(&format!("{medals}/SpecialMedals"))) {
                group.children.retain(|c|!c.name.ends_with("Badge"));
                for (name,count) in data["medals"]["SpecialMedals"].as_object().into_iter().flatten().filter(|(name,_)|name.ends_with("Badge")) {
                    let mut medal=template.clone();medal.name=name.clone();medal.active=true;
                    medal.set_sprite("",Some(name));medal.set_text("Text",&number(count.as_f64().unwrap_or(0.0)));
                    group.children.push(medal);
                }
            }
        }
        for group in ["SPMedals","CoopMedals","SpecialMedals"] {
            if let Some(row)=node.find_mut(&format!("{medals}/{group}")) {
                row.active=true;
                for medal in &mut row.children {
                    let count=data["medals"][group][&medal.name].as_f64().unwrap_or(0.0) as u64;
                    medal.active=count>0;
                    medal.set_text("Text",&count.to_string());
                }
            }
        }
        node.set_active(&format!("{STATS}/Medals/FoundNothingTxt"),data["medals"].as_object().is_some_and(|groups|groups.values().all(|g|g.as_object().is_none_or(|m|m.is_empty()))));
        node.hide_named(&["FriendCode","OnlineStatus","FollowButton","Accolades"]);
        node.set_text(&format!("{STATS}/TopInfo/PlayerName"),data["displayName"].as_str().unwrap_or(""));
        for (name,key) in [("PlayerRank","rank"),("VeteranPlayerRank","veteranRank")] {
            let base=format!("{STATS}/TopInfo/{name}/XpBar");
            if let Some(rank)=data["profile"][key].as_f64() {
                node.set_text(&format!("{base}/LvlStar/LvlTxt"),&number(rank));
                if rank>=1000.0{if let Some(text)=node.find_mut(&format!("{base}/LvlStar/LvlTxt")).and_then(|n|n.txt.as_mut()){text.auto=true;text.min=12.0;text.wrap=false;}}
                if name=="VeteranPlayerRank" {node.set_active(&format!("{STATS}/TopInfo/{name}"),rank>0.0);}
            }
            let progress=if name=="VeteranPlayerRank"{&data["rankProgress"]["veteran"]}else{&data["rankProgress"]};
            bind_xp(node,&base,progress);
        }
        let content=format!("{STATS}/OverallStats/ConfigureStats/PlayerStats/Stats");
        if initializing {
            let template=Node::load(&root.join("ui/prefabs/ProfileStat.json"))
                .or_else(||node.find(&format!("{STATS}/OverallStats/PublicStats/PlayerStats/Stats/PublicStat")).cloned());
            if let (Some(template),Some(list))=(template,node.find_mut(&content)) {
                list.children.clear();
                for (key,label) in [("gamesPlayed","Games Played"),("gamesLost","Games Lost"),("totalTowersPlaced","Monkeys Placed"),
                    ("bloonsPopped","Bloons Popped"),("camosPopped","Camo Bloons Popped"),
                    ("leadPopped","Lead Bloons Popped"),("purplesPopped","Purple Bloons Popped"),
                    ("regrowPopped","Regrow Bloons Popped"),("ceramicsPopped","Ceramic Bloons Popped"),
                    ("moabsPopped","MOABs Popped"),("bfbsPopped","BFBs Popped"),("zomgsPopped","ZOMGs Popped"),
                    ("ddtsPopped","DDTs Popped"),("badsPopped","BADs Popped"),("bossesPopped","Bosses Popped"),
                    ("bloonsLeaked","Bloons Leaked"),("cashEarned","Cash Generated"),
                    ("totalAbilitiesActivated","Abilities Used"),("totalPowersActivated","Powers Used"),
                    ("instaMonkeysUsed","Insta Monkeys Used"),("damageDoneToBosses","Damage Done to Bosses")] {
                    let Some(value)=data["stats"][key].as_f64() else {continue};
                    let mut row=template.clone();row.name=format!("AccountStat_{key}");row.active=true;
                    row.set_text("Name",label);row.set_text("Value",&number(value));
                    row.set_active("Toggle/Tick",data["visibleStats"][key]==true);
                    list.children.push(row);
                }
                for (key,label) in [("completedGame","Games Won"),("highestSeenRound","Highest Round"),
                    ("highestSeenRoundCurrentVersion","Highest Round This Version"),("totalCompletedOdysseys","Odysseys Completed"),
                    ("lifetimeTrophies","Lifetime Trophies"),("goldenBloonsPopped","Golden Bloons Popped"),
                    ("monkeyTeamsWins","Monkey Teams Wins")] {
                    let Some(value)=data["profile"][key].as_f64() else {continue};
                    let mut row=template.clone();row.name=format!("AccountStat_{key}");row.active=true;
                    row.set_text("Name",label);row.set_text("Value",&number(value));
                    row.set_active("Toggle/Tick",data["visibleStats"][key]==true);list.children.push(row);
                }
            }
        }
        if initializing {
            let rows=node.find(&content).map(|n|n.children.clone()).unwrap_or_default();
            let visible:Vec<Node>=rows.iter().filter(|r|data["visibleStats"][r.name.trim_start_matches("AccountStat_")]==true).cloned().map(|mut r|{r.set_active("Toggle",false);r}).collect();
            node.set_active(&format!("{STATS}/OverallStats/SharedStats"),true);
            node.set_text(&format!("{STATS}/OverallStats/SharedStats"),&format!("Shared {}/7",visible.len()));
            if let Some(public)=node.find_mut(&format!("{STATS}/OverallStats/PublicStats/PlayerStats/Stats")){public.children=visible;}
        }
        for (group,path) in [("heroes","HeroContainer/HeroContent"),("towers","TowerContainer/TowerContent")] {
            let content=format!("{STATS}/TopTowers/{path}");
            if initializing {
                if let (Some(template),Some(list))=(Node::load(&root.join("ui/prefabs/ProfileTopTower.json")),node.find_mut(&content)) {
                    list.children.clear();
                    for (i,item) in data["top"][group].as_array().into_iter().flatten().enumerate() {
                        let mut card=template.clone();card.name=format!("AccountTop{i}");
                        card.set_sprite("Icon",item["icon"].as_str());
                        if let Some(icon)=card.find_mut("Icon").and_then(|n|n.img.as_mut()){icon.color.a=1.0;}
                        card.set_text("Count",&number(item["count"].as_f64().unwrap_or(0.0)));
                        list.children.push(card);
                    }
                }
            }
        }
        node.set_active(&format!("{STATS}/TopTowers/HeroContainer/FoundNothingTxt"),data["top"]["heroes"].as_array().is_some_and(|a|a.is_empty()));
        bind_named_monkeys(node,data,initializing);
        return;
    }
    const PLAYER: &str = "MainMenuCanvas/MainMenu/PlayerInfo";
    const TROPHY_BASE:&str="Canvas/TrophyStoreScreen/Container";
    if let Some(quantity)=data["profile"]["trophies"].as_f64(){node.set_text(&format!("{TROPHY_BASE}/TrophiesEarned/TrophiesQtyTxt"),&number(quantity));}
    node.set_active("MainMenuCanvas/MainMenu/BottomButtonGroup/Knowledge/KnowledgeAnim/Button/MKindicator",data["profile"]["knowledgeDisabled"]==true);
    node.set_active("Canvas/KnowledgeMain/EnableDisableAllMK/MKOnOffToggle/BtnOn",data["profile"]["knowledgeDisabled"]!=true);
    if let (Some(owned),Some(total))=(data["heroCollection"]["owned"].as_u64(),data["heroCollection"]["total"].as_u64()) {
        if total>0 {node.set_text("Canvas/HeroDetailsScreen/LeftSideGroup/HeroCount",&format!("{owned}/{total}"));}
    }
    for (path,key) in [("Canvas/ForegroundScreen/CashInfo/Layout/CashInfo/Text","monkeyMoney"),
        ("Canvas/ForegroundScreen/Knowledge/Layout/Knowledge/PointsTxt","knowledgePoints")] {
        if let Some(value)=data["profile"][key].as_f64(){node.set_text(path,&number(value));}
    }
    if let Some(avatar)=data["profileArtwork"]["profileAvatar"].as_str().or_else(||data["profile"]["profileAvatar"].as_str()){node.set_sprite(&format!("{PLAYER}/PlayerAvatar/Image"),Some(avatar));}
    if let Some(name) = data["displayName"].as_str() {
        node.set_text(&format!("{PLAYER}/Name"), name);
    }
    let veteran=data["profile"]["veteranRank"].as_f64().unwrap_or(0.0)>0.0;
    let rank=&data["profile"][if veteran{"veteranRank"}else{"rank"}];
    if let Some(rank)=rank.as_f64(){
        node.set_text(&format!("{PLAYER}/LvlTxt"),&number(rank));
        if let Some(text)=node.find_mut(&format!("{PLAYER}/LvlTxt")).and_then(|n|n.txt.as_mut()){
            text.auto=true;text.min=12.0;text.wrap=false;
        }
    }
    for (path,normal,vet) in [("LvlStar","LvlHolder","VeteranLvlHolder"),
        ("XpBar","BarfillContainer","BarfillVeteranContainer"),("XpBar/Mask/InnerBar","BarfillBlue","BarfillPurple")]{
        node.set_sprite(&format!("{PLAYER}/{path}"),Some(if veteran{vet}else{normal}));
    }
    bind_xp(node,&format!("{PLAYER}/XpBar"),if veteran{&data["rankProgress"]["veteran"]}else{&data["rankProgress"]});
}

const NAMED:&str="Canvas/PlayerStatsScreen/Stats/NamedMonkeysMenu";
fn bind_named_monkeys(node:&mut Node,data:&serde_json::Value,initializing:bool){
    let items=data["namedMonkeys"].as_array();
    node.set_active(NAMED,items.is_some_and(|v|!v.is_empty()));
    if !initializing{return;}
    node.set_active(&format!("{NAMED}/OpenPurchaseNamedMonkeysBtn"),false);
    node.set_active(&format!("{NAMED}/RenameMonkeyBtn"),false);
    node.set_text(&format!("{NAMED}/Banner/Label"),"Named Monkeys");
    let path=format!("{NAMED}/OptionScroll/Content");
    if let Some(list)=node.find_mut(&path){
        let Some(template)=list.children.first().cloned() else{return;};
        list.children.clear();
        for (i,item) in std::iter::once(None).chain(items.into_iter().flatten().map(Some)).enumerate(){
            let mut row=template.clone();row.name=format!("Named{i}");row.active=true;
            row.set_text("NamedMonkeyPanel/Text",item.and_then(|v|v["name"].as_str()).unwrap_or("All"));
            row.set_sprite("NamedMonkeyPanel/Container/Icon",Some(item.and_then(|v|v["icon"].as_str()).unwrap_or("StatsIconAll")));
            row.set_active("NamedMonkeyPanel/Selected",i==0);
            list.children.push(row);
        }
    }
}
pub(super) fn named_monkey(index:usize)->Option<serde_json::Value>{
    let state=PREVIEW.get()?.try_lock().ok()?;
    state.data.as_ref()?["namedMonkeys"].as_array()?.get(index).cloned()
}
pub(super) fn daily_reward()->Option<(u64,String,serde_json::Value)>{
    let state=PREVIEW.get()?.try_lock().ok()?;let data=state.data.as_ref()?;
    Some((data["profile"]["dailyRewardIndex"].as_u64().unwrap_or(0),
        data["profile"]["nextDailyRewardDateTime"].as_str().unwrap_or("").to_owned(),
        serde_json::json!([data["localProfile"],data["displayName"],data["downloadedAt"]])))
}
pub(super) fn stats_category(category:&str)->Option<serde_json::Value>{
    let state=PREVIEW.get()?.try_lock().ok()?;let data=state.data.as_ref()?;
    if category=="Main Game"&&data["localProfile"]==true{return Some(data["stats"].clone());}
    data["statsCategories"].get(category).cloned()
}
pub(super) fn saved_game(map:&str)->Option<serde_json::Value>{
    let state=PREVIEW.get()?.try_lock().ok()?;let data=state.data.as_ref()?;
    data["savedGames"].as_array()?.iter().find(|v|v["map"].as_str()==Some(map)).cloned()
}
/// Bind only real cached friend records; never show the scene's sample avatar/round 85.
/// The friends service adapter has not yet supplied this optional section.
pub(super) fn bind_map_friends(node:&mut Node,path:&str,map:&str,hidden:bool,details:bool){
    let state=PREVIEW.get().and_then(|s|s.try_lock().ok());
    let records=state.as_ref().and_then(|s|s.data.as_ref())
        .and_then(|d|d["friends"]["mapRecords"][map].as_array());
    let mut records:Vec<_>=records.into_iter().flatten()
        .filter(|r|r["round"].as_u64().is_some_and(|n|n>0)&&r["displayName"].as_str().is_some()&&r["avatar"].as_str().is_some())
        .collect();
    records.sort_by_key(|r|std::cmp::Reverse(r["round"].as_u64().unwrap_or(0)));
    node.set_active(path,!hidden&&!records.is_empty());
    if hidden||records.is_empty(){return;}
    let bind=|node:&mut Node,path:&str,record:&serde_json::Value,index:usize|{
        node.set_sprite(&format!("{path}/Mask/Icon"),record["avatar"].as_str());
        node.set_text(&format!("{path}/Difficulty/BestRound"),&record["round"].as_u64().unwrap_or(0).to_string());
        node.set_text(&format!("{path}/Username"),&format!("{}. {}",index+1,record["displayName"].as_str().unwrap_or("")));
        let medal=match record["difficulty"].as_str(){Some("Easy")=>Some("MedalBronze"),Some("Medium")=>Some("MedalSilver"),Some("Hard")=>Some("MedalGold"),_=>None};
        node.set_sprite(&format!("{path}/Difficulty"),medal);
    };
    if details{
        for i in 0..5{
            let row=format!("{path}/{}",if i==0{"FriendRank".into()}else{format!("FriendRank ({i})")});
            node.set_active(&row,records.get(i).is_some());
            if let Some(record)=records.get(i){bind(node,&row,record,i);}
        }
    }else{bind(node,path,records[0],0);}
}

pub(crate) fn helper_project(root:&Path)->std::path::PathBuf {
    let beside_data=root.parent().unwrap_or(root).to_owned();
    [Some(beside_data.clone()),std::env::current_dir().ok(),Path::new(env!("CARGO_MANIFEST_DIR")).parent().map(Path::to_owned)]
        .into_iter().flatten().find(|p|p.join("tools/assets/cloud_account.py").is_file()).unwrap_or(beside_data)
}

fn loader_note(project:&Path,note:&str){
    use std::io::Write;
    if let Ok(mut file)=std::fs::OpenOptions::new().create(true).append(true).open(project.join("local_extracted/profile-loader.log")){
        let _=writeln!(file,"{note}");
    }
}

fn read_preview(project:&Path,mode:&str)->Option<serde_json::Value> {
    let bundled=project.join("tools/.venv/Scripts/python.exe");
    let runtimes=if bundled.is_file(){vec![bundled,std::path::PathBuf::from("python")]}else{vec![std::path::PathBuf::from("python")]};
    for runtime in runtimes {
        let mut command=std::process::Command::new(&runtime);
        command.current_dir(project).arg("-I").arg(project.join("tools/assets/cloud_account.py")).arg(mode);
        #[cfg(windows)] {use std::os::windows::process::CommandExt;command.creation_flags(0x08000000);}
        let output=match command.output(){Ok(output)=>output,Err(error)=>{loader_note(project,&format!("{mode}: runtime={} spawn={:?}",runtime.display(),error.kind()));continue;}};
        loader_note(project,&format!("{mode}: runtime={} exit={} stdout_bytes={} stderr_bytes={}",runtime.display(),output.status,output.stdout.len(),output.stderr.len()));
        if !output.status.success() || output.stdout.len()>4*1024*1024{if mode!="--preview"{break;}continue;}
        let data=match serde_json::from_slice::<serde_json::Value>(&output.stdout){Ok(data)=>data,Err(error)=>{loader_note(project,&format!("{mode}: invalid JSON at {}:{} ({:?})",error.line(),error.column(),error.classify()));continue;}};
        if data["readOnly"]==true && data["uploadsEnabled"]==false{return Some(data);}
        // Refresh can make a network request: do not repeat it through a second runtime.
        if mode!="--preview"{break;}
    }
    None
}

pub(crate) fn number(value:f64)->String {
    let raw=format!("{value:.0}");
    let digits=raw.strip_prefix('-').unwrap_or(&raw);
    let mut result=if raw.starts_with('-'){"-".to_owned()}else{String::new()};
    for (i,c) in digits.chars().enumerate(){if i>0 && (digits.len()-i)%3==0 && c.is_ascii_digit(){result.push(',');}result.push(c);}
    result
}

fn bind_xp(node:&mut Node,base:&str,progress:&serde_json::Value) {
    let fraction=progress["fraction"].as_f64();
    node.set_active(&format!("{base}/Text"),fraction.is_some());
    node.set_active(&format!("{base}/Mask/InnerBar"),fraction.is_some());
    let Some(fraction)=fraction else{return};
    let text=if progress["max"]==true {progress["maxLabel"].as_str().unwrap_or("Max Level").to_owned()}
        else{format!("{}/{}",number(progress["current"].as_f64().unwrap_or(0.0)),number(progress["required"].as_f64().unwrap_or(0.0)))};
    node.set_text(&format!("{base}/Text"),&text);
    if let Some(bar)=node.find_mut(&format!("{base}/Mask/InnerBar")) {
        if let Some(image)=bar.img.as_mut().filter(|i|i.kind==3){image.fill_amount=fraction.clamp(0.0,1.0) as f32;}
        else{bar.rt.amax.x=fraction.clamp(0.0,1.0) as f32;}
    }
}

pub fn daily_completed(id:&str)->bool {
    PREVIEW.get().and_then(|p|p.try_lock().ok()).and_then(|p|p.data.as_ref().map(|d|
        d["profile"]["wonDailyChallengeIds"].as_array().is_some_and(|ids|ids.iter().any(|v|v.as_str()==Some(id)))))
        .unwrap_or(false)
}

pub(super) fn event_progress(kind:&str,id:&str)->Option<serde_json::Value>{
    let state=PREVIEW.get()?.try_lock().ok()?;let data=&state.data.as_ref()?["eventProgress"];
    let (list,key)=match kind{"raceEvent"=>("races","id"),"bossBloon"|"bossElite"=>("bosses","eventId"),_=>return None};
    data[list].as_array()?.iter().find(|r|r[key].as_str()==Some(id)).cloned()
}

/// (acquiredKnowledge, disabledKnowledge, knowledgePoints, downloadedAt) for KnowledgeSkillTree states.
pub(super) fn knowledge_profile()->Option<(Vec<String>,Vec<String>,i64,f64)> {
    let state=PREVIEW.get()?.try_lock().ok()?;let data=state.data.as_ref()?;let profile=&data["profile"];
    let ids=|key:&str|profile[key].as_array().into_iter().flatten().filter_map(|v|v.as_str().map(str::to_owned)).collect();
    Some((ids("acquiredKnowledge"),ids("disabledKnowledge"),profile["knowledgePoints"].as_f64().unwrap_or(0.0) as i64,data["downloadedAt"].as_f64().unwrap_or(0.0)))
}

/// Per Trophy Store catalogue entry (trophy_items.json order): [owned, enabled, favourite, canEnable],
/// with the snapshot's downloadedAt. The Trophy Store page binds these (TrophyItemPanel.Bind).
pub(super) fn trophy_account()->Option<(Vec<[bool;4]>,f64)> {
    let state=PREVIEW.get()?.try_lock().ok()?;let data=state.data.as_ref()?;
    let rows=data["trophyOwnership"].as_array()?.iter().map(|r|[r["owned"]==true,r["enabled"]==true,r["favorite"]==true,r["canEnable"]==true]).collect();
    Some((rows,data["downloadedAt"].as_f64().unwrap_or(0.0)))
}

/// (friendScreenShowCode, toggleFriendRequests, Friends_List friendsCode, downloadedAt). The code comes
/// only from the friends service (it differs from the account short code); absent offline, the game
/// shows its empty default too.
pub(super) fn friends_profile()->Option<(bool,bool,String,f64)> {
    let state=PREVIEW.get()?.try_lock().ok()?;let data=state.data.as_ref()?;let profile=&data["profile"];
    Some((profile["friendScreenShowCode"]!=false,profile["toggleFriendRequests"]==true,
        data["friends"]["code"].as_str().unwrap_or("").to_owned(),data["downloadedAt"].as_f64().unwrap_or(0.0)))
}

/// MonkeyButton.Init inputs: towerXp, acquiredUpgrades, viewedUpgrades and unlockedTowers.
pub(crate) struct MonkeyAccount {
    pub xp:std::collections::HashMap<String,f64>,
    pub acquired:std::collections::HashSet<String>,
    pub viewed:std::collections::HashSet<String>,
    pub unlocked:std::collections::HashSet<String>,
    /// towerUnlockProgresses, selectedTowerForUnlockProgression and rank (tower gift).
    pub gift_progress:std::collections::HashMap<String,f64>,
    pub gift_selected:String,
    pub rank:f64,
}
pub(crate) fn monkey_account()->Option<MonkeyAccount> {
    let state=PREVIEW.get()?.try_lock().ok()?;let profile=&state.data.as_ref()?["profile"];
    let set=|key:&str|profile[key].as_array().into_iter().flatten().filter_map(|v|v.as_str().map(str::to_owned)).collect();
    Some(MonkeyAccount {xp:profile["towerXp"].as_object().into_iter().flatten().filter_map(|(k,v)|Some((k.clone(),v.as_f64()?))).collect(),
        acquired:set("acquiredUpgrades"),viewed:set("viewedUpgrades"),unlocked:set("unlockedTowers"),
        gift_progress:profile["towerUnlockProgresses"].as_object().into_iter().flatten().filter_map(|(k,v)|Some((k.clone(),v.as_f64()?))).collect(),
        gift_selected:profile["selectedTowerForUnlockProgression"].as_str().unwrap_or("").to_owned(),
        rank:profile["rank"].as_f64().unwrap_or(1.0)})
}

/// PowersSelectScreen inputs: powersData (PowerData quantity/isNew), seenPowers,
/// powersProSaveData.dataByPower, achievementsClaimed and the Monkey Knowledge used by GetModdedPowers.
pub(crate) struct PowerAccount {
    pub data:std::collections::HashMap<String,(f64,bool)>,
    pub seen:std::collections::HashSet<String>,
    /// PowersProPowerSaveData: xp, unlockedTier, seenUnlockPro, seenUnlockTier.
    pub pro:std::collections::HashMap<String,(f64,f64,bool,f64)>,
    pub claimed:std::collections::HashSet<i64>,
    pub knowledge:std::collections::HashSet<String>,
    pub knowledge_disabled:bool,
}
pub(crate) fn power_account()->Option<PowerAccount> {
    let state=PREVIEW.get()?.try_lock().ok()?;let profile=&state.data.as_ref()?["profile"];
    let set=|key:&str|profile[key].as_array().into_iter().flatten().filter_map(|v|v.as_str().map(str::to_owned)).collect();
    let num=|v:&serde_json::Value|v.as_f64().or_else(||v.as_bool().map(f64::from)).unwrap_or(0.0);
    Some(PowerAccount {
        data:profile["powersData"].as_object().into_iter().flatten()
            .map(|(k,v)|(k.clone(),(num(&v["quantity"]),v["isNew"]==true))).collect(),
        seen:set("seenPowers"),
        pro:profile["powersProSaveData"]["dataByPower"].as_object().into_iter().flatten()
            .map(|(k,v)|(k.clone(),(num(&v["xp"]),num(&v["unlockedTier"]),v["seenUnlockPro"]==true,num(&v["seenUnlockTier"])))).collect(),
        claimed:profile["achievementsClaimed"].as_array().into_iter().flatten().filter_map(|v|v.as_f64().map(|v|v as i64)).collect(),
        knowledge:set("acquiredKnowledge"),
        knowledge_disabled:profile["knowledgeDisabled"]==true,
    })
}

pub fn trophy_favorite(index:usize)->bool {
    PREVIEW.get().and_then(|p|p.try_lock().ok()).and_then(|p|p.data.as_ref().map(|d|d["trophyOwnership"][index]["favorite"]==true)).unwrap_or(false)
}

/// Returns MapButton.Init's border level (borderAnimator integer): 0 basic, 1 bronze, 2 silver,
/// 3 gold, 4 black CHIMPS; None without account data.
pub fn bind_map_mode(node:&mut Node,path:&str,id:&str,coop:bool)->Option<u8> {
    if coop {
        if let Some(divisions)=node.find_mut(&format!("{path}/MapMask/CoopMapDivisions")){
            for division in &mut divisions.children{division.active=division.name=="FreeForAll";}
        }
    }
    let state=PREVIEW.get().and_then(|p|p.try_lock().ok())?;
    let data=state.data.as_ref()?;
    let map=&data["profile"]["mapInfo"]["maps"][id]["difficult"];
    let border=map_border(map,if coop{"coopModes"}else{"modes"});
    node.set_sprite(&format!("{path}/Bg"),Some(border));
    node.set_active(&format!("{path}/EmptyMedalContainer"),true);
    if let Some(tile)=node.find_mut(path){tile.hide_named(&["Stars"]);}
    let level=match border {"MapPanel"=>0,"MainBGPanelBronze"=>1,"MainBGPanelSilver"=>2,"MainBGPanelYellow"=>3,_=>4};
    // MapButton.Init: chimpsHematiteSparkles for a black-CHIMPS border, chimpsGoldSparkles for gold.
    node.set_active(&format!("{path}/ChimpsSparklesFxHematite"),level==4);
    node.set_active(&format!("{path}/ChimpsGlintsFxGold"),level==3);
    for (group,key) in [("Medals","modes"),("CoopMedals","coopModes")] {
        node.set_active(&format!("{path}/{group}"),group==if coop{"CoopMedals"}else{"Medals"});
        for difficulty in ["Easy","Medium","Hard"] {
            if let Some(row)=node.find_mut(&format!("{path}/{group}/{difficulty}")) {
                row.active=true;
                for medal in &mut row.children {
                    let mode=if medal.name=="SpecialClicks" {"Clicks"}else{&medal.name};
                    let packed=map[difficulty][key][mode].as_u64().unwrap_or(0);
                    let completed=((packed>>20)&65535)+((packed>>36)&65535)>0;
                    medal.active=completed && match medal.name.as_str(){"Clicks"=>packed&8==0,"SpecialClicks"=>packed&8!=0,_=>true};
                }
            }
        }
    }
    Some(level)
}

// MapButton's five native background assets; completion is cumulative by difficulty.
fn map_border(map:&serde_json::Value,mode_key:&str)->&'static str {
    let won=|difficulty:&str,mode:&str| {
        let p=map[difficulty][mode_key][mode].as_u64().unwrap_or(0);
        ((p>>20)&65535)+((p>>36)&65535)>0
    };
    if !["Standard","PrimaryOnly","Deflation"].iter().all(|m|won("Easy",m)){return "MapPanel";}
    if !["Standard","MilitaryOnly","Apopalypse","Reverse"].iter().all(|m|won("Medium",m)){return "MainBGPanelBronze";}
    if !["Standard","MagicOnly","AlternateBloonsRounds","DoubleMoabHealth","HalfCash","Impoppable","Clicks"].iter().all(|m|won("Hard",m)){return "MainBGPanelSilver";}
    if map["Hard"][mode_key]["Clicks"].as_u64().unwrap_or(0)&8!=0{"MainBgPanelHematite"}else{"MainBGPanelYellow"}
}
#[cfg(test)]
mod map_border_tests {
    use super::*;
    #[test]
    fn cumulative_medals_select_native_borders_and_black_chimps_is_distinct(){
        let mut map=serde_json::json!({});
        assert_eq!(map_border(&map,"modes"),"MapPanel");
        for (difficulty,modes,border) in [
            ("Easy",vec!["Standard","PrimaryOnly","Deflation"],"MainBGPanelBronze"),
            ("Medium",vec!["Standard","MilitaryOnly","Apopalypse","Reverse"],"MainBGPanelSilver"),
            ("Hard",vec!["Standard","MagicOnly","AlternateBloonsRounds","DoubleMoabHealth","HalfCash","Impoppable","Clicks"],"MainBGPanelYellow")
        ] {
            for mode in modes {map[difficulty]["modes"][mode]=serde_json::json!(1u64<<20);}
            assert_eq!(map_border(&map,"modes"),border);
        }
        map["Hard"]["modes"]["Clicks"]=serde_json::json!((1u64<<20)|8);
        assert_eq!(map_border(&map,"modes"),"MainBgPanelHematite");
        assert_eq!(map_border(&map,"coopModes"),"MapPanel");
    }
}

pub(crate) fn player_name()->Option<String>{let state=PREVIEW.get()?.try_lock().ok()?;let data=state.data.as_ref()?;if data["localProfile"]==true{return None;}Some(data["displayName"].as_str()?.trim()).filter(|name|!name.is_empty()).map(str::to_owned)}
pub(crate) fn is_local()->bool{PREVIEW.get().and_then(|s|s.try_lock().ok()).and_then(|s|s.data.as_ref().map(|d|d["localProfile"]==true)).unwrap_or(true)}
pub(crate) fn refresh_local(root:&Path){if is_local(){if let Ok(mut state)=preview_state(root).lock(){state.data=Some(crate::local_profile::preview(root));state.bound.clear();}}}

/// Publish a verified login result and invalidate any previous account read in flight.
pub(crate) fn signed_in(root:&Path,data:serde_json::Value) {
    if data["readOnly"]!=true || data["uploadsEnabled"]!=false{return;}
    if let Ok(mut state)=preview_state(root).lock() {
        let (_,rx)=mpsc::channel();state.rx=rx;state.finished=true;
        state.data=Some(if data["displayName"].as_str().is_none_or(str::is_empty){crate::local_profile::preview(root)}else{data});state.bound.clear();
    }
}
