#![allow(dead_code)]
use std::{
    cell::RefCell,
    collections::{HashMap, HashSet},
    rc::Rc,
};

use glam::*;
use itertools::Itertools;
use murrelet_common::*;
use murrelet_draw::{
    curve_drawer::CurveDrawer,
    draw::{MurreletColorStyle, MurreletStyle},
    newtypes::RGBandANewtype,
    style::{MurreletCurve, MurreletPath, StyledPath, StyledPathSvgFill},
};
use murrelet_perform::perform::SvgDrawConfig;
use svg::{node::element::path::Data, Document, Node};

#[derive(Debug, Clone)]
pub struct StyledText {
    text: String,
    loc: Vec2,
    size: f32,
    style: MurreletStyle,
}

impl StyledText {
    pub fn new(text: String, loc: Vec2, size: f32, style: MurreletStyle) -> Self {
        Self {
            text,
            loc,
            size,
            style,
        }
    }

    fn transform_with<T: TransformVec2>(&self, t: &T) -> StyledText {
        Self {
            loc: t.transform_vec2(self.loc),
            ..self.clone()
        }
    }
}

pub trait ToSvgData {
    fn to_svg(&self) -> Option<Data>;
}

impl ToSvgData for MurreletCurve {
    fn to_svg(&self) -> Option<Data> {
        self.curve().to_svg()
    }
}

impl ToSvgData for MurreletPath {
    fn to_svg(&self) -> Option<Data> {
        match self {
            MurreletPath::Polyline(path) => path.into_iter_vec2().collect_vec().to_svg(),
            MurreletPath::Curve(c) => c.to_svg(),
        }
    }
}

pub trait ToSvgMatrix {
    fn to_svg_matrix(&self) -> String;
}
impl ToSvgMatrix for Mat4 {
    fn to_svg_matrix(&self) -> String {
        let [[a, b, _, _], [c, d, _, _], _, [e, f, _, _]] = self.to_cols_array_2d();

        let value = format!("matrix({}, {}, {}, {}, {}, {})", a, b, c, d, e, f);

        value
    }
}

trait AddSvgAttributes {
    fn add_svg_attributes(&self, p: svg::node::element::Path) -> svg::node::element::Path;
}

impl AddSvgAttributes for StyledPathSvgFill {
    fn add_svg_attributes(&self, p: svg::node::element::Path) -> svg::node::element::Path {
        p.set("fill", format!("url(#P{})", self.hash()))
    }
}

impl AddSvgAttributes for MurreletColor {
    fn add_svg_attributes(&self, p: svg::node::element::Path) -> svg::node::element::Path {
        let [r, g, b, a] = self.into_rgba_components();
        let fill = format!(
            "rgba({} {} {} / {})",
            (r * 255.0) as i32,
            (g * 255.0) as i32,
            (b * 255.0) as i32,
            a
        );
        p.set("fill", fill)
    }
}

impl AddSvgAttributes for RGBandANewtype {
    fn add_svg_attributes(&self, p: svg::node::element::Path) -> svg::node::element::Path {
        self.color().add_svg_attributes(p)
    }
}

impl AddSvgAttributes for MurreletColorStyle {
    fn add_svg_attributes(&self, p: svg::node::element::Path) -> svg::node::element::Path {
        match self {
            MurreletColorStyle::Color(c) => c.add_svg_attributes(p),
            MurreletColorStyle::RgbaFill(c) => c.add_svg_attributes(p),
            MurreletColorStyle::SvgFill(c) => c.add_svg_attributes(p),
        }
    }
}

impl AddSvgAttributes for MurreletPath {
    fn add_svg_attributes(&self, p: svg::node::element::Path) -> svg::node::element::Path {
        let mut p = p;
        if let Some(t) = self.transform() {
            p = p.set("transform", t.to_svg_matrix());
        }
        p
    }
}

impl AddSvgAttributes for MurreletStyle {
    fn add_svg_attributes(&self, p: svg::node::element::Path) -> svg::node::element::Path {
        let mut p = p;
        if self.stroke_weight > 0.0 {
            p = p.set("stroke-width", self.stroke_weight / 10.0);
        }

        if self.filled {
            p = self.color.add_svg_attributes(p);

            if self.stroke_weight > 0.0 {
                p = p.set("stroke", "black");
            }
        } else {
            p = p.set("fill", "none");
            p = p.set("stroke", "black");
        }

        p
    }
}

impl AddSvgAttributes for StyledPath {
    fn add_svg_attributes(&self, p: svg::node::element::Path) -> svg::node::element::Path {
        let mut p = p;
        p = self.path.add_svg_attributes(p);
        p = self.style.add_svg_attributes(p);
        p
    }
}

impl ToSvgData for StyledPath {
    fn to_svg(&self) -> Option<Data> {
        self.path.to_svg()
    }
}

pub trait ToSvgPath {
    fn make_path(&self) -> Option<svg::node::element::Path>;
    fn make_pattern(&self) -> Option<(String, svg::node::element::Pattern)>;
}

impl ToSvgPath for StyledPath {
    fn make_path(&self) -> Option<svg::node::element::Path> {
        if let Some(d) = self.path.to_svg() {
            let mut d = d;
            if self.style.closed {
                d = d.close();
            }
            let mut p = svg::node::element::Path::new().set("d", d);
            p = self.add_svg_attributes(p);
            Some(p)
        } else {
            None
        }
    }

    fn make_pattern(&self) -> Option<(String, svg::node::element::Pattern)> {
        if let MurreletColorStyle::SvgFill(f) = self.style.color {
            // ooookay so the whole purpose of this is to have pattern transform

            // these reference the canvases being written to!
            let img_src = svg::node::element::Use::new()
                .set("href", format!("#{}Img", f.src.as_str()))
                .set("x", 0)
                .set("y", 0);

            let p = svg::node::element::Pattern::new()
                .set("id", format!("P{}", f.hash())) // now this is the id that
                .set("x", 0)
                .set("y", 0)
                .set("width", f.width)
                .set("height", f.height)
                .set("patternTransform", f.transform.to_svg_matrix())
                .add(img_src);

            Some((f.hash(), p))
        } else {
            None
        }
    }
}

impl TransformVec2 for SvgDocCreator {
    fn transform_vec2(&self, v: Vec2) -> Vec2 {
        self.svg_draw_config.transform_vec2(v)
    }
}

pub struct SvgDocCreator {
    svg_draw_config: SvgDrawConfig,
}
impl SvgDocCreator {
    pub fn new(svg_draw_config: &SvgDrawConfig) -> SvgDocCreator {
        SvgDocCreator {
            svg_draw_config: svg_draw_config.clone(),
        }
    }

    pub fn save_doc(&self, paths: &SvgPathCache) {
        if let Some(svg_draw_config) = self.svg_draw_config.capture_path() {
            let document = self.make_doc(paths);
            let path = svg_draw_config.with_extension("svg");
            println!("{:?}", path);
            std::fs::create_dir_all(path.parent().expect("?")).ok();
            svg::save(path, &document).unwrap();
        }
    }

    fn make_text(&self, text: &StyledText) -> svg::node::element::Text {
        // todo, i'm not sure this is the right size
        let text_size =
            text.size * self.svg_draw_config.full_target_width() / self.svg_draw_config.size();

        let text = svg::node::element::Text::new()
            .set("x", text.loc.x)
            .set("y", text.loc.y)
            .set("text-anchor", "middle")
            .set("font-size", format!("{}px", text_size))
            .add(svg::node::Text::new(text.text.clone()));

        text
    }

    fn make_layer(
        &self,
        name: &String,
        layer: &SvgLayer,
        for_inkscape: bool,
    ) -> (svg::node::element::Group, Vec<svg::node::element::Pattern>) {
        let mut g = if for_inkscape {
            svg::node::element::Group::new()
                .set("inkscape:groupmode", "layer")
                .set("inkscape:label", name.clone())
        } else {
            svg::node::element::Group::new().set("id", format!("layer{}", name))
        };

        let mut seen_pattern_keys = HashSet::new();
        let mut patterns = Vec::new();

        for path in layer.paths.iter() {
            if let Some(d) = path.make_path() {
                // if it's using a svg pattern, we should add that attribute
                if let Some((key, pattern)) = path.make_pattern() {
                    // the path already will have the id on it, so just make sure it's
                    // in the list of defs if
                    let pattern_is_new = seen_pattern_keys.insert(key);
                    if pattern_is_new {
                        patterns.push(pattern);
                    }
                }

                g.append(d)
            }
        }

        for text in layer.text.iter() {
            let t = self.make_text(text);
            g.append(t);
        }

        (g, patterns)
    }

    fn make_html(&self, paths: &SvgPathCache) -> (svg::node::element::Group, String) {
        let mut doc = svg::node::element::Group::new();

        // let mut defs = svg::node::element::Definitions::new();
        let mut defs = vec![];

        for (name, layer) in paths.layers.iter() {
            let (g, patterns) = self.make_layer(name, layer, false);
            doc.append(g);
            for p in patterns {
                defs.push(p.to_string());
            }
        }

        (doc, defs.into_iter().join("\n"))
    }

    // this one's meant for svgs for pen plotters, so it drops fill styles
    fn make_doc(&self, paths: &SvgPathCache) -> Document {
        let target_size = self.svg_draw_config.full_target_width(); // guides are at 10x 10x, gives 1cm margin
        let mut doc = Document::new()
            .set(
                "xmlns:inkscape",
                "http://www.inkscape.org/namespaces/inkscape",
            )
            .set(
                "viewBox",
                (
                    target_size / 2.0,
                    target_size / 2.0,
                    target_size,
                    target_size,
                ),
            )
            .set("width", format!("{:?}mm", target_size))
            .set("height", format!("{:?}mm", target_size));

        for (name, layer) in paths.layers.iter() {
            let (g, _) = self.make_layer(name, layer, true);
            doc.append(g);
        }

        doc
    }

    pub fn create_guides(&self) -> Vec<Vec<Vec2>> {
        let size = 0.5;
        let multi = 8.0;
        let orig_width: f32 = self.svg_draw_config.full_target_width();
        let width = orig_width - 2.0 * size;
        let center = vec2(orig_width, orig_width);
        let guide_size_tall = vec2(size * multi, size);
        let guide_size_wide = vec2(size, size * multi);

        vec![
            Rect::from_xy_wh(center + vec2(-width * 0.5, -width * 0.5), guide_size_tall)
                .to_polyline()
                .vertices()
                .to_vec(),
            Rect::from_xy_wh(center + vec2(-width * 0.5, width * 0.5), guide_size_tall)
                .to_polyline()
                .vertices()
                .to_vec(),
            Rect::from_xy_wh(center + vec2(width * 0.5, -width * 0.5), guide_size_tall)
                .to_polyline()
                .vertices()
                .to_vec(),
            Rect::from_xy_wh(center + vec2(width * 0.5, width * 0.5), guide_size_tall)
                .to_polyline()
                .vertices()
                .to_vec(),
            Rect::from_xy_wh(center + vec2(-width * 0.5, -width * 0.5), guide_size_wide)
                .to_polyline()
                .vertices()
                .to_vec(),
            Rect::from_xy_wh(center + vec2(-width * 0.5, width * 0.5), guide_size_wide)
                .to_polyline()
                .vertices()
                .to_vec(),
            Rect::from_xy_wh(center + vec2(width * 0.5, -width * 0.5), guide_size_wide)
                .to_polyline()
                .vertices()
                .to_vec(),
            Rect::from_xy_wh(center + vec2(width * 0.5, width * 0.5), guide_size_wide)
                .to_polyline()
                .vertices()
                .to_vec(),
        ]
    }

    pub fn bounds(&self) -> Vec<Vec2> {
        let orig_width: f32 = self.svg_draw_config.full_target_width();
        let center = vec2(orig_width, orig_width);
        let guide_size = vec2(
            self.svg_draw_config.target_size(),
            self.svg_draw_config.target_size(),
        );

        Rect::from_xy_wh(center, guide_size).to_vec2()
    }

    pub fn svg_draw_config(&self) -> &SvgDrawConfig {
        &self.svg_draw_config
    }

    pub fn copy_transform(&self) -> SvgDrawConfig {
        self.svg_draw_config.clone()
    }
}

pub fn make_canvas_imgs(canvas_ids: &Vec<String>) -> Vec<svg::node::element::Image> {
    let mut defs = Vec::new();

    for canvas_id in canvas_ids {
        let mut img = svg::node::element::Image::new();
        img = img.set("id", format!("{}Img", canvas_id));
        defs.push(img);
    }

    defs
}

pub struct SvgLayer {
    paths: Vec<StyledPath>,
    text: Vec<StyledText>,
}
impl SvgLayer {
    pub fn new() -> SvgLayer {
        SvgLayer {
            paths: Vec::new(),
            text: Vec::new(),
        }
    }

    pub fn add_simple_path(&mut self, line: Vec<Vec2>) {
        self.add_styled_path(StyledPath::from_path(line));
    }

    pub fn add_styled_path(&mut self, styled_path: StyledPath) {
        self.paths.push(styled_path);
    }

    pub fn clear(&mut self) {
        self.paths = Vec::new();
    }
}

pub type SvgPathCacheRef = Rc<RefCell<SvgPathCache>>;

pub struct SvgPathCache {
    config: SvgDocCreator,
    layers: HashMap<String, SvgLayer>,
}
impl SvgPathCache {
    pub fn new(svg_draw_config: &SvgDrawConfig) -> Self {
        Self {
            config: SvgDocCreator::new(svg_draw_config),
            layers: HashMap::new(),
        }
    }

    pub fn svg_draw(svg_draw_config: &SvgDrawConfig) -> SvgPathCacheRef {
        Rc::new(RefCell::new(SvgPathCache::new(svg_draw_config)))
    }

    pub fn add_guides(&mut self) {
        self.config
            .create_guides()
            .into_iter()
            .for_each(|x| self.add_simple_path_no_transform("guides", x))
    }

    pub fn add_simple_path_no_transform(&mut self, layer: &str, line: Vec<Vec2>) {
        let layer = self
            .layers
            .entry(layer.to_owned())
            .or_insert(SvgLayer::new());
        layer.paths.push(StyledPath::from_path(line));
    }

    pub fn add_simple_path(&mut self, layer: &str, line: Vec<Vec2>) {
        let layer = self
            .layers
            .entry(layer.to_owned())
            .or_insert(SvgLayer::new());
        layer.paths.push(StyledPath::from_path(
            self.config.transform_many_vec2(&line),
        ));
    }

    pub fn add_styled_path(&mut self, layer: &str, styled_path: StyledPath) {
        let layer = self
            .layers
            .entry(layer.to_owned())
            .or_insert(SvgLayer::new());
        layer.paths.push(
            styled_path
                .transform_with_mat4_after(self.config.svg_draw_config().transform_for_size()),
        );
    }

    pub fn add_styled_text(&mut self, layer: &str, text: StyledText) {
        let layer = self
            .layers
            .entry(layer.to_owned())
            .or_insert(SvgLayer::new());
        layer
            .text
            .push(text.transform_with(self.config.svg_draw_config()));
    }

    pub fn clear(&mut self, layer: &str) {
        let layer = self
            .layers
            .entry(layer.to_owned())
            .or_insert(SvgLayer::new());
        layer.clear();
    }

    fn add_closed<P: IsPolyline>(&mut self, layer: &str, path: P) {
        self.add_styled_path(
            layer,
            StyledPath {
                path: MurreletPath::Polyline(path.as_polyline()),
                style: MurreletStyle::new_outline(),
            },
        )
    }

    fn add_closed_default_layer(&mut self, shape: Vec<Vec2>) {
        self.add_closed("default", shape)
    }

    pub fn save_doc(&self) {
        self.config.save_doc(self);
    }

    // can add these to a document. I don't give the full svg so I can leave things
    // like <image> defs alone and just update the paths and patternTransforms.
    pub fn make_html(&self) -> Vec<String> {
        let (paths, defs) = self.config.make_html(self);

        vec![defs.to_string(), paths.to_string()]
    }
}

pub trait ToSvg {
    fn to_svg(&self) -> Option<Data>;

    fn to_svg_closed(&self) -> Option<Data> {
        self.to_svg().map(|x| x.close())
    }
}

impl ToSvg for CurveDrawer {
    fn to_svg(&self) -> Option<Data> {
        let segments = self.segments();
        if segments.is_empty() {
            return None;
        }

        let mut path = Data::new();

        let mut curr_point = None;

        for curve in segments {
            // if this is the first point, then move to the start of this
            if curr_point.is_none() {
                let f = curve.first_point();
                path = path.move_to((f.x, f.y));
                curr_point = Some(f)
            }

            match curve {
                murrelet_draw::curve_drawer::CurveSegment::Arc(a) => {
                    let f = a.first_point();
                    // first make sure we're at the first point
                    if curr_point != Some(f) {
                        path = path.line_to((f.x, f.y));
                    }

                    let last_point = a.last_point();

                    let params = (
                        a.radius,
                        a.radius, // same as other rad because it's a circle
                        0.0,      // angle of ellipse doesn't matter, so 0
                        if a.is_large_arc() { 1 } else { 0 }, // large arc flag
                        if a.is_ccw() { 1 } else { 0 }, // sweep-flag
                        last_point.x,
                        last_point.y,
                    );

                    path = path.elliptical_arc_to(params);

                    curr_point = Some(last_point)
                }
                murrelet_draw::curve_drawer::CurveSegment::Points(a) => {
                    let maybe_first_and_rest = a.points().split_first();
                    match maybe_first_and_rest {
                        Some((f, rest)) => {
                            // first make sure we're at the first point
                            if curr_point != Some(*f) {
                                path = path.line_to((f.x, f.y));
                            }

                            for v in rest {
                                path = path.line_to((v.x, v.y));
                            }

                            curr_point = Some(a.last_point())
                        }
                        None => {}
                    }
                }
            }
        }

        Some(path)
    }
}

// just connect the dots with lines
impl ToSvg for Vec<Vec2> {
    fn to_svg(&self) -> Option<Data> {
        if self.len() == 0 {
            return None;
        }

        let mut curr_item: Vec2 = *self.first().unwrap();

        let mut data = Data::new().move_to((curr_item.x, curr_item.y));

        for loc in self[1..].iter() {
            data = data.line_by((loc.x - curr_item.x, loc.y - curr_item.y));
            curr_item = *loc;
        }
        Some(data)
    }
}
