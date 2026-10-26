//! CPU-only loading jobs and cooperative render-thread work budgets.
use std::{future::Future,pin::Pin,task::{Context,Poll},time::{Duration,Instant}};
pub struct Worker<T>{handle:Option<std::thread::JoinHandle<T>>}
pub struct Trace<'a>{kind:&'static str,path:&'a std::path::Path,start:Option<Instant>}
impl<'a> Trace<'a>{pub fn new(kind:&'static str,path:&'a std::path::Path)->Self{
    static ENABLED:std::sync::OnceLock<bool>=std::sync::OnceLock::new();
    Self{kind,path,start:ENABLED.get_or_init(||std::env::var_os("RRBTD6_TIMING").is_some()).then(Instant::now)}
}}
impl Drop for Trace<'_>{fn drop(&mut self){if let Some(start)=self.start{let ms=start.elapsed().as_secs_f64()*1000.0;if ms>=2.0{eprintln!("asset-stall: {} {:.2} ms {}",self.kind,ms,self.path.display());}}}}
pub fn spawn<T:Send+'static>(work:impl FnOnce()->T+Send+'static)->Worker<T>{Worker{handle:Some(std::thread::spawn(work))}}
impl<T> Future for Worker<T>{
    type Output=T;
    fn poll(mut self:Pin<&mut Self>,_cx:&mut Context<'_>)->Poll<T>{
        // Macroquad polls again next frame; its waker deliberately panics.
        if !self.handle.as_ref().expect("loading task polled after completion").is_finished(){return Poll::Pending;}
        Poll::Ready(self.handle.take().unwrap().join().unwrap_or_else(|error|std::panic::resume_unwind(error)))
    }
}
struct YieldOnce(bool);
impl Future for YieldOnce{
    type Output=();
    fn poll(mut self:Pin<&mut Self>,_cx:&mut Context<'_>)->Poll<()>{
        if self.0{Poll::Ready(())}else{self.0=true;Poll::Pending}
    }
}
pub async fn yield_once(){YieldOnce(false).await;}
/// Finish a demand-loaded asset inside the current presentation. Only use for
/// asset futures (workers/cooperative budgets), never timers or next_frame().
/// Prepared assets remain asynchronous; this fallback prevents partially drawn
/// or black presentations when an unexpected asset is needed immediately.
pub fn complete_asset<F:Future>(future:F)->F::Output{
    let mut future=std::pin::pin!(future);
    let mut cx=Context::from_waker(std::task::Waker::noop());
    loop{match future.as_mut().poll(&mut cx){Poll::Ready(value)=>return value,Poll::Pending=>std::thread::yield_now()}}
}
pub struct Budget{began:Instant}
impl Budget{
    pub fn new()->Self{Self{began:Instant::now()}}
    pub async fn checkpoint(&mut self){
        if self.began.elapsed()>=Duration::from_millis(3){yield_once().await;self.began=Instant::now();}
    }
}

#[cfg(test)]mod tests{
    use super::*;
    #[test]fn demand_asset_finishes_worker_and_cooperative_yields_without_presenting(){
        assert_eq!(complete_asset(async{
            let n=spawn(||42).await;yield_once().await;yield_once().await;n
        }),42);
    }
    #[test]fn unfinished_cpu_work_yields_instead_of_joining(){
        let barrier=std::sync::Arc::new(std::sync::Barrier::new(2));let other=barrier.clone();
        let mut worker=Box::pin(spawn(move||{other.wait();42}));let mut cx=Context::from_waker(std::task::Waker::noop());
        assert!(worker.as_mut().poll(&mut cx).is_pending());barrier.wait();
        loop{if let Poll::Ready(value)=worker.as_mut().poll(&mut cx){assert_eq!(value,42);break;}std::thread::yield_now();}
    }
    #[test]fn cooperative_yield_finishes_on_the_next_poll(){
        let mut task=Box::pin(yield_once());let mut cx=Context::from_waker(std::task::Waker::noop());
        assert!(task.as_mut().poll(&mut cx).is_pending());assert!(task.as_mut().poll(&mut cx).is_ready());
    }
    #[test]fn expired_work_budget_yields_before_continuing(){
        let mut budget=Budget{began:Instant::now()-Duration::from_secs(1)};
        let mut task=Box::pin(budget.checkpoint());let mut cx=Context::from_waker(std::task::Waker::noop());
        assert!(task.as_mut().poll(&mut cx).is_pending());assert!(task.as_mut().poll(&mut cx).is_ready());
    }
}
