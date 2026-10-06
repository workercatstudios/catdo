//! Painted shapes: what the web components draw with transforms, `clip-path`, masks and SVG.
//!
//! GPUI paints quads and glyphs without a transform, and has no clip-path, mask or stroke cap
//! options. Decorative shapes are painted instead, inside a `canvas`:
//!
//! - [`Affine`] is a 2D transform, composed like nested SVG groups and CSS `transform-origin`.
//! - [`Shape`] collects filled outlines in window pixels and paints them as one GPUI path. Overlaps
//!   join (the non-zero rule), so a fill and its stroke painted at half opacity blend once, like an
//!   SVG group with `opacity`.
//! - SVG path data ([`Shape::path`], [`Shape::stroke_path`]) is flattened after transforming, so
//!   curves stay smooth at any scale. Strokes have round caps and joins, the only kind Kirakira
//!   draws.

use std::f32::consts::{PI, TAU};

use gpui_kit::{
    AnyElement, App, Background, Bounds, ContentMask, Element, ElementId, FillOptions, FillRule,
    GlobalElementId, InspectorElementId, IntoElement, LayoutId, PathBuilder, PathStyle, Pixels,
    Point, Window, point, px,
};

/// A 2D affine transform: `x' = a·x + c·y + e`, `y' = b·x + d·y + f`, in y-down coordinates like
/// CSS and SVG.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Affine {
    pub a: f32,
    pub b: f32,
    pub c: f32,
    pub d: f32,
    pub e: f32,
    pub f: f32,
}

impl Default for Affine {
    fn default() -> Self {
        Self::IDENTITY
    }
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

    pub const fn translate(x: f32, y: f32) -> Self {
        Self {
            e: x,
            f: y,
            ..Self::IDENTITY
        }
    }

    pub const fn scale(sx: f32, sy: f32) -> Self {
        Self {
            a: sx,
            d: sy,
            ..Self::IDENTITY
        }
    }

    /// A clockwise rotation in degrees, like CSS `rotate()` on screen.
    pub fn rotate(degrees: f32) -> Self {
        let (sin, cos) = degrees.to_radians().sin_cos();
        Self {
            a: cos,
            b: sin,
            c: -sin,
            d: cos,
            ..Self::IDENTITY
        }
    }

    /// This transform, then `next`: an inner SVG group inside an outer one is
    /// `inner.then(outer)`.
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

    /// This transform applied around `(x, y)`, like CSS `transform-origin`.
    pub fn around(self, x: f32, y: f32) -> Self {
        Self::translate(-x, -y)
            .then(self)
            .then(Self::translate(x, y))
    }

    pub fn apply(&self, x: f32, y: f32) -> (f32, f32) {
        (
            self.a * x + self.c * y + self.e,
            self.b * x + self.d * y + self.f,
        )
    }

    /// How much the transform scales lengths on average: the square root of its area scale. Stroke
    /// widths are multiplied by it.
    pub fn scale_factor(&self) -> f32 {
        (self.a * self.d - self.b * self.c).abs().sqrt()
    }
}

/// The 4-arc cubic approximation of a circle: control points sit this far along the tangent.
const KAPPA: f32 = 0.552_284_8;

/// Filled outlines in window pixels, painted together as one path.
#[derive(Clone, Debug, Default)]
pub struct Shape {
    contours: Vec<Vec<(f32, f32)>>,
}

/// Twice the signed area of a polygon; positive when it turns clockwise on screen.
fn signed_area(points: &[(f32, f32)]) -> f32 {
    let mut area = 0.0;
    for (i, &(x0, y0)) in points.iter().enumerate() {
        let (x1, y1) = points[(i + 1) % points.len()];
        area += x0 * y1 - x1 * y0;
    }
    area
}

/// Segments for a curve or circle of `length` pixels: one every couple of pixels, within limits.
fn segments(length: f32, min: usize, max: usize) -> usize {
    ((length / 2.5).ceil() as usize).clamp(min, max)
}

fn distance(a: (f32, f32), b: (f32, f32)) -> f32 {
    (b.0 - a.0).hypot(b.1 - a.1)
}

impl Shape {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn is_empty(&self) -> bool {
        self.contours.is_empty()
    }

    fn push(&mut self, points: Vec<(f32, f32)>, hole: bool) {
        if points.len() < 3 {
            return;
        }
        let area = signed_area(&points);
        if area.abs() < 1e-6 {
            return;
        }
        let mut points = points;
        // Every fill turns the same way and every hole the other way, so overlapping fills join
        // and holes cut, under the non-zero rule.
        if (area < 0.0) != hole {
            points.reverse();
        }
        self.contours.push(points);
    }

    /// A filled polygon.
    pub fn polygon(&mut self, points: impl IntoIterator<Item = (f32, f32)>) -> &mut Self {
        self.push(points.into_iter().collect(), false);
        self
    }

    /// A hole cut out of the fills it overlaps.
    pub fn hole(&mut self, points: impl IntoIterator<Item = (f32, f32)>) -> &mut Self {
        self.push(points.into_iter().collect(), true);
        self
    }

    /// The points of a circle, for [`Shape::polygon`] or [`Shape::hole`].
    pub fn circle_points(cx: f32, cy: f32, r: f32) -> Vec<(f32, f32)> {
        let n = segments(TAU * r, 12, 160);
        (0..n)
            .map(|i| {
                let a = i as f32 / n as f32 * TAU;
                (cx + r * a.cos(), cy + r * a.sin())
            })
            .collect()
    }

    pub fn circle(&mut self, cx: f32, cy: f32, r: f32) -> &mut Self {
        if r > 0.0 {
            self.polygon(Self::circle_points(cx, cy, r));
        }
        self
    }

    /// An ellipse in local coordinates, transformed.
    pub fn ellipse(&mut self, cx: f32, cy: f32, rx: f32, ry: f32, transform: &Affine) -> &mut Self {
        self.polygon(Self::ellipse_points(cx, cy, rx, ry, transform));
        self
    }

    /// The outline of an ellipse in local coordinates, transformed, for strokes and holes.
    pub fn ellipse_points(
        cx: f32,
        cy: f32,
        rx: f32,
        ry: f32,
        transform: &Affine,
    ) -> Vec<(f32, f32)> {
        let mut points = Vec::new();
        let (kx, ky) = (rx * KAPPA, ry * KAPPA);
        let start = (cx + rx, cy);
        let arcs = [
            ((cx + rx, cy + ky), (cx + kx, cy + ry), (cx, cy + ry)),
            ((cx - kx, cy + ry), (cx - rx, cy + ky), (cx - rx, cy)),
            ((cx - rx, cy - ky), (cx - kx, cy - ry), (cx, cy - ry)),
            ((cx + kx, cy - ry), (cx + rx, cy - ky), (cx + rx, cy)),
        ];
        let mut from = transform.apply(start.0, start.1);
        points.push(from);
        for (c1, c2, to) in arcs {
            let (c1, c2, to) = (
                transform.apply(c1.0, c1.1),
                transform.apply(c2.0, c2.1),
                transform.apply(to.0, to.1),
            );
            cubic(&mut points, from, c1, c2, to);
            from = to;
        }
        points.pop();
        points
    }

    /// A rounded line through `points`, `width` wide, with round caps and joins: every segment as
    /// a bar and every vertex as a disc, joined by the non-zero rule.
    pub fn stroke_polyline(
        &mut self,
        points: &[(f32, f32)],
        closed: bool,
        width: f32,
    ) -> &mut Self {
        let r = width / 2.0;
        if r <= 0.0 || points.is_empty() {
            return self;
        }
        let count = if closed {
            points.len()
        } else {
            points.len() - 1
        };
        for i in 0..count {
            let p = points[i];
            let q = points[(i + 1) % points.len()];
            let length = distance(p, q);
            if length < 1e-4 {
                continue;
            }
            let (nx, ny) = (-(q.1 - p.1) / length * r, (q.0 - p.0) / length * r);
            self.polygon([
                (p.0 + nx, p.1 + ny),
                (q.0 + nx, q.1 + ny),
                (q.0 - nx, q.1 - ny),
                (p.0 - nx, p.1 - ny),
            ]);
        }
        let mut last: Option<(f32, f32)> = None;
        for &p in points {
            // Skip discs that would land on the previous one; flattened curves are dense.
            if last.is_some_and(|last| distance(last, p) < r * 0.5) {
                continue;
            }
            self.circle(p.0, p.1, r);
            last = Some(p);
        }
        if let (false, Some(&end)) = (closed, points.last()) {
            self.circle(end.0, end.1, r);
        }
        self
    }

    /// Fills SVG path data, transformed. Supports `M L H V C Q Z` in both cases.
    pub fn path(&mut self, d: &str, transform: &Affine) -> &mut Self {
        for (points, _) in flatten(d, transform) {
            self.polygon(points);
        }
        self
    }

    /// Strokes SVG path data, transformed, with round caps and joins. `width` is in the path's own
    /// units and scales with the transform.
    pub fn stroke_path(&mut self, d: &str, transform: &Affine, width: f32) -> &mut Self {
        let width = width * transform.scale_factor();
        for (points, closed) in flatten(d, transform) {
            self.stroke_polyline(&points, closed, width);
        }
        self
    }

    /// A ring segment from `start` sweeping `sweep` degrees clockwise (0° points right), between
    /// radii `r - width / 2` and `r + width / 2`, with round ends when `round` is set: an SVG
    /// circle stroked with a dash and `stroke-linecap: round`.
    pub fn arc_stroke(
        &mut self,
        center: (f32, f32),
        r: f32,
        start: f32,
        sweep: f32,
        width: f32,
        round: bool,
    ) -> &mut Self {
        let half = width / 2.0;
        if sweep <= 0.0 || half <= 0.0 {
            return self;
        }
        let (cx, cy) = center;
        if sweep >= 360.0 {
            self.polygon(Self::circle_points(cx, cy, r + half));
            if r - half > 0.0 {
                self.hole(Self::circle_points(cx, cy, r - half));
            }
            return self;
        }
        let n = segments(sweep.to_radians() * (r + half), 2, 256);
        let at = |i: usize, radius: f32| {
            let a = (start + sweep * i as f32 / n as f32).to_radians();
            (cx + radius * a.cos(), cy + radius * a.sin())
        };
        let mut points: Vec<_> = (0..=n).map(|i| at(i, r + half)).collect();
        points.extend((0..=n).rev().map(|i| at(i, (r - half).max(0.0))));
        self.polygon(points);
        if round {
            let (x0, y0) = at(0, r);
            let (x1, y1) = at(n, r);
            self.circle(x0, y0, half).circle(x1, y1, half);
        }
        self
    }

    /// Builds the GPUI path, or `None` when there is nothing to paint.
    pub fn build(&self) -> Option<gpui_kit::Path<Pixels>> {
        if self.contours.is_empty() {
            return None;
        }
        let mut builder = PathBuilder::fill().with_style(PathStyle::Fill(
            FillOptions::default().with_fill_rule(FillRule::NonZero),
        ));
        for contour in &self.contours {
            builder.move_to(point(px(contour[0].0), px(contour[0].1)));
            for &(x, y) in &contour[1..] {
                builder.line_to(point(px(x), px(y)));
            }
            builder.close();
        }
        builder.build().ok()
    }

    /// Paints the shape. Call it from a `canvas` paint closure.
    pub fn paint(&self, window: &mut Window, color: impl Into<Background>) {
        if let Some(path) = self.build() {
            window.paint_path(path, color);
        }
    }
}

/// Flattens a cubic Bézier from `from` (already in `points`) into `points`.
fn cubic(
    points: &mut Vec<(f32, f32)>,
    from: (f32, f32),
    c1: (f32, f32),
    c2: (f32, f32),
    to: (f32, f32),
) {
    let length = distance(from, c1) + distance(c1, c2) + distance(c2, to);
    let n = segments(length, 1, 96);
    for i in 1..=n {
        let t = i as f32 / n as f32;
        let u = 1.0 - t;
        let (a, b, c, d) = (u * u * u, 3.0 * u * u * t, 3.0 * u * t * t, t * t * t);
        points.push((
            a * from.0 + b * c1.0 + c * c2.0 + d * to.0,
            a * from.1 + b * c1.1 + c * c2.1 + d * to.1,
        ));
    }
}

fn quadratic(points: &mut Vec<(f32, f32)>, from: (f32, f32), c: (f32, f32), to: (f32, f32)) {
    let length = distance(from, c) + distance(c, to);
    let n = segments(length, 1, 96);
    for i in 1..=n {
        let t = i as f32 / n as f32;
        let u = 1.0 - t;
        points.push((
            u * u * from.0 + 2.0 * u * t * c.0 + t * t * to.0,
            u * u * from.1 + 2.0 * u * t * c.1 + t * t * to.1,
        ));
    }
}

/// Splits SVG path data into command letters and numbers.
fn tokens(d: &str) -> Vec<Result<f32, char>> {
    let mut out = Vec::new();
    let bytes = d.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        let c = bytes[i] as char;
        if c.is_ascii_alphabetic() && c != 'e' && c != 'E' {
            out.push(Err(c));
            i += 1;
        } else if c.is_ascii_digit() || c == '-' || c == '+' || c == '.' {
            let start = i;
            i += 1;
            let mut dot = c == '.';
            while i < bytes.len() {
                let n = bytes[i] as char;
                if n.is_ascii_digit() {
                    i += 1;
                } else if n == '.' && !dot {
                    dot = true;
                    i += 1;
                } else if (n == 'e' || n == 'E') && i + 1 < bytes.len() {
                    i += 1;
                    if matches!(bytes[i], b'-' | b'+') {
                        i += 1;
                    }
                } else {
                    break;
                }
            }
            if let Ok(value) = d[start..i].parse() {
                out.push(Ok(value));
            }
        } else {
            i += 1;
        }
    }
    out
}

/// Flattens SVG path data into subpaths of window points, each with whether it was closed.
pub fn flatten(d: &str, transform: &Affine) -> Vec<(Vec<(f32, f32)>, bool)> {
    let tokens = tokens(d);
    let mut subpaths = Vec::new();
    let mut points: Vec<(f32, f32)> = Vec::new();
    // Current and subpath start, in local units.
    let (mut cur, mut start) = ((0.0_f32, 0.0_f32), (0.0_f32, 0.0_f32));
    let mut command = 'M';
    let mut i = 0;
    let finish = |points: &mut Vec<(f32, f32)>, subpaths: &mut Vec<_>, closed: bool| {
        if points.len() > 1 {
            subpaths.push((std::mem::take(points), closed));
        } else {
            points.clear();
        }
    };
    let number = |i: &mut usize| -> Option<f32> {
        match tokens.get(*i) {
            Some(Ok(value)) => {
                *i += 1;
                Some(*value)
            }
            _ => None,
        }
    };
    while i < tokens.len() {
        if let Err(letter) = tokens[i] {
            command = letter;
            i += 1;
            if letter.eq_ignore_ascii_case(&'z') {
                if let Some(&first) = points.first()
                    && points.last() == Some(&first)
                {
                    points.pop();
                }
                finish(&mut points, &mut subpaths, true);
                cur = start;
                continue;
            }
        }
        let relative = command.is_ascii_lowercase();
        let base = if relative { cur } else { (0.0, 0.0) };
        let window = |p: (f32, f32)| transform.apply(p.0, p.1);
        match command.to_ascii_uppercase() {
            'M' => {
                let (Some(x), Some(y)) = (number(&mut i), number(&mut i)) else {
                    break;
                };
                finish(&mut points, &mut subpaths, false);
                cur = (base.0 + x, base.1 + y);
                start = cur;
                points.push(window(cur));
                // Further pairs after a move are lines.
                command = if relative { 'l' } else { 'L' };
            }
            'L' => {
                let (Some(x), Some(y)) = (number(&mut i), number(&mut i)) else {
                    break;
                };
                cur = (base.0 + x, base.1 + y);
                points.push(window(cur));
            }
            'H' => {
                let Some(x) = number(&mut i) else { break };
                cur = (base.0 + x, cur.1);
                points.push(window(cur));
            }
            'V' => {
                let Some(y) = number(&mut i) else { break };
                cur = (cur.0, base.1 + y);
                points.push(window(cur));
            }
            'C' => {
                let values: Option<Vec<f32>> = (0..6).map(|_| number(&mut i)).collect();
                let Some(v) = values else { break };
                let c1 = (base.0 + v[0], base.1 + v[1]);
                let c2 = (base.0 + v[2], base.1 + v[3]);
                let to = (base.0 + v[4], base.1 + v[5]);
                let from = window(cur);
                cubic(&mut points, from, window(c1), window(c2), window(to));
                cur = to;
            }
            'Q' => {
                let values: Option<Vec<f32>> = (0..4).map(|_| number(&mut i)).collect();
                let Some(v) = values else { break };
                let c = (base.0 + v[0], base.1 + v[1]);
                let to = (base.0 + v[2], base.1 + v[3]);
                let from = window(cur);
                quadratic(&mut points, from, window(c), window(to));
                cur = to;
            }
            _ => break,
        }
    }
    finish(&mut points, &mut subpaths, false);
    subpaths
}

/// The window pixel at `(fx, fy)` fractions of `bounds`.
pub fn at_fraction(bounds: Bounds<Pixels>, fx: f32, fy: f32) -> (f32, f32) {
    (
        f32::from(bounds.origin.x) + f32::from(bounds.size.width) * fx,
        f32::from(bounds.origin.y) + f32::from(bounds.size.height) * fy,
    )
}

/// A rectangle from window pixel edges, for content masks.
pub fn rect(x0: f32, y0: f32, x1: f32, y1: f32) -> Bounds<Pixels> {
    Bounds::from_corners(
        Point::new(px(x0.min(x1)), px(y0.min(y1))),
        Point::new(px(x0.max(x1)), px(y0.max(y1))),
    )
}

/// Paints `child` inside a rectangle computed from its bounds, then optionally `after` on top,
/// unclipped (it gets the child's bounds). The rectangular cousin of `clip-path`: GPUI clips to
/// rectangles only.
pub struct Clip {
    child: AnyElement,
    mask: Box<dyn Fn(Bounds<Pixels>) -> Option<Bounds<Pixels>>>,
    after: Option<Box<dyn FnOnce(Bounds<Pixels>, &mut Window)>>,
    shown: Option<Bounds<Pixels>>,
}

/// Wraps `child` so it only paints inside `mask(bounds)`, or not at all when that is `None`. The
/// child keeps its layout.
pub fn clip(
    child: impl IntoElement,
    mask: impl Fn(Bounds<Pixels>) -> Option<Bounds<Pixels>> + 'static,
) -> Clip {
    Clip {
        child: child.into_any_element(),
        mask: Box::new(mask),
        after: None,
        shown: None,
    }
}

impl Clip {
    /// Paints over the child, outside the clip, with the child's bounds.
    pub fn paint_after(
        mut self,
        after: impl FnOnce(Bounds<Pixels>, &mut Window) + 'static,
    ) -> Self {
        self.after = Some(Box::new(after));
        self
    }
}

impl IntoElement for Clip {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

impl Element for Clip {
    type RequestLayoutState = ();
    type PrepaintState = ();

    fn id(&self) -> Option<ElementId> {
        None
    }

    fn source_location(&self) -> Option<&'static std::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, ()) {
        (self.child.request_layout(window, cx), ())
    }

    fn prepaint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        _: &mut (),
        window: &mut Window,
        cx: &mut App,
    ) {
        self.shown = (self.mask)(bounds).filter(|mask| !mask.is_empty());
        if let Some(mask) = self.shown {
            window.with_content_mask(Some(ContentMask { bounds: mask }), |window| {
                self.child.prepaint(window, cx)
            });
        }
    }

    fn paint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        _: &mut (),
        _: &mut (),
        window: &mut Window,
        cx: &mut App,
    ) {
        let Some(mask) = self.shown else { return };
        window.with_content_mask(Some(ContentMask { bounds: mask }), |window| {
            self.child.paint(window, cx);
        });
        if let Some(after) = self.after.take() {
            after(bounds, window);
        }
    }
}

/// `bounds` with its edges rounded to whole device pixels. A content mask on a fractional edge
/// blends what it clips into its neighbour; snapped masks of adjacent cells meet exactly.
pub fn snap_to_pixels(bounds: Bounds<Pixels>, scale: f32) -> Bounds<Pixels> {
    let snap = |v: Pixels| (f32::from(v) * scale).round() / scale;
    rect(
        snap(bounds.origin.x),
        snap(bounds.origin.y),
        snap(bounds.origin.x + bounds.size.width),
        snap(bounds.origin.y + bounds.size.height),
    )
}

/// The `rotate()` of a point around `origin`, in degrees clockwise.
pub fn rotate_point(p: (f32, f32), origin: (f32, f32), degrees: f32) -> (f32, f32) {
    Affine::rotate(degrees)
        .around(origin.0, origin.1)
        .apply(p.0, p.1)
}

/// Degrees to the angle CSS gradients use: 0° points up, 90° right.
pub fn css_direction(degrees: f32) -> (f32, f32) {
    let a = degrees * PI / 180.0;
    (a.sin(), -a.cos())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: (f32, f32), b: (f32, f32)) -> bool {
        (a.0 - b.0).abs() < 1e-3 && (a.1 - b.1).abs() < 1e-3
    }

    #[test]
    fn rotation_turns_clockwise_on_screen() {
        // Right becomes down, like CSS rotate(90deg).
        assert!(close(Affine::rotate(90.0).apply(1.0, 0.0), (0.0, 1.0)));
    }

    #[test]
    fn then_applies_in_order() {
        let t = Affine::scale(2.0, 2.0).then(Affine::translate(10.0, 0.0));
        assert!(close(t.apply(1.0, 1.0), (12.0, 2.0)));
        let t = Affine::translate(10.0, 0.0).then(Affine::scale(2.0, 2.0));
        assert!(close(t.apply(1.0, 1.0), (22.0, 2.0)));
    }

    #[test]
    fn around_keeps_the_origin_fixed() {
        let t = Affine::scale(1.3, 0.7).around(64.0, 112.0);
        assert!(close(t.apply(64.0, 112.0), (64.0, 112.0)));
        assert!(close(t.apply(74.0, 112.0), (77.0, 112.0)));
    }

    #[test]
    fn parses_compact_path_data() {
        // Relative h and l after a number, and a closing z.
        let paths = flatten("M61.6 75.2h4.8l-2.4 2.6z", &Affine::IDENTITY);
        assert_eq!(paths.len(), 1);
        let (points, closed) = &paths[0];
        assert!(closed);
        assert_eq!(points.len(), 3);
        assert!(close(points[1], (66.4, 75.2)));
        assert!(close(points[2], (64.0, 77.8)));
    }

    #[test]
    fn moves_start_new_subpaths() {
        let paths = flatten("M58 41V47M64 39V47.5M70 41V47", &Affine::IDENTITY);
        assert_eq!(paths.len(), 3);
        assert!(close(paths[1].0[1], (64.0, 47.5)));
    }

    #[test]
    fn curves_end_on_their_end_point() {
        let paths = flatten(
            "M94 101C116 103 129 92 127 77Q120 70 112 73",
            &Affine::IDENTITY,
        );
        let points = &paths[0].0;
        assert!(close(*points.last().unwrap(), (112.0, 73.0)));
        assert!(points.len() > 4);
    }

    #[test]
    fn holes_turn_against_fills() {
        let mut shape = Shape::new();
        shape.polygon(Shape::circle_points(0.0, 0.0, 10.0));
        shape.hole(Shape::circle_points(0.0, 0.0, 5.0));
        let areas: Vec<f32> = shape.contours.iter().map(|c| signed_area(c)).collect();
        assert!(areas[0] > 0.0 && areas[1] < 0.0);
    }

    #[test]
    fn css_directions() {
        assert!(close(css_direction(0.0), (0.0, -1.0)));
        assert!(close(css_direction(90.0), (1.0, 0.0)));
        assert!(close(css_direction(180.0), (0.0, 1.0)));
    }
}
