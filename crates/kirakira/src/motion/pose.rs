use gpui_kit::base::animation::Lerp;
use gpui_kit::{Pixels, Point, Size, point, px};

/// What a CSS keyframe animates with `transform`, `translate`, `scale`, `rotate` and `opacity`.
///
/// Translation comes in two parts that add up: `x`/`y` in pixels and `xp`/`yp` as a fraction of
/// the element's own size, like `translateY(100%)`. Rotation is in degrees.
///
/// GPUI paints quads and text without a transform, so how a pose lands depends on the target:
/// [`Transform`](super::Transform) applies translation, opacity and a uniform scale to any element;
/// painted shapes (SVG and paths) take all of it, rotation included.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Pose {
    pub x: f32,
    pub y: f32,
    pub xp: f32,
    pub yp: f32,
    pub sx: f32,
    pub sy: f32,
    pub rotate: f32,
    pub opacity: f32,
}

impl Default for Pose {
    fn default() -> Self {
        Self::IDENTITY
    }
}

impl Pose {
    /// No transform, fully opaque: an element's natural state.
    pub const IDENTITY: Self = Self {
        x: 0.0,
        y: 0.0,
        xp: 0.0,
        yp: 0.0,
        sx: 1.0,
        sy: 1.0,
        rotate: 0.0,
        opacity: 1.0,
    };

    pub const fn new() -> Self {
        Self::IDENTITY
    }

    /// Translate by `x` and `y` pixels.
    pub const fn at(mut self, x: f32, y: f32) -> Self {
        self.x = x;
        self.y = y;
        self
    }

    pub const fn x(mut self, x: f32) -> Self {
        self.x = x;
        self
    }

    pub const fn y(mut self, y: f32) -> Self {
        self.y = y;
        self
    }

    /// Translate by a fraction of the element's own width, like `translateX(50%)` as `0.5`.
    pub const fn xp(mut self, xp: f32) -> Self {
        self.xp = xp;
        self
    }

    /// Translate by a fraction of the element's own height, like `translateY(100%)` as `1.0`.
    pub const fn yp(mut self, yp: f32) -> Self {
        self.yp = yp;
        self
    }

    pub const fn scale(mut self, scale: f32) -> Self {
        self.sx = scale;
        self.sy = scale;
        self
    }

    pub const fn scale_xy(mut self, sx: f32, sy: f32) -> Self {
        self.sx = sx;
        self.sy = sy;
        self
    }

    pub const fn rotate(mut self, degrees: f32) -> Self {
        self.rotate = degrees;
        self
    }

    pub const fn opacity(mut self, opacity: f32) -> Self {
        self.opacity = opacity;
        self
    }

    /// The translation in pixels for an element of `size`.
    pub fn offset(&self, size: Size<Pixels>) -> Point<Pixels> {
        point(
            px(self.x) + size.width * self.xp,
            px(self.y) + size.height * self.yp,
        )
    }

    /// One scale for targets that can only scale evenly: the geometric mean of `sx` and `sy`, so a
    /// squash that keeps its area keeps its size.
    pub fn uniform_scale(&self) -> f32 {
        (self.sx.max(0.0) * self.sy.max(0.0)).sqrt()
    }

    /// Opacity clamped to `0..=1`; keyframe easing may overshoot it.
    pub fn alpha(&self) -> f32 {
        self.opacity.clamp(0.0, 1.0)
    }

    pub fn is_identity(&self) -> bool {
        *self == Self::IDENTITY
    }
}

impl Lerp for Pose {
    fn lerp(&self, to: &Self, t: f32) -> Self {
        let mix = |a: f32, b: f32| a + (b - a) * t;
        Self {
            x: mix(self.x, to.x),
            y: mix(self.y, to.y),
            xp: mix(self.xp, to.xp),
            yp: mix(self.yp, to.yp),
            sx: mix(self.sx, to.sx),
            sy: mix(self.sy, to.sy),
            rotate: mix(self.rotate, to.rotate),
            opacity: mix(self.opacity, to.opacity),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lerp_mixes_every_field() {
        let from = Pose::new().at(0.0, 10.0).scale(0.0).opacity(0.0);
        let to = Pose::new().at(10.0, 0.0).rotate(90.0);
        let mid = from.lerp(&to, 0.5);
        assert_eq!(mid.x, 5.0);
        assert_eq!(mid.y, 5.0);
        assert_eq!(mid.sx, 0.5);
        assert_eq!(mid.rotate, 45.0);
        assert_eq!(mid.opacity, 0.5);
    }

    #[test]
    fn uniform_scale_keeps_area() {
        let squash = Pose::new().scale_xy(2.0, 0.5);
        assert!((squash.uniform_scale() - 1.0).abs() < 1e-6);
    }
}
