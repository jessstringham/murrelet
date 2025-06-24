use std::{
    fmt,
    ops::{Add, Mul},
};

use lerpable::{IsLerpingMethod, Lerpable};
use murrelet_gui::CanMakeGUI;
use palette::{rgb::Rgb, FromColor, Hsva, IntoColor, LinSrgb, LinSrgba, Srgb, Srgba, WithAlpha};

use crate::rgb_to_hex;

// hrm, color is confusing, so make a newtype around LinSrgba for all our color stuff
// i need to double check i'm handling linear/not rgb right
#[derive(Copy, Clone, Default)]
pub struct MurreletColor([f32; 4]); // hsva

impl fmt::Debug for MurreletColor {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let [h, s, v, a] = self.into_hsva_components();
        write!(f, "Color {{ h: {}, s: {}, v: {}, a: {} }}", h, s, v, a)
    }
}

impl MurreletColor {
    // 0 to 1
    pub fn alpha(&self) -> f32 {
        let [_, _, _, a] = self.into_hsva_components();
        a
    }

    pub fn from_palette_linsrgba(c: LinSrgba) -> Self {
        let srgba: Srgba = Srgba::from_linear(c.into_format::<f32, f32>());
        Self::from_hsva(Hsva::from_color(srgba))
    }

    pub fn from_hsva(c: Hsva) -> Self {
        // let c = Srgba::from_color(c).into_color();

        let (h, s, v, a) = c.into_components();
        Self([h.into_degrees() / 360.0, s, v, a])
    }

    pub fn hsva(h: f32, s: f32, v: f32, a: f32) -> MurreletColor {
        // let c = Hsva::new(RgbHue::from_degrees(h * 360.0), s, v, a);
        // Self::from_hsva(c)

        Self([h, s, v, a])
    }

    pub fn srgb(r: f32, g: f32, b: f32) -> Self {
        let c = Srgb::new(r, g, b);
        Self::from_srgb(c)
    }

    pub fn rgba(r: f32, g: f32, b: f32, a: f32) -> Self {
        let c = Srgba::new(r, g, b, a);
        Self::from_srgba(c)
    }

    pub fn gray(g: f32) -> Self {
        Self::srgb(g, g, g)
    }

    pub fn rgb_u8(r: u8, g: u8, b: u8) -> Self {
        let r = r as f32 / 255.0;
        let g = g as f32 / 255.0;
        let b = b as f32 / 255.0;
        let c = LinSrgba::from_components((r, g, b, 1.0));

        MurreletColor::from_palette_linsrgba(c)
    }

    pub fn rgb_u8_tuple(rgb: (u8, u8, u8)) -> Self {
        let (r, g, b) = rgb;
        Self::rgb_u8(r, g, b)
    }

    // from palette

    pub fn from_srgb(c: Srgb) -> Self {
        let c = LinSrgb::from_color(c).with_alpha(1.0);
        MurreletColor::from_palette_linsrgba(c)
    }

    pub fn from_srgba(c: Srgba) -> Self {
        let c = LinSrgba::from_color(c);
        MurreletColor::from_palette_linsrgba(c)
    }

    pub fn from_rgb_u8(c: Rgb<Srgb, u8>) -> Self {
        Self::rgb_u8(c.red, c.green, c.blue)
    }

    pub fn from_srgb_u8(c: Srgb<u8>) -> Self {
        Self::rgb_u8(c.red, c.green, c.blue)
    }

    // getting info out of it

    pub fn into_hsva_components(&self) -> [f32; 4] {
        // let srgba: Srgba = self.0.into_format().into_color();
        // println!("sRGBA before HSV conversion: {:?}", srgba);

        // let hsva: Hsva = Hsva::from_color(srgba);
        self.0
    }

    pub fn to_srgba(&self) -> Srgba {
        let [h, s, v, a] = self.into_hsva_components();
        let hsva: Hsva = Hsva::from_components((h * 360.0, s, v, a));
        Srgba::from_color(hsva)
    }

    pub fn into_rgba_components(&self) -> [f32; 4] {
        self.to_srgba().into_components().into()
    }

    pub fn to_linsrgba(&self) -> LinSrgba {
        self.to_srgba().into_color()
    }

    pub fn with_alpha(&self, alpha: f32) -> MurreletColor {
        let [h, s, v, _a] = self.into_hsva_components();
        Self([h, s, v, alpha])
    }

    pub fn black() -> Self {
        Self::hsva(0.0, 0.0, 0.0, 1.0)
    }

    pub fn white() -> Self {
        Self::rgba(1.0, 1.0, 1.0, 1.0)
    }

    pub fn transparent() -> Self {
        Self::hsva(0.0, 0.0, 0.0, 0.0)
    }

    // shorthand for a really bright color
    pub fn hue(h: f32) -> MurreletColor {
        Self::hsva(h, 1.0, 1.0, 1.0)
    }

    pub fn to_svg_rgb(&self) -> String {
        self.hex()
    }

    pub fn to_fill_opacity(&self) -> String {
        format!("{}", self.alpha())
    }

    pub fn hex(&self) -> String {
        let [r, g, b, _a] = self.into_rgba_components();
        rgb_to_hex(r, g, b)
    }
}

impl CanMakeGUI for MurreletColor {
    fn make_gui() -> murrelet_gui::MurreletGUISchema {
        murrelet_gui::MurreletGUISchema::Val(murrelet_gui::ValueGUI::Color)
    }
}

pub trait MurreletIntoLinSrgba {
    fn into_murrelet_color(&self) -> MurreletColor;
}

impl MurreletIntoLinSrgba for Rgb<Srgb, u8> {
    fn into_murrelet_color(&self) -> MurreletColor {
        MurreletColor::from_rgb_u8(*self)
    }
}

impl MurreletIntoLinSrgba for LinSrgba {
    fn into_murrelet_color(&self) -> MurreletColor {
        MurreletColor::from_palette_linsrgba(*self)
    }
}

impl MurreletIntoLinSrgba for Srgb {
    fn into_murrelet_color(&self) -> MurreletColor {
        MurreletColor::from_srgb(*self)
    }
}

impl MurreletIntoLinSrgba for Hsva {
    fn into_murrelet_color(&self) -> MurreletColor {
        MurreletColor::from_hsva(*self)
    }
}

impl Lerpable for MurreletColor {
    fn lerpify<T: IsLerpingMethod>(&self, other: &Self, method: &T) -> Self {
        let [h, s, v, a] = self.into_hsva_components();
        let [h2, s2, v2, a2] = other.into_hsva_components();
        MurreletColor::hsva(
            h.lerpify(&h2, method),
            s.lerpify(&s2, method),
            v.lerpify(&v2, method),
            a.lerpify(&a2, method),
        )
    }
}

impl Add for MurreletColor {
    type Output = Self;

    fn add(self, other: Self) -> Self::Output {
        let [h, s, v, a] = self.into_hsva_components();
        let [h2, s2, v2, a2] = other.into_hsva_components();
        MurreletColor::hsva(h + h2, s + s2, v + v2, a + a2)
    }
}

impl Mul<f32> for MurreletColor {
    type Output = Self;

    fn mul(self, scalar: f32) -> Self::Output {
        let [h, s, v, a] = self.into_hsva_components();

        MurreletColor::hsva(h * scalar, s * scalar, v * scalar, a * scalar)
    }
}
