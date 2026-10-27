//! Sound effects and music from local_extracted/audio (tools/assets/export_audio.py).
//!
//! - PopEffectModel groups: a pop sound in group G starts only if fewer than `limit` of G are
//!   playing and the last one started at least `frames` (fast forward: `frames_ff`) display
//!   frames ago (HYPOTHESIS P-SOUNDGROUP: frames at 60 per second of real time).
//! - STAND-IN P-SFXLIMIT: other clips are limited per family (name without trailing digits:
//!   "Explosion01".."05") to 3 playing and one start per 3 frames, and to 24 sounds overall.
//!   The game's own limiter (CreateSoundOn*.limiter, AudioFactory) isn't reversed.
//! - STAND-IN playlist: the in-game tracks loop one after another; the game's music selection
//!   isn't reversed.

use crate::game::Sfx;
use macroquad::audio::{load_sound_from_bytes, play_sound, set_sound_volume, stop_sound, PlaySoundParams, Sound};
use std::collections::HashMap;
use std::future::Future;
use std::path::PathBuf;
use std::pin::pin;
use std::task::{Context, Poll, Waker};

/// Fallback when ui/runtime/jukebox.json (export_jukebox.py) is missing.
const FALLBACK_PLAYLIST: [&str; 6] = ["MusicUpbeat1A", "MusicUpbeat2A", "MusicUpbeat3A", "MusicCityA", "MusicBTD5JazzA", "MusicSailsAgain"];

/// The jukebox's tracks (AudioJukeBox.musicTrackData, in its order): the ones listed in the
/// jukebox (not unavailableInJukeBox) that are free; Trophy Store tracks need owning and
/// aren't offered yet. (clip id, loc key of its name)
fn playlist(root: &std::path::Path) -> Vec<(String, String)> {
    let tracks: Vec<serde_json::Value> = std::fs::read_to_string(root.join("ui/runtime/jukebox.json"))
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default();
    let list: Vec<(String, String)> = tracks
        .iter()
        .filter(|t| t["jukebox"] == true && t["free"] == true)
        .filter_map(|t| Some((t["id"].as_str()?.to_string(), t["loc"].as_str().unwrap_or("").to_string())))
        .collect();
    let list=if list.is_empty(){FALLBACK_PLAYLIST.iter().map(|s|(s.to_string(),String::new())).collect()}else{list};
    let settings:serde_json::Value=std::fs::read(root.join("settings.json")).ok().and_then(|b|serde_json::from_slice(&b).ok()).unwrap_or_default();
    if let Some(ids)=settings["jukebox"].as_array(){ids.iter().filter_map(|id|list.iter().find(|(name,_)|id.as_str()==Some(name)).cloned()).collect()}
    else{list}
}
pub const FAMILY_LIMIT: usize = 3;
const FAMILY_GAP: f64 = 3.0 / 60.0;
pub const TOTAL_LIMIT: usize = 24;

/// Clip and its length in seconds.
struct Clip {
    sound: Sound,
    secs: f64,
}

/// Volumes at slider 100%.
pub const DEFAULT_SFX: f32 = 0.6;
pub const DEFAULT_MUSIC: f32 = 0.4;

pub struct Audio {
    dir: PathBuf,
    clips: HashMap<String, Option<Clip>>,
    /// End times of playing sounds per group / family, and the last start.
    playing: HashMap<String, (Vec<f64>, f64)>,
    all: Vec<f64>,
    music: Option<(usize, Clip, f64)>,
    playlist: Vec<(String, String)>,
    music_enabled:bool,
    requested_track:Option<usize>,
    pub sfx_volume: f32,
    pub music_volume: f32,
    /// Jukebox: repeat the current track / pick the next track at random.
    pub loop_track: bool,
    pub shuffle: bool,
    /// Requested track change (+1 next, -1 previous), applied on the next `music` call.
    skip: i32,
}

/// Runs a future that never waits (macroquad's sound loading on desktop).
fn ready<F: Future>(f: F) -> Option<F::Output> {
    let mut cx = Context::from_waker(Waker::noop());
    match pin!(f).poll(&mut cx) {
        Poll::Ready(v) => Some(v),
        Poll::Pending => None,
    }
}

/// Seconds of 16-bit PCM in a WAV file.
fn wav_secs(b: &[u8]) -> f64 {
    let u16_at = |o: usize| b.get(o..o + 2).map_or(0, |s| u16::from_le_bytes([s[0], s[1]]) as u32);
    let u32_at = |o: usize| b.get(o..o + 4).map_or(0, |s| u32::from_le_bytes([s[0], s[1], s[2], s[3]]));
    let (mut o, mut rate, mut frame) = (12, 0, 0);
    while o + 8 <= b.len() {
        let len = u32_at(o + 4) as usize;
        match &b[o..o + 4] {
            b"fmt " => {
                rate = u32_at(o + 12);
                frame = u16_at(o + 20);
            }
            b"data" if rate > 0 && frame > 0 => return len as f64 / (rate * frame) as f64,
            _ => {}
        }
        o += 8 + len + (len & 1);
    }
    0.0
}

impl Audio {
    pub fn new(root: &std::path::Path) -> Self {
        Self {
            dir: root.join("audio"),
            clips: HashMap::new(),
            playing: HashMap::new(),
            all: Vec::new(),
            music: None,
            playlist: playlist(root),music_enabled:true,requested_track:None,
            sfx_volume: DEFAULT_SFX,
            music_volume: DEFAULT_MUSIC,
            loop_track: false,
            shuffle: false,
            skip: 0,
        }
    }

    fn load(dir: &std::path::Path, name: &str) -> Option<Clip> {
        let bytes = std::fs::read(dir.join(format!("{name}.wav"))).ok()?;
        let sound = ready(load_sound_from_bytes(&bytes))?.ok()?;
        Some(Clip { sound, secs: wav_secs(&bytes) })
    }

    /// Plays the queued sounds. `now`: seconds of real time; `fast`: fast forward is on.
    pub fn play(&mut self, sfx: &mut Vec<Sfx>, now: f64, fast: bool) {
        self.all.retain(|&e| e > now);
        for s in sfx.drain(..) {
            if self.sfx_volume <= 0.0 || self.all.len() >= TOTAL_LIMIT {
                continue;
            }
            let (key, limit, gap) = match &s.group {
                Some((g, limit, f, ff)) => (g.clone(), *limit as usize, *(if fast { ff } else { f }) as f64 / 60.0),
                None => (s.name.trim_end_matches(|c: char| c.is_ascii_digit()).to_string(), FAMILY_LIMIT, FAMILY_GAP),
            };
            let slot = self.playing.entry(key).or_insert((Vec::new(), f64::MIN));
            slot.0.retain(|&e| e > now);
            if slot.0.len() >= limit.max(1) || now - slot.1 < gap {
                continue;
            }
            let dir = &self.dir;
            let Some(clip) = self.clips.entry(s.name.clone()).or_insert_with(|| Self::load(dir, &s.name)) else { continue };
            play_sound(&clip.sound, PlaySoundParams { looped: false, volume: self.sfx_volume });
            let end = now + clip.secs.max(0.05);
            slot.0.push(end);
            slot.1 = now;
            self.all.push(end);
        }
    }

    /// Keeps the playlist going: the next track starts when the current one ends.
    pub fn music(&mut self, now: f64) {
        if self.music_volume <= 0.0 || !self.music_enabled || self.playlist.is_empty() {
            if let Some((_, c, _)) = self.music.take() {
                stop_sound(&c.sound);
            }
            return;
        }
        let n = self.playlist.len().max(1);
        let next = match &self.music {
            Some((i, _, _)) if self.skip != 0 => (*i as i32 + self.skip).rem_euclid(n as i32) as usize,
            Some((i, _, end)) if now >= *end => {
                if self.loop_track {
                    *i
                } else if self.shuffle && n>1 {
                    // Any other track (time-seeded; display only, not the game's RNG).
                    (i + 1 + (now * 1000.0) as usize % (n - 1)) % n
                } else {
                    (i + 1) % n
                }
            }
            Some(_) => return,
            None => self.requested_track.take().unwrap_or(0).min(n-1),
        };
        self.skip = 0;
        if let Some((_, c, _)) = self.music.take() {
            stop_sound(&c.sound);
        }
        if let Some(c) = self.playlist.get(next).and_then(|(id, _)| Self::load(&self.dir, id)) {
            play_sound(&c.sound, PlaySoundParams { looped: false, volume: self.music_volume });
            let end = now + c.secs.max(1.0);
            self.music = Some((next, c, end));
        }
    }

    /// (clips loaded, clips missing, music track playing).
    pub fn stats(&self) -> (usize, usize, Option<&str>) {
        let ok = self.clips.values().filter(|c| c.is_some()).count();
        (ok, self.clips.len() - ok, self.track())
    }

    pub fn set_jukebox(&mut self,change:&crate::frontend::jukebox::Change){
        let same_track=self.track().is_some_and(|id|change.tracks.get(change.selected).is_some_and(|(next,_)|next==id));
        if !same_track || !change.playing{
            if let Some((_,clip,_))=self.music.take(){stop_sound(&clip.sound);}
        }else if let Some((index,_,_))=self.music.as_mut(){*index=change.selected;}
        self.playlist=change.tracks.clone();self.music_enabled=change.playing;
        self.requested_track=Some(change.selected);self.skip=0;
    }

    /// Jukebox previous / next.
    pub fn skip_track(&mut self, delta: i32) {
        self.skip = delta;
    }

    /// Clip name of the playing track.
    pub fn track(&self) -> Option<&str> {
        self.music.as_ref().and_then(|m| self.playlist.get(m.0)).map(|(id, _)| id.as_str())
    }

    /// Loc key of the playing track's jukebox name (MusicItem.locKey; "" when unknown).
    pub fn track_loc(&self) -> Option<&str> {
        self.music.as_ref().and_then(|m| self.playlist.get(m.0)).map(|(_, k)| k.as_str())
    }

    pub fn set_music_volume(&mut self, v: f32) {
        self.music_volume = v;
        if let Some((_, c, _)) = &self.music {
            set_sound_volume(&c.sound, v);
        }
    }
}
