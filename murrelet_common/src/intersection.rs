use glam::{vec2, Vec2};

use crate::SpotOnCurve;

// todo, can i replace this with geo?
pub fn find_intersection_inf(line0: (Vec2, Vec2), line1: (Vec2, Vec2)) -> Option<Vec2> {
    let (line0_start, line0_end) = line0;
    let (line1_start, line1_end) = line1;

    let x1 = line0_start.x;
    let y1 = line0_start.y;
    let x2 = line0_end.x;
    let y2 = line0_end.y;
    let x3 = line1_start.x;
    let y3 = line1_start.y;
    let x4 = line1_end.x;
    let y4 = line1_end.y;

    // first find if the lines intersect
    let d = (y2 - y1) * (x3 - x4) - (x2 - x1) * (y3 - y4);
    let epsilon = 1e-7;
    if d.abs() < epsilon {
        None // parallel, we're done
    } else {
        let intersection_point_f32: Vec2;

        let self_is_vertical = (x1 - x2).abs() < epsilon;
        let other_is_vertical = (x3 - x4).abs() < epsilon;

        // some help from gemini 2.5 pro
        let pt = if self_is_vertical && other_is_vertical {
            // Both vertical and den != 0. This should not happen if logic is sound,
            // as two distinct vertical lines would have den = 0.
            // This implies they might be collinear and overlapping if den was non-zero due to epsilon,
            // but the den check should have caught true parallelism.
            // For safety, returning None if this unexpected state is reached.
            return None;
        } else if self_is_vertical {
            // Self is vertical, other is not.
            let px = x1;
            // Slope of other segment
            let m_other = (y4 - y3) / (x4 - x3);
            // y = m(x - x_pt) + y_pt. Using (x3,y3) from other segment.
            let py = m_other * (px - x3) + y3;
            Vec2::new(px, py)
        } else if other_is_vertical {
            // Other is vertical, self is not.
            let px = x3;
            // Slope of self segment (safe as it's not vertical)
            let m_self = (y2 - y1) / (x2 - x1);
            // y = m(x - x_pt) + y_pt. Using (x1,y1) from self segment.
            let py = m_self * (px - x1) + y1;
            Vec2::new(px, py)
        } else {
            let t_numerator = (x1 - x3) * (y3 - y4) - (y1 - y3) * (x3 - x4);
            let t = t_numerator / d;
            let px = x1 + t * (x2 - x1);
            let py = y1 + t * (y2 - y1);
            Vec2::new(px, py)
        };
        Some(pt)
    }
}

pub fn find_intersect_spots(spot0: SpotOnCurve, spot1: SpotOnCurve) -> Option<Vec2> {
    find_intersection_inf(
        (spot0.loc(), spot0.to_line(-100.0).to_last_point()),
        (spot1.loc(), spot1.to_line(-100.0).to_last_point()),
    )
}

pub fn find_intersection_segments(line0: (Vec2, Vec2), line1: (Vec2, Vec2)) -> Option<Vec2> {
    find_intersection_inf(line0, line1).filter(|&intersection| {
        within_segment(line0, intersection, 0.0001) && within_segment(line1, intersection, 0.0001)
    })
}

pub fn within_segment(line: (Vec2, Vec2), intersection: Vec2, eps: f32) -> bool {
    let (start, end) = line;

    let left_x = start.x;
    let left_y = start.y;
    let right_x = end.x;
    let right_y = end.y;

    let (max_x, min_x) = if left_x > right_x {
        (left_x, right_x)
    } else {
        (right_x, left_x)
    };

    let (max_y, min_y) = if left_y > right_y {
        (left_y, right_y)
    } else {
        (right_y, left_y)
    };

    let inter_x = intersection.x;
    let inter_y = intersection.y;

    inter_x + eps >= min_x
        && inter_x - eps <= max_x
        && inter_y + eps >= min_y
        && inter_y - eps <= max_y
}
