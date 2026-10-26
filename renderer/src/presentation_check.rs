//! Opt-in audit of physical presentations, including frames hidden by inner awaits.
use std::{future::Future,pin::Pin,sync::atomic::{AtomicBool,Ordering},task::{Context,Poll}};
static ARMED:AtomicBool=AtomicBool::new(false);
pub fn arm(){ARMED.store(true,Ordering::Relaxed);}
pub fn wrap(future:impl Future<Output=()>+'static)->Pin<Box<dyn Future<Output=()>>>{
    let future=Box::pin(future);
    let Some(path)=std::env::var_os("RRBTD6_PRESENTATION_CHECK") else{return future;};
    let directory=std::path::PathBuf::from(path);std::fs::create_dir_all(&directory).unwrap();
    Box::pin(Audited{future,directory,frames:0,bad:0})
}
struct Audited<F>{future:Pin<Box<F>>,directory:std::path::PathBuf,frames:u32,bad:u32}
impl<F:Future<Output=()>> Future for Audited<F>{
    type Output=();
    fn poll(self:Pin<&mut Self>,cx:&mut Context<'_>)->Poll<()>{
        let this=self.get_mut();let result=this.future.as_mut().poll(cx);
        if result.is_pending()&&ARMED.load(Ordering::Relaxed){
            let image=macroquad::prelude::get_screen_data();this.frames+=1;
            let first=image.bytes.get(..3).unwrap_or(&[0,0,0]);
            let samples:Vec<_>=image.bytes.chunks_exact(4).step_by(64).collect();
            let black=samples.iter().filter(|p|p[0]<4&&p[1]<4&&p[2]<4).count();
            let uniform=samples.iter().filter(|p|p[..3]==*first).count();
            if !samples.is_empty()&&(black*1000>samples.len()*995||uniform*1000>samples.len()*995){
                this.bad+=1;
                if this.bad<=8{image.export_png(this.directory.join(format!("blank-{}.png",this.frames)).to_str().unwrap());}
                eprintln!("presentation-check: blank presentation {}",this.frames);
            }
        }
        if result.is_ready()&&std::env::var_os("RRBTD6_PRESENTATION_STRICT").is_some(){assert_eq!(this.bad,0,"blank gameplay presentations detected");}
        result
    }
}
impl<F> Drop for Audited<F>{fn drop(&mut self){eprintln!("presentation-check: {} gameplay presentations, {} blank",self.frames,self.bad);}}
