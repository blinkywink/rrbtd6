//! Unity UI (uGUI) trees dumped from the game's prefabs by tools/assets/dump_ui_prefab.py,
//! laid out with RectTransform anchoring and the Horizontal/Vertical/Grid layout group rules,
//! and drawn with the game's sprites and TMP fonts.
//!
//! Coordinates: canvas units with y up (Unity), origin at the canvas bottom-left. The canvas
//! is the screen divided by the CanvasScaler factor.

use crate::ui::{color, draw_sliced, Fonts, Sprites, TextBox, TmpStyle};
use macroquad::miniquad::{BlendFactor, BlendState, BlendValue, Equation, PipelineParams};
use macroquad::prelude::*;
use serde_json::Value;

#[derive(Clone, Copy, Debug)]
pub struct Rt {
    pub amin: Vec2,
    pub amax: Vec2,
    pub pos: Vec2,
    pub size: Vec2,
    pub pivot: Vec2,
    pub scale: Vec2,
    /// Z rotation in degrees (counter-clockwise, Unity).
    pub rot: f32,
}

impl Default for Rt {
    fn default() -> Self {
        Self { amin: Vec2::ZERO, amax: Vec2::ONE, pos: Vec2::ZERO, size: Vec2::ZERO, pivot: Vec2::splat(0.5), scale: Vec2::ONE, rot: 0.0 }
    }
}

#[derive(Clone, Debug)]
pub struct Img {
    pub sprite: Option<String>,
    pub color: Color,
    pub tint: Color,
    pub kind: i64,
    pub preserve: bool,
    pub fill_center: bool,
    pub fill_method: i64,
    pub fill_amount: f32,
    pub ppum: f32,
    pub enabled: bool,
    /// Drawn with the GrayscaleUI material (TowerPurchaseButton2D.SetNotEnoughCash).
    pub gray: bool,
}

#[derive(Clone, Debug)]
pub struct Txt {
    cached_layout:std::cell::RefCell<Option<CachedTextLayout>>,
    pub text: String,
    pub font: String,
    pub size: f32,
    pub auto: bool,
    pub min: f32,
    pub max: f32,
    pub color: Color,
    pub tint: Color,
    pub h: i64,
    pub v: i64,
    /// left, top, right, bottom.
    pub margin: [f32; 4],
    pub style: TmpStyle,
    pub bold: bool,
    /// TMP FontStyles: 8 LowerCase, 16 UpperCase (applied when drawn).
    pub case: i64,
    pub spacing: f32,
    /// TMP word wrapping (m_TextWrappingMode / m_enableWordWrapping).
    pub wrap: bool,
    pub enabled: bool,
    /// m_enableVertexGradient: top-left, top-right, bottom-left, bottom-right.
    pub gradient: Option<[Color; 4]>,
}

#[derive(Clone,Debug,PartialEq)]
struct TextLayoutKey{source:String,font:String,dimensions:[u32;6],case:i64,wrap:bool,auto:bool,font_identity:(u64,[u32;4])}
#[derive(Clone,Debug)]
struct TextLayout{text:String,size:f32}
#[derive(Clone,Debug)]
struct CachedTextLayout{key:TextLayoutKey,layout:TextLayout}

impl Txt {
    fn layout<'a>(&'a self,fonts:&Fonts,bw:f32,bh:f32)->std::cell::Ref<'a,TextLayout>{
        // Reference path for whole-game A/B captures; resolved once, never read per frame.
        static DISABLED:std::sync::OnceLock<bool>=std::sync::OnceLock::new();
        if *DISABLED.get_or_init(||std::env::var_os("RRBTD6_NO_TEXT_CACHE").is_some()){
            *self.cached_layout.borrow_mut()=None;
        }
        self.layout_with(fonts.layout_identity(&self.font),bw,bh,
            |text,size|fonts.line_size(&self.font,text,size,self.spacing),
            |text,size|fonts.wrap(&self.font,text,size,self.spacing,bw))
    }
    fn layout_with<'a>(&'a self,font_identity:(u64,[u32;4]),bw:f32,bh:f32,measure:impl Fn(&str,f32)->Vec2,wrap:impl Fn(&str,f32)->String)->std::cell::Ref<'a,TextLayout>{
        let dimensions=[bw.to_bits(),bh.to_bits(),self.size.to_bits(),self.min.to_bits(),self.max.to_bits(),self.spacing.to_bits()];
        let hit=self.cached_layout.borrow().as_ref().is_some_and(|c|c.key.source==self.text&&c.key.font==self.font&&c.key.dimensions==dimensions&&c.key.case==self.case&&c.key.wrap==self.wrap&&c.key.auto==self.auto&&c.key.font_identity==font_identity);
        if !hit{
            let shown=self.shown();let mut size=self.size;let mut wrapped=None;
            if self.wrap{
                let lay=|sz|wrap(&shown,sz);
                if self.auto{
                    let fits=|sz|{let d=measure(&lay(sz),sz);d.x<=bw+0.5&&d.y<=bh+0.5};
                    let one_line=measure(&shown,self.max);
                    if one_line.x<=bw+0.5&&one_line.y<=bh+0.5{size=self.max;}else{
                        let (mut lo,mut hi)=(self.min,self.max);
                        while hi-lo>0.5{let mid=(lo+hi)/2.0;if fits(mid){lo=mid}else{hi=mid}}
                        size=lo;
                    }
                }
                wrapped=Some(lay(size));
            }else if self.auto{
                let one=measure(&shown,1.0);let fit=(bw/one.x.max(1e-3)).min(bh/one.y.max(1e-3));size=unity_clamp(fit,self.min,self.max);
            }
            *self.cached_layout.borrow_mut()=Some(CachedTextLayout{
                key:TextLayoutKey{source:self.text.clone(),font:self.font.clone(),dimensions,case:self.case,wrap:self.wrap,auto:self.auto,font_identity},
                layout:TextLayout{text:wrapped.unwrap_or_else(||shown.into_owned()),size},
            });
        }
        std::cell::Ref::map(self.cached_layout.borrow(),|c|&c.as_ref().unwrap().layout)
    }
    /// The text as displayed (TMP case styles applied).
    pub fn shown(&self) -> std::borrow::Cow<'_, str> {
        match self.case {
            16 => self.text.to_uppercase().into(),
            8 => self.text.to_lowercase().into(),
            _ => self.text.as_str().into(),
        }
    }
}

#[derive(Clone, Debug)]
pub enum Layout {
    Grid { pad: [f32; 4], cell: Vec2, spacing: Vec2, corner: i64, axis: i64, align: i64, constraint: i64, count: i64 },
    Line { vertical: bool, pad: [f32; 4], spacing: f32, align: i64, control: [bool; 2], expand: [bool; 2], reverse: bool },
}

#[derive(Clone, Debug)]
pub struct CommonPanel {
    pub heading: String,
    pub money: bool, pub buy_money: bool, pub knowledge: bool, pub buy_knowledge: bool,
    pub back: bool, pub change_hero: bool, pub friends: bool, pub extras: bool, pub perks: bool,
}

#[derive(Clone, Debug)]
pub struct Selectable {
    target:i64, transition:i64, interactable:bool, colors:[Color;4], duration:f32,
    current:Color, from:Color, desired:Color, began:Option<f64>, pressed:bool,
}
impl Selectable {
    fn sample(&mut self,hovered:bool,now:f64)->Color {
        let desired=self.colors[if !self.interactable{3}else if self.pressed{2}else if hovered{1}else{0}];
        if desired!=self.desired {self.from=self.current;self.desired=desired;self.began=Some(now);}
        let t=self.began.map(|began|if self.duration>0.0{((now-began) as f32/self.duration).clamp(0.0,1.0)}else{1.0}).unwrap_or(1.0);
        let from=self.from;let to=self.desired;
        self.current=Color::new(from.r+(to.r-from.r)*t,from.g+(to.g-from.g)*t,from.b+(to.b-from.b)*t,from.a+(to.a-from.a)*t);
        if t>=1.0{self.began=None;}
        self.current
    }
}
fn multiply_color(a:Color,b:Color)->Color {Color::new(a.r*b.r,a.g*b.g,a.b*b.b,a.a*b.a)}

#[cfg(test)]
mod mask_graphic_tests {
    use super::*;
    use serde_json::json;
    #[test]
    fn hidden_mask_graphic_keeps_clipping_and_child_graphics(){
        let node=Node::from_json(&json!({"name":"Viewport","active":true,"comps":[
            {"type":"Image","enabled":true,"color":[1,1,1,1]},
            {"type":"Mask","enabled":true,"m_ShowMaskGraphic":0}],
            "children":[{"name":"Item","active":true,"comps":[{"type":"Image","enabled":true,"color":[1,1,1,1]}]}]}));
        assert!(node.clip);assert!(!node.img.unwrap().enabled);
        assert!(node.children[0].img.as_ref().unwrap().enabled);
        let shown=Node::from_json(&json!({"name":"Mask","active":true,"comps":[
            {"type":"Image","enabled":true},{"type":"Mask","enabled":true,"m_ShowMaskGraphic":1}]}));
        assert!(shown.img.unwrap().enabled);
    }
}

#[derive(Clone, Debug)]
pub struct Node {
    pub name: String,
    source_id:Option<i64>,
    selectable:Option<Selectable>,
    pub common: Option<Box<CommonPanel>>,
    pub active: bool,
    pub rt: Rt,
    pub img: Option<Img>,
    pub txt: Option<Txt>,
    pub layout: Option<Layout>,
    pub ignore_layout: bool,
    /// LayoutElement min / preferred / flexible (negative = unset).
    pub le_min: Vec2,
    pub le_pref: Vec2,
    pub le_flex: Vec2,
    pub alpha: f32,
    /// Has a Button (pressed buttons are drawn slightly smaller).
    pub button: bool,
    pub native_button_animation:bool,
    /// RectMask2D: children are clipped to this rect.
    pub clip: bool,
    /// ContentSizeFitter per axis: 0 unconstrained, 1 min size, 2 preferred size.
    pub fit: [i64; 2],
    pub children: Vec<Node>,
    /// Screen rect from the last layout (y down, pixels), clipped by enclosing masks.
    pub screen: Rect,
    /// Full laid-out bounds before masks; use these for scroll extents.
    pub unclipped_screen: Rect,
}

fn f(v: &Value, d: f32) -> f32 {
    v.as_f64().map(|x| x as f32).unwrap_or(d)
}

fn v2(v: &Value) -> Vec2 {
    vec2(f(&v[0], 0.0), f(&v[1], 0.0))
}

fn b(v: &Value) -> bool {
    v.as_bool().unwrap_or_else(|| v.as_i64().unwrap_or(0) != 0)
}

fn pad(v: &Value) -> [f32; 4] {
    [f(&v["m_Left"], 0.0), f(&v["m_Right"], 0.0), f(&v["m_Top"], 0.0), f(&v["m_Bottom"], 0.0)]
}

impl Node {
    pub fn from_json(v: &Value) -> Node {
        let mut n = Node {
            name: v["name"].as_str().unwrap_or("").to_string(),
            source_id:v["id"].as_i64(),selectable:None,
            common: None,
            active: b(&v["active"]),
            rt: Rt::default(),
            img: None,
            txt: None,
            layout: None,
            ignore_layout: false,
            le_min: Vec2::splat(-1.0),
            le_pref: Vec2::splat(-1.0),
            le_flex: Vec2::splat(-1.0),
            alpha: 1.0,
            button: false,
            native_button_animation:false,
            clip: false,
            fit: [0, 0],
            children: Vec::new(),
            screen: Rect::default(),
            unclipped_screen: Rect::default(),
        };
        if let Some(r) = v.get("rect") {
            n.rt = Rt {
                amin: v2(&r["anchor_min"]),
                amax: v2(&r["anchor_max"]),
                pos: v2(&r["pos"]),
                size: v2(&r["size"]),
                pivot: v2(&r["pivot"]),
                scale: v2(&r["scale"]),
                rot: f(&r["rot_z"], 0.0),
            };
        }
        for c in v["comps"].as_array().into_iter().flatten() {
            let enabled = c["enabled"].as_bool().unwrap_or(true);
            match c["type"].as_str().unwrap_or("") {
                "CommonPanel" if enabled => {
                    n.common = Some(Box::new(CommonPanel {
                        heading: c["locHeading"].as_str().unwrap_or("").to_string(),
                        money: b(&c["showMonkeyMoney"]), buy_money: b(&c["showBuyMonkeyMoney"]),
                        knowledge: b(&c["showKnowledge"]), buy_knowledge: b(&c["showBuyKnowledge"]),
                        back: b(&c["showBackButton"]), change_hero: b(&c["showChangeHeroButton"]),
                        friends: b(&c["showFriendsButton"]), extras: b(&c["showExtraSettings"]), perks: b(&c["showPerkButton"]),
                    }));
                }
                "Image" => {
                    n.img = Some(Img {
                        tint:WHITE,
                        sprite: c["sprite"].as_str().filter(|s| !s.starts_with('?')).map(String::from),
                        color: color(&c["color"]).unwrap_or(WHITE),
                        kind: c["m_Type"].as_i64().unwrap_or(0),
                        preserve: b(&c["m_PreserveAspect"]),
                        fill_center: c.get("m_FillCenter").map(b).unwrap_or(true),
                        fill_method: c["m_FillMethod"].as_i64().unwrap_or(4),
                        fill_amount: f(&c["m_FillAmount"], 1.0),
                        ppum: f(&c["m_PixelsPerUnitMultiplier"], 1.0).max(0.01),
                        enabled,
                        gray: false,
                    })
                }
                "TextMeshProUGUI" => {
                    let ta = c["m_textAlignment"].as_i64().unwrap_or(514);
                    let m = &c["m_margin"];
                    n.txt = Some(Txt {
                        cached_layout:Default::default(),
                        tint:WHITE,
                        text: c["m_text"].as_str().unwrap_or("").to_string(),
                        font: c["font"].as_str().unwrap_or("Btd6FontTitleSDF").to_string(),
                        size: f(&c["m_fontSize"], 36.0),
                        auto: b(&c["m_enableAutoSizing"]),
                        min: f(&c["m_fontSizeMin"], 18.0),
                        max: f(&c["m_fontSizeMax"], 72.0),
                        color: color(&c["color"]).unwrap_or(WHITE),
                        gradient: c["gradient"].as_array().filter(|g| g.len() == 4)
                            .map(|g| [0, 1, 2, 3].map(|i| color(&g[i]).unwrap_or(WHITE))),
                        h: c["m_HorizontalAlignment"].as_i64().unwrap_or(ta & 0xff),
                        v: c["m_VerticalAlignment"].as_i64().unwrap_or(ta & 0xff00),
                        margin: [f(&m["x"], 0.0), f(&m["y"], 0.0), f(&m["z"], 0.0), f(&m["w"], 0.0)],
                        style: c.get("mat").filter(|m| m.is_object()).map(TmpStyle::from_json).unwrap_or_default(),
                        bold: c["m_fontStyle"].as_i64().unwrap_or(0) & 1 != 0,
                        case: c["m_fontStyle"].as_i64().unwrap_or(0) & 24,
                        spacing: f(&c["m_characterSpacing"], 0.0),
                        wrap: c["m_TextWrappingMode"].as_i64().map(|m| m != 0).unwrap_or_else(|| b(&c["m_enableWordWrapping"])),
                        enabled,
                    })
                }
                t @ ("HorizontalLayoutGroup" | "VerticalLayoutGroup") if enabled => {
                    n.layout = Some(Layout::Line {
                        vertical: t.starts_with('V'),
                        pad: pad(&c["m_Padding"]),
                        spacing: f(&c["m_Spacing"], 0.0),
                        align: c["m_ChildAlignment"].as_i64().unwrap_or(0),
                        control: [b(&c["m_ChildControlWidth"]), b(&c["m_ChildControlHeight"])],
                        expand: [b(&c["m_ChildForceExpandWidth"]), b(&c["m_ChildForceExpandHeight"])],
                        reverse: b(&c["m_ReverseArrangement"]),
                    })
                }
                "GridLayoutGroup" if enabled => {
                    n.layout = Some(Layout::Grid {
                        pad: pad(&c["m_Padding"]),
                        cell: vec2(f(&c["m_CellSize"]["x"], 100.0), f(&c["m_CellSize"]["y"], 100.0)),
                        spacing: vec2(f(&c["m_Spacing"]["x"], 0.0), f(&c["m_Spacing"]["y"], 0.0)),
                        corner: c["m_StartCorner"].as_i64().unwrap_or(0),
                        axis: c["m_StartAxis"].as_i64().unwrap_or(0),
                        align: c["m_ChildAlignment"].as_i64().unwrap_or(0),
                        constraint: c["m_Constraint"].as_i64().unwrap_or(0),
                        count: c["m_ConstraintCount"].as_i64().unwrap_or(2),
                    })
                }
                "LayoutElement" if enabled => {
                    n.ignore_layout = b(&c["m_IgnoreLayout"]);
                    n.le_min = vec2(f(&c["m_MinWidth"], -1.0), f(&c["m_MinHeight"], -1.0));
                    n.le_pref = vec2(f(&c["m_PreferredWidth"], -1.0), f(&c["m_PreferredHeight"], -1.0));
                    n.le_flex = vec2(f(&c["m_FlexibleWidth"], -1.0), f(&c["m_FlexibleHeight"], -1.0));
                }
                "CanvasGroup" => n.alpha = f(&c["m_Alpha"], 1.0),
                "Button" if enabled => n.button = true,
                // Mask (image stencil) clipped to its rect: STAND-IN, rounded corners are lost.
                "RectMask2D" | "Mask" if enabled => n.clip = true,
                "ContentSizeFitter" if enabled => {
                    n.fit = [c["m_HorizontalFit"].as_i64().unwrap_or(0), c["m_VerticalFit"].as_i64().unwrap_or(0)]
                }
                _ => {}
            }
            if let (Some(target),Some(colors))=(c["target_go"].as_i64(),c["m_Colors"].as_object()) {
                let multiplier=colors.get("m_ColorMultiplier").map(|v|f(v,1.0)).unwrap_or(1.0);
                let read=|key:&str|colors.get(key).map(|v|Color::new(f(&v["r"],1.0)*multiplier,f(&v["g"],1.0)*multiplier,
                    f(&v["b"],1.0)*multiplier,f(&v["a"],1.0)*multiplier)).unwrap_or(WHITE);
                let normal=read("m_NormalColor");
                n.selectable=Some(Selectable {target,transition:c["m_Transition"].as_i64().unwrap_or(1),
                    interactable:c.get("m_Interactable").map(b).unwrap_or(true),
                    colors:[normal,read("m_HighlightedColor"),read("m_PressedColor"),read("m_DisabledColor")],
                    duration:colors.get("m_FadeDuration").map(|v|f(v,0.1)).unwrap_or(0.1),
                    current:normal,from:normal,desired:normal,began:None,pressed:false});
            }
        }
        // Unity Mask clips with its graphic even when that graphic is not rendered.
        // Drawing hidden mask images produces white rectangles in native dropdown viewports.
        if v["comps"].as_array().into_iter().flatten().any(|c|
            c["type"]=="Mask" && c["enabled"]!=false && c["enabled"]!=0 &&
            (c["m_ShowMaskGraphic"]==false || c["m_ShowMaskGraphic"]==0)) {
            if let Some(img)=&mut n.img {img.enabled=false;}
        }
        // Jewel.UpdateActive: the gamepad button prompt's image is enabled only in gamepad input mode.
        if v["comps"].as_array().into_iter().flatten().any(|c|c["type"]=="Jewel") {
            if let Some(img)=&mut n.img {img.enabled=false;}
        }
        n.children = v["children"].as_array().into_iter().flatten().map(Node::from_json).collect();
        n
    }

    pub fn common_panel(&self) -> Option<&CommonPanel> {
        if !self.active { return None; }
        self.common.as_deref().or_else(|| self.children.iter().find_map(Node::common_panel))
    }

    pub fn load(path: &std::path::Path) -> Option<Node> {
        let s = std::fs::read_to_string(path).ok()?;
        Some(Node::from_json(&serde_json::from_str(&s).ok()?))
    }

    fn tint_target(&mut self,id:i64,tint:Color)->bool {
        if self.source_id==Some(id) {
            if let Some(image)=self.img.as_mut(){image.tint=tint;}
            if let Some(text)=self.txt.as_mut(){text.tint=tint;}
            return true;
        }
        self.children.iter_mut().any(|n|n.tint_target(id,tint))
    }

    pub fn animated_button_state(&self,inside:bool,down:bool)->Option<&'static str> {
        if self.selectable.as_ref().is_some_and(|s|s.transition!=3){return None;}
        Some(if self.selectable.as_ref().is_some_and(|s|!s.interactable){"Disabled"}
            else if down{"Pressed"}else if inside{"Highlighted"}else{"Normal"})
    }

    /// Descendant by '/'-separated child names (first match at each level).
    pub fn find(&self, path: &str) -> Option<&Node> {
        let mut n = self;
        for part in path.split('/').filter(|p| !p.is_empty()) {
            n = n.children.iter().find(|c| c.name == part)?;
        }
        Some(n)
    }

    pub fn find_mut(&mut self, path: &str) -> Option<&mut Node> {
        let mut n = self;
        for part in path.split('/').filter(|p| !p.is_empty()) {
            n = n.children.iter_mut().find(|c| c.name == part)?;
        }
        Some(n)
    }

    /// Deactivates every descendant with one of these names.
    pub fn hide_named(&mut self, names: &[&str]) {
        for c in &mut self.children {
            if names.contains(&c.name.as_str()) {
                c.active = false;
            }
            c.hide_named(names);
        }
    }

    pub fn set_active(&mut self, path: &str, on: bool) {
        if let Some(n) = self.find_mut(path) {
            n.active = on;
        }
    }

    pub fn set_text(&mut self, path: &str, s: &str) {
        if let Some(t) = self.find_mut(path).and_then(|n| n.txt.as_mut()) {
            if t.text != s { t.text.clear(); t.text.push_str(s); }
        }
    }

    pub fn set_sprite(&mut self, path: &str, s: Option<&str>) {
        if let Some(i) = self.find_mut(path).and_then(|n| n.img.as_mut()) {
            if i.sprite.as_deref() != s { i.sprite = s.map(String::from); }
        }
    }

    pub fn set_img_gray(&mut self, path: &str, gray: bool) {
        if let Some(i) = self.find_mut(path).and_then(|n| n.img.as_mut()) {
            i.gray = gray;
        }
    }

    pub fn set_img_color(&mut self, path: &str, c: Color) {
        if let Some(i) = self.find_mut(path).and_then(|n| n.img.as_mut()) {
            i.color = c;
        }
    }

    pub fn set_text_color(&mut self, path: &str, c: Color) {
        if let Some(t) = self.find_mut(path).and_then(|n| n.txt.as_mut()) {
            t.color = c;
        }
    }

    /// Screen rect of a descendant from the last layout, if it was active.
    pub fn rect(&self, path: &str) -> Option<Rect> {
        let mut n = self;
        if !n.active { return None; }
        for part in path.split('/').filter(|p| !p.is_empty()) {
            n = n.children.iter().find(|c| c.name == part)?;
            if !n.active { return None; }
        }
        Some(n.screen)
    }

    pub fn clicked(&self, path: &str) -> bool {
        let (mx, my) = mouse_position();
        !INPUT_BLOCKED.load(std::sync::atomic::Ordering::Relaxed)
            && is_mouse_button_pressed(MouseButton::Left)
            && self.rect(path).is_some_and(|r| r.contains(vec2(mx, my)))
    }
}

/// While set, `Node::clicked` reports nothing (a modal popup owns the input).
static INPUT_BLOCKED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

pub fn block_input(on: bool) {
    INPUT_BLOCKED.store(on, std::sync::atomic::Ordering::Relaxed);
}

/// Local-space rect (y up): min corner + size.
#[derive(Clone, Copy, Debug)]
pub struct R {
    pub min: Vec2,
    pub size: Vec2,
}

/// Local -> canvas transform: canvas = o + k * local.
#[derive(Clone, Copy)]
struct T {
    o: Vec2,
    k: Vec2,
}

pub struct Ctx<'a> {
    pub sprites: &'a mut Sprites,
    pub fonts: &'a Fonts,
    /// Screen px per canvas unit, and canvas height (for the y flip).
    pub scale: f32,
    pub canvas_h: f32,
    /// Current RectMask2D clip (screen px), if any.
    pub clip: Option<Rect>,
}

fn intersect(a: Rect, b: Rect) -> Rect {
    let (x0, y0) = (a.x.max(b.x), a.y.max(b.y));
    let (x1, y1) = (a.right().min(b.right()), a.bottom().min(b.bottom()));
    Rect::new(x0, y0, (x1 - x0).max(0.0), (y1 - y0).max(0.0))
}

fn set_scissor(r: Option<Rect>) {
    let gl = unsafe { get_internal_gl() };
    let d = screen_dpi_scale();
    gl.quad_gl.scissor(r.map(|r| ((r.x * d) as i32, (r.y * d) as i32, (r.w * d).ceil() as i32, (r.h * d).ceil() as i32)));
}

impl Ctx<'_> {
    fn to_screen(&self, t: T, r: R) -> Rect {
        let a = t.o + t.k * r.min;
        let b = t.o + t.k * (r.min + r.size);
        let (x0, x1) = (a.x.min(b.x), a.x.max(b.x));
        let (y0, y1) = (a.y.min(b.y), a.y.max(b.y));
        Rect::new(x0 * self.scale, (self.canvas_h - y1) * self.scale, (x1 - x0) * self.scale, (y1 - y0) * self.scale)
    }

    fn point_to_screen(&self, p: Vec2) -> Vec2 {
        vec2(p.x, self.canvas_h - p.y) * self.scale
    }

    fn sprite_size(&mut self, name: &str, ppum: f32) -> Option<Vec2> {
        let s = self.sprites.get(name)?;
        // Canvas referencePixelsPerUnit is 10.8 (InGame CanvasScaler), same as the sprites'.
        let k = 10.8 / (s.ppu * ppum);
        Some(vec2(s.tex.width(), s.tex.height()) * k)
    }

    /// LayoutUtility min / preferred / flexible sizes along `axis`.
    fn sizes(&mut self, n: &Node, axis: usize) -> (f32, f32, f32) {
        let (mut min, mut pref, mut flex) = (0.0f32, 0.0f32, 0.0f32);
        if let Some(l) = &n.layout {
            let (m, p) = self.group_sizes(n, l, axis);
            min = m;
            pref = p;
        } else {
            if let Some(t) = n.txt.as_ref().filter(|t| t.enabled) {
                let size = if t.auto { t.max } else { t.size };
                let s = self.fonts.line_size(&t.font, &t.shown(), size, t.spacing);
                pref = pref.max(s[axis] + if axis == 0 { t.margin[0] + t.margin[2] } else { t.margin[1] + t.margin[3] });
            }
            if let Some(i) = n.img.as_ref().filter(|i| i.enabled) {
                if let Some(name) = &i.sprite {
                    let sz = self.sprite_size(name, i.ppum).unwrap_or(Vec2::ZERO);
                    let p = if i.kind == 1 || i.kind == 2 {
                        let s = self.sprites.get(name).map(|s| s.border).unwrap_or([0.0; 4]);
                        let k = sz[axis] / self.sprites.get(name).map(|s| if axis == 0 { s.tex.width() } else { s.tex.height() }).unwrap_or(1.0).max(1.0);
                        (if axis == 0 { s[0] + s[2] } else { s[1] + s[3] }) * k
                    } else {
                        sz[axis]
                    };
                    pref = pref.max(p);
                }
            }
        }
        if n.le_min[axis] >= 0.0 {
            min = n.le_min[axis];
        }
        if n.le_pref[axis] >= 0.0 {
            pref = n.le_pref[axis];
        }
        if n.le_flex[axis] >= 0.0 {
            flex = n.le_flex[axis];
        }
        (min, pref.max(min), flex)
    }

    fn layout_children<'n>(&self, n: &'n Node) -> Vec<usize> {
        n.children.iter().enumerate().filter(|(_, c)| c.active && !c.ignore_layout).map(|(i, _)| i).collect()
    }

    fn group_sizes(&mut self, n: &Node, l: &Layout, axis: usize) -> (f32, f32) {
        let kids = self.layout_children(n);
        match l {
            Layout::Line { vertical, pad, spacing, control, expand, .. } => {
                let along = (axis == 1) == *vertical;
                let padding = if axis == 0 { pad[0] + pad[1] } else { pad[2] + pad[3] };
                let (mut tmin, mut tpref) = (0.0f32, 0.0f32);
                for &i in &kids {
                    let (mi, pr, _) = self.child_sizes(&n.children[i], axis, control[axis], expand[axis]);
                    if along {
                        tmin += mi + spacing;
                        tpref += pr + spacing;
                    } else {
                        tmin = tmin.max(mi);
                        tpref = tpref.max(pr);
                    }
                }
                if along && !kids.is_empty() {
                    tmin -= spacing;
                    tpref -= spacing;
                }
                (tmin + padding, tpref + padding)
            }
            Layout::Grid { pad, cell, spacing, constraint, count, .. } => {
                if axis == 1 && *constraint == 0 {
                    let height = flexible_grid_height(n.rt.size.x, *pad, *cell, *spacing, kids.len());
                    return (height,height);
                }
                let nk = kids.len().max(1) as f32;
                let cols = match constraint {
                    1 => *count as f32,
                    2 => (nk / *count as f32).ceil(),
                    _ => nk.sqrt().ceil(),
                };
                let lines = if axis == 0 { cols } else { (nk / cols.max(1.0)).ceil() };
                let p = if axis == 0 { pad[0] + pad[1] } else { pad[2] + pad[3] };
                let v = p + (cell[axis] + spacing[axis]) * lines - spacing[axis];
                (v, v)
            }
        }
    }

    fn child_sizes(&mut self, c: &Node, axis: usize, control: bool, expand: bool) -> (f32, f32, f32) {
        let (mut mi, mut pr, mut fl) = if control { self.sizes(c, axis) } else { (c.rt.size[axis], c.rt.size[axis], 0.0) };
        if !control {
            mi = c.rt.size[axis];
            pr = mi;
        }
        if expand {
            fl = fl.max(1.0);
        }
        (mi, pr, fl)
    }

    /// Positions of layout-driven children: (child index, local rect).
    fn run_group(&mut self, n: &Node, r: R) -> Vec<(usize, R)> {
        let Some(l) = n.layout.clone() else { return Vec::new() };
        let kids = self.layout_children(n);
        let mut out: Vec<(usize, R)> = kids.iter().map(|&i| (i, R { min: Vec2::ZERO, size: n.children[i].rt.size })).collect();
        match l {
            Layout::Grid { pad, cell, spacing, corner, axis, align, constraint, count } => {
                let w = r.size.x - pad[0] - pad[1];
                let h = r.size.y - pad[2] - pad[3];
                let nk = kids.len().max(1);
                let (cols, rows) = match constraint {
                    1 => (count.max(1) as usize, nk.div_ceil(count.max(1) as usize)),
                    2 => (nk.div_ceil(count.max(1) as usize), count.max(1) as usize),
                    _ => {
                        let c = (((w + spacing.x + 0.001) / (cell.x + spacing.x)).floor() as usize).max(1);
                        let rws = (((h + spacing.y + 0.001) / (cell.y + spacing.y)).floor() as usize).max(1);
                        if axis == 0 { (c, nk.div_ceil(c)) } else { (nk.div_ceil(rws), rws) }
                    }
                };
                let (ax, ay) = (cols.min(nk), rows.min(nk));
                let req = vec2(ax as f32 * cell.x + (ax as f32 - 1.0) * spacing.x, ay as f32 * cell.y + (ay as f32 - 1.0) * spacing.y);
                let start = vec2(
                    pad[0] + (w - req.x) * (align % 3) as f32 * 0.5,
                    pad[2] + (h - req.y) * (align / 3) as f32 * 0.5,
                );
                for (k, o) in out.iter_mut().enumerate() {
                    let (mut cx, mut cy) = if axis == 0 { (k % cols, k / cols) } else { (k / rows, k % rows) };
                    if corner % 2 == 1 {
                        cx = ax - 1 - cx;
                    }
                    if corner / 2 == 1 {
                        cy = ay - 1 - cy;
                    }
                    let x = start.x + cx as f32 * (cell.x + spacing.x);
                    let y_top = start.y + cy as f32 * (cell.y + spacing.y);
                    o.1 = R { min: vec2(r.min.x + x, r.min.y + r.size.y - y_top - cell.y), size: cell };
                }
            }
            Layout::Line { vertical, pad, spacing, align, control, expand, reverse } => {
                if reverse {
                    out.reverse();
                }
                for axis in 0..2 {
                    let along = (axis == 1) == vertical;
                    let size = r.size[axis];
                    let padding = if axis == 0 { pad[0] + pad[1] } else { pad[2] + pad[3] };
                    let lead = if axis == 0 { pad[0] } else { pad[2] };
                    let align_k = if axis == 0 { (align % 3) as f32 * 0.5 } else { (align / 3) as f32 * 0.5 };
                    let sizes: Vec<(f32, f32, f32)> = out.iter().map(|&(i, _)| self.child_sizes(&n.children[i], axis, control[axis], expand[axis])).collect();
                    let place = |o: &mut (usize, R), pos: f32, len: f32| {
                        if axis == 0 {
                            o.1.min.x = r.min.x + pos;
                            o.1.size.x = len;
                        } else {
                            o.1.size.y = len;
                            o.1.min.y = r.min.y + r.size.y - pos - len;
                        }
                    };
                    if !along {
                        let inner = size - padding;
                        for (o, &(mi, pr, fl)) in out.iter_mut().zip(&sizes) {
                            let req = unity_clamp(inner, mi, if fl > 0.0 { size } else { pr.max(mi) });
                            let start = lead + (size - (req + padding)) * align_k;
                            if control[axis] {
                                place(o, start, req);
                            } else {
                                let own = n.children[o.0].rt.size[axis];
                                place(o, start + (req - own) * align_k, own);
                            }
                        }
                    } else {
                        let n_sp = (out.len().max(1) - 1) as f32 * spacing;
                        let tmin: f32 = sizes.iter().map(|s| s.0).sum::<f32>() + n_sp + padding;
                        let tpref: f32 = sizes.iter().map(|s| s.1).sum::<f32>() + n_sp + padding;
                        let tflex: f32 = sizes.iter().map(|s| s.2).sum();
                        let mut pos = lead;
                        let mut flex_mul = 0.0;
                        let surplus = size - tpref;
                        if surplus > 0.0 {
                            if tflex == 0.0 {
                                pos = lead + surplus * align_k;
                            } else {
                                flex_mul = surplus / tflex;
                            }
                        }
                        let lerp = if tmin != tpref { ((size - tmin) / (tpref - tmin)).clamp(0.0, 1.0) } else { 0.0 };
                        for (o, &(mi, pr, fl)) in out.iter_mut().zip(&sizes) {
                            let len = mi + (pr - mi) * lerp + fl * flex_mul;
                            if control[axis] {
                                place(o, pos, len);
                            } else {
                                let own = n.children[o.0].rt.size[axis];
                                place(o, pos + (len - own) * align_k, own);
                            }
                            pos += len + spacing;
                        }
                    }
                }
            }
        }
        out
    }

    fn rt_rect(rt: &Rt, parent: R) -> R {
        let amin = parent.min + parent.size * rt.amin;
        let amax = parent.min + parent.size * rt.amax;
        let size = amax - amin + rt.size;
        let piv = amin + (amax - amin) * rt.pivot + rt.pos;
        R { min: piv - size * rt.pivot, size }
    }
}

/// Unity GridLayoutGroup.CalculateLayoutInputVertical (flexible constraint): width
/// determines the column count, even when filling vertically. Verified in native code.
fn flexible_grid_height(width: f32, pad: [f32;4], cell: Vec2, spacing: Vec2, children: usize) -> f32 {
    let columns = ((width-pad[0]-pad[1]+spacing.x+0.001)/(cell.x+spacing.x)).floor().max(1.0);
    let rows = (children as f32/columns).ceil();
    pad[2]+pad[3]+rows*(cell.y+spacing.y)-spacing.y
}

#[cfg(test)]
mod grid_regressions {
    use super::*;
    #[test]
    fn hero_skins_remain_one_column_at_original_width() {
        assert_eq!(flexible_grid_height(320.0,[0.0,0.0,30.0,80.0],vec2(320.0,320.0),vec2(64.0,20.0),3),1110.0);
    }
    #[test]
    fn flexible_grid_reflows_when_width_fits_two_columns() {
        assert_eq!(flexible_grid_height(704.0,[0.0,0.0,30.0,80.0],vec2(320.0,320.0),vec2(64.0,20.0),3),770.0);
    }
}

/// Lays out and draws `n` (and its active subtree) inside `parent` (canvas units, y up).
pub fn draw_tree(n: &mut Node, parent: R, ctx: &mut Ctx) {
    let r = Ctx::rt_rect(&n.rt, parent);
    draw_node(n, r, T { o: Vec2::ZERO, k: Vec2::ONE }, 1.0, ctx);
}

fn draw_node(n: &mut Node, r: R, pt: T, alpha: f32, ctx: &mut Ctx) {
    if !n.active {
        return;
    }
    let pivot = r.min + r.size * n.rt.pivot;
    let mut scale = n.rt.scale;
    if n.button && !n.native_button_animation && n.selectable.as_ref().is_none_or(|s|s.transition!=1) && is_mouse_button_down(MouseButton::Left) && n.screen.contains(mouse_position().into()) {
        scale *= 0.95;
    }
    // Half turns are exact as a flip of both axes (children included); other angles only
    // rotate this node's own graphic and hit box.
    let rot = n.rt.rot.rem_euclid(360.0);
    let half_turn = (rot - 180.0).abs() < 0.5;
    if half_turn {
        scale = -scale;
    }
    let t = T { o: pt.o + pt.k * pivot * (Vec2::ONE - scale), k: pt.k * scale };
    let screen = ctx.to_screen(t, r);
    let spin = (!half_turn && rot > 0.5 && rot < 359.5).then(|| {
        let mirrored = (t.k.x < 0.0) != (t.k.y < 0.0);
        let a = if mirrored { rot } else { -rot }.to_radians();
        (a, ctx.point_to_screen(pt.o + pt.k * pivot))
    });
    let bounds = spin.map_or(screen, |(a, p)| rotated_bounds(screen, a, p));
    n.unclipped_screen = bounds;
    n.screen = ctx.clip.map_or(bounds, |c| intersect(bounds, c));
    if let Some(selectable)=n.selectable.as_mut().filter(|s|s.transition==1) {
        let inside=n.screen.contains(mouse_position().into());
        if is_mouse_button_pressed(MouseButton::Left){selectable.pressed=inside;}
        if !is_mouse_button_down(MouseButton::Left){selectable.pressed=false;}
        let tint=selectable.sample(inside,get_time());let target=selectable.target;
        n.tint_target(target,tint);
    }
    let alpha = alpha * n.alpha;
    // Preserve layout and hit bounds outside masks, but do not decode textures or shape
    // text for off-screen catalog cards (hundreds of Trophy Store / Achievement entries).
    let graphic_visible = n.screen.w > 0.0 && n.screen.h > 0.0 && alpha > 0.001;
    if let Some(img) = n.img.as_ref().filter(|i| i.enabled && graphic_visible && i.color.a*i.tint.a>0.0) {
        draw_image(img, screen, alpha, [t.k.x < 0.0, t.k.y < 0.0], spin, ctx);
    }
    if let Some(txt) = n.txt.as_ref().filter(|t| t.enabled && !t.text.is_empty() && graphic_visible && t.color.a*t.tint.a>0.0) {
        let k = screen.h / r.size.y.max(1e-3);
        let box_ = Rect::new(
            screen.x + txt.margin[0] * k,
            screen.y + txt.margin[1] * k,
            screen.w - (txt.margin[0] + txt.margin[2]) * k,
            screen.h - (txt.margin[1] + txt.margin[3]) * k,
        );
        let (bw, bh) = (box_.w / k, box_.h / k);
        let layout=txt.layout(ctx.fonts,bw,bh);
        let mut c = multiply_color(txt.color,txt.tint);
        c.a *= alpha;
        ctx.fonts.draw(&txt.font, &TextBox {
            text:&layout.text,
            rect: box_,
            size: layout.size * k,
            color: c,
            h_align: txt.h,
            v_align: txt.v,
            style: &txt.style,
            bold: txt.bold,
            spacing: txt.spacing,
            gradient: txt.gradient,
        });
    }
    for c in n.children.iter_mut().filter(|c| c.active) {
        for axis in 0..2 {
            if c.fit[axis] != 0 {
                let (min, pref, _) = if let (1, Some(Layout::Grid {pad,cell,spacing,constraint:0,..})) = (axis,&c.layout) {
                    let width = Ctx::rt_rect(&c.rt,r).size.x;
                    let height = flexible_grid_height(width,*pad,*cell,*spacing,ctx.layout_children(c).len());
                    (height,height,0.0)
                } else { ctx.sizes(c, axis) };
                c.rt.size[axis] = if c.fit[axis] == 1 { min } else { pref };
            }
        }
    }
    let driven = ctx.run_group(n, r);
    let outer_clip = ctx.clip;
    if n.clip {
        ctx.clip = Some(outer_clip.map_or(screen, |c| intersect(screen, c)));
        set_scissor(ctx.clip);
    }
    for i in 0..n.children.len() {
        let cr = match driven.iter().find(|(j, _)| *j == i) {
            Some((_, cr)) => *cr,
            None => Ctx::rt_rect(&n.children[i].rt, r),
        };
        draw_node(&mut n.children[i], cr, t, alpha, ctx);
    }
    if n.clip {
        ctx.clip = outer_clip;
        set_scissor(outer_clip);
    }
}

/// Screen bounding box of `r` rotated by `a` radians (clockwise on screen) about `p`.
fn rotated_bounds(r: Rect, a: f32, p: Vec2) -> Rect {
    let (s, c) = a.sin_cos();
    let corners = [r.point(), vec2(r.right(), r.y), vec2(r.x, r.bottom()), vec2(r.right(), r.bottom())]
        .map(|q| p + vec2((q.x - p.x) * c - (q.y - p.y) * s, (q.x - p.x) * s + (q.y - p.y) * c));
    let lo = corners.iter().fold(Vec2::splat(f32::MAX), |m, q| m.min(*q));
    let hi = corners.iter().fold(Vec2::splat(f32::MIN), |m, q| m.max(*q));
    Rect::new(lo.x, lo.y, hi.x - lo.x, hi.y - lo.y)
}

const GRAY_VS: &str = r#"#version 100
attribute vec3 position;
attribute vec2 texcoord;
attribute vec4 color0;
varying lowp vec2 uv;
varying lowp vec4 color;
uniform mat4 Model;
uniform mat4 Projection;
void main() {
    gl_Position = Projection * Model * vec4(position, 1);
    color = color0 / 255.0;
    uv = texcoord;
}"#;

/// GrayscaleUI: _GrayscaleAmount 0.8 toward luminance, then x _Tint (0.643, 0.725, 0.824).
const GRAY_FS: &str = r#"#version 100
varying lowp vec4 color;
varying lowp vec2 uv;
uniform sampler2D Texture;
void main() {
    lowp vec4 c = texture2D(Texture, uv) * color;
    lowp float g = dot(c.rgb, vec3(0.299, 0.587, 0.114));
    gl_FragColor = vec4(mix(c.rgb, vec3(g), 0.8) * vec3(0.6431, 0.7255, 0.8235), c.a);
}"#;

thread_local! {
    static GRAY: std::cell::OnceCell<Option<Material>> = const { std::cell::OnceCell::new() };
}

/// Runs `f` with the GrayscaleUI material bound (if it compiled).
fn with_gray(f: impl FnOnce()) {
    GRAY.with(|g| {
        let m = g.get_or_init(|| load_material(ShaderSource::Glsl { vertex: GRAY_VS, fragment: GRAY_FS }, MaterialParams {
            pipeline_params: PipelineParams {
                color_blend: Some(BlendState::new(Equation::Add, BlendFactor::Value(BlendValue::SourceAlpha), BlendFactor::OneMinusValue(BlendValue::SourceAlpha))),
                ..Default::default()
            },
            ..Default::default()
        }).ok());
        if let Some(m) = m {
            gl_use_material(m);
            f();
            gl_use_default_material();
        } else {
            f();
        }
    });
}

/// Draws an Image into screen rect `r`, mirrored per axis and optionally rotated
/// (screen radians, about a screen point).
fn draw_image(img: &Img, r: Rect, alpha: f32, flip: [bool; 2], spin: Option<(f32, Vec2)>, ctx: &mut Ctx) {
    if img.gray && img.sprite.is_some() {
        let plain = Img { gray: false, ..img.clone() };
        with_gray(|| draw_image_inner(&plain, r, alpha, flip, spin, ctx));
    } else {
        draw_image_inner(img, r, alpha, flip, spin, ctx);
    }
}

fn draw_image_inner(img: &Img, r: Rect, alpha: f32, flip: [bool; 2], spin: Option<(f32, Vec2)>, ctx: &mut Ctx) {
    let mut tint = multiply_color(img.color,img.tint);
    tint.a *= alpha;
    // An Image without a sprite draws its colour as a solid rect (Unity's default UI sprite).
    let Some(name) = &img.sprite else {
        if tint.a > 0.0 {
            match spin {
                Some((a, p)) if r.w > 0.0 && r.h > 0.0 => draw_rectangle_ex(p.x, p.y, r.w, r.h, DrawRectangleParams {
                    offset: vec2((p.x - r.x) / r.w, (p.y - r.y) / r.h),
                    rotation: a,
                    color: tint,
                }),
                _ => draw_rectangle(r.x, r.y, r.w, r.h, tint),
            }
        }
        return;
    };
    let scale = ctx.scale;
    let Some(s) = ctx.sprites.get(name) else { return };
    let (tw, th) = (s.tex.width(), s.tex.height());
    match img.kind {
        1 | 2 => {
            // Border in canvas units = pixels * referencePPU / (spritePPU * multiplier).
            let k = 10.8 / (s.ppu * img.ppum) * scale;
            let [l, b, rr, t] = s.border;
            draw_sliced(&s.tex, r, s.border, [l * k, b * k, rr * k, t * k], img.fill_center, tint, flip);
        }
        3 => {
            // Filled: horizontal (0) / vertical (1) from the left / bottom; radial drawn whole.
            let a = img.fill_amount.clamp(0.0, 1.0);
            if a <= 0.0 {
                return;
            }
            let (dst, src) = match img.fill_method {
                0 => (Rect::new(r.x, r.y, r.w * a, r.h), Rect::new(0.0, 0.0, tw * a, th)),
                1 => (Rect::new(r.x, r.y + r.h * (1.0 - a), r.w, r.h * a), Rect::new(0.0, th * (1.0 - a), tw, th * a)),
                _ => (r, Rect::new(0.0, 0.0, tw, th)),
            };
            draw_texture_ex(&s.tex, dst.x, dst.y, tint, DrawTextureParams { dest_size: Some(dst.size()), source: Some(src), ..Default::default() });
        }
        _ => {
            let mut d = r;
            if img.preserve {
                let k = (r.w / tw).min(r.h / th);
                d = Rect::new(r.x + (r.w - tw * k) / 2.0, r.y + (r.h - th * k) / 2.0, tw * k, th * k);
            }
            draw_texture_ex(&s.tex, d.x, d.y, tint, DrawTextureParams {
                dest_size: Some(d.size()),
                flip_x: flip[0],
                flip_y: flip[1],
                rotation: spin.map_or(0.0, |s| s.0),
                pivot: spin.map(|s| s.1),
                ..Default::default()
            });
        }
    }
}

/// Unity's Mathf.Clamp: below `min` gives `min`, else above `max` gives `max` (no panic when
/// min > max, which negative layout padding produces: CommonPopup).
fn unity_clamp(v: f32, min: f32, max: f32) -> f32 {
    if v < min {
        min
    } else if v > max {
        max
    } else {
        v
    }
}

/// Opt-in real-font GPU equivalence check and layout-only CPU benchmark.
pub async fn verify_text_layout_cache(root:&std::path::Path){
    let fonts=Fonts::load(root,&[("Btd6FontTitleSDF","fonts/title"),("Btd6FontBodySDF","fonts/body")]).await;
    assert_eq!(fonts.fonts.len(),2,"text cache audit requires both real fonts");
    let mut t=Node::from_json(&serde_json::json!({"name":"Text","active":true,"comps":[{"type":"TextMeshProUGUI","enabled":true,"m_text":"Some words for wrapping and automatic sizing","m_enableAutoSizing":true,"m_enableWordWrapping":true,"m_fontSize":48,"m_fontSizeMin":8,"m_fontSizeMax":72}]})).txt.unwrap();
    t.font="Btd6FontTitleSDF".into();
    let rt=render_target(256,256);let mut camera=Camera2D::from_display_rect(Rect::new(0.0,0.0,256.0,256.0));camera.render_target=Some(rt.clone());
    for case in 0..12{
        let (w,h)=if case==5{(175.0,130.0)}else{(230.0,220.0)};
        match case{1=>t.text="Cash: $123,456".into(),2=>t.font="Btd6FontBodySDF".into(),3=>t.case=16,4=>t.spacing=2.0,6=>t.wrap=false,7=>t.auto=false,8=>t.size=31.0,9=>{t.color=YELLOW;t.bold=true;t.h=2;},10=>t.font="Missing fallback".into(),11=>{t.wrap=true;t.auto=true;t.min=12.0;t.max=44.0;},_=>{}}
        let draw=||{set_camera(&camera);clear_background(BLACK);let l=t.layout(&fonts,w,h);fonts.draw(&t.font,&TextBox{gradient:None,text:&l.text,rect:Rect::new(8.0,8.0,w,h),size:l.size,color:t.color,h_align:t.h,v_align:t.v,style:&t.style,bold:t.bold,spacing:t.spacing});set_default_camera();rt.texture.get_texture_data().bytes};
        let cached=draw();*t.cached_layout.borrow_mut()=None;let fresh=draw();assert_eq!(cached,fresh,"text cache GPU mismatch case {case}");
        assert!(fresh.chunks_exact(4).any(|p|p[0]!=0||p[1]!=0||p[2]!=0),"blank text audit case {case}");
    }
    t.text="Some words for wrapping and automatic sizing across several lines".into();t.font="Btd6FontTitleSDF".into();
    let mut samples=[Vec::new(),Vec::new()];
    for pass in 0..16{for offset in 0..2{let mode=(pass+offset)%2;let start=std::time::Instant::now();for _ in 0..1000{if mode==0{*t.cached_layout.borrow_mut()=None;}std::hint::black_box(t.layout(&fonts,180.0,100.0));}if pass>=4{samples[mode].push(start.elapsed().as_secs_f64()*1e6/1000.0);}}}
    for s in &mut samples{s.sort_by(f64::total_cmp);}
    eprintln!("text-cache: 12 GPU pixel comparisons passed; layout CPU median fresh={:.3}us cached={:.3}us per label",samples[0][samples[0].len()/2],samples[1][samples[1].len()/2]);
}

#[cfg(test)]mod text_cache_tests{
    use super::*;use std::cell::Cell;
    fn text()->Txt{Node::from_json(&serde_json::json!({"name":"Text","active":true,"comps":[{"type":"TextMeshProUGUI","enabled":true,"m_text":"Some words for wrapping","m_enableAutoSizing":true,"m_enableWordWrapping":true,"m_fontSize":48,"m_fontSizeMin":8,"m_fontSizeMax":72}]})).txt.unwrap()}
    fn evaluate(t:&Txt,id:(u64,[u32;4]),w:f32,h:f32,calls:&Cell<u32>)->TextLayout{
        t.layout_with(id,w,h,|text,size|{calls.set(calls.get()+1);vec2(text.lines().map(|l|l.len()).max().unwrap_or(0) as f32*size*0.5,text.lines().count() as f32*size)},|text,_|{calls.set(calls.get()+1);text.replace(' ',"\n")}).clone()
    }
    #[test]fn unchanged_text_skips_measurement_and_wrapping(){
        let t=text();let calls=Cell::new(0);let a=evaluate(&t,(1,[0;4]),120.0,80.0,&calls);let first=calls.get();assert!(first>0);
        let b=evaluate(&t,(1,[0;4]),120.0,80.0,&calls);assert_eq!(calls.get(),first);assert_eq!(a.text,b.text);assert_eq!(a.size.to_bits(),b.size.to_bits());
    }
    #[test]fn every_layout_input_invalidates_cached_results(){
        let changes:[fn(&mut Txt);9]=[|t|t.text.push_str(" more"),|t|t.font="Other".into(),|t|t.case=16,|t|t.auto=false,|t|t.wrap=false,|t|t.size+=1.0,|t|t.min+=1.0,|t|t.max+=1.0,|t|t.spacing+=1.0];
        for change in changes{let mut t=text();let calls=Cell::new(0);evaluate(&t,(1,[0;4]),120.0,80.0,&calls);let before=calls.get();change(&mut t);evaluate(&t,(1,[0;4]),120.0,80.0,&calls);assert!(calls.get()>before);}
        for (id,w,h) in [((2,[0;4]),120.0,80.0),((1,[1,0,0,0]),120.0,80.0),((1,[0;4]),121.0,80.0),((1,[0;4]),120.0,81.0)]{
            let t=text();let calls=Cell::new(0);evaluate(&t,(1,[0;4]),120.0,80.0,&calls);let before=calls.get();evaluate(&t,id,w,h,&calls);assert!(calls.get()>before);
        }
    }
    #[test]fn visual_only_changes_do_not_rebuild_layout_and_clones_stay_independent(){
        let mut t=text();let calls=Cell::new(0);let a=evaluate(&t,(1,[0;4]),120.0,80.0,&calls);let before=calls.get();
        t.color=RED;t.tint=BLUE;t.h=4;t.v=1024;t.bold=true;let b=evaluate(&t,(1,[0;4]),120.0,80.0,&calls);assert_eq!(calls.get(),before);assert_eq!(a.text,b.text);
        let mut cloned=t.clone();cloned.text="Different".into();evaluate(&cloned,(1,[0;4]),120.0,80.0,&calls);assert_eq!(evaluate(&t,(1,[0;4]),120.0,80.0,&calls).text,a.text);
    }
}

#[cfg(test)]
mod selectable_regressions {
    use super::*;
    #[test]
    fn native_color_block_fades_between_hover_press_and_disabled() {
        let normal=Color::new(0.2,0.2,0.2,1.0);
        let mut state=Selectable {target:1,transition:1,interactable:true,
            colors:[normal,Color::new(0.8,0.8,0.8,1.0),Color::new(0.1,0.1,0.1,1.0),Color::new(0.3,0.3,0.3,0.5)],
            duration:0.1,current:normal,from:normal,desired:normal,began:None,pressed:false};
        assert_eq!(state.sample(false,0.0),normal);
        state.sample(true,1.0);
        assert!((state.sample(true,1.05).r-0.5).abs()<0.001);
        state.pressed=true;state.sample(true,2.0);
        assert!((state.sample(true,2.2).r-0.1).abs()<0.001);
        state.interactable=false;state.sample(true,3.0);
        assert!((state.sample(true,3.2).a-0.5).abs()<0.001);
    }
    #[test]
    fn graphic_tint_stays_inside_each_cloned_control() {
        let source=serde_json::json!({"name":"Button","id":1,"active":true,"children":[
            {"name":"Image","id":2,"active":true,"comps":[{"type":"Image","color":[1,1,1,1]}]}]});
        let mut first=Node::from_json(&source);let second=first.clone();
        assert!(first.tint_target(2,RED));
        assert_eq!(first.children[0].img.as_ref().unwrap().tint,RED);
        assert_eq!(second.children[0].img.as_ref().unwrap().tint,WHITE);
    }
}
