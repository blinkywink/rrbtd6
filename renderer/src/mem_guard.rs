//! Keeps a sandbox stress test from taking Windows down with it (DEVIATIONS.md): when the game holds
//! too much memory, or frames take seconds, the bloons go and the player is told why. Running Windows out
//! of memory has left the display driver stuck at 60 Hz with broken scaling until a restart.
//!
//! Only the sandbox clears anything: a normal game can't send this many bloons, and low memory there is
//! as likely to be another program's doing. `limit_process` is the backstop for every mode: past it
//! allocations fail and the game crashes, which is better than freezing the PC.

const GB: f64 = 1024.0 * 1024.0 * 1024.0;
/// The game alone holding this much is always too much.
const MAX_GAME_GB: f64 = 10.0;
/// With less than this left for the whole PC, and the game a big part of the reason...
const MIN_FREE_COMMIT_GB: f64 = 2.5;
const MIN_FREE_RAM_GB: f64 = 1.5;
/// ...which it is past this size (a normal game holds about 2 GB).
const LARGE_GAME_GB: f64 = 4.0;
/// Frames this long, this many in a row, mean the GPU or simulation can't keep up.
const SLOW_FRAME_SECONDS: f64 = 1.0;
const SLOW_FRAMES: u32 = 3;
/// iOS ends an app outright at its memory limit; this close to it, the bloons go.
const MIN_APP_HEADROOM_GB: f64 = 0.4;
/// Seconds between memory readings, and before it can act again.
const CHECK_EVERY: f64 = 0.5;
const COOLDOWN: f64 = 5.0;
/// Seconds the message stays up.
const MESSAGE_SECONDS: f64 = 8.0;

#[derive(Clone, Copy, Debug, Default)]
pub struct Reading {
    pub game_gb: f64,
    pub free_commit_gb: f64,
    pub free_ram_gb: f64,
    /// What's left before the OS ends the app, where it says (iOS).
    pub app_headroom_gb: Option<f64>,
}

/// Why the bloons should go, if they should.
pub fn memory_reason(r: Reading) -> Option<&'static str> {
    if r.app_headroom_gb.is_some_and(|gb| gb < MIN_APP_HEADROOM_GB) {
        return Some("the game is near this device's memory limit");
    }
    if r.game_gb >= MAX_GAME_GB {
        return Some("the game is using too much memory");
    }
    if r.game_gb >= LARGE_GAME_GB && (r.free_commit_gb < MIN_FREE_COMMIT_GB || r.free_ram_gb < MIN_FREE_RAM_GB) {
        return Some("your PC is almost out of memory");
    }
    None
}

#[derive(Default)]
pub struct MemGuard {
    next_check: f64,
    ready_at: f64,
    slow_frames: u32,
    message: Option<(String, f64)>,
}

impl MemGuard {
    /// Once a frame. True when the bloons should be cleared now.
    pub fn tick(&mut self, now: f64, frame_seconds: f64, sandbox: bool) -> bool {
        self.slow_frames = if frame_seconds >= SLOW_FRAME_SECONDS { self.slow_frames + 1 } else { 0 };
        if !sandbox || now < self.ready_at {
            return false;
        }
        let mut reason = (self.slow_frames >= SLOW_FRAMES).then_some("the game can't keep up");
        if reason.is_none() && now >= self.next_check {
            self.next_check = now + CHECK_EVERY;
            reason = read().and_then(memory_reason);
        }
        let Some(reason) = reason else { return false };
        self.slow_frames = 0;
        self.ready_at = now + COOLDOWN;
        self.message = Some((format!("Sandbox bloons cleared: {reason}"), now + MESSAGE_SECONDS));
        eprintln!("mem_guard: {reason}: {:?}", read());
        true
    }

    /// What to tell the player right now.
    pub fn message(&self, now: f64) -> Option<&str> {
        self.message.as_ref().filter(|(_, until)| now < *until).map(|(text, _)| text.as_str())
    }
}

#[cfg(windows)]
mod os {
    use std::ffi::c_void;

    #[repr(C)]
    #[derive(Default)]
    struct MemoryStatusEx {
        length: u32,
        memory_load: u32,
        total_phys: u64,
        avail_phys: u64,
        total_page_file: u64,
        avail_page_file: u64,
        total_virtual: u64,
        avail_virtual: u64,
        avail_extended_virtual: u64,
    }

    #[repr(C)]
    #[derive(Default)]
    struct ProcessMemoryCounters {
        cb: u32,
        page_fault_count: u32,
        peak_working_set: usize,
        working_set: usize,
        quota_peak_paged: usize,
        quota_paged: usize,
        quota_peak_non_paged: usize,
        quota_non_paged: usize,
        pagefile_usage: usize,
        peak_pagefile_usage: usize,
    }

    #[repr(C)]
    #[derive(Default)]
    struct BasicLimit {
        per_process_time: i64,
        per_job_time: i64,
        flags: u32,
        min_working_set: usize,
        max_working_set: usize,
        active_processes: u32,
        affinity: usize,
        priority: u32,
        scheduling: u32,
    }

    #[repr(C)]
    #[derive(Default)]
    struct ExtendedLimit {
        basic: BasicLimit,
        io: [u64; 6],
        process_memory: usize,
        job_memory: usize,
        peak_process: usize,
        peak_job: usize,
    }

    const JOB_OBJECT_LIMIT_PROCESS_MEMORY: u32 = 0x100;
    const JOB_OBJECT_EXTENDED_LIMIT_INFORMATION: i32 = 9;

    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn GlobalMemoryStatusEx(status: *mut MemoryStatusEx) -> i32;
        fn GetCurrentProcess() -> *mut c_void;
        fn K32GetProcessMemoryInfo(process: *mut c_void, counters: *mut ProcessMemoryCounters, size: u32) -> i32;
        fn CreateJobObjectA(attributes: *mut c_void, name: *const i8) -> *mut c_void;
        fn SetInformationJobObject(job: *mut c_void, class: i32, info: *mut c_void, size: u32) -> i32;
        fn AssignProcessToJobObject(job: *mut c_void, process: *mut c_void) -> i32;
    }

    fn status() -> Option<MemoryStatusEx> {
        let mut s = MemoryStatusEx { length: std::mem::size_of::<MemoryStatusEx>() as u32, ..Default::default() };
        (unsafe { GlobalMemoryStatusEx(&mut s) } != 0).then_some(s)
    }

    pub fn read() -> Option<super::Reading> {
        let s = status()?;
        let mut c = ProcessMemoryCounters { cb: std::mem::size_of::<ProcessMemoryCounters>() as u32, ..Default::default() };
        if unsafe { K32GetProcessMemoryInfo(GetCurrentProcess(), &mut c, c.cb) } == 0 {
            return None;
        }
        Some(super::Reading {
            game_gb: c.pagefile_usage as f64 / super::GB,
            free_commit_gb: s.avail_page_file as f64 / super::GB,
            free_ram_gb: s.avail_phys as f64 / super::GB,
            app_headroom_gb: None,
        })
    }

    pub fn limit_process() {
        let Some(s) = status() else { return };
        let limit = super::process_limit(s.total_page_file as f64 / super::GB);
        unsafe {
            let job = CreateJobObjectA(std::ptr::null_mut(), std::ptr::null());
            if job.is_null() {
                return;
            }
            let mut info = ExtendedLimit { process_memory: (limit * super::GB) as usize, ..Default::default() };
            info.basic.flags = JOB_OBJECT_LIMIT_PROCESS_MEMORY;
            let size = std::mem::size_of::<ExtendedLimit>() as u32;
            if SetInformationJobObject(job, JOB_OBJECT_EXTENDED_LIMIT_INFORMATION, &mut info as *mut _ as *mut c_void, size) != 0 {
                AssignProcessToJobObject(job, GetCurrentProcess());
            }
            // The handle stays open: closing it would end the limit.
        }
    }
}

/// The hard limit in GB, for a PC with this much commit in total: well below it, so Windows keeps
/// room to work, and at least what a normal game needs.
pub fn process_limit(total_commit_gb: f64) -> f64 {
    (total_commit_gb * 0.5).clamp(6.0, 16.0)
}

#[cfg(windows)]
pub use os::{limit_process, read};

#[cfg(not(windows))]
pub fn limit_process() {}

#[cfg(not(any(windows, target_os = "ios")))]
pub fn read() -> Option<Reading> {
    None
}

/// Only the headroom: the rest is the PC rule's, and iOS ends the app long before those numbers.
#[cfg(target_os = "ios")]
pub fn read() -> Option<Reading> {
    unsafe extern "C" {
        fn os_proc_available_memory() -> usize;
    }
    let headroom = unsafe { os_proc_available_memory() };
    // 0 means it couldn't tell.
    (headroom > 0).then(|| Reading { free_commit_gb: f64::MAX, free_ram_gb: f64::MAX, app_headroom_gb: Some(headroom as f64 / GB), ..Default::default() })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn reading(game_gb: f64, free_commit_gb: f64, free_ram_gb: f64) -> Reading {
        Reading { game_gb, free_commit_gb, free_ram_gb, app_headroom_gb: None }
    }

    #[test]
    fn a_phone_near_its_limit_is_stopped() {
        let near = Reading { app_headroom_gb: Some(0.3), ..reading(1.0, 10.0, 10.0) };
        assert!(memory_reason(near).is_some());
        assert!(memory_reason(Reading { app_headroom_gb: Some(1.5), ..near }).is_none());
    }

    #[test]
    fn a_big_game_on_a_full_pc_is_stopped() {
        assert!(memory_reason(reading(7.8, 1.0, 8.0)).is_some(), "the freeze this was made for");
        assert!(memory_reason(reading(5.0, 8.0, 1.0)).is_some());
        assert!(memory_reason(reading(11.0, 20.0, 20.0)).is_some());
    }

    #[test]
    fn other_programs_filling_the_pc_do_not_clear_a_normal_game() {
        assert!(memory_reason(reading(2.0, 1.0, 1.0)).is_none());
        assert!(memory_reason(reading(5.0, 8.0, 8.0)).is_none());
    }

    #[test]
    fn slow_frames_only_act_in_the_sandbox_and_in_a_row() {
        let mut guard = MemGuard::default();
        for i in 0..5 {
            assert!(!guard.tick(i as f64, 2.0, false), "not outside the sandbox");
        }
        let mut guard = MemGuard::default();
        guard.next_check = 1e9;
        assert!(!guard.tick(0.0, 2.0, true));
        assert!(!guard.tick(2.0, 0.1, true), "a quick frame resets it");
        assert!(!guard.tick(4.0, 2.0, true));
        assert!(!guard.tick(6.0, 2.0, true));
        assert!(guard.tick(8.0, 2.0, true));
        assert!(guard.message(9.0).is_some_and(|m| m.contains("can't keep up")));
        assert!(!guard.tick(9.0, 2.0, true), "cooling down");
        assert!(guard.message(20.0).is_none());
    }

    #[test]
    fn the_hard_limit_leaves_windows_room() {
        assert_eq!(process_limit(35.4), 16.0);
        assert_eq!(process_limit(20.0), 10.0);
        assert_eq!(process_limit(8.0), 6.0);
    }

    #[cfg(windows)]
    #[test]
    fn windows_answers_with_sensible_numbers() {
        let r = read().expect("memory reading");
        assert!(r.game_gb > 0.0 && r.game_gb < 1024.0, "{r:?}");
        assert!(r.free_commit_gb > 0.0 && r.free_ram_gb > 0.0, "{r:?}");
    }
}
