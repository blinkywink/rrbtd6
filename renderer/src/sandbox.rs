//! Sandbox mode's in-game bloon menu (SandboxBloonsMenu prefab, opened with the shop's
//! ToggleBloons button): a button per bloon type, camo / regrow / fortified toggles, Count and
//! Spacing fields, Set Round, and destroy bloons / monkeys and reset cooldowns / damage.
//! It slides in and out with its SandBoxSlideOut animator (BloonMenu.Show / Hide), and stays
//! slid out during tower placement (BloonMenu.hideExternal). Scene hits close it.

use crate::hud::{Action, Hud};
use crate::uitree::{draw_tree, Ctx, Node, R};
use macroquad::prelude::*;
use std::path::Path;

const MENU: &str = "SandboxContainer/Layout";
const LIST: &str = "SandboxContainer/Layout/BloonsScroll/Viewport/BloonList";

/// Exported animator bindings include the prefab root, unlike Node-relative paths.
/// Normalize parsed values so pretty and compact JSON work identically.
fn relative_runtime_paths(value: &mut serde_json::Value) {
    match value {
        serde_json::Value::Object(fields) => {
            if let Some(path) = fields.get_mut("path") {
                if let Some(relative) = path.as_str().and_then(|p| p.strip_prefix("SandboxBloonsMenu/")) {
                    *path = serde_json::Value::String(relative.to_owned());
                }
            }
            for child in fields.values_mut() { relative_runtime_paths(child); }
        }
        serde_json::Value::Array(items) => {
            for child in items { relative_runtime_paths(child); }
        }
        _ => {}
    }
}
/// Long enough for the second press of a double-click.
const DOUBLE_CLICK_SECONDS: f64 = 0.4;

/// The bloon buttons, in the game's order (BloonMenu: base types; the toggles add variants).
/// TestBloon / TestBloonMoab (999999 / 100000000 health, no leak damage) open the list (owner, from the game).
pub const BLOONS: [&str; 19] = [
    "TestBloon", "TestBloonMoab", "Red", "Blue", "Green", "Yellow", "Pink", "Black", "White", "Purple", "Lead", "Zebra", "Rainbow", "Ceramic",
    "Moab", "Bfb", "Zomg", "Ddt", "Bad",
];

/// What the menu asks the game to do.
pub enum SandboxAction {
    Spawn { bloon: String, camo: bool, regrow: bool, fortified: bool, count: u32, spacing: f32 },
    SetRound(usize),
    SetCash(f64),
    SetLives(f64),
    ClearBloons,
    ClearTowers,
    ResetCooldowns,
    ResetDamage,
}

impl SandboxAction {
    pub fn apply(self, game: &mut crate::game::Game) {
        match self {
            SandboxAction::Spawn { bloon, camo, regrow, fortified, count, spacing } => {
                game.sandbox_spawn(&bloon, camo, regrow, fortified, count, spacing);
            }
            SandboxAction::SetRound(n) => game.set_round(n),
            // InGame.OnSetCash / OnSetSandboxHealth (UnityToSimulation.SetCash / SetSandboxHealth):
            // the value as typed, and health's becomes what it refills to.
            SandboxAction::SetCash(n) if game.sandbox => game.cash=n,
            SandboxAction::SetLives(n) if game.sandbox => {game.lives=n;game.last_set_health=n;game.game_over=false;},
            SandboxAction::SetCash(_) | SandboxAction::SetLives(_) => {},
            SandboxAction::ClearBloons => game.clear_bloons(),
            SandboxAction::ClearTowers => game.clear_towers(),
            SandboxAction::ResetCooldowns => game.reset_cooldowns(),
            SandboxAction::ResetDamage => game.reset_damage(),
        }
    }
}

#[derive(Clone, Copy, PartialEq)]
enum Field {
    Count,
    Spacing,
    Round,
}

#[derive(Clone,Copy,Debug,PartialEq)]
pub enum Resource {Cash,Lives}
/// The open SetValuePopup (PopupScreen.ShowSetValuePopup): its InputField's text.
struct ResourceEdit {kind:Resource,text:String,replace:bool,opened:f64}
impl ResourceEdit {
    /// ShowSetValuePopup's confirm: Int32.TryParse of the text, 0 when it doesn't parse.
    fn value(&self)->i32 {self.text.parse::<i32>().unwrap_or(0)}
    fn action(&self)->SandboxAction {
        let value=self.value() as f64;
        match self.kind{Resource::Cash=>SandboxAction::SetCash(value),Resource::Lives=>SandboxAction::SetLives(value)}
    }
}
/// InGame.lastSetCash / lastSetHealth before anything is set (InGame..ctor).
const LAST_SET_DEFAULT: i32 = 9999999;
/// The InputField's m_CharacterLimit (SetValuePopup).
const VALUE_LIMIT: usize = 7;
const POPUP: &str = "AnimatedObject";
/// Popup.TransitionAnim.Scale: seconds the popup takes to scale in (as the Sun Temple popup).
const POPUP_OPEN_SECS: f64 = 0.25;
pub struct SandboxUi {
    menu: Option<Node>,
    resource:Option<ResourceEdit>,
    /// SetValuePopup prefab.
    popup: Option<Node>,
    /// InGame.lastSetCash / lastSetHealth: the popup's starting value.
    last_cash: i32,
    last_health: i32,
    bloons:Vec<String>,
    pub open: bool,
    /// When the menu was last open, so the rest of a double-click that closed it doesn't reach the map.
    last_open: f64,
    camo: bool,
    regrow: bool,
    fortified: bool,
    count: String,
    spacing: String,
    round: String,
    /// The text field being typed into.
    editing: Option<Field>,
    /// SandboxBloonsMenu's animators (ui/runtime), paths relative to the menu.
    runtime: serde_json::Value,
    /// Seconds into SandboxSlideOutOpen: 0 slid out, SLIDE in; it runs back to hide.
    slide: f32,
    scroll: f32,
    /// The variant toggles the button icons were last set for.
    icons_for: Option<(bool, bool, bool)>,
}

/// The button icon of `name` with these variants: BloonModel.icon of the closest variant that
/// exists (CeramicRegrowFortifiedCamo, ...): "Bloons[Red]", "UiBloonIcons[MoabIcon]"...
fn bloon_icon(game: &crate::game::Game, name: &str, camo: bool, regrow: bool, fortified: bool) -> Option<String> {
    [(regrow, fortified, camo), (false, fortified, camo), (regrow, false, camo), (regrow, fortified, false),
        (false, false, camo), (false, fortified, false), (regrow, false, false), (false, false, false)]
        .into_iter()
        .map(|(r, f, c)| format!("{name}{}{}{}", if r { "Regrow" } else { "" }, if f { "Fortified" } else { "" }, if c { "Camo" } else { "" }))
        .find_map(|v| game.bloon_sprite(&v))
        .map(str::to_string)
}

impl SandboxUi {
    /// BloonMenu.Initialise subscribes this to InputManager.OnSceneHit.
    /// The menu is open, or closed so recently that this press is the same double-click.
    pub fn owns_map_press(&self, now: f64) -> bool {
        self.open || now - self.last_open < DOUBLE_CLICK_SECONDS
    }

    pub fn scene_hit(&mut self) {
        self.open = false;
        self.editing = None;
    }

    pub fn load(prefabs: &Path) -> Self {
        let mut menu = Node::load(&prefabs.join("SandboxBloonsMenu.json"));
        // The prefab export names bindings from the menu itself; Node paths start below it.
        let runtime = prefabs.parent().map(|ui| ui.join("runtime/SandboxBloonsMenu.json"))
            .and_then(|p| std::fs::read_to_string(p).ok())
            .and_then(|text| serde_json::from_str(&text).ok())
            .map(|mut value| { relative_runtime_paths(&mut value); value })
            .unwrap_or_default();
        let button = Node::load(&prefabs.join("SpawnBloonButton.json"));
        if let Some(m) = menu.as_mut() {
            // "Place crosspaths" and the variants list aren't part of the plain sandbox menu.
            m.set_active("PlaceCrosspthsBtn", false);
            m.set_active("SandboxContainer/ShowVariants", false);
            if let (Some(list), Some(b)) = (m.find_mut(LIST), button) {
                list.children = BLOONS
                    .iter()
                    .map(|name| {
                        let mut n = b.clone();
                        n.name = name.to_string();
                        // The Icon Image stretches the sprite's full rect over itself
                        // (m_PreserveAspect 0), as SpawnBloonButton.UpdateIcon leaves it.
                        n.set_active("RawIcon", false);
                        n
                    })
                    .collect();
            }
        }
        Self {
            menu,
            resource:None,
            popup: Node::load(&prefabs.join("SetValuePopup.json")),
            last_cash: LAST_SET_DEFAULT,
            last_health: LAST_SET_DEFAULT,
            bloons:BLOONS.iter().map(|s|s.to_string()).collect(),
            open: false,
            last_open: f64::NEG_INFINITY,
            camo: false,
            regrow: false,
            fortified: false,
            count: "1".into(),
            spacing: "1".into(),
            round: "1".into(),
            editing: None,
            runtime,
            slide: 0.0,
            scroll: 0.0,
            icons_for: None,
        }
    }

    /// InGame.CheatSetCash / CheatSetHealth: the Set Cash / Set Health popup, starting from the
    /// value last set (not the current one). InputFieldPopup.Init selects the field, so typing
    /// replaces that value.
    pub fn edit_resource(&mut self,kind:Resource,game:&crate::game::Game){
        if !game.sandbox||self.popup.is_none(){return;}
        self.editing=None;
        let last=match kind{Resource::Cash=>self.last_cash,Resource::Lives=>self.last_health};
        self.resource=Some(ResourceEdit{kind,text:last.to_string(),replace:true,opened:get_time()});
    }
    /// Spawning `bloon` with the menu's current toggles, count and spacing (the Send hotkeys).
    pub fn spawn(&self, bloon: &str) -> SandboxAction {
        SandboxAction::Spawn {
            bloon: bloon.into(),
            camo: self.camo,
            regrow: self.regrow,
            fortified: self.fortified,
            count: self.count.parse().unwrap_or(1u32).clamp(1, 9999),
            spacing: self.spacing.parse().unwrap_or(1.0),
        }
    }

    /// A text field has keyboard focus (hotkeys are off meanwhile).
    pub fn typing(&self) -> bool {
        self.resource.is_some() || (self.open && self.editing.is_some())
    }
}

#[cfg(test)]
mod runtime_tests {
    use super::*;

    #[test]
    fn a_double_click_that_closes_the_menu_stays_off_the_map() {
        let mut ui = SandboxUi::load(Path::new("no-such-folder"));
        assert!(!ui.owns_map_press(100.0), "never opened");
        ui.open = true;
        assert!(ui.owns_map_press(100.0));
        ui.last_open = 100.0;
        ui.scene_hit();
        assert!(ui.owns_map_press(100.2), "second press of the double-click");
        assert!(!ui.owns_map_press(100.5), "a later click selects as usual");
    }

    #[test]
    fn set_value_popup_parses_like_int32_try_parse(){
        let edit=|kind,text:&str|ResourceEdit{kind,text:text.into(),replace:false,opened:0.0};
        assert!(matches!(edit(Resource::Cash,"").action(),SandboxAction::SetCash(v) if v==0.0));
        assert!(matches!(edit(Resource::Cash,"-").action(),SandboxAction::SetCash(v) if v==0.0));
        assert!(matches!(edit(Resource::Lives,"-25").action(),SandboxAction::SetLives(v) if v==-25.0));
        assert!(matches!(edit(Resource::Lives,"9999999").action(),SandboxAction::SetLives(v) if v==9999999.0));
    }
    #[test]
    fn animation_bindings_are_independent_of_json_whitespace() {
        let source = serde_json::json!({"animators": [{
            "path": "SandboxBloonsMenu/Mask",
            "clips": [{"bindings": [{"path": "SandboxBloonsMenu/SandboxContainer", "start": [975.0]}]}]
        }], "description": "SandboxBloonsMenu/keep this text", "other": {"path": "OtherMenu/Mask"}});
        let mut pretty: serde_json::Value = serde_json::from_str(&serde_json::to_string_pretty(&source).unwrap()).unwrap();
        let mut compact: serde_json::Value = serde_json::from_str(&serde_json::to_string(&source).unwrap()).unwrap();
        relative_runtime_paths(&mut pretty);
        relative_runtime_paths(&mut compact);
        assert_eq!(pretty, compact);
        assert_eq!(compact["animators"][0]["path"], "Mask");
        assert_eq!(compact["animators"][0]["clips"][0]["bindings"][0]["path"], "SandboxContainer");
        assert_eq!(compact["description"], source["description"]);
        assert_eq!(compact["other"], source["other"]);
    }
}

/// The field's InputField node.
fn field_path(f: Field) -> &'static str {
    match f {
        Field::Count => "SandboxContainer/Layout/TopInfo/BloonCount",
        Field::Spacing => "SandboxContainer/Layout/TopInfo/BloonRate",
        Field::Round => "SandboxContainer/Layout/TopInfo/RoundSpawn",
    }
}

impl Hud {
    /// Sandbox: clicking the cash / health counters opens Set Cash / Set Health.
    pub fn sandbox_resources(&mut self,game:&crate::game::Game,_out:&mut Vec<Action>){
        if !game.sandbox{self.sandbox.resource=None;return;}
        if self.sandbox.resource.is_some(){return;}
        // Each group's Button also takes clicks on its raycast-target children: the number
        // (TextMeshPro, its whole rect) and the coin icon.
        const HITS:[(&str,Resource);5]=[
            ("LeftGroup/LayoutGroup/CashGroup",Resource::Cash),("LeftGroup/LayoutGroup/CashGroup/Cash",Resource::Cash),
            ("LeftGroup/LayoutGroup/CashGroup/CashIcon",Resource::Cash),
            ("LeftGroup/LayoutGroup/HealthGroup",Resource::Lives),("LeftGroup/LayoutGroup/HealthGroup/Health",Resource::Lives)];
        if let Some((_,kind))=HITS.into_iter().find(|(path,_)|self.left.clicked(path)){
            self.sandbox.edit_resource(kind,game);
        }
    }
    /// The open SetValuePopup over everything (modal: other HUD actions this frame are dropped).
    /// Title / Description are "Set Cash" / "Set Cash Description" (or Health), the buttons OK /
    /// Cancel. The InputField takes an integer (m_CharacterValidation Integer: digits, a leading
    /// minus) of at most 7 characters. OK gives the value to InGame.OnSetCash / OnSetSandboxHealth,
    /// which remember it for next time.
    pub fn draw_set_value(&mut self, full: R, canvas_h: f32, s: f32, out: &mut Vec<Action>) {
        let Some(edit)=self.sandbox.resource.as_mut() else{return};
        out.clear();
        while let Some(c)=get_char_pressed(){
            if c.is_ascii_digit()||c=='-'{
                if edit.replace{edit.text.clear();edit.replace=false;}
                if edit.text.len()<VALUE_LIMIT&&(c!='-'||edit.text.is_empty()){edit.text.push(c);}
            }
        }
        if is_key_pressed(KeyCode::Backspace){if edit.replace{edit.text.clear();edit.replace=false;}else{edit.text.pop();}}
        let (kind,opened)=(edit.kind,edit.opened);
        let caret=if !edit.replace&&(get_time()*2.0) as i64%2==0{"|"}else{""};
        let field=format!("{}{caret}",edit.text);
        let (title,body)=match kind{Resource::Cash=>("Set Cash","Set Cash Description"),Resource::Lives=>("Set Health","Set Health Description")};
        let text=|k:&str|self.menus.loc.get(k).cloned().unwrap_or_else(||k.to_string()).replace("\\n","\n");
        let (title,body,ok,cancel)=(text(title),text(body),text("OK"),text("Cancel"));
        let k=((get_time()-opened)/POPUP_OPEN_SECS).clamp(0.0,1.0) as f32;
        let Some(n)=self.sandbox.popup.as_mut() else{self.sandbox.resource=None;return};
        n.set_text(&format!("{POPUP}/Title"),&title);
        n.set_text(&format!("{POPUP}/Description"),&body);
        n.set_text(&format!("{POPUP}/Confirm/Text"),&ok);
        n.set_text(&format!("{POPUP}/Cancel/Text"),&cancel);
        n.set_text(&format!("{POPUP}/InputField/Text Area/Text"),&field);
        // Scale in with a little overshoot.
        let back=1.70158;
        let e=1.0+(back+1.0)*(k-1.0).powi(3)+back*(k-1.0).powi(2);
        if let Some(a)=n.find_mut(POPUP){a.rt.scale=vec2(e,e);}
        if k>=1.0{
            // Enter also confirms (DEVIATIONS.md: the game's InputField doesn't submit on Enter).
            if n.clicked(&format!("{POPUP}/Confirm"))||is_key_pressed(KeyCode::Enter)||is_key_pressed(KeyCode::KpEnter){
                if let Some(edit)=self.sandbox.resource.take(){
                    match edit.kind{Resource::Cash=>self.sandbox.last_cash=edit.value(),Resource::Lives=>self.sandbox.last_health=edit.value()}
                    out.push(Action::Sandbox(edit.action()));
                }
            }else if n.clicked(&format!("{POPUP}/Cancel"))||is_key_pressed(KeyCode::Escape){
                self.sandbox.resource=None;
            }
        }
        draw_rectangle(0.0,0.0,screen_width(),screen_height(),Color::new(0.0,0.0,0.0,0.5*k));
        let mut ctx=Ctx{sprites:&mut self.sprites,fonts:&self.fonts,scale:s,canvas_h,clip:None};
        draw_tree(n,full,&mut ctx);
    }
    /// The sandbox bloon menu over the shop; returns the game actions clicked.
    /// `hide` is temporary placement blocking, preserving the user's open state.
    pub fn draw_sandbox(&mut self, game: &crate::game::Game, full: R, canvas_h: f32, s: f32, hide: bool, out: &mut Vec<Action>) {
        let ui = &mut self.sandbox;
        if ui.open {
            ui.last_open = get_time();
        }
        // BloonMenu.Update: shown while toggled on and not hidden by another menu; the Open clip
        // runs forward to show and back to hide.
        let shown = ui.open && !hide;
        let slide_len = ui.runtime["animators"].as_array().into_iter().flatten()
            .filter(|a| a["controller"] == "SandBoxSlideOut")
            .flat_map(|a| a["clips"].as_array().into_iter().flatten())
            .find(|c| c["name"] == "SandboxSlideOutOpen")
            .map_or(0.0, |c| (c["stop"].as_f64().unwrap_or(0.0) - c["start"].as_f64().unwrap_or(0.0)) as f32);
        let dt = get_frame_time().min(0.1);
        ui.slide = if slide_len <= 0.0 { if shown { 1.0 } else { 0.0 } } else if shown { (ui.slide + dt).min(slide_len) } else { (ui.slide - dt).max(0.0) };
        if !shown {
            ui.editing = None;
        }
        if !shown && ui.slide <= 0.0 {
            return;
        }
        let Some(m) = ui.menu.as_mut() else { return };
        if ui.icons_for.is_none(){
            if let Some(list)=m.find_mut(LIST){if let Some(template)=list.children.first().cloned(){
                for name in game.custom_bloons(){if !ui.bloons.contains(&name){let mut button=template.clone();button.name=name.clone();list.children.push(button);ui.bloons.push(name);}}
            }}
        }

        if slide_len > 0.0 {
            crate::frontend::apply_animation(m, &ui.runtime, "SandBoxSlideOut", "SandboxSlideOutOpen", Some(ui.slide));
        }
        // Text fields: click to edit, type digits (and "." for spacing), Enter / click away ends.
        let clicked_field = [Field::Count, Field::Spacing, Field::Round].into_iter().find(|&f| m.clicked(field_path(f)));
        if is_mouse_button_pressed(MouseButton::Left) {
            ui.editing = clicked_field;
            if let Some(f) = clicked_field {
                match f {
                    Field::Count => ui.count.clear(),
                    Field::Spacing => ui.spacing.clear(),
                    Field::Round => ui.round.clear(),
                }
            }
        }
        if let Some(f) = ui.editing {
            let text = match f {
                Field::Count => &mut ui.count,
                Field::Spacing => &mut ui.spacing,
                Field::Round => &mut ui.round,
            };
            while let Some(c) = get_char_pressed() {
                if (c.is_ascii_digit() || (c == '.' && f == Field::Spacing && !text.contains('.'))) && text.len() < 6 {
                    text.push(c);
                }
            }
            if is_key_pressed(KeyCode::Backspace) {
                text.pop();
            }
            if is_key_pressed(KeyCode::Enter) || is_key_pressed(KeyCode::KpEnter) || is_key_pressed(KeyCode::Escape) {
                ui.editing = None;
            }
        }
        if ui.editing.is_none() {
            // Empty fields fall back to the defaults.
            for (t, d) in [(&mut ui.count, "1"), (&mut ui.spacing, "1"), (&mut ui.round, "1")] {
                if t.is_empty() {
                    *t = d.into();
                }
            }
        }
        let caret = |f: Field, t: &str| if ui.editing == Some(f) && (get_time() * 2.0) as i64 % 2 == 0 { format!("{t}|") } else { t.to_string() };
        m.set_text(&format!("{}/Text Area/Text", field_path(Field::Count)), &caret(Field::Count, &ui.count));
        m.set_text(&format!("{}/Text Area/Text", field_path(Field::Spacing)), &caret(Field::Spacing, &ui.spacing));
        m.set_text(&format!("{}/Text Area/Text", field_path(Field::Round)), &caret(Field::Round, &ui.round));
        let count: u32 = ui.count.parse().unwrap_or(1).clamp(1, 9999);
        let spacing: f32 = ui.spacing.parse().unwrap_or(1.0);
        // Variant toggles.
        for (name, flag) in [("CamoToggle", &mut ui.camo), ("Regen Toggle", &mut ui.regrow), ("Fortified Toggle", &mut ui.fortified)] {
            let p = format!("{MENU}/Toggles/{name}");
            if m.clicked(&p) {
                *flag = !*flag;
            }
            // Toggle.graphic is the On image, drawn over Off.
            m.set_active(&format!("{p}/On"), *flag);
        }
        let key = (ui.camo, ui.regrow, ui.fortified);
        if ui.icons_for != Some(key) {
            ui.icons_for = Some(key);
            for name in &ui.bloons {
                let icon = bloon_icon(game, name, ui.camo, ui.regrow, ui.fortified);
                m.set_sprite(&format!("{LIST}/{name}/Icon"), icon.as_deref());
            }
        }
        let mut acts = Vec::new();
        if !shown {
            // Sliding out: drawn, not used.
            let mut ctx = Ctx { sprites: &mut self.sprites, fonts: &self.fonts, scale: s, canvas_h, clip: None };
            if let Some(m) = self.sandbox.menu.as_mut() {
                draw_tree(m, full, &mut ctx);
            }
            return;
        }
        for name in &ui.bloons {
            if m.clicked(&format!("{LIST}/{name}")) {
                acts.push(SandboxAction::Spawn { bloon: name.into(), camo: ui.camo, regrow: ui.regrow, fortified: ui.fortified, count, spacing });
            }
        }
        if m.clicked(&format!("{}/Go", field_path(Field::Round))) {
            acts.push(SandboxAction::SetRound(ui.round.parse().unwrap_or(1)));
        }
        // BloonMenu.TogglePowers (@0x180b5e500): SBPowersBtn opens or closes the Powers menu.
        // Not done: its powersAnimator icon flip.
        let powers = m.clicked("Mask/ButtonStack/SBPowersBtnContainer/SBPowersBtn");
        for (b, a) in [
            ("Mask/ButtonStack/DestroyBloonsBtn", SandboxAction::ClearBloons),
            ("Mask/ButtonStack/DestroyMonkeysBtn", SandboxAction::ClearTowers),
            ("Mask/ButtonStack/ResetCooldownsBtn", SandboxAction::ResetCooldowns),
            ("Mask/ButtonStack/ResetDamageBtn", SandboxAction::ResetDamage),
        ] {
            if m.clicked(b) {
                acts.push(a);
            }
        }
        // ScrollRect BloonsScroll: m_ScrollSensitivity 100 canvas units per wheel notch (miniquad
        // reports 120 per notch), over a 3-column grid of 180-unit cells.
        let view = m.rect("SandboxContainer/Layout/BloonsScroll");
        let over = view.is_some_and(|r| r.contains(mouse_position().into()));
        let wheel = crate::hud::wheel_notches();
        if over && wheel != 0.0 {
            let content = ui.bloons.len().div_ceil(3) as f32 * 180.0;
            let max = view.map_or(0.0, |r| (content - r.h / s).max(0.0));
            ui.scroll = (ui.scroll - wheel * 100.0).clamp(0.0, max);
        }
        if let Some(l) = m.find_mut(LIST) {
            l.rt.pos.y = ui.scroll;
        }
        let mut ctx = Ctx { sprites: &mut self.sprites, fonts: &self.fonts, scale: s, canvas_h, clip: None };
        if let Some(m) = self.sandbox.menu.as_mut() {
            draw_tree(m, full, &mut ctx);
        }
        out.extend(acts.into_iter().map(Action::Sandbox));
        if powers {
            self.toggle_powers();
        }
    }

    /// Screen rect of the open sandbox menu (map clicks under it are ignored).
    pub fn sandbox_blocks(&self, p: Vec2) -> bool {
        self.sandbox.open
            && self.sandbox.slide > 0.0
            && self.sandbox.menu.as_ref().is_some_and(|m| {
                // (All of it: the top row with Set Round sticks out of its container.)
                fn any_over(n: &crate::uitree::Node, p: Vec2) -> bool {
                    n.active && (n.screen.contains(p) || n.children.iter().any(|c| any_over(c, p)))
                }
                ["SandboxContainer", "Mask"].iter().any(|n| m.rect(n).is_some_and(|r| r.contains(p)) || m.find(n).is_some_and(|x| any_over(x, p)))
            })
    }
}
