//! The installed Jukebox scene and track rows (JukeBoxScreen, TrackItem); playlist changes stay
//! local. Our own addition (DEVIATIONS.md): a "My Music" button that opens the custom music
//! folder; the audio files in it join the jukebox as free tracks.
use crate::uitree::Node;
use macroquad::prelude::*;
use serde_json::Value;
use std::{cell::RefCell,collections::{HashMap,HashSet},path::{Path,PathBuf}};
/// Availability uses the MusicTrackData pointer's Trophy Store wallet, exported from
/// the original data. Enabled/favourite state does not determine ownership.
fn available(track:&Value,owned:&HashSet<String>)->bool {
    track["free"]==true || track["wallet"].as_str().is_some_and(|id|!id.is_empty()&&owned.contains(id))
}
fn queue_from_saved(tracks:&[Value],saved:&Value,owned:&HashSet<String>)->Vec<usize> {
    let ids:Vec<usize>=if let Some(ids)=saved["jukebox"].as_array(){
        ids.iter().filter_map(|id|tracks.iter().position(|v|v["id"]==*id&&available(v,owned))).collect()
    }else{tracks.iter().enumerate().filter(|(_,v)|available(v,owned)&&v["custom"]!=true).map(|(i,_)|i).collect()};
    let mut seen=HashSet::new();ids.into_iter().filter(|i|seen.insert(*i)).collect()
}
/// Track ids of the player's own files: this, then the file name.
pub(crate) const CUSTOM:&str="custom:";
/// Audio files the custom music folder can hold.
pub(crate) const CUSTOM_EXT:[&str;4]=["mp3","wav","flac","ogg"];
/// The custom music folder: "Custom Music" beside the data folder (made when opened).
pub(crate) fn custom_dir(root:&Path)->PathBuf{root.parent().unwrap_or(root).join("Custom Music")}
/// The folder's audio files, by name, as jukebox tracks (always free; named after the file).
fn custom_tracks(root:&Path)->Vec<Value>{
    let mut files:Vec<String>=std::fs::read_dir(custom_dir(root)).into_iter().flatten().flatten()
        .filter(|e|e.path().is_file()&&e.path().extension().and_then(|x|x.to_str()).is_some_and(|x|CUSTOM_EXT.contains(&x.to_ascii_lowercase().as_str())))
        .filter_map(|e|e.file_name().into_string().ok()).collect();
    files.sort_by_key(|f|f.to_lowercase());
    files.into_iter().map(|f|{
        let stem=Path::new(&f).file_stem().and_then(|s|s.to_str()).unwrap_or(&f).to_owned();
        serde_json::json!({"id":format!("{CUSTOM}{f}"),"loc":stem,"free":true,"jukebox":true,"custom":true})
    }).collect()
}
/// The game's jukebox tracks, then the custom ones.
fn all_tracks(root:&Path)->Option<Vec<Value>>{
    let tracks:Vec<Value>=serde_json::from_slice(&std::fs::read(root.join("ui/runtime/jukebox.json")).ok()?).ok()?;
    let mut tracks:Vec<_>=tracks.into_iter().filter(|t|t["jukebox"]==true).collect();
    tracks.extend(custom_tracks(root));
    Some(tracks)
}
pub(crate) fn saved_playlist(root:&Path,owned:&HashSet<String>)->Option<Vec<(String,String)>> {
    let tracks=all_tracks(root)?;
    let saved:Value=std::fs::read(root.join("settings.json")).ok().and_then(|b|serde_json::from_slice(&b).ok()).unwrap_or_default();
    Some(queue_from_saved(&tracks,&saved,owned).into_iter().filter_map(|i|Some((tracks[i]["id"].as_str()?.to_owned(),tracks[i]["loc"].as_str().unwrap_or("").to_owned()))).collect())
}
fn move_track(queue:&mut [usize],selected:usize,delta:isize)->bool {
    let Some(from)=queue.iter().position(|&i|i==selected) else{return false;};
    let to=from as isize+delta;
    if !(0..queue.len() as isize).contains(&to){return false;}
    queue.swap(from,to as usize);true
}
thread_local! {
    /// What the music player is doing, for the screen: the playing track's id and how far
    /// through it is (0..1). Audio::music sets it each frame.
    static NOW_PLAYING:RefCell<Option<(String,f32)>>=const{RefCell::new(None)};
}
pub(crate) fn publish(track:Option<(&str,f32)>){NOW_PLAYING.with(|n|*n.borrow_mut()=track.map(|(id,t)|(id.to_owned(),t)));}
const BASE:&str="Canvas/JukeboxScreen";
const EQ:&str="Canvas/JukeboxScreen/TrackPlayer/InnerPanel/Equaliser";
const FOLDER_BTN:&str="AvailableTracks/OpenFolderBtn";
#[derive(Clone)]
pub struct Change {pub tracks:Vec<(String,String)>,pub selected:usize,pub playing:bool,
    /// A Songs track being previewed (SelectedTrackClicked on a track not in the playlist).
    pub preview:Option<(String,String)>}
pub(super) struct Browser {
    tracks:Vec<Value>,queue:Vec<usize>,selected:usize,playing:bool,
    owned:HashSet<String>,account:Option<(bool,u64)>,saved:Value,
    template:Node,labels:HashMap<String,String>,pub pending:Option<Change>,
    root:PathBuf,runtime:Value,opened:f64,scanned:f64,
    /// EQAnimation re-pointed at the selected row's TrackEQ (laid out as the player's bars).
    row_eq:Option<Value>,
}
/// EQAnimation with its bindings moved from the player's Equaliser to `to`.
fn eq_clips(runtime:&Value,to:&str)->Option<Value>{
    let mut a=runtime["animators"].as_array()?.iter().find(|a|a["controller"]=="EQAnimation")?.clone();
    for c in a["clips"].as_array_mut()?{
        for b in c["bindings"].as_array_mut().into_iter().flatten(){
            if let Some(p)=b["path"].as_str().and_then(|p|p.strip_prefix(EQ)).map(str::to_owned){b["path"]=Value::String(format!("{to}{p}"));}
        }
    }
    Some(serde_json::json!({"animators":[a]}))
}
impl Browser {
    pub fn load(root:&Path,node:&mut Node,labels:HashMap<String,String>)->Option<Self>{
        let tracks=all_tracks(root)?;
        let saved:Value=std::fs::read(root.join("settings.json")).ok().and_then(|b|serde_json::from_slice(&b).ok()).unwrap_or_default();
        let ownership=super::account_preview::music_ownership(None);
        let (owned,account)=ownership.map(|(ids,key)|(ids,Some(key))).unwrap_or_default();
        let queue=queue_from_saved(&tracks,&saved,&owned);
        if tracks.is_empty(){return None;}
        let template=Node::load(&root.join("ui/prefabs/JukeboxTrack.json"))?;
        let runtime=std::fs::read(root.join("ui/runtime/JukeboxUI.json")).ok().and_then(|b|serde_json::from_slice(&b).ok()).unwrap_or_default();
        let playing_now=NOW_PLAYING.with(|n|n.borrow().as_ref().map(|(id,_)|id.clone()));
        let selected=playing_now.and_then(|id|tracks.iter().position(|t|t["id"]==id.as_str())).or(queue.first().copied()).unwrap_or(0);
        let mut b=Self{tracks,selected,playing:!queue.is_empty(),queue,owned,account,saved,template,labels,pending:None,
            root:root.to_owned(),runtime,opened:get_time(),scanned:get_time(),row_eq:None};
        node.set_active(&format!("{BASE}/BlockClicks"),false);
        Self::add_folder_button(node);
        b.rebuild(node);Some(b)
    }
    /// Our "My Music" button: the Add All button's look, a little smaller, just left of it in
    /// the bar.
    fn add_folder_button(node:&mut Node){
        let Some(mut btn)=node.find(&format!("{BASE}/AvailableTracks/AddAllBtn")).cloned() else{return};
        btn.name="OpenFolderBtn".into();
        let k=0.86;
        let add=btn.rt.clone();
        btn.rt.scale=Vec2::splat(k);
        btn.rt.pos.x=add.pos.x-add.size.x*0.5-30.0-add.size.x*k*0.5;
        btn.set_text("AddAllBtn/Text","My Music");
        if let Some(side)=node.find_mut(&format!("{BASE}/AvailableTracks")){side.children.retain(|c|c.name!="OpenFolderBtn");side.children.push(btn);}
    }
    pub fn refresh(&mut self,node:&mut Node){
        if let Some((owned,key))=super::account_preview::music_ownership(self.account){
            self.owned=owned;self.account=Some(key);
            self.queue=queue_from_saved(&self.tracks,&self.saved,&self.owned);
            self.repair_selection();self.rebuild(node);
        }
        let now=get_time();
        // Files added to or removed from the custom music folder (checked every 2 s while open).
        if now-self.scanned>=2.0{
            self.scanned=now;
            if let Some(tracks)=all_tracks(&self.root).filter(|t|t.len()!=self.tracks.len()||t.iter().zip(&self.tracks).any(|(a,b)|a["id"]!=b["id"])){
                let queue:Vec<Value>=self.queue.iter().map(|&i|self.tracks[i]["id"].clone()).collect();
                let selected=self.tracks.get(self.selected).map(|t|t["id"].clone());
                self.tracks=tracks;
                self.queue=queue.iter().filter_map(|id|self.tracks.iter().position(|t|t["id"]==*id)).collect();
                self.selected=selected.and_then(|s|self.tracks.iter().position(|t|t["id"]==s)).unwrap_or(0);
                self.changed(node);
            }
        }
        // The track the player moved on to (UpdateCurrentTrackTxt), and its time (UpdateTrackTime).
        let now_playing=NOW_PLAYING.with(|n|n.borrow().clone());
        if let Some((id,_))=&now_playing{
            if self.playing&&self.tracks[self.selected]["id"]!=id.as_str(){
                if let Some(i)=self.tracks.iter().position(|t|t["id"]==id.as_str()){self.selected=i;self.rebuild(node);}
            }
        }
        let t=now_playing.filter(|_|self.playing).map_or(0.0,|(_,t)|t);
        if let Some(img)=node.find_mut(&format!("{BASE}/TrackPlayer/InnerPanel/FillBarContainer/FillBar")).and_then(|n|n.img.as_mut()){img.fill_amount=t;}
        // EQAnimation: EQPlaying (looping) while playing, else EQIdle; the selected row's too.
        let eq=self.runtime["animators"].as_array().into_iter().flatten().filter(|a|a["controller"]=="EQAnimation")
            .flat_map(|a|a["clips"].as_array().into_iter().flatten()).find(|c|c["name"]=="EQPlaying")
            .map_or(1.0,|c|(c["stop"].as_f64().unwrap_or(1.0)-c["start"].as_f64().unwrap_or(0.0)).max(1e-3));
        let (clip,at)=if self.playing{("EQPlaying",Some(((now-self.opened)%eq) as f32))}else{("EQIdle",None)};
        crate::frontend::apply_animation(node,&self.runtime,"EQAnimation",clip,at);
        if let Some(row)=&self.row_eq{crate::frontend::apply_animation(node,row,"EQAnimation",clip,at);}
    }
    fn repair_selection(&mut self){
        if self.selected>=self.tracks.len(){self.selected=self.queue.first().copied().unwrap_or(0);}
        if self.queue.is_empty()&&!self.previewing(){self.playing=false;}
    }
    /// The selected track is a Songs track (not in the playlist) the player owns.
    fn previewing(&self)->bool{
        self.selected<self.tracks.len()&&!self.queue.contains(&self.selected)&&available(&self.tracks[self.selected],&self.owned)
    }
    fn label(&self,i:usize)->&str{
        let key=self.tracks[i]["loc"].as_str().unwrap_or("");
        self.labels.get(key).map(String::as_str).unwrap_or(key)
    }
    /// The Songs list holds the tracks not in the playlist (AddTrackToPlaylist moves a track's
    /// item into the playlist; RemoveTrackFromPlayList moves it back, OrderTrackList restoring
    /// the order). TrackItem.SetBtns: owned tracks show Add (Songs) or Remove (playlist), the
    /// rest the Trophy Store lock. The selected track's item is highlighted and has the EQ.
    fn rebuild(&mut self,node:&mut Node){
        let songs:Vec<usize>=(0..self.tracks.len()).filter(|i|!self.queue.contains(i)).collect();
        let mut row_eq=None;
        for (side,ids) in [("AvailableTracks",songs),("Playlist",self.queue.clone())]{
            let mut rows=Vec::new();
            for i in ids{
                let mut row=self.template.clone();row.name=format!("Track{i}");row.active=true;
                row.set_text("TrackName",self.label(i));
                let owned=available(&self.tracks[i],&self.owned);
                row.set_active("TrophyLockBtn",!owned);
                row.set_active("AddButton",side=="AvailableTracks"&&owned);
                row.set_active("RemoveButton",side=="Playlist"&&owned);
                row.set_active("SelectTrackButton/BtnImg",i==self.selected);
                if i==self.selected{row_eq=eq_clips(&self.runtime,&format!("{BASE}/{side}/Scroll/Container/Content/Track{i}/TrackEQ"));}
                rows.push(row);
            }
            let empty=rows.is_empty();
            if let Some(content)=node.find_mut(&format!("{BASE}/{side}/Scroll/Container/Content")){content.children=rows;}
            node.set_active(&format!("{BASE}/{side}/FoundNothingTxt"),empty);
        }
        self.row_eq=row_eq;
        // SelectedTrackClicked: the move buttons only for a playlist track.
        node.set_active(&format!("{BASE}/Playlist/EditTrack"),self.queue.contains(&self.selected));
        node.set_text(&format!("{BASE}/TrackPlayer/InnerPanel/TrackName"),self.label(self.selected));
        node.set_active(&format!("{BASE}/TrackPlayer/PlayButton/PlayIcon"),!self.playing);
        node.set_active(&format!("{BASE}/TrackPlayer/PlayButton/PauseIcon"),self.playing);
    }
    fn changed(&mut self,node:&mut Node){
        self.repair_selection();
        self.saved["jukebox"]=serde_json::json!(self.queue.iter().map(|&i|&self.tracks[i]["id"]).collect::<Vec<_>>());
        let entry=|t:&Value|(t["id"].as_str().unwrap_or("").to_owned(),t["loc"].as_str().unwrap_or("").to_owned());
        self.pending=Some(Change{tracks:self.queue.iter().map(|&i|entry(&self.tracks[i])).collect(),
            selected:self.queue.iter().position(|&i|i==self.selected).unwrap_or(0),playing:self.playing,
            preview:self.previewing().then(||entry(&self.tracks[self.selected]))});
        self.rebuild(node);
    }
    pub fn click(&mut self,node:&mut Node,pointer:Vec2)->bool{
        let hit=|p:&str|node.rect(&format!("{BASE}/{p}")).is_some_and(|r|r.contains(pointer));
        if hit(FOLDER_BTN){
            let dir=custom_dir(&self.root);
            let _=std::fs::create_dir_all(&dir);
            let opener=if cfg!(windows){"explorer"}else if cfg!(target_os="macos"){"open"}else{"xdg-open"};
            let _=std::process::Command::new(opener).arg(&dir).spawn();
            self.scanned=0.0;
            return true;
        }
        if hit("Playlist/EditTrack/EditUpBtn/Icon")||hit("Playlist/EditTrack/EditDownBtn/Icon"){
            let delta=if hit("Playlist/EditTrack/EditUpBtn/Icon"){-1}else{1};
            if !move_track(&mut self.queue,self.selected,delta){return true;}
        }else if hit("AvailableTracks/AddAllBtn/AddAllBtn"){
            for (i,v) in self.tracks.iter().enumerate(){if available(v,&self.owned)&&!self.queue.contains(&i){self.queue.push(i);}}
        }else if hit("Playlist/RemoveAllBtn/RemoveAllBtn"){self.queue.clear();self.playing=self.previewing();}
        else if hit("TrackPlayer/PlayButton") {self.playing=!self.playing&&(!self.queue.is_empty()||self.previewing());}
        else if hit("TrackPlayer/NextTrack")||hit("TrackPlayer/PreviousTrack"){
            if !self.queue.is_empty(){
                let delta=if hit("TrackPlayer/NextTrack"){1}else{-1};
                let i=match self.queue.iter().position(|&i|i==self.selected){
                    Some(i)=>(i as isize+delta).rem_euclid(self.queue.len() as isize) as usize,None=>0};
                self.selected=self.queue[i];self.playing=true;}
        }else{
            let mut picked=None;
            for i in 0..self.tracks.len(){
                if hit(&format!("AvailableTracks/Scroll/Container/Content/Track{i}/AddButton")){picked=Some((i,1));break;}
                if hit(&format!("Playlist/Scroll/Container/Content/Track{i}/RemoveButton")){picked=Some((i,2));break;}
                if ["AvailableTracks","Playlist"].iter().any(|side|hit(&format!("{side}/Scroll/Container/Content/Track{i}/TrackIcon"))||hit(&format!("{side}/Scroll/Container/Content/Track{i}/SelectTrackButton"))){picked=Some((i,0));break;}
            }
            let Some((i,action))=picked else{return false;};
            if !available(&self.tracks[i],&self.owned){return true;}
            // Add / remove move the track between the lists; selecting previews it.
            match action{1=>{if !self.queue.contains(&i){self.queue.push(i);}},2=>self.queue.retain(|&v|v!=i),_=>{self.selected=i;self.playing=true;}}
        }
        self.changed(node);true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    fn tracks()->Vec<Value>{vec![
        json!({"id":"Free","loc":"FreeName","free":true}),
        json!({"id":"Owned","free":false,"wallet":"UIMusic-Owned"}),
        json!({"id":"Locked","free":false,"wallet":"UIMusic-Locked"}),
        json!({"id":"Unmapped","free":false,"wallet":""})]}
    #[test]
    fn reorder_keeps_track_identity_and_stops_at_both_ends(){
        let mut queue=vec![7,3,9];
        assert!(move_track(&mut queue,3,-1));assert_eq!(queue,vec![3,7,9]);
        assert!(!move_track(&mut queue,3,-1));
        assert!(move_track(&mut queue,3,1));assert!(move_track(&mut queue,3,1));
        assert_eq!(queue,vec![7,9,3]);assert!(!move_track(&mut queue,3,1));
        assert!(!move_track(&mut queue,55,1));assert!(!move_track(&mut [],3,1));
    }
    #[test]
    fn trophy_music_uses_exact_wallet_ownership(){
        let t=tracks();let owned=HashSet::from(["UIMusic-Owned".into(),"Locked".into(),"".into()]);
        assert_eq!(queue_from_saved(&t,&Value::Null,&owned),vec![0,1]);
        assert_eq!(queue_from_saved(&t,&Value::Null,&HashSet::new()),vec![0]);
    }
    #[test]
    fn saved_queue_preserves_order_but_drops_duplicates_unknown_and_unowned(){
        let saved=json!({"jukebox":["Owned","Missing","Free","Owned","Locked"]});
        assert_eq!(queue_from_saved(&tracks(),&saved,&HashSet::from(["UIMusic-Owned".into()])),vec![1,0]);
        assert_eq!(queue_from_saved(&tracks(),&saved,&HashSet::new()),vec![0]);
        assert!(queue_from_saved(&tracks(),&json!({"jukebox":[]}),&HashSet::new()).is_empty());
    }
    #[test]
    fn removed_selected_track_keeps_playing_as_a_preview_and_a_locked_one_stays_paused(){
        let mut node=Node::from_json(&json!({"name":"Scene","active":true,"children":[{"name":"Canvas","active":true,
            "children":[{"name":"JukeboxScreen","active":true,"children":[
                {"name":"Playlist","active":true,"children":[{"name":"Scroll","active":true,"children":[
                    {"name":"Container","active":true,"children":[{"name":"Content","active":true,"children":[
                        {"name":"Track1","active":true,"children":[{"name":"RemoveButton","active":true}]}]}]}]}]},
                {"name":"TrackPlayer","active":true,"children":[{"name":"PlayButton","active":true}]}
            ]}]}]}));
        let mut browser=Browser{tracks:tracks(),queue:vec![1,0],selected:1,playing:true,
            owned:HashSet::from(["UIMusic-Owned".into()]),account:None,saved:json!({}),
            template:Node::from_json(&json!({"name":"Track","active":true})),labels:HashMap::new(),pending:None,
            root:PathBuf::new(),runtime:Value::Null,opened:0.0,scanned:0.0,row_eq:None};
        node.find_mut(&format!("{BASE}/Playlist/Scroll/Container/Content/Track1/RemoveButton")).unwrap().screen=Rect::new(10.0,10.0,40.0,40.0);
        assert!(browser.click(&mut node,vec2(20.0,20.0)));
        // RemoveTrackFromPlayList: the track goes back to Songs, still the one selected.
        assert_eq!(browser.selected,1);let change=browser.pending.take().unwrap();
        assert_eq!(change.tracks,vec![("Free".into(),"FreeName".into())]);assert!(change.playing);
        assert_eq!(change.preview,Some(("Owned".into(),"".into())));
        browser.queue.clear();browser.selected=2;browser.changed(&mut node);assert!(!browser.pending.take().unwrap().playing);
        node.find_mut(&format!("{BASE}/TrackPlayer/PlayButton")).unwrap().screen=Rect::new(10.0,10.0,40.0,40.0);
        assert!(browser.click(&mut node,vec2(20.0,20.0)));assert!(!browser.pending.take().unwrap().playing);
    }
}
