//! Collapsible: content that opens like a Pop Accordion panel.
//!
//! Replaces `gpui_kit::component::collapsible`, rebuilt on gpui-base, so
//! `use kirakira::collapsible::*` is a drop-in.
//!
//! With a [`Collapsible::motion_id`] (GPUI Component's switch for a measured reveal), opening grows
//! the content to its measured height, runs 6 px past it and settles in 0.38 s, and the content
//! fades and rises half a rem 0.08 s later on Kirakira's `out` curve. Closing falls shut in 0.2 s on
//! the `in` curve while the content fades. Without a motion id the content shows and hides at once,
//! as in GPUI Component. Nothing plays on first render.
//!
//! Under reduced motion the content opens and closes at once and fades in.
//!
//! Differences from the web version: the web clips with `clip-path` only while it moves, so focus
//! rings inside aren't cut off at rest; GPUI clips to the reveal's bounds whenever it is drawn, and
//! the content rises as one piece rather than child by child. Interactive, so not
//! composition-safe.

use gpui_kit::base::{Collapsible as BaseCollapsible, StyledExt as _};
use gpui_kit::{
    AnyElement, App, ElementId, IntoElement, ParentElement, RenderOnce, StyleRefinement, Styled,
    Window,
};

use crate::accordion::panel_frame;
use crate::motion::transform;
use crate::state_motion::Reveal;

/// A child of a [`Collapsible`], in the order it was added.
enum Child {
    /// Always shown: `child()` and `children()`.
    Element(AnyElement),
    /// Shown while open: `content()`.
    Content(AnyElement),
}

/// An interactive element which expands/collapses.
#[derive(IntoElement)]
pub struct Collapsible {
    base: BaseCollapsible,
    style: StyleRefinement,
    motion_id: Option<ElementId>,
    open: bool,
    children: Vec<Child>,
}

impl Default for Collapsible {
    fn default() -> Self {
        Self::new()
    }
}

impl Collapsible {
    /// Creates a new `Collapsible` instance.
    pub fn new() -> Self {
        Self {
            base: BaseCollapsible::new(),
            style: StyleRefinement::default(),
            motion_id: None,
            open: false,
            children: Vec::new(),
        }
    }

    /// Sets whether the collapsible is open. default is false.
    pub fn open(mut self, open: bool) -> Self {
        self.open = open;
        self.base = self.base.open(open);
        self
    }

    /// Enables the Kirakira reveal under a stable identity.
    pub fn motion_id(mut self, id: impl Into<ElementId>) -> Self {
        self.motion_id = Some(id.into());
        self
    }

    /// Sets the content of the collapsible.
    ///
    /// If `open` is false, content will be hidden.
    pub fn content(mut self, content: impl IntoElement) -> Self {
        self.children
            .push(Child::Content(content.into_any_element()));
        self
    }
}

impl Styled for Collapsible {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl ParentElement for Collapsible {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children
            .extend(elements.into_iter().map(Child::Element));
    }
}

impl RenderOnce for Collapsible {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let mut base = self.base;
        let frame = self
            .motion_id
            .as_ref()
            .map(|id| (id, panel_frame(id, self.open, window, cx)));
        if frame.is_some() {
            // The content stays mounted; the reveal measures it and draws it closed.
            base = base.open(true);
        }
        // One ordered list, as in gpui-base, so content sits where it was added among the
        // children; only the content entries are revealed.
        let mut content_ix = 0;
        for child in self.children {
            base = match child {
                Child::Element(element) => base.child(element),
                Child::Content(content) => match &frame {
                    Some((id, frame)) => {
                        let key = crate::motion::child_id(id, content_ix);
                        content_ix += 1;
                        base.content(Reveal::new(
                            (key.clone(), "kk-reveal"),
                            frame.fraction,
                            frame.extra,
                            transform((key, "kk-inner"), frame.inner, content),
                        ))
                    }
                    None => base.content(content),
                },
            };
        }
        base.v_flex().refine_style(&self.style)
    }
}

#[cfg(test)]
mod tests {
    use crate::accordion::{fade_track, rise_tracks};
    use crate::motion::Easing;
    use crate::parity::assert_number_track;

    #[test]
    fn keyframes_match_the_web() {
        let (rise, appear) = rise_tracks(Easing::EaseInOut);
        // The parser reads `0.5rem` as 0.5, and the track is in rems.
        assert_number_track("pop-collapsible", "kk-pop-collapsible-rise", "y", &rise);
        assert_number_track(
            "pop-collapsible",
            "kk-pop-collapsible-rise",
            "opacity",
            &appear,
        );
        assert_number_track(
            "pop-collapsible",
            "kk-pop-collapsible-fade",
            "opacity",
            &fade_track(Easing::EaseIn),
        );
        // `kk-pop-collapsible-open` keys `height` on `calc(var(--collapsible-panel-height))`,
        // which the parity parser can't read; it is the accordion's open track, checked there.
    }
}
