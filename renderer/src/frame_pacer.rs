//! Limit the whole presentation interval, including work after the limiter.
use std::time::{Duration, Instant};

#[derive(Default)]
pub struct FramePacer {
    next: Option<(u32, Instant)>,
    #[cfg(windows)]
    timer: Option<windows::Timer>,
    #[cfg(windows)]
    timer_tried: bool,
}

impl FramePacer {
    /// Keep an absolute schedule while frames fit. Missed deadlines start a new
    /// interval, so loading or slow frames cannot trigger a burst of catch-up frames.
    fn deadline(&mut self, cap: u32, frame_start: Instant, now: Instant) -> Option<Instant> {
        if cap == 0 {
            self.next = None;
            return None;
        }
        let period = Duration::from_secs_f64(1.0 / cap as f64);
        let deadline = match self.next {
            Some((previous_cap, deadline)) if previous_cap == cap => deadline,
            _ => frame_start + period,
        }.max(now);
        self.next = Some((cap, deadline + period));
        Some(deadline)
    }

    pub fn wait(&mut self, cap: u32, frame_start: Instant) {
        let Some(deadline) = self.deadline(cap, frame_start, Instant::now()) else { return };
        let left = deadline.saturating_duration_since(Instant::now());
        if left.is_zero() { return }
        #[cfg(windows)]
        {
            if !self.timer_tried {
                self.timer = windows::Timer::new();
                self.timer_tried = true;
            }
            // A private high-resolution wait avoids burning a CPU core for the
            // end of every capped frame. Failure uses the portable fallback.
            if self.timer.as_ref().is_some_and(|timer| timer.wait_until(deadline)) { return }
        }
        if left > Duration::from_millis(2) {
            std::thread::sleep(left - Duration::from_millis(1));
        }
        while Instant::now() < deadline {
            std::hint::spin_loop();
        }
    }
}

#[cfg(windows)]
mod windows {
    use super::*;
    use std::ffi::c_void;
    type Handle = *mut c_void;

    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn CreateWaitableTimerExW(attributes: *const c_void, name: *const u16, flags: u32, access: u32) -> Handle;
        fn SetWaitableTimerEx(timer: Handle, due: *const i64, period: i32, callback: Option<unsafe extern "system" fn(*mut c_void, u32, u32)>, argument: *mut c_void, context: *const c_void, tolerance: u32) -> i32;
        fn WaitForSingleObject(handle: Handle, milliseconds: u32) -> u32;
        fn CloseHandle(handle: Handle) -> i32;
    }

    pub(super) struct Timer(Handle);
    impl Timer {
        pub(super) fn new() -> Option<Self> {
            // CREATE_WAITABLE_TIMER_HIGH_RESOLUTION; SYNCHRONIZE | TIMER_MODIFY_STATE.
            let handle = unsafe { CreateWaitableTimerExW(std::ptr::null(), std::ptr::null(), 2, 0x0010_0002) };
            (!handle.is_null()).then_some(Self(handle))
        }

        pub(super) fn wait_until(&self, deadline: Instant) -> bool {
            let left = deadline.saturating_duration_since(Instant::now());
            if left.is_zero() { return true }
            // Negative due times are relative, in 100 ns units. Round up so
            // truncation cannot wake the frame earlier than its deadline.
            let ticks = left.as_nanos().div_ceil(100).min(i64::MAX as u128) as i64;
            let due = -ticks;
            unsafe {
                SetWaitableTimerEx(self.0, &due, 0, None, std::ptr::null_mut(), std::ptr::null(), 0) != 0
                    && WaitForSingleObject(self.0, u32::MAX) == 0
            }
        }
    }
    impl Drop for Timer {
        fn drop(&mut self) { unsafe { CloseHandle(self.0); } }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn includes_submission_and_swap_in_following_interval() {
        let start = Instant::now();
        let ms = |n| start + Duration::from_millis(n);
        let mut pacer = FramePacer::default();
        assert_eq!(pacer.deadline(100, start, ms(2)), Some(ms(10)));
        // Three milliseconds spent presenting after the limiter must not
        // shift the next deadline to 23 ms.
        assert_eq!(pacer.deadline(100, ms(13), ms(15)), Some(ms(20)));
        assert_eq!(pacer.deadline(100, ms(23), ms(25)), Some(ms(30)));
    }

    #[test]
    fn stall_does_not_produce_catch_up_burst() {
        let start = Instant::now();
        let ms = |n| start + Duration::from_millis(n);
        let mut pacer = FramePacer::default();
        pacer.deadline(100, start, ms(2));
        assert_eq!(pacer.deadline(100, ms(12), ms(300)), Some(ms(300)));
        assert_eq!(pacer.deadline(100, ms(302), ms(304)), Some(ms(310)));
    }

    #[test]
    fn cap_changes_and_unlimited_reset_schedule() {
        let start = Instant::now();
        let ms = |n| start + Duration::from_millis(n);
        let mut pacer = FramePacer::default();
        pacer.deadline(100, start, ms(2));
        assert_eq!(pacer.deadline(50, ms(13), ms(15)), Some(ms(33)));
        assert_eq!(pacer.deadline(0, ms(35), ms(36)), None);
        assert_eq!(pacer.deadline(100, ms(100), ms(102)), Some(ms(110)));
    }

    #[cfg(windows)]
    #[test]
    fn native_timer_waits_until_deadline() {
        // Older Windows may not support the high-resolution flag; normal play
        // then uses the portable sleep/spin fallback.
        if let Some(timer) = windows::Timer::new() {
            let deadline = Instant::now() + Duration::from_millis(3);
            assert!(timer.wait_until(deadline));
            assert!(Instant::now() >= deadline);
        }
    }

    /// Small independent limiter benchmark for memory-constrained machines.
    /// This models presentation delay; it is not a game-rendering benchmark.
    #[cfg(windows)]
    #[test]
    #[ignore]
    fn benchmark_limiter() {
        #[repr(C)]
        #[derive(Default)]
        struct FileTime { low: u32, high: u32 }
        #[link(name = "kernel32")]
        unsafe extern "system" {
            fn GetCurrentProcess() -> isize;
            fn GetProcessTimes(process: isize, creation: *mut FileTime, exit: *mut FileTime, kernel: *mut FileTime, user: *mut FileTime) -> i32;
        }
        let cpu = || {
            let (mut creation, mut exit, mut kernel, mut user) = (FileTime::default(), FileTime::default(), FileTime::default(), FileTime::default());
            assert_ne!(unsafe { GetProcessTimes(GetCurrentProcess(), &mut creation, &mut exit, &mut kernel, &mut user) }, 0);
            let ticks = |t: FileTime| ((t.high as u64) << 32) | t.low as u64;
            (ticks(kernel) + ticks(user)) as f64 / 10_000.0
        };
        println!("cap,reference,frames,fps,cpu_ms_per_frame,p99_ms");
        for cap in [60, 120, 144] {
            for pair in 0..3 {
                for legacy in if pair % 2 == 0 { [true, false] } else { [false, true] } {
                    let mut pacer = FramePacer::default();
                    let period = Duration::from_secs_f64(1.0 / cap as f64);
                    let mut samples = Vec::new();
                    let cpu_start = cpu();
                    let started = Instant::now();
                    for _ in 0..180 {
                        let frame = Instant::now();
                        // Identical small CPU workload and post-limiter delay.
                        while frame.elapsed() < Duration::from_micros(250) { std::hint::spin_loop(); }
                        if legacy {
                            let left = period.saturating_sub(frame.elapsed());
                            if left > Duration::from_millis(2) { std::thread::sleep(left - Duration::from_millis(1)); }
                            while frame.elapsed() < period { std::hint::spin_loop(); }
                        } else {
                            pacer.wait(cap, frame);
                        }
                        std::thread::sleep(Duration::from_micros(800));
                        samples.push(frame.elapsed().as_secs_f64() * 1000.0);
                    }
                    let elapsed = started.elapsed().as_secs_f64();
                    let cpu_ms = cpu() - cpu_start;
                    samples.sort_by(f64::total_cmp);
                    println!("{cap},{legacy},{},{:.3},{:.5},{:.5}", samples.len(), samples.len() as f64 / elapsed, cpu_ms / samples.len() as f64, samples[(samples.len() - 1) * 99 / 100]);
                }
            }
        }
    }
}
