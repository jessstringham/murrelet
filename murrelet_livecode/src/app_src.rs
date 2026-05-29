#![allow(dead_code)]
use std::collections::HashMap;

use glam::{Vec2, vec2};
use murrelet_common::{
    CustomVars, IsLivecodeSrc, LivecodeSrcUpdateInput, LivecodeValue, StrId, ToStrId,
};

// hacky, and maybe should include more keys or maybe it has too many, but this is quick to type (kDt)
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub enum MurreletKey {
    A,
    B,
    C,
    D,
    E,
    F,
    G,
    H,
    I,
    J,
    K,
    L,
    M,
    N,
    O,
    P,
    Q,
    R,
    S,
    T,
    U,
    V,
    W,
    X,
    Y,
    Z,
}
impl MurreletKey {
    fn to_str(&self) -> &str {
        match self {
            MurreletKey::A => "A",
            MurreletKey::B => "B",
            MurreletKey::C => "C",
            MurreletKey::D => "D",
            MurreletKey::E => "E",
            MurreletKey::F => "F",
            MurreletKey::G => "G",
            MurreletKey::H => "H",
            MurreletKey::I => "I",
            MurreletKey::J => "J",
            MurreletKey::K => "K",
            MurreletKey::L => "L",
            MurreletKey::M => "M",
            MurreletKey::N => "N",
            MurreletKey::O => "O",
            MurreletKey::P => "P",
            MurreletKey::Q => "Q",
            MurreletKey::R => "R",
            MurreletKey::S => "S",
            MurreletKey::T => "T",
            MurreletKey::U => "U",
            MurreletKey::V => "V",
            MurreletKey::W => "W",
            MurreletKey::X => "X",
            MurreletKey::Y => "Y",
            MurreletKey::Z => "Z",
        }
    }
}

#[derive(Debug, Clone)]
pub struct AppInputValues {
    window_dims: Vec2,
    keys_fire: [bool; 26],    // which key is currently pressed
    keys_changed: [bool; 26], // which keys have just changed
    keys_cycle: [u32; 26], // how many times a key has been pressed, so you can compute if it's triggered
    key_trigger_names: [StrId; 26],
    key_fire_names: [StrId; 26],
    lookup: HashMap<MurreletKey, usize>,
    click_fire: bool,
    click_changed: bool,
    click_cycle: u32,
    click_loc: Vec2,
    mouse_loc: Vec2, // doesn't need a click
    // When true, `update()` won't overwrite `mouse_loc` from incoming app input.
    // Headless arms pass a default MurreletAppInput (mouse at Vec2::ZERO) every
    // frame, which would otherwise clobber any non-zero seed and collapse
    // `[mx, my]`-driven config geometry. See BUG-L348.
    freeze_mouse_loc: bool,
    // can refactor. for now, this is a quick way to exclude, say, keyboard things from livecode web
    include_keyboard: bool,
    custom_vars: CustomVars,
}
impl AppInputValues {
    // there's a nicer way to write this for keys...
    fn result(
        &self,
        has_click: bool,
        cx: f32,
        cy: f32,
        mx: f32,
        my: f32,
        keys_cycle: [u32; 26],
        key_fire: [bool; 26],
        w: f32,
        h: f32,
    ) -> Vec<(StrId, LivecodeValue)> {
        let mut r = if self.include_keyboard {
            let mut v = Vec::with_capacity(26 + 26 + 5);
            for i in 0..26 {
                v.push((
                    self.key_trigger_names[i],
                    LivecodeValue::Bool(keys_cycle[i] % 2 == 1),
                ));
                v.push((self.key_fire_names[i], LivecodeValue::Bool(key_fire[i])));
            }
            v
        } else {
            Vec::with_capacity(5)
        };

        r.extend(vec![
            ("has_click".to_strid(), LivecodeValue::Bool(has_click)),
            ("cx".to_strid(), LivecodeValue::Float(cx as f64)),
            ("cy".to_strid(), LivecodeValue::Float(cy as f64)),
            ("mx".to_strid(), LivecodeValue::Float(mx as f64)),
            ("my".to_strid(), LivecodeValue::Float(my as f64)),
            ("w".to_strid(), LivecodeValue::Float(w as f64)),
            ("h".to_strid(), LivecodeValue::Float(h as f64)),
        ]);
        r.extend(self.custom_vars.to_exec_funcs());
        r
    }
}

impl IsLivecodeSrc for AppInputValues {
    fn to_exec_funcs(&self) -> Vec<(StrId, LivecodeValue)> {
        let dims = self.window_dims;

        let has_click = self.click_fire;
        let cx = self.click_loc.x;
        let cy = self.click_loc.y;
        let mx = self.mouse_loc.x;
        let my = self.mouse_loc.y;

        let w = dims.x;
        let h = dims.y;

        // for now just do one..
        let keys_cycle = self.keys_cycle;
        let keys_fired = self.keys_fire;

        // i don't remember why i wrote it this way...
        self.result(has_click, cx, cy, mx, my, keys_cycle, keys_fired, w, h)
    }

    fn update(&mut self, src_input: &LivecodeSrcUpdateInput) {
        let app = src_input.app();

        self.keys_changed = [false; 26];
        if let Some(keys) = app.keys {
            for (idx, &k) in keys.iter().enumerate() {
                if k != self.keys_fire[idx] {
                    self.keys_changed[idx] = true;

                    if k {
                        self.keys_cycle[idx] += 1;
                    }
                }
                self.keys_fire[idx] = k;
            }
        }

        if !self.freeze_mouse_loc {
            self.mouse_loc = app.mouse_position;
        }
        // only update clicks if they are clicking!
        self.click_fire = false;
        self.click_changed = false;
        if app.mouse_left_is_down {
            self.click_loc = self.mouse_loc;
            self.click_fire = true;
            self.click_cycle += 1;
            self.click_changed = true;
        }

        self.custom_vars.update(&src_input.app().custom_vars);

        self.window_dims = app.window_dims;
    }
}

impl AppInputValues {
    const VALID_KEYS: [MurreletKey; 26] = [
        MurreletKey::A,
        MurreletKey::B,
        MurreletKey::C,
        MurreletKey::D,
        MurreletKey::E,
        MurreletKey::F,
        MurreletKey::G,
        MurreletKey::H,
        MurreletKey::I,
        MurreletKey::J,
        MurreletKey::K,
        MurreletKey::L,
        MurreletKey::M,
        MurreletKey::N,
        MurreletKey::O,
        MurreletKey::P,
        MurreletKey::Q,
        MurreletKey::R,
        MurreletKey::S,
        MurreletKey::T,
        MurreletKey::U,
        MurreletKey::V,
        MurreletKey::W,
        MurreletKey::X,
        MurreletKey::Y,
        MurreletKey::Z,
    ];

    pub fn all_keys_fire_bool(&self) -> HashMap<MurreletKey, bool> {
        Self::VALID_KEYS
            .into_iter()
            .map(|key| (key, self.key_fire_bool(key)))
            .collect()
    }

    pub fn key_cycle_bool(&self, key: MurreletKey) -> bool {
        // just need to check if this one's pressed right now
        if let Some(k) = self.lookup.get(&key) {
            self.keys_cycle[*k].is_multiple_of(2)
        } else {
            false
        }
    }

    pub fn key_fire_bool(&self, key: MurreletKey) -> bool {
        // just need to check if this one's pressed right now
        if let Some(k) = self.lookup.get(&key) {
            self.keys_changed[*k] && self.keys_fire[*k]
        } else {
            false
        }
    }

    pub fn click(&self) -> Option<Vec2> {
        if self.click_fire {
            Some(self.click_loc)
        } else {
            None
        }
    }

    pub fn has_click(&self) -> bool {
        self.click_fire
    }

    // Headless variant: seeds `mouse_loc` to a non-zero default so configs
    // whose geometry derives from `[mx, my]` (e.g. leafed's blade.loc) don't
    // collapse to (0,0) and render degenerate. Leaves uses centered-at-origin
    // world coords with sketches typically seeded below center (e.g. leafed
    // seeds at y=-100..-250); (0, -200) lands the headless mouse inside that
    // typical seeded region so the auxin/vein geometry can actually grow.
    // The companion `freeze_mouse_loc` flag prevents `update()` from clobbering
    // this value with the default Vec2::ZERO from `MurreletAppInput`.
    // See BUG-L348.
    pub fn new_centered(include_keyboard: bool) -> AppInputValues {
        let mut v = Self::new(include_keyboard);
        v.mouse_loc = vec2(0.0, -200.0);
        v.freeze_mouse_loc = true;
        v
    }

    pub fn new(include_keyboard: bool) -> AppInputValues {
        let lookup = Self::VALID_KEYS
            .iter()
            .enumerate()
            .map(|(a, b)| (*b, a))
            .collect();

        // build these once
        let key_trigger_names =
            std::array::from_fn(|i| StrId::new(&format!("k{}t", Self::VALID_KEYS[i].to_str())));
        let key_fire_names =
            std::array::from_fn(|i| StrId::new(&format!("k{}f", Self::VALID_KEYS[i].to_str())));

        AppInputValues {
            window_dims: vec2(100.0, 100.0), // todo, is this supposed to be updated?
            keys_fire: [false; 26],
            keys_changed: [false; 26],
            keys_cycle: [0; 26],
            lookup,
            click_fire: false,
            click_changed: false,
            click_cycle: 0,
            mouse_loc: Vec2::ZERO,
            click_loc: Vec2::ZERO,
            freeze_mouse_loc: false,
            include_keyboard,
            custom_vars: CustomVars::default(),
            key_trigger_names,
            key_fire_names,
        }
    }

    pub fn window_dims(&self) -> Vec2 {
        self.window_dims
    }
}

impl Default for AppInputValues {
    fn default() -> Self {
        Self::new(false)
    }
}
