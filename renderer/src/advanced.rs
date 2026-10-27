//! Settings > Accessibility: the original AccessibilitySettingsUI options (effects scale, ambient
//! map effects, stereoscopic, range circle style, tower snapping, numerical ability timers, cash
//! spent tracker, memory optimization), then our own (not in BTD6, see DEVIATIONS.md): frame
//! rate, vsync, render scale and sharpening, anti-aliasing, texture quality and an FPS counter in
//! the window title. Stored in settings.json "advanced".

use serde::{Deserialize, Serialize};

/// FPS limit stops on its slider (0: unlimited, last).
pub const FPS_CAPS: [u32; 9] = [30, 60, 90, 120, 144, 165, 240, 360, 0];
/// MSAA sample count stops.
pub const MSAA: [i32; 4] = [1, 2, 4, 8];
/// Render scale range (fraction of native); above native costs speed for little gain.
pub const SCALE_MIN: f32 = 0.2;

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
#[serde(default)]
pub struct Advanced {
    // The original's (AccessibilitySettingsUI).
    /// EffectScaleControl: 0..=1.
    pub effects_scale: f32,
    pub ambient_fx: bool,
    /// StereoScaleControl: 0..=1.
    pub stereo: f32,
    /// AccessibleRangeCircles: 0 Default, 1 High Contrast, 2 Colour Blind, 3 both.
    pub range_circle: u8,
    pub tower_snapping: bool,
    pub numerical_timers: bool,
    pub cash_spent: bool,
    pub memory_opt: bool,
    // Ours.
    /// The frame rate in the window's title bar.
    pub show_fps: bool,
    /// Frames per second limit, 0 for none.
    pub fps_cap: u32,
    /// Wait for the display's refresh (applies on the next launch).
    pub vsync: bool,
    /// The 3D view is drawn at this fraction of the window's resolution, then scaled to it.
    pub render_scale: f32,
    /// Contrast-adaptive sharpening when the 3D view is scaled (FSR's RCAS idea).
    pub sharpen: bool,
    /// Multisample anti-aliasing samples (applies on the next launch).
    pub msaa: i32,
    /// Always load full-size textures (otherwise 4K map textures are halved on screens up to
    /// 2048 px, where it looks the same).
    pub full_textures: bool,
    /// Main menu: hide the Legends button (the Legends DLCs aren't supported).
    pub hide_legends: bool,
    /// Main menu: hide the Shop button.
    pub hide_store: bool,
    /// Main menu: hide the event buttons on the right (teams, contested territory, bosses, races...).
    pub hide_events: bool,
}

impl Default for Advanced {
    fn default() -> Self {
        Self {
            effects_scale: 1.0,
            ambient_fx: true,
            stereo: 1.0,
            range_circle: 0,
            tower_snapping: true,
            numerical_timers: false,
            cash_spent: false,
            memory_opt: false,
            show_fps: false, fps_cap: 0, vsync: true, render_scale: 1.0, sharpen: true, msaa: 4, full_textures: false,
            hide_legends: false, hide_store: false, hide_events: false }
    }
}

/// One of our settings rows: an on/off toggle, or a slider (position 0..=1, value text, stop
/// count: 0 for continuous); each with a description.
pub enum Row {
    Toggle(&'static str, bool, &'static str),
    Slider(&'static str, f32, String, usize, &'static str),
}

fn stop_t(i: usize, n: usize) -> f32 {
    i as f32 / (n - 1).max(1) as f32
}

fn stop_of(t: f32, n: usize) -> usize {
    (t.clamp(0.0, 1.0) * (n - 1) as f32).round() as usize
}

impl Advanced {
    pub fn load(settings: &serde_json::Value) -> Self {
        let mut a: Self = serde_json::from_value(settings["advanced"].clone()).unwrap_or_default();
        a.render_scale = a.render_scale.clamp(SCALE_MIN, 1.0);
        a.effects_scale = a.effects_scale.clamp(0.0, 1.0);
        a.stereo = a.stereo.clamp(0.0, 1.0);
        a.range_circle = a.range_circle.min(3);
        a
    }

    pub fn rows(&self) -> Vec<Row> {
        let fps_i = FPS_CAPS.iter().position(|&c| c == self.fps_cap).unwrap_or(FPS_CAPS.len() - 1);
        let msaa_i = MSAA.iter().position(|&c| c == self.msaa).unwrap_or(2);
        vec![
            Row::Toggle("Show FPS", self.show_fps, "Show the frame rate in the window's title bar."),
            Row::Slider("FPS Limit", stop_t(fps_i, FPS_CAPS.len()),
                if self.fps_cap == 0 { "Unlimited".into() } else { self.fps_cap.to_string() }, FPS_CAPS.len(),
                "Caps how many frames are drawn each second. Lower limits save power and keep fans quiet."),
            Row::Toggle("VSync", self.vsync, "Wait for the display's refresh to avoid tearing. Applies after a restart."),
            Row::Slider("Render Scale", (self.render_scale - SCALE_MIN) / (1.0 - SCALE_MIN),
                format!("{}%", (self.render_scale * 100.0).round()), 0,
                "Draws the 3D view at a fraction of the window's resolution, then scales it up. Lower is faster."),
            Row::Toggle("Sharpening", self.sharpen, "Sharpens the 3D view when Render Scale is below 100%."),
            Row::Slider("Anti-aliasing", stop_t(msaa_i, MSAA.len()),
                if self.msaa <= 1 { "Off".into() } else { format!("{}x", self.msaa) }, MSAA.len(),
                "Smooths jagged edges. Higher costs more. Applies after a restart."),
            Row::Toggle("Full Resolution Textures", self.full_textures,
                "Always load full-size map textures. Off halves 4K textures on screens up to 2048 pixels, where they look the same."),
            Row::Toggle("Hide Legends Button", self.hide_legends, "Hides Legends on the main menu. The Legends DLCs aren't supported."),
            Row::Toggle("Hide Shop Button", self.hide_store, "Hides the Shop button on the main menu."),
            Row::Toggle("Hide Event Buttons", self.hide_events, "Hides the event buttons on the right of the main menu."),
        ]
    }

    /// Toggle row `i` clicked.
    pub fn toggle(&mut self, i: usize) {
        match i {
            0 => self.show_fps = !self.show_fps,
            2 => self.vsync = !self.vsync,
            4 => self.sharpen = !self.sharpen,
            6 => self.full_textures = !self.full_textures,
            7 => self.hide_legends = !self.hide_legends,
            8 => self.hide_store = !self.hide_store,
            9 => self.hide_events = !self.hide_events,
            _ => {}
        }
    }

    /// Slider row `i` moved to `t` (0..=1).
    pub fn slide(&mut self, i: usize, t: f32) {
        match i {
            1 => self.fps_cap = FPS_CAPS[stop_of(t, FPS_CAPS.len())],
            // 5% steps.
            3 => self.render_scale = ((SCALE_MIN + t.clamp(0.0, 1.0) * (1.0 - SCALE_MIN)) * 20.0).round() / 20.0,
            5 => self.msaa = MSAA[stop_of(t, MSAA.len())],
            _ => {}
        }
    }
}
