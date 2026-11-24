// https://github.com/floooh/sokol/blob/master/sokol_audio.h
// https://github.com/norse-rs/audir/blob/master/audir/src/wasapi/mod.rs

use crate::PlaySoundParams;

pub use crate::mixer::Playback;

use winapi::shared::guiddef::{CLSID, IID};
use winapi::shared::ksmedia;
use winapi::shared::minwindef::*;
use winapi::shared::mmreg::*;
use winapi::um::audioclient::*;
use winapi::um::audiosessiontypes::*;
use winapi::um::combaseapi::*;
use winapi::um::mmdeviceapi::*;
use winapi::um::objbase::*;
use winapi::um::synchapi::*;
use winapi::um::winbase::*;

use std::sync::mpsc;

// thanks sokol_audio!
// https://github.com/floooh/sokol/blob/master/sokol_audio.h#L559
static IID_IAudioClient: IID = IID {
    Data1: 0x1cb9ad4c,
    Data2: 0xdbfa,
    Data3: 0x4c32,
    Data4: [0xb1, 0x78, 0xc2, 0xf5, 0x68, 0xa7, 0x03, 0xb2],
};
static IID_IMMDeviceEnumerator: IID = IID {
    Data1: 0xa95664d2,
    Data2: 0x9614,
    Data3: 0x4f35,
    Data4: [0xa7, 0x46, 0xde, 0x8d, 0xb6, 0x36, 0x17, 0xe6],
};
static CLSID_IMMDeviceEnumerator: CLSID = CLSID {
    Data1: 0xbcde0395,
    Data2: 0xe52f,
    Data3: 0x467c,
    Data4: [0x8e, 0x3d, 0xc4, 0x57, 0x92, 0x91, 0x69, 0x2e],
};
static IID_IAudioRenderClient: IID = IID {
    Data1: 0xf294acfc,
    Data2: 0x3146,
    Data3: 0x4483,
    Data4: [0xa7, 0xbf, 0xad, 0xdc, 0xa7, 0xc2, 0x60, 0xe2],
};
static IID_Devinterface_Audio_Render: IID = IID {
    Data1: 0xe6327cad,
    Data2: 0xdcec,
    Data3: 0x4949,
    Data4: [0xae, 0x8a, 0x99, 0x1e, 0x97, 0x6a, 0x79, 0xd2],
};
static IID_IActivateAudioInterface_Completion_Handler: IID = IID {
    Data1: 0x94ea2b94,
    Data2: 0xe9cc,
    Data3: 0x49e0,
    Data4: [0xc0, 0xff, 0xee, 0x64, 0xca, 0x8f, 0x5b, 0x90],
};

mod consts {
    pub const CHANNELS: u32 = 2;
    pub const SAMPLE_RATE: u32 = 44100;
    pub const BUFFER_FRAMES: u32 = 4096;
}

/// (rrbtd6) An open output: the default render device's shared-mode client.
struct Output {
    device: *mut IMMDevice,
    client: *mut IAudioClient,
    render: *mut IAudioRenderClient,
    frames: u32,
    id: Vec<u16>,
}

/// The default render device's id (changes when headphones are plugged in or the user switches
/// outputs).
unsafe fn default_id(enumerator: *mut IMMDeviceEnumerator) -> Option<Vec<u16>> {
    let mut device: *mut IMMDevice = std::ptr::null_mut();
    if (*enumerator).GetDefaultAudioEndpoint(eRender, eConsole, &mut device) < 0 || device.is_null() {
        return None;
    }
    let id = device_id(device);
    (*device).Release();
    id
}

unsafe fn device_id(device: *mut IMMDevice) -> Option<Vec<u16>> {
    let mut wide: winapi::shared::ntdef::LPWSTR = std::ptr::null_mut();
    if (*device).GetId(&mut wide) < 0 || wide.is_null() {
        return None;
    }
    let mut n = 0;
    while *wide.add(n) != 0 {
        n += 1;
    }
    let id = std::slice::from_raw_parts(wide, n).to_vec();
    CoTaskMemFree(wide as _);
    Some(id)
}

unsafe fn close(out: Output) {
    (*out.client).Stop();
    (*out.render).Release();
    (*out.client).Release();
    (*out.device).Release();
}

/// Opens the default render device for 44.1 kHz stereo float (shared mode, converted by Windows);
/// None when there is no usable device.
unsafe fn open(enumerator: *mut IMMDeviceEnumerator, event: winapi::um::winnt::HANDLE) -> Option<Output> {
    let mut device: *mut IMMDevice = std::ptr::null_mut();
    if (*enumerator).GetDefaultAudioEndpoint(eRender, eConsole, &mut device) < 0 || device.is_null() {
        return None;
    }
    let mut state = 0;
    (*device).GetState(&mut state);
    let mut client: *mut IAudioClient = std::ptr::null_mut();
    if state & DEVICE_STATE_ACTIVE == 0
        || (*device).Activate(&IID_IAudioClient, CLSCTX_ALL, std::ptr::null_mut(), &mut client as *mut _ as _) < 0
        || client.is_null()
    {
        (*device).Release();
        return None;
    }
    let format = WAVEFORMATEX {
        nChannels: consts::CHANNELS as _,
        nSamplesPerSec: consts::SAMPLE_RATE as _,
        wFormatTag: WAVE_FORMAT_EXTENSIBLE,
        wBitsPerSample: 32,
        nBlockAlign: consts::CHANNELS as u16 * 4,
        nAvgBytesPerSec: consts::CHANNELS as u32 * consts::SAMPLE_RATE as u32 * 4,
        cbSize: (std::mem::size_of::<WAVEFORMATEXTENSIBLE>() - std::mem::size_of::<WAVEFORMATEX>()) as _,
    };
    const FRONT_LEFT: u32 = 0b0001;
    const FRONT_RIGHT: u32 = 0b0010;
    let format_extensible = WAVEFORMATEXTENSIBLE {
        Format: format,
        Samples: 4 * 8,
        dwChannelMask: FRONT_LEFT | FRONT_RIGHT,
        SubFormat: ksmedia::KSDATAFORMAT_SUBTYPE_IEEE_FLOAT,
    };
    // https://docs.microsoft.com/en-us/windows/win32/coreaudio/audclnt-streamflags-xxx-constants
    const AUDCLNT_STREAMFLAGS_AUTOCONVERTPCM: u32 = 0x80000000;
    const AUDCLNT_STREAMFLAGS_SRC_DEFAULT_QUALITY: u32 = 0x08000000;
    let dur = consts::BUFFER_FRAMES as f64 / (consts::SAMPLE_RATE as f64 * 1.0 / 10000000.0);
    let mut frames = 0;
    let mut render: *mut IAudioRenderClient = std::ptr::null_mut();
    let ok = (*client).Initialize(
        AUDCLNT_SHAREMODE_SHARED,
        AUDCLNT_STREAMFLAGS_EVENTCALLBACK | AUDCLNT_STREAMFLAGS_AUTOCONVERTPCM | AUDCLNT_STREAMFLAGS_SRC_DEFAULT_QUALITY,
        dur as _,
        0,
        &format_extensible as *const _ as _,
        std::ptr::null(),
    ) >= 0
        && (*client).GetBufferSize(&mut frames) >= 0
        && (*client).GetService(&IID_IAudioRenderClient, &mut render as *mut _ as _) >= 0
        && !render.is_null()
        && (*client).SetEventHandle(event) >= 0;
    let Some(id) = device_id(device).filter(|_| ok) else {
        if !render.is_null() {
            (*render).Release();
        }
        (*client).Release();
        (*device).Release();
        return None;
    };
    (*client).Start();
    Some(Output { device, client, render, frames, id })
}

/// (rrbtd6) The output thread survives having no device and device changes: without a device the
/// mixer keeps running silently in real time (sounds and music stay where they would be) and the
/// device is tried again every 2 s; a device that goes away (unplugged headset, disabled card) is
/// reopened, and a change of the default output (checked every second) moves to the new one.
/// RRBTD6_NO_AUDIO_DEVICE: run as if there were none (testing).
unsafe fn audio_thread(mut mixer: crate::mixer::Mixer) {
    use std::time::{Duration, Instant};
    const WAIT_OBJECT_0: u32 = 0;
    CoInitializeEx(std::ptr::null_mut(), COINIT_MULTITHREADED);
    let event = CreateEventA(std::ptr::null_mut(), FALSE, FALSE, std::ptr::null());
    let mut enumerator: *mut IMMDeviceEnumerator = std::ptr::null_mut();
    let have_api = !event.is_null()
        && CoCreateInstance(&CLSID_IMMDeviceEnumerator, std::ptr::null_mut(), CLSCTX_ALL, &IID_IMMDeviceEnumerator, &mut enumerator as *mut _ as _) >= 0
        && !enumerator.is_null()
        && std::env::var_os("RRBTD6_NO_AUDIO_DEVICE").is_none();
    // Silent running: this many frames per sleep (~23 ms at 44.1 kHz).
    const SILENT_FRAMES: usize = 1024;
    let mut scratch = vec![0f32; SILENT_FRAMES * consts::CHANNELS as usize];
    let mut out: Option<Output> = None;
    let mut next_try = Instant::now();
    let mut last_check = Instant::now();
    let mut reported = false;
    loop {
        if out.is_none() && have_api && Instant::now() >= next_try {
            out = open(enumerator, event);
            if out.is_none() {
                if !reported {
                    eprintln!("audio: no usable output device; running silently and retrying");
                    reported = true;
                }
                next_try = Instant::now() + Duration::from_secs(2);
            } else {
                reported = false;
            }
        }
        let Some(o) = out.as_ref() else {
            std::thread::sleep(Duration::from_millis(23));
            mixer.fill_audio_buffer(&mut scratch, SILENT_FRAMES);
            continue;
        };
        let waited = WaitForSingleObject(event, 200);
        if last_check.elapsed() >= Duration::from_secs(1) {
            last_check = Instant::now();
            if default_id(enumerator).as_ref() != Some(&o.id) {
                close(out.take().unwrap());
                next_try = Instant::now();
                continue;
            }
        }
        if waited != WAIT_OBJECT_0 {
            continue;
        }
        let mut padding = 0;
        if (*o.client).GetCurrentPadding(&mut padding) < 0 {
            // AUDCLNT_E_DEVICE_INVALIDATED and the like: the device went away.
            close(out.take().unwrap());
            next_try = Instant::now() + Duration::from_millis(500);
            continue;
        }
        let num_frames = o.frames.saturating_sub(padding);
        if num_frames == 0 {
            continue;
        }
        let mut wasapi_buffer: *mut u8 = std::ptr::null_mut();
        if (*o.render).GetBuffer(num_frames, &mut wasapi_buffer) < 0 || wasapi_buffer.is_null() {
            close(out.take().unwrap());
            next_try = Instant::now() + Duration::from_millis(500);
            continue;
        }
        let buffer = std::slice::from_raw_parts_mut(wasapi_buffer as *mut f32, num_frames as usize * consts::CHANNELS as usize);
        mixer.fill_audio_buffer(buffer, num_frames as _);
        (*o.render).ReleaseBuffer(num_frames, 0);
    }
}

pub struct AudioContext {
    pub(crate) mixer_ctrl: crate::mixer::MixerControl,
}

impl AudioContext {
    pub fn new() -> AudioContext {
        use crate::mixer::Mixer;

        let (mixer_builder, mixer_ctrl) = Mixer::new();
        std::thread::spawn(move || unsafe {
            audio_thread(mixer_builder.build());
        });

        AudioContext { mixer_ctrl }
    }
}

pub struct Sound {
    sound_id: u32,
}

impl Sound {
    /// Queue already decoded 44100 Hz stereo PCM; no codec runs on this thread.
    pub fn from_samples(ctx: &AudioContext, samples: std::sync::Arc<[f32]>) -> Sound {
        Sound { sound_id: ctx.mixer_ctrl.load_samples(samples) }
    }

    pub fn load(ctx: &AudioContext, data: &[u8]) -> Sound {
        let sound_id = ctx.mixer_ctrl.load(data);

        Sound { sound_id }
    }

    pub fn play(&self, ctx: &AudioContext, params: PlaySoundParams) -> Playback {
        ctx.mixer_ctrl.play(self.sound_id, params)
    }

    pub fn stop(&self, ctx: &AudioContext) {
        ctx.mixer_ctrl.stop_all(self.sound_id);
    }

    pub fn set_volume(&self, ctx: &AudioContext, volume: f32) {
        ctx.mixer_ctrl.set_volume_all(self.sound_id, volume);
    }

    pub fn delete(&self, ctx: &AudioContext) {
        ctx.mixer_ctrl.delete(self.sound_id);
    }
}
