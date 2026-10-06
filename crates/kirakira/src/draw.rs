//! Drawing helpers for what CSS does with `clip-path`, gradients and SVG strokes.
//!
//! - [`clip`] clips a subtree to its own box grown or shrunk on each side, like
//!   `clip-path: inset(...)`, without touching layout. GPUI's `overflow_hidden` clips both axes to
//!   the border box; this clips any rectangle, one axis only too.
//! - [`offset`] moves a subtree by an amount worked out from its laid-out bounds, for motion that
//!   depends on where an element landed (a letter riding a tilted ribbon).
//! - Polygons: [`clip_half_plane`], [`clip_convex`] and [`rounded_rect`] cut hard-edged gradient
//!   bands into shapes; [`fill_polygon`] paints one.
//! - Polylines: [`polyline_lengths`], [`sub_polyline`] and [`stroke_polyline`] draw part of a pen
//!   stroke with round caps and joins, the way SVG's `stroke-dasharray` reveals a path.

use std::f32::consts::TAU;

use gpui_kit::{
    AnyElement, App, Bounds, ContentMask, Element, ElementId, GlobalElementId, Hsla,
    InspectorElementId, IntoElement, LayoutId, PathBuilder, Pixels, Point, Window, point, px,
};

/// One side of a [`clip`] inset: a fraction of the box's size plus pixels, like
/// `calc(50% + 0.5em)`. Positive values cut into the box, negative ones grow the clip past it.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct Side {
    pub frac: f32,
    pub px: f32,
}

impl Side {
    /// The box's own edge.
    pub const EDGE: Self = Self { frac: 0.0, px: 0.0 };
    /// Far enough out that nothing on this side is clipped.
    pub const OPEN: Self = Self {
        frac: 0.0,
        px: -1.0e5,
    };

    pub const fn frac(frac: f32) -> Self {
        Self { frac, px: 0.0 }
    }

    pub const fn px(px: f32) -> Self {
        Self { frac: 0.0, px }
    }

    fn resolve(self, size: Pixels) -> Pixels {
        size * self.frac + px(self.px)
    }
}

/// The four sides of a [`clip`], in CSS order: top, right, bottom, left.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct Inset {
    pub top: Side,
    pub right: Side,
    pub bottom: Side,
    pub left: Side,
}

impl Inset {
    pub const fn new(top: Side, right: Side, bottom: Side, left: Side) -> Self {
        Self {
            top,
            right,
            bottom,
            left,
        }
    }

    /// The clip rectangle for a box at `bounds`.
    pub fn apply(&self, bounds: Bounds<Pixels>) -> Bounds<Pixels> {
        let size = bounds.size;
        let min = point(
            bounds.origin.x + self.left.resolve(size.width),
            bounds.origin.y + self.top.resolve(size.height),
        );
        let max = point(
            bounds.origin.x + size.width - self.right.resolve(size.width),
            bounds.origin.y + size.height - self.bottom.resolve(size.height),
        );
        let max = point(max.x.max(min.x), max.y.max(min.y));
        Bounds::from_corners(min, max)
    }
}

/// Clips `child` to its own box adjusted by `inset`. Layout is the child's own.
pub(crate) fn clip(inset: Inset, child: impl IntoElement) -> Clip {
    Clip {
        inset,
        child: child.into_any_element(),
    }
}

pub(crate) struct Clip {
    inset: Inset,
    child: AnyElement,
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
        let mask = ContentMask {
            bounds: self.inset.apply(bounds),
        };
        window.with_content_mask(Some(mask), |window| self.child.prepaint(window, cx));
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
        let mask = ContentMask {
            bounds: self.inset.apply(bounds),
        };
        window.with_content_mask(Some(mask), |window| self.child.paint(window, cx));
    }
}

/// Moves `child` by `by(bounds)` once it is laid out, without moving anything around it.
pub(crate) fn offset(
    by: impl Fn(Bounds<Pixels>) -> Point<Pixels> + 'static,
    child: impl IntoElement,
) -> Offset {
    Offset {
        by: Box::new(by),
        child: child.into_any_element(),
    }
}

pub(crate) struct Offset {
    by: Box<dyn Fn(Bounds<Pixels>) -> Point<Pixels>>,
    child: AnyElement,
}

impl IntoElement for Offset {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

impl Element for Offset {
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
        let by = (self.by)(bounds);
        // Bounds are fixed at prepaint, so the paint pass follows.
        window.with_element_offset(by, |window| self.child.prepaint(window, cx));
    }

    fn paint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        _: Bounds<Pixels>,
        _: &mut (),
        _: &mut (),
        window: &mut Window,
        cx: &mut App,
    ) {
        self.child.paint(window, cx);
    }
}

/// A point in plain `f32`s, for geometry that is easier without units.
pub(crate) type Pt = (f32, f32);

/// Keeps the part of the convex or concave `polygon` where `normal · p <= limit`
/// (Sutherland–Hodgman against one half-plane).
pub(crate) fn clip_half_plane(polygon: &[Pt], normal: Pt, limit: f32) -> Vec<Pt> {
    let inside = |p: Pt| normal.0 * p.0 + normal.1 * p.1 <= limit;
    let cross = |a: Pt, b: Pt| {
        let da = normal.0 * a.0 + normal.1 * a.1 - limit;
        let db = normal.0 * b.0 + normal.1 * b.1 - limit;
        let t = da / (da - db);
        (a.0 + (b.0 - a.0) * t, a.1 + (b.1 - a.1) * t)
    };
    let mut out = Vec::with_capacity(polygon.len() + 1);
    for (i, &current) in polygon.iter().enumerate() {
        let previous = polygon[(i + polygon.len() - 1) % polygon.len()];
        match (inside(previous), inside(current)) {
            (true, true) => out.push(current),
            (true, false) => out.push(cross(previous, current)),
            (false, true) => {
                out.push(cross(previous, current));
                out.push(current);
            }
            (false, false) => {}
        }
    }
    out
}

/// Twice the signed area of `polygon`: positive when it turns clockwise on screen (y down).
pub(crate) fn signed_area2(polygon: &[Pt]) -> f32 {
    (0..polygon.len())
        .map(|i| {
            let a = polygon[i];
            let b = polygon[(i + 1) % polygon.len()];
            a.0 * b.1 - b.0 * a.1
        })
        .sum()
}

/// Clips `polygon` to the inside of the convex polygon `window`, which may wind either way.
pub(crate) fn clip_convex(polygon: &[Pt], window: &[Pt]) -> Vec<Pt> {
    let winding = signed_area2(window).signum();
    let mut out = polygon.to_vec();
    for i in 0..window.len() {
        if out.is_empty() {
            break;
        }
        let a = window[i];
        let b = window[(i + 1) % window.len()];
        // The outward normal of edge a→b, given the winding. A near-zero edge has no reliable
        // direction (float noise can flip it), so it is skipped.
        let edge = (b.0 - a.0, b.1 - a.1);
        if edge.0.hypot(edge.1) < 1e-3 {
            continue;
        }
        let normal = (edge.1 * winding, -edge.0 * winding);
        let limit = normal.0 * a.0 + normal.1 * a.1;
        out = clip_half_plane(&out, normal, limit);
    }
    out
}

/// A rectangle with rounded corners as a convex polygon, clockwise on screen. `radii` are top-left,
/// top-right, bottom-right and bottom-left, clamped to fit.
pub(crate) fn rounded_rect(origin: Pt, size: Pt, radii: [f32; 4], segments: usize) -> Vec<Pt> {
    let (x, y) = origin;
    let (w, h) = size;
    let limit = (w.min(h) / 2.0).max(0.0);
    let corners = [
        ((x, y), 0.5, radii[0]),
        ((x + w, y), 0.75, radii[1]),
        ((x + w, y + h), 0.0, radii[2]),
        ((x, y + h), 0.25, radii[3]),
    ];
    let mut out = Vec::new();
    for ((cx, cy), start, radius) in corners {
        let r = radius.clamp(0.0, limit);
        if r <= 0.0 {
            out.push((cx, cy));
            continue;
        }
        // The centre of the corner's circle sits `r` inside the corner.
        let centre = (
            if cx > x { cx - r } else { cx + r },
            if cy > y { cy - r } else { cy + r },
        );
        for step in 0..=segments {
            let turn = start + 0.25 * step as f32 / segments as f32;
            let angle = turn * TAU;
            out.push((centre.0 + r * angle.cos(), centre.1 + r * angle.sin()));
        }
    }
    // A pill's arcs meet end to end; drop the repeated points.
    out.dedup_by(|b, a| (a.0 - b.0).hypot(a.1 - b.1) < 1e-3);
    if out.len() > 1
        && (out[0].0 - out[out.len() - 1].0).hypot(out[0].1 - out[out.len() - 1].1) < 1e-3
    {
        out.pop();
    }
    out
}

/// Fills a polygon given in window pixels.
pub(crate) fn fill_polygon(window: &mut Window, polygon: &[Pt], color: Hsla) {
    if polygon.len() < 3 {
        return;
    }
    let mut builder = PathBuilder::fill();
    builder.move_to(point(px(polygon[0].0), px(polygon[0].1)));
    for &(x, y) in &polygon[1..] {
        builder.line_to(point(px(x), px(y)));
    }
    builder.close();
    if let Ok(path) = builder.build() {
        window.paint_path(path, color);
    }
}

/// The running length at each point of `points`: 0 at the first, the total at the last.
pub(crate) fn polyline_lengths(points: &[Pt]) -> Vec<f32> {
    let mut total = 0.0;
    let mut out = Vec::with_capacity(points.len());
    for (i, &p) in points.iter().enumerate() {
        if i > 0 {
            let q = points[i - 1];
            total += ((p.0 - q.0).powi(2) + (p.1 - q.1).powi(2)).sqrt();
        }
        out.push(total);
    }
    out
}

/// The part of the polyline between lengths `from` and `to`, with its ends interpolated.
pub(crate) fn sub_polyline(points: &[Pt], lengths: &[f32], from: f32, to: f32) -> Vec<Pt> {
    let at = |length: f32| -> Pt {
        let i = lengths
            .partition_point(|&l| l < length)
            .clamp(1, points.len() - 1);
        let (a, b) = (points[i - 1], points[i]);
        let span = lengths[i] - lengths[i - 1];
        let t = if span > 0.0 {
            ((length - lengths[i - 1]) / span).clamp(0.0, 1.0)
        } else {
            0.0
        };
        (a.0 + (b.0 - a.0) * t, a.1 + (b.1 - a.1) * t)
    };
    if points.len() < 2 || to <= from {
        return Vec::new();
    }
    let mut out = vec![at(from)];
    for (i, &p) in points.iter().enumerate() {
        if lengths[i] > from && lengths[i] < to {
            out.push(p);
        }
    }
    out.push(at(to));
    out
}

/// A circle as a polygon winding the same way as [`stroke_polyline`]'s segments.
fn dot(centre: Pt, radius: f32, segments: usize) -> Vec<Pt> {
    (0..segments)
        .map(|i| {
            let angle = TAU * i as f32 / segments as f32;
            (
                centre.0 + radius * angle.cos(),
                centre.1 + radius * angle.sin(),
            )
        })
        .collect()
}

/// The outline pieces of a polyline stroke: a quad per segment, plus a dot at each end and at
/// every turn sharper than a few degrees, so caps and joins are round. They all wind the same
/// way, so a non-zero fill paints their union once.
pub(crate) fn stroke_outline(points: &[Pt], width: f32) -> Vec<Vec<Pt>> {
    let half = width / 2.0;
    let mut pieces = Vec::new();
    if points.is_empty() || half <= 0.0 {
        return pieces;
    }
    let segments = ((width * 1.5) as usize).clamp(10, 32);
    pieces.push(dot(points[0], half, segments));
    let mut previous: Option<Pt> = None;
    for pair in points.windows(2) {
        let (a, b) = (pair[0], pair[1]);
        let d = (b.0 - a.0, b.1 - a.1);
        let length = (d.0 * d.0 + d.1 * d.1).sqrt();
        if length < 1e-4 {
            continue;
        }
        let dir = (d.0 / length, d.1 / length);
        if let Some(previous) = previous {
            let turn = previous.0 * dir.0 + previous.1 * dir.1;
            if turn < 0.995 {
                pieces.push(dot(a, half, segments));
            }
        }
        previous = Some(dir);
        let n = (-dir.1 * half, dir.0 * half);
        pieces.push(vec![
            (a.0 - n.0, a.1 - n.1),
            (b.0 - n.0, b.1 - n.1),
            (b.0 + n.0, b.1 + n.1),
            (a.0 + n.0, a.1 + n.1),
        ]);
    }
    if points.len() > 1 {
        pieces.push(dot(points[points.len() - 1], half, segments));
    }
    pieces
}

/// Strokes a polyline given in window pixels with round caps and joins.
pub(crate) fn stroke_polyline(window: &mut Window, points: &[Pt], width: f32, color: Hsla) {
    let pieces = stroke_outline(points, width);
    if pieces.is_empty() {
        return;
    }
    let mut builder = PathBuilder::fill();
    for piece in &pieces {
        builder.move_to(point(px(piece[0].0), px(piece[0].1)));
        for &(x, y) in &piece[1..] {
            builder.line_to(point(px(x), px(y)));
        }
        builder.close();
    }
    if let Ok(path) = builder.build() {
        window.paint_path(path, color);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui_kit::size;

    #[test]
    fn insets_grow_and_shrink_the_box() {
        let bounds = Bounds::new(point(px(10.), px(20.)), size(px(100.), px(40.)));
        // clip-path: inset(-50% 100% -50% -50%): nothing shows yet.
        let hidden = Inset::new(
            Side::frac(-0.5),
            Side::frac(1.0),
            Side::frac(-0.5),
            Side::frac(-0.5),
        )
        .apply(bounds);
        // The clip ends at the box's left edge.
        assert_eq!(hidden.origin, point(px(-40.), px(0.)));
        assert_eq!(hidden.origin.x + hidden.size.width, bounds.origin.x);
        assert_eq!(hidden.size.height, px(80.));
        let clipped = Inset::new(Side::EDGE, Side::px(5.), Side::px(-3.), Side::EDGE).apply(bounds);
        assert_eq!(clipped.size, size(px(95.), px(43.)));
    }

    #[test]
    fn half_plane_cuts_a_square() {
        let square = [(0.0, 0.0), (2.0, 0.0), (2.0, 2.0), (0.0, 2.0)];
        let left = clip_half_plane(&square, (1.0, 0.0), 1.0);
        assert!((signed_area2(&left).abs() - 4.0).abs() < 1e-5);
        assert!(clip_half_plane(&square, (1.0, 0.0), -1.0).is_empty());
    }

    #[test]
    fn convex_clip_works_either_winding() {
        let square = [(0.0, 0.0), (4.0, 0.0), (4.0, 4.0), (0.0, 4.0)];
        let window = [(2.0, -1.0), (6.0, -1.0), (6.0, 6.0), (2.0, 6.0)];
        let mut reversed = window;
        reversed.reverse();
        for window in [window, reversed] {
            let part = clip_convex(&square, &window);
            assert!((signed_area2(&part).abs() / 2.0 - 8.0).abs() < 1e-4);
        }
    }

    #[test]
    fn rounded_rect_loses_the_corners() {
        let rect = rounded_rect((0.0, 0.0), (10.0, 10.0), [2.0; 4], 8);
        let area = signed_area2(&rect) / 2.0;
        // A square less four corners of (1 - pi/4) r².
        let expected = 100.0 - 4.0 * (1.0 - std::f32::consts::FRAC_PI_4) * 4.0;
        assert!((area - expected).abs() < 0.1, "{area} vs {expected}");
        assert!(
            rect.iter()
                .all(|&(x, y)| (0.0..=10.0).contains(&x) && (0.0..=10.0).contains(&y))
        );
    }

    #[test]
    fn sub_polyline_takes_a_stretch() {
        let line = [(0.0, 0.0), (10.0, 0.0), (10.0, 10.0)];
        let lengths = polyline_lengths(&line);
        assert_eq!(lengths, [0.0, 10.0, 20.0]);
        let part = sub_polyline(&line, &lengths, 5.0, 15.0);
        assert_eq!(part, [(5.0, 0.0), (10.0, 0.0), (10.0, 5.0)]);
        assert!(sub_polyline(&line, &lengths, 5.0, 5.0).is_empty());
    }

    #[test]
    fn stroke_pieces_wind_one_way() {
        let line = [(0.0, 0.0), (10.0, 0.0), (10.0, 10.0), (0.0, 12.0)];
        let pieces = stroke_outline(&line, 4.0);
        let sign = signed_area2(&pieces[0]).signum();
        assert!(
            pieces
                .iter()
                .all(|piece| signed_area2(piece).signum() == sign)
        );
        // Two caps, three segments and two turns.
        assert_eq!(pieces.len(), 7);
    }
}
