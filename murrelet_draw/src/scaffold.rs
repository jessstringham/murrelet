use glam::{vec2, Vec2};
use itertools::Itertools;

pub fn line_to_multipolygon(curves: &[Vec2]) -> geo::MultiPolygon {
    geo::MultiPolygon::new(vec![line_to_polygon(curves)])
}

pub fn line_to_polygon(curves: &[Vec2]) -> geo::Polygon {
    geo::Polygon::new(vec2_to_line_string(curves), vec![])
}

fn vec2_to_line_string(vs: &[Vec2]) -> geo::LineString {
    let coords = vs.iter().map(|x| x.to_coord()).collect_vec();
    geo::LineString::new(coords)
}

pub fn multipolygon_to_vec2(p: &geo::MultiPolygon) -> Vec<Vec<Vec2>> {
    p.iter().map(|pp| polygon_to_vec2(pp)).collect_vec()
}

pub fn polygon_to_vec2(p: &geo::Polygon) -> Vec<Vec2> {
    let mut coords = p
        .exterior()
        .coords()
        .into_iter()
        .map(|coord| coord.to_vec2())
        .collect_vec();

    if coords.first() == coords.last() {
        coords.pop();
    }

    coords
}

trait ToCoord {
    fn to_coord(&self) -> ::geo::Coord;
}

impl ToCoord for Vec2 {
    fn to_coord(&self) -> ::geo::Coord {
        ::geo::coord! {x: self.x as f64, y: self.y as f64}
    }
}

trait ToVec2 {
    fn to_vec2(&self) -> Vec2;
}

impl ToVec2 for ::geo::Coord {
    fn to_vec2(&self) -> Vec2 {
        let (x, y) = self.x_y();
        vec2(x as f32, y as f32)
    }
}
