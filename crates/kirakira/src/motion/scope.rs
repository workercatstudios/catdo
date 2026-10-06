use gpui_kit::{
    AnyElement, App, Bounds, Element, ElementId, GlobalElementId, InspectorElementId, IntoElement,
    LayoutId, Pixels, TextStyleRefinement, Window,
};

use super::{ScopeTime, with_scope_time};

/// Lays out `child` with `time` as the scope time, so every [`Clock`](super::Clock) inside reads
/// it. This is how a [`Timeline`](crate::timeline::Timeline) and its scenes drive the components
/// under them.
pub fn scoped(time: ScopeTime, child: impl IntoElement) -> impl IntoElement {
    Scoped {
        time,
        child: child.into_any_element(),
    }
}

struct Scoped {
    time: ScopeTime,
    child: AnyElement,
}

impl IntoElement for Scoped {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

impl Element for Scoped {
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
        let child = &mut self.child;
        let layout_id = with_scope_time(self.time, || child.request_layout(window, cx));
        (layout_id, ())
    }

    fn prepaint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        _: Bounds<Pixels>,
        _: &mut (),
        window: &mut Window,
        cx: &mut App,
    ) {
        self.child.prepaint(window, cx);
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

/// Draws `child` with every rem-based size and the inherited text size multiplied by `scale`.
///
/// GPUI has no transform for quads and glyphs; this is the scale it does have. Pixel sizes don't
/// change, so callers position the result themselves.
pub(crate) fn rescaled(scale: f32, child: impl IntoElement) -> impl IntoElement {
    Rescaled {
        scale,
        child: child.into_any_element(),
        rescale: None,
    }
}

struct Rescaled {
    scale: f32,
    child: AnyElement,
    rescale: Option<(Pixels, TextStyleRefinement)>,
}

fn with_rescale<R>(
    rescale: &Option<(Pixels, TextStyleRefinement)>,
    window: &mut Window,
    f: impl FnOnce(&mut Window) -> R,
) -> R {
    {
        match rescale {
            Some((rem, text)) => window.with_rem_size(Some(*rem), |window| {
                window.with_text_style(Some(text.clone()), f)
            }),
            None => f(window),
        }
    }
}

impl IntoElement for Rescaled {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

impl Element for Rescaled {
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
        let rem = window.rem_size();
        let font_size = window.text_style().font_size.to_pixels(rem);
        self.rescale = Some((
            rem * self.scale,
            TextStyleRefinement {
                font_size: Some((font_size * self.scale).into()),
                ..Default::default()
            },
        ));
        let layout_id = with_rescale(&self.rescale, window, |window| {
            self.child.request_layout(window, cx)
        });
        (layout_id, ())
    }

    fn prepaint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        _: Bounds<Pixels>,
        _: &mut (),
        window: &mut Window,
        cx: &mut App,
    ) {
        with_rescale(&self.rescale, window, |window| {
            self.child.prepaint(window, cx)
        });
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
        with_rescale(&self.rescale, window, |window| self.child.paint(window, cx));
    }
}
