//! Opt-in frame capture. No per-frame IO; buffers exist only during capture.
pub struct Capture {
    path:std::path::PathBuf,
    samples:Vec<(u32,f64,f64,usize,usize,usize,usize,bool,usize)>,
}
impl Capture {
    pub fn from_env()->Option<Self>{
        std::env::var_os("RRBTD6_FRAME_BENCH").map(|p|Self{path:p.into(),samples:Vec::with_capacity(4096)})
    }
    pub fn record(&mut self,frame:u32,wall:f64,cpu:f64,bloons:usize,projectiles:usize,towers:usize,round:usize,active:bool,paragons:usize){
        self.samples.push((frame,wall,cpu,bloons,projectiles,towers,round,active,paragons));
    }
}
impl Drop for Capture {
    fn drop(&mut self){
        use std::io::Write;
        let result=(||->std::io::Result<()>{
            let mut out=std::io::BufWriter::new(std::fs::File::create(&self.path)?);
            writeln!(out,"frame,wall_ms,cpu_submit_ms,bloons,projectiles,towers,round,round_active,paragons")?;
            for (frame,wall,cpu,b,p,t,r,a,n) in &self.samples{writeln!(out,"{frame},{:.6},{:.6},{b},{p},{t},{r},{a},{n}",wall*1000.0,cpu*1000.0)?;}
            out.flush()
        })();
        if let Err(e)=result{eprintln!("frame benchmark write failed: {e}");}
    }
}
