use std::fmt;

use palette::{
    rgb::Rgb, FromColor, Hsva, IntoColor, LinSrgb, LinSrgba, RgbHue, Srgb, Srgba, WithAlpha,
};

// hrm, color is confusing, so make a newtype around LinSrgba for all our color stuff
// i need to double check i'm handling linear/not rgb right
#[derive(Copy, Clone, Default)]
pub struct MurreletColor(LinSrgba);

impl fmt::Debug for MurreletColor {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let [h, s, v, a] = self.into_hsva_components();
        write!(f, "Color {{ h: {}, s: {}, v: {}, a: {} }}", h, s, v, a)
    }
}

impl MurreletColor {
    pub fn from_palette_linsrgba(c: LinSrgba) -> Self {
        Self(c)
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

    pub fn hsva(h: f32, s: f32, v: f32, a: f32) -> MurreletColor {
        let c = Hsva::new(RgbHue::from_degrees(h * 360.0), s, v, a);
        Self::from_hsva(c)
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

        MurreletColor(c)
    }

    pub fn rgb_u8_tuple(rgb: (u8, u8, u8)) -> Self {
        let (r, g, b) = rgb;
        Self::rgb_u8(r, g, b)
    }

    // from palette

    pub fn from_srgb(c: Srgb) -> Self {
        let c = LinSrgb::from_color(c).with_alpha(1.0);
        MurreletColor(c)
    }

    pub fn from_srgb_u8(c: Srgb<u8>) -> Self {
        Self::rgb_u8(c.red, c.green, c.blue)
    }

    pub fn from_srgba(c: Srgba) -> Self {
        let c = LinSrgba::from_color(c);
        MurreletColor(c)
    }

    pub fn from_rgb_u8(c: Rgb<Srgb, u8>) -> Self {
        Self::rgb_u8(c.red, c.green, c.blue)
    }

    pub fn from_hsva(c: Hsva) -> Self {
        let c = Srgba::from_color(c).into_color();
        MurreletColor(c)
    }

    // getting info out of it

    pub fn into_hsva_components(&self) -> [f32; 4] {
        let srgba: Srgba = self.0.into_format().into_color();
        let hsva: Hsva = Hsva::from_color(srgba);
        [
            hsva.hue.into_raw_degrees() / 360.0,
            hsva.saturation,
            hsva.value,
            self.0.alpha,
        ]
    }

    pub fn into_rgba_components(&self) -> [f32; 4] {
        let srgb: Srgba = self.0.into_format().into_color();
        srgb.into_components().into()
    }

    pub fn to_linsrgba(&self) -> LinSrgba {
        self.0.clone()
    }

    pub fn with_alpha(&self, alpha: f32) -> MurreletColor {
        Self(self.0.with_alpha(alpha))
    }

    pub fn to_svg_rgb(&self) -> String {
        let [r, g, b, a] = self.into_rgba_components();
        format!(
            "rgba({} {} {} / {})",
            (r * 255.0) as i32,
            (g * 255.0) as i32,
            (b * 255.0) as i32,
            a
        )
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
        MurreletColor(*self)
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
