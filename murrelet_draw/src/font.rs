use std::collections::HashMap;

use glam::*;
use itertools::Itertools;
use lerpable::Lerpable;
use murrelet_common::*;
use murrelet_livecode::Livecode;

use crate::svg_read::{load_all_data, load_all_data_from_str};

const GLYPH_SVG: &str = include_str!("../assets/timesr.svg");

// use this letter to figure out how to scael it
const CAP_REFERENCE: &str = "H";

#[derive(Clone, Debug)]
pub struct Letterform {
    #[allow(dead_code)]
    conf: FontConf,
    lines: Vec<Vec<Vec2>>,
    width: f32,
}
impl Letterform {
    pub fn new(raw_data: Vec<Vec<Vec2>>, conf: &FontConf) -> Self {
        let bounds = BoundMetric::new_from_vec_vecs(&raw_data);
        let lower_left = vec2(bounds.lower_left().x, conf.lowercase_baseline);
        let width = bounds.width();

        let mut lines = raw_data;
        for row in &mut lines {
            for vec in row {
                *vec -= lower_left;
            }
        }

        Self {
            conf: conf.clone(),
            lines,
            width,
        }
    }

    pub fn lines_as_pen_up_and_down(
        &self,
        point_count: usize,
        repeat_endpoints: usize,
    ) -> Vec<(Vec2, bool)> {
        let mut maybe_last_val = None;

        let mut result = vec![];
        for line in &self.lines {
            if let (Some(last_val), Some(first_val)) = (maybe_last_val, line.first()) {
                // need to create some points between these. they'll probably
                // actually be curved, but hopefully the fourier stuff will help there.

                for _ in 0..repeat_endpoints {
                    result.push((last_val, false));
                }

                let vals = lerp_x_points_between(last_val, *first_val, point_count)
                    .into_iter()
                    .map(|x| (x, false))
                    .collect_vec();

                result.extend_from_slice(&vals);

                for _ in 0..repeat_endpoints {
                    result.push((*first_val, false));
                }
            }

            for v in line {
                result.push((*v, true));
            }
            maybe_last_val = line.last().copied();
        }
        result
    }

    pub fn lines(&self) -> &Vec<Vec<Vec2>> {
        &self.lines
    }
}

// A kerning pair. Serializes exactly like the `(char, char)` it replaces (a
// 2-element seq), but has a `Display` so nestedit can key on it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct CharPair(pub char, pub char);

impl std::fmt::Display for CharPair {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}{}", self.0, self.1)
    }
}

#[derive(Debug, Clone, Livecode, Lerpable)]
pub struct FontConf {
    filename: String,
    inter_char_spacing_len: usize, // how many points to put in between pen lift within a character
    repeat_endpoints: usize,
    font_tolerance: f32, // sent to svg reader, not sure what it's doing right now..
    #[livecode(kind = "none")]
    #[lerpable(method = "skip")]
    mapping: HashMap<CharPair, f32>,
    spacing: f32,            // spacing between all letters
    lowercase_baseline: f32, // where the lowercase baseline shoudl be

    // these add a few more points so the osc works better
    // return len is split between the beginning and end
    // the expectation is that osc won't actually use the x and y values here
    return_len: usize,
    spacing_len: usize,
    word_gap: f32,

    // if we need to rotate it...
    rotate_quarter_turn: bool,
}

impl FontConf {
    pub fn load(&self) -> HashMap<String, Letterform> {
        self.letterforms(load_all_data(&self.filename, self.font_tolerance))
    }

    pub fn load_str(&self, svg: &str) -> HashMap<String, Letterform> {
        self.letterforms(load_all_data_from_str(svg, self.font_tolerance))
    }

    fn letterforms(
        &self,
        raw_data: HashMap<String, Vec<Vec<Vec2>>>,
    ) -> HashMap<String, Letterform> {
        let mut result = HashMap::new();

        // let current_loc = None;

        // in this case we drop layers and use the data we have

        for (k, v) in raw_data.iter() {
            // todo, v should be a vec of vecs...

            // hrm, rotate it, might need to fix this
            let mut pts_as_vec2 = v.clone();
            for row in &mut pts_as_vec2 {
                for item in row {
                    *item = if self.rotate_quarter_turn {
                        vec2(item.y, -item.x)
                    } else {
                        vec2(item.x, -item.y)
                    };
                }
            }

            // todo, add in some points for moving in between

            result.insert(k.clone(), Letterform::new(pts_as_vec2, self));
        }

        result
    }
}

impl Default for FontConf {
    fn default() -> Self {
        Self {
            filename: String::new(), // the default font is embedded, not read
            inter_char_spacing_len: 15,
            repeat_endpoints: 5,
            font_tolerance: 1.0,
            mapping: HashMap::new(),
            spacing: 2.0,
            lowercase_baseline: 0.0, // hershey units: the baseline is y=0
            return_len: 32,
            spacing_len: 12,
            word_gap: 10.0,
            rotate_quarter_turn: false,
        }
    }
}

#[derive(Debug, Clone)]
pub struct MurreletFont {
    data: HashMap<String, Letterform>,
    cap_height: f32,
}

impl MurreletFont {
    pub fn new(data: HashMap<String, Letterform>) -> MurreletFont {
        let cap_height = data
            .get(CAP_REFERENCE)
            .map(|l| BoundMetric::new_from_vec_vecs(l.lines()).height())
            .filter(|h| *h > 0.0)
            .unwrap_or(1.0);
        MurreletFont { data, cap_height }
    }

    pub fn load(conf: &FontConf) -> Self {
        Self::new(FontConf::load(conf))
    }

    pub fn cap_height(&self) -> f32 {
        self.cap_height
    }

    pub fn to_shapes(&self, what: &str, conf: &FontConf) -> Vec<(Vec2, bool)> {
        let mut result = vec![];

        let mut x_loc = 0.0;

        let mut maybe_last_loc = None;
        let mut maybe_first_loc = None;

        for char in what.chars() {
            if let Some(letterform) = self.data.get(&char.to_string()) {
                let lines: Vec<(Vec2, bool)> = letterform
                    .lines_as_pen_up_and_down(conf.inter_char_spacing_len, conf.repeat_endpoints); // todo, actually i think that logic shoudl be here...

                if maybe_first_loc.is_none() {
                    maybe_first_loc = lines.first().map(|x| x.0);
                }

                if let (Some(last_loc), Some((raw_curr_loc, _))) = (maybe_last_loc, lines.first()) {
                    let first_loc = *raw_curr_loc + vec2(x_loc, 0.0);

                    // let change = 1.0 / conf.spacing_len as f32;
                    // for i in 0..conf.spacing_len {
                    //     // since these are false, i'd assume the x/y isn't used
                    //     result.push((prev_loc + i as f32 * change * (curr_loc - prev_loc), false));
                    // }

                    let mut v = vec![];
                    for _ in 0..conf.repeat_endpoints {
                        v.push(last_loc);
                    }

                    v.extend_from_slice(&lerp_x_points_between(
                        last_loc,
                        first_loc,
                        conf.return_len,
                    ));

                    for _ in 0..conf.repeat_endpoints {
                        v.push(first_loc);
                    }

                    let r = v.into_iter().map(|x| (x, false)).collect_vec();

                    result.extend_from_slice(&r);
                }

                for (pt, is_visible) in &lines {
                    let v = *pt + vec2(x_loc, 0.0);
                    result.push((v, *is_visible));
                    maybe_last_loc = Some(v);
                }

                // add spacing to result

                x_loc += letterform.width + conf.spacing;
            }
        }

        if let (Some(last_loc), Some(first_loc)) = (maybe_last_loc, maybe_first_loc) {
            // let delta = 1.0 / conf.return_len as f32;
            // for i in 0..conf.return_len {
            //     // since these are false, i'd assume the x/y isn't used
            //     result.push((last_loc + i as f32 * delta * (first_loc - last_loc), false));
            // }

            let mut v = vec![];
            for _ in 0..conf.repeat_endpoints {
                v.push(last_loc);
            }

            v.extend_from_slice(&lerp_x_points_between(last_loc, first_loc, conf.return_len));

            for _ in 0..conf.repeat_endpoints {
                v.push(first_loc);
            }

            let r = v.into_iter().map(|x| (x, false)).collect_vec();

            result.extend_from_slice(&r);
        }

        result
    }

    pub fn to_shapes_separate(&self, what: &str, conf: &FontConf) -> (BoundMetric, Vec<Vec<Vec2>>) {
        let mut result = vec![];

        let mut bounds = BoundMetric::new();

        let mut x_loc = 0.0;

        for char in what.chars() {
            if char.is_whitespace() {
                x_loc += conf.word_gap;
                continue;
            }

            if let Some(letterform) = self.data.get(&char.to_string()) {
                let lines = letterform.lines(); // todo, actually i think that logic shoudl be here...

                for line in lines {
                    let mut vv = vec![];
                    for pt in line {
                        let v = *pt + vec2(x_loc, 0.0);
                        bounds.add_point(v);
                        vv.push(v);
                    }
                    result.push(vv);
                }

                // add spacing to result

                x_loc += letterform.width + conf.spacing;
            }
        }

        (bounds, result)
    }

    // some claude
    pub fn default_font() -> &'static MurreletFont {
        static FONT: std::sync::OnceLock<MurreletFont> = std::sync::OnceLock::new();
        FONT.get_or_init(|| MurreletFont::new(FontConf::default().load_str(GLYPH_SVG)))
    }
    pub fn text_to_polylines(
        &self,
        text: &str,
        loc: Vec2,
        font_size: f32,
        conf: &FontConf,
    ) -> Vec<Vec<Vec2>> {
        let (bounds, mut runs) = self.to_shapes_separate(text, conf);
        if runs.is_empty() {
            return runs;
        }
        let scale = font_size / self.cap_height;

        let anchor = vec2(bounds.center().x, self.cap_height * 0.5);
        for run in &mut runs {
            for p in run.iter_mut() {
                *p = (*p - anchor) * scale + loc;
            }
        }
        runs
    }
}
