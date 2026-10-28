//! The installed Jukebox scene and track rows; playlist changes stay local.
use crate::uitree::Node;
use macroquad::prelude::*;
use serde_json::Value;
use std::{collections::{HashMap,HashSet},path::Path};
/// Availability uses the MusicTrackData pointer's Trophy Store wallet, exported from
/// the original data. Enabled/favourite state does not determine ownership.
fn available(track:&Value,owned:&HashSet<String>)->bool {
    track["free"]==true || track["wallet"].as_str().is_some_and(|id|!id.is_empty()&&owned.contains(id))
}
fn queue_from_saved(tracks:&[Value],saved:&Value,owned:&HashSet<String>)->Vec<usize> {
    let ids:Vec<usize>=if let Some(ids)=saved["jukebox"].as_array(){
        ids.iter().filter_map(|id|tracks.iter().position(|v|v["id"]==*id&&available(v,owned))).collect()
    }else{tracks.iter().enumerate().filter(|(_,v)|available(v,owned)).map(|(i,_)|i).collect()};
    let mut seen=HashSet::new();ids.into_iter().filter(|i|seen.insert(*i)).collect()
}
pub(crate) fn saved_playlist(root:&Path,owned:&HashSet<String>)->Option<Vec<(String,String)>> {
    let tracks:Vec<Value>=serde_json::from_slice(&std::fs::read(root.join("ui/runtime/jukebox.json")).ok()?).ok()?;
    let tracks:Vec<_>=tracks.into_iter().filter(|t|t["jukebox"]==true).collect();
    let saved:Value=std::fs::read(root.join("settings.json")).ok().and_then(|b|serde_json::from_slice(&b).ok()).unwrap_or_default();
    Some(queue_from_saved(&tracks,&saved,owned).into_iter().filter_map(|i|Some((tracks[i]["id"].as_str()?.to_owned(),tracks[i]["loc"].as_str().unwrap_or("").to_owned()))).collect())
}
fn move_track(queue:&mut [usize],selected:usize,delta:isize)->bool {
    let Some(from)=queue.iter().position(|&i|i==selected) else{return false;};
    let to=from as isize+delta;
    if !(0..queue.len() as isize).contains(&to){return false;}
    queue.swap(from,to as usize);true
}
const BASE:&str="Canvas/JukeboxScreen";
#[derive(Clone)]
pub struct Change {pub tracks:Vec<(String,String)>,pub selected:usize,pub playing:bool}
pub(super) struct Browser {
    tracks:Vec<Value>,queue:Vec<usize>,selected:usize,playing:bool,
    owned:HashSet<String>,account:Option<(bool,u64)>,saved:Value,
    template:Node,labels:HashMap<String,String>,pub pending:Option<Change>,
}
impl Browser {
    pub fn load(root:&Path,node:&mut Node,labels:HashMap<String,String>)->Option<Self>{
        let tracks:Vec<Value>=serde_json::from_slice(&std::fs::read(root.join("ui/runtime/jukebox.json")).ok()?).ok()?;
        let tracks:Vec<_>=tracks.into_iter().filter(|v|v["jukebox"]==true).collect();
        let saved:Value=std::fs::read(root.join("settings.json")).ok().and_then(|b|serde_json::from_slice(&b).ok()).unwrap_or_default();
        let ownership=super::account_preview::music_ownership(None);
        let (owned,account)=ownership.map(|(ids,key)|(ids,Some(key))).unwrap_or_default();
        let queue=queue_from_saved(&tracks,&saved,&owned);
        if tracks.is_empty(){return None;}
        let template=Node::load(&root.join("ui/prefabs/JukeboxTrack.json"))?;
        let mut b=Self{tracks,selected:queue.first().copied().unwrap_or(0),playing:!queue.is_empty(),queue,owned,account,saved,template,labels,pending:None};
        node.set_active(&format!("{BASE}/BlockClicks"),false);

        b.rebuild(node);Some(b)
    }
    pub fn refresh(&mut self,node:&mut Node){
        if let Some((owned,key))=super::account_preview::music_ownership(self.account){
            self.owned=owned;self.account=Some(key);
            self.queue=queue_from_saved(&self.tracks,&self.saved,&self.owned);
            self.repair_selection();self.rebuild(node);
        }
    }
    fn repair_selection(&mut self){
        if !self.queue.contains(&self.selected){self.selected=self.queue.first().copied().unwrap_or(0);}
        if self.queue.is_empty(){self.playing=false;}
    }
    fn label(&self,i:usize)->&str{
        let key=self.tracks[i]["loc"].as_str().unwrap_or("");
        self.labels.get(key).map(String::as_str).unwrap_or(key)
    }
    fn rebuild(&mut self,node:&mut Node){
        node.set_active(&format!("{BASE}/Playlist/EditTrack"),!self.queue.is_empty());
        for (side,ids) in [("AvailableTracks",(0..self.tracks.len()).collect::<Vec<_>>()),("Playlist",self.queue.clone())]{
            let mut rows=Vec::new();
            for i in ids{
                let mut row=self.template.clone();row.name=format!("Track{i}");row.active=true;
                row.set_text("TrackName",self.label(i));
                row.set_active("TrackEQ",false);
                row.set_active("TrophyLockBtn",!available(&self.tracks[i],&self.owned));
                row.set_active("AddButton",side=="AvailableTracks"&&available(&self.tracks[i],&self.owned)&&!self.queue.contains(&i));
                row.set_active("RemoveButton",side=="Playlist");
                row.set_active("SelectTrackButton/BtnImg",i==self.selected&&side=="Playlist");
                rows.push(row);
            }
            let empty=rows.is_empty();
            if let Some(content)=node.find_mut(&format!("{BASE}/{side}/Scroll/Container/Content")){content.children=rows;}
            node.set_active(&format!("{BASE}/{side}/FoundNothingTxt"),empty);
        }
        node.set_text(&format!("{BASE}/TrackPlayer/InnerPanel/TrackName"),self.label(self.selected));
        node.set_active(&format!("{BASE}/TrackPlayer/PlayButton/PlayIcon"),!self.playing);
        node.set_active(&format!("{BASE}/TrackPlayer/PlayButton/PauseIcon"),self.playing);
    }
    fn changed(&mut self,node:&mut Node){
        self.repair_selection();
        self.saved["jukebox"]=serde_json::json!(self.queue.iter().map(|&i|&self.tracks[i]["id"]).collect::<Vec<_>>());
        self.pending=Some(Change{tracks:self.queue.iter().map(|&i|(self.tracks[i]["id"].as_str().unwrap_or("").into(),self.tracks[i]["loc"].as_str().unwrap_or("").into())).collect(),
            selected:self.queue.iter().position(|&i|i==self.selected).unwrap_or(0),playing:self.playing});
        self.rebuild(node);
    }
    pub fn click(&mut self,node:&mut Node,pointer:Vec2)->bool{
        let hit=|p:&str|node.rect(&format!("{BASE}/{p}")).is_some_and(|r|r.contains(pointer));
        if hit("Playlist/EditTrack/EditUpBtn/Icon")||hit("Playlist/EditTrack/EditDownBtn/Icon"){
            let delta=if hit("Playlist/EditTrack/EditUpBtn/Icon"){-1}else{1};
            if !move_track(&mut self.queue,self.selected,delta){return true;}
        }else if hit("AvailableTracks/AddAllBtn/AddAllBtn"){
            self.queue=self.tracks.iter().enumerate().filter(|(_,v)|available(v,&self.owned)).map(|(i,_)|i).collect();
        }else if hit("Playlist/RemoveAllBtn/RemoveAllBtn"){self.queue.clear();self.playing=false;}
        else if hit("TrackPlayer/PlayButton") {self.playing=!self.playing&&!self.queue.is_empty();}
        else if hit("TrackPlayer/NextTrack")||hit("TrackPlayer/PreviousTrack"){
            if !self.queue.is_empty(){let i=self.queue.iter().position(|&i|i==self.selected).unwrap_or(0);
                let delta=if hit("TrackPlayer/NextTrack"){1}else{-1};self.selected=self.queue[(i as isize+delta).rem_euclid(self.queue.len() as isize) as usize];self.playing=true;}
        }else{
            let mut picked=None;
            for i in 0..self.tracks.len(){
                if hit(&format!("AvailableTracks/Scroll/Container/Content/Track{i}/AddButton")){picked=Some((i,1));break;}
                if hit(&format!("Playlist/Scroll/Container/Content/Track{i}/RemoveButton")){picked=Some((i,2));break;}
                if ["AvailableTracks","Playlist"].iter().any(|side|hit(&format!("{side}/Scroll/Container/Content/Track{i}/TrackIcon"))||hit(&format!("{side}/Scroll/Container/Content/Track{i}/SelectTrackButton"))){picked=Some((i,0));break;}
            }
            let Some((i,action))=picked else{return false;};
            if !available(&self.tracks[i],&self.owned){return true;}
            match action{1=>{if !self.queue.contains(&i){self.queue.push(i);if self.queue.len()==1{self.selected=i;}}},2=>self.queue.retain(|&v|v!=i),_=>{self.selected=i;self.playing=true;if !self.queue.contains(&i){self.queue.push(i);}}}
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
    fn removing_selected_track_updates_playback_index_and_empty_queue_stays_paused(){
        let mut node=Node::from_json(&json!({"name":"Scene","active":true,"children":[{"name":"Canvas","active":true,
            "children":[{"name":"JukeboxScreen","active":true,"children":[
                {"name":"Playlist","active":true,"children":[{"name":"Scroll","active":true,"children":[
                    {"name":"Container","active":true,"children":[{"name":"Content","active":true,"children":[
                        {"name":"Track1","active":true,"children":[{"name":"RemoveButton","active":true}]}]}]}]}]},
                {"name":"TrackPlayer","active":true,"children":[{"name":"PlayButton","active":true}]}
            ]}]}]}));
        let mut browser=Browser{tracks:tracks(),queue:vec![1,0],selected:1,playing:true,
            owned:HashSet::from(["UIMusic-Owned".into()]),account:None,saved:json!({}),
            template:Node::from_json(&json!({"name":"Track","active":true})),labels:HashMap::new(),pending:None};
        node.find_mut(&format!("{BASE}/Playlist/Scroll/Container/Content/Track1/RemoveButton")).unwrap().screen=Rect::new(10.0,10.0,40.0,40.0);
        assert!(browser.click(&mut node,vec2(20.0,20.0)));
        assert_eq!(browser.selected,0);let change=browser.pending.take().unwrap();
        assert_eq!(change.tracks,vec![("Free".into(),"FreeName".into())]);assert_eq!(change.selected,0);assert!(change.playing);
        browser.queue.clear();browser.changed(&mut node);assert!(!browser.pending.take().unwrap().playing);
        node.find_mut(&format!("{BASE}/TrackPlayer/PlayButton")).unwrap().screen=Rect::new(10.0,10.0,40.0,40.0);
        assert!(browser.click(&mut node,vec2(20.0,20.0)));assert!(!browser.pending.take().unwrap().playing);
    }
}
