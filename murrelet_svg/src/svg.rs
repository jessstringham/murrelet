#![allow(dead_code)]
use std::{
    cell::RefCell,
    collections::{HashMap, HashSet},
    fmt,
    rc::Rc,
};

use glam::*;
use itertools::Itertools;
use murrelet_common::*;
use murrelet_draw::{
    curve_drawer::CurveDrawer,
    draw::{MurreletColorStyle, MurreletStyle},
    newtypes::RGBandANewtype,
    style::{MurreletCurve, MurreletPath, MurreletPathAnnotation, StyledPath, StyledPathSvgFill},
    svg::{SvgPathDef, SvgShape, TransformedSvgShape},
};
use murrelet_perform::perform::SvgDrawConfig;
use svg::{
    node::element::{path::Data, Group},
    Document, Node,
};

// this area actually follows the spec more closely

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

// this can take all of our favorite types, and returns the Group that applies the shape's transforms
// and styles
pub trait ToStyledGroup {
    fn to_group(&self, style: &MurreletStyle) -> Option<Group>;
}

pub trait ToSvgData {
    fn to_svg_data(&self) -> Option<Data>;

    fn transform(&self) -> Option<Mat4> {
        None
    }

    fn make_path(&self, style: &MurreletStyle) -> Option<svg::node::element::Path> {
        if let Some(d) = self.to_svg_data() {
            let mut d = d;
            if style.closed {
                d = d.close();
            }
            let mut p = svg::node::element::Path::new().set("d", d);
            p = style.add_svg_attributes(p);
            Some(p)
        } else {
            None
        }
    }
}

// if you implement ToSvgData, we can get the group for you
impl<T: ToSvgData> ToStyledGroup for T {
    fn to_group(&self, style: &MurreletStyle) -> Option<Group> {
        if let Some(p) = self.make_path(style) {
            let mut g = Group::new();
            if let Some(t) = self.transform() {
                g = g.set("transform", t.to_svg_matrix());
            }
            g.append(p);
            Some(g)
        } else {
            None
        }
    }
}

impl ToSvgData for MurreletCurve {
    fn to_svg_data(&self) -> Option<Data> {
        // self.curve().to_svg_data()
        todo!()
    }

    fn transform(&self) -> Option<Mat4> {
        Some(self.mat4())
    }
}

impl ToStyledGroup for TransformedSvgShape {
    fn to_group(&self, style: &MurreletStyle) -> Option<Group> {
        let mut g = Group::new();
        g = g.set("transform", self.t.to_svg_matrix());
        match &self.shape {
            SvgShape::Rect(s) => {
                let mut rect = svg::node::element::Rectangle::new()
                    .set("x", s.x)
                    .set("y", s.y)
                    .set("rx", s.rx)
                    .set("ry", s.ry)
                    .set("width", s.width)
                    .set("height", s.height);

                rect = style.add_svg_attributes(rect);

                g.append(rect);

                Some(g)
            }
            SvgShape::Circle(s) => {
                let mut circ = svg::node::element::Circle::new()
                    .set("cx", s.x)
                    .set("cy", s.y)
                    .set("r", s.r);

                circ = style.add_svg_attributes(circ);
                g.append(circ);
                Some(g)
            }
            SvgShape::Path(svg_path) => {
                if let Some(data) = svg_path.to_svg_data() {
                    let mut path = svg::node::element::Path::new().set("d", data);
                    path = style.add_svg_attributes(path);
                    g.append(path);
                }
                Some(g)
            }
        }
    }
}

impl ToStyledGroup for MurreletPath {
    fn to_group(&self, style: &MurreletStyle) -> Option<Group> {
        match self {
            MurreletPath::Polyline(path) => path.into_iter_vec2().collect_vec().to_group(style),
            MurreletPath::Curve(c) => c.to_group(style),
            MurreletPath::Svg(c) => c.to_group(style),
            MurreletPath::MaskedCurve(_, _) => todo!(), //curve.to_group_with_mask(style, mask),
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

// for a type that means something, e.g. "style fro a shape"
trait AddSvgAttributes {
    fn add_svg_attributes<T: Node>(&self, p: T) -> T;
}

// for component types, e.g. "color"
trait GetSvgAttributes {
    fn get_svg_attributes(&self) -> String;
}

impl GetSvgAttributes for StyledPathSvgFill {
    fn get_svg_attributes(&self) -> String {
        format!("url(#P{})", self.hash())
    }
}

impl GetSvgAttributes for MurreletColor {
    fn get_svg_attributes(&self) -> String {
        let [r, g, b, a] = self.into_rgba_components();
        let fill = format!(
            "rgba({}, {}, {}, {})",
            (r * 255.0) as i32,
            (g * 255.0) as i32,
            (b * 255.0) as i32,
            a
        );
        fill
    }
}

impl GetSvgAttributes for RGBandANewtype {
    fn get_svg_attributes(&self) -> String {
        self.color().get_svg_attributes()
    }
}

impl AddSvgAttributes for MurreletColorStyle {
    fn add_svg_attributes<T: Node>(&self, p: T) -> T {
        let mut p = p;
        p.assign("fill-rule", "evenodd");
        match self {
            MurreletColorStyle::Color(c) => p.assign("fill", c.get_svg_attributes()),
            MurreletColorStyle::RgbaFill(c) => p.assign("fill", c.get_svg_attributes()),
            MurreletColorStyle::SvgFill(c) => p.assign("fill", c.get_svg_attributes()),
        }
        p
    }
}

impl GetSvgAttributes for MurreletColorStyle {
    fn get_svg_attributes(&self) -> String {
        match self {
            MurreletColorStyle::Color(c) => c.get_svg_attributes(),
            MurreletColorStyle::RgbaFill(c) => c.get_svg_attributes(),
            MurreletColorStyle::SvgFill(c) => c.get_svg_attributes(),
        }
    }
}

impl AddSvgAttributes for MurreletPath {
    fn add_svg_attributes<T: Node>(&self, p: T) -> T {
        let mut p = p;
        let svg_str = self.get_svg_attributes();
        if !svg_str.is_empty() {
            p.assign("transform", svg_str);
        }
        p
    }
}
impl GetSvgAttributes for MurreletPath {
    fn get_svg_attributes(&self) -> String {
        if let Some(t) = self.transform() {
            t.to_svg_matrix()
        } else {
            "".to_string()
        }
    }
}

impl AddSvgAttributes for MurreletStyle {
    fn add_svg_attributes<T: Node>(&self, p: T) -> T {
        let mut p = p;

        p.assign("fill-rule", "evenodd");

        match self.drawing_plan() {
            murrelet_draw::draw::MurreletDrawPlan::Shader(fill) => {
                p.assign("fill", fill.get_svg_attributes())
            }
            murrelet_draw::draw::MurreletDrawPlan::DebugPoints(_) => unimplemented!(),
            murrelet_draw::draw::MurreletDrawPlan::FilledClosed => {
                p.assign("fill", self.color.get_svg_attributes());

                if self.stroke_weight > 0.0 {
                    p.assign("stroke-width", self.stroke_weight);
                    p.assign("stroke-linejoin", "round");
                    p.assign("stroke-linecap", "round");
                    p.assign("stroke", self.stroke_color.get_svg_attributes());
                }
            }
            murrelet_draw::draw::MurreletDrawPlan::Outline => {
                p.assign("fill", "none");

                if self.stroke_weight > 0.0 {
                    p.assign("stroke-width", self.stroke_weight);
                    p.assign("stroke-linejoin", "round");
                    p.assign("stroke-linecap", "round");
                    p.assign("stroke", self.color.get_svg_attributes());
                }
            }
            murrelet_draw::draw::MurreletDrawPlan::Line => {
                p.assign("fill", "none");

                if self.stroke_weight > 0.0 {
                    p.assign("stroke-width", self.stroke_weight);
                    p.assign("stroke-linejoin", "round");
                    p.assign("stroke-linecap", "round");
                    p.assign("stroke", self.color.get_svg_attributes());
                }
            }
        }

        p
    }
}

impl AddSvgAttributes for StyledPath {
    fn add_svg_attributes<T: Node>(&self, p: T) -> T {
        let mut p = p;
        p = self.annotations.add_svg_attributes(p);
        p = self.path.add_svg_attributes(p);
        p = self.style.add_svg_attributes(p);
        p
    }
}

impl AddSvgAttributes for MurreletPathAnnotation {
    fn add_svg_attributes<T: Node>(&self, p: T) -> T {
        let mut p = p;
        for (k, v) in self.vals() {
            p.assign(k.to_string(), v.to_string());
        }
        p
    }
}

pub trait ToSvgPath {
    fn make_group(&self) -> Option<svg::node::element::Group>;
    fn make_pattern(&self) -> Option<(String, svg::node::element::Pattern)>;
}

impl ToSvgPath for StyledPath {
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

    fn make_group(&self) -> Option<svg::node::element::Group> {
        self.path
            .to_group(&self.style)
            .map(|x| self.annotations.add_svg_attributes(x))
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
            .set("font-family", "monospace".to_string())
            .set("font-size", format!("{}px", text_size))
            .set("fill", text.style.color.get_svg_attributes())
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
            if let Some(d) = path.make_group() {
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

            // TODO REMOVE THISS
            defs.push(name.clone());
        }

        (doc, defs.into_iter().join("\n"))
    }

    // this one's meant for svgs for pen plotters, so it drops fill styles
    fn make_doc(&self, paths: &SvgPathCache) -> Document {
        let target_size = self.svg_draw_config.full_target_width(); // guides are at 10x 10x, gives 1cm margin

        let (view_box_x, view_box_y) = if let Some(r) = self.svg_draw_config.resolution {
            let [width, height] = r.as_dims();
            (width * 2, height * 2)
        } else {
            (800, 800)
        };
        let mut doc = Document::new()
            .set(
                "xmlns:inkscape",
                "http://www.inkscape.org/namespaces/inkscape",
            )
            .set("viewBox", (0, 0, view_box_x, view_box_y))
            .set("width", format!("{:?}mm", target_size))
            .set("height", format!("{:?}mm", target_size));

        if let Some(bg_color) = self.svg_draw_config.bg_color() {
            let bg_rect = svg::node::element::Rectangle::new()
                .set("x", 0)
                .set("y", 0)
                .set("width", view_box_x)
                .set("height", view_box_y)
                .set("fill", bg_color.to_svg_rgb());
            doc = doc.add(bg_rect);
        }

        // todo, maybe figure out defs?
        let (group, _) = self.make_html(paths);

        let mut centering_group = svg::node::element::Group::new();
        centering_group = centering_group.set(
            "transform",
            format!("translate({}px, {}px)", view_box_x / 2, view_box_y / 2),
        );
        centering_group = centering_group.add(group);

        doc = doc.add(centering_group);

        doc

        // for (name, layer) in paths.layers.iter() {
        //     let (g, _) = self.make_layer(name, layer, true);
        //     doc.append(g);
        // }

        // doc
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

#[derive(Clone)]
pub struct SvgPathCacheRef(Rc<RefCell<SvgPathCache>>);

impl SvgPathCacheRef {
    pub fn add_guides(&self) {
        self.0.borrow_mut().add_guides();
    }

    pub fn save_doc(&self) {
        self.0.borrow().save_doc();
    }

    pub fn clear(&self, layer: &str) {
        self.0.borrow_mut().clear(layer);
    }

    pub fn add_styled_path(&self, layer: &str, styled_path: StyledPath) {
        self.0.borrow_mut().add_styled_path(layer, styled_path)
    }

    pub fn add_styled_text(&self, layer: &str, text: StyledText) {
        self.0.borrow_mut().add_styled_text(layer, text)
    }

    pub fn make_html(&self) -> Vec<String> {
        self.0.borrow().make_html()
    }
}

impl fmt::Debug for SvgPathCacheRef {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "SvgPathCache")
    }
}

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
        SvgPathCacheRef(Rc::new(RefCell::new(SvgPathCache::new(svg_draw_config))))
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
                annotations: MurreletPathAnnotation::noop(),
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

// pub trait ToSvg {
//     fn to_svg(&self) -> Option<Data>;

// }

// why am i doing this again, it is a good question
impl ToSvgData for SvgPathDef {
    fn to_svg_data(&self) -> Option<Data> {
        let mut path = Data::new();

        path = path.move_to(self.svg_move_to());

        for v in self.cmds() {
            match v {
                murrelet_draw::svg::SvgCmd::Line(svg_to) => path = path.line_to(svg_to.params()),
                murrelet_draw::svg::SvgCmd::CubicBezier(svg_cubic_bezier) => {
                    path = path.cubic_curve_to(svg_cubic_bezier.params())
                }
            }
        }

        Some(path)
    }
}

impl ToSvgData for CurveDrawer {
    fn to_svg_data(&self) -> Option<Data> {
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
impl ToSvgData for Vec<Vec2> {
    fn to_svg_data(&self) -> Option<Data> {
        if self.len() == 0 {
            return None;
        }

        // todo, hmmm, see if we can consolidate this.
        let cd = CurveDrawer::new_simple_points(self.clone(), false);
        cd.to_svg_data()

        // // whee, flip y's so we stop drawing everything upside down

        // let mut curr_item: Vec2 = *self.first().unwrap();

        // let mut data = Data::new().move_to((curr_item.x, -curr_item.y));

        // for loc in self[1..].iter() {
        //     data = data.line_by((loc.x - curr_item.x, -(loc.y - curr_item.y)));
        //     curr_item = *loc;
        // }
        // Some(data)
    }
}
