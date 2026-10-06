//! Separator: a line that draws out from its centre.
//!
//! Replaces `gpui_kit::component::separator`, rebuilt with GPUI Component's structure, so
//! `use kirakira::separator::*` is a drop-in.
//!
//! The line grows along its own axis from nothing at its centre to full length in 0.5 s on
//! Kirakira's `snap` curve: it hangs at the centre for a moment, shoots out and brakes hard. It
//! waits until it scrolls into view by default ([`Trigger::InView`]); a label stays put while the
//! line draws behind it. Under reduced motion the line is simply there.
//!
//! Composition-safe: inside a [`Timeline`](crate::timeline::Timeline) it draws on the timeline's
//! clock.
//!
//! GPUI Component's separators take no id. Each separator's clock is keyed by
//! [`Separator::id`], which defaults to the place in the source that built it, so separators
//! written out one by one start apart, each when it scrolls into view. Separators built in a loop
//! share that place: under one parent they share a clock, so give each its own id when they should
//! start apart (their `delay`s still stagger them either way). The line is an owned shape, so its
//! length is exact, dashed lines included.

use std::panic::Location;

use gpui_kit::base::StyledExt as _;
use gpui_kit::component::ActiveTheme as _;
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::{
    App, Axis, Div, ElementId, Hsla, IntoElement, ParentElement as _, PathBuilder, RenderOnce,
    SharedString, StyleRefinement, Styled, Window, canvas, div, point, px, relative,
};

pub use gpui_kit::component::separator::SeparatorStyle;

use crate::motion::{Clock, Keyframes, Timing, Track, Trigger, delay_ms, ms};
use crate::theme::ActiveKira as _;

/// A separator that can be either vertical or horizontal.
#[derive(IntoElement)]
pub struct Separator {
    base: Div,
    style: StyleRefinement,
    label: Option<SharedString>,
    axis: Axis,
    color: Option<Hsla>,
    line_style: SeparatorStyle,
    id: ElementId,
    trigger: Trigger,
    delay: u64,
    duration: u64,
}

impl Separator {
    fn new(base: Div, axis: Axis, caller: &'static Location<'static>) -> Self {
        Self {
            base,
            axis,
            label: None,
            color: None,
            style: StyleRefinement::default(),
            line_style: SeparatorStyle::Solid,
            id: default_id(caller, axis),
            trigger: Trigger::InView,
            delay: 0,
            duration: 500,
        }
    }

    /// Creates a vertical separator.
    #[track_caller]
    pub fn vertical() -> Self {
        Self::new(div().h_full(), Axis::Vertical, Location::caller())
    }

    /// Creates a horizontal separator.
    #[track_caller]
    pub fn horizontal() -> Self {
        Self::new(div(), Axis::Horizontal, Location::caller())
    }

    /// Creates a vertical dashed separator.
    #[track_caller]
    pub fn vertical_dashed() -> Self {
        Self::vertical().dashed()
    }

    /// Creates a horizontal dashed separator.
    #[track_caller]
    pub fn horizontal_dashed() -> Self {
        Self::horizontal().dashed()
    }

    /// Sets the label for the separator.
    pub fn label(mut self, label: impl Into<SharedString>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Sets the color for the separator line.
    pub fn color(mut self, color: impl Into<Hsla>) -> Self {
        self.color = Some(color.into());
        self
    }

    /// Sets the style of the separator to dashed.
    pub fn dashed(mut self) -> Self {
        self.line_style = SeparatorStyle::Dashed;
        self
    }

    /// Keys the draw's clock. By default, the source location that built the separator.
    pub fn id(mut self, id: impl Into<ElementId>) -> Self {
        self.id = id.into();
        self
    }

    /// When the line draws. [`Trigger::InView`] by default.
    pub fn trigger(mut self, trigger: Trigger) -> Self {
        self.trigger = trigger;
        self
    }

    /// Milliseconds before the line draws. 0 by default.
    pub fn delay(mut self, delay_ms: u64) -> Self {
        self.delay = delay_ms;
        self
    }

    /// Milliseconds for the draw. 500 by default.
    pub fn duration(mut self, duration_ms: u64) -> Self {
        self.duration = duration_ms;
        self
    }

    fn render_base(axis: Axis, length: f32) -> Div {
        let offset = relative((1.0 - length) * 0.5);
        div().absolute().map(|this| match axis {
            Axis::Vertical => this.w(px(1.)).h(relative(length)).top(offset),
            Axis::Horizontal => this.h(px(1.)).w(relative(length)).left(offset),
        })
    }

    fn render_solid(axis: Axis, length: f32, color: Hsla) -> impl IntoElement {
        Self::render_base(axis, length).bg(color)
    }

    fn render_dashed(axis: Axis, length: f32, color: Hsla) -> impl IntoElement {
        Self::render_base(axis, length).child(
            canvas(
                move |_, _, _| {},
                move |bounds, _, window, _| {
                    let mut builder = PathBuilder::stroke(px(1.)).dash_array(&[px(4.), px(2.)]);
                    let (start, end) = match axis {
                        Axis::Horizontal => {
                            let x = bounds.origin.x;
                            let y = bounds.origin.y + px(0.5);
                            (point(x, y), point(x + bounds.size.width, y))
                        }
                        Axis::Vertical => {
                            let x = bounds.origin.x + px(0.5);
                            let y = bounds.origin.y;
                            (point(x, y), point(x, y + bounds.size.height))
                        }
                    };
                    builder.move_to(start);
                    builder.line_to(end);
                    if let Ok(line) = builder.build() {
                        window.paint_path(line, color);
                    }
                },
            )
            .size_full(),
        )
    }
}

/// The clock key of a separator built at `caller`: one per call site and axis.
fn default_id(caller: &'static Location<'static>, axis: Axis) -> ElementId {
    let axis = match axis {
        Axis::Vertical => "kk-separator-v",
        Axis::Horizontal => "kk-separator-h",
    };
    (ElementId::from(caller), axis).into()
}

impl Styled for Separator {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

/// `kk-pop-separator-draw`: the line's length as a fraction of its own, from 0.
pub(crate) fn draw_track(snap: crate::motion::Easing) -> Keyframes<f32> {
    Track::new(snap).at(0.0, 0.0).at(1.0, 1.0).build()
}

impl RenderOnce for Separator {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let clock = Clock::new(self.id.clone(), self.trigger, window, cx);
        let timing = Timing::new(ms(self.duration)).delay(delay_ms(self.delay));
        clock.animate(Some(ms(self.delay + self.duration)), window);
        let length = clock.sample(&draw_track(cx.curves().snap), &timing);

        let color = self.color.unwrap_or(cx.theme().border);
        let axis = self.axis;
        let line_style = self.line_style;

        clock.observe(
            self.base
                .flex()
                .flex_shrink_0()
                .items_center()
                .justify_center()
                .refine_style(&self.style)
                .child(match line_style {
                    SeparatorStyle::Solid => {
                        Self::render_solid(axis, length, color).into_any_element()
                    }
                    SeparatorStyle::Dashed => {
                        Self::render_dashed(axis, length, color).into_any_element()
                    }
                })
                .when_some(self.label, |this, label| {
                    this.child(
                        div()
                            .px_2()
                            .py_1()
                            .mx_auto()
                            .text_xs()
                            .bg(cx.theme().tokens.background)
                            .text_color(cx.theme().muted_foreground)
                            .child(label),
                    )
                }),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keyframes_match_the_web() {
        use crate::parity::assert_number_track;
        let track = draw_track(crate::theme::Curves::default().snap);
        assert_number_track("pop-separator", "kk-pop-separator-draw", "sx", &track);
        assert_number_track(
            "pop-separator",
            "kk-pop-separator-draw-vertical",
            "sy",
            &track,
        );
    }

    #[test]
    fn separators_built_apart_get_their_own_clocks() {
        let a = Separator::horizontal();
        let b = Separator::horizontal();
        let dashed = Separator::horizontal_dashed();
        assert_ne!(a.id, b.id);
        assert_ne!(a.id, dashed.id);
        // A loop builds them in one place, so they share one.
        let looped: Vec<_> = (0..2).map(|_| Separator::vertical().id).collect();
        assert_eq!(looped[0], looped[1]);
        assert_eq!(Separator::horizontal().id("x").id, ElementId::from("x"));
    }

    #[test]
    fn draws_from_nothing_to_full_length() {
        let track = draw_track(crate::theme::Curves::default().snap);
        assert_eq!(track.sample(0.0), 0.0);
        assert_eq!(track.sample(1.0), 1.0);
        // The snap curve hangs at the centre first.
        assert!(track.sample(0.2) < 0.05);
    }
}
