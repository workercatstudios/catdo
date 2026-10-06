//! Non-uniform squash for the base kit's navigation controls: page links, carousel arrows,
//! calendar days.
//!
//! The web components squash a control with `scale: x y` while it is held and spring it back when
//! it is let go. GPUI can't scale a quad, so [`Squashed`] treats the control's box as an owned
//! shape: it measures the control at rest, then lays it out in a box of the squashed size, centred
//! where it rests, so its background and border squash exactly. Its text and icons scale evenly
//! (rem size), by the factor the caller passes.
//!
//! [`Plated`] is exact for a control that is a plate and a label (a page link, a day pill): while
//! it moves, the real control turns transparent (it keeps its layout, hover and clicks) and a
//! [`Vector`](crate::vector::Vector) of its plate, border and label is drawn over it under the
//! full pose, so the label squashes with the plate. At rest it is the real control.

use gpui_kit::base::ElementExt as _;
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::{
    App, Bounds, ElementId, Hsla, IntoElement, ParentElement as _, Pixels, RenderOnce,
    SharedString, Size, Styled, TextStyleRefinement, Window, div, point,
};

use crate::motion::{Easing, Pose, ms, rescaled};
use crate::state_motion::glide;
use crate::theme::ActiveKira as _;
use crate::vector::{Layer, TextSetting, Vector};

const EPSILON: f32 = 1e-4;

/// How far a held control squashes, as `scale: x y`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Squash {
    pub x: f32,
    pub y: f32,
}

impl Squash {
    pub const fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }

    /// The scale this frame: towards the squash in 0.08 s (ease-out) while `pressed`, back to 1 on
    /// the spring curve in 0.35 s once let go. `(1, 1)` under reduced motion. Timed by
    /// [`motion::now`](crate::motion::now), so screenshots can pin it.
    pub fn scale(
        self,
        id: &ElementId,
        pressed: bool,
        window: &mut Window,
        cx: &mut App,
    ) -> (f32, f32) {
        let (duration, easing) = if pressed {
            (ms(80), Easing::EaseOut)
        } else {
            (ms(350), cx.curves().spring)
        };
        let (x, y) = if pressed {
            (self.x, self.y)
        } else {
            (1.0, 1.0)
        };
        let x = glide(
            (id.clone(), "kk-squash-x"),
            x,
            duration,
            easing.clone(),
            window,
            cx,
        );
        let y = glide((id.clone(), "kk-squash-y"), y, duration, easing, window, cx);
        if cx.reduce_motion() {
            (1.0, 1.0)
        } else {
            (x, y)
        }
    }
}

/// Draws `child` scaled by `(x, y)` around `origin` without moving anything around it.
///
/// The child's box is resized directly (exact, non-uniform); everything sized in rems inside it
/// and its text scale evenly by `text`. The child must take an explicit size, so this is for
/// controls whose own box is the shape: buttons, day cells.
#[derive(IntoElement)]
pub(crate) struct Squashed<E: Styled + IntoElement + 'static> {
    id: ElementId,
    x: f32,
    y: f32,
    text: f32,
    origin: (f32, f32),
    natural: Option<Size<Pixels>>,
    child: E,
}

pub(crate) fn squashed<E: Styled + IntoElement + 'static>(
    id: impl Into<ElementId>,
    (x, y): (f32, f32),
    child: E,
) -> Squashed<E> {
    Squashed {
        id: id.into(),
        x,
        y,
        text: 1.0,
        origin: (0.5, 0.5),
        natural: None,
        child,
    }
}

impl<E: Styled + IntoElement + 'static> Squashed<E> {
    /// Scale the text and rem-sized contents evenly by `text`.
    pub fn text(mut self, text: f32) -> Self {
        self.text = text;
        self
    }

    /// The child's size at rest, when it's fixed: skips measuring it.
    pub fn natural(mut self, size: Size<Pixels>) -> Self {
        self.natural = Some(size);
        self
    }
}

/// Where a box of `natural` size lands when scaled by `(x, y)` around `origin`: its offset and
/// size inside the natural box.
fn squashed_box(natural: Size<Pixels>, (x, y): (f32, f32), origin: (f32, f32)) -> Bounds<Pixels> {
    let size = gpui_kit::size(natural.width * x.max(0.0), natural.height * y.max(0.0));
    Bounds::new(
        point(
            (natural.width - size.width) * origin.0,
            (natural.height - size.height) * origin.1,
        ),
        size,
    )
}

struct Natural(Option<Size<Pixels>>);

impl<E: Styled + IntoElement + 'static> RenderOnce for Squashed<E> {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let natural =
            window.use_keyed_state((self.id.clone(), "kk-natural"), cx, |_, _| Natural(None));
        let at_rest = (self.x - 1.0).abs() < EPSILON
            && (self.y - 1.0).abs() < EPSILON
            && (self.text - 1.0).abs() < EPSILON;
        let measured = self.natural.or(natural.read(cx).0);
        match measured.filter(|_| !at_rest) {
            Some(size) => {
                let squashed = squashed_box(size, (self.x, self.y), self.origin);
                div()
                    .relative()
                    .flex_none()
                    .w(size.width)
                    .h(size.height)
                    .child(rescaled(
                        self.text.max(0.0),
                        self.child
                            .absolute()
                            .left(squashed.origin.x)
                            .top(squashed.origin.y)
                            .w(squashed.size.width)
                            .h(squashed.size.height),
                    ))
                    .into_any_element()
            }
            None => div()
                .flex()
                .flex_none()
                .child(self.child)
                .when(self.natural.is_none(), |this| {
                    this.on_prepaint(move |bounds, _, cx| {
                        if natural.read(cx).0 != Some(bounds.size) {
                            natural.update(cx, |natural, _| natural.0 = Some(bounds.size));
                        }
                    })
                })
                .into_any_element(),
        }
    }
}

/// What a [`Plated`] control looks like while it moves: one colour per layer.
#[derive(Clone, Debug)]
pub(crate) struct Plate {
    pub fill: Option<Hsla>,
    pub border: Option<Hsla>,
    pub radius: Pixels,
    pub label: SharedString,
    pub color: Hsla,
    /// The label's text style, as the control sets it.
    pub text: TextStyleRefinement,
}

/// Draws `child` under `pose` (around its centre) as a [`Plate`] while it moves; the real `child`
/// at rest.
#[derive(IntoElement)]
pub(crate) struct Plated<E: Styled + IntoElement + 'static> {
    id: ElementId,
    pose: Pose,
    plate: Plate,
    natural: Option<Size<Pixels>>,
    child: E,
}

pub(crate) fn plated<E: Styled + IntoElement + 'static>(
    id: impl Into<ElementId>,
    pose: Pose,
    plate: Plate,
    child: E,
) -> Plated<E> {
    Plated {
        id: id.into(),
        pose,
        plate,
        natural: None,
        child,
    }
}

impl<E: Styled + IntoElement + 'static> Plated<E> {
    /// The child's size, when it's fixed: skips measuring it.
    pub fn natural(mut self, size: Size<Pixels>) -> Self {
        self.natural = Some(size);
        self
    }
}

/// The plate's layers for a box of `size`: border, fill, then the label centred on its baseline.
fn plate_layers(plate: &Plate, size: Size<Pixels>, window: &Window) -> Vec<Layer> {
    let mut layers = Vec::with_capacity(3);
    let mut inset = Pixels::ZERO;
    if let Some(border) = plate.border {
        layers.push(Layer::rounded_rect(
            size,
            plate.radius,
            Pixels::ZERO,
            border,
        ));
        inset = gpui_kit::px(1.);
    }
    if let Some(fill) = plate.fill {
        layers.push(Layer::rounded_rect(
            size,
            (plate.radius - inset).max(Pixels::ZERO),
            inset,
            fill,
        ));
    }
    let setting = TextSetting::new(&plate.text, window);
    let line = setting.line_box(&plate.label, window);
    let baseline = (size.height - line.size.height) / 2. + line.baseline;
    layers.push(Layer::text(
        &plate.label,
        &setting.face,
        size,
        baseline,
        plate.color,
    ));
    layers
}

impl<E: Styled + IntoElement + 'static> RenderOnce for Plated<E> {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let natural =
            window.use_keyed_state((self.id.clone(), "kk-natural"), cx, |_, _| Natural(None));
        let measured = self.natural.or(natural.read(cx).0);
        let moving = !self.pose.is_identity();
        match measured.filter(|_| moving) {
            Some(size) => div()
                .relative()
                .flex()
                .flex_none()
                .child(self.child.opacity(0.))
                .child(
                    div().absolute().top_0().left_0().child(
                        Vector::new(size)
                            .layers(plate_layers(&self.plate, size, window))
                            .pose(self.pose),
                    ),
                )
                .into_any_element(),
            None => div()
                .flex()
                .flex_none()
                .child(self.child)
                .when(self.natural.is_none(), |this| {
                    this.on_prepaint(move |bounds, _, cx| {
                        if natural.read(cx).0 != Some(bounds.size) {
                            natural.update(cx, |natural, _| natural.0 = Some(bounds.size));
                        }
                    })
                })
                .into_any_element(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use gpui_kit::{px, size};

    #[test]
    fn a_squash_stays_centred_where_it_rests() {
        let squashed = squashed_box(size(px(32.), px(32.)), (1.08, 0.9), (0.5, 0.5));
        assert!((f32::from(squashed.size.width) - 34.56).abs() < 1e-3);
        assert!((f32::from(squashed.size.height) - 28.8).abs() < 1e-3);
        assert!((f32::from(squashed.origin.x) + 1.28).abs() < 1e-3);
        assert!((f32::from(squashed.origin.y) - 1.6).abs() < 1e-3);
    }

    #[test]
    fn a_squash_can_hang_from_its_top() {
        let squashed = squashed_box(size(px(40.), px(20.)), (0.5, 0.5), (0.5, 0.0));
        assert_eq!(squashed.origin, point(px(10.), px(0.)));
    }
}
