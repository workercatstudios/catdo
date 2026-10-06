use gpui_kit::base::StyledExt as _;
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::{
    AnyElement, App, Bounds, Element, ElementId, GlobalElementId, InspectorElementId, IntoElement,
    LayoutId, ParentElement as _, Pixels, Point, Size, StyleRefinement, Styled as _,
    TextStyleRefinement, Window, div, point, relative,
};

use super::Pose;

/// Applies a [`Pose`] to any element without disturbing the layout around it.
///
/// - Translation and opacity are exact: the element moves like a CSS `translate`, so its siblings
///   stay where they are. It moves inside the wrapper, so the wrapper keeps the position and insets
///   [`Transform::outer_style`] gives it: an absolutely placed element moves from where it's
///   placed.
/// - Scale is uniform ([`Pose::uniform_scale`]): GPUI can't scale painted quads or glyphs, so the
///   element is drawn with its rem size and inherited text size scaled instead. Everything sized
///   in rems (GPUI's `p_4`, `text_sm`, `h_9`...) and every inherited font size follows; pixel
///   sizes and borders don't. Meanwhile the element keeps its natural size in the layout, measured
///   while it rests at scale 1, and scales around `origin`.
/// - Rotation needs a painted shape; see [`crate::shapes`].
///
/// An element that mounts already scaled is drawn hidden for its first frame while it is
/// measured. Entrances start hidden anyway, so this doesn't show.
pub struct Transform {
    id: ElementId,
    pose: Pose,
    origin: Point<f32>,
    outer: StyleRefinement,
    child: Option<AnyElement>,
}

/// Wraps `child` in a [`Transform`] keyed by `id`, scaling around its centre.
pub fn transform(id: impl Into<ElementId>, pose: Pose, child: impl IntoElement) -> Transform {
    Transform {
        id: id.into(),
        pose,
        origin: point(0.5, 0.5),
        outer: StyleRefinement::default(),
        child: Some(child.into_any_element()),
    }
}

/// Moves the layout half of `style` out: position and inset, size, margins and the flex and grid
/// item fields. Put it on the wrapper with [`Transform::outer_style`] and the rest on the
/// element, so `.flex_1()` or `.w_full()` on a wrapped component still sizes it in its parent.
///
/// Where the wrapper's size comes from its parent, the element is set to fill the wrapper.
pub fn split_layout(style: &mut StyleRefinement) -> StyleRefinement {
    let outer = StyleRefinement {
        position: style.position.take(),
        inset: std::mem::take(&mut style.inset),
        margin: std::mem::take(&mut style.margin),
        align_self: style.align_self.take(),
        flex_basis: style.flex_basis.take(),
        flex_grow: style.flex_grow.take(),
        flex_shrink: style.flex_shrink.take(),
        size: std::mem::take(&mut style.size),
        min_size: std::mem::take(&mut style.min_size),
        max_size: std::mem::take(&mut style.max_size),
        grid_location: style.grid_location.take(),
        ..Default::default()
    };
    let grows = outer.flex_grow.is_some_and(|grow| grow > 0.0);
    if outer.size.width.is_some() || outer.min_size.width.is_some() || grows {
        style.size.width = Some(relative(1.0).into());
    }
    if outer.size.height.is_some() || outer.min_size.height.is_some() {
        style.size.height = Some(relative(1.0).into());
    }
    outer
}

impl Transform {
    /// The point that stays put while scaling, as fractions of the element's size, like
    /// `transform-origin`. `(0.5, 1.0)` is the bottom centre.
    pub fn origin(mut self, x: f32, y: f32) -> Self {
        self.origin = point(x, y);
        self
    }

    /// Layout styles for the wrapper, usually from [`split_layout`].
    pub fn outer_style(mut self, style: StyleRefinement) -> Self {
        self.outer = style;
        self
    }
}

#[derive(Clone, Copy, Default)]
struct TransformState {
    natural: Option<Size<Pixels>>,
    /// The rem size the natural size was measured at. Inside a `Stage` or another rescaled
    /// subtree the rem size changes, and so does the natural size with it.
    rem: Pixels,
}

pub struct TransformLayout {
    element: AnyElement,
    /// Laid out at scale 1, so prepaint should record its size.
    measure: bool,
    /// The rem size and text size to draw with while scaled.
    scaled: Option<(Pixels, TextStyleRefinement)>,
}

impl IntoElement for Transform {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

const EPSILON: f32 = 1e-4;

/// `child` moved by `offset` without moving anything around it.
///
/// The offset goes on a box of its own inside the wrapper, never on the wrapper: the wrapper
/// carries the element's position and insets (`absolute().bottom_4().right_4()`), which a `left`
/// or `top` there would replace. The box fills the wrapper (a block stretches across it, and
/// `h_full` resolves only where the wrapper's height is definite), so the child sizes against the
/// same box as before. At rest there's no box at all.
fn translated(child: AnyElement, offset: Point<Pixels>) -> AnyElement {
    if offset == Point::default() {
        return child;
    }
    div()
        .relative()
        .h_full()
        .when(offset.x != Pixels::ZERO, |this| this.left(offset.x))
        .when(offset.y != Pixels::ZERO, |this| this.top(offset.y))
        .child(child)
        .into_any_element()
}

impl Transform {
    fn with_scale<R>(
        scaled: &Option<(Pixels, TextStyleRefinement)>,
        window: &mut Window,
        f: impl FnOnce(&mut Window) -> R,
    ) -> R {
        match scaled {
            Some((rem, text)) => window.with_rem_size(Some(*rem), |window| {
                window.with_text_style(Some(text.clone()), f)
            }),
            None => f(window),
        }
    }
}

impl Element for Transform {
    type RequestLayoutState = TransformLayout;
    type PrepaintState = ();

    fn id(&self) -> Option<ElementId> {
        Some(self.id.clone())
    }

    fn source_location(&self) -> Option<&'static std::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        global_id: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, Self::RequestLayoutState) {
        let rem_now = window.rem_size();
        let natural = window.with_element_state(
            global_id.expect("Transform has an id"),
            |state: Option<TransformState>, _| {
                let state = state.unwrap_or_default();
                let natural = state.natural.map(|natural| {
                    if state.rem > Pixels::ZERO && state.rem != rem_now {
                        let ratio = rem_now / state.rem;
                        gpui_kit::size(natural.width * ratio, natural.height * ratio)
                    } else {
                        natural
                    }
                });
                (natural, state)
            },
        );
        let pose = self.pose;
        let child = self.child.take().expect("Transform lays out once");
        let scale = pose.uniform_scale();
        let is_scaled = (scale - 1.0).abs() > EPSILON;
        let offset = |size: Size<Pixels>| pose.offset(size);

        let (element, measure, scaled) = match natural.filter(|_| is_scaled) {
            Some(natural) => {
                let rem = window.rem_size();
                let font_size = window.text_style().font_size.to_pixels(rem);
                let text = TextStyleRefinement {
                    font_size: Some((font_size * scale).into()),
                    ..Default::default()
                };
                let inner = gpui_kit::size(natural.width * scale, natural.height * scale);
                let offset = offset(natural);
                let element = div()
                    .relative()
                    .refine_style(&self.outer)
                    .flex_none()
                    .min_w_0()
                    .min_h_0()
                    .w(natural.width)
                    .h(natural.height)
                    .opacity(pose.alpha())
                    .child(
                        div()
                            .absolute()
                            .left((natural.width - inner.width) * self.origin.x + offset.x)
                            .top((natural.height - inner.height) * self.origin.y + offset.y)
                            // A minimum, not a fixed size: scaled glyphs don't scale exactly
                            // linearly, and a label one pixel short would ellipsize.
                            .min_w(inner.width)
                            .min_h(inner.height)
                            .child(child),
                    )
                    .into_any_element();
                (element, false, Some((rem * scale, text)))
            }
            None => {
                // At rest, or scaled before it was ever measured: lay out naturally and measure.
                // A pose that needs the size it hasn't measured yet stays hidden for that frame.
                let unmeasured =
                    natural.is_none() && (is_scaled || pose.xp != 0.0 || pose.yp != 0.0);
                let offset = offset(natural.unwrap_or_default());
                let element = div()
                    .relative()
                    .refine_style(&self.outer)
                    .opacity(if unmeasured { 0.0 } else { pose.alpha() })
                    .when(unmeasured, |this| this.invisible())
                    .child(translated(child, offset))
                    .into_any_element();
                if unmeasured {
                    window.request_animation_frame();
                }
                (element, true, None)
            }
        };

        let mut layout = TransformLayout {
            element,
            measure,
            scaled,
        };
        let layout_id = Self::with_scale(&layout.scaled, window, |window| {
            layout.element.request_layout(window, cx)
        });
        (layout_id, layout)
    }

    fn prepaint(
        &mut self,
        global_id: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        layout: &mut Self::RequestLayoutState,
        window: &mut Window,
        cx: &mut App,
    ) {
        if layout.measure {
            window.with_element_state(
                global_id.expect("Transform has an id"),
                |_: Option<TransformState>, window| {
                    (
                        (),
                        TransformState {
                            natural: Some(bounds.size),
                            rem: window.rem_size(),
                        },
                    )
                },
            );
        }
        let scaled = layout.scaled.clone();
        Self::with_scale(&scaled, window, |window| {
            layout.element.prepaint(window, cx);
        });
    }

    fn paint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        _: Bounds<Pixels>,
        layout: &mut Self::RequestLayoutState,
        _: &mut Self::PrepaintState,
        window: &mut Window,
        cx: &mut App,
    ) {
        let scaled = layout.scaled.clone();
        Self::with_scale(&scaled, window, |window| {
            layout.element.paint(window, cx);
        });
    }
}
