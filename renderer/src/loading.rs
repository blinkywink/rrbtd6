//! Original LoadingUI scene and Animator curves, cached for map transitions.
use super::{apply_animation, scene};
use crate::{hud::Hud, menus::Backdrop, uitree::{Node,Ctx,R,draw_tree}};
use macroquad::prelude::*;
use std::path::Path;

struct Trace {dir:std::path::PathBuf,seen:u16,opaque:u8}

pub(super) struct LoadingScreen {
    node: Node,
    runtime: serde_json::Value,
    backdrop: Option<Backdrop>,
    began: f64,
    closing: Option<f64>,
    open_duration: f32,
    idle_duration: f32,
    close_duration: f32,
    trace: Option<Trace>,
    last_draw:Option<f64>,intervals:u32,total_gap:f64,max_gap:f64,long_gaps:u32,
}
impl LoadingScreen {
    pub(super) fn load(root: &Path) -> Option<Self> {
        let runtime: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(root.join("ui/runtime/LoadingUI.json")).ok()?).ok()?;
        let duration = |name: &str| runtime["animators"].as_array().into_iter().flatten()
            .flat_map(|a|a["clips"].as_array().into_iter().flatten()).find(|c|c["name"]==name)
            .and_then(|c|c["stop"].as_f64()).unwrap_or(0.0) as f32;
        let (open_duration,idle_duration,close_duration) =
            (duration("GeneralLoadingOpen"),duration("GeneralLoadingIdle"),duration("GeneralLoadingClose"));
        let trace=std::env::var_os("RRBTD6_LOADING_CAPTURE_DIR").and_then(|dir| {
            let dir=std::path::PathBuf::from(dir);
            std::fs::create_dir_all(&dir).ok()?;
            Some(Trace {dir,seen:0,opaque:0})
        });
        Some(Self { node: scene(&root.join("ui/prefabs/LoadingUI.json"))?, runtime,
            backdrop: None, began: get_time(), closing: None, open_duration, idle_duration, close_duration,trace,
            last_draw:None,intervals:0,total_gap:0.0,max_gap:0.0,long_gaps:0 })
    }
    pub(super) fn open_done(&self) -> bool { get_time()-self.began >= self.open_duration as f64 }
    pub(super) fn finish(&mut self) {
        if std::env::var_os("RRBTD6_TIMING").is_some(){eprintln!("loading cadence: {} intervals, average {:.1} ms, max {:.1} ms, {} over 33 ms",self.intervals,self.total_gap*1000.0/self.intervals.max(1) as f64,self.max_gap*1000.0,self.long_gaps);}
        self.closing = Some(get_time());
    }
    pub(super) fn closing(&self) -> bool { self.closing.is_some_and(|t| get_time()-t < self.close_duration as f64) }

    pub(super) fn begin(&mut self) {
        self.began = get_time();
        self.closing = None;
        self.last_draw=None;self.intervals=0;self.total_gap=0.0;self.max_gap=0.0;self.long_gaps=0;
        if let Some(trace)=&mut self.trace {trace.seen=0;trace.opaque=0;}
        self.backdrop = Some(Backdrop::capture_sharp());
    }
    pub(super) fn draw(&mut self, hud: &mut Hud) {
        let now=get_time();
        if self.closing.is_none(){if let Some(previous)=self.last_draw{let gap=now-previous;self.intervals+=1;self.total_gap+=gap;self.max_gap=self.max_gap.max(gap);self.long_gaps+=(gap>1.0/30.0) as u32;}self.last_draw=Some(now);}
        set_default_camera();
        gl_use_default_material();
        if self.closing.is_none() {
            clear_background(BLACK);
            if let Some(bg) = &self.backdrop { bg.draw(); }
        }
        let elapsed=(get_time()-self.began) as f32;
        if let Some(began) = self.closing {
            apply_animation(&mut self.node,&self.runtime,"GeneralLoadingAnim","GeneralLoadingClose",Some((get_time()-began) as f32));
        } else if apply_animation(&mut self.node,&self.runtime,"GeneralLoadingAnim","GeneralLoadingOpen",Some(elapsed)) {
            apply_animation(&mut self.node,&self.runtime,"GeneralLoadingAnim","GeneralLoadingIdle",
                Some((elapsed-self.open_duration).rem_euclid(self.idle_duration.max(0.001))));
        }
        let scale=Hud::canvas_scale();
        let canvas=vec2(screen_width()/scale,screen_height()/scale);
        let mut ctx=Ctx {sprites:&mut hud.sprites,fonts:&hud.fonts,scale,canvas_h:canvas.y,clip:None};
        draw_tree(&mut self.node,R {min:Vec2::ZERO,size:canvas},&mut ctx);
        // Opt-in transition regression captures; no readback during normal play.
        if let Some(trace)=&mut self.trace {
            let (phase,slot)=if let Some(began)=self.closing {
                ("close",7+(((get_time()-began) as f32/self.close_duration.max(0.001)*3.0) as u8).min(2))
            } else if elapsed<self.open_duration {
                ("open",((elapsed/self.open_duration.max(0.001)*3.0) as u8).min(2))
            } else {
                let slot=3+trace.opaque.min(3);trace.opaque=trace.opaque.saturating_add(1);("opaque",slot)
            };
            if trace.seen & (1<<slot)==0 {
                trace.seen|=1<<slot;
                get_screen_data().export_png(trace.dir.join(format!("{slot:02}_{phase}.png")).to_str().unwrap());
            }
        }
    }
}
