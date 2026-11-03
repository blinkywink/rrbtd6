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
use quad_snd::{AudioContext, PlaySoundParams};
use std::rc::Rc;
use std::collections::HashMap;
use std::path::PathBuf;

/// Fallback when ui/runtime/jukebox.json (export_jukebox.py) is missing.
const FALLBACK_PLAYLIST: [&str; 6] = ["MusicUpbeat1A", "MusicUpbeat2A", "MusicUpbeat3A", "MusicCityA", "MusicBTD5JazzA", "MusicSailsAgain"];

/// The jukebox's tracks (AudioJukeBox.musicTrackData, in its order): the ones listed in the
/// jukebox (not unavailableInJukeBox). Account ownership is applied when its cached
/// snapshot arrives; startup initially offers free tracks. (clip id, localized name key)
fn playlist(root: &std::path::Path) -> Vec<(String, String)> {
    crate::frontend::jukebox::saved_playlist(root,&Default::default()).unwrap_or_else(||FALLBACK_PLAYLIST.iter().map(|s|(s.to_string(),String::new())).collect())
}

#[cfg(test)]
mod storage_tests {
    use super::*;

    #[test]
    fn flac_duration_and_invalid_headers() {
        let mut header=vec![0;42];
        header[..4].copy_from_slice(b"fLaC");
        header[4]=0x80;header[7]=34;
        let bits=(48000u64<<44)|(1<<41)|(15<<36)|96000;
        header[18..26].copy_from_slice(&bits.to_be_bytes());
        assert_eq!(audio_secs(&header),2.0);
        for n in 0..42{assert_eq!(audio_secs(&header[..n]),0.0);}
        header[7]=33;assert_eq!(audio_secs(&header),0.0);
        assert!(quad_snd::decode_samples(b"invalid audio").is_err());
    }

    /// RRBTD6_MP3=<file>: the jukebox's MP3 decoder gives stereo 44.1 kHz samples.
    #[test]
    #[ignore]
    fn custom_mp3_decodes() {
        let path=PathBuf::from(std::env::var("RRBTD6_MP3").expect("set RRBTD6_MP3 to an MP3 file"));
        let p=Prepared::read_custom(&path).expect("decodes");
        assert!(p.samples.len()%2==0&&p.secs>0.5,"{} samples",p.samples.len());
        assert!(p.samples.iter().any(|s|s.abs()>0.01));
        eprintln!("{}: {:.2} s",path.display(),p.secs);
    }

    // Generate synthetic pairs with tools/test_audio_storage.py, or use local
    // WAV+FLAC pairs. No proprietary audio is included in the repository.
    #[test]
    #[ignore]
    fn lossless_mixer_equivalence() {
        let root=PathBuf::from(std::env::var("RRBTD6_AUDIO_FIXTURES").expect("set RRBTD6_AUDIO_FIXTURES to WAV+FLAC fixture directory"));
        let root=if root.is_relative(){PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..").join(root)}else{root};
        let mut count=0;
        for file in std::fs::read_dir(root).unwrap(){
            let p=file.unwrap().path();
            if p.extension().and_then(|s|s.to_str())!=Some("wav"){continue;}
            let wav=std::fs::read(&p).unwrap();
            let flac=std::fs::read(p.with_extension("flac")).unwrap();
            assert_eq!(audio_secs(&wav),audio_secs(&flac),"duration: {}",p.display());
            let start=std::time::Instant::now();
            let a=quad_snd::decode_samples(&wav).unwrap();let wav_ms=start.elapsed().as_secs_f64()*1000.0;
            let start=std::time::Instant::now();
            let b=quad_snd::decode_samples(&flac).unwrap();let flac_ms=start.elapsed().as_secs_f64()*1000.0;
            assert_eq!(a.len(),b.len());
            assert!(a.iter().zip(&b).all(|(a,b)|a.to_bits()==b.to_bits()),"mixer samples: {}",p.display());
            eprintln!("{}: {} -> {} bytes; decode {:.2} -> {:.2} ms; {} exact mixer samples",p.display(),wav.len(),flac.len(),wav_ms,flac_ms,a.len());
            count+=1;
        }
        assert!(count>=4,"include mono, stereo, 48kHz and 22.05kHz fixtures");
    }
}

#[cfg(test)]
mod vorbis_tests {
    use super::*;

    /// RRBTD6_OGG_FIXTURES=<dir> of <clip>.ogg (export_audio.py) + <clip>.flac (FMOD's decode,
    /// older setups): same duration and mixer length, and how far the two decoders differ.
    #[test]
    #[ignore]
    fn vorbis_matches_fmod_decode() {
        let root=PathBuf::from(std::env::var("RRBTD6_OGG_FIXTURES").expect("set RRBTD6_OGG_FIXTURES to an OGG+FLAC fixture directory"));
        let mut count=0;
        for file in std::fs::read_dir(root).unwrap(){
            let p=file.unwrap().path();
            if p.extension().and_then(|s|s.to_str())!=Some("ogg"){continue;}
            let ogg=std::fs::read(&p).unwrap();
            let flac=std::fs::read(p.with_extension("flac")).unwrap();
            assert!((audio_secs(&ogg)-audio_secs(&flac)).abs()<1e-9,"duration {}: {} vs {}",p.display(),audio_secs(&ogg),audio_secs(&flac));
            let start=std::time::Instant::now();
            let a=quad_snd::decode_samples(&ogg).unwrap();let ogg_ms=start.elapsed().as_secs_f64()*1000.0;
            let b=quad_snd::decode_samples(&flac).unwrap();
            assert_eq!(a.len(),b.len(),"mixer samples: {}",p.display());
            let worst=a.iter().zip(&b).map(|(a,b)|(a-b).abs()).fold(0f32,f32::max);
            eprintln!("{}: {} -> {} bytes, {:.3} s, decode {:.2} ms, largest difference {:.6} ({:.1} dB)",
                      p.display(),flac.len(),ogg.len(),audio_secs(&ogg),ogg_ms,worst,20.0*worst.max(1e-9).log10());
            assert!(worst<2e-3,"{}: decoders differ by {worst}",p.display());
            count+=1;
        }
        assert!(count>0);
    }
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
    context: Rc<AudioContext>,
    jobs: HashMap<String,Job>,
    prepared_music: Option<(String,Prepared)>,
    failed_music: std::collections::HashSet<String>,
    clips: HashMap<String, Option<Clip>>,
    /// End times of playing sounds per group / family, and the last start.
    playing: HashMap<String, (Vec<f64>, f64)>,
    all: Vec<f64>,
    music: Option<(usize, Clip, f64)>,
    playlist: Vec<(String, String)>,
    music_enabled:bool,
    music_account:Option<(bool,u64)>,
    requested_track:Option<usize>,
    pub sfx_volume: f32,
    pub music_volume: f32,
    /// Jukebox: repeat the current track / pick the next track at random.
    pub loop_track: bool,
    pub shuffle: bool,
    /// Requested track change (+1 next, -1 previous), applied on the next `music` call.
    skip: i32,
}

// Retain the same quad-snd mixer and resampler. The guard replaces macroquad's
// equivalent guard so prepared PCM can enter the mixer without decoding again.
struct Sound { native:quad_snd::Sound, context:Rc<AudioContext> }
impl Drop for Sound {fn drop(&mut self){self.native.delete(&self.context);}}
fn play_sound(sound:&Sound,params:PlaySoundParams){sound.native.play(&sound.context,params);}
fn stop_sound(sound:&Sound){sound.native.stop(&sound.context);}
fn set_sound_volume(sound:&Sound,volume:f32){sound.native.set_volume(&sound.context,volume);}

struct Prepared {samples:std::sync::Arc<[f32]>,secs:f64}
struct Job {music:bool,handle:std::thread::JoinHandle<Option<Prepared>>}
impl Prepared {
    fn read(dir:&std::path::Path,name:&str)->Option<Self>{
        if let Some(file)=name.strip_prefix(crate::frontend::jukebox::CUSTOM){
            return Self::read_custom(&crate::frontend::jukebox::custom_dir(dir.parent().unwrap_or(dir)).join(file));
        }
        // A fresh WAV export overrides compressed data; the game's own Vorbis (export_audio.py)
        // overrides FLAC decoded by older setups.
        let mut bytes=None;
        for ext in ["wav","ogg","flac"]{
            match std::fs::read(dir.join(format!("{name}.{ext}"))){
                Ok(b)=>{bytes=Some(b);break;}
                Err(e) if e.kind()==std::io::ErrorKind::NotFound=>{}
                Err(_)=>return None,
            }
        }
        let bytes=bytes?;
        let secs=audio_secs(&bytes);
        let samples=quad_snd::decode_samples(&bytes).ok()?;
        Some(Self{samples:samples.into(),secs})
    }

    /// A file from the jukebox's custom music folder: MP3 through symphonia, the rest through
    /// quad-snd; the mixer's interleaved stereo at 44.1 kHz either way.
    fn read_custom(path:&std::path::Path)->Option<Self>{
        let bytes=std::fs::read(path).ok()?;
        let mp3=path.extension().and_then(|x|x.to_str()).is_some_and(|x|x.eq_ignore_ascii_case("mp3"));
        let samples=if mp3{decode_mp3(bytes)?}else{quad_snd::decode_samples(&bytes).ok()?};
        let secs=samples.len() as f64/2.0/44100.0;
        Some(Self{samples:samples.into(),secs})
    }
}

/// An MP3's samples as the mixer takes them (interleaved stereo, 44.1 kHz; mono doubled, other
/// rates resampled nearest-neighbour as quad-snd does).
fn decode_mp3(bytes:Vec<u8>)->Option<Vec<f32>>{
    use symphonia::core::{audio::SampleBuffer,codecs::DecoderOptions,formats::FormatOptions,io::MediaSourceStream,meta::MetadataOptions,probe::Hint};
    let mss=MediaSourceStream::new(Box::new(std::io::Cursor::new(bytes)),Default::default());
    let mut hint=Hint::new();hint.with_extension("mp3");
    let probed=symphonia::default::get_probe().format(&hint,mss,&FormatOptions::default(),&MetadataOptions::default()).ok()?;
    let mut format=probed.format;
    let track=format.default_track()?.clone();
    let mut decoder=symphonia::default::get_codecs().make(&track.codec_params,&DecoderOptions::default()).ok()?;
    let mut rate=track.codec_params.sample_rate.unwrap_or(44100);
    let mut out:Vec<f32>=Vec::new();
    while let Ok(packet)=format.next_packet(){
        if packet.track_id()!=track.id{continue;}
        let Ok(decoded)=decoder.decode(&packet) else{continue};
        let spec=*decoded.spec();rate=spec.rate;
        let mut buf=SampleBuffer::<f32>::new(decoded.capacity() as u64,spec);
        buf.copy_interleaved_ref(decoded);
        match spec.channels.count(){
            1=>for &s in buf.samples(){out.extend_from_slice(&[s,s]);},
            n if n>=2=>for f in buf.samples().chunks_exact(n){out.extend_from_slice(&f[..2]);},
            _=>{}
        }
    }
    if out.is_empty(){return None;}
    if rate!=44100{
        let frames=out.len()/2;
        let new_frames=(frames as f64*44100.0/rate as f64) as usize;
        out=(0..new_frames).flat_map(|n|{let i=2*((n as f64/new_frames as f64*frames as f64) as usize);[out[i],out[i+1]]}).collect();
    }
    Some(out)
}

fn audio_secs(bytes:&[u8])->f64{
    if bytes.starts_with(b"RIFF"){return wav_secs(bytes);}
    if bytes.starts_with(b"OggS"){return ogg_secs(bytes);}
    // FLAC STREAMINFO: 20-bit rate, 3-bit channels, 5-bit depth, 36-bit samples.
    if bytes.len()<42 || &bytes[..4]!=b"fLaC" || bytes[4]&0x7f!=0 || bytes[5..8]!=[0,0,34]{return 0.0;}
    let bits=u64::from_be_bytes(bytes[18..26].try_into().unwrap());
    let rate=bits>>44;let samples=bits&0x0f_ffff_ffff;
    if rate==0{0.0}else{samples as f64/rate as f64}
}

/// Seconds of an Ogg Vorbis file: the last page's granule position (its sample count) over
/// the identification header's rate.
fn ogg_secs(b:&[u8])->f64{
    // The identification packet starts the first page: 27-byte header, its lacing, then packet.
    if b.len()<28{return 0.0;}
    let first=28+b[26] as usize-1;
    if b.len()<first+16 || &b[first..first+7]!=b"\x01vorbis"{return 0.0;}
    let rate=u32::from_le_bytes(b[first+12..first+16].try_into().unwrap());
    let Some(last)=b.windows(4).rposition(|w|w==b"OggS") else{return 0.0};
    if b.len()<last+14 || rate==0{return 0.0;}
    let granule=i64::from_le_bytes(b[last+6..last+14].try_into().unwrap());
    if granule<0{0.0}else{granule as f64/rate as f64}
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
            context: Rc::new(AudioContext::new()),
            jobs: HashMap::new(),
            prepared_music: None,
            failed_music: Default::default(),
            clips: HashMap::new(),
            playing: HashMap::new(),
            all: Vec::new(),
            music: None,
            playlist: playlist(root),music_enabled:true,music_account:None,requested_track:None,
            sfx_volume: DEFAULT_SFX,
            music_volume: DEFAULT_MUSIC,
            loop_track: false,
            shuffle: false,
            skip: 0,
        }
    }

    fn install(&self,prepared:Prepared)->Clip{
        let native=quad_snd::Sound::from_samples(&self.context,prepared.samples);
        Clip{sound:Sound{native,context:self.context.clone()},secs:prepared.secs}
    }

    fn request(&mut self,name:&str,music:bool){
        if (music&&self.failed_music.contains(name))||self.jobs.contains_key(name)||self.jobs.len()>=2||(!music&&self.clips.contains_key(name))
            ||(music&&self.prepared_music.as_ref().is_some_and(|(n,_)|n==name)){return;}
        let dir=self.dir.clone();let id=name.to_owned();
        self.jobs.insert(name.to_owned(),Job{music,handle:std::thread::spawn(move||Prepared::read(&dir,&id))});
    }

    fn pump(&mut self){
        let finished:Vec<_>=self.jobs.iter().filter(|(_,j)|j.handle.is_finished()).map(|(n,_)|n.clone()).collect();
        for name in finished{
            let job=self.jobs.remove(&name).unwrap();
            let prepared=job.handle.join().ok().flatten();
            if job.music{
                if let Some(prepared)=prepared{if self.music_enabled{self.prepared_music=Some((name,prepared));}}else{self.failed_music.insert(name);}
            }else{let clip=prepared.map(|p|self.install(p));self.clips.insert(name,clip);}
        }
    }

    fn load_effect(&mut self,name:&str)->Option<Clip>{
        // Preserve first-use event timing. An unprepared effect can still wait;
        // queued burst effects decode concurrently, and music never waits here.
        let prepared=if let Some(job)=self.jobs.remove(name){job.handle.join().ok().flatten()}
            else{Prepared::read(&self.dir,name)};
        prepared.map(|p|self.install(p))
    }

    /// Plays the queued sounds. `now`: seconds of real time; `fast`: fast forward is on.
    pub fn play(&mut self, sfx: &mut Vec<Sfx>, now: f64, fast: bool) {
        self.pump();
        if self.sfx_volume>0.0{for s in sfx.iter(){self.request(&s.name,false);}}
        self.all.retain(|&e| e > now);
        for s in sfx.drain(..) {
            if self.sfx_volume <= 0.0 || self.all.len() >= TOTAL_LIMIT {
                continue;
            }
            let (key, limit, gap) = match &s.group {
                Some((g, limit, f, ff)) => (g.clone(), *limit as usize, *(if fast { ff } else { f }) as f64 / 60.0),
                None => (s.name.trim_end_matches(|c: char| c.is_ascii_digit()).to_string(), FAMILY_LIMIT, FAMILY_GAP),
            };
            let slot = self.playing.entry(key.clone()).or_insert((Vec::new(), f64::MIN));
            slot.0.retain(|&e| e > now);
            if slot.0.len() >= limit.max(1) || now - slot.1 < gap {
                continue;
            }
            if !self.clips.contains_key(&s.name){
                let clip=self.load_effect(&s.name);self.clips.insert(s.name.clone(),clip);
            }
            let Some(clip)=self.clips.get(&s.name).and_then(|c|c.as_ref()) else{continue};
            play_sound(&clip.sound, PlaySoundParams { looped: false, volume: self.sfx_volume });
            let end = now + clip.secs.max(0.05);
            let slot=self.playing.get_mut(&key).unwrap();
            slot.0.push(end);
            slot.1 = now;
            self.all.push(end);
        }
    }

    /// Keeps the playlist going: the next track starts when the current one ends.
    pub fn music(&mut self, now: f64) {
        // The account reader is asynchronous. Refresh once per new snapshot, without
        // polling files or cloning the inventory on ordinary frames.
        if let Some((owned,revision))=crate::frontend::account_preview::music_ownership(self.music_account){
            self.music_account=Some(revision);
            if let Some(tracks)=crate::frontend::jukebox::saved_playlist(self.dir.parent().unwrap_or(&self.dir),&owned){
                let selected=self.track().and_then(|id|tracks.iter().position(|(next,_)|next==id)).unwrap_or(0);
                self.set_jukebox(&crate::frontend::jukebox::Change{tracks,selected,playing:self.music_enabled,preview:None});
            }
        }
        self.pump();
        // For the jukebox screen: what plays and how far through it is.
        let progress=self.music.as_ref().and_then(|(i,c,end)|Some((self.playlist.get(*i)?.0.as_str(),((c.secs-(end-now))/c.secs.max(1e-3)).clamp(0.0,1.0) as f32)));
        crate::frontend::jukebox::publish(progress);
        if self.music_volume <= 0.0 || !self.music_enabled || self.playlist.is_empty() {
            self.prepared_music=None;
            if let Some((_, c, _)) = self.music.take() {
                stop_sound(&c.sound);
            }
            return;
        }
        let n = self.playlist.len().max(1);
        let mut next = if let Some(index)=self.requested_track {index.min(n-1)} else {match &self.music {
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
            Some((i,_,end)) => {
                // One short lookahead window; do not retain a whole playlist in RAM.
                if n>1 && *end-now<2.0 && !self.shuffle && !self.loop_track {
                    let name=self.playlist[(i+1)%n].0.clone();self.request(&name,true);
                }
                return;
            },
            None => self.requested_track.unwrap_or(0).min(n-1),
        }};
        for _ in 0..n{
            if !self.failed_music.contains(&self.playlist[next].0){break;}
            next=(next+1)%n;
        }
        if self.failed_music.contains(&self.playlist[next].0){return;}
        // Keep shuffle/manual targets stable while the worker is decoding.
        self.requested_track=Some(next);
        if self.music.as_ref().is_some_and(|(i,_,_)|*i==next) && self.skip==0{
            let (_,c,end)=self.music.as_mut().unwrap();
            stop_sound(&c.sound);play_sound(&c.sound,PlaySoundParams{looped:false,volume:self.music_volume});
            *end=now+c.secs.max(1.0);self.requested_track=None;self.prepared_music=None;return;
        }
        let name=self.playlist[next].0.clone();
        if self.prepared_music.as_ref().is_some_and(|(n,_)|n!=&name){self.prepared_music=None;}
        if self.prepared_music.as_ref().is_none_or(|(n,_)|n!=&name){self.request(&name,true);return;}
        let prepared=self.prepared_music.take().unwrap().1;
        let c=self.install(prepared);
        self.skip=0;self.requested_track=None;
        if let Some((_,old,_))=self.music.take(){stop_sound(&old.sound);}
        play_sound(&c.sound,PlaySoundParams{looped:false,volume:self.music_volume});
        let end=now+c.secs.max(1.0);self.music=Some((next,c,end));
    }

    /// (clips loaded, clips missing, music track playing).
    pub fn stats(&self) -> (usize, usize, Option<&str>) {
        let ok = self.clips.values().filter(|c| c.is_some()).count();
        (ok, self.clips.len() - ok, self.track())
    }

    pub fn set_jukebox(&mut self,change:&crate::frontend::jukebox::Change){
        // A previewed Songs track plays on its own.
        let (playlist,selected)=match &change.preview{Some(p)=>(vec![p.clone()],0),None=>(change.tracks.clone(),change.selected)};
        let same_track=self.track().is_some_and(|id|playlist.get(selected).is_some_and(|(next,_)|next==id));
        if !same_track || !change.playing{
            if let Some((_,clip,_))=self.music.take(){stop_sound(&clip.sound);}
        }else if let Some((index,_,_))=self.music.as_mut(){*index=selected;}
        self.playlist=playlist;self.music_enabled=change.playing;
        self.requested_track=if same_track&&change.playing{None}else{Some(selected)};self.skip=0;
    }

    /// Jukebox previous / next.
    pub fn skip_track(&mut self, delta: i32) {
        self.skip = delta;
        self.requested_track=None;
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
