use glam::Vec2;

pub fn glam_to_lyon(v: Vec2) -> lyon::math::Point {
    lyon::math::point(v.x, v.y)
}

pub fn glam_to_kurbo(v: Vec2) -> kurbo::Point {
    kurbo::Point::new(v.x as f64, v.y as f64)
}
