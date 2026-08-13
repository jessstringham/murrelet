// some chatgpt
#[derive(Clone, Copy, Debug)]
pub enum Ease {
    Linear,
    EaseIn,
    EaseOut,
    EaseInOut,
    Custom(fn(f32) -> f32),
}

impl Ease {
    #[inline]
    pub fn apply(self, t: f32) -> f32 {
        let t = t.clamp(0.0, 1.0);
        match self {
            Ease::Linear => t,
            Ease::EaseIn => t * t,
            Ease::EaseOut => 1.0 - (1.0 - t) * (1.0 - t),
            Ease::EaseInOut => {
                if t < 0.5 {
                    2.0 * t * t
                } else {
                    1.0 - 2.0 * (1.0 - t) * (1.0 - t)
                }
            }
            Ease::Custom(f) => f(t),
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Segment {
    pub from: f32,
    pub to: f32,
    pub value: f32,
    pub ease: Ease,
}

#[derive(Clone, Debug)]
pub struct Curve {
    pub start: f32,
    pub segments: Vec<Segment>,
}
impl Curve {
    /// Sample the curve at position `x`.
    ///
    /// Semantics:
    /// - `start` is the output before the first segment.
    /// - each segment transitions from the previous output to `segment.value`
    ///   over [from, to] using the segment's easing.
    /// - after the last segment, it holds the last value.
    pub fn sample(&self, x: f32) -> f32 {
        if self.segments.is_empty() {
            return self.start;
        }

        let mut prev_y = self.start;

        for s in &self.segments {
            if x <= s.from {
                return prev_y;
            }
            if x < s.to {
                let denom = s.to - s.from;
                if denom.abs() < 1e-6 {
                    return s.value;
                }
                let t = (x - s.from) / denom;
                let t = s.ease.apply(t);
                return prev_y + (s.value - prev_y) * t;
            }
            prev_y = s.value;
        }

        prev_y
    }
}

#[macro_export]
macro_rules! ease {
    (linear) => {
        $crate::timecurve::Ease::Linear
    };
    (ease_in) => {
        $crate::timecurve::Ease::EaseIn
    };
    (ease_out) => {
        $crate::timecurve::Ease::EaseOut
    };
    (ease_in_out) => {
        $crate::timecurve::Ease::EaseInOut
    };
    ($f:expr) => {
        $crate::timecurve::Ease::Custom($f)
    };
}

#[macro_export]
macro_rules! __curve_ease_or_linear {
    () => {
        $crate::timecurve::Ease::Linear
    };
    ($ease:tt) => {
        $crate::ease!($ease)
    };
}

#[macro_export]
macro_rules! make_time_curve {
    (
        start: $start:tt;
        $(
            from $from:tt to $to:tt : $value:tt $( @ $ease:tt )? ;
        )*
    ) => {{
        let mut segments = Vec::new();
        $(
            segments.push($crate::timecurve::Segment{
                from: ($from) as f32,
                to: ($to) as f32,
                value: ($value) as f32,
                ease: $crate::__curve_ease_or_linear!($( $ease )?),
            });
        )*
        $crate::timecurve::Curve { start: ($start) as f32, segments }
    }};
}
