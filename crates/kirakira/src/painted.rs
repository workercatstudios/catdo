//! Painted shapes: what CSS draws with `transform` on a box, as paths for a `canvas`.
//!
//! GPUI paints quads without a transform, so a box that rotates or squashes unevenly (a swinging
//! badge, a tipping tile, a squashed dot) is painted here instead: a rounded rectangle or an
//! ellipse in the box's own coordinates, mapped through an [`Affine`] the way CSS maps a box
//! through `translate`, `rotate` and `scale` about its `transform-origin`. Rounded corners squash
//! with the box, exactly as they do on the web.

use gpui_kit::{Background, Path, PathBuilder, Pixels, Window, point, px};

/// A 2D affine map, `(x, y) → (a·x + c·y + e, b·x + d·y + f)`, in pixels.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Affine {
    a: f32,
    b: f32,
    c: f32,
    d: f32,
    e: f32,
    f: f32,
}

impl Affine {
    pub const IDENTITY: Self = Self {
        a: 1.0,
        b: 0.0,
        c: 0.0,
        d: 1.0,
        e: 0.0,
        f: 0.0,
    };

    pub fn translate(x: f32, y: f32) -> Self {
        Self {
            e: x,
            f: y,
            ..Self::IDENTITY
        }
    }

    /// CSS `rotate(degrees)` about `(x, y)`: clockwise on screen, where y points down.
    pub fn rotate_about(x: f32, y: f32, degrees: f32) -> Self {
        let (sin, cos) = degrees.to_radians().sin_cos();
        let turn = Self {
            a: cos,
            b: sin,
            c: -sin,
            d: cos,
            e: 0.0,
            f: 0.0,
        };
        Self::translate(-x, -y)
            .then(turn)
            .then(Self::translate(x, y))
    }

    /// CSS `scale(sx, sy)` about `(x, y)`.
    pub fn scale_about(x: f32, y: f32, sx: f32, sy: f32) -> Self {
        let scale = Self {
            a: sx,
            d: sy,
            ..Self::IDENTITY
        };
        Self::translate(-x, -y)
            .then(scale)
            .then(Self::translate(x, y))
    }

    /// This map followed by `next`.
    pub fn then(self, next: Self) -> Self {
        Self {
            a: next.a * self.a + next.c * self.b,
            b: next.b * self.a + next.d * self.b,
            c: next.a * self.c + next.c * self.d,
            d: next.b * self.c + next.d * self.d,
            e: next.a * self.e + next.c * self.f + next.e,
            f: next.b * self.e + next.d * self.f + next.f,
        }
    }

    pub fn apply(&self, x: f32, y: f32) -> (f32, f32) {
        (
            self.a * x + self.c * y + self.e,
            self.b * x + self.d * y + self.f,
        )
    }
}

/// How far a cubic Bézier's control points sit along the tangents to draw a quarter circle.
const KAPPA: f32 = 0.552_284_8;

/// A rectangle at `(x, y)`, `w × h`, with corners of radius `r` (clamped to fit), through `map`.
pub(crate) fn rounded_rect(x: f32, y: f32, w: f32, h: f32, r: f32, map: Affine) -> PathBuilder {
    let r = r.clamp(0.0, w.min(h) / 2.0);
    let k = r * (1.0 - KAPPA);
    let to = |x: f32, y: f32| {
        let (x, y) = map.apply(x, y);
        point(px(x), px(y))
    };
    let (right, bottom) = (x + w, y + h);
    let mut path = PathBuilder::fill();
    path.move_to(to(x + r, y));
    path.line_to(to(right - r, y));
    path.cubic_bezier_to(to(right, y + r), to(right - k, y), to(right, y + k));
    path.line_to(to(right, bottom - r));
    path.cubic_bezier_to(
        to(right - r, bottom),
        to(right, bottom - k),
        to(right - k, bottom),
    );
    path.line_to(to(x + r, bottom));
    path.cubic_bezier_to(to(x, bottom - r), to(x + k, bottom), to(x, bottom - k));
    path.line_to(to(x, y + r));
    path.cubic_bezier_to(to(x + r, y), to(x, y + k), to(x + k, y));
    path.close();
    path
}

/// An ellipse centred on `(cx, cy)` with radii `rx` and `ry`, through `map`.
pub(crate) fn ellipse(cx: f32, cy: f32, rx: f32, ry: f32, map: Affine) -> PathBuilder {
    let map = Affine::scale_about(0.0, 0.0, rx, ry)
        .then(Affine::translate(cx, cy))
        .then(map);
    rounded_rect(-1.0, -1.0, 2.0, 2.0, 1.0, map)
}

/// A ring: the band between radius `r - width` and `r` around `(cx, cy)`, like a circle with a
/// `border` of `width`.
pub(crate) fn ring(cx: f32, cy: f32, r: f32, width: f32) -> PathBuilder {
    let width = width.clamp(0.0, r);
    let mut path = PathBuilder::stroke(px(width));
    let mid = r - width / 2.0;
    let k = mid * KAPPA;
    let at = |x: f32, y: f32| point(px(cx + x), px(cy + y));
    path.move_to(at(mid, 0.0));
    path.cubic_bezier_to(at(0.0, mid), at(mid, k), at(k, mid));
    path.cubic_bezier_to(at(-mid, 0.0), at(-k, mid), at(-mid, k));
    path.cubic_bezier_to(at(0.0, -mid), at(-mid, -k), at(-k, -mid));
    path.cubic_bezier_to(at(mid, 0.0), at(k, -mid), at(mid, -k));
    path.close();
    path
}

/// Paints `path`, skipping shapes too small to tessellate.
pub(crate) fn paint(window: &mut Window, path: PathBuilder, color: impl Into<Background>) {
    if let Ok(path) = path.build() {
        let path: Path<Pixels> = path;
        window.paint_path(path, color);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: (f32, f32), b: (f32, f32)) -> bool {
        (a.0 - b.0).abs() < 1e-4 && (a.1 - b.1).abs() < 1e-4
    }

    #[test]
    fn rotation_is_clockwise_on_screen() {
        // A point below the pivot swings to the left under a positive CSS rotation.
        let turn = Affine::rotate_about(0.0, 0.0, 90.0);
        assert!(close(turn.apply(0.0, 1.0), (-1.0, 0.0)));
        // The pivot stays put.
        let turn = Affine::rotate_about(5.0, 5.0, 37.0);
        assert!(close(turn.apply(5.0, 5.0), (5.0, 5.0)));
    }

    #[test]
    fn scale_keeps_its_origin() {
        let squash = Affine::scale_about(10.0, 20.0, 1.3, 0.7);
        assert!(close(squash.apply(10.0, 20.0), (10.0, 20.0)));
        assert!(close(squash.apply(20.0, 20.0), (23.0, 20.0)));
        assert!(close(squash.apply(10.0, 10.0), (10.0, 13.0)));
    }

    #[test]
    fn then_applies_in_order() {
        let map = Affine::translate(1.0, 0.0).then(Affine::scale_about(0.0, 0.0, 2.0, 2.0));
        assert!(close(map.apply(0.0, 0.0), (2.0, 0.0)));
    }
}
