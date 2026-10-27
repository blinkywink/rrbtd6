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
    /// Main menu event buttons on the right, each hidden on its own (the rest close up).
    pub hide_race: bool,
    pub hide_boss: bool,
    pub hide_boss_rush: bool,
    pub hide_odyssey: bool,
    pub hide_contested_territory: bool,
    pub hide_social_season: bool,
    /// Start at the main menu, skipping the splash, loading and title screens.
    pub skip_startup: bool,
    pub hide_perks:bool,
    pub hide_double_cash:bool,
    pub hide_fast_track:bool,
    pub perks_enabled:bool,
    pub double_cash_enabled:bool,
    pub fast_track_enabled:bool,
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
            hide_legends: false, hide_store: false, hide_race: false, hide_boss: false, hide_boss_rush: false,
            hide_odyssey: false, hide_contested_territory: false, hide_social_season: false, skip_startup: false,hide_perks:false,hide_double_cash:false,hide_fast_track:false,
            perks_enabled:true,double_cash_enabled:false,fast_track_enabled:false }
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
        a.disable_hidden_features();
        a
    }

    pub fn disable_hidden_features(&mut self){
        if self.hide_perks{self.perks_enabled=false;}
        if self.hide_double_cash{self.double_cash_enabled=false;}
        if self.hide_fast_track{self.fast_track_enabled=false;}
    }

    /// The main menu event kinds turned off.
    pub fn hidden_events(&self) -> Vec<crate::frontend::home_events::EventKind> {
        use crate::frontend::home_events::EventKind as K;
        [(self.hide_race, K::Race), (self.hide_boss, K::Boss), (self.hide_boss_rush, K::BossRush), (self.hide_odyssey, K::Odyssey),
         (self.hide_contested_territory, K::ContestedTerritory), (self.hide_social_season, K::SocialSeason)]
            .into_iter().filter(|(on, _)| *on).map(|(_, k)| k).collect()
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
            Row::Toggle("Hide Shop Button", self.hide_store, "Hides the Shop button on the main menu. The Trophy Store moves up into its place."),
            Row::Toggle("Hide Race Button", self.hide_race, "Hides the race event button on the right of the main menu."),
            Row::Toggle("Hide Boss Button", self.hide_boss, "Hides the boss event button on the right of the main menu."),
            Row::Toggle("Hide Boss Rush Button", self.hide_boss_rush, "Hides the Boss Rush button on the right of the main menu."),
            Row::Toggle("Hide Odyssey Button", self.hide_odyssey, "Hides the Odyssey button on the right of the main menu."),
            Row::Toggle("Hide Contested Territory Button", self.hide_contested_territory,
                "Hides the Contested Territory button on the right of the main menu."),
            Row::Toggle("Hide Social Season Button", self.hide_social_season, "Hides the Social Season button on the right of the main menu."),
            Row::Toggle("Skip Startup Screen", self.skip_startup, "Go straight to the main menu instead of the startup screen."),
            Row::Toggle("Hide Perks Button",self.hide_perks,"Turns Perks off, then hides the Perks controls."),
            Row::Toggle("Hide Double Cash Button",self.hide_double_cash,"Turns Double Cash off, then hides its control."),
            Row::Toggle("Hide Fast Track Button",self.hide_fast_track,"Turns Fast Track off, then hides its control."),
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
            9 => self.hide_race = !self.hide_race,
            10 => self.hide_boss = !self.hide_boss,
            11 => self.hide_boss_rush = !self.hide_boss_rush,
            12 => self.hide_odyssey = !self.hide_odyssey,
            13 => self.hide_contested_territory = !self.hide_contested_territory,
            14 => self.hide_social_season = !self.hide_social_season,
            15 => self.skip_startup = !self.skip_startup,
            16 => self.hide_perks=!self.hide_perks,
            17 => self.hide_double_cash=!self.hide_double_cash,
            18 => self.hide_fast_track=!self.hide_fast_track,
            _ => {}
        }
        self.disable_hidden_features();
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

#[cfg(test)]
mod hidden_features_tests{
    use super::*;
    #[test]
    fn hiding_each_feature_switches_it_off_and_unhiding_does_not_reenable(){
        let mut a=Advanced::default();
        a.perks_enabled=true;a.double_cash_enabled=true;a.fast_track_enabled=true;
        for i in 16..=18{a.toggle(i);}
        assert!(!a.perks_enabled && !a.double_cash_enabled && !a.fast_track_enabled);
        for i in 16..=18{a.toggle(i);}
        assert!(!a.perks_enabled && !a.double_cash_enabled && !a.fast_track_enabled);
    }
    #[test]
    fn loading_old_or_inconsistent_preferences_cannot_enable_hidden_features(){
        let a=Advanced::load(&serde_json::json!({"advanced":{"hide_double_cash":true,"double_cash_enabled":true}}));
        assert!(a.hide_double_cash);assert!(!a.double_cash_enabled);
    }
}
