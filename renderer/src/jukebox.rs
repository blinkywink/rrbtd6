//! The installed Jukebox scene and track rows; playlist changes stay local.
use crate::uitree::Node;
use macroquad::prelude::*;
use serde_json::Value;
use std::{collections::HashMap,path::Path};
const BASE:&str="Canvas/JukeboxScreen";
#[derive(Clone)]
pub struct Change {pub tracks:Vec<(String,String)>,pub selected:usize,pub playing:bool}
pub(super) struct Browser {
    tracks:Vec<Value>,queue:Vec<usize>,selected:usize,playing:bool,
    template:Node,labels:HashMap<String,String>,pub pending:Option<Change>,
}
impl Browser {
    pub fn load(root:&Path,node:&mut Node,labels:HashMap<String,String>)->Option<Self>{
        let tracks:Vec<Value>=serde_json::from_slice(&std::fs::read(root.join("ui/runtime/jukebox.json")).ok()?).ok()?;
        let tracks:Vec<_>=tracks.into_iter().filter(|v|v["jukebox"]==true).collect();
        let saved:Value=std::fs::read(root.join("settings.json")).ok().and_then(|b|serde_json::from_slice(&b).ok()).unwrap_or_default();
        let queue=if let Some(ids)=saved["jukebox"].as_array(){ids.iter().filter_map(|id|tracks.iter().position(|v|v["id"]==*id&&v["free"]==true)).collect()}
            else{tracks.iter().enumerate().filter(|(_,v)|v["free"]==true).map(|(i,_)|i).collect()};
        let template=Node::load(&root.join("ui/prefabs/JukeboxTrack.json"))?;
        let mut b=Self{tracks,queue,selected:0,playing:true,template,labels,pending:None};
        node.set_active(&format!("{BASE}/BlockClicks"),false);
        node.set_active(&format!("{BASE}/Playlist/EditTrack"),false);
        b.rebuild(node);Some(b)
    }
    fn label(&self,i:usize)->&str{
        let key=self.tracks[i]["loc"].as_str().unwrap_or("");
        self.labels.get(key).map(String::as_str).unwrap_or(key)
    }
    fn rebuild(&mut self,node:&mut Node){
        for (side,ids) in [("AvailableTracks",(0..self.tracks.len()).collect::<Vec<_>>()),("Playlist",self.queue.clone())]{
            let mut rows=Vec::new();
            for i in ids{
                let mut row=self.template.clone();row.name=format!("Track{i}");row.active=true;
                row.set_text("TrackName",self.label(i));
                row.set_active("TrackEQ",false);
                row.set_active("TrophyLockBtn",self.tracks[i]["free"]!=true);
                row.set_active("AddButton",side=="AvailableTracks"&&self.tracks[i]["free"]==true&&!self.queue.contains(&i));
                row.set_active("RemoveButton",side=="Playlist");
                row.set_active("SelectTrackButton/BtnImg",i==self.selected);
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
        self.pending=Some(Change{tracks:self.queue.iter().map(|&i|(self.tracks[i]["id"].as_str().unwrap_or("").into(),self.tracks[i]["loc"].as_str().unwrap_or("").into())).collect(),
            selected:self.queue.iter().position(|&i|i==self.selected).unwrap_or(0),playing:self.playing});
        self.rebuild(node);
    }
    pub fn click(&mut self,node:&mut Node,pointer:Vec2)->bool{
        let hit=|p:&str|node.rect(&format!("{BASE}/{p}")).is_some_and(|r|r.contains(pointer));
        if hit("AvailableTracks/AddAllBtn/AddAllBtn"){
            self.queue=self.tracks.iter().enumerate().filter(|(_,v)|v["free"]==true).map(|(i,_)|i).collect();
        }else if hit("Playlist/RemoveAllBtn/RemoveAllBtn"){self.queue.clear();self.playing=false;}
        else if hit("TrackPlayer/PlayButton") {self.playing=!self.playing;}
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
            if self.tracks[i]["free"]!=true{return true;}
            match action{1=>{if !self.queue.contains(&i){self.queue.push(i);}},2=>self.queue.retain(|&v|v!=i),_=>{self.selected=i;self.playing=true;if !self.queue.contains(&i){self.queue.push(i);}}}
        }
        self.changed(node);true
    }
}
