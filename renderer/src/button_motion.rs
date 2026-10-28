//! Original GlobalButtonAnimation clips, evaluated only when feedback changes.
use crate::uitree::Node;
use macroquad::prelude::*;
use serde_json::{json,Value};

#[derive(Default)]
pub(super) struct ButtonMotion {tracks:Vec<Track>}
struct Track {controller:String,path:String,data:Value,state:Option<&'static str>,began:f64,complete:bool,down:bool,base_color:Option<Color>}
impl ButtonMotion {
    pub fn from_runtime(runtime:&Value)->Self {
        Self {tracks:runtime["animators"].as_array().into_iter().flatten()
            .filter(|a|matches!(a["controller"].as_str(),Some("GlobalButtonAnimation"|"GlobalButtonHoverAnimation"|"GlobalTabAnimation"|"MapButton")))
            .filter_map(|a|Some(Track {controller:a["controller"].as_str()?.to_owned(),path:a["path"].as_str()?.to_owned(),data:json!({"animators":[a]}),
                state:None,began:0.0,complete:false,down:false,base_color:None})).collect()}
    }
    pub fn reset(&mut self){for track in &mut self.tracks{track.state=None;track.down=false;track.complete=false;}}
    pub fn update(&mut self,node:&mut Node) {
        self.update_input(node,mouse_position().into(),is_mouse_button_pressed(MouseButton::Left),is_mouse_button_down(MouseButton::Left),get_time());
    }
    fn update_input(&mut self,node:&mut Node,pointer:Vec2,pressed:bool,held:bool,now:f64) {
        for track in &mut self.tracks {
            let Some(rect)=node.rect(&track.path).filter(|r|r.w>0.0&&r.h>0.0) else {track.down=false;continue};
            let inside=rect.contains(pointer);
            if pressed{track.down=inside;}if !held{track.down=false;}
            let Some(target)=node.find_mut(&track.path) else{continue};
            let Some(state)=target.animated_button_state(inside,track.down) else{continue};
            target.native_button_animation=true;
            if track.base_color.is_none(){track.base_color=target.img.as_ref().map(|img|img.color);}
            if track.state!=Some(state){
                // Shared Normal clips can omit color bindings that Pressed writes. Restore
                // the authored image color before evaluating the new state's overrides.
                if let (Some(img),Some(color))=(target.img.as_mut(),track.base_color){img.color=color;}
                track.state=Some(state);track.began=now;track.complete=false;
            }
            if track.complete{continue;}
            let clip=track.data["animators"][0]["clips"].as_array().into_iter().flatten().find(|c|c["name"]==state);
            let duration=clip.map(|c|c["stop"].as_f64().unwrap_or(0.0)-c["start"].as_f64().unwrap_or(0.0)).unwrap_or(0.0);
            let looping=clip.is_some_and(|c|c["loop"]==true)&&duration>0.0;
            let elapsed=if looping{(now-track.began).rem_euclid(duration)}else{now-track.began};
            track.complete=super::apply_animation(node,&track.data,&track.controller,state,Some(elapsed as f32))&&!looping;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn release_and_reopen_restore_color_when_normal_clip_omits_it(){
        let runtime=json!({"animators":[{"path":"Settings","controller":"GlobalButtonHoverAnimation","clips":[
            {"name":"Pressed","start":0,"stop":0,"bindings":[{"path":"Settings","type":114,"property":"m_Color.r","start":[0.64],"end":[0.64]}]},
            {"name":"Normal","start":0,"stop":0,"bindings":[]}
        ]}]});
        let mut node=Node::from_json(&json!({"name":"Scene","active":true,"children":[{"name":"Settings","active":true,
            "comps":[{"type":"Button"},{"type":"Image","enabled":true,"color":[1,1,1,1]}]}]}));
        node.find_mut("Settings").unwrap().screen=Rect::new(0.0,0.0,100.0,100.0);
        let mut motion=ButtonMotion::from_runtime(&runtime);
        motion.update_input(&mut node,vec2(50.0,50.0),true,true,0.0);
        assert_eq!(node.find("Settings").unwrap().img.as_ref().unwrap().color.r,0.64);
        motion.update_input(&mut node,vec2(150.0,150.0),false,false,1.0);
        assert_eq!(node.find("Settings").unwrap().img.as_ref().unwrap().color,WHITE);
        motion.update_input(&mut node,vec2(50.0,50.0),true,true,2.0);
        motion.reset();
        motion.update_input(&mut node,vec2(150.0,150.0),false,false,3.0);
        assert_eq!(node.find("Settings").unwrap().img.as_ref().unwrap().color,WHITE);
    }
    #[test]
    fn native_map_and_hover_controllers_use_their_own_press_clips(){
        for controller in ["GlobalButtonHoverAnimation","GlobalTabAnimation","MapButton"]{
            let runtime=json!({"animators":[{"path":"Button","controller":controller,"clips":[{
                "name":"Pressed","start":0.0,"stop":0.0,"bindings":[{"path":"Button","type":4,"attribute":3,
                "start":[0.9,0.9,1.0],"end":[0.9,0.9,1.0]}]}]}]});
            let mut node=Node::from_json(&json!({"name":"Scene","active":true,"children":[{"name":"Button","active":true,"comps":[{"type":"Button"}]}]}));
            node.find_mut("Button").unwrap().screen=Rect::new(0.0,0.0,100.0,100.0);
            let mut motion=ButtonMotion::from_runtime(&runtime);
            motion.update_input(&mut node,vec2(50.0,50.0),true,true,0.0);
            assert_eq!(node.find("Button").unwrap().rt.scale,vec2(0.9,0.9),"{controller}");
        }
    }
    #[test]
    fn source_press_pose_is_restored_on_release_without_double_scaling() {
        let clip=|name:&str,scale:f32|json!({"name":name,"start":0,"stop":0,"bindings":[
            {"path":"Canvas/Button","type":4,"attribute":3,"start":[scale,scale,1],"end":[scale,scale,1]}]});
        let runtime=json!({"animators":[{"path":"Canvas/Button","controller":"GlobalButtonAnimation", "clips":[clip("Normal",1.0),clip("Pressed",0.95),clip("Highlighted",1.0)]}]});
        let mut motion=ButtonMotion::from_runtime(&runtime);
        let mut node=Node::from_json(&json!({"name":"Scene","active":true,"children":[{"name":"Canvas","active":true,
            "children":[{"name":"Button","active":true,"comps":[{"type":"Button"}]}]}]}));
        node.find_mut("Canvas/Button").unwrap().screen=Rect::new(0.0,0.0,100.0,100.0);
        motion.update_input(&mut node,vec2(50.0,50.0),true,true,0.0);
        let button=node.find("Canvas/Button").unwrap();
        assert_eq!(button.rt.scale,vec2(0.95,0.95));assert!(button.native_button_animation);
        motion.update_input(&mut node,vec2(150.0,150.0),false,false,1.0);
        assert_eq!(node.find("Canvas/Button").unwrap().rt.scale,Vec2::ONE);
    }
}

/// Prime only visible-content sprites; offscreen opening transforms must not defer
/// texture decode until the middle of an authored entrance. Cache hits are shared.
pub(super) fn warm_visible(node:&Node,sprites:&mut crate::ui::Sprites){
    fn gather(n:&Node,names:&mut std::collections::HashSet<String>,limit:usize){
        if !n.active || names.len()>=limit{return;}
        if let Some(name)=n.img.as_ref().and_then(|i|i.sprite.as_ref()){names.insert(name.clone());}
        // Catalog masks can contain hundreds of offscreen cards. Prime a bounded
        // first group; scrolling retains normal lazy texture decoding.
        let limit=if n.clip{limit.min(names.len()+16)}else{limit};
        for child in &n.children{gather(child,names,limit);if names.len()>=limit{break;}}
    }
    let mut names=std::collections::HashSet::new();gather(node,&mut names,64);
    for name in names{let _=sprites.get(&name);}
}
