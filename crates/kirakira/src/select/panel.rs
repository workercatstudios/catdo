//! The list's panel: a plate that squashes on two axes, with the list scaled evenly inside it.
//!
//! `motion::transform` can only scale a subtree evenly (by its rem size), so a list squashed to
//! (0.95, 0.6) would shrink to their geometric mean, 0.75, on both axes. The plate is an owned
//! shape, though: its background, ring and shadow are one rounded rect, which takes any size. So
//! the plate is sized to the exact two-axis scale, and the list scales evenly with the plate's
//! width and is clipped to the plate. Where the web squashes the rows vertically, the plate unfolds
//! over them instead.

use gpui_kit::component::ThemeStyled as _;
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::{
    AnyElement, App, Bounds, ContentMask, Element, ElementId, GlobalElementId, InspectorElementId,
    IntoElement, LayoutId, ParentElement as _, Pixels, Size, Styled as _, Window, div, point,
};

use super::motion::PanelPose;
use crate::motion::{Pose, transform};

/// The plate's rect inside a panel of `natural` size scaled by `pose` about `origin` (fractions
/// of its size, like `transform-origin`).
pub(super) fn plate_rect(
    natural: Size<Pixels>,
    pose: PanelPose,
    origin: (f32, f32),
) -> Bounds<Pixels> {
    let size = gpui_kit::size(natural.width * pose.sx, natural.height * pose.sy);
    Bounds::new(
        point(
            (natural.width - size.width) * origin.0,
            (natural.height - size.height) * origin.1,
        ),
        size,
    )
}

/// The panel at `pose`: `content` on a plate styled like GPUI Component's popovers.
///
/// `natural` is the panel's size at rest. Until it is known the panel draws unscaled.
pub(super) fn panel(
    id: impl Into<ElementId>,
    pose: PanelPose,
    natural: Option<Size<Pixels>>,
    origin: (f32, f32),
    content: impl IntoElement,
    cx: &App,
) -> gpui_kit::Div {
    let plate = natural
        .filter(|_| !pose.unscaled())
        .map(|natural| plate_rect(natural, pose, origin));
    let shadow = (pose.alpha() < 1.0).then(|| {
        let alpha = pose.alpha();
        crate::overlay::shadow(alpha * alpha * alpha, cx)
    });
    div()
        .relative()
        .opacity(pose.alpha())
        .child(
            div()
                .absolute()
                .popover_style(cx)
                .map(|this| match plate {
                    Some(plate) => this
                        .left(plate.origin.x)
                        .top(plate.origin.y)
                        .w(plate.size.width)
                        .h(plate.size.height),
                    None => this.top_0().left_0().size_full(),
                })
                .when_some(shadow, |this, shadow| this.shadow(shadow)),
        )
        .child(ClipTo {
            rect: plate,
            child: transform(id, Pose::new().scale(pose.sx), content)
                .origin(origin.0, origin.1)
                .into_any_element(),
        })
}

/// Clips `child` to `rect`, in its own coordinates: hit testing too. Lays out as `child` does.
struct ClipTo {
    rect: Option<Bounds<Pixels>>,
    child: AnyElement,
}

impl ClipTo {
    /// The mask for `rect` in an element at `bounds`.
    fn mask(&self, bounds: Bounds<Pixels>) -> Option<ContentMask<Pixels>> {
        self.rect.map(|rect| ContentMask {
            bounds: Bounds::new(bounds.origin + rect.origin, rect.size),
        })
    }
}

impl IntoElement for ClipTo {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

impl Element for ClipTo {
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
    ) -> (LayoutId, Self::RequestLayoutState) {
        (self.child.request_layout(window, cx), ())
    }

    fn prepaint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        _: &mut Self::RequestLayoutState,
        window: &mut Window,
        cx: &mut App,
    ) {
        let mask = self.mask(bounds);
        window.with_content_mask(mask, |window| self.child.prepaint(window, cx));
    }

    fn paint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        _: &mut Self::RequestLayoutState,
        _: &mut Self::PrepaintState,
        window: &mut Window,
        cx: &mut App,
    ) {
        let mask = self.mask(bounds);
        window.with_content_mask(mask, |window| self.child.paint(window, cx));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui_kit::px;

    #[test]
    fn the_plate_squashes_towards_its_origin() {
        let near = |actual: Pixels, expected: f32| (f32::from(actual) - expected).abs() < 1e-3;
        let natural = gpui_kit::size(px(200.), px(100.));
        let pose = PanelPose {
            sx: 0.95,
            sy: 0.6,
            opacity: 1.0,
        };
        // Below its trigger: from the top centre.
        let rect = plate_rect(natural, pose, (0.5, 0.0));
        assert!(near(rect.size.width, 190.) && near(rect.size.height, 60.));
        assert!(near(rect.origin.x, 5.) && near(rect.origin.y, 0.));
        // Above it: from the bottom centre.
        let rect = plate_rect(natural, pose, (0.5, 1.0));
        assert!(near(rect.origin.x, 5.) && near(rect.origin.y, 40.));
    }
}
