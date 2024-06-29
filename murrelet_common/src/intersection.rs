use glam::{vec2, Vec2};

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
    let d: f32 = (x1 - x2) * (y3 - y4) - (y1 - y2) * (x3 - x4);

    if d == 0.0 {
        None // the lines are parallel, we're done
    } else {
        let t_num = (x1 - x3) * (y3 - y4) - (y1 - y3) * (x3 - x4);
        let t = t_num / d;

        let px = x1 + t * (x2 - x1);
        let py = y1 + t * (y2 - y1);

        let intersection = vec2(px, py);

        // okay we have an intersection! now just make sure it's in each segment
        Some(intersection)
    }
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
