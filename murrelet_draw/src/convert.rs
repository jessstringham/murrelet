use geo::Coord;
use glam::Vec2;

pub fn glam_to_lyon(v: Vec2) -> lyon::math::Point {
    lyon::math::point(v.x, v.y)
}

pub fn glam_to_kurbo(v: Vec2) -> kurbo::Point {
    kurbo::Point::new(v.x as f64, v.y as f64)
}

pub fn glam_to_geo(v: Vec2) -> Coord<f64> {
    Coord::<f64> {
        x: v.x as f64,
        y: v.y as f64,
    }
}


pub fn glam_to_geo_pt(v: Vec2) -> geo::Point {
    geo::Point::from(glam_to_geo(v))
}